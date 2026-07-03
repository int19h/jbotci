//! Public semantic object graph model serialized by `tersmu --format json`.

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::str::FromStr;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use jbotci_source::SourceSpan;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

pub const SEMANTIC_JSON_VERSION: &str = "lojban-semantics-json-1";

/// One-based numbered argument place such as `x1`.
///
/// `Ord` follows the numeric index, not the serialized label text. This
/// deliberately replaces the old string-key lexicographic JSON map order, so
/// maps containing `x2` and `x10` serialize in numeric place order.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceIndex(NonZeroUsize);

impl PlaceIndex {
    #[requires(index > 0)]
    #[ensures(ret.get() == index)]
    pub fn new(index: usize) -> Self {
        Self(NonZeroUsize::new(index).expect("place indices are one-based"))
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn get(self) -> usize {
        self.0.get()
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|place| place.get() > 0))]
    pub fn from_numbered_label(place: &str) -> Option<Self> {
        let digits = place.strip_prefix('x')?;
        if digits.is_empty() || digits.starts_with('0') {
            return None;
        }
        digits
            .parse::<usize>()
            .ok()
            .and_then(NonZeroUsize::new)
            .map(Self)
    }
}

impl fmt::Display for PlaceIndex {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "x{}", self.get())
    }
}

impl FromStr for PlaceIndex {
    type Err = ();

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|place| place.get() > 0) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_numbered_label(value).ok_or(())
    }
}

impl Serialize for PlaceIndex {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[invariant(*index > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticObjectId {
    prefix: SemanticIdPrefix,
    index: usize,
}

impl SemanticObjectId {
    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Utterance)]
    pub fn utterance(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Utterance),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Sequence),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn eventuality(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::eventuality(), index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn referent(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::Entity, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(sort))]
    pub fn referent_with_sort(sort: SemanticSort, index: usize) -> Self {
        Self::numbered(SemanticIdPrefix::Referent(sort), index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Parameter)]
    pub fn parameter(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Parameter),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn predication(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Predication),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn formula(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Formula),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::AbstractNature))]
    pub fn abstraction(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::AbstractNature, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::Sign))]
    pub fn sign(index: usize) -> Self {
        Self::referent_with_sort(SemanticSort::Sign, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::DisplayedContent)]
    pub fn displayed_content(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::DisplayedContent),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::MathExpression),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Quantity)]
    pub fn quantity(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Quantity),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::RelationMetadata)]
    pub fn relation_metadata(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::RelationMetadata),
            index,
        )
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    pub fn question(index: usize) -> Self {
        Self::numbered(
            SemanticIdPrefix::Structural(SemanticObjectKind::Question),
            index,
        )
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn speaker() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 1)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn addressee() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 2)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn now() -> Self {
        Self::referent_with_sort(SemanticSort::eventuality(), 3)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::eventuality()))]
    pub fn speech_time() -> Self {
        Self::now()
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn here() -> Self {
        Self::referent_with_sort(SemanticSort::Entity, 4)
    }

    #[requires(index > 0)]
    #[ensures(ret.prefix == prefix)]
    fn numbered(prefix: SemanticIdPrefix, index: usize) -> Self {
        new!(SemanticObjectId {
            prefix: prefix,
            index: index,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn object_kind(self) -> SemanticObjectKind {
        self.prefix.object_kind()
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (self.object_kind() == SemanticObjectKind::Referent))]
    pub fn referent_sort(self) -> Option<SemanticSort> {
        match self.prefix {
            SemanticIdPrefix::Referent(sort) => Some(sort),
            SemanticIdPrefix::Structural(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for SemanticObjectId {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.prefix, self.index)
    }
}

impl Serialize for SemanticObjectId {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[invariant(::Structural(_) => true)]
#[invariant(::Referent(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticIdPrefix {
    Structural(SemanticObjectKind),
    Referent(SemanticSort),
}

pub type SemanticReferentId = SemanticObjectId;

impl SemanticIdPrefix {
    #[requires(true)]
    #[ensures(true)]
    fn object_kind(self) -> SemanticObjectKind {
        match self {
            Self::Structural(kind) => kind,
            Self::Referent(_) => SemanticObjectKind::Referent,
        }
    }
}

impl fmt::Display for SemanticIdPrefix {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structural(kind) => formatter.write_str(kind.id_prefix_label()),
            Self::Referent(sort) => formatter.write_str(sort.label()),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticObjectKind {
    Utterance,
    Sequence,
    Eventuality,
    Referent,
    Parameter,
    Predication,
    Formula,
    Abstraction,
    Sign,
    DisplayedContent,
    MathExpression,
    Quantity,
    RelationMetadata,
    Question,
}

impl SemanticObjectKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn id_prefix_label(self) -> &'static str {
        match self {
            Self::Utterance => "utterance",
            Self::Sequence => "sequence",
            Self::Eventuality => "eventuality",
            Self::Referent => "referent",
            Self::Parameter => "parameter",
            Self::Predication => "predication",
            Self::Formula => "formula",
            Self::Abstraction => "abstraction",
            Self::Sign => "sign",
            Self::DisplayedContent => "display",
            Self::MathExpression => "math",
            Self::Quantity => "quantity",
            Self::RelationMetadata => "relationMetadata",
            Self::Question => "question",
        }
    }
}

#[invariant(*version == SEMANTIC_JSON_VERSION)]
#[invariant(objects.contains_key(root))]
#[expensive_invariant(semantic_object_ids_match_types(objects))]
#[expensive_invariant(semantic_object_references_are_defined(objects))]
#[expensive_invariant(semantic_object_references_match_roles(objects))]
#[expensive_invariant(semantic_object_arguments_are_valid(objects))]
#[expensive_invariant(semantic_object_compositions_are_valid(objects))]
#[expensive_invariant(semantic_object_question_slots_are_valid(objects))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGraph {
    pub version: &'static str,
    pub root: SemanticObjectId,
    #[serde(serialize_with = "serialize_objects")]
    pub objects: BTreeMap<SemanticObjectId, SemanticObject>,
}

#[invariant(::ObjectIdTypeMismatch(message) => !message.is_empty())]
#[invariant(::UndefinedReference { source, missing } => source != missing)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticGraphError {
    ObjectIdTypeMismatch(String),
    UndefinedReference {
        source: SemanticObjectId,
        missing: SemanticObjectId,
    },
    ReferenceRoleMismatch,
    InvalidArguments,
    InvalidCompositions,
    InvalidQuestionSlots,
}

impl fmt::Display for SemanticGraphError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(SemanticGraphError::ObjectIdTypeMismatch(message)) => {
                write!(
                    formatter,
                    "semantic object ID prefixes must match object types: {message}"
                )
            }
            data!(SemanticGraphError::UndefinedReference { source, missing }) => {
                write!(
                    formatter,
                    "semantic object references must not dangle: {source} references missing {missing}"
                )
            }
            data!(SemanticGraphError::ReferenceRoleMismatch) => {
                formatter.write_str("semantic object references must match semantic roles")
            }
            data!(SemanticGraphError::InvalidArguments) => formatter.write_str(
                "semantic arguments must use valid numbered places and argument fillers",
            ),
            data!(SemanticGraphError::InvalidCompositions) => {
                formatter.write_str("semantic compositions must use coherent parameters")
            }
            data!(SemanticGraphError::InvalidQuestionSlots) => {
                formatter.write_str("semantic question slots must use coherent parameters")
            }
        }
    }
}

impl std::error::Error for SemanticGraphError {}

