//! Syntax-to-semantic-graph builder for the public JSON semantics model.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};
use jbotci_morphology::{
    Cmavo, LujvoPart, Selmaho, Word, WordData, WordLike, WordLikeData, strip_diacritics,
};
use jbotci_syntax::ast::{
    AbstractionSyntax, AbstractorConnectionSyntax, AfterthoughtBridiTailSyntax,
    BoGroupedBridiTailSyntax, BoundBridiTailConnectionSyntax, BridiSyntax,
    BridiTailConnectionSyntax, BridiTailSyntax, CompositeTenseModalPartSyntax,
    CompositeTenseModalPartSyntaxData, ConnectiveSyntax, ConnectiveSyntaxData, DescriptionSyntax,
    DescriptionTailElementSyntax, DescriptionTailElementSyntaxData,
    ForethoughtBridiConnectionSyntax, ForethoughtBridiConnectionSyntaxData, FragmentSyntax,
    FragmentSyntaxData, FreeModifierSyntax, FreeModifierSyntaxData,
    GroupedBridiTailConnectionSyntax, Indicator, MeksoOperatorSyntax, MeksoOperatorSyntaxData,
    MeksoSyntax, MeksoSyntaxData, ParagraphStatementSyntax, QuantifierSyntax, QuantifierSyntaxData,
    QuoteSyntax, QuoteSyntaxData, RelativeClauseSyntax, RelativeClauseSyntaxData, SelbriSyntax,
    SelbriSyntaxData, SimpleBridiTailSyntax, SimpleBridiTailSyntaxData, StatementSyntax,
    StatementSyntaxData, SubbridiSyntax, SubbridiSyntaxData, SumtiAssociationPhraseSyntax,
    SumtiSyntax, SumtiSyntaxData, SumtiTagSyntaxData, TanruUnitSyntax, TanruUnitSyntaxData,
    TenseModalSyntax, TenseModalSyntaxData, TermSyntax, TermSyntaxData, TextSyntax, Token,
    WithFreeModifiers, WithIndicators, WordRun,
};

use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorMagnitude, AnchorRelation, AnchorRelationData,
    ArgumentValue, ArgumentValueKind, Aspect, AssignedName, AssignedNameData, Composition,
    Connector, Descriptor, DisplayedContentAssertionEffect, DisplayedContentFamily,
    DisplayedContentModifier, DisplayedContentPolarity, EndpointInclusion, EventualityClass,
    FormulaOperator, IndexicalKind, IntervalEndpointInclusion, LetteralUnit, LetteralUnitKind,
    MathLiteral, ModalArgument, ModalNegation, ModalNegationKind, PlaceQuestionBinding,
    PredicationMode, QuantityForm, QuantityScale, QuantityValue, QuestionKind, QuestionMode,
    QuestionSlot, QuestionSlotRole, Quotation, RafsiBinding, ReciprocalExchange, Recurrence,
    RecurrenceConnection, RecurrenceConnectionKind, RecurrenceKind, ReferentCategory,
    RelationExpansion, RelativeClause, RelativeClauseKind, ScalarNegation, ScalarNegationKind,
    SemanticDiagnostic, SemanticGraph, SemanticObject, SemanticObjectId, SemanticOperatorData,
    SemanticSort, SequenceRelation, SignKind, SpaceInterval, SpatialMotion, SpatialMotionKind,
    TemporalPathAnchor, TemporalPathStep, TemporalPathStepData, TimeInterval, TimeSpan,
    TimeSpanEndpoint, UtteranceForce, diagnostic, source_from_spans,
};
use crate::references::{
    BridiNodeId, PlaceFrameKind, PlaceSlot, RawSyntaxNodeId, ReferenceAnalysis,
    ReferenceAnalysisError, ReferenceKind, ReferenceTarget, SelbriPlaceFrameId, SumtiNodeId,
    SumtiPlaceAssignment, SumtiPlaceAssignmentId, TermNodeId, analyze_references,
};

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsError {
    pub kind: SemanticsErrorKind,
    pub message: String,
}

impl From<ReferenceAnalysisError> for SemanticsError {
    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::ReferenceAnalysis)]
    fn from(error: ReferenceAnalysisError) -> Self {
        Self {
            kind: SemanticsErrorKind::ReferenceAnalysis,
            message: error.to_string(),
        }
    }
}

impl SemanticsError {
    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::MissingSyntaxNode)]
    fn missing_syntax_node() -> Self {
        Self {
            kind: SemanticsErrorKind::MissingSyntaxNode,
            message: "semantic builder could not find a syntax node recorded by reference analysis"
                .to_owned(),
        }
    }

    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::DuplicateObject)]
    fn duplicate_object(id: SemanticObjectId) -> Self {
        Self {
            kind: SemanticsErrorKind::DuplicateObject,
            message: format!("semantic builder attempted to insert duplicate object ID {id}"),
        }
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
    fn invalid_graph(message: String) -> Self {
        Self {
            kind: SemanticsErrorKind::InvalidGraph,
            message: format!("semantic graph invariant failed: {message}"),
        }
    }
}

impl fmt::Display for SemanticsError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SemanticsError {}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticsErrorKind {
    ReferenceAnalysis,
    MissingSyntaxNode,
    DuplicateObject,
    InvalidGraph,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct SemanticBuildOptions<'a> {
    pub source_text: Option<&'a str>,
    pub story_time: bool,
}

impl Default for SemanticBuildOptions<'_> {
    #[requires(true)]
    #[ensures(ret.source_text.is_none())]
    #[ensures(!ret.story_time)]
    fn default() -> Self {
        Self {
            source_text: None,
            story_time: false,
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
pub fn build_semantic_graph(
    syntax: &TextSyntax,
    source_text: Option<&str>,
) -> Result<SemanticGraph, SemanticsError> {
    build_semantic_graph_with_place_resolver(syntax, source_text, |_| None)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
pub fn build_semantic_graph_with_dictionary(
    syntax: &TextSyntax,
    source_text: Option<&str>,
    dictionary: &Dictionary<'_>,
) -> Result<SemanticGraph, SemanticsError> {
    build_semantic_graph_with_dictionary_and_options(
        syntax,
        SemanticBuildOptions {
            source_text,
            story_time: false,
        },
        dictionary,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
pub fn build_semantic_graph_with_dictionary_and_options<'a>(
    syntax: &TextSyntax,
    options: SemanticBuildOptions<'a>,
    dictionary: &Dictionary<'_>,
) -> Result<SemanticGraph, SemanticsError> {
    build_semantic_graph_with_resolvers(
        syntax,
        options,
        |relation| dictionary_relation_place_count(dictionary, relation),
        |rafsi| {
            dictionary
                .lookup_rafsi(rafsi)
                .next()
                .map(|rafsi_match| rafsi_match.entry.word.to_owned())
        },
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
pub fn build_semantic_graph_with_place_resolver<F>(
    syntax: &TextSyntax,
    source_text: Option<&str>,
    relation_place_count: F,
) -> Result<SemanticGraph, SemanticsError>
where
    F: Fn(&str) -> Option<usize>,
{
    build_semantic_graph_with_resolvers(
        syntax,
        SemanticBuildOptions {
            source_text,
            story_time: false,
        },
        relation_place_count,
        |_| None,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
fn build_semantic_graph_with_resolvers<'a, F, R>(
    syntax: &TextSyntax,
    options: SemanticBuildOptions<'a>,
    relation_place_count: F,
    rafsi_source_word: R,
) -> Result<SemanticGraph, SemanticsError>
where
    F: Fn(&str) -> Option<usize>,
    R: Fn(&str) -> Option<String>,
{
    let analysis = analyze_references(syntax)?;
    let mut builder = GraphBuilder::new(
        &analysis,
        options,
        &relation_place_count,
        &rafsi_source_word,
    );
    builder.build_text(syntax)
}

#[requires(!relation.is_empty())]
#[ensures(true)]
pub fn dictionary_relation_place_count(
    dictionary: &Dictionary<'_>,
    relation: &str,
) -> Option<usize> {
    let normalized = normalize_lookup_query(relation);
    let entry = dictionary.lookup_word(&normalized)?;
    if !word_type_is_brivla_like(entry.word_type) {
        return None;
    }
    let keyword_count = (!entry.place_keywords.is_empty()).then_some(entry.place_keywords.len());
    let definition_count = dictionary_definition_place_count(entry.definition);
    keyword_count.max(definition_count)
}

#[requires(true)]
#[ensures(true)]
fn dictionary_definition_place_count(definition: &str) -> Option<usize> {
    let mut max_place = 0usize;
    let mut chars = definition.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '$' {
            if chars.next() != Some('x') || chars.next() != Some('_') {
                continue;
            }
            let braced = chars.peek() == Some(&'{');
            if braced {
                chars.next();
            }
            let mut digits = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    digits.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if braced && chars.next() != Some('}') {
                continue;
            }
            if chars.next() != Some('$') {
                continue;
            }
            if let Ok(place) = digits.parse::<usize>() {
                max_place = max_place.max(place);
            }
            continue;
        }
        if character != '⟨' && character != '<' {
            continue;
        }
        let mut digits = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_ascii_digit() {
                digits.push(next);
                chars.next();
            } else {
                break;
            }
        }
        let Some(closing) = chars.next() else {
            continue;
        };
        if (character == '⟨' && closing != '⟩') || (character == '<' && closing != '>') {
            continue;
        }
        if let Ok(place) = digits.parse::<usize>() {
            max_place = max_place.max(place);
        }
    }
    (max_place > 0).then_some(max_place)
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdCounters {
    utterance: usize,
    sequence: usize,
    eventuality: usize,
    referent: usize,
    parameter: usize,
    predication: usize,
    formula: usize,
    abstraction: usize,
    sign: usize,
    display: usize,
    math: usize,
    quantity: usize,
    relation: usize,
    question: usize,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct TanruFormulaForArgument {
    formula: SemanticObjectId,
    x1_argument: ArgumentValue,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct AlternativeArgument {
    argument: ArgumentValue,
    negated: bool,
}

impl AlternativeArgument {
    #[requires(true)]
    #[ensures(ret.negated == negated)]
    fn new(argument: ArgumentValue, negated: bool) -> Self {
        Self { argument, negated }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ModalAssignmentKey {
    sumti: RawSyntaxNodeId,
    tag: Option<RawSyntaxNodeId>,
}

#[invariant(!introduced_by.is_empty(), "sticky modal key must preserve its source marker")]
#[invariant(!relation.is_empty(), "sticky modal key must preserve its source relation")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StickyModalKey {
    introduced_by: String,
    relation: String,
}

impl StickyModalKey {
    #[requires(!modal_argument.introduced_by.is_empty())]
    #[requires(!modal_argument.relation.is_empty())]
    #[ensures(ret.introduced_by == modal_argument.introduced_by)]
    fn for_modal_argument(modal_argument: &ModalArgument) -> Self {
        Self::from_data(data!(StickyModalKey {
            introduced_by: modal_argument.introduced_by.clone(),
            relation: modal_argument.relation.clone(),
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct EventTenseModifier<'tree> {
    order: usize,
    tense_modal: &'tree TenseModalSyntax,
    anchor: Option<SemanticObjectId>,
    magnitude: Option<AnchorMagnitude>,
    consumed_terms: Vec<RawSyntaxNodeId>,
}

#[invariant(true)]
#[derive(Debug, Clone, Default)]
struct GovernedTermset {
    anchor: Option<SemanticObjectId>,
    magnitude: Option<AnchorMagnitude>,
    consumed_terms: Vec<RawSyntaxNodeId>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct DescriptionAbstraction<'tree> {
    abstraction: &'tree AbstractionSyntax,
    output_sort: SemanticSort,
    link_relation: &'static str,
}

#[invariant(true)]
#[derive(Debug, Clone, Default)]
struct EventModifierApplication {
    temporal_modifier: bool,
    sticky_temporal_modifier: bool,
    consumed_terms: HashSet<RawSyntaxNodeId>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct TemporalPathRelation {
    relation: String,
    introduced_by: String,
    distance: Option<String>,
    scalar_negation: Option<ScalarNegation>,
    motion: Option<SpatialMotion>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct BoundSelbriTanruPair<'tree> {
    leading: &'tree SelbriSyntax,
    trailing: &'tree SelbriSyntax,
}

#[invariant(::Explicit => true)]
#[invariant(::Bare => true)]
#[derive(Debug, Clone, Copy)]
enum DaSeriesScopeSource<'tree> {
    Explicit {
        quantified_sumti: &'tree SumtiSyntax,
        quantifier: &'tree QuantifierSyntax,
    },
    Bare {
        da_series_sumti: &'tree SumtiSyntax,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct RelationVariableScopeSource<'tree> {
    quantified_sumti: &'tree SumtiSyntax,
    selbri: &'tree SelbriSyntax,
    quantifier: &'tree QuantifierSyntax,
}

#[invariant(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))]
#[derive(Debug, Clone)]
struct QuantifiedProSumtiScope {
    variable: SemanticObjectId,
    quantity: Option<SemanticObjectId>,
    operator: FormulaOperator,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(variable.object_kind() == crate::model::SemanticObjectKind::Parameter)]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))]
#[derive(Debug, Clone)]
struct QuantifiedRelationVariableScope {
    variable: SemanticObjectId,
    quantity: Option<SemanticObjectId>,
    operator: FormulaOperator,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(::Negation => true)]
#[invariant(::Quantifier => true)]
#[invariant(::RelationQuantifier => true)]
#[derive(Debug, Clone)]
enum PrenexFormulaScope {
    Negation {
        source: Option<crate::model::SemanticSource>,
    },
    Quantifier {
        scope: QuantifiedProSumtiScope,
        restrictions: Vec<SemanticObjectId>,
    },
    RelationQuantifier {
        scope: QuantifiedRelationVariableScope,
    },
}

#[invariant(focus.object_kind() == crate::model::SemanticObjectKind::Parameter || focus.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(presupposed_answer.is_none_or(|answer| answer.object_kind() == crate::model::SemanticObjectKind::Referent || answer.object_kind() == crate::model::SemanticObjectKind::Parameter))]
#[invariant(slots.iter().all(|slot| slot.parameter.object_kind() == crate::model::SemanticObjectKind::Parameter))]
#[derive(Debug, Clone)]
struct IndirectQuestionFocus {
    focus: SemanticObjectId,
    presupposed_answer: Option<SemanticObjectId>,
    slots: Vec<QuestionSlot>,
    kind: QuestionKind,
    domain: SemanticSort,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct IndicatorPart {
    cmavo: Cmavo,
    nai: bool,
    tokens: Vec<Token>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct IndicatorDisplayDraft {
    family: DisplayedContentFamily,
    relation: String,
    polarity: DisplayedContentPolarity,
    assertion_effect: DisplayedContentAssertionEffect,
    intensity: Option<String>,
    phase: Option<String>,
    modifiers: Vec<DisplayedContentModifier>,
    question: bool,
    empathy: bool,
    source_tokens: Vec<Token>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct IndicatorBaseSpec {
    family: DisplayedContentFamily,
    relation: &'static str,
    assertion_effect: DisplayedContentAssertionEffect,
}

impl IdCounters {
    #[requires(true)]
    #[ensures(ret.utterance == 1)]
    #[ensures(ret.eventuality == 0)]
    fn new() -> Self {
        Self {
            utterance: 1,
            sequence: 1,
            eventuality: 0,
            referent: 1,
            parameter: 1,
            predication: 1,
            formula: 1,
            abstraction: 1,
            sign: 1,
            display: 1,
            math: 1,
            quantity: 1,
            relation: 1,
            question: 1,
        }
    }
}

#[invariant(true)]
#[derive(Debug)]
struct GraphBuilder<'analysis, 'tree, 'resolver, F, R>
where
    F: Fn(&str) -> Option<usize>,
    R: Fn(&str) -> Option<String>,
{
    analysis: &'analysis ReferenceAnalysis<'tree>,
    options: SemanticBuildOptions<'analysis>,
    relation_place_count: &'resolver F,
    rafsi_source_word: &'resolver R,
    objects: BTreeMap<SemanticObjectId, SemanticObject>,
    counters: IdCounters,
    sumti_objects: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    math_variable_referents: HashMap<String, SemanticObjectId>,
    sumti_quantities: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    relation_question_parameters: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    relation_variable_parameters: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    modal_assignment_arguments: HashMap<ModalAssignmentKey, ModalArgument>,
    sticky_modal_arguments: BTreeMap<StickyModalKey, ModalArgument>,
    sticky_time_path: Vec<TemporalPathStep>,
    sticky_space_path: Vec<TemporalPathStep>,
    utterance_objects: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    content_eventualities: HashMap<SemanticObjectId, SemanticObjectId>,
    parameter_slots: Vec<QuestionSlot>,
    indirect_question_stack: Vec<Vec<IndirectQuestionFocus>>,
    abstraction_parameter_stack: Vec<Vec<SemanticObjectId>>,
    temporal_context_stack: Vec<SemanticObjectId>,
    story_time_anchor: Option<SemanticObjectId>,
    pending_asides: Vec<SemanticObjectId>,
    current_utterance_anchor: Option<SemanticObjectId>,
}

impl<'analysis, 'tree, 'resolver, F, R> GraphBuilder<'analysis, 'tree, 'resolver, F, R>
where
    F: Fn(&str) -> Option<usize>,
    R: Fn(&str) -> Option<String>,
{
    #[requires(true)]
    #[ensures(ret.objects.contains_key(&SemanticObjectId::speaker()))]
    fn new(
        analysis: &'analysis ReferenceAnalysis<'tree>,
        options: SemanticBuildOptions<'analysis>,
        relation_place_count: &'resolver F,
        rafsi_source_word: &'resolver R,
    ) -> Self {
        let mut builder = Self {
            analysis,
            options,
            relation_place_count,
            rafsi_source_word,
            objects: BTreeMap::new(),
            counters: IdCounters::new(),
            sumti_objects: HashMap::new(),
            math_variable_referents: HashMap::new(),
            sumti_quantities: HashMap::new(),
            relation_question_parameters: HashMap::new(),
            relation_variable_parameters: HashMap::new(),
            modal_assignment_arguments: HashMap::new(),
            sticky_modal_arguments: BTreeMap::new(),
            sticky_time_path: Vec::new(),
            sticky_space_path: Vec::new(),
            utterance_objects: HashMap::new(),
            content_eventualities: HashMap::new(),
            parameter_slots: Vec::new(),
            indirect_question_stack: Vec::new(),
            abstraction_parameter_stack: Vec::new(),
            temporal_context_stack: Vec::new(),
            story_time_anchor: None,
            pending_asides: Vec::new(),
            current_utterance_anchor: None,
        };
        builder.insert_deictic_referents();
        builder
    }

    #[requires(true)]
    #[ensures(self.objects.contains_key(&SemanticObjectId::speaker()))]
    fn insert_deictic_referents(&mut self) {
        self.insert_known(
            SemanticObjectId::speaker(),
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Speaker),
                None,
                None,
                None,
                Vec::new(),
            ),
        );
        self.insert_known(
            SemanticObjectId::addressee(),
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Audience),
                None,
                None,
                None,
                Vec::new(),
            ),
        );
        self.insert_known(
            SemanticObjectId::speech_time(),
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Eventuality,
                Some(IndexicalKind::SpeechTime),
                None,
                None,
                None,
                Vec::new(),
            ),
        );
        self.insert_known(
            SemanticObjectId::here(),
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Here),
                None,
                None,
                None,
                Vec::new(),
            ),
        );
    }

    #[requires(id.object_kind() == object.object_kind())]
    #[ensures(self.objects.contains_key(&id))]
    fn insert_known(&mut self, id: SemanticObjectId, object: SemanticObject) {
        self.objects.insert(id, object);
    }

    #[requires(id.object_kind() == object.object_kind())]
    #[ensures(ret.as_ref().is_ok_and(|_| self.objects.contains_key(&id)) || ret.is_err())]
    fn insert(
        &mut self,
        id: SemanticObjectId,
        object: SemanticObject,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if self.objects.insert(id, object).is_some() {
            return Err(SemanticsError::duplicate_object(id));
        }
        Ok(id)
    }

    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Eventuality)]
    #[ensures(self.temporal_context_stack.len() == old(self.temporal_context_stack.len()))]
    fn with_temporal_context<T>(
        &mut self,
        anchor: SemanticObjectId,
        build: impl FnOnce(&mut Self) -> Result<T, SemanticsError>,
    ) -> Result<T, SemanticsError> {
        self.temporal_context_stack.push(anchor);
        let result = build(self);
        self.temporal_context_stack.pop();
        result
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Eventuality))]
    fn current_temporal_context(&self) -> Option<SemanticObjectId> {
        self.temporal_context_stack.last().copied()
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_utterance(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::utterance(self.counters.utterance);
        self.counters.utterance += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_sequence(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::sequence(self.counters.sequence);
        self.counters.sequence += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_eventuality(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::eventuality(self.counters.eventuality);
        self.counters.eventuality += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_referent(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent(self.counters.referent);
        self.counters.referent += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_parameter(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::parameter(self.counters.parameter);
        self.counters.parameter += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_predication(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::predication(self.counters.predication);
        self.counters.predication += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_formula(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::formula(self.counters.formula);
        self.counters.formula += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_abstraction(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::abstraction(self.counters.abstraction);
        self.counters.abstraction += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_sign(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::sign(self.counters.sign);
        self.counters.sign += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_display(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::displayed_content(self.counters.display);
        self.counters.display += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_math(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::math_expression(self.counters.math);
        self.counters.math += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_quantity(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::quantity(self.counters.quantity);
        self.counters.quantity += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::RelationMetadata)]
    fn next_relation_metadata(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::relation_metadata(self.counters.relation);
        self.counters.relation += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_question(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::question(self.counters.question);
        self.counters.question += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_node(
        &self,
        node: RawSyntaxNodeId,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let metadata = self.analysis.syntax_index.metadata(node)?;
        source_from_spans(
            &metadata.source_spans,
            self.options.source_text,
            Some(construct),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_quantifier(
        &self,
        quantifier: &QuantifierSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        quantifier.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_mekso(
        &self,
        expression: &'tree MeksoSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        expression.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_selbri(
        &self,
        selbri: &'tree SelbriSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_tanru_unit(
        &self,
        unit: &'tree TanruUnitSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.analysis
            .syntax_index
            .tanru_unit_node_id(unit)
            .and_then(|node| self.source_for_node(node.0, construct))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_description(
        &self,
        description: &'tree DescriptionSyntax,
        fallback: RawSyntaxNodeId,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        if let Some(descriptor) = &description.description {
            descriptor.visit_words(&mut |token| {
                spans.extend(token.source_spans().into_iter().cloned());
            });
        }
        for element in description.tail_elements.iter().filter(|element| {
            description.description.is_some()
                || !matches!(
                    element.as_data(),
                    data!(
                        jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailQuantifier(
                            ..
                        )
                    )
                )
        }) {
            element.visit_words(&mut |token| {
                spans.extend(token.source_spans().into_iter().cloned());
            });
        }
        if let Some(selbri) = &description.selbri {
            selbri.visit_words(&mut |token| {
                spans.extend(token.source_spans().into_iter().cloned());
            });
        }
        for relative_clause in &description.relative_clauses {
            relative_clause.visit_words(&mut |token| {
                spans.extend(token.source_spans().into_iter().cloned());
            });
        }
        if let Some(ku) = &description.ku {
            ku.visit_words(&mut |token| {
                spans.extend(token.source_spans().into_iter().cloned());
            });
        }
        source_from_spans(&spans, self.options.source_text, Some(construct))
            .or_else(|| self.source_for_node(fallback, construct))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_subbridi(
        &self,
        subbridi: &'tree SubbridiSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        match subbridi.as_data() {
            data!(SubbridiSyntax::Bridi(bridi)) => self
                .analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, construct)),
            data!(SubbridiSyntax::Prenex { inner_subbridi, .. }) => {
                self.source_for_subbridi(inner_subbridi, construct)
            }
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_sumti(
        &self,
        sumti: &'tree SumtiSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.analysis
            .syntax_index
            .sumti_node_id(sumti)
            .and_then(|node| self.source_for_node(node.0, construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_abstraction(
        &self,
        abstraction: &'tree AbstractionSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.analysis
            .syntax_index
            .abstraction_node_id(abstraction)
            .and_then(|node| self.source_for_node(node.0, construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_sumti_association_phrase(
        &self,
        phrase: &SumtiAssociationPhraseSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        phrase.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_term(
        &self,
        term: &TermSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        term.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_tense_modal(
        &self,
        tense_modal: &TenseModalSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        tense_modal.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_free_modifier(
        &self,
        free_modifier: &FreeModifierSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        free_modifier.visit_words(&mut |token| {
            spans.extend(token.source_spans().into_iter().cloned());
        });
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!tokens.is_empty())]
    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_tokens(
        &self,
        tokens: &[Token],
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let spans = tokens
            .iter()
            .flat_map(|token| token.source_spans().into_iter().cloned())
            .collect::<Vec<_>>();
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_token(
        &self,
        token: &Token,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.source_for_tokens(std::slice::from_ref(token), construct)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
    fn build_text(&mut self, text: &'tree TextSyntax) -> Result<SemanticGraph, SemanticsError> {
        let truth_question = text
            .leading_indicators
            .iter()
            .any(|indicator| indicator.indicator.cmavo() == Some(Cmavo::Xu));
        let mut leading_asides = self.build_vocative_asides(&text.leading_free_modifiers)?;
        let mut items = Vec::new();
        if let Some(leading_names) = self.build_leading_cmevla_utterance(text)? {
            items.push(leading_names);
        }
        let mut truth_question_pending = truth_question;
        let mut leading_reciprocity_attached = false;
        let mut leading_indicators_attached = false;
        for paragraph in &text.paragraphs {
            let mut paragraph_asides = self.build_vocative_asides(&paragraph.free_modifiers)?;
            let first_paragraph_item = items.len();
            let mut paragraph_reciprocity_attached = false;
            for statement in &paragraph.statements {
                if let Some(statement) = statement.statement.as_deref() {
                    self.reserve_forward_reference_utterance_for_statement(statement);
                }
            }
            for statement in &paragraph.statements {
                let statement_truth_question =
                    truth_question_pending && statement.statement.is_some();
                if let Some(statement_id) =
                    self.build_paragraph_statement(statement, statement_truth_question)?
                {
                    if let Some(statement) = statement.statement.as_deref() {
                        if !leading_reciprocity_attached {
                            self.attach_statement_reciprocity_to_discourse_item(
                                statement_id,
                                statement,
                                &text.leading_free_modifiers,
                            )?;
                            leading_reciprocity_attached = true;
                        }
                        if !paragraph_reciprocity_attached {
                            self.attach_statement_reciprocity_to_discourse_item(
                                statement_id,
                                statement,
                                &paragraph.free_modifiers,
                            )?;
                            paragraph_reciprocity_attached = true;
                        }
                        if !leading_indicators_attached {
                            self.attach_leading_indicators_to_discourse_item(
                                statement_id,
                                &text.leading_indicators,
                                statement_truth_question,
                            )?;
                            leading_indicators_attached = true;
                        }
                    }
                    items.push(statement_id);
                    if statement_truth_question {
                        truth_question_pending = false;
                    }
                } else if let Some(previous_item) = items.last().copied().filter(|item| {
                    item.object_kind() == crate::model::SemanticObjectKind::Utterance
                }) {
                    self.attach_statement_separator_indicators_to_discourse_item(
                        previous_item,
                        statement,
                    )?;
                }
            }
            if !paragraph_asides.is_empty() {
                if let Some(first_item) = items.get(first_paragraph_item).copied() {
                    self.add_asides_to_discourse_item(
                        first_item,
                        std::mem::take(&mut paragraph_asides),
                    );
                } else {
                    items.extend(paragraph_asides);
                }
            }
        }
        if !leading_asides.is_empty() {
            if let Some(first_item) = items.first().copied() {
                self.add_asides_to_discourse_item(first_item, std::mem::take(&mut leading_asides));
            } else {
                items.extend(leading_asides);
            }
        }
        if !leading_indicators_attached
            && !text.leading_indicators.is_empty()
            && let Some(first_item) = items.first().copied()
        {
            self.attach_leading_indicators_to_discourse_item(
                first_item,
                &text.leading_indicators,
                truth_question,
            )?;
        }
        if items.is_empty() && !text.leading_indicators.is_empty() {
            items.push(self.build_standalone_indicator_utterance(text)?);
        }
        if items.is_empty()
            && let Some(connective) = &text.leading_connective
        {
            items.push(self.build_standalone_connective_utterance(text, connective)?);
        }
        let root = if let [single] = items.as_slice() {
            *single
        } else {
            let id = self.next_sequence();
            let source = self
                .analysis
                .syntax_index
                .text_node_id(text)
                .and_then(|node| self.source_for_node(node.0, "text"));
            self.insert(
                id,
                SemanticObject::sequence(
                    items,
                    SequenceRelation::SameTopicContinuation,
                    source,
                    Vec::new(),
                ),
            )?
        };
        SemanticGraph::new(root, std::mem::take(&mut self.objects))
            .map_err(SemanticsError::invalid_graph)
    }

    #[requires(!text.leading_indicators.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    fn build_standalone_indicator_utterance(
        &mut self,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let utterance = self.next_utterance();
        let parts = text
            .leading_indicators
            .iter()
            .flat_map(indicator_parts_for_indicator)
            .collect::<Vec<_>>();
        let source_tokens = parts
            .iter()
            .flat_map(|part| part.tokens.iter().cloned())
            .collect::<Vec<_>>();
        let sign = self.next_sign();
        let source = self.source_for_tokens(&source_tokens, "indicator-expression");
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Text,
                source_tokens
                    .iter()
                    .map(token_text)
                    .collect::<Vec<_>>()
                    .join(" "),
                source.clone(),
                Vec::new(),
            ),
        )?;
        let mut displays = Vec::new();
        for draft in indicator_display_drafts(parts) {
            displays.push(self.insert_indicator_display(draft, sign, utterance, "indicator")?);
        }
        let content = if let [display] = displays.as_slice() {
            *display
        } else {
            let sequence = self.next_sequence();
            self.insert(
                sequence,
                SemanticObject::sequence(
                    displays,
                    SequenceRelation::SameTopicContinuation,
                    source.clone(),
                    Vec::new(),
                ),
            )?
        };
        self.build_utterance(
            UtteranceForce::Mention,
            Some(content),
            self.analysis
                .syntax_index
                .text_node_id(text)
                .and_then(|node| self.source_for_node(node.0, "indicator-utterance")),
            Vec::new(),
            Some(utterance),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    fn build_standalone_connective_utterance(
        &mut self,
        text: &'tree TextSyntax,
        connective: &'tree ConnectiveSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sign = self.build_connective_sign(None, connective, "connective-expression")?;
        self.build_utterance(
            UtteranceForce::Mention,
            Some(sign),
            self.analysis
                .syntax_index
                .text_node_id(text)
                .and_then(|node| self.source_for_node(node.0, "connective-utterance")),
            Vec::new(),
            None,
        )
    }

    #[requires(!source_construct.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sign) || ret.is_err())]
    fn build_connective_sign(
        &mut self,
        prefix: Option<&Token>,
        connective: &ConnectiveSyntax,
        source_construct: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut tokens = Vec::new();
        if let Some(prefix) = prefix {
            tokens.push(prefix.clone());
        }
        connective.visit_words(&mut |token| tokens.push(token.clone()));
        let id = self.next_sign();
        self.insert(
            id,
            SemanticObject::text_sign(
                SignKind::Connective,
                tokens.iter().map(token_text).collect::<Vec<_>>().join(" "),
                self.source_for_tokens(&tokens, source_construct),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_leading_cmevla_utterance(
        &mut self,
        text: &'tree TextSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if text.leading_cmevla.is_empty() {
            return Ok(None);
        }
        let sign = self.next_sign();
        let source = self.source_for_tokens(&text.leading_cmevla, "name-words");
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Text,
                token_vec_text(&text.leading_cmevla),
                source.clone(),
                Vec::new(),
            ),
        )?;
        self.build_utterance(
            UtteranceForce::Mention,
            Some(sign),
            source,
            Vec::new(),
            None,
        )
        .map(Some)
    }

    #[requires(true)]
    #[ensures(true)]
    fn build_paragraph_statement(
        &mut self,
        paragraph_statement: &'tree ParagraphStatementSyntax,
        truth_question: bool,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut asides = self.build_vocative_asides(&paragraph_statement.free_modifiers)?;
        let Some(statement) = paragraph_statement.statement.as_deref() else {
            return self.build_standalone_asides(asides);
        };
        let statement_id = self.build_statement(statement, truth_question)?;
        self.attach_statement_reciprocity_to_discourse_item(
            statement_id,
            statement,
            &paragraph_statement.free_modifiers,
        )?;
        if let Some(node) = self.analysis.syntax_index.statement_node_id(statement) {
            self.utterance_objects.insert(node.0, statement_id);
        }
        if !asides.is_empty() {
            self.add_asides_to_discourse_item(statement_id, std::mem::take(&mut asides));
        }
        Ok(Some(statement_id))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_standalone_asides(
        &mut self,
        asides: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match asides.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            _ => {
                let id = self.next_sequence();
                self.insert(
                    id,
                    SemanticObject::sequence(
                        asides,
                        SequenceRelation::SameTopicContinuation,
                        None,
                        Vec::new(),
                    ),
                )
                .map(Some)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_vocative_asides(
        &mut self,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut asides = Vec::new();
        for free_modifier in free_modifiers {
            if let Some(aside) = self.build_vocative_aside(free_modifier)? {
                asides.push(aside);
            }
        }
        Ok(asides)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn queue_vocative_asides(
        &mut self,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        let asides = self.build_vocative_asides(free_modifiers)?;
        self.pending_asides.extend(asides);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_vocative_aside(
        &mut self,
        free_modifier: &'tree FreeModifierSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let data!(FreeModifierSyntax::Vocative {
            vocative_markers,
            sumti,
            ..
        }) = free_modifier.as_data()
        else {
            return Ok(None);
        };
        let vocative_kind = vocative_kind_for_markers(vocative_markers);
        let previous_slots = std::mem::take(&mut self.parameter_slots);
        let previous_pending_asides = std::mem::take(&mut self.pending_asides);
        let addressed_or_identified = if let Some(sumti) = sumti.as_deref() {
            let referent = self.build_sumti_referent(sumti)?;
            if referent.object_kind() == crate::model::SemanticObjectKind::Referent {
                self.attach_relative_clauses_to_referent(referent, sumti)?;
            }
            referent
        } else {
            SemanticObjectId::addressee()
        };
        let nested_asides = std::mem::replace(&mut self.pending_asides, previous_pending_asides);
        let slots = std::mem::replace(&mut self.parameter_slots, previous_slots);
        let content = if slots.is_empty() {
            None
        } else {
            Some(self.build_vocative_question_content(
                addressed_or_identified,
                slots,
                self.source_for_free_modifier(free_modifier, "vocative-question"),
            )?)
        };
        let diagnostics = if addressed_or_identified.object_kind()
            == crate::model::SemanticObjectKind::Referent
            || addressed_or_identified.object_kind() == crate::model::SemanticObjectKind::Parameter
        {
            Vec::new()
        } else {
            vec![diagnostic(
                "vocative target is not referent-valued; audience remains contextual",
            )]
        };
        let id = self.build_utterance(
            UtteranceForce::Vocative,
            content,
            self.source_for_free_modifier(free_modifier, "vocative"),
            diagnostics,
            None,
        )?;
        if addressed_or_identified.object_kind() == crate::model::SemanticObjectKind::Referent {
            if vocative_kind == "selfIdentification" {
                self.set_referent_target(addressed_or_identified, SemanticObjectId::speaker());
            } else {
                self.set_utterance_audience(id, addressed_or_identified);
                self.set_referent_target(addressed_or_identified, SemanticObjectId::addressee());
            }
        }
        self.add_utterance_asides(id, nested_asides);
        self.set_vocative_kind(id, vocative_kind);
        Ok(Some(id))
    }

    #[requires(!slots.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Question) || ret.is_err())]
    fn build_vocative_question_content(
        &mut self,
        target: SemanticObjectId,
        slots: Vec<QuestionSlot>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let predication = self.next_predication();
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(target, None));
        self.insert(
            predication,
            SemanticObject::predication(
                "vocativeTarget".to_owned(),
                None,
                arguments,
                PredicationMode::Performative,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source.clone(), Vec::new()),
        )?;
        let (kind, domain) = self.question_kind_and_domain(false, &slots);
        let question = self.next_question();
        self.insert(
            question,
            SemanticObject::question(kind, QuestionMode::Direct, domain, formula, slots, source),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_statement(
        &mut self,
        statement: &'tree StatementSyntax,
        truth_question: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match statement.as_data() {
            data!(StatementSyntax::Bridi(bridi)) => {
                let reserved = self.reserve_utterance_for_statement(statement);
                self.build_bridi_utterance(bridi, truth_question, reserved)
            }
            data!(StatementSyntax::TextGroup {
                tense_modal,
                text,
                ..
            }) => {
                let reserved = self.reserve_utterance_for_statement(statement);
                let source = self
                    .analysis
                    .syntax_index
                    .statement_node_id(statement)
                    .and_then(|node| self.source_for_node(node.0, "statement"));
                let utterance = self.build_utterance(
                    UtteranceForce::Parenthetical,
                    None,
                    source,
                    Vec::new(),
                    reserved,
                )?;
                let nested = self.build_text_group_sequence(text)?;
                let nested = self.ensure_text_group_sequence_content(nested, text)?;
                if let Some(tense_modal) = tense_modal
                    && tense_relation_spec_for_tense_modal(tense_modal).is_none()
                    && let Some(modal_argument) =
                        self.modal_argument_for_tense_modal(tense_modal, "modal-argument")?
                {
                    self.attach_modal_argument_to_discourse_item(nested, &modal_argument)?;
                }
                let object = self.objects.get_mut(&utterance).ok_or_else(|| {
                    SemanticsError::invalid_graph(format!(
                        "missing text-group utterance {utterance}"
                    ))
                })?;
                object.content = Some(nested);
                object.diagnostics.push(diagnostic(
                    "tu'e text group is represented as a nested discourse sequence",
                ));
                Ok(utterance)
            }
            data!(StatementSyntax::Prenex {
                prenex_terms,
                inner_statement,
                ..
            }) => {
                let id = self.build_statement(inner_statement, truth_question)?;
                self.apply_prenex_terms_to_discourse_item(id, prenex_terms)?;
                Ok(id)
            }
            data!(StatementSyntax::StatementConnection {
                leading_statement,
                trailing_statement,
                connective,
                ..
            })
            | data!(StatementSyntax::PreposedIStatementConnection {
                leading_statement,
                trailing_statement,
                connective,
                ..
            }) => {
                let first = self.build_statement(leading_statement, truth_question)?;
                let second = self.build_statement(trailing_statement, false)?;
                let source = self
                    .analysis
                    .syntax_index
                    .statement_node_id(statement)
                    .and_then(|node| self.source_for_node(node.0, "statement-connection"));
                let mut diagnostics = Vec::new();
                let content = if connective_has_logical_component(connective) {
                    match self.build_statement_logical_connection_formula(
                        first,
                        second,
                        connective,
                        source.clone(),
                    )? {
                        Some(formula) => Some(formula),
                        None => {
                            diagnostics.push(diagnostic(
                                "logical statement connection could not find formula-bearing statements to connect",
                            ));
                            None
                        }
                    }
                } else {
                    None
                };
                let mut connection_claims = Vec::new();
                let trailing_text_group_tense = text_group_tense_modal(trailing_statement);
                let modal_connection_spec =
                    modal_statement_connection_spec(connective).or_else(|| {
                        trailing_text_group_tense
                            .and_then(modal_statement_connection_spec_for_tense_modal)
                    });
                if let Some(spec) = modal_connection_spec {
                    let claim_source = trailing_text_group_tense
                        .and_then(|tense_modal| {
                            self.source_for_tense_modal(tense_modal, "statement-connection-claim")
                        })
                        .or_else(|| source.clone());
                    match self.build_modal_statement_connection_claim(
                        first,
                        second,
                        &spec,
                        claim_source,
                    )? {
                        Some(claim) => connection_claims.push(claim),
                        None => {
                            diagnostics.push(diagnostic(
                                "modal statement connection could not find formula-bearing bridi events to relate",
                            ));
                        }
                    }
                }
                let id = self.next_sequence();
                let mut sequence = SemanticObject::sequence_with_connection_claims(
                    vec![first, second],
                    SequenceRelation::SameTopicContinuation,
                    connection_claims,
                    source,
                    diagnostics,
                );
                sequence.content = content;
                self.insert(id, sequence)
            }
            data!(StatementSyntax::Iau {
                inner_statement,
                ..
            }) => {
                let id = self.build_statement(inner_statement, truth_question)?;
                self.add_object_diagnostic(
                    id,
                    diagnostic("iau reset is recorded as a diagnostic; discourse reset semantics are pending"),
                );
                Ok(id)
            }
            data!(StatementSyntax::ExperimentalBridiContinuation {
                leading_statement,
                ..
            }) => {
                let id = self.build_statement(leading_statement, truth_question)?;
                self.add_object_diagnostic(
                    id,
                    diagnostic("experimental bridi continuation is not fully lowered yet"),
                );
                Ok(id)
            }
            data!(StatementSyntax::Fragment(fragment)) => {
                let reserved = self.reserve_utterance_for_statement(statement);
                self.build_fragment_utterance(statement, fragment, reserved)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance))]
    fn reserve_utterance_for_statement(
        &mut self,
        statement: &'tree StatementSyntax,
    ) -> Option<SemanticObjectId> {
        let raw = self.analysis.syntax_index.statement_node_id(statement)?.0;
        if let Some(id) = self.utterance_objects.get(&raw) {
            return Some(*id);
        }
        let id = self.next_utterance();
        self.utterance_objects.insert(raw, id);
        Some(id)
    }

    #[requires(true)]
    #[ensures(true)]
    fn reserve_forward_reference_utterance_for_statement(
        &mut self,
        statement: &'tree StatementSyntax,
    ) {
        match statement.as_data() {
            data!(StatementSyntax::Bridi(_))
            | data!(StatementSyntax::TextGroup { .. })
            | data!(StatementSyntax::Fragment(_)) => {
                let _ = self.reserve_utterance_for_statement(statement);
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance))]
    fn reserve_utterance_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Option<SemanticObjectId> {
        let raw = self.analysis.syntax_index.bridi_node_id(bridi)?.0;
        if let Some(id) = self.utterance_objects.get(&raw) {
            return Some(*id);
        }
        let id = self.next_utterance();
        self.utterance_objects.insert(raw, id);
        Some(id)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_fragment_utterance(
        &mut self,
        statement: &'tree StatementSyntax,
        fragment: &'tree FragmentSyntax,
        reserved_utterance: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self
            .analysis
            .syntax_index
            .statement_node_id(statement)
            .and_then(|node| self.source_for_node(node.0, "fragment"));
        let anchor = reserved_utterance
            .unwrap_or_else(|| SemanticObjectId::utterance(self.counters.utterance));
        let previous_anchor = self.current_utterance_anchor.replace(anchor);
        let content = self.build_fragment_content(fragment);
        self.current_utterance_anchor = previous_anchor;
        if let Some(content) = content? {
            return self.build_utterance(
                UtteranceForce::Mention,
                Some(content),
                source,
                Vec::new(),
                reserved_utterance,
            );
        }
        self.build_utterance(
            UtteranceForce::Mention,
            None,
            source,
            vec![diagnostic("fragment has no truth-bearing semantic formula")],
            reserved_utterance,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_fragment_content(
        &mut self,
        fragment: &'tree FragmentSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match fragment.as_data() {
            data!(FragmentSyntax::Ek(connective))
            | data!(FragmentSyntax::BridiTailConnective(connective)) => self
                .build_connective_sign(None, connective, "connective-fragment")
                .map(Some),
            data!(FragmentSyntax::BridiConnective { i, connective }) => self
                .build_connective_sign(Some(i), connective, "connective-fragment")
                .map(Some),
            data!(FragmentSyntax::Terms { terms, .. }) if terms.len() == 1 => {
                self.build_fragment_term_content(&terms[0])
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_fragment_term_content(
        &mut self,
        term: &'tree TermSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match term.as_data() {
            data!(TermSyntax::TaggedSumti {
                tense_modal: Some(tense_modal),
                sumti
            }) => self
                .build_tense_modal_fragment_content(tense_modal, sumti)
                .map(Some),
            data!(TermSyntax::Sumti(sumti)) | data!(TermSyntax::PlaceTaggedSumti { sumti, .. })
                if lerfu_string_sumti_letters(sumti).is_some() =>
            {
                self.build_letteral_sign_for_sumti(sumti).map(Some)
            }
            data!(TermSyntax::Sumti(sumti)) | data!(TermSyntax::PlaceTaggedSumti { sumti, .. }) => {
                let referent = self.build_sumti_referent(sumti)?;
                if referent.object_kind() == crate::model::SemanticObjectKind::Referent {
                    self.attach_relative_clauses_to_referent(referent, sumti)?;
                }
                Ok(Some(referent))
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Eventuality) || ret.is_err())]
    fn build_tense_modal_fragment_content(
        &mut self,
        tense_modal: &'tree TenseModalSyntax,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_eventuality();
        let mut event = SemanticObject::eventuality(
            EventualityClass::Event,
            None,
            self.source_for_tense_modal(tense_modal, "tense-modal-fragment"),
        );
        if tense_modal_has_event_modifier(tense_modal) {
            let anchor = if sumti_is_elided(sumti) {
                None
            } else {
                Some(self.build_sumti_referent(sumti)?)
            };
            apply_tense_modal_event_modifiers_to_event_with_anchor(tense_modal, &mut event, anchor);
            if let Some(parameter) =
                self.build_tense_question_parameter_for_tense_modal(tense_modal)?
            {
                event.tense_modal = Some(parameter);
            }
        } else if let Some((introduced_by, relation, visible_place)) =
            modal_relation_spec_for_tense_modal(tense_modal)
        {
            let argument = self.build_argument_for_sumti(sumti)?;
            let arguments = self.modal_argument_map_for_visible_place(
                argument,
                visible_place,
                self.place_count_for_relation(&relation),
            )?;
            event
                .modal_arguments
                .push(self.modal_argument_with_tense_modal_modifiers(
                    tense_modal,
                    relation,
                    introduced_by,
                    arguments,
                    modal_negation_for_tense_modal(tense_modal),
                    modal_scalar_negation_for_tense_modal(tense_modal),
                    "modal-fragment",
                ));
        } else {
            event.diagnostics.push(diagnostic(
                "tense/modal fragment has no implemented semantic value",
            ));
        }
        self.insert(id, event)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_text_group_sequence(
        &mut self,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut nested = GraphBuilder::new(
            self.analysis,
            self.options,
            self.relation_place_count,
            self.rafsi_source_word,
        );
        nested.counters = self.counters;
        nested.objects = std::mem::take(&mut self.objects);
        nested.utterance_objects = std::mem::take(&mut self.utterance_objects);
        nested.relation_question_parameters =
            std::mem::take(&mut self.relation_question_parameters);
        nested.modal_assignment_arguments = std::mem::take(&mut self.modal_assignment_arguments);
        nested.sticky_modal_arguments = std::mem::take(&mut self.sticky_modal_arguments);
        nested.sticky_time_path = std::mem::take(&mut self.sticky_time_path);
        let graph = nested.build_text(text)?;
        self.counters = nested.counters;
        self.objects = graph.objects;
        self.utterance_objects = nested.utterance_objects;
        self.relation_question_parameters = nested.relation_question_parameters;
        self.modal_assignment_arguments = nested.modal_assignment_arguments;
        self.sticky_modal_arguments = nested.sticky_modal_arguments;
        self.sticky_time_path = nested.sticky_time_path;
        Ok(graph.root)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    fn ensure_text_group_sequence_content(
        &mut self,
        item: SemanticObjectId,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            return Ok(item);
        }
        let sequence = self.next_sequence();
        self.insert(
            sequence,
            SemanticObject::sequence(
                vec![item],
                SequenceRelation::SameTopicContinuation,
                self.analysis
                    .syntax_index
                    .text_node_id(text)
                    .and_then(|node| self.source_for_node(node.0, "text-group-sequence")),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_utterance(
        &mut self,
        bridi: &'tree BridiSyntax,
        truth_question: bool,
        reserved_utterance: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let reserved_utterance =
            reserved_utterance.or_else(|| self.reserve_utterance_for_bridi(bridi));
        let previous_anchor = self.current_utterance_anchor.replace(
            reserved_utterance
                .unwrap_or_else(|| SemanticObjectId::utterance(self.counters.utterance)),
        );
        let previous_slots = std::mem::take(&mut self.parameter_slots);
        let previous_asides = std::mem::take(&mut self.pending_asides);
        let formula = self.build_bridi_formula(bridi)?;
        let formula = self.wrap_bridi_formula_with_quantified_pro_sumti(bridi, formula)?;
        let formula = self.wrap_bridi_formula_with_internal_naku_negations(bridi, formula)?;
        let formula =
            self.wrap_bridi_formula_with_contradictory_event_tense_negation(bridi, formula)?;
        let slots = std::mem::replace(&mut self.parameter_slots, previous_slots);
        let asides = std::mem::replace(&mut self.pending_asides, previous_asides);
        let is_question = truth_question || !slots.is_empty();
        let content = if is_question {
            let id = self.next_question();
            let (kind, domain) = self.question_kind_and_domain(truth_question, &slots);
            self.insert(
                id,
                SemanticObject::question(
                    kind,
                    QuestionMode::Direct,
                    domain,
                    formula,
                    slots,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "question")),
                ),
            )?
        } else {
            formula
        };
        if let Some(anchor) = reserved_utterance
            && let Some(selbri) = main_selbri_for_tail(&bridi.bridi_tail)
        {
            self.attach_indicator_displays(
                indicator_parts_for_selbri(selbri),
                formula,
                anchor,
                "indicator",
            )?;
        }
        let force = if is_question {
            UtteranceForce::Ask
        } else if bridi_contains_ko(bridi) {
            UtteranceForce::Command
        } else {
            UtteranceForce::Assert
        };
        let utterance = self.build_utterance(
            force,
            Some(content),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "bridi")),
            Vec::new(),
            reserved_utterance,
        )?;
        self.current_utterance_anchor = previous_anchor;
        if let Some(node) = self.analysis.syntax_index.bridi_node_id(bridi) {
            self.utterance_objects.insert(node.0, utterance);
        }
        self.add_utterance_asides(utterance, asides);
        Ok(utterance)
    }

    #[requires(true)]
    #[ensures(true)]
    fn question_kind_and_domain(
        &self,
        truth_question: bool,
        slots: &[QuestionSlot],
    ) -> (QuestionKind, SemanticSort) {
        if truth_question && slots.is_empty() {
            return (QuestionKind::Truth, SemanticSort::TruthValue);
        }
        if slots.iter().any(|slot| {
            self.parameter_role(slot.parameter)
                == Some(crate::model::ParameterRole::RelationQuestion)
        }) {
            return (QuestionKind::Relation, SemanticSort::Relation);
        }
        if slots.iter().any(|slot| {
            self.parameter_role(slot.parameter) == Some(crate::model::ParameterRole::PlaceQuestion)
        }) {
            return (QuestionKind::Place, SemanticSort::Place);
        }
        if slots.iter().any(|slot| {
            self.parameter_role(slot.parameter)
                == Some(crate::model::ParameterRole::ConnectiveQuestion)
        }) {
            return (QuestionKind::Connective, SemanticSort::Connective);
        }
        if slots.iter().any(|slot| {
            self.parameter_role(slot.parameter) == Some(crate::model::ParameterRole::TenseQuestion)
        }) {
            return (QuestionKind::Tense, SemanticSort::TenseModal);
        }
        (QuestionKind::Argument, SemanticSort::Entity)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(true)]
    fn parameter_role(&self, parameter: SemanticObjectId) -> Option<crate::model::ParameterRole> {
        self.objects.get(&parameter).and_then(|object| object.role)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_bridi_formula_with_quantified_pro_sumti(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut scopes = self.quantified_pro_sumti_scopes_for_bridi(bridi)?;
        let mut scoped_variables = scopes
            .iter()
            .map(|scope| scope.variable)
            .collect::<HashSet<_>>();
        self.collect_bare_da_series_scopes_from_formula(
            formula,
            &mut scoped_variables,
            &mut scopes,
        );
        let mut body = formula;
        while let Some(scope) = scopes.pop() {
            let data!(QuantifiedProSumtiScope {
                variable,
                quantity,
                operator,
                source,
            }) = scope.into_data();
            let restriction = self
                .restriction_formula_for_variable_in_formula_with_explicit_restrictions(
                    body,
                    variable,
                    Vec::new(),
                )?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::quantified_formula(
                    operator,
                    variable,
                    restriction,
                    body,
                    quantity,
                    source,
                    Vec::new(),
                ),
            )?;
            body = formula;
        }
        Ok(body)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn apply_prenex_terms_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        terms: &'tree [TermSyntax],
    ) -> Result<(), SemanticsError> {
        if terms.is_empty() {
            return Ok(());
        }
        let Some(content) = self.objects.get(&item).and_then(|object| object.content) else {
            return Ok(());
        };
        let wrapped = self.wrap_content_with_prenex_terms(content, terms)?;
        if wrapped != content
            && let Some(object) = self.objects.get_mut(&item)
        {
            object.content = Some(wrapped);
        }
        Ok(())
    }

    #[requires(content.object_kind() == crate::model::SemanticObjectKind::Formula || content.object_kind() == crate::model::SemanticObjectKind::Question || content.object_kind() == crate::model::SemanticObjectKind::Referent || content.object_kind() == crate::model::SemanticObjectKind::Sign || content.object_kind() == crate::model::SemanticObjectKind::DisplayedContent || content.object_kind() == crate::model::SemanticObjectKind::Sequence || content.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == content.object_kind()) || ret.is_err())]
    fn wrap_content_with_prenex_terms(
        &mut self,
        content: SemanticObjectId,
        terms: &'tree [TermSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.wrap_formula_with_prenex_terms(content, terms)
            }
            crate::model::SemanticObjectKind::Question => {
                let body = self.objects.get(&content).and_then(|object| object.body);
                if let Some(body) = body {
                    let wrapped = self.wrap_formula_with_prenex_terms(body, terms)?;
                    if wrapped != body
                        && let Some(object) = self.objects.get_mut(&content)
                    {
                        object.body = Some(wrapped);
                    }
                }
                Ok(content)
            }
            _ => Ok(content),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_prenex_terms(
        &mut self,
        formula: SemanticObjectId,
        terms: &'tree [TermSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let scopes = self.prenex_formula_scopes_for_terms(terms)?;
        let prenex_variables = scopes
            .iter()
            .filter_map(|scope| match scope {
                PrenexFormulaScope::Quantifier { scope, .. } => Some(scope.variable),
                PrenexFormulaScope::RelationQuantifier { scope } => Some(scope.variable),
                PrenexFormulaScope::Negation { .. } => None,
            })
            .collect::<HashSet<_>>();
        let mut body = self.strip_implicit_quantifiers_for_variables(formula, &prenex_variables)?;
        for scope in scopes.into_iter().rev() {
            body = self.wrap_formula_with_prenex_scope(body, scope)?;
        }
        Ok(body)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn prenex_formula_scopes_for_terms(
        &mut self,
        terms: &'tree [TermSyntax],
    ) -> Result<Vec<PrenexFormulaScope>, SemanticsError> {
        let mut scopes = Vec::new();
        for term in terms {
            if let Some(scope) = self.prenex_formula_scope_for_term(term)? {
                scopes.push(scope);
            }
        }
        Ok(scopes)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn strip_implicit_quantifiers_for_variables(
        &mut self,
        formula: SemanticObjectId,
        variables: &HashSet<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if variables.is_empty() {
            return Ok(formula);
        }
        let Some(object) = self.objects.get(&formula) else {
            return Ok(formula);
        };
        let is_implicit_target = object.operator.as_ref().is_some_and(|operator| {
            matches!(
                operator.as_data(),
                data!(SemanticOperator::Formula(FormulaOperator::Exists))
            )
        }) && object.quantity.is_none()
            && object
                .variable
                .is_some_and(|variable| variables.contains(&variable));
        if is_implicit_target && let Some(body) = object.body {
            let stripped = self.strip_implicit_quantifiers_for_variables(body, variables)?;
            self.objects.remove(&formula);
            return Ok(stripped);
        }
        let body = object.body;
        if let Some(body) = body {
            let stripped = self.strip_implicit_quantifiers_for_variables(body, variables)?;
            if stripped != body
                && let Some(object) = self.objects.get_mut(&formula)
            {
                object.body = Some(stripped);
            }
        }
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn prenex_formula_scope_for_term(
        &mut self,
        term: &'tree TermSyntax,
    ) -> Result<Option<PrenexFormulaScope>, SemanticsError> {
        match term.as_data() {
            data!(TermSyntax::BridiNegation { .. }) | data!(TermSyntax::BareNegation(_)) => {
                Ok(Some(PrenexFormulaScope::Negation {
                    source: self.source_for_term(term, "prenex-negation"),
                }))
            }
            data!(TermSyntax::Sumti(sumti))
            | data!(TermSyntax::PlaceTaggedSumti { sumti, .. })
            | data!(TermSyntax::TaggedSumti { sumti, .. }) => {
                if let Some(scope) = self.quantified_relation_variable_scope_for_sumti(sumti)? {
                    return Ok(Some(PrenexFormulaScope::RelationQuantifier { scope }));
                }
                let Some(scope) = self.quantified_pro_sumti_scope_for_sumti(sumti)? else {
                    return Ok(None);
                };
                let restrictions = self
                    .lower_relative_clauses_for_sumti(sumti, scope.variable)?
                    .into_iter()
                    .map(|clause| clause.body)
                    .collect();
                Ok(Some(PrenexFormulaScope::Quantifier {
                    scope,
                    restrictions,
                }))
            }
            _ => Ok(None),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_prenex_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: PrenexFormulaScope,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match scope {
            PrenexFormulaScope::Negation { source } => {
                self.build_unary_formula(FormulaOperator::Not, formula, source, Vec::new())
            }
            PrenexFormulaScope::Quantifier {
                scope,
                restrictions,
            } => self.wrap_formula_with_quantified_pro_sumti_scope(formula, scope, restrictions),
            PrenexFormulaScope::RelationQuantifier { scope } => {
                self.wrap_formula_with_quantified_relation_variable_scope(formula, scope)
            }
        }
    }

    #[requires(explicit_restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|restriction| restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn combine_restriction_formulas(
        &mut self,
        mut explicit_restrictions: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        explicit_restrictions.sort_unstable();
        explicit_restrictions.dedup();
        match explicit_restrictions.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            _ => {
                let conjunction = self.next_formula();
                self.insert(
                    conjunction,
                    SemanticObject::connective_formula(
                        FormulaOperator::And,
                        explicit_restrictions,
                        None,
                        None,
                        Vec::new(),
                    ),
                )
                .map(Some)
            }
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(explicit_restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_quantified_pro_sumti_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: QuantifiedProSumtiScope,
        explicit_restrictions: Vec<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let data!(QuantifiedProSumtiScope {
            variable,
            quantity,
            operator,
            source,
        }) = scope.into_data();
        let restriction = self
            .restriction_formula_for_variable_in_formula_with_explicit_restrictions(
                formula,
                variable,
                explicit_restrictions,
            )?;
        let scoped = self.next_formula();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                operator,
                variable,
                restriction,
                formula,
                quantity,
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_quantified_relation_variable_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: QuantifiedRelationVariableScope,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let data!(QuantifiedRelationVariableScope {
            variable,
            quantity,
            operator,
            source,
        }) = scope.into_data();
        let scoped = self.next_formula();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                operator,
                variable,
                None,
                formula,
                quantity,
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_bridi_formula_with_contradictory_event_tense_negation(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(tense_modal) = first_contradictory_event_tense_modal_for_bridi(bridi) else {
            return Ok(formula);
        };
        self.build_unary_formula(
            FormulaOperator::Not,
            formula,
            self.source_for_tense_modal(tense_modal, "tense-negation"),
            Vec::new(),
        )
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_bridi_formula_with_internal_naku_negations(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut terms = Vec::new();
        collect_bridi_negation_terms_for_bridi(bridi, &mut terms);
        let mut body = formula;
        for term in terms.into_iter().rev() {
            body = self.build_unary_formula(
                FormulaOperator::Not,
                body,
                self.source_for_term(term, "bridi-negation-boundary"),
                Vec::new(),
            )?;
        }
        Ok(body)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn quantified_pro_sumti_scopes_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Result<Vec<QuantifiedProSumtiScope>, SemanticsError> {
        let Some(frame) = self.bridi_frame(bridi) else {
            return Ok(Vec::new());
        };
        let mut scopes = Vec::new();
        let mut scoped_variables = HashSet::new();
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            if !matches!(assignment.slot, PlaceSlot::Numbered(_)) {
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let Some(scope) = self.quantified_pro_sumti_scope_for_sumti(sumti)? else {
                continue;
            };
            let variable = scope.variable;
            if !scoped_variables.insert(variable) {
                continue;
            }
            scopes.push(scope);
        }
        Ok(scopes)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| scope.as_ref().is_none_or(|scope| scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    fn quantified_pro_sumti_scope_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<Option<QuantifiedProSumtiScope>, SemanticsError> {
        let Some(scope_source) = da_series_scope_source(sumti) else {
            return Ok(None);
        };
        let variable = self.build_sumti_referent(sumti)?;
        let (quantity, operator, source) = match scope_source {
            DaSeriesScopeSource::Explicit {
                quantified_sumti,
                quantifier,
            } => {
                let raw = self
                    .analysis
                    .syntax_index
                    .sumti_node_id(quantified_sumti)
                    .ok_or_else(SemanticsError::missing_syntax_node)?
                    .0;
                (
                    Some(self.build_quantity_for_sumti_quantifier(raw, quantifier)?),
                    quantified_pro_sumti_formula_operator(quantifier),
                    self.source_for_node(raw, "quantifier-scope"),
                )
            }
            DaSeriesScopeSource::Bare { da_series_sumti } => {
                let raw = self
                    .analysis
                    .syntax_index
                    .sumti_node_id(da_series_sumti)
                    .ok_or_else(SemanticsError::missing_syntax_node)?
                    .0;
                (
                    None,
                    FormulaOperator::Exists,
                    self.source_for_node(raw, "quantifier-scope"),
                )
            }
        };
        Ok(Some(QuantifiedProSumtiScope::from_data(data!(
            QuantifiedProSumtiScope {
                variable,
                quantity,
                operator,
                source,
            }
        ))))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| scope.as_ref().is_none_or(|scope| scope.variable.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    fn quantified_relation_variable_scope_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<Option<QuantifiedRelationVariableScope>, SemanticsError> {
        let Some(scope_source) = relation_variable_scope_source(sumti) else {
            return Ok(None);
        };
        let variable = self
            .build_relation_variable_parameter_for_selbri(scope_source.selbri)?
            .ok_or_else(SemanticsError::missing_syntax_node)?;
        let raw = self
            .analysis
            .syntax_index
            .sumti_node_id(scope_source.quantified_sumti)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        let quantity =
            Some(self.build_quantity_for_sumti_quantifier(raw, scope_source.quantifier)?);
        Ok(Some(QuantifiedRelationVariableScope::from_data(data!(
            QuantifiedRelationVariableScope {
                variable,
                quantity,
                operator: quantified_pro_sumti_formula_operator(scope_source.quantifier),
                source: self.source_for_node(raw, "quantifier-scope"),
            }
        ))))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn collect_bare_da_series_scopes_from_formula(
        &self,
        formula: SemanticObjectId,
        scoped_variables: &mut HashSet<SemanticObjectId>,
        out: &mut Vec<QuantifiedProSumtiScope>,
    ) {
        let Some(object) = self.objects.get(&formula) else {
            return;
        };
        if let Some(predication) = object.predication
            && let Some(predication) = self.objects.get(&predication)
        {
            for argument in predication.arguments.values() {
                if let Some(value) = argument.value
                    && value.object_kind() == crate::model::SemanticObjectKind::Referent
                {
                    self.push_bare_da_series_scope(value, scoped_variables, out);
                }
                for relative_clause in &argument.relative_clauses {
                    self.collect_bare_da_series_scopes_from_formula(
                        relative_clause.body,
                        scoped_variables,
                        out,
                    );
                }
            }
        }
        for child in &object.children {
            self.collect_bare_da_series_scopes_from_formula(*child, scoped_variables, out);
        }
        if let Some(restriction) = object.restriction {
            self.collect_bare_da_series_scopes_from_formula(restriction, scoped_variables, out);
        }
        if let Some(body) = object.body {
            self.collect_bare_da_series_scopes_from_formula(body, scoped_variables, out);
        }
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn push_bare_da_series_scope(
        &self,
        referent: SemanticObjectId,
        scoped_variables: &mut HashSet<SemanticObjectId>,
        out: &mut Vec<QuantifiedProSumtiScope>,
    ) {
        let Some(object) = self.objects.get(&referent) else {
            return;
        };
        if object.object_kind() != crate::model::SemanticObjectKind::Referent {
            return;
        }
        let Some(descriptor) = &object.descriptor else {
            return;
        };
        if descriptor.kind != "proSumti"
            || !matches!(descriptor.word.as_str(), "da" | "de" | "di")
            || descriptor.quantity.is_some()
            || !scoped_variables.insert(referent)
        {
            return;
        }
        let source = object
            .source
            .clone()
            .map(|source| crate::model::SemanticSource {
                construct: Some("quantifier-scope".to_owned()),
                ..source
            });
        out.push(QuantifiedProSumtiScope::from_data(data!(
            QuantifiedProSumtiScope {
                variable: referent,
                quantity: None,
                operator: FormulaOperator::Exists,
                source,
            }
        )));
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restriction| restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn restriction_formula_for_variable_in_formula(
        &mut self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        self.restriction_formula_for_variable_in_formula_with_explicit_restrictions(
            formula,
            variable,
            Vec::new(),
        )
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(explicit_restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|restriction| restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn restriction_formula_for_variable_in_formula_with_explicit_restrictions(
        &mut self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
        explicit_restrictions: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut restrictions = Vec::new();
        restrictions.extend(explicit_restrictions);
        self.collect_restriction_formulas_attached_to_referent(variable, &mut restrictions);
        self.collect_restriction_formulas_for_variable(formula, variable, &mut restrictions);
        self.combine_restriction_formulas(restrictions)
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn collect_restriction_formulas_attached_to_referent(
        &self,
        variable: SemanticObjectId,
        out: &mut Vec<SemanticObjectId>,
    ) {
        let Some(object) = self.objects.get(&variable) else {
            return;
        };
        out.extend(object.relative_clauses.iter().map(|clause| clause.body));
        if let Some(descriptor) = &object.descriptor {
            out.extend(descriptor.relative_clauses.iter().map(|clause| clause.body));
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn collect_restriction_formulas_for_variable(
        &self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
        out: &mut Vec<SemanticObjectId>,
    ) {
        let Some(object) = self.objects.get(&formula) else {
            return;
        };
        if let Some(predication) = object.predication
            && let Some(predication) = self.objects.get(&predication)
        {
            for argument in predication.arguments.values() {
                if argument.value == Some(variable) {
                    out.extend(argument.relative_clauses.iter().map(|clause| clause.body));
                }
            }
        }
        for child in &object.children {
            self.collect_restriction_formulas_for_variable(*child, variable, out);
        }
        if let Some(restriction) = object.restriction {
            self.collect_restriction_formulas_for_variable(restriction, variable, out);
        }
        if let Some(body) = object.body {
            self.collect_restriction_formulas_for_variable(body, variable, out);
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_utterance(
        &mut self,
        force: UtteranceForce,
        content: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
        reserved_id: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        self.insert(
            eventuality,
            SemanticObject::eventuality(
                EventualityClass::Locution,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                source.clone(),
            ),
        )?;
        let id = reserved_id.unwrap_or_else(|| self.next_utterance());
        self.insert(
            id,
            SemanticObject::utterance(force, eventuality, content, source, diagnostics),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if bridi.bridi_tail.ke_continuation.is_some()
            || bridi.bridi_tail.first.first.bo_continuation.is_some()
            || !bridi.bridi_tail.first.continuations.is_empty()
        {
            return self.build_afterthought_bridi_tail_formula(bridi);
        }
        if bridi.bridi_tail.ke_continuation.is_none()
            && bridi.bridi_tail.first.continuations.is_empty()
            && bridi.bridi_tail.first.first.bo_continuation.is_none()
            && let data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
                connection
            )) = bridi.bridi_tail.first.first.first.as_data()
        {
            return self.build_forethought_bridi_connection_formula(
                connection,
                bridi.leading_terms.is_empty(),
            );
        }
        let selbri = main_selbri_for_tail(&bridi.bridi_tail);
        if let Some(selbri) = selbri
            && let Some(formula) = self.build_scoped_selbri_formula_for_bridi(bridi, selbri)?
        {
            return Ok(formula);
        }
        if let Some(selbri) = selbri
            && let Some(target_bridi) = self.resolved_goha_target_bridi_for_selbri(selbri)
            && !self.bridi_nodes_equal(bridi, target_bridi)
        {
            return self.build_resolved_pro_bridi_formula(bridi, selbri, target_bridi);
        }
        if let Some(selbri) = selbri
            && let Some(target_bridi) = self.resolved_broda_target_bridi_for_selbri(selbri)
        {
            return self.build_resolved_pro_bridi_formula(bridi, selbri, target_bridi);
        }
        if let Some(selbri) = selbri
            && let Some(units) = tanru_units_for_selbri(selbri)
            && tanru_units_require_lowering(&units)
        {
            return self.build_tanru_formula_for_bridi(bridi, selbri, &units);
        }
        if let Some(selbri) = selbri
            && let Some(connected) = self.build_connected_selbri_formula_for_frame(
                selbri,
                self.bridi_frame(bridi),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "connected-selbri-formula")),
                None,
            )?
        {
            return Ok(connected.formula);
        }
        if let Some(selbri) = selbri
            && let Some(bound_tanru) = connectorless_bound_selbri_pair(selbri)
        {
            return self.build_bound_selbri_tanru_formula_for_frame(
                selbri,
                bound_tanru.leading,
                bound_tanru.trailing,
                self.bridi_frame(bridi),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "tanru-formula")),
            );
        }
        if let Some(selbri) = selbri
            && let data!(SelbriSyntax::InvertedTanru {
                leading_selbri,
                trailing_selbri,
                ..
            }) = selbri.as_data()
        {
            return self.build_inverted_tanru_formula_for_frame(
                selbri,
                leading_selbri,
                trailing_selbri,
                self.bridi_frame(bridi),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "tanru-inversion-formula")),
            );
        }
        if let Some(selbri) = selbri
            && let data!(SelbriSyntax::Abstraction(_)) = selbri.as_data()
        {
            return self
                .build_selbri_tanru_formula_for_frame_with_visible_x1_override(
                    selbri,
                    selbri,
                    self.bridi_frame(bridi),
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
                    None,
                )
                .map(|result| result.formula);
        }
        if let Some(selbri) = selbri
            && selbri_is_single_relation_question(selbri)
        {
            return self.build_relation_question_formula_for_bridi(bridi, selbri);
        }
        if let Some(selbri) = selbri
            && selbri_is_single_relation_variable(selbri)
        {
            return self.build_relation_variable_formula_for_bridi(bridi, selbri);
        }
        let relation = selbri
            .map(relation_label_for_selbri)
            .unwrap_or_else(|| "unknown-relation".to_owned());
        if let Some(formula) =
            self.build_forethought_termset_connection_formula(bridi, selbri, relation.clone())?
        {
            return Ok(formula);
        }
        if let Some(selbri) = selbri
            && relation == "identity"
            && let Some(formula) = self.build_connected_mekso_identity_formula(bridi, selbri)?
        {
            return Ok(formula);
        }
        if let Some(formula) =
            self.build_logical_sumti_connection_formula(bridi, selbri, relation.clone())?
        {
            return Ok(formula);
        }
        if let Some(formula) =
            self.build_logical_modal_connection_formula(bridi, selbri, relation.clone())?
        {
            return Ok(formula);
        }
        if let Some(selbri) = selbri
            && let Some(formula) = self.build_connected_event_tense_formula_for_frame(
                self.bridi_frame(bridi),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "predication")),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
                selbri,
                relation.clone(),
            )?
        {
            return Ok(formula);
        }
        let predication = self.build_predication_for_bridi(bridi, selbri, relation)?;
        let id = self.next_formula();
        self.insert(
            id,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_scoped_selbri_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
                let child = self.build_selbri_formula_for_bridi_scope_child(bridi, inner_selbri)?;
                self.build_unary_formula(
                    FormulaOperator::Not,
                    child,
                    self.source_for_selbri(selbri, "bridi-negation"),
                    Vec::new(),
                )
                .map(Some)
            }
            data!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }) if selbri_has_formula_scope(inner_selbri) => {
                let child = self.build_selbri_formula_for_bridi_scope_child(bridi, inner_selbri)?;
                self.build_tense_scope_formula(
                    child,
                    tense_modal,
                    self.source_for_selbri(selbri, "tense-scope"),
                )
                .map(Some)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_selbri_formula_for_bridi_scope_child(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(scoped) = self.build_scoped_selbri_formula_for_bridi(bridi, selbri)? {
            return Ok(scoped);
        }
        let relation = relation_label_for_selbri(selbri);
        if let Some(formula) =
            self.build_logical_sumti_connection_formula(bridi, Some(selbri), relation.clone())?
        {
            return Ok(formula);
        }
        self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
            selbri,
            selbri,
            self.bridi_frame(bridi),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
            None,
        )
        .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_unary_formula(
        &mut self,
        operator: FormulaOperator,
        child: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(operator, vec![child], None, source, diagnostics),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tense_scope_formula(
        &mut self,
        child: SemanticObjectId,
        tense_modal: &'tree TenseModalSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut diagnostics = Vec::new();
        let time_relation = temporal_path_relations_for_tense_modal(tense_modal)
            .into_iter()
            .next();
        let space_relation = space_path_relations_for_tense_modal(tense_modal)
            .into_iter()
            .next();
        let actuality = actuality_for_tense_modal(tense_modal);
        let eventuality =
            if time_relation.is_some() || space_relation.is_some() || actuality.is_some() {
                let eventuality = self.next_eventuality();
                let mut event =
                    SemanticObject::eventuality(EventualityClass::Event, actuality, source.clone());
                if let Some(relation) = time_relation {
                    event.time = Some(new!(AnchorRelation {
                        relation: relation.relation,
                        anchor: SemanticObjectId::speech_time(),
                        distance: relation.distance,
                        magnitude: None,
                        scalar_negation: relation.scalar_negation,
                        motion: relation.motion,
                    }));
                }
                if let Some(relation) = space_relation {
                    event.space = Some(new!(AnchorRelation {
                        relation: relation.relation,
                        anchor: SemanticObjectId::here(),
                        distance: relation.distance,
                        magnitude: None,
                        scalar_negation: relation.scalar_negation,
                        motion: relation.motion,
                    }));
                }
                self.insert(eventuality, event)?;
                Some(eventuality)
            } else {
                diagnostics.push(diagnostic(
                    "scoped tense or modal is not fully lowered beyond source preservation",
                ));
                None
            };
        let formula = self.next_formula();
        let mut object = SemanticObject::connective_formula(
            FormulaOperator::Scoped,
            vec![child],
            None,
            source,
            diagnostics,
        );
        object.eventuality = eventuality;
        self.insert(formula, object)
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_logical_sumti_connection_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(frame) = self.bridi_frame(bridi) else {
            return Ok(None);
        };
        let mut alternatives = BTreeMap::<String, Vec<AlternativeArgument>>::new();
        let mut highest_assigned_place = 0usize;
        let mut connector = None;
        let mut connector_question_token = None;
        let mut modal_connection_spec = None;
        let mut modal_connection_visible_first = true;
        let mut operator = FormulaOperator::And;
        let mut assigned_sumtis = Vec::new();
        let mut assignment_counts = BTreeMap::<String, usize>::new();
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let PlaceSlot::Numbered(place) = assignment.slot else {
                continue;
            };
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let place = place.get() as usize;
            highest_assigned_place = highest_assigned_place.max(place);
            let key = format!("x{place}");
            if let Some((_leading_sumti, connective, tense_modal, _trailing_sumti)) =
                logical_sumti_connection_parts(sumti)
            {
                if connector.is_none() {
                    let connector_question = connective_question_token_for_connective(connective);
                    operator = if connector_question.is_some() {
                        FormulaOperator::ConnectiveQuestion
                    } else {
                        formula_operator_for_connective(connective)
                    };
                    modal_connection_spec = tense_modal
                        .and_then(modal_statement_connection_spec_for_tense_modal)
                        .or_else(|| modal_statement_connection_spec(connective));
                    modal_connection_visible_first =
                        modal_connection_visible_argument_is_first(connective, tense_modal);
                    connector_question_token = connector_question.cloned();
                    connector = Some(Connector {
                        source: modal_connective_text(connective, tense_modal),
                        locus: "sumti".to_owned(),
                        truth_table: None,
                        parameter: None,
                    });
                }
            }
            assigned_sumtis.push((key.clone(), sumti));
            *assignment_counts.entry(key).or_default() += 1;
        }
        let has_duplicate_numbered_assignments = assignment_counts.values().any(|count| *count > 1);
        if connector.is_none() && !has_duplicate_numbered_assignments {
            return Ok(None);
        }
        let recursive_connection = (!has_duplicate_numbered_assignments)
            .then(|| {
                assigned_sumtis
                    .iter()
                    .filter(|(_key, sumti)| {
                        logical_sumti_connection_parts_degrouped(sumti).is_some()
                    })
                    .collect::<Vec<_>>()
            })
            .and_then(|connections| match connections.as_slice() {
                [(key, sumti)] => Some((key.clone(), *sumti)),
                _ => None,
            });
        if let Some((connected_place, connected_sumti)) = recursive_connection {
            let fill_through = self
                .place_count_for_relation(&relation)
                .unwrap_or_else(|| highest_assigned_place.max(1));
            return self.build_recursive_logical_sumti_connection_formula(
                bridi,
                selbri,
                relation,
                &connected_place,
                connected_sumti,
                &assigned_sumtis,
                fill_through,
            );
        }
        for (key, sumti) in assigned_sumtis {
            if let Some((leading_sumti, connective, _tense_modal, trailing_sumti)) =
                logical_sumti_connection_parts(sumti)
            {
                alternatives.entry(key).or_default().extend([
                    AlternativeArgument::new(
                        self.build_argument_for_sumti(leading_sumti)?,
                        connective_negates_left(connective),
                    ),
                    AlternativeArgument::new(
                        self.build_argument_for_sumti(trailing_sumti)?,
                        connective_negates_right(connective),
                    ),
                ]);
            } else {
                alternatives
                    .entry(key)
                    .or_default()
                    .push(AlternativeArgument::new(
                        self.build_argument_for_sumti(sumti)?,
                        false,
                    ));
            }
        }
        let fill_through = self
            .place_count_for_relation(&relation)
            .unwrap_or_else(|| highest_assigned_place.max(1));
        for place in 1..=fill_through {
            let key = format!("x{place}");
            if !alternatives.contains_key(&key) {
                alternatives.insert(
                    key,
                    vec![AlternativeArgument::new(
                        self.build_elided_argument_for_place(place)?,
                        false,
                    )],
                );
            }
        }
        let mut branches = vec![BTreeMap::new()];
        for (place, values) in alternatives {
            let mut next = Vec::new();
            for branch in &branches {
                for value in &values {
                    let mut branch = branch.clone();
                    branch.insert(place.clone(), value.clone());
                    next.push(branch);
                }
            }
            branches = next;
        }
        let mut children = Vec::new();
        for branch in branches {
            let branch_negated = branch.values().any(|value| value.negated);
            let arguments = branch
                .into_iter()
                .map(|(place, value)| (place, value.argument))
                .collect();
            let predication = self.build_predication_from_arguments(
                relation.clone(),
                selbri,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "distributed-predication")),
                arguments,
                Vec::new(),
            )?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::atom_formula(
                    predication,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "distributed-formula")),
                    Vec::new(),
                ),
            )?;
            let formula = if branch_negated {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    formula,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "distributed-negation")),
                    Vec::new(),
                )?
            } else {
                formula
            };
            children.push(formula);
        }
        let mut diagnostics = Vec::new();
        if let Some(spec) = modal_connection_spec {
            if let [first_formula, second_formula] = children.as_slice() {
                let (visible_formula, other_formula) = if modal_connection_visible_first {
                    (*first_formula, *second_formula)
                } else {
                    (*second_formula, *first_formula)
                };
                match self.build_modal_formula_connection_claim(
                    visible_formula,
                    other_formula,
                    &spec,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "sumti-connection-claim")),
                )? {
                    Some(claim) => children.push(claim),
                    None => diagnostics.push(diagnostic(
                        "modal sumti connection could not find formula-bearing bridi events to relate",
                    )),
                }
            } else {
                diagnostics.push(diagnostic(
                    "modal sumti connection with more than two distributed branches is not fully lowered yet",
                ));
            }
        }
        let connector_parameter = connector_question_token
            .as_ref()
            .map(|token| self.build_connective_question_parameter_for_token(token))
            .transpose()?;
        if let Some(connector) = connector.as_mut() {
            connector.parameter = connector_parameter;
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                connector,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "sumti-connection-formula")),
                diagnostics,
            ),
        )
        .map(Some)
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_logical_modal_connection_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let frame = selbri
            .and_then(|selbri| {
                self.semantic_predication_frame_for_selbri(selbri, self.bridi_frame(bridi))
            })
            .or_else(|| self.bridi_frame(bridi));
        let Some(frame) = frame else {
            return Ok(None);
        };
        let Some(connection) = self.logical_modal_connection_assignment(frame)? else {
            return Ok(None);
        };
        let data!(LogicalModalConnectionAssignment {
            argument: modal_argument,
            operator,
            source: connector_source,
            truth_table,
            terms,
        }) = connection.into_data();
        let mut children = Vec::new();
        for term in terms {
            let data!(ConnectedModalTerm {
                introduced_by,
                relation: modal_relation,
                visible_place,
                tokens,
                negation,
                scalar_negation,
            }) = term.into_data();
            let arguments = self.modal_argument_map_for_visible_place(
                modal_argument.clone(),
                visible_place,
                self.place_count_for_relation(&modal_relation),
            )?;
            let modal_argument = ModalArgument::new_with_polarity(
                modal_relation,
                introduced_by,
                arguments,
                negation,
                scalar_negation,
                self.source_for_tokens(&tokens, "modal-argument"),
            );
            let predication = self.build_predication_for_frame_with_modal_arguments(
                Some(frame),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "predication")),
                selbri,
                relation.clone(),
                BTreeMap::new(),
                vec![modal_argument],
                false,
            )?;
            self.attach_reciprocity_to_predication(predication, bridi, &[])?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::atom_formula(
                    predication,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "modal-branch-formula")),
                    Vec::new(),
                ),
            )?;
            children.push(formula);
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(Connector {
                    source: connector_source,
                    locus: "modal".to_owned(),
                    truth_table: Some(truth_table),
                    parameter: None,
                }),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "modal-connection-formula")),
                Vec::new(),
            ),
        )
        .map(Some)
    }

    #[requires(!relation.is_empty())]
    #[requires(argument_place_index(connected_place).is_some())]
    #[requires(assigned_sumtis.iter().any(|(place, _sumti)| place == connected_place))]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_some_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_recursive_logical_sumti_connection_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
        connected_place: &str,
        connected_sumti: &'tree SumtiSyntax,
        assigned_sumtis: &[(String, &'tree SumtiSyntax)],
        fill_through: usize,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut base_arguments = BTreeMap::new();
        for (place, sumti) in assigned_sumtis {
            if place == connected_place {
                continue;
            }
            base_arguments.insert(place.clone(), self.build_argument_for_sumti(sumti)?);
        }
        for place in 1..=fill_through {
            let key = format!("x{place}");
            if key != connected_place && !base_arguments.contains_key(&key) {
                base_arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        self.build_sumti_connection_formula_for_place(
            bridi,
            selbri,
            &relation,
            connected_place,
            &base_arguments,
            connected_sumti,
        )
        .map(Some)
    }

    #[requires(!relation.is_empty())]
    #[requires(argument_place_index(connected_place).is_some())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_sumti_connection_formula_for_place(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: &str,
        connected_place: &str,
        base_arguments: &BTreeMap<String, ArgumentValue>,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some((leading_sumti, connective, tense_modal, trailing_sumti)) =
            logical_sumti_connection_parts_degrouped(sumti)
        else {
            return self.build_sumti_connection_branch_formula(
                bridi,
                selbri,
                relation,
                connected_place,
                base_arguments,
                sumti,
                false,
            );
        };
        let leading_formula = self.build_sumti_connection_branch_formula(
            bridi,
            selbri,
            relation,
            connected_place,
            base_arguments,
            leading_sumti,
            connective_negates_left(connective),
        )?;
        let trailing_formula = self.build_sumti_connection_branch_formula(
            bridi,
            selbri,
            relation,
            connected_place,
            base_arguments,
            trailing_sumti,
            connective_negates_right(connective),
        )?;
        let mut children = vec![leading_formula, trailing_formula];
        let mut diagnostics = Vec::new();
        let relation_only = !connective_has_logical_component(connective)
            && (modal_tense_relation_spec_for_connective(connective).is_some()
                || tense_modal.is_some_and(|tense_modal| {
                    tense_relation_spec_for_tense_modal(tense_modal).is_some()
                }));
        if let Some(spec) = modal_connection_spec_for_connective_and_tense(connective, tense_modal)
        {
            let (visible_formula, other_formula) =
                if modal_connection_visible_argument_is_first(connective, tense_modal) {
                    (leading_formula, trailing_formula)
                } else {
                    (trailing_formula, leading_formula)
                };
            match self.build_modal_formula_connection_claim(
                visible_formula,
                other_formula,
                &spec,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "sumti-connection-claim")),
            )? {
                Some(claim) => {
                    if relation_only {
                        self.set_formula_predication_mode(leading_formula, PredicationMode::Inert);
                        self.set_formula_predication_mode(trailing_formula, PredicationMode::Inert);
                        return Ok(claim);
                    }
                    children.push(claim);
                }
                None => diagnostics.push(diagnostic(
                    "modal sumti connection could not find formula-bearing bridi events to relate",
                )),
            }
        }
        let formula = self.next_formula();
        let connector_question = connective_question_token_for_connective(connective);
        let connector_parameter = connector_question
            .map(|token| self.build_connective_question_parameter_for_token(token))
            .transpose()?;
        self.insert(
            formula,
            SemanticObject::connective_formula(
                if connector_question.is_some() {
                    FormulaOperator::ConnectiveQuestion
                } else {
                    formula_operator_for_connective(connective)
                },
                children,
                Some(Connector {
                    source: modal_connective_text(connective, tense_modal),
                    locus: "sumti".to_owned(),
                    truth_table: None,
                    parameter: connector_parameter,
                }),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "sumti-connection-formula")),
                diagnostics,
            ),
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(argument_place_index(connected_place).is_some())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_sumti_connection_branch_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: &str,
        connected_place: &str,
        base_arguments: &BTreeMap<String, ArgumentValue>,
        sumti: &'tree SumtiSyntax,
        negated: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let formula = if logical_sumti_connection_parts_degrouped(sumti).is_some() {
            self.build_sumti_connection_formula_for_place(
                bridi,
                selbri,
                relation,
                connected_place,
                base_arguments,
                sumti,
            )?
        } else {
            let mut arguments = base_arguments.clone();
            arguments.insert(
                connected_place.to_owned(),
                self.build_argument_for_sumti(sumti)?,
            );
            let predication = self.build_predication_from_arguments(
                relation.to_owned(),
                selbri,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "distributed-predication")),
                arguments,
                Vec::new(),
            )?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::atom_formula(
                    predication,
                    self.analysis
                        .syntax_index
                        .bridi_node_id(bridi)
                        .and_then(|node| self.source_for_node(node.0, "distributed-formula")),
                    Vec::new(),
                ),
            )?
        };
        if negated {
            self.build_unary_formula(
                FormulaOperator::Not,
                formula,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "distributed-negation")),
                Vec::new(),
            )
        } else {
            Ok(formula)
        }
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_forethought_termset_connection_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((term, gek, leading_terms, trailing_terms)) =
            bridi
                .leading_terms
                .iter()
                .find_map(|term| match term.as_data() {
                    data!(TermSyntax::ForethoughtTermsetConnection {
                        gek,
                        terms,
                        gik_terms,
                        ..
                    }) => Some((term, gek, terms.as_slice(), gik_terms.as_slice())),
                    _ => None,
                })
        else {
            return Ok(None);
        };
        let source = self
            .analysis
            .syntax_index
            .term_node_id(term)
            .and_then(|node| self.source_for_node(node.0, "termset-connection-formula"));
        let leading = self.build_termset_branch_formula(
            leading_terms,
            selbri,
            relation.clone(),
            source.clone(),
        )?;
        let trailing =
            self.build_termset_branch_formula(trailing_terms, selbri, relation, source.clone())?;
        let mut children = vec![leading, trailing];
        let mut diagnostics = Vec::new();
        if bridi
            .leading_terms
            .iter()
            .filter(|candidate| !std::ptr::eq(*candidate, term))
            .any(|candidate| !matches!(candidate.as_data(), data!(TermSyntax::Termset { .. })))
        {
            diagnostics.push(diagnostic(
                "forethought termset connection with extra outer terms is not fully lowered yet",
            ));
        }
        let operator = if let Some(spec) = modal_statement_connection_spec(gek) {
            match self.build_modal_formula_connection_claim(leading, trailing, &spec, source.clone())?
            {
                Some(claim) => children.push(claim),
                None => diagnostics.push(diagnostic(
                    "modal termset connection could not find formula-bearing bridi events or propositions to relate",
                )),
            }
            FormulaOperator::And
        } else {
            formula_operator_for_connective(gek)
        };
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(Connector {
                    source: connective_text(gek),
                    locus: "termset".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                diagnostics,
            ),
        )
        .map(Some)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_mekso_identity_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(frame) = self.bridi_frame(bridi) else {
            return Ok(None);
        };
        let assignments = self
            .analysis
            .place_analysis
            .assignments_for_frame(frame)
            .iter()
            .filter_map(|id| self.analysis.place_analysis.assignment(*id).cloned())
            .collect::<Vec<_>>();
        for connected_assignment in &assignments {
            let PlaceSlot::Numbered(connected_place) = connected_assignment.slot else {
                continue;
            };
            let connected_sumti = self
                .analysis
                .syntax_index
                .sumti(connected_assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let data!(SumtiSyntax::NumberSumti { li, expression, .. }) = connected_sumti.as_data()
            else {
                continue;
            };
            let data!(MeksoSyntax::ForethoughtMeksoConnection {
                gek,
                left_expression,
                right_expression,
                ..
            }) = expression.as_data()
            else {
                continue;
            };
            let source = self
                .analysis
                .syntax_index
                .sumti_node_id(connected_sumti)
                .and_then(|node| self.source_for_node(node.0, "operand-connection-formula"));
            let left = self.build_connected_mekso_identity_branch_formula(
                &assignments,
                connected_assignment.id,
                connected_place.get() as usize,
                left_expression,
                li,
                connected_assignment.sumti.0,
                selbri,
                source.clone(),
            )?;
            let right = self.build_connected_mekso_identity_branch_formula(
                &assignments,
                connected_assignment.id,
                connected_place.get() as usize,
                right_expression,
                li,
                connected_assignment.sumti.0,
                selbri,
                source.clone(),
            )?;
            let mut children = vec![left, right];
            let mut diagnostics = Vec::new();
            let operator = if let Some(spec) = modal_statement_connection_spec(gek) {
                match self.build_modal_formula_connection_claim(
                    left,
                    right,
                    &spec,
                    source.clone(),
                )? {
                    Some(claim) => children.push(claim),
                    None => diagnostics.push(diagnostic(
                        "modal operand connection could not find formulas to relate",
                    )),
                }
                FormulaOperator::And
            } else {
                formula_operator_for_connective(gek)
            };
            let formula = self.next_formula();
            return self
                .insert(
                    formula,
                    SemanticObject::connective_formula(
                        operator,
                        children,
                        Some(Connector {
                            source: connective_text(gek),
                            locus: "operand".to_owned(),
                            truth_table: None,
                            parameter: None,
                        }),
                        source,
                        diagnostics,
                    ),
                )
                .map(Some);
        }
        Ok(None)
    }

    #[requires(connected_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_mekso_identity_branch_formula(
        &mut self,
        assignments: &[SumtiPlaceAssignment],
        connected_assignment: SumtiPlaceAssignmentId,
        connected_place: usize,
        expression: &'tree MeksoSyntax,
        li: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
        selbri: &'tree SelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        for assignment in assignments {
            let PlaceSlot::Numbered(place) = assignment.slot else {
                continue;
            };
            if assignment.id == connected_assignment {
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            arguments.insert(
                format!("x{}", place.get()),
                self.build_argument_for_sumti(sumti)?,
            );
        }
        let branch_referent = self.build_number_referent(expression, li, raw)?;
        arguments.insert(
            format!("x{connected_place}"),
            ArgumentValue::filled(branch_referent, None),
        );
        let predication = self.build_predication_from_arguments(
            "identity".to_owned(),
            Some(selbri),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_termset_branch_formula(
        &mut self,
        terms: &'tree [TermSyntax],
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        let mut modal_arguments = Vec::new();
        let mut diagnostics = Vec::new();
        let mut next_sequential_place = 1usize;
        for term in terms {
            match term.as_data() {
                data!(TermSyntax::Sumti(sumti)) => {
                    let argument = self.build_argument_for_sumti(sumti)?;
                    arguments.insert(format!("x{next_sequential_place}"), argument);
                    next_sequential_place += 1;
                }
                data!(TermSyntax::PlaceTaggedSumti { fa, sumti, .. }) => {
                    if let Some(place) = numbered_place_for_fa_token(&fa.value) {
                        arguments
                            .insert(format!("x{place}"), self.build_argument_for_sumti(sumti)?);
                    } else {
                        diagnostics.push(diagnostic(
                            "forethought termset branch place question is not fully lowered yet",
                        ));
                    }
                }
                data!(TermSyntax::TaggedSumti {
                    tense_modal: Some(tense_modal),
                    sumti,
                }) => {
                    if let Some((introduced_by, relation, visible_place)) =
                        modal_relation_spec_for_tense_modal(tense_modal)
                    {
                        let argument = self.build_argument_for_sumti(sumti)?;
                        let arguments = self.modal_argument_map_for_visible_place(
                            argument,
                            visible_place,
                            self.place_count_for_relation(&relation),
                        )?;
                        modal_arguments.push(self.modal_argument_with_tense_modal_modifiers(
                            tense_modal,
                            relation,
                            introduced_by,
                            arguments,
                            modal_negation_for_tense_modal(tense_modal),
                            modal_scalar_negation_for_tense_modal(tense_modal),
                            "modal-argument",
                        ));
                    } else {
                        diagnostics.push(diagnostic(
                            "forethought termset branch tagged term is not fully lowered yet",
                        ));
                    }
                }
                data!(TermSyntax::TaggedSumti {
                    tense_modal: None,
                    ..
                }) => diagnostics.push(diagnostic(
                    "forethought termset branch tagged term is missing its tag",
                )),
                _ => diagnostics.push(diagnostic(
                    "forethought termset branch term is not fully lowered yet",
                )),
            }
        }
        let highest_place = arguments
            .keys()
            .filter_map(|place| argument_place_index(place))
            .max()
            .unwrap_or(0);
        if let Some(place_count) = self.place_count_for_relation(&relation) {
            for place in 1..=place_count {
                let key = format!("x{place}");
                if !arguments.contains_key(&key) {
                    arguments.insert(key, self.build_elided_argument_for_place(place)?);
                }
            }
        } else if highest_place == 0 {
            arguments.insert("x1".to_owned(), self.build_elided_argument_for_place(1)?);
        }
        let predication = self.build_predication_from_arguments(
            relation,
            selbri,
            source.clone(),
            arguments,
            diagnostics,
        )?;
        if !modal_arguments.is_empty() {
            self.objects
                .get_mut(&predication)
                .ok_or_else(|| {
                    SemanticsError::invalid_graph(format!("missing predication {predication}"))
                })?
                .modal_arguments
                .extend(modal_arguments);
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(bridi.bridi_tail.ke_continuation.is_some() || bridi.bridi_tail.first.first.bo_continuation.is_some() || !bridi.bridi_tail.first.continuations.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_afterthought_bridi_tail_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_connected_bridi_tail_formula(
            &bridi.bridi_tail,
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "compound-bridi-formula")),
        )
    }

    #[requires(tail.ke_continuation.is_some() || tail.first.first.bo_continuation.is_some() || !tail.first.continuations.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_bridi_tail_formula(
        &mut self,
        tail: &'tree BridiTailSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if tail.ke_continuation.is_none() && tail.first.continuations.is_empty() {
            return self.build_bo_grouped_tail_formula(&tail.first.first);
        }
        let mut children = Vec::new();
        children.push(self.build_bo_grouped_tail_formula(&tail.first.first)?);
        let mut connector = None;
        let mut operator = FormulaOperator::And;
        let mut diagnostics = Vec::new();
        for continuation in &tail.first.continuations {
            if connector.is_none() {
                operator = formula_operator_for_connective(&continuation.connective);
                connector = Some(Connector {
                    source: modal_connective_text(
                        &continuation.connective,
                        continuation.tense_modal.as_deref(),
                    ),
                    locus: "bridiTail".to_owned(),
                    truth_table: None,
                    parameter: None,
                });
            }
            let previous_formula = *children
                .last()
                .expect("connected bridi tail starts with one child");
            let next_formula = self.build_bo_grouped_tail_formula(&continuation.bridi_tail)?;
            children.push(next_formula);
            self.push_modal_bridi_tail_connection_claim(
                &mut children,
                &mut diagnostics,
                &continuation.connective,
                continuation.tense_modal.as_deref(),
                next_formula,
                previous_formula,
            )?;
        }
        if let Some(continuation) = &tail.ke_continuation {
            if connector.is_none() {
                operator = formula_operator_for_connective(&continuation.connective);
                connector = Some(Connector {
                    source: modal_connective_text(
                        &continuation.connective,
                        continuation.tense_modal.as_deref(),
                    ),
                    locus: "bridiTail".to_owned(),
                    truth_table: None,
                    parameter: None,
                });
            }
            let previous_formula = *children
                .last()
                .expect("connected bridi tail starts with one child");
            let next_formula =
                self.build_connected_or_single_bridi_tail_formula(&continuation.bridi_tail)?;
            children.push(next_formula);
            self.push_modal_bridi_tail_connection_claim(
                &mut children,
                &mut diagnostics,
                &continuation.connective,
                continuation.tense_modal.as_deref(),
                next_formula,
                previous_formula,
            )?;
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(operator, children, connector, source, diagnostics),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_or_single_bridi_tail_formula(
        &mut self,
        tail: &'tree BridiTailSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if tail.ke_continuation.is_some()
            || tail.first.first.bo_continuation.is_some()
            || !tail.first.continuations.is_empty()
        {
            self.build_connected_bridi_tail_formula(tail, None)
        } else {
            self.build_bo_grouped_tail_formula(&tail.first.first)
        }
    }

    #[requires(visible_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(other_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn push_modal_bridi_tail_connection_claim(
        &mut self,
        children: &mut Vec<SemanticObjectId>,
        diagnostics: &mut Vec<SemanticDiagnostic>,
        connective: &ConnectiveSyntax,
        tense_modal: Option<&TenseModalSyntax>,
        visible_formula: SemanticObjectId,
        other_formula: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        let Some(spec) = modal_connection_spec_for_connective_and_tense(connective, tense_modal)
        else {
            return Ok(());
        };
        let source = tense_modal.and_then(|tense_modal| {
            self.source_for_tense_modal(tense_modal, "bridi-tail-connection-claim")
        });
        match self.build_modal_formula_connection_claim(
            visible_formula,
            other_formula,
            &spec,
            source,
        )? {
            Some(claim) => children.push(claim),
            None => diagnostics.push(diagnostic(
                "modal bridi-tail connection could not find formula-bearing bridi events to relate",
            )),
        }
        Ok(())
    }

    #[requires(!children.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connective_formula(
        &mut self,
        operator: FormulaOperator,
        children: Vec<SemanticObjectId>,
        connector: Option<Connector>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(operator, children, connector, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_event_tense_formula_for_frame(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        selbri: &'tree SelbriSyntax,
        relation: String,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(tense_modal) = connected_event_tense_modal_for_selbri(selbri) else {
            return Ok(None);
        };
        let Some(spec) = connected_event_tense_spec_for_tense_modal(tense_modal) else {
            return Ok(None);
        };
        let data!(ConnectedEventTenseSpec {
            operator,
            source,
            truth_table,
            connector_question,
            branches,
        }) = spec.into_data();
        let mut children = Vec::new();
        let branch_selbri = selbri_without_connected_event_tense(selbri);
        for branch in branches {
            let data!(ConnectedEventTenseBranch {
                tense_modal,
                negated,
            }) = branch.into_data();
            let predication = self.build_predication_for_frame(
                frame,
                predication_source.clone(),
                branch_selbri,
                relation.clone(),
            )?;
            self.replace_predication_event_modifiers(predication, &tense_modal)?;
            let atom = self.next_formula();
            let atom = self.insert(
                atom,
                SemanticObject::atom_formula(predication, formula_source.clone(), Vec::new()),
            )?;
            let branch_formula = if negated {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    atom,
                    self.source_for_tense_modal(&tense_modal, "tense-negation"),
                    Vec::new(),
                )?
            } else {
                atom
            };
            children.push(branch_formula);
        }
        let connector_parameter = connector_question
            .as_ref()
            .map(|token| self.build_connective_question_parameter_for_token(token))
            .transpose()?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(Connector {
                    source,
                    locus: "tense".to_owned(),
                    truth_table,
                    parameter: connector_parameter,
                }),
                formula_source,
                Vec::new(),
            ),
        )
        .map(Some)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn replace_predication_event_modifiers(
        &mut self,
        predication: SemanticObjectId,
        tense_modal: &TenseModalSyntax,
    ) -> Result<(), SemanticsError> {
        let eventuality = self
            .objects
            .get(&predication)
            .and_then(|object| object.eventuality)
            .ok_or_else(|| {
                SemanticsError::invalid_graph(format!(
                    "predication {predication} has no event to retune"
                ))
            })?;
        let tense_question_parameter =
            self.build_tense_question_parameter_for_tense_modal(tense_modal)?;
        let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
            SemanticsError::invalid_graph(format!(
                "predication {predication} points to missing event {eventuality}"
            ))
        })?;
        clear_event_modifiers(event);
        apply_tense_modal_event_modifiers_to_event(tense_modal, event);
        if let Some(parameter) = tense_question_parameter {
            event.tense_modal = Some(parameter);
        }
        Ok(())
    }

    #[requires(tanru_units_require_lowering(units))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_formula_for_tail(
        &mut self,
        selbri: &'tree SelbriSyntax,
        units: &[&'tree TanruUnitSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "predication"));
        self.build_tanru_sequence_formula_for_frame(
            Some(selbri),
            units,
            self.branch_frame_for_selbri(selbri),
            source,
        )
        .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bo_grouped_tail_formula(
        &mut self,
        tail: &'tree BoGroupedBridiTailSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(continuation) = &tail.bo_continuation {
            return self.build_bound_bridi_tail_connection_formula(tail, continuation);
        }
        self.build_bo_grouped_tail_formula_core(tail)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bo_grouped_tail_formula_core(
        &mut self,
        tail: &'tree BoGroupedBridiTailSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
            connection
        )) = tail.first.as_data()
        {
            return self.build_forethought_bridi_connection_formula(connection, false);
        }
        let Some(selbri) = simple_bo_grouped_tail_selbri(tail) else {
            let predication =
                self.build_predication_for_frame(None, None, None, "unknown-relation".to_owned())?;
            let formula = self.next_formula();
            return self.insert(
                formula,
                SemanticObject::atom_formula(
                    predication,
                    None,
                    vec![diagnostic("bridi-tail branch is not fully lowered yet")],
                ),
            );
        };
        if let Some(units) = tanru_units_for_selbri(selbri)
            && tanru_units_require_lowering(&units)
        {
            return self.build_tanru_formula_for_tail(selbri, &units);
        }
        if let Some(connected) = self.build_connected_selbri_formula_for_frame(
            selbri,
            self.branch_frame_for_selbri(selbri),
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "connected-selbri-formula")),
            None,
        )? {
            return Ok(connected.formula);
        }
        if let Some(bound_tanru) = connectorless_bound_selbri_pair(selbri) {
            return self.build_bound_selbri_tanru_formula_for_frame(
                selbri,
                bound_tanru.leading,
                bound_tanru.trailing,
                self.branch_frame_for_selbri(selbri),
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "tanru-formula")),
            );
        }
        if let data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        }) = selbri.as_data()
        {
            return self.build_inverted_tanru_formula_for_frame(
                selbri,
                leading_selbri,
                trailing_selbri,
                self.branch_frame_for_selbri(selbri),
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "tanru-inversion-formula")),
            );
        }
        if let data!(SelbriSyntax::Abstraction(_)) = selbri.as_data() {
            return self
                .build_selbri_tanru_formula_for_frame_with_visible_x1_override(
                    selbri,
                    selbri,
                    self.branch_frame_for_selbri(selbri),
                    self.analysis
                        .syntax_index
                        .selbri_node_id(selbri)
                        .and_then(|node| self.source_for_node(node.0, "bridi-tail-formula")),
                    None,
                )
                .map(|result| result.formula);
        }
        let relation = relation_label_for_selbri(selbri);
        if let Some(formula) = self.build_connected_event_tense_formula_for_frame(
            self.branch_frame_for_selbri(selbri),
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "predication")),
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "bridi-tail-formula")),
            selbri,
            relation.clone(),
        )? {
            return Ok(formula);
        }
        let predication = self.build_predication_for_frame(
            self.branch_frame_for_selbri(selbri),
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "predication")),
            Some(selbri),
            relation,
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "bridi-tail-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bound_bridi_tail_connection_formula(
        &mut self,
        leading_tail: &'tree BoGroupedBridiTailSyntax,
        continuation: &'tree BoundBridiTailConnectionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let first_formula = self.build_bo_grouped_tail_formula_core(leading_tail)?;
        let second_formula = self.build_bo_grouped_tail_formula(&continuation.bridi_tail)?;
        let mut children = vec![first_formula, second_formula];
        let mut diagnostics = Vec::new();
        if let Some(tense_modal) = continuation.tense_modal.as_deref()
            && let Some(spec) = modal_statement_connection_spec_for_tense_modal(tense_modal)
        {
            match self.build_modal_formula_connection_claim(
                second_formula,
                first_formula,
                &spec,
                self.source_for_tense_modal(tense_modal, "bridi-tail-connection-claim"),
            )? {
                Some(claim) => children.push(claim),
                None => diagnostics.push(diagnostic(
                    "modal bridi-tail connection could not find formula-bearing bridi events to relate",
                )),
            }
        }
        let source = continuation.tense_modal.as_deref().and_then(|tense_modal| {
            self.source_for_tense_modal(tense_modal, "bridi-tail-connection-formula")
        });
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                formula_operator_for_connective(&continuation.connective),
                children,
                Some(Connector {
                    source: modal_connective_text(
                        &continuation.connective,
                        continuation.tense_modal.as_deref(),
                    ),
                    locus: "bridiTail".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                diagnostics,
            ),
        )
    }

    #[requires(tanru_units_require_lowering(units))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
        units: &[&'tree TanruUnitSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.bridi_frame(bridi))
            .or_else(|| self.bridi_frame(bridi));
        self.build_tanru_sequence_formula_for_frame(
            Some(selbri),
            units,
            frame,
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "tanru-formula")),
        )
        .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bound_selbri_tanru_formula_for_frame(
        &mut self,
        selbri: &'tree SelbriSyntax,
        leading: &'tree SelbriSyntax,
        trailing: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_bound_selbri_tanru_formula_for_argument(selbri, leading, trailing, frame, source)
            .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_inverted_tanru_formula_for_frame(
        &mut self,
        selbri: &'tree SelbriSyntax,
        tertau: &'tree SelbriSyntax,
        seltau: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_inverted_tanru_formula_for_argument(selbri, tertau, seltau, frame, source, None)
            .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bound_selbri_tanru_formula_for_argument(
        &mut self,
        selbri: &'tree SelbriSyntax,
        leading: &'tree SelbriSyntax,
        trailing: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let tertau =
            self.build_selbri_tanru_formula_for_frame(selbri, trailing, frame, source.clone())?;
        let modifier = self.build_property_abstraction_for_selbri(leading, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            tertau.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_selbri_pair(leading, trailing),
            PredicationMode::Asserted,
            source.clone(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau.formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "selbri".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument: tertau.x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_forethought_bridi_connection_formula(
        &mut self,
        connection: &'tree ForethoughtBridiConnectionSyntax,
        claim_tense_branches: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match connection.as_data() {
            data!(ForethoughtBridiConnectionSyntax::BridiConnection {
                gek,
                first,
                gik,
                second,
                tail_terms,
                free_modifiers,
                ..
            }) => {
                let Some(first_formula) = self.build_subbridi_formula(first)? else {
                    let predication = self.build_predication_for_frame(
                        None,
                        None,
                        None,
                        "unknown-relation".to_owned(),
                    )?;
                    let formula = self.next_formula();
                    return self.insert(
                        formula,
                        SemanticObject::atom_formula(
                            predication,
                            None,
                            vec![diagnostic(
                                "forethought bridi connection first branch is not fully lowered",
                            )],
                        ),
                    );
                };
                let Some(second_formula) = self.build_subbridi_formula(second)? else {
                    let predication = self.build_predication_for_frame(
                        None,
                        None,
                        None,
                        "unknown-relation".to_owned(),
                    )?;
                    let formula = self.next_formula();
                    return self.insert(
                        formula,
                        SemanticObject::atom_formula(
                            predication,
                            None,
                            vec![diagnostic(
                                "forethought bridi connection second branch is not fully lowered",
                            )],
                        ),
                    );
                };
                if let Some(anchor) = self.current_utterance_anchor {
                    let mut first_indicator_parts = indicator_parts_for_connective_cmavo(gek);
                    first_indicator_parts.extend(indicator_parts_for_connective_nai(gek));
                    self.attach_indicator_displays(
                        first_indicator_parts,
                        first_formula,
                        anchor,
                        "indicator",
                    )?;
                    let mut second_indicator_parts = indicator_parts_for_connective_cmavo(gik);
                    second_indicator_parts.extend(indicator_parts_for_connective_nai(gik));
                    self.attach_indicator_displays(
                        second_indicator_parts,
                        second_formula,
                        anchor,
                        "indicator",
                    )?;
                }
                let tense_relation = modal_tense_relation_spec_for_connective(gek).is_some();
                let relation_only = tense_relation && !claim_tense_branches;
                let mut children = Vec::new();
                if !relation_only {
                    children.push(first_formula);
                    children.push(second_formula);
                }
                let mut diagnostics = Vec::new();
                let operator = if let Some(spec) = modal_statement_connection_spec(gek) {
                    let (visible_formula, other_formula) =
                        if modal_connection_visible_argument_is_first(gek, None) {
                            (first_formula, second_formula)
                        } else {
                            (second_formula, first_formula)
                        };
                    match self.build_modal_formula_connection_claim(
                        visible_formula,
                        other_formula,
                        &spec,
                        None,
                    )? {
                        Some(claim) => {
                            if relation_only {
                                self.set_formula_predication_mode(
                                    first_formula,
                                    PredicationMode::Inert,
                                );
                                self.set_formula_predication_mode(
                                    second_formula,
                                    PredicationMode::Inert,
                                );
                                return Ok(claim);
                            }
                            children.push(claim);
                        }
                        None => diagnostics.push(diagnostic(
                            "modal forethought connection could not find formula-bearing bridi events to relate",
                        )),
                    }
                    FormulaOperator::And
                } else {
                    formula_operator_for_connective(gek)
                };
                if !tail_terms.is_empty() {
                    // Shared bridi-tail terms are propagated into each branch frame by
                    // reference analysis and are therefore already represented above.
                }
                if !free_modifiers.is_empty() {
                    diagnostics.push(diagnostic(
                        "forethought bridi connection free modifiers are not fully lowered yet",
                    ));
                }
                if children.len() == 1 {
                    return Ok(children[0]);
                }
                let formula = self.next_formula();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        operator,
                        children,
                        Some(Connector {
                            source: connective_text(gek),
                            locus: "bridi".to_owned(),
                            truth_table: None,
                            parameter: None,
                        }),
                        None,
                        diagnostics,
                    ),
                )
            }
            data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection { inner, .. }) => {
                self.build_forethought_bridi_connection_formula(inner, claim_tense_branches)
            }
            data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => {
                let child =
                    self.build_forethought_bridi_connection_formula(inner, claim_tense_branches)?;
                self.build_unary_formula(FormulaOperator::Not, child, None, Vec::new())
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_selbri_formula_for_frame(
        &mut self,
        selbri: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<Option<TanruFormulaForArgument>, SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::SelbriConnection {
                leading_selbri,
                connective,
                trailing_selbri,
            }) => self
                .build_connected_selbri_pair_formula_for_frame(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    frame,
                    source,
                    visible_x1_override,
                )
                .map(Some),
            data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective: Some(connective),
                trailing_selbri,
                ..
            }) => self
                .build_connected_selbri_pair_formula_for_frame(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    frame,
                    source,
                    visible_x1_override,
                )
                .map(Some),
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                trailing_bridi,
                ..
            }) => {
                let Some(leading_selbri) = main_selbri_for_bridi(leading_bridi) else {
                    return Ok(None);
                };
                let Some(trailing_selbri) = main_selbri_for_bridi(trailing_bridi) else {
                    return Ok(None);
                };
                self.build_connected_selbri_pair_formula_for_frame(
                    leading_selbri,
                    guhek,
                    trailing_selbri,
                    frame,
                    source,
                    visible_x1_override,
                )
                .map(Some)
            }
            data!(SelbriSyntax::GroupedSelbri { selbri, .. })
            | data!(SelbriSyntax::TaggedSelbri {
                inner_selbri: selbri,
                ..
            }) => self.build_connected_selbri_formula_for_frame(
                selbri,
                frame,
                source,
                visible_x1_override,
            ),
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_selbri_pair_formula_for_frame(
        &mut self,
        leading_selbri: &'tree SelbriSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_selbri: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let shared_x1 = match visible_x1_override {
            Some(argument) => Some(argument),
            None if self.frame_has_numbered_assignment(frame, 1) => None,
            None => Some(self.build_elided_argument_for_place(1)?),
        };
        let leading = self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
            leading_selbri,
            leading_selbri,
            self.branch_frame_for_selbri(leading_selbri).or(frame),
            source.clone(),
            shared_x1.clone(),
        )?;
        let trailing = self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
            trailing_selbri,
            trailing_selbri,
            self.branch_frame_for_selbri(trailing_selbri).or(frame),
            source.clone(),
            shared_x1,
        )?;
        let formula = self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading.formula, trailing.formula],
            Some(connective_connector(connective, "selbri")),
            source,
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument: leading.x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_selbri_tanru_formula_for_frame(
        &mut self,
        selbri: &'tree SelbriSyntax,
        relation_selbri: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
            selbri,
            relation_selbri,
            frame,
            source,
            None,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_selbri_tanru_formula_for_frame_with_visible_x1_override(
        &mut self,
        selbri: &'tree SelbriSyntax,
        relation_selbri: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        if let Some(connected) = self.build_connected_selbri_formula_for_frame(
            relation_selbri,
            frame,
            source.clone(),
            visible_x1_override.clone(),
        )? {
            return Ok(connected);
        }
        if let Some(units) = tanru_units_for_selbri(relation_selbri)
            && tanru_units_require_lowering(&units)
        {
            let frame = self
                .semantic_predication_frame_for_selbri(relation_selbri, frame)
                .or(frame);
            return self.build_tanru_sequence_formula_for_frame_with_visible_x1_override(
                Some(relation_selbri),
                &units,
                frame,
                source,
                visible_x1_override,
            );
        }
        if let Some(bound_tanru) = connectorless_bound_selbri_pair(relation_selbri) {
            return self.build_bound_selbri_tanru_formula_for_argument(
                selbri,
                bound_tanru.leading,
                bound_tanru.trailing,
                frame,
                source,
            );
        }
        if let data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        }) = relation_selbri.as_data()
        {
            return self.build_inverted_tanru_formula_for_argument(
                selbri,
                leading_selbri,
                trailing_selbri,
                frame,
                source,
                visible_x1_override,
            );
        }
        if let data!(SelbriSyntax::Abstraction(abstraction)) = relation_selbri.as_data() {
            return self.build_abstraction_tanru_unit_formula_for_frame(
                abstraction,
                frame,
                source,
                visible_x1_override,
                PredicationMode::Asserted,
            );
        }
        let visible_x1_place = visible_x1_place_for_selbri(relation_selbri);
        let mut overrides = BTreeMap::new();
        if let Some(argument) = visible_x1_override {
            overrides.insert(format!("x{visible_x1_place}"), argument);
        }
        let predication = self.build_predication_for_frame_with_overrides(
            self.semantic_predication_frame_for_selbri(relation_selbri, frame),
            source.clone(),
            Some(selbri),
            relation_label_for_selbri(relation_selbri),
            overrides,
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        let x1_argument = self.predication_argument(predication, visible_x1_place)?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_inverted_tanru_formula_for_argument(
        &mut self,
        selbri: &'tree SelbriSyntax,
        tertau: &'tree SelbriSyntax,
        seltau: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let tertau_formula = self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
            selbri,
            tertau,
            self.branch_frame_for_selbri(tertau).or(frame),
            source.clone(),
            visible_x1_override,
        )?;
        let modifier = self.build_property_abstraction_for_selbri(seltau, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            tertau_formula.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_selbri_pair(seltau, tertau),
            PredicationMode::Asserted,
            source.clone(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau_formula.formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "selbri-inversion".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument: tertau_formula.x1_argument,
        })
    }

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_sequence_formula_for_frame(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        units: &[&'tree TanruUnitSyntax],
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        self.build_tanru_sequence_formula_for_frame_with_visible_x1_override(
            selbri, units, frame, source, None,
        )
    }

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_sequence_formula_for_frame_with_visible_x1_override(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        units: &[&'tree TanruUnitSyntax],
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        if let [single] = units {
            return self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
                selbri,
                single,
                frame,
                source,
                visible_x1_override,
            );
        }
        let tertau = units
            .last()
            .expect("precondition guarantees at least one tanru unit");
        let tertau = self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
            selbri,
            tertau,
            frame,
            source.clone(),
            visible_x1_override,
        )?;
        let modifier =
            self.build_property_abstraction_for_units(&units[..units.len() - 1], source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            tertau.x1_argument.clone(),
            modifier,
            tanru_relation_name(units),
            PredicationMode::Asserted,
            source.clone(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau.formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "selbri".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument: tertau.x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_unit_formula_for_frame(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
            selbri, unit, frame, source, None,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_unit_formula_for_frame_with_visible_x1_override(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        match unit.as_data() {
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                bo_connective: Some(connective),
                trailing_unit,
                ..
            }) => self.build_connected_tanru_unit_formula_for_frame(
                selbri,
                leading_unit,
                connective,
                trailing_unit,
                frame,
                source,
                visible_x1_override,
            ),
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => self.build_connected_tanru_unit_formula_for_frame(
                selbri,
                leading_unit,
                connective,
                trailing_unit,
                frame,
                source,
                visible_x1_override,
            ),
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                let tertau = self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
                    selbri,
                    trailing_unit,
                    frame,
                    source.clone(),
                    visible_x1_override,
                )?;
                let modifier =
                    self.build_property_abstraction_for_tanru_unit(leading_unit, source.clone())?;
                let relation_formula = self.build_tanru_relation_formula(
                    tertau.x1_argument.clone(),
                    modifier,
                    tanru_unit_relation_name(unit),
                    PredicationMode::Asserted,
                    source.clone(),
                )?;
                let formula = self.next_formula();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        FormulaOperator::And,
                        vec![tertau.formula, relation_formula],
                        Some(Connector {
                            source: "tanru".to_owned(),
                            locus: "tanru-unit".to_owned(),
                            truth_table: None,
                            parameter: None,
                        }),
                        source,
                        Vec::new(),
                    ),
                )?;
                Ok(TanruFormulaForArgument {
                    formula,
                    x1_argument: tertau.x1_argument,
                })
            }
            data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => self
                .build_scalar_negated_tanru_unit_formula_for_frame(
                    selbri,
                    unit,
                    nahe,
                    inner_unit,
                    frame,
                    source,
                    visible_x1_override,
                ),
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
                if let Some(units) = tanru_units_for_selbri(selbri)
                    && !units.is_empty()
                {
                    return self.build_tanru_sequence_formula_for_frame_with_visible_x1_override(
                        Some(selbri.as_ref()),
                        &units,
                        frame,
                        source,
                        visible_x1_override,
                    );
                }
                self.build_selbri_tanru_formula_for_frame_with_visible_x1_override(
                    selbri.as_ref(),
                    selbri.as_ref(),
                    self.branch_frame_for_selbri(selbri).or(frame),
                    source,
                    visible_x1_override,
                )
            }
            data!(TanruUnitSyntax::SumtiSelbri { sumti, .. }) => self
                .build_sumti_selbri_formula_for_frame(
                    sumti,
                    frame,
                    source,
                    visible_x1_override,
                    PredicationMode::Asserted,
                ),
            data!(TanruUnitSyntax::Abstraction(abstraction)) => self
                .build_abstraction_tanru_unit_formula_for_frame(
                    abstraction,
                    frame,
                    source,
                    visible_x1_override,
                    PredicationMode::Asserted,
                ),
            _ => self.build_simple_tanru_unit_formula_for_frame(
                selbri,
                unit,
                frame,
                source,
                visible_x1_override,
            ),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_tanru_unit_formula_for_frame(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        leading_unit: &'tree TanruUnitSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let shared_x1 = match visible_x1_override {
            Some(argument) => Some(argument),
            None if self.frame_has_numbered_assignment(frame, 1) => None,
            None => Some(self.build_elided_argument_for_place(1)?),
        };
        let leading = self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
            selbri,
            leading_unit,
            self.branch_frame_for_tanru_unit(leading_unit).or(frame),
            source.clone(),
            shared_x1.clone(),
        )?;
        let trailing = self.build_tanru_unit_formula_for_frame_with_visible_x1_override(
            selbri,
            trailing_unit,
            self.branch_frame_for_tanru_unit(trailing_unit).or(frame),
            source.clone(),
            shared_x1,
        )?;
        let formula = self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading.formula, trailing.formula],
            Some(connective_connector(connective, "tanru-unit")),
            source,
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument: leading.x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_simple_tanru_unit_formula_for_frame(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let visible_x1_place = visible_x1_place_for_tanru_unit(unit);
        let mut overrides = BTreeMap::new();
        if let Some(argument) = visible_x1_override {
            overrides.insert(format!("x{visible_x1_place}"), argument);
        }
        if let Some(relation_parameter) =
            self.build_relation_question_parameter_for_tanru_unit(unit)?
        {
            let predication = self.build_relation_parameter_predication_for_frame_with_overrides(
                self.semantic_predication_frame_for_tanru_unit(unit, frame),
                source.clone(),
                selbri,
                relation_parameter,
                overrides,
            )?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::atom_formula(predication, source, Vec::new()),
            )?;
            let x1_argument = self.predication_argument(predication, visible_x1_place)?;
            return Ok(TanruFormulaForArgument {
                formula,
                x1_argument,
            });
        }
        let predication = self.build_predication_for_frame_with_overrides(
            self.semantic_predication_frame_for_tanru_unit(unit, frame),
            source.clone(),
            selbri,
            relation_label_for_tanru_unit(unit),
            overrides,
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        let x1_argument = self.predication_argument(predication, visible_x1_place)?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_sumti_selbri_formula_for_frame(
        &mut self,
        sumti: &'tree SumtiSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
        mode: PredicationMode,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let eventuality = self.next_eventuality();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
        )?;
        let source_operand = if lerfu_string_sumti_letters(sumti).is_some() {
            self.build_letteral_sign_for_sumti(sumti)?
        } else {
            self.build_sumti_referent(sumti)?
        };
        let mut arguments = BTreeMap::new();
        self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        if let Some(argument) = visible_x1_override {
            arguments.insert("x1".to_owned(), argument);
        }
        if !arguments.contains_key("x1") {
            arguments.insert("x1".to_owned(), self.build_elided_argument_for_place(1)?);
        }
        arguments.insert("x2".to_owned(), ArgumentValue::filled(source_operand, None));
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            "referentOf".to_owned(),
            Some(eventuality),
            arguments,
            mode,
            source.clone(),
            Vec::new(),
        );
        object.modal_arguments = modal_arguments;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        let x1_argument = self.predication_argument(predication, 1)?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_abstraction_tanru_unit_formula_for_frame(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
        mode: PredicationMode,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let kind = abstraction_kind_for_nu(abstraction);
        let x1_argument = if let Some(argument) = visible_x1_override {
            argument
        } else if let Some(frame) = frame {
            match self.numbered_assignment_argument_for_frame(frame, 1)? {
                Some(argument) => argument,
                None => self
                    .build_elided_argument_for_place_with_sort(1, abstraction_output_sort(kind))?,
            }
        } else {
            self.build_elided_argument_for_place_with_sort(1, abstraction_output_sort(kind))?
        };
        let formula = self.build_abstraction_link_formula_for_argument(
            abstraction,
            kind,
            x1_argument.clone(),
            frame,
            source,
            mode,
        )?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_scalar_negated_tanru_unit_formula_for_frame(
        &mut self,
        selbri: Option<&'tree SelbriSyntax>,
        unit: &'tree TanruUnitSyntax,
        marker: &WithFreeModifiers<Token>,
        inner_unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        visible_x1_override: Option<ArgumentValue>,
    ) -> Result<TanruFormulaForArgument, SemanticsError> {
        let visible_x1_place = visible_x1_place_for_tanru_unit(inner_unit);
        let mut overrides = BTreeMap::new();
        if let Some(argument) = visible_x1_override {
            overrides.insert(format!("x{visible_x1_place}"), argument);
        }
        let predication = self.build_predication_for_frame_with_overrides(
            self.semantic_predication_frame_for_tanru_unit(unit, frame),
            source.clone(),
            selbri,
            relation_label_for_tanru_unit(inner_unit),
            overrides,
        )?;
        self.set_scalar_negation(predication, scalar_negation_for_marker(marker));
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        let x1_argument = self.predication_argument(predication, visible_x1_place)?;
        Ok(TanruFormulaForArgument {
            formula,
            x1_argument,
        })
    }

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_abstraction_for_units(
        &mut self,
        units: &[&'tree TanruUnitSyntax],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let [single] = units
            && let data!(TanruUnitSyntax::Abstraction(abstraction)) = single.as_data()
        {
            return self.build_property_abstraction_for_abstraction_tanru_unit(abstraction, source);
        }
        if let [single] = units
            && let Some(composition) =
                self.build_property_composition_for_tanru_unit(single, source.clone())?
        {
            return Ok(composition);
        }
        let parameter = self.next_parameter();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                crate::model::ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_property_formula_for_units(units, parameter, source.clone())?;
        let abstraction = self.next_abstraction();
        self.insert(
            abstraction,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                vec![parameter],
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Abstraction) || ret.is_err())]
    fn build_property_abstraction_for_abstraction_tanru_unit(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let kind = abstraction_kind_for_nu(abstraction);
        let parameter = self.next_parameter();
        self.insert(
            parameter,
            SemanticObject::parameter(
                abstraction_output_sort(kind),
                crate::model::ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_abstraction_link_formula_for_argument(
            abstraction,
            kind,
            ArgumentValue::filled(parameter, None),
            None,
            source.clone(),
            PredicationMode::Restrictive,
        )?;
        let property = self.next_abstraction();
        self.insert(
            property,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                vec![parameter],
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_composition_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match unit.as_data() {
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) if !connective_is_logical(connective) => self
                .build_property_composition_for_tanru_unit_pair(
                    leading_unit,
                    connective,
                    trailing_unit,
                    source,
                )
                .map(Some),
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                bo_connective: Some(connective),
                trailing_unit,
                ..
            }) if !connective_is_logical(connective) => self
                .build_property_composition_for_tanru_unit_pair(
                    leading_unit,
                    connective.as_ref(),
                    trailing_unit,
                    source,
                )
                .map(Some),
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
                if let Some(units) = tanru_units_for_selbri(selbri)
                    && let [single] = units.as_slice()
                {
                    return self.build_property_composition_for_tanru_unit(single, source);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_composition_for_tanru_unit_pair(
        &mut self,
        leading_unit: &'tree TanruUnitSyntax,
        connective: &ConnectiveSyntax,
        trailing_unit: &'tree TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading =
            self.build_property_abstraction_for_tanru_unit(leading_unit, source.clone())?;
        let trailing =
            self.build_property_abstraction_for_tanru_unit(trailing_unit, source.clone())?;
        let operator = nonlogical_composition_operator(connective);
        let collective = (operator == "mass").then_some(true);
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Composite,
                SemanticSort::Concept,
                None,
                None,
                Some(new!(Composition {
                    operator,
                    operator_parameter: None,
                    members: vec![leading, trailing],
                    excluded_members: Vec::new(),
                    collective,
                    scalar_negated: None,
                    complement: None,
                    endpoint_inclusion: interval_endpoint_inclusion(connective, false),
                })),
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_formula_for_units(
        &mut self,
        units: &[&'tree TanruUnitSyntax],
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let [single] = units {
            return self.build_property_formula_for_tanru_unit(single, parameter, source);
        }
        let tertau = units
            .last()
            .expect("precondition guarantees at least one tanru unit");
        let tertau_formula =
            self.build_property_formula_for_tanru_unit(tertau, parameter, source.clone())?;
        let modifier =
            self.build_property_abstraction_for_units(&units[..units.len() - 1], source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            ArgumentValue::filled(parameter, None),
            modifier,
            tanru_relation_name(units),
            PredicationMode::Restrictive,
            source.clone(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau_formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "property-abstraction".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_abstraction_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let units = [unit];
        self.build_property_abstraction_for_units(&units, source)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_abstraction_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let data!(SelbriSyntax::Abstraction(abstraction)) = selbri.as_data() {
            return self.build_property_abstraction_for_abstraction_tanru_unit(abstraction, source);
        }
        let parameter = self.next_parameter();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                crate::model::ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_property_formula_for_selbri(selbri, parameter, source.clone())?;
        let abstraction = self.next_abstraction();
        self.insert(
            abstraction,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                vec![parameter],
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(formula) =
            self.build_connected_property_formula_for_selbri(selbri, parameter, source.clone())?
        {
            return Ok(formula);
        }
        if let Some(units) = tanru_units_for_selbri(selbri)
            && tanru_units_require_lowering(&units)
        {
            return self.build_property_formula_for_units(&units, parameter, source);
        }
        if let Some(bound_tanru) = connectorless_bound_selbri_pair(selbri) {
            let tertau_formula = self.build_property_formula_for_selbri(
                bound_tanru.trailing,
                parameter,
                source.clone(),
            )?;
            let modifier =
                self.build_property_abstraction_for_selbri(bound_tanru.leading, source.clone())?;
            let relation_formula = self.build_tanru_relation_formula(
                ArgumentValue::filled(parameter, None),
                modifier,
                tanru_relation_name_for_selbri_pair(bound_tanru.leading, bound_tanru.trailing),
                PredicationMode::Restrictive,
                source.clone(),
            )?;
            let formula = self.next_formula();
            return self.insert(
                formula,
                SemanticObject::connective_formula(
                    FormulaOperator::And,
                    vec![tertau_formula, relation_formula],
                    Some(Connector {
                        source: "tanru".to_owned(),
                        locus: "property-abstraction".to_owned(),
                        truth_table: None,
                        parameter: None,
                    }),
                    source,
                    Vec::new(),
                ),
            );
        }
        if let data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        }) = selbri.as_data()
        {
            let tertau_formula =
                self.build_property_formula_for_selbri(leading_selbri, parameter, source.clone())?;
            let modifier =
                self.build_property_abstraction_for_selbri(trailing_selbri, source.clone())?;
            let relation_formula = self.build_tanru_relation_formula(
                ArgumentValue::filled(parameter, None),
                modifier,
                tanru_relation_name_for_selbri_pair(trailing_selbri, leading_selbri),
                PredicationMode::Restrictive,
                source.clone(),
            )?;
            let formula = self.next_formula();
            return self.insert(
                formula,
                SemanticObject::connective_formula(
                    FormulaOperator::And,
                    vec![tertau_formula, relation_formula],
                    Some(Connector {
                        source: "tanru".to_owned(),
                        locus: "property-inversion".to_owned(),
                        truth_table: None,
                        parameter: None,
                    }),
                    source,
                    Vec::new(),
                ),
            );
        }
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        if let data!(SelbriSyntax::Abstraction(abstraction)) = selbri.as_data() {
            return self.build_abstraction_link_formula_for_argument(
                abstraction,
                abstraction_kind_for_nu(abstraction),
                ArgumentValue::filled(parameter, None),
                frame,
                source,
                PredicationMode::Restrictive,
            );
        }
        if selbri_is_single_relation_question(selbri)
            && let Some(relation_parameter) =
                self.build_relation_question_parameter_for_selbri(selbri)?
        {
            return self.build_property_atom_for_relation_parameter(
                relation_parameter,
                parameter,
                source,
                frame,
                visible_x1_place_for_selbri(selbri),
            );
        }
        let relation = relation_label_for_selbri(selbri);
        let relation_metadata =
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?;
        self.build_property_atom_for_relation(
            relation,
            relation_metadata,
            parameter,
            source,
            frame,
            visible_x1_place_for_selbri(selbri),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_property_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::SelbriConnection {
                leading_selbri,
                connective,
                trailing_selbri,
            }) => self
                .build_connected_property_formula_for_selbri_pair(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    parameter,
                    source,
                )
                .map(Some),
            data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective: Some(connective),
                trailing_selbri,
                ..
            }) => self
                .build_connected_property_formula_for_selbri_pair(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    parameter,
                    source,
                )
                .map(Some),
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                trailing_bridi,
                ..
            }) => {
                let Some(leading_selbri) = main_selbri_for_bridi(leading_bridi) else {
                    return Ok(None);
                };
                let Some(trailing_selbri) = main_selbri_for_bridi(trailing_bridi) else {
                    return Ok(None);
                };
                self.build_connected_property_formula_for_selbri_pair(
                    leading_selbri,
                    guhek,
                    trailing_selbri,
                    parameter,
                    source,
                )
                .map(Some)
            }
            data!(SelbriSyntax::GroupedSelbri { selbri, .. })
            | data!(SelbriSyntax::TaggedSelbri {
                inner_selbri: selbri,
                ..
            }) => self.build_connected_property_formula_for_selbri(selbri, parameter, source),
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_property_formula_for_selbri_pair(
        &mut self,
        leading_selbri: &'tree SelbriSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_selbri: &'tree SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading =
            self.build_property_formula_for_selbri(leading_selbri, parameter, source.clone())?;
        let trailing =
            self.build_property_formula_for_selbri(trailing_selbri, parameter, source.clone())?;
        self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading, trailing],
            Some(connective_connector(connective, "property-abstraction")),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_formula_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match unit.as_data() {
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                bo_connective: Some(connective),
                trailing_unit,
                ..
            }) => self.build_connected_property_formula_for_tanru_units(
                leading_unit,
                connective,
                trailing_unit,
                parameter,
                source,
            ),
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => self.build_connected_property_formula_for_tanru_units(
                leading_unit,
                connective,
                trailing_unit,
                parameter,
                source,
            ),
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                let tertau_formula = self.build_property_formula_for_tanru_unit(
                    trailing_unit,
                    parameter,
                    source.clone(),
                )?;
                let modifier =
                    self.build_property_abstraction_for_tanru_unit(leading_unit, source.clone())?;
                let relation_formula = self.build_tanru_relation_formula(
                    ArgumentValue::filled(parameter, None),
                    modifier,
                    tanru_unit_relation_name(unit),
                    PredicationMode::Restrictive,
                    source.clone(),
                )?;
                let formula = self.next_formula();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        FormulaOperator::And,
                        vec![tertau_formula, relation_formula],
                        Some(Connector {
                            source: "tanru".to_owned(),
                            locus: "property-abstraction".to_owned(),
                            truth_table: None,
                            parameter: None,
                        }),
                        source,
                        Vec::new(),
                    ),
                )
            }
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
                if let Some(units) = tanru_units_for_selbri(selbri)
                    && !units.is_empty()
                {
                    return self.build_property_formula_for_units(&units, parameter, source);
                }
                self.build_property_formula_for_selbri(selbri, parameter, source)
            }
            data!(TanruUnitSyntax::SumtiSelbri { sumti, .. }) => self
                .build_sumti_selbri_formula_for_frame(
                    sumti,
                    self.branch_frame_for_tanru_unit(unit),
                    source,
                    Some(ArgumentValue::filled(parameter, None)),
                    PredicationMode::Restrictive,
                )
                .map(|result| result.formula),
            data!(TanruUnitSyntax::Abstraction(abstraction)) => self
                .build_abstraction_link_formula_for_argument(
                    abstraction,
                    abstraction_kind_for_nu(abstraction),
                    ArgumentValue::filled(parameter, None),
                    self.branch_frame_for_tanru_unit(unit),
                    source,
                    PredicationMode::Restrictive,
                ),
            data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => {
                let frame = self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit),
                );
                self.build_property_atom_for_relation_with_scalar_negation(
                    relation_label_for_tanru_unit(inner_unit),
                    None,
                    parameter,
                    source,
                    frame,
                    visible_x1_place_for_tanru_unit(inner_unit),
                    Some(scalar_negation_for_marker(nahe)),
                )
            }
            _ => {
                let frame = self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit),
                );
                if let Some(relation_parameter) =
                    self.build_relation_question_parameter_for_tanru_unit(unit)?
                {
                    return self.build_property_atom_for_relation_parameter(
                        relation_parameter,
                        parameter,
                        source,
                        frame,
                        visible_x1_place_for_tanru_unit(unit),
                    );
                }
                self.build_property_atom_for_relation(
                    relation_label_for_tanru_unit(unit),
                    None,
                    parameter,
                    source,
                    frame,
                    visible_x1_place_for_tanru_unit(unit),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_property_formula_for_tanru_units(
        &mut self,
        leading_unit: &'tree TanruUnitSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_unit: &'tree TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading =
            self.build_property_formula_for_tanru_unit(leading_unit, parameter, source.clone())?;
        let trailing =
            self.build_property_formula_for_tanru_unit(trailing_unit, parameter, source.clone())?;
        self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading, trailing],
            Some(connective_connector(connective, "property-abstraction")),
            source,
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(relation_metadata.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::RelationMetadata))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_atom_for_relation(
        &mut self,
        relation: String,
        relation_metadata: Option<SemanticObjectId>,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        frame: Option<SelbriPlaceFrameId>,
        visible_x1_place: usize,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_atom_for_relation_with_scalar_negation(
            relation,
            relation_metadata,
            parameter,
            source,
            frame,
            visible_x1_place,
            None,
        )
    }

    #[requires(relation_parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_atom_for_relation_parameter(
        &mut self,
        relation_parameter: SemanticObjectId,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        frame: Option<SelbriPlaceFrameId>,
        visible_x1_place: usize,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
        )?;
        let mut arguments = BTreeMap::new();
        let mut highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        highest_assigned_place = highest_assigned_place.max(visible_x1_place);
        arguments.insert(
            format!("x{visible_x1_place}"),
            ArgumentValue::filled(parameter, None),
        );
        for place in 1..=highest_assigned_place.max(1) {
            let key = format!("x{place}");
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::relation_parameter_predication(
            relation_parameter,
            Some(eventuality),
            arguments,
            PredicationMode::Restrictive,
            source.clone(),
            Vec::new(),
        );
        object.modal_arguments = modal_arguments;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(relation_metadata.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::RelationMetadata))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_atom_for_relation_with_scalar_negation(
        &mut self,
        relation: String,
        relation_metadata: Option<SemanticObjectId>,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        frame: Option<SelbriPlaceFrameId>,
        visible_x1_place: usize,
        scalar_negation: Option<ScalarNegation>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
        )?;
        let mut arguments = BTreeMap::new();
        let mut highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        highest_assigned_place = highest_assigned_place.max(visible_x1_place);
        arguments.insert(
            format!("x{visible_x1_place}"),
            ArgumentValue::filled(parameter, None),
        );
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(1) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only explicit assigned places are represented",
                    ));
                }
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            PredicationMode::Restrictive,
            source.clone(),
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.relation_metadata = relation_metadata;
        object.scalar_negation = scalar_negation;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_relation_formula(
        &mut self,
        x1_argument: ArgumentValue,
        modifier: SemanticObjectId,
        relation: String,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), x1_argument);
        arguments.insert("x2".to_owned(), ArgumentValue::filled(modifier, None));
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                mode,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let frame = self.bridi_frame(bridi);
        let frame = selbri
            .and_then(|selbri| self.semantic_predication_frame_for_selbri(selbri, frame))
            .or(frame);
        let predication = self.build_predication_for_frame(
            frame,
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "predication")),
            selbri,
            relation,
        )?;
        self.attach_reciprocity_to_predication(predication, bridi, &[])?;
        Ok(predication)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_question_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation_parameter = self
            .build_relation_question_parameter_for_selbri(selbri)?
            .ok_or_else(SemanticsError::missing_syntax_node)?;
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.bridi_frame(bridi))
            .or_else(|| self.bridi_frame(bridi));
        let source = self
            .analysis
            .syntax_index
            .bridi_node_id(bridi)
            .and_then(|node| self.source_for_node(node.0, "relation-question-formula"));
        let predication = self.build_relation_parameter_predication_for_frame_with_overrides(
            frame,
            source.clone(),
            Some(selbri),
            relation_parameter,
            BTreeMap::new(),
        )?;
        self.attach_reciprocity_to_predication(predication, bridi, &[])?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_variable_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation_parameter = self
            .build_relation_variable_parameter_for_selbri(selbri)?
            .ok_or_else(SemanticsError::missing_syntax_node)?;
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.bridi_frame(bridi))
            .or_else(|| self.bridi_frame(bridi));
        let source = self
            .analysis
            .syntax_index
            .bridi_node_id(bridi)
            .and_then(|node| self.source_for_node(node.0, "relation-variable-formula"));
        let predication = self.build_relation_parameter_predication_for_frame_with_overrides(
            frame,
            source.clone(),
            Some(selbri),
            relation_parameter,
            BTreeMap::new(),
        )?;
        self.attach_reciprocity_to_predication(predication, bridi, &[])?;
        let atom = self.next_formula();
        self.insert(
            atom,
            SemanticObject::atom_formula(predication, source.clone(), Vec::new()),
        )?;
        let scoped = self.next_formula();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                FormulaOperator::Exists,
                relation_parameter,
                None,
                atom,
                None,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "quantifier-scope")),
                Vec::new(),
            ),
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_reciprocity_to_predication(
        &mut self,
        predication: SemanticObjectId,
        bridi: &'tree BridiSyntax,
        extra_free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        let mut exchanges = Vec::new();
        self.collect_reciprocal_exchanges_from_bridi(bridi, predication, &mut exchanges)?;
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            extra_free_modifiers,
            None,
            predication,
            &mut exchanges,
        )?;
        self.append_reciprocal_exchanges_to_predication(predication, exchanges)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_reciprocity_free_modifiers_to_predication(
        &mut self,
        predication: SemanticObjectId,
        bridi: &'tree BridiSyntax,
        free_modifiers: &'tree [FreeModifierSyntax],
        host_sumti: Option<&'tree SumtiSyntax>,
    ) -> Result<(), SemanticsError> {
        let mut exchanges = Vec::new();
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            free_modifiers,
            host_sumti,
            predication,
            &mut exchanges,
        )?;
        self.append_reciprocal_exchanges_to_predication(predication, exchanges)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn append_reciprocal_exchanges_to_predication(
        &mut self,
        predication: SemanticObjectId,
        exchanges: Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        if exchanges.is_empty() {
            return Ok(());
        }
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            SemanticsError::invalid_graph(format!(
                "semantic builder could not find reciprocal predication {predication}"
            ))
        })?;
        object.reciprocity.extend(exchanges);
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_terms(
            &bridi.leading_terms,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_bridi_tail(
            &bridi.bridi_tail,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            &bridi.free_modifiers,
            None,
            predication,
            out,
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_bridi_tail(
        &mut self,
        tail: &'tree BridiTailSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_afterthought_bridi_tail(
            &tail.first,
            bridi,
            predication,
            out,
        )?;
        if let Some(continuation) = &tail.ke_continuation {
            self.collect_reciprocal_exchanges_from_grouped_bridi_tail_connection(
                continuation,
                bridi,
                predication,
                out,
            )?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_grouped_bridi_tail_connection(
        &mut self,
        continuation: &'tree GroupedBridiTailConnectionSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_bridi_tail(
            &continuation.bridi_tail,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_terms(
            &continuation.tail_terms,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            &continuation.free_modifiers,
            None,
            predication,
            out,
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_afterthought_bridi_tail(
        &mut self,
        tail: &'tree AfterthoughtBridiTailSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_bo_grouped_bridi_tail(
            &tail.first,
            bridi,
            predication,
            out,
        )?;
        for continuation in &tail.continuations {
            self.collect_reciprocal_exchanges_from_bridi_tail_connection(
                continuation,
                bridi,
                predication,
                out,
            )?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_bridi_tail_connection(
        &mut self,
        continuation: &'tree BridiTailConnectionSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_bo_grouped_bridi_tail(
            &continuation.bridi_tail,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_terms(
            &continuation.tail_terms,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            &continuation.free_modifiers,
            None,
            predication,
            out,
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_bo_grouped_bridi_tail(
        &mut self,
        tail: &'tree BoGroupedBridiTailSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_simple_bridi_tail(
            &tail.first,
            bridi,
            predication,
            out,
        )?;
        if let Some(continuation) = &tail.bo_continuation {
            self.collect_reciprocal_exchanges_from_bound_bridi_tail_connection(
                continuation,
                bridi,
                predication,
                out,
            )?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_bound_bridi_tail_connection(
        &mut self,
        continuation: &'tree BoundBridiTailConnectionSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        self.collect_reciprocal_exchanges_from_bo_grouped_bridi_tail(
            &continuation.bridi_tail,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_terms(
            &continuation.tail_terms,
            bridi,
            predication,
            out,
        )?;
        self.collect_reciprocal_exchanges_from_free_modifiers(
            bridi,
            &continuation.free_modifiers,
            None,
            predication,
            out,
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_simple_bridi_tail(
        &mut self,
        tail: &'tree SimpleBridiTailSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match tail.as_data() {
            data!(SimpleBridiTailSyntax::SelbriBridiTail {
                terms,
                free_modifiers,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_terms(terms, bridi, predication, out)?;
                self.collect_reciprocal_exchanges_from_free_modifiers(
                    bridi,
                    free_modifiers,
                    None,
                    predication,
                    out,
                )
            }
            data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
                connection
            )) => self.collect_reciprocal_exchanges_from_forethought_bridi_connection(
                connection,
                bridi,
                predication,
                out,
            ),
            data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => {
                self.collect_reciprocal_exchanges_from_terms(terms, bridi, predication, out)?;
                self.collect_reciprocal_exchanges_from_bridi_tail(
                    bridi_tail,
                    bridi,
                    predication,
                    out,
                )
            }
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_forethought_bridi_connection(
        &mut self,
        connection: &'tree ForethoughtBridiConnectionSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match connection.as_data() {
            data!(ForethoughtBridiConnectionSyntax::BridiConnection {
                tail_terms,
                free_modifiers,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_terms(tail_terms, bridi, predication, out)?;
                self.collect_reciprocal_exchanges_from_free_modifiers(
                    bridi,
                    free_modifiers,
                    None,
                    predication,
                    out,
                )
            }
            data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection { inner, .. })
            | data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => self
                .collect_reciprocal_exchanges_from_forethought_bridi_connection(
                    inner,
                    bridi,
                    predication,
                    out,
                ),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_terms(
        &mut self,
        terms: &'tree [TermSyntax],
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        for term in terms {
            self.collect_reciprocal_exchanges_from_term(term, bridi, predication, out)?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_term(
        &mut self,
        term: &'tree TermSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match term.as_data() {
            data!(TermSyntax::Termset { termset, .. }) => {
                self.collect_reciprocal_exchanges_from_terms(termset, bridi, predication, out)
            }
            data!(TermSyntax::ForethoughtTermsetConnection {
                terms,
                gik_terms,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_terms(terms, bridi, predication, out)?;
                self.collect_reciprocal_exchanges_from_terms(gik_terms, bridi, predication, out)
            }
            data!(TermSyntax::TermsetGroup {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::TermsetConnection {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::TermConnection {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_terms(
                    leading_terms,
                    bridi,
                    predication,
                    out,
                )?;
                self.collect_reciprocal_exchanges_from_terms(
                    trailing_terms,
                    bridi,
                    predication,
                    out,
                )
            }
            data!(TermSyntax::BoundTermConnection {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_terms(
                    leading_terms,
                    bridi,
                    predication,
                    out,
                )?;
                self.collect_reciprocal_exchanges_from_term(trailing_term, bridi, predication, out)
            }
            data!(TermSyntax::Sumti(sumti))
            | data!(TermSyntax::PlaceTaggedSumti { sumti, .. })
            | data!(TermSyntax::JaiTaggedSumti { sumti, .. })
            | data!(TermSyntax::TaggedSumti { sumti, .. }) => {
                self.collect_reciprocal_exchanges_from_sumti(sumti, bridi, predication, out)
            }
            _ => Ok(()),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match sumti.as_data() {
            data!(SumtiSyntax::ProSumti(token)) => self
                .collect_reciprocal_exchanges_from_free_modifiers(
                    bridi,
                    &token.free_modifiers,
                    Some(sumti),
                    predication,
                    out,
                ),
            data!(SumtiSyntax::ElidedSumti { free_modifiers, .. }) => self
                .collect_reciprocal_exchanges_from_free_modifiers(
                    bridi,
                    free_modifiers,
                    Some(sumti),
                    predication,
                    out,
                ),
            data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::SumtiWithRelativeClauses {
                base_sumti: inner_sumti,
                ..
            })
            | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                base_sumti: inner_sumti,
                ..
            })
            | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
            | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::ReferentSumti { inner_sumti, .. })
            | data!(SumtiSyntax::GroupedSumti { inner_sumti, .. }) => {
                self.collect_reciprocal_exchanges_from_sumti(inner_sumti, bridi, predication, out)
            }
            data!(SumtiSyntax::QualifiedTerm { inner_term, .. }) => {
                self.collect_reciprocal_exchanges_from_term(inner_term, bridi, predication, out)
            }
            data!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                trailing_sumti,
                ..
            })
            | data!(SumtiSyntax::BoundSumtiConnection {
                leading_sumti,
                trailing_sumti,
                ..
            })
            | data!(SumtiSyntax::ForethoughtSumtiConnection {
                leading_sumti,
                trailing_sumti,
                ..
            }) => {
                self.collect_reciprocal_exchanges_from_sumti(
                    leading_sumti,
                    bridi,
                    predication,
                    out,
                )?;
                self.collect_reciprocal_exchanges_from_sumti(
                    trailing_sumti,
                    bridi,
                    predication,
                    out,
                )
            }
            _ => Ok(()),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_reciprocal_exchanges_from_free_modifiers(
        &mut self,
        bridi: &'tree BridiSyntax,
        free_modifiers: &'tree [FreeModifierSyntax],
        host_sumti: Option<&'tree SumtiSyntax>,
        predication: SemanticObjectId,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        for free_modifier in free_modifiers {
            let data!(FreeModifierSyntax::ReciprocalSumti {
                leading_sumti,
                trailing_sumti,
                ..
            }) = free_modifier.as_data()
            else {
                continue;
            };
            let left =
                self.build_reciprocal_argument_for_sumti(bridi, predication, leading_sumti)?;
            let right = if let Some(trailing_sumti) = trailing_sumti {
                self.build_reciprocal_argument_for_sumti(bridi, predication, trailing_sumti)?
            } else if let Some(host_sumti) = host_sumti {
                self.build_reciprocal_argument_for_sumti(bridi, predication, host_sumti)?
            } else {
                self.add_object_diagnostic(
                    predication,
                    diagnostic(
                        "soi with one explicit participant has no preceding sumti in this position",
                    ),
                );
                continue;
            };
            if left.kind == crate::model::ArgumentValueKind::Deleted
                || right.kind == crate::model::ArgumentValueKind::Deleted
            {
                self.add_object_diagnostic(
                    predication,
                    diagnostic("soi reciprocity participant was deleted; exchange omitted"),
                );
                continue;
            }
            out.push(ReciprocalExchange::new(
                left,
                right,
                "soi".to_owned(),
                self.source_for_free_modifier(free_modifier, "reciprocity"),
            ));
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_reciprocal_argument_for_sumti(
        &mut self,
        bridi: &'tree BridiSyntax,
        predication: SemanticObjectId,
        sumti: &'tree SumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        if let Some(place) = voha_place_for_sumti(sumti) {
            return self.predication_argument(predication, place);
        }
        if let Some(place) = self.assigned_place_for_sumti(bridi, sumti) {
            return self.predication_argument(predication, place);
        }
        self.build_argument_for_sumti(sumti)
    }

    #[requires(true)]
    #[ensures(true)]
    fn assigned_place_for_sumti(
        &self,
        bridi: &'tree BridiSyntax,
        sumti: &'tree SumtiSyntax,
    ) -> Option<usize> {
        let frame = self.bridi_frame(bridi)?;
        let sumti_node = self.analysis.syntax_index.sumti_node_id(sumti)?.0;
        self.analysis
            .place_analysis
            .assignments_for_frame(frame)
            .iter()
            .filter_map(|assignment_id| self.analysis.place_analysis.assignment(*assignment_id))
            .find_map(|assignment| {
                let PlaceSlot::Numbered(place) = assignment.slot else {
                    return None;
                };
                self.syntax_node_contains(assignment.sumti.0, sumti_node)
                    .then_some(place.get() as usize)
            })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_statement_reciprocity_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        statement: &'tree StatementSyntax,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        if !free_modifiers_have_reciprocity(free_modifiers) {
            return Ok(());
        }
        let Some(bridi) = direct_statement_bridi(statement) else {
            self.add_object_diagnostic(
                item,
                diagnostic("statement-level soi is not lowered for this statement shape"),
            );
            return Ok(());
        };
        let Some(formula) = self.content_formula_for_discourse_item(item) else {
            self.add_object_diagnostic(
                item,
                diagnostic("statement-level soi has no formula-bearing statement to modify"),
            );
            return Ok(());
        };
        self.attach_reciprocity_to_formula(formula, bridi, free_modifiers)
    }

    #[requires(connective_has_logical_component(connective))]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_statement_logical_connection_formula(
        &mut self,
        first_item: SemanticObjectId,
        second_item: SemanticObjectId,
        connective: &ConnectiveSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(first_formula) = self.content_formula_for_discourse_item(first_item) else {
            return Ok(None);
        };
        let Some(second_formula) = self.content_formula_for_discourse_item(second_item) else {
            return Ok(None);
        };
        let first_formula = if connective_negates_left(connective) {
            self.build_unary_formula(
                FormulaOperator::Not,
                first_formula,
                source.clone(),
                Vec::new(),
            )?
        } else {
            first_formula
        };
        let second_formula = if connective_negates_right(connective) {
            self.build_unary_formula(
                FormulaOperator::Not,
                second_formula,
                source.clone(),
                Vec::new(),
            )?
        } else {
            second_formula
        };
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                formula_operator_for_connective(connective),
                vec![first_formula, second_formula],
                Some(Connector {
                    source: full_connective_text(connective),
                    locus: "statement".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )
        .map(Some)
    }

    #[requires(true)]
    #[ensures(true)]
    fn content_formula_for_discourse_item(
        &self,
        item: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&item)?;
        let content = object.content?;
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => Some(content),
            crate::model::SemanticObjectKind::Sequence => self
                .objects
                .get(&content)
                .and_then(|sequence| sequence.content)
                .filter(|content| {
                    content.object_kind() == crate::model::SemanticObjectKind::Formula
                }),
            crate::model::SemanticObjectKind::Question => self
                .objects
                .get(&content)
                .and_then(|question| question.body),
            _ => None,
        }
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.is_some() || !self.objects.contains_key(&item))]
    fn displayed_content_target_for_utterance(
        &self,
        item: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&item)?;
        let content = object.content.unwrap_or(item);
        let content_object = self.objects.get(&content)?;
        if content_object.object_type == crate::model::SemanticObjectKind::Question {
            return content_object.body;
        }
        Some(content)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_leading_indicators_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        indicators: &'tree [Indicator],
        truth_question_consumed: bool,
    ) -> Result<(), SemanticsError> {
        if indicators.is_empty() {
            return Ok(());
        }
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            let first_item = self
                .objects
                .get(&item)
                .and_then(|object| object.items.first().copied());
            if let Some(first_item) = first_item {
                self.attach_leading_indicators_to_discourse_item(
                    first_item,
                    indicators,
                    truth_question_consumed,
                )?;
            }
            return Ok(());
        }
        let parts = leading_indicator_parts(indicators, truth_question_consumed);
        if parts.is_empty() {
            return Ok(());
        }
        let Some(target) = self.displayed_content_target_for_utterance(item) else {
            return Ok(());
        };
        self.attach_indicator_displays(parts, target, item, "indicator")
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_statement_separator_indicators_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        paragraph_statement: &'tree ParagraphStatementSyntax,
    ) -> Result<(), SemanticsError> {
        let Some(i) = &paragraph_statement.i else {
            return Ok(());
        };
        let parts = indicator_parts_for_token(i);
        if parts.is_empty() {
            return Ok(());
        }
        let Some(target) = self.displayed_content_target_for_utterance(item) else {
            return Ok(());
        };
        self.attach_indicator_displays(parts, target, item, "indicator")
    }

    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Utterance || crate::model::argument_object_kind_can_fill(target.object_kind()))]
    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(!source_construct.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_indicator_displays(
        &mut self,
        parts: Vec<IndicatorPart>,
        target: SemanticObjectId,
        anchor: SemanticObjectId,
        source_construct: &str,
    ) -> Result<(), SemanticsError> {
        for draft in indicator_display_drafts(parts) {
            self.insert_indicator_display(draft, target, anchor, source_construct)?;
        }
        Ok(())
    }

    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Utterance || crate::model::argument_object_kind_can_fill(target.object_kind()))]
    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(!source_construct.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_indicator_display(
        &mut self,
        draft: IndicatorDisplayDraft,
        target: SemanticObjectId,
        anchor: SemanticObjectId,
        source_construct: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_display();
        let source = self.source_for_tokens(&draft.source_tokens, source_construct);
        let experiencer = if draft.empathy {
            self.build_elided_referent(None, "dai experiencer".to_owned())?
        } else {
            SemanticObjectId::speaker()
        };
        let family = if draft.question {
            DisplayedContentFamily::QuestionPrompt
        } else {
            draft.family
        };
        let relation = if draft.question {
            attitude_question_relation(&draft.relation)
        } else {
            draft.relation
        };
        let mut object = SemanticObject::displayed_content(
            family,
            relation,
            draft.polarity,
            draft.assertion_effect,
            experiencer,
            target,
            anchor,
            source,
            Vec::new(),
        );
        object.intensity = draft.intensity;
        object.phase = draft.phase;
        object.modifiers = draft.modifiers;
        self.insert(id, object)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(|eventuality| eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality))]
    fn primary_eventuality_for_formula(
        &self,
        formula: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&formula)?;
        if let Some(eventuality) = object.eventuality {
            return Some(eventuality);
        }
        match object.operator.as_ref()?.as_data() {
            data!(SemanticOperator::Formula(FormulaOperator::Atom)) => {
                let predication = self.objects.get(&object.predication?)?;
                (predication.mode == Some(PredicationMode::Asserted))
                    .then_some(predication.eventuality)
                    .flatten()
            }
            data!(SemanticOperator::Formula(_))
                if object
                    .connector
                    .as_ref()
                    .is_some_and(|connector| connector.source == "tanru") =>
            {
                object
                    .children
                    .first()
                    .and_then(|child| self.primary_eventuality_for_formula(*child))
            }
            data!(SemanticOperator::Formula(_)) | data!(SemanticOperator::Math(_)) => None,
        }
    }

    #[requires(matches!(
        content.object_kind(),
        crate::model::SemanticObjectKind::Formula | crate::model::SemanticObjectKind::Sequence
    ))]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality) || ret.is_err())]
    fn reified_eventuality_for_content(
        &mut self,
        content: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(eventuality) = self.content_eventualities.get(&content) {
            return Ok(*eventuality);
        }
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source);
        event.content = Some(content);
        self.insert(eventuality, event)?;
        self.content_eventualities.insert(content, eventuality);
        Ok(eventuality)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality) || ret.is_err())]
    fn modal_eventuality_argument_for_formula(
        &mut self,
        formula: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(eventuality) = self.primary_eventuality_for_formula(formula) {
            return Ok(eventuality);
        }
        self.reified_eventuality_for_content(formula, source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.is_none_or(|eventuality| eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality)) || ret.is_err())]
    fn modal_eventuality_argument_for_discourse_item(
        &mut self,
        item: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(object) = self.objects.get(&item) else {
            return Ok(None);
        };
        let object_kind = object.object_kind();
        let content = object.content;
        match object_kind {
            crate::model::SemanticObjectKind::Sequence => {
                self.reified_eventuality_for_content(item, source).map(Some)
            }
            crate::model::SemanticObjectKind::Utterance => {
                let Some(content) = content else {
                    return Ok(None);
                };
                match content.object_kind() {
                    crate::model::SemanticObjectKind::Formula => self
                        .modal_eventuality_argument_for_formula(content, source)
                        .map(Some),
                    crate::model::SemanticObjectKind::Sequence => self
                        .reified_eventuality_for_content(content, source)
                        .map(Some),
                    crate::model::SemanticObjectKind::Question => {
                        let body = self
                            .objects
                            .get(&content)
                            .and_then(|question| question.body);
                        match body {
                            Some(body) => self
                                .modal_eventuality_argument_for_formula(body, source)
                                .map(Some),
                            None => Ok(None),
                        }
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_modal_statement_connection_claim(
        &mut self,
        leading_item: SemanticObjectId,
        trailing_item: SemanticObjectId,
        spec: &ModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match spec.argument_kind {
            ModalConnectionArgumentKind::Eventuality => {
                let Some(leading_eventuality) = self
                    .modal_eventuality_argument_for_discourse_item(leading_item, source.clone())?
                else {
                    return Ok(None);
                };
                let Some(trailing_eventuality) = self
                    .modal_eventuality_argument_for_discourse_item(trailing_item, source.clone())?
                else {
                    return Ok(None);
                };
                self.build_modal_connection_claim_from_arguments(
                    trailing_eventuality,
                    leading_eventuality,
                    spec,
                    source,
                )
            }
            ModalConnectionArgumentKind::Formula => {
                let Some(leading_formula) = self.content_formula_for_discourse_item(leading_item)
                else {
                    return Ok(None);
                };
                let Some(trailing_formula) = self.content_formula_for_discourse_item(trailing_item)
                else {
                    return Ok(None);
                };
                self.build_modal_formula_connection_claim(
                    trailing_formula,
                    leading_formula,
                    spec,
                    source,
                )
            }
        }
    }

    #[requires(visible_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(other_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_modal_formula_connection_claim(
        &mut self,
        visible_formula: SemanticObjectId,
        other_formula: SemanticObjectId,
        spec: &ModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let (visible_argument, other_argument) = match spec.argument_kind {
            ModalConnectionArgumentKind::Eventuality => {
                let visible_eventuality =
                    self.modal_eventuality_argument_for_formula(visible_formula, source.clone())?;
                let other_eventuality =
                    self.modal_eventuality_argument_for_formula(other_formula, source.clone())?;
                (visible_eventuality, other_eventuality)
            }
            ModalConnectionArgumentKind::Formula => (visible_formula, other_formula),
        };
        self.build_modal_connection_claim_from_arguments(
            visible_argument,
            other_argument,
            spec,
            source,
        )
    }

    #[requires(visible_argument.object_kind() == crate::model::SemanticObjectKind::Eventuality || visible_argument.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(other_argument.object_kind() == crate::model::SemanticObjectKind::Eventuality || other_argument.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_modal_connection_claim_from_arguments(
        &mut self,
        visible_argument: SemanticObjectId,
        other_argument: SemanticObjectId,
        spec: &ModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let other_place = convert_numbered_place(2, spec.visible_place);
        let highest_place = self
            .place_count_for_relation(&spec.relation)
            .unwrap_or(spec.visible_place.max(other_place))
            .max(spec.visible_place)
            .max(other_place);
        let mut arguments = BTreeMap::new();
        arguments.insert(
            format!("x{}", spec.visible_place),
            ArgumentValue::filled(visible_argument, None),
        );
        arguments.insert(
            format!("x{other_place}"),
            ArgumentValue::filled(other_argument, None),
        );
        for place in 1..=highest_place {
            let key = format!("x{place}");
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let predication = self.build_predication_from_arguments(
            spec.relation.clone(),
            None,
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        if let Some(object) = self.objects.get_mut(&predication) {
            object.introduced_by = Some(spec.introduced_by.clone());
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
        .map(Some)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_reciprocity_to_formula(
        &mut self,
        formula: SemanticObjectId,
        bridi: &'tree BridiSyntax,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        let object = self.objects.get(&formula).cloned().ok_or_else(|| {
            SemanticsError::invalid_graph(format!(
                "semantic builder could not find reciprocal formula {formula}"
            ))
        })?;
        if let Some(predication) = object.predication {
            self.attach_reciprocity_free_modifiers_to_predication(
                predication,
                bridi,
                free_modifiers,
                None,
            )?;
        }
        for child in object.children {
            self.attach_reciprocity_to_formula(child, bridi, free_modifiers)?;
        }
        if let Some(body) = object.body {
            self.attach_reciprocity_to_formula(body, bridi, free_modifiers)?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_resolved_pro_bridi_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
        target_bridi: &'tree BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(target_selbri) = main_selbri_for_bridi(target_bridi) else {
            return self
                .build_predication_for_bridi(bridi, Some(selbri), relation_label_for_selbri(selbri))
                .and_then(|predication| {
                    let formula = self.next_formula();
                    self.insert(
                        formula,
                        SemanticObject::atom_formula(
                            predication,
                            self.analysis
                                .syntax_index
                                .bridi_node_id(bridi)
                                .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
                            vec![diagnostic(
                                "resolved pro-bridi target has no direct selbri relation",
                            )],
                        ),
                    )
                });
        };
        let mut inherited_arguments = BTreeMap::new();
        let recursive_source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .map(|id| id.0);
        let (_, skipped_recursive_argument) = self
            .insert_numbered_assignment_arguments_excluding_source(
                &mut inherited_arguments,
                self.bridi_frame(target_bridi),
                recursive_source,
            )?;
        let event_selbri = if selbri_has_event_modifiers(selbri) {
            selbri
        } else {
            target_selbri
        };
        let predication = self.build_predication_for_frame_with_overrides(
            self.bridi_frame(bridi),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "predication")),
            Some(event_selbri),
            relation_label_for_selbri(target_selbri),
            inherited_arguments,
        )?;
        if skipped_recursive_argument && let Some(object) = self.objects.get_mut(&predication) {
            object.diagnostics.push(diagnostic(
                "recursive inherited pro-bridi argument was elided to keep the semantic graph finite",
            ));
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "bridi-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_broda_target_bridi_for_selbri(
        &self,
        selbri: &'tree SelbriSyntax,
    ) -> Option<&'tree BridiSyntax> {
        let raw = self.analysis.syntax_index.selbri_node_id(selbri)?.0;
        self.resolved_target_bridi_for_raw(raw, ReferenceKind::BrodaSeries)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_broda_target_bridi_for_tanru_unit(
        &self,
        unit: &'tree TanruUnitSyntax,
    ) -> Option<&'tree BridiSyntax> {
        let raw = self.analysis.syntax_index.tanru_unit_node_id(unit)?.0;
        self.resolved_target_bridi_for_raw(raw, ReferenceKind::BrodaSeries)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_goha_target_bridi_for_selbri(
        &self,
        selbri: &'tree SelbriSyntax,
    ) -> Option<&'tree BridiSyntax> {
        let raw = self.analysis.syntax_index.selbri_node_id(selbri)?.0;
        self.resolved_target_bridi_for_raw(raw, ReferenceKind::GohaSeries)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_goha_target_bridi_for_tanru_unit(
        &self,
        unit: &'tree TanruUnitSyntax,
    ) -> Option<&'tree BridiSyntax> {
        let raw = self.analysis.syntax_index.tanru_unit_node_id(unit)?.0;
        self.resolved_target_bridi_for_raw(raw, ReferenceKind::GohaSeries)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_target_bridi_for_raw(
        &self,
        raw: RawSyntaxNodeId,
        kind: ReferenceKind,
    ) -> Option<&'tree BridiSyntax> {
        self.resolved_target_raw_for_raw(raw, kind)
            .and_then(|target| self.analysis.syntax_index.bridi(BridiNodeId(target)))
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_target_raw_for_raw(
        &self,
        raw: RawSyntaxNodeId,
        kind: ReferenceKind,
    ) -> Option<RawSyntaxNodeId> {
        self.analysis
            .discourse_references
            .references_from_node(raw)
            .iter()
            .filter_map(|edge_id| self.analysis.discourse_references.edge(*edge_id))
            .filter(|edge| edge.kind == kind)
            .find_map(|edge| match edge.target {
                ReferenceTarget::ResolvedNode(target) => Some(target),
                _ => None,
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn bridi_nodes_equal(&self, left: &'tree BridiSyntax, right: &'tree BridiSyntax) -> bool {
        self.analysis.syntax_index.bridi_node_id(left)
            == self.analysis.syntax_index.bridi_node_id(right)
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_for_frame(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_predication_for_frame_with_overrides(
            frame,
            source,
            selbri,
            relation,
            BTreeMap::new(),
        )
    }

    #[requires(relation_parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_parameter_predication_for_frame_with_overrides(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        selbri: Option<&'tree SelbriSyntax>,
        relation_parameter: SemanticObjectId,
        argument_overrides: BTreeMap<String, ArgumentValue>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source.clone());
        let modifier_application =
            self.apply_ordered_event_modifiers_to_event(frame, selbri, &mut event)?;
        let consumed_terms = modifier_application.consumed_terms.clone();
        self.insert(eventuality, event)?;
        self.clear_sticky_modals_for_selbri_if_needed(selbri);
        let (mut arguments, mut highest_assigned_place, modal_arguments) = self
            .with_temporal_context(eventuality, |builder| {
                let mut arguments = BTreeMap::new();
                let highest_assigned_place = builder
                    .insert_numbered_assignment_arguments_excluding_terms(
                        &mut arguments,
                        frame,
                        &consumed_terms,
                    )?;
                let mut modal_arguments =
                    builder.modal_assignment_arguments_excluding_terms(frame, &consumed_terms)?;
                builder.append_sticky_modal_arguments(&mut modal_arguments);
                Ok((arguments, highest_assigned_place, modal_arguments))
            })?;
        for (place, argument) in argument_overrides {
            if let Some(place_index) = argument_place_index(&place) {
                highest_assigned_place = highest_assigned_place.max(place_index);
            }
            arguments.entry(place).or_insert(argument);
        }
        let place_questions =
            self.place_question_bindings(frame, &arguments, None, highest_assigned_place)?;
        for place in 1..=highest_assigned_place.max(1) {
            let key = format!("x{place}");
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::relation_parameter_predication(
            relation_parameter,
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source,
            Vec::new(),
        );
        object.modal_arguments = modal_arguments;
        object.place_questions = place_questions;
        self.insert(predication, object)
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_for_frame_with_overrides(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
        argument_overrides: BTreeMap<String, ArgumentValue>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_predication_for_frame_with_modal_arguments(
            frame,
            source,
            selbri,
            relation,
            argument_overrides,
            Vec::new(),
            true,
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_for_frame_with_modal_arguments(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
        argument_overrides: BTreeMap<String, ArgumentValue>,
        modal_arguments: Vec<ModalArgument>,
        collect_frame_modal_arguments: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source.clone());
        let modifier_application =
            self.apply_ordered_event_modifiers_to_event(frame, selbri, &mut event)?;
        let consumed_terms = modifier_application.consumed_terms.clone();
        self.apply_story_time_to_event(eventuality, &mut event, modifier_application);
        self.insert(eventuality, event)?;
        self.clear_sticky_modals_for_selbri_if_needed(selbri);
        let (mut arguments, mut highest_assigned_place, modal_arguments) = self
            .with_temporal_context(eventuality, |builder| {
                let mut arguments = BTreeMap::new();
                let highest_assigned_place = builder
                    .insert_numbered_assignment_arguments_excluding_terms(
                        &mut arguments,
                        frame,
                        &consumed_terms,
                    )?;
                let mut modal_arguments = modal_arguments;
                if let Some(selbri) = selbri {
                    modal_arguments.extend(builder.selbri_modal_arguments(selbri)?);
                }
                if collect_frame_modal_arguments {
                    modal_arguments.extend(
                        builder
                            .modal_assignment_arguments_excluding_terms(frame, &consumed_terms)?,
                    );
                }
                builder.append_sticky_modal_arguments(&mut modal_arguments);
                Ok((arguments, highest_assigned_place, modal_arguments))
            })?;
        let place_count = self.place_count_for_relation(&relation);
        for (place, argument) in argument_overrides {
            if let Some(place_index) = argument_place_index(&place) {
                highest_assigned_place = highest_assigned_place.max(place_index);
            }
            arguments.entry(place).or_insert(argument);
        }
        if let Some(selbri) = selbri {
            self.insert_bare_jai_abstraction_argument_for_selbri(
                selbri,
                &mut arguments,
                &mut highest_assigned_place,
            )?;
        }
        let place_questions =
            self.place_question_bindings(frame, &arguments, place_count, highest_assigned_place)?;
        let mut diagnostics = if selbri.is_none() {
            vec![diagnostic("bridi tail has no direct selbri relation")]
        } else {
            Vec::new()
        };
        match place_count {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(1) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let id = self.next_predication();
        let mode = asserted_predication_mode_for_relation(&relation);
        let relation_metadata = if let Some(selbri) = selbri {
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?
        } else {
            None
        };
        let mut object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            mode,
            source,
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.place_questions = place_questions;
        object.relation_metadata = relation_metadata;
        self.insert(id, object)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_bare_jai_abstraction_argument_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        highest_assigned_place: &mut usize,
    ) -> Result<(), SemanticsError> {
        if arguments.contains_key("x1") {
            return Ok(());
        }
        let Some(unit) = bare_jai_conversion_for_selbri(selbri) else {
            return Ok(());
        };
        let Some(frame) = self.branch_frame_for_tanru_unit(unit) else {
            return Ok(());
        };
        let Some(argument) = self.numbered_assignment_argument_for_frame(frame, 1)? else {
            return Ok(());
        };
        let Some(operand) = argument.value else {
            return Ok(());
        };
        let source = self.source_for_tanru_unit(unit, "abstraction-about");
        let referent = self.build_abstraction_about_referent("jai", operand, source)?;
        arguments.insert("x1".to_owned(), ArgumentValue::filled(referent, None));
        *highest_assigned_place = (*highest_assigned_place).max(1);
        Ok(())
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::RelationMetadata)) || ret.is_err())]
    fn build_relation_metadata_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        relation: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(rafsis) = lujvo_rafsi_parts_for_selbri(selbri) else {
            return Ok(None);
        };
        let mut source_words = Vec::new();
        let mut rafsi_bindings = Vec::new();
        for rafsi in &rafsis {
            let Some(source_word) = self.source_word_for_lujvo_rafsi(rafsi) else {
                continue;
            };
            source_words.push(source_word.clone());
            let Some(cmavo) = assignable_koha_cmavo_for_word(&source_word) else {
                continue;
            };
            if let Some(referent) = self.bound_koha_referent(cmavo)? {
                rafsi_bindings.push(RafsiBinding::new(
                    rafsi.clone(),
                    Some(source_word),
                    Some(referent),
                ));
            }
        }
        if rafsi_bindings.is_empty() {
            return Ok(None);
        }
        let id = self.next_relation_metadata();
        self.insert(
            id,
            SemanticObject::relation_metadata(
                relation.to_owned(),
                source_words,
                Vec::new(),
                Some(RelationExpansion {
                    kind: "lujvo".to_owned(),
                    source_words: rafsis,
                    rafsi_bindings,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(Some(id))
    }

    #[requires(!rafsi.is_empty())]
    #[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
    fn source_word_for_lujvo_rafsi(&self, rafsi: &str) -> Option<String> {
        if let Some(source_word) = (self.rafsi_source_word)(rafsi) {
            return Some(source_word);
        }
        let stripped = rafsi
            .strip_suffix('r')
            .or_else(|| rafsi.strip_suffix('n'))?;
        if stripped.is_empty() {
            return None;
        }
        (self.rafsi_source_word)(stripped)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    fn bound_koha_referent(
        &mut self,
        cmavo: Cmavo,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(sumti_id) = self
            .analysis
            .discourse_references
            .koha_binding(cmavo)
            .or_else(|| self.bound_koha_sumti_from_association_edges(cmavo))
        else {
            return Ok(None);
        };
        let sumti = self
            .analysis
            .syntax_index
            .sumti(sumti_id)
            .ok_or_else(SemanticsError::missing_syntax_node)?;
        self.build_sumti_referent(sumti).map(Some)
    }

    #[requires(true)]
    #[ensures(true)]
    fn bound_koha_sumti_from_association_edges(&self, cmavo: Cmavo) -> Option<SumtiNodeId> {
        for edge in self.analysis.discourse_references.edges() {
            if edge.kind != ReferenceKind::SumtiAssociation {
                continue;
            }
            let Some(source_sumti) = self.analysis.syntax_index.argument_node(edge.source) else {
                continue;
            };
            if sumti_koha_cmavo(source_sumti) != Some(cmavo) {
                continue;
            }
            let ReferenceTarget::ResolvedNode(target) = edge.target else {
                continue;
            };
            self.analysis.syntax_index.argument_node(target)?;
            return Some(SumtiNodeId(target));
        }
        None
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_numbered_assignment_arguments(
        &mut self,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        frame: Option<SelbriPlaceFrameId>,
    ) -> Result<usize, SemanticsError> {
        let excluded_terms = HashSet::new();
        self.insert_numbered_assignment_arguments_excluding_source_and_terms(
            arguments,
            frame,
            None,
            &excluded_terms,
        )
        .map(|(highest_assigned_place, _)| highest_assigned_place)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_numbered_assignment_arguments_excluding_terms(
        &mut self,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        frame: Option<SelbriPlaceFrameId>,
        excluded_terms: &HashSet<RawSyntaxNodeId>,
    ) -> Result<usize, SemanticsError> {
        self.insert_numbered_assignment_arguments_excluding_source_and_terms(
            arguments,
            frame,
            None,
            excluded_terms,
        )
        .map(|(highest_assigned_place, _)| highest_assigned_place)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_numbered_assignment_arguments_excluding_source(
        &mut self,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        frame: Option<SelbriPlaceFrameId>,
        excluded_source: Option<RawSyntaxNodeId>,
    ) -> Result<(usize, bool), SemanticsError> {
        let excluded_terms = HashSet::new();
        self.insert_numbered_assignment_arguments_excluding_source_and_terms(
            arguments,
            frame,
            excluded_source,
            &excluded_terms,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_numbered_assignment_arguments_excluding_source_and_terms(
        &mut self,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        frame: Option<SelbriPlaceFrameId>,
        excluded_source: Option<RawSyntaxNodeId>,
        excluded_terms: &HashSet<RawSyntaxNodeId>,
    ) -> Result<(usize, bool), SemanticsError> {
        let Some(frame) = frame else {
            return Ok((0, false));
        };
        let mut highest_assigned_place = 0usize;
        let mut skipped_excluded_source = false;
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let PlaceSlot::Numbered(place) = assignment.slot else {
                continue;
            };
            if excluded_source
                .is_some_and(|source| self.syntax_node_contains(assignment.sumti.0, source))
                || self.assignment_term_is_consumed(assignment, excluded_terms)
            {
                skipped_excluded_source = true;
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let argument = self.build_argument_for_sumti(sumti)?;
            let place = place.get() as usize;
            highest_assigned_place = highest_assigned_place.max(place);
            arguments.insert(format!("x{place}"), argument);
        }
        Ok((highest_assigned_place, skipped_excluded_source))
    }

    #[requires(place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn numbered_assignment_argument_for_frame(
        &mut self,
        frame: SelbriPlaceFrameId,
        place: u8,
    ) -> Result<Option<ArgumentValue>, SemanticsError> {
        let mut argument = None;
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let PlaceSlot::Numbered(assigned_place) = assignment.slot else {
                continue;
            };
            if assigned_place.get() != place {
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            argument = Some(self.build_argument_for_sumti(sumti)?);
        }
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(true)]
    fn syntax_node_contains(&self, outer: RawSyntaxNodeId, inner: RawSyntaxNodeId) -> bool {
        let Some(outer) = self.analysis.syntax_index.metadata(outer) else {
            return false;
        };
        let Some(inner) = self.analysis.syntax_index.metadata(inner) else {
            return false;
        };
        outer.leaf_start <= inner.leaf_start && inner.leaf_end <= outer.leaf_end
    }

    #[requires(true)]
    #[ensures(true)]
    fn assignment_term_is_consumed(
        &self,
        assignment: &SumtiPlaceAssignment,
        consumed_terms: &HashSet<RawSyntaxNodeId>,
    ) -> bool {
        assignment.term.is_some_and(|term| {
            consumed_terms.iter().any(|consumed| {
                self.syntax_node_contains(*consumed, term.0)
                    || self.syntax_node_contains(term.0, *consumed)
            })
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn apply_ordered_event_modifiers_to_event(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        selbri: Option<&'tree SelbriSyntax>,
        event: &mut SemanticObject,
    ) -> Result<EventModifierApplication, SemanticsError> {
        let mut application = EventModifierApplication::default();
        let mut modifiers = Vec::new();
        self.collect_frame_tense_event_modifiers(frame, &mut modifiers)?;
        if let Some(selbri) = selbri {
            self.collect_selbri_tense_event_modifiers(selbri, &mut modifiers)?;
        }
        modifiers.sort_by_key(|modifier| modifier.order);
        application.temporal_modifier = modifiers.iter().any(|modifier| {
            tense_modal_anchors_to_speech_time(modifier.tense_modal)
                || !temporal_path_relations_for_tense_modal(modifier.tense_modal).is_empty()
        });
        let story_anchor = self
            .options
            .story_time
            .then_some(self.story_time_anchor)
            .flatten();
        if !self.sticky_time_path.is_empty() && !(self.options.story_time && story_anchor.is_some())
        {
            event.time_path = self.sticky_time_path.clone();
        }
        if !self.sticky_space_path.is_empty() {
            event.space_path = self.sticky_space_path.clone();
        }
        for modifier in modifiers {
            if tense_modal_resets_sticky_tense(modifier.tense_modal) {
                self.sticky_time_path.clear();
                self.sticky_space_path.clear();
                self.story_time_anchor = None;
                clear_event_time_path(event);
                clear_event_space_path(event);
                continue;
            }
            if tense_modal_anchors_to_speech_time(modifier.tense_modal) {
                clear_event_time_path(event);
                event.time = Some(new!(AnchorRelation {
                    relation: "at".to_owned(),
                    anchor: SemanticObjectId::speech_time(),
                    distance: None,
                    magnitude: None,
                    scalar_negation: None,
                    motion: None,
                }));
                if let Some(actuality) = actuality_for_tense_modal(modifier.tense_modal) {
                    event.actuality = Some(actuality);
                }
                continue;
            }
            let anchor = modifier.anchor.or_else(|| {
                if self.options.story_time
                    && application.temporal_modifier
                    && !temporal_path_relations_for_tense_modal(modifier.tense_modal).is_empty()
                    && story_anchor.is_some()
                {
                    story_anchor
                } else {
                    event
                        .time_path
                        .is_empty()
                        .then(|| self.current_temporal_context())
                        .flatten()
                }
            });
            apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
                modifier.tense_modal,
                event,
                anchor,
                false,
            );
            if let Some(magnitude) = modifier.magnitude.clone() {
                attach_magnitude_to_event_modifier(event, modifier.tense_modal, magnitude);
            }
            if let Some(parameter) =
                self.build_tense_question_parameter_for_tense_modal(modifier.tense_modal)?
            {
                event.tense_modal = Some(parameter);
            }
            application
                .consumed_terms
                .extend(modifier.consumed_terms.iter().copied());
            if tense_modal_makes_tense_sticky(modifier.tense_modal) {
                self.sticky_time_path = event.time_path.clone();
                application.sticky_temporal_modifier = true;
            }
            if tense_modal_makes_space_sticky(modifier.tense_modal) {
                self.sticky_space_path = event.space_path.clone();
            }
        }
        normalize_event_time_path(event);
        normalize_event_space_path(event);
        Ok(application)
    }

    #[requires(eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality)]
    #[ensures(true)]
    fn apply_story_time_to_event(
        &mut self,
        eventuality: SemanticObjectId,
        event: &mut SemanticObject,
        modifier_application: EventModifierApplication,
    ) {
        if !self.options.story_time {
            return;
        }
        if let Some(anchor) = self.story_time_anchor
            && !modifier_application.temporal_modifier
        {
            event.time_path.clear();
            event.time = Some(new!(AnchorRelation {
                relation: "after".to_owned(),
                anchor,
                distance: None,
                magnitude: None,
                scalar_negation: None,
                motion: None,
            }));
        }
        if !modifier_application.temporal_modifier
            || modifier_application.sticky_temporal_modifier
            || self.story_time_anchor.is_none()
        {
            self.story_time_anchor = Some(eventuality);
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_frame_tense_event_modifiers(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        modifiers: &mut Vec<EventTenseModifier<'tree>>,
    ) -> Result<(), SemanticsError> {
        let Some(frame) = frame else {
            return Ok(());
        };
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let PlaceSlot::Modal(Some(tag_node)) = assignment.slot else {
                continue;
            };
            let Some(tense_modal) = self.analysis.syntax_index.tense_modal(tag_node) else {
                continue;
            };
            if !tense_modal_has_event_modifier(tense_modal)
                && !tense_modal_makes_tense_sticky(tense_modal)
                && !tense_modal_makes_space_sticky(tense_modal)
                && !tense_modal_resets_sticky_tense(tense_modal)
            {
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let governed = if sumti_is_omitted_placeholder(sumti) {
                self.governed_termset_for_event_modifier(frame, assignment, tag_node)?
            } else {
                None
            };
            let anchor = if let Some(governed) = &governed {
                governed.anchor
            } else if sumti_is_omitted_placeholder(sumti) {
                None
            } else {
                self.build_argument_for_sumti(sumti)?.value
            };
            modifiers.push(EventTenseModifier {
                order: self.source_order_for_node(tag_node),
                tense_modal,
                anchor,
                magnitude: governed
                    .as_ref()
                    .and_then(|termset| termset.magnitude.clone()),
                consumed_terms: governed
                    .map(|termset| termset.consumed_terms)
                    .unwrap_or_default(),
            });
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn governed_termset_for_event_modifier(
        &mut self,
        frame: SelbriPlaceFrameId,
        modifier_assignment: &SumtiPlaceAssignment,
        tag_node: RawSyntaxNodeId,
    ) -> Result<Option<GovernedTermset>, SemanticsError> {
        let Some(termset) =
            self.nearest_following_termset_for_assignment(frame, modifier_assignment)
        else {
            return Ok(None);
        };
        let mut governed = GovernedTermset::default();
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let Some(term) = assignment.term else {
                continue;
            };
            if self.termset_ancestor_for_term(term.0) != Some(termset) {
                continue;
            }
            governed.consumed_terms.push(term.0);
            match assignment.slot {
                PlaceSlot::Numbered(_) if governed.anchor.is_none() => {
                    let sumti = self
                        .analysis
                        .syntax_index
                        .sumti(assignment.sumti)
                        .ok_or_else(SemanticsError::missing_syntax_node)?;
                    governed.anchor = self.build_argument_for_sumti(sumti)?.value;
                }
                PlaceSlot::Modal(Some(modal_tag))
                    if governed.magnitude.is_none()
                        && self
                            .analysis
                            .syntax_index
                            .tense_modal(modal_tag)
                            .is_some_and(tense_modal_is_lahu_modal) =>
                {
                    let sumti = self
                        .analysis
                        .syntax_index
                        .sumti(assignment.sumti)
                        .ok_or_else(SemanticsError::missing_syntax_node)?;
                    if let Some(value) = self.build_argument_for_sumti(sumti)?.value {
                        governed.magnitude = Some(AnchorMagnitude::new(
                            value,
                            "la'u".to_owned(),
                            self.source_for_node(modal_tag, "exact-magnitude"),
                        ));
                    }
                }
                _ => {}
            }
        }
        if governed.anchor.is_none() && governed.magnitude.is_none() {
            Ok(None)
        } else {
            governed.consumed_terms.sort();
            governed.consumed_terms.dedup();
            let _ = tag_node;
            Ok(Some(governed))
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn nearest_following_termset_for_assignment(
        &self,
        frame: SelbriPlaceFrameId,
        modifier_assignment: &SumtiPlaceAssignment,
    ) -> Option<RawSyntaxNodeId> {
        let modifier_order = modifier_assignment
            .term
            .map(|term| self.source_order_for_node(term.0))
            .unwrap_or_else(|| self.source_order_for_node(modifier_assignment.sumti.0));
        self.analysis
            .place_analysis
            .assignments_for_frame(frame)
            .iter()
            .filter_map(|assignment_id| self.analysis.place_analysis.assignment(*assignment_id))
            .filter_map(|assignment| {
                let term = assignment.term?;
                let termset = self.termset_ancestor_for_term(term.0)?;
                let termset_order = self.source_order_for_node(termset);
                (termset_order > modifier_order).then_some((termset_order, termset))
            })
            .min_by_key(|(order, _)| *order)
            .map(|(_, termset)| termset)
    }

    #[requires(true)]
    #[ensures(true)]
    fn termset_ancestor_for_term(&self, term: RawSyntaxNodeId) -> Option<RawSyntaxNodeId> {
        let mut parent = self
            .analysis
            .syntax_index
            .metadata(term)
            .and_then(|metadata| metadata.parent);
        while let Some(node) = parent {
            if self
                .analysis
                .syntax_index
                .term(TermNodeId(node))
                .is_some_and(|term| matches!(term.as_data(), data!(TermSyntax::Termset { .. })))
            {
                return Some(node);
            }
            parent = self
                .analysis
                .syntax_index
                .metadata(node)
                .and_then(|metadata| metadata.parent);
        }
        None
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_selbri_tense_event_modifiers(
        &mut self,
        selbri: &'tree SelbriSyntax,
        modifiers: &mut Vec<EventTenseModifier<'tree>>,
    ) -> Result<(), SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }) => {
                if tense_modal_has_event_modifier(tense_modal)
                    || tense_modal_makes_tense_sticky(tense_modal)
                    || tense_modal_makes_space_sticky(tense_modal)
                    || tense_modal_resets_sticky_tense(tense_modal)
                {
                    modifiers.push(EventTenseModifier {
                        order: self.source_order_for_tense_modal(tense_modal),
                        tense_modal,
                        anchor: None,
                        magnitude: None,
                        consumed_terms: Vec::new(),
                    });
                }
                self.collect_selbri_tense_event_modifiers(inner_selbri, modifiers)?;
            }
            data!(SelbriSyntax::GroupedSelbri {
                ke_tense_modal,
                selbri,
                ..
            }) => {
                if let Some(tense_modal) = ke_tense_modal
                    && (tense_modal_has_event_modifier(tense_modal)
                        || tense_modal_makes_tense_sticky(tense_modal)
                        || tense_modal_makes_space_sticky(tense_modal)
                        || tense_modal_resets_sticky_tense(tense_modal))
                {
                    modifiers.push(EventTenseModifier {
                        order: self.source_order_for_tense_modal(tense_modal),
                        tense_modal,
                        anchor: None,
                        magnitude: None,
                        consumed_terms: Vec::new(),
                    });
                }
                self.collect_selbri_tense_event_modifiers(selbri, modifiers)?;
            }
            data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
            | data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
                self.collect_selbri_tense_event_modifiers(inner_selbri, modifiers)?;
            }
            data!(SelbriSyntax::Tanru(units)) => {
                for unit in units.iter() {
                    self.collect_tanru_unit_tense_event_modifiers(unit, modifiers)?;
                }
            }
            data!(SelbriSyntax::InvertedTanru {
                leading_selbri,
                trailing_selbri,
                ..
            })
            | data!(SelbriSyntax::SelbriConnection {
                leading_selbri,
                trailing_selbri,
                ..
            })
            | data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                trailing_selbri,
                ..
            }) => {
                self.collect_selbri_tense_event_modifiers(leading_selbri, modifiers)?;
                self.collect_selbri_tense_event_modifiers(trailing_selbri, modifiers)?;
            }
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                leading_bridi,
                trailing_bridi,
                ..
            }) => {
                if let Some(selbri) = main_selbri_for_bridi(leading_bridi) {
                    self.collect_selbri_tense_event_modifiers(selbri, modifiers)?;
                }
                if let Some(selbri) = main_selbri_for_bridi(trailing_bridi) {
                    self.collect_selbri_tense_event_modifiers(selbri, modifiers)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn collect_tanru_unit_tense_event_modifiers(
        &mut self,
        unit: &'tree TanruUnitSyntax,
        modifiers: &mut Vec<EventTenseModifier<'tree>>,
    ) -> Result<(), SemanticsError> {
        match unit.as_data() {
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
                self.collect_selbri_tense_event_modifiers(selbri, modifiers)?;
            }
            data!(TanruUnitSyntax::ModalConversion {
                tense_modal,
                inner_unit,
                ..
            }) => {
                if let Some(tense_modal) = tense_modal
                    && (tense_modal_has_event_modifier(tense_modal)
                        || tense_modal_makes_tense_sticky(tense_modal)
                        || tense_modal_makes_space_sticky(tense_modal)
                        || tense_modal_resets_sticky_tense(tense_modal))
                {
                    let anchor = self
                        .branch_frame_for_tanru_unit(unit)
                        .map(|frame| self.numbered_assignment_argument_for_frame(frame, 1))
                        .transpose()?
                        .flatten()
                        .and_then(|argument| argument.value);
                    modifiers.push(EventTenseModifier {
                        order: self.source_order_for_tense_modal(tense_modal),
                        tense_modal,
                        anchor,
                        magnitude: None,
                        consumed_terms: Vec::new(),
                    });
                }
                self.collect_tanru_unit_tense_event_modifiers(inner_unit, modifiers)?;
            }
            data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::RelativeClauses {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::AssignedProBridi {
                base: inner_unit,
                ..
            }) => self.collect_tanru_unit_tense_event_modifiers(inner_unit, modifiers)?,
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            })
            | data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                self.collect_tanru_unit_tense_event_modifiers(leading_unit, modifiers)?;
                self.collect_tanru_unit_tense_event_modifiers(trailing_unit, modifiers)?;
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_order_for_node(&self, node: RawSyntaxNodeId) -> usize {
        self.source_for_node(node, "tense-modal")
            .map(|source| source.span.byte_start)
            .unwrap_or(usize::MAX - 1)
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_order_for_tense_modal(&self, tense_modal: &TenseModalSyntax) -> usize {
        self.source_for_tense_modal(tense_modal, "tense-modal")
            .map(|source| source.span.byte_start)
            .unwrap_or(usize::MAX - 1)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_assignment_arguments(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let excluded_terms = HashSet::new();
        self.modal_assignment_arguments_excluding_terms(frame, &excluded_terms)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_assignment_arguments_excluding_terms(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        excluded_terms: &HashSet<RawSyntaxNodeId>,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let Some(frame) = frame else {
            return Ok(Vec::new());
        };
        let mut modal_arguments = Vec::new();
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            if self.assignment_term_is_consumed(assignment, excluded_terms) {
                continue;
            }
            let PlaceSlot::Modal(tag_node) = assignment.slot else {
                continue;
            };
            if tag_node
                .and_then(|node| self.analysis.syntax_index.tense_modal(node))
                .is_some_and(tense_modal_has_event_modifier)
            {
                continue;
            }
            let key = ModalAssignmentKey {
                sumti: assignment.sumti.0,
                tag: tag_node,
            };
            if let Some(modal_argument) = self.modal_assignment_arguments.get(&key) {
                modal_arguments.push(modal_argument.clone());
                continue;
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let argument = self.build_argument_for_sumti(sumti)?;
            let (introduced_by, relation, arguments, negation, scalar_negation) =
                self.modal_relation_arguments_for_tag(tag_node, argument)?;
            let modal_argument = if let Some(tense_modal) =
                tag_node.and_then(|node| self.analysis.syntax_index.tense_modal(node))
            {
                self.modal_argument_with_tense_modal_modifiers(
                    tense_modal,
                    relation,
                    introduced_by,
                    arguments,
                    negation,
                    scalar_negation,
                    "modal-argument",
                )
            } else {
                let source = tag_node.and_then(|node| self.source_for_node(node, "modal-argument"));
                ModalArgument::new_with_polarity(
                    relation,
                    introduced_by,
                    arguments,
                    negation,
                    scalar_negation,
                    source,
                )
            };
            if let Some(tense_modal) =
                tag_node.and_then(|node| self.analysis.syntax_index.tense_modal(node))
            {
                self.record_sticky_modal_argument_if_needed(tense_modal, &modal_argument);
            }
            self.modal_assignment_arguments
                .insert(key, modal_argument.clone());
            modal_arguments.push(modal_argument);
        }
        Ok(modal_arguments)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|connection| connection.as_ref().is_none_or(|connection| connection.terms.len() >= 2)) || ret.is_err())]
    fn logical_modal_connection_assignment(
        &mut self,
        frame: SelbriPlaceFrameId,
    ) -> Result<Option<LogicalModalConnectionAssignment>, SemanticsError> {
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        let mut connection = None;
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            let PlaceSlot::Modal(Some(tag_node)) = assignment.slot else {
                continue;
            };
            let Some(tense_modal) = self.analysis.syntax_index.tense_modal(tag_node) else {
                continue;
            };
            let Some(spec) = logical_modal_connection_spec_for_tense_modal(tense_modal) else {
                continue;
            };
            if connection.is_some() {
                return Ok(None);
            }
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let data!(LogicalModalConnectionSpec {
                operator,
                source,
                truth_table,
                terms,
            }) = spec.into_data();
            connection = Some(LogicalModalConnectionAssignment::from_data(data!(
                LogicalModalConnectionAssignment {
                    argument: self.build_argument_for_sumti(sumti)?,
                    operator,
                    source,
                    truth_table,
                    terms,
                }
            )));
        }
        Ok(connection)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn selbri_modal_arguments(
        &mut self,
        selbri: &'tree SelbriSyntax,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::TaggedSelbri {
                tense_modal,
                inner_selbri,
            }) => {
                let mut modal_arguments = Vec::new();
                if let Some(modal_argument) =
                    self.modal_argument_for_tense_modal(tense_modal, "modal-argument")?
                {
                    modal_arguments.push(modal_argument);
                }
                modal_arguments.extend(self.selbri_modal_arguments(inner_selbri)?);
                Ok(modal_arguments)
            }
            data!(SelbriSyntax::GroupedSelbri {
                ke_tense_modal,
                selbri,
                ..
            }) => {
                let mut modal_arguments = Vec::new();
                if let Some(tense_modal) = ke_tense_modal
                    && let Some(modal_argument) =
                        self.modal_argument_for_tense_modal(tense_modal, "modal-argument")?
                {
                    modal_arguments.push(modal_argument);
                }
                modal_arguments.extend(self.selbri_modal_arguments(selbri)?);
                Ok(modal_arguments)
            }
            data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
            | data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
                self.selbri_modal_arguments(inner_selbri)
            }
            data!(SelbriSyntax::Tanru(units)) => {
                let mut modal_arguments = Vec::new();
                for unit in units.iter() {
                    modal_arguments.extend(self.tanru_unit_modal_arguments(unit)?);
                }
                Ok(modal_arguments)
            }
            _ => Ok(Vec::new()),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn tanru_unit_modal_arguments(
        &mut self,
        unit: &'tree TanruUnitSyntax,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        match unit.as_data() {
            data!(TanruUnitSyntax::ModalConversion {
                tense_modal,
                inner_unit,
                ..
            }) => {
                let mut modal_arguments = Vec::new();
                if let Some(tense_modal) = tense_modal.as_deref()
                    && !tense_modal_has_event_modifier(tense_modal)
                    && let Some((introduced_by, relation, visible_place)) =
                        modal_relation_spec_for_tense_modal(tense_modal)
                {
                    let argument = match self
                        .branch_frame_for_tanru_unit(unit)
                        .map(|frame| self.numbered_assignment_argument_for_frame(frame, 1))
                        .transpose()?
                        .flatten()
                    {
                        Some(argument) => argument,
                        None => self.build_elided_argument_for_place(visible_place)?,
                    };
                    let arguments = self.modal_argument_map_for_visible_place(
                        argument,
                        visible_place,
                        self.place_count_for_relation(&relation),
                    )?;
                    modal_arguments.push(self.modal_argument_with_tense_modal_modifiers(
                        tense_modal,
                        relation,
                        introduced_by,
                        arguments,
                        modal_negation_for_tense_modal(tense_modal),
                        modal_scalar_negation_for_tense_modal(tense_modal),
                        "modal-argument",
                    ));
                }
                modal_arguments.extend(self.tanru_unit_modal_arguments(inner_unit)?);
                Ok(modal_arguments)
            }
            data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::RelativeClauses {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
                base: inner_unit,
                ..
            })
            | data!(TanruUnitSyntax::AssignedProBridi {
                base: inner_unit,
                ..
            }) => self.tanru_unit_modal_arguments(inner_unit),
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
                self.selbri_modal_arguments(selbri)
            }
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            })
            | data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                let mut modal_arguments = self.tanru_unit_modal_arguments(leading_unit)?;
                modal_arguments.extend(self.tanru_unit_modal_arguments(trailing_unit)?);
                Ok(modal_arguments)
            }
            _ => Ok(Vec::new()),
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_argument_for_tense_modal(
        &mut self,
        tense_modal: &'tree TenseModalSyntax,
        construct: &str,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        if tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        let Some((introduced_by, relation, visible_place)) =
            modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Ok(None);
        };
        let argument = self.build_elided_argument_for_place(visible_place)?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            self.place_count_for_relation(&relation),
        )?;
        let modal_argument = self.modal_argument_with_tense_modal_modifiers(
            tense_modal,
            relation,
            introduced_by,
            arguments,
            modal_negation_for_tense_modal(tense_modal),
            modal_scalar_negation_for_tense_modal(tense_modal),
            construct,
        );
        self.record_sticky_modal_argument_if_needed(tense_modal, &modal_argument);
        Ok(Some(modal_argument))
    }

    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(!arguments.is_empty())]
    #[requires(arguments.keys().all(|place| crate::model::is_numbered_argument_place(place)))]
    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn modal_argument_with_tense_modal_modifiers(
        &self,
        tense_modal: &'tree TenseModalSyntax,
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<String, ArgumentValue>,
        negation: Option<ModalNegation>,
        scalar_negation: Option<ScalarNegation>,
        construct: &str,
    ) -> ModalArgument {
        let mut modal_argument = ModalArgument::new_with_polarity(
            relation,
            introduced_by,
            arguments,
            negation,
            scalar_negation,
            self.source_for_tense_modal(tense_modal, construct),
        );
        let modifiers = self.modal_argument_modifiers_for_tense_modal(tense_modal);
        if !modifiers.is_empty() {
            modal_argument = modal_argument.with_data(data! { modifiers: modifiers });
        }
        modal_argument
    }

    #[requires(true)]
    #[ensures(true)]
    fn modal_argument_modifiers_for_tense_modal(
        &self,
        tense_modal: &'tree TenseModalSyntax,
    ) -> Vec<DisplayedContentModifier> {
        let mut parts = Vec::new();
        tense_modal.visit_words(&mut |token| {
            parts.extend(indicator_parts_for_token(token));
        });
        indicator_display_drafts(parts)
            .into_iter()
            .map(|draft| {
                let source = if draft.source_tokens.is_empty() {
                    None
                } else {
                    self.source_for_tokens(&draft.source_tokens, "modal-indicator")
                }
                .or_else(|| self.source_for_tense_modal(tense_modal, "modal-indicator"));
                new!(DisplayedContentModifier {
                    relation: if draft.question {
                        attitude_question_relation(&draft.relation)
                    } else {
                        draft.relation
                    },
                    family: Some(draft.family),
                    polarity: Some(draft.polarity),
                    intensity: draft.intensity,
                    assertion_effect: Some(draft.assertion_effect),
                    source,
                })
            })
            .collect()
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_argument_for_jai_conversion(
        &mut self,
        tense_modal: &'tree TenseModalSyntax,
        referent: SemanticObjectId,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        let Some((introduced_by, relation, visible_place)) =
            modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Ok(None);
        };
        let arguments = self.modal_argument_map_for_visible_place(
            ArgumentValue::filled(referent, None),
            visible_place,
            self.place_count_for_relation(&relation),
        )?;
        Ok(Some(self.modal_argument_with_tense_modal_modifiers(
            tense_modal,
            relation,
            introduced_by,
            arguments,
            modal_negation_for_tense_modal(tense_modal),
            modal_scalar_negation_for_tense_modal(tense_modal),
            "modal-argument",
        )))
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_sticky_modal_argument_if_needed(
        &mut self,
        tense_modal: &TenseModalSyntax,
        modal_argument: &ModalArgument,
    ) {
        if !tense_modal_makes_modal_sticky(tense_modal) {
            return;
        }
        self.sticky_modal_arguments.insert(
            StickyModalKey::for_modal_argument(modal_argument),
            modal_argument.clone(),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn clear_sticky_modals_for_selbri_if_needed(&mut self, selbri: Option<&'tree SelbriSyntax>) {
        if selbri.is_some_and(selbri_resets_sticky_modals) {
            self.sticky_modal_arguments.clear();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn append_sticky_modal_arguments(&self, modal_arguments: &mut Vec<ModalArgument>) {
        for sticky_modal in self.sticky_modal_arguments.values() {
            if modal_arguments
                .iter()
                .any(|modal_argument| modal_argument == sticky_modal)
            {
                continue;
            }
            modal_arguments.push(sticky_modal.clone());
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_relation_arguments_for_tag(
        &mut self,
        tag_node: Option<RawSyntaxNodeId>,
        argument: ArgumentValue,
    ) -> Result<
        (
            String,
            String,
            BTreeMap<String, ArgumentValue>,
            Option<ModalNegation>,
            Option<ScalarNegation>,
        ),
        SemanticsError,
    > {
        let Some(tense_modal) =
            tag_node.and_then(|node| self.analysis.syntax_index.tense_modal(node))
        else {
            return Ok((
                "modal".to_owned(),
                "modal".to_owned(),
                self.modal_argument_map_for_visible_place(argument, 1, None)?,
                None,
                None,
            ));
        };
        match tense_modal.as_data() {
            data!(TenseModalSyntax::AdHocModal { selbri, .. }) => {
                let relation = relation_label_for_selbri(selbri);
                let visible_x1_place = visible_x1_place_for_selbri(selbri);
                let arguments = self.modal_argument_map_for_visible_place(
                    argument,
                    visible_x1_place,
                    self.place_count_for_relation(&relation),
                )?;
                Ok(("fi'o".to_owned(), relation, arguments, None, None))
            }
            data!(TenseModalSyntax::Modal { se, bai, .. }) => {
                let marker = token_text(&bai.value);
                let relation = modal_relation_for_marker(&marker);
                let visible_x1_place = se
                    .as_ref()
                    .and_then(se_conversion_place)
                    .map(usize::from)
                    .unwrap_or(1);
                let arguments = self.modal_argument_map_for_visible_place(
                    argument,
                    visible_x1_place,
                    self.place_count_for_relation(&relation),
                )?;
                let introduced_by = se
                    .as_ref()
                    .map(|se| format!("{} {marker}", token_text(&se.value)))
                    .unwrap_or(marker);
                Ok((
                    introduced_by,
                    relation,
                    arguments,
                    modal_negation_for_tense_modal(tense_modal),
                    modal_scalar_negation_for_tense_modal(tense_modal),
                ))
            }
            _ => {
                let marker = self
                    .source_for_node(
                        tag_node.expect("tense modal came from a recorded syntax node"),
                        "modal-argument",
                    )
                    .and_then(|source| source.text)
                    .unwrap_or_else(|| "modal".to_owned());
                let relation = modal_relation_for_marker(&marker);
                Ok((
                    marker,
                    relation,
                    self.modal_argument_map_for_visible_place(argument, 1, None)?,
                    modal_negation_for_tense_modal(tense_modal),
                    modal_scalar_negation_for_tense_modal(tense_modal),
                ))
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_modal_argument_to_discourse_item(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object =
            self.objects.get(&id).cloned().ok_or_else(|| {
                SemanticsError::invalid_graph(format!("missing discourse item {id}"))
            })?;
        match object.object_kind() {
            crate::model::SemanticObjectKind::Utterance => {
                if let Some(content) = object.content {
                    self.attach_modal_argument_to_content(content, modal_argument)?;
                }
            }
            crate::model::SemanticObjectKind::Sequence => {
                for item in object.items {
                    self.attach_modal_argument_to_discourse_item(item, modal_argument)?;
                }
            }
            crate::model::SemanticObjectKind::Formula => {
                self.attach_modal_argument_to_formula(id, modal_argument)?;
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_modal_argument_to_content(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        match id.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.attach_modal_argument_to_formula(id, modal_argument)
            }
            crate::model::SemanticObjectKind::Sequence => {
                self.attach_modal_argument_to_discourse_item(id, modal_argument)
            }
            crate::model::SemanticObjectKind::Question => Ok(()),
            _ => Ok(()),
        }
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_modal_argument_to_formula(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&id)
            .cloned()
            .ok_or_else(|| SemanticsError::invalid_graph(format!("missing formula {id}")))?;
        if let Some(predication) = object.predication {
            self.attach_modal_argument_to_predication(predication, modal_argument)?;
        }
        for child in object.children {
            self.attach_modal_argument_to_formula(child, modal_argument)?;
        }
        if let Some(body) = object.body {
            self.attach_modal_argument_to_formula(body, modal_argument)?;
        }
        Ok(())
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_modal_argument_to_predication(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| SemanticsError::invalid_graph(format!("missing predication {id}")))?;
        if object.mode == Some(PredicationMode::Asserted)
            && !object.modal_arguments.contains(modal_argument)
        {
            object.modal_arguments.push(modal_argument.clone());
        }
        Ok(())
    }

    #[requires(visible_x1_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|arguments| !arguments.is_empty()) || ret.is_err())]
    fn modal_argument_map_for_visible_place(
        &mut self,
        argument: ArgumentValue,
        visible_x1_place: usize,
        place_count: Option<usize>,
    ) -> Result<BTreeMap<String, ArgumentValue>, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(format!("x{visible_x1_place}"), argument);
        let highest_place = place_count
            .unwrap_or(visible_x1_place)
            .max(visible_x1_place);
        for place in 1..=highest_place {
            let key = format!("x{place}");
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        Ok(arguments)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn place_question_bindings(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
        arguments: &BTreeMap<String, ArgumentValue>,
        place_count: Option<usize>,
        highest_assigned_place: usize,
    ) -> Result<Vec<PlaceQuestionBinding>, SemanticsError> {
        let Some(frame) = frame else {
            return Ok(Vec::new());
        };
        let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
        let mut occupied = HashSet::new();
        for place in arguments
            .keys()
            .filter_map(|place| argument_place_index(place))
        {
            occupied.insert(place);
        }
        let mut question_assignments = Vec::new();
        for assignment_id in assignment_ids {
            let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id) else {
                continue;
            };
            match assignment.slot {
                PlaceSlot::Numbered(place) => {
                    occupied.insert(place.get() as usize);
                }
                PlaceSlot::PlaceQuestion => question_assignments.push(assignment.clone()),
                PlaceSlot::Modal(_) | PlaceSlot::Fai => {}
            }
        }
        if question_assignments.is_empty() {
            return Ok(Vec::new());
        }
        let candidate_limit = place_count.unwrap_or_else(|| highest_assigned_place.max(1));
        let candidate_places = (1..=candidate_limit)
            .filter(|place| !occupied.contains(place))
            .map(|place| format!("x{place}"))
            .collect::<Vec<_>>();
        if candidate_places.is_empty() {
            return Ok(Vec::new());
        }
        let mut bindings = Vec::new();
        for assignment in question_assignments {
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let introduced_by = self.place_question_introducer(&assignment);
            let source_node = assignment
                .term
                .map(|term| term.0)
                .or(Some(assignment.sumti.0));
            let parameter = self.build_place_question_parameter(introduced_by, source_node)?;
            let argument = self.build_argument_for_sumti(sumti)?;
            let source = source_node.and_then(|node| self.source_for_node(node, "place-question"));
            bindings.push(PlaceQuestionBinding::new(
                parameter,
                argument,
                candidate_places.clone(),
                source,
            ));
        }
        Ok(bindings)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn place_question_introducer(&self, assignment: &SumtiPlaceAssignment) -> String {
        assignment
            .term
            .and_then(|term| self.analysis.syntax_index.term(term))
            .and_then(|term| match term.as_data() {
                data!(TermSyntax::PlaceTaggedSumti { fa, .. })
                    if fa.cmavo() == Some(Cmavo::Fiha) =>
                {
                    Some(token_text(&fa.value))
                }
                _ => None,
            })
            .unwrap_or_else(|| "fi'a".to_owned())
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_from_arguments(
        &mut self,
        relation: String,
        selbri: Option<&'tree SelbriSyntax>,
        source: Option<crate::model::SemanticSource>,
        arguments: BTreeMap<String, ArgumentValue>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source.clone());
        if let Some(selbri) = selbri {
            apply_selbri_event_modifiers_to_event(selbri, &mut event);
        }
        self.insert(eventuality, event)?;
        let id = self.next_predication();
        let mode = asserted_predication_mode_for_relation(&relation);
        self.insert(
            id,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                mode,
                source,
                diagnostics,
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.is_none_or(|eventuality| eventuality.object_kind() == crate::model::SemanticObjectKind::Eventuality)) || ret.is_err())]
    fn build_tagged_eventuality_for_selbri(
        &mut self,
        selbri: &SelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if !selbri_has_event_modifiers(selbri) {
            return Ok(None);
        }
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source);
        apply_selbri_event_modifiers_to_event_with_anchor(
            selbri,
            &mut event,
            self.current_temporal_context(),
        );
        self.insert(eventuality, event).map(Some)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        let raw = self
            .analysis
            .syntax_index
            .sumti_node_id(sumti)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        if sumti_deletes_place(sumti) {
            return Ok(ArgumentValue::deleted(
                "zi'o".to_owned(),
                self.source_for_node(raw, "deleted-place"),
            ));
        }
        let explicit_quantity = self.build_argument_quantity_for_sumti(raw, sumti)?;
        let referent = self.build_sumti_referent(sumti)?;
        let mut argument = if sumti_is_elided(sumti) {
            ArgumentValue::elided(
                referent,
                "zo'e".to_owned(),
                self.source_for_node(raw, "elided-place"),
            )
        } else {
            ArgumentValue::filled(referent, None)
        };
        if let Some(quantity) = explicit_quantity {
            if !self.referent_descriptor_quantity_is(referent, quantity)
                && argument.kind != crate::model::ArgumentValueKind::Deleted
            {
                argument = argument.with_quantity(quantity);
            }
        }
        self.attach_relative_clauses_to_argument(argument, sumti, referent)
    }

    #[requires(argument.kind != crate::model::ArgumentValueKind::Deleted)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_relative_clauses_to_argument(
        &mut self,
        argument: ArgumentValue,
        sumti: &'tree SumtiSyntax,
        head: SemanticObjectId,
    ) -> Result<ArgumentValue, SemanticsError> {
        let lowered = self.lower_relative_clauses_for_sumti(sumti, head)?;
        if lowered.is_empty() {
            Ok(argument)
        } else {
            Ok(argument.with_relative_clauses(lowered))
        }
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_relative_clauses_to_referent(
        &mut self,
        head: SemanticObjectId,
        sumti: &'tree SumtiSyntax,
    ) -> Result<(), SemanticsError> {
        let lowered = self.lower_relative_clauses_for_sumti(sumti, head)?;
        if lowered.is_empty() {
            return Ok(());
        }
        let object = self.objects.get_mut(&head).ok_or_else(|| {
            SemanticsError::invalid_graph(format!(
                "semantic builder could not find relative-clause head {head}"
            ))
        })?;
        object.extend_relative_clauses(lowered);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_relative_clauses_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError> {
        let mut clauses = Vec::new();
        occurrence_relative_clauses_for_sumti(sumti, &mut clauses);
        self.lower_relative_clauses(clauses, head)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_relative_clauses<I>(
        &mut self,
        clauses: I,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError>
    where
        I: IntoIterator<Item = &'tree RelativeClauseSyntax>,
    {
        let mut lowered = Vec::new();
        for clause in clauses {
            if let Some(clause) = self.build_relative_clause(clause, head)? {
                lowered.push(clause);
            }
        }
        Ok(lowered)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relative_clause(
        &mut self,
        clause: &'tree RelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        match clause.as_data() {
            data!(RelativeClauseSyntax::IncidentalRelativeBridi { noi, subbridi, .. })
                if noi
                    .cmavo()
                    .is_some_and(cmavo_is_nonveridical_relative_marker) =>
            {
                self.build_nonveridical_relative_bridi_clause(noi, subbridi, head)
                    .map(Some)
            }
            data!(RelativeClauseSyntax::IncidentalRelativeBridi { subbridi, .. }) => self
                .build_relative_bridi_clause(subbridi, head, RelativeClauseKind::Incidental)
                .map(Some),
            data!(RelativeClauseSyntax::RestrictiveRelativeBridi { subbridi, .. }) => self
                .build_relative_bridi_clause(subbridi, head, RelativeClauseKind::Restrictive)
                .map(Some),
            data!(RelativeClauseSyntax::JoinedRelativeClauses { inner, .. })
            | data!(RelativeClauseSyntax::RelativeClauseConnection { inner, .. }) => {
                self.build_relative_clause(inner, head)
            }
            data!(RelativeClauseSyntax::SumtiAssociationPhrase(phrase)) => {
                self.build_sumti_association_phrase_clause(phrase, head)
            }
        }
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_nonveridical_relative_bridi_clause(
        &mut self,
        marker: &'tree WithFreeModifiers<Token>,
        subbridi: &'tree SubbridiSyntax,
        head: SemanticObjectId,
    ) -> Result<RelativeClause, SemanticsError> {
        let marker_text = token_text(&marker.value);
        let source = self.source_for_subbridi(subbridi, "relative-clause");
        let formula = if let Some(selbri) = main_selbri_for_subbridi(subbridi) {
            self.build_nonveridical_relative_formula_for_selbri(selbri, head, source.clone())?
        } else {
            let formula = self.build_diagnostic_relative_formula(subbridi)?;
            self.set_formula_predication_mode(formula, PredicationMode::Restrictive);
            formula
        };
        Ok(RelativeClause::nonveridical(
            RelativeClauseKind::Restrictive,
            formula,
            marker_text,
            source,
        ))
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_nonveridical_relative_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        head: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let property = self.build_property_abstraction_for_selbri(selbri, source.clone())?;
        let mut arguments = BTreeMap::new();
        arguments.insert(
            "x1".to_owned(),
            ArgumentValue::filled(SemanticObjectId::speaker(), None),
        );
        arguments.insert("x2".to_owned(), ArgumentValue::filled(head, None));
        arguments.insert("x3".to_owned(), ArgumentValue::filled(property, None));
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                "describedAs".to_owned(),
                None,
                arguments,
                PredicationMode::Restrictive,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_sumti_association_phrase_clause(
        &mut self,
        phrase: &'tree SumtiAssociationPhraseSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        let marker_text = token_text(&phrase.association_marker.value);
        if phrase.association_marker.cmavo() == Some(Cmavo::Goi) {
            return Ok(None);
        }
        let source = self.source_for_sumti_association_phrase(phrase, "relative-phrase");
        let marker = phrase.association_marker.cmavo();
        let kind = marker
            .and_then(relative_phrase_kind_for_marker)
            .unwrap_or(RelativeClauseKind::Restrictive);
        let mode = predication_mode_for_relative_clause_kind(kind);
        let modal_tagged_sumti = tense_modal_tagged_sumti(&phrase.sumti);
        if let Some((tense_modal, associated_sumti)) = modal_tagged_sumti
            && let Some(clause) = self.build_modal_sumti_association_phrase_clause(
                tense_modal,
                associated_sumti,
                head,
                kind,
                marker_text.clone(),
                source.clone(),
            )?
        {
            return Ok(Some(clause));
        }
        let relation = marker
            .and_then(relative_phrase_relation_for_marker)
            .unwrap_or("relativePhrase")
            .to_owned();
        let mut diagnostics = Vec::new();
        if marker
            .and_then(relative_phrase_relation_for_marker)
            .is_none()
        {
            diagnostics.push(diagnostic(
                "GOI relative phrase marker is not semantically lowered yet",
            ));
        }
        if modal_tagged_sumti.is_some() {
            diagnostics.push(diagnostic(
                "modal relative phrase source relation is not semantically lowered yet",
            ));
        }
        let associated_sumti = modal_tagged_sumti
            .map(|(_, associated_sumti)| associated_sumti)
            .unwrap_or(&phrase.sumti);
        let associated_argument = self.build_argument_for_sumti(associated_sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(head, None));
        arguments.insert("x2".to_owned(), associated_argument);
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                mode,
                source.clone(),
                diagnostics,
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source.clone(), Vec::new()),
        )?;
        Ok(Some(RelativeClause::with_introducer(
            kind,
            formula,
            marker_text,
            source,
        )))
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(!marker_text.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_modal_sumti_association_phrase_clause(
        &mut self,
        tense_modal: &'tree TenseModalSyntax,
        associated_sumti: &'tree SumtiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
        marker_text: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        let Some((introduced_by, relation, visible_place)) =
            modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Ok(None);
        };
        let Some(head_place) = modal_relative_phrase_head_place(&relation, visible_place) else {
            return Ok(None);
        };
        let mode = predication_mode_for_relative_clause_kind(kind);
        let associated_argument = self.build_argument_for_sumti(associated_sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(format!("x{head_place}"), ArgumentValue::filled(head, None));
        arguments.insert(format!("x{visible_place}"), associated_argument);
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count.max(head_place).max(visible_place) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=head_place.max(visible_place) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            None,
            arguments,
            mode,
            source.clone(),
            diagnostics,
        );
        object.introduced_by = Some(introduced_by);
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source.clone(), Vec::new()),
        )?;
        Ok(Some(RelativeClause::with_introducer(
            kind,
            formula,
            marker_text,
            source,
        )))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relative_bridi_clause(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
    ) -> Result<RelativeClause, SemanticsError> {
        let mode = predication_mode_for_relative_clause_kind(kind);
        if subbridi_contains_keha(subbridi)
            && let Some(formula) = self.build_subbridi_formula(subbridi)?
        {
            self.set_formula_predication_mode(formula, mode);
            return Ok(RelativeClause::new(
                kind,
                formula,
                self.source_for_subbridi(subbridi, "relative-clause"),
            ));
        }
        let Some(selbri) = main_selbri_for_subbridi(subbridi) else {
            let formula = self.build_diagnostic_relative_formula(subbridi)?;
            return Ok(RelativeClause::new(
                kind,
                formula,
                self.source_for_subbridi(subbridi, "relative-clause"),
            ));
        };
        let formula = self.build_implicit_relative_head_formula_for_selbri(
            selbri,
            head,
            mode,
            self.source_for_subbridi(subbridi, "relative-clause"),
        )?;
        Ok(RelativeClause::new(
            kind,
            formula,
            self.source_for_subbridi(subbridi, "relative-clause"),
        ))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_subbridi_formula(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match subbridi.as_data() {
            data!(SubbridiSyntax::Bridi(bridi)) => {
                let formula = self.build_bridi_formula(bridi)?;
                let formula = self.wrap_bridi_formula_with_quantified_pro_sumti(bridi, formula)?;
                self.wrap_bridi_formula_with_contradictory_event_tense_negation(bridi, formula)
                    .map(Some)
            }
            data!(SubbridiSyntax::Prenex {
                prenex_terms,
                inner_subbridi,
                ..
            }) => {
                let Some(formula) = self.build_subbridi_formula(inner_subbridi)? else {
                    return Ok(None);
                };
                self.wrap_formula_with_prenex_terms(formula, prenex_terms)
                    .map(Some)
            }
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn set_formula_predication_mode(&mut self, formula: SemanticObjectId, mode: PredicationMode) {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return;
        };
        if let Some(predication) = object.predication
            && let Some(object) = self.objects.get_mut(&predication)
        {
            object.mode = Some(mode);
        }
        for child in object.children {
            self.set_formula_predication_mode(child, mode);
        }
        if let Some(restriction) = object.restriction {
            self.set_formula_predication_mode(restriction, mode);
        }
        if let Some(body) = object.body {
            self.set_formula_predication_mode(body, mode);
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_referent_predication_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = relation_label_for_selbri(selbri);
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        let visible_x1_place = visible_x1_place_for_selbri(selbri);
        let intrinsic_modal_arguments = self.selbri_modal_arguments(selbri)?;
        self.build_referent_predication_formula_for_relation(
            relation,
            frame,
            visible_x1_place,
            ArgumentValue::filled(referent, None),
            intrinsic_modal_arguments,
            mode,
            source,
        )
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_implicit_relative_head_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        head: SemanticObjectId,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(units) = tanru_units_for_selbri(selbri)
            && tanru_units_require_lowering(&units)
        {
            let formula = self.build_restrictive_formula(selbri, head)?;
            self.set_formula_predication_mode(formula, mode);
            return Ok(formula);
        }
        let relation = relation_label_for_selbri(selbri);
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        let mut arguments = BTreeMap::new();
        let highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        let head_place =
            first_unfilled_visible_place_for_selbri(selbri, &arguments, highest_assigned_place);
        arguments.insert(format!("x{head_place}"), ArgumentValue::filled(head, None));
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(head_place) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let relation_metadata =
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?;
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            None,
            arguments,
            mode,
            source.clone(),
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.relation_metadata = relation_metadata;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(visible_x1_place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_referent_predication_formula_for_relation(
        &mut self,
        relation: String,
        frame: Option<SelbriPlaceFrameId>,
        visible_x1_place: usize,
        visible_argument: ArgumentValue,
        intrinsic_modal_arguments: Vec<ModalArgument>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let mut modal_arguments = intrinsic_modal_arguments;
        modal_arguments.extend(self.modal_assignment_arguments(frame)?);
        arguments.insert(format!("x{visible_x1_place}"), visible_argument);
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            None,
            arguments,
            mode,
            source.clone(),
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_diagnostic_relative_formula(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                "relative-clause".to_owned(),
                None,
                BTreeMap::new(),
                PredicationMode::Incidental,
                self.source_for_subbridi(subbridi, "relative-clause"),
                vec![diagnostic("relative clause bridi is not fully lowered yet")],
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.source_for_subbridi(subbridi, "relative-clause"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_argument_for_place(
        &mut self,
        place: usize,
    ) -> Result<ArgumentValue, SemanticsError> {
        self.build_elided_argument_for_place_with_sort(place, SemanticSort::Entity)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_argument_for_place_with_sort(
        &mut self,
        place: usize,
        sort: SemanticSort,
    ) -> Result<ArgumentValue, SemanticsError> {
        self.build_elided_argument_for_place_with_label_and_sort(place, sort)
    }

    #[requires(surface_place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_argument_for_place_with_label_and_sort(
        &mut self,
        surface_place: usize,
        sort: SemanticSort,
    ) -> Result<ArgumentValue, SemanticsError> {
        let referent =
            self.build_elided_referent_with_sort(None, format!("zo'e x{surface_place}"), sort)?;
        Ok(ArgumentValue::elided(referent, "zo'e".to_owned(), None))
    }

    #[requires(place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn predication_argument(
        &self,
        predication: SemanticObjectId,
        place: usize,
    ) -> Result<ArgumentValue, SemanticsError> {
        self.objects
            .get(&predication)
            .and_then(|object| object.arguments.get(&format!("x{place}")))
            .cloned()
            .ok_or_else(|| {
                SemanticsError::invalid_graph(format!(
                    "predication has no visible x1 argument at x{place}"
                ))
            })
    }

    #[requires((1..=u8::MAX as usize).contains(&place))]
    #[ensures(true)]
    fn frame_has_numbered_assignment(
        &self,
        frame: Option<SelbriPlaceFrameId>,
        place: usize,
    ) -> bool {
        let Some(frame) = frame else {
            return false;
        };
        let Some(slot) = PlaceSlot::numbered(place as u8) else {
            return false;
        };
        !self
            .analysis
            .place_analysis
            .assignments_for_frame_slot(frame, slot)
            .is_empty()
    }

    #[requires(true)]
    #[ensures(true)]
    fn place_count_for_relation(&self, relation: &str) -> Option<usize> {
        constructed_relation_place_count(relation).or_else(|| (self.relation_place_count)(relation))
    }

    #[requires(true)]
    #[ensures(true)]
    fn semantic_predication_frame_for_selbri(
        &self,
        selbri: &'tree SelbriSyntax,
        frame: Option<SelbriPlaceFrameId>,
    ) -> Option<SelbriPlaceFrameId> {
        match selbri.as_data() {
            data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. }) => self
                .semantic_predication_frame_for_selbri(
                    inner_selbri,
                    self.branch_frame_for_selbri(inner_selbri).or(frame),
                ),
            data!(SelbriSyntax::GroupedSelbri { selbri, .. })
            | data!(SelbriSyntax::TaggedSelbri {
                inner_selbri: selbri,
                ..
            }) => self.semantic_predication_frame_for_selbri(
                selbri,
                self.branch_frame_for_selbri(selbri).or(frame),
            ),
            data!(SelbriSyntax::Tanru(units)) => {
                let unit = units.last();
                self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit).or(frame),
                )
            }
            _ => frame,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn semantic_predication_frame_for_tanru_unit(
        &self,
        unit: &'tree TanruUnitSyntax,
        frame: Option<SelbriPlaceFrameId>,
    ) -> Option<SelbriPlaceFrameId> {
        match unit.as_data() {
            data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
            | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => self
                .semantic_predication_frame_for_tanru_unit(
                    inner_unit,
                    self.branch_frame_for_tanru_unit(inner_unit).or(frame),
                ),
            data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => self
                .semantic_predication_frame_for_selbri(
                    selbri,
                    self.branch_frame_for_selbri(selbri).or(frame),
                ),
            data!(TanruUnitSyntax::RelativeClauses { base, .. })
            | data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
            | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
            | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => self
                .semantic_predication_frame_for_tanru_unit(
                    base,
                    self.branch_frame_for_tanru_unit(base).or(frame),
                ),
            _ => frame,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bridi_frame(&self, bridi: &'tree BridiSyntax) -> Option<SelbriPlaceFrameId> {
        let bridi_id = self.analysis.syntax_index.bridi_node_id(bridi)?;
        self.analysis
            .place_analysis
            .frames_for_node(bridi_id.0)
            .iter()
            .copied()
            .find(|frame| {
                self.analysis
                    .place_analysis
                    .frame(*frame)
                    .is_some_and(|frame| frame.kind == PlaceFrameKind::Bridi)
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn branch_frame_for_selbri(&self, selbri: &'tree SelbriSyntax) -> Option<SelbriPlaceFrameId> {
        let selbri_id = self.analysis.syntax_index.selbri_node_id(selbri)?;
        self.analysis
            .place_analysis
            .frames_for_node(selbri_id.0)
            .iter()
            .copied()
            .find(|frame| {
                self.analysis
                    .place_analysis
                    .frame(*frame)
                    .is_some_and(|frame| {
                        matches!(
                            frame.kind,
                            PlaceFrameKind::BaseSelbri
                                | PlaceFrameKind::TanruUnit
                                | PlaceFrameKind::Compound
                                | PlaceFrameKind::Converted
                                | PlaceFrameKind::JaiConverted
                                | PlaceFrameKind::CoInverted
                                | PlaceFrameKind::Forwarding
                                | PlaceFrameKind::Abstraction
                                | PlaceFrameKind::LinkedUnit
                                | PlaceFrameKind::ConnectiveBranching
                                | PlaceFrameKind::ProBridi
                        )
                    })
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn branch_frame_for_tanru_unit(
        &self,
        unit: &'tree TanruUnitSyntax,
    ) -> Option<SelbriPlaceFrameId> {
        let unit_id = self.analysis.syntax_index.tanru_unit_node_id(unit)?;
        self.analysis
            .place_analysis
            .frames_for_node(unit_id.0)
            .iter()
            .copied()
            .find(|frame| {
                self.analysis
                    .place_analysis
                    .frame(*frame)
                    .is_some_and(|frame| {
                        matches!(
                            frame.kind,
                            PlaceFrameKind::BaseSelbri
                                | PlaceFrameKind::TanruUnit
                                | PlaceFrameKind::Compound
                                | PlaceFrameKind::Converted
                                | PlaceFrameKind::JaiConverted
                                | PlaceFrameKind::CoInverted
                                | PlaceFrameKind::Forwarding
                                | PlaceFrameKind::Abstraction
                                | PlaceFrameKind::LinkedUnit
                                | PlaceFrameKind::ConnectiveBranching
                                | PlaceFrameKind::ProBridi
                        )
                    })
            })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_sumti_referent(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let raw = self
            .analysis
            .syntax_index
            .sumti_node_id(sumti)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        if let Some(id) = self.sumti_objects.get(&raw) {
            return Ok(*id);
        }
        let id = match sumti.as_data() {
            data!(SumtiSyntax::QuotedSumti(quote)) => self.build_quote_sign(quote, raw)?,
            data!(SumtiSyntax::ProSumti(token)) => self.build_pro_sumti(token, raw)?,
            data!(SumtiSyntax::NumberSumti { expression, li, .. }) => {
                self.build_number_referent(expression, li, raw)?
            }
            data!(SumtiSyntax::LerfuStringSumti { .. }) => {
                if let Some(referent) = self.build_resolved_sumti_reference(raw)? {
                    referent
                } else {
                    self.build_diagnostic_referent(
                        raw,
                        "letteral pro-sumti did not resolve to an antecedent",
                    )?
                }
            }
            data!(SumtiSyntax::ElidedSumti { .. }) => {
                self.build_elided_referent(Some(raw), "zo'e".to_owned())?
            }
            data!(SumtiSyntax::Description(description)) => {
                self.build_description_referent(description, raw)?
            }
            data!(SumtiSyntax::NameDescription { la, names }) => {
                self.queue_vocative_asides(&names.free_modifiers)?;
                self.build_named_referent(
                    raw,
                    word_run_text(&names.value),
                    &token_text(&la.value),
                    gadri_name_sort(la.cmavo()),
                )?
            }
            data!(SumtiSyntax::NameWords(names)) => {
                self.queue_vocative_asides(&names.free_modifiers)?;
                self.build_named_referent(
                    raw,
                    word_run_text(&names.value),
                    "la",
                    SemanticSort::Entity,
                )?
            }
            data!(SumtiSyntax::SelbriVocative {
                leading_relative_clauses,
                selbri,
                trailing_relative_clauses,
            }) => self.build_selbri_vocative_referent(
                raw,
                leading_relative_clauses,
                selbri,
                trailing_relative_clauses,
            )?,
            data!(SumtiSyntax::QuantifiedSumti {
                quantifier,
                inner_sumti,
            }) => {
                let quantity = self.build_quantity_for_sumti_quantifier(raw, quantifier)?;
                let referent = self.build_sumti_referent(inner_sumti)?;
                self.add_quantity_to_referent(referent, quantity);
                referent
            }
            data!(SumtiSyntax::SumtiWithRelativeClauses {
                base_sumti,
                relative_clauses,
                ..
            })
            | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                base_sumti,
                relative_clauses,
                ..
            }) => {
                if let Some(referent) =
                    self.build_goi_associated_referent(base_sumti, relative_clauses)?
                {
                    referent
                } else {
                    self.build_sumti_referent(base_sumti)?
                }
            }
            data!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                connective,
                trailing_sumti,
            }) => {
                self.build_connected_sumti_referent(raw, leading_sumti, connective, trailing_sumti)?
            }
            data!(SumtiSyntax::BoundSumtiConnection {
                leading_sumti,
                bo_connective: Some(connective),
                bo_tense_modal: None,
                trailing_sumti,
                ..
            }) => {
                self.build_connected_sumti_referent(raw, leading_sumti, connective, trailing_sumti)?
            }
            data!(SumtiSyntax::ForethoughtSumtiConnection {
                gek,
                leading_sumti,
                trailing_sumti,
                ..
            }) => self.build_connected_sumti_referent(raw, leading_sumti, gek, trailing_sumti)?,
            data!(SumtiSyntax::ReferentSumti {
                lahe,
                inner_sumti,
                ..
            }) => self.build_qualified_referent(raw, lahe, inner_sumti)?,
            data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
                self.build_sumti_referent(inner_sumti)?
            }
            data!(SumtiSyntax::ScalarNegatedSumti {
                nahe,
                inner_sumti,
                ..
            }) => self.build_scalar_negated_sumti_referent(
                raw,
                nahe.cmavo(),
                token_text(&nahe.value),
                inner_sumti,
            )?,
            data!(SumtiSyntax::ScalarNegatedSumtiWithBo {
                nahe,
                inner_sumti,
                ..
            }) => self.build_scalar_negated_sumti_referent(
                raw,
                nahe.cmavo(),
                format!("{} bo", token_text(nahe)),
                inner_sumti,
            )?,
            _ => self.build_diagnostic_referent(raw, "sumti construct is not fully lowered yet")?,
        };
        if id.object_kind() == crate::model::SemanticObjectKind::Referent
            && sumti_has_current_kau_focus(sumti)
        {
            self.record_indirect_question_focus(new!(IndirectQuestionFocus {
                focus: id,
                presupposed_answer: Some(id),
                slots: Vec::new(),
                kind: QuestionKind::Argument,
                domain: SemanticSort::Entity,
                source: self.source_for_node(raw, "indirect-question"),
            }));
        }
        if let Some(anchor) = self.current_utterance_anchor
            && !sumti_connection_has_branch_indicator_attachment(sumti)
        {
            self.attach_indicator_displays(
                indicator_parts_for_sumti(sumti),
                id,
                anchor,
                "indicator",
            )?;
        }
        self.sumti_objects.insert(raw, id);
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_goi_associated_referent(
        &mut self,
        base_sumti: &'tree SumtiSyntax,
        relative_clauses: &'tree [RelativeClauseSyntax],
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(phrase) = relative_clauses.iter().find_map(goi_assignment_phrase) else {
            return Ok(None);
        };
        let associated_sumti = &phrase.sumti;
        if sumti_is_assignable_reference(base_sumti) {
            return self.build_sumti_referent(associated_sumti).map(Some);
        }
        if sumti_is_assignable_reference(associated_sumti) {
            return self.build_sumti_referent(base_sumti).map(Some);
        }
        if let Some(assigned_name) = self.assigned_name_for_sumti(associated_sumti, phrase) {
            let referent = self.build_sumti_referent(base_sumti)?;
            self.add_assigned_name_to_referent(referent, assigned_name);
            return Ok(Some(referent));
        }
        Ok(None)
    }

    #[requires(true)]
    #[ensures(true)]
    fn assigned_name_for_sumti(
        &self,
        sumti: &SumtiSyntax,
        phrase: &SumtiAssociationPhraseSyntax,
    ) -> Option<AssignedName> {
        let (word, name) = match sumti.as_data() {
            data!(SumtiSyntax::NameDescription { la, names }) => {
                (token_text(&la.value), word_run_text(&names.value))
            }
            data!(SumtiSyntax::NameWords(names)) => ("la".to_owned(), word_run_text(&names.value)),
            data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
                return self.assigned_name_for_sumti(inner_sumti, phrase);
            }
            _ => return None,
        };
        Some(AssignedName::from_data(data!(AssignedName {
            name,
            word,
            introduced_by: token_text(&phrase.association_marker.value),
            source: self.source_for_sumti_association_phrase(phrase, "assigned-name"),
        })))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_quote_sign(
        &mut self,
        quote: &'tree QuoteSyntax,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(raw, "quotation");
        let source_text = source.as_ref().and_then(|source| source.text.clone());
        let quotation = match quote.as_data() {
            data!(QuoteSyntax::TextQuote { lu, text, .. }) => {
                let mut marker_asides = self.build_vocative_asides(&lu.free_modifiers)?;
                let utterance = if text_has_semantic_content(text) {
                    let utterance = self.build_text_group_sequence(text)?;
                    self.add_asides_to_discourse_item(
                        utterance,
                        std::mem::take(&mut marker_asides),
                    );
                    Some(utterance)
                } else if marker_asides.is_empty() {
                    None
                } else {
                    self.build_standalone_asides(marker_asides)?
                };
                Quotation {
                    mode: "parsed".to_owned(),
                    utterance,
                    delimiter: None,
                    text: source_text,
                }
            }
            data!(QuoteSyntax::WordQuote(marker))
            | data!(QuoteSyntax::DelimitedWordQuote(marker))
            | data!(QuoteSyntax::DelimitedNonLojbanQuote(marker))
            | data!(QuoteSyntax::WordsQuote(marker)) => Quotation {
                mode: "opaque".to_owned(),
                utterance: None,
                delimiter: Some(token_text(&marker.value)),
                text: source_text,
            },
        };
        let id = self.next_sign();
        self.insert(
            id,
            SemanticObject::sign(SignKind::Quotation, Some(quotation), source, Vec::new()),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_number_referent(
        &mut self,
        expression: &'tree MeksoSyntax,
        li: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if li.cmavo() == Some(Cmavo::Meho) {
            if let Some(letters) = mekso_letteral_word_run(expression) {
                return self
                    .build_letteral_sign(letters, self.source_for_mekso(expression, "letteral"));
            }
            return self.build_math_expression_sign(expression, raw);
        }

        let variable_name = math_variable_name(expression);
        if let Some(variable_name) = &variable_name
            && let Some(referent) = self.math_variable_referents.get(variable_name)
        {
            return Ok(*referent);
        }
        let text = mekso_surface_text(expression);
        let quantity = self.build_quantity_for_mekso(expression, raw)?;
        let id = self.next_referent();
        let referent = self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Number,
                None,
                Some(Descriptor {
                    kind: "number".to_owned(),
                    word: token_text(&li.value),
                    speaker: None,
                    body: None,
                    relative_clauses: Vec::new(),
                    quantity: Some(quantity),
                    name: Some(text),
                    operand: None,
                }),
                None,
                self.source_for_node(raw, "number-sumti"),
                Vec::new(),
            ),
        )?;
        if let Some(variable_name) = variable_name {
            self.math_variable_referents.insert(variable_name, referent);
        }
        Ok(referent)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_expression_sign(
        &mut self,
        expression: &'tree MeksoSyntax,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let expression_id =
            self.build_math_expression(expression, self.source_for_node(raw, "math-expression"))?;
        let mut sign = SemanticObject::text_sign(
            SignKind::MathExpression,
            mekso_surface_text(expression),
            self.source_for_node(raw, "number-sumti"),
            Vec::new(),
        );
        sign.denotes = Some(expression_id);
        let id = self.next_sign();
        self.insert(id, sign)
    }

    #[requires(lerfu_string_sumti_letters(sumti).is_some())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sign) || ret.is_err())]
    fn build_letteral_sign_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let raw = self
            .analysis
            .syntax_index
            .sumti_node_id(sumti)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        let letters = lerfu_string_sumti_letters(sumti)
            .expect("precondition guarantees a lerfu-string sumti");
        self.build_letteral_sign(letters, self.source_for_node(raw, "letteral"))
    }

    #[requires(!letters.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sign) || ret.is_err())]
    fn build_letteral_sign(
        &mut self,
        letters: &WordRun,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let letterals = letteral_units_for_word_run(letters);
        let text = letteral_display_text(&letterals)
            .or_else(|| source.as_ref().and_then(|source| source.text.clone()))
            .unwrap_or_else(|| word_run_text(letters));
        let mut sign = SemanticObject::text_sign(SignKind::Letteral, text, source, Vec::new());
        sign.letterals = letterals;
        let id = self.next_sign();
        self.insert(id, sign)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_quantity_for_mekso(
        &mut self,
        expression: &'tree MeksoSyntax,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = mekso_surface_text(expression);
        let value = parse_decimal_integer(&text)
            .map(QuantityValue::integer)
            .map_or_else(
                || {
                    self.build_math_expression(
                        expression,
                        self.source_for_node(raw, "math-expression"),
                    )
                    .map(QuantityValue::math_expression)
                },
                Ok,
            )?;
        let id = self.next_quantity();
        self.insert(
            id,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                self.source_for_node(raw, "quantity"),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_expression(
        &mut self,
        expression: &'tree MeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match expression.as_data() {
            data!(MeksoSyntax::NumberMekso(quantifier)) => {
                self.build_math_expression_for_quantifier(quantifier, source)
            }
            data!(MeksoSyntax::LerfuStringMekso { letter, .. }) => self.build_math_literal(
                MathLiteral::text("variable".to_owned(), math_letteral_text(&letter.value)),
                source,
            ),
            data!(MeksoSyntax::ParenthesizedMekso {
                inner_expression,
                ..
            })
            | data!(MeksoSyntax::QualifiedOperand {
                inner_expression,
                ..
            }) => self.build_math_expression(inner_expression, source),
            data!(MeksoSyntax::Infix {
                left_expression,
                operator,
                right_expression,
            })
            | data!(MeksoSyntax::PrecedenceInfix {
                left_expression,
                operator,
                right_expression,
                ..
            }) => {
                let operands = vec![
                    self.build_math_expression(left_expression, None)?,
                    self.build_math_expression(right_expression, None)?,
                ];
                self.build_math_operator_expression(math_operator_label(operator), operands, source)
            }
            data!(MeksoSyntax::ForethoughtCall {
                operator,
                operands,
                ..
            }) => {
                let operands = operands
                    .iter()
                    .map(|operand| self.build_math_expression(operand, None))
                    .collect::<Result<Vec<_>, _>>()?;
                self.build_math_operator_expression(math_operator_label(operator), operands, source)
            }
            data!(MeksoSyntax::MeksoArray { expressions, .. }) => {
                let operands = expressions
                    .iter()
                    .map(|operand| self.build_math_expression(operand, None))
                    .collect::<Result<Vec<_>, _>>()?;
                self.build_math_operator_expression("array".to_owned(), operands, source)
            }
            data!(MeksoSyntax::SumtiOperand { sumti, .. }) => {
                let referent = self.build_sumti_referent(sumti)?;
                let operand_source =
                    source.or_else(|| self.source_for_mekso(expression, "sumti-operand"));
                self.build_math_sumti_operand(referent, operand_source)
            }
            _ => self.build_math_literal(
                MathLiteral::text("expression".to_owned(), mekso_surface_text(expression)),
                source,
            ),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_expression_for_quantifier(
        &mut self,
        quantifier: &'tree QuantifierSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match quantifier.as_data() {
            data!(QuantifierSyntax::NumberQuantifier { number, .. }) => {
                let text = word_run_text(&number.value);
                let literal = parse_decimal_integer(&text)
                    .map(MathLiteral::integer)
                    .unwrap_or_else(|| MathLiteral::text("number".to_owned(), text));
                self.build_math_literal(literal, source)
            }
            data!(QuantifierSyntax::MeksoQuantifier { mekso, .. }) => {
                self.build_math_expression(mekso, source)
            }
        }
    }

    #[requires(!operator.is_empty())]
    #[requires(!operands.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_operator_expression(
        &mut self,
        operator: String,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math();
        self.insert(
            id,
            SemanticObject::math_expression(Some(operator), operands, None, source, Vec::new()),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_literal(
        &mut self,
        literal: MathLiteral,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math();
        self.insert(
            id,
            SemanticObject::math_expression(None, Vec::new(), Some(literal), source, Vec::new()),
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    fn build_math_sumti_operand(
        &mut self,
        referent: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math();
        self.insert(
            id,
            SemanticObject::math_sumti_operand(referent, source, Vec::new()),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_pro_sumti(
        &mut self,
        token: &'tree WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.queue_vocative_asides(&token.free_modifiers)?;
        match token.cmavo() {
            Some(Cmavo::Mi) => Ok(SemanticObjectId::speaker()),
            Some(Cmavo::Do) => Ok(SemanticObjectId::addressee()),
            Some(Cmavo::Ko) => Ok(SemanticObjectId::addressee()),
            Some(Cmavo::Ma) => self.build_argument_parameter(token, raw),
            Some(Cmavo::Cehu) => {
                self.build_parameter(token, raw, crate::model::ParameterRole::PropertySlot)
            }
            Some(Cmavo::Keha) => self.build_relative_head_referent(token, raw),
            Some(
                Cmavo::Dei
                | Cmavo::Dihu
                | Cmavo::Dehu
                | Cmavo::Dahu
                | Cmavo::Dihe
                | Cmavo::Dehe
                | Cmavo::Dahe
                | Cmavo::Dohi,
            ) => self.build_utterance_reference_referent(token, raw),
            Some(Cmavo::Zohe) => self.build_elided_referent(Some(raw), "zo'e".to_owned()),
            Some(Cmavo::Zuhi) => self.build_typical_place_value_referent(token, raw),
            Some(Cmavo::Ti) => {
                self.build_demonstrative_referent(raw, IndexicalKind::ProximalDemonstrative)
            }
            Some(Cmavo::Ta) => {
                self.build_demonstrative_referent(raw, IndexicalKind::MedialDemonstrative)
            }
            Some(Cmavo::Tu) => {
                self.build_demonstrative_referent(raw, IndexicalKind::DistalDemonstrative)
            }
            _ => {
                if let Some(referent) = self.build_resolved_sumti_reference(raw)? {
                    return Ok(referent);
                }
                self.build_plain_referent(
                    raw,
                    ReferentCategory::Constant,
                    SemanticSort::Entity,
                    Descriptor {
                        kind: "proSumti".to_owned(),
                        word: token_text(&token.value),
                        speaker: None,
                        body: None,
                        relative_clauses: Vec::new(),
                        quantity: None,
                        name: None,
                        operand: None,
                    },
                    Vec::new(),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_typical_place_value_referent(
        &mut self,
        token: &'tree WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            SemanticSort::Entity,
            Descriptor {
                kind: "typicalPlaceValue".to_owned(),
                word: token_text(&token.value),
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                operand: None,
            },
            Vec::new(),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relative_head_referent(
        &mut self,
        token: &'tree WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(referent) = self.build_resolved_sumti_reference(raw)? {
            return Ok(referent);
        }
        self.build_parameter(token, raw, crate::model::ParameterRole::RelativeClauseHead)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_utterance_reference_referent(
        &mut self,
        token: &'tree WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let target = self.resolved_utterance_reference_target(raw);
        let mut diagnostics = Vec::new();
        if target.is_none() && token.cmavo() != Some(Cmavo::Dohi) {
            diagnostics.push(diagnostic(
                "utterance pro-sumti did not resolve to a concrete discourse item",
            ));
        }
        let id = self.next_referent();
        let mut object = SemanticObject::referent(
            ReferentCategory::Constant,
            SemanticSort::Sign,
            None,
            Some(Descriptor {
                kind: "utteranceReference".to_owned(),
                word: token_text(&token.value),
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                operand: None,
            }),
            None,
            self.source_for_node(raw, "sumti"),
            diagnostics,
        );
        object.target = target;
        self.insert(id, object)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_resolved_sumti_reference(
        &mut self,
        raw: RawSyntaxNodeId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(target) = self.resolved_sumti_reference_target(raw) else {
            return Ok(None);
        };
        let Some(sumti) = self.analysis.syntax_index.argument_node(target) else {
            return Err(SemanticsError::missing_syntax_node());
        };
        self.build_sumti_referent(sumti).map(Some)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_sumti_reference_target(&self, raw: RawSyntaxNodeId) -> Option<RawSyntaxNodeId> {
        self.analysis
            .discourse_references
            .references_from_node(raw)
            .iter()
            .filter_map(|edge_id| self.analysis.discourse_references.edge(*edge_id))
            .filter(|edge| sumti_reference_kind_is_direct_reference(&edge.kind))
            .find_map(|edge| match edge.target {
                ReferenceTarget::ResolvedNode(target) if target != raw => Some(target),
                _ => None,
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolved_utterance_reference_target(
        &self,
        raw: RawSyntaxNodeId,
    ) -> Option<SemanticObjectId> {
        self.analysis
            .discourse_references
            .references_from_node(raw)
            .iter()
            .filter_map(|edge_id| self.analysis.discourse_references.edge(*edge_id))
            .filter(|edge| edge.kind == ReferenceKind::Utterance)
            .find_map(|edge| match edge.target {
                ReferenceTarget::ResolvedNode(target) if target != raw => {
                    self.utterance_objects.get(&target).copied()
                }
                _ => None,
            })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_demonstrative_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        indexical: IndexicalKind,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(indexical),
                None,
                None,
                self.source_for_node(raw, "sumti"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_parameter(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_parameter(token, raw, crate::model::ParameterRole::ArgumentQuestion)?;
        if with_free_modifiers_has_indicator_cmavo(token, Cmavo::Kau)
            && self.record_indirect_question_focus(new!(IndirectQuestionFocus {
                focus: id,
                presupposed_answer: None,
                slots: vec![QuestionSlot {
                    parameter: id,
                    role: QuestionSlotRole::Answer,
                }],
                kind: QuestionKind::Argument,
                domain: SemanticSort::Entity,
                source: self.source_for_node(raw, "indirect-question"),
            }))
        {
            return Ok(id);
        }
        self.push_question_answer_slot(id);
        Ok(id)
    }

    #[requires(focus.focus.object_kind() == crate::model::SemanticObjectKind::Parameter || focus.focus.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret == old(self.indirect_question_stack.last().is_some()) || (!ret && self.indirect_question_stack.last().is_none()))]
    fn record_indirect_question_focus(&mut self, focus: IndirectQuestionFocus) -> bool {
        let Some(foci) = self.indirect_question_stack.last_mut() else {
            return false;
        };
        foci.push(focus);
        true
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_parameter(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
        role: crate::model::ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_parameter_with_sort(token, raw, SemanticSort::Entity, role)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_parameter_with_sort(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
        sort: SemanticSort,
        role: crate::model::ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_parameter_with_source(
            token_text(&token.value),
            self.source_for_node(raw, "parameter"),
            sort,
            role,
        )?;
        if role == crate::model::ParameterRole::PropertySlot
            && token.cmavo() == Some(Cmavo::Cehu)
            && let Some(parameters) = self.abstraction_parameter_stack.last_mut()
        {
            parameters.push(id);
        }
        Ok(id)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_parameter_with_source(
        &mut self,
        introduced_by: String,
        source: Option<crate::model::SemanticSource>,
        sort: SemanticSort,
        role: crate::model::ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_parameter();
        self.insert(
            id,
            SemanticObject::parameter(sort, role, introduced_by, source),
        )?;
        Ok(id)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(self.parameter_slots.iter().any(|slot| slot.parameter == parameter))]
    fn push_question_answer_slot(&mut self, parameter: SemanticObjectId) {
        if self
            .parameter_slots
            .iter()
            .any(|slot| slot.parameter == parameter)
        {
            return;
        }
        self.parameter_slots.push(QuestionSlot {
            parameter,
            role: QuestionSlotRole::Answer,
        });
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_question_parameter_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match selbri.as_data() {
            data!(SelbriSyntax::SelbriWord(token)) if token.cmavo() == Some(Cmavo::Mo) => {
                let raw = self
                    .analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .ok_or_else(SemanticsError::missing_syntax_node)?
                    .0;
                self.build_relation_question_parameter_from_raw(raw, token_text(token))
                    .map(Some)
            }
            data!(SelbriSyntax::Tanru(units)) if units.len() == 1 => {
                self.build_relation_question_parameter_for_tanru_unit(units.first())
            }
            data!(SelbriSyntax::GroupedSelbri { selbri, .. })
            | data!(SelbriSyntax::TaggedSelbri {
                inner_selbri: selbri,
                ..
            }) => self.build_relation_question_parameter_for_selbri(selbri),
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_question_parameter_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(introduced_by) = relation_question_word_for_tanru_unit(unit) else {
            return Ok(None);
        };
        let raw = self
            .analysis
            .syntax_index
            .tanru_unit_node_id(unit)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        self.build_relation_question_parameter_from_raw(raw, introduced_by)
            .map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    fn build_relation_variable_parameter_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(introduced_by) = relation_variable_word_for_selbri(selbri) else {
            return Ok(None);
        };
        let raw = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        self.build_relation_variable_parameter_from_raw(raw, introduced_by)
            .map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    fn build_relation_variable_parameter_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(introduced_by) = relation_variable_word_for_tanru_unit(unit) else {
            return Ok(None);
        };
        let raw = self
            .analysis
            .syntax_index
            .tanru_unit_node_id(unit)
            .ok_or_else(SemanticsError::missing_syntax_node)?
            .0;
        self.build_relation_variable_parameter_from_raw(raw, introduced_by)
            .map(Some)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_relation_variable_parameter_from_raw(
        &mut self,
        raw: RawSyntaxNodeId,
        introduced_by: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let key = self
            .resolved_target_raw_for_raw(raw, ReferenceKind::BrodaSeries)
            .unwrap_or(raw);
        if let Some(id) = self.relation_variable_parameters.get(&key).copied() {
            return Ok(id);
        }
        let id = self.next_parameter();
        self.insert(
            id,
            SemanticObject::parameter(
                SemanticSort::Relation,
                crate::model::ParameterRole::RelationVariable,
                introduced_by,
                self.source_for_node(key, "parameter")
                    .or_else(|| self.source_for_node(raw, "parameter")),
            ),
        )?;
        self.relation_variable_parameters.insert(key, id);
        Ok(id)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_relation_question_parameter_from_raw(
        &mut self,
        raw: RawSyntaxNodeId,
        introduced_by: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(id) = self.relation_question_parameters.get(&raw).copied() {
            self.push_question_answer_slot(id);
            return Ok(id);
        }
        let id = self.next_parameter();
        self.insert(
            id,
            SemanticObject::parameter(
                SemanticSort::Relation,
                crate::model::ParameterRole::RelationQuestion,
                introduced_by,
                self.source_for_node(raw, "parameter"),
            ),
        )?;
        self.relation_question_parameters.insert(raw, id);
        self.push_question_answer_slot(id);
        Ok(id)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_place_question_parameter(
        &mut self,
        introduced_by: String,
        source_node: Option<RawSyntaxNodeId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_parameter();
        self.insert(
            id,
            SemanticObject::parameter(
                SemanticSort::Place,
                crate::model::ParameterRole::PlaceQuestion,
                introduced_by,
                source_node.and_then(|node| self.source_for_node(node, "parameter")),
            ),
        )?;
        self.push_question_answer_slot(id);
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.as_ref().is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    fn build_tense_question_parameter_for_tense_modal(
        &mut self,
        tense_modal: &TenseModalSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(token) = tense_question_token_for_tense_modal(tense_modal) else {
            return Ok(None);
        };
        let id = self.build_parameter_with_source(
            token_text(token),
            self.source_for_token(token, "parameter"),
            SemanticSort::TenseModal,
            crate::model::ParameterRole::TenseQuestion,
        )?;
        self.push_question_answer_slot(id);
        Ok(Some(id))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_connective_question_parameter_for_token(
        &mut self,
        token: &Token,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_token(token, "parameter");
        let id = self.build_parameter_with_source(
            token_text(token),
            source.clone(),
            SemanticSort::Connective,
            crate::model::ParameterRole::ConnectiveQuestion,
        )?;
        if token_has_indicator_cmavo(token, Cmavo::Kau)
            && self.record_indirect_question_focus(new!(IndirectQuestionFocus {
                focus: id,
                presupposed_answer: None,
                slots: vec![QuestionSlot {
                    parameter: id,
                    role: QuestionSlotRole::Answer,
                }],
                kind: QuestionKind::Connective,
                domain: SemanticSort::Connective,
                source: self.source_for_token(token, "indirect-question"),
            }))
        {
            return Ok(id);
        }
        self.push_question_answer_slot(id);
        Ok(id)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_referent(
        &mut self,
        raw: Option<RawSyntaxNodeId>,
        label: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort(raw, label, SemanticSort::Entity)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_referent_with_sort(
        &mut self,
        raw: Option<RawSyntaxNodeId>,
        label: String,
        sort: SemanticSort,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind: "elided".to_owned(),
                    word: label,
                    speaker: None,
                    body: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    operand: None,
                }),
                None,
                raw.and_then(|raw| self.source_for_node(raw, "elided-sumti")),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_description_referent(
        &mut self,
        description: &'tree DescriptionSyntax,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        let word = description
            .description
            .as_ref()
            .map(|word| token_text(&word.value))
            .unwrap_or_else(|| "lo".to_owned());
        let kind = match description
            .description
            .as_ref()
            .and_then(|word| word.cmavo())
        {
            Some(Cmavo::Lo) => "veridicalDescription",
            Some(Cmavo::Loi) => "veridicalMassDescription",
            Some(Cmavo::Lohi) => "veridicalSetDescription",
            Some(Cmavo::Le) => "speakerDescription",
            Some(Cmavo::Lei) => "speakerMassDescription",
            Some(Cmavo::Lehi) => "speakerSetDescription",
            Some(Cmavo::Lehe) => "speakerStereotypeDescription",
            Some(Cmavo::La) => "name",
            Some(Cmavo::Lai) => "massNameDescription",
            Some(Cmavo::Lahi) => "setNameDescription",
            Some(Cmavo::Lohe) => "typicalDescription",
            _ => "description",
        }
        .to_owned();
        let abstraction = description
            .selbri
            .as_deref()
            .and_then(description_abstraction_for_selbri);
        let sort = abstraction
            .map(|abstraction| abstraction.output_sort)
            .unwrap_or_else(|| {
                match description
                    .description
                    .as_ref()
                    .and_then(|word| word.cmavo())
                {
                    Some(Cmavo::Loi | Cmavo::Lei) => SemanticSort::Mass,
                    Some(Cmavo::Lohi | Cmavo::Lehi) => SemanticSort::Set,
                    Some(Cmavo::Lai) => SemanticSort::Mass,
                    Some(Cmavo::Lahi) => SemanticSort::Set,
                    _ => SemanticSort::Entity,
                }
            });
        let body = if let Some(selbri) = description.selbri.as_deref() {
            Some(if let Some(abstraction) = abstraction {
                let link_source = self
                    .analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "abstraction-description"));
                let frame = self.semantic_predication_frame_for_selbri(
                    selbri,
                    self.branch_frame_for_selbri(selbri),
                );
                self.build_abstraction_description_formula(abstraction, id, frame, link_source)?
            } else {
                self.build_restrictive_formula(selbri, id)?
            })
        } else {
            None
        };
        let operand_sumti = description_tail_sumti(&description.tail_elements);
        let operand = operand_sumti
            .map(|sumti| self.build_sumti_referent(sumti))
            .transpose()?;
        let quantity = self.build_description_quantity(description, raw)?;
        let mut object = SemanticObject::referent(
            ReferentCategory::Constant,
            sort,
            None,
            Some(Descriptor {
                kind,
                word,
                speaker: Some(SemanticObjectId::speaker()),
                body,
                relative_clauses: Vec::new(),
                quantity,
                name: None,
                operand,
            }),
            None,
            self.source_for_description(description, raw, "description"),
            Vec::new(),
        );
        self.push_goi_assigned_names_to_referent(&mut object, &description.relative_clauses);
        self.insert(id, object)?;
        self.sumti_objects.insert(raw, id);
        let mut relative_clauses = if description.description.is_some() {
            let mut clauses = Vec::new();
            descriptor_relative_clauses_for_description_tail(
                &description.tail_elements,
                &mut clauses,
            );
            clauses.extend(description.relative_clauses.iter());
            self.lower_relative_clauses(clauses, id)?
        } else {
            Vec::new()
        };
        if description.description.is_some()
            && description.selbri.is_some()
            && let (Some(operand_sumti), Some(operand)) = (operand_sumti, operand)
        {
            relative_clauses.push(self.build_possessive_association_clause(
                id,
                operand,
                operand_sumti,
                &description.tail_elements,
            )?);
        }
        if !relative_clauses.is_empty() {
            let object = self.objects.get_mut(&id).ok_or_else(|| {
                SemanticsError::invalid_graph(format!(
                    "semantic builder could not find description referent {id}"
                ))
            })?;
            let Some(descriptor) = object.descriptor.as_mut() else {
                return Err(SemanticsError::invalid_graph(format!(
                    "semantic builder description referent {id} has no descriptor"
                )));
            };
            descriptor.relative_clauses = relative_clauses;
        }
        Ok(id)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(operand.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_possessive_association_clause(
        &mut self,
        head: SemanticObjectId,
        operand: SemanticObjectId,
        operand_sumti: &'tree SumtiSyntax,
        tail_elements: &'tree [DescriptionTailElementSyntax],
    ) -> Result<RelativeClause, SemanticsError> {
        let source = self.source_for_sumti(operand_sumti, "possessive-sumti");
        let operand_relative_clauses =
            self.lower_relative_clauses(possessive_sumti_relative_clauses(tail_elements), operand)?;
        let mut associated_argument = ArgumentValue::filled(operand, None);
        if !operand_relative_clauses.is_empty() {
            associated_argument =
                associated_argument.with_relative_clauses(operand_relative_clauses);
        }
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(head, None));
        arguments.insert("x2".to_owned(), associated_argument);
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                "associatedWith".to_owned(),
                None,
                arguments,
                PredicationMode::Restrictive,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source.clone(), Vec::new()),
        )?;
        Ok(RelativeClause::new(
            RelativeClauseKind::Restrictive,
            formula,
            source,
        ))
    }

    #[requires(object.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn push_goi_assigned_names_to_referent(
        &self,
        object: &mut SemanticObject,
        clauses: &'tree [RelativeClauseSyntax],
    ) {
        for phrase in clauses.iter().filter_map(goi_assignment_phrase) {
            if let Some(assigned_name) = self.assigned_name_for_sumti(&phrase.sumti, phrase) {
                object.push_assigned_name(assigned_name);
            }
        }
    }

    #[requires(!name.is_empty())]
    #[requires(!word.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_named_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        name: String,
        word: &str,
        sort: SemanticSort,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let kind = match sort {
            SemanticSort::Mass => "massName",
            SemanticSort::Set => "setName",
            _ => "name",
        };
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            sort,
            Descriptor {
                kind: kind.to_owned(),
                word: word.to_owned(),
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: Some(name),
                operand: None,
            },
            Vec::new(),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_selbri_vocative_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        leading_relative_clauses: &'tree [RelativeClauseSyntax],
        selbri: &'tree SelbriSyntax,
        trailing_relative_clauses: &'tree [RelativeClauseSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.sumti_objects.insert(raw, id);
        let body = self.build_restrictive_formula(selbri, id)?;
        let relative_clauses = self.lower_relative_clauses(
            leading_relative_clauses
                .iter()
                .chain(trailing_relative_clauses.iter()),
            id,
        )?;
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "speakerDescription".to_owned(),
                    word: "le".to_owned(),
                    speaker: Some(SemanticObjectId::speaker()),
                    body: Some(body),
                    relative_clauses,
                    quantity: None,
                    name: None,
                    operand: None,
                }),
                None,
                self.source_for_node(raw, "vocative-description"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_qualified_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        qualifier: &WithFreeModifiers<Token>,
        inner_sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operand = self.build_sumti_referent(inner_sumti)?;
        let word = token_text(&qualifier.value);
        let kind = referent_qualifier_kind(qualifier.cmavo()).to_owned();
        let sort = referent_qualifier_sort(qualifier.cmavo());
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            sort,
            Descriptor {
                kind,
                word,
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                operand: Some(operand),
            },
            Vec::new(),
        )
    }

    #[requires(!word.is_empty())]
    #[requires(crate::model::argument_object_kind_can_fill(operand.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_abstraction_about_referent(
        &mut self,
        word: &str,
        operand: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Proposition,
                None,
                Some(Descriptor {
                    kind: "abstractionAbout".to_owned(),
                    word: word.to_owned(),
                    speaker: Some(SemanticObjectId::speaker()),
                    body: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    operand: Some(operand),
                }),
                None,
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_scalar_negated_sumti_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        cmavo: Option<Cmavo>,
        word: String,
        inner_sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operand = self.build_sumti_referent(inner_sumti)?;
        let sort = self
            .objects
            .get(&operand)
            .and_then(|object| object.sort)
            .unwrap_or(SemanticSort::Entity);
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            sort,
            Descriptor {
                kind: scalar_negated_sumti_qualifier_kind(cmavo).to_owned(),
                word,
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                operand: Some(operand),
            },
            Vec::new(),
        )
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_diagnostic_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        message: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            SemanticSort::Entity,
            Descriptor {
                kind: "unloweredSumti".to_owned(),
                word: "sumti".to_owned(),
                speaker: None,
                body: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                operand: None,
            },
            vec![diagnostic(message)],
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_plain_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        category: ReferentCategory,
        sort: SemanticSort,
        descriptor: Descriptor,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                category,
                sort,
                None,
                Some(descriptor),
                None,
                self.source_for_node(raw, "sumti"),
                diagnostics,
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_connected_sumti_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        leading_sumti: &'tree SumtiSyntax,
        connective: &ConnectiveSyntax,
        trailing_sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading = self.build_sumti_referent(leading_sumti)?;
        let trailing = self.build_sumti_referent(trailing_sumti)?;
        let interval_connective = connective_is_interval(connective);
        let logical_connective = connective_is_logical(connective);
        let operator_parameter =
            if let Some(token) = direct_connective_question_token_for_connective(connective) {
                Some(self.build_connective_question_parameter_for_token(token)?)
            } else {
                None
            };
        let right_negated = operator_parameter.is_none()
            && connective_negates_right(connective)
            && logical_connective;
        let complement = (operator_parameter.is_none()
            && interval_connective
            && connective_negates_right(connective))
        .then_some(true);
        let scalar_negated = (operator_parameter.is_none()
            && !logical_connective
            && !interval_connective
            && connective_negates_right(connective))
        .then_some(true);
        let operator = if operator_parameter.is_some() {
            "connectiveQuestion".to_owned()
        } else if logical_connective {
            "joint".to_owned()
        } else {
            nonlogical_composition_operator(connective)
        };
        let reverse_members = connective_reverses_composition_members(connective);
        let (first, second) = if reverse_members {
            (trailing, leading)
        } else {
            (leading, trailing)
        };
        let members = if right_negated {
            vec![leading]
        } else {
            vec![first, second]
        };
        let excluded_members = if right_negated {
            vec![trailing]
        } else {
            Vec::new()
        };
        let collective = (operator == "mass").then_some(true);
        let referent = self.build_composite_referent(
            raw,
            new!(Composition {
                operator,
                operator_parameter,
                members,
                excluded_members,
                collective,
                scalar_negated,
                complement,
                endpoint_inclusion: interval_endpoint_inclusion(connective, reverse_members),
            }),
        )?;
        if let Some(anchor) = self.current_utterance_anchor {
            self.attach_indicator_displays(
                indicator_parts_for_connective_cmavo(connective),
                trailing,
                anchor,
                "indicator",
            )?;
            self.attach_indicator_displays(
                indicator_parts_for_connective_nai(connective),
                referent,
                anchor,
                "indicator",
            )?;
        }
        Ok(referent)
    }

    #[requires(!composition.operator.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_composite_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        composition: Composition,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Composite,
                SemanticSort::Entity,
                None,
                None,
                Some(composition),
                self.source_for_node(raw, "connected-sumti"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(target_bridi) = self.resolved_goha_target_bridi_for_selbri(selbri)
            && let Some(target_selbri) = main_selbri_for_bridi(target_bridi)
        {
            return self.build_restrictive_formula(target_selbri, referent);
        }
        if let Some(target_bridi) = self.resolved_broda_target_bridi_for_selbri(selbri)
            && let Some(target_selbri) = main_selbri_for_bridi(target_bridi)
        {
            return self.build_restrictive_formula(target_selbri, referent);
        }
        if let Some(formula) =
            self.build_connected_restrictive_formula_for_selbri(selbri, referent)?
        {
            return Ok(formula);
        }
        if let Some(units) = tanru_units_for_selbri(selbri) {
            if let [unit] = units.as_slice()
                && (tanru_unit_is_event_modal_conversion(unit)
                    || tanru_unit_is_jai_conversion(unit))
            {
                return self.build_restrictive_tanru_formula(selbri, &units, referent);
            }
            if tanru_units_require_lowering(&units) {
                return self.build_restrictive_tanru_formula(selbri, &units, referent);
            }
        }
        let relation = relation_label_for_selbri(selbri);
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        let visible_x1_place = visible_x1_place_for_selbri(selbri);
        let mut arguments = BTreeMap::new();
        let highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        arguments.insert(
            format!("x{visible_x1_place}"),
            ArgumentValue::filled(referent, None),
        );
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(visible_x1_place) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-predication"));
        let eventuality = self.build_tagged_eventuality_for_selbri(selbri, source.clone())?;
        let relation_metadata =
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?;
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            eventuality,
            arguments,
            PredicationMode::Restrictive,
            source,
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.relation_metadata = relation_metadata;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "restrictive-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_tanru_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        units: &[&'tree TanruUnitSyntax],
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let [single] = units {
            return self.build_restrictive_tanru_unit_formula(selbri, single, referent);
        }
        let tertau = units
            .last()
            .expect("precondition guarantees at least one tanru unit");
        let tertau_formula = self.build_restrictive_tanru_unit_formula(selbri, tertau, referent)?;
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-tanru-formula"));
        let modifier =
            self.build_property_abstraction_for_units(&units[..units.len() - 1], source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            ArgumentValue::filled(referent, None),
            modifier,
            tanru_relation_name(units),
            PredicationMode::Restrictive,
            source.clone(),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau_formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "description".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn build_connected_restrictive_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-selbri-formula"));
        match selbri.as_data() {
            data!(SelbriSyntax::SelbriConnection {
                leading_selbri,
                connective,
                trailing_selbri,
            }) => self
                .build_connected_restrictive_formula_for_selbri_pair(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    referent,
                    source,
                )
                .map(Some),
            data!(SelbriSyntax::BoundSelbriConnection {
                leading_selbri,
                bo_connective: Some(connective),
                trailing_selbri,
                ..
            }) => self
                .build_connected_restrictive_formula_for_selbri_pair(
                    leading_selbri,
                    connective,
                    trailing_selbri,
                    referent,
                    source,
                )
                .map(Some),
            data!(SelbriSyntax::ForethoughtSelbriConnection {
                guhek,
                leading_bridi,
                trailing_bridi,
                ..
            }) => {
                let Some(leading_selbri) = main_selbri_for_bridi(leading_bridi) else {
                    return Ok(None);
                };
                let Some(trailing_selbri) = main_selbri_for_bridi(trailing_bridi) else {
                    return Ok(None);
                };
                self.build_connected_restrictive_formula_for_selbri_pair(
                    leading_selbri,
                    guhek,
                    trailing_selbri,
                    referent,
                    source,
                )
                .map(Some)
            }
            data!(SelbriSyntax::GroupedSelbri { selbri, .. })
            | data!(SelbriSyntax::TaggedSelbri {
                inner_selbri: selbri,
                ..
            }) => self.build_connected_restrictive_formula_for_selbri(selbri, referent),
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_restrictive_formula_for_selbri_pair(
        &mut self,
        leading_selbri: &'tree SelbriSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading = self.build_restrictive_formula(leading_selbri, referent)?;
        let trailing = self.build_restrictive_formula(trailing_selbri, referent)?;
        self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading, trailing],
            Some(connective_connector(connective, "selbri")),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_tanru_unit_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        unit: &'tree TanruUnitSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(target_bridi) = self.resolved_goha_target_bridi_for_tanru_unit(unit)
            && let Some(target_selbri) = main_selbri_for_bridi(target_bridi)
        {
            return self.build_restrictive_formula(target_selbri, referent);
        }
        if let Some(target_bridi) = self.resolved_broda_target_bridi_for_tanru_unit(unit)
            && let Some(target_selbri) = main_selbri_for_bridi(target_bridi)
        {
            return self.build_restrictive_formula(target_selbri, referent);
        }
        match unit.as_data() {
            data!(TanruUnitSyntax::TanruUnitConnection {
                leading_unit,
                connective,
                trailing_unit,
            }) => self.build_connected_restrictive_formula_for_tanru_units(
                selbri,
                leading_unit,
                connective,
                trailing_unit,
                referent,
            ),
            data!(TanruUnitSyntax::BoundTanruUnitConnection {
                leading_unit,
                bo_connective: Some(connective),
                trailing_unit,
                ..
            }) => self.build_connected_restrictive_formula_for_tanru_units(
                selbri,
                leading_unit,
                connective,
                trailing_unit,
                referent,
            ),
            data!(TanruUnitSyntax::GroupedTanruUnit {
                selbri: grouped,
                ..
            })
            | data!(TanruUnitSyntax::SelbriGroupTanruUnit(grouped)) => {
                if let Some(units) = tanru_units_for_selbri(grouped)
                    && tanru_units_require_lowering(&units)
                {
                    return self.build_restrictive_tanru_formula(grouped, &units, referent);
                }
                self.build_restrictive_formula(grouped, referent)
            }
            data!(TanruUnitSyntax::SumtiSelbri { sumti, .. }) => self
                .build_sumti_selbri_formula_for_frame(
                    sumti,
                    self.branch_frame_for_tanru_unit(unit),
                    self.analysis
                        .syntax_index
                        .selbri_node_id(selbri)
                        .and_then(|node| self.source_for_node(node.0, "restrictive-predication")),
                    Some(ArgumentValue::filled(referent, None)),
                    PredicationMode::Restrictive,
                )
                .map(|result| result.formula),
            _ => {
                if let Some((inner_unit, tense_modal)) = event_modal_conversion_for_tanru_unit(unit)
                {
                    return self.build_restrictive_event_modal_conversion_formula(
                        selbri,
                        unit,
                        inner_unit,
                        tense_modal,
                        referent,
                    );
                }
                if let Some((_inner_unit, tense_modal)) =
                    non_event_modal_jai_conversion_for_tanru_unit(unit)
                {
                    return self.build_restrictive_jai_modal_conversion_formula(
                        selbri,
                        unit,
                        tense_modal,
                        referent,
                    );
                }
                if bare_jai_conversion_for_tanru_unit(unit).is_some() {
                    return self
                        .build_restrictive_bare_jai_conversion_formula(selbri, unit, referent);
                }
                let relation = relation_label_for_tanru_unit(unit);
                let frame = self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit),
                );
                let visible_x1_place = visible_x1_place_for_tanru_unit(unit);
                let source = self
                    .analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "restrictive-predication"));
                let intrinsic_modal_arguments = self.tanru_unit_modal_arguments(unit)?;
                self.build_referent_predication_formula_for_relation(
                    relation,
                    frame,
                    visible_x1_place,
                    ArgumentValue::filled(referent, None),
                    intrinsic_modal_arguments,
                    PredicationMode::Restrictive,
                    source,
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_connected_restrictive_formula_for_tanru_units(
        &mut self,
        selbri: &'tree SelbriSyntax,
        leading_unit: &'tree TanruUnitSyntax,
        connective: &'tree ConnectiveSyntax,
        trailing_unit: &'tree TanruUnitSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading = self.build_restrictive_tanru_unit_formula(selbri, leading_unit, referent)?;
        let trailing =
            self.build_restrictive_tanru_unit_formula(selbri, trailing_unit, referent)?;
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-tanru-formula"));
        self.build_connective_formula(
            formula_operator_for_connective(connective),
            vec![leading, trailing],
            Some(connective_connector(connective, "tanru-unit")),
            source,
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_bare_jai_conversion_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        unit: &'tree TanruUnitSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-predication"));
        let abstract_referent = self.build_abstraction_about_referent(
            "jai",
            referent,
            self.source_for_tanru_unit(unit, "abstraction-about"),
        )?;
        let intrinsic_modal_arguments = self.tanru_unit_modal_arguments(unit)?;
        self.build_referent_predication_formula_for_relation(
            relation_label_for_tanru_unit(unit),
            self.semantic_predication_frame_for_tanru_unit(
                unit,
                self.branch_frame_for_tanru_unit(unit),
            ),
            visible_x1_place_for_tanru_unit(unit),
            ArgumentValue::filled(abstract_referent, None),
            intrinsic_modal_arguments,
            PredicationMode::Restrictive,
            source,
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_jai_modal_conversion_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        unit: &'tree TanruUnitSyntax,
        tense_modal: &'tree TenseModalSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = relation_label_for_tanru_unit(unit);
        let frame = self.semantic_predication_frame_for_tanru_unit(
            unit,
            self.branch_frame_for_tanru_unit(unit),
        );
        let mut arguments = BTreeMap::new();
        let highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let visible_x1_place = visible_x1_place_for_tanru_unit(unit);
        let visible_key = format!("x{visible_x1_place}");
        if !arguments.contains_key(&visible_key) {
            arguments.insert(
                visible_key,
                self.build_elided_argument_for_place(visible_x1_place)?,
            );
        }
        let mut modal_arguments = self.modal_assignment_arguments(frame)?;
        if let Some(modal_argument) =
            self.modal_argument_for_jai_conversion(tense_modal, referent)?
        {
            modal_arguments.push(modal_argument);
        }
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(visible_x1_place) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-predication"));
        let eventuality = self.build_tagged_eventuality_for_selbri(selbri, source.clone())?;
        let relation_metadata =
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?;
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            eventuality,
            arguments,
            PredicationMode::Restrictive,
            source.clone(),
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.relation_metadata = relation_metadata;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "restrictive-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_restrictive_event_modal_conversion_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        unit: &'tree TanruUnitSyntax,
        inner_unit: &'tree TanruUnitSyntax,
        tense_modal: &'tree TenseModalSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = relation_label_for_tanru_unit(inner_unit);
        let frame = self.semantic_predication_frame_for_tanru_unit(
            unit,
            self.branch_frame_for_tanru_unit(unit),
        );
        let mut arguments = BTreeMap::new();
        let highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        let mut diagnostics = Vec::new();
        match self.place_count_for_relation(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=highest_assigned_place.max(1) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
            }
        }
        let source = self
            .analysis
            .syntax_index
            .selbri_node_id(selbri)
            .and_then(|node| self.source_for_node(node.0, "restrictive-predication"));
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(EventualityClass::Event, None, source.clone());
        apply_tense_modal_event_modifiers_to_event_with_anchor(
            tense_modal,
            &mut event,
            Some(referent),
        );
        self.insert(eventuality, event)?;
        let relation_metadata =
            self.build_relation_metadata_for_selbri(selbri, &relation, source.clone())?;
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            PredicationMode::Restrictive,
            source.clone(),
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        object.relation_metadata = relation_metadata;
        self.insert(predication, object)?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "restrictive-formula")),
                Vec::new(),
            ),
        )
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_quantity_for_words(
        &mut self,
        text: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let value = parse_decimal_integer(&text)
            .map(QuantityValue::integer)
            .unwrap_or_else(|| QuantityValue::text(text.clone()));
        let id = self.next_quantity();
        self.insert(
            id,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                source,
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_quantity_for_sumti(
        &mut self,
        raw: RawSyntaxNodeId,
        sumti: &'tree SumtiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if da_series_scope_source(sumti).is_some() {
            return Ok(None);
        }
        let quantifier = argument_quantifier_for_sumti(sumti);
        quantifier
            .map(|quantifier| self.build_quantity_for_sumti_quantifier(raw, quantifier))
            .transpose()
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_quantity_for_sumti_quantifier(
        &mut self,
        raw: RawSyntaxNodeId,
        quantifier: &QuantifierSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(quantity) = self.sumti_quantities.get(&raw) {
            return Ok(*quantity);
        }
        let quantity = self.build_quantity_for_words(
            quantifier_text(quantifier).unwrap_or_else(|| "xo'e".to_owned()),
            self.source_for_quantifier(quantifier, "quantity")
                .or_else(|| self.source_for_node(raw, "quantity")),
        )?;
        self.sumti_quantities.insert(raw, quantity);
        Ok(quantity)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_description_quantity(
        &mut self,
        description: &'tree DescriptionSyntax,
        raw: RawSyntaxNodeId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let quantifier = description
            .description
            .as_ref()
            .and_then(|_| description_tail_quantifier(description));
        quantifier
            .map(|quantifier| {
                self.build_quantity_for_words(
                    quantifier_text(quantifier).unwrap_or_else(|| "xo'e".to_owned()),
                    self.source_for_quantifier(quantifier, "quantity")
                        .or_else(|| self.source_for_node(raw, "quantity")),
                )
            })
            .transpose()
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_abstraction_description_formula(
        &mut self,
        description_abstraction: DescriptionAbstraction<'tree>,
        referent: SemanticObjectId,
        frame: Option<SelbriPlaceFrameId>,
        link_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let abstraction = description_abstraction.abstraction;
        let kind = abstraction_kind_for_nu(abstraction);
        self.build_connected_abstraction_link_formula(
            abstraction,
            kind,
            description_abstraction.link_relation,
            ArgumentValue::filled(referent, None),
            frame,
            link_source,
            PredicationMode::Restrictive,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Abstraction) || ret.is_err())]
    fn build_abstraction_object(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        kind: AbstractionKind,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.abstraction_parameter_stack.push(Vec::new());
        self.indirect_question_stack.push(Vec::new());
        let body = match self
            .build_subbridi_formula(&abstraction.subbridi)
            .and_then(|body| {
                body.map(Ok)
                    .unwrap_or_else(|| self.build_diagnostic_abstraction_body_formula(abstraction))
            }) {
            Ok(body) => body,
            Err(error) => {
                let _ = self.abstraction_parameter_stack.pop();
                let _ = self.indirect_question_stack.pop();
                return Err(error);
            }
        };
        let indirect_questions = self
            .indirect_question_stack
            .pop()
            .expect("indirect question stack was just pushed");
        let parameters = self
            .abstraction_parameter_stack
            .pop()
            .expect("abstraction parameter stack was just pushed");
        let mut parameters = parameters;
        if kind == AbstractionKind::Property && parameters.is_empty() {
            let parameter_source =
                self.source_for_abstraction(abstraction, "implicit-property-slot");
            self.insert_implicit_property_slot_parameter(
                body,
                &mut parameters,
                parameter_source,
                main_selbri_for_subbridi(&abstraction.subbridi),
            )?;
        }
        self.set_formula_predication_mode(body, abstraction_body_mode(kind));
        let embedded_questions =
            self.build_embedded_indirect_questions(body, indirect_questions)?;

        let source = self.source_for_abstraction(abstraction, "abstraction");
        let abstraction_id = self.next_abstraction();
        let mut object =
            SemanticObject::abstraction(kind, body, parameters, source.clone(), Vec::new());
        object.embedded_questions = embedded_questions;
        self.insert(abstraction_id, object)?;
        Ok(abstraction_id)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|questions| questions.iter().all(|question| question.object_kind() == crate::model::SemanticObjectKind::Question)) || ret.is_err())]
    fn build_embedded_indirect_questions(
        &mut self,
        body: SemanticObjectId,
        foci: Vec<IndirectQuestionFocus>,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut questions = Vec::new();
        for focus in foci {
            let data!(IndirectQuestionFocus {
                focus,
                presupposed_answer,
                slots,
                kind,
                domain,
                source,
            }) = focus.into_data();
            let id = self.next_question();
            let mut object =
                SemanticObject::question(kind, QuestionMode::Indirect, domain, body, slots, source);
            object.focus = Some(focus);
            object.presupposed_answer = presupposed_answer;
            self.insert(id, object)?;
            questions.push(id);
        }
        Ok(questions)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_implicit_property_slot_parameter(
        &mut self,
        body: SemanticObjectId,
        parameters: &mut Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        preferred_selbri: Option<&'tree SelbriSyntax>,
    ) -> Result<(), SemanticsError> {
        if !parameters.is_empty() {
            return Ok(());
        }
        let parameter = self.build_parameter_with_source(
            "implicit ce'u".to_owned(),
            source,
            SemanticSort::Entity,
            crate::model::ParameterRole::PropertySlot,
        )?;
        if self.replace_first_elided_formula_argument(body, parameter, preferred_selbri)? {
            parameters.push(parameter);
        } else {
            self.objects.remove(&parameter);
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn replace_first_elided_formula_argument(
        &mut self,
        formula: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&'tree SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(false);
        };
        if let Some(predication) = object.predication
            && self.replace_first_elided_predication_argument(
                predication,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        for child in object.children {
            if self.replace_first_elided_formula_argument(child, parameter, preferred_selbri)? {
                return Ok(true);
            }
        }
        if let Some(restriction) = object.restriction
            && self.replace_first_elided_formula_argument(
                restriction,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        if let Some(body) = object.body
            && self.replace_first_elided_formula_argument(body, parameter, preferred_selbri)?
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn replace_first_elided_predication_argument(
        &mut self,
        predication: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&'tree SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let Some(object) = self.objects.get(&predication) else {
            return Ok(false);
        };
        let place = object
            .arguments
            .iter()
            .filter(|(_, argument)| argument.kind == ArgumentValueKind::Elided)
            .filter_map(|(place, _)| {
                argument_place_index(place).map(|index| {
                    let visible_rank = preferred_selbri
                        .map(|selbri| raw_place_visible_rank_for_selbri(selbri, index))
                        .unwrap_or(index);
                    (visible_rank, index, place)
                })
            })
            .min_by_key(|(visible_rank, index, _)| (*visible_rank, *index))
            .map(|(_, _, place)| place.clone());
        let Some(place) = place else {
            return Ok(false);
        };
        let old_value = {
            let object = self.objects.get_mut(&predication).ok_or_else(|| {
                SemanticsError::invalid_graph(format!(
                    "semantic builder could not find predication {predication}"
                ))
            })?;
            let argument = object.arguments.get_mut(&place).ok_or_else(|| {
                SemanticsError::invalid_graph(format!(
                    "semantic builder could not find predication argument {place}"
                ))
            })?;
            let old_value = argument.value;
            let source = argument.source.clone();
            *argument = ArgumentValue::filled(parameter, source);
            old_value
        };
        if let Some(old_value) = old_value {
            self.remove_unreferenced_elided_referent(old_value);
        }
        Ok(true)
    }

    #[requires(true)]
    #[ensures(true)]
    fn remove_unreferenced_elided_referent(&mut self, id: SemanticObjectId) {
        if id.object_kind() != crate::model::SemanticObjectKind::Referent {
            return;
        }
        let Some(object) = self.objects.get(&id) else {
            return;
        };
        if !object
            .descriptor
            .as_ref()
            .is_some_and(|descriptor| descriptor.kind == "elided")
        {
            return;
        }
        let mut references = Vec::new();
        for (object_id, object) in &self.objects {
            if *object_id != id {
                object.references_into(&mut references);
            }
        }
        if !references.contains(&id) {
            self.objects.remove(&id);
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_abstraction_link_formula_for_argument(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        kind: AbstractionKind,
        x1_argument: ArgumentValue,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_connected_abstraction_link_formula(
            abstraction,
            kind,
            abstraction_link_relation(kind),
            x1_argument,
            frame,
            source,
            mode,
        )
    }

    #[requires(!link_relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_abstraction_link_formula(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        kind: AbstractionKind,
        link_relation: &str,
        x1_argument: ArgumentValue,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = self.build_abstraction_link_atom_formula(
            abstraction,
            kind,
            link_relation,
            x1_argument.clone(),
            frame,
            source.clone(),
            mode,
        )?;
        for connection in &abstraction.abstractor_connections {
            let connection_kind = abstraction_kind_for_abstractor_connection(connection);
            let right_formula = self.build_abstraction_link_atom_formula(
                abstraction,
                connection_kind,
                abstraction_link_relation(connection_kind),
                x1_argument.clone(),
                frame,
                source.clone(),
                mode,
            )?;
            let connection_source =
                self.source_for_abstraction(abstraction, "abstraction-connection-formula");
            let left_formula = if connective_negates_left(&connection.connective) {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    formula,
                    connection_source.clone(),
                    Vec::new(),
                )?
            } else {
                formula
            };
            let right_formula = if connective_negates_right(&connection.connective) {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    right_formula,
                    connection_source.clone(),
                    Vec::new(),
                )?
            } else {
                right_formula
            };
            formula = self.build_connective_formula(
                formula_operator_for_connective(&connection.connective),
                vec![left_formula, right_formula],
                Some(Connector {
                    source: full_connective_text(&connection.connective),
                    locus: "abstraction".to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                connection_source,
            )?;
        }
        Ok(formula)
    }

    #[requires(!link_relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_abstraction_link_atom_formula(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
        kind: AbstractionKind,
        link_relation: &str,
        x1_argument: ArgumentValue,
        frame: Option<SelbriPlaceFrameId>,
        source: Option<crate::model::SemanticSource>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let abstraction_id = self.build_abstraction_object(abstraction, kind)?;
        let predication = self.next_predication();
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), x1_argument);
        arguments.insert("x2".to_owned(), ArgumentValue::filled(abstraction_id, None));
        self.insert_abstraction_link_extra_arguments(kind, frame, &mut arguments)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        self.insert(predication, {
            let mut object = SemanticObject::predication(
                link_relation.to_owned(),
                None,
                arguments,
                mode,
                source.clone(),
                Vec::new(),
            );
            object.modal_arguments = modal_arguments;
            object
        })?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                abstraction_link_formula_source(source, mode),
                Vec::new(),
            ),
        )
    }

    #[requires(arguments.contains_key("x1"))]
    #[requires(arguments.contains_key("x2"))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_abstraction_link_extra_arguments(
        &mut self,
        kind: AbstractionKind,
        frame: Option<SelbriPlaceFrameId>,
        arguments: &mut BTreeMap<String, ArgumentValue>,
    ) -> Result<(), SemanticsError> {
        let Some(surface_place) = abstraction_extra_surface_place(kind) else {
            return Ok(());
        };
        let value = if let Some(frame) = frame {
            match self.numbered_assignment_argument_for_frame(frame, surface_place)? {
                Some(argument) => argument,
                None => self.build_elided_argument_for_place_with_label_and_sort(
                    usize::from(surface_place),
                    SemanticSort::Entity,
                )?,
            }
        } else {
            self.build_elided_argument_for_place_with_label_and_sort(
                usize::from(surface_place),
                SemanticSort::Entity,
            )?
        };
        arguments.insert("x3".to_owned(), value);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_diagnostic_abstraction_body_formula(
        &mut self,
        abstraction: &'tree AbstractionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                "abstractionBody".to_owned(),
                None,
                BTreeMap::new(),
                PredicationMode::Inert,
                self.source_for_abstraction(abstraction, "abstraction-body"),
                vec![diagnostic("abstraction body is not fully lowered yet")],
            ),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.source_for_abstraction(abstraction, "abstraction-body"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret -> self.referent_descriptor_quantity_is(referent, quantity))]
    fn add_quantity_to_referent(
        &mut self,
        referent: SemanticObjectId,
        quantity: SemanticObjectId,
    ) -> bool {
        if let Some(object) = self.objects.get_mut(&referent) {
            object.set_descriptor_quantity(quantity);
            return self.referent_descriptor_quantity_is(referent, quantity);
        }
        false
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn add_assigned_name_to_referent(
        &mut self,
        referent: SemanticObjectId,
        assigned_name: AssignedName,
    ) {
        if let Some(object) = self.objects.get_mut(&referent) {
            object.push_assigned_name(assigned_name);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn referent_descriptor_quantity_is(
        &self,
        referent: SemanticObjectId,
        quantity: SemanticObjectId,
    ) -> bool {
        self.objects
            .get(&referent)
            .and_then(|object| object.descriptor.as_ref())
            .is_some_and(|descriptor| descriptor.quantity == Some(quantity))
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_object_diagnostic(&mut self, id: SemanticObjectId, diagnostic: SemanticDiagnostic) {
        let Some(object) = self.objects.get_mut(&id) else {
            return;
        };
        object.push_diagnostic(diagnostic);
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(true)]
    fn add_asides_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        asides: Vec<SemanticObjectId>,
    ) {
        if asides.is_empty() {
            return;
        }
        match item.object_kind() {
            crate::model::SemanticObjectKind::Utterance => self.add_utterance_asides(item, asides),
            crate::model::SemanticObjectKind::Sequence => {
                let first_item = self
                    .objects
                    .get(&item)
                    .and_then(|object| object.items.first().copied());
                if let Some(first_item) = first_item {
                    self.add_asides_to_discourse_item(first_item, asides);
                }
            }
            _ => {}
        }
    }

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(asides.iter().all(|aside| aside.object_kind() == crate::model::SemanticObjectKind::Utterance))]
    #[ensures(true)]
    fn add_utterance_asides(&mut self, utterance: SemanticObjectId, asides: Vec<SemanticObjectId>) {
        if asides.is_empty() {
            return;
        }
        let Some(object) = self.objects.get_mut(&utterance) else {
            return;
        };
        object.asides.extend(asides);
    }

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(audience.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn set_utterance_audience(&mut self, utterance: SemanticObjectId, audience: SemanticObjectId) {
        if let Some(object) = self.objects.get_mut(&utterance) {
            object.audience = Some(audience);
        }
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    fn set_referent_target(&mut self, referent: SemanticObjectId, target: SemanticObjectId) {
        if let Some(object) = self.objects.get_mut(&referent) {
            object.target = Some(target);
        }
    }

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(!kind.is_empty())]
    #[ensures(true)]
    fn set_vocative_kind(&mut self, utterance: SemanticObjectId, kind: String) {
        if let Some(object) = self.objects.get_mut(&utterance) {
            object.vocative_kind = Some(kind);
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    fn set_scalar_negation(
        &mut self,
        predication: SemanticObjectId,
        scalar_negation: ScalarNegation,
    ) {
        if let Some(object) = self.objects.get_mut(&predication) {
            object.scalar_negation = Some(scalar_negation);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn bare_description_tail_quantifier(description: &DescriptionSyntax) -> Option<&QuantifierSyntax> {
    if description.description.is_some() {
        return None;
    }
    description_tail_quantifier(description)
}

#[requires(true)]
#[ensures(true)]
fn description_tail_quantifier(description: &DescriptionSyntax) -> Option<&QuantifierSyntax> {
    description
        .tail_elements
        .iter()
        .find_map(|element| match element.as_data() {
            data!(
                jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailQuantifier(
                    quantifier
                )
            ) => Some(quantifier),
            _ => None,
        })
}

#[requires(true)]
#[ensures(true)]
fn argument_quantifier_for_sumti(sumti: &SumtiSyntax) -> Option<&QuantifierSyntax> {
    match sumti.as_data() {
        data!(SumtiSyntax::QuantifiedSumti { quantifier, .. }) => Some(quantifier),
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
            argument_quantifier_for_sumti(base_sumti)
        }
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ReferentSumti { inner_sumti, .. }) => {
            argument_quantifier_for_sumti(inner_sumti)
        }
        data!(SumtiSyntax::Description(description)) => description
            .outer_quantifier
            .as_deref()
            .or_else(|| bare_description_tail_quantifier(description)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn occurrence_relative_clauses_for_sumti<'a>(
    sumti: &'a SumtiSyntax,
    out: &mut Vec<&'a RelativeClauseSyntax>,
) {
    match sumti.as_data() {
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. }) => {
            occurrence_relative_clauses_for_sumti(inner_sumti, out);
        }
        data!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti,
            relative_clauses,
            ..
        })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti,
            relative_clauses,
            ..
        }) => {
            occurrence_relative_clauses_for_sumti(base_sumti, out);
            out.extend(relative_clauses.iter());
        }
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. }) => {
            occurrence_relative_clauses_for_sumti(inner_sumti, out);
        }
        data!(SumtiSyntax::ReferentSumti {
            relative_clauses,
            inner_sumti,
            ..
        }) => {
            out.extend(relative_clauses.iter());
            occurrence_relative_clauses_for_sumti(inner_sumti, out);
        }
        data!(SumtiSyntax::Description(description)) => {
            occurrence_relative_clauses_for_description_tail(&description.tail_elements, out);
            if description.description.is_none() {
                out.extend(description.relative_clauses.iter());
            }
        }
        data!(SumtiSyntax::DescriptionConnection(description)) => {
            occurrence_relative_clauses_for_description_tail(&description.tail_elements, out);
        }
        _ => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn occurrence_relative_clauses_for_description_tail<'a>(
    tail_elements: &'a [DescriptionTailElementSyntax],
    out: &mut Vec<&'a RelativeClauseSyntax>,
) {
    let mut saw_possessor_sumti = false;
    for element in tail_elements {
        match element.as_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(_)) => {
                saw_possessor_sumti = true;
            }
            data!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(clauses))
                if !saw_possessor_sumti =>
            {
                out.extend(
                    clauses
                        .iter()
                        .filter(|clause| !relative_clause_is_sumti_association_phrase(clause)),
                );
            }
            _ => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn descriptor_relative_clauses_for_description_tail<'a>(
    tail_elements: &'a [DescriptionTailElementSyntax],
    out: &mut Vec<&'a RelativeClauseSyntax>,
) {
    let mut saw_possessor_sumti = false;
    for element in tail_elements {
        match element.as_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(_)) => {
                saw_possessor_sumti = true;
            }
            data!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(clauses))
                if !saw_possessor_sumti =>
            {
                out.extend(
                    clauses
                        .iter()
                        .filter(|clause| relative_clause_is_sumti_association_phrase(clause)),
                );
            }
            _ => {}
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn possessive_sumti_relative_clauses<'a>(
    tail_elements: &'a [DescriptionTailElementSyntax],
) -> Vec<&'a RelativeClauseSyntax> {
    let mut after_possessor_sumti = false;
    let mut clauses = Vec::new();
    for element in tail_elements {
        match element.as_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(_)) => {
                after_possessor_sumti = true;
            }
            data!(
                DescriptionTailElementSyntax::DescriptionTailRelativeClauses(relative_clauses)
            ) if after_possessor_sumti => {
                clauses.extend(relative_clauses.iter());
            }
            _ => {}
        }
    }
    clauses
}

#[requires(true)]
#[ensures(ret.is_some() == tail_elements.iter().any(|element| matches!(element.as_data(), data!(DescriptionTailElementSyntax::DescriptionTailSumti(_)))))]
fn description_tail_sumti(tail_elements: &[DescriptionTailElementSyntax]) -> Option<&SumtiSyntax> {
    tail_elements
        .iter()
        .find_map(|element| match element.as_data() {
            data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => {
                Some(sumti.as_ref())
            }
            _ => None,
        })
}

#[requires(true)]
#[ensures(true)]
fn relative_clause_is_sumti_association_phrase(clause: &RelativeClauseSyntax) -> bool {
    match clause.as_data() {
        data!(RelativeClauseSyntax::SumtiAssociationPhrase(_)) => true,
        data!(RelativeClauseSyntax::JoinedRelativeClauses { inner, .. })
        | data!(RelativeClauseSyntax::RelativeClauseConnection { inner, .. }) => {
            relative_clause_is_sumti_association_phrase(inner)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn goi_assignment_phrase(clause: &RelativeClauseSyntax) -> Option<&SumtiAssociationPhraseSyntax> {
    match clause.as_data() {
        data!(RelativeClauseSyntax::SumtiAssociationPhrase(phrase))
            if phrase.association_marker.cmavo() == Some(Cmavo::Goi) =>
        {
            Some(phrase)
        }
        data!(RelativeClauseSyntax::JoinedRelativeClauses { inner, .. })
        | data!(RelativeClauseSyntax::RelativeClauseConnection { inner, .. }) => {
            goi_assignment_phrase(inner)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_is_assignable_reference(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => token.cmavo().is_some_and(is_assignable_koha),
        data!(SumtiSyntax::LerfuStringSumti { .. }) => true,
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
            sumti_is_assignable_reference(inner_sumti)
        }
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
            sumti_is_assignable_reference(base_sumti)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_assignable_koha(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Koha
            | Cmavo::Kohe
            | Cmavo::Kohi
            | Cmavo::Koho
            | Cmavo::Kohu
            | Cmavo::Foha
            | Cmavo::Fohe
            | Cmavo::Fohi
            | Cmavo::Foho
            | Cmavo::Fohu
    )
}

#[requires(true)]
#[ensures(true)]
fn sumti_deletes_place(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => token.cmavo() == Some(Cmavo::Ziho),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_is_elided(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => token.cmavo() == Some(Cmavo::Zohe),
        data!(SumtiSyntax::ElidedSumti { .. }) => true,
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_is_omitted_placeholder(sumti: &SumtiSyntax) -> bool {
    matches!(sumti.as_data(), data!(SumtiSyntax::ElidedSumti { .. }))
}

#[requires(true)]
#[ensures(true)]
fn voha_place_for_sumti(sumti: &SumtiSyntax) -> Option<usize> {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => match token.cmavo() {
            Some(Cmavo::Voha) => Some(1),
            Some(Cmavo::Vohe) => Some(2),
            Some(Cmavo::Vohi) => Some(3),
            Some(Cmavo::Voho) => Some(4),
            Some(Cmavo::Vohu) => Some(5),
            _ => None,
        },
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti: inner_sumti,
            ..
        })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti: inner_sumti,
            ..
        }) => voha_place_for_sumti(inner_sumti),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_question_word_for_tanru_unit(unit: &TanruUnitSyntax) -> Option<String> {
    match unit.as_data() {
        data!(TanruUnitSyntax::TanruUnitWord(word)) if word.cmavo() == Some(Cmavo::Mo) => {
            Some(token_text(&word.value))
        }
        data!(TanruUnitSyntax::ProBridi { goha, .. }) if goha.cmavo() == Some(Cmavo::Mo) => {
            Some(token_text(&goha.value))
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            relation_question_word_for_selbri(selbri)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
fn relation_variable_word_for_tanru_unit(unit: &TanruUnitSyntax) -> Option<String> {
    match unit.as_data() {
        data!(TanruUnitSyntax::ProBridi { goha, .. })
            if goha
                .cmavo()
                .is_some_and(|cmavo| matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)) =>
        {
            Some(token_text(&goha.value))
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            relation_variable_word_for_selbri(selbri)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_question_word_for_selbri(selbri: &SelbriSyntax) -> Option<String> {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriWord(token)) if token.cmavo() == Some(Cmavo::Mo) => {
            Some(token_text(token))
        }
        data!(SelbriSyntax::Tanru(units)) if units.len() == 1 => {
            relation_question_word_for_tanru_unit(units.first())
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => relation_question_word_for_selbri(selbri),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
fn relation_variable_word_for_selbri(selbri: &SelbriSyntax) -> Option<String> {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriWord(token))
            if token
                .cmavo()
                .is_some_and(|cmavo| matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)) =>
        {
            Some(token_text(token))
        }
        data!(SelbriSyntax::Tanru(units)) if units.len() == 1 => {
            relation_variable_word_for_tanru_unit(units.first())
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => relation_variable_word_for_selbri(selbri),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_is_single_relation_question(selbri: &SelbriSyntax) -> bool {
    relation_question_word_for_selbri(selbri).is_some()
}

#[requires(true)]
#[ensures(true)]
fn selbri_is_single_relation_variable(selbri: &SelbriSyntax) -> bool {
    relation_variable_word_for_selbri(selbri).is_some()
}

#[requires(true)]
#[ensures(true)]
fn free_modifiers_have_reciprocity(free_modifiers: &[FreeModifierSyntax]) -> bool {
    free_modifiers.iter().any(|free_modifier| {
        matches!(
            free_modifier.as_data(),
            data!(FreeModifierSyntax::ReciprocalSumti { .. })
        )
    })
}

#[requires(true)]
#[ensures(ret == indicators_have_indicator_cmavo(token.as_indicators(), cmavo))]
fn token_has_indicator_cmavo(token: &Token, cmavo: Cmavo) -> bool {
    indicators_have_indicator_cmavo(token.as_indicators(), cmavo)
}

#[requires(true)]
#[ensures(true)]
fn indicators_have_indicator_cmavo(indicators: &WithIndicators<WordLike>, cmavo: Cmavo) -> bool {
    match indicators {
        WithIndicators::Plain(_) | WithIndicators::Emphasized { .. } => false,
        WithIndicators::WithIndicator {
            base, indicator, ..
        } => indicator.cmavo() == Some(cmavo) || indicators_have_indicator_cmavo(base, cmavo),
    }
}

#[requires(true)]
#[ensures(true)]
fn with_free_modifiers_has_indicator_cmavo(token: &WithFreeModifiers<Token>, cmavo: Cmavo) -> bool {
    token_has_indicator_cmavo(&token.value, cmavo)
        || free_modifiers_have_indicator_cmavo(&token.free_modifiers, cmavo)
}

#[requires(true)]
#[ensures(true)]
fn word_run_has_indicator_cmavo(words: &WithFreeModifiers<WordRun>, cmavo: Cmavo) -> bool {
    let mut found = false;
    words.visit_words(&mut |token| {
        found |= token_has_indicator_cmavo(token, cmavo);
    });
    found
}

#[requires(true)]
#[ensures(true)]
fn free_modifiers_have_indicator_cmavo(
    free_modifiers: &[FreeModifierSyntax],
    cmavo: Cmavo,
) -> bool {
    let mut found = false;
    for free_modifier in free_modifiers {
        free_modifier.visit_words(&mut |token| {
            found |= token.cmavo() == Some(cmavo) || token_has_indicator_cmavo(token, cmavo);
        });
    }
    found
}

#[requires(true)]
#[ensures(true)]
fn sumti_has_current_kau_focus(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => {
            with_free_modifiers_has_indicator_cmavo(token, Cmavo::Kau)
        }
        data!(SumtiSyntax::NameDescription { names, .. })
        | data!(SumtiSyntax::NameWords(names)) => word_run_has_indicator_cmavo(names, Cmavo::Kau),
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::SumtiWithRelativeClauses {
            base_sumti: inner_sumti,
            ..
        })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
            base_sumti: inner_sumti,
            ..
        })
        | data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
            sumti_has_current_kau_focus(inner_sumti)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|token| token.cmavo() == Some(Cmavo::Ji) && token_has_indicator_cmavo(token, Cmavo::Kau)))]
fn connective_question_token_for_connective(connective: &ConnectiveSyntax) -> Option<&Token> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo.value.iter().find(|token| {
            token.cmavo() == Some(Cmavo::Ji) && token_has_indicator_cmavo(token, Cmavo::Kau)
        }),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi))))]
fn direct_connective_question_token_for_connective(
    connective: &ConnectiveSyntax,
) -> Option<&Token> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo.value.iter().find(|token| {
            matches!(
                token.cmavo(),
                Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi)
            )
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_indicator(indicator: &Indicator) -> Vec<IndicatorPart> {
    let mut parts = if let Some(cmavo) = indicator.indicator.core_word().cmavo() {
        vec![IndicatorPart {
            cmavo,
            nai: false,
            tokens: vec![Token::bare(indicator.indicator.core_word().clone())],
        }]
    } else {
        Vec::new()
    };
    parts.extend(indicator_parts_for_token(&indicator.indicator));
    if let Some(nai) = &indicator.nai
        && let Some(last) = parts.last_mut()
    {
        last.nai = true;
        last.tokens.push(Token::bare(WordLike::bare(nai.clone())));
    }
    parts
}

#[requires(true)]
#[ensures(!truth_question_consumed || ret.iter().all(|part| part.cmavo != Cmavo::Xu))]
fn leading_indicator_parts(
    indicators: &[Indicator],
    truth_question_consumed: bool,
) -> Vec<IndicatorPart> {
    indicators
        .iter()
        .flat_map(indicator_parts_for_indicator)
        .filter(|part| !truth_question_consumed || part.cmavo != Cmavo::Xu)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_sumti(sumti: &SumtiSyntax) -> Vec<IndicatorPart> {
    let mut parts = Vec::new();
    sumti.visit_words(&mut |token| {
        parts.extend(indicator_parts_for_token(token));
    });
    parts
}

#[requires(true)]
#[ensures(true)]
fn sumti_connection_has_branch_indicator_attachment(sumti: &SumtiSyntax) -> bool {
    matches!(
        sumti.as_data(),
        data!(SumtiSyntax::SumtiConnection { .. })
            | data!(SumtiSyntax::BoundSumtiConnection { .. })
            | data!(SumtiSyntax::ForethoughtSumtiConnection { .. })
    )
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_selbri(selbri: &SelbriSyntax) -> Vec<IndicatorPart> {
    let mut parts = Vec::new();
    selbri.visit_words(&mut |token| {
        parts.extend(indicator_parts_for_token(token));
    });
    parts
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_connective_cmavo(connective: &ConnectiveSyntax) -> Vec<IndicatorPart> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo
            .value
            .iter()
            .flat_map(indicator_parts_for_token)
            .collect(),
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_connective_nai(connective: &ConnectiveSyntax) -> Vec<IndicatorPart> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { nai, .. })
        | data!(ConnectiveSyntax::Selbri { nai, .. })
        | data!(ConnectiveSyntax::BridiTail { nai, .. })
        | data!(ConnectiveSyntax::Forethought { nai, .. })
        | data!(ConnectiveSyntax::NonLogical { nai, .. })
        | data!(ConnectiveSyntax::Interval { nai, .. }) => nai
            .as_ref()
            .map(|nai| indicator_parts_for_token(&nai.value))
            .unwrap_or_default(),
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_token(token: &Token) -> Vec<IndicatorPart> {
    let mut parts = Vec::new();
    indicator_parts_for_with_indicators(token.as_indicators(), &mut parts);
    parts
}

#[requires(true)]
#[ensures(true)]
fn indicator_parts_for_with_indicators(
    indicators: &WithIndicators<WordLike>,
    out: &mut Vec<IndicatorPart>,
) {
    match indicators {
        WithIndicators::Plain(_) | WithIndicators::Emphasized { .. } => {}
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            indicator_parts_for_with_indicators(base, out);
            let Some(cmavo) = indicator.cmavo() else {
                return;
            };
            let mut tokens = vec![Token::bare(WordLike::bare(indicator.clone()))];
            if let Some(nai) = nai {
                tokens.push(Token::bare(WordLike::bare(nai.clone())));
            }
            out.push(IndicatorPart {
                cmavo,
                nai: nai.is_some(),
                tokens,
            });
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_display_drafts(parts: Vec<IndicatorPart>) -> Vec<IndicatorDisplayDraft> {
    let mut drafts = Vec::new();
    let mut current: Option<IndicatorDisplayDraft> = None;
    let mut pending_question_tokens = Vec::new();
    for part in parts {
        if part.cmavo == Cmavo::Kau {
            continue;
        }
        if part.cmavo == Cmavo::Pei {
            if let Some(draft) = &mut current {
                draft.question = true;
                draft.source_tokens.extend(part.tokens);
            } else {
                pending_question_tokens.extend(part.tokens);
            }
            continue;
        }
        if let Some(draft) = current.as_mut()
            && apply_indicator_modifier_to_draft(draft, &part)
        {
            continue;
        }
        if current.is_none()
            && let Some(relation) = indicator_modifier_relation(part.cmavo)
        {
            current = Some(IndicatorDisplayDraft {
                family: DisplayedContentFamily::AttitudeModifier,
                relation: relation.to_owned(),
                polarity: if part.nai {
                    DisplayedContentPolarity::Negative
                } else {
                    DisplayedContentPolarity::Positive
                },
                assertion_effect: DisplayedContentAssertionEffect::None,
                intensity: None,
                phase: None,
                modifiers: Vec::new(),
                question: false,
                empathy: false,
                source_tokens: part.tokens,
            });
            continue;
        }
        if let Some(spec) = indicator_base_spec(part.cmavo) {
            if let Some(draft) = current.take() {
                drafts.push(draft);
            }
            let mut source_tokens = std::mem::take(&mut pending_question_tokens);
            source_tokens.extend(part.tokens);
            current = Some(IndicatorDisplayDraft {
                family: spec.family,
                relation: indicator_relation_for_polarity(spec.relation, part.nai).to_owned(),
                polarity: if part.nai {
                    DisplayedContentPolarity::Negative
                } else {
                    DisplayedContentPolarity::Positive
                },
                assertion_effect: spec.assertion_effect,
                intensity: None,
                phase: None,
                modifiers: Vec::new(),
                question: !source_tokens.is_empty() && source_tokens[0].cmavo() == Some(Cmavo::Pei),
                empathy: false,
                source_tokens,
            });
            continue;
        }
        let Some(draft) = current.as_mut() else {
            continue;
        };
        draft.source_tokens.extend(part.tokens.clone());
    }
    if let Some(draft) = current {
        drafts.push(draft);
    } else if !pending_question_tokens.is_empty() {
        drafts.push(IndicatorDisplayDraft {
            family: DisplayedContentFamily::QuestionPrompt,
            relation: "attitudeQuestion".to_owned(),
            polarity: DisplayedContentPolarity::Neutral,
            assertion_effect: DisplayedContentAssertionEffect::None,
            intensity: None,
            phase: None,
            modifiers: Vec::new(),
            question: false,
            empathy: false,
            source_tokens: pending_question_tokens,
        });
    }
    drafts
}

#[requires(true)]
#[ensures(true)]
fn apply_indicator_modifier_to_draft(
    draft: &mut IndicatorDisplayDraft,
    part: &IndicatorPart,
) -> bool {
    if let Some(intensity) = indicator_intensity(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.intensity = Some(intensity.to_owned());
        return true;
    }
    if let Some(polarity) = indicator_polarity_modifier(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.polarity = polarity;
        return true;
    }
    if let Some(phase) = indicator_phase(part.cmavo, part.nai) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.phase = Some(phase.to_owned());
        return true;
    }
    if part.cmavo == Cmavo::Dai {
        draft.source_tokens.extend(part.tokens.clone());
        draft.empathy = true;
        return true;
    }
    if let Some(relation) = indicator_modifier_relation(part.cmavo) {
        draft.source_tokens.extend(part.tokens.clone());
        draft.modifiers.push(new!(DisplayedContentModifier {
            relation: relation.to_owned(),
            family: None,
            polarity: Some(if part.nai {
                DisplayedContentPolarity::Negative
            } else {
                DisplayedContentPolarity::Positive
            }),
            intensity: None,
            assertion_effect: None,
            source: None,
        }));
        return true;
    }
    false
}

#[requires(!relation.is_empty())]
#[ensures(!ret.is_empty())]
fn attitude_question_relation(relation: &str) -> String {
    if relation.ends_with("Question") {
        relation.to_owned()
    } else {
        format!("{relation}Question")
    }
}

#[requires(!relation.is_empty())]
#[ensures(!ret.is_empty())]
fn indicator_relation_for_polarity(relation: &'static str, nai: bool) -> &'static str {
    match (relation, nai) {
        ("hope", true) => "despair",
        ("belief", true) => "disbelief",
        ("agreement", true) => "disagreement",
        ("approval", true) => "disapproval",
        ("obligation", true) => "freedom",
        ("permission", true) => "prohibition",
        ("competence", true) => "incompetence",
        ("desire", true) => "reluctance",
        ("interest", true) => "repulsion",
        ("surprise", true) => "expectation",
        ("happiness", true) => "unhappiness",
        ("love", true) => "hatred",
        ("respect", true) => "disrespect",
        ("patience", true) => "anger",
        ("relaxation", true) => "stress",
        ("caution", true) => "rashness",
        ("pity", true) => "cruelty",
        ("repentance", true) => "innocence",
        ("hypothetical", true) => "factual",
        ("figurative", true) => "literal",
        ("newInformation", true) => "oldInformation",
        _ => relation,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|spec| !spec.relation.is_empty()))]
fn indicator_base_spec(cmavo: Cmavo) -> Option<IndicatorBaseSpec> {
    let attitude = DisplayedContentAssertionEffect::HostSubordinated;
    let none = DisplayedContentAssertionEffect::None;
    let host = DisplayedContentAssertionEffect::HostAsserted;
    let performative = DisplayedContentAssertionEffect::Performative;
    let spec = match cmavo {
        Cmavo::Ua => (DisplayedContentFamily::Emotion, "discovery", none),
        Cmavo::Uha => (DisplayedContentFamily::Emotion, "gain", none),
        Cmavo::Ue => (DisplayedContentFamily::Emotion, "surprise", none),
        Cmavo::Ui => (DisplayedContentFamily::Emotion, "happiness", none),
        Cmavo::Uo => (DisplayedContentFamily::Emotion, "completion", none),
        Cmavo::Uu => (DisplayedContentFamily::Emotion, "pity", none),
        Cmavo::Uhu => (DisplayedContentFamily::Emotion, "repentance", none),
        Cmavo::Ii => (DisplayedContentFamily::Emotion, "fear", none),
        Cmavo::Iu => (DisplayedContentFamily::Emotion, "love", none),
        Cmavo::Io => (DisplayedContentFamily::Emotion, "respect", none),
        Cmavo::Oi => (DisplayedContentFamily::Emotion, "complaint", none),
        Cmavo::Ohi => (DisplayedContentFamily::Emotion, "caution", none),
        Cmavo::Ohe => (DisplayedContentFamily::Emotion, "detachment", none),
        Cmavo::Oho => (DisplayedContentFamily::Emotion, "patience", none),
        Cmavo::Ohu => (DisplayedContentFamily::Emotion, "relaxation", none),
        Cmavo::Aha => (
            DisplayedContentFamily::PropositionalAttitude,
            "attention",
            attitude,
        ),
        Cmavo::Ahe => (
            DisplayedContentFamily::PropositionalAttitude,
            "alertness",
            attitude,
        ),
        Cmavo::Ai => (
            DisplayedContentFamily::PropositionalAttitude,
            "intent",
            attitude,
        ),
        Cmavo::Ahi => (
            DisplayedContentFamily::PropositionalAttitude,
            "effort",
            attitude,
        ),
        Cmavo::Aho => (
            DisplayedContentFamily::PropositionalAttitude,
            "hope",
            attitude,
        ),
        Cmavo::Au => (
            DisplayedContentFamily::PropositionalAttitude,
            "desire",
            attitude,
        ),
        Cmavo::Ahu => (
            DisplayedContentFamily::PropositionalAttitude,
            "interest",
            attitude,
        ),
        Cmavo::Eha => (
            DisplayedContentFamily::PropositionalAttitude,
            "permission",
            attitude,
        ),
        Cmavo::Ehe => (
            DisplayedContentFamily::PropositionalAttitude,
            "competence",
            attitude,
        ),
        Cmavo::Ei => (
            DisplayedContentFamily::PropositionalAttitude,
            "obligation",
            attitude,
        ),
        Cmavo::Eho => (
            DisplayedContentFamily::PropositionalAttitude,
            "request",
            attitude,
        ),
        Cmavo::Ehu => (
            DisplayedContentFamily::PropositionalAttitude,
            "suggestion",
            attitude,
        ),
        Cmavo::Ia => (
            DisplayedContentFamily::PropositionalAttitude,
            "belief",
            attitude,
        ),
        Cmavo::Iha => (
            DisplayedContentFamily::PropositionalAttitude,
            "acceptance",
            attitude,
        ),
        Cmavo::Ie => (
            DisplayedContentFamily::PropositionalAttitude,
            "agreement",
            attitude,
        ),
        Cmavo::Ihe => (
            DisplayedContentFamily::PropositionalAttitude,
            "approval",
            attitude,
        ),
        Cmavo::Cahe => (
            DisplayedContentFamily::Evidential,
            "definition",
            performative,
        ),
        Cmavo::Baha => (DisplayedContentFamily::Evidential, "expectation", host),
        Cmavo::Tihe => (DisplayedContentFamily::Evidential, "hearsay", host),
        Cmavo::Zaha => (DisplayedContentFamily::Evidential, "observation", host),
        Cmavo::Pehi => (DisplayedContentFamily::Evidential, "opinion", host),
        Cmavo::Ruha => (DisplayedContentFamily::Evidential, "presumption", host),
        Cmavo::Juho => (DisplayedContentFamily::Discursive, "certainty", attitude),
        Cmavo::Dahi => (DisplayedContentFamily::Discursive, "hypothetical", attitude),
        Cmavo::Poho => (DisplayedContentFamily::Discursive, "onlyRelevantCase", none),
        Cmavo::Kiha => (DisplayedContentFamily::Metalinguistic, "confusion", none),
        Cmavo::Peha => (DisplayedContentFamily::Metalinguistic, "figurative", none),
        Cmavo::Pau => (
            DisplayedContentFamily::QuestionPrompt,
            "questionPrompt",
            none,
        ),
        Cmavo::Xu => (
            DisplayedContentFamily::QuestionPrompt,
            "truthQuestionPrompt",
            none,
        ),
        Cmavo::Gehe => (
            DisplayedContentFamily::Metalinguistic,
            "unspecifiedAttitude",
            none,
        ),
        _ if cmavo.is_selmaho(jbotci_morphology::Selmaho::Ui) => (
            DisplayedContentFamily::Metalinguistic,
            cmavo.canonical_text(),
            none,
        ),
        _ => return None,
    };
    Some(IndicatorBaseSpec {
        family: spec.0,
        relation: spec.1,
        assertion_effect: spec.2,
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|intensity| !intensity.is_empty()))]
fn indicator_intensity(cmavo: Cmavo, nai: bool) -> Option<&'static str> {
    match (cmavo, nai) {
        (Cmavo::Cai, false) => Some("maximal"),
        (Cmavo::Sai, false) => Some("strong"),
        (Cmavo::Ruhe, false) => Some("weak"),
        (Cmavo::Cai, true) => Some("negativeMaximal"),
        (Cmavo::Sai, true) => Some("negativeStrong"),
        (Cmavo::Ruhe, true) => Some("negativeWeak"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_polarity_modifier(cmavo: Cmavo, nai: bool) -> Option<DisplayedContentPolarity> {
    match (cmavo, nai) {
        (Cmavo::Cuhi, false) => Some(DisplayedContentPolarity::Neutral),
        (Cmavo::Cuhi, true) => Some(DisplayedContentPolarity::Negative),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|phase| !phase.is_empty()))]
fn indicator_phase(cmavo: Cmavo, nai: bool) -> Option<&'static str> {
    match (cmavo, nai) {
        (Cmavo::Buho, false) => Some("starting"),
        (Cmavo::Buho, true) => Some("ending"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|relation| !relation.is_empty()))]
fn indicator_modifier_relation(cmavo: Cmavo) -> Option<&'static str> {
    match cmavo {
        Cmavo::Gahi => Some("rank"),
        Cmavo::Sehi => Some("selfOrientation"),
        Cmavo::Rihe => Some("emotionalRelease"),
        Cmavo::Behu => Some("need"),
        Cmavo::Seha => Some("selfSufficiency"),
        Cmavo::Roho => Some("physical"),
        Cmavo::Rehe => Some("spiritual"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn direct_statement_bridi(statement: &StatementSyntax) -> Option<&BridiSyntax> {
    match statement.as_data() {
        data!(StatementSyntax::Bridi(bridi)) => Some(bridi),
        data!(StatementSyntax::Prenex {
            inner_statement,
            ..
        })
        | data!(StatementSyntax::Iau {
            inner_statement,
            ..
        }) => direct_statement_bridi(inner_statement),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_place_index(place: &str) -> Option<usize> {
    let digits = place.strip_prefix('x')?;
    if digits.is_empty() || digits.starts_with('0') {
        return None;
    }
    digits.parse::<usize>().ok()
}

#[requires(true)]
#[ensures(ret > 0)]
fn visible_x1_place_for_selbri(selbri: &SelbriSyntax) -> usize {
    visible_place_for_selbri(selbri, 1)
}

#[requires(true)]
#[ensures(ret > 0)]
fn first_unfilled_visible_place_for_selbri(
    selbri: &SelbriSyntax,
    arguments: &BTreeMap<String, ArgumentValue>,
    highest_assigned_place: usize,
) -> usize {
    for visible_place in 1..=highest_assigned_place.max(1) + 1 {
        let place = visible_place_for_selbri(selbri, visible_place);
        if !arguments.contains_key(&format!("x{place}")) {
            return place;
        }
    }
    visible_place_for_selbri(selbri, highest_assigned_place.max(1) + 2)
}

#[requires(place > 0)]
#[ensures(ret > 0)]
fn visible_place_for_selbri(selbri: &SelbriSyntax, place: usize) -> usize {
    match selbri.as_data() {
        data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => {
            let converted_place = se_conversion_place(se).unwrap_or(2);
            visible_place_for_selbri(inner_selbri, convert_numbered_place(place, converted_place))
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => visible_place_for_selbri(selbri, place),
        _ => place,
    }
}

#[requires(place > 0)]
#[ensures(ret > 0)]
fn raw_place_visible_rank_for_selbri(selbri: &SelbriSyntax, place: usize) -> usize {
    match selbri.as_data() {
        data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => {
            let converted_place = se_conversion_place(se).unwrap_or(2);
            convert_numbered_place(
                raw_place_visible_rank_for_selbri(inner_selbri, place),
                converted_place,
            )
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => raw_place_visible_rank_for_selbri(selbri, place),
        _ => place,
    }
}

#[requires(true)]
#[ensures(ret > 0)]
fn visible_x1_place_for_tanru_unit(unit: &TanruUnitSyntax) -> usize {
    visible_place_for_tanru_unit(unit, 1)
}

#[requires(place > 0)]
#[ensures(ret > 0)]
fn visible_place_for_tanru_unit(unit: &TanruUnitSyntax, place: usize) -> usize {
    match unit.as_data() {
        data!(TanruUnitSyntax::ConvertedTanruUnit { se, inner_unit }) => {
            let converted_place = se_conversion_place(se).unwrap_or(2);
            visible_place_for_tanru_unit(inner_unit, convert_numbered_place(place, converted_place))
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            visible_place_for_selbri(selbri, place)
        }
        data!(TanruUnitSyntax::RelativeClauses { base, .. })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => {
            visible_place_for_tanru_unit(base, place)
        }
        data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => {
            visible_place_for_tanru_unit(inner_unit, place)
        }
        _ => place,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
fn se_conversion_place(se: &WithFreeModifiers<Token>) -> Option<usize> {
    match se.value.cmavo() {
        Some(Cmavo::Se) => Some(2),
        Some(Cmavo::Te) => Some(3),
        Some(Cmavo::Ve) => Some(4),
        Some(Cmavo::Xe) => Some(5),
        _ => None,
    }
}

#[requires(place > 0)]
#[requires(converted_place > 0)]
#[ensures(ret > 0)]
fn convert_numbered_place(place: usize, converted_place: usize) -> usize {
    if place == 1 {
        converted_place
    } else if place == converted_place {
        1
    } else {
        place
    }
}

#[requires(true)]
#[ensures(true)]
fn logical_sumti_connection_parts(
    sumti: &SumtiSyntax,
) -> Option<(
    &SumtiSyntax,
    &ConnectiveSyntax,
    Option<&TenseModalSyntax>,
    &SumtiSyntax,
)> {
    match sumti.as_data() {
        data!(SumtiSyntax::SumtiConnection {
            leading_sumti,
            connective,
            trailing_sumti,
        }) if connective_is_logical(connective) => {
            Some((leading_sumti, connective, None, trailing_sumti))
        }
        data!(SumtiSyntax::BoundSumtiConnection {
            leading_sumti,
            bo_connective,
            bo_tense_modal,
            trailing_sumti,
            ..
        }) => bo_connective
            .as_deref()
            .filter(|connective| connective_is_logical(connective))
            .map(|connective| {
                (
                    leading_sumti.as_ref(),
                    connective,
                    bo_tense_modal.as_deref(),
                    trailing_sumti.as_ref(),
                )
            }),
        data!(SumtiSyntax::ForethoughtSumtiConnection {
            leading_sumti,
            gek,
            trailing_sumti,
            ..
        }) if connective_is_logical(gek) => Some((leading_sumti, gek, None, trailing_sumti)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn logical_sumti_connection_parts_degrouped(
    sumti: &SumtiSyntax,
) -> Option<(
    &SumtiSyntax,
    &ConnectiveSyntax,
    Option<&TenseModalSyntax>,
    &SumtiSyntax,
)> {
    match sumti.as_data() {
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. }) => {
            logical_sumti_connection_parts_degrouped(inner_sumti)
        }
        _ => logical_sumti_connection_parts(sumti),
    }
}

#[requires(true)]
#[ensures(true)]
fn text_group_tense_modal(statement: &StatementSyntax) -> Option<&TenseModalSyntax> {
    match statement.as_data() {
        data!(StatementSyntax::TextGroup { tense_modal, .. }) => tense_modal.as_deref(),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_is_logical(connective: &ConnectiveSyntax) -> bool {
    !matches!(
        connective.as_data(),
        data!(ConnectiveSyntax::NonLogical { .. }) | data!(ConnectiveSyntax::Interval { .. })
    )
}

#[requires(true)]
#[ensures(matches!(ret, Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)) == connective_is_interval(connective))]
fn connective_primary_cmavo(connective: &ConnectiveSyntax) -> Option<Cmavo> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo
            .value
            .iter()
            .filter_map(Token::cmavo)
            .find(|cmavo| Selmaho::Joi.contains(*cmavo) || Selmaho::Bihi.contains(*cmavo)),
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_is_interval(connective: &ConnectiveSyntax) -> bool {
    matches!(
        connective.as_data(),
        data!(ConnectiveSyntax::Interval { .. })
    )
}

#[requires(true)]
#[ensures(true)]
fn connective_has_se_conversion(connective: &ConnectiveSyntax) -> bool {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { se, .. })
        | data!(ConnectiveSyntax::Selbri { se, .. })
        | data!(ConnectiveSyntax::BridiTail { se, .. })
        | data!(ConnectiveSyntax::Forethought { se, .. })
        | data!(ConnectiveSyntax::NonLogical { se, .. })
        | data!(ConnectiveSyntax::Interval { se, .. }) => se.is_some(),
    }
}

#[requires(true)]
#[ensures(!connective_has_se_conversion(connective) -> !ret)]
fn connective_reverses_composition_members(connective: &ConnectiveSyntax) -> bool {
    connective_has_se_conversion(connective)
        && matches!(
            connective_primary_cmavo(connective),
            Some(Cmavo::Ceho | Cmavo::Fahu | Cmavo::Pihu | Cmavo::Biho | Cmavo::Mihi)
        )
}

#[requires(true)]
#[ensures(true)]
fn connective_has_logical_component(connective: &ConnectiveSyntax) -> bool {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => cmavo.value.iter().any(|token| {
            matches!(
                token.cmavo(),
                Some(
                    Cmavo::A
                        | Cmavo::E
                        | Cmavo::O
                        | Cmavo::U
                        | Cmavo::Ga
                        | Cmavo::Ge
                        | Cmavo::Go
                        | Cmavo::Gu
                        | Cmavo::Giha
                        | Cmavo::Gihe
                        | Cmavo::Giho
                        | Cmavo::Gihu
                        | Cmavo::Ja
                        | Cmavo::Je
                        | Cmavo::Jo
                        | Cmavo::Ju
                )
            )
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_has_formula_scope(selbri: &SelbriSyntax) -> bool {
    match selbri.as_data() {
        data!(SelbriSyntax::Negated { .. }) => true,
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::GroupedSelbri {
            selbri: inner_selbri,
            ..
        }) => selbri_has_formula_scope(inner_selbri),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn apply_selbri_event_modifiers_to_event(selbri: &SelbriSyntax, event: &mut SemanticObject) {
    apply_selbri_event_modifiers_to_event_with_anchor(selbri, event, None);
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn apply_selbri_event_modifiers_to_event_with_anchor(
    selbri: &SelbriSyntax,
    event: &mut SemanticObject,
    anchor: Option<SemanticObjectId>,
) {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => {
            let local_anchor =
                anchor.filter(|_| event.time_path.is_empty() && event.time.is_none());
            apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
                tense_modal,
                event,
                local_anchor,
                false,
            );
            apply_selbri_event_modifiers_to_event_with_anchor(inner_selbri, event, anchor);
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            selbri,
            ..
        }) => {
            if let Some(tense_modal) = ke_tense_modal {
                let local_anchor =
                    anchor.filter(|_| event.time_path.is_empty() && event.time.is_none());
                apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
                    tense_modal,
                    event,
                    local_anchor,
                    false,
                );
            }
            apply_selbri_event_modifiers_to_event_with_anchor(selbri, event, anchor);
        }
        _ => {}
    }
    normalize_event_time_path(event);
    normalize_event_space_path(event);
}

#[requires(true)]
#[ensures(true)]
fn apply_tense_modal_event_modifiers_to_event(
    tense_modal: &TenseModalSyntax,
    event: &mut SemanticObject,
) {
    apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
        tense_modal,
        event,
        None,
        true,
    );
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn apply_tense_modal_event_modifiers_to_event_with_anchor(
    tense_modal: &TenseModalSyntax,
    event: &mut SemanticObject,
    anchor: Option<SemanticObjectId>,
) {
    apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
        tense_modal,
        event,
        anchor,
        true,
    );
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn apply_tense_modal_event_modifiers_to_event_with_anchor_and_normalization(
    tense_modal: &TenseModalSyntax,
    event: &mut SemanticObject,
    anchor: Option<SemanticObjectId>,
    normalize_time_path: bool,
) {
    if let Some(actuality) = actuality_for_tense_modal(tense_modal) {
        event.actuality = Some(actuality);
    }
    let time_span = time_span_for_tense_modal_with_anchor(tense_modal, anchor);
    if time_span.is_none() {
        append_temporal_path_relations_to_event(
            event,
            temporal_path_relations_for_tense_modal(tense_modal),
            anchor,
        );
    }
    if let Some(time_interval) = time_interval_for_tense_modal_with_anchor(tense_modal, anchor) {
        event.time_interval = Some(time_interval);
    }
    if let Some(time_span) = time_span {
        event.time_span = Some(time_span);
    }
    append_space_path_relations_to_event(
        event,
        space_path_relations_for_tense_modal(tense_modal),
        anchor,
    );
    if let Some(space_interval) = space_interval_for_tense_modal_with_anchor(tense_modal, anchor) {
        event.space_interval = Some(space_interval);
    }
    apply_aspect_contours_to_event(
        event,
        temporal_aspect_contours_for_tense_modal(tense_modal),
        anchor,
        modal_scalar_negation_for_tense_modal(tense_modal),
        false,
    );
    event.recurrence.extend(
        temporal_recurrences_for_tense_modal(tense_modal)
            .into_iter()
            .map(|recurrence| recurrence_with_interval(recurrence, anchor)),
    );
    apply_aspect_contours_to_event(
        event,
        spatial_aspect_contours_for_tense_modal(tense_modal),
        anchor,
        modal_scalar_negation_for_tense_modal(tense_modal),
        true,
    );
    event.spatial_recurrence.extend(
        spatial_recurrences_for_tense_modal(tense_modal)
            .into_iter()
            .map(|recurrence| recurrence_with_interval(recurrence, anchor)),
    );
    if normalize_time_path {
        normalize_event_time_path(event);
        normalize_event_space_path(event);
    }
}

#[requires(true)]
#[ensures(true)]
fn attach_magnitude_to_event_modifier(
    event: &mut SemanticObject,
    tense_modal: &TenseModalSyntax,
    magnitude: AnchorMagnitude,
) {
    if !space_path_relations_for_tense_modal(tense_modal).is_empty() {
        if let Some(step) = event.space_path.pop() {
            event
                .space_path
                .push(step.with_data(data! { magnitude: Some(magnitude) }));
        } else if let Some(space) = event.space.as_mut() {
            let updated = space
                .clone()
                .with_data(data! { magnitude: Some(magnitude) });
            event.space = Some(updated);
        }
        return;
    }
    if !temporal_path_relations_for_tense_modal(tense_modal).is_empty() {
        if let Some(step) = event.time_path.pop() {
            event
                .time_path
                .push(step.with_data(data! { magnitude: Some(magnitude) }));
        } else if let Some(time) = event.time.as_mut() {
            let updated = time.clone().with_data(data! { magnitude: Some(magnitude) });
            event.time = Some(updated);
        }
    }
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn append_temporal_path_relations_to_event(
    event: &mut SemanticObject,
    relations: Vec<TemporalPathRelation>,
    anchor: Option<SemanticObjectId>,
) {
    if relations.is_empty() {
        return;
    }
    if let Some(time) = event.time.take() {
        let data!(AnchorRelation {
            relation,
            anchor,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }) = time.into_data();
        event.time_path.push(TemporalPathStep::new(
            relation,
            TemporalPathAnchor::object(anchor),
            "implicit".to_owned(),
            distance,
            magnitude,
            scalar_negation,
            motion,
        ));
    }
    let mut first_relation = true;
    for relation in relations {
        let path_anchor = if first_relation && let Some(anchor) = anchor {
            TemporalPathAnchor::object(anchor)
        } else if event.time_path.is_empty() {
            TemporalPathAnchor::object(SemanticObjectId::speech_time())
        } else {
            TemporalPathAnchor::previous()
        };
        first_relation = false;
        event.time_path.push(TemporalPathStep::new(
            relation.relation,
            path_anchor,
            relation.introduced_by,
            relation.distance,
            None,
            relation.scalar_negation,
            relation.motion,
        ));
    }
}

#[requires(true)]
#[ensures(event.time.is_none() || event.time_path.is_empty())]
fn normalize_event_time_path(event: &mut SemanticObject) {
    if event.time_path.len() != 1 {
        if !event.time_path.is_empty() {
            event.time = None;
        }
        return;
    }
    let step = event.time_path.pop().expect("single temporal path step");
    let data!(TemporalPathStep {
        relation,
        anchor,
        introduced_by: _,
        distance,
        magnitude,
        scalar_negation,
        motion,
    }) = step.into_data();
    if let Some(anchor) = anchor.object_id() {
        event.time = Some(new!(AnchorRelation {
            relation,
            anchor,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }));
    } else {
        event.time_path.push(TemporalPathStep::new(
            relation,
            anchor,
            "implicit".to_owned(),
            distance,
            magnitude,
            scalar_negation,
            motion,
        ));
    }
}

#[requires(true)]
#[ensures(true)]
fn clear_event_time_path(event: &mut SemanticObject) {
    event.time = None;
    event.time_path.clear();
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn apply_aspect_contours_to_event(
    event: &mut SemanticObject,
    contours: Vec<String>,
    anchor: Option<SemanticObjectId>,
    scalar_negation: Option<ScalarNegation>,
    spatial: bool,
) {
    let mut aspects = contours
        .into_iter()
        .map(|contour| Aspect::new_with_polarity(contour, anchor, scalar_negation.clone()))
        .collect::<Vec<_>>();
    if aspects.len() == 1 {
        if spatial {
            event.spatial_aspect = aspects.pop();
        } else {
            event.aspect = aspects.pop();
        }
    } else if !aspects.is_empty() {
        if spatial {
            event.spatial_aspects.extend(aspects);
        } else {
            event.aspects.extend(aspects);
        }
    }
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn append_space_path_relations_to_event(
    event: &mut SemanticObject,
    relations: Vec<TemporalPathRelation>,
    anchor: Option<SemanticObjectId>,
) {
    if relations.is_empty() {
        return;
    }
    if let Some(space) = event.space.take() {
        let data!(AnchorRelation {
            relation,
            anchor,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }) = space.into_data();
        event.space_path.push(TemporalPathStep::new(
            relation,
            TemporalPathAnchor::object(anchor),
            "implicit".to_owned(),
            distance,
            magnitude,
            scalar_negation,
            motion,
        ));
    }
    let mut first_relation = true;
    for relation in relations {
        let path_anchor = if first_relation && let Some(anchor) = anchor {
            TemporalPathAnchor::object(anchor)
        } else if event.space_path.is_empty() {
            TemporalPathAnchor::object(SemanticObjectId::here())
        } else {
            TemporalPathAnchor::previous()
        };
        first_relation = false;
        event.space_path.push(TemporalPathStep::new(
            relation.relation,
            path_anchor,
            relation.introduced_by,
            relation.distance,
            None,
            relation.scalar_negation,
            relation.motion,
        ));
    }
}

#[requires(true)]
#[ensures(event.space.is_none() || event.space_path.is_empty())]
fn normalize_event_space_path(event: &mut SemanticObject) {
    if event.space_path.len() != 1 {
        if !event.space_path.is_empty() {
            event.space = None;
        }
        return;
    }
    let step = event.space_path.pop().expect("single spatial path step");
    let data!(TemporalPathStep {
        relation,
        anchor,
        introduced_by: _,
        distance,
        magnitude,
        scalar_negation,
        motion,
    }) = step.into_data();
    if let Some(anchor) = anchor.object_id() {
        event.space = Some(new!(AnchorRelation {
            relation,
            anchor,
            distance,
            magnitude,
            scalar_negation,
            motion,
        }));
    } else {
        event.space_path.push(TemporalPathStep::new(
            relation,
            anchor,
            "implicit".to_owned(),
            distance,
            magnitude,
            scalar_negation,
            motion,
        ));
    }
}

#[requires(true)]
#[ensures(true)]
fn clear_event_space_path(event: &mut SemanticObject) {
    event.space = None;
    event.space_path.clear();
}

#[requires(event.object_type == crate::model::SemanticObjectKind::Eventuality)]
#[ensures(true)]
fn clear_event_modifiers(event: &mut SemanticObject) {
    event.actuality = None;
    event.tense_modal = None;
    event.time = None;
    event.time_path.clear();
    event.time_interval = None;
    event.time_span = None;
    event.aspect = None;
    event.aspects.clear();
    event.recurrence.clear();
    event.space = None;
    event.space_path.clear();
    event.space_interval = None;
    event.spatial_aspect = None;
    event.spatial_aspects.clear();
    event.spatial_recurrence.clear();
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_anchors_to_speech_time(tense_modal: &TenseModalSyntax) -> bool {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts.value.iter().any(|part| {
            matches!(
                part.as_data(),
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(token))
                    if token.cmavo() == Some(Cmavo::Nau)
            )
        }),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_has_event_modifier(tense_modal: &TenseModalSyntax) -> bool {
    tense_modal_anchors_to_speech_time(tense_modal)
        || tense_question_token_for_tense_modal(tense_modal).is_some()
        || actuality_for_tense_modal(tense_modal).is_some()
        || !temporal_path_relations_for_tense_modal(tense_modal).is_empty()
        || time_interval_for_tense_modal(tense_modal).is_some()
        || !space_path_relations_for_tense_modal(tense_modal).is_empty()
        || space_interval_for_tense_modal(tense_modal).is_some()
        || !temporal_aspect_contours_for_tense_modal(tense_modal).is_empty()
        || !temporal_recurrences_for_tense_modal(tense_modal).is_empty()
        || !spatial_aspect_contours_for_tense_modal(tense_modal).is_empty()
        || !spatial_recurrences_for_tense_modal(tense_modal).is_empty()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_is_lahu_modal(tense_modal: &TenseModalSyntax) -> bool {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Modal { bai, .. }) => bai.value.is_cmavo(Cmavo::Lahu),
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|token| token.is_cmavo(Cmavo::Cuhe)))]
fn tense_question_token_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<&Token> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts.value.iter().find_map(|part| {
            let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
                return None;
            };
            token.is_cmavo(Cmavo::Cuhe).then_some(token)
        }),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|token| token.is_cmavo(Cmavo::Jehi)))]
fn connective_question_token_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<&Token> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts.value.iter().find_map(|part| {
            let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
                return None;
            };
            token.is_cmavo(Cmavo::Jehi).then_some(token)
        }),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_has_contradictory_event_negation(tense_modal: &TenseModalSyntax) -> bool {
    modal_negation_for_tense_modal(tense_modal).is_some()
        && !matches!(
            tense_modal.as_data(),
            data!(TenseModalSyntax::Modal { .. }) | data!(TenseModalSyntax::AdHocModal { .. })
        )
        && tense_modal_has_event_modifier(tense_modal)
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_bridi(
    bridi: &BridiSyntax,
) -> Option<&TenseModalSyntax> {
    bridi
        .leading_terms
        .iter()
        .find_map(first_contradictory_event_tense_modal_for_term)
        .or_else(|| first_contradictory_event_tense_modal_for_bridi_tail(&bridi.bridi_tail))
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_bridi<'a>(
    bridi: &'a BridiSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_terms(&bridi.leading_terms, out);
    collect_bridi_negation_terms_for_bridi_tail(&bridi.bridi_tail, out);
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_bridi_tail<'a>(
    tail: &'a BridiTailSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_afterthought_bridi_tail(&tail.first, out);
    if let Some(connection) = tail.ke_continuation.as_deref() {
        collect_bridi_negation_terms_for_grouped_bridi_tail_connection(connection, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_afterthought_bridi_tail<'a>(
    tail: &'a AfterthoughtBridiTailSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_bo_grouped_bridi_tail(&tail.first, out);
    for connection in &tail.continuations {
        collect_bridi_negation_terms_for_bridi_tail_connection(connection, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_bridi_tail_connection<'a>(
    connection: &'a BridiTailConnectionSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_bo_grouped_bridi_tail(&connection.bridi_tail, out);
    collect_bridi_negation_terms_for_terms(&connection.tail_terms, out);
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_grouped_bridi_tail_connection<'a>(
    connection: &'a GroupedBridiTailConnectionSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_bridi_tail(&connection.bridi_tail, out);
    collect_bridi_negation_terms_for_terms(&connection.tail_terms, out);
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_bo_grouped_bridi_tail<'a>(
    tail: &'a BoGroupedBridiTailSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_simple_bridi_tail(&tail.first, out);
    if let Some(connection) = tail.bo_continuation.as_deref() {
        collect_bridi_negation_terms_for_bound_bridi_tail_connection(connection, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_bound_bridi_tail_connection<'a>(
    connection: &'a BoundBridiTailConnectionSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    collect_bridi_negation_terms_for_bo_grouped_bridi_tail(&connection.bridi_tail, out);
    collect_bridi_negation_terms_for_terms(&connection.tail_terms, out);
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_simple_bridi_tail<'a>(
    tail: &'a SimpleBridiTailSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    match tail.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { terms, .. }) => {
            collect_bridi_negation_terms_for_terms(terms, out);
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
            connection
        )) => {
            collect_bridi_negation_terms_for_forethought_bridi_connection(connection, out);
        }
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => {
            collect_bridi_negation_terms_for_terms(terms, out);
            collect_bridi_negation_terms_for_bridi_tail(bridi_tail, out);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_forethought_bridi_connection<'a>(
    connection: &'a ForethoughtBridiConnectionSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    match connection.as_data() {
        data!(ForethoughtBridiConnectionSyntax::BridiConnection {
            first,
            second,
            tail_terms,
            ..
        }) => {
            collect_bridi_negation_terms_for_subbridi(first, out);
            collect_bridi_negation_terms_for_subbridi(second, out);
            collect_bridi_negation_terms_for_terms(tail_terms, out);
        }
        data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection { inner, .. })
        | data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => {
            collect_bridi_negation_terms_for_forethought_bridi_connection(inner, out);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_subbridi<'a>(
    subbridi: &'a SubbridiSyntax,
    out: &mut Vec<&'a TermSyntax>,
) {
    match subbridi.as_data() {
        data!(SubbridiSyntax::Bridi(bridi)) => collect_bridi_negation_terms_for_bridi(bridi, out),
        data!(SubbridiSyntax::Prenex { inner_subbridi, .. }) => {
            collect_bridi_negation_terms_for_subbridi(inner_subbridi, out);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_terms<'a>(
    terms: &'a [TermSyntax],
    out: &mut Vec<&'a TermSyntax>,
) {
    for term in terms {
        collect_bridi_negation_terms_for_term(term, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bridi_negation_terms_for_term<'a>(term: &'a TermSyntax, out: &mut Vec<&'a TermSyntax>) {
    match term.as_data() {
        data!(TermSyntax::BridiNegation { .. }) | data!(TermSyntax::BareNegation(_)) => {
            out.push(term);
        }
        data!(TermSyntax::Termset { termset, .. }) => {
            collect_bridi_negation_terms_for_terms(termset, out);
        }
        data!(TermSyntax::ForethoughtTermsetConnection {
            terms,
            gik_terms,
            ..
        }) => {
            collect_bridi_negation_terms_for_terms(terms, out);
            collect_bridi_negation_terms_for_terms(gik_terms, out);
        }
        data!(TermSyntax::TermsetGroup {
            leading_terms,
            trailing_terms,
            ..
        })
        | data!(TermSyntax::TermsetConnection {
            leading_terms,
            trailing_terms,
            ..
        })
        | data!(TermSyntax::TermConnection {
            leading_terms,
            trailing_terms,
            ..
        }) => {
            collect_bridi_negation_terms_for_terms(leading_terms, out);
            collect_bridi_negation_terms_for_terms(trailing_terms, out);
        }
        data!(TermSyntax::BoundTermConnection {
            leading_terms,
            trailing_term,
            ..
        }) => {
            collect_bridi_negation_terms_for_terms(leading_terms, out);
            collect_bridi_negation_terms_for_term(trailing_term, out);
        }
        data!(TermSyntax::AdHocBridiAdverbialTerm { subbridi, .. })
        | data!(TermSyntax::ReciprocalBridiAdverbialTerm { subbridi, .. }) => {
            collect_bridi_negation_terms_for_subbridi(subbridi, out);
        }
        _ => {}
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_subbridi(
    subbridi: &SubbridiSyntax,
) -> Option<&TenseModalSyntax> {
    match subbridi.as_data() {
        data!(SubbridiSyntax::Bridi(bridi)) => {
            first_contradictory_event_tense_modal_for_bridi(bridi)
        }
        data!(SubbridiSyntax::Prenex { inner_subbridi, .. }) => {
            first_contradictory_event_tense_modal_for_subbridi(inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_bridi_tail(
    tail: &BridiTailSyntax,
) -> Option<&TenseModalSyntax> {
    first_contradictory_event_tense_modal_for_afterthought_bridi_tail(&tail.first).or_else(|| {
        tail.ke_continuation
            .as_deref()
            .and_then(first_contradictory_event_tense_modal_for_grouped_bridi_tail_connection)
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_afterthought_bridi_tail(
    tail: &AfterthoughtBridiTailSyntax,
) -> Option<&TenseModalSyntax> {
    first_contradictory_event_tense_modal_for_bo_grouped_bridi_tail(&tail.first).or_else(|| {
        tail.continuations
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_bridi_tail_connection)
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_bridi_tail_connection(
    connection: &BridiTailConnectionSyntax,
) -> Option<&TenseModalSyntax> {
    connection
        .tense_modal
        .as_deref()
        .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
        .or_else(|| {
            first_contradictory_event_tense_modal_for_bo_grouped_bridi_tail(&connection.bridi_tail)
        })
        .or_else(|| {
            connection
                .tail_terms
                .iter()
                .find_map(first_contradictory_event_tense_modal_for_term)
        })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_grouped_bridi_tail_connection(
    connection: &GroupedBridiTailConnectionSyntax,
) -> Option<&TenseModalSyntax> {
    connection
        .tense_modal
        .as_deref()
        .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
        .or_else(|| first_contradictory_event_tense_modal_for_bridi_tail(&connection.bridi_tail))
        .or_else(|| {
            connection
                .tail_terms
                .iter()
                .find_map(first_contradictory_event_tense_modal_for_term)
        })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_bo_grouped_bridi_tail(
    tail: &BoGroupedBridiTailSyntax,
) -> Option<&TenseModalSyntax> {
    first_contradictory_event_tense_modal_for_simple_bridi_tail(&tail.first).or_else(|| {
        tail.bo_continuation
            .as_deref()
            .and_then(first_contradictory_event_tense_modal_for_bound_bridi_tail_connection)
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_bound_bridi_tail_connection(
    connection: &BoundBridiTailConnectionSyntax,
) -> Option<&TenseModalSyntax> {
    connection
        .tense_modal
        .as_deref()
        .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
        .or_else(|| {
            first_contradictory_event_tense_modal_for_bo_grouped_bridi_tail(&connection.bridi_tail)
        })
        .or_else(|| {
            connection
                .tail_terms
                .iter()
                .find_map(first_contradictory_event_tense_modal_for_term)
        })
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_simple_bridi_tail(
    tail: &SimpleBridiTailSyntax,
) -> Option<&TenseModalSyntax> {
    match tail.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { selbri, terms, .. }) => {
            first_contradictory_event_tense_modal_for_selbri(selbri).or_else(|| {
                terms
                    .iter()
                    .find_map(first_contradictory_event_tense_modal_for_term)
            })
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
            connection
        )) => first_contradictory_event_tense_modal_for_forethought_bridi_connection(connection),
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => terms
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term)
            .or_else(|| first_contradictory_event_tense_modal_for_bridi_tail(bridi_tail)),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_forethought_bridi_connection(
    connection: &ForethoughtBridiConnectionSyntax,
) -> Option<&TenseModalSyntax> {
    match connection.as_data() {
        data!(ForethoughtBridiConnectionSyntax::BridiConnection {
            first,
            second,
            tail_terms,
            ..
        }) => first_contradictory_event_tense_modal_for_subbridi(first)
            .or_else(|| first_contradictory_event_tense_modal_for_subbridi(second))
            .or_else(|| {
                tail_terms
                    .iter()
                    .find_map(first_contradictory_event_tense_modal_for_term)
            }),
        data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection {
            tense_modal,
            inner,
            ..
        }) => tense_modal
            .as_deref()
            .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            .or_else(|| {
                first_contradictory_event_tense_modal_for_forethought_bridi_connection(inner)
            }),
        data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => {
            first_contradictory_event_tense_modal_for_forethought_bridi_connection(inner)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_selbri(
    selbri: &SelbriSyntax,
) -> Option<&TenseModalSyntax> {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => {
            if connected_event_tense_spec_for_tense_modal(tense_modal).is_some() {
                first_contradictory_event_tense_modal_for_selbri(inner_selbri)
            } else if tense_modal_has_contradictory_event_negation(tense_modal) {
                Some(tense_modal)
            } else {
                first_contradictory_event_tense_modal_for_selbri(inner_selbri)
            }
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            selbri,
            ..
        }) => ke_tense_modal
            .as_deref()
            .filter(|tense_modal| connected_event_tense_spec_for_tense_modal(tense_modal).is_none())
            .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            .or_else(|| first_contradictory_event_tense_modal_for_selbri(selbri)),
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
            first_contradictory_event_tense_modal_for_selbri(inner_selbri)
        }
        data!(SelbriSyntax::Tanru(units)) => units
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_tanru_unit),
        data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::SelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        }) => first_contradictory_event_tense_modal_for_selbri(leading_selbri)
            .or_else(|| first_contradictory_event_tense_modal_for_selbri(trailing_selbri)),
        data!(SelbriSyntax::ForethoughtSelbriConnection {
            leading_bridi,
            trailing_bridi,
            ..
        }) => first_contradictory_event_tense_modal_for_bridi(leading_bridi)
            .or_else(|| first_contradictory_event_tense_modal_for_bridi(trailing_bridi)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Option<&TenseModalSyntax> {
    match unit.as_data() {
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            first_contradictory_event_tense_modal_for_selbri(selbri)
        }
        data!(TanruUnitSyntax::ModalConversion {
            tense_modal,
            inner_unit,
            ..
        }) => tense_modal
            .as_deref()
            .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            .or_else(|| first_contradictory_event_tense_modal_for_tanru_unit(inner_unit)),
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => first_contradictory_event_tense_modal_for_tanru_unit(inner_unit),
        data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        })
        | data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => first_contradictory_event_tense_modal_for_tanru_unit(leading_unit)
            .or_else(|| first_contradictory_event_tense_modal_for_tanru_unit(trailing_unit)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_term(term: &TermSyntax) -> Option<&TenseModalSyntax> {
    match term.as_data() {
        data!(TermSyntax::Termset { termset, .. }) => termset
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term),
        data!(TermSyntax::ForethoughtTermsetConnection {
            terms,
            gik_terms,
            ..
        }) => terms
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                gik_terms
                    .iter()
                    .find_map(first_contradictory_event_tense_modal_for_term)
            }),
        data!(TermSyntax::TermsetGroup {
            leading_terms,
            trailing_terms,
            ..
        })
        | data!(TermSyntax::TermsetConnection {
            leading_terms,
            trailing_terms,
            ..
        }) => leading_terms
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                trailing_terms
                    .iter()
                    .find_map(first_contradictory_event_tense_modal_for_term)
            }),
        data!(TermSyntax::Sumti(sumti)) | data!(TermSyntax::PlaceTaggedSumti { sumti, .. }) => {
            first_contradictory_event_tense_modal_for_sumti(sumti)
        }
        data!(TermSyntax::JaiTaggedSumti { tag, sumti, .. }) => tag
            .as_deref()
            .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            .or_else(|| first_contradictory_event_tense_modal_for_sumti(sumti)),
        data!(TermSyntax::TaggedSumti { tense_modal, sumti }) => tense_modal
            .as_deref()
            .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            .or_else(|| first_contradictory_event_tense_modal_for_sumti(sumti)),
        data!(TermSyntax::RelativeAdverbialTerm {
            tail_elements,
            selbri,
            ..
        })
        | data!(TermSyntax::BridiVariableAdverbialTerm {
            tail_elements,
            selbri,
            ..
        }) => tail_elements
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_description_tail_element)
            .or_else(|| {
                selbri
                    .as_deref()
                    .and_then(first_contradictory_event_tense_modal_for_selbri)
            }),
        data!(TermSyntax::AdHocBridiAdverbialTerm { subbridi, .. })
        | data!(TermSyntax::ReciprocalBridiAdverbialTerm { subbridi, .. }) => {
            first_contradictory_event_tense_modal_for_subbridi(subbridi)
        }
        data!(TermSyntax::TermConnection {
            leading_terms,
            trailing_terms,
            ..
        }) => leading_terms
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                trailing_terms
                    .iter()
                    .find_map(first_contradictory_event_tense_modal_for_term)
            }),
        data!(TermSyntax::BoundTermConnection {
            leading_terms,
            tense_modal,
            trailing_term,
            ..
        }) => leading_terms
            .iter()
            .find_map(first_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                tense_modal
                    .as_deref()
                    .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
            })
            .or_else(|| first_contradictory_event_tense_modal_for_term(trailing_term)),
        data!(TermSyntax::BridiNegation { .. }) | data!(TermSyntax::BareNegation(_)) => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_description_tail_element(
    element: &DescriptionTailElementSyntax,
) -> Option<&TenseModalSyntax> {
    match element.as_data() {
        data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => {
            first_contradictory_event_tense_modal_for_sumti(sumti)
        }
        data!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(_))
        | data!(DescriptionTailElementSyntax::DescriptionTailQuantifier(_)) => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(tense_modal_has_contradictory_event_negation))]
fn first_contradictory_event_tense_modal_for_sumti(
    sumti: &SumtiSyntax,
) -> Option<&TenseModalSyntax> {
    match sumti.as_data() {
        data!(SumtiSyntax::TaggedSumti { tag, inner_sumti }) => {
            let tag_tense = match tag.as_data() {
                data!(SumtiTagSyntax::TenseModal(tense_modal)) => Some(tense_modal.as_ref()),
                data!(SumtiTagSyntax::PlaceTag(_)) => None,
            };
            tag_tense
                .filter(|tense_modal| tense_modal_has_contradictory_event_negation(tense_modal))
                .or_else(|| first_contradictory_event_tense_modal_for_sumti(inner_sumti))
        }
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ReferentSumti { inner_sumti, .. }) => {
            first_contradictory_event_tense_modal_for_sumti(inner_sumti)
        }
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
            first_contradictory_event_tense_modal_for_sumti(base_sumti)
        }
        data!(SumtiSyntax::QualifiedTerm { inner_term, .. }) => {
            first_contradictory_event_tense_modal_for_term(inner_term)
        }
        data!(SumtiSyntax::SumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        })
        | data!(SumtiSyntax::BoundSumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        }) => first_contradictory_event_tense_modal_for_sumti(leading_sumti)
            .or_else(|| first_contradictory_event_tense_modal_for_sumti(trailing_sumti)),
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. }) => {
            first_contradictory_event_tense_modal_for_sumti(inner_sumti)
        }
        data!(SumtiSyntax::ForethoughtSumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        }) => first_contradictory_event_tense_modal_for_sumti(leading_sumti)
            .or_else(|| first_contradictory_event_tense_modal_for_sumti(trailing_sumti)),
        data!(SumtiSyntax::BridiDescription { subbridi, .. }) => {
            first_contradictory_event_tense_modal_for_subbridi(subbridi)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_has_event_modifiers(selbri: &SelbriSyntax) -> bool {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => {
            tense_modal_has_event_modifier(tense_modal) || selbri_has_event_modifiers(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            selbri,
            ..
        }) => {
            ke_tense_modal
                .as_deref()
                .is_some_and(tense_modal_has_event_modifier)
                || selbri_has_event_modifiers(selbri)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn temporal_path_relations_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Vec<TemporalPathRelation> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            let mut relations = Vec::new();
            let mut previous_relation_accepts_distance = false;
            for part in &parts.value {
                let data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                    token
                )) = part.as_data()
                else {
                    previous_relation_accepts_distance = false;
                    continue;
                };
                if let Some(relation) = time_relation_for_pu_token(token) {
                    relations.push(path_relation_for_tense_modal(
                        relation,
                        token_text(token),
                        None,
                        tense_modal,
                    ));
                    previous_relation_accepts_distance = true;
                    continue;
                }
                if let Some(distance) = time_distance_for_zi_token(token) {
                    if previous_relation_accepts_distance
                        && let Some(relation) = relations.last_mut()
                        && relation.distance.is_none()
                    {
                        relation.distance = Some(distance);
                        previous_relation_accepts_distance = false;
                        continue;
                    }
                    if let Some(relation) = time_relation_for_time_distance_token(token) {
                        relations.push(path_relation_for_tense_modal(
                            relation,
                            token_text(token),
                            None,
                            tense_modal,
                        ));
                    }
                    previous_relation_accepts_distance = false;
                    continue;
                }
                previous_relation_accepts_distance = false;
            }
            relations
        }
        data!(TenseModalSyntax::TimeDirection(word)) => time_relation_for_pu_token(&word.value)
            .map(|relation| {
                vec![path_relation_for_tense_modal(
                    relation,
                    token_text(&word.value),
                    None,
                    tense_modal,
                )]
            })
            .unwrap_or_default(),
        data!(TenseModalSyntax::TimeDirectionDistance { pu, distance }) => {
            time_relation_for_pu_token(pu)
                .map(|relation| {
                    vec![path_relation_for_tense_modal(
                        relation,
                        token_text(pu),
                        time_distance_for_zi_token(&distance.value),
                        tense_modal,
                    )]
                })
                .unwrap_or_default()
        }
        data!(TenseModalSyntax::TimeDirectionActuality { pu, .. }) => {
            time_relation_for_pu_token(pu)
                .map(|relation| {
                    vec![path_relation_for_tense_modal(
                        relation,
                        token_text(pu),
                        None,
                        tense_modal,
                    )]
                })
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

#[requires(!relation.is_empty())]
#[requires(!introduced_by.is_empty())]
#[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
#[ensures(!ret.relation.is_empty())]
fn path_relation_for_tense_modal(
    relation: String,
    introduced_by: String,
    distance: Option<String>,
    tense_modal: &TenseModalSyntax,
) -> TemporalPathRelation {
    TemporalPathRelation {
        relation,
        introduced_by,
        distance,
        scalar_negation: modal_scalar_negation_for_tense_modal(tense_modal),
        motion: None,
    }
}

#[requires(!relation.is_empty())]
#[requires(!introduced_by.is_empty())]
#[requires(distance.as_ref().is_none_or(|distance| !distance.is_empty()))]
#[ensures(!ret.relation.is_empty())]
fn spatial_motion_path_relation_for_tense_modal(
    relation: String,
    introduced_by: String,
    distance: Option<String>,
    tense_modal: &TenseModalSyntax,
    motion_introduced_by: String,
) -> TemporalPathRelation {
    TemporalPathRelation {
        relation,
        introduced_by,
        distance,
        scalar_negation: modal_scalar_negation_for_tense_modal(tense_modal),
        motion: Some(SpatialMotion::new(
            SpatialMotionKind::Toward,
            motion_introduced_by,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn space_path_relations_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Vec<TemporalPathRelation> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            let mut relations = Vec::new();
            let mut previous_relation_accepts_distance = false;
            let mut pending_motion = None::<String>;
            for part in &parts.value {
                let data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                    token
                )) = part.as_data()
                else {
                    previous_relation_accepts_distance = false;
                    pending_motion = None;
                    continue;
                };
                if token.is_cmavo(Cmavo::Mohi) {
                    previous_relation_accepts_distance = false;
                    pending_motion = Some(token_text(token));
                    continue;
                }
                if let Some(relation) = space_relation_for_faha_token(token) {
                    let introduced_by = token_text(token);
                    relations.push(if let Some(motion_introduced_by) = pending_motion.take() {
                        spatial_motion_path_relation_for_tense_modal(
                            relation,
                            introduced_by,
                            None,
                            tense_modal,
                            motion_introduced_by,
                        )
                    } else {
                        path_relation_for_tense_modal(relation, introduced_by, None, tense_modal)
                    });
                    previous_relation_accepts_distance = true;
                    continue;
                }
                if let Some(distance) = space_distance_for_va_token(token) {
                    if previous_relation_accepts_distance
                        && let Some(relation) = relations.last_mut()
                        && relation.distance.is_none()
                    {
                        relation.distance = Some(distance);
                        previous_relation_accepts_distance = false;
                        continue;
                    }
                    if let Some(relation) = space_relation_for_space_distance_token(token) {
                        relations.push(path_relation_for_tense_modal(
                            relation,
                            token_text(token),
                            None,
                            tense_modal,
                        ));
                    }
                    previous_relation_accepts_distance = false;
                    pending_motion = None;
                    continue;
                }
                previous_relation_accepts_distance = false;
                pending_motion = None;
            }
            relations
        }
        data!(TenseModalSyntax::SpaceDistance(word)) => {
            space_relation_for_space_distance_token(&word.value)
                .map(|relation| {
                    vec![path_relation_for_tense_modal(
                        relation,
                        token_text(&word.value),
                        None,
                        tense_modal,
                    )]
                })
                .unwrap_or_default()
        }
        data!(TenseModalSyntax::SpaceDirection(word)) => space_relation_for_faha_token(&word.value)
            .map(|relation| {
                vec![path_relation_for_tense_modal(
                    relation,
                    token_text(&word.value),
                    None,
                    tense_modal,
                )]
            })
            .unwrap_or_default(),
        data!(TenseModalSyntax::SpaceMovement {
            mohi,
            direction,
            distance,
            ..
        }) => space_relation_for_faha_token(&direction.value)
            .map(|relation| {
                vec![spatial_motion_path_relation_for_tense_modal(
                    relation,
                    token_text(&direction.value),
                    distance
                        .as_ref()
                        .and_then(|distance| space_distance_for_va_token(&distance.value)),
                    tense_modal,
                    token_text(mohi),
                )]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn time_interval_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<TimeInterval> {
    time_interval_for_tense_modal_with_anchor(tense_modal, None)
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn time_interval_for_tense_modal_with_anchor(
    tense_modal: &TenseModalSyntax,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeInterval> {
    time_interval_extent_for_tense_modal(tense_modal)
        .map(|extent| TimeInterval::new(extent, anchor))
}

#[requires(true)]
#[ensures(true)]
fn time_interval_extent_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<String> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            parts.value.iter().find_map(|part| match part.as_data() {
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                    token
                )) => time_interval_extent_for_zeha_token(token),
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::AdHocModal(..)) => None,
            })
        }
        data!(TenseModalSyntax::TimeInterval(word)) => {
            time_interval_extent_for_zeha_token(&word.value)
        }
        _ => None,
    }
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn time_span_for_tense_modal_with_anchor(
    tense_modal: &TenseModalSyntax,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeSpan> {
    let data!(TenseModalSyntax::Composite { parts }) = tense_modal.as_data() else {
        return None;
    };
    let mut tokens = Vec::new();
    for part in &parts.value {
        let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
            return None;
        };
        tokens.push(token.clone());
    }
    let connector_index = tokens
        .iter()
        .position(|token| matches!(token.cmavo(), Some(Cmavo::Bihi | Cmavo::Biho)))?;
    let connector = tokens.get(connector_index)?;
    let start_tokens = tokens[..connector_index].to_vec();
    let end_tokens = tokens[connector_index + 1..].to_vec();
    if start_tokens.is_empty() || end_tokens.is_empty() {
        return None;
    }
    let anchor = anchor.or(Some(SemanticObjectId::speech_time()));
    Some(TimeSpan::new(
        time_span_endpoint_from_tokens(start_tokens, anchor)?,
        time_span_endpoint_from_tokens(end_tokens, anchor)?,
        token_text(connector),
    ))
}

#[requires(!tokens.is_empty())]
#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|endpoint| !endpoint.relation.is_empty()))]
fn time_span_endpoint_from_tokens(
    tokens: Vec<Token>,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeSpanEndpoint> {
    let tense_modal = composite_tense_modal_from_tokens(tokens)?;
    let mut relations = temporal_path_relations_for_tense_modal(&tense_modal);
    if relations.len() != 1 {
        return None;
    }
    let relation = relations.pop()?;
    Some(TimeSpanEndpoint::new(
        relation.relation,
        anchor,
        relation.introduced_by,
        relation.distance,
        relation.scalar_negation,
    ))
}

#[requires(true)]
#[ensures(true)]
fn space_interval_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<SpaceInterval> {
    space_interval_for_tense_modal_with_anchor(tense_modal, None)
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(true)]
fn space_interval_for_tense_modal_with_anchor(
    tense_modal: &TenseModalSyntax,
    anchor: Option<SemanticObjectId>,
) -> Option<SpaceInterval> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => space_interval_for_composite_parts(
            parts.value.iter().filter_map(|part| match part.as_data() {
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                    token
                )) => Some(token),
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::AdHocModal(..)) => None,
            }),
            anchor,
        ),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn temporal_aspect_contours_for_tense_modal(tense_modal: &TenseModalSyntax) -> Vec<String> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            scoped_interval_modifiers_for_composite_parts(&parts.value).temporal_aspects
        }
        data!(TenseModalSyntax::EventContour(words)) => words
            .value
            .iter()
            .filter_map(aspect_contour_for_zaho_token)
            .collect(),
        _ => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn spatial_aspect_contours_for_tense_modal(tense_modal: &TenseModalSyntax) -> Vec<String> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            scoped_interval_modifiers_for_composite_parts(&parts.value).spatial_aspects
        }
        _ => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn actuality_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<Actuality> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            parts.value.iter().find_map(|part| match part.as_data() {
                data!(CompositeTenseModalPartSyntax::Cmavo(token)) => {
                    actuality_for_caha_token(token)
                }
                data!(CompositeTenseModalPartSyntax::AdHocModal(..)) => None,
            })
        }
        data!(TenseModalSyntax::Actuality(token)) => actuality_for_caha_token(&token.value),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn actuality_for_caha_token(token: &Token) -> Option<Actuality> {
    let kind = match token.cmavo() {
        Some(Cmavo::Caha) => ActualityKind::Actual,
        Some(Cmavo::Kahe) => ActualityKind::Capable,
        Some(Cmavo::Nuho) => ActualityKind::Potential,
        Some(Cmavo::Puhi) => ActualityKind::Demonstrated,
        _ => return None,
    };
    Some(Actuality { kind })
}

#[requires(true)]
#[ensures(true)]
fn temporal_recurrences_for_tense_modal(tense_modal: &TenseModalSyntax) -> Vec<Recurrence> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            scoped_interval_modifiers_for_composite_parts(&parts.value).temporal_recurrences
        }
        data!(TenseModalSyntax::IntervalProperty {
            number,
            roi_or_tahe,
            nai,
        }) => recurrence_for_interval_marker(
            &roi_or_tahe.value,
            number.as_ref().map(word_run_text),
            nai.as_ref().map(|nai| &nai.value),
        )
        .into_iter()
        .collect(),
        _ => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn spatial_recurrences_for_tense_modal(tense_modal: &TenseModalSyntax) -> Vec<Recurrence> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            scoped_interval_modifiers_for_composite_parts(&parts.value).spatial_recurrences
        }
        _ => Vec::new(),
    }
}

#[invariant(true)]
#[derive(Debug, Default)]
struct ScopedIntervalModifiers {
    temporal_aspects: Vec<String>,
    temporal_recurrences: Vec<Recurrence>,
    spatial_aspects: Vec<String>,
    spatial_recurrences: Vec<Recurrence>,
}

#[invariant(!text.is_empty(), "pending recurrence number text must not be empty")]
#[derive(Debug, Clone)]
struct PendingRecurrenceNumber {
    text: String,
    spatial: bool,
}

#[requires(true)]
#[ensures(true)]
fn scoped_interval_modifiers_for_composite_parts(
    parts: &[jbotci_syntax::ast::CompositeTenseModalPartSyntax],
) -> ScopedIntervalModifiers {
    let mut modifiers = ScopedIntervalModifiers::default();
    let mut pending_number = None::<PendingRecurrenceNumber>;
    let mut pending_recurrence_index = None::<(bool, usize)>;
    let mut pending_recurrence_connection = None::<RecurrenceConnection>;
    let mut next_interval_property_is_spatial = false;
    for part in parts {
        let data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
            token
        )) = part.as_data()
        else {
            pending_number = None;
            pending_recurrence_index = None;
            pending_recurrence_connection = None;
            next_interval_property_is_spatial = false;
            continue;
        };
        if token.is_cmavo(Cmavo::Fehe) {
            pending_number = None;
            pending_recurrence_index = None;
            pending_recurrence_connection = None;
            next_interval_property_is_spatial = true;
            continue;
        }
        if token.is_cmavo(Cmavo::Pihu) {
            pending_number = None;
            pending_recurrence_index = None;
            pending_recurrence_connection = Some(RecurrenceConnection::new(
                RecurrenceConnectionKind::Product,
                token_text(token),
            ));
            continue;
        }
        if token.is_cmavo(Cmavo::Nai) {
            if let Some((spatial, index)) = pending_recurrence_index.take() {
                let target = if spatial {
                    modifiers.spatial_recurrences.get_mut(index)
                } else {
                    modifiers.temporal_recurrences.get_mut(index)
                };
                if let Some(recurrence) = target {
                    *recurrence = recurrence.clone().with_data(data! {
                        negation: Some(ModalNegation::new(
                            ModalNegationKind::Contradictory,
                            token_text(token),
                        )),
                    });
                    continue;
                }
            }
            pending_number = None;
            pending_recurrence_connection = None;
            next_interval_property_is_spatial = false;
            continue;
        }
        if token.is_selmaho(Selmaho::Pa) {
            let text = token_text(token);
            if let Some(pending) = pending_number.take() {
                let mut joined = pending.text.clone();
                joined.push(' ');
                joined.push_str(&text);
                pending_number = Some(new!(PendingRecurrenceNumber {
                    text: joined,
                    spatial: pending.spatial,
                }));
            } else {
                pending_number = Some(new!(PendingRecurrenceNumber {
                    text,
                    spatial: next_interval_property_is_spatial,
                }));
            }
            pending_recurrence_index = None;
            continue;
        }
        let pending = pending_number.take();
        if let Some(mut recurrence) = recurrence_for_interval_marker(
            token,
            pending.as_ref().map(|pending| pending.text.clone()),
            None,
        ) {
            if let Some(connection) = pending_recurrence_connection.take() {
                recurrence = recurrence.with_data(data! {
                    connection: Some(connection),
                });
            }
            if pending.as_ref().is_some_and(|pending| pending.spatial)
                || next_interval_property_is_spatial
            {
                modifiers.spatial_recurrences.push(recurrence);
                pending_recurrence_index = Some((true, modifiers.spatial_recurrences.len() - 1));
            } else {
                modifiers.temporal_recurrences.push(recurrence);
                pending_recurrence_index = Some((false, modifiers.temporal_recurrences.len() - 1));
            }
            next_interval_property_is_spatial = false;
            continue;
        }
        if let Some(contour) = aspect_contour_for_zaho_token(token) {
            pending_recurrence_index = None;
            pending_recurrence_connection = None;
            if next_interval_property_is_spatial {
                modifiers.spatial_aspects.push(contour);
            } else {
                modifiers.temporal_aspects.push(contour);
            }
            next_interval_property_is_spatial = false;
            continue;
        }
        pending_number = None;
        pending_recurrence_index = None;
        pending_recurrence_connection = None;
        next_interval_property_is_spatial = false;
    }
    modifiers
}

#[requires(true)]
#[ensures(true)]
fn space_interval_for_composite_parts<'a>(
    tokens: impl Iterator<Item = &'a Token>,
    anchor: Option<SemanticObjectId>,
) -> Option<SpaceInterval> {
    let mut extent = None;
    let mut directions = Vec::new();
    let mut dimensions = Vec::new();
    for token in tokens {
        if extent.is_none() {
            extent = space_interval_extent_for_veha_token(token);
        }
        if let Some(direction) = space_interval_direction_for_faha_token(token) {
            directions.push(direction);
        }
        if let Some(dimension) = space_interval_dimension_for_viha_token(token) {
            dimensions.push(dimension);
        }
    }
    (extent.is_some() || !directions.is_empty() || !dimensions.is_empty())
        .then(|| SpaceInterval::new(extent, directions, dimensions, anchor))
}

#[requires(true)]
#[ensures(true)]
fn time_interval_extent_for_zeha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zehi) => Some("short".to_owned()),
        Some(Cmavo::Zeha) => Some("medium".to_owned()),
        Some(Cmavo::Zehu) => Some("long".to_owned()),
        Some(Cmavo::Zehe) => Some("whole".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_interval_extent_for_veha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vehi) => Some("short".to_owned()),
        Some(Cmavo::Veha) => Some("medium".to_owned()),
        Some(Cmavo::Vehu) => Some("long".to_owned()),
        Some(Cmavo::Vehe) => Some("whole".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_interval_dimension_for_viha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vihi) => Some("line".to_owned()),
        Some(Cmavo::Viha) => Some("area".to_owned()),
        Some(Cmavo::Vihu) => Some("volume".to_owned()),
        Some(Cmavo::Vihe) => Some("spaceTime".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_interval_direction_for_faha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Beha) => Some("north".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn aspect_contour_for_zaho_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Puho) => Some("prospective".to_owned()),
        Some(Cmavo::Caho) => Some("continuative".to_owned()),
        Some(Cmavo::Baho) => Some("retrospective".to_owned()),
        Some(Cmavo::Coha) => Some("initiative".to_owned()),
        Some(Cmavo::Cohu) => Some("cessitive".to_owned()),
        Some(Cmavo::Mohu) => Some("completitive".to_owned()),
        Some(Cmavo::Zaho) => Some("superfective".to_owned()),
        Some(Cmavo::Cohi) => Some("achievative".to_owned()),
        Some(Cmavo::Deha) => Some("pausative".to_owned()),
        Some(Cmavo::Diha) => Some("resumptive".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recurrence_for_interval_marker(
    marker: &Token,
    value_text: Option<String>,
    negation_marker: Option<&Token>,
) -> Option<Recurrence> {
    let kind = match marker.cmavo() {
        Some(Cmavo::Roi) => RecurrenceKind::OccurrenceCount,
        Some(Cmavo::Rehu) => RecurrenceKind::OrdinalOccurrence,
        Some(Cmavo::Dihi) => RecurrenceKind::Regular,
        Some(Cmavo::Naho) => RecurrenceKind::Typically,
        Some(Cmavo::Ruhi) => RecurrenceKind::Continuously,
        Some(Cmavo::Tahe) => RecurrenceKind::Habitually,
        _ => return None,
    };
    let value = value_text.map(quantity_value_for_recurrence_text);
    Some(Recurrence::new(
        kind,
        token_text(marker),
        None,
        value,
        None,
        negation_marker
            .map(|marker| ModalNegation::new(ModalNegationKind::Contradictory, token_text(marker))),
        None,
    ))
}

#[requires(interval.is_none_or(|interval| crate::model::argument_object_kind_can_fill(interval.object_kind())))]
#[ensures(true)]
fn recurrence_with_interval(
    recurrence: Recurrence,
    interval: Option<SemanticObjectId>,
) -> Recurrence {
    let data = recurrence.into_data();
    Recurrence::new(
        data.kind,
        data.introduced_by,
        data.connection,
        data.value,
        interval,
        data.negation,
        data.source,
    )
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn quantity_value_for_recurrence_text(text: String) -> QuantityValue {
    if text == "ro" {
        return QuantityValue::text("all".to_owned());
    }
    parse_decimal_integer(&text)
        .map(QuantityValue::integer)
        .unwrap_or_else(|| QuantityValue::text(text))
}

#[requires(true)]
#[ensures(true)]
fn time_relation_for_pu_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Pu) => Some("before".to_owned()),
        Some(Cmavo::Ca) => Some("at".to_owned()),
        Some(Cmavo::Ba) => Some("after".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn time_distance_for_zi_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zi) => Some("short".to_owned()),
        Some(Cmavo::Za) => Some("medium".to_owned()),
        Some(Cmavo::Zu) => Some("long".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn time_relation_for_time_distance_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zi) => Some("near".to_owned()),
        Some(Cmavo::Za) => Some("mediumDistance".to_owned()),
        Some(Cmavo::Zu) => Some("far".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_distance_for_va_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vi) => Some("short".to_owned()),
        Some(Cmavo::Va) => Some("medium".to_owned()),
        Some(Cmavo::Vu) => Some("long".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_relation_for_faha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Buhu) => Some("coincidentWith".to_owned()),
        Some(Cmavo::Cahu) => Some("inFrontOf".to_owned()),
        Some(Cmavo::Tiha) => Some("behind".to_owned()),
        Some(Cmavo::Zuha) => Some("leftOf".to_owned()),
        Some(Cmavo::Rihu) => Some("rightOf".to_owned()),
        Some(Cmavo::Gahu) => Some("above".to_owned()),
        Some(Cmavo::Niha) => Some("below".to_owned()),
        Some(Cmavo::Nehi) => Some("within".to_owned()),
        Some(Cmavo::Ruhu) => Some("surrounding".to_owned()),
        Some(Cmavo::Paho) => Some("through".to_owned()),
        Some(Cmavo::Neha) => Some("nextTo".to_owned()),
        Some(Cmavo::Tehe) => Some("bordering".to_owned()),
        Some(Cmavo::Reho) => Some("adjacentTo".to_owned()),
        Some(Cmavo::Faha) => Some("toward".to_owned()),
        Some(Cmavo::Toho) => Some("awayFrom".to_owned()),
        Some(Cmavo::Zohi) => Some("inwardFrom".to_owned()),
        Some(Cmavo::Zeho) => Some("outwardFrom".to_owned()),
        Some(Cmavo::Zoha) => Some("tangentialTo".to_owned()),
        Some(Cmavo::Beha) => Some("northOf".to_owned()),
        Some(Cmavo::Nehu) => Some("southOf".to_owned()),
        Some(Cmavo::Duha) => Some("eastOf".to_owned()),
        Some(Cmavo::Vuha) => Some("westOf".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn space_relation_for_space_distance_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vi) => Some("near".to_owned()),
        Some(Cmavo::Va) => Some("mediumDistance".to_owned()),
        Some(Cmavo::Vu) => Some("far".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn main_selbri_for_tail(tail: &'_ BridiTailSyntax) -> Option<&'_ SelbriSyntax> {
    match tail.first.first.first.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { selbri, .. }) => Some(selbri),
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { bridi_tail, .. }) => {
            main_selbri_for_tail(bridi_tail)
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(..)) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn main_selbri_for_bridi(bridi: &'_ BridiSyntax) -> Option<&'_ SelbriSyntax> {
    main_selbri_for_tail(&bridi.bridi_tail)
}

#[requires(true)]
#[ensures(true)]
fn main_selbri_for_subbridi(subbridi: &'_ SubbridiSyntax) -> Option<&'_ SelbriSyntax> {
    match subbridi.as_data() {
        data!(SubbridiSyntax::Bridi(bridi)) => main_selbri_for_bridi(bridi),
        data!(SubbridiSyntax::Prenex { inner_subbridi, .. }) => {
            main_selbri_for_subbridi(inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn simple_bo_grouped_tail_selbri(tail: &BoGroupedBridiTailSyntax) -> Option<&SelbriSyntax> {
    match tail.first.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { selbri, .. }) => Some(selbri),
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { bridi_tail, .. }) => {
            main_selbri_for_tail(bridi_tail)
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(..)) => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn connective_text(connective: &ConnectiveSyntax) -> String {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => token_vec_text(&cmavo.value),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn full_connective_text(connective: &ConnectiveSyntax) -> String {
    let mut words = Vec::new();
    connective.visit_words(&mut |token| words.push(token_text(token)));
    words.join(" ")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn connective_label(connective: &ConnectiveSyntax) -> String {
    full_connective_text(connective).replace(' ', "-")
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModalStatementConnectionSpec {
    introduced_by: String,
    relation: String,
    visible_place: usize,
    argument_kind: ModalConnectionArgumentKind,
}

#[invariant(!introduced_by.is_empty())]
#[invariant(!relation.is_empty())]
#[invariant(*visible_place > 0)]
#[invariant(!tokens.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectedModalTerm {
    introduced_by: String,
    relation: String,
    visible_place: usize,
    tokens: Vec<Token>,
    negation: Option<ModalNegation>,
    scalar_negation: Option<ScalarNegation>,
}

#[invariant(terms.len() >= 2)]
#[invariant(!source.is_empty())]
#[invariant(!truth_table.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalModalConnectionSpec {
    operator: FormulaOperator,
    source: String,
    truth_table: String,
    terms: Vec<ConnectedModalTerm>,
}

#[invariant(terms.len() >= 2)]
#[invariant(!source.is_empty())]
#[invariant(!truth_table.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalModalConnectionAssignment {
    argument: ArgumentValue,
    operator: FormulaOperator,
    source: String,
    truth_table: String,
    terms: Vec<ConnectedModalTerm>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalConnectionArgumentKind {
    Eventuality,
    Formula,
}

#[invariant(branches.len() >= 2)]
#[invariant(!source.is_empty())]
#[invariant(truth_table.is_some() != connector_question.is_some())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectedEventTenseSpec {
    operator: FormulaOperator,
    source: String,
    truth_table: Option<String>,
    connector_question: Option<Token>,
    branches: Vec<ConnectedEventTenseBranch>,
}

#[invariant(tense_modal_has_event_modifier(tense_modal))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectedEventTenseBranch {
    tense_modal: TenseModalSyntax,
    negated: bool,
}

#[requires(true)]
#[ensures(ret.is_none_or(|tense_modal| tense_modal_has_event_modifier(tense_modal)))]
fn connected_event_tense_modal_for_selbri(selbri: &SelbriSyntax) -> Option<&TenseModalSyntax> {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri { tense_modal, .. })
            if tense_modal_has_event_modifier(tense_modal) =>
        {
            Some(tense_modal)
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal: Some(tense_modal),
            ..
        }) if tense_modal_has_event_modifier(tense_modal) => Some(tense_modal),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some())]
fn selbri_without_connected_event_tense(selbri: &SelbriSyntax) -> Option<&SelbriSyntax> {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) if connected_event_tense_spec_for_tense_modal(tense_modal).is_some() => {
            Some(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal: Some(tense_modal),
            selbri,
            ..
        }) if connected_event_tense_spec_for_tense_modal(tense_modal).is_some() => Some(selbri),
        _ => Some(selbri),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| spec.branches.len() >= 2))]
fn connected_event_tense_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<ConnectedEventTenseSpec> {
    let data!(TenseModalSyntax::Composite { parts }) = tense_modal.as_data() else {
        return None;
    };
    let mut all_tokens = Vec::new();
    for part in &parts.value {
        let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
            return None;
        };
        all_tokens.push(token.clone());
    }
    let mut branch_tokens = Vec::new();
    let mut current_branch = Vec::new();
    let mut operator = None;
    let mut connector_text = None;
    let mut connector_question = None;
    for token in &all_tokens {
        if token.is_selmaho(Selmaho::Ja) || token.is_cmavo(Cmavo::Jehi) {
            let negated = current_branch
                .last()
                .is_some_and(|token: &Token| token.is_cmavo(Cmavo::Na));
            if negated {
                current_branch.pop();
            }
            if current_branch.is_empty() {
                return None;
            }
            if token.is_cmavo(Cmavo::Jehi) {
                if operator.is_some() || connector_text.is_some() || connector_question.is_some() {
                    return None;
                }
                operator = Some(FormulaOperator::ConnectiveQuestion);
                connector_question = Some(token.clone());
            } else {
                if connector_question.is_some() {
                    return None;
                }
                let next_operator = formula_operator_for_logical_connector_token(token)?;
                if let Some(operator) = operator
                    && operator != next_operator
                {
                    return None;
                }
                operator = Some(next_operator);
                connector_text = Some(token_text(token));
            }
            branch_tokens.push((std::mem::take(&mut current_branch), negated));
            continue;
        }
        current_branch.push(token.clone());
    }
    let operator = operator?;
    if connector_text.is_none() && connector_question.is_none() {
        return None;
    }
    if current_branch.is_empty() {
        return None;
    }
    branch_tokens.push((current_branch, false));
    let mut branches = Vec::new();
    for (tokens, negated) in branch_tokens {
        let tense_modal = composite_tense_modal_from_tokens(tokens)?;
        branches.push(ConnectedEventTenseBranch::from_data(data!(
            ConnectedEventTenseBranch {
                negated: negated || modal_negation_for_tense_modal(&tense_modal).is_some(),
                tense_modal,
            }
        )));
    }
    Some(ConnectedEventTenseSpec::from_data(data!(
        ConnectedEventTenseSpec {
            operator,
            source: token_vec_text(&all_tokens),
            truth_table: connector_text,
            connector_question,
            branches,
        }
    )))
}

#[requires(!tokens.is_empty())]
#[ensures(ret.as_ref().is_none_or(tense_modal_has_event_modifier))]
fn composite_tense_modal_from_tokens(tokens: Vec<Token>) -> Option<TenseModalSyntax> {
    let parts = tokens
        .into_iter()
        .map(|token| new!(CompositeTenseModalPartSyntax::Cmavo(token)))
        .collect::<Vec<_>>();
    let tense_modal = new!(TenseModalSyntax::Composite {
        parts: WithFreeModifiers::new(parts, Vec::new()),
    });
    tense_modal_has_event_modifier(&tense_modal).then_some(tense_modal)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| spec.terms.len() >= 2))]
fn logical_modal_connection_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<LogicalModalConnectionSpec> {
    let data!(TenseModalSyntax::Composite { parts }) = tense_modal.as_data() else {
        return None;
    };
    let mut all_tokens = Vec::new();
    for part in &parts.value {
        let data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
            token
        )) = part.as_data()
        else {
            return None;
        };
        all_tokens.push(token.clone());
    }
    let mut term_tokens = Vec::new();
    let mut current_term = Vec::new();
    let mut connector = None;
    for token in &all_tokens {
        if token.is_selmaho(Selmaho::Ja) {
            if current_term.is_empty() || connector.is_some() {
                return None;
            }
            connector = Some(token.clone());
            term_tokens.push(std::mem::take(&mut current_term));
            continue;
        }
        current_term.push(token.clone());
    }
    if connector.is_none() || current_term.is_empty() {
        return None;
    }
    term_tokens.push(current_term);
    if term_tokens.len() != 2 {
        return None;
    }
    let connector = connector?;
    let operator = formula_operator_for_logical_connector_token(&connector)?;
    let mut terms = Vec::new();
    for tokens in term_tokens {
        terms.push(connected_modal_term_from_tokens(tokens)?);
    }
    Some(LogicalModalConnectionSpec::from_data(data!(
        LogicalModalConnectionSpec {
            operator,
            source: token_vec_text(&all_tokens),
            truth_table: token_text(&connector),
            terms,
        }
    )))
}

#[requires(!tokens.is_empty())]
#[ensures(ret.as_ref().is_none_or(|term| !term.relation.is_empty() && term.visible_place > 0))]
fn connected_modal_term_from_tokens(tokens: Vec<Token>) -> Option<ConnectedModalTerm> {
    let mut index = 0usize;
    let scalar_negation = match tokens.get(index) {
        Some(token) if token.is_selmaho(Selmaho::Nahe) => {
            index += 1;
            Some(ScalarNegation::new(
                scalar_negation_kind_for_cmavo(token.cmavo()),
                token_text(token),
            ))
        }
        _ => None,
    };
    let conversion = match tokens.get(index) {
        Some(token) if token.is_selmaho(Selmaho::Se) => {
            index += 1;
            Some(token.clone())
        }
        _ => None,
    };
    let marker_token = tokens.get(index)?;
    if !marker_token.is_selmaho(Selmaho::Bai) {
        return None;
    }
    index += 1;
    let negation = match tokens.get(index) {
        Some(token) if token.is_cmavo(Cmavo::Nai) => {
            index += 1;
            Some(ModalNegation::new(
                ModalNegationKind::Contradictory,
                token_text(token),
            ))
        }
        _ => None,
    };
    if index != tokens.len() {
        return None;
    }
    let marker = token_text(marker_token);
    let visible_place = conversion
        .as_ref()
        .and_then(se_token_conversion_place)
        .unwrap_or(1);
    let introduced_by = conversion
        .as_ref()
        .map(|se| format!("{} {marker}", token_text(se)))
        .unwrap_or_else(|| marker.clone());
    Some(ConnectedModalTerm::from_data(data!(ConnectedModalTerm {
        relation: modal_relation_for_marker(&marker),
        introduced_by,
        visible_place,
        tokens,
        negation,
        scalar_negation,
    })))
}

#[requires(true)]
#[ensures(true)]
fn formula_operator_for_logical_connector_token(token: &Token) -> Option<FormulaOperator> {
    match token.cmavo() {
        Some(Cmavo::Ja) => Some(FormulaOperator::Or),
        Some(Cmavo::Je) => Some(FormulaOperator::And),
        Some(Cmavo::Jo) => Some(FormulaOperator::Iff),
        Some(Cmavo::Ju) => Some(FormulaOperator::WhetherOrNot),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.relation.is_empty() && spec.visible_place > 0))]
fn modal_statement_connection_spec(
    connective: &ConnectiveSyntax,
) -> Option<ModalStatementConnectionSpec> {
    let (se, nahe, na, cmavo, nai) = match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Selbri {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Forethought {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::BridiTail {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::NonLogical {
            se,
            nahe,
            na,
            cmavo,
            nai,
        })
        | data!(ConnectiveSyntax::Interval {
            se,
            nahe,
            na,
            cmavo,
            nai,
        }) => (se, nahe, na, cmavo, nai),
    };
    if nahe.is_some() || na.is_some() || nai.is_some() {
        return None;
    }
    let (inline_se, marker_token, _terminator) = match cmavo.value.as_slice() {
        [marker_token, terminator]
            if marker_token.is_selmaho(Selmaho::Bai)
                && matches!(terminator.cmavo(), Some(Cmavo::Bo | Cmavo::Gi)) =>
        {
            (None, marker_token, terminator)
        }
        [se_token, marker_token, terminator]
            if se_token.is_selmaho(Selmaho::Se)
                && marker_token.is_selmaho(Selmaho::Bai)
                && matches!(terminator.cmavo(), Some(Cmavo::Bo | Cmavo::Gi)) =>
        {
            (Some(se_token), marker_token, terminator)
        }
        [_logical_token, marker_token, terminator]
            if marker_token.is_selmaho(Selmaho::Bai)
                && matches!(terminator.cmavo(), Some(Cmavo::Bo | Cmavo::Gi)) =>
        {
            (None, marker_token, terminator)
        }
        [_logical_token, se_token, marker_token, terminator]
            if se_token.is_selmaho(Selmaho::Se)
                && marker_token.is_selmaho(Selmaho::Bai)
                && matches!(terminator.cmavo(), Some(Cmavo::Bo | Cmavo::Gi)) =>
        {
            (Some(se_token), marker_token, terminator)
        }
        [_logical_token, marker_token] if marker_token.is_selmaho(Selmaho::Bai) => {
            (None, marker_token, marker_token)
        }
        [_logical_token, se_token, marker_token]
            if se_token.is_selmaho(Selmaho::Se) && marker_token.is_selmaho(Selmaho::Bai) =>
        {
            (Some(se_token), marker_token, marker_token)
        }
        _ => {
            let (introduced_by, relation, visible_place) =
                modal_tense_relation_spec_for_connective(connective)?;
            return Some(ModalStatementConnectionSpec {
                introduced_by,
                relation,
                visible_place,
                argument_kind: ModalConnectionArgumentKind::Eventuality,
            });
        }
    };
    let marker = token_text(marker_token);
    let conversion = se.as_ref().or(inline_se);
    let visible_place = conversion.and_then(se_token_conversion_place).unwrap_or(1);
    let introduced_by = conversion
        .as_ref()
        .map(|se| format!("{} {marker}", token_text(se)))
        .unwrap_or_else(|| marker.clone());
    Some(ModalStatementConnectionSpec {
        relation: modal_relation_for_marker(&marker),
        introduced_by,
        visible_place,
        argument_kind: modal_connection_argument_kind_for_marker(&marker),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.relation.is_empty() && spec.visible_place > 0))]
fn modal_statement_connection_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<ModalStatementConnectionSpec> {
    let (introduced_by, relation, visible_place) =
        modal_relation_spec_for_tense_modal(tense_modal)?;
    let argument_kind = match tense_modal.as_data() {
        data!(TenseModalSyntax::Modal { bai, .. }) => {
            modal_connection_argument_kind_for_marker(&token_text(&bai.value))
        }
        _ => ModalConnectionArgumentKind::Eventuality,
    };
    Some(ModalStatementConnectionSpec {
        introduced_by,
        relation,
        visible_place,
        argument_kind,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.relation.is_empty() && spec.visible_place > 0))]
fn modal_connection_spec_for_connective_and_tense(
    connective: &ConnectiveSyntax,
    tense_modal: Option<&TenseModalSyntax>,
) -> Option<ModalStatementConnectionSpec> {
    tense_modal
        .and_then(modal_statement_connection_spec_for_tense_modal)
        .or_else(|| modal_statement_connection_spec(connective))
}

#[requires(true)]
#[ensures(true)]
fn modal_connection_visible_argument_is_first(
    connective: &ConnectiveSyntax,
    tense_modal: Option<&TenseModalSyntax>,
) -> bool {
    if tense_modal
        .is_some_and(|tense_modal| tense_relation_spec_for_tense_modal(tense_modal).is_some())
        || modal_tense_relation_spec_for_connective(connective).is_some()
    {
        return false;
    }
    matches!(
        connective.as_data(),
        data!(ConnectiveSyntax::Forethought { .. })
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn modal_connective_text(
    connective: &ConnectiveSyntax,
    tense_modal: Option<&TenseModalSyntax>,
) -> String {
    if let Some(tense_modal) = tense_modal
        && let Some((introduced_by, _relation, _visible_place)) =
            modal_relation_spec_for_tense_modal(tense_modal)
    {
        return format!("{} {introduced_by} bo", connective_text(connective));
    }
    full_connective_text(connective)
}

#[requires(!marker.is_empty())]
#[ensures(true)]
fn modal_connection_argument_kind_for_marker(marker: &str) -> ModalConnectionArgumentKind {
    match marker {
        "ni'i" => ModalConnectionArgumentKind::Formula,
        _ => ModalConnectionArgumentKind::Eventuality,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn modal_relation_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<(String, String, usize)> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::AdHocModal { selbri, .. }) => Some((
            "fi'o".to_owned(),
            relation_label_for_selbri(selbri),
            visible_x1_place_for_selbri(selbri),
        )),
        data!(TenseModalSyntax::Modal { se, bai, .. }) => {
            let marker = token_text(&bai.value);
            let relation = modal_relation_for_marker(&marker);
            let visible_x1_place = se
                .as_ref()
                .and_then(se_conversion_place)
                .map(usize::from)
                .unwrap_or(1);
            let introduced_by = se
                .as_ref()
                .map(|se| format!("{} {marker}", token_text(&se.value)))
                .unwrap_or(marker);
            Some((introduced_by, relation, visible_x1_place))
        }
        _ => tense_relation_spec_for_tense_modal(tense_modal),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn tense_relation_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<(String, String, usize)> {
    temporal_path_relations_for_tense_modal(tense_modal)
        .into_iter()
        .next()
        .or_else(|| {
            space_path_relations_for_tense_modal(tense_modal)
                .into_iter()
                .next()
        })
        .map(|relation| (relation.introduced_by, relation.relation, 1))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn modal_tense_relation_spec_for_connective(
    connective: &ConnectiveSyntax,
) -> Option<(String, String, usize)> {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => {
            tense_relation_spec_for_connective_tokens(&cmavo.value)
        }
    }
}

#[requires(!tokens.is_empty())]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn tense_relation_spec_for_connective_tokens(tokens: &[Token]) -> Option<(String, String, usize)> {
    let mut end = tokens.len();
    if matches!(
        tokens.last().and_then(Token::cmavo),
        Some(Cmavo::Bo | Cmavo::Gi)
    ) {
        end -= 1;
    }
    let mut start = 0usize;
    if tokens
        .get(start)
        .is_some_and(|token| connective_token_is_logical_prefix(token))
    {
        start += 1;
    }
    if start >= end {
        return None;
    }
    first_tense_relation_spec_for_tokens(&tokens[start..end])
}

#[requires(!tokens.is_empty())]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn first_tense_relation_spec_for_tokens(tokens: &[Token]) -> Option<(String, String, usize)> {
    for token in tokens {
        if let Some(relation) = time_relation_for_pu_token(token)
            .or_else(|| space_relation_for_faha_token(token))
            .or_else(|| space_relation_for_space_distance_token(token))
        {
            return Some((token_text(token), relation, 1));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn connective_token_is_logical_prefix(token: &Token) -> bool {
    matches!(
        token.cmavo(),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ga
                | Cmavo::Ge
                | Cmavo::Go
                | Cmavo::Gu
                | Cmavo::Giha
                | Cmavo::Gihe
                | Cmavo::Giho
                | Cmavo::Gihu
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
        )
    )
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (1..=5).contains(&place)))]
fn numbered_place_for_fa_token(fa: &Token) -> Option<usize> {
    match fa.cmavo() {
        Some(Cmavo::Fa) => Some(1),
        Some(Cmavo::Fe) => Some(2),
        Some(Cmavo::Fi) => Some(3),
        Some(Cmavo::Fo) => Some(4),
        Some(Cmavo::Fu) => Some(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
fn se_token_conversion_place(se: &Token) -> Option<usize> {
    match se.cmavo() {
        Some(Cmavo::Se) => Some(2),
        Some(Cmavo::Te) => Some(3),
        Some(Cmavo::Ve) => Some(4),
        Some(Cmavo::Xe) => Some(5),
        _ => None,
    }
}

#[requires(!marker.is_empty())]
#[ensures(!ret.is_empty())]
fn modal_relation_for_marker(marker: &str) -> String {
    match marker {
        "bai" => "bapli".to_owned(),
        "bau" => "bangu".to_owned(),
        "cu'u" => "cusku".to_owned(),
        "do'e" => "unspecified-modal".to_owned(),
        "du'i" => "dunli".to_owned(),
        "fi'e" => "finti".to_owned(),
        "ga'a" => "zgana".to_owned(),
        "gau" => "gasnu".to_owned(),
        "ka'a" => "klama".to_owned(),
        "ki'u" => "krinu".to_owned(),
        "ma'i" => "manri".to_owned(),
        "mau" => "zmadu".to_owned(),
        "me'a" => "mleca".to_owned(),
        "mu'i" => "mukti".to_owned(),
        "ni'i" => "nibli".to_owned(),
        "pi'o" => "pilno".to_owned(),
        "ri'a" => "rinka".to_owned(),
        _ => marker.replace(' ', "-"),
    }
}

#[requires(!locus.is_empty())]
#[ensures(!ret.source.is_empty())]
#[ensures(ret.truth_table.is_some())]
fn connective_connector(connective: &ConnectiveSyntax, locus: &str) -> Connector {
    Connector {
        source: if connective_is_logical(connective) {
            "logical-connective".to_owned()
        } else {
            "nonlogical-connective".to_owned()
        },
        locus: locus.to_owned(),
        truth_table: Some(full_connective_text(connective)),
        parameter: None,
    }
}

#[requires(true)]
#[ensures(true)]
fn formula_operator_for_connective(connective: &ConnectiveSyntax) -> FormulaOperator {
    if connective_is_na_ja(connective) {
        return FormulaOperator::Implies;
    }
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => match () {
            _ if cmavo.value.iter().any(|token| {
                matches!(
                    token.cmavo(),
                    Some(Cmavo::A | Cmavo::Giha | Cmavo::Ja | Cmavo::Ga)
                )
            }) =>
            {
                FormulaOperator::Or
            }
            _ if cmavo.value.iter().any(|token| {
                matches!(
                    token.cmavo(),
                    Some(Cmavo::E | Cmavo::Gihe | Cmavo::Je | Cmavo::Ge)
                )
            }) =>
            {
                FormulaOperator::And
            }
            _ if cmavo.value.iter().any(|token| {
                matches!(
                    token.cmavo(),
                    Some(Cmavo::O | Cmavo::Giho | Cmavo::Jo | Cmavo::Go)
                )
            }) =>
            {
                FormulaOperator::Iff
            }
            _ if cmavo.value.iter().any(|token| {
                matches!(
                    token.cmavo(),
                    Some(Cmavo::U | Cmavo::Gihu | Cmavo::Ju | Cmavo::Gu)
                )
            }) =>
            {
                FormulaOperator::WhetherOrNot
            }
            _ => FormulaOperator::And,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_is_na_ja(connective: &ConnectiveSyntax) -> bool {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { na, cmavo, .. })
        | data!(ConnectiveSyntax::Selbri { na, cmavo, .. })
        | data!(ConnectiveSyntax::BridiTail { na, cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { na, cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { na, cmavo, .. })
        | data!(ConnectiveSyntax::Interval { na, cmavo, .. }) => {
            na.is_some()
                && cmavo
                    .value
                    .iter()
                    .any(|token| matches!(token.cmavo(), Some(Cmavo::Ja)))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_negates_left(connective: &ConnectiveSyntax) -> bool {
    if connective_is_na_ja(connective) {
        return false;
    }
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { na, .. })
        | data!(ConnectiveSyntax::Selbri { na, .. })
        | data!(ConnectiveSyntax::BridiTail { na, .. })
        | data!(ConnectiveSyntax::Forethought { na, .. })
        | data!(ConnectiveSyntax::NonLogical { na, .. })
        | data!(ConnectiveSyntax::Interval { na, .. }) => na.is_some(),
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_negates_right(connective: &ConnectiveSyntax) -> bool {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { nai, .. })
        | data!(ConnectiveSyntax::Selbri { nai, .. })
        | data!(ConnectiveSyntax::BridiTail { nai, .. })
        | data!(ConnectiveSyntax::Forethought { nai, .. })
        | data!(ConnectiveSyntax::NonLogical { nai, .. })
        | data!(ConnectiveSyntax::Interval { nai, .. }) => nai.is_some(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn nonlogical_composition_operator(connective: &ConnectiveSyntax) -> String {
    match connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => "joint".to_owned(),
        Some(Cmavo::Joi) => "mass".to_owned(),
        Some(Cmavo::Ce) => "set".to_owned(),
        Some(Cmavo::Ceho) => "sequence".to_owned(),
        Some(Cmavo::Fahu) => "respectively".to_owned(),
        Some(Cmavo::Johe) => "union".to_owned(),
        Some(Cmavo::Kuha) => "intersection".to_owned(),
        Some(Cmavo::Pihu) => "crossProduct".to_owned(),
        Some(Cmavo::Bihi) => "unorderedInterval".to_owned(),
        Some(Cmavo::Biho) => "orderedInterval".to_owned(),
        Some(Cmavo::Mihi) => "centeredInterval".to_owned(),
        _ => format!("nonlogical:{}", full_connective_text(connective)),
    }
}

#[requires(true)]
#[ensures(matches!(ret, Some(EndpointInclusion::Inclusive)) == (cmavo == Cmavo::Gaho))]
fn endpoint_inclusion_for_cmavo(cmavo: Cmavo) -> Option<EndpointInclusion> {
    match cmavo {
        Cmavo::Gaho => Some(EndpointInclusion::Inclusive),
        Cmavo::Kehi => Some(EndpointInclusion::Exclusive),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> connective_is_interval(connective))]
fn interval_endpoint_inclusion(
    connective: &ConnectiveSyntax,
    reverse_members: bool,
) -> Option<IntervalEndpointInclusion> {
    if !connective_is_interval(connective) {
        return None;
    }
    let (left, right) = match connective.as_data() {
        data!(ConnectiveSyntax::Interval { cmavo, .. }) => {
            let mut inclusions = cmavo
                .value
                .iter()
                .filter_map(Token::cmavo)
                .filter_map(endpoint_inclusion_for_cmavo);
            let left = inclusions.next()?;
            let right = inclusions.next()?;
            if inclusions.next().is_some() {
                return None;
            }
            (left, right)
        }
        _ => return None,
    };
    if reverse_members {
        Some(IntervalEndpointInclusion {
            left: right,
            right: left,
        })
    } else {
        Some(IntervalEndpointInclusion { left, right })
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relation_label_for_selbri(selbri: &SelbriSyntax) -> String {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriWord(token)) => semantic_relation_label(token_text(token)),
        data!(SelbriSyntax::Tanru(units)) => units
            .iter()
            .map(relation_label_for_tanru_unit)
            .collect::<Vec<_>>()
            .join(" "),
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. }) => {
            relation_label_for_selbri(inner_selbri)
        }
        data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
            relation_label_for_selbri(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => relation_label_for_selbri(selbri),
        data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        }) => format!(
            "{} co {}",
            relation_label_for_selbri(leading_selbri),
            relation_label_for_selbri(trailing_selbri)
        ),
        data!(SelbriSyntax::SelbriConnection {
            leading_selbri,
            connective,
            trailing_selbri,
            ..
        }) => format!(
            "{} {} {}",
            relation_label_for_selbri(leading_selbri),
            connective_label(connective),
            relation_label_for_selbri(trailing_selbri)
        ),
        data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            bo_connective: Some(connective),
            trailing_selbri,
            ..
        }) => format!(
            "{} {} {}",
            relation_label_for_selbri(leading_selbri),
            connective_label(connective),
            relation_label_for_selbri(trailing_selbri)
        ),
        data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        }) => format!(
            "{} bo {}",
            relation_label_for_selbri(leading_selbri),
            relation_label_for_selbri(trailing_selbri)
        ),
        data!(SelbriSyntax::Abstraction(abstraction)) => abstraction_relation_label(abstraction),
        data!(SelbriSyntax::ForethoughtSelbriConnection {
            guhek,
            leading_bridi,
            trailing_bridi,
            ..
        }) => format!(
            "{} {} {}",
            connective_label(guhek),
            main_selbri_for_bridi(leading_bridi)
                .map(relation_label_for_selbri)
                .unwrap_or_else(|| "bridi".to_owned()),
            main_selbri_for_bridi(trailing_bridi)
                .map(relation_label_for_selbri)
                .unwrap_or_else(|| "bridi".to_owned())
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn lujvo_rafsi_parts_for_selbri(selbri: &SelbriSyntax) -> Option<Vec<String>> {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriWord(token)) => lujvo_rafsi_parts_for_token(token),
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::Negated { inner_selbri, .. })
        | data!(SelbriSyntax::GroupedSelbri {
            selbri: inner_selbri,
            ..
        })
        | data!(SelbriSyntax::TaggedSelbri { inner_selbri, .. }) => {
            lujvo_rafsi_parts_for_selbri(inner_selbri)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn lujvo_rafsi_parts_for_token(token: &Token) -> Option<Vec<String>> {
    let word = token.core_word().bare_word()?;
    match word.as_data() {
        data!(Word::Lujvo { parts, .. }) => Some(
            parts
                .iter()
                .filter_map(|part| match part {
                    LujvoPart::Rafsi(phonemes) => Some(strip_diacritics(phonemes.as_str())),
                    LujvoPart::Hyphen(_) => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

#[requires(!word.is_empty())]
#[ensures(true)]
fn assignable_koha_cmavo_for_word(word: &str) -> Option<Cmavo> {
    let cmavo = Cmavo::from_text(word)?;
    matches!(
        cmavo,
        Cmavo::Koha
            | Cmavo::Kohe
            | Cmavo::Kohi
            | Cmavo::Koho
            | Cmavo::Kohu
            | Cmavo::Foha
            | Cmavo::Fohe
            | Cmavo::Fohi
            | Cmavo::Foho
            | Cmavo::Fohu
    )
    .then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn sumti_koha_cmavo(sumti: &SumtiSyntax) -> Option<Cmavo> {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(koha)) => koha.cmavo(),
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => sumti_koha_cmavo(inner_sumti),
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
            sumti_koha_cmavo(base_sumti)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_units_for_selbri(selbri: &SelbriSyntax) -> Option<Vec<&TanruUnitSyntax>> {
    match selbri.as_data() {
        data!(SelbriSyntax::Tanru(units)) => Some(units.iter().collect()),
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. }) => {
            tanru_units_for_selbri(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => tanru_units_for_selbri(selbri),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|description_abstraction| !description_abstraction.link_relation.is_empty()))]
fn description_abstraction_for_selbri(selbri: &SelbriSyntax) -> Option<DescriptionAbstraction<'_>> {
    match selbri.as_data() {
        data!(SelbriSyntax::Abstraction(abstraction)) => {
            Some(description_abstraction_for_nu(abstraction))
        }
        data!(SelbriSyntax::ConvertedSelbri { se, inner_selbri }) => {
            let converted_place = se_conversion_place(se).unwrap_or(2);
            let abstraction = match inner_selbri.as_data() {
                data!(SelbriSyntax::Abstraction(abstraction)) => abstraction.as_ref(),
                _ => return None,
            };
            if converted_place == 2
                && abstraction_kind_for_nu(abstraction) == AbstractionKind::Proposition
            {
                Some(DescriptionAbstraction {
                    abstraction,
                    output_sort: SemanticSort::Text,
                    link_relation: "sentenceExpresses",
                })
            } else {
                None
            }
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => description_abstraction_for_selbri(selbri),
        data!(SelbriSyntax::Tanru(units)) if units.len() == 1 => {
            description_abstraction_for_tanru_unit(&units[0])
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.link_relation.is_empty())]
fn description_abstraction_for_nu(abstraction: &AbstractionSyntax) -> DescriptionAbstraction<'_> {
    let kind = abstraction_kind_for_nu(abstraction);
    DescriptionAbstraction {
        abstraction,
        output_sort: abstraction_output_sort(kind),
        link_relation: abstraction_link_relation(kind),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|description_abstraction| !description_abstraction.link_relation.is_empty()))]
fn description_abstraction_for_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Option<DescriptionAbstraction<'_>> {
    match unit.as_data() {
        data!(TanruUnitSyntax::Abstraction(abstraction)) => {
            Some(description_abstraction_for_nu(abstraction))
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            description_abstraction_for_selbri(selbri)
        }
        data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::RelativeClauses { base, .. }) => {
            description_abstraction_for_tanru_unit(base)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn connectorless_bound_selbri_pair(selbri: &SelbriSyntax) -> Option<BoundSelbriTanruPair<'_>> {
    match selbri.as_data() {
        data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            bo_connective,
            bo_tense_modal,
            trailing_selbri,
            ..
        }) if bo_connective.is_none() && bo_tense_modal.is_none() => Some(BoundSelbriTanruPair {
            leading: leading_selbri,
            trailing: trailing_selbri,
        }),
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => connectorless_bound_selbri_pair(selbri),
        _ => None,
    }
}

#[requires(!units.is_empty())]
#[ensures(!ret.is_empty())]
fn tanru_relation_name(units: &[&TanruUnitSyntax]) -> String {
    let labels = units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            tanru_sequence_unit_label(unit, index + 1 == units.len() && units.len() > 1)
        })
        .collect::<Vec<_>>()
        .join("-");
    format!("R[tanru:{labels}]")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tanru_relation_name_for_selbri_pair(leading: &SelbriSyntax, trailing: &SelbriSyntax) -> String {
    let trailing_label = tanru_label_for_selbri(trailing);
    let trailing_label = if selbri_has_explicit_grouping(trailing) {
        format!("({trailing_label})")
    } else {
        trailing_label
    };
    format!(
        "R[tanru:{}-{}]",
        tanru_label_for_selbri(leading),
        trailing_label
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tanru_unit_relation_name(unit: &TanruUnitSyntax) -> String {
    format!("R[tanru:{}]", tanru_unit_label(unit))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tanru_sequence_unit_label(unit: &TanruUnitSyntax, is_tertau: bool) -> String {
    let label = tanru_unit_label(unit);
    if is_tertau && tanru_unit_has_explicit_grouping(unit) {
        format!("({label})")
    } else {
        label
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tanru_unit_label(unit: &TanruUnitSyntax) -> String {
    match unit.as_data() {
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => {
            let trailing_label = tanru_unit_label(trailing_unit);
            let trailing_label = if tanru_unit_has_explicit_grouping(trailing_unit) {
                format!("({trailing_label})")
            } else {
                trailing_label
            };
            format!("{}-{trailing_label}", tanru_unit_label(leading_unit))
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => {
            tanru_unit_label(inner_unit)
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            if let Some(units) = tanru_units_for_selbri(selbri)
                && !units.is_empty()
            {
                return tanru_units_label(&units);
            }
            relation_label_for_selbri(selbri)
        }
        data!(TanruUnitSyntax::RelativeClauses { base, .. })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => tanru_unit_label(base),
        _ => relation_label_for_tanru_unit(unit),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn tanru_label_for_selbri(selbri: &SelbriSyntax) -> String {
    if let Some(units) = tanru_units_for_selbri(selbri)
        && !units.is_empty()
    {
        return tanru_units_label(&units);
    }
    if let Some(bound_tanru) = connectorless_bound_selbri_pair(selbri) {
        return format!(
            "{}-{}",
            tanru_label_for_selbri(bound_tanru.leading),
            tanru_label_for_selbri(bound_tanru.trailing)
        );
    }
    if let data!(SelbriSyntax::InvertedTanru {
        leading_selbri,
        trailing_selbri,
        ..
    }) = selbri.as_data()
    {
        return format!(
            "{}-{}",
            tanru_label_for_selbri(trailing_selbri),
            tanru_label_for_selbri(leading_selbri)
        );
    }
    relation_label_for_selbri(selbri)
}

#[requires(true)]
#[ensures(true)]
fn selbri_has_explicit_grouping(selbri: &SelbriSyntax) -> bool {
    if let Some(units) = tanru_units_for_selbri(selbri)
        && !units.is_empty()
    {
        return tanru_sequence_has_explicit_grouping(&units);
    }
    connectorless_bound_selbri_pair(selbri).is_some()
        || matches!(selbri.as_data(), data!(SelbriSyntax::InvertedTanru { .. }))
}

#[requires(!units.is_empty())]
#[ensures(!ret.is_empty())]
fn tanru_units_label(units: &[&TanruUnitSyntax]) -> String {
    units
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            tanru_sequence_unit_label(unit, index + 1 == units.len() && units.len() > 1)
        })
        .collect::<Vec<_>>()
        .join("-")
}

#[requires(!units.is_empty())]
#[ensures(true)]
fn tanru_units_require_lowering(units: &[&TanruUnitSyntax]) -> bool {
    units.len() > 1
        || tanru_sequence_has_explicit_grouping(units)
        || units.iter().any(|unit| tanru_unit_requires_lowering(unit))
}

#[requires(!units.is_empty())]
#[ensures(true)]
fn tanru_sequence_has_explicit_grouping(units: &[&TanruUnitSyntax]) -> bool {
    units
        .iter()
        .any(|unit| tanru_unit_has_explicit_grouping(unit))
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_has_explicit_grouping(unit: &TanruUnitSyntax) -> bool {
    match unit.as_data() {
        data!(TanruUnitSyntax::BoundTanruUnitConnection { .. }) => true,
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            tanru_units_for_selbri(selbri).is_some_and(|units| units.len() > 1)
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => tanru_unit_has_explicit_grouping(inner_unit),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_requires_lowering(unit: &TanruUnitSyntax) -> bool {
    match unit.as_data() {
        data!(TanruUnitSyntax::SumtiSelbri { .. }) => true,
        data!(TanruUnitSyntax::ScalarNegatedTanruUnit { .. }) => true,
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => tanru_unit_requires_lowering(inner_unit),
        data!(TanruUnitSyntax::TanruUnitConnection { .. }) => true,
        data!(TanruUnitSyntax::BoundTanruUnitConnection { .. })
        | data!(TanruUnitSyntax::GroupedTanruUnit { .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(_)) => tanru_unit_has_explicit_grouping(unit),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_is_event_modal_conversion(unit: &TanruUnitSyntax) -> bool {
    event_modal_conversion_for_tanru_unit(unit).is_some()
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_is_jai_conversion(unit: &TanruUnitSyntax) -> bool {
    bare_jai_conversion_for_tanru_unit(unit).is_some()
        || non_event_modal_jai_conversion_for_tanru_unit(unit).is_some()
}

#[requires(true)]
#[ensures(true)]
fn bare_jai_conversion_for_selbri(selbri: &SelbriSyntax) -> Option<&TanruUnitSyntax> {
    match selbri.as_data() {
        data!(SelbriSyntax::Tanru(units)) if units.len() == 1 => {
            bare_jai_conversion_for_tanru_unit(&units[0])
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => bare_jai_conversion_for_selbri(selbri),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn bare_jai_conversion_for_tanru_unit(unit: &TanruUnitSyntax) -> Option<&TanruUnitSyntax> {
    match unit.as_data() {
        data!(TanruUnitSyntax::ModalConversion {
            tense_modal: None,
            ..
        }) => Some(unit),
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => bare_jai_conversion_for_tanru_unit(inner_unit),
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            let units = tanru_units_for_selbri(selbri)?;
            let [unit] = units.as_slice() else {
                return None;
            };
            bare_jai_conversion_for_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|(_, tense_modal)| !tense_modal_has_event_modifier(tense_modal)))]
fn non_event_modal_jai_conversion_for_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Option<(&TanruUnitSyntax, &TenseModalSyntax)> {
    match unit.as_data() {
        data!(TanruUnitSyntax::ModalConversion {
            tense_modal: Some(tense_modal),
            inner_unit,
            ..
        }) if !tense_modal_has_event_modifier(tense_modal)
            && modal_relation_spec_for_tense_modal(tense_modal).is_some() =>
        {
            Some((inner_unit, tense_modal))
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => non_event_modal_jai_conversion_for_tanru_unit(inner_unit),
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            let units = tanru_units_for_selbri(selbri)?;
            let [unit] = units.as_slice() else {
                return None;
            };
            non_event_modal_jai_conversion_for_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|(_, tense_modal)| tense_modal_has_event_modifier(tense_modal)))]
fn event_modal_conversion_for_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Option<(&TanruUnitSyntax, &TenseModalSyntax)> {
    match unit.as_data() {
        data!(TanruUnitSyntax::ModalConversion {
            tense_modal: Some(tense_modal),
            inner_unit,
            ..
        }) if tense_modal_has_event_modifier(tense_modal) => Some((inner_unit, tense_modal)),
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => event_modal_conversion_for_tanru_unit(inner_unit),
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            let units = tanru_units_for_selbri(selbri)?;
            let [unit] = units.as_slice() else {
                return None;
            };
            event_modal_conversion_for_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relation_label_for_tanru_unit(unit: &TanruUnitSyntax) -> String {
    match unit.as_data() {
        data!(TanruUnitSyntax::TanruUnitWord(token)) => {
            semantic_relation_label(token_text(&token.value))
        }
        data!(TanruUnitSyntax::ProBridi { goha, .. }) => {
            semantic_relation_label(token_text(&goha.value))
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => {
            relation_label_for_tanru_unit(inner_unit)
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => relation_label_for_selbri(selbri),
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            bo_connective: Some(connective),
            trailing_unit,
            ..
        }) => format!(
            "{} {} {}",
            relation_label_for_tanru_unit(leading_unit),
            connective_label(connective),
            relation_label_for_tanru_unit(trailing_unit)
        ),
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => format!(
            "{} bo {}",
            relation_label_for_tanru_unit(leading_unit),
            relation_label_for_tanru_unit(trailing_unit)
        ),
        data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            connective,
            trailing_unit,
            ..
        }) => format!(
            "{} {} {}",
            relation_label_for_tanru_unit(leading_unit),
            connective_label(connective),
            relation_label_for_tanru_unit(trailing_unit)
        ),
        data!(TanruUnitSyntax::RelativeClauses { base, .. })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => {
            relation_label_for_tanru_unit(base)
        }
        data!(TanruUnitSyntax::Abstraction(abstraction)) => abstraction_relation_label(abstraction),
        data!(TanruUnitSyntax::SumtiSelbri { .. }) => "referentOf".to_owned(),
        data!(TanruUnitSyntax::QuotedWordSelbri(token))
        | data!(TanruUnitSyntax::QuotedBridiSelbri(token))
        | data!(TanruUnitSyntax::QuotedTextSelbri(token)) => token_text(&token.value),
        data!(TanruUnitSyntax::TextSelbri { .. }) => "text-selbri".to_owned(),
        data!(TanruUnitSyntax::OrdinalSelbri { number, moi }) => {
            format!("{} {}", word_run_text(number), token_text(&moi.value))
        }
        data!(TanruUnitSyntax::OperatorSelbri {
            nuha,
            mekso_operator,
        }) => {
            format!(
                "{} {}",
                token_text(&nuha.value),
                mekso_operator_label(mekso_operator)
            )
        }
        data!(TanruUnitSyntax::TagSelbri { .. }) => "tag-selbri".to_owned(),
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> *ret.as_ref().unwrap() > 0)]
fn constructed_relation_place_count(relation: &str) -> Option<usize> {
    if relation == "referentOf" {
        Some(2)
    } else if matches!(
        relation,
        "eventOf"
            | "propertyOf"
            | "amountOf"
            | "truthValueOf"
            | "propositionOf"
            | "associatedWith"
            | "specificallyAssociatedWith"
            | "intrinsicallyPossessedBy"
    ) {
        Some(2)
    } else if matches!(relation, "conceptOf" | "experienceOf" | "abstractionOf") {
        Some(3)
    } else if relation == "describedAs" {
        Some(3)
    } else if relation.starts_with("nu ") {
        Some(1)
    } else if relation.ends_with(" moi") || relation.ends_with(" mei") {
        Some(3)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn cmavo_is_nonveridical_relative_marker(cmavo: Cmavo) -> bool {
    matches!(cmavo, Cmavo::Voi | Cmavo::Voihi)
}

#[requires(true)]
#[ensures(true)]
fn predication_mode_for_relative_clause_kind(kind: RelativeClauseKind) -> PredicationMode {
    match kind {
        RelativeClauseKind::Incidental => PredicationMode::Incidental,
        RelativeClauseKind::Restrictive => PredicationMode::Restrictive,
    }
}

#[requires(true)]
#[ensures(true)]
fn relative_phrase_kind_for_marker(marker: Cmavo) -> Option<RelativeClauseKind> {
    match marker {
        Cmavo::Ne | Cmavo::Nohu => Some(RelativeClauseKind::Incidental),
        Cmavo::Pe | Cmavo::Po | Cmavo::Pohe | Cmavo::Pohu => Some(RelativeClauseKind::Restrictive),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|relation| !relation.is_empty()))]
fn relative_phrase_relation_for_marker(marker: Cmavo) -> Option<&'static str> {
    match marker {
        Cmavo::Pe | Cmavo::Ne => Some("associatedWith"),
        Cmavo::Po => Some("specificallyAssociatedWith"),
        Cmavo::Pohe => Some("intrinsicallyPossessedBy"),
        Cmavo::Pohu | Cmavo::Nohu => Some("identity"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, associated_sumti)| !matches!(associated_sumti.as_data(), data!(SumtiSyntax::TaggedSumti { .. }))))]
fn tense_modal_tagged_sumti(sumti: &SumtiSyntax) -> Option<(&TenseModalSyntax, &SumtiSyntax)> {
    match sumti.as_data() {
        data!(SumtiSyntax::TaggedSumti { tag, inner_sumti }) => match tag.as_data() {
            data!(jbotci_syntax::ast::SumtiTagSyntax::TenseModal(tense_modal)) => {
                Some((tense_modal, inner_sumti))
            }
            data!(jbotci_syntax::ast::SumtiTagSyntax::PlaceTag(..)) => None,
        },
        _ => None,
    }
}

#[requires(!relation.is_empty())]
#[requires(visible_place > 0)]
#[ensures(ret.is_none_or(|place| place > 0 && place != visible_place))]
fn modal_relative_phrase_head_place(relation: &str, visible_place: usize) -> Option<usize> {
    match (relation, visible_place) {
        ("cusku" | "finti", 1) => Some(2),
        ("zmadu" | "mleca", 2) => Some(1),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_has_open_place_structure(relation: &str) -> bool {
    relation == "identity" || relation.starts_with("nu'a ")
}

#[requires(!relation.is_empty())]
#[ensures(true)]
fn asserted_predication_mode_for_relation(relation: &str) -> PredicationMode {
    if relation == "identity" {
        PredicationMode::Definitional
    } else {
        PredicationMode::Asserted
    }
}

#[requires(!relation.is_empty())]
#[ensures(!ret.is_empty())]
fn semantic_relation_label(relation: String) -> String {
    if relation == "du" {
        "identity".to_owned()
    } else {
        relation
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn abstraction_relation_label(abstraction: &AbstractionSyntax) -> String {
    let abstractor = token_text(&abstraction.nu.value);
    match main_selbri_for_subbridi(&abstraction.subbridi) {
        Some(selbri) => format!("{abstractor} {}", relation_label_for_selbri(selbri)),
        None => abstractor,
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_kind_for_nu(abstraction: &AbstractionSyntax) -> AbstractionKind {
    abstraction_kind_for_cmavo(abstraction.nu.cmavo())
}

#[requires(true)]
#[ensures(true)]
fn abstraction_kind_for_abstractor_connection(
    connection: &AbstractorConnectionSyntax,
) -> AbstractionKind {
    abstraction_kind_for_cmavo(connection.nu.cmavo())
}

#[requires(true)]
#[ensures(true)]
fn abstraction_kind_for_cmavo(cmavo: Option<Cmavo>) -> AbstractionKind {
    match cmavo {
        Some(Cmavo::Nu) => AbstractionKind::Event,
        Some(Cmavo::Muhe) => AbstractionKind::Achievement,
        Some(Cmavo::Puhu) => AbstractionKind::Process,
        Some(Cmavo::Zuho) => AbstractionKind::Activity,
        Some(Cmavo::Zahi) => AbstractionKind::State,
        Some(Cmavo::Ka) => AbstractionKind::Property,
        Some(Cmavo::Ni) => AbstractionKind::Amount,
        Some(Cmavo::Jei) => AbstractionKind::TruthValue,
        Some(Cmavo::Duhu) => AbstractionKind::Proposition,
        Some(Cmavo::Siho) => AbstractionKind::Concept,
        Some(Cmavo::Lihi) => AbstractionKind::Experience,
        _ => AbstractionKind::Unspecified,
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_link_formula_source(
    source: Option<crate::model::SemanticSource>,
    mode: PredicationMode,
) -> Option<crate::model::SemanticSource> {
    if mode != PredicationMode::Restrictive {
        return source;
    }
    source.map(|mut source| {
        if source.construct.as_deref() == Some("abstraction-description") {
            source.construct = Some("restrictive-formula".to_owned());
        }
        source
    })
}

#[requires(true)]
#[ensures(true)]
fn abstraction_body_mode(kind: AbstractionKind) -> PredicationMode {
    if kind == AbstractionKind::Property {
        PredicationMode::Restrictive
    } else {
        PredicationMode::Inert
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_output_sort(kind: AbstractionKind) -> SemanticSort {
    match kind {
        AbstractionKind::Event
        | AbstractionKind::Achievement
        | AbstractionKind::Process
        | AbstractionKind::Activity
        | AbstractionKind::State => SemanticSort::Eventuality,
        AbstractionKind::Property => SemanticSort::Relation,
        AbstractionKind::Amount => SemanticSort::Amount,
        AbstractionKind::TruthValue => SemanticSort::TruthValue,
        AbstractionKind::Proposition => SemanticSort::Proposition,
        AbstractionKind::Concept => SemanticSort::Concept,
        AbstractionKind::Experience => SemanticSort::Eventuality,
        AbstractionKind::SentenceSign => SemanticSort::Sign,
        AbstractionKind::Unspecified => SemanticSort::Entity,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn abstraction_link_relation(kind: AbstractionKind) -> &'static str {
    match kind {
        AbstractionKind::Event => "eventOf",
        AbstractionKind::Achievement => "achievementOf",
        AbstractionKind::Process => "processOf",
        AbstractionKind::Activity => "activityOf",
        AbstractionKind::State => "stateOf",
        AbstractionKind::Property => "propertyOf",
        AbstractionKind::Amount => "amountOf",
        AbstractionKind::TruthValue => "truthValueOf",
        AbstractionKind::Proposition => "propositionOf",
        AbstractionKind::Concept => "conceptOf",
        AbstractionKind::Experience => "experienceOf",
        AbstractionKind::SentenceSign | AbstractionKind::Unspecified => "abstractionOf",
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
fn abstraction_extra_surface_place(kind: AbstractionKind) -> Option<u8> {
    match kind {
        AbstractionKind::Process
        | AbstractionKind::Activity
        | AbstractionKind::Concept
        | AbstractionKind::Experience
        | AbstractionKind::Unspecified => Some(2),
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mekso_operator_label(operator: &MeksoOperatorSyntax) -> String {
    match operator.as_data() {
        data!(MeksoOperatorSyntax::Primitive(token)) => token_text(&token.value),
        data!(MeksoOperatorSyntax::Converted { inner_operator, .. })
        | data!(MeksoOperatorSyntax::ScalarNegated { inner_operator, .. })
        | data!(MeksoOperatorSyntax::GroupedOperator { inner_operator, .. }) => {
            mekso_operator_label(inner_operator)
        }
        data!(MeksoOperatorSyntax::SelbriAsOperator { selbri, .. }) => {
            relation_label_for_selbri(selbri)
        }
        data!(MeksoOperatorSyntax::BoundOperatorConnection {
            left_operator,
            right_operator,
            ..
        })
        | data!(MeksoOperatorSyntax::OperatorConnection {
            left_operator,
            right_operator,
            ..
        }) => format!(
            "{} {}",
            mekso_operator_label(left_operator),
            mekso_operator_label(right_operator)
        ),
        data!(MeksoOperatorSyntax::OperandAsOperator { .. }) => "operand-operator".to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mekso_surface_text(expression: &MeksoSyntax) -> String {
    match expression.as_data() {
        data!(MeksoSyntax::NumberMekso(quantifier)) => {
            quantifier_text(quantifier).unwrap_or_else(|| "mekso".to_owned())
        }
        data!(MeksoSyntax::ParenthesizedMekso {
            inner_expression,
            ..
        })
        | data!(MeksoSyntax::QualifiedOperand {
            inner_expression,
            ..
        }) => mekso_surface_text(inner_expression),
        data!(MeksoSyntax::LerfuStringMekso { letter, .. }) => math_letteral_text(&letter.value),
        data!(MeksoSyntax::Infix {
            left_expression,
            operator,
            right_expression,
        })
        | data!(MeksoSyntax::PrecedenceInfix {
            left_expression,
            operator,
            right_expression,
            ..
        }) => format!(
            "{} {} {}",
            mekso_surface_text(left_expression),
            mekso_operator_label(operator),
            mekso_surface_text(right_expression)
        ),
        data!(MeksoSyntax::ForethoughtCall {
            operator,
            operands,
            ..
        }) => {
            let mut parts = Vec::with_capacity(operands.len() + 1);
            parts.push(mekso_operator_label(operator));
            parts.extend(operands.iter().map(mekso_surface_text));
            parts.join(" ")
        }
        data!(MeksoSyntax::MeksoArray { expressions, .. }) => expressions
            .iter()
            .map(mekso_surface_text)
            .collect::<Vec<_>>()
            .join(" "),
        data!(MeksoSyntax::SumtiOperand { mohe, sumti, .. }) => {
            let mut parts = vec![token_text(&mohe.value)];
            sumti.visit_words(&mut |token| parts.push(token_text(token)));
            parts.join(" ")
        }
        _ => "mekso".to_owned(),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|name| !name.is_empty()))]
fn math_variable_name(expression: &MeksoSyntax) -> Option<String> {
    match expression.as_data() {
        data!(MeksoSyntax::LerfuStringMekso { letter, .. }) => {
            Some(math_letteral_text(&letter.value))
        }
        data!(MeksoSyntax::ParenthesizedMekso {
            inner_expression,
            ..
        })
        | data!(MeksoSyntax::QualifiedOperand {
            inner_expression,
            ..
        }) => math_variable_name(inner_expression),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> !ret.as_ref().unwrap().is_empty())]
fn mekso_letteral_word_run(expression: &MeksoSyntax) -> Option<&WordRun> {
    match expression.as_data() {
        data!(MeksoSyntax::LerfuStringMekso { letter, .. }) => Some(&letter.value),
        data!(MeksoSyntax::ParenthesizedMekso {
            inner_expression,
            ..
        })
        | data!(MeksoSyntax::QualifiedOperand {
            inner_expression,
            ..
        }) => mekso_letteral_word_run(inner_expression),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> !ret.as_ref().unwrap().is_empty())]
fn lerfu_string_sumti_letters(sumti: &SumtiSyntax) -> Option<&WordRun> {
    match sumti.as_data() {
        data!(SumtiSyntax::LerfuStringSumti { letter, .. }) => Some(&letter.value),
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
            lerfu_string_sumti_letters(inner_sumti)
        }
        _ => None,
    }
}

#[requires(!letters.is_empty())]
#[ensures(!ret.is_empty())]
fn letteral_units_for_word_run(letters: &WordRun) -> Vec<LetteralUnit> {
    letteral_units_for_tokens(letters.as_slice())
}

#[requires(true)]
#[ensures(tokens.is_empty() || !ret.is_empty())]
fn letteral_units_for_tokens(tokens: &[Token]) -> Vec<LetteralUnit> {
    let mut units = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match token_cmavo(&tokens[index]) {
            Some(Cmavo::Tei) => {
                if let Some(relative_end) = tokens[index + 1..]
                    .iter()
                    .position(|token| token_cmavo(token) == Some(Cmavo::Foi))
                {
                    let end = index + 1 + relative_end;
                    let inner = letteral_units_for_tokens(&tokens[index + 1..end]);
                    if !inner.is_empty() {
                        let source_words = letteral_source_words_for_tokens(&tokens[index..=end]);
                        let value = letteral_unit_values_joined(&inner);
                        units.push(LetteralUnit::compound(source_words, value, inner));
                        index = end + 1;
                        continue;
                    }
                }
                units.push(letteral_unit_for_token(&tokens[index]));
                index += 1;
            }
            Some(Cmavo::Sehe) => {
                let source_words = letteral_source_words_for_tokens(&tokens[index..]);
                let value = if tokens[index + 1..].is_empty() {
                    None
                } else {
                    Some(
                        tokens[index + 1..]
                            .iter()
                            .map(token_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                };
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::CharacterCode,
                    source_words,
                    Some(token_vec_text(&tokens[index..])),
                    value,
                    None,
                    None,
                ));
                break;
            }
            Some(Cmavo::Tau | Cmavo::Zai | Cmavo::Ceha) if index + 1 < tokens.len() => {
                let marker = token_text(&tokens[index]);
                let next = &tokens[index + 1];
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::Shift,
                    letteral_source_words_for_tokens(&tokens[index..=index + 1]),
                    Some(format!("{marker} {}", token_text(next))),
                    Some(token_text(next)),
                    letteral_shift_modifier(&tokens[index]),
                    None,
                ));
                index += 2;
            }
            Some(
                Cmavo::Gahe
                | Cmavo::Toha
                | Cmavo::Naha
                | Cmavo::Loha
                | Cmavo::Geho
                | Cmavo::Jeho
                | Cmavo::Joho
                | Cmavo::Ruho,
            ) => {
                units.push(LetteralUnit::simple(
                    LetteralUnitKind::Shift,
                    letteral_source_words_for_token(&tokens[index]),
                    Some(token_text(&tokens[index])),
                    None,
                    letteral_shift_modifier(&tokens[index]),
                    None,
                ));
                index += 1;
            }
            _ => {
                units.push(letteral_unit_for_token(&tokens[index]));
                index += 1;
            }
        }
    }
    units
}

#[requires(true)]
#[ensures(!ret.source_words.is_empty())]
fn letteral_unit_for_token(token: &Token) -> LetteralUnit {
    let source_words = letteral_source_words_for_token(token);
    let source_text = token_text(token);
    let bu_depth = letteral_bu_depth(token.core_word());
    let value = basic_letteral_value(&source_words);
    let kind = if parse_decimal_integer(&source_text).is_some() && bu_depth == 0 {
        LetteralUnitKind::Digit
    } else {
        LetteralUnitKind::Glyph
    };
    LetteralUnit::simple(
        kind,
        source_words,
        Some(source_text),
        value,
        None,
        (bu_depth > 0).then_some(bu_depth),
    )
}

#[requires(true)]
#[ensures(true)]
fn letteral_shift_modifier(token: &Token) -> Option<String> {
    let modifier = match token_cmavo(token)? {
        Cmavo::Gahe => "upperCase",
        Cmavo::Toha => "lowerCase",
        Cmavo::Tau => "singleCaseShift",
        Cmavo::Zai => "script",
        Cmavo::Ceha => "font",
        Cmavo::Naha => "cancel",
        Cmavo::Loha => "lojbanScript",
        Cmavo::Geho => "greekScript",
        Cmavo::Jeho => "hebrewScript",
        Cmavo::Joho => "arabicScript",
        Cmavo::Ruho => "cyrillicScript",
        _ => return None,
    };
    Some(modifier.to_owned())
}

#[requires(true)]
#[ensures(tokens.is_empty() || !ret.is_empty())]
fn letteral_source_words_for_tokens(tokens: &[Token]) -> Vec<String> {
    let mut words = Vec::new();
    for token in tokens {
        words.extend(letteral_source_words_for_token(token));
    }
    words
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn letteral_source_words_for_token(token: &Token) -> Vec<String> {
    let mut words = Vec::new();
    letteral_source_words_for_word_like(token.core_word(), &mut words);
    if words.is_empty() {
        words.push(token_text(token));
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn letteral_source_words_for_word_like(word_like: &WordLike, out: &mut Vec<String>) {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => out.push(word_text(word)),
        data!(WordLike::LerfuWord { base, bu }) => {
            letteral_source_words_for_word_like(base, out);
            out.push(word_text(bu));
        }
        _ => out.push(word_like.to_string()),
    }
}

#[requires(true)]
#[ensures(true)]
fn token_cmavo(token: &Token) -> Option<Cmavo> {
    token.core_word().bare_word().and_then(Word::cmavo)
}

#[requires(true)]
#[ensures(true)]
fn letteral_bu_depth(word_like: &WordLike) -> usize {
    match word_like.as_data() {
        data!(WordLike::LerfuWord { base, .. }) => 1 + letteral_bu_depth(base),
        _ => 0,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn basic_letteral_value(source_words: &[String]) -> Option<String> {
    match source_words {
        [word] => basic_letteral_word_value(word).or_else(|| {
            parse_decimal_integer(word)
                .filter(|value| (0..=9).contains(value))
                .map(|value| value.to_string())
        }),
        [base, bu] if bu == "bu" => match base.as_str() {
            "ky" => Some("q".to_owned()),
            "vy" => Some("w".to_owned()),
            "y'y" => Some("h".to_owned()),
            "a" | "e" | "i" | "o" | "u" | "y" => Some(base.clone()),
            _ => basic_letteral_word_value(base),
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn basic_letteral_word_value(word: &str) -> Option<String> {
    let value = match word {
        "by" => "b",
        "cy" => "c",
        "dy" => "d",
        "fy" => "f",
        "gy" => "g",
        "jy" => "j",
        "ky" => "k",
        "ly" => "l",
        "my" => "m",
        "ny" => "n",
        "py" => "p",
        "ry" => "r",
        "sy" => "s",
        "ty" => "t",
        "vy" => "v",
        "xy" => "x",
        "zy" => "z",
        "y'y" => "'",
        _ => return None,
    };
    Some(value.to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn letteral_unit_values_joined(units: &[LetteralUnit]) -> Option<String> {
    let mut value = String::new();
    for unit in units {
        value.push_str(unit.value.as_ref()?);
    }
    Some(value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn letteral_display_text(units: &[LetteralUnit]) -> Option<String> {
    if units.iter().all(|unit| {
        matches!(unit.kind, LetteralUnitKind::Glyph | LetteralUnitKind::Digit)
            && unit.value.is_some()
    }) {
        letteral_unit_values_joined(units)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn math_operator_label(operator: &MeksoOperatorSyntax) -> String {
    let source = mekso_operator_label(operator);
    match source.as_str() {
        "su'i" => "add".to_owned(),
        "pi'i" => "multiply".to_owned(),
        "te'a" => "power".to_owned(),
        "vu'u" => "subtract".to_owned(),
        "fe'i" => "divide".to_owned(),
        _ => source,
    }
}

#[requires(!letters.is_empty())]
#[ensures(!ret.is_empty())]
fn math_letteral_text(letters: &WordRun) -> String {
    letters
        .iter()
        .map(math_letteral_token_text)
        .collect::<Vec<_>>()
        .join("")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn math_letteral_token_text(token: &Token) -> String {
    match token.core_word().as_data() {
        data!(WordLike::PlainWord(word)) => math_letteral_word_text(word),
        data!(WordLike::LerfuWord { base, .. }) => math_letteral_word_like_text(base),
        _ => token_text(token),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn math_letteral_word_like_text(word_like: &WordLike) -> String {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => math_letteral_word_text(word),
        data!(WordLike::LerfuWord { base, .. }) => math_letteral_word_like_text(base),
        _ => word_like.to_string(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn math_letteral_word_text(word: &Word) -> String {
    match word.cmavo() {
        Some(Cmavo::A) => "a".to_owned(),
        Some(Cmavo::By) => "b".to_owned(),
        Some(Cmavo::Cy) => "c".to_owned(),
        Some(Cmavo::Xy) => "x".to_owned(),
        _ => word_text(word),
    }
}

#[requires(true)]
#[ensures(ret.is_some() -> !ret.as_ref().unwrap().is_empty())]
fn quantifier_text(quantifier: &QuantifierSyntax) -> Option<String> {
    match quantifier.as_data() {
        data!(QuantifierSyntax::NumberQuantifier { number, .. }) => {
            Some(word_run_text(&number.value))
        }
        data!(QuantifierSyntax::MeksoQuantifier { .. }) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn da_series_scope_source(sumti: &SumtiSyntax) -> Option<DaSeriesScopeSource<'_>> {
    match sumti.as_data() {
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::GroupedSumti {
            inner_sumti: base_sumti,
            ..
        })
        | data!(SumtiSyntax::TaggedSumti {
            inner_sumti: base_sumti,
            ..
        }) => da_series_scope_source(base_sumti),
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
            if sumti_is_da_series_pro_sumti(inner_sumti) =>
        {
            let data!(SumtiSyntax::QuantifiedSumti { quantifier, .. }) = sumti.as_data() else {
                return None;
            };
            Some(DaSeriesScopeSource::Explicit {
                quantified_sumti: sumti,
                quantifier,
            })
        }
        data!(SumtiSyntax::ProSumti(token))
            if matches!(token.cmavo(), Some(Cmavo::Da | Cmavo::De | Cmavo::Di)) =>
        {
            Some(DaSeriesScopeSource::Bare {
                da_series_sumti: sumti,
            })
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_variable_scope_source(sumti: &SumtiSyntax) -> Option<RelationVariableScopeSource<'_>> {
    match sumti.as_data() {
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::GroupedSumti {
            inner_sumti: base_sumti,
            ..
        })
        | data!(SumtiSyntax::TaggedSumti {
            inner_sumti: base_sumti,
            ..
        }) => relation_variable_scope_source(base_sumti),
        data!(SumtiSyntax::Description(description))
            if description.description.is_none()
                && description.outer_quantifier.is_none()
                && description.relative_clauses.is_empty() =>
        {
            let selbri = description.selbri.as_deref()?;
            relation_variable_word_for_selbri(selbri)?;
            let quantifier = bare_description_tail_quantifier(description)?;
            Some(RelationVariableScopeSource {
                quantified_sumti: sumti,
                selbri,
                quantifier,
            })
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_is_da_series_pro_sumti(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::ProSumti(token)) => {
            matches!(token.cmavo(), Some(Cmavo::Da | Cmavo::De | Cmavo::Di))
        }
        data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. }) => {
            sumti_is_da_series_pro_sumti(inner_sumti)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn subbridi_contains_keha(subbridi: &SubbridiSyntax) -> bool {
    subbridi_contains_current_level_keha(subbridi)
}

#[requires(true)]
#[ensures(true)]
fn subbridi_contains_current_level_keha(subbridi: &SubbridiSyntax) -> bool {
    match subbridi.as_data() {
        data!(SubbridiSyntax::Bridi(bridi)) => bridi_contains_current_level_keha(bridi),
        data!(SubbridiSyntax::Prenex {
            prenex_terms,
            inner_subbridi,
            ..
        }) => {
            prenex_terms.iter().any(term_contains_current_level_keha)
                || subbridi_contains_current_level_keha(inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn bridi_contains_current_level_keha(bridi: &BridiSyntax) -> bool {
    bridi
        .leading_terms
        .iter()
        .any(term_contains_current_level_keha)
        || bridi_tail_contains_current_level_keha(&bridi.bridi_tail)
}

#[requires(true)]
#[ensures(true)]
fn bridi_tail_contains_current_level_keha(tail: &BridiTailSyntax) -> bool {
    afterthought_bridi_tail_contains_current_level_keha(&tail.first)
        || tail.ke_continuation.as_ref().is_some_and(|continuation| {
            bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                || continuation
                    .tail_terms
                    .iter()
                    .any(term_contains_current_level_keha)
        })
}

#[requires(true)]
#[ensures(true)]
fn afterthought_bridi_tail_contains_current_level_keha(tail: &AfterthoughtBridiTailSyntax) -> bool {
    bo_grouped_bridi_tail_contains_current_level_keha(&tail.first)
        || tail.continuations.iter().any(|continuation| {
            bo_grouped_bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                || continuation
                    .tail_terms
                    .iter()
                    .any(term_contains_current_level_keha)
        })
}

#[requires(true)]
#[ensures(true)]
fn bo_grouped_bridi_tail_contains_current_level_keha(tail: &BoGroupedBridiTailSyntax) -> bool {
    simple_bridi_tail_contains_current_level_keha(&tail.first)
        || tail.bo_continuation.as_ref().is_some_and(|continuation| {
            bo_grouped_bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                || continuation
                    .tail_terms
                    .iter()
                    .any(term_contains_current_level_keha)
        })
}

#[requires(true)]
#[ensures(true)]
fn simple_bridi_tail_contains_current_level_keha(tail: &SimpleBridiTailSyntax) -> bool {
    match tail.as_data() {
        data!(SimpleBridiTailSyntax::SelbriBridiTail { selbri, terms, .. }) => {
            selbri_contains_current_level_keha(selbri)
                || terms.iter().any(term_contains_current_level_keha)
        }
        data!(SimpleBridiTailSyntax::ForethoughtBridiTailConnection(
            connection
        )) => forethought_bridi_connection_contains_current_level_keha(connection),
        data!(SimpleBridiTailSyntax::TermPrefixedBridiTail { terms, bridi_tail }) => {
            terms.iter().any(term_contains_current_level_keha)
                || bridi_tail_contains_current_level_keha(bridi_tail)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn forethought_bridi_connection_contains_current_level_keha(
    connection: &ForethoughtBridiConnectionSyntax,
) -> bool {
    match connection.as_data() {
        data!(ForethoughtBridiConnectionSyntax::BridiConnection {
            first,
            second,
            tail_terms,
            ..
        }) => {
            subbridi_contains_current_level_keha(first)
                || subbridi_contains_current_level_keha(second)
                || tail_terms.iter().any(term_contains_current_level_keha)
        }
        data!(ForethoughtBridiConnectionSyntax::GroupedBridiConnection { inner, .. })
        | data!(ForethoughtBridiConnectionSyntax::NegatedBridiConnection { inner, .. }) => {
            forethought_bridi_connection_contains_current_level_keha(inner)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn term_contains_current_level_keha(term: &TermSyntax) -> bool {
    match term.as_data() {
        data!(TermSyntax::Termset { termset, .. }) => {
            termset.iter().any(term_contains_current_level_keha)
        }
        data!(TermSyntax::ForethoughtTermsetConnection {
            terms,
            gik_terms,
            ..
        }) => {
            terms.iter().any(term_contains_current_level_keha)
                || gik_terms.iter().any(term_contains_current_level_keha)
        }
        data!(TermSyntax::TermsetGroup {
            leading_terms,
            trailing_terms,
            ..
        })
        | data!(TermSyntax::TermsetConnection {
            leading_terms,
            trailing_terms,
            ..
        }) => {
            leading_terms.iter().any(term_contains_current_level_keha)
                || trailing_terms.iter().any(term_contains_current_level_keha)
        }
        data!(TermSyntax::Sumti(sumti))
        | data!(TermSyntax::PlaceTaggedSumti { sumti, .. })
        | data!(TermSyntax::JaiTaggedSumti { sumti, .. })
        | data!(TermSyntax::TaggedSumti { sumti, .. }) => sumti_contains_current_level_keha(sumti),
        data!(TermSyntax::RelativeAdverbialTerm {
            tail_elements,
            selbri,
            ..
        })
        | data!(TermSyntax::BridiVariableAdverbialTerm {
            tail_elements,
            selbri,
            ..
        }) => {
            tail_elements
                .iter()
                .any(description_tail_element_contains_current_level_keha)
                || selbri
                    .as_ref()
                    .is_some_and(|selbri| selbri_contains_current_level_keha(selbri))
        }
        data!(TermSyntax::AdHocBridiAdverbialTerm { subbridi, .. })
        | data!(TermSyntax::ReciprocalBridiAdverbialTerm { subbridi, .. }) => {
            subbridi_contains_current_level_keha(subbridi)
        }
        data!(TermSyntax::TermConnection {
            leading_terms,
            trailing_terms,
            ..
        }) => {
            leading_terms.iter().any(term_contains_current_level_keha)
                || trailing_terms.iter().any(term_contains_current_level_keha)
        }
        data!(TermSyntax::BoundTermConnection {
            leading_terms,
            trailing_term,
            ..
        }) => {
            leading_terms.iter().any(term_contains_current_level_keha)
                || term_contains_current_level_keha(trailing_term)
        }
        data!(TermSyntax::BridiNegation { .. }) | data!(TermSyntax::BareNegation(_)) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_contains_current_level_keha(sumti: &SumtiSyntax) -> bool {
    match sumti.as_data() {
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
        | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. })
        | data!(SumtiSyntax::GroupedSumti { inner_sumti, .. }) => {
            sumti_contains_current_level_keha(inner_sumti)
        }
        data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
        | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
            sumti_contains_current_level_keha(base_sumti)
        }
        data!(SumtiSyntax::BridiDescription { subbridi, .. }) => {
            subbridi_contains_current_level_keha(subbridi)
        }
        data!(SumtiSyntax::QualifiedTerm { inner_term, .. }) => {
            term_contains_current_level_keha(inner_term)
        }
        data!(SumtiSyntax::ProSumti(koha)) => koha.value.cmavo() == Some(Cmavo::Keha),
        data!(SumtiSyntax::ReferentSumti { inner_sumti, .. }) => {
            sumti_contains_current_level_keha(inner_sumti)
        }
        data!(SumtiSyntax::SumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        })
        | data!(SumtiSyntax::BoundSumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        })
        | data!(SumtiSyntax::ForethoughtSumtiConnection {
            leading_sumti,
            trailing_sumti,
            ..
        }) => {
            sumti_contains_current_level_keha(leading_sumti)
                || sumti_contains_current_level_keha(trailing_sumti)
        }
        data!(SumtiSyntax::Description(description)) => {
            description_contains_current_level_keha(description)
        }
        data!(SumtiSyntax::DescriptionConnection(description)) => {
            description
                .tail_elements
                .iter()
                .any(description_tail_element_contains_current_level_keha)
                || description
                    .selbri
                    .as_ref()
                    .is_some_and(|selbri| selbri_contains_current_level_keha(selbri))
        }
        data!(SumtiSyntax::SelbriVocative { selbri, .. }) => {
            selbri_contains_current_level_keha(selbri)
        }
        data!(SumtiSyntax::QuotedSumti(_))
        | data!(SumtiSyntax::NumberSumti { .. })
        | data!(SumtiSyntax::LerfuStringSumti { .. })
        | data!(SumtiSyntax::NegatedSumti { .. })
        | data!(SumtiSyntax::ElidedSumti { .. })
        | data!(SumtiSyntax::NameDescription { .. })
        | data!(SumtiSyntax::NameWords(_)) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn description_contains_current_level_keha(description: &DescriptionSyntax) -> bool {
    description
        .tail_elements
        .iter()
        .any(description_tail_element_contains_current_level_keha)
        || description
            .selbri
            .as_ref()
            .is_some_and(|selbri| selbri_contains_current_level_keha(selbri))
}

#[requires(true)]
#[ensures(true)]
fn description_tail_element_contains_current_level_keha(
    element: &DescriptionTailElementSyntax,
) -> bool {
    match element.as_data() {
        data!(DescriptionTailElementSyntax::DescriptionTailSumti(sumti)) => {
            sumti_contains_current_level_keha(sumti)
        }
        data!(DescriptionTailElementSyntax::DescriptionTailRelativeClauses(_))
        | data!(DescriptionTailElementSyntax::DescriptionTailQuantifier(_)) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn selbri_contains_current_level_keha(selbri: &SelbriSyntax) -> bool {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        }) => {
            selbri_contains_current_level_keha(leading_selbri)
                || selbri_contains_current_level_keha(trailing_selbri)
        }
        data!(SelbriSyntax::Negated { inner_selbri, .. })
        | data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri { inner_selbri, .. }) => {
            selbri_contains_current_level_keha(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri { selbri, .. }) => {
            selbri_contains_current_level_keha(selbri)
        }
        data!(SelbriSyntax::ForethoughtSelbriConnection {
            leading_bridi,
            trailing_bridi,
            ..
        }) => {
            bridi_contains_current_level_keha(leading_bridi)
                || bridi_contains_current_level_keha(trailing_bridi)
        }
        data!(SelbriSyntax::Abstraction(abstraction)) => {
            subbridi_contains_current_level_keha(&abstraction.subbridi)
        }
        data!(SelbriSyntax::Tanru(units)) => {
            units.iter().any(tanru_unit_contains_current_level_keha)
        }
        data!(SelbriSyntax::SelbriWord(token)) => token.cmavo() == Some(Cmavo::Keha),
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_contains_current_level_keha(unit: &TanruUnitSyntax) -> bool {
    match unit.as_data() {
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => {
            tanru_unit_contains_current_level_keha(inner_unit)
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            selbri_contains_current_level_keha(selbri)
        }
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        })
        | data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => {
            tanru_unit_contains_current_level_keha(leading_unit)
                || tanru_unit_contains_current_level_keha(trailing_unit)
        }
        data!(TanruUnitSyntax::RelativeClauses { base, .. })
        | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => {
            tanru_unit_contains_current_level_keha(base)
        }
        data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base,
            first_sumti,
            bei_links,
            ..
        }) => {
            tanru_unit_contains_current_level_keha(base)
                || first_sumti
                    .as_ref()
                    .is_some_and(|sumti| sumti_contains_current_level_keha(sumti))
                || bei_links.iter().any(|link| {
                    link.sumti
                        .as_ref()
                        .is_some_and(|sumti| sumti_contains_current_level_keha(sumti))
                })
        }
        data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base,
            first_sumti,
            bei_links,
            ..
        }) => {
            tanru_unit_contains_current_level_keha(base)
                || first_sumti
                    .as_ref()
                    .is_some_and(|sumti| sumti_contains_current_level_keha(sumti))
                || bei_links.iter().any(|link| {
                    link.sumti
                        .as_ref()
                        .is_some_and(|sumti| sumti_contains_current_level_keha(sumti))
                })
        }
        data!(TanruUnitSyntax::Abstraction(abstraction)) => {
            subbridi_contains_current_level_keha(&abstraction.subbridi)
        }
        data!(TanruUnitSyntax::SumtiSelbri { sumti, .. }) => {
            sumti_contains_current_level_keha(sumti)
        }
        data!(TanruUnitSyntax::TanruUnitWord(word)) => word.value.cmavo() == Some(Cmavo::Keha),
        data!(TanruUnitSyntax::ProBridi { .. })
        | data!(TanruUnitSyntax::QuotedWordSelbri(_))
        | data!(TanruUnitSyntax::QuotedBridiSelbri(_))
        | data!(TanruUnitSyntax::QuotedTextSelbri(_))
        | data!(TanruUnitSyntax::TextSelbri { .. })
        | data!(TanruUnitSyntax::OrdinalSelbri { .. })
        | data!(TanruUnitSyntax::OperatorSelbri { .. })
        | data!(TanruUnitSyntax::TagSelbri { .. }) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn quantified_pro_sumti_formula_operator(quantifier: &QuantifierSyntax) -> FormulaOperator {
    match quantifier_text(quantifier).as_deref() {
        Some("ro") => FormulaOperator::Forall,
        Some("no") => FormulaOperator::None,
        _ => FormulaOperator::Cardinality,
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn quantity_form_for_text(text: &str) -> QuantityForm {
    match text {
        "ro" => QuantityForm::All,
        _ => QuantityForm::Exact,
    }
}

#[requires(true)]
#[ensures(true)]
fn gadri_name_sort(cmavo: Option<Cmavo>) -> SemanticSort {
    match cmavo {
        Some(Cmavo::Lai) => SemanticSort::Mass,
        Some(Cmavo::Lahi) => SemanticSort::Set,
        _ => SemanticSort::Entity,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn referent_qualifier_kind(cmavo: Option<Cmavo>) -> &'static str {
    match cmavo {
        Some(Cmavo::Lahe) => "referentOfSymbol",
        Some(Cmavo::Luhe) => "symbolForReferent",
        Some(Cmavo::Tuha) => "abstractionAbout",
        Some(Cmavo::Luha) => "memberOf",
        Some(Cmavo::Luhi) => "setFrom",
        Some(Cmavo::Luho) => "massFrom",
        Some(Cmavo::Vuhi) => "sequenceFrom",
        _ => "qualifiedSumti",
    }
}

#[requires(true)]
#[ensures(true)]
fn referent_qualifier_sort(cmavo: Option<Cmavo>) -> SemanticSort {
    match cmavo {
        Some(Cmavo::Luhe) => SemanticSort::Sign,
        Some(Cmavo::Tuha) => SemanticSort::Proposition,
        Some(Cmavo::Luhi) => SemanticSort::Set,
        Some(Cmavo::Luho) => SemanticSort::Mass,
        Some(Cmavo::Vuhi) => SemanticSort::Sequence,
        _ => SemanticSort::Entity,
    }
}

#[requires(true)]
#[ensures(true)]
fn scalar_negated_sumti_qualifier_kind(cmavo: Option<Cmavo>) -> &'static str {
    match cmavo {
        Some(Cmavo::Tohe) => "oppositeOf",
        Some(Cmavo::Nohe) => "neutralOf",
        Some(Cmavo::Jeha) => "affirmedAs",
        _ => "otherThan",
    }
}

#[requires(true)]
#[ensures(true)]
fn sumti_reference_kind_is_direct_reference(kind: &ReferenceKind) -> bool {
    matches!(
        kind,
        ReferenceKind::Koha
            | ReferenceKind::Ri
            | ReferenceKind::Ra
            | ReferenceKind::Ru
            | ReferenceKind::Keha
            | ReferenceKind::Letter
            | ReferenceKind::VohaSeries
            | ReferenceKind::DaSeries
    )
}

#[requires(!markers.value.is_empty())]
#[ensures(!ret.is_empty())]
fn vocative_kind_for_markers(markers: &WithFreeModifiers<Vec<Token>>) -> String {
    let Some(first) = markers.value.first() else {
        return "vocative".to_owned();
    };
    match first.cmavo() {
        Some(Cmavo::Coi) => "greeting".to_owned(),
        Some(Cmavo::Jehe) => "acknowledgement".to_owned(),
        Some(Cmavo::Coho) => "farewell".to_owned(),
        Some(Cmavo::Fihi) => "welcome".to_owned(),
        Some(Cmavo::Mihe) => "selfIdentification".to_owned(),
        Some(Cmavo::Doi) => "address".to_owned(),
        _ => token_text(first),
    }
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
fn scalar_negation_for_marker(marker: &WithFreeModifiers<Token>) -> ScalarNegation {
    scalar_negation_for_token(&marker.value)
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
fn scalar_negation_for_token(token: &Token) -> ScalarNegation {
    ScalarNegation::new(
        scalar_negation_kind_for_cmavo(token.cmavo()),
        token_text(token),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
fn modal_negation_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<ModalNegation> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts.value.iter().find_map(|part| {
            let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
                return None;
            };
            (token.cmavo() == Some(Cmavo::Nai))
                .then(|| ModalNegation::new(ModalNegationKind::Contradictory, token_text(token)))
        }),
        data!(TenseModalSyntax::Modal { nai: Some(nai), .. }) => Some(ModalNegation::new(
            ModalNegationKind::Contradictory,
            token_text(&nai.value),
        )),
        data!(TenseModalSyntax::IntervalProperty { nai: Some(nai), .. }) => Some(
            ModalNegation::new(ModalNegationKind::Contradictory, token_text(&nai.value)),
        ),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
fn modal_scalar_negation_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<ScalarNegation> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => parts.value.iter().find_map(|part| {
            let data!(CompositeTenseModalPartSyntax::Cmavo(token)) = part.as_data() else {
                return None;
            };
            matches!(
                token.cmavo(),
                Some(Cmavo::Nahe | Cmavo::Tohe | Cmavo::Nohe | Cmavo::Jeha)
            )
            .then(|| scalar_negation_for_token(token))
        }),
        data!(TenseModalSyntax::Modal {
            nahe: Some(nahe),
            ..
        }) => Some(scalar_negation_for_marker(nahe)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_makes_modal_sticky(tense_modal: &TenseModalSyntax) -> bool {
    matches!(
        tense_modal.as_data(),
        data!(TenseModalSyntax::Modal { ki: Some(_), .. })
    )
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_makes_tense_sticky(tense_modal: &TenseModalSyntax) -> bool {
    tense_modal.composite_ki().is_some()
        && !temporal_path_relations_for_tense_modal(tense_modal).is_empty()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_makes_space_sticky(tense_modal: &TenseModalSyntax) -> bool {
    tense_modal.composite_ki().is_some()
        && !space_path_relations_for_tense_modal(tense_modal).is_empty()
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_resets_sticky_tense(tense_modal: &TenseModalSyntax) -> bool {
    matches!(tense_modal.as_data(), data!(TenseModalSyntax::Sticky(_)))
}

#[requires(true)]
#[ensures(true)]
fn selbri_resets_sticky_modals(selbri: &SelbriSyntax) -> bool {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => {
            tense_modal_resets_sticky_modals(tense_modal)
                || selbri_resets_sticky_modals(inner_selbri)
        }
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            selbri,
            ..
        }) => {
            ke_tense_modal
                .as_deref()
                .is_some_and(tense_modal_resets_sticky_modals)
                || selbri_resets_sticky_modals(selbri)
        }
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. })
        | data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
            selbri_resets_sticky_modals(inner_selbri)
        }
        data!(SelbriSyntax::Tanru(units)) => units.iter().any(tanru_unit_resets_sticky_modals),
        data!(SelbriSyntax::InvertedTanru {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::SelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        }) => {
            selbri_resets_sticky_modals(leading_selbri)
                || selbri_resets_sticky_modals(trailing_selbri)
        }
        data!(SelbriSyntax::ForethoughtSelbriConnection {
            leading_bridi,
            trailing_bridi,
            ..
        }) => {
            main_selbri_for_bridi(leading_bridi).is_some_and(selbri_resets_sticky_modals)
                || main_selbri_for_bridi(trailing_bridi).is_some_and(selbri_resets_sticky_modals)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_resets_sticky_modals(unit: &TanruUnitSyntax) -> bool {
    match unit.as_data() {
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => {
            selbri_resets_sticky_modals(selbri)
        }
        data!(TanruUnitSyntax::ModalConversion {
            tense_modal,
            inner_unit,
            ..
        }) => {
            tense_modal
                .as_deref()
                .is_some_and(tense_modal_resets_sticky_modals)
                || tanru_unit_resets_sticky_modals(inner_unit)
        }
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::RelativeClauses {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit {
            base: inner_unit,
            ..
        })
        | data!(TanruUnitSyntax::AssignedProBridi {
            base: inner_unit,
            ..
        }) => tanru_unit_resets_sticky_modals(inner_unit),
        data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        })
        | data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => {
            tanru_unit_resets_sticky_modals(leading_unit)
                || tanru_unit_resets_sticky_modals(trailing_unit)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_resets_sticky_modals(tense_modal: &TenseModalSyntax) -> bool {
    matches!(tense_modal.as_data(), data!(TenseModalSyntax::Sticky(_)))
}

#[requires(true)]
#[ensures(true)]
fn scalar_negation_kind_for_cmavo(cmavo: Option<Cmavo>) -> ScalarNegationKind {
    match cmavo {
        Some(Cmavo::Tohe) => ScalarNegationKind::Opposite,
        Some(Cmavo::Nohe) => ScalarNegationKind::Neutral,
        Some(Cmavo::Jeha) => ScalarNegationKind::Affirmed,
        _ => ScalarNegationKind::OtherThan,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn token_text(token: &Token) -> String {
    token
        .core_word()
        .bare_word()
        .map(word_text)
        .unwrap_or_else(|| token.to_string())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_text(word: &Word) -> String {
    strip_diacritics(word.phonemes().as_str())
}

#[requires(!words.is_empty())]
#[ensures(!ret.is_empty())]
fn word_run_text(words: &WordRun) -> String {
    words.iter().map(token_text).collect::<Vec<_>>().join(" ")
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.is_empty())]
fn token_vec_text(tokens: &[Token]) -> String {
    tokens.iter().map(token_text).collect::<Vec<_>>().join(" ")
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn parse_decimal_integer(text: &str) -> Option<i64> {
    match text {
        "no" => Some(0),
        "pa" => Some(1),
        "re" => Some(2),
        "ci" => Some(3),
        "vo" => Some(4),
        "mu" => Some(5),
        "xa" => Some(6),
        "ze" => Some(7),
        "bi" => Some(8),
        "so" => Some(9),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_type_is_brivla_like(word_type: WordType) -> bool {
    matches!(
        word_type,
        WordType::Gismu
            | WordType::ExperimentalGismu
            | WordType::Lujvo
            | WordType::ZeiLujvo
            | WordType::ObsoleteZeiLujvo
            | WordType::Fuivla
            | WordType::ObsoleteFuivla
    )
}

#[requires(true)]
#[ensures(true)]
fn text_has_semantic_content(text: &TextSyntax) -> bool {
    !text.leading_cmevla.is_empty()
        || !text.leading_free_modifiers.is_empty()
        || text.paragraphs.iter().any(|paragraph| {
            paragraph
                .statements
                .iter()
                .any(|statement| statement.statement.is_some())
        })
}

#[requires(true)]
#[ensures(true)]
fn bridi_contains_ko(bridi: &BridiSyntax) -> bool {
    let mut contains_ko = false;
    bridi.visit_words(&mut |token| {
        if token.cmavo() == Some(Cmavo::Ko) {
            contains_ko = true;
        }
    });
    contains_ko
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_morphology::{
        MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
    };
    use jbotci_source::SourceId;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_source_and_options};
    use serde_json::Value;
    use std::collections::BTreeSet;

    #[requires(!source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|value| value.get("objects").is_some()) || ret.is_err())]
    fn semantic_json_for(source: &str) -> Result<Value, Box<dyn std::error::Error>> {
        semantic_json_for_options(source, SemanticBuildOptions::default())
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|value| value.get("objects").is_some()) || ret.is_err())]
    fn semantic_json_for_options(
        source: &str,
        options: SemanticBuildOptions<'_>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let words = segment_words_with_modifiers_with_options_and_source_id(
            source,
            &MorphologyOptions::default(),
            Some(SourceId("<test>".to_owned())),
        )?;
        let parsed =
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())?;
        let graph = build_semantic_graph_with_dictionary_and_options(
            &parsed.parse_tree,
            SemanticBuildOptions {
                source_text: Some(source),
                story_time: options.story_time,
            },
            jbotci_dictionary_data::english(),
        )?;
        Ok(serde_json::from_str(&graph.to_json_string(0)?)?)
    }

    #[requires(true)]
    #[ensures(true)]
    fn object<'a>(json: &'a Value, id: &str) -> &'a Value {
        json.pointer(&format!("/objects/{id}"))
            .unwrap_or_else(|| panic!("missing semantic object {id}"))
    }

    #[requires(true)]
    #[ensures(true)]
    fn root_object(json: &Value) -> &Value {
        object(json, json["root"].as_str().expect("root object ID"))
    }

    #[requires(!operator.is_empty())]
    #[ensures(ret["operator"] == operator)]
    fn composition_with_operator<'a>(json: &'a Value, operator: &str) -> &'a Value {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter_map(|object| object.get("composition"))
            .find(|composition| composition["operator"] == operator)
            .unwrap_or_else(|| panic!("missing composition with operator {operator}"))
    }

    #[requires(true)]
    #[ensures(true)]
    fn predication_relations(json: &Value) -> Vec<String> {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter(|object| object["type"] == "predication")
            .filter_map(|object| object["relation"].as_str())
            .map(ToOwned::to_owned)
            .collect()
    }

    #[requires(!relation.is_empty())]
    #[requires(!mode.is_empty())]
    #[ensures(true)]
    fn predication_with_relation_and_mode<'a>(
        json: &'a Value,
        relation: &str,
        mode: &str,
    ) -> &'a Value {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| {
                object["type"] == "predication"
                    && object["relation"] == relation
                    && object["mode"] == mode
            })
            .unwrap_or_else(|| panic!("missing {mode} predication for relation {relation}"))
    }

    #[requires(!relation.is_empty())]
    #[requires(!mode.is_empty())]
    #[ensures(true)]
    fn predications_with_relation_and_mode<'a>(
        json: &'a Value,
        relation: &str,
        mode: &str,
    ) -> Vec<&'a Value> {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter(|object| {
                object["type"] == "predication"
                    && object["relation"] == relation
                    && object["mode"] == mode
            })
            .collect()
    }

    #[requires(!relation_parameter.is_empty())]
    #[ensures(true)]
    fn predication_with_relation_parameter<'a>(
        json: &'a Value,
        relation_parameter: &str,
    ) -> &'a Value {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| {
                object["type"] == "predication" && object["relationParameter"] == relation_parameter
            })
            .unwrap_or_else(|| {
                panic!("missing predication with relation parameter {relation_parameter}")
            })
    }

    #[requires(!kind.is_empty())]
    #[ensures(true)]
    fn referent_with_descriptor_kind<'a>(json: &'a Value, kind: &str) -> &'a Value {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| object["type"] == "referent" && object["descriptor"]["kind"] == kind)
            .unwrap_or_else(|| panic!("missing referent descriptor kind {kind}"))
    }

    #[requires(!name.is_empty())]
    #[ensures(!ret.is_empty())]
    fn named_referent_id<'a>(json: &'a Value, name: &str) -> &'a str {
        json["objects"]
            .as_object()
            .expect("semantic objects")
            .iter()
            .find(|(_id, object)| {
                object["type"] == "referent" && object["descriptor"]["name"] == name
            })
            .map(|(id, _object)| id.as_str())
            .unwrap_or_else(|| panic!("missing named referent {name}"))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_arity_uses_definition_place_markers() {
        assert_eq!(
            dictionary_relation_place_count(jbotci_dictionary_data::english(), "klama"),
            Some(5)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stable_ids_and_elided_places_are_deterministic() {
        let json = semantic_json_for("mi klama").expect("semantic JSON");
        assert_eq!(json["root"], "utterance:u1");
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["kind"],
            "filled"
        );
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["value"],
            "referent:speaker"
        );
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x5"]["kind"],
            "elided"
        );
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x5"]["value"],
            "referent:r4"
        );
        assert_eq!(
            object(&json, "referent:r4")["descriptor"]["word"],
            "zo'e x5"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn moved_pu_terms_anchor_the_event_not_modal_arguments() {
        for text in [
            "mi cu pu klama le zarci",
            "puku mi klama le zarci",
            "mi klama puku le zarci",
            "mi klama le zarci pu",
        ] {
            let json = semantic_json_for(text).expect("semantic JSON");
            let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
            assert!(klama.get("modalArguments").is_none(), "{text}");
            let event = object(
                &json,
                klama["eventuality"]
                    .as_str()
                    .expect("klama predication eventuality"),
            );
            assert_eq!(event["time"]["relation"], "before", "{text}");
            assert_eq!(event["time"]["anchor"], "referent:speech-time", "{text}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn event_contours_and_recurrence_are_event_properties() {
        let prospective = semantic_json_for("mi pu'o damba").expect("semantic JSON");
        let damba = predication_with_relation_and_mode(&prospective, "damba", "asserted");
        assert!(damba.get("modalArguments").is_none());
        let event = object(
            &prospective,
            damba["eventuality"].as_str().expect("damba event"),
        );
        assert_eq!(event["aspect"]["contour"], "prospective");
        assert!(event.get("recurrence").is_none());

        let regular_initiative = semantic_json_for("mi ba di'i co'a bajra").expect("semantic JSON");
        let bajra = predication_with_relation_and_mode(&regular_initiative, "bajra", "asserted");
        assert!(bajra.get("modalArguments").is_none());
        let event = object(
            &regular_initiative,
            bajra["eventuality"].as_str().expect("bajra event"),
        );
        assert_eq!(event["time"]["relation"], "after");
        assert_eq!(event["aspect"]["contour"], "initiative");
        assert_eq!(event["recurrence"][0]["kind"], "regular");
        assert_eq!(event["recurrence"][0]["introducedBy"], "di'i");

        let ordinal_then_count =
            semantic_json_for("mi pare'u paroi klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&ordinal_then_count, "klama", "asserted");
        let event = object(
            &ordinal_then_count,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["recurrence"][0]["kind"], "ordinalOccurrence");
        assert_eq!(event["recurrence"][0]["introducedBy"], "re'u");
        assert_eq!(event["recurrence"][0]["value"]["integer"], 1);
        assert_eq!(event["recurrence"][1]["kind"], "occurrenceCount");
        assert_eq!(event["recurrence"][1]["introducedBy"], "roi");
        assert_eq!(event["recurrence"][1]["value"]["integer"], 1);

        let count_then_ordinal =
            semantic_json_for("mi paroi pare'u klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&count_then_ordinal, "klama", "asserted");
        let event = object(
            &count_then_ordinal,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["recurrence"][0]["kind"], "occurrenceCount");
        assert_eq!(event["recurrence"][0]["introducedBy"], "roi");
        assert_eq!(event["recurrence"][0]["value"]["integer"], 1);
        assert_eq!(event["recurrence"][1]["kind"], "ordinalOccurrence");
        assert_eq!(event["recurrence"][1]["introducedBy"], "re'u");
        assert_eq!(event["recurrence"][1]["value"]["integer"], 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordered_aspects_and_recurrence_products_are_preserved() {
        let aspect_chain = semantic_json_for("la .djordj. ca'o co'a ciska").expect("semantic JSON");
        let ciska = predication_with_relation_and_mode(&aspect_chain, "ciska", "asserted");
        let event = object(
            &aspect_chain,
            ciska["eventuality"].as_str().expect("ciska event"),
        );
        assert!(event.get("aspect").is_none());
        assert_eq!(event["aspects"][0]["contour"], "continuative");
        assert_eq!(event["aspects"][1]["contour"], "initiative");

        let product =
            semantic_json_for("mi reroi pi'u xaroi celgau le seldanti").expect("semantic JSON");
        let celgau = predication_with_relation_and_mode(&product, "celgau", "asserted");
        let event = object(&product, celgau["eventuality"].as_str().expect("event"));
        assert_eq!(event["recurrence"][0]["value"]["integer"], 2);
        assert!(event["recurrence"][0].get("connection").is_none());
        assert_eq!(event["recurrence"][1]["value"]["integer"], 6);
        assert_eq!(event["recurrence"][1]["connection"]["kind"], "product");
        assert_eq!(event["recurrence"][1]["connection"]["introducedBy"], "pi'u");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recurrence_nai_marks_interval_property_negation() {
        let intermittent =
            semantic_json_for("le verba ru'inai cadzu le bisli").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&intermittent, "cadzu", "asserted");
        let event = object(
            &intermittent,
            cadzu["eventuality"].as_str().expect("cadzu event"),
        );
        assert_eq!(event["recurrence"][0]["kind"], "continuously");
        assert_eq!(event["recurrence"][0]["introducedBy"], "ru'i");
        assert_eq!(event["recurrence"][0]["negation"]["kind"], "contradictory");
        assert_eq!(event["recurrence"][0]["negation"]["introducedBy"], "nai");

        let not_twice =
            semantic_json_for("le ratcu reroinai citka le cirla").expect("semantic JSON");
        let citka = predication_with_relation_and_mode(&not_twice, "citka", "asserted");
        let event = object(
            &not_twice,
            citka["eventuality"].as_str().expect("citka event"),
        );
        assert_eq!(event["recurrence"][0]["kind"], "occurrenceCount");
        assert_eq!(event["recurrence"][0]["value"]["integer"], 2);
        assert_eq!(event["recurrence"][0]["negation"]["introducedBy"], "nai");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn spatial_mohi_marks_directed_motion() {
        let moving_right =
            semantic_json_for("le verba mo'i ri'u cadzu le bisli").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&moving_right, "cadzu", "asserted");
        let event = object(
            &moving_right,
            cadzu["eventuality"].as_str().expect("cadzu event"),
        );
        assert_eq!(event["space"]["relation"], "rightOf");
        assert_eq!(event["space"]["motion"]["kind"], "toward");
        assert_eq!(event["space"]["motion"]["introducedBy"], "mo'i");

        let static_right =
            semantic_json_for("le verba ri'u cadzu le bisli").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&static_right, "cadzu", "asserted");
        let event = object(
            &static_right,
            cadzu["eventuality"].as_str().expect("cadzu event"),
        );
        assert_eq!(event["space"]["relation"], "rightOf");
        assert!(event["space"].get("motion").is_none());

        let static_then_motion =
            semantic_json_for("le verba zu'avu mo'i ri'uvi cadzu le bisli").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&static_then_motion, "cadzu", "asserted");
        let event = object(
            &static_then_motion,
            cadzu["eventuality"].as_str().expect("cadzu event"),
        );
        assert_eq!(event["spacePath"][0]["relation"], "leftOf");
        assert!(event["spacePath"][0].get("motion").is_none());
        assert_eq!(event["spacePath"][1]["relation"], "rightOf");
        assert_eq!(event["spacePath"][1]["motion"]["introducedBy"], "mo'i");

        let reference_frame = semantic_json_for("le verba mo'i ri'u cadzu le bisli ma'i vo'a")
            .expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&reference_frame, "cadzu", "asserted");
        assert_eq!(cadzu["modalArguments"][0]["relation"], "manri");
        assert_eq!(
            cadzu["modalArguments"][0]["arguments"]["x1"]["value"],
            "referent:r1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn contradictory_tense_negation_wraps_positive_event_relation() {
        let json = semantic_json_for("mi punai klama le zarci").expect("semantic JSON");
        let root = object(&json, json["root"].as_str().expect("root id"));
        let content = object(&json, root["content"].as_str().expect("utterance content"));
        assert_eq!(content["operator"], "not");
        assert_eq!(content["source"]["text"], "punai");

        let child = object(
            &json,
            content["children"][0]
                .as_str()
                .expect("negated child formula"),
        );
        let klama = object(
            &json,
            child["predication"]
                .as_str()
                .expect("negated atom predication"),
        );
        assert!(klama.get("modalArguments").is_none());
        let event = object(&json, klama["eventuality"].as_str().expect("klama event"));
        assert_eq!(event["time"]["relation"], "before");
        assert_eq!(event["time"]["anchor"], "referent:speech-time");
        assert!(event["time"].get("negation").is_none());
        assert!(event["time"].get("scalarNegation").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn contradictory_sumtcita_and_aspect_negation_wrap_formula_only() {
        let spatial = semantic_json_for("le nanmu cu batci le gerku ne'inai le kumfa")
            .expect("semantic JSON");
        let spatial_root = object(&spatial, spatial["root"].as_str().expect("root id"));
        let spatial_content = object(
            &spatial,
            spatial_root["content"].as_str().expect("utterance content"),
        );
        assert_eq!(spatial_content["operator"], "not");
        assert_eq!(spatial_content["source"]["text"], "ne'inai");
        let batci = predication_with_relation_and_mode(&spatial, "batci", "asserted");
        let spatial_event = object(
            &spatial,
            batci["eventuality"].as_str().expect("batci event"),
        );
        assert_eq!(spatial_event["space"]["relation"], "within");
        assert!(spatial_event["space"].get("negation").is_none());

        let aspect = semantic_json_for("mi morsi ca'onai le nu mi jmive").expect("semantic JSON");
        let aspect_root = object(&aspect, aspect["root"].as_str().expect("root id"));
        let aspect_content = object(
            &aspect,
            aspect_root["content"].as_str().expect("utterance content"),
        );
        assert_eq!(aspect_content["operator"], "not");
        assert_eq!(aspect_content["source"]["text"], "ca'onai");
        let morsi = predication_with_relation_and_mode(&aspect, "morsi", "asserted");
        let aspect_event = object(&aspect, morsi["eventuality"].as_str().expect("morsi event"));
        assert_eq!(aspect_event["aspect"]["contour"], "continuative");
        assert!(aspect_event["aspect"].get("negation").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_tense_negation_marks_event_relation_or_aspect() {
        let temporal = semantic_json_for("mi na'e pu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&temporal, "klama", "asserted");
        let temporal_event = object(
            &temporal,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(temporal_event["time"]["relation"], "before");
        assert_eq!(
            temporal_event["time"]["scalarNegation"]["kind"],
            "otherThan"
        );
        assert_eq!(
            temporal_event["time"]["scalarNegation"]["introducedBy"],
            "na'e"
        );

        let spatial = semantic_json_for("le nanmu cu batci le gerku to'e ne'i le kumfa")
            .expect("semantic JSON");
        let batci = predication_with_relation_and_mode(&spatial, "batci", "asserted");
        let spatial_event = object(
            &spatial,
            batci["eventuality"].as_str().expect("batci event"),
        );
        assert_eq!(spatial_event["space"]["relation"], "within");
        assert_eq!(spatial_event["space"]["scalarNegation"]["kind"], "opposite");
        assert_eq!(
            spatial_event["space"]["scalarNegation"]["introducedBy"],
            "to'e"
        );

        let aspect = semantic_json_for("mi morsi na'e ca'o le nu mi jmive").expect("semantic JSON");
        let morsi = predication_with_relation_and_mode(&aspect, "morsi", "asserted");
        let aspect_event = object(&aspect, morsi["eventuality"].as_str().expect("morsi event"));
        assert_eq!(aspect_event["aspect"]["contour"], "continuative");
        assert_eq!(
            aspect_event["aspect"]["scalarNegation"]["kind"],
            "otherThan"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn caha_actuality_is_explicit_not_defaulted() {
        let tenseless = semantic_json_for("ta jelca").expect("semantic JSON");
        let jelca = predication_with_relation_and_mode(&tenseless, "jelca", "asserted");
        let event = object(
            &tenseless,
            jelca["eventuality"].as_str().expect("jelca event"),
        );
        assert!(event.get("actuality").is_none());

        let present = semantic_json_for("ro datka ca flulimna").expect("semantic JSON");
        let flulimna = predication_with_relation_and_mode(&present, "flulimna", "asserted");
        let event = object(
            &present,
            flulimna["eventuality"].as_str().expect("flulimna event"),
        );
        assert_eq!(event["time"]["relation"], "at");
        assert!(event.get("actuality").is_none());

        let actual = semantic_json_for("ro datka ca ca'a flulimna").expect("semantic JSON");
        let flulimna = predication_with_relation_and_mode(&actual, "flulimna", "asserted");
        let event = object(
            &actual,
            flulimna["eventuality"].as_str().expect("flulimna event"),
        );
        assert_eq!(event["actuality"]["kind"], "actual");
        assert_eq!(event["time"]["relation"], "at");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn caha_capability_potential_and_demonstrated_are_event_actuality() {
        let capable = semantic_json_for("ro datka ka'e flulimna").expect("semantic JSON");
        let flulimna = predication_with_relation_and_mode(&capable, "flulimna", "asserted");
        let event = object(
            &capable,
            flulimna["eventuality"].as_str().expect("flulimna event"),
        );
        assert_eq!(event["actuality"]["kind"], "capable");

        let potential = semantic_json_for("ro cifydatka nu'o flulimna").expect("semantic JSON");
        let flulimna = predication_with_relation_and_mode(&potential, "flulimna", "asserted");
        let event = object(
            &potential,
            flulimna["eventuality"].as_str().expect("flulimna event"),
        );
        assert_eq!(event["actuality"]["kind"], "potential");

        let demonstrated = semantic_json_for("la .frank. pu'i viska").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&demonstrated, "viska", "asserted");
        let event = object(
            &demonstrated,
            viska["eventuality"].as_str().expect("viska event"),
        );
        assert_eq!(event["actuality"]["kind"], "demonstrated");

        let future_potential =
            semantic_json_for("la .frank. ba nu'o klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&future_potential, "klama", "asserted");
        let event = object(
            &future_potential,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["actuality"]["kind"], "potential");
        assert_eq!(event["time"]["relation"], "after");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fehe_routes_interval_properties_to_spatial_event_fields() {
        let line = semantic_json_for("ko vi'i fe'e di'i sombo le gurni").expect("semantic JSON");
        let sombo = predication_with_relation_and_mode(&line, "sombo", "asserted");
        let event = object(&line, sombo["eventuality"].as_str().expect("sombo event"));
        assert!(event.get("recurrence").is_none());
        assert_eq!(event["spaceInterval"]["dimensions"][0], "line");
        assert_eq!(event["spatialRecurrence"][0]["kind"], "regular");
        assert_eq!(event["spatialRecurrence"][0]["introducedBy"], "di'i");

        let everywhere = semantic_json_for("ze'e roroi ve'e fe'e roroi ku li re su'i re du li vo")
            .expect("semantic JSON");
        let identity = predication_with_relation_and_mode(&everywhere, "identity", "definitional");
        let event = object(
            &everywhere,
            identity["eventuality"].as_str().expect("identity event"),
        );
        assert_eq!(event["timeInterval"]["extent"], "whole");
        assert_eq!(event["recurrence"][0]["value"]["text"], "all");
        assert_eq!(event["spaceInterval"]["extent"], "whole");
        assert_eq!(event["spatialRecurrence"][0]["value"]["text"], "all");

        let spatial_start =
            semantic_json_for("tu ve'abe'a fe'e co'a rokci").expect("semantic JSON");
        let rokci = predication_with_relation_and_mode(&spatial_start, "rokci", "asserted");
        let event = object(
            &spatial_start,
            rokci["eventuality"].as_str().expect("rokci event"),
        );
        assert!(event.get("aspect").is_none());
        assert_eq!(event["spaceInterval"]["extent"], "medium");
        assert_eq!(event["spaceInterval"]["directions"][0], "north");
        assert_eq!(event["spatialAspect"]["contour"], "initiative");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_sumtcita_anchor_event_fields_to_tagged_sumti() {
        let simultaneous = semantic_json_for("mi klama le zarci ca le nu do klama le zdani")
            .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&simultaneous, "klama", "asserted");
        let event = object(
            &simultaneous,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["time"]["relation"], "at");
        assert_ne!(event["time"]["anchor"], "referent:speech-time");

        let near =
            semantic_json_for("le ratcu cu citka le cirla vi le panka").expect("semantic JSON");
        let citka = predication_with_relation_and_mode(&near, "citka", "asserted");
        let event = object(&near, citka["eventuality"].as_str().expect("citka event"));
        assert_eq!(event["space"]["relation"], "near");
        assert_ne!(event["space"]["anchor"], "referent:here");

        let retrospective =
            semantic_json_for("mi morsi ba'o le nu mi jmive").expect("semantic JSON");
        let morsi = predication_with_relation_and_mode(&retrospective, "morsi", "asserted");
        let event = object(
            &retrospective,
            morsi["eventuality"].as_str().expect("morsi event"),
        );
        assert_eq!(event["aspect"]["contour"], "retrospective");
        assert!(event["aspect"].get("anchor").is_some());

        let twice_today =
            semantic_json_for("mi klama le zarci reroi le ca djedi").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&twice_today, "klama", "asserted");
        let event = object(
            &twice_today,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["recurrence"][0]["kind"], "occurrenceCount");
        assert_eq!(event["recurrence"][0]["value"]["integer"], 2);
        assert!(event["recurrence"][0].get("interval").is_some());

        let long_winter =
            semantic_json_for("loi snime cu carvi ze'u le ca dunra").expect("semantic JSON");
        let carvi = predication_with_relation_and_mode(&long_winter, "carvi", "asserted");
        let event = object(
            &long_winter,
            carvi["eventuality"].as_str().expect("carvi event"),
        );
        assert_eq!(event["timeInterval"]["extent"], "long");
        assert!(event["timeInterval"].get("anchor").is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordered_tense_paths_distinguish_puba_and_bapu() {
        let past_future = semantic_json_for("mi puba klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&past_future, "klama", "asserted");
        let event = object(
            &past_future,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert!(event.get("time").is_none());
        assert_eq!(event["timePath"][0]["relation"], "before");
        assert_eq!(event["timePath"][0]["introducedBy"], "pu");
        assert_eq!(event["timePath"][0]["anchor"]["kind"], "object");
        assert_eq!(
            event["timePath"][0]["anchor"]["value"],
            "referent:speech-time"
        );
        assert_eq!(event["timePath"][1]["relation"], "after");
        assert_eq!(event["timePath"][1]["introducedBy"], "ba");
        assert_eq!(event["timePath"][1]["anchor"]["kind"], "previous");

        let future_past = semantic_json_for("mi bapu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&future_past, "klama", "asserted");
        let event = object(
            &future_past,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["timePath"][0]["relation"], "after");
        assert_eq!(event["timePath"][0]["introducedBy"], "ba");
        assert_eq!(event["timePath"][1]["relation"], "before");
        assert_eq!(event["timePath"][1]["introducedBy"], "pu");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn time_direction_distance_attaches_to_temporal_relation() {
        let remote_past = semantic_json_for("mi pu zu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&remote_past, "klama", "asserted");
        let event = object(
            &remote_past,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["time"]["relation"], "before");
        assert_eq!(event["time"]["distance"], "long");

        let past_then_future =
            semantic_json_for("mi pu ba za klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&past_then_future, "klama", "asserted");
        let event = object(
            &past_then_future,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["timePath"][0]["relation"], "before");
        assert!(event["timePath"][0].get("distance").is_none());
        assert_eq!(event["timePath"][1]["relation"], "after");
        assert_eq!(event["timePath"][1]["distance"], "medium");

        let remote_time = semantic_json_for("mi zu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&remote_time, "klama", "asserted");
        let event = object(
            &remote_time,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["time"]["relation"], "far");
        assert_eq!(event["time"]["anchor"], "referent:speech-time");

        let near_then_past = semantic_json_for("mi zi pu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&near_then_past, "klama", "asserted");
        let event = object(
            &near_then_past,
            klama["eventuality"].as_str().expect("klama event"),
        );
        assert_eq!(event["timePath"][0]["relation"], "near");
        assert_eq!(event["timePath"][0]["introducedBy"], "zi");
        assert_eq!(
            event["timePath"][0]["anchor"]["value"],
            "referent:speech-time"
        );
        assert_eq!(event["timePath"][1]["relation"], "before");
        assert_eq!(event["timePath"][1]["introducedBy"], "pu");
        assert_eq!(event["timePath"][1]["anchor"]["kind"], "previous");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn spatial_direction_sumtcita_anchors_event_space() {
        let json = semantic_json_for("ne'i le kevna mi zutse le rokci").expect("semantic JSON");
        let zutse = predication_with_relation_and_mode(&json, "zutse", "asserted");
        let event = object(&json, zutse["eventuality"].as_str().expect("zutse event"));
        assert_eq!(event["space"]["relation"], "within");
        assert_eq!(event["space"]["anchor"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sticky_spatial_direction_applies_to_later_bridi() {
        let json = semantic_json_for("ne'i ki le kevna mi zutse le rokci .i mi citka lo rectu")
            .expect("semantic JSON");
        let zutse = predication_with_relation_and_mode(&json, "zutse", "asserted");
        let first_event = object(&json, zutse["eventuality"].as_str().expect("zutse event"));
        assert_eq!(first_event["space"]["relation"], "within");
        assert_eq!(first_event["space"]["anchor"], "referent:r1");

        let citka = predication_with_relation_and_mode(&json, "citka", "asserted");
        let second_event = object(&json, citka["eventuality"].as_str().expect("citka event"));
        assert_eq!(second_event["space"]["relation"], "within");
        assert_eq!(second_event["space"]["anchor"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compound_spatial_path_preserves_order_and_distances() {
        let json = semantic_json_for("le nanmu ca'u vi ni'a va ri'u vu ne'i batci le gerku")
            .expect("semantic JSON");
        let batci = predication_with_relation_and_mode(&json, "batci", "asserted");
        let event = object(&json, batci["eventuality"].as_str().expect("batci event"));
        assert!(event.get("space").is_none());
        assert_eq!(event["spacePath"][0]["relation"], "inFrontOf");
        assert_eq!(event["spacePath"][0]["distance"], "short");
        assert_eq!(event["spacePath"][0]["anchor"]["value"], "referent:here");
        assert_eq!(event["spacePath"][1]["relation"], "below");
        assert_eq!(event["spacePath"][1]["distance"], "medium");
        assert_eq!(event["spacePath"][1]["anchor"]["kind"], "previous");
        assert_eq!(event["spacePath"][2]["relation"], "rightOf");
        assert_eq!(event["spacePath"][2]["distance"], "long");
        assert_eq!(event["spacePath"][3]["relation"], "within");
        assert_eq!(event["spacePath"][3]["anchor"]["kind"], "previous");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sticky_tense_applies_to_later_bridi_and_composes_with_local_tense() {
        let sticky_then_tenseless =
            semantic_json_for("mi puki klama le zarci .i le nanmu cu batci le gerku")
                .expect("semantic JSON");
        let batci = predication_with_relation_and_mode(&sticky_then_tenseless, "batci", "asserted");
        let event = object(
            &sticky_then_tenseless,
            batci["eventuality"].as_str().expect("batci event"),
        );
        assert_eq!(event["time"]["relation"], "before");
        assert_eq!(event["time"]["anchor"], "referent:speech-time");

        let sticky_then_past =
            semantic_json_for("mi puki klama le zarci .i le nanmu pu batci le gerku")
                .expect("semantic JSON");
        let batci = predication_with_relation_and_mode(&sticky_then_past, "batci", "asserted");
        let event = object(
            &sticky_then_past,
            batci["eventuality"].as_str().expect("batci event"),
        );
        assert!(event.get("time").is_none());
        assert_eq!(event["timePath"][0]["relation"], "before");
        assert_eq!(event["timePath"][0]["introducedBy"], "pu");
        assert_eq!(event["timePath"][0]["anchor"]["kind"], "object");
        assert_eq!(event["timePath"][1]["relation"], "before");
        assert_eq!(event["timePath"][1]["introducedBy"], "pu");
        assert_eq!(event["timePath"][1]["anchor"]["kind"], "previous");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nau_anchors_current_event_without_clearing_sticky_tense() {
        let json = semantic_json_for(
            "mi puki klama le zarci .i mi nau citka lo rectu .i mi pinxe lo djacu",
        )
        .expect("semantic JSON");

        let citka = predication_with_relation_and_mode(&json, "citka", "asserted");
        let citka_event = object(&json, citka["eventuality"].as_str().expect("citka event"));
        assert_eq!(citka_event["time"]["relation"], "at");
        assert_eq!(citka_event["time"]["anchor"], "referent:speech-time");

        let pinxe = predication_with_relation_and_mode(&json, "pinxe", "asserted");
        let pinxe_event = object(&json, pinxe["eventuality"].as_str().expect("pinxe event"));
        assert_eq!(pinxe_event["time"]["relation"], "before");
        assert_eq!(pinxe_event["time"]["anchor"], "referent:speech-time");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn story_time_advances_tenseless_bridi_after_previous_event() {
        let json = semantic_json_for_options(
            "mi puki klama le zarci .i mi citka lo rectu",
            SemanticBuildOptions {
                source_text: None,
                story_time: true,
            },
        )
        .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let klama_event = klama["eventuality"].as_str().expect("klama event");
        let citka = predication_with_relation_and_mode(&json, "citka", "asserted");
        let citka_event = object(&json, citka["eventuality"].as_str().expect("citka event"));
        assert_eq!(citka_event["time"]["relation"], "after");
        assert_eq!(citka_event["time"]["anchor"], klama_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn story_time_explicit_flashback_does_not_advance_anchor() {
        let json = semantic_json_for_options(
            "mi puki klama le zarci .i mi pu jukpa lo rectu .i mi citka lo rectu",
            SemanticBuildOptions {
                source_text: None,
                story_time: true,
            },
        )
        .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let klama_event = klama["eventuality"].as_str().expect("klama event");
        let jukpa = predication_with_relation_and_mode(&json, "jukpa", "asserted");
        let jukpa_event = object(&json, jukpa["eventuality"].as_str().expect("jukpa event"));
        assert_eq!(jukpa_event["time"]["relation"], "before");
        assert_eq!(jukpa_event["time"]["anchor"], klama_event);

        let citka = predication_with_relation_and_mode(&json, "citka", "asserted");
        let citka_event = object(&json, citka["eventuality"].as_str().expect("citka event"));
        assert_eq!(citka_event["time"]["relation"], "after");
        assert_eq!(citka_event["time"]["anchor"], klama_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_sticky_tense_captures_only_marked_segment() {
        let json = semantic_json_for("pu ki ku mi ba klama le zarci .i le nanmu cu batci le gerku")
            .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let klama_event = object(&json, klama["eventuality"].as_str().expect("klama event"));
        assert_eq!(klama_event["timePath"][0]["relation"], "before");
        assert_eq!(klama_event["timePath"][0]["introducedBy"], "pu");
        assert_eq!(klama_event["timePath"][1]["relation"], "after");
        assert_eq!(klama_event["timePath"][1]["introducedBy"], "ba");

        let batci = predication_with_relation_and_mode(&json, "batci", "asserted");
        let batci_event = object(&json, batci["eventuality"].as_str().expect("batci event"));
        assert!(batci_event.get("timePath").is_none());
        assert_eq!(batci_event["time"]["relation"], "before");
        assert_eq!(batci_event["time"]["anchor"], "referent:speech-time");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn subordinate_tenses_anchor_to_containing_event() {
        let former_market = semantic_json_for("mi pu klama le ba'o zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&former_market, "klama", "asserted");
        let klama_event = klama["eventuality"].as_str().expect("klama event");
        let zarci = predication_with_relation_and_mode(&former_market, "zarci", "restrictive");
        let zarci_event = object(
            &former_market,
            zarci["eventuality"].as_str().expect("zarci event"),
        );
        assert_eq!(zarci_event["aspect"]["contour"], "retrospective");
        assert_eq!(zarci_event["aspect"]["anchor"], klama_event);

        let future_dead =
            semantic_json_for("mi ca jinvi le du'u mi ba morsi").expect("semantic JSON");
        let jinvi = predication_with_relation_and_mode(&future_dead, "jinvi", "asserted");
        let jinvi_event = jinvi["eventuality"].as_str().expect("jinvi event");
        let morsi = predication_with_relation_and_mode(&future_dead, "morsi", "inert");
        let morsi_event = object(
            &future_dead,
            morsi["eventuality"].as_str().expect("morsi event"),
        );
        assert_eq!(morsi_event["time"]["relation"], "after");
        assert_eq!(morsi_event["time"]["anchor"], jinvi_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn demonstrative_sumti_are_indexical_referents() {
        let json = semantic_json_for("ta bloti").expect("semantic JSON");
        let referent = object(&json, "referent:r1");
        assert_eq!(referent["category"], "indexical");
        assert_eq!(referent["indexical"], "medialDemonstrative");

        let json = semantic_json_for("tu bloti").expect("semantic JSON");
        let referent = object(&json, "referent:r1");
        assert_eq!(referent["category"], "indexical");
        assert_eq!(referent["indexical"], "distalDemonstrative");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ko_resolves_to_addressee_and_marks_command_force() {
        let json = semantic_json_for("ko sarji la .lojban.").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["force"], "command");
        assert_eq!(
            predication_with_relation_and_mode(&json, "sarji", "asserted")["arguments"]["x1"]["value"],
            "referent:addressee"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_sumti_fragment_mentions_referent_content() {
        let json = semantic_json_for("le zarci").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["force"], "mention");
        assert_eq!(object(&json, "utterance:u1")["content"], "referent:r1");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["kind"],
            "speakerDescription"
        );
        assert!(object(&json, "utterance:u1").get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_sumti_fragment_preserves_relative_clause() {
        let json = semantic_json_for("ti noi bloti").expect("semantic JSON");
        let referent_id = object(&json, "utterance:u1")["content"]
            .as_str()
            .expect("referent content");
        let referent = object(&json, referent_id);
        let relative_clause = referent["relativeClauses"][0]
            .as_object()
            .expect("relative clause");
        assert_eq!(relative_clause["kind"], "incidental");
        let body = relative_clause["body"].as_str().expect("relative body");
        let predication = object(&json, object(&json, body)["predication"].as_str().unwrap());
        assert_eq!(predication["mode"], "incidental");
        assert_eq!(predication["arguments"]["x1"]["value"], referent_id);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typical_descriptors_are_explicit() {
        let json = semantic_json_for("lo'e cinfo cu xabju").expect("semantic JSON");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["kind"],
            "typicalDescription"
        );

        let json = semantic_json_for("le'e skina cu finti").expect("semantic JSON");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["kind"],
            "speakerStereotypeDescription"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zuhi_is_filled_typical_place_value_not_elision() {
        let json = semantic_json_for("mi klama zu'i").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        assert_eq!(klama["arguments"]["x2"]["kind"], "filled");
        let typical = klama["arguments"]["x2"]["value"]
            .as_str()
            .expect("typical place value");
        let descriptor = &object(&json, typical)["descriptor"];
        assert_eq!(descriptor["kind"], "typicalPlaceValue");
        assert_eq!(descriptor["word"], "zu'i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn soi_reciprocity_preserves_explicit_participants() {
        let json = semantic_json_for("mi prami do soi vo'a vo'e").expect("semantic JSON");
        let prami = predication_with_relation_and_mode(&json, "prami", "asserted");
        assert_eq!(prami["reciprocity"][0]["introducedBy"], "soi");
        assert_eq!(prami["reciprocity"][0]["left"]["value"], "referent:speaker");
        assert_eq!(
            prami["reciprocity"][0]["right"]["value"],
            "referent:addressee"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn soi_single_participant_uses_host_sumti() {
        let json = semantic_json_for("mi bajykla ti ta soi vo'e").expect("semantic JSON");
        let bajykla = predication_with_relation_and_mode(&json, "bajykla", "asserted");
        assert_eq!(
            bajykla["reciprocity"][0]["left"]["value"],
            bajykla["arguments"]["x2"]["value"]
        );
        assert_eq!(
            bajykla["reciprocity"][0]["right"]["value"],
            bajykla["arguments"]["x3"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preposed_soi_resolves_voha_slots_from_predication() {
        let json = semantic_json_for("soi vo'e vo'i mi bajykla ti ta").expect("semantic JSON");
        let bajykla = predication_with_relation_and_mode(&json, "bajykla", "asserted");
        assert_eq!(
            bajykla["reciprocity"][0]["left"]["value"],
            bajykla["arguments"]["x2"]["value"]
        );
        assert_eq!(
            bajykla["reciprocity"][0]["right"]["value"],
            bajykla["arguments"]["x3"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sumti_connective_pre_na_negates_left_branch() {
        let json =
            semantic_json_for("mi xabju le fi'ortu'a na.e le gligugde").expect("semantic JSON");
        let formula = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| {
                object["type"] == "formula"
                    && object["operator"] == "and"
                    && object
                        .pointer("/connector/locus")
                        .is_some_and(|locus| locus == "sumti")
            })
            .expect("sumti connective formula");
        assert_eq!(formula["connector"]["source"], "na e");
        let first_child = formula["children"][0].as_str().expect("first child id");
        assert_eq!(object(&json, first_child)["operator"], "not");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quoted_sumti_are_sign_arguments_with_parsed_utterances() {
        let json = semantic_json_for("mi cusku lu mi klama li'u do").expect("semantic JSON");
        let objects = json["objects"].as_object().expect("semantic objects");
        let (sign_id, sign) = objects
            .iter()
            .find(|(_id, object)| object["type"] == "sign")
            .expect("quotation sign");
        assert_eq!(sign["kind"], "quotation");
        assert_eq!(sign["quotation"]["mode"], "parsed");
        assert!(
            sign["quotation"]["utterance"]
                .as_str()
                .is_some_and(|id| id.starts_with("utterance:"))
        );
        assert_eq!(
            predication_with_relation_and_mode(&json, "cusku", "asserted")["arguments"]["x2"]["value"],
            sign_id.as_str()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_quotation_preserves_vocative_only_text() {
        let json = semantic_json_for("mi cusku lu mi'e .djan. li'u").expect("semantic JSON");
        let sign = object(&json, "sign:s1");
        assert_eq!(sign["quotation"]["mode"], "parsed");
        let quoted_utterance = sign["quotation"]["utterance"]
            .as_str()
            .expect("quoted vocative utterance");
        let utterance = object(&json, quoted_utterance);
        assert_eq!(utterance["force"], "vocative");
        assert_eq!(utterance["vocativeKind"], "selfIdentification");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quoted_indicator_text_preserves_sign_without_nested_utterance() {
        let json = semantic_json_for("mi cusku lu e'osai li'u do").expect("semantic JSON");
        let sign = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| object["type"] == "sign")
            .expect("quotation sign");
        assert_eq!(sign["kind"], "quotation");
        assert_eq!(sign["quotation"]["mode"], "parsed");
        assert_eq!(sign["quotation"]["text"], "lu e'osai li'u");
        assert!(sign["quotation"].get("utterance").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn named_vocative_is_vocative_utterance() {
        let json = semantic_json_for("coi .djan.").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["force"], "vocative");
        assert_eq!(utterance["vocativeKind"], "greeting");
        let audience = utterance["audience"].as_str().expect("audience referent");
        assert_eq!(object(&json, audience)["descriptor"]["kind"], "name");
        assert_eq!(object(&json, audience)["descriptor"]["name"], "djan");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn welcome_vocative_kind_uses_english_label() {
        let json = semantic_json_for("fi'i la .frank.").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["force"], "vocative");
        assert_eq!(utterance["vocativeKind"], "welcome");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_cmevla_text_is_mentioned_as_name_word_text() {
        let json = semantic_json_for(".lojban.").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["force"], "mention");
        assert_eq!(utterance["content"], "sign:s1");
        assert_eq!(object(&json, "sign:s1")["kind"], "text");
        assert_eq!(object(&json, "sign:s1")["text"], "lojban");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_selbri_vocative_targets_implicit_speaker_description() {
        let json = semantic_json_for("coi xunre pastu nixli").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["force"], "vocative");
        let audience = utterance["audience"].as_str().expect("audience referent");
        let descriptor = &object(&json, audience)["descriptor"];
        assert_eq!(descriptor["kind"], "speakerDescription");
        assert_eq!(descriptor["word"], "le");
        let body = descriptor["body"].as_str().expect("vocative restriction");
        assert_eq!(object(&json, body)["type"], "formula");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn named_vocative_relative_clause_qualifies_audience() {
        let json = semantic_json_for("coi .frank. poi xunre se bende").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["force"], "vocative");
        let audience = utterance["audience"].as_str().expect("audience referent");
        let relative_clauses = object(&json, audience)["relativeClauses"]
            .as_array()
            .expect("audience relative clauses");
        assert_eq!(relative_clauses.len(), 1);
        assert_eq!(relative_clauses[0]["kind"], "restrictive");
        let body = relative_clauses[0]["body"]
            .as_str()
            .expect("relative-clause body");
        assert_eq!(object(&json, body)["type"], "formula");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selbri_vocative_relative_clauses_are_descriptor_scoped() {
        for source in [
            "co'o poi mi zvati ke'a ku'o xirma",
            "co'o xirma poi mi zvati",
        ] {
            let json = semantic_json_for(source).expect("semantic JSON");
            let utterance = root_object(&json);
            assert_eq!(utterance["force"], "vocative");
            let audience = utterance["audience"].as_str().expect("audience referent");
            let descriptor = &object(&json, audience)["descriptor"];
            let relative_clauses = descriptor["relativeClauses"]
                .as_array()
                .expect("descriptor relative clauses");
            assert_eq!(relative_clauses.len(), 1);
            assert_eq!(relative_clauses[0]["kind"], "restrictive");
            let zvati = predication_with_relation_and_mode(&json, "zvati", "restrictive");
            assert_eq!(zvati["arguments"]["x1"]["value"], "referent:speaker");
            assert_eq!(zvati["arguments"]["x2"]["value"], audience);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sentence_internal_vocatives_are_utterance_asides() {
        for source in ["doi .djan. ko klama mi", "ko klama mi doi .djan."] {
            let json = semantic_json_for(source).expect("semantic JSON");
            let utterance = root_object(&json);
            assert_eq!(utterance["force"], "command");
            let aside = utterance["asides"][0].as_str().expect("vocative aside");
            let vocative = object(&json, aside);
            assert_eq!(vocative["force"], "vocative");
            assert_eq!(vocative["vocativeKind"], "address");
            let audience = vocative["audience"].as_str().expect("vocative audience");
            assert_eq!(object(&json, audience)["descriptor"]["name"], "djan");
            let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
            assert_eq!(klama["arguments"]["x1"]["value"], "referent:addressee");
            assert_eq!(klama["arguments"]["x2"]["value"], "referent:speaker");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vocative_assignments_target_indexicals() {
        let json = semantic_json_for("mi'e .djan. doi .frank. mi cusku lu mi bajra li'u do")
            .expect("semantic JSON");
        let utterance = root_object(&json);
        let asides = utterance["asides"].as_array().expect("vocative asides");
        assert_eq!(asides.len(), 1);

        let self_identification = object(
            &json,
            asides[0].as_str().expect("self-identification aside"),
        );
        assert_eq!(self_identification["vocativeKind"], "selfIdentification");
        let nested_asides = self_identification["asides"]
            .as_array()
            .expect("nested address aside");

        let address = object(&json, nested_asides[0].as_str().expect("address aside"));
        assert_eq!(address["vocativeKind"], "address");
        let frank = address["audience"].as_str().expect("address audience");
        assert_eq!(object(&json, frank)["descriptor"]["name"], "frank");
        assert_eq!(object(&json, frank)["target"], "referent:addressee");

        let john = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| object["descriptor"]["name"] == "djan")
            .expect("self-identified speaker name");
        assert_eq!(john["target"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn deleted_place_is_explicit_and_distinct_from_elision() {
        let json = semantic_json_for("mi klama zi'o").expect("semantic JSON");
        let arguments = object(&json, "predication:p1")["arguments"]
            .as_object()
            .expect("predication arguments");
        assert_eq!(arguments["x1"]["kind"], "filled");
        assert_eq!(arguments["x1"]["value"], "referent:speaker");
        assert_eq!(arguments["x2"]["kind"], "deleted");
        assert_eq!(arguments["x2"]["introducedBy"], "zi'o");
        assert!(arguments["x2"].get("value").is_none());
        assert!(arguments.contains_key("x3"));
        assert_eq!(arguments["x3"]["kind"], "elided");
        assert!(json["objects"].as_object().unwrap().values().all(|object| {
            object
                .pointer("/descriptor/word")
                .is_none_or(|word| word != "zi'o")
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn time_direction_tag_anchors_eventuality_to_speech_time() {
        let json = semantic_json_for("mi pu klama").expect("semantic JSON");
        let event = object(&json, "eventuality:e0");
        assert_eq!(event["time"]["relation"], "before");
        assert_eq!(event["time"]["anchor"], "referent:speech-time");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn space_distance_tag_anchors_eventuality_to_here() {
        let json = semantic_json_for("le vi bloti").expect("semantic JSON");
        let event = object(&json, "predication:p1")["eventuality"]
            .as_str()
            .expect("tagged restrictive eventuality");
        let event = object(&json, event);
        assert_eq!(event["space"]["relation"], "near");
        assert_eq!(event["space"]["anchor"], "referent:here");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn governed_termset_supplies_spatial_anchor_and_exact_magnitude() {
        let json =
            semantic_json_for("la .frank. sanli zu'a nu'i la .djordj. la'u lo mitre be li mu")
                .expect("semantic JSON");
        let sanli = predication_with_relation_and_mode(&json, "sanli", "asserted");
        assert_eq!(sanli["arguments"]["x2"]["kind"], "elided");
        assert_eq!(sanli["modalArguments"].as_array().map(Vec::len), None);

        let event = object(&json, sanli["eventuality"].as_str().expect("sanli event"));
        assert_eq!(event["space"]["relation"], "leftOf");
        let anchor = event["space"]["anchor"].as_str().expect("space anchor");
        assert_eq!(object(&json, anchor)["descriptor"]["name"], "djordj");

        let magnitude = &event["space"]["magnitude"];
        assert_eq!(magnitude["introducedBy"], "la'u");
        let magnitude_value = magnitude["value"].as_str().expect("magnitude value");
        assert_eq!(
            object(&json, magnitude_value)["source"]["text"],
            "lo mitre be li mu"
        );
        let mitre = predication_with_relation_and_mode(&json, "mitre", "restrictive");
        let number = mitre["arguments"]["x2"]["value"]
            .as_str()
            .expect("mitre x2 number");
        let quantity = object(&json, number)["descriptor"]["quantity"]
            .as_str()
            .expect("number quantity");
        assert_eq!(object(&json, quantity)["value"]["integer"], 5);

        let no_origin = semantic_json_for("la .frank. sanli zu'a nu'i la'u lo mitre be li mu")
            .expect("semantic JSON");
        let sanli = predication_with_relation_and_mode(&no_origin, "sanli", "asserted");
        let event = object(
            &no_origin,
            sanli["eventuality"].as_str().expect("sanli event"),
        );
        assert_eq!(event["space"]["anchor"], "referent:here");
        assert_eq!(event["space"]["magnitude"]["introducedBy"], "la'u");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bridi_negation_wraps_formula_instead_of_relation_label() {
        let json = semantic_json_for("la .djonz. na pamoi cusku").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f5");
        assert_eq!(object(&json, "formula:f5")["operator"], "not");
        assert_eq!(object(&json, "formula:f5")["children"][0], "formula:f4");
        assert!(
            predication_relations(&json)
                .iter()
                .all(|relation| !relation.contains("scalar-not"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn double_bridi_negation_preserves_nested_scope() {
        let json = semantic_json_for("mi na na klama le zarci").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f4");
        assert_eq!(object(&json, "formula:f4")["operator"], "not");
        assert_eq!(object(&json, "formula:f4")["children"][0], "formula:f3");
        assert_eq!(object(&json, "formula:f3")["operator"], "not");
        assert_eq!(object(&json, "formula:f3")["children"][0], "formula:f2");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_can_scope_over_negated_formula() {
        let json = semantic_json_for("mi na pu na ca klama le zarci").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f5");
        assert_eq!(object(&json, "formula:f5")["operator"], "not");
        assert_eq!(object(&json, "formula:f5")["children"][0], "formula:f4");
        assert_eq!(object(&json, "formula:f4")["operator"], "scoped");
        assert_eq!(object(&json, "formula:f4")["children"][0], "formula:f3");
        assert_eq!(object(&json, "formula:f4")["eventuality"], "eventuality:e1");
        assert_eq!(
            object(&json, "eventuality:e1")["time"]["relation"],
            "before"
        );
        assert_eq!(object(&json, "formula:f3")["operator"], "not");
        let klama_event = object(&json, "predication:p2")["eventuality"]
            .as_str()
            .expect("klama eventuality");
        assert_eq!(object(&json, klama_event)["time"]["relation"], "at");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_connective_adds_sequence_content_formula() {
        let json = semantic_json_for("mi klama .ije do cadzu").expect("semantic JSON");
        let sequence = object(&json, "sequence:s1");
        assert_eq!(json["root"], "sequence:s1");
        assert_eq!(sequence["items"][0], "utterance:u1");
        let content = sequence["content"]
            .as_str()
            .expect("statement connection content");
        assert_eq!(object(&json, content)["operator"], "and");
        assert_eq!(object(&json, content)["connector"]["source"], "je");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_statement_connection_adds_event_relation_claim() {
        let because = semantic_json_for("le spati cu banro .iri'abo do djacu dunda fi le spati")
            .expect("semantic JSON");
        let sequence = object(&because, "sequence:s1");
        let claim = sequence["connectionClaims"][0]
            .as_str()
            .expect("connection claim");
        let claim_formula = object(&because, claim);
        let rinka = object(
            &because,
            claim_formula["predication"]
                .as_str()
                .expect("claim predication"),
        );
        let banro_event =
            predication_with_relation_and_mode(&because, "banro", "asserted")["eventuality"]
                .as_str()
                .expect("banro eventuality");
        let dunda_event =
            predication_with_relation_and_mode(&because, "dunda", "asserted")["eventuality"]
                .as_str()
                .expect("dunda eventuality");
        assert_eq!(rinka["relation"], "rinka");
        assert_eq!(rinka["introducedBy"], "ri'a");
        assert_eq!(rinka["arguments"]["x1"]["value"], dunda_event);
        assert_eq!(rinka["arguments"]["x2"]["value"], banro_event);

        let therefore =
            semantic_json_for("do djacu dunda fi le spati .iseri'abo le spati cu banro")
                .expect("semantic JSON");
        let sequence = object(&therefore, "sequence:s1");
        let claim = sequence["connectionClaims"][0]
            .as_str()
            .expect("connection claim");
        let claim_formula = object(&therefore, claim);
        let rinka = object(
            &therefore,
            claim_formula["predication"]
                .as_str()
                .expect("claim predication"),
        );
        let dunda_event =
            predication_with_relation_and_mode(&therefore, "dunda", "asserted")["eventuality"]
                .as_str()
                .expect("dunda eventuality");
        let banro_event =
            predication_with_relation_and_mode(&therefore, "banro", "asserted")["eventuality"]
                .as_str()
                .expect("banro eventuality");
        assert_eq!(rinka["relation"], "rinka");
        assert_eq!(rinka["introducedBy"], "se ri'a");
        assert_eq!(rinka["arguments"]["x1"]["value"], dunda_event);
        assert_eq!(rinka["arguments"]["x2"]["value"], banro_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_statement_connection_adds_event_relation_claim() {
        let json =
            semantic_json_for("le nanmu cu batci le gerku .izu'abo le verba cu cadzu le bisli")
                .expect("semantic JSON");
        let sequence = object(&json, "sequence:s1");
        let claim = sequence["connectionClaims"][0]
            .as_str()
            .expect("connection claim");
        let claim_formula = object(&json, claim);
        let left_of = object(
            &json,
            claim_formula["predication"]
                .as_str()
                .expect("claim predication"),
        );
        let batci_event =
            predication_with_relation_and_mode(&json, "batci", "asserted")["eventuality"]
                .as_str()
                .expect("batci event");
        let cadzu_event =
            predication_with_relation_and_mode(&json, "cadzu", "asserted")["eventuality"]
                .as_str()
                .expect("cadzu event");
        assert_eq!(left_of["relation"], "leftOf");
        assert_eq!(left_of["introducedBy"], "zu'a");
        assert_eq!(left_of["arguments"]["x1"]["value"], cadzu_event);
        assert_eq!(left_of["arguments"]["x2"]["value"], batci_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_tense_sentence_connection_claims_branches_and_relation() {
        let json = semantic_json_for("pugi mi klama le zarci gi mi klama le zdani")
            .expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["operator"], "and");
        assert_eq!(content["connector"]["source"], "pu gi");
        let klama = predications_with_relation_and_mode(&json, "klama", "asserted");
        assert_eq!(klama.len(), 2);
        let before = predication_with_relation_and_mode(&json, "before", "asserted");
        assert_eq!(before["introducedBy"], "pu");
        assert_eq!(before["arguments"]["x1"]["value"], klama[1]["eventuality"]);
        assert_eq!(before["arguments"]["x2"]["value"], klama[0]["eventuality"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_sumti_and_bridi_tail_connections_claim_only_relation() {
        for source in [
            "mi klama pugi le zarci gi le zdani",
            "mi pugi klama le zarci gi klama le zdani",
        ] {
            let json = semantic_json_for(source).expect("semantic JSON");
            let content_id = object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content");
            let content = object(&json, content_id);
            assert_eq!(content["operator"], "atom");
            let before = object(
                &json,
                content["predication"]
                    .as_str()
                    .expect("relation claim predication"),
            );
            assert_eq!(before["relation"], "before");
            assert_eq!(before["mode"], "asserted");
            assert_eq!(before["introducedBy"], "pu");
            let asserted_klama = predications_with_relation_and_mode(&json, "klama", "asserted");
            assert!(asserted_klama.is_empty());
            let inert_klama = predications_with_relation_and_mode(&json, "klama", "inert");
            assert_eq!(inert_klama.len(), 2);
            assert_eq!(
                before["arguments"]["x1"]["value"],
                inert_klama[1]["eventuality"]
            );
            assert_eq!(
                before["arguments"]["x2"]["value"],
                inert_klama[0]["eventuality"]
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_tensed_sumti_connection_claims_branches_and_relation() {
        let json =
            semantic_json_for("la .teris. satre le mlatu .ebabo le ractu").expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["operator"], "and");
        assert_eq!(content["connector"]["source"], "e ba bo");
        let satre = predications_with_relation_and_mode(&json, "satre", "asserted");
        assert_eq!(satre.len(), 2);
        let after = predication_with_relation_and_mode(&json, "after", "asserted");
        assert_eq!(after["introducedBy"], "ba");
        assert_eq!(after["arguments"]["x1"]["value"], satre[1]["eventuality"]);
        assert_eq!(after["arguments"]["x2"]["value"], satre[0]["eventuality"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tensed_grouped_statement_connection_relates_whole_group() {
        let json = semantic_json_for(
            "mi bevri le dakli .ije ba tu'e mi bevri le gerku .ija cabo mi bevri le mlatu tu'u",
        )
        .expect("semantic JSON");
        let outer = object(&json, "sequence:s2");
        let outer_claim = outer["connectionClaims"][0]
            .as_str()
            .expect("outer connection claim");
        let after = object(
            &json,
            object(&json, outer_claim)["predication"]
                .as_str()
                .expect("after predication"),
        );
        assert_eq!(after["relation"], "after");
        assert_eq!(after["introducedBy"], "ba");
        let sack_event =
            predications_with_relation_and_mode(&json, "bevri", "asserted")[0]["eventuality"]
                .as_str()
                .expect("sack carrying event");
        assert_eq!(after["arguments"]["x2"]["value"], sack_event);
        let grouped_event = after["arguments"]["x1"]["value"]
            .as_str()
            .expect("group event");
        assert_eq!(object(&json, grouped_event)["content"], "sequence:s1");

        let inner = object(&json, "sequence:s1");
        let inner_claim = inner["connectionClaims"][0]
            .as_str()
            .expect("inner connection claim");
        let at = object(
            &json,
            object(&json, inner_claim)["predication"]
                .as_str()
                .expect("at predication"),
        );
        assert_eq!(at["relation"], "at");
        assert_eq!(at["introducedBy"], "ca");
        for bevri in predications_with_relation_and_mode(&json, "bevri", "asserted") {
            assert!(bevri.get("modalArguments").is_none());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jai_bai_raises_modal_argument_without_replacing_inner_x1() {
        let json = semantic_json_for("la .lojban. jai bau cusku fai mi").expect("semantic JSON");
        let cusku = predication_with_relation_and_mode(&json, "cusku", "asserted");
        assert_eq!(cusku["arguments"]["x1"]["value"], "referent:speaker");

        let lojban = named_referent_id(&json, "lojban");
        let modal_argument = cusku["modalArguments"]
            .as_array()
            .expect("modal arguments")
            .iter()
            .find(|argument| argument["relation"] == "bangu")
            .expect("bau modal argument");
        assert_eq!(modal_argument["introducedBy"], "bau");
        assert_eq!(modal_argument["arguments"]["x1"]["value"], lojban);
        assert_eq!(modal_argument["arguments"]["x2"]["kind"], "elided");
        assert_eq!(modal_argument["arguments"]["x3"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn asserted_bare_jai_uses_abstraction_about_raised_operand() {
        let json = semantic_json_for("mi jai rinka le nu do morsi").expect("semantic JSON");
        let rinka = predication_with_relation_and_mode(&json, "rinka", "asserted");
        let raised = rinka["arguments"]["x1"]["value"]
            .as_str()
            .expect("raised abstraction referent");
        let raised = object(&json, raised);
        assert_eq!(raised["sort"], "proposition");
        assert_eq!(raised["descriptor"]["kind"], "abstractionAbout");
        assert_eq!(raised["descriptor"]["word"], "jai");
        assert_eq!(raised["descriptor"]["operand"], "referent:speaker");
        assert_eq!(rinka["arguments"]["x2"]["kind"], "filled");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restrictive_bare_jai_description_uses_abstraction_about_described_referent() {
        let json = semantic_json_for("le jai rinka be le nu do morsi").expect("semantic JSON");
        let described = root_object(&json)["content"]
            .as_str()
            .expect("mentioned description");
        let rinka = predication_with_relation_and_mode(&json, "rinka", "restrictive");
        assert_ne!(rinka["arguments"]["x1"]["value"], described);
        let raised = rinka["arguments"]["x1"]["value"]
            .as_str()
            .expect("raised abstraction referent");
        let raised = object(&json, raised);
        assert_eq!(raised["descriptor"]["kind"], "abstractionAbout");
        assert_eq!(raised["descriptor"]["word"], "jai");
        assert_eq!(raised["descriptor"]["operand"], described);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restrictive_jai_bai_description_raises_modal_argument() {
        let json = semantic_json_for("le jai gau rinka be le nu do morsi").expect("semantic JSON");
        let described = root_object(&json)["content"]
            .as_str()
            .expect("mentioned description");
        let rinka = predication_with_relation_and_mode(&json, "rinka", "restrictive");
        assert_eq!(rinka["arguments"]["x1"]["kind"], "elided");
        assert_eq!(rinka["arguments"]["x2"]["kind"], "filled");
        let modal_argument = rinka["modalArguments"]
            .as_array()
            .expect("modal arguments")
            .iter()
            .find(|argument| argument["introducedBy"] == "gau")
            .expect("gau modal argument");
        assert_eq!(modal_argument["relation"], "gasnu");
        assert_eq!(modal_argument["arguments"]["x1"]["value"], described);
        assert_eq!(modal_argument["arguments"]["x2"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restrictive_jai_tense_raises_anchor_without_replacing_inner_places() {
        let place_json =
            semantic_json_for("mi viska le jai vi citka be le cirla").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&place_json, "viska", "asserted");
        let raised_place = viska["arguments"]["x2"]["value"]
            .as_str()
            .expect("visible place");
        let citka = predication_with_relation_and_mode(&place_json, "citka", "restrictive");
        assert_eq!(citka["arguments"]["x1"]["kind"], "elided");
        assert_eq!(citka["arguments"]["x2"]["kind"], "filled");
        let citka_event = object(
            &place_json,
            citka["eventuality"].as_str().expect("citka eventuality"),
        );
        assert_eq!(citka_event["space"]["relation"], "near");
        assert_eq!(citka_event["space"]["anchor"], raised_place);

        let time_json = semantic_json_for("mi djuno fi le jai ca morsi be fai la .djan.")
            .expect("semantic JSON");
        let djuno = predication_with_relation_and_mode(&time_json, "djuno", "asserted");
        let raised_time = djuno["arguments"]["x3"]["value"]
            .as_str()
            .expect("visible time");
        let morsi = predication_with_relation_and_mode(&time_json, "morsi", "restrictive");
        assert_eq!(
            morsi["arguments"]["x1"]["value"],
            named_referent_id(&time_json, "djan")
        );
        let morsi_event = object(
            &time_json,
            morsi["eventuality"].as_str().expect("morsi eventuality"),
        );
        assert_eq!(morsi_event["time"]["relation"], "at");
        assert_eq!(morsi_event["time"]["anchor"], raised_time);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_tenses_split_into_branch_scoped_event_claims() {
        let temporal =
            semantic_json_for("mi punai je canai je ba klama le zarci").expect("semantic JSON");
        let root = object(
            &temporal,
            object(&temporal, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(root["operator"], "and");
        assert_eq!(root["children"].as_array().expect("children").len(), 3);
        assert_eq!(
            object(&temporal, root["children"][0].as_str().unwrap())["operator"],
            "not"
        );
        assert_eq!(
            object(&temporal, root["children"][1].as_str().unwrap())["operator"],
            "not"
        );
        let klama = predications_with_relation_and_mode(&temporal, "klama", "asserted");
        assert_eq!(klama.len(), 3);
        let times = klama
            .iter()
            .map(|predication| {
                let event = object(
                    &temporal,
                    predication["eventuality"].as_str().expect("eventuality"),
                );
                event["time"]["relation"].as_str().expect("time relation")
            })
            .collect::<Vec<_>>();
        assert_eq!(times, ["before", "at", "after"]);

        let spatial = semantic_json_for("mi mo'izu'a naje mo'iri'u cadzu").expect("semantic JSON");
        let root = object(
            &spatial,
            object(&spatial, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(root["operator"], "and");
        assert_eq!(
            object(&spatial, root["children"][0].as_str().unwrap())["operator"],
            "not"
        );
        let cadzu = predications_with_relation_and_mode(&spatial, "cadzu", "asserted");
        assert_eq!(cadzu.len(), 2);
        let left_event = object(
            &spatial,
            cadzu[0]["eventuality"].as_str().expect("eventuality"),
        );
        let right_event = object(
            &spatial,
            cadzu[1]["eventuality"].as_str().expect("eventuality"),
        );
        assert_eq!(left_event["space"]["relation"], "leftOf");
        assert_eq!(right_event["space"]["relation"], "rightOf");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bihi_temporal_interval_uses_time_span_endpoints() {
        let json = semantic_json_for("mi puza bi'o bazu vasxu").expect("semantic JSON");
        let vasxu = predication_with_relation_and_mode(&json, "vasxu", "asserted");
        let event = object(
            &json,
            vasxu["eventuality"].as_str().expect("vasxu eventuality"),
        );
        assert!(event.get("timePath").is_none());
        assert_eq!(event["timeSpan"]["introducedBy"], "bi'o");
        assert_eq!(event["timeSpan"]["start"]["relation"], "before");
        assert_eq!(event["timeSpan"]["start"]["distance"], "medium");
        assert_eq!(event["timeSpan"]["end"]["relation"], "after");
        assert_eq!(event["timeSpan"]["end"]["distance"], "long");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_bai_negation_preserves_polarity() {
        let contradictory =
            semantic_json_for("mi nelci do mu'inai le nu do nelci mi").expect("semantic JSON");
        let nelci = predication_with_relation_and_mode(&contradictory, "nelci", "asserted");
        let modal_argument = &nelci["modalArguments"][0];
        assert_eq!(modal_argument["relation"], "mukti");
        assert_eq!(modal_argument["introducedBy"], "mu'i");
        assert_eq!(modal_argument["negation"]["kind"], "contradictory");
        assert_eq!(modal_argument["negation"]["introducedBy"], "nai");
        assert!(modal_argument.get("scalarNegation").is_none());

        let scalar =
            semantic_json_for("le spati cu banro na'emu'i le nu do djacu dunda fi le spati")
                .expect("semantic JSON");
        let banro = predication_with_relation_and_mode(&scalar, "banro", "asserted");
        let modal_argument = &banro["modalArguments"][0];
        assert_eq!(modal_argument["relation"], "mukti");
        assert_eq!(modal_argument["introducedBy"], "mu'i");
        assert!(modal_argument.get("negation").is_none());
        assert_eq!(modal_argument["scalarNegation"]["kind"], "otherThan");
        assert_eq!(modal_argument["scalarNegation"]["introducedBy"], "na'e");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_tag_indicators_are_preserved_on_modal_argument() {
        let json = semantic_json_for("go'i ji'una'iku").expect("semantic JSON");
        let gohi = predication_with_relation_and_mode(&json, "go'i", "asserted");
        let modal_argument = &gohi["modalArguments"][0];
        assert_eq!(modal_argument["relation"], "ji'u");
        assert_eq!(modal_argument["modifiers"][0]["relation"], "na'i");
        assert_eq!(modal_argument["modifiers"][0]["family"], "metalinguistic");
        assert_eq!(modal_argument["modifiers"][0]["assertionEffect"], "none");
        assert_eq!(modal_argument["modifiers"][0]["source"]["text"], "na'i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sticky_modal_repeats_until_bare_ki_reset() {
        let sticky =
            semantic_json_for("mi tavla bai ki tu'a la .frank. .i mi tavla bau la .lojban.")
                .expect("semantic JSON");
        let first = object(&sticky, "predication:p1");
        let second = object(&sticky, "predication:p2");
        assert_eq!(first["modalArguments"][0]["relation"], "bapli");
        assert_eq!(
            first["modalArguments"][0]["arguments"]["x1"]["value"],
            "referent:r2"
        );
        assert_eq!(second["modalArguments"][0]["relation"], "bangu");
        assert_eq!(second["modalArguments"][1]["relation"], "bapli");
        assert_eq!(
            second["modalArguments"][1]["arguments"]["x1"]["value"],
            "referent:r2"
        );

        let reset = semantic_json_for("mi tavla bai ki tu'a la .frank. .i mi ki tavla")
            .expect("semantic JSON");
        assert!(
            object(&reset, "predication:p2")
                .get("modalArguments")
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_modal_tag_connection_distributes_host_predication() {
        let json =
            semantic_json_for("la .frank. bajra seka'a je teka'a le zdani").expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["operator"], "and");
        assert_eq!(content["connector"]["locus"], "modal");
        assert_eq!(content["connector"]["truthTable"], "je");

        let bajra = predications_with_relation_and_mode(&json, "bajra", "asserted");
        assert_eq!(bajra.len(), 2);
        assert_ne!(bajra[0]["eventuality"], bajra[1]["eventuality"]);
        assert_eq!(bajra[0]["arguments"]["x1"], bajra[1]["arguments"]["x1"]);
        let destination_modal = bajra
            .iter()
            .flat_map(|predication| {
                predication["modalArguments"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .find(|modal| modal["introducedBy"] == "se ka'a")
            .expect("destination modal");
        let origin_modal = bajra
            .iter()
            .flat_map(|predication| {
                predication["modalArguments"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .find(|modal| modal["introducedBy"] == "te ka'a")
            .expect("origin modal");
        assert_eq!(destination_modal["relation"], "klama");
        assert_eq!(origin_modal["relation"], "klama");
        assert_eq!(
            destination_modal["arguments"]["x2"]["value"],
            origin_modal["arguments"]["x3"]["value"]
        );

        let termset = semantic_json_for("la .frank. bajra seka'a le zdani ce'e teka'a le zdani")
            .expect("semantic JSON");
        let shared_bajra = predication_with_relation_and_mode(&termset, "bajra", "asserted");
        let shared_modals = shared_bajra["modalArguments"]
            .as_array()
            .expect("shared modal arguments");
        assert_eq!(shared_modals.len(), 2);
        assert_eq!(shared_modals[0]["introducedBy"], "se ka'a");
        assert_eq!(shared_modals[1]["introducedBy"], "te ka'a");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_modal_bridi_tail_connection_shares_outer_places() {
        let json =
            semantic_json_for("mi mu'igi viska gi lebna vau le cukta").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&json, "viska", "asserted");
        let lebna = predication_with_relation_and_mode(&json, "lebna", "asserted");
        assert_eq!(viska["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(lebna["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(
            viska["arguments"]["x2"]["value"],
            lebna["arguments"]["x2"]["value"]
        );
        let mukti = predication_with_relation_and_mode(&json, "mukti", "asserted");
        assert_eq!(mukti["introducedBy"], "mu'i");
        assert_eq!(
            mukti["arguments"]["x1"]["value"],
            viska["eventuality"].as_str().expect("viska event")
        );
        assert_eq!(
            mukti["arguments"]["x2"]["value"],
            lebna["eventuality"].as_str().expect("lebna event")
        );
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["connector"]["source"], "mu'i gi");
        assert_eq!(content["connector"]["locus"], "bridi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn spread_modal_sumti_shares_complete_modal_relation() {
        let json = semantic_json_for("mi bai ke ge klama le zarci gi cadzu le bisli ke'e")
            .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let cadzu = predication_with_relation_and_mode(&json, "cadzu", "asserted");
        assert_eq!(klama["modalArguments"][0]["relation"], "bapli");
        assert_eq!(cadzu["modalArguments"][0]["relation"], "bapli");
        assert_eq!(
            klama["modalArguments"][0]["arguments"]["x1"]["value"],
            cadzu["modalArguments"][0]["arguments"]["x1"]["value"]
        );
        assert_eq!(
            klama["modalArguments"][0]["arguments"]["x2"]["value"],
            cadzu["modalArguments"][0]["arguments"]["x2"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_modal_termset_connection_pairs_whole_branches() {
        let json = semantic_json_for(
            "nu'i mu'igi la .djan. lei jdini mi gi mi le cukta la .djan. nu'u dunda",
        )
        .expect("semantic JSON");
        let dundas = predications_with_relation_and_mode(&json, "dunda", "asserted");
        assert_eq!(dundas.len(), 2);
        let mukti = predication_with_relation_and_mode(&json, "mukti", "asserted");
        assert_eq!(mukti["introducedBy"], "mu'i");
        assert_eq!(
            mukti["arguments"]["x1"]["value"],
            dundas[0]["eventuality"]
                .as_str()
                .expect("first dunda event")
        );
        assert_eq!(
            mukti["arguments"]["x2"]["value"],
            dundas[1]["eventuality"]
                .as_str()
                .expect("second dunda event")
        );
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["connector"]["source"], "mu'i gi");
        assert_eq!(content["connector"]["locus"], "termset");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nibli_modal_connections_use_formula_arguments() {
        let json = semantic_json_for("li ny. du li vo .ini'ibo li ny. du li re su'i re")
            .expect("semantic JSON");
        let identities = predications_with_relation_and_mode(&json, "identity", "definitional");
        assert_eq!(identities.len(), 2);
        assert_eq!(
            identities[0]["arguments"]["x1"]["value"],
            identities[1]["arguments"]["x1"]["value"]
        );
        let nibli = predication_with_relation_and_mode(&json, "nibli", "asserted");
        for place in ["x1", "x2"] {
            let formula = nibli["arguments"][place]["value"]
                .as_str()
                .expect("formula argument");
            assert_eq!(object(&json, formula)["type"], "formula");
        }
        assert_eq!(nibli["introducedBy"], "ni'i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mixed_modal_statement_connection_keeps_modal_claim() {
        let json =
            semantic_json_for("mi nelci do .ijeki'ubo mi nelci la .djein.").expect("semantic JSON");
        let sequence = object(&json, "sequence:s1");
        let content = sequence["content"]
            .as_str()
            .expect("mixed statement logical content");
        assert_eq!(object(&json, content)["operator"], "and");
        assert_eq!(object(&json, content)["connector"]["source"], "je ki'u bo");
        let claim = sequence["connectionClaims"][0]
            .as_str()
            .expect("mixed connection claim");
        let krinu = object(
            &json,
            object(&json, claim)["predication"]
                .as_str()
                .expect("claim predication"),
        );
        let nelci = predications_with_relation_and_mode(&json, "nelci", "asserted");
        assert_eq!(krinu["introducedBy"], "ki'u");
        assert_eq!(krinu["arguments"]["x1"]["value"], nelci[1]["eventuality"]);
        assert_eq!(krinu["arguments"]["x2"]["value"], nelci[0]["eventuality"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mixed_modal_sumti_connection_keeps_modal_claim() {
        let json = semantic_json_for("mi nelci do .eki'ubo la .djein.").expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["connector"]["source"], "e ki'u bo");
        assert_eq!(content["children"].as_array().expect("children").len(), 3);
        let krinu = predication_with_relation_and_mode(&json, "krinu", "asserted");
        let nelci = predications_with_relation_and_mode(&json, "nelci", "asserted");
        assert_eq!(krinu["arguments"]["x1"]["value"], nelci[1]["eventuality"]);
        assert_eq!(krinu["arguments"]["x2"]["value"], nelci[0]["eventuality"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_grouped_modal_sumti_connection_reifies_group_effect() {
        let json =
            semantic_json_for("mi bevri le dakli .eseri'ake le gerku .adu'ibo le mlatu ke'e")
                .expect("semantic JSON");
        let content_id = object(&json, "utterance:u1")["content"]
            .as_str()
            .expect("utterance content");
        let content = object(&json, content_id);
        assert_eq!(content["connector"]["source"], "e se ri'a");
        let inner_id = content["children"][1]
            .as_str()
            .expect("inner grouped sumti formula");
        let inner = object(&json, inner_id);
        assert_eq!(inner["connector"]["source"], "a du'i bo");

        let carries = predications_with_relation_and_mode(&json, "bevri", "asserted");
        let dunli = predication_with_relation_and_mode(&json, "dunli", "asserted");
        assert_eq!(dunli["introducedBy"], "du'i");
        assert_eq!(dunli["arguments"]["x1"]["value"], carries[2]["eventuality"]);
        assert_eq!(dunli["arguments"]["x2"]["value"], carries[1]["eventuality"]);

        let rinka = predication_with_relation_and_mode(&json, "rinka", "asserted");
        assert_eq!(rinka["introducedBy"], "se ri'a");
        assert_eq!(rinka["arguments"]["x1"]["value"], carries[0]["eventuality"]);
        let effect_event = rinka["arguments"]["x2"]["value"]
            .as_str()
            .expect("effect eventuality");
        assert_eq!(object(&json, effect_event)["content"], inner_id);
        for place in ["x3", "x4", "x5"] {
            assert_eq!(
                carries[0]["arguments"][place]["value"],
                carries[1]["arguments"][place]["value"]
            );
            assert_eq!(
                carries[1]["arguments"][place]["value"],
                carries[2]["arguments"][place]["value"]
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mixed_bound_bridi_tail_connection_keeps_modal_claim() {
        let json =
            semantic_json_for("mi nelci do gi'eki'ubo nelci la .djein.").expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["connector"]["source"], "gi'e ki'u bo");
        assert_eq!(content["children"].as_array().expect("children").len(), 3);
        let krinu = predication_with_relation_and_mode(&json, "krinu", "asserted");
        let nelci = predications_with_relation_and_mode(&json, "nelci", "asserted");
        assert_eq!(krinu["arguments"]["x1"]["value"], nelci[1]["eventuality"]);
        assert_eq!(krinu["arguments"]["x2"]["value"], nelci[0]["eventuality"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duhi_modal_connection_uses_dunli_source_relation() {
        let json = semantic_json_for("mi bevri le gerku gi'adu'ibo bevri le mlatu")
            .expect("semantic JSON");
        let dunli = predication_with_relation_and_mode(&json, "dunli", "asserted");
        assert_eq!(dunli["introducedBy"], "du'i");
        assert_eq!(dunli["arguments"]["x3"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_mekso_identity_expands_to_entailment_between_identities() {
        let json = semantic_json_for("li ny. du li ni'igi vei re su'i re ve'o gi vo")
            .expect("semantic JSON");
        let identities = predications_with_relation_and_mode(&json, "identity", "definitional");
        assert_eq!(identities.len(), 2);
        assert_eq!(
            identities[0]["arguments"]["x1"]["value"],
            identities[1]["arguments"]["x1"]["value"]
        );
        let nibli = predication_with_relation_and_mode(&json, "nibli", "asserted");
        assert_eq!(nibli["arguments"]["x1"]["value"], "formula:f1");
        assert_eq!(nibli["arguments"]["x2"]["value"], "formula:f2");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["children"].as_array().expect("children").len(), 3);
        assert_eq!(content["connector"]["source"], "ni'i gi");
        assert_eq!(content["connector"]["locus"], "operand");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn uniform_tanru_reifies_seltau_as_property_modifier() {
        let json = semantic_json_for("ti barda nanla").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f4");
        assert_eq!(object(&json, "predication:p1")["relation"], "nanla");
        assert_eq!(object(&json, "predication:p2")["relation"], "barda");
        assert_eq!(object(&json, "predication:p2")["mode"], "restrictive");
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x1"]["value"],
            "parameter:p1"
        );
        assert_eq!(
            object(&json, "abstraction:a1")["abstractionKind"],
            "property"
        );
        assert_eq!(object(&json, "abstraction:a1")["arity"], 1);
        assert_eq!(object(&json, "abstraction:a1")["body"], "formula:f2");
        assert_eq!(
            object(&json, "abstraction:a1")["parameters"][0],
            "parameter:p1"
        );
        assert_eq!(
            object(&json, "predication:p3")["relation"],
            "R[tanru:barda-nanla]"
        );
        assert_eq!(
            object(&json, "predication:p3")["arguments"]["x2"]["value"],
            "abstraction:a1"
        );
        assert_eq!(object(&json, "formula:f4")["operator"], "and");
        assert_eq!(object(&json, "formula:f4")["children"][0], "formula:f1");
        assert_eq!(object(&json, "formula:f4")["children"][1], "formula:f3");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_description_reifies_body_formula() {
        let json = semantic_json_for("mi klama le zarci .i mi nelci le si'o mi go'i")
            .expect("semantic JSON");
        let concept_link = predication_with_relation_and_mode(&json, "conceptOf", "restrictive");
        let concept = concept_link["arguments"]["x1"]["value"]
            .as_str()
            .expect("concept referent");
        let abstraction = concept_link["arguments"]["x2"]["value"]
            .as_str()
            .expect("concept abstraction");
        assert_eq!(object(&json, concept)["sort"], "concept");
        assert_eq!(object(&json, abstraction)["abstractionKind"], "concept");

        let body = object(&json, abstraction)["body"]
            .as_str()
            .expect("abstraction body");
        let body_predication = object(&json, object(&json, body)["predication"].as_str().unwrap());
        assert_eq!(body_predication["relation"], "klama");
        assert_eq!(body_predication["mode"], "inert");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn converted_duhu_description_describes_text_expressing_proposition() {
        let json = semantic_json_for("la .djan. pu cusku le se du'u la .djordj. ca klama le zarci")
            .expect("semantic JSON");

        let sentence_link =
            predication_with_relation_and_mode(&json, "sentenceExpresses", "restrictive");
        let text = sentence_link["arguments"]["x1"]["value"]
            .as_str()
            .expect("text referent");
        let abstraction = sentence_link["arguments"]["x2"]["value"]
            .as_str()
            .expect("proposition abstraction");
        assert_eq!(object(&json, text)["sort"], "text");
        assert_eq!(object(&json, abstraction)["abstractionKind"], "proposition");

        let cusku = predication_with_relation_and_mode(&json, "cusku", "asserted");
        let cusku_event = cusku["eventuality"].as_str().expect("cusku event");
        let klama = predication_with_relation_and_mode(&json, "klama", "inert");
        let klama_event = object(&json, klama["eventuality"].as_str().expect("klama event"));
        assert_eq!(klama_event["time"]["relation"], "at");
        assert_eq!(klama_event["time"]["anchor"], cusku_event);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ka_description_records_distinct_cehu_parameters() {
        let json = semantic_json_for("le ka ce'u prami ce'u").expect("semantic JSON");
        let property_link = predication_with_relation_and_mode(&json, "propertyOf", "restrictive");
        let abstraction = property_link["arguments"]["x2"]["value"]
            .as_str()
            .expect("property abstraction");
        let abstraction_object = object(&json, abstraction);
        assert_eq!(abstraction_object["abstractionKind"], "property");
        assert_eq!(abstraction_object["arity"], 2);
        let parameters = abstraction_object["parameters"]
            .as_array()
            .expect("abstraction parameters");
        assert_ne!(parameters[0], parameters[1]);
        let prami = predication_with_relation_and_mode(&json, "prami", "restrictive");
        assert_eq!(prami["arguments"]["x1"]["value"], parameters[0]);
        assert_eq!(prami["arguments"]["x2"]["value"], parameters[1]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ka_without_cehu_uses_first_omitted_place_as_property_slot() {
        let loved_json = semantic_json_for("le ka mi prami").expect("semantic JSON");
        let loved_abstraction = object(&loved_json, "abstraction:a1");
        assert_eq!(loved_abstraction["abstractionKind"], "property");
        assert_eq!(loved_abstraction["arity"], 1);
        assert_eq!(loved_abstraction["parameters"][0], "parameter:p1");
        let loved = predication_with_relation_and_mode(&loved_json, "prami", "restrictive");
        assert_eq!(loved["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(loved["arguments"]["x2"]["value"], "parameter:p1");

        let lover_json = semantic_json_for("le ka prami mi").expect("semantic JSON");
        let lover_abstraction = object(&lover_json, "abstraction:a1");
        assert_eq!(lover_abstraction["arity"], 1);
        assert_eq!(lover_abstraction["parameters"][0], "parameter:p1");
        let lover = predication_with_relation_and_mode(&lover_json, "prami", "restrictive");
        assert_eq!(lover["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(lover["arguments"]["x2"]["value"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn implicit_ka_slot_uses_converted_visible_place_order() {
        let json = semantic_json_for("le ka se risna").expect("semantic JSON");
        let abstraction = object(&json, "abstraction:a1");
        assert_eq!(abstraction["abstractionKind"], "property");
        assert_eq!(abstraction["arity"], 1);
        assert_eq!(abstraction["parameters"][0], "parameter:p1");

        let risna = predication_with_relation_and_mode(&json, "risna", "restrictive");
        assert_eq!(risna["arguments"]["x1"]["kind"], "elided");
        assert_eq!(risna["arguments"]["x2"]["value"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recursive_nei_pro_bridi_elides_self_containing_argument() {
        let json = semantic_json_for("do pensi le nu do pensi le nu nei").expect("semantic JSON");
        let nei_copy = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| {
                object["type"] == "predication"
                    && object["relation"] == "pensi"
                    && object["source"]["text"] == "nei"
            })
            .expect("nei copy predication");
        assert_eq!(nei_copy["arguments"]["x2"]["kind"], "elided");
        assert!(
            nei_copy["diagnostics"]
                .as_array()
                .expect("recursive diagnostic")
                .iter()
                .any(|diagnostic| diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("recursive inherited pro-bridi")))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_single_tanru_unit_uses_semantic_lowering() {
        let json = semantic_json_for("mi na'e cadzu").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&json, "cadzu", "asserted");
        assert_eq!(cadzu["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(cadzu["scalarNegation"]["kind"], "otherThan");
        assert_eq!(cadzu["scalarNegation"]["introducedBy"], "na'e");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_seltau_does_not_negate_tertau() {
        let json =
            semantic_json_for("la .alis. cu na'e cadzu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        assert!(klama.get("scalarNegation").is_none());

        let cadzu = predication_with_relation_and_mode(&json, "cadzu", "restrictive");
        assert_eq!(cadzu["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(cadzu["scalarNegation"]["kind"], "otherThan");
        assert_eq!(cadzu["scalarNegation"]["introducedBy"], "na'e");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_group_preserves_linked_places_and_omitted_terminator_scope() {
        let grouped =
            semantic_json_for("mi na'e ke sutra cadzu be fi le birka ke'e klama le zarci")
                .expect("semantic JSON");
        let sutra_cadzu =
            predication_with_relation_and_mode(&grouped, "sutra cadzu", "restrictive");
        assert_eq!(sutra_cadzu["arguments"]["x3"]["kind"], "filled");
        assert_eq!(sutra_cadzu["scalarNegation"]["kind"], "otherThan");
        assert_eq!(sutra_cadzu["scalarNegation"]["introducedBy"], "na'e");

        let omitted =
            semantic_json_for("mi na'e ke sutra bo cadzu be fi le birka je masno klama le zarci")
                .expect("semantic JSON");
        let whole_group =
            predication_with_relation_and_mode(&omitted, "sutra bo cadzu", "asserted");
        assert_eq!(whole_group["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(whole_group["arguments"]["x4"]["kind"], "filled");
        assert_eq!(whole_group["scalarNegation"]["kind"], "otherThan");
        assert_eq!(whole_group["scalarNegation"]["introducedBy"], "na'e");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inverted_tanru_lowers_to_same_tertau_and_modifier_shape() {
        let json = semantic_json_for("ta zdani co blanu").expect("semantic JSON");
        let zdani = predication_with_relation_and_mode(&json, "zdani", "asserted");
        assert_eq!(zdani["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(zdani["arguments"]["x2"]["kind"], "elided");

        let blanu = predication_with_relation_and_mode(&json, "blanu", "restrictive");
        assert_eq!(blanu["arguments"]["x1"]["value"], "parameter:p1");

        let relation =
            predication_with_relation_and_mode(&json, "R[tanru:blanu-zdani]", "asserted");
        assert_eq!(relation["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(relation["arguments"]["x2"]["value"], "abstraction:a1");
        assert_eq!(
            object(&json, "formula:f4")["connector"]["locus"],
            "selbri-inversion"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inverted_tanru_tail_terms_fill_seltau_places() {
        let json = semantic_json_for("mi troci co klama le zarci le zdani").expect("semantic JSON");
        let troci = predication_with_relation_and_mode(&json, "troci", "asserted");
        assert_eq!(troci["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(troci["arguments"]["x2"]["kind"], "elided");
        assert_eq!(troci["arguments"]["x3"]["kind"], "elided");

        let klama = predication_with_relation_and_mode(&json, "klama", "restrictive");
        assert_eq!(klama["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(klama["arguments"]["x2"]["kind"], "filled");
        assert_eq!(klama["arguments"]["x3"]["kind"], "filled");
        assert_eq!(klama["arguments"]["x4"]["kind"], "elided");
        assert_eq!(klama["arguments"]["x5"]["kind"], "elided");

        let relation =
            predication_with_relation_and_mode(&json, "R[tanru:klama-troci]", "asserted");
        assert_eq!(relation["arguments"]["x1"]["value"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multiple_inverted_tanru_lower_in_non_inverted_order() {
        let json = semantic_json_for("ckule co nixli co cmalu").expect("semantic JSON");
        let relations = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter(|object| object["type"] == "predication")
            .filter_map(|object| object["relation"].as_str())
            .collect::<Vec<_>>();
        assert!(relations.iter().any(|relation| *relation == "ckule"));
        assert!(relations.iter().any(|relation| *relation == "nixli"));
        assert!(relations.iter().any(|relation| *relation == "cmalu"));
        assert!(
            relations
                .iter()
                .any(|relation| *relation == "R[tanru:cmalu-nixli]")
        );
        assert!(
            relations
                .iter()
                .any(|relation| *relation == "R[tanru:cmalu-nixli-ckule]")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn number_sumti_and_nuha_operator_selbri_are_explicit() {
        let json = semantic_json_for("li vo nu'a su'i li re li re").expect("semantic JSON");
        assert_eq!(object(&json, "referent:r1")["sort"], "number");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["quantity"],
            "quantity:q1"
        );
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 4);
        let sum = predication_with_relation_and_mode(&json, "nu'a su'i", "asserted");
        assert_eq!(sum["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(sum["arguments"]["x2"]["value"], "referent:r2");
        assert_eq!(sum["arguments"]["x3"]["value"], "referent:r3");
        assert!(sum.get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn meho_sumti_mentions_math_expression_sign() {
        let json = semantic_json_for("me'o re su'i re").expect("semantic JSON");
        let utterance = root_object(&json);
        assert_eq!(utterance["content"], "sign:s1");
        let sign = object(&json, "sign:s1");
        assert_eq!(sign["kind"], "mathExpression");
        assert_eq!(sign["text"], "re su'i re");
        assert_eq!(sign["denotes"], "math:m3");
        assert_eq!(object(&json, "math:m3")["operator"], "add");
        assert_eq!(object(&json, "math:m3")["operands"][0], "math:m1");
        assert_eq!(object(&json, "math:m1")["literal"]["value"], 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complex_li_preserves_math_expression_tree() {
        let json =
            semantic_json_for("li .abu bi'epi'i xy. bi'ete'a re su'i by. bi'epi'i xy. su'i cy.")
                .expect("semantic JSON");
        assert_eq!(object(&json, "referent:r1")["sort"], "number");
        assert_eq!(
            object(&json, "quantity:q1")["value"]["mathExpression"],
            "math:m11"
        );
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["name"],
            "a pi'i x te'a re su'i b pi'i x su'i c"
        );
        assert_eq!(object(&json, "math:m11")["operator"], "add");
        assert_eq!(object(&json, "math:m5")["operator"], "multiply");
        assert_eq!(object(&json, "math:m4")["operator"], "power");
        assert_eq!(object(&json, "math:m1")["literal"]["value"], "a");
        assert_eq!(object(&json, "math:m2")["literal"]["value"], "x");
        assert_eq!(object(&json, "math:m6")["literal"]["value"], "b");
        assert_eq!(object(&json, "math:m10")["literal"]["value"], "c");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mohe_sumti_operand_preserves_referent_inside_math_expression() {
        let json =
            semantic_json_for("li pa vu'u mo'e le ni le pixra cu blanu").expect("semantic JSON");
        assert_eq!(
            object(&json, "quantity:q1")["value"]["mathExpression"],
            "math:m3"
        );
        assert_eq!(object(&json, "math:m3")["operator"], "subtract");
        assert_eq!(object(&json, "math:m3")["operands"][1], "math:m2");
        assert_eq!(object(&json, "math:m2")["literal"]["kind"], "sumtiOperand");
        assert_eq!(object(&json, "math:m2")["denotes"], "referent:r1");
        assert_eq!(object(&json, "referent:r1")["sort"], "amount");
        let amount_of = predication_with_relation_and_mode(&json, "amountOf", "restrictive");
        assert_eq!(amount_of["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(amount_of["arguments"]["x2"]["value"], "abstraction:a1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn moi_selbri_preserve_marker_and_place_structure() {
        let ordinal = semantic_json_for("la .prim. .palvr. pamoi cusku").expect("semantic JSON");
        let pamoi = predication_with_relation_and_mode(&ordinal, "pa moi", "restrictive");
        assert_eq!(pamoi["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(pamoi["arguments"]["x2"]["kind"], "elided");
        assert_eq!(pamoi["arguments"]["x3"]["kind"], "elided");
        assert!(pamoi.get("diagnostics").is_none());

        let cardinal =
            semantic_json_for("la .anis. joi la .asun. bruna remei").expect("semantic JSON");
        let remei = predication_with_relation_and_mode(&cardinal, "re mei", "asserted");
        assert_eq!(remei["arguments"]["x1"]["value"], "referent:r3");
        assert_eq!(remei["arguments"]["x2"]["kind"], "elided");
        assert_eq!(remei["arguments"]["x3"]["kind"], "elided");
        assert!(remei.get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_tanru_unit_preserves_embedded_relation_label() {
        let json = semantic_json_for("ti nu zdile kei kumfa").expect("semantic JSON");
        let event_property = predication_with_relation_and_mode(&json, "eventOf", "restrictive");
        assert_eq!(object(&json, "parameter:p1")["sort"], "eventuality");
        assert_eq!(event_property["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(event_property["arguments"]["x2"]["value"], "abstraction:a1");
        assert!(event_property.get("diagnostics").is_none());
        let tanru =
            predication_with_relation_and_mode(&json, "R[tanru:nu zdile-kumfa]", "asserted");
        assert_eq!(tanru["arguments"]["x2"]["value"], "abstraction:a2");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_abstraction_selbri_reifies_body_formula() {
        let json = semantic_json_for("nu mi klama le zarci").expect("semantic JSON");
        let event_of = predication_with_relation_and_mode(&json, "eventOf", "asserted");
        let event = event_of["arguments"]["x1"]["value"]
            .as_str()
            .expect("event x1");
        assert_eq!(object(&json, event)["sort"], "eventuality");
        assert_eq!(event_of["arguments"]["x2"]["value"], "abstraction:a1");
        assert_eq!(object(&json, "abstraction:a1")["abstractionKind"], "event");
        let klama = predication_with_relation_and_mode(&json, "klama", "inert");
        assert_eq!(klama["arguments"]["x1"]["value"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn process_abstraction_exposes_stage_place() {
        let json = semantic_json_for("le pu'u mi klama").expect("semantic JSON");
        let process_of = predication_with_relation_and_mode(&json, "processOf", "restrictive");
        assert_eq!(object(&json, "referent:r1")["sort"], "eventuality");
        assert_eq!(process_of["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(process_of["arguments"]["x2"]["value"], "abstraction:a1");
        assert_eq!(process_of["arguments"]["x3"]["kind"], "elided");
        assert_eq!(process_of["arguments"]["x3"]["introducedBy"], "zo'e");
        assert_eq!(
            object(&json, "abstraction:a1")["abstractionKind"],
            "process"
        );
        let klama = predication_with_relation_and_mode(&json, "klama", "inert");
        assert_eq!(klama["arguments"]["x1"]["value"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_abstractors_build_connected_link_formula() {
        let json = semantic_json_for("le mikce cu se cinri le pu'u jenai za'i mi sipna")
            .expect("semantic JSON");
        let process_of = predication_with_relation_and_mode(&json, "processOf", "restrictive");
        let state_of = predication_with_relation_and_mode(&json, "stateOf", "restrictive");
        let described_eventuality = process_of["arguments"]["x1"]["value"]
            .as_str()
            .expect("process described eventuality");
        assert_eq!(state_of["arguments"]["x1"]["value"], described_eventuality);
        assert_eq!(process_of["arguments"]["x3"]["kind"], "elided");

        let description = object(&json, described_eventuality);
        let body = object(
            &json,
            description["descriptor"]["body"]
                .as_str()
                .expect("description body"),
        );
        assert_eq!(body["operator"], "and");
        assert_eq!(body["connector"]["source"], "je nai");
        assert_eq!(body["connector"]["locus"], "abstraction");
        let children = body["children"].as_array().expect("connected children");
        let process_formula = object(&json, children[0].as_str().expect("process formula"));
        let process_predication = object(
            &json,
            process_formula["predication"]
                .as_str()
                .expect("process predication"),
        );
        assert_eq!(process_predication["relation"], "processOf");
        let negation = object(&json, children[1].as_str().expect("negated state formula"));
        assert_eq!(negation["operator"], "not");
        let negated_child = object(
            &json,
            negation["children"][0]
                .as_str()
                .expect("state formula child"),
        );
        let state_predication = object(
            &json,
            negated_child["predication"]
                .as_str()
                .expect("state predication"),
        );
        assert_eq!(state_predication["relation"], "stateOf");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn minor_abstraction_links_expose_second_place() {
        for (source, relation, kind) in [
            ("mi morji le li'i mi verba", "experienceOf", "experience"),
            (
                "mi nelci le si'o la .lojban. cu mulno",
                "conceptOf",
                "concept",
            ),
            (
                "ko zgana le su'u le ci smacu cu bajra",
                "abstractionOf",
                "unspecified",
            ),
        ] {
            let json = semantic_json_for(source).expect("semantic JSON");
            let link = predication_with_relation_and_mode(&json, relation, "restrictive");
            let abstraction = link["arguments"]["x2"]["value"]
                .as_str()
                .expect("abstraction argument");
            assert_eq!(object(&json, abstraction)["abstractionKind"], kind);
            assert_eq!(link["arguments"]["x3"]["kind"], "elided");
            assert_eq!(link["arguments"]["x3"]["introducedBy"], "zo'e");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn linked_suhu_description_fills_abstraction_type_place() {
        let json = semantic_json_for("le su'u mi klama kei be lo fasnu").expect("semantic JSON");
        let abstraction_of =
            predication_with_relation_and_mode(&json, "abstractionOf", "restrictive");
        assert_eq!(abstraction_of["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(abstraction_of["arguments"]["x2"]["value"], "abstraction:a1");
        assert_eq!(abstraction_of["arguments"]["x3"]["kind"], "filled");
        let type_referent = abstraction_of["arguments"]["x3"]["value"]
            .as_str()
            .expect("type referent");
        assert_eq!(
            object(&json, type_referent)["descriptor"]["kind"],
            "veridicalDescription"
        );
        assert_eq!(
            object(&json, "abstraction:a1")["abstractionKind"],
            "unspecified"
        );
        assert!(
            !predication_relations(&json)
                .iter()
                .any(|relation| relation == "su'u klama")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tanru_inside_description_uses_uniform_lowering() {
        let json = semantic_json_for("mi jimpe tu'a loi nu'a su'i nabmi").expect("semantic JSON");
        assert_eq!(object(&json, "referent:r1")["sort"], "mass");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["kind"],
            "veridicalMassDescription"
        );
        let nabmi = predication_with_relation_and_mode(&json, "nabmi", "restrictive");
        assert_eq!(nabmi["arguments"]["x1"]["value"], "referent:r1");
        let operator = predication_with_relation_and_mode(&json, "nu'a su'i", "restrictive");
        assert_eq!(operator["arguments"]["x1"]["value"], "parameter:p1");
        let tanru =
            predication_with_relation_and_mode(&json, "R[tanru:nu'a su'i-nabmi]", "restrictive");
        assert_eq!(tanru["arguments"]["x1"]["value"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn me_sumti_selbri_preserves_source_referent() {
        let json = semantic_json_for("do me la .djan.").expect("semantic JSON");
        let referent_of = predication_with_relation_and_mode(&json, "referentOf", "asserted");
        assert_eq!(
            referent_of["arguments"]["x1"]["value"],
            "referent:addressee"
        );
        let source = referent_of["arguments"]["x2"]["value"]
            .as_str()
            .expect("source referent id");
        assert_eq!(object(&json, source)["descriptor"]["name"], "djan");
        assert!(referent_of.get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn me_sumti_seltau_preserves_lai_mass_name() {
        let json = semantic_json_for("ta me lai .kraislr. karce").expect("semantic JSON");
        let referent_of = predication_with_relation_and_mode(&json, "referentOf", "restrictive");
        let source = referent_of["arguments"]["x2"]["value"]
            .as_str()
            .expect("source referent id");
        assert_eq!(object(&json, source)["sort"], "mass");
        assert_eq!(object(&json, source)["descriptor"]["kind"], "massName");
        assert_eq!(object(&json, source)["descriptor"]["word"], "lai");
        assert_eq!(object(&json, source)["descriptor"]["name"], "kraislr");
        let tanru =
            predication_with_relation_and_mode(&json, "R[tanru:referentOf-karce]", "asserted");
        assert_eq!(tanru["arguments"]["x2"]["value"], "abstraction:a1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lai_selbri_description_is_mass_name_description() {
        let json = semantic_json_for("lai cribe cu finti").expect("semantic JSON");
        let referent = object(&json, "referent:r1");
        assert_eq!(referent["sort"], "mass");
        assert_eq!(referent["descriptor"]["kind"], "massNameDescription");
        assert_eq!(referent["descriptor"]["word"], "lai");
        assert_eq!(referent["descriptor"]["body"], "formula:f1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lahe_qualified_sumti_preserves_operand_referent() {
        let json =
            semantic_json_for("ta me la'e le se cusku be do me'u cukta").expect("semantic JSON");
        let referent_of = predication_with_relation_and_mode(&json, "referentOf", "restrictive");
        let source = referent_of["arguments"]["x2"]["value"]
            .as_str()
            .expect("source referent id");
        assert_eq!(
            object(&json, source)["descriptor"]["kind"],
            "referentOfSymbol"
        );
        assert_eq!(object(&json, source)["descriptor"]["word"], "la'e");
        let operand = object(&json, source)["descriptor"]["operand"]
            .as_str()
            .expect("operand referent id");
        assert_eq!(object(&json, operand)["descriptor"]["word"], "le");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_sumti_qualifier_preserves_operand_referent() {
        let json = semantic_json_for("mi viska na'ebo le gerku").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&json, "viska", "asserted");
        let qualified = viska["arguments"]["x2"]["value"]
            .as_str()
            .expect("qualified referent id");
        let descriptor = &object(&json, qualified)["descriptor"];
        assert_eq!(descriptor["kind"], "otherThan");
        assert_eq!(descriptor["word"], "na'e bo");
        let operand = descriptor["operand"].as_str().expect("operand referent id");
        assert_eq!(object(&json, operand)["descriptor"]["word"], "le");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ri_resolves_inside_luhe_member_qualifier() {
        let json =
            semantic_json_for("lo'i ratcu cu barda .iku'i lu'a ri cmalu").expect("semantic JSON");
        let barda = predication_with_relation_and_mode(&json, "barda", "asserted");
        let rat_set = barda["arguments"]["x1"]["value"]
            .as_str()
            .expect("rat-set referent id");
        let cmalu = predication_with_relation_and_mode(&json, "cmalu", "asserted");
        let member = cmalu["arguments"]["x1"]["value"]
            .as_str()
            .expect("member referent id");
        let descriptor = &object(&json, member)["descriptor"];
        assert_eq!(descriptor["kind"], "memberOf");
        assert_eq!(descriptor["operand"], rat_set);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_sumti_qualifier_handles_resolved_and_vague_anaphora() {
        let json = semantic_json_for(
            "mi nelci loi glare cidja .ije do nelci to'ebo ri .ije la .djein. nelci no'ebo ra",
        )
        .expect("semantic JSON");
        let opposite = referent_with_descriptor_kind(&json, "oppositeOf");
        assert_eq!(opposite["sort"], "mass");
        let opposite_operand = opposite["descriptor"]["operand"]
            .as_str()
            .expect("opposite operand");
        assert_eq!(
            object(&json, opposite_operand)["descriptor"]["kind"],
            "veridicalMassDescription"
        );
        let neutral = referent_with_descriptor_kind(&json, "neutralOf");
        let neutral_operand = neutral["descriptor"]["operand"]
            .as_str()
            .expect("neutral operand");
        assert_eq!(
            object(&json, neutral_operand)["descriptor"]["kind"],
            "proSumti"
        );
        assert_eq!(object(&json, neutral_operand)["descriptor"]["word"], "ra");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mehu_changes_sumti_connection_scope() {
        let no_mehu = semantic_json_for("re me le ci nolraitru .e la .djan. cu blabi")
            .expect("semantic JSON");
        let source =
            predication_with_relation_and_mode(&no_mehu, "referentOf", "restrictive")["arguments"]
                ["x2"]["value"]
                .as_str()
                .expect("source referent id")
                .to_owned();
        assert_eq!(object(&no_mehu, &source)["category"], "composite");
        let no_mehu_blabi = no_mehu["objects"]
            .as_object()
            .expect("objects")
            .values()
            .filter(|object| object["type"] == "predication" && object["relation"] == "blabi")
            .count();
        assert_eq!(no_mehu_blabi, 1);

        let with_mehu = semantic_json_for("re me le ci nolraitru me'u .e la .djan. cu blabi")
            .expect("semantic JSON");
        let with_mehu_blabi = with_mehu["objects"]
            .as_object()
            .expect("objects")
            .values()
            .filter(|object| object["type"] == "predication" && object["relation"] == "blabi")
            .count();
        assert_eq!(with_mehu_blabi, 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn linked_sumti_fill_main_selbri_places() {
        let json =
            semantic_json_for("mi klama be le zarci bei le zdani be'o").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        assert_eq!(klama["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(klama["arguments"]["x2"]["kind"], "filled");
        assert_eq!(klama["arguments"]["x2"]["value"], "referent:r1");
        assert_eq!(klama["arguments"]["x3"]["kind"], "filled");
        assert_eq!(klama["arguments"]["x3"]["value"], "referent:r4");
        assert_eq!(klama["arguments"]["x4"]["kind"], "elided");
        assert_eq!(klama["arguments"]["x5"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn linked_sumti_fill_tanru_modifier_places() {
        let json = semantic_json_for("ti xamgu be do bei mi be'o zdani").expect("semantic JSON");
        let xamgu = predication_with_relation_and_mode(&json, "xamgu", "restrictive");
        assert_eq!(xamgu["arguments"]["x1"]["value"], "parameter:p1");
        assert_eq!(xamgu["arguments"]["x2"]["kind"], "filled");
        assert_eq!(xamgu["arguments"]["x2"]["value"], "referent:addressee");
        assert_eq!(xamgu["arguments"]["x3"]["kind"], "filled");
        assert_eq!(xamgu["arguments"]["x3"]["value"], "referent:speaker");

        let fa_ordered =
            semantic_json_for("ti xamgu be fi mi bei fe do be'o zdani").expect("semantic JSON");
        let xamgu = predication_with_relation_and_mode(&fa_ordered, "xamgu", "restrictive");
        assert_eq!(xamgu["arguments"]["x2"]["value"], "referent:addressee");
        assert_eq!(xamgu["arguments"]["x3"]["value"], "referent:speaker");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_linkargs_preserve_modifier_vs_bridi_scope() {
        let linked = semantic_json_for("ta blanu be ga'a mi be'o zdani").expect("semantic JSON");
        let blanu = predication_with_relation_and_mode(&linked, "blanu", "restrictive");
        assert_eq!(blanu["modalArguments"][0]["relation"], "zgana");
        assert_eq!(blanu["modalArguments"][0]["introducedBy"], "ga'a");
        assert_eq!(
            blanu["modalArguments"][0]["arguments"]["x1"]["kind"],
            "filled"
        );
        assert_eq!(
            blanu["modalArguments"][0]["arguments"]["x1"]["value"],
            "referent:speaker"
        );
        assert_eq!(
            blanu["modalArguments"][0]["arguments"]["x2"]["kind"],
            "elided"
        );
        assert_eq!(
            blanu["modalArguments"][0]["arguments"]["x3"]["kind"],
            "elided"
        );
        assert_eq!(
            blanu["modalArguments"][0]["arguments"]["x4"]["kind"],
            "elided"
        );
        let zdani = predication_with_relation_and_mode(&linked, "zdani", "asserted");
        assert!(zdani.get("modalArguments").is_none());

        let tail = semantic_json_for("ta blanu zdani ga'a mi").expect("semantic JSON");
        let zdani = predication_with_relation_and_mode(&tail, "zdani", "asserted");
        assert_eq!(zdani["modalArguments"][0]["relation"], "zgana");
        assert_eq!(
            zdani["modalArguments"][0]["arguments"]["x1"]["value"],
            "referent:speaker"
        );
        let blanu = predication_with_relation_and_mode(&tail, "blanu", "restrictive");
        assert!(blanu.get("modalArguments").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ad_hoc_modal_uses_tag_selbri_place_structure() {
        let kanla =
            semantic_json_for("mi viska do fi'o kanla fe'u le zunle").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&kanla, "viska", "asserted");
        let modal = &viska["modalArguments"][0];
        assert_eq!(modal["introducedBy"], "fi'o");
        assert_eq!(modal["relation"], "kanla");
        assert_eq!(modal["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(modal["arguments"]["x2"]["kind"], "elided");

        let pilno =
            semantic_json_for("mi viska do fi'o se pilno le zunle kanla").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&pilno, "viska", "asserted");
        let modal = &viska["modalArguments"][0];
        assert_eq!(modal["introducedBy"], "fi'o");
        assert_eq!(modal["relation"], "pilno");
        assert_eq!(modal["arguments"]["x1"]["kind"], "elided");
        assert_eq!(modal["arguments"]["x2"]["value"], "referent:r1");
        assert_eq!(modal["arguments"]["x3"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn converted_seltau_property_fills_all_known_places() {
        let json = semantic_json_for("mi se bapli tavla").expect("semantic JSON");
        let bapli = predication_with_relation_and_mode(&json, "bapli", "restrictive");
        assert_eq!(bapli["arguments"]["x1"]["kind"], "elided");
        assert_eq!(bapli["arguments"]["x2"]["kind"], "filled");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pre_selbri_modals_are_modal_arguments() {
        let bai = semantic_json_for("mi bai tavla").expect("semantic JSON");
        let tavla = predication_with_relation_and_mode(&bai, "tavla", "asserted");
        let modal = &tavla["modalArguments"][0];
        assert_eq!(modal["relation"], "bapli");
        assert_eq!(modal["introducedBy"], "bai");
        assert_eq!(modal["arguments"]["x1"]["kind"], "elided");
        assert_eq!(modal["arguments"]["x2"]["kind"], "elided");

        let fiho = semantic_json_for("mi fi'o kanla fe'u viska do").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&fiho, "viska", "asserted");
        let modal = &viska["modalArguments"][0];
        assert_eq!(modal["relation"], "kanla");
        assert_eq!(modal["introducedBy"], "fi'o");
        assert_eq!(modal["arguments"]["x1"]["kind"], "elided");
        assert_eq!(modal["arguments"]["x2"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_group_modal_scopes_over_nested_assertions() {
        let json = semantic_json_for("bai tu'e mi klama le zarci .i mi cadzu le bisli tu'u")
            .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let cadzu = predication_with_relation_and_mode(&json, "cadzu", "asserted");
        assert_eq!(klama["modalArguments"][0]["relation"], "bapli");
        assert_eq!(cadzu["modalArguments"][0]["relation"], "bapli");
        assert_eq!(
            klama["modalArguments"][0]["arguments"]["x1"]["value"],
            cadzu["modalArguments"][0]["arguments"]["x1"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn relative_clause_attachment_preserves_beho_scope() {
        let linked = semantic_json_for("le xamgu be do noi barda cu zdani").expect("semantic JSON");
        let xamgu = predication_with_relation_and_mode(&linked, "xamgu", "restrictive");
        assert_eq!(xamgu["arguments"]["x2"]["value"], "referent:addressee");
        assert_eq!(
            xamgu["arguments"]["x2"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        assert_eq!(
            xamgu["arguments"]["x2"]["relativeClauses"][0]["body"],
            "formula:f1"
        );
        let barda = predication_with_relation_and_mode(&linked, "barda", "incidental");
        assert_eq!(barda["arguments"]["x1"]["value"], "referent:addressee");

        let outer =
            semantic_json_for("le xamgu be do be'o noi barda cu zdani").expect("semantic JSON");
        let xamgu = predication_with_relation_and_mode(&outer, "xamgu", "restrictive");
        assert!(xamgu["arguments"]["x2"].get("relativeClauses").is_none());
        let zdani = predication_with_relation_and_mode(&outer, "zdani", "asserted");
        let outer_head = zdani["arguments"]["x1"]["value"]
            .as_str()
            .expect("outer description head");
        assert_eq!(
            object(&outer, outer_head)["descriptor"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        assert_eq!(
            object(&outer, outer_head)["descriptor"]["relativeClauses"][0]["body"],
            "formula:f2"
        );
        assert!(zdani["arguments"]["x1"].get("relativeClauses").is_none());
        let barda = predication_with_relation_and_mode(&outer, "barda", "incidental");
        assert_eq!(
            barda["arguments"]["x1"]["value"],
            zdani["arguments"]["x1"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantified_da_series_wraps_formula_scope() {
        let json =
            semantic_json_for("ro da poi prenu cu prami pa de poi finpe").expect("semantic JSON");
        let root = root_object(&json);
        assert_eq!(root["content"], "formula:f5");

        let universal = object(&json, "formula:f5");
        assert_eq!(universal["operator"], "forall");
        assert_eq!(universal["variable"], "referent:r1");
        assert_eq!(universal["restriction"], "formula:f1");
        assert_eq!(universal["body"], "formula:f4");
        assert_eq!(universal["quantity"], "quantity:q1");
        assert_eq!(object(&json, "quantity:q1")["form"], "all");

        let exact_one = object(&json, "formula:f4");
        assert_eq!(exact_one["operator"], "cardinality");
        assert_eq!(exact_one["variable"], "referent:r2");
        assert_eq!(exact_one["restriction"], "formula:f2");
        assert_eq!(exact_one["body"], "formula:f3");
        assert_eq!(exact_one["quantity"], "quantity:q2");
        assert_eq!(object(&json, "quantity:q2")["value"]["integer"], 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_da_series_introduces_implicit_existential_scope_once() {
        let json = semantic_json_for("la djan cu lafti da poi grana ku'o gi'e desku da")
            .expect("semantic JSON");
        let root = root_object(&json);
        assert_eq!(root["content"], "formula:f5");

        let exists = object(&json, "formula:f5");
        assert_eq!(exists["operator"], "exists");
        assert_eq!(exists["variable"], "referent:r1");
        assert_eq!(exists["restriction"], "formula:f1");
        assert_eq!(exists["body"], "formula:f4");
        assert!(exists.get("quantity").is_none());

        let lafti = predication_with_relation_and_mode(&json, "lafti", "asserted");
        assert_eq!(lafti["arguments"]["x2"]["value"], "referent:r1");
        let desku = predication_with_relation_and_mode(&json, "desku", "asserted");
        assert_eq!(desku["arguments"]["x2"]["value"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_da_series_uses_explicit_scope_without_inner_implicit_duplicate() {
        let json = semantic_json_for("da zo'u da viska mi").expect("semantic JSON");
        let root = root_object(&json);
        let exists_id = root["content"].as_str().expect("root content");
        let exists = object(&json, exists_id);
        assert_eq!(exists["operator"], "exists");
        assert_eq!(exists["variable"], "referent:r1");
        assert_eq!(exists["body"], "formula:f1");
        assert!(exists.get("quantity").is_none());

        let exists_count = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter(|object| {
                object["type"] == "formula"
                    && object["operator"] == "exists"
                    && object["variable"] == "referent:r1"
            })
            .count();
        assert_eq!(exists_count, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_multiple_quantifiers_preserve_left_to_right_scope() {
        let json = semantic_json_for("ro da ro de zo'u da prami de").expect("semantic JSON");
        let root = root_object(&json);
        let first = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(first["operator"], "forall");
        assert_eq!(first["variable"], "referent:r1");
        assert_eq!(first["quantity"], "quantity:q1");

        let second = object(&json, first["body"].as_str().expect("first body"));
        assert_eq!(second["operator"], "forall");
        assert_eq!(second["variable"], "referent:r2");
        assert_eq!(second["quantity"], "quantity:q2");
        assert_eq!(
            object(&json, second["body"].as_str().expect("second body"))["operator"],
            "atom"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_negation_scope_precedes_quantifier() {
        let json = semantic_json_for("naku da zo'u da viska mi").expect("semantic JSON");
        let root = root_object(&json);
        let negation = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(negation["operator"], "not");
        let exists_id = negation["children"][0]
            .as_str()
            .expect("negation child formula");
        let exists = object(&json, exists_id);
        assert_eq!(exists["operator"], "exists");
        assert_eq!(exists["variable"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_negation_preserves_position_between_quantifiers() {
        let json =
            semantic_json_for("su'oda poi verba ku'o naku su'ode poi ckule zo'u da klama de")
                .expect("semantic JSON");
        let root = root_object(&json);
        let first = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(first["operator"], "cardinality");
        assert_eq!(first["variable"], "referent:r1");

        let negation = object(&json, first["body"].as_str().expect("first body"));
        assert_eq!(negation["operator"], "not");
        let second = object(
            &json,
            negation["children"][0]
                .as_str()
                .expect("negation child formula"),
        );
        assert_eq!(second["operator"], "cardinality");
        assert_eq!(second["variable"], "referent:r2");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn internal_naku_is_preserved_as_negation_boundary() {
        let json = semantic_json_for("su'o verba naku klama su'o ckule").expect("semantic JSON");
        let root = root_object(&json);
        let negation = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(negation["operator"], "not");
        assert_eq!(negation["source"]["construct"], "bridi-negation-boundary");
        let atom = object(
            &json,
            negation["children"][0]
                .as_str()
                .expect("negation child formula"),
        );
        assert_eq!(atom["operator"], "atom");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn adjacent_internal_naku_boundaries_remain_visible() {
        let json = semantic_json_for("mi naku naku le zarci cu klama").expect("semantic JSON");
        let root = root_object(&json);
        let outer = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(outer["operator"], "not");
        let inner = object(
            &json,
            outer["children"][0]
                .as_str()
                .expect("outer negation child formula"),
        );
        assert_eq!(inner["operator"], "not");
        assert_eq!(outer["source"]["construct"], "bridi-negation-boundary");
        assert_eq!(inner["source"]["construct"], "bridi-negation-boundary");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_relative_clause_becomes_quantifier_restriction() {
        let json = semantic_json_for("da poi prenu zo'u da viska la djim.").expect("semantic JSON");
        let root = root_object(&json);
        let exists = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(exists["operator"], "exists");
        assert_eq!(exists["variable"], "referent:r1");

        let restriction = object(
            &json,
            exists["restriction"]
                .as_str()
                .expect("prenex restriction formula"),
        );
        assert_eq!(restriction["operator"], "atom");
        let prenu = object(
            &json,
            restriction["predication"]
                .as_str()
                .expect("restriction predication"),
        );
        assert_eq!(prenu["relation"], "prenu");
        assert_eq!(prenu["arguments"]["x1"]["value"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_relation_variable_introduces_implicit_existential_scope() {
        let json = semantic_json_for("la djim. bu'a la djan.").expect("semantic JSON");
        let root = root_object(&json);
        let exists = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(exists["operator"], "exists");
        assert_eq!(exists["variable"], "parameter:p1");

        let parameter = object(&json, "parameter:p1");
        assert_eq!(parameter["sort"], "relation");
        assert_eq!(parameter["role"], "relationVariable");
        assert_eq!(parameter["introducedBy"], "bu'a");

        let atom = object(&json, exists["body"].as_str().expect("relation body"));
        let predication = object(&json, atom["predication"].as_str().expect("predication"));
        assert_eq!(predication["relationParameter"], "parameter:p1");
        assert!(predication.get("relation").is_none());
        assert_eq!(predication["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(predication["arguments"]["x2"]["value"], "referent:r2");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prenex_relation_variable_reuses_bound_relation_parameter() {
        let json = semantic_json_for("ro bu'a zo'u la djim. bu'a la djan.").expect("semantic JSON");
        let root = root_object(&json);
        let universal = object(&json, root["content"].as_str().expect("root content"));
        assert_eq!(universal["operator"], "forall");
        assert_eq!(universal["variable"], "parameter:p1");
        assert_eq!(universal["quantity"], "quantity:q1");

        let relation_parameter_count = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter(|object| object["type"] == "parameter" && object["role"] == "relationVariable")
            .count();
        assert_eq!(relation_parameter_count, 1);

        let atom = object(&json, universal["body"].as_str().expect("relation body"));
        let predication = object(&json, atom["predication"].as_str().expect("predication"));
        assert_eq!(predication["relationParameter"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn du_identity_is_definitional_not_ordinary_mintu() {
        let identity = semantic_json_for("ko'a du le nanmu").expect("semantic JSON");
        let identity_predication =
            predication_with_relation_and_mode(&identity, "identity", "definitional");
        assert_eq!(
            identity_predication["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(
            identity_predication["arguments"]["x2"]["value"],
            "referent:r2"
        );

        let sameness = semantic_json_for("ko'a mintu le nanmu").expect("semantic JSON");
        let mintu = predication_with_relation_and_mode(&sameness, "mintu", "asserted");
        assert_eq!(mintu["arguments"]["x1"]["value"], "referent:r1");
        assert_eq!(mintu["arguments"]["x2"]["value"], "referent:r2");
        assert_eq!(mintu["arguments"]["x3"]["kind"], "elided");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_pro_sumti_rafsi_metadata_resolves_bound_referent() {
        let json = semantic_json_for("fo'a goi le kulnrsu'omi .i lo fo'arselsanga")
            .expect("semantic JSON");
        let predication = predication_with_relation_and_mode(&json, "fo'arselsanga", "restrictive");
        assert_eq!(predication["relationMetadata"], "relation:r1");
        let metadata = object(&json, "relation:r1");
        assert_eq!(metadata["relation"], "fo'arselsanga");
        assert_eq!(metadata["sourceWords"][0], "fo'a");
        assert_eq!(metadata["expansion"]["kind"], "lujvo");
        assert_eq!(metadata["expansion"]["sourceWords"][0], "fo'ar");
        assert_eq!(
            metadata["expansion"]["rafsiBindings"][0]["sourceWord"],
            "fo'a"
        );
        assert_eq!(
            metadata["expansion"]["rafsiBindings"][0]["referent"],
            "referent:r1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_keha_relative_clause_reuses_head_in_surface_place() {
        let json = semantic_json_for("mi viska le mlatu ku poi zo'e zbasu ke'a loi slasi")
            .expect("semantic JSON");
        let zbasu = predication_with_relation_and_mode(&json, "zbasu", "restrictive");
        let viska = predication_with_relation_and_mode(&json, "viska", "asserted");
        assert_eq!(zbasu["arguments"]["x1"]["kind"], "elided");
        assert_eq!(
            zbasu["arguments"]["x2"]["value"],
            viska["arguments"]["x2"]["value"]
        );
        assert!(
            json["objects"]
                .as_object()
                .expect("objects")
                .values()
                .all(|object| object["type"] != "parameter")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn omitted_keha_relative_clause_uses_first_unfilled_visible_place() {
        let json = semantic_json_for("tu poi le mlatu pu lacpu cu ratcu").expect("semantic JSON");
        let lacpu = predication_with_relation_and_mode(&json, "lacpu", "restrictive");
        let ratcu = predication_with_relation_and_mode(&json, "ratcu", "asserted");
        assert_ne!(
            lacpu["arguments"]["x1"]["value"],
            ratcu["arguments"]["x1"]["value"]
        );
        assert_eq!(
            lacpu["arguments"]["x2"]["value"],
            ratcu["arguments"]["x1"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn goi_relative_phrases_lower_to_semantic_relations() {
        let pe = semantic_json_for("le stizu pe mi cu blanu").expect("semantic JSON");
        let associated = predication_with_relation_and_mode(&pe, "associatedWith", "restrictive");
        let blanu = predication_with_relation_and_mode(&pe, "blanu", "asserted");
        assert_eq!(associated["arguments"]["x2"]["value"], "referent:speaker");
        assert_eq!(
            associated["arguments"]["x1"]["value"],
            blanu["arguments"]["x1"]["value"]
        );
        let pe_head = blanu["arguments"]["x1"]["value"]
            .as_str()
            .expect("PE head referent");
        assert_eq!(
            object(&pe, pe_head)["descriptor"]["relativeClauses"][0]["kind"],
            "restrictive"
        );
        assert_eq!(
            object(&pe, pe_head)["descriptor"]["relativeClauses"][0]["introducedBy"],
            "pe"
        );
        assert!(blanu["arguments"]["x1"].get("relativeClauses").is_none());

        let ne = semantic_json_for("le gerku ne mi cu batci do").expect("semantic JSON");
        let associated = predication_with_relation_and_mode(&ne, "associatedWith", "incidental");
        let batci = predication_with_relation_and_mode(&ne, "batci", "asserted");
        assert_eq!(
            associated["arguments"]["x1"]["value"],
            batci["arguments"]["x1"]["value"]
        );
        let ne_head = batci["arguments"]["x1"]["value"]
            .as_str()
            .expect("NE head referent");
        assert_eq!(
            object(&ne, ne_head)["descriptor"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        assert_eq!(
            object(&ne, ne_head)["descriptor"]["relativeClauses"][0]["introducedBy"],
            "ne"
        );
        assert!(batci["arguments"]["x1"].get("relativeClauses").is_none());

        let po = semantic_json_for("le stizu po mi cu xunre").expect("semantic JSON");
        let specific =
            predication_with_relation_and_mode(&po, "specificallyAssociatedWith", "restrictive");
        assert_eq!(specific["arguments"]["x2"]["value"], "referent:speaker");

        let pohe = semantic_json_for("le birka po'e mi cu spofu").expect("semantic JSON");
        let intrinsic =
            predication_with_relation_and_mode(&pohe, "intrinsicallyPossessedBy", "restrictive");
        assert_eq!(intrinsic["arguments"]["x2"]["value"], "referent:speaker");

        let pohu =
            semantic_json_for("le gerku po'u le mi pendo cu cinba mi").expect("semantic JSON");
        let identity = predication_with_relation_and_mode(&pohu, "identity", "restrictive");
        let cinba = predication_with_relation_and_mode(&pohu, "cinba", "asserted");
        assert_eq!(
            identity["arguments"]["x1"]["value"],
            cinba["arguments"]["x1"]["value"]
        );

        let nohu = semantic_json_for("le nanmu no'u la .djim. cu terpemci").expect("semantic JSON");
        let identity = predication_with_relation_and_mode(&nohu, "identity", "incidental");
        let terpemci = predication_with_relation_and_mode(&nohu, "terpemci", "asserted");
        assert_eq!(
            identity["arguments"]["x1"]["value"],
            terpemci["arguments"]["x1"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_relative_phrases_lower_to_source_relations() {
        let expressed_by =
            semantic_json_for("la .apasonatas pe cu'u la .artr. .rubnstain. cu se nelci mi")
                .expect("semantic JSON");
        let nelci = predication_with_relation_and_mode(&expressed_by, "nelci", "asserted");
        let cusku = predication_with_relation_and_mode(&expressed_by, "cusku", "restrictive");
        assert_eq!(cusku["introducedBy"], "cu'u");
        assert_eq!(
            cusku["arguments"]["x2"]["value"],
            nelci["arguments"]["x2"]["value"]
        );
        assert_eq!(cusku["arguments"]["x3"]["kind"], "elided");
        assert_eq!(cusku["arguments"]["x4"]["kind"], "elided");
        assert_eq!(
            nelci["arguments"]["x2"]["relativeClauses"][0]["introducedBy"],
            "pe"
        );

        let created_by = semantic_json_for("la .apasonatas ne fi'e la .betovn. cu se nelci mi")
            .expect("semantic JSON");
        let nelci = predication_with_relation_and_mode(&created_by, "nelci", "asserted");
        let finti = predication_with_relation_and_mode(&created_by, "finti", "incidental");
        assert_eq!(finti["introducedBy"], "fi'e");
        assert_eq!(
            finti["arguments"]["x2"]["value"],
            nelci["arguments"]["x2"]["value"]
        );
        assert_eq!(finti["arguments"]["x3"]["kind"], "elided");
        assert_eq!(finti["arguments"]["x4"]["kind"], "elided");
        assert_eq!(
            nelci["arguments"]["x2"]["relativeClauses"][0]["introducedBy"],
            "ne"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn comparative_modal_relative_phrases_lower_to_source_relations() {
        let relative = semantic_json_for("la .frank. nelci la .betis. ne semau la .meiris.")
            .expect("semantic JSON");
        let nelci = predication_with_relation_and_mode(&relative, "nelci", "asserted");
        let zmadu = predication_with_relation_and_mode(&relative, "zmadu", "incidental");
        assert_eq!(zmadu["introducedBy"], "se mau");
        assert_eq!(
            zmadu["arguments"]["x1"]["value"],
            nelci["arguments"]["x2"]["value"]
        );
        assert_eq!(zmadu["arguments"]["x2"]["kind"], "filled");
        assert_eq!(zmadu["arguments"]["x3"]["kind"], "elided");
        assert_eq!(zmadu["arguments"]["x4"]["kind"], "elided");
        assert_eq!(
            nelci["arguments"]["x2"]["relativeClauses"][0]["kind"],
            "incidental"
        );

        let attached_modal = semantic_json_for("la .frank. nelci la .meiris. seme'a la .betis.")
            .expect("semantic JSON");
        let nelci = predication_with_relation_and_mode(&attached_modal, "nelci", "asserted");
        assert_eq!(nelci["modalArguments"][0]["relation"], "mleca");
        assert_eq!(nelci["modalArguments"][0]["introducedBy"], "se me'a");
        assert_eq!(
            nelci["modalArguments"][0]["arguments"]["x1"]["kind"],
            "elided"
        );
        assert_eq!(
            nelci["modalArguments"][0]["arguments"]["x2"]["kind"],
            "filled"
        );
        assert_eq!(
            nelci["modalArguments"][0]["arguments"]["x3"]["kind"],
            "elided"
        );
        assert_eq!(
            nelci["modalArguments"][0]["arguments"]["x4"]["kind"],
            "elided"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn possessive_sumti_lowers_to_association_clause() {
        let json = semantic_json_for("le mi karce cu xunre").expect("semantic JSON");
        let associated = predication_with_relation_and_mode(&json, "associatedWith", "restrictive");
        let xunre = predication_with_relation_and_mode(&json, "xunre", "asserted");
        assert_eq!(
            associated["arguments"]["x1"]["value"],
            xunre["arguments"]["x1"]["value"]
        );
        assert_eq!(associated["arguments"]["x2"]["value"], "referent:speaker");
        let head = xunre["arguments"]["x1"]["value"]
            .as_str()
            .expect("possessed referent");
        assert_eq!(
            object(&json, head)["descriptor"]["relativeClauses"][0]["body"],
            "formula:f2"
        );
        assert!(xunre["arguments"]["x1"].get("relativeClauses").is_none());

        let preposed = semantic_json_for("le pe mi karce cu xunre").expect("semantic JSON");
        let xunre = predication_with_relation_and_mode(&preposed, "xunre", "asserted");
        let head = xunre["arguments"]["x1"]["value"]
            .as_str()
            .expect("preposed associated referent");
        assert_eq!(
            object(&preposed, head)["descriptor"]["relativeClauses"][0]["introducedBy"],
            "pe"
        );
        assert!(xunre["arguments"]["x1"].get("relativeClauses").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn possessor_relative_clause_attaches_to_possessor_argument() {
        let json =
            semantic_json_for("le mi noi sipna vau karce cu na klama").expect("semantic JSON");
        let associated = predication_with_relation_and_mode(&json, "associatedWith", "restrictive");
        assert_eq!(associated["arguments"]["x2"]["value"], "referent:speaker");
        assert_eq!(
            associated["arguments"]["x2"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        let sipna = predication_with_relation_and_mode(&json, "sipna", "incidental");
        assert_eq!(sipna["arguments"]["x1"]["value"], "referent:speaker");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        assert!(klama["arguments"]["x1"].get("relativeClauses").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn implicit_relative_tanru_uses_uniform_lowering() {
        let json = semantic_json_for("le birka poi jinzi ke se steci srana mi cu spofu")
            .expect("semantic JSON");
        let spofu = predication_with_relation_and_mode(&json, "spofu", "asserted");
        let head = spofu["arguments"]["x1"]["value"]
            .as_str()
            .expect("spofu head referent");
        assert_eq!(
            object(&json, head)["descriptor"]["relativeClauses"][0]["kind"],
            "restrictive"
        );
        assert!(spofu["arguments"]["x1"].get("relativeClauses").is_none());
        predication_with_relation_and_mode(&json, "srana", "restrictive");
        predication_with_relation_and_mode(&json, "jinzi", "restrictive");
        let objects = json["objects"].as_object().expect("objects");
        assert!(
            objects
                .values()
                .all(|object| object["relation"] != "jinzi steci srana")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn voi_relative_clause_is_restrictive_and_nonveridical() {
        let json = semantic_json_for("ti voi mlatu cu gerku").expect("semantic JSON");
        let described = predication_with_relation_and_mode(&json, "describedAs", "restrictive");
        let gerku = predication_with_relation_and_mode(&json, "gerku", "asserted");
        let relative_clause = &gerku["arguments"]["x1"]["relativeClauses"][0];
        assert_eq!(relative_clause["kind"], "restrictive");
        assert_eq!(relative_clause["introducedBy"], "voi");
        assert_eq!(relative_clause["veridical"], false);
        assert_eq!(described["arguments"]["x1"]["value"], "referent:speaker");
        assert_eq!(
            described["arguments"]["x2"]["value"],
            gerku["arguments"]["x1"]["value"]
        );
        assert!(
            described["arguments"]["x3"]["value"]
                .as_str()
                .expect("property abstraction")
                .starts_with("abstraction:")
        );
        let objects = json["objects"].as_object().expect("objects");
        assert!(
            objects
                .values()
                .all(|object| { object["relation"] != "mlatu" || object["mode"] != "incidental" })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_relative_keha_does_not_satisfy_outer_omitted_head() {
        let json =
            semantic_json_for("le prenu poi zvati le kumfa poi ke'axire zbasu ke'a cu masno")
                .expect("semantic JSON");
        let zbasu = predication_with_relation_and_mode(&json, "zbasu", "restrictive");
        let zvati = predication_with_relation_and_mode(&json, "zvati", "restrictive");
        let masno = predication_with_relation_and_mode(&json, "masno", "asserted");
        assert_eq!(
            zvati["arguments"]["x1"]["value"],
            masno["arguments"]["x1"]["value"]
        );
        assert_eq!(
            zbasu["arguments"]["x1"]["value"],
            masno["arguments"]["x1"]["value"]
        );
        assert_eq!(
            zbasu["arguments"]["x2"]["value"],
            zvati["arguments"]["x2"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn metalinguistic_pro_sumti_targets_previous_utterance() {
        let json =
            semantic_json_for("li re su'i re du li vo .i la'e di'u jetnu").expect("semantic JSON");
        let dihu = object(&json, "referent:r3");
        assert_eq!(dihu["descriptor"]["kind"], "utteranceReference");
        assert_eq!(dihu["descriptor"]["word"], "di'u");
        assert_eq!(dihu["target"], "utterance:u1");
        assert!(dihu.get("diagnostics").is_none());

        let lahe = object(&json, "referent:r4");
        assert_eq!(lahe["descriptor"]["kind"], "referentOfSymbol");
        assert_eq!(lahe["descriptor"]["operand"], "referent:r3");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn utterance_pro_sumti_cover_current_and_unspecified_utterances() {
        let json = semantic_json_for("dei jetnu jufra").expect("semantic JSON");
        let dei = object(&json, "referent:r1");
        assert_eq!(dei["descriptor"]["kind"], "utteranceReference");
        assert_eq!(dei["descriptor"]["word"], "dei");
        assert_eq!(dei["sort"], "sign");
        assert_eq!(dei["target"], "utterance:u1");
        assert!(dei.get("diagnostics").is_none());

        let json = semantic_json_for("do'i jetnu jufra").expect("semantic JSON");
        let dohi = object(&json, "referent:r1");
        assert_eq!(dohi["descriptor"]["kind"], "utteranceReference");
        assert_eq!(dohi["descriptor"]["word"], "do'i");
        assert_eq!(dohi["sort"], "sign");
        assert!(dohi.get("target").is_none());
        assert!(dohi.get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn goi_assignable_sumti_uses_associated_referent_publicly() {
        let json = semantic_json_for("la .alis. klama le zarci .i ko'a goi la .alis. cu blanu")
            .expect("semantic JSON");
        let blanu = predication_with_relation_and_mode(&json, "blanu", "asserted");
        let x1 = blanu["arguments"]["x1"]["value"]
            .as_str()
            .expect("x1 value");
        assert_eq!(object(&json, x1)["descriptor"]["name"], "alis");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cei_pro_bridi_inherits_non_overridden_places() {
        let json =
            semantic_json_for("mi klama cei brode le zarci .i do brode").expect("semantic JSON");
        let objects = json["objects"].as_object().expect("objects");
        let first_klama = objects
            .values()
            .find(|object| {
                object["type"] == "predication"
                    && object["relation"] == "klama"
                    && object["source"]["text"] == "mi klama cei brode le zarci"
            })
            .expect("antecedent klama");
        let second_klama = objects
            .values()
            .find(|object| {
                object["type"] == "predication"
                    && object["relation"] == "klama"
                    && object["source"]["text"] == "do brode"
            })
            .expect("pro-bridi klama");
        assert_eq!(
            second_klama["arguments"]["x1"]["value"],
            "referent:addressee"
        );
        assert_eq!(
            second_klama["arguments"]["x2"]["value"],
            first_klama["arguments"]["x2"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gohi_pro_bridi_inherits_non_overridden_places() {
        let json = semantic_json_for("mi klama le zarci .i do go'i").expect("semantic JSON");
        let objects = json["objects"].as_object().expect("objects");
        let klama = objects
            .values()
            .filter(|object| object["type"] == "predication" && object["relation"] == "klama")
            .collect::<Vec<_>>();
        assert_eq!(klama.len(), 2);
        assert_eq!(klama[1]["arguments"]["x1"]["value"], "referent:addressee");
        assert_eq!(
            klama[1]["arguments"]["x2"]["value"],
            klama[0]["arguments"]["x2"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gohi_pro_bridi_inherits_antecedent_tense() {
        let json = semantic_json_for("mi ba klama le zarci .i do go'i").expect("semantic JSON");
        let objects = json["objects"].as_object().expect("objects");
        let second_klama = objects
            .values()
            .find(|object| {
                object["type"] == "predication"
                    && object["relation"] == "klama"
                    && object["source"]["text"] == "do go'i"
            })
            .expect("repeated klama");
        let event = object(
            &json,
            second_klama["eventuality"]
                .as_str()
                .expect("eventuality id"),
        );
        assert_eq!(event["time"]["relation"], "after");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn letteral_pro_sumti_resolves_by_initial_letter() {
        let json =
            semantic_json_for("mi viska le gerku .i gy. cusku zo .arf.").expect("semantic JSON");
        let viska = predication_with_relation_and_mode(&json, "viska", "asserted");
        let cusku = predication_with_relation_and_mode(&json, "cusku", "asserted");
        assert_eq!(
            cusku["arguments"]["x1"]["value"],
            viska["arguments"]["x2"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn letteral_pro_sumti_resolves_by_multi_name_initials() {
        let json = semantic_json_for(
            "la .stivn. .mark. .djonz. cu merko \
             .i la .aleksandr. .pavlovitc. .kuznetsof. cu rusko \
             .i symydy. tavla .abupyky. bau la .lojban.",
        )
        .expect("semantic JSON");
        let merko = predication_with_relation_and_mode(&json, "merko", "asserted");
        let rusko = predication_with_relation_and_mode(&json, "rusko", "asserted");
        let tavla = predication_with_relation_and_mode(&json, "tavla", "asserted");
        assert_eq!(
            tavla["arguments"]["x1"]["value"],
            merko["arguments"]["x1"]["value"]
        );
        assert_eq!(
            tavla["arguments"]["x2"]["value"],
            rusko["arguments"]["x1"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_lerfu_string_is_letteral_sign() {
        let json = semantic_json_for("ty. .abu ny. ry. .ubu").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        assert_eq!(utterance["force"], "mention");
        assert_eq!(utterance["content"], "sign:s1");
        let sign = object(&json, "sign:s1");
        assert_eq!(sign["kind"], "letteral");
        assert_eq!(sign["text"], "tanru");
        assert_eq!(sign["letterals"][0]["value"], "t");
        assert_eq!(sign["letterals"][1]["sourceWords"][0], "a");
        assert_eq!(sign["letterals"][1]["sourceWords"][1], "bu");
        assert_eq!(sign["letterals"][1]["buDepth"], 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn meho_lerfu_string_is_letteral_sign_argument() {
        let json = semantic_json_for("me'o .abu cu lerfu").expect("semantic JSON");
        let lerfu = predication_with_relation_and_mode(&json, "lerfu", "asserted");
        assert_eq!(lerfu["arguments"]["x1"]["value"], "sign:s1");
        let sign = object(&json, "sign:s1");
        assert_eq!(sign["kind"], "letteral");
        assert_eq!(sign["text"], "a");
        assert!(sign.get("denotes").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn me_lerfu_string_selbri_uses_letteral_sign_source() {
        let json = semantic_json_for("la me dy ny. .abu").expect("semantic JSON");
        let referent_of = predication_with_relation_and_mode(&json, "referentOf", "restrictive");
        assert_eq!(referent_of["arguments"]["x2"]["value"], "sign:s1");
        assert_eq!(object(&json, "sign:s1")["kind"], "letteral");
        assert_eq!(object(&json, "sign:s1")["text"], "dna");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn character_code_letteral_preserves_code_source() {
        let json =
            semantic_json_for("me'o se'e cixa cu lerfu la .asycy'i'is.").expect("semantic JSON");
        let sign = object(&json, "sign:s1");
        assert_eq!(sign["kind"], "letteral");
        assert_eq!(sign["letterals"][0]["kind"], "characterCode");
        assert_eq!(sign["letterals"][0]["value"], "cixa");
        assert_eq!(sign["letterals"][0]["sourceWords"][0], "se'e");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn goi_named_assignment_is_public_on_referent() {
        let json =
            semantic_json_for("le ninmu goi la .sam. cu klama le zarci").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let x1 = klama["arguments"]["x1"]["value"]
            .as_str()
            .expect("x1 value");
        let assigned_names = object(&json, x1)["assignedNames"]
            .as_array()
            .expect("assigned names");
        assert_eq!(assigned_names[0]["name"], "sam");
        assert_eq!(assigned_names[0]["word"], "la");
        assert_eq!(assigned_names[0]["introducedBy"], "goi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cei_pro_bridi_expands_inside_restrictive_tanru() {
        let json = semantic_json_for(
            "ti slasi je mlatu bo cidja lante gacri cei broda .i le crino broda cu barda",
        )
        .expect("semantic JSON");
        let relations = predication_relations(&json);
        assert!(relations.iter().any(|relation| relation == "gacri"));
        assert!(!relations.iter().any(|relation| relation == "broda"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bo_grouped_tanru_uses_nested_uniform_tanru_lowering() {
        let right_grouped = semantic_json_for("ta cmalu nixli bo ckule").expect("semantic JSON");
        let relations = predication_relations(&right_grouped);
        assert!(relations.iter().any(|relation| relation == "ckule"));
        assert!(relations.iter().any(|relation| relation == "nixli"));
        assert!(relations.iter().any(|relation| relation == "cmalu"));
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:nixli-ckule]")
        );
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:cmalu-(nixli-ckule)]")
        );
        assert!(
            !relations
                .iter()
                .any(|relation| relation.contains("connected"))
        );

        let left_grouped = semantic_json_for("ta cmalu bo nixli ckule").expect("semantic JSON");
        let relations = predication_relations(&left_grouped);
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:cmalu-nixli]")
        );
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:cmalu-nixli-ckule]")
        );
        assert!(
            !relations
                .iter()
                .any(|relation| relation.contains("connected"))
        );

        let simple_bo = semantic_json_for("ta klama bo jubme").expect("semantic JSON");
        let relations = predication_relations(&simple_bo);
        assert!(relations.iter().any(|relation| relation == "jubme"));
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:klama-jubme]")
        );
        assert!(
            !relations
                .iter()
                .any(|relation| relation == "klama connected jubme")
        );

        let repeated_bo = semantic_json_for("ta cmalu bo nixli bo ckule").expect("semantic JSON");
        let relations = predication_relations(&repeated_bo);
        assert!(relations.iter().any(|relation| relation == "ckule"));
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:nixli-ckule]")
        );
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:cmalu-(nixli-ckule)]")
        );
        assert!(
            !relations
                .iter()
                .any(|relation| relation.contains("connected"))
        );

        let nested_unit_bo =
            semantic_json_for("ta melbi cmalu bo nixli bo ckule").expect("semantic JSON");
        let relations = predication_relations(&nested_unit_bo);
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:cmalu-(nixli-ckule)]")
        );
        assert!(
            relations
                .iter()
                .any(|relation| relation == "R[tanru:melbi-(cmalu-(nixli-ckule))]")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_tanru_connective_lowers_inside_property_abstraction() {
        let json = semantic_json_for("barda je xunre gerku").expect("semantic JSON");
        let relations = predication_relations(&json);
        assert!(relations.iter().any(|relation| relation == "gerku"));
        assert!(relations.iter().any(|relation| relation == "barda"));
        assert!(relations.iter().any(|relation| relation == "xunre"));
        assert!(
            !relations
                .iter()
                .any(|relation| relation.contains("connected"))
        );
        assert_eq!(object(&json, "abstraction:a1")["body"], "formula:f4");
        assert_eq!(object(&json, "formula:f4")["operator"], "and");
        assert_eq!(
            object(&json, "formula:f4")["connector"]["locus"],
            "property-abstraction"
        );
        assert_eq!(object(&json, "formula:f4")["connector"]["truthTable"], "je");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_tanru_connective_lowers_inside_description_restriction() {
        let json = semantic_json_for("mi viska pa mlatu je gerku").expect("semantic JSON");
        let relations = predication_relations(&json);
        assert!(relations.iter().any(|relation| relation == "mlatu"));
        assert!(relations.iter().any(|relation| relation == "gerku"));
        assert!(
            !relations
                .iter()
                .any(|relation| relation == "mlatu je gerku")
        );
        let description = object(&json, "referent:r1");
        let body = object(
            &json,
            description["descriptor"]["body"]
                .as_str()
                .expect("description body"),
        );
        assert_eq!(body["operator"], "and");
        assert_eq!(body["connector"]["locus"], "selbri");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_tanru_connective_lowers_when_tertau_is_connected() {
        let json = semantic_json_for("melbi cmalu nixli je ckule").expect("semantic JSON");
        let relations = predication_relations(&json);
        assert!(relations.iter().any(|relation| relation == "nixli"));
        assert!(relations.iter().any(|relation| relation == "ckule"));
        assert!(
            !relations
                .iter()
                .any(|relation| relation == "nixli je ckule")
        );

        let nixli = predication_with_relation_and_mode(&json, "nixli", "asserted");
        let ckule = predication_with_relation_and_mode(&json, "ckule", "asserted");
        assert_eq!(
            nixli["arguments"]["x1"]["value"],
            ckule["arguments"]["x1"]["value"]
        );
        assert_eq!(object(&json, "formula:f3")["operator"], "and");
        assert_eq!(
            object(&json, "formula:f3")["connector"]["locus"],
            "tanru-unit"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nonlogical_tanru_connective_builds_composite_concept_modifier() {
        let json = semantic_json_for("ti blanu joi xunre bolci").expect("semantic JSON");
        let modifier = object(&json, "referent:r3");
        assert_eq!(modifier["category"], "composite");
        assert_eq!(modifier["sort"], "concept");
        assert_eq!(modifier["composition"]["operator"], "mass");
        assert_eq!(modifier["composition"]["members"][0], "abstraction:a1");
        assert_eq!(modifier["composition"]["members"][1], "abstraction:a2");
        assert_eq!(modifier["composition"]["collective"], true);
        assert_eq!(
            object(&json, "predication:p4")["arguments"]["x2"]["value"],
            "referent:r3"
        );
        assert!(json["objects"].as_object().unwrap().values().all(|object| {
            object
                .pointer("/connector/truthTable")
                .is_none_or(|truth_table| truth_table != "joi")
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_selbri_uses_base_relation_and_converted_place_routing() {
        let json = semantic_json_for("le bajra cu jinga ja te jinga").expect("semantic JSON");
        let relations = predication_relations(&json);
        assert!(
            !relations
                .iter()
                .any(|relation| relation.contains("converted"))
        );
        assert_eq!(object(&json, "predication:p2")["relation"], "jinga");
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(object(&json, "predication:p3")["relation"], "jinga");
        assert_eq!(
            object(&json, "predication:p3")["arguments"]["x3"]["value"],
            "referent:r1"
        );
        assert_eq!(object(&json, "formula:f4")["operator"], "or");
        assert_eq!(object(&json, "formula:f4")["connector"]["truthTable"], "ja");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn converted_tanru_uses_visible_x1_for_modifier_relation() {
        let whole = semantic_json_for("le zarci cu se ke cadzu klama ke'e la .alis.")
            .expect("semantic JSON");
        let whole_klama = predication_with_relation_and_mode(&whole, "klama", "asserted");
        assert_eq!(whole_klama["arguments"]["x1"]["value"], "referent:r4");
        assert_eq!(whole_klama["arguments"]["x2"]["value"], "referent:r1");
        let whole_tanru =
            predication_with_relation_and_mode(&whole, "R[tanru:cadzu-klama]", "asserted");
        assert_eq!(whole_tanru["arguments"]["x1"]["value"], "referent:r4");

        let tertau =
            semantic_json_for("le zarci cu cadzu se klama la .alis.").expect("semantic JSON");
        let tertau_klama = predication_with_relation_and_mode(&tertau, "klama", "asserted");
        assert_eq!(tertau_klama["arguments"]["x1"]["value"], "referent:r4");
        assert_eq!(tertau_klama["arguments"]["x2"]["value"], "referent:r1");
        let tertau_tanru =
            predication_with_relation_and_mode(&tertau, "R[tanru:cadzu-klama]", "asserted");
        assert_eq!(tertau_tanru["arguments"]["x1"]["value"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_selbri_shares_omitted_visible_x1() {
        let json = semantic_json_for("ricfu je blanu jabo crino").expect("semantic JSON");
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(
            object(&json, "predication:p3")["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(object(&json, "formula:f4")["operator"], "or");
        assert_eq!(object(&json, "formula:f4")["connector"]["truthTable"], "ja");
        assert_eq!(object(&json, "formula:f5")["operator"], "and");
        assert_eq!(object(&json, "formula:f5")["connector"]["truthTable"], "je");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gihe_omitted_x1_is_not_shared_but_explicit_x1_is_shared() {
        let omitted =
            semantic_json_for("klama le zarci gi'e klama le briju").expect("semantic JSON");
        let first_omitted = object(&omitted, "predication:p2")["arguments"]["x1"]["value"]
            .as_str()
            .expect("first x1 value");
        let second_omitted = object(&omitted, "predication:p4")["arguments"]["x1"]["value"]
            .as_str()
            .expect("second x1 value");
        assert_ne!(first_omitted, second_omitted);
        assert_eq!(object(&omitted, "formula:f5")["operator"], "and");
        assert_eq!(
            object(&omitted, "formula:f5")["connector"]["source"],
            "gi'e"
        );

        let explicit =
            semantic_json_for("mi klama le zarci gi'e klama le briju").expect("semantic JSON");
        assert_eq!(
            object(&explicit, "predication:p2")["arguments"]["x1"]["value"],
            "referent:speaker"
        );
        assert_eq!(
            object(&explicit, "predication:p4")["arguments"]["x1"]["value"],
            "referent:speaker"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn logical_sumti_connection_distributes_and_shares_elided_places() {
        let json = semantic_json_for("la djan .e la alis klama le zarci").expect("semantic JSON");
        let djan = named_referent_id(&json, "djan");
        let alis = named_referent_id(&json, "alis");
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f4");
        assert_eq!(object(&json, "formula:f4")["operator"], "and");
        assert_eq!(object(&json, "formula:f4")["connector"]["source"], "e");
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x1"]["value"],
            djan
        );
        assert_eq!(
            object(&json, "predication:p3")["arguments"]["x1"]["value"],
            alis
        );
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x3"]["value"],
            object(&json, "predication:p3")["arguments"]["x3"]["value"]
        );
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x4"]["value"],
            object(&json, "predication:p3")["arguments"]["x4"]["value"]
        );
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x5"]["value"],
            object(&json, "predication:p3")["arguments"]["x5"]["value"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicate_fa_expands_to_conjoined_claims() {
        let json =
            semantic_json_for("fa mi fa do klama fe le zarci fe le zdani").expect("semantic JSON");
        let content = object(
            &json,
            object(&json, "utterance:u1")["content"]
                .as_str()
                .expect("utterance content"),
        );
        assert_eq!(content["operator"], "and");
        assert!(content.get("connector").is_none());

        let klamas = predications_with_relation_and_mode(&json, "klama", "asserted");
        assert_eq!(klamas.len(), 4);
        let x1_values = klamas
            .iter()
            .map(|predication| {
                predication["arguments"]["x1"]["value"]
                    .as_str()
                    .expect("x1 value")
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            x1_values,
            BTreeSet::from(["referent:addressee", "referent:speaker"])
        );
        let x2_values = klamas
            .iter()
            .map(|predication| {
                predication["arguments"]["x2"]["value"]
                    .as_str()
                    .expect("x2 value")
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(x2_values.len(), 2);
        let shared_x3_values = klamas
            .iter()
            .map(|predication| {
                predication["arguments"]["x3"]["value"]
                    .as_str()
                    .expect("x3 value")
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(shared_x3_values.len(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn truth_question_has_no_fillable_slot() {
        let json = semantic_json_for("xu do klama").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "truth");
        assert_eq!(question["mode"], "direct");
        assert_eq!(question["domain"], "truthValue");
        assert!(question.get("slots").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_truth_question_scopes_to_first_statement() {
        let json = semantic_json_for("xu zo .djan. cmene do .i go'i").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        assert_eq!(object(&json, "utterance:u2")["force"], "assert");
        assert_eq!(object(&json, "question:q1")["kind"], "truth");
        assert!(json.pointer("/objects/question:q2").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_aho_attitude_is_displayed_content() {
        let json = semantic_json_for(".a'o do jimpe").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        let display = object(&json, "display:d1");
        assert_eq!(display["type"], "displayedContent");
        assert_eq!(display["family"], "propositionalAttitude");
        assert_eq!(display["relation"], "hope");
        assert_eq!(display["polarity"], "positive");
        assert_eq!(display["assertionEffect"], "hostSubordinated");
        assert_eq!(display["experiencer"], "referent:speaker");
        assert_eq!(display["anchor"], "utterance:u1");
        assert_eq!(display["target"], utterance["content"]);
        assert_eq!(display["source"]["text"], "a'o");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn post_sumti_indicator_targets_referent() {
        let json = semantic_json_for("la djan .iu klama").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["relation"], "love");
        assert_eq!(display["target"], "referent:r1");
        assert_eq!(display["anchor"], "utterance:u1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn post_selbri_indicator_targets_formula() {
        let json = semantic_json_for("la djan klama .iu").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["relation"], "love");
        assert_eq!(display["target"], "formula:f1");
        assert_eq!(display["anchor"], "utterance:u1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_attitude_question_is_displayed_content_question_prompt() {
        let json = semantic_json_for(".iepei").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        let sign = object(&json, "sign:s1");
        let display = object(&json, "display:d1");
        assert_eq!(utterance["force"], "mention");
        assert_eq!(utterance["content"], "display:d1");
        assert_eq!(sign["text"], "ie pei");
        assert_eq!(display["family"], "questionPrompt");
        assert_eq!(display["relation"], "agreementQuestion");
        assert_eq!(display["target"], "sign:s1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_indicator_modifiers_attach_to_base_display() {
        let json = semantic_json_for(".ause'inai la djan klama").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["relation"], "desire");
        assert_eq!(display["source"]["text"], "ause'inai");
        assert_eq!(display["modifiers"][0]["relation"], "selfOrientation");
        assert_eq!(display["modifiers"][0]["polarity"], "negative");
        assert!(json.pointer("/objects/display:d2").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dai_changes_displayed_content_experiencer() {
        let json = semantic_json_for(".oiro'odai la djan klama").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["relation"], "complaint");
        assert_eq!(display["modifiers"][0]["relation"], "physical");
        let experiencer = display["experiencer"]
            .as_str()
            .expect("display experiencer");
        assert_ne!(experiencer, "referent:speaker");
        assert_eq!(
            object(&json, experiencer)["descriptor"]["word"],
            "dai experiencer"
        );
        assert!(json.pointer("/objects/display:d2").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_attitude_modifier_uses_english_relation() {
        let json = semantic_json_for("ko ga'inai nenri klama le mi zdani").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["family"], "attitudeModifier");
        assert_eq!(display["relation"], "rank");
        assert_eq!(display["polarity"], "negative");
        assert_eq!(display["target"], "referent:addressee");

        let json = semantic_json_for("le cukta be'u cu zvati ma").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["family"], "attitudeModifier");
        assert_eq!(display["relation"], "need");
        assert_eq!(display["target"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_multiple_attitudes_use_display_sequence_content() {
        let json = semantic_json_for(".iu bu'onai .uinai").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        assert_eq!(utterance["content"], "sequence:s1");
        let sequence = object(&json, "sequence:s1");
        assert_eq!(sequence["items"][0], "display:d1");
        assert_eq!(sequence["items"][1], "display:d2");
        assert_eq!(object(&json, "display:d1")["phase"], "ending");
        assert_eq!(object(&json, "display:d2")["relation"], "unhappiness");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_sumti_indicators_target_right_branch_or_exclusion() {
        let json = semantic_json_for("mi .e .ui nai do").expect("semantic JSON");
        assert_eq!(object(&json, "display:d1")["target"], "referent:addressee");
        assert_eq!(object(&json, "display:d1")["relation"], "unhappiness");
        assert!(json.pointer("/objects/display:d2").is_none());

        let json = semantic_json_for("mi .e nai .ui do").expect("semantic JSON");
        let composition = &object(&json, "referent:r1")["composition"];
        assert_eq!(composition["members"][0], "referent:speaker");
        assert_eq!(composition["excludedMembers"][0], "referent:addressee");
        assert_eq!(object(&json, "display:d1")["target"], "referent:r1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nonlogical_sumti_composition_uses_cll_operator() {
        let json =
            semantic_json_for("la djan. joi la .alis. cu bevri le pipno").expect("semantic JSON");
        let mass = composition_with_operator(&json, "mass");
        assert_eq!(mass["collective"], true);
        assert_eq!(mass["members"].as_array().expect("members").len(), 2);

        let json =
            semantic_json_for("lo'i ricfu ku jo'e lo'i dotco cu barda").expect("semantic JSON");
        let union = composition_with_operator(&json, "union");
        assert_eq!(union["members"].as_array().expect("members").len(), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joi_nai_is_scalar_negation_not_right_exclusion() {
        let json = semantic_json_for("mi jo'u nai do cu remei").expect("semantic JSON");
        let joint = composition_with_operator(&json, "joint");
        assert_eq!(joint["scalarNegated"], true);
        assert_eq!(joint["members"][0], "referent:speaker");
        assert_eq!(joint["members"][1], "referent:addressee");
        assert!(joint.get("excludedMembers").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn se_ordered_nonlogical_connective_reverses_members() {
        let json = semantic_json_for("ti liste do se ce'o mi").expect("semantic JSON");
        let sequence = composition_with_operator(&json, "sequence");
        assert_eq!(sequence["members"][0], "referent:speaker");
        assert_eq!(sequence["members"][1], "referent:addressee");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn interval_composition_records_bounds_and_complement() {
        let json = semantic_json_for("mi ca sanli la drezdn. ga'o bi'i ke'i la frankfurt.")
            .expect("semantic JSON");
        let interval = composition_with_operator(&json, "unorderedInterval");
        assert_eq!(interval["endpointInclusion"]["left"], "inclusive");
        assert_eq!(interval["endpointInclusion"]["right"], "exclusive");

        let json =
            semantic_json_for("mi sanli la drezdn. bi'i nai la frankfurt.").expect("semantic JSON");
        let complement = composition_with_operator(&json, "unorderedInterval");
        assert_eq!(complement["complement"], true);
        assert!(complement.get("excludedMembers").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sumti_connective_questions_use_composition_operator_parameter() {
        let json =
            semantic_json_for("do djica tu'a loi ckafi ji loi tcati").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        let composition = composition_with_operator(&json, "connectiveQuestion");
        assert_eq!(composition["operatorParameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["role"], "connectiveQuestion");

        let json =
            semantic_json_for("do djica tu'a ge'i loi ckafi gi loi tcati").expect("semantic JSON");
        let composition = composition_with_operator(&json, "connectiveQuestion");
        assert_eq!(composition["operatorParameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["introducedBy"], "ge'i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bound_and_forethought_nonlogical_sumti_connections_lower_to_compositions() {
        let json = semantic_json_for(
            "la djeimyz. cebo la djordj. pi'u la meris. cebo la martas. cu prami se remei",
        )
        .expect("semantic JSON");
        assert_eq!(
            composition_with_operator(&json, "crossProduct")["operator"],
            "crossProduct"
        );
        let set_count = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .filter_map(|object| object.get("composition"))
            .filter(|composition| composition["operator"] == "set")
            .count();
        assert_eq!(set_count, 2);

        let json =
            semantic_json_for("joigi la djan. gi la .alis. bevri le pipno").expect("semantic JSON");
        assert_eq!(composition_with_operator(&json, "mass")["collective"], true);

        let json = semantic_json_for("mi ca sanli ke'i bi'i ga'o gi la drezdn. gi la frankfurt.")
            .expect("semantic JSON");
        let interval = composition_with_operator(&json, "unorderedInterval");
        assert_eq!(interval["endpointInclusion"]["left"], "exclusive");
        assert_eq!(interval["endpointInclusion"]["right"], "inclusive");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connective_answer_fragments_mention_connective_signs() {
        let json = semantic_json_for("gi'enai").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        let sign = object(&json, "sign:s1");
        assert_eq!(utterance["force"], "mention");
        assert_eq!(utterance["content"], "sign:s1");
        assert_eq!(sign["kind"], "connective");
        assert_eq!(sign["text"], "gi'e nai");
        assert!(utterance.get("diagnostics").is_none());

        let json = semantic_json_for("joi").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        let sign = object(&json, "sign:s1");
        assert_eq!(utterance["content"], "sign:s1");
        assert_eq!(sign["kind"], "connective");
        assert_eq!(sign["text"], "joi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn future_utterance_reference_resolves_to_following_text_group() {
        let json = semantic_json_for("mi ba gasnu la'e di'e .i tu'e kanji lo ni cteki tu'u")
            .expect("semantic JSON");
        let dihe = json["objects"]
            .as_object()
            .expect("semantic objects")
            .values()
            .find(|object| {
                object["type"] == "referent"
                    && object["descriptor"]["kind"] == "utteranceReference"
                    && object["descriptor"]["word"] == "di'e"
            })
            .expect("di'e referent");
        assert_eq!(dihe["target"], "utterance:u2");
        assert!(dihe.get("diagnostics").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_bridi_indicators_target_their_branch_formulas() {
        let json = semantic_json_for(
            "ganai da'i do viska le mi citno mensi gi ju'o do djuno le du'u ri pazvau",
        )
        .expect("semantic JSON");
        let hypothetical = object(&json, "display:d1");
        let certainty = object(&json, "display:d2");
        assert_eq!(hypothetical["relation"], "hypothetical");
        assert_eq!(certainty["relation"], "certainty");
        assert_ne!(hypothetical["target"], certainty["target"]);
        assert_eq!(hypothetical["anchor"], "utterance:u1");
        assert_eq!(certainty["anchor"], "utterance:u1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_indicator_attaches_to_vocative_only_utterance() {
        let json = semantic_json_for("ru'a doi .livinston.").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(root_object(&json)["force"], "vocative");
        assert_eq!(display["family"], "evidential");
        assert_eq!(display["relation"], "presumption");
        assert_eq!(display["target"], "utterance:u1");
        assert_eq!(display["anchor"], "utterance:u1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_separator_indicator_targets_previous_content() {
        let json = semantic_json_for("do sazri le karce .i .e'a").expect("semantic JSON");
        let display = object(&json, "display:d1");
        assert_eq!(display["family"], "propositionalAttitude");
        assert_eq!(display["relation"], "permission");
        assert_eq!(display["target"], root_object(&json)["content"]);
        assert_eq!(display["anchor"], "utterance:u1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ma_question_slot_is_inside_formula() {
        let json = semantic_json_for("ma klama").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "argument");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["kind"],
            "filled"
        );
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["value"],
            "parameter:p1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ma_kau_is_embedded_indirect_question_inside_duhu() {
        let json =
            semantic_json_for("mi djuno le du'u ma kau pu klama le zarci").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        assert_eq!(utterance["force"], "assert");
        assert_eq!(
            object(&json, utterance["content"].as_str().unwrap())["type"],
            "formula"
        );

        let abstraction = object(&json, "abstraction:a1");
        assert_eq!(abstraction["embeddedQuestions"][0], "question:q1");

        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "argument");
        assert_eq!(question["mode"], "indirect");
        assert_eq!(question["body"], abstraction["body"]);
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(question["focus"], "parameter:p1");

        let klama = predication_with_relation_and_mode(&json, "klama", "inert");
        assert_eq!(klama["arguments"]["x1"]["value"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_ma_inside_duhu_remains_direct_outer_question() {
        let json =
            semantic_json_for("mi djuno le du'u ma pu klama le zarci").expect("semantic JSON");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        assert_eq!(object(&json, "utterance:u1")["content"], "question:q1");
        assert_eq!(object(&json, "question:q1")["mode"], "direct");
        assert!(
            object(&json, "abstraction:a1")
                .get("embeddedQuestions")
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn concrete_kau_focus_records_presupposed_answer() {
        let json = semantic_json_for("mi djuno le du'u la .djan. kau pu klama le zarci")
            .expect("semantic JSON");
        let djan = named_referent_id(&json, "djan");
        let question = object(&json, "question:q1");
        assert_eq!(object(&json, "utterance:u1")["force"], "assert");
        assert_eq!(question["mode"], "indirect");
        assert_eq!(question["focus"], djan);
        assert_eq!(question["presupposedAnswer"], djan);
        assert!(question.get("slots").is_none());
        assert_eq!(
            object(&json, "abstraction:a1")["embeddedQuestions"][0],
            "question:q1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jikau_is_embedded_indirect_connective_question_inside_duhu() {
        let json =
            semantic_json_for("mi ba zgana le du'u la .djan. jikau la .djordj. cu zvati le panka")
                .expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(object(&json, "utterance:u1")["force"], "assert");
        assert_eq!(question["kind"], "connective");
        assert_eq!(question["mode"], "indirect");
        assert_eq!(question["domain"], "connective");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(question["focus"], "parameter:p1");
        assert_eq!(
            object(&json, "abstraction:a1")["embeddedQuestions"][0],
            "question:q1"
        );

        let connective = object(&json, "formula:f4");
        assert_eq!(connective["operator"], "connectiveQuestion");
        assert_eq!(connective["connector"]["locus"], "sumti");
        assert_eq!(connective["connector"]["parameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["role"], "connectiveQuestion");
        assert!(json.pointer("/objects/question:q2").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cuhe_tense_question_slot_is_inside_eventuality() {
        let json = semantic_json_for("le nanmu cu'e batci le gerku").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "tense");
        assert_eq!(question["domain"], "tenseModal");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["sort"], "tenseModal");
        assert_eq!(object(&json, "parameter:p1")["role"], "tenseQuestion");

        let batci = predication_with_relation_and_mode(&json, "batci", "asserted");
        let event = object(&json, batci["eventuality"].as_str().expect("batci event"));
        assert_eq!(event["tenseModal"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_cuhe_preserves_one_tense_slot_in_question_branch() {
        let json = semantic_json_for("do puzi je cu'e sombo le gurni").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "tense");
        assert_eq!(question["domain"], "tenseModal");
        assert_eq!(question["slots"].as_array().expect("slots").len(), 1);
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert!(json.pointer("/objects/parameter:p2").is_none());

        let connective = json["objects"]
            .as_object()
            .expect("objects")
            .values()
            .find(|object| {
                object["type"] == "formula"
                    && object["operator"] == "and"
                    && object.pointer("/connector/locus") == Some(&Value::String("tense".into()))
            })
            .expect("connected tense formula");
        assert_eq!(connective["connector"]["truthTable"], "je");

        let tense_slot_events = json["objects"]
            .as_object()
            .expect("objects")
            .values()
            .filter(|object| {
                object["type"] == "eventuality" && object["tenseModal"] == "parameter:p1"
            })
            .count();
        assert_eq!(tense_slot_events, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jehi_tense_connective_question_slot_is_inside_connector() {
        let json = semantic_json_for("la .artr. pu je'i ba nolraitru").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(question["kind"], "connective");
        assert_eq!(question["domain"], "connective");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["sort"], "connective");
        assert_eq!(object(&json, "parameter:p1")["role"], "connectiveQuestion");

        let connective = json["objects"]
            .as_object()
            .expect("objects")
            .values()
            .find(|object| {
                object["type"] == "formula" && object["operator"] == "connectiveQuestion"
            })
            .expect("connective question formula");
        assert_eq!(connective["connector"]["locus"], "tense");
        assert_eq!(connective["connector"]["parameter"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_modal_fragments_expose_answer_eventuality_content() {
        let spatial = semantic_json_for("vi le lunra").expect("semantic JSON");
        let content = object(&spatial, "utterance:u1")["content"]
            .as_str()
            .expect("fragment content");
        let event = object(&spatial, content);
        assert_eq!(event["space"]["relation"], "near");
        assert_eq!(event["space"]["anchor"], "referent:r1");

        let modal = semantic_json_for("seka'a le briju").expect("semantic JSON");
        let content = object(&modal, "utterance:u1")["content"]
            .as_str()
            .expect("fragment content");
        let event = object(&modal, content);
        assert_eq!(event["modalArguments"][0]["relation"], "klama");
        assert_eq!(event["modalArguments"][0]["introducedBy"], "se ka'a");
        assert_eq!(
            event["modalArguments"][0]["arguments"]["x2"]["value"],
            "referent:r1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fiha_place_question_binds_known_argument_to_candidate_places() {
        let json = semantic_json_for("fi'a do dunda fe le vi rozgu").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        assert_eq!(question["kind"], "place");
        assert_eq!(question["domain"], "place");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["sort"], "place");
        assert_eq!(object(&json, "parameter:p1")["role"], "placeQuestion");

        let dunda = predication_with_relation_and_mode(&json, "dunda", "asserted");
        assert_eq!(dunda["arguments"]["x1"]["kind"], "elided");
        assert_eq!(dunda["arguments"]["x2"]["kind"], "filled");
        assert_eq!(dunda["arguments"]["x3"]["kind"], "elided");
        assert_eq!(dunda["placeQuestions"][0]["parameter"], "parameter:p1");
        assert_eq!(
            dunda["placeQuestions"][0]["argument"]["value"],
            "referent:addressee"
        );
        assert_eq!(dunda["placeQuestions"][0]["candidatePlaces"][0], "x1");
        assert_eq!(dunda["placeQuestions"][0]["candidatePlaces"][1], "x3");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mo_question_slot_is_relation_parameter_inside_formula() {
        let json = semantic_json_for("do mo").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        assert_eq!(question["kind"], "relation");
        assert_eq!(question["domain"], "relation");
        assert_eq!(question["slots"][0]["parameter"], "parameter:p1");
        assert_eq!(object(&json, "parameter:p1")["sort"], "relation");
        assert_eq!(object(&json, "parameter:p1")["role"], "relationQuestion");
        let predication = predication_with_relation_parameter(&json, "parameter:p1");
        assert_eq!(
            predication["arguments"]["x1"]["value"],
            "referent:addressee"
        );
        assert!(predication.get("relation").is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mo_inside_tanru_description_is_relation_question() {
        let json = semantic_json_for("lo mo prenu cu darxi do .i barda").expect("semantic JSON");
        let question = object(&json, "question:q1");
        assert_eq!(object(&json, "utterance:u1")["force"], "ask");
        assert_eq!(object(&json, "utterance:u2")["force"], "assert");
        assert_eq!(question["kind"], "relation");
        assert_eq!(object(&json, "parameter:p2")["sort"], "relation");
        let relation_slot = predication_with_relation_parameter(&json, "parameter:p2");
        assert_eq!(relation_slot["mode"], "restrictive");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vocative_ma_is_question_content() {
        let json = semantic_json_for("doi ma").expect("semantic JSON");
        let utterance = object(&json, "utterance:u1");
        assert_eq!(utterance["force"], "vocative");
        assert_eq!(utterance["content"], "question:q1");
        assert_eq!(object(&json, "question:q1")["kind"], "argument");
        assert_eq!(
            object(&json, "question:q1")["slots"][0]["parameter"],
            "parameter:p1"
        );
        let target = predication_with_relation_and_mode(&json, "vocativeTarget", "performative");
        assert_eq!(target["arguments"]["x1"]["value"], "parameter:p1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantified_pro_sumti_quantity_is_argument_scoped() {
        let json = semantic_json_for("re do cadzu le bisli").expect("semantic JSON");
        let cadzu = predication_with_relation_and_mode(&json, "cadzu", "asserted");
        assert_eq!(cadzu["arguments"]["x1"]["value"], "referent:addressee");
        assert_eq!(cadzu["arguments"]["x1"]["quantity"], "quantity:q1");
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 2);
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "re");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantified_quotation_quantity_is_argument_scoped() {
        let json =
            semantic_json_for("mi cusku re lu do cadzu le bisli li'u").expect("semantic JSON");
        let cusku = predication_with_relation_and_mode(&json, "cusku", "asserted");
        assert_eq!(cusku["arguments"]["x2"]["value"], "sign:s1");
        assert_eq!(cusku["arguments"]["x2"]["quantity"], "quantity:q1");
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 2);
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "re");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_quantified_description_quantity_is_argument_scoped() {
        let json = semantic_json_for("mi ponse su'o ci cutci").expect("semantic JSON");
        let ponse = predication_with_relation_and_mode(&json, "ponse", "asserted");
        let shoes = ponse["arguments"]["x2"]["value"]
            .as_str()
            .expect("shoe referent ID");
        assert_eq!(ponse["arguments"]["x2"]["quantity"], "quantity:q1");
        assert!(object(&json, shoes)["descriptor"].get("quantity").is_none());
        assert_eq!(object(&json, shoes)["source"]["text"], "cutci");
        assert_eq!(object(&json, "quantity:q1")["value"]["text"], "su'o ci");
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "su'o ci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inner_and_outer_description_quantifiers_are_distinct() {
        let json = semantic_json_for("re le ci gerku cu blabi").expect("semantic JSON");
        let blabi = predication_with_relation_and_mode(&json, "blabi", "asserted");
        let dogs = blabi["arguments"]["x1"].as_object().expect("dog argument");
        assert_eq!(dogs["quantity"], "quantity:q1");
        let dog_referent = dogs["value"].as_str().expect("dog referent ID");
        assert_eq!(
            object(&json, dog_referent)["descriptor"]["quantity"],
            "quantity:q2"
        );
        assert_eq!(object(&json, dog_referent)["source"]["text"], "le ci gerku");
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 2);
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "re");
        assert_eq!(object(&json, "quantity:q2")["value"]["integer"], 3);
        assert_eq!(object(&json, "quantity:q2")["source"]["text"], "ci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sumti_based_description_preserves_operand_and_quantity() {
        let json = semantic_json_for("le re do cu nanmu").expect("semantic JSON");
        let nanmu = predication_with_relation_and_mode(&json, "nanmu", "asserted");
        let described = nanmu["arguments"]["x1"]["value"]
            .as_str()
            .expect("description referent");
        let descriptor = &object(&json, described)["descriptor"];
        assert_eq!(descriptor["word"], "le");
        assert_eq!(descriptor["quantity"], "quantity:q1");
        assert_eq!(descriptor["operand"], "referent:addressee");
        assert_eq!(object(&json, described)["source"]["text"], "le re do");
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 2);
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "re");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_sumti_based_description_preserves_all_quantities() {
        let json = semantic_json_for("pa le re le ci cribe cu bunre").expect("semantic JSON");
        let bunre = predication_with_relation_and_mode(&json, "bunre", "asserted");
        let outer_argument = bunre["arguments"]["x1"]
            .as_object()
            .expect("brown argument");
        assert_eq!(outer_argument["quantity"], "quantity:q1");
        let outer_referent = outer_argument["value"]
            .as_str()
            .expect("outer description referent");
        let outer_descriptor = &object(&json, outer_referent)["descriptor"];
        let outer_quantity = outer_descriptor["quantity"]
            .as_str()
            .expect("outer descriptor quantity");
        let inner_referent = outer_descriptor["operand"]
            .as_str()
            .expect("inner description referent");
        let inner_descriptor = &object(&json, inner_referent)["descriptor"];
        let inner_quantity = inner_descriptor["quantity"]
            .as_str()
            .expect("inner descriptor quantity");
        assert_eq!(inner_descriptor["body"], "formula:f1");
        assert_eq!(object(&json, "quantity:q1")["value"]["integer"], 1);
        assert_eq!(object(&json, outer_quantity)["value"]["integer"], 2);
        assert_eq!(object(&json, inner_quantity)["value"]["integer"], 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_description_relative_clause_is_occurrence_scoped() {
        let json = semantic_json_for("le poi blabi ku'o gerku cu klama").expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let argument = &klama["arguments"]["x1"];
        assert_eq!(argument["value"], "referent:r1");
        assert_eq!(argument["relativeClauses"][0]["kind"], "restrictive");
        assert_eq!(argument["relativeClauses"][0]["body"], "formula:f2");
        assert!(
            object(&json, "referent:r1")["descriptor"]
                .get("relativeClauses")
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn in_description_and_post_ku_relative_clauses_have_distinct_scopes() {
        let in_description =
            semantic_json_for("lo prenu noi blabi cu klama le zarci").expect("semantic JSON");
        let in_description_referent = object(&in_description, "referent:r1");
        assert_eq!(
            in_description_referent["descriptor"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        let in_description_klama =
            predication_with_relation_and_mode(&in_description, "klama", "asserted");
        assert!(
            in_description_klama["arguments"]["x1"]
                .get("relativeClauses")
                .is_none()
        );

        let post_ku =
            semantic_json_for("lo prenu ku noi blabi cu klama le zarci").expect("semantic JSON");
        assert!(
            object(&post_ku, "referent:r1")["descriptor"]
                .get("relativeClauses")
                .is_none()
        );
        let post_ku_klama = predication_with_relation_and_mode(&post_ku, "klama", "asserted");
        assert_eq!(
            post_ku_klama["arguments"]["x1"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        assert_eq!(
            post_ku_klama["arguments"]["x1"]["relativeClauses"][0]["body"],
            "formula:f2"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn post_ku_relative_clause_preserves_outer_quantifier() {
        let json = semantic_json_for("re le mu prenu ku poi ninmu cu klama le zarci")
            .expect("semantic JSON");
        let klama = predication_with_relation_and_mode(&json, "klama", "asserted");
        let argument = &klama["arguments"]["x1"];
        assert_eq!(argument["quantity"], "quantity:q1");
        assert_eq!(argument["relativeClauses"][0]["kind"], "restrictive");
        assert_eq!(argument["relativeClauses"][0]["body"], "formula:f2");
        assert_eq!(object(&json, "quantity:q1")["source"]["text"], "re");
        assert_eq!(
            object(&json, "referent:r1")["descriptor"]["quantity"],
            "quantity:q2"
        );
        assert_eq!(object(&json, "quantity:q2")["source"]["text"], "mu");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_indefinite_relative_clause_is_occurrence_scoped() {
        let json = semantic_json_for("mi ponse re karce poi xekri").expect("semantic JSON");
        let ponse = predication_with_relation_and_mode(&json, "ponse", "asserted");
        let argument = &ponse["arguments"]["x2"];
        assert_eq!(argument["quantity"], "quantity:q1");
        assert_eq!(argument["relativeClauses"][0]["kind"], "restrictive");
        assert_eq!(argument["relativeClauses"][0]["body"], "formula:f2");
        let described = argument["value"].as_str().expect("car referent");
        assert!(
            object(&json, described)["descriptor"]
                .get("relativeClauses")
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn xorlo_description_does_not_add_default_quantifier() {
        let json = semantic_json_for("lo mlatu cu klama").expect("semantic JSON");
        let referent = object(&json, "referent:r1");
        assert_eq!(referent["descriptor"]["kind"], "veridicalDescription");
        assert_eq!(referent["descriptor"]["word"], "lo");
        assert!(referent["descriptor"].get("quantity").is_none());
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["kind"],
            "filled"
        );
        assert_eq!(
            object(&json, "predication:p1")["arguments"]["x1"]["value"],
            "referent:r1"
        );
    }
}
