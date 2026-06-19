//! Syntax-to-semantic-graph builder for the public JSON semantics model.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};
use jbotci_morphology::{Cmavo, Word, strip_diacritics};
use jbotci_syntax::ast::{
    BridiSyntax, BridiTailSyntax, DescriptionSyntax, ParagraphStatementSyntax, QuantifierSyntax,
    QuantifierSyntaxData, SelbriSyntax, SelbriSyntaxData, SimpleBridiTailSyntaxData,
    StatementSyntax, StatementSyntaxData, SumtiSyntax, SumtiSyntaxData, TanruUnitSyntax,
    TanruUnitSyntaxData, TenseModalSyntax, TenseModalSyntaxData, TextSyntax, Token,
    WithFreeModifiers, WordRun,
};

use crate::model::{
    Actuality, ActualityKind, AnchorRelation, ArgumentValue, Composition, Descriptor,
    EventualityClass, IndexicalKind, PredicationMode, QuantityForm, QuantityScale, QuantityValue,
    QuestionKind, QuestionMode, QuestionSlot, QuestionSlotRole, ReferentCategory,
    SemanticDiagnostic, SemanticGraph, SemanticObject, SemanticObjectId, SemanticSort,
    SequenceRelation, UtteranceForce, diagnostic, source_from_spans,
};
use crate::references::{
    PlaceFrameKind, PlaceSlot, RawSyntaxNodeId, ReferenceAnalysis, ReferenceAnalysisError,
    SelbriPlaceFrameId, analyze_references,
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
}

impl Default for SemanticBuildOptions<'_> {
    #[requires(true)]
    #[ensures(ret.source_text.is_none())]
    fn default() -> Self {
        Self { source_text: None }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()))]
pub fn build_semantic_graph(
    syntax: &TextSyntax,
    source_text: Option<&str>,
) -> Result<SemanticGraph, SemanticsError> {
    build_semantic_graph_with_place_resolver(syntax, source_text, |_| None)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()))]
pub fn build_semantic_graph_with_dictionary(
    syntax: &TextSyntax,
    source_text: Option<&str>,
    dictionary: &Dictionary<'_>,
) -> Result<SemanticGraph, SemanticsError> {
    build_semantic_graph_with_place_resolver(syntax, source_text, |relation| {
        dictionary_relation_place_count(dictionary, relation)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()))]