impl SemanticGraph {
    #[requires(objects.contains_key(&root))]
    #[ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| graph.root == root))]
    pub fn new(
        root: SemanticObjectId,
        objects: BTreeMap<SemanticObjectId, SemanticObject>,
    ) -> Result<Self, SemanticGraphError> {
        if let Some(mismatch) = first_semantic_object_id_type_mismatch(&objects) {
            return Err(new!(SemanticGraphError::ObjectIdTypeMismatch(mismatch)));
        }
        if let Some((source, missing)) = first_undefined_semantic_reference(&objects) {
            return Err(new!(SemanticGraphError::UndefinedReference {
                source,
                missing,
            }));
        }
        if !semantic_object_references_match_roles(&objects) {
            return Err(new!(SemanticGraphError::ReferenceRoleMismatch));
        }
        if !semantic_object_arguments_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidArguments));
        }
        if !semantic_object_compositions_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidCompositions));
        }
        if !semantic_object_question_slots_are_valid(&objects) {
            return Err(new!(SemanticGraphError::InvalidQuestionSlots));
        }
        Ok(new!(SemanticGraph {
            version: SEMANTIC_JSON_VERSION,
            root: root,
            objects: objects,
        }))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
    pub fn to_json_string(&self, indent: usize) -> Result<String, serde_json::Error> {
        if indent == 0 {
            serde_json::to_string(self)
        } else {
            let mut buffer = Vec::new();
            let indent_bytes = vec![b' '; indent];
            let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
            let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
            self.serialize(&mut serializer)?;
            String::from_utf8(buffer)
                .map_err(|error| serde_json::Error::io(std::io::Error::other(error)))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn serialize_objects<S>(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(objects.len()))?;
    for (id, object) in objects {
        map.serialize_entry(&id.to_string(), object)?;
    }
    map.end()
}

#[requires(true)]
#[ensures(ret == !*value)]
fn bool_is_false(value: &bool) -> bool {
    !*value
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticObject {
    #[serde(rename = "type")]
    pub object_type: SemanticObjectKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<UtteranceForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eventuality: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deictic_ground: Option<DeicticGround>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asides: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocative_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connection_claims: Vec<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ordinal_labels: Vec<OrdinalLabel>,
    #[serde(rename = "relation", skip_serializing_if = "Option::is_none")]
    pub sequence_relation: Option<SequenceRelation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonlogical_connection: Option<NonlogicalConnection>,
    #[serde(skip)]
    pub class: Option<EventualityClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actuality: Option<Actuality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tense_modal: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<AnchorRelation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub time_path: Vec<TemporalPathStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_interval: Option<TimeInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_span: Option<TimeSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect: Option<Aspect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aspects: Vec<Aspect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recurrence: Vec<Recurrence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interval_modifiers: Vec<IntervalModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space: Option<AnchorRelation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub space_path: Vec<TemporalPathStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_interval: Option<SpaceInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spatial_aspect: Option<Aspect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spatial_aspects: Vec<Aspect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spatial_recurrence: Vec<Recurrence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spatial_interval_modifiers: Vec<IntervalModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ReferentCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SemanticSort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexical: Option<IndexicalKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<Descriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition: Option<Composition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assigned_names: Vec<AssignedName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ParameterRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_parameter: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tanru_link: Option<TanruLink>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<PlaceIndex, ArgumentValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub place_questions: Vec<PlaceQuestionBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modal_arguments: Vec<ModalArgument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reciprocity: Vec<ReciprocalExchange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PredicationMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_metadata: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<SemanticOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_parameter: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_denotes: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_inclusion: Option<IntervalEndpointInclusion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predication: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<Connector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<SemanticObjectId>,
    #[serde(rename = "sourceVariable", skip_serializing_if = "Option::is_none")]
    pub source_variable: Option<SemanticObjectId>,
    #[serde(rename = "selectionSource", skip_serializing_if = "Option::is_none")]
    pub selection_source: Option<SelectionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<QuantifierBinding>,
    #[serde(rename = "coequalScope", skip_serializing_if = "bool_is_false")]
    pub coequal_scope: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streams: Vec<RespectivelyStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct_partition: Option<bool>,
    #[serde(skip)]
    pub abstraction_kind: Option<AbstractionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstracted: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arity: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embedded_questions: Vec<SemanticObjectId>,
    #[serde(rename = "kind", skip_serializing_if = "Option::is_none")]
    pub sign_kind: Option<SignKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub letterals: Vec<LetteralUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotation: Option<Quotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denotes: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<DisplayedContentFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polarity: Option<DisplayedContentPolarity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<DisplayedContentModifier>,
    #[serde(rename = "assertionEffect", skip_serializing_if = "Option::is_none")]
    pub assertion_effect: Option<DisplayedContentAssertionEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiencer: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SemanticObjectId>,
    #[serde(rename = "targetFocus", skip_serializing_if = "Option::is_none")]
    pub target_focus: Option<DisplayedContentTargetFocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operands: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub literal: Option<MathLiteral>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<QuantityForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<QuantityValue>,
    #[serde(rename = "scale", skip_serializing_if = "Option::is_none")]
    pub quantity_scale: Option<QuantityScale>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison_set: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_words: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub place_structure: Vec<PlaceDescription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<RelationExpansion>,
    #[serde(rename = "kind", skip_serializing_if = "Option::is_none")]
    pub question_kind: Option<QuestionKind>,
    #[serde(rename = "mode", skip_serializing_if = "Option::is_none")]
    pub question_mode: Option<QuestionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asker: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respondent: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<SemanticSort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<QuestionSlot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presupposed_answer: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscript: Option<Subscript>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticObject {
    #[requires(true)]
    #[ensures(true)]
    fn empty(object_type: SemanticObjectKind) -> Self {
        Self {
            object_type,
            force: None,
            speaker: None,
            audience: None,
            eventuality: None,
            content: None,
            deictic_ground: None,
            asides: Vec::new(),
            vocative_kind: None,
            items: Vec::new(),
            connection_claims: Vec::new(),
            ordinal_labels: Vec::new(),
            sequence_relation: None,
            nonlogical_connection: None,
            class: None,
            actuality: None,
            tense_modal: None,
            time: None,
            time_path: Vec::new(),
            time_interval: None,
            time_span: None,
            aspect: None,
            aspects: Vec::new(),
            recurrence: Vec::new(),
            interval_modifiers: Vec::new(),
            space: None,
            space_path: Vec::new(),
            space_interval: None,
            spatial_aspect: None,
            spatial_aspects: Vec::new(),
            spatial_recurrence: Vec::new(),
            spatial_interval_modifiers: Vec::new(),
            category: None,
            sort: None,
            indexical: None,
            descriptor: None,
            composition: None,
            relative_clauses: Vec::new(),
            assigned_names: Vec::new(),
            role: None,
            introduced_by: None,
            relation: None,
            relation_parameter: None,
            tanru_link: None,
            arguments: BTreeMap::new(),
            place_questions: Vec::new(),
            modal_arguments: Vec::new(),
            reciprocity: Vec::new(),
            mode: None,
            scalar_negation: None,
            relation_metadata: None,
            operator: None,
            operator_parameter: None,
            operator_denotes: None,
            endpoint_inclusion: None,
            predication: None,
            children: Vec::new(),
            connector: None,
            variable: None,
            source_variable: None,
            selection_source: None,
            restriction: None,
            body: None,
            quantity: None,
            bindings: Vec::new(),
            coequal_scope: false,
            streams: Vec::new(),
            distinct_partition: None,
            abstraction_kind: None,
            abstracted: None,
            parameters: Vec::new(),
            arity: None,
            embedded_questions: Vec::new(),
            sign_kind: None,
            text: None,
            letterals: Vec::new(),
            quotation: None,
            denotes: None,
            family: None,
            intensity: None,
            polarity: None,
            phase: None,
            modifiers: Vec::new(),
            assertion_effect: None,
            experiencer: None,
            target: None,
            target_focus: None,
            anchor: None,
            operands: Vec::new(),
            literal: None,
            form: None,
            value: None,
            quantity_scale: None,
            scale: None,
            comparison_set: None,
            source_words: Vec::new(),
            place_structure: Vec::new(),
            expansion: None,
            question_kind: None,
            question_mode: None,
            asker: None,
            respondent: None,
            domain: None,
            slots: Vec::new(),
            focus: None,
            presupposed_answer: None,
            subscript: None,
            source: None,
            diagnostics: Vec::new(),
        }
    }

    #[requires(eventuality.object_kind() == SemanticObjectKind::Referent)]
    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(speaker.object_kind() == SemanticObjectKind::Referent)]
    #[requires(audience.object_kind() == SemanticObjectKind::Referent)]
    #[requires(now.object_kind() == SemanticObjectKind::Referent)]
    #[requires(now.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(here.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Utterance)]
    pub fn utterance(
        force: UtteranceForce,
        eventuality: SemanticObjectId,
        content: Option<SemanticObjectId>,
        speaker: SemanticObjectId,
        audience: SemanticObjectId,
        now: SemanticObjectId,
        here: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Utterance);
        object.force = Some(force);
        object.speaker = Some(speaker);
        object.audience = Some(audience);
        object.eventuality = Some(eventuality);
        object.content = content;
        object.deictic_ground = Some(DeicticGround {
            time: now,
            place: here,
        });
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence_with_nonlogical_connection(
        items: Vec<SemanticObjectId>,
        relation: SequenceRelation,
        nonlogical_connection: NonlogicalConnection,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::sequence(items, relation, source, diagnostics);
        object.nonlogical_connection = Some(nonlogical_connection);
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence(
        items: Vec<SemanticObjectId>,
        relation: SequenceRelation,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Sequence);
        object.items = items;
        object.sequence_relation = Some(relation);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence_with_connection_claims(
        items: Vec<SemanticObjectId>,
        relation: SequenceRelation,
        connection_claims: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::sequence(items, relation, source, diagnostics);
        object.connection_claims = connection_claims;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.sort.is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    pub fn eventuality(
        class: EventualityClass,
        actuality: Option<Actuality>,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Referent);
        object.category = Some(ReferentCategory::Constant);
        object.sort = Some(class.sort());
        object.class = Some(class);
        object.actuality = actuality;
        object.source = source;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn referent(
        category: ReferentCategory,
        sort: SemanticSort,
        indexical: Option<IndexicalKind>,
        descriptor: Option<Descriptor>,
        composition: Option<Composition>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Referent);
        object.category = Some(category);
        object.sort = Some(sort);
        object.indexical = indexical;
        object.descriptor = descriptor;
        object.composition = composition;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Parameter)]
    pub fn parameter(
        sort: SemanticSort,
        role: ParameterRole,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Parameter);
        object.sort = Some(sort);
        object.role = Some(role);
        object.introduced_by = Some(introduced_by);
        object.source = source;
        object
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn predication(
        relation: String,
        eventuality: Option<SemanticObjectId>,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        mode: PredicationMode,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Predication);
        object.relation = Some(relation);
        object.eventuality = eventuality;
        object.arguments = arguments;
        object.mode = Some(mode);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(!relation.is_empty())]
    #[requires(tanru_link.head.object_kind() == SemanticObjectKind::Predication)]
    #[requires(argument_object_kind_can_fill(tanru_link.modifier.object_kind()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn tanru_link_predication(
        relation: String,
        eventuality: Option<SemanticObjectId>,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        tanru_link: TanruLink,
        mode: PredicationMode,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object =
            Self::predication(relation, eventuality, arguments, mode, source, diagnostics);
        object.tanru_link = Some(tanru_link);
        object
    }

    #[requires(relation_parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn relation_parameter_predication(
        relation_parameter: SemanticObjectId,
        eventuality: Option<SemanticObjectId>,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        mode: PredicationMode,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Predication);
        object.relation_parameter = Some(relation_parameter);
        object.eventuality = eventuality;
        object.arguments = arguments;
        object.mode = Some(mode);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn atom_formula(
        predication: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Formula);
        object.operator = Some(SemanticOperator::formula(FormulaOperator::Atom));
        object.predication = Some(predication);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(!children.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn connective_formula(
        operator: FormulaOperator,
        children: Vec<SemanticObjectId>,
        connector: Option<Connector>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Formula);
        object.operator = Some(SemanticOperator::formula(operator));
        object.children = children;
        object.connector = connector;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(quantifier_formula_operator_is_allowed(operator))]
    #[requires(quantifier_variable_kind_is_allowed(variable.object_kind()))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn quantified_formula(
        operator: FormulaOperator,
        variable: SemanticObjectId,
        restriction: Option<SemanticObjectId>,
        body: SemanticObjectId,
        quantity: Option<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Formula);
        object.operator = Some(SemanticOperator::formula(operator));
        object.variable = Some(variable);
        object.restriction = restriction;
        object.body = Some(body);
        object.quantity = quantity;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(!bindings.is_empty())]
    #[requires(bindings.iter().all(quantifier_binding_matches_role))]
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn quantifier_bundle_formula(
        bindings: Vec<QuantifierBinding>,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Formula);
        object.operator = Some(SemanticOperator::formula(FormulaOperator::QuantifierBundle));
        object.bindings = bindings;
        object.coequal_scope = true;
        object.body = Some(body);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(!streams.is_empty())]
    #[requires(streams.iter().all(|stream| !stream.items.is_empty()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn respectively_distribution_formula(
        body: SemanticObjectId,
        streams: Vec<RespectivelyStream>,
        distinct_partition: Option<bool>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Formula);
        object.operator = Some(SemanticOperator::formula(
            FormulaOperator::RespectivelyDistribution,
        ));
        object.body = Some(body);
        object.streams = streams;
        object.distinct_partition = distinct_partition;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(parameters
        .iter()
        .all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn abstraction(
        kind: AbstractionKind,
        body: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Referent);
        object.category = Some(ReferentCategory::Constant);
        object.sort = Some(kind.output_sort());
        object.abstraction_kind = Some(kind);
        object.body = Some(body);
        if kind == AbstractionKind::Property {
            object.arity = Some(parameters.len());
        }
        object.parameters = parameters;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(slots.iter().all(|slot| slot.parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[requires(asker.object_kind() == SemanticObjectKind::Referent)]
    #[requires(respondent.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    pub fn question(
        kind: QuestionKind,
        mode: QuestionMode,
        domain: SemanticSort,
        body: SemanticObjectId,
        slots: Vec<QuestionSlot>,
        asker: SemanticObjectId,
        respondent: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Question);
        object.question_kind = Some(kind);
        object.question_mode = Some(mode);
        object.asker = Some(asker);
        object.respondent = Some(respondent);
        object.domain = Some(domain);
        object.body = Some(body);
        object.slots = slots;
        object.source = source;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.sort == Some(SemanticSort::Sign))]
    pub fn sign(
        sign_kind: SignKind,
        quotation: Option<Quotation>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Referent);
        object.category = Some(ReferentCategory::Constant);
        object.sort = Some(SemanticSort::Sign);
        object.sign_kind = Some(sign_kind);
        object.quotation = quotation;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(sign_kind != SignKind::Quotation)]
    #[requires(!text.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.sort == Some(SemanticSort::Sign))]
    pub fn text_sign(
        sign_kind: SignKind,
        text: String,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::sign(sign_kind, None, source, diagnostics);
        object.text = Some(text);
        object
    }

    #[requires(!relation.is_empty())]
    #[requires(experiencer.object_kind() == SemanticObjectKind::Referent)]
    #[requires(displayed_content_target_kind_is_allowed(target.object_kind()))]
    #[requires(anchor.object_kind() == SemanticObjectKind::Utterance)]
    #[ensures(ret.object_kind() == SemanticObjectKind::DisplayedContent)]
    pub fn displayed_content(
        family: DisplayedContentFamily,
        relation: String,
        polarity: DisplayedContentPolarity,
        assertion_effect: DisplayedContentAssertionEffect,
        experiencer: SemanticObjectId,
        target: SemanticObjectId,
        anchor: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::DisplayedContent);
        object.family = Some(family);
        object.relation = Some(relation);
        object.polarity = Some(polarity);
        object.assertion_effect = Some(assertion_effect);
        object.experiencer = Some(experiencer);
        object.target = Some(target);
        object.target_focus = None;
        object.anchor = Some(anchor);
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(literal.is_some() || operator.as_ref().is_some_and(|operator| !operator.is_empty()))]
    #[requires(literal.is_some() == operands.is_empty())]
    #[requires(operands
        .iter()
        .all(|operand| operand.object_kind() == SemanticObjectKind::MathExpression))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression(
        operator: Option<String>,
        operands: Vec<SemanticObjectId>,
        literal: Option<MathLiteral>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::MathExpression);
        object.operator = operator.map(SemanticOperator::math);
        object.operands = operands;
        object.literal = literal;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(operator.ends_with("Interval"))]
    #[requires(!operands.is_empty())]
    #[requires(operands
        .iter()
        .all(|operand| operand.object_kind() == SemanticObjectKind::MathExpression))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_interval_expression(
        operator: String,
        operands: Vec<SemanticObjectId>,
        endpoint_inclusion: Option<IntervalEndpointInclusion>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::math_expression(Some(operator), operands, None, source, diagnostics);
        object.endpoint_inclusion = endpoint_inclusion;
        object
    }

    #[requires(operator_parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!operands.is_empty())]
    #[requires(operands
        .iter()
        .all(|operand| operand.object_kind() == SemanticObjectKind::MathExpression))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression_with_operator_parameter(
        operator_parameter: SemanticObjectId,
        operands: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::MathExpression);
        object.operator_parameter = Some(operator_parameter);
        object.operands = operands;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(argument_object_kind_can_fill(denotes.object_kind()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_sumti_operand(
        denotes: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::math_expression(
            None,
            Vec::new(),
            Some(MathLiteral::text(
                "sumtiOperand".to_owned(),
                "mo'e".to_owned(),
            )),
            source,
            diagnostics,
        );
        object.denotes = Some(denotes);
        object
    }

    #[requires(argument_object_kind_can_fill(denotes.object_kind()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_selbri_operand(
        denotes: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::math_expression(
            None,
            Vec::new(),
            Some(MathLiteral::text(
                "selbriOperand".to_owned(),
                "ni'e".to_owned(),
            )),
            source,
            diagnostics,
        );
        object.denotes = Some(denotes);
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Quantity)]
    pub fn quantity(
        form: QuantityForm,
        value: QuantityValue,
        scale: QuantityScale,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Quantity);
        object.form = Some(form);
        object.value = Some(value);
        object.quantity_scale = Some(scale);
        object.source = source;
        object
    }

    #[requires(!relation.is_empty())]
    #[requires(source_words.iter().all(|word| !word.is_empty()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::RelationMetadata)]
    pub fn relation_metadata(
        relation: String,
        source_words: Vec<String>,
        place_structure: Vec<PlaceDescription>,
        expansion: Option<RelationExpansion>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::RelationMetadata);
        object.relation = Some(relation);
        object.source_words = source_words;
        object.place_structure = place_structure;
        object.expansion = expansion;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn object_kind(&self) -> SemanticObjectKind {
        self.object_type
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.speaker);
        extend_optional(out, self.audience);
        extend_optional(out, self.eventuality);
        extend_optional(out, self.content);
        if let Some(ground) = self.deictic_ground {
            out.extend([ground.time, ground.place]);
        }
        out.extend(self.asides.iter().copied());
        out.extend(self.items.iter().copied());
        out.extend(self.connection_claims.iter().copied());
        for label in &self.ordinal_labels {
            label.references_into(out);
        }
        if let Some(connection) = &self.nonlogical_connection {
            connection.references_into(out);
        }
        if let Some(time) = &self.time {
            time.references_into(out);
        }
        extend_optional(out, self.tense_modal);
        for step in &self.time_path {
            step.references_into(out);
        }
        if let Some(space) = &self.space {
            space.references_into(out);
        }
        for step in &self.space_path {
            step.references_into(out);
        }
        if let Some(time_interval) = &self.time_interval {
            time_interval.references_into(out);
        }
        if let Some(time_span) = &self.time_span {
            time_span.references_into(out);
        }
        if let Some(aspect) = &self.aspect {
            aspect.references_into(out);
        }
        for aspect in &self.aspects {
            aspect.references_into(out);
        }
        if let Some(space_interval) = &self.space_interval {
            space_interval.references_into(out);
        }
        if let Some(aspect) = &self.spatial_aspect {
            aspect.references_into(out);
        }
        for aspect in &self.spatial_aspects {
            aspect.references_into(out);
        }
        for recurrence in &self.recurrence {
            recurrence.references_into(out);
        }
        for modifier in &self.interval_modifiers {
            modifier.references_into(out);
        }
        for recurrence in &self.spatial_recurrence {
            recurrence.references_into(out);
        }
        for modifier in &self.spatial_interval_modifiers {
            modifier.references_into(out);
        }
        if let Some(descriptor) = &self.descriptor {
            descriptor.references_into(out);
        }
        if let Some(composition) = &self.composition {
            out.extend(composition.members.iter().copied());
            out.extend(composition.excluded_members.iter().copied());
            extend_optional(out, composition.operator_parameter);
        }
        out.extend(self.relative_clauses.iter().map(|clause| clause.body));
        for argument in self.arguments.values() {
            argument.references_into(out);
        }
        for question in &self.place_questions {
            question.references_into(out);
        }
        for argument in &self.modal_arguments {
            argument.references_into(out);
        }
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
        for exchange in &self.reciprocity {
            exchange.references_into(out);
        }
        extend_optional(out, self.relation_parameter);
        if let Some(tanru_link) = &self.tanru_link {
            tanru_link.references_into(out);
        }
        extend_optional(out, self.relation_metadata);
        extend_optional(out, self.operator_parameter);
        extend_optional(out, self.operator_denotes);
        if let Some(expansion) = &self.expansion {
            expansion.references_into(out);
        }
        if let Some(connector) = &self.connector {
            connector.references_into(out);
        }
        extend_optional(out, self.predication);
        out.extend(self.children.iter().copied());
        extend_optional(out, self.variable);
        extend_optional(out, self.source_variable);
        if let Some(selection_source) = &self.selection_source {
            selection_source.references_into(out);
        }
        extend_optional(out, self.restriction);
        extend_optional(out, self.body);
        extend_optional(out, self.quantity);
        extend_optional(out, self.scale);
        for binding in &self.bindings {
            binding.references_into(out);
        }
        for stream in &self.streams {
            stream.references_into(out);
        }
        extend_optional(out, self.abstracted);
        out.extend(self.parameters.iter().copied());
        out.extend(self.embedded_questions.iter().copied());
        if let Some(quotation) = &self.quotation {
            quotation.references_into(out);
        }
        extend_optional(out, self.denotes);
        extend_optional(out, self.experiencer);
        extend_optional(out, self.target);
        extend_optional(out, self.anchor);
        out.extend(self.operands.iter().copied());
        if let Some(value) = &self.value {
            value.references_into(out);
        }
        extend_optional(out, self.comparison_set);
        out.extend(self.slots.iter().map(|slot| slot.parameter));
        extend_optional(out, self.focus);
        extend_optional(out, self.presupposed_answer);
        if let Some(subscript) = &self.subscript {
            subscript.references_into(out);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn push_diagnostic(&mut self, diagnostic: SemanticDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(true)]
    pub fn set_descriptor_quantity(&mut self, quantity: SemanticObjectId) {
        if let Some(descriptor) = self.descriptor.take() {
            self.descriptor = Some(descriptor.with_data(data! {
                quantity: Some(quantity),
            }));
        }
    }

    #[requires(!relative_clauses.is_empty())]
    #[requires(self.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(!self.relative_clauses.is_empty())]
    pub fn extend_relative_clauses(&mut self, relative_clauses: Vec<RelativeClause>) {
        self.relative_clauses.extend(relative_clauses);
    }

    #[requires(!assigned_name.name.is_empty())]
    #[requires(self.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(!self.assigned_names.is_empty())]
    pub fn push_assigned_name(&mut self, assigned_name: AssignedName) {
        self.assigned_names.push(assigned_name);
    }

    #[requires(subscript.value.object_kind() == SemanticObjectKind::MathExpression)]
    #[ensures(self.subscript.is_some())]
    pub fn set_subscript(&mut self, subscript: Subscript) {
        self.subscript = Some(subscript);
    }

    #[requires(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
    #[ensures(ret.source_variable == source_variable)]
    pub fn with_quantifier_selection(
        self,
        source_variable: Option<SemanticObjectId>,
        selection_source: Option<SelectionSource>,
    ) -> Self {
        Self {
            source_variable,
            selection_source,
            ..self
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn extend_optional(out: &mut Vec<SemanticObjectId>, value: Option<SemanticObjectId>) {
    if let Some(value) = value {
        out.push(value);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSource {
    pub span: SourceByteSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construct: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceByteSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

impl SourceByteSpan {
    #[requires(span.byte_start <= span.byte_end)]
    #[ensures(ret.byte_start == span.byte_start)]
    pub fn from_source_span(span: &SourceSpan) -> Self {
        Self {
            byte_start: span.byte_start,
            byte_end: span.byte_end,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

impl SemanticDiagnostic {
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    pub fn warning(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            severity: DiagnosticSeverity::Warning,
            message,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UtteranceForce {
    Assert,
    Ask,
    Command,
    Mention,
    Quote,
    Parenthetical,
    Subordinated,
    Vocative,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceRelation {
    SameTopicContinuation,
}

#[invariant(!operator.is_empty(), "nonlogical sequence operator must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonlogicalConnection {
    pub operator: String,
    pub connector: Connector,
}

impl NonlogicalConnection {
    #[requires(!operator.is_empty())]
    #[ensures(ret.operator == old(operator.clone()))]
    pub fn new(operator: String, connector: Connector) -> Self {
        Self::from_data(data!(NonlogicalConnection {
            operator,
            connector,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.connector.references_into(out);
    }
}

#[invariant(value.object_kind() == SemanticObjectKind::MathExpression, "ordinal label value must be a math expression")]
#[invariant(target.is_none_or(|target| {
    matches!(
        target.object_kind(),
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Formula
            | SemanticObjectKind::Referent
            | SemanticObjectKind::DisplayedContent
    )
}), "ordinal labels target discourse-visible objects")]
#[invariant(!introduced_by.is_empty(), "ordinal label source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrdinalLabel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SemanticObjectId>,
    pub level: OrdinalLabelLevel,
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl OrdinalLabel {
    #[requires(value.object_kind() == SemanticObjectKind::MathExpression)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        target: Option<SemanticObjectId>,
        level: OrdinalLabelLevel,
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(OrdinalLabel {
            target,
            level,
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.target);
        out.push(self.value);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OrdinalLabelLevel {
    Item,
    Division,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventualityClass {
    Locution,
    Event,
    State,
    Process,
    Activity,
    Achievement,
}

impl EventualityClass {
    #[requires(true)]
    #[ensures(ret.is_subsort_of(SemanticSort::eventuality()))]
    pub fn sort(self) -> SemanticSort {
        SemanticSort::Eventuality(match self {
            Self::Locution => EventualitySort::Locution,
            Self::Event => EventualitySort::General,
            Self::State => EventualitySort::State,
            Self::Process => EventualitySort::Process,
            Self::Activity => EventualitySort::Activity,
            Self::Achievement => EventualitySort::Achievement,
        })
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Actuality {
    pub kind: ActualityKind,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActualityKind {
    Actual,
    Capable,
    Potential,
    Demonstrated,
}

#[invariant(!relation.is_empty(), "anchor relation must be named")]
#[invariant(argument_object_kind_can_fill(anchor.object_kind()), "anchor must be referent-like")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "anchor relation distance must be named when present")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRelation {
    pub relation: String,
    pub anchor: SemanticObjectId,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub sticky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<AnchorMagnitude>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<SpatialMotion>,
}

impl AnchorRelation {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.anchor);
        if let Some(magnitude) = &self.magnitude {
            magnitude.references_into(out);
        }
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(argument_object_kind_can_fill(value.object_kind()), "anchor magnitude value must be referent-like")]
#[invariant(!introduced_by.is_empty(), "anchor magnitude source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorMagnitude {
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl AnchorMagnitude {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(AnchorMagnitude {
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.value);
    }
}

#[invariant(!introduced_by.is_empty(), "spatial motion source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialMotion {
    pub kind: SpatialMotionKind,
    pub introduced_by: String,
}

impl SpatialMotion {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: SpatialMotionKind, introduced_by: String) -> Self {
        Self::from_data(data!(SpatialMotion {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpatialMotionKind {
    Toward,
}

#[invariant(!relation.is_empty(), "temporal path relation must be named")]
#[invariant(!introduced_by.is_empty(), "temporal path source marker must be named")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "temporal path distance must be named when present")]
#[invariant(anchor.object_id().is_none_or(|id| argument_object_kind_can_fill(id.object_kind())), "temporal path object anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalPathStep {
    pub relation: String,
    pub anchor: TemporalPathAnchor,
    pub introduced_by: String,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub sticky: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<AnchorMagnitude>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<SpatialMotion>,
}

impl TemporalPathStep {
    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
    #[requires(anchor.object_id().is_none_or(|id| argument_object_kind_can_fill(id.object_kind())))]
    #[ensures(ret.relation == old(relation.clone()))]
    pub fn new(
        relation: String,
        anchor: TemporalPathAnchor,
        introduced_by: String,
        distance: Option<String>,
        magnitude: Option<AnchorMagnitude>,
        scalar_negation: Option<ScalarNegation>,
        motion: Option<SpatialMotion>,
    ) -> Self {
        Self::from_data(data!(TemporalPathStep {
            relation,
            anchor,
            introduced_by,
            sticky: false,
            inherited: None,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        if let Some(anchor) = self.anchor.object_id() {
            out.push(anchor);
        }
        if let Some(magnitude) = &self.magnitude {
            magnitude.references_into(out);
        }
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant((*kind == TemporalPathAnchorKind::Object) == value.is_some(), "object anchors carry a value and non-object anchors do not")]
#[invariant(value.is_none_or(|value| argument_object_kind_can_fill(value.object_kind())), "temporal path object anchor must be referent-like")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemporalPathAnchor {
    pub kind: TemporalPathAnchorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<SemanticObjectId>,
}

impl TemporalPathAnchor {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[ensures(ret.object_id() == Some(value))]
    pub fn object(value: SemanticObjectId) -> Self {
        Self::from_data(data!(TemporalPathAnchor {
            kind: TemporalPathAnchorKind::Object,
            value: Some(value),
        }))
    }

    #[requires(true)]
    #[ensures(ret.object_id().is_none())]
    pub fn previous() -> Self {
        Self::from_data(data!(TemporalPathAnchor {
            kind: TemporalPathAnchorKind::Previous,
            value: None,
        }))
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| argument_object_kind_can_fill(id.object_kind())))]
    pub fn object_id(&self) -> Option<SemanticObjectId> {
        self.value
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TemporalPathAnchorKind {
    Object,
    Previous,
}

#[invariant(!extent.is_empty(), "time interval extent must be named")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "time interval anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeInterval {
    pub extent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
}

impl TimeInterval {
    #[requires(!extent.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.extent == old(extent.clone()))]
    pub fn new(extent: String, anchor: Option<SemanticObjectId>) -> Self {
        Self::from_data(data!(TimeInterval { extent, anchor }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(!introduced_by.is_empty(), "time span introducer must be recorded")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSpan {
    pub start: TimeSpanEndpoint,
    pub end: TimeSpanEndpoint,
    pub introduced_by: String,
}

impl TimeSpan {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(start: TimeSpanEndpoint, end: TimeSpanEndpoint, introduced_by: String) -> Self {
        Self::from_data(data!(TimeSpan {
            start,
            end,
            introduced_by,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.start.references_into(out);
        self.end.references_into(out);
    }
}

#[invariant(!relation.is_empty(), "time span endpoint relation must be named")]
#[invariant(!introduced_by.is_empty(), "time span endpoint introducer must be recorded")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "time span endpoint anchor must be referent-like")]
#[invariant(distance.as_ref().is_none_or(|distance| !distance.is_empty()), "time span endpoint distance must be named when present")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSpanEndpoint {
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
}

impl TimeSpanEndpoint {
    #[requires(!relation.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[requires(!introduced_by.is_empty())]
    #[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
    #[ensures(ret.relation == old(relation.clone()))]
    pub fn new(
        relation: String,
        anchor: Option<SemanticObjectId>,
        introduced_by: String,
        distance: Option<String>,
        scalar_negation: Option<ScalarNegation>,
    ) -> Self {
        Self::from_data(data!(TimeSpanEndpoint {
            relation,
            anchor,
            introduced_by,
            distance,
            scalar_negation,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(extent.as_ref().is_none_or(|extent| !extent.is_empty()), "space interval extent must be named when present")]
#[invariant(directions.iter().all(|direction| !direction.is_empty()), "space interval directions must be named")]
#[invariant(dimensions.iter().all(|dimension| !dimension.is_empty()), "space interval dimensions must be named")]
#[invariant(extent.is_some() || !directions.is_empty() || !dimensions.is_empty(), "space interval must carry at least one spatial attribute")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "space interval anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInterval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dimensions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
}

impl SpaceInterval {
    #[requires(extent.as_ref().is_none_or(|extent| !extent.is_empty()))]
    #[requires(directions.iter().all(|direction| !direction.is_empty()))]
    #[requires(dimensions.iter().all(|dimension| !dimension.is_empty()))]
    #[requires(extent.is_some() || !directions.is_empty() || !dimensions.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.extent == old(extent.clone()))]
    pub fn new(
        extent: Option<String>,
        directions: Vec<String>,
        dimensions: Vec<String>,
        anchor: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(SpaceInterval {
            extent,
            directions,
            dimensions,
            anchor,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
    }
}

#[invariant(!contour.is_empty(), "aspect contour must be named")]
#[invariant(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())), "aspect anchor must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aspect {
    pub contour: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
}

impl Aspect {
    #[requires(!contour.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.contour == old(contour.clone()))]
    pub fn new(contour: String, anchor: Option<SemanticObjectId>) -> Self {
        Self::new_with_polarity(contour, anchor, None)
    }

    #[requires(!contour.is_empty())]
    #[requires(anchor.is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(ret.contour == old(contour.clone()))]
    pub fn new_with_polarity(
        contour: String,
        anchor: Option<SemanticObjectId>,
        scalar_negation: Option<ScalarNegation>,
    ) -> Self {
        Self::from_data(data!(Aspect {
            contour,
            anchor,
            scalar_negation,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.anchor);
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(!introduced_by.is_empty(), "recurrence marker must be named")]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity), "recurrence quantity must be a quantity object")]
#[invariant(interval.is_none_or(|interval| argument_object_kind_can_fill(interval.object_kind())), "recurrence interval must be referent-like")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    pub kind: RecurrenceKind,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<RecurrenceConnection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<QuantityValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negation: Option<ModalNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl Recurrence {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(
        kind: RecurrenceKind,
        introduced_by: String,
        connection: Option<RecurrenceConnection>,
        value: Option<QuantityValue>,
        interval: Option<SemanticObjectId>,
        negation: Option<ModalNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Recurrence {
            kind,
            introduced_by,
            connection,
            quantity: None,
            value,
            interval,
            negation,
            source,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(ret.quantity == Some(quantity))]
    pub fn new_with_quantity(
        kind: RecurrenceKind,
        introduced_by: String,
        connection: Option<RecurrenceConnection>,
        quantity: SemanticObjectId,
        interval: Option<SemanticObjectId>,
        negation: Option<ModalNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Recurrence {
            kind,
            introduced_by,
            connection,
            quantity: Some(quantity),
            value: None,
            interval,
            negation,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.quantity);
        if let Some(value) = &self.value {
            value.references_into(out);
        }
        extend_optional(out, self.interval);
    }
}

#[invariant(::Aspect(aspect) => !aspect.contour.is_empty(), "aspect interval modifier must carry a named contour")]
#[invariant(::Recurrence(recurrence) => !recurrence.introduced_by.is_empty(), "recurrence interval modifier must carry its source marker")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum IntervalModifier {
    Aspect(Aspect),
    Recurrence(Recurrence),
}

impl IntervalModifier {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        match self.as_data() {
            data!(IntervalModifier::Aspect(aspect)) => aspect.references_into(out),
            data!(IntervalModifier::Recurrence(recurrence)) => recurrence.references_into(out),
        }
    }
}

#[invariant(!introduced_by.is_empty(), "recurrence connection source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceConnection {
    pub kind: RecurrenceConnectionKind,
    pub introduced_by: String,
}

impl RecurrenceConnection {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: RecurrenceConnectionKind, introduced_by: String) -> Self {
        Self::from_data(data!(RecurrenceConnection {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceConnectionKind {
    Product,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecurrenceKind {
    OccurrenceCount,
    OrdinalOccurrence,
    Regular,
    Typically,
    Continuously,
    Habitually,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeicticGround {
    pub time: SemanticObjectId,
    pub place: SemanticObjectId,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferentCategory {
    Constant,
    Variable,
    Indexical,
    Composite,
}

#[invariant(::Eventuality(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticSort {
    Entity,
    Mass,
    Set,
    Sequence,
    Time,
    Eventuality(EventualitySort),
    Predication,
    TruthValue,
    Proposition,
    Concept,
    Amount,
    Quantity,
    Number,
    Scale,
    Text,
    Sign,
    Relation,
    Place,
    Connective,
    TenseModal,
    MathOperator,
    ArgumentBundle,
    AbstractNature,
}

impl SemanticSort {
    #[requires(true)]
    #[ensures(ret == SemanticSort::Eventuality(EventualitySort::General))]
    pub fn eventuality() -> Self {
        Self::Eventuality(EventualitySort::General)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Mass => "mass",
            Self::Set => "set",
            Self::Sequence => "sequence",
            Self::Time => "time",
            Self::Eventuality(sort) => sort.label(),
            Self::Predication => "predication",
            Self::TruthValue => "truthValue",
            Self::Proposition => "proposition",
            Self::Concept => "concept",
            Self::Amount => "amount",
            Self::Quantity => "quantity",
            Self::Number => "number",
            Self::Scale => "scale",
            Self::Text => "text",
            Self::Sign => "sign",
            Self::Relation => "relation",
            Self::Place => "place",
            Self::Connective => "connective",
            Self::TenseModal => "tenseModal",
            Self::MathOperator => "mathOperator",
            Self::ArgumentBundle => "argumentBundle",
            Self::AbstractNature => "abstractNature",
        }
    }

    #[requires(true)]
    #[ensures(ret || self != required)]
    pub fn is_subsort_of(self, required: Self) -> bool {
        self == required
            || matches!(
                (self, required),
                (
                    Self::Eventuality(_),
                    Self::Eventuality(EventualitySort::General)
                )
            )
    }
}

impl Serialize for SemanticSort {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

impl fmt::Display for SemanticSort {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventualitySort {
    General,
    State,
    Process,
    Activity,
    Achievement,
    Experience,
    Locution,
}

impl EventualitySort {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "eventuality",
            Self::State => "eventuality/state",
            Self::Process => "eventuality/process",
            Self::Activity => "eventuality/activity",
            Self::Achievement => "eventuality/achievement",
            Self::Experience => "eventuality/experience",
            Self::Locution => "eventuality/locution",
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexicalKind {
    Speaker,
    Audience,
    Now,
    Here,
    ProximalDemonstrative,
    MedialDemonstrative,
    DistalDemonstrative,
}

#[invariant(!kind.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub kind: String,
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veridical: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definiteness: Option<DescriptorDefiniteness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<SemanticObjectId>,
}

impl Descriptor {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.speaker);
        extend_optional(out, self.body);
        out.extend(self.relative_clauses.iter().map(|clause| clause.body));
        extend_optional(out, self.quantity);
        extend_optional(out, self.scale);
        extend_optional(out, self.operand);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DescriptorDefiniteness {
    AffirmedPoint,
    IndefiniteAlternative,
    NeutralPoint,
    UniqueExtreme,
}

#[invariant(!operator.is_empty(), "composition operator must be named")]
#[invariant(members.iter().all(|member| argument_object_kind_can_fill(member.object_kind())), "composition members must be semantic objects that can fill an argument")]
#[invariant(excluded_members.iter().all(|member| argument_object_kind_can_fill(member.object_kind())), "excluded composition members must be semantic objects that can fill an argument")]
#[invariant(endpoint_inclusion.is_none() || operator.ends_with("Interval"), "endpoint inclusion only applies to interval compositions")]
#[invariant(*complement != Some(true) || operator.ends_with("Interval"), "composition complements are interval complements")]
#[invariant((operator == "connectiveQuestion") == operator_parameter.is_some(), "connective-question compositions must carry exactly one operator parameter")]
#[invariant(operator_parameter.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter), "composition operator parameter must be a parameter object")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    pub operator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_parameter: Option<SemanticObjectId>,
    pub members: Vec<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_members: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_inclusion: Option<IntervalEndpointInclusion>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntervalEndpointInclusion {
    pub left: EndpointInclusion,
    pub right: EndpointInclusion,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointInclusion {
    Inclusive,
    Exclusive,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterRole {
    PropertySlot,
    RelativeClauseHead,
    ArgumentQuestion,
    RelationQuestion,
    RelationVariable,
    UnspecifiedRelation,
    PlaceQuestion,
    ConnectiveQuestion,
    TenseQuestion,
    MathOperatorQuestion,
    AttitudeQuestion,
    RespectiveSlot,
}

#[invariant(variable.object_kind() == SemanticObjectKind::Referent)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionSource {
    pub kind: SelectionSourceKind,
    pub variable: SemanticObjectId,
}

impl SelectionSource {
    #[requires(variable.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.variable == variable)]
    pub fn witness_set(variable: SemanticObjectId) -> Self {
        Self::from_data(data!(SelectionSource {
            kind: SelectionSourceKind::WitnessSet,
            variable,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.variable);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionSourceKind {
    WitnessSet,
}

#[invariant(argument_value_shape_is_valid(*kind, *value, introduced_by.as_deref()))]
#[invariant(*kind != ArgumentValueKind::Deleted || relative_clauses.is_empty())]
#[invariant(*kind != ArgumentValueKind::Deleted || quantity.is_none())]
#[invariant(*kind != ArgumentValueKind::Deleted || command_target.is_none())]
#[invariant((*quantity).is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentValue {
    pub kind: ArgumentValueKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_target: Option<CommandTarget>,
}

impl ArgumentValue {
    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[ensures(true)]
    pub fn filled(value: SemanticObjectId, source: Option<SemanticSource>) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Filled,
            value: Some(value),
            quantity: None,
            introduced_by: None,
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(argument_object_kind_can_fill(value.object_kind()))]
    #[requires(!introduced_by.is_empty())]
    #[ensures(true)]
    pub fn elided(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Elided,
            value: Some(value),
            quantity: None,
            introduced_by: Some(introduced_by),
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(true)]
    pub fn deleted(introduced_by: String, source: Option<SemanticSource>) -> Self {
        Self::from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: None,
            quantity: None,
            introduced_by: Some(introduced_by),
            source,
            relative_clauses: Vec::new(),
            command_target: None,
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[requires(!relative_clauses.is_empty())]
    #[ensures(!ret.relative_clauses.is_empty())]
    pub fn with_relative_clauses(self, relative_clauses: Vec<RelativeClause>) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            relative_clauses,
            ..data
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(ret.quantity == Some(quantity))]
    pub fn with_quantity(self, quantity: SemanticObjectId) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            quantity: Some(quantity),
            ..data
        }))
    }

    #[requires(self.kind != ArgumentValueKind::Deleted)]
    #[ensures(ret.command_target.is_some())]
    pub fn with_command_target(self, command_target: CommandTarget) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ArgumentValue {
            command_target: Some(command_target),
            ..data
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        if let Some(value) = self.value {
            out.push(value);
        }
        extend_optional(out, self.quantity);
        out.extend(self.relative_clauses.iter().map(|clause| clause.body));
    }
}

#[invariant(!introduced_by.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandTarget {
    pub introduced_by: String,
}

impl CommandTarget {
    #[requires(!introduced_by.is_empty())]
    #[ensures(!ret.introduced_by.is_empty())]
    pub fn new(introduced_by: String) -> Self {
        Self::from_data(data!(CommandTarget { introduced_by }))
    }
}

#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(introduced_by.as_ref().is_none_or(|introduced_by| !introduced_by.is_empty()))]
#[invariant(!matches!(*veridical, Some(true)), "true veridicality is omitted")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeClause {
    pub kind: RelativeClauseKind,
    pub body: SemanticObjectId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introduced_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veridical: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl RelativeClause {
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.body == body)]
    pub fn new(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: None,
            veridical: None,
            source
        }))
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.body == body)]
    pub fn with_introducer(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: Some(introduced_by),
            veridical: None,
            source
        }))
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.body == body)]
    pub fn nonveridical(
        kind: RelativeClauseKind,
        body: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(RelativeClause {
            kind,
            body,
            introduced_by: Some(introduced_by),
            veridical: Some(false),
            source
        }))
    }
}

#[invariant(!name.is_empty(), "assigned names must preserve the assigned cmevla")]
#[invariant(!word.is_empty(), "assigned names must record the naming word")]
#[invariant(!introduced_by.is_empty(), "assigned names must record the assignment marker")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignedName {
    pub name: String,
    pub word: String,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RelativeClauseKind {
    Incidental,
    Restrictive,
}

#[invariant(parameter.object_kind() == SemanticObjectKind::Parameter)]
#[invariant(!candidate_places.is_empty(), "place questions must enumerate candidate places")]
#[invariant(candidate_places.iter().all(|place| place.get() > 0))]
#[invariant(candidate_places.iter().enumerate().all(|(index, place)| !candidate_places[..index].contains(place)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceQuestionBinding {
    pub parameter: SemanticObjectId,
    pub argument: ArgumentValue,
    pub candidate_places: Vec<PlaceIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl PlaceQuestionBinding {
    #[requires(parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!candidate_places.is_empty())]
    #[requires(candidate_places.iter().all(|place| place.get() > 0))]
    #[ensures(ret.parameter == parameter)]
    pub fn new(
        parameter: SemanticObjectId,
        argument: ArgumentValue,
        candidate_places: Vec<PlaceIndex>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(PlaceQuestionBinding {
            parameter,
            argument,
            candidate_places,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.parameter);
        self.argument.references_into(out);
    }
}

#[invariant(!introduced_by.is_empty(), "modal negation source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalNegation {
    pub kind: ModalNegationKind,
    pub introduced_by: String,
}

impl ModalNegation {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: ModalNegationKind, introduced_by: String) -> Self {
        Self::from_data(data!(ModalNegation {
            kind,
            introduced_by,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModalNegationKind {
    Contradictory,
    OtherThan,
}

#[invariant(!introduced_by.is_empty(), "modal source marker must be named")]
#[invariant(relation.as_ref().is_none_or(|relation| !relation.is_empty()), "modal relation must be named when present")]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula), "modal body must be a formula")]
#[invariant(relation.is_some() != body.is_some(), "modal argument must use either relation arguments or a body formula")]
#[invariant(body.is_some() || !arguments.is_empty(), "modal relation must have at least one explicit place")]
#[invariant(body.is_none() || arguments.is_empty(), "modal body arguments are represented inside the body formula")]
#[invariant(arguments.keys().all(|place| place.get() > 0))]
#[invariant(component.is_none_or(|component| argument_object_kind_can_fill(component.object_kind())))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalArgument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    pub introduced_by: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<PlaceIndex, ArgumentValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negation: Option<ModalNegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scalar_negation: Option<ScalarNegation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<DisplayedContentModifier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl ModalArgument {
    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(true)]
    pub fn new(
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::new_with_polarity(relation, introduced_by, arguments, None, None, source)
    }

    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(true)]
    pub fn new_with_polarity(
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        negation: Option<ModalNegation>,
        scalar_negation: Option<ScalarNegation>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ModalArgument {
            relation: Some(relation),
            introduced_by,
            arguments,
            body: None,
            component: None,
            negation,
            scalar_negation,
            modifiers: Vec::new(),
            source,
        }))
    }

    #[requires(!introduced_by.is_empty())]
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.body == Some(body))]
    pub fn body(
        introduced_by: String,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ModalArgument {
            relation: None,
            introduced_by,
            arguments: BTreeMap::new(),
            body: Some(body),
            component: None,
            negation: None,
            scalar_negation: None,
            modifiers: Vec::new(),
            source,
        }))
    }

    #[requires(argument_object_kind_can_fill(component.object_kind()))]
    #[ensures(ret.component == Some(component))]
    pub fn with_component(self, component: SemanticObjectId) -> Self {
        let data = self.into_data();
        Self::from_data(data!(ModalArgument {
            component: Some(component),
            ..data
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        for argument in self.arguments.values() {
            argument.references_into(out);
        }
        extend_optional(out, self.body);
        extend_optional(out, self.component);
        if let Some(scalar_negation) = &self.scalar_negation {
            scalar_negation.references_into(out);
        }
    }
}

#[invariant(!introduced_by.is_empty(), "reciprocity source marker must be named")]
#[invariant(left.kind != ArgumentValueKind::Deleted, "reciprocity participants must exist")]
#[invariant(right.kind != ArgumentValueKind::Deleted, "reciprocity participants must exist")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReciprocalExchange {
    pub left: ArgumentValue,
    pub right: ArgumentValue,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl ReciprocalExchange {
    #[requires(!introduced_by.is_empty())]
    #[requires(left.kind != ArgumentValueKind::Deleted)]
    #[requires(right.kind != ArgumentValueKind::Deleted)]
    #[ensures(true)]
    pub fn new(
        left: ArgumentValue,
        right: ArgumentValue,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ReciprocalExchange {
            left,
            right,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        self.left.references_into(out);
        self.right.references_into(out);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArgumentValueKind {
    Filled,
    Elided,
    Deleted,
}

#[requires(true)]
#[ensures(true)]
pub fn argument_object_kind_can_fill(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Referent | SemanticObjectKind::Parameter | SemanticObjectKind::Formula
    )
}

#[requires(true)]
#[ensures(true)]
pub fn displayed_content_target_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Formula
            | SemanticObjectKind::Question
            | SemanticObjectKind::Parameter
            | SemanticObjectKind::Referent
            | SemanticObjectKind::DisplayedContent
    )
}

#[requires(true)]
#[ensures(true)]
fn argument_value_shape_is_valid(
    kind: ArgumentValueKind,
    value: Option<SemanticObjectId>,
    introduced_by: Option<&str>,
) -> bool {
    let value_allowed =
        value.is_none_or(|value| argument_object_kind_can_fill(value.object_kind()));
    match kind {
        ArgumentValueKind::Filled => value_allowed && value.is_some() && introduced_by.is_none(),
        ArgumentValueKind::Elided => {
            value_allowed
                && value.is_some()
                && introduced_by.is_some_and(|introduced_by| !introduced_by.is_empty())
        }
        ArgumentValueKind::Deleted => {
            value.is_none() && introduced_by.is_some_and(|introduced_by| !introduced_by.is_empty())
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PredicationMode {
    Asserted,
    Definitional,
    Restrictive,
    Incidental,
    Displayed,
    Inert,
    Performative,
}

#[invariant(!introduced_by.is_empty(), "scalar negation source marker must be named")]
#[invariant(scale.is_none_or(|scale| scale.object_kind() == SemanticObjectKind::Referent), "scalar negation scale must be a referent")]
#[invariant(argument_scope.iter().all(|place| place.get() > 0), "scalar negation argument scope must use numbered argument places")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalarNegation {
    pub kind: ScalarNegationKind,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argument_scope: Vec<PlaceIndex>,
}

impl ScalarNegation {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: ScalarNegationKind, introduced_by: String) -> Self {
        Self::from_data(data!(ScalarNegation {
            kind,
            introduced_by,
            scale: None,
            argument_scope: Vec::new(),
        }))
    }

    #[requires(scale.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.scale == Some(scale))]
    pub fn with_scale(self, scale: SemanticObjectId) -> Self {
        self.with_data(data! { scale: Some(scale) })
    }

    #[requires(argument_scope.iter().all(|place| place.get() > 0))]
    #[ensures(ret.argument_scope == argument_scope)]
    pub fn with_argument_scope(self, argument_scope: Vec<PlaceIndex>) -> Self {
        self.with_data(data! { argument_scope: argument_scope.clone() })
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.scale);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScalarNegationKind {
    OtherThan,
    Opposite,
    Neutral,
    Affirmed,
}

#[invariant(::Formula(_) => true)]
#[invariant(::Math(operator) => !operator.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticOperator {
    Formula(FormulaOperator),
    Math(String),
}

impl SemanticOperator {
    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(SemanticOperator::Formula(_))))]
    fn formula(operator: FormulaOperator) -> Self {
        Self::from_data(data!(SemanticOperator::Formula(operator)))
    }

    #[requires(!operator.is_empty())]
    #[ensures(matches!(ret.as_data(), data!(SemanticOperator::Math(_))))]
    fn math(operator: String) -> Self {
        Self::from_data(data!(SemanticOperator::Math(operator)))
    }
}

impl Serialize for SemanticOperator {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.as_data() {
            data!(SemanticOperator::Formula(operator)) => operator.serialize(serializer),
            data!(SemanticOperator::Math(operator)) => serializer.serialize_str(operator),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FormulaOperator {
    Atom,
    Affirmed,
    Not,
    Scoped,
    And,
    Or,
    Implies,
    Iff,
    ExclusiveOr,
    WhetherOrNot,
    ConnectiveQuestion,
    Exists,
    Forall,
    None,
    Cardinality,
    PluralExists,
    PluralForall,
    QuantifierBundle,
    RespectivelyDistribution,
}

#[invariant(quantifier_formula_operator_is_allowed(*operator))]
#[invariant(quantifier_variable_kind_is_allowed(variable.object_kind()))]
#[invariant(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantifierBinding {
    pub operator: FormulaOperator,
    pub variable: SemanticObjectId,
    #[serde(rename = "sourceVariable", skip_serializing_if = "Option::is_none")]
    pub source_variable: Option<SemanticObjectId>,
    #[serde(rename = "selectionSource", skip_serializing_if = "Option::is_none")]
    pub selection_source: Option<SelectionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl QuantifierBinding {
    #[requires(quantifier_formula_operator_is_allowed(operator))]
    #[requires(quantifier_variable_kind_is_allowed(variable.object_kind()))]
    #[requires(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
    #[requires(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
    #[ensures(ret.variable == variable)]
    pub fn new(
        operator: FormulaOperator,
        variable: SemanticObjectId,
        source_variable: Option<SemanticObjectId>,
        selection_source: Option<SelectionSource>,
        restriction: Option<SemanticObjectId>,
        quantity: Option<SemanticObjectId>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(QuantifierBinding {
            operator,
            variable,
            source_variable,
            selection_source,
            restriction,
            quantity,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.variable);
        extend_optional(out, self.source_variable);
        if let Some(selection_source) = &self.selection_source {
            selection_source.references_into(out);
        }
        extend_optional(out, self.restriction);
        extend_optional(out, self.quantity);
    }
}

#[invariant(!source.is_empty())]
#[invariant(!locus.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    pub source: String,
    pub locus: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth_table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<SemanticObjectId>,
}

impl Connector {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.parameter);
    }
}

#[invariant(head.object_kind() == SemanticObjectKind::Predication)]
#[invariant(argument_object_kind_can_fill(modifier.object_kind()), "tanru modifier must be a semantic argument value")]
#[invariant(!relation_label.is_empty(), "tanru relation label must be displayable")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TanruLink {
    pub head: SemanticObjectId,
    pub modifier: SemanticObjectId,
    pub relation_label: String,
}

impl TanruLink {
    #[requires(head.object_kind() == SemanticObjectKind::Predication)]
    #[requires(argument_object_kind_can_fill(modifier.object_kind()))]
    #[requires(!relation_label.is_empty())]
    #[ensures(ret.head == head)]
    pub fn new(head: SemanticObjectId, modifier: SemanticObjectId, relation_label: String) -> Self {
        Self::from_data(data!(TanruLink {
            head,
            modifier,
            relation_label,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.head);
        out.push(self.modifier);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AbstractionKind {
    Event,
    Achievement,
    Process,
    Activity,
    State,
    Property,
    Amount,
    TruthValue,
    Proposition,
    SentenceSign,
    Concept,
    Experience,
    Unspecified,
}

impl AbstractionKind {
    #[requires(true)]
    #[ensures(true)]
    pub fn output_sort(self) -> SemanticSort {
        match self {
            Self::Event => SemanticSort::eventuality(),
            Self::Achievement => SemanticSort::Eventuality(EventualitySort::Achievement),
            Self::Process => SemanticSort::Eventuality(EventualitySort::Process),
            Self::Activity => SemanticSort::Eventuality(EventualitySort::Activity),
            Self::State => SemanticSort::Eventuality(EventualitySort::State),
            Self::Experience => SemanticSort::Eventuality(EventualitySort::Experience),
            Self::Property => SemanticSort::Relation,
            Self::Amount => SemanticSort::Amount,
            Self::TruthValue => SemanticSort::TruthValue,
            Self::Proposition => SemanticSort::Proposition,
            Self::SentenceSign => SemanticSort::Sign,
            Self::Concept => SemanticSort::Concept,
            Self::Unspecified => SemanticSort::AbstractNature,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignKind {
    Quotation,
    Letteral,
    MathExpression,
    Connective,
    Word,
    Text,
}

#[invariant(!source_words.is_empty(), "letteral units must preserve their source words")]
#[invariant(text.as_ref().is_none_or(|text| !text.is_empty()), "letteral unit display text must not be empty when present")]
#[invariant(value.as_ref().is_none_or(|value| !value.is_empty()), "letteral unit value must not be empty when present")]
#[invariant(modifier.as_ref().is_none_or(|modifier| !modifier.is_empty()), "letteral modifier must not be empty when present")]
#[invariant(bu_depth.is_none_or(|depth| depth > 0), "BU depth is recorded only when positive")]
#[invariant((*kind == LetteralUnitKind::Compound) == !parts.is_empty(), "only compound letterals have parts")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LetteralUnit {
    pub kind: LetteralUnitKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_words: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bu_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<LetteralUnit>,
}

impl LetteralUnit {
    #[requires(!source_words.is_empty())]
    #[requires(text.as_ref().is_none_or(|text| !text.is_empty()))]
    #[requires(value.as_ref().is_none_or(|value| !value.is_empty()))]
    #[requires(modifier.as_ref().is_none_or(|modifier| !modifier.is_empty()))]
    #[requires(bu_depth.is_none_or(|depth| depth > 0))]
    #[ensures(ret.kind == old(kind))]
    pub fn simple(
        kind: LetteralUnitKind,
        source_words: Vec<String>,
        text: Option<String>,
        value: Option<String>,
        modifier: Option<String>,
        bu_depth: Option<usize>,
    ) -> Self {
        Self::from_data(data!(LetteralUnit {
            kind,
            source_words,
            text,
            value,
            modifier,
            bu_depth,
            parts: Vec::new(),
        }))
    }

    #[requires(!source_words.is_empty())]
    #[requires(!parts.is_empty())]
    #[requires(value.as_ref().is_none_or(|value| !value.is_empty()))]
    #[ensures(ret.kind == LetteralUnitKind::Compound)]
    pub fn compound(
        source_words: Vec<String>,
        value: Option<String>,
        parts: Vec<LetteralUnit>,
    ) -> Self {
        Self::from_data(data!(LetteralUnit {
            kind: LetteralUnitKind::Compound,
            source_words,
            text: None,
            value,
            modifier: None,
            bu_depth: None,
            parts,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LetteralUnitKind {
    Glyph,
    Digit,
    Shift,
    CharacterCode,
    Compound,
}

#[invariant(!mode.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quotation {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utterance: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Quotation {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.utterance);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentFamily {
    Emotion,
    AttitudeModifier,
    PropositionalAttitude,
    Evidential,
    Discursive,
    Metalinguistic,
    Emphasis,
    QuestionPrompt,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentPolarity {
    Positive,
    Neutral,
    Negative,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentTargetFocus {
    Bridi,
    Selbri,
}

#[invariant(!relation.is_empty(), "displayed-content modifier relation must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayedContentModifier {
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<DisplayedContentFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polarity: Option<DisplayedContentPolarity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,
    #[serde(rename = "assertionEffect", skip_serializing_if = "Option::is_none")]
    pub assertion_effect: Option<DisplayedContentAssertionEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayedContentAssertionEffect {
    None,
    HostAsserted,
    HostSubordinated,
    MetalinguisticallyVoided,
    Performative,
}

#[invariant(value.object_kind() == SemanticObjectKind::MathExpression, "subscript value must be a math expression")]
#[invariant(!introduced_by.is_empty(), "subscript source marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscript {
    pub value: SemanticObjectId,
    pub introduced_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl Subscript {
    #[requires(value.object_kind() == SemanticObjectKind::MathExpression)]
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.value == value)]
    pub fn new(
        value: SemanticObjectId,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(Subscript {
            value,
            introduced_by,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.value);
    }
}

#[invariant(!kind.is_empty(), "math literal kind must be named")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MathLiteral {
    pub kind: String,
    pub value: MathLiteralValue,
}

impl MathLiteral {
    #[requires(true)]
    #[ensures(ret.kind == "integer")]
    pub fn integer(value: i64) -> Self {
        Self::from_data(data!(MathLiteral {
            kind: "integer".to_owned(),
            value: MathLiteralValue::from_data(data!(MathLiteralValue::Integer(value))),
        }))
    }

    #[requires(!value.is_empty())]
    #[ensures(ret.kind == old(kind.clone()))]
    pub fn text(kind: String, value: String) -> Self {
        Self::from_data(data!(MathLiteral {
            kind,
            value: MathLiteralValue::from_data(data!(MathLiteralValue::Text(value))),
        }))
    }

    #[requires(components.len() >= 2)]
    #[ensures(ret.kind == "mixedRadix")]
    pub fn mixed_radix(components: Vec<MixedRadixComponent>) -> Self {
        Self::from_data(data!(MathLiteral {
            kind: "mixedRadix".to_owned(),
            value: MathLiteralValue::from_data(data!(MathLiteralValue::MixedRadix(
                MixedRadixLiteral::from_data(data!(MixedRadixLiteral { components }))
            ))),
        }))
    }
}

#[invariant(::Integer(_) => true)]
#[invariant(::Text(value) => !value.is_empty())]
#[invariant(::MixedRadix(value) => value.components.len() >= 2)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MathLiteralValue {
    Integer(i64),
    Text(String),
    MixedRadix(MixedRadixLiteral),
}

#[invariant(components.len() >= 2, "mixed-radix literals need at least two components")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixedRadixLiteral {
    pub components: Vec<MixedRadixComponent>,
}

#[invariant(!text.is_empty(), "mixed-radix component text must be preserved")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MixedRadixComponent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer: Option<i64>,
}

impl MixedRadixComponent {
    #[requires(!text.is_empty())]
    #[ensures(ret.text == old(text.clone()))]
    pub fn new(text: String, integer: Option<i64>) -> Self {
        Self::from_data(data!(MixedRadixComponent { text, integer }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuantityForm {
    Exact,
    All,
    AtLeast,
    AtMost,
    MoreThan,
    LessThan,
    Approximate,
    Indefinite,
    Enough,
    TooMany,
    TooFew,
}

#[invariant((integer.is_some() as usize + text.is_some() as usize + math_expression.is_some() as usize) == 1)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuantityValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub math_expression: Option<SemanticObjectId>,
}

impl QuantityValue {
    #[requires(true)]
    #[ensures(ret.integer == Some(integer))]
    pub fn integer(integer: i64) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: Some(integer),
            text: None,
            math_expression: None,
        }))
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.text.is_some())]
    pub fn text(text: String) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: None,
            text: Some(text),
            math_expression: None,
        }))
    }

    #[requires(math_expression.object_kind() == SemanticObjectKind::MathExpression)]
    #[ensures(ret.math_expression == Some(math_expression))]
    pub fn math_expression(math_expression: SemanticObjectId) -> Self {
        Self::from_data(data!(QuantityValue {
            integer: None,
            text: None,
            math_expression: Some(math_expression),
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.math_expression);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuantityScale {
    Count,
    Fraction,
    Ordinal,
    Amount,
    Extent,
    Frequency,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceDescription {
    pub place: String,
    pub description: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationExpansion {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_words: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rafsi_bindings: Vec<RafsiBinding>,
}

impl RelationExpansion {
    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        for binding in &self.rafsi_bindings {
            binding.references_into(out);
        }
    }
}

#[invariant(!rafsi.is_empty(), "rafsi binding must preserve the source rafsi")]
#[invariant(source_word.as_ref().is_none_or(|word| !word.is_empty()))]
#[invariant(referent.is_none_or(|referent| referent.object_kind() == SemanticObjectKind::Referent))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RafsiBinding {
    pub rafsi: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referent: Option<SemanticObjectId>,
}

impl RafsiBinding {
    #[requires(!rafsi.is_empty())]
    #[requires(source_word.as_ref().is_none_or(|word| !word.is_empty()))]
    #[requires(referent.is_none_or(|referent| referent.object_kind() == SemanticObjectKind::Referent))]
    #[ensures(!ret.rafsi.is_empty())]
    pub fn new(
        rafsi: String,
        source_word: Option<String>,
        referent: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(RafsiBinding {
            rafsi,
            source_word,
            referent,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        extend_optional(out, self.referent);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionKind {
    Truth,
    Argument,
    Relation,
    Place,
    Connective,
    Tense,
    MathOperator,
    Attitude,
    Quantity,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionMode {
    Direct,
    Indirect,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionSlot {
    pub parameter: SemanticObjectId,
    pub role: QuestionSlotRole,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionSlotRole {
    Answer,
    RespectiveSlot,
}

#[invariant(slot.object_kind() == SemanticObjectKind::Parameter)]
#[invariant(!items.is_empty(), "respectively stream cannot be empty")]
#[invariant(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RespectivelyStream {
    pub slot: SemanticObjectId,
    pub items: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
}

impl RespectivelyStream {
    #[requires(slot.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!items.is_empty())]
    #[requires(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
    #[ensures(ret.items == old(items.clone()))]
    pub fn new(slot: SemanticObjectId, items: Vec<SemanticObjectId>) -> Self {
        Self::new_with_details(slot, items, None, None)
    }

    #[requires(slot.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!items.is_empty())]
    #[requires(items.iter().all(|item| argument_object_kind_can_fill(item.object_kind())))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
    #[ensures(ret.slot == old(slot))]
    pub fn new_with_details(
        slot: SemanticObjectId,
        items: Vec<SemanticObjectId>,
        restriction: Option<SemanticObjectId>,
        quantity: Option<SemanticObjectId>,
    ) -> Self {
        Self::from_data(data!(RespectivelyStream {
            slot,
            items,
            restriction,
            quantity,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        out.push(self.slot);
        out.extend(self.items.iter().copied());
        extend_optional(out, self.restriction);
        extend_optional(out, self.quantity);
    }
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_ids_match_types(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    first_semantic_object_id_type_mismatch(objects).is_none()
}

#[requires(true)]
#[ensures(true)]
fn first_semantic_object_id_type_mismatch(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Option<String> {
    let mut seen_indices = BTreeMap::new();
    for (id, object) in objects {
        if let Some(previous) = seen_indices.insert(id.index(), *id) {
            return Some(format!("{id} reuses numeric ID already used by {previous}"));
        }
        if id.object_kind() != object.object_kind() {
            return Some(format!(
                "{id} has ID kind {:?}, but object type is {:?}",
                id.object_kind(),
                object.object_kind()
            ));
        }
        if id.object_kind() == SemanticObjectKind::Referent {
            if id.referent_sort() != object.sort {
                return Some(format!(
                    "{id} has ID sort {:?}, but object sort is {:?}",
                    id.referent_sort(),
                    object.sort
                ));
            }
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_references_are_defined(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    first_undefined_semantic_reference(objects).is_none()
}

#[requires(true)]
#[ensures(true)]
fn first_undefined_semantic_reference(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Option<(SemanticObjectId, SemanticObjectId)> {
    let mut references = Vec::new();
    for (source, object) in objects {
        references.clear();
        object.references_into(&mut references);
        if let Some(missing) = references
            .iter()
            .copied()
            .find(|reference| !objects.contains_key(reference))
        {
            return Some((*source, missing));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_references_match_roles(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects
        .values()
        .all(semantic_object_references_match_roles_for_object)
}

#[requires(true)]
#[ensures(true)]
fn semantic_object_references_match_roles_for_object(object: &SemanticObject) -> bool {
    optional_reference_has_kind(object.speaker, SemanticObjectKind::Referent)
        && optional_reference_has_kind(object.audience, SemanticObjectKind::Referent)
        && optional_eventuality_reference(object.eventuality)
        && content_reference_matches_role(object, object.content)
        && object.deictic_ground.is_none_or(|ground| {
            ground.time.object_kind() == SemanticObjectKind::Referent
                && ground.place.object_kind() == SemanticObjectKind::Referent
        })
        && object.asides.iter().all(|aside| {
            matches!(
                aside.object_kind(),
                SemanticObjectKind::Utterance | SemanticObjectKind::DisplayedContent
            )
        })
        && object
            .items
            .iter()
            .all(|item| sequence_item_kind_is_allowed(item.object_kind()))
        && references_have_kind(&object.connection_claims, SemanticObjectKind::Formula)
        && object
            .nonlogical_connection
            .as_ref()
            .is_none_or(|connection| {
                optional_reference_has_kind(
                    connection.connector.parameter,
                    SemanticObjectKind::Parameter,
                )
            })
        && optional_reference_has_kind(object.tense_modal, SemanticObjectKind::Parameter)
        && object.time_path.iter().all(|step| {
            step.anchor
                .object_id()
                .is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind()))
        })
        && object.space_path.iter().all(|step| {
            step.anchor
                .object_id()
                .is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind()))
        })
        && optional_reference_has_kind(
            object.relation_metadata,
            SemanticObjectKind::RelationMetadata,
        )
        && optional_reference_has_kind(object.relation_parameter, SemanticObjectKind::Parameter)
        && object.tanru_link.as_ref().is_none_or(|tanru_link| {
            tanru_link.head.object_kind() == SemanticObjectKind::Predication
                && argument_object_kind_can_fill(tanru_link.modifier.object_kind())
        })
        && optional_reference_has_kind(object.operator_parameter, SemanticObjectKind::Parameter)
        && optional_reference_has_kind(object.predication, SemanticObjectKind::Predication)
        && references_have_kind(&object.children, SemanticObjectKind::Formula)
        && object.connector.as_ref().is_none_or(|connector| {
            optional_reference_has_kind(connector.parameter, SemanticObjectKind::Parameter)
        })
        && object.operator_denotes.is_none_or(|operator_denotes| {
            argument_object_kind_can_fill(operator_denotes.object_kind())
        })
        && object
            .variable
            .is_none_or(|variable| quantifier_variable_kind_is_allowed(variable.object_kind()))
        && optional_reference_has_kind(object.source_variable, SemanticObjectKind::Referent)
        && object.selection_source.as_ref().is_none_or(|source| {
            source.variable.object_kind() == SemanticObjectKind::Referent
                && object
                    .source_variable
                    .is_none_or(|variable| variable == source.variable)
        })
        && optional_reference_has_kind(object.restriction, SemanticObjectKind::Formula)
        && optional_reference_has_kind(object.body, SemanticObjectKind::Formula)
        && optional_reference_has_kind(object.quantity, SemanticObjectKind::Quantity)
        && optional_reference_has_kind(object.scale, SemanticObjectKind::Referent)
        && object.bindings.iter().all(quantifier_binding_matches_role)
        && quantifier_bundle_shape_matches_role(object)
        && object.ordinal_labels.iter().all(|label| {
            optional_ordinal_label_target_matches_role(label.target)
                && label.value.object_kind() == SemanticObjectKind::MathExpression
        })
        && object.streams.iter().all(|stream| {
            stream.slot.object_kind() == SemanticObjectKind::Parameter
                && stream
                    .items
                    .iter()
                    .all(|item| argument_object_kind_can_fill(item.object_kind()))
                && optional_reference_has_kind(stream.restriction, SemanticObjectKind::Formula)
                && optional_reference_has_kind(stream.quantity, SemanticObjectKind::Quantity)
        })
        && target_reference_matches_role(object.object_kind(), object.target)
        && denotes_reference_matches_role(object.object_kind(), object.denotes)
        && object.relative_clauses.iter().all(|clause| {
            clause.body.object_kind() == SemanticObjectKind::Formula
                && object.object_kind() == SemanticObjectKind::Referent
        })
        && object.descriptor.as_ref().is_none_or(|descriptor| {
            optional_reference_has_kind(descriptor.speaker, SemanticObjectKind::Referent)
                && optional_reference_has_kind(descriptor.body, SemanticObjectKind::Formula)
                && optional_reference_has_kind(descriptor.quantity, SemanticObjectKind::Quantity)
                && optional_reference_has_kind(descriptor.scale, SemanticObjectKind::Referent)
                && descriptor
                    .operand
                    .is_none_or(|operand| argument_object_kind_can_fill(operand.object_kind()))
        })
        && references_have_kind(&object.parameters, SemanticObjectKind::Parameter)
        && references_have_kind(&object.embedded_questions, SemanticObjectKind::Question)
        && object.quotation.as_ref().is_none_or(|quotation| {
            optional_reference_has_kind(quotation.utterance, SemanticObjectKind::Utterance)
        })
        && displayed_content_shape_matches_role(object)
        && references_have_kind(&object.operands, SemanticObjectKind::MathExpression)
        && math_operator_parameter_matches_role(object)
        && math_endpoint_inclusion_matches_role(object)
        && object
            .value
            .as_ref()
            .is_none_or(quantity_value_references_match_roles)
        && optional_reference_has_kind(object.asker, SemanticObjectKind::Referent)
        && optional_reference_has_kind(object.respondent, SemanticObjectKind::Referent)
        && object
            .slots
            .iter()
            .all(|slot| slot.parameter.object_kind() == SemanticObjectKind::Parameter)
        && object.place_questions.iter().all(|question| {
            question.parameter.object_kind() == SemanticObjectKind::Parameter
                && question
                    .candidate_places
                    .iter()
                    .all(|place| place.get() > 0)
        })
        && question_focus_matches_role(object.focus)
        && question_focus_matches_role(object.presupposed_answer)
        && object.subscript.as_ref().is_none_or(|subscript| {
            subscript.value.object_kind() == SemanticObjectKind::MathExpression
        })
}

#[requires(true)]
#[ensures(true)]
fn optional_ordinal_label_target_matches_role(target: Option<SemanticObjectId>) -> bool {
    target.is_none_or(|target| {
        matches!(
            target.object_kind(),
            SemanticObjectKind::Utterance
                | SemanticObjectKind::Sequence
                | SemanticObjectKind::Formula
                | SemanticObjectKind::Referent
                | SemanticObjectKind::DisplayedContent
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn question_focus_matches_role(focus: Option<SemanticObjectId>) -> bool {
    focus.is_none_or(|focus| {
        focus.object_kind() == SemanticObjectKind::Parameter
            || focus.object_kind() == SemanticObjectKind::Referent
    })
}

#[requires(true)]
#[ensures(true)]
fn displayed_content_shape_matches_role(object: &SemanticObject) -> bool {
    if object.object_kind() != SemanticObjectKind::DisplayedContent {
        return true;
    }
    object.family.is_some()
        && object
            .relation
            .as_ref()
            .is_some_and(|relation| !relation.is_empty())
        && object.polarity.is_some()
        && object.assertion_effect.is_some()
        && optional_reference_has_kind(object.experiencer, SemanticObjectKind::Referent)
        && object
            .target
            .is_some_and(|target| displayed_content_target_kind_is_allowed(target.object_kind()))
        && optional_reference_has_kind(object.anchor, SemanticObjectKind::Utterance)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_question_slots_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.iter().all(|(id, object)| {
        if id.object_kind() == SemanticObjectKind::Parameter
            && !parameter_role_matches_sort(object.sort, object.role)
        {
            return false;
        }

        if object.tense_modal.is_some_and(|parameter| {
            !parameter_has_sort_and_role(
                objects,
                parameter,
                SemanticSort::TenseModal,
                ParameterRole::TenseQuestion,
            )
        }) {
            return false;
        }

        if object.operator_parameter.is_some_and(|parameter| {
            !parameter_has_sort_and_role(
                objects,
                parameter,
                SemanticSort::MathOperator,
                ParameterRole::MathOperatorQuestion,
            )
        }) {
            return false;
        }

        if object.variable.is_some_and(|variable| {
            variable.object_kind() == SemanticObjectKind::Parameter
                && !parameter_has_sort_and_role(
                    objects,
                    variable,
                    SemanticSort::Relation,
                    ParameterRole::RelationVariable,
                )
        }) {
            return false;
        }

        for binding in &object.bindings {
            if binding.variable.object_kind() == SemanticObjectKind::Parameter
                && !parameter_has_sort_and_role(
                    objects,
                    binding.variable,
                    SemanticSort::Relation,
                    ParameterRole::RelationVariable,
                )
            {
                return false;
            }
        }

        connector_question_slot_is_valid(objects, object)
    })
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_compositions_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        object
            .composition
            .as_ref()
            .is_none_or(|composition| composition_operator_parameter_is_valid(objects, composition))
    })
}

#[requires(true)]
#[ensures(true)]
fn composition_operator_parameter_is_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    composition: &Composition,
) -> bool {
    let Some(parameter) = composition.operator_parameter else {
        return composition.operator != "connectiveQuestion";
    };
    composition.operator == "connectiveQuestion"
        && parameter_has_sort_and_role(
            objects,
            parameter,
            SemanticSort::Connective,
            ParameterRole::ConnectiveQuestion,
        )
}

#[requires(true)]
#[ensures(true)]
fn connector_question_slot_is_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    object: &SemanticObject,
) -> bool {
    let operator = object
        .operator
        .as_ref()
        .and_then(|operator| match operator.as_data() {
            data!(SemanticOperator::Formula(operator)) => Some(*operator),
            data!(SemanticOperator::Math(_)) => None,
        });
    let Some(connector) = &object.connector else {
        return operator != Some(FormulaOperator::ConnectiveQuestion);
    };
    if connector.truth_table.is_some() && connector.parameter.is_some() {
        return false;
    }
    if operator == Some(FormulaOperator::ConnectiveQuestion) {
        return connector.truth_table.is_none()
            && connector.parameter.is_some_and(|parameter| {
                parameter_has_sort_and_role(
                    objects,
                    parameter,
                    SemanticSort::Connective,
                    ParameterRole::ConnectiveQuestion,
                )
            });
    }
    connector.parameter.is_none()
}

#[requires(true)]
#[ensures(true)]
fn parameter_has_sort_and_role(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    id: SemanticObjectId,
    sort: SemanticSort,
    role: ParameterRole,
) -> bool {
    objects
        .get(&id)
        .is_some_and(|object| object.sort == Some(sort) && object.role == Some(role))
}

#[requires(true)]
#[ensures(true)]
fn parameter_role_matches_sort(sort: Option<SemanticSort>, role: Option<ParameterRole>) -> bool {
    match role {
        Some(ParameterRole::PropertySlot) => sort.is_some(),
        Some(ParameterRole::RelativeClauseHead)
        | Some(ParameterRole::ArgumentQuestion)
        | Some(ParameterRole::AttitudeQuestion) => sort == Some(SemanticSort::Entity),
        Some(ParameterRole::RelationQuestion)
        | Some(ParameterRole::RelationVariable)
        | Some(ParameterRole::UnspecifiedRelation) => sort == Some(SemanticSort::Relation),
        Some(ParameterRole::PlaceQuestion) => sort == Some(SemanticSort::Place),
        Some(ParameterRole::ConnectiveQuestion) => sort == Some(SemanticSort::Connective),
        Some(ParameterRole::TenseQuestion) => sort == Some(SemanticSort::TenseModal),
        Some(ParameterRole::MathOperatorQuestion) => sort == Some(SemanticSort::MathOperator),
        Some(ParameterRole::RespectiveSlot) => sort.is_some(),
        None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn math_operator_parameter_matches_role(object: &SemanticObject) -> bool {
    let Some(parameter) = object.operator_parameter else {
        return true;
    };
    object.object_kind() == SemanticObjectKind::MathExpression
        && object.operator.is_none()
        && object.literal.is_none()
        && !object.operands.is_empty()
        && parameter.object_kind() == SemanticObjectKind::Parameter
}

#[requires(true)]
#[ensures(true)]
fn math_endpoint_inclusion_matches_role(object: &SemanticObject) -> bool {
    let Some(_endpoint_inclusion) = object.endpoint_inclusion else {
        return true;
    };
    if object.object_kind() != SemanticObjectKind::MathExpression {
        return false;
    }
    object
        .operator
        .as_ref()
        .is_some_and(|operator| matches!(operator.as_data(), data!(SemanticOperator::Math(operator)) if operator.ends_with("Interval")))
}

#[requires(true)]
#[ensures(true)]
fn quantifier_variable_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    kind == SemanticObjectKind::Referent || kind == SemanticObjectKind::Parameter
}

#[requires(true)]
#[ensures(true)]
fn quantifier_formula_operator_is_allowed(operator: FormulaOperator) -> bool {
    matches!(
        operator,
        FormulaOperator::Exists
            | FormulaOperator::Forall
            | FormulaOperator::None
            | FormulaOperator::Cardinality
            | FormulaOperator::PluralExists
            | FormulaOperator::PluralForall
    )
}

#[requires(true)]
#[ensures(true)]
fn quantifier_binding_matches_role(binding: &QuantifierBinding) -> bool {
    quantifier_formula_operator_is_allowed(binding.operator)
        && quantifier_variable_kind_is_allowed(binding.variable.object_kind())
        && optional_reference_has_kind(binding.source_variable, SemanticObjectKind::Referent)
        && binding.selection_source.as_ref().is_none_or(|source| {
            source.variable.object_kind() == SemanticObjectKind::Referent
                && binding
                    .source_variable
                    .is_none_or(|variable| variable == source.variable)
        })
        && optional_reference_has_kind(binding.restriction, SemanticObjectKind::Formula)
        && optional_reference_has_kind(binding.quantity, SemanticObjectKind::Quantity)
}

#[requires(true)]
#[ensures(true)]
fn quantifier_bundle_shape_matches_role(object: &SemanticObject) -> bool {
    let is_bundle = object.operator.as_ref().is_some_and(|operator| {
        matches!(
            operator.as_data(),
            data!(SemanticOperator::Formula(FormulaOperator::QuantifierBundle))
        )
    });
    if is_bundle {
        !object.bindings.is_empty()
            && object.coequal_scope
            && optional_reference_has_kind(object.body, SemanticObjectKind::Formula)
    } else {
        object.bindings.is_empty() && !object.coequal_scope
    }
}

#[requires(true)]
#[ensures(true)]
fn optional_reference_has_kind(
    reference: Option<SemanticObjectId>,
    kind: SemanticObjectKind,
) -> bool {
    reference.is_none_or(|reference| reference.object_kind() == kind)
}

#[requires(true)]
#[ensures(true)]
fn target_reference_matches_role(
    object_kind: SemanticObjectKind,
    target: Option<SemanticObjectId>,
) -> bool {
    let Some(target) = target else {
        return true;
    };
    match object_kind {
        SemanticObjectKind::Referent => matches!(
            target.object_kind(),
            SemanticObjectKind::Utterance
                | SemanticObjectKind::Sequence
                | SemanticObjectKind::Formula
                | SemanticObjectKind::Referent
        ),
        _ => true,
    }
}

#[requires(true)]
#[ensures(true)]
fn denotes_reference_matches_role(
    object_kind: SemanticObjectKind,
    denotes: Option<SemanticObjectId>,
) -> bool {
    let Some(denotes) = denotes else {
        return true;
    };
    match object_kind {
        SemanticObjectKind::MathExpression => argument_object_kind_can_fill(denotes.object_kind()),
        _ => true,
    }
}

#[requires(true)]
#[ensures(true)]
fn quantity_value_references_match_roles(value: &QuantityValue) -> bool {
    value.math_expression.is_none_or(|math_expression| {
        math_expression.object_kind() == SemanticObjectKind::MathExpression
    })
}

#[requires(true)]
#[ensures(true)]
fn content_reference_matches_role(
    object: &SemanticObject,
    content: Option<SemanticObjectId>,
) -> bool {
    let Some(content) = content else {
        return true;
    };
    match object.object_kind() {
        SemanticObjectKind::Utterance => {
            utterance_content_reference_matches_force(object.force, content)
        }
        SemanticObjectKind::Sequence => content.object_kind() == SemanticObjectKind::Formula,
        SemanticObjectKind::Referent
            if object
                .sort
                .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())) =>
        {
            matches!(
                content.object_kind(),
                SemanticObjectKind::Formula | SemanticObjectKind::Sequence
            )
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn optional_eventuality_reference(reference: Option<SemanticObjectId>) -> bool {
    reference.is_none_or(|reference| {
        reference.object_kind() == SemanticObjectKind::Referent
            && reference
                .referent_sort()
                .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
    })
}

#[requires(true)]
#[ensures(true)]
fn utterance_content_reference_matches_force(
    force: Option<UtteranceForce>,
    content: SemanticObjectId,
) -> bool {
    let ordinary_content = matches!(
        content.object_kind(),
        SemanticObjectKind::Formula
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Question
            | SemanticObjectKind::DisplayedContent
    );
    if ordinary_content {
        return true;
    }
    matches!(
        force,
        Some(UtteranceForce::Mention | UtteranceForce::Vocative)
    ) && argument_object_kind_can_fill(content.object_kind())
}

#[requires(true)]
#[ensures(true)]
fn references_have_kind(references: &[SemanticObjectId], kind: SemanticObjectKind) -> bool {
    references
        .iter()
        .all(|reference| reference.object_kind() == kind)
}

#[requires(true)]
#[ensures(true)]
fn sequence_item_kind_is_allowed(kind: SemanticObjectKind) -> bool {
    matches!(
        kind,
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::DisplayedContent
    )
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_arguments_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        let modal_arguments_valid = object.modal_arguments.iter().all(|argument| {
            argument.arguments.iter().all(|(place, value)| {
                place.get() > 0 && argument_value_references_allowed_objects(value, objects)
            })
        });
        if object.object_kind() != SemanticObjectKind::Predication {
            return modal_arguments_valid;
        }
        let has_relation = object.relation.is_some() ^ object.relation_parameter.is_some();
        has_relation
            && object.arguments.iter().all(|(place, value)| {
                place.get() > 0 && argument_value_references_allowed_objects(value, objects)
            })
            && object.place_questions.iter().all(|question| {
                objects
                    .get(&question.parameter)
                    .is_some_and(|object| object.object_kind() == SemanticObjectKind::Parameter)
                    && argument_value_references_allowed_objects(&question.argument, objects)
                    && question
                        .candidate_places
                        .iter()
                        .all(|place| place.get() > 0)
            })
            && modal_arguments_valid
            && object.reciprocity.iter().all(|exchange| {
                argument_value_references_allowed_objects(&exchange.left, objects)
                    && argument_value_references_allowed_objects(&exchange.right, objects)
            })
    })
}

#[requires(true)]
#[ensures(true)]
fn argument_value_references_allowed_objects(
    value: &ArgumentValue,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let value_is_valid = match value.value {
        Some(value) => objects
            .get(&value)
            .is_some_and(|object| argument_object_kind_can_fill(object.object_kind())),
        None => value.kind == ArgumentValueKind::Deleted,
    };
    value_is_valid
        && value.quantity.is_none_or(|quantity| {
            objects
                .get(&quantity)
                .is_some_and(|object| object.object_kind() == SemanticObjectKind::Quantity)
        })
        && value.relative_clauses.iter().all(|clause| {
            objects
                .get(&clause.body)
                .is_some_and(|object| object.object_kind() == SemanticObjectKind::Formula)
        })
}

#[requires(true)]
#[ensures(true)]
pub fn is_numbered_argument_place(place: &str) -> bool {
    PlaceIndex::from_numbered_label(place).is_some()
}

#[requires(true)]
#[ensures(true)]
pub fn source_from_spans(
    spans: &[SourceSpan],
    source_text: Option<&str>,
    construct: Option<&str>,
) -> Option<SemanticSource> {
    let first = spans.first()?;
    let byte_start = spans
        .iter()
        .map(|span| span.byte_start)
        .min()
        .unwrap_or(first.byte_start);
    let byte_end = spans
        .iter()
        .map(|span| span.byte_end)
        .max()
        .unwrap_or(first.byte_end);
    let text = source_text
        .and_then(|text| text.get(byte_start..byte_end))
        .map(str::to_owned);
    Some(SemanticSource {
        span: SourceByteSpan {
            byte_start,
            byte_end,
        },
        text,
        construct: construct.map(str::to_owned),
    })
}

#[requires(true)]
#[ensures(true)]
pub fn diagnostic(message: impl Into<String>) -> SemanticDiagnostic {
    SemanticDiagnostic::warning(message)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_graph_object_ids_match_types(graph: &SemanticGraph) -> bool {
    semantic_object_ids_match_types(&graph.objects)
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_graph_references_are_defined(graph: &SemanticGraph) -> bool {
    semantic_object_references_are_defined(&graph.objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn eventuality_subsorts_satisfy_general_eventuality() {
        let general = SemanticSort::eventuality();
        let process = SemanticSort::Eventuality(EventualitySort::Process);
        assert!(process.is_subsort_of(general));
        assert!(general.is_subsort_of(general));
        assert!(!process.is_subsort_of(SemanticSort::Entity));
        assert!(!SemanticSort::Entity.is_subsort_of(general));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_object_id_rejects_zero_index_data() {
        let invalid = SemanticObjectId::try_from_data(data!(SemanticObjectId {
            prefix: SemanticIdPrefix::Structural(SemanticObjectKind::Formula),
            index: 0,
        }));

        assert!(invalid.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_dangling_object_references() {
        let root = SemanticObjectId::formula(1);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(SemanticObjectId::predication(2), None, Vec::new()),
        );

        let error = SemanticGraph::new(root, objects).expect_err("dangling reference");
        assert!(error.to_string().contains("must not dangle"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_dangling_scalar_negation_scale() {
        let root = SemanticObjectId::formula(1);
        let predication = SemanticObjectId::predication(2);
        let mut object = SemanticObject::predication(
            "klama".to_owned(),
            None,
            BTreeMap::new(),
            PredicationMode::Asserted,
            None,
            Vec::new(),
        );
        object.scalar_negation = Some(
            ScalarNegation::new(ScalarNegationKind::OtherThan, "na'e".to_owned())
                .with_scale(SemanticObjectId::referent(3)),
        );

        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(predication, object);

        let error = SemanticGraph::new(root, objects).expect_err("dangling scale reference");
        assert!(error.to_string().contains("must not dangle"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_wrong_kind_role_references() {
        let root = SemanticObjectId::formula(1);
        let referent = SemanticObjectId::referent(2);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![referent],
                None,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            referent,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                None,
                None,
                None,
                Vec::new(),
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("wrong reference kind");
        assert!(error.to_string().contains("match semantic roles"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_malformed_displayed_content() {
        let root = SemanticObjectId::displayed_content(1);
        let mut display = SemanticObject::empty(SemanticObjectKind::DisplayedContent);
        display.family = Some(DisplayedContentFamily::PropositionalAttitude);
        display.relation = Some("hope".to_owned());
        display.polarity = Some(DisplayedContentPolarity::Positive);

        let mut objects = BTreeMap::new();
        objects.insert(root, display);

        let error = SemanticGraph::new(root, objects).expect_err("malformed displayed content");
        assert!(error.to_string().contains("match semantic roles"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn place_index_rejects_malformed_argument_places() {
        assert!(PlaceIndex::from_numbered_label("01").is_none());
        assert!(PlaceIndex::from_numbered_label("x0").is_none());
        assert!(PlaceIndex::from_numbered_label("x01").is_none());
        assert_eq!(
            serde_json::to_string(&PlaceIndex::new(10)).expect("place index serializes"),
            "\"x10\""
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn place_index_serializes_argument_maps_in_numeric_order() {
        let mut arguments = BTreeMap::new();
        arguments.insert(
            PlaceIndex::new(10),
            ArgumentValue::filled(SemanticObjectId::referent(10), None),
        );
        arguments.insert(
            PlaceIndex::new(2),
            ArgumentValue::filled(SemanticObjectId::referent(2), None),
        );

        let predication = SemanticObject::predication(
            "broda".to_owned(),
            None,
            arguments,
            PredicationMode::Restrictive,
            None,
            Vec::new(),
        );
        let json = serde_json::to_string(&predication).expect("predication serializes");

        let x2_position = json.find(r#""x2""#).expect("x2 key is serialized");
        let x10_position = json.find(r#""x10""#).expect("x10 key is serialized");
        assert!(
            x2_position < x10_position,
            "argument map must serialize in numeric place order: {json}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_incoherent_parameter_sort() {
        let root = SemanticObjectId::eventuality(1);
        let parameter = SemanticObjectId::parameter(2);
        let mut eventuality = SemanticObject::eventuality(EventualityClass::Event, None, None);
        eventuality.tense_modal = Some(parameter);

        let mut objects = BTreeMap::new();
        objects.insert(root, eventuality);
        objects.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::TenseQuestion,
                "cu'e".to_owned(),
                None,
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("wrong parameter sort");
        assert!(error.to_string().contains("coherent parameters"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_impossible_connector_question() {
        let root = SemanticObjectId::formula(1);
        let child = SemanticObjectId::formula(2);
        let predication = SemanticObjectId::predication(3);
        let parameter = SemanticObjectId::parameter(4);

        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::connective_formula(
                FormulaOperator::ConnectiveQuestion,
                vec![child],
                Some(new!(Connector {
                    source: "je'i".to_owned(),
                    locus: "tense".to_owned(),
                    truth_table: Some("je".to_owned()),
                    parameter: Some(parameter),
                })),
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            child,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(
            predication,
            SemanticObject::predication(
                "king".to_owned(),
                None,
                BTreeMap::new(),
                PredicationMode::Asserted,
                None,
                Vec::new(),
            ),
        );
        objects.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Connective,
                ParameterRole::ConnectiveQuestion,
                "je'i".to_owned(),
                None,
            ),
        );

        let error = SemanticGraph::new(root, objects).expect_err("impossible connector");
        assert!(error.to_string().contains("coherent parameters"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn argument_value_invariant_rejects_deleted_values() {
        let invalid = ArgumentValue::try_from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: Some(SemanticObjectId::referent(1)),
            quantity: None,
            introduced_by: Some("zi'o".to_owned()),
            source: None,
            relative_clauses: Vec::new(),
            command_target: None,
        }));

        assert!(invalid.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn argument_value_invariant_rejects_deleted_quantities() {
        let invalid = ArgumentValue::try_from_data(data!(ArgumentValue {
            kind: ArgumentValueKind::Deleted,
            value: None,
            quantity: Some(SemanticObjectId::quantity(1)),
            introduced_by: Some("zi'o".to_owned()),
            source: None,
            relative_clauses: Vec::new(),
            command_target: None,
        }));

        assert!(invalid.is_err());
    }
}
