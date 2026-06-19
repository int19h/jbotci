//! Syntax-to-semantic-graph builder for the public JSON semantics model.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};
use jbotci_morphology::{Cmavo, Word, WordLike, WordLikeData, strip_diacritics};
use jbotci_syntax::ast::{
    AbstractionSyntax, BoGroupedBridiTailSyntax, BridiSyntax, BridiTailSyntax, ConnectiveSyntax,
    ConnectiveSyntaxData, DescriptionSyntax, FragmentSyntax, FragmentSyntaxData,
    FreeModifierSyntax, FreeModifierSyntaxData, MeksoOperatorSyntax, MeksoOperatorSyntaxData,
    MeksoSyntax, MeksoSyntaxData, ParagraphStatementSyntax, QuantifierSyntax, QuantifierSyntaxData,
    QuoteSyntax, QuoteSyntaxData, RelativeClauseSyntax, RelativeClauseSyntaxData, SelbriSyntax,
    SelbriSyntaxData, SimpleBridiTailSyntaxData, StatementSyntax, StatementSyntaxData,
    SubbridiSyntax, SubbridiSyntaxData, SumtiSyntax, SumtiSyntaxData, TanruUnitSyntax,
    TanruUnitSyntaxData, TenseModalSyntax, TenseModalSyntaxData, TermSyntax, TermSyntaxData,
    TextSyntax, Token, WithFreeModifiers, WordRun,
};

use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorRelation, ArgumentValue, Composition,
    Connector, Descriptor, EventualityClass, FormulaOperator, IndexicalKind, MathLiteral,
    ModalArgument, PredicationMode, QuantityForm, QuantityScale, QuantityValue, QuestionKind,
    QuestionMode, QuestionSlot, QuestionSlotRole, Quotation, ReferentCategory, RelativeClause,
    RelativeClauseKind, ScalarNegation, ScalarNegationKind, SemanticDiagnostic, SemanticGraph,
    SemanticObject, SemanticObjectId, SemanticSort, SequenceRelation, SignKind, UtteranceForce,
    diagnostic, source_from_spans,
};
use crate::references::{
    PlaceFrameKind, PlaceSlot, RawSyntaxNodeId, ReferenceAnalysis, ReferenceAnalysisError,
    ReferenceKind, ReferenceTarget, SelbriPlaceFrameId, analyze_references,
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
    build_semantic_graph_with_place_resolver(syntax, source_text, |relation| {
        dictionary_relation_place_count(dictionary, relation)
    })
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
#[derive(Debug, Clone, Copy)]
struct BoundSelbriTanruPair<'tree> {
    leading: &'tree SelbriSyntax,
    trailing: &'tree SelbriSyntax,
}

