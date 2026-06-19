//! Public semantic object graph model serialized by `tersmu --format json`.

use std::collections::BTreeMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_source::SourceSpan;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

pub const SEMANTIC_JSON_VERSION: &str = "lojban-semantics-json-1";

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticObjectId {
    kind: SemanticObjectKind,
    index: usize,
    referent_special: Option<SemanticReferentSpecial>,
}

impl SemanticObjectId {
    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Utterance)]
    pub fn utterance(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Utterance, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Sequence, index)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Eventuality)]
    pub fn eventuality(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Eventuality, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn referent(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Referent, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Parameter)]
    pub fn parameter(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Parameter, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn predication(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Predication, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn formula(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Formula, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Abstraction)]
    pub fn abstraction(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Abstraction, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sign)]
    pub fn sign(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Sign, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::DisplayedContent)]
    pub fn displayed_content(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::DisplayedContent, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::MathExpression, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Quantity)]
    pub fn quantity(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Quantity, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::RelationMetadata)]
    pub fn relation_metadata(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::RelationMetadata, index)
    }

    #[requires(index > 0)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    pub fn question(index: usize) -> Self {
        Self::numbered(SemanticObjectKind::Question, index)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn speaker() -> Self {
        Self::special_referent(SemanticReferentSpecial::Speaker)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn addressee() -> Self {
        Self::special_referent(SemanticReferentSpecial::Addressee)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn speech_time() -> Self {
        Self::special_referent(SemanticReferentSpecial::SpeechTime)
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn here() -> Self {
        Self::special_referent(SemanticReferentSpecial::Here)
    }

    #[requires(index > 0 || kind == SemanticObjectKind::Eventuality)]
    #[ensures(ret.object_kind() == kind)]
    fn numbered(kind: SemanticObjectKind, index: usize) -> Self {
        Self {
            kind,
            index,
            referent_special: None,
        }
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    fn special_referent(referent_special: SemanticReferentSpecial) -> Self {
        Self {
            kind: SemanticObjectKind::Referent,
            index: 0,
            referent_special: Some(referent_special),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn object_kind(self) -> SemanticObjectKind {
        self.kind
    }
}

impl fmt::Display for SemanticObjectId {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(referent) = self.referent_special {
            return write!(formatter, "referent:{referent}");
        }
        match self.kind {
            SemanticObjectKind::Utterance => write!(formatter, "utterance:u{}", self.index),
            SemanticObjectKind::Sequence => write!(formatter, "sequence:s{}", self.index),
            SemanticObjectKind::Eventuality => write!(formatter, "eventuality:e{}", self.index),
            SemanticObjectKind::Referent => write!(formatter, "referent:r{}", self.index),
            SemanticObjectKind::Parameter => write!(formatter, "parameter:p{}", self.index),
            SemanticObjectKind::Predication => write!(formatter, "predication:p{}", self.index),
            SemanticObjectKind::Formula => write!(formatter, "formula:f{}", self.index),
            SemanticObjectKind::Abstraction => write!(formatter, "abstraction:a{}", self.index),
            SemanticObjectKind::Sign => write!(formatter, "sign:s{}", self.index),
            SemanticObjectKind::DisplayedContent => write!(formatter, "display:d{}", self.index),
            SemanticObjectKind::MathExpression => write!(formatter, "math:m{}", self.index),
            SemanticObjectKind::Quantity => write!(formatter, "quantity:q{}", self.index),
            SemanticObjectKind::RelationMetadata => write!(formatter, "relation:r{}", self.index),
            SemanticObjectKind::Question => write!(formatter, "question:q{}", self.index),
        }
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticReferentSpecial {
    Speaker,
    Addressee,
    SpeechTime,
    Here,
}

pub type SemanticReferentId = SemanticReferentSpecial;

impl fmt::Display for SemanticReferentSpecial {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Speaker => formatter.write_str("speaker"),
            Self::Addressee => formatter.write_str("addressee"),
            Self::SpeechTime => formatter.write_str("speech-time"),
            Self::Here => formatter.write_str("here"),
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

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGraph {
    pub version: &'static str,
    pub root: SemanticObjectId,
    #[serde(serialize_with = "serialize_objects")]
    pub objects: BTreeMap<SemanticObjectId, SemanticObject>,
}

impl SemanticGraph {
    #[requires(objects.contains_key(&root))]
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|graph| graph.root == root))]
    pub fn new(
        root: SemanticObjectId,
        objects: BTreeMap<SemanticObjectId, SemanticObject>,
    ) -> Result<Self, String> {
        if !semantic_object_ids_match_types(&objects) {
            return Err("semantic object ID prefixes must match object types".to_owned());
        }
        if !semantic_object_references_are_defined(&objects) {
            return Err("semantic object references must not dangle".to_owned());
        }
        if !semantic_object_references_match_roles(&objects) {
            return Err("semantic object references must match semantic roles".to_owned());
        }
        if !semantic_object_arguments_are_valid(&objects) {
            return Err(
                "predication arguments must use valid numbered places and argument fillers"
                    .to_owned(),
            );
        }
        Ok(Self {
            version: SEMANTIC_JSON_VERSION,
            root,
            objects,
        })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
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
    #[serde(rename = "relation", skip_serializing_if = "Option::is_none")]
    pub sequence_relation: Option<SequenceRelation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<EventualityClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actuality: Option<Actuality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<AnchorRelation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect: Option<Aspect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space: Option<AnchorRelation>,
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, ArgumentValue>,
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
    pub predication: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<Connector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restriction: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotation: Option<Quotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denotes: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<DisplayedContentFamily>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiencer: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SemanticObjectId>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<QuantityScale>,
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
            sequence_relation: None,
            class: None,
            actuality: None,
            time: None,
            aspect: None,
            space: None,
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
            arguments: BTreeMap::new(),
            place_questions: Vec::new(),
            modal_arguments: Vec::new(),
            reciprocity: Vec::new(),
            mode: None,
            scalar_negation: None,
            relation_metadata: None,
            operator: None,
            predication: None,
            children: Vec::new(),
            connector: None,
            variable: None,
            restriction: None,
            body: None,
            quantity: None,
            abstraction_kind: None,
            abstracted: None,
            parameters: Vec::new(),
            arity: None,
            embedded_questions: Vec::new(),
            sign_kind: None,
            text: None,
            quotation: None,
            denotes: None,
            family: None,
            experiencer: None,
            target: None,
            anchor: None,
            operands: Vec::new(),
            literal: None,
            form: None,
            value: None,
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
            source: None,
            diagnostics: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Utterance)]
    pub fn utterance(
        force: UtteranceForce,
        eventuality: SemanticObjectId,
        content: Option<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Utterance);
        object.force = Some(force);
        object.speaker = Some(SemanticObjectId::speaker());
        object.audience = Some(SemanticObjectId::addressee());
        object.eventuality = Some(eventuality);
        object.content = content;
        object.deictic_ground = Some(DeicticGround {
            time: SemanticObjectId::speech_time(),
            place: SemanticObjectId::here(),
        });
        object.source = source;
        object.diagnostics = diagnostics;
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
    #[ensures(ret.object_kind() == SemanticObjectKind::Eventuality)]
    pub fn eventuality(
        class: EventualityClass,
        actuality: Option<Actuality>,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Eventuality);
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
        arguments: BTreeMap<String, ArgumentValue>,
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

    #[requires(relation_parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    pub fn relation_parameter_predication(
        relation_parameter: SemanticObjectId,
        eventuality: Option<SemanticObjectId>,
        arguments: BTreeMap<String, ArgumentValue>,
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

    #[requires(matches!(
        operator,
        FormulaOperator::Exists
            | FormulaOperator::Forall
            | FormulaOperator::None
            | FormulaOperator::Cardinality
            | FormulaOperator::PluralExists
            | FormulaOperator::PluralForall
    ))]
    #[requires(variable.object_kind() == SemanticObjectKind::Referent)]
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

    #[requires(parameters
        .iter()
        .all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Abstraction)]
    pub fn abstraction(
        kind: AbstractionKind,
        body: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Abstraction);
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

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    pub fn question(
        kind: QuestionKind,
        mode: QuestionMode,
        domain: SemanticSort,
        body: SemanticObjectId,
        slots: Vec<QuestionSlot>,
        source: Option<SemanticSource>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Question);
        object.question_kind = Some(kind);
        object.question_mode = Some(mode);
        object.asker = Some(SemanticObjectId::speaker());
        object.respondent = Some(SemanticObjectId::addressee());
        object.domain = Some(domain);
        object.body = Some(body);
        object.slots = slots;
        object.source = source;
        object
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sign)]
    pub fn sign(
        sign_kind: SignKind,
        quotation: Option<Quotation>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let mut object = Self::empty(SemanticObjectKind::Sign);
        object.sign_kind = Some(sign_kind);
        object.quotation = quotation;
        object.source = source;
        object.diagnostics = diagnostics;
        object
    }

    #[requires(sign_kind != SignKind::Quotation)]
    #[requires(!text.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sign)]
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
        object.scale = Some(scale);
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
        if let Some(time) = &self.time {
            out.push(time.anchor);
        }
        if let Some(space) = &self.space {
            out.push(space.anchor);
        }
        if let Some(descriptor) = &self.descriptor {
            descriptor.references_into(out);
        }
        if let Some(composition) = &self.composition {
            out.extend(composition.members.iter().copied());
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
        for exchange in &self.reciprocity {
            exchange.references_into(out);
        }
        extend_optional(out, self.relation_parameter);
        extend_optional(out, self.relation_metadata);
        if let Some(expansion) = &self.expansion {
            expansion.references_into(out);
        }
        extend_optional(out, self.predication);
        out.extend(self.children.iter().copied());
        extend_optional(out, self.variable);
        extend_optional(out, self.restriction);
        extend_optional(out, self.body);
        extend_optional(out, self.quantity);
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
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn push_diagnostic(&mut self, diagnostic: SemanticDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    #[requires(quantity.object_kind() == SemanticObjectKind::Quantity)]
    #[ensures(true)]
    pub fn set_descriptor_quantity(&mut self, quantity: SemanticObjectId) {
        if let Some(descriptor) = &mut self.descriptor {
            descriptor.quantity = Some(quantity);
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
    Vocative,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceRelation {
    SameTopicContinuation,
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

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRelation {
    pub relation: String,
    pub anchor: SemanticObjectId,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aspect {
    pub contour: String,
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SemanticSort {
    Entity,
    Mass,
    Set,
    Sequence,
    Eventuality,
    Predication,
    TruthValue,
    Proposition,
    Concept,
    Amount,
    Quantity,
    Number,
    Text,
    Sign,
    Relation,
    Place,
    ArgumentBundle,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexicalKind {
    Speaker,
    Audience,
    SpeechTime,
    Here,
    ProximalDemonstrative,
    MedialDemonstrative,
    DistalDemonstrative,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub kind: String,
    pub word: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<SemanticObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_clauses: Vec<RelativeClause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
        extend_optional(out, self.operand);
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    pub operator: String,
    pub members: Vec<SemanticObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective: Option<bool>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterRole {
    PropertySlot,
    RelativeClauseHead,
    ArgumentQuestion,
    RelationQuestion,
    PlaceQuestion,
    ConnectiveQuestion,
    TenseQuestion,
    AttitudeQuestion,
}

#[invariant(argument_value_shape_is_valid(*kind, *value, introduced_by.as_deref()))]
#[invariant(*kind != ArgumentValueKind::Deleted || relative_clauses.is_empty())]
#[invariant(*kind != ArgumentValueKind::Deleted || quantity.is_none())]
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
#[invariant(candidate_places.iter().all(|place| is_numbered_argument_place(place)))]
#[invariant(candidate_places.iter().enumerate().all(|(index, place)| !candidate_places[..index].contains(place)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceQuestionBinding {
    pub parameter: SemanticObjectId,
    pub argument: ArgumentValue,
    pub candidate_places: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl PlaceQuestionBinding {
    #[requires(parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[requires(!candidate_places.is_empty())]
    #[requires(candidate_places.iter().all(|place| is_numbered_argument_place(place)))]
    #[ensures(ret.parameter == parameter)]
    pub fn new(
        parameter: SemanticObjectId,
        argument: ArgumentValue,
        candidate_places: Vec<String>,
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

#[invariant(!relation.is_empty(), "modal relation must be named")]
#[invariant(!introduced_by.is_empty(), "modal source marker must be named")]
#[invariant(!arguments.is_empty(), "modal relation must have at least one explicit place")]
#[invariant(arguments.keys().all(|place| is_numbered_argument_place(place)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalArgument {
    pub relation: String,
    pub introduced_by: String,
    pub arguments: BTreeMap<String, ArgumentValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SemanticSource>,
}

impl ModalArgument {
    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| is_numbered_argument_place(place)))]
    #[ensures(true)]
    pub fn new(
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<String, ArgumentValue>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::from_data(data!(ModalArgument {
            relation,
            introduced_by,
            arguments,
            source,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        for argument in self.arguments.values() {
            argument.references_into(out);
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
        SemanticObjectKind::Referent
            | SemanticObjectKind::Parameter
            | SemanticObjectKind::Eventuality
            | SemanticObjectKind::Abstraction
            | SemanticObjectKind::Sign
            | SemanticObjectKind::DisplayedContent
            | SemanticObjectKind::MathExpression
            | SemanticObjectKind::Quantity
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalarNegation {
    pub kind: ScalarNegationKind,
    pub introduced_by: String,
}

impl ScalarNegation {
    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    pub fn new(kind: ScalarNegationKind, introduced_by: String) -> Self {
        Self::from_data(data!(ScalarNegation {
            kind,
            introduced_by,
        }))
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
    Not,
    Scoped,
    And,
    Or,
    Implies,
    Iff,
    ExclusiveOr,
    WhetherOrNot,
    Exists,
    Forall,
    None,
    Cardinality,
    PluralExists,
    PluralForall,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    pub source: String,
    pub locus: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth_table: Option<String>,
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignKind {
    Quotation,
    Letteral,
    MathExpression,
    Word,
    Text,
}

#[invariant(true)]
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
    PropositionalAttitude,
    Evidential,
    Discursive,
    Metalinguistic,
    Emphasis,
    QuestionPrompt,
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
}

#[invariant(::Integer(_) => true)]
#[invariant(::Text(value) => !value.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MathLiteralValue {
    Integer(i64),
    Text(String),
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
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_ids_match_types(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects
        .iter()
        .all(|(id, object)| id.object_kind() == object.object_kind())
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_references_are_defined(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let mut references = Vec::new();
    for object in objects.values() {
        object.references_into(&mut references);
    }
    references
        .into_iter()
        .all(|reference| objects.contains_key(&reference))
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
        && optional_reference_has_kind(object.eventuality, SemanticObjectKind::Eventuality)
        && utterance_content_reference_matches_force(object.force, object.content)
        && object.deictic_ground.is_none_or(|ground| {
            ground.time.object_kind() == SemanticObjectKind::Referent
                && ground.place.object_kind() == SemanticObjectKind::Referent
        })
        && references_have_kind(&object.asides, SemanticObjectKind::Utterance)
        && object
            .items
            .iter()
            .all(|item| sequence_item_kind_is_allowed(item.object_kind()))
        && optional_reference_has_kind(
            object.relation_metadata,
            SemanticObjectKind::RelationMetadata,
        )
        && optional_reference_has_kind(object.relation_parameter, SemanticObjectKind::Parameter)
        && optional_reference_has_kind(object.predication, SemanticObjectKind::Predication)
        && references_have_kind(&object.children, SemanticObjectKind::Formula)
        && optional_reference_has_kind(object.variable, SemanticObjectKind::Referent)
        && optional_reference_has_kind(object.restriction, SemanticObjectKind::Formula)
        && optional_reference_has_kind(object.body, SemanticObjectKind::Formula)
        && optional_reference_has_kind(object.quantity, SemanticObjectKind::Quantity)
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
                && descriptor
                    .operand
                    .is_none_or(|operand| argument_object_kind_can_fill(operand.object_kind()))
        })
        && references_have_kind(&object.parameters, SemanticObjectKind::Parameter)
        && references_have_kind(&object.embedded_questions, SemanticObjectKind::Question)
        && object.quotation.as_ref().is_none_or(|quotation| {
            optional_reference_has_kind(quotation.utterance, SemanticObjectKind::Utterance)
        })
        && references_have_kind(&object.operands, SemanticObjectKind::MathExpression)
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
                    .all(|place| is_numbered_argument_place(place))
        })
        && optional_reference_has_kind(object.focus, SemanticObjectKind::Parameter)
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
                | SemanticObjectKind::Sign
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
        SemanticObjectKind::Sign => matches!(
            denotes.object_kind(),
            SemanticObjectKind::Referent | SemanticObjectKind::MathExpression
        ),
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
fn utterance_content_reference_matches_force(
    force: Option<UtteranceForce>,
    content: Option<SemanticObjectId>,
) -> bool {
    let Some(content) = content else {
        return true;
    };
    let ordinary_content = matches!(
        content.object_kind(),
        SemanticObjectKind::Formula | SemanticObjectKind::Sequence | SemanticObjectKind::Question
    );
    if ordinary_content {
        return true;
    }
    force == Some(UtteranceForce::Mention) && argument_object_kind_can_fill(content.object_kind())
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
        SemanticObjectKind::Utterance | SemanticObjectKind::Sequence
    )
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_object_arguments_are_valid(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    objects.values().all(|object| {
        if object.object_kind() != SemanticObjectKind::Predication {
            return true;
        }
        let has_relation = object.relation.is_some() ^ object.relation_parameter.is_some();
        has_relation
            && object.arguments.iter().all(|(place, value)| {
                is_numbered_argument_place(place)
                    && argument_value_references_allowed_objects(value, objects)
            })
            && object.place_questions.iter().all(|question| {
                objects
                    .get(&question.parameter)
                    .is_some_and(|object| object.object_kind() == SemanticObjectKind::Parameter)
                    && argument_value_references_allowed_objects(&question.argument, objects)
                    && question
                        .candidate_places
                        .iter()
                        .all(|place| is_numbered_argument_place(place))
            })
            && object.modal_arguments.iter().all(|argument| {
                argument.arguments.iter().all(|(place, value)| {
                    is_numbered_argument_place(place)
                        && argument_value_references_allowed_objects(value, objects)
                })
            })
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
    let Some(digits) = place.strip_prefix('x') else {
        return false;
    };
    !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
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
    fn semantic_graph_rejects_dangling_object_references() {
        let root = SemanticObjectId::formula(1);
        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(SemanticObjectId::predication(1), None, Vec::new()),
        );

        let error = SemanticGraph::new(root, objects).expect_err("dangling reference");
        assert!(error.contains("must not dangle"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_wrong_kind_role_references() {
        let root = SemanticObjectId::formula(1);
        let referent = SemanticObjectId::referent(1);
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
        assert!(error.contains("match semantic roles"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_graph_rejects_malformed_argument_places() {
        let root = SemanticObjectId::formula(1);
        let predication = SemanticObjectId::predication(1);
        let referent = SemanticObjectId::referent(1);
        let mut arguments = BTreeMap::new();
        arguments.insert("01".to_owned(), ArgumentValue::filled(referent, None));

        let mut objects = BTreeMap::new();
        objects.insert(
            root,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        );
        objects.insert(
            predication,
            SemanticObject::predication(
                "klama".to_owned(),
                None,
                arguments,
                PredicationMode::Asserted,
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

        let error = SemanticGraph::new(root, objects).expect_err("malformed argument place");
        assert!(error.contains("valid numbered places"));
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
        }));

        assert!(invalid.is_err());
    }
}
