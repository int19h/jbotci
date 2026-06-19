//! Syntax-to-semantic-graph builder for the public JSON semantics model.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};
use jbotci_morphology::{Cmavo, Word, strip_diacritics};
use jbotci_syntax::ast::{
    BoGroupedBridiTailSyntax, BridiSyntax, BridiTailSyntax, ConnectiveSyntax, ConnectiveSyntaxData,
    DescriptionSyntax, ParagraphStatementSyntax, QuantifierSyntax, QuantifierSyntaxData,
    RelativeClauseSyntax, RelativeClauseSyntaxData, SelbriSyntax, SelbriSyntaxData,
    SimpleBridiTailSyntaxData, StatementSyntax, StatementSyntaxData, SubbridiSyntax,
    SubbridiSyntaxData, SumtiSyntax, SumtiSyntaxData, TanruUnitSyntax, TanruUnitSyntaxData,
    TenseModalSyntax, TenseModalSyntaxData, TextSyntax, Token, WithFreeModifiers, WordRun,
};

use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorRelation, ArgumentValue, Composition,
    Connector, Descriptor, EventualityClass, FormulaOperator, IndexicalKind, ModalArgument,
    PredicationMode, QuantityForm, QuantityScale, QuantityValue, QuestionKind, QuestionMode,
    QuestionSlot, QuestionSlotRole, ReferentCategory, RelativeClause, RelativeClauseKind,
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