#[invariant(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[derive(Debug, Clone)]
struct QuantifiedProSumtiScope {
    variable: SemanticObjectId,
    quantity: SemanticObjectId,
    operator: FormulaOperator,
    source: Option<crate::model::SemanticSource>,
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
    sumti_quantities: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    utterance_objects: HashMap<RawSyntaxNodeId, SemanticObjectId>,
    parameter_slots: Vec<QuestionSlot>,
    pending_asides: Vec<SemanticObjectId>,
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
            sumti_quantities: HashMap::new(),
            utterance_objects: HashMap::new(),
            parameter_slots: Vec::new(),
            pending_asides: Vec::new(),
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
    fn next_sign(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::sign(self.counters.sign);
        self.counters.sign += 1;
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
        for paragraph in &text.paragraphs {
            let mut paragraph_asides = self.build_vocative_asides(&paragraph.free_modifiers)?;
            let first_paragraph_item = items.len();
            for statement in &paragraph.statements {
                if let Some(statement_id) =
                    self.build_paragraph_statement(statement, truth_question)?
                {
                    items.push(statement_id);
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
        self.build_utterance(UtteranceForce::Mention, Some(sign), source, Vec::new())
            .map(Some)
    }

    #[requires(true)]
    #[ensures(true)]
    fn build_paragraph_statement(
        &mut self,
        statement: &'tree ParagraphStatementSyntax,
        truth_question: bool,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut asides = self.build_vocative_asides(&statement.free_modifiers)?;
        let Some(statement) = statement.statement.as_deref() else {
            return self.build_standalone_asides(asides);
        };
        let statement_id = self.build_statement(statement, truth_question)?;
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
        let audience = if let Some(sumti) = sumti.as_deref() {
            self.build_sumti_referent(sumti)?
        } else {
            SemanticObjectId::addressee()
        };
        let diagnostics = if audience.object_kind() == crate::model::SemanticObjectKind::Referent {
            Vec::new()
        } else {
            vec![diagnostic(
                "vocative target is not referent-valued; audience remains contextual",
            )]
        };
        let id = self.build_utterance(
            UtteranceForce::Vocative,
            None,
            self.source_for_free_modifier(free_modifier, "vocative"),
            diagnostics,
        )?;
        if audience.object_kind() == crate::model::SemanticObjectKind::Referent {
            self.set_utterance_audience(id, audience);
        }
        self.set_vocative_kind(id, vocative_kind_for_markers(vocative_markers));
        Ok(Some(id))
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
            data!(StatementSyntax::Fragment(fragment)) => {
                self.build_fragment_utterance(statement, fragment)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_fragment_utterance(
        &mut self,
        statement: &'tree StatementSyntax,
        fragment: &'tree FragmentSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self
            .analysis
            .syntax_index
            .statement_node_id(statement)
            .and_then(|node| self.source_for_node(node.0, "fragment"));
        if let Some(content) = self.build_fragment_content(fragment)? {
            return self.build_utterance(
                UtteranceForce::Mention,
                Some(content),
                source,
                Vec::new(),
            );
        }
        self.build_utterance(
            UtteranceForce::Mention,
            None,
            source,
            vec![diagnostic("fragment has no truth-bearing semantic formula")],
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_fragment_content(
        &mut self,
        fragment: &'tree FragmentSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match fragment.as_data() {
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
            data!(TermSyntax::Sumti(sumti)) | data!(TermSyntax::PlaceTaggedSumti { sumti, .. }) => {
                self.build_sumti_referent(sumti).map(Some)
            }
            _ => Ok(None),
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
        nested.utterance_objects = std::mem::take(&mut self.utterance_objects);
        let graph = nested.build_text(text)?;
        self.counters = nested.counters;
        self.objects = graph.objects;
        self.utterance_objects = nested.utterance_objects;
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
        let previous_asides = std::mem::take(&mut self.pending_asides);
        let formula = self.build_bridi_formula(bridi)?;
        let formula = self.wrap_bridi_formula_with_quantified_pro_sumti(bridi, formula)?;
        let slots = std::mem::replace(&mut self.parameter_slots, previous_slots);
        let asides = std::mem::replace(&mut self.pending_asides, previous_asides);
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
        )?;
        if let Some(node) = self.analysis.syntax_index.bridi_node_id(bridi) {
            self.utterance_objects.insert(node.0, utterance);
        }
        self.add_utterance_asides(utterance, asides);
        Ok(utterance)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_bridi_formula_with_quantified_pro_sumti(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut scopes = self.quantified_pro_sumti_scopes_for_bridi(bridi)?;
        let mut body = formula;
        while let Some(scope) = scopes.pop() {
            let data!(QuantifiedProSumtiScope {
                variable,
                quantity,
                operator,
                source,
            }) = scope.into_data();
            let restriction = self.restriction_formula_for_variable_in_formula(body, variable)?;
            let formula = self.next_formula();
            self.insert(
                formula,
                SemanticObject::quantified_formula(
                    operator,
                    variable,
                    restriction,
                    body,
                    Some(quantity),
                    source,
                    Vec::new(),
                ),
            )?;
            body = formula;
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
            let Some(quantified_sumti) = quantified_da_series_sumti(sumti) else {
                continue;
            };
            let data!(SumtiSyntax::QuantifiedSumti { quantifier, .. }) = quantified_sumti.as_data()
            else {
                continue;
            };
            let raw = self
                .analysis
                .syntax_index
                .sumti_node_id(quantified_sumti)
                .ok_or_else(SemanticsError::missing_syntax_node)?
                .0;
            let variable = self.build_sumti_referent(sumti)?;
            let quantity = self.build_quantity_for_sumti_quantifier(raw, quantifier)?;
            scopes.push(QuantifiedProSumtiScope::from_data(data!(
                QuantifiedProSumtiScope {
                    variable,
                    quantity,
                    operator: quantified_pro_sumti_formula_operator(quantifier),
                    source: self.source_for_node(raw, "quantifier-scope"),
                }
            )));
        }
        Ok(scopes)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restriction| restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn restriction_formula_for_variable_in_formula(
        &mut self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut restrictions = Vec::new();
        self.collect_restriction_formulas_for_variable(formula, variable, &mut restrictions);
        restrictions.sort_unstable();
        restrictions.dedup();
        match restrictions.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            _ => {
                let conjunction = self.next_formula();
                self.insert(
                    conjunction,
                    SemanticObject::connective_formula(
                        FormulaOperator::And,
                        restrictions,
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
            && let Some(formula) = self.build_scoped_selbri_formula_for_bridi(bridi, selbri)?
        {
            return Ok(formula);
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
        let eventuality = if let Some(relation) = time_relation_for_tense_modal(tense_modal) {
            let eventuality = self.next_eventuality();
            let mut event = SemanticObject::eventuality(
                EventualityClass::Event,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                source.clone(),
            );
            event.time = Some(AnchorRelation {
                relation,
                anchor: SemanticObjectId::speech_time(),
            });
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
                        source: full_connective_text(connective),
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
            if let Some((leading_sumti, connective, trailing_sumti)) =
                logical_sumti_connection_parts(sumti)
            {
                alternatives.insert(
                    key,
                    vec![
                        AlternativeArgument::new(
                            self.build_argument_for_sumti(leading_sumti)?,
                            connective_negates_left(connective),
                        ),
                        AlternativeArgument::new(
                            self.build_argument_for_sumti(trailing_sumti)?,
                            connective_negates_right(connective),
                        ),
                    ],
                );
            } else {
                alternatives.insert(
                    key,
                    vec![AlternativeArgument::new(
                        self.build_argument_for_sumti(sumti)?,
                        false,
                    )],
                );
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
        let source_referent = self.build_sumti_referent(sumti)?;
        let mut arguments = BTreeMap::new();
        self.insert_numbered_assignment_arguments(&mut arguments, frame)?;
        if let Some(argument) = visible_x1_override {
            arguments.insert("x1".to_owned(), argument);
        }
        if !arguments.contains_key("x1") {
            arguments.insert("x1".to_owned(), self.build_elided_argument_for_place(1)?);
        }
        arguments.insert(
            "x2".to_owned(),
            ArgumentValue::filled(source_referent, None),
        );
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
            data!(TanruUnitSyntax::SumtiSelbri { sumti, .. }) => self
                .build_sumti_selbri_formula_for_frame(
                    sumti,
                    self.branch_frame_for_tanru_unit(unit),
                    source,
                    Some(ArgumentValue::filled(parameter, None)),
                    PredicationMode::Restrictive,
                )
                .map(|result| result.formula),
            data!(TanruUnitSyntax::ScalarNegatedTanruUnit { nahe, inner_unit }) => {
                let frame = self.semantic_predication_frame_for_tanru_unit(
                    unit,
                    self.branch_frame_for_tanru_unit(unit),
                );
                self.build_property_atom_for_relation_with_scalar_negation(
                    relation_label_for_tanru_unit(inner_unit),
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
        self.build_property_atom_for_relation_with_scalar_negation(
            relation,
            parameter,
            source,
            frame,
            visible_x1_place,
            None,
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_property_atom_for_relation_with_scalar_negation(
        &mut self,
        relation: String,
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
        let mode = match kind {
            RelativeClauseKind::Incidental => PredicationMode::Incidental,
            RelativeClauseKind::Restrictive => PredicationMode::Restrictive,
        };
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
    fn build_subbridi_formula(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match subbridi.as_data() {
            data!(SubbridiSyntax::Bridi(bridi)) => {
                let formula = self.build_bridi_formula(bridi)?;
                self.wrap_bridi_formula_with_quantified_pro_sumti(bridi, formula)
                    .map(Some)
            }
            data!(SubbridiSyntax::Prenex { inner_subbridi, .. }) => {
                self.build_subbridi_formula(inner_subbridi)
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
        self.build_referent_predication_formula_for_relation(
            relation,
            frame,
            visible_x1_place,
            referent,
            mode,
            source,
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
        referent: SemanticObjectId,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
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
            data!(SumtiSyntax::QuotedSumti(quote)) => self.build_quote_sign(quote, raw)?,
            data!(SumtiSyntax::ProSumti(token)) => self.build_pro_sumti(token, raw)?,
            data!(SumtiSyntax::NumberSumti { expression, li, .. }) => {
                self.build_number_referent(expression, li, raw)?
            }
            data!(SumtiSyntax::ElidedSumti { .. }) => {
                self.build_elided_referent(Some(raw), "zo'e".to_owned())?
            }
            data!(SumtiSyntax::Description(description)) => {
                self.build_description_referent(description, raw)?
            }
            data!(SumtiSyntax::NameDescription { la, names }) => self.build_named_referent(
                raw,
                word_run_text(&names.value),
                &token_text(&la.value),
                gadri_name_sort(la.cmavo()),
            )?,
            data!(SumtiSyntax::NameWords(names)) => self.build_named_referent(
                raw,
                word_run_text(&names.value),
                "la",
                SemanticSort::Entity,
            )?,
            data!(SumtiSyntax::SelbriVocative { selbri, .. }) => {
                self.build_selbri_vocative_referent(raw, selbri)?
            }
            data!(SumtiSyntax::QuantifiedSumti {
                quantifier,
                inner_sumti,
            }) => {
                let quantity = self.build_quantity_for_sumti_quantifier(raw, quantifier)?;
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
        self.sumti_objects.insert(raw, id);
        Ok(id)
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
        expression: &MeksoSyntax,
        li: &WithFreeModifiers<Token>,
        raw: RawSyntaxNodeId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if li.cmavo() == Some(Cmavo::Meho) {
            return self.build_math_expression_sign(expression, raw);
        }

        let text = mekso_surface_text(expression);
        let quantity = self.build_quantity_for_mekso(expression, raw)?;
        let id = self.next_referent();
        self.insert(
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
                    quantity: Some(quantity),
                    name: Some(text),
                    operand: None,
                }),
                None,
                self.source_for_node(raw, "number-sumti"),
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_math_expression_sign(
        &mut self,
        expression: &MeksoSyntax,
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

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_quantity_for_mekso(
        &mut self,
        expression: &MeksoSyntax,
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
        expression: &MeksoSyntax,
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
        quantifier: &QuantifierSyntax,
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
            Some(Cmavo::Dei | Cmavo::Dihu | Cmavo::Dihe) => {
                self.build_utterance_reference_referent(token, raw)
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
        if target.is_none() {
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
        let sort = match description
            .description
            .as_ref()
            .and_then(|word| word.cmavo())
        {
            Some(Cmavo::Loi | Cmavo::Lei) => SemanticSort::Mass,
            Some(Cmavo::Lohi | Cmavo::Lehi) => SemanticSort::Set,
            Some(Cmavo::Lai) => SemanticSort::Mass,
            Some(Cmavo::Lahi) => SemanticSort::Set,
            _ => SemanticSort::Entity,
        };
        let body = if let Some(selbri) = description.selbri.as_deref() {
            Some(self.build_restrictive_formula(selbri, id)?)
        } else {
            None
        };
        let operand = self.build_description_operand(description)?;
        let quantity = self.build_description_quantity(description, raw)?;
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind,
                    word,
                    speaker: Some(SemanticObjectId::speaker()),
                    body,
                    quantity,
                    name: None,
                    operand,
                }),
                None,
                self.source_for_description(description, raw, "description"),
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
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent();
        self.sumti_objects.insert(raw, id);
        let body = self.build_restrictive_formula(selbri, id)?;
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
                quantity: None,
                name: None,
                operand: Some(operand),
            },
            Vec::new(),
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
        if let Some(units) = tanru_units_for_selbri(selbri)
            && tanru_units_require_lowering(&units)
        {
            return self.build_restrictive_tanru_formula(selbri, &units, referent);
        }
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
        if let Some(place_count) = self.place_count_for_relation(&relation) {
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
                }),
                source,
                Vec::new(),
            ),
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
        match unit.as_data() {
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
                self.build_referent_predication_formula_for_relation(
                    relation,
                    frame,
                    visible_x1_place,
                    referent,
                    PredicationMode::Restrictive,
                    source,
                )
            }
        }
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
        let quantifier = match sumti.as_data() {
            data!(SumtiSyntax::QuantifiedSumti { quantifier, .. }) => Some(quantifier),
            data!(SumtiSyntax::Description(description)) => description
                .outer_quantifier
                .as_deref()
                .or_else(|| bare_description_tail_quantifier(description)),
            _ => None,
        };
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

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_description_operand(
        &mut self,
        description: &'tree DescriptionSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        description
            .tail_elements
            .iter()
            .find_map(|element| match element.as_data() {
                data!(
                    jbotci_syntax::ast::DescriptionTailElementSyntax::DescriptionTailSumti(sumti)
                ) => Some(sumti),
                _ => None,
            })
            .map(|sumti| self.build_sumti_referent(sumti))
            .transpose()
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
        data!(TanruUnitSyntax::TanruUnitConnection { .. })
        | data!(TanruUnitSyntax::BoundTanruUnitConnection { .. })
        | data!(TanruUnitSyntax::GroupedTanruUnit { .. })
        | data!(TanruUnitSyntax::SelbriGroupTanruUnit(_)) => tanru_unit_has_explicit_grouping(unit),
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
fn relation_has_open_place_structure(relation: &str) -> bool {
    relation == "du" || relation.starts_with("nu'a ")
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
        _ => "mekso".to_owned(),
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
fn quantified_da_series_sumti(sumti: &SumtiSyntax) -> Option<&SumtiSyntax> {
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
        }) => quantified_da_series_sumti(base_sumti),
        data!(SumtiSyntax::QuantifiedSumti { inner_sumti, .. })
            if sumti_is_da_series_pro_sumti(inner_sumti) =>
        {
            Some(sumti)
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
    let mut found = false;
    subbridi.visit_words(&mut |token| {
        found |= token.cmavo() == Some(Cmavo::Keha);
    });
    found
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
        Some(Cmavo::Mihe) => "selfIdentification".to_owned(),
        Some(Cmavo::Doi) => "address".to_owned(),
        _ => token_text(first),
    }
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
fn scalar_negation_for_marker(marker: &WithFreeModifiers<Token>) -> ScalarNegation {
    ScalarNegation::new(
        scalar_negation_kind_for_cmavo(marker.cmavo()),
        token_text(&marker.value),
    )
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
    fn root_object(json: &Value) -> &Value {
        object(json, json["root"].as_str().expect("root object ID"))
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
        let event_property = predication_with_relation_and_mode(&json, "nu zdile", "restrictive");
        assert_eq!(event_property["arguments"]["x1"]["value"], "parameter:p1");
        assert!(event_property["arguments"].get("x2").is_none());
        assert!(event_property.get("diagnostics").is_none());
        let tanru =
            predication_with_relation_and_mode(&json, "R[tanru:nu zdile-kumfa]", "asserted");
        assert_eq!(tanru["arguments"]["x2"]["value"], "abstraction:a1");
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
            predication_with_relation_and_mode(&json, "R[tanru:nu'a su'i-nabmi]", "asserted");
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