pub fn build_semantic_graph_with_place_resolver<F>(
    syntax: &TextSyntax,
    source_text: Option<&str>,
    relation_place_count: F,
) -> Result<SemanticGraph, SemanticsError>
where
    F: Fn(&str) -> Option<usize>,
{
    let analysis = analyze_references(syntax)?;
    let mut builder = GraphBuilder::new(
        &analysis,
        SemanticBuildOptions { source_text },
        &relation_place_count,
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
struct GraphBuilder<'analysis, 'tree, 'resolver, F>
where
    F: Fn(&str) -> Option<usize>,
{
    analysis: &'analysis ReferenceAnalysis<'tree>,
    options: SemanticBuildOptions<'analysis>,
    relation_place_count: &'resolver F,
    objects: BTreeMap<SemanticObjectId, SemanticObject>,
    counters: IdCounters,
    sumti_objects: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    parameter_slots: Vec<QuestionSlot>,
}

impl<'analysis, 'tree, 'resolver, F> GraphBuilder<'analysis, 'tree, 'resolver, F>
where
    F: Fn(&str) -> Option<usize>,
{
    #[requires(true)]
    #[ensures(ret.objects.contains_key(&SemanticObjectId::speaker()))]
    fn new(
        analysis: &'analysis ReferenceAnalysis<'tree>,
        options: SemanticBuildOptions<'analysis>,
        relation_place_count: &'resolver F,
    ) -> Self {
        let mut builder = Self {
            analysis,
            options,
            relation_place_count,
            objects: BTreeMap::new(),
            counters: IdCounters::new(),
            sumti_objects: HashMap::new(),
            parameter_slots: Vec::new(),
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
    fn next_quantity(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::quantity(self.counters.quantity);
        self.counters.quantity += 1;
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
    #[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
    fn build_text(&mut self, text: &'tree TextSyntax) -> Result<SemanticGraph, SemanticsError> {
        let truth_question = text
            .leading_indicators
            .iter()
            .any(|indicator| indicator.indicator.cmavo() == Some(Cmavo::Xu));
        let mut items = Vec::new();
        for paragraph in &text.paragraphs {
            for statement in &paragraph.statements {
                if let Some(statement_id) =
                    self.build_paragraph_statement(statement, truth_question)?
                {
                    items.push(statement_id);
                }
            }
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

    #[requires(true)]
    #[ensures(true)]
    fn build_paragraph_statement(
        &mut self,
        statement: &'tree ParagraphStatementSyntax,
        truth_question: bool,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(statement) = statement.statement.as_deref() else {
            return Ok(None);
        };
        self.build_statement(statement, truth_question).map(Some)
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
                self.build_bridi_utterance(bridi, truth_question)
            }
            data!(StatementSyntax::TextGroup { text, .. }) => {
                let nested = self.build_text_group_sequence(text)?;
                self.build_utterance(
                    UtteranceForce::Parenthetical,
                    Some(nested),
                    self.analysis
                        .syntax_index
                        .statement_node_id(statement)
                        .and_then(|node| self.source_for_node(node.0, "statement")),
                    vec![diagnostic(
                        "tu'e text group is represented as a nested discourse sequence",
                    )],
                )
            }
            data!(StatementSyntax::Prenex {
                inner_statement,
                ..
            }) => {
                let id = self.build_statement(inner_statement, truth_question)?;
                self.add_object_diagnostic(
                    id,
                    diagnostic(
                        "prenex scope is not fully lowered yet; inner statement is preserved",
                    ),
                );
                Ok(id)
            }
            data!(StatementSyntax::StatementConnection {
                leading_statement,
                trailing_statement,
                ..
            })
            | data!(StatementSyntax::PreposedIStatementConnection {
                leading_statement,
                trailing_statement,
                ..
            }) => {
                let first = self.build_statement(leading_statement, truth_question)?;
                let second = self.build_statement(trailing_statement, false)?;
                let id = self.next_sequence();
                self.insert(
                    id,
                    SemanticObject::sequence(
                        vec![first, second],
                        SequenceRelation::SameTopicContinuation,
                        self
                            .analysis
                            .syntax_index
                            .statement_node_id(statement)
                            .and_then(|node| self.source_for_node(node.0, "statement-connection")),
                        vec![diagnostic(
                            "statement connective is preserved as discourse sequencing until truth-functional lowering is implemented",
                        )],
                    ),
                )
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
            data!(StatementSyntax::Fragment(..)) => self.build_utterance(
                UtteranceForce::Mention,
                None,
                self.analysis
                    .syntax_index
                    .statement_node_id(statement)
                    .and_then(|node| self.source_for_node(node.0, "fragment")),
                vec![diagnostic("fragment has no truth-bearing semantic formula")],
            ),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_text_group_sequence(
        &mut self,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut nested = GraphBuilder::new(self.analysis, self.options, self.relation_place_count);
        nested.counters = self.counters;
        nested.objects = std::mem::take(&mut self.objects);
        let graph = nested.build_text(text)?;
        self.counters = nested.counters;
        self.objects = graph.objects;
        Ok(graph.root)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_utterance(
        &mut self,
        bridi: &'tree BridiSyntax,
        truth_question: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let previous_slots = std::mem::take(&mut self.parameter_slots);
        let formula = self.build_bridi_formula(bridi)?;
        let slots = std::mem::replace(&mut self.parameter_slots, previous_slots);
        let is_question = truth_question || !slots.is_empty();
        let content = if is_question {
            let id = self.next_question();
            let kind = if truth_question && slots.is_empty() {
                QuestionKind::Truth
            } else {
                QuestionKind::Argument
            };
            let domain = if truth_question && slots.is_empty() {
                SemanticSort::TruthValue
            } else {
                SemanticSort::Entity
            };
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
        let force = if is_question {
            UtteranceForce::Ask
        } else {
            UtteranceForce::Assert
        };
        self.build_utterance(
            force,
            Some(content),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "bridi")),
            Vec::new(),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_utterance(
        &mut self,
        force: UtteranceForce,
        content: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
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
        let id = self.next_utterance();
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
        let selbri = main_selbri_for_tail(&bridi.bridi_tail);
        let relation = selbri
            .map(relation_label_for_selbri)
            .unwrap_or_else(|| "unknown-relation".to_owned());
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

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_predication_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: Option<&'tree SelbriSyntax>,
        relation: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(
            EventualityClass::Event,
            Some(Actuality {
                kind: ActualityKind::Actual,
            }),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "eventuality")),
        );
        if let Some(relation) = selbri.and_then(time_relation_for_selbri) {
            event.time = Some(AnchorRelation {
                relation,
                anchor: SemanticObjectId::speech_time(),
            });
        }
        self.insert(eventuality, event)?;
        let frame = self.bridi_frame(bridi);
        let mut arguments = BTreeMap::new();
        if let Some(frame) = frame {
            let assignment_ids = self.analysis.place_analysis.assignments_for_frame(frame);
            for assignment_id in assignment_ids {
                let Some(assignment) = self.analysis.place_analysis.assignment(*assignment_id)
                else {
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
                let argument = self.build_argument_for_sumti(sumti)?;
                arguments.insert(format!("x{}", place.get()), argument);
            }
        }
        let mut diagnostics = if selbri.is_none() {
            vec![diagnostic("bridi tail has no direct selbri relation")]
        } else {
            Vec::new()
        };
        match (self.relation_place_count)(&relation) {
            Some(place_count) => {
                for place in 1..=place_count {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => diagnostics.push(diagnostic(
                "relation place structure is unavailable; only explicit assigned places are represented",
            )),
        }
        let id = self.next_predication();
        self.insert(
            id,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                PredicationMode::Asserted,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "predication")),
                diagnostics,
            ),
        )
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
        let referent = self.build_sumti_referent(sumti)?;
        if sumti_is_elided(sumti) {
            return Ok(ArgumentValue::elided(
                referent,
                "zo'e".to_owned(),
                self.source_for_node(raw, "elided-place"),
            ));
        }
        Ok(ArgumentValue::filled(referent, None))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_argument_for_place(
        &mut self,
        place: usize,
    ) -> Result<ArgumentValue, SemanticsError> {
        let referent = self.build_elided_referent(None, format!("zo'e x{place}"))?;
        Ok(ArgumentValue::elided(referent, "zo'e".to_owned(), None))
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
            data!(SumtiSyntax::ProSumti(token)) => self.build_pro_sumti(token, raw)?,
            data!(SumtiSyntax::ElidedSumti { .. }) => {
                self.build_elided_referent(Some(raw), "zo'e".to_owned())?
            }
            data!(SumtiSyntax::Description(description)) => {
                self.build_description_referent(description, raw)?
            }
            data!(SumtiSyntax::NameDescription { names, .. }) => {
                self.build_named_referent(raw, word_run_text(&names.value), "la")?
            }
            data!(SumtiSyntax::NameWords(names)) => {
                self.build_named_referent(raw, word_run_text(&names.value), "la")?
            }
            data!(SumtiSyntax::QuantifiedSumti {
                quantifier,
                inner_sumti,
            }) => {
                let quantity = self.build_quantity_for_words(
                    quantifier_first_word_text(quantifier).unwrap_or_else(|| "xo'e".to_owned()),
                    Some(raw),
                )?;
                let referent = self.build_sumti_referent(inner_sumti)?;
                self.add_quantity_to_referent(referent, quantity);
                referent
            }
            data!(SumtiSyntax::SumtiConnection {
                leading_sumti,
                trailing_sumti,
                ..
            }) => {
                let leading = self.build_sumti_referent(leading_sumti)?;
                let trailing = self.build_sumti_referent(trailing_sumti)?;
                self.build_composite_referent(raw, vec![leading, trailing], "joint")?
            }
            data!(SumtiSyntax::GroupedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::TaggedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::ScalarNegatedSumti { inner_sumti, .. })
            | data!(SumtiSyntax::ScalarNegatedSumtiWithBo { inner_sumti, .. })
            | data!(SumtiSyntax::ReferentSumti { inner_sumti, .. }) => {
                self.build_sumti_referent(inner_sumti)?
            }
            _ => self.build_diagnostic_referent(raw, "sumti construct is not fully lowered yet")?,
        };
        self.sumti_objects.insert(raw, id);
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_pro_sumti(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match token.cmavo() {
            Some(Cmavo::Mi) => Ok(SemanticObjectId::speaker()),
            Some(Cmavo::Do) => Ok(SemanticObjectId::addressee()),
            Some(Cmavo::Ma) => self.build_argument_parameter(token, raw),
            Some(Cmavo::Cehu) => {
                self.build_parameter(token, raw, crate::model::ParameterRole::PropertySlot)
            }
            Some(Cmavo::Keha) => {
                self.build_parameter(token, raw, crate::model::ParameterRole::RelativeClauseHead)
            }
            Some(Cmavo::Zohe) => self.build_elided_referent(Some(raw), "zo'e".to_owned()),
            _ => self.build_plain_referent(
                raw,
                ReferentCategory::Constant,
                Descriptor {
                    kind: "proSumti".to_owned(),
                    word: token_text(&token.value),
                    speaker: None,
                    body: None,
                    quantity: None,
                    name: None,
                },
                Vec::new(),
            ),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_parameter(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_parameter(token, raw, crate::model::ParameterRole::ArgumentQuestion)?;
        self.parameter_slots.push(QuestionSlot {
            parameter: id,
            role: QuestionSlotRole::Answer,
        });
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_parameter(
        &mut self,
        token: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
        role: crate::model::ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_parameter();
        self.insert(
            id,
            SemanticObject::parameter(
                SemanticSort::Entity,
                role,
                token_text(&token.value),
                self.source_for_node(raw, "parameter"),
            ),
        )
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_elided_referent(
        &mut self,
        raw: Option<RawSyntaxNodeId>,
        label: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "elided".to_owned(),
                    word: label,
                    speaker: None,
                    body: None,
                    quantity: None,
                    name: None,
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
            Some(Cmavo::Le) => "speakerDescription",
            Some(Cmavo::La) => "name",
            _ => "description",
        }
        .to_owned();
        let body = if let Some(selbri) = description.selbri.as_deref() {
            Some(self.build_restrictive_formula(selbri, id)?)
        } else {
            None
        };
        let quantity = if let Some(quantifier) = description.outer_quantifier.as_deref() {
            Some(self.build_quantity_for_words(
                quantifier_first_word_text(quantifier).unwrap_or_else(|| "xo'e".to_owned()),
                Some(raw),
            )?)
        } else {
            None
        };
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind,
                    word,
                    speaker: Some(SemanticObjectId::speaker()),
                    body,
                    quantity,
                    name: None,
                }),
                None,
                self.source_for_node(raw, "description"),
                Vec::new(),
            ),
        )
    }

    #[requires(!name.is_empty())]
    #[requires(!word.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_named_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        name: String,
        word: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_plain_referent(
            raw,
            ReferentCategory::Constant,
            Descriptor {
                kind: "name".to_owned(),
                word: word.to_owned(),
                speaker: Some(SemanticObjectId::speaker()),
                body: None,
                quantity: None,
                name: Some(name),
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
            Descriptor {
                kind: "unloweredSumti".to_owned(),
                word: "sumti".to_owned(),
                speaker: None,
                body: None,
                quantity: None,
                name: None,
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
        descriptor: Descriptor,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                category,
                SemanticSort::Entity,
                None,
                Some(descriptor),
                None,
                self.source_for_node(raw, "sumti"),
                diagnostics,
            ),
        )
    }

    #[requires(!operator.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_composite_referent(
        &mut self,
        raw: RawSyntaxNodeId,
        members: Vec<SemanticObjectId>,
        operator: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Composite,
                SemanticSort::Entity,
                None,
                None,
                Some(Composition {
                    operator: operator.to_owned(),
                    members,
                    collective: None,
                }),
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
        let relation = relation_label_for_selbri(selbri);
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(referent, None));
        if let Some(place_count) = (self.relation_place_count)(&relation) {
            for place in 2..=place_count {
                arguments.insert(
                    format!("x{place}"),
                    self.build_elided_argument_for_place(place)?,
                );
            }
        }
        let predication = self.next_predication();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                PredicationMode::Restrictive,
                self.analysis
                    .syntax_index
                    .selbri_node_id(selbri)
                    .and_then(|node| self.source_for_node(node.0, "restrictive-predication")),
                Vec::new(),
            ),
        )?;
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
        raw: Option<RawSyntaxNodeId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let value = parse_decimal_integer(&text)
            .map(QuantityValue::integer)
            .unwrap_or_else(|| QuantityValue::text(text.clone()));
        let id = self.next_quantity();
        self.insert(
            id,
            SemanticObject::quantity(
                QuantityForm::Exact,
                value,
                QuantityScale::Count,
                raw.and_then(|raw| self.source_for_node(raw, "quantity")),
            ),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_quantity_to_referent(&mut self, referent: SemanticObjectId, quantity: SemanticObjectId) {
        if let Some(object) = self.objects.get_mut(&referent) {
            object.set_descriptor_quantity(quantity);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_object_diagnostic(&mut self, id: SemanticObjectId, diagnostic: SemanticDiagnostic) {
        let Some(object) = self.objects.get_mut(&id) else {
            return;
        };
        object.push_diagnostic(diagnostic);
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_first_word_text(quantifier: &QuantifierSyntax) -> Option<String> {
    match quantifier.as_data() {
        data!(QuantifierSyntax::NumberQuantifier { number, .. }) => {
            Some(word_run_text(&number.value))
        }
        data!(QuantifierSyntax::MeksoQuantifier { .. }) => None,
    }
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
fn time_relation_for_selbri(selbri: &SelbriSyntax) -> Option<String> {
    match selbri.as_data() {
        data!(SelbriSyntax::TaggedSelbri {
            tense_modal,
            inner_selbri,
        }) => time_relation_for_tense_modal(tense_modal)
            .or_else(|| time_relation_for_selbri(inner_selbri)),
        data!(SelbriSyntax::GroupedSelbri {
            ke_tense_modal,
            selbri,
            ..
        }) => ke_tense_modal
            .as_deref()
            .and_then(time_relation_for_tense_modal)
            .or_else(|| time_relation_for_selbri(selbri)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn time_relation_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<String> {
    match tense_modal.as_data() {
        data!(TenseModalSyntax::Composite { parts }) => {
            parts.value.iter().find_map(|part| match part.as_data() {
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Cmavo(
                    token
                )) => time_relation_for_pu_token(token),
                data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::AdHocModal(..)) => None,
            })
        }
        data!(TenseModalSyntax::TimeDirection(word)) => time_relation_for_pu_token(&word.value),
        data!(TenseModalSyntax::TimeDirectionDistance { pu, .. })
        | data!(TenseModalSyntax::TimeDirectionActuality { pu, .. }) => {
            time_relation_for_pu_token(pu)
        }
        _ => None,
    }
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
#[ensures(!ret.is_empty())]
fn relation_label_for_selbri(selbri: &SelbriSyntax) -> String {
    match selbri.as_data() {
        data!(SelbriSyntax::SelbriWord(token)) => token_text(token),
        data!(SelbriSyntax::Tanru(units)) => units
            .iter()
            .map(relation_label_for_tanru_unit)
            .collect::<Vec<_>>()
            .join(" "),
        data!(SelbriSyntax::ConvertedSelbri { inner_selbri, .. }) => {
            format!("converted {}", relation_label_for_selbri(inner_selbri))
        }
        data!(SelbriSyntax::Negated { inner_selbri, .. }) => {
            format!("scalar-not {}", relation_label_for_selbri(inner_selbri))
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
            trailing_selbri,
            ..
        })
        | data!(SelbriSyntax::BoundSelbriConnection {
            leading_selbri,
            trailing_selbri,
            ..
        }) => format!(
            "{} connected {}",
            relation_label_for_selbri(leading_selbri),
            relation_label_for_selbri(trailing_selbri)
        ),
        data!(SelbriSyntax::Abstraction(abstraction)) => token_text(&abstraction.nu.value),
        data!(SelbriSyntax::ForethoughtSelbriConnection { .. }) => "connected-bridi".to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relation_label_for_tanru_unit(unit: &TanruUnitSyntax) -> String {
    match unit.as_data() {
        data!(TanruUnitSyntax::TanruUnitWord(token)) => token_text(&token.value),
        data!(TanruUnitSyntax::ProBridi { goha, .. }) => token_text(&goha.value),
        data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ScalarNegatedTanruUnit { inner_unit, .. })
        | data!(TanruUnitSyntax::ModalConversion { inner_unit, .. }) => {
            relation_label_for_tanru_unit(inner_unit)
        }
        data!(TanruUnitSyntax::GroupedTanruUnit { selbri, .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(selbri)) => relation_label_for_selbri(selbri),
        data!(TanruUnitSyntax::BoundTanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        })
        | data!(TanruUnitSyntax::TanruUnitConnection {
            leading_unit,
            trailing_unit,
            ..
        }) => format!(
            "{} connected {}",
            relation_label_for_tanru_unit(leading_unit),
            relation_label_for_tanru_unit(trailing_unit)
        ),
        data!(TanruUnitSyntax::RelativeClauses { base, .. })
        | data!(TanruUnitSyntax::LinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::PreposedLinkedSumtiTanruUnit { base, .. })
        | data!(TanruUnitSyntax::AssignedProBridi { base, .. }) => {
            relation_label_for_tanru_unit(base)
        }
        data!(TanruUnitSyntax::Abstraction(abstraction)) => token_text(&abstraction.nu.value),
        data!(TanruUnitSyntax::SumtiSelbri { .. }) => "me-sumti".to_owned(),
        data!(TanruUnitSyntax::QuotedWordSelbri(token))
        | data!(TanruUnitSyntax::QuotedBridiSelbri(token))
        | data!(TanruUnitSyntax::QuotedTextSelbri(token)) => token_text(&token.value),
        data!(TanruUnitSyntax::TextSelbri { .. }) => "text-selbri".to_owned(),
        data!(TanruUnitSyntax::OrdinalSelbri { number, .. }) => {
            format!("{} moi", word_run_text(number))
        }
        data!(TanruUnitSyntax::OperatorSelbri { .. }) => "operator-selbri".to_owned(),
        data!(TanruUnitSyntax::TagSelbri { .. }) => "tag-selbri".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_morphology::{
        MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
    };
    use jbotci_source::SourceId;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_source_and_options};
    use serde_json::Value;

    #[requires(!source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|value| value.get("objects").is_some()) || ret.is_err())]
    fn semantic_json_for(source: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let words = segment_words_with_modifiers_with_options_and_source_id(
            source,
            &MorphologyOptions::default(),
            Some(SourceId("<test>".to_owned())),
        )?;
        let parsed =
            parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())?;
        let graph = build_semantic_graph_with_dictionary(
            &parsed.parse_tree,
            Some(source),
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
    fn statement_connective_is_not_silently_lowered_to_plain_sequence() {
        let json = semantic_json_for("mi klama .ije do cadzu").expect("semantic JSON");
        let sequence = object(&json, "sequence:s1");
        assert_eq!(json["root"], "sequence:s1");
        assert_eq!(sequence["items"][0], "utterance:u1");
        assert!(
            sequence["diagnostics"][0]["message"]
                .as_str()
                .is_some_and(|message| message.contains("truth-functional lowering"))
        );
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