#[invariant(true)]
#[derive(Debug, Clone)]
struct TanruFormulaForArgument {
    formula: SemanticObjectId,
    x1_argument: ArgumentValue,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct BoundSelbriTanruPair<'tree> {
    leading: &'tree SelbriSyntax,
    trailing: &'tree SelbriSyntax,
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
    fn next_abstraction(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::abstraction(self.counters.abstraction);
        self.counters.abstraction += 1;
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
        if bridi.bridi_tail.ke_continuation.is_none()
            && !bridi.bridi_tail.first.continuations.is_empty()
        {
            return self.build_afterthought_bridi_tail_formula(bridi);
        }
        let selbri = main_selbri_for_tail(&bridi.bridi_tail);
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
        let relation = selbri
            .map(relation_label_for_selbri)
            .unwrap_or_else(|| "unknown-relation".to_owned());
        if let Some(formula) =
            self.build_logical_sumti_connection_formula(bridi, selbri, relation.clone())?
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
        let mut alternatives = BTreeMap::<String, Vec<ArgumentValue>>::new();
        let mut highest_assigned_place = 0usize;
        let mut connector = None;
        let mut operator = FormulaOperator::And;
        let mut assigned_sumtis = Vec::new();
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
            if let Some((_leading_sumti, connective, _trailing_sumti)) =
                logical_sumti_connection_parts(sumti)
            {
                if connector.is_none() {
                    operator = formula_operator_for_connective(connective);
                    connector = Some(Connector {
                        source: connective_text(connective),
                        locus: "sumti".to_owned(),
                        truth_table: None,
                    });
                }
            }
            assigned_sumtis.push((key, sumti));
        }
        let Some(connector) = connector else {
            return Ok(None);
        };
        for (key, sumti) in assigned_sumtis {
            if let Some((leading_sumti, _connective, trailing_sumti)) =
                logical_sumti_connection_parts(sumti)
            {
                alternatives.insert(
                    key,
                    vec![
                        self.build_argument_for_sumti(leading_sumti)?,
                        self.build_argument_for_sumti(trailing_sumti)?,
                    ],
                );
            } else {
                alternatives.insert(key, vec![self.build_argument_for_sumti(sumti)?]);
            }
        }
        let fill_through =
            (self.relation_place_count)(&relation).unwrap_or_else(|| highest_assigned_place.max(1));
        for place in 1..=fill_through {
            let key = format!("x{place}");
            if !alternatives.contains_key(&key) {
                alternatives.insert(key, vec![self.build_elided_argument_for_place(place)?]);
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
        for arguments in branches {
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
            children.push(formula);
        }
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(connector),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "sumti-connection-formula")),
                Vec::new(),
            ),
        )
        .map(Some)
    }

    #[requires(!bridi.bridi_tail.first.continuations.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_afterthought_bridi_tail_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut children = Vec::new();
        children.push(self.build_bo_grouped_tail_formula(&bridi.bridi_tail.first.first)?);
        let mut connector = None;
        let mut operator = FormulaOperator::And;
        for continuation in &bridi.bridi_tail.first.continuations {
            if connector.is_none() {
                operator = formula_operator_for_connective(&continuation.connective);
                connector = Some(Connector {
                    source: connective_text(&continuation.connective),
                    locus: "bridiTail".to_owned(),
                    truth_table: None,
                });
            }
            children.push(self.build_bo_grouped_tail_formula(&continuation.bridi_tail)?);
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
                    .and_then(|node| self.source_for_node(node.0, "compound-bridi-formula")),
                Vec::new(),
            ),
        )
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
        let relation = relation_label_for_selbri(selbri);
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

    #[requires(tanru_units_require_lowering(units))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_tanru_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
        units: &[&'tree TanruUnitSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !tanru_sequence_has_explicit_grouping(units) {
            return self.build_flat_tanru_formula_for_bridi(bridi, selbri, units);
        }
        self.build_tanru_sequence_formula_for_frame(
            Some(selbri),
            units,
            self.bridi_frame(bridi),
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "tanru-formula")),
        )
        .map(|result| result.formula)
    }

    #[requires(units.len() > 1)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_flat_tanru_formula_for_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        selbri: &'tree SelbriSyntax,
        units: &[&'tree TanruUnitSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let tertau = units
            .last()
            .expect("precondition guarantees at least one tanru unit");
        let tertau_relation = relation_label_for_tanru_unit(tertau);
        let tertau_predication =
            self.build_predication_for_bridi(bridi, Some(selbri), tertau_relation)?;
        let tertau_formula = self.next_formula();
        self.insert(
            tertau_formula,
            SemanticObject::atom_formula(
                tertau_predication,
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "tertau-formula")),
                Vec::new(),
            ),
        )?;
        let x1_argument = self
            .objects
            .get(&tertau_predication)
            .and_then(|object| object.arguments.get("x1"))
            .cloned()
            .ok_or_else(|| {
                SemanticsError::invalid_graph("tanru tertau has no x1 argument".to_owned())
            })?;
        let modifier = self.build_property_abstraction_for_units(
            &units[..units.len() - 1],
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "tanru-modifier")),
        )?;
        let relation_formula = self.build_tanru_relation_formula(
            x1_argument,
            modifier,
            tanru_relation_name(units),
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "tanru-relation")),
        )?;
        let formula = self.next_formula();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau_formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: "selbri".to_owned(),
                    truth_table: None,
                }),
                self.analysis
                    .syntax_index
                    .bridi_node_id(bridi)
                    .and_then(|node| self.source_for_node(node.0, "tanru-formula")),
                Vec::new(),
            ),
        )
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

    #[requires(!units.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_abstraction_for_units(
        &mut self,
        units: &[&'tree TanruUnitSyntax],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
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
                Some(Composition {
                    operator,
                    members: vec![leading, trailing],
                    collective,
                }),
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
                    }),
                    source,
                    Vec::new(),
                ),
            );
        }
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        self.build_property_atom_for_relation(
            relation_label_for_selbri(selbri),
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
            _ => {
                let frame = self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit),
                );
                self.build_property_atom_for_relation(
                    relation_label_for_tanru_unit(unit),
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
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_atom_for_relation(
        &mut self,
        relation: String,
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
        let mut diagnostics = Vec::new();
        match (self.relation_place_count)(&relation) {
            Some(place_count) => {
                for place in 2..=place_count {
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
                diagnostics.push(diagnostic(
                    "relation place structure is unavailable; only explicit assigned places are represented",
                ));
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
                PredicationMode::Asserted,
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
        self.build_predication_for_frame(
            frame,
            self.analysis
                .syntax_index
                .bridi_node_id(bridi)
                .and_then(|node| self.source_for_node(node.0, "predication")),
            selbri,
            relation,
        )
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
        let eventuality = self.next_eventuality();
        let mut event = SemanticObject::eventuality(
            EventualityClass::Event,
            Some(Actuality {
                kind: ActualityKind::Actual,
            }),
            source.clone(),
        );
        if let Some(relation) = selbri.and_then(time_relation_for_selbri) {
            event.time = Some(AnchorRelation {
                relation,
                anchor: SemanticObjectId::speech_time(),
            });
        }
        self.insert(eventuality, event)?;
        let mut arguments = BTreeMap::new();
        let mut highest_assigned_place =
            self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        for (place, argument) in argument_overrides {
            if let Some(place_index) = argument_place_index(&place) {
                highest_assigned_place = highest_assigned_place.max(place_index);
            }
            arguments.entry(place).or_insert(argument);
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
            None => {
                for place in 1..=highest_assigned_place.max(1) {
                    let key = format!("x{place}");
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
                diagnostics.push(diagnostic(
                    "relation place structure is unavailable; only places required by explicit assignments are represented",
                ));
            }
        }
        let id = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source,
            diagnostics,
        );
        object.modal_arguments = modal_arguments;
        self.insert(id, object)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_numbered_assignment_arguments(
        &mut self,
        arguments: &mut BTreeMap<String, ArgumentValue>,
        frame: Option<SelbriPlaceFrameId>,
    ) -> Result<usize, SemanticsError> {
        let Some(frame) = frame else {
            return Ok(0);
        };
        let mut highest_assigned_place = 0usize;
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
            let argument = self.build_argument_for_sumti(sumti)?;
            let place = place.get() as usize;
            highest_assigned_place = highest_assigned_place.max(place);
            arguments.insert(format!("x{place}"), argument);
        }
        Ok(highest_assigned_place)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn modal_assignment_arguments(
        &mut self,
        frame: Option<SelbriPlaceFrameId>,
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
            let PlaceSlot::Modal(tag_node) = assignment.slot else {
                continue;
            };
            let sumti = self
                .analysis
                .syntax_index
                .sumti(assignment.sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?;
            let argument = self.build_argument_for_sumti(sumti)?;
            let source = tag_node.and_then(|node| self.source_for_node(node, "modal-argument"));
            let introduced_by = source
                .as_ref()
                .and_then(|source| source.text.clone())
                .unwrap_or_else(|| "modal".to_owned());
            let relation = modal_relation_for_marker(&introduced_by);
            modal_arguments.push(ModalArgument::new(
                relation,
                introduced_by,
                argument,
                source,
            ));
        }
        Ok(modal_arguments)
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
        let mut event = SemanticObject::eventuality(
            EventualityClass::Event,
            Some(Actuality {
                kind: ActualityKind::Actual,
            }),
            source.clone(),
        );
        if let Some(relation) = selbri.and_then(time_relation_for_selbri) {
            event.time = Some(AnchorRelation {
                relation,
                anchor: SemanticObjectId::speech_time(),
            });
        }
        self.insert(eventuality, event)?;
        let id = self.next_predication();
        self.insert(
            id,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                PredicationMode::Asserted,
                source,
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
        let argument = ArgumentValue::filled(referent, None);
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
        let clauses = match sumti.as_data() {
            data!(SumtiSyntax::SumtiWithRelativeClauses {
                relative_clauses,
                ..
            })
            | data!(SumtiSyntax::SumtiWithComplexRelativeClauses {
                relative_clauses,
                ..
            }) => relative_clauses.as_slice(),
            data!(SumtiSyntax::Description(description)) => description.relative_clauses.as_slice(),
            data!(SumtiSyntax::DescriptionConnection(description)) => {
                description.relative_clauses.as_slice()
            }
            _ => return Ok(argument),
        };
        let mut lowered = Vec::new();
        for clause in clauses {
            if let Some(clause) = self.build_relative_clause(clause, head)? {
                lowered.push(clause);
            }
        }
        if lowered.is_empty() {
            Ok(argument)
        } else {
            Ok(argument.with_relative_clauses(lowered))
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relative_clause(
        &mut self,
        clause: &'tree RelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        match clause.as_data() {
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
            data!(RelativeClauseSyntax::SumtiAssociationPhrase(..)) => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relative_bridi_clause(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
    ) -> Result<RelativeClause, SemanticsError> {
        let Some(selbri) = main_selbri_for_subbridi(subbridi) else {
            let formula = self.build_diagnostic_relative_formula(subbridi)?;
            return Ok(RelativeClause::new(
                kind,
                formula,
                self.source_for_subbridi(subbridi, "relative-clause"),
            ));
        };
        let mode = match kind {
            RelativeClauseKind::Incidental => PredicationMode::Incidental,
            RelativeClauseKind::Restrictive => PredicationMode::Restrictive,
        };
        let formula = self.build_referent_predication_formula_for_selbri(
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
        let mut arguments = BTreeMap::new();
        self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        arguments.insert(
            format!("x{visible_x1_place}"),
            ArgumentValue::filled(referent, None),
        );
        if let Some(place_count) = (self.relation_place_count)(&relation) {
            for place in 1..=place_count {
                let key = format!("x{place}");
                if !arguments.contains_key(&key) {
                    arguments.insert(key, self.build_elided_argument_for_place(place)?);
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
        let referent = self.build_elided_referent(None, format!("zo'e x{place}"))?;
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
            data!(TanruUnitSyntax::ConvertedTanruUnit { inner_unit, .. }) => self
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
            data!(SumtiSyntax::SumtiWithRelativeClauses { base_sumti, .. })
            | data!(SumtiSyntax::SumtiWithComplexRelativeClauses { base_sumti, .. }) => {
                self.build_sumti_referent(base_sumti)?
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
            Some(Cmavo::Ti) => {
                self.build_demonstrative_referent(raw, IndexicalKind::ProximalDemonstrative)
            }
            Some(Cmavo::Ta) => {
                self.build_demonstrative_referent(raw, IndexicalKind::MedialDemonstrative)
            }
            Some(Cmavo::Tu) => {
                self.build_demonstrative_referent(raw, IndexicalKind::DistalDemonstrative)
            }
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
        let frame = self
            .semantic_predication_frame_for_selbri(selbri, self.branch_frame_for_selbri(selbri));
        let visible_x1_place = visible_x1_place_for_selbri(selbri);
        let mut arguments = BTreeMap::new();
        self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        let modal_arguments = self.modal_assignment_arguments(frame)?;
        arguments.insert(
            format!("x{visible_x1_place}"),
            ArgumentValue::filled(referent, None),
        );
        if let Some(place_count) = (self.relation_place_count)(&relation) {
            for place in 1..=place_count {
                let key = format!("x{place}");
                if !arguments.contains_key(&key) {
                    arguments.insert(key, self.build_elided_argument_for_place(place)?);
                }
            }
        }
        let predication = self.next_predication();
        let mut object = SemanticObject::predication(
            relation,
            None,
            arguments,
            PredicationMode::Restrictive,
            self.analysis
                .syntax_index
                .selbri_node_id(selbri)
                .and_then(|node| self.source_for_node(node.0, "restrictive-predication")),
            Vec::new(),
        );
        object.modal_arguments = modal_arguments;
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
) -> Option<(&SumtiSyntax, &ConnectiveSyntax, &SumtiSyntax)> {
    match sumti.as_data() {
        data!(SumtiSyntax::SumtiConnection {
            leading_sumti,
            connective,
            trailing_sumti,
        }) if connective_is_logical(connective) => {
            Some((leading_sumti, connective, trailing_sumti))
        }
        data!(SumtiSyntax::BoundSumtiConnection {
            leading_sumti,
            bo_connective,
            trailing_sumti,
            ..
        }) => bo_connective
            .as_deref()
            .filter(|connective| connective_is_logical(connective))
            .map(|connective| (leading_sumti.as_ref(), connective, trailing_sumti.as_ref())),
        data!(SumtiSyntax::ForethoughtSumtiConnection {
            leading_sumti,
            gek,
            trailing_sumti,
            ..
        }) if connective_is_logical(gek) => Some((leading_sumti, gek, trailing_sumti)),
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

#[requires(!marker.is_empty())]
#[ensures(!ret.is_empty())]
fn modal_relation_for_marker(marker: &str) -> String {
    match marker {
        "ga'a" => "observer".to_owned(),
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
#[ensures(!ret.is_empty())]
fn nonlogical_composition_operator(connective: &ConnectiveSyntax) -> String {
    match connective_text(connective).as_str() {
        "jo'u" => "joint".to_owned(),
        "joi" => "mass".to_owned(),
        "ce" => "set".to_owned(),
        "ce'o" => "sequence".to_owned(),
        "fa'u" => "respectively".to_owned(),
        other => format!("nonlogical:{other}"),
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
            relation_label_for_selbri(inner_selbri)
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
        data!(SelbriSyntax::Abstraction(abstraction)) => token_text(&abstraction.nu.value),
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
fn tanru_units_for_selbri(selbri: &SelbriSyntax) -> Option<Vec<&TanruUnitSyntax>> {
    match selbri.as_data() {
        data!(SelbriSyntax::Tanru(units)) => Some(units.iter().collect()),
        data!(SelbriSyntax::GroupedSelbri { selbri, .. })
        | data!(SelbriSyntax::TaggedSelbri {
            inner_selbri: selbri,
            ..
        }) => tanru_units_for_selbri(selbri),
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
    units.len() > 1 || tanru_sequence_has_explicit_grouping(units)
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
        assert_eq!(blanu["modalArguments"][0]["relation"], "observer");
        assert_eq!(blanu["modalArguments"][0]["introducedBy"], "ga'a");
        assert_eq!(blanu["modalArguments"][0]["argument"]["kind"], "filled");
        assert_eq!(
            blanu["modalArguments"][0]["argument"]["value"],
            "referent:speaker"
        );
        let zdani = predication_with_relation_and_mode(&linked, "zdani", "asserted");
        assert!(zdani.get("modalArguments").is_none());

        let tail = semantic_json_for("ta blanu zdani ga'a mi").expect("semantic JSON");
        let zdani = predication_with_relation_and_mode(&tail, "zdani", "asserted");
        assert_eq!(zdani["modalArguments"][0]["relation"], "observer");
        assert_eq!(
            zdani["modalArguments"][0]["argument"]["value"],
            "referent:speaker"
        );
        let blanu = predication_with_relation_and_mode(&tail, "blanu", "restrictive");
        assert!(blanu.get("modalArguments").is_none());
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
        assert_eq!(
            zdani["arguments"]["x1"]["relativeClauses"][0]["kind"],
            "incidental"
        );
        assert_eq!(
            zdani["arguments"]["x1"]["relativeClauses"][0]["body"],
            "formula:f2"
        );
        let barda = predication_with_relation_and_mode(&outer, "barda", "incidental");
        assert_eq!(
            barda["arguments"]["x1"]["value"],
            zdani["arguments"]["x1"]["value"]
        );
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
        assert_eq!(object(&json, "utterance:u1")["content"], "formula:f4");
        assert_eq!(object(&json, "formula:f4")["operator"], "and");
        assert_eq!(object(&json, "formula:f4")["connector"]["source"], "e");
        assert_eq!(
            object(&json, "predication:p2")["arguments"]["x1"]["value"],
            "referent:r1"
        );
        assert_eq!(
            object(&json, "predication:p3")["arguments"]["x1"]["value"],
            "referent:r2"
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
