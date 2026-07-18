//! Kind-specific semantic objects and their flat JSON boundary.

use super::*;

use bityzba::{data, invariant, new};
use serde::ser::SerializeMap;

#[requires(true)]
#[ensures(true)]
fn referent_target_kind_is_allowed(target: SemanticObjectId) -> bool {
    matches!(
        target.object_kind(),
        SemanticObjectKind::Utterance
            | SemanticObjectKind::Sequence
            | SemanticObjectKind::Formula
            | SemanticObjectKind::Referent
    )
}

#[invariant(diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()), "semantic diagnostics must have messages")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticObjectCommon {
    pub source: Option<SemanticSource>,
    pub diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticObjectCommon {
    #[requires(diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()))]
    #[ensures(ret.diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()))]
    fn new(source: Option<SemanticSource>, diagnostics: Vec<SemanticDiagnostic>) -> Self {
        new!(SemanticObjectCommon {
            source,
            diagnostics
        })
    }
}

#[invariant(speaker.object_kind() == SemanticObjectKind::Referent)]
#[invariant(audience.object_kind() == SemanticObjectKind::Referent)]
#[invariant(eventuality_is_referent(*eventuality))]
#[invariant(content.is_none_or(|content| utterance_content_reference_matches_force(Some(*force), content)))]
#[invariant(deictic_ground.time.object_kind() == SemanticObjectKind::Referent)]
#[invariant(deictic_ground.place.object_kind() == SemanticObjectKind::Referent)]
#[invariant(asides.iter().all(|aside| matches!(aside.object_kind(), SemanticObjectKind::Utterance | SemanticObjectKind::DisplayedContent)))]
#[derive(Debug, Clone, PartialEq)]
pub struct UtteranceNode {
    pub force: UtteranceForce,
    pub speaker: SemanticObjectId,
    pub audience: SemanticObjectId,
    pub eventuality: SemanticObjectId,
    pub content: Option<SemanticObjectId>,
    pub deictic_ground: DeicticGround,
    pub asides: Vec<SemanticObjectId>,
    pub vocative_kind: Option<String>,
    pub common: SemanticObjectCommon,
}

#[invariant(items.iter().all(|item| sequence_item_kind_is_allowed(item.object_kind())))]
#[invariant(content.is_none_or(|content| matches!(content.object_kind(), SemanticObjectKind::Formula | SemanticObjectKind::Question)))]
#[invariant(connection_claims.iter().all(|claim| claim.object_kind() == SemanticObjectKind::Formula))]
#[invariant(elided_connection_operand.is_none() || content.is_some() || !connection_claims.is_empty() || nonlogical_connection.is_some())]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[invariant(force.is_none_or(|force| force == UtteranceForce::Subordinated))]
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceNode {
    pub force: Option<UtteranceForce>,
    pub items: Vec<SemanticObjectId>,
    pub content: Option<SemanticObjectId>,
    pub connection_claims: Vec<SemanticObjectId>,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub ordinal_labels: Vec<OrdinalLabel>,
    pub relation: SequenceRelation,
    pub nonlogical_connection: Option<NonlogicalConnection>,
    pub elided_connection_operand: Option<ElidedConnectionOperand>,
    pub common: SemanticObjectCommon,
}

#[invariant(class.is_none_or(|class| class.sort() == SemanticSort::Eventuality(*sort) || (class == EventualityClass::Event && *sort == EventualitySort::Experience)))]
#[invariant(denotation.category() != Some(ReferentCategory::Indexical) || indexical.is_some())]
#[invariant(denotation.category() == Some(ReferentCategory::Indexical) || indexical.is_none())]
#[invariant(tense_modal.is_none_or(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[invariant(time_path.iter().all(|step| step.anchor.object_id().is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind()))))]
#[invariant(space_path.iter().all(|step| step.anchor.object_id().is_none_or(|anchor| argument_object_kind_can_fill(anchor.object_kind()))))]
#[invariant(content.is_none_or(|content| matches!(content.object_kind(), SemanticObjectKind::Formula | SemanticObjectKind::Sequence)))]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
#[invariant(parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[invariant(embedded_questions.iter().all(|question| question.object_kind() == SemanticObjectKind::Question))]
#[invariant(experiencer.is_none_or(|experiencer| experiencer.object_kind() == SemanticObjectKind::Referent))]
#[invariant(scale.is_none_or(|scale| scale.object_kind() == SemanticObjectKind::Referent))]
#[invariant(target.is_none_or(referent_target_kind_is_allowed))]
#[derive(Debug, Clone, PartialEq)]
pub struct EventualityNode {
    pub denotation: EventualityDenotation,
    pub sort: EventualitySort,
    pub class: Option<EventualityClass>,
    pub indexical: Option<IndexicalKind>,
    pub descriptor: Option<Descriptor>,
    pub composition: Option<Composition>,
    pub relative_clauses: Vec<RelativeClause>,
    pub assigned_names: Vec<AssignedName>,
    pub modal_arguments: Vec<ModalArgument>,
    pub actuality: Option<Actuality>,
    pub tense_modal: Option<SemanticObjectId>,
    pub time: Option<AnchorRelation>,
    pub time_path: Vec<TemporalPathStep>,
    pub time_interval: Option<TimeInterval>,
    pub time_span: Option<TimeSpan>,
    pub aspect: Option<Aspect>,
    pub aspects: Vec<Aspect>,
    pub recurrence: Vec<Recurrence>,
    pub interval_modifiers: Vec<IntervalModifier>,
    pub space: Option<AnchorRelation>,
    pub space_path: Vec<TemporalPathStep>,
    pub space_interval: Option<SpaceInterval>,
    pub spatial_aspect: Option<Aspect>,
    pub spatial_aspects: Vec<Aspect>,
    pub spatial_recurrence: Vec<Recurrence>,
    pub spatial_interval_modifiers: Vec<IntervalModifier>,
    pub content: Option<SemanticObjectId>,
    pub body: Option<SemanticObjectId>,
    pub parameters: Vec<SemanticObjectId>,
    pub arity: Option<usize>,
    pub embedded_questions: Vec<SemanticObjectId>,
    pub abstraction_kind: Option<AbstractionKind>,
    pub experiencer: Option<SemanticObjectId>,
    pub scale: Option<SemanticObjectId>,
    pub target: Option<SemanticObjectId>,
    pub subscript: Option<Subscript>,
    pub common: SemanticObjectCommon,
}

#[invariant(!sort.is_subsort_of(SemanticSort::eventuality()))]
#[invariant(*sort != SemanticSort::Sign)]
#[invariant(category != &ReferentCategory::Indexical || indexical.is_some())]
#[invariant(category == &ReferentCategory::Indexical || indexical.is_none())]
#[invariant((category == &ReferentCategory::Constant) == scope_dependence.is_some())]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
#[invariant(parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[invariant(embedded_questions.iter().all(|question| question.object_kind() == SemanticObjectKind::Question))]
#[invariant(arity.is_none_or(|arity| arity == parameters.len()))]
#[invariant(experiencer.is_none_or(|experiencer| experiencer.object_kind() == SemanticObjectKind::Referent))]
#[invariant(scale.is_none_or(|scale| scale.object_kind() == SemanticObjectKind::Referent))]
#[invariant(target.is_none_or(referent_target_kind_is_allowed))]
#[derive(Debug, Clone, PartialEq)]
pub struct ReferentNode {
    pub category: ReferentCategory,
    pub scope_dependence: Option<ScopeDependence>,
    pub sort: SemanticSort,
    pub indexical: Option<IndexicalKind>,
    pub descriptor: Option<Descriptor>,
    pub composition: Option<Composition>,
    pub relative_clauses: Vec<RelativeClause>,
    pub assigned_names: Vec<AssignedName>,
    pub body: Option<SemanticObjectId>,
    pub parameters: Vec<SemanticObjectId>,
    pub arity: Option<usize>,
    pub embedded_questions: Vec<SemanticObjectId>,
    pub abstraction_kind: Option<AbstractionKind>,
    pub abstracted: Option<SemanticObjectId>,
    pub experiencer: Option<SemanticObjectId>,
    pub scale: Option<SemanticObjectId>,
    pub target: Option<SemanticObjectId>,
    pub subscript: Option<Subscript>,
    pub common: SemanticObjectCommon,
}

#[invariant(parameter_role_matches_sort(Some(*sort), Some(*role)))]
#[invariant(!introduced_by.is_empty())]
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterNode {
    pub sort: SemanticSort,
    pub role: ParameterRole,
    pub introduced_by: String,
    pub subscript: Option<Subscript>,
    pub common: SemanticObjectCommon,
}

#[invariant(::Named { relation } => !relation.is_empty())]
#[invariant(::Parameter { parameter } => parameter.object_kind() == SemanticObjectKind::Parameter)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicationRelation {
    Named { relation: String },
    Parameter { parameter: SemanticObjectId },
}

#[invariant(eventuality.is_none_or(eventuality_is_referent))]
#[invariant(arguments.keys().all(|place| place.get() > 0))]
#[invariant(relation_metadata.is_none_or(|metadata| metadata.object_kind() == SemanticObjectKind::RelationMetadata))]
#[invariant(introduced_by.as_ref().is_none_or(|introduced_by| !introduced_by.is_empty()))]
#[derive(Debug, Clone, PartialEq)]
pub struct PredicationNode {
    pub relation: PredicationRelation,
    pub eventuality: Option<SemanticObjectId>,
    pub tanru_link: Option<TanruLink>,
    pub arguments: BTreeMap<PlaceIndex, ArgumentValue>,
    pub place_questions: Vec<PlaceQuestionBinding>,
    pub modal_arguments: Vec<ModalArgument>,
    pub reciprocity: Vec<ReciprocalExchange>,
    pub mode: PredicationMode,
    pub scalar_negation: Option<ScalarNegation>,
    pub relation_metadata: Option<SemanticObjectId>,
    pub introduced_by: Option<String>,
    pub common: SemanticObjectCommon,
}

#[invariant(predication.object_kind() == SemanticObjectKind::Predication)]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[derive(Debug, Clone, PartialEq)]
pub struct AtomFormulaNode {
    pub predication: SemanticObjectId,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub common: SemanticObjectCommon,
}

#[invariant(formula_connective_operator_is_allowed(*operator))]
#[invariant(!children.is_empty())]
#[invariant(children.iter().all(|child| child.object_kind() == SemanticObjectKind::Formula))]
#[invariant(eventuality.is_none_or(eventuality_is_referent))]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[invariant(connector.as_ref().is_none_or(|connector| connector.truth_table.is_none() || connector.parameter.is_none()))]
#[invariant((*operator == FormulaOperator::ConnectiveQuestion) == connector.as_ref().is_some_and(|connector| connector.truth_table.is_none() && connector.parameter.is_some()))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectiveFormulaNode {
    pub operator: FormulaOperator,
    pub children: Vec<SemanticObjectId>,
    pub connector: Option<Connector>,
    pub eventuality: Option<SemanticObjectId>,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub common: SemanticObjectCommon,
}

#[invariant(quantifier_formula_operator_is_allowed(*operator))]
#[invariant(quantifier_variable_kind_is_allowed(variable.object_kind()))]
#[invariant(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source.variable.object_kind() == SemanticObjectKind::Referent))]
#[invariant(selection_source.as_ref().is_none_or(|source| source_variable.is_none_or(|variable| variable == source.variable)))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == SemanticObjectKind::Quantity))]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[derive(Debug, Clone, PartialEq)]
pub struct QuantifiedFormulaNode {
    pub operator: FormulaOperator,
    pub variable: SemanticObjectId,
    pub source_variable: Option<SemanticObjectId>,
    pub selection_source: Option<SelectionSource>,
    pub restriction: Option<SemanticObjectId>,
    pub body: SemanticObjectId,
    pub quantity: Option<SemanticObjectId>,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub common: SemanticObjectCommon,
}

#[invariant(!bindings.is_empty())]
#[invariant(bindings.iter().all(quantifier_binding_matches_role))]
#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[derive(Debug, Clone, PartialEq)]
pub struct QuantifierBundleFormulaNode {
    pub bindings: Vec<QuantifierBinding>,
    pub body: SemanticObjectId,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub common: SemanticObjectCommon,
}

#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(!streams.is_empty())]
#[invariant(streams.iter().all(|stream| !stream.items.is_empty()))]
#[invariant(generated_eventuality_bindings_are_sorted(bound_eventualities))]
#[derive(Debug, Clone, PartialEq)]
pub struct RespectivelyDistributionFormulaNode {
    pub body: SemanticObjectId,
    pub streams: Vec<RespectivelyStream>,
    pub distinct_partition: Option<bool>,
    pub bound_eventualities: Vec<GeneratedEventualityId>,
    pub common: SemanticObjectCommon,
}

#[invariant(::Atom(node) => node.predication.object_kind() == SemanticObjectKind::Predication)]
#[invariant(::Connective(node) => formula_connective_operator_is_allowed(node.operator) && !node.children.is_empty())]
#[invariant(::Quantified(node) => quantifier_formula_operator_is_allowed(node.operator) && node.body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(::QuantifierBundle(node) => !node.bindings.is_empty() && node.body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(::RespectivelyDistribution(node) => !node.streams.is_empty() && node.body.object_kind() == SemanticObjectKind::Formula)]
#[derive(Debug, Clone, PartialEq)]
pub enum FormulaNode {
    Atom(AtomFormulaNode),
    Connective(ConnectiveFormulaNode),
    Quantified(QuantifiedFormulaNode),
    QuantifierBundle(QuantifierBundleFormulaNode),
    RespectivelyDistribution(RespectivelyDistributionFormulaNode),
}

#[invariant(predication.is_none_or(|predication| predication.object_kind() == SemanticObjectKind::Predication))]
#[invariant(children.iter().all(|child| child.object_kind() == SemanticObjectKind::Formula))]
#[invariant(restriction.is_none_or(|restriction| restriction.object_kind() == SemanticObjectKind::Formula))]
#[invariant(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaTraversal {
    pub predication: Option<SemanticObjectId>,
    pub children: Vec<SemanticObjectId>,
    pub restriction: Option<SemanticObjectId>,
    pub body: Option<SemanticObjectId>,
}

#[invariant(*category != ReferentCategory::Indexical, "sign referents have no indexical role field")]
#[invariant((*category == ReferentCategory::Constant) == scope_dependence.is_some())]
#[invariant(sign_kind.as_ref().is_some_and(|kind| *kind == SignKind::Quotation) == quotation.is_some())]
#[invariant(text.as_ref().is_none_or(|text| !text.is_empty()))]
#[invariant(relative_clauses.iter().all(|clause| clause.body.object_kind() == SemanticObjectKind::Formula))]
#[invariant(target.is_none_or(referent_target_kind_is_allowed))]
#[derive(Debug, Clone, PartialEq)]
pub struct SignNode {
    pub category: ReferentCategory,
    pub scope_dependence: Option<ScopeDependence>,
    pub sign_kind: Option<SignKind>,
    pub text: Option<String>,
    pub letterals: Vec<LetteralUnit>,
    pub quotation: Option<Quotation>,
    pub denotes: Option<SemanticObjectId>,
    pub descriptor: Option<Descriptor>,
    pub relative_clauses: Vec<RelativeClause>,
    pub target: Option<SemanticObjectId>,
    pub subscript: Option<Subscript>,
    pub common: SemanticObjectCommon,
}

#[invariant(!relation.is_empty())]
#[invariant(experiencer.object_kind() == SemanticObjectKind::Referent)]
#[invariant(displayed_content_target_kind_is_allowed(target.object_kind()))]
#[invariant(anchor.object_kind() == SemanticObjectKind::Utterance)]
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayedContentNode {
    pub family: DisplayedContentFamily,
    pub relation: String,
    pub intensity: Option<String>,
    pub polarity: DisplayedContentPolarity,
    pub phase: Option<String>,
    pub modifiers: Vec<DisplayedContentModifier>,
    pub assertion_effect: DisplayedContentAssertionEffect,
    pub experiencer: SemanticObjectId,
    pub target: SemanticObjectId,
    pub target_focus: Option<DisplayedContentTargetFocus>,
    pub anchor: SemanticObjectId,
    pub common: SemanticObjectCommon,
}

#[invariant(::Literal { denotes, .. } => denotes.is_none_or(|denotes| argument_object_kind_can_fill(denotes.object_kind())))]
#[invariant(::Operator { operator, operands, operator_denotes, endpoint_inclusion } => !operands.is_empty() && operands.iter().all(|operand| operand.object_kind() == SemanticObjectKind::MathExpression) && operator_denotes.is_none_or(|denotes| argument_object_kind_can_fill(denotes.object_kind())) && endpoint_inclusion.is_none_or(|_| operator.is_interval()))]
#[invariant(::QuestionedOperator { operator_parameter, operands } => operator_parameter.object_kind() == SemanticObjectKind::Parameter && !operands.is_empty() && operands.iter().all(|operand| operand.object_kind() == SemanticObjectKind::MathExpression))]
#[derive(Debug, Clone, PartialEq)]
pub enum MathExpressionNodeKind {
    Literal {
        literal: MathLiteral,
        denotes: Option<SemanticObjectId>,
    },
    Operator {
        operator: MathOperator,
        operands: Vec<SemanticObjectId>,
        operator_denotes: Option<SemanticObjectId>,
        endpoint_inclusion: Option<IntervalEndpointInclusion>,
    },
    QuestionedOperator {
        operator_parameter: SemanticObjectId,
        operands: Vec<SemanticObjectId>,
    },
}

#[invariant(subscript.as_ref().is_none_or(|subscript| subscript.value.object_kind() == SemanticObjectKind::MathExpression))]
#[derive(Debug, Clone, PartialEq)]
pub struct MathExpressionNode {
    pub kind: MathExpressionNodeKind,
    pub scalar_negation: Option<ScalarNegation>,
    pub subscript: Option<Subscript>,
    pub common: SemanticObjectCommon,
}

#[invariant(value.math_expression.is_none_or(|expression| expression.object_kind() == SemanticObjectKind::MathExpression))]
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityNode {
    pub form: QuantityForm,
    pub value: QuantityValue,
    pub scale: QuantityScale,
    pub comparison_set: Option<SemanticObjectId>,
    pub common: SemanticObjectCommon,
}

#[invariant(!relation.is_empty())]
#[invariant(source_words.iter().all(|word| !word.is_empty()))]
#[invariant(expansion.as_ref().is_none_or(|expansion| expansion.kind != "lujvo" || (!source_words.is_empty() && !expansion.source_words.is_empty() && place_structure.is_empty())), "lujvo metadata contains only a complete mechanical decomposition, never place claims")]
#[derive(Debug, Clone, PartialEq)]
pub struct RelationMetadataNode {
    pub relation: String,
    pub source_words: Vec<String>,
    pub place_structure: Vec<PlaceDescription>,
    pub expansion: Option<RelationExpansion>,
    pub common: SemanticObjectCommon,
}

#[invariant(body.object_kind() == SemanticObjectKind::Formula)]
#[invariant(asker.object_kind() == SemanticObjectKind::Referent)]
#[invariant(respondent.object_kind() == SemanticObjectKind::Referent)]
#[invariant(question_node_shape_is_valid(*kind, *mode, *domain, slots, *focus, *presupposed_answer))]
#[invariant(focus.is_none_or(question_focus_kind_is_allowed))]
#[invariant(presupposed_answer.is_none_or(question_focus_kind_is_allowed))]
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionNode {
    pub kind: QuestionKind,
    pub mode: QuestionMode,
    pub asker: SemanticObjectId,
    pub respondent: SemanticObjectId,
    pub domain: SemanticSort,
    pub body: SemanticObjectId,
    pub slots: Vec<QuestionSlot>,
    pub focus: Option<SemanticObjectId>,
    pub presupposed_answer: Option<SemanticObjectId>,
    pub common: SemanticObjectCommon,
}

#[requires(true)]
#[ensures(true)]
fn question_node_shape_is_valid(
    kind: QuestionKind,
    mode: QuestionMode,
    domain: SemanticSort,
    slots: &[QuestionSlot],
    focus: Option<SemanticObjectId>,
    presupposed_answer: Option<SemanticObjectId>,
) -> bool {
    if !question_kind_domain_are_coherent(kind, domain) {
        return false;
    }
    if kind == QuestionKind::Multiple {
        let Some(first) = slots.first().and_then(QuestionSlot::kind_and_domain) else {
            return false;
        };
        return slots.len() >= 2
            && slots.iter().all(|slot| slot.kind_and_domain().is_some())
            && slots
                .iter()
                .filter_map(QuestionSlot::kind_and_domain)
                .any(|slot| slot != first);
    }
    if kind == QuestionKind::Truth {
        return slots.is_empty();
    }
    if mode == QuestionMode::Indirect && slots.is_empty() {
        return focus.is_some() && presupposed_answer.is_some();
    }
    !slots.is_empty()
        && slots
            .iter()
            .all(|slot| slot.kind_and_domain().is_none() && slot.parameter().is_some())
}

#[invariant(::Utterance(node) => eventuality_is_referent(node.eventuality))]
#[invariant(::Sequence(node) => node.items.iter().all(|item| sequence_item_kind_is_allowed(item.object_kind())))]
#[invariant(::Eventuality(node) => SemanticSort::Eventuality(node.sort).is_subsort_of(SemanticSort::eventuality()))]
#[invariant(::Referent(node) => !node.sort.is_subsort_of(SemanticSort::eventuality()) && node.sort != SemanticSort::Sign)]
#[invariant(::Parameter(node) => parameter_role_matches_sort(Some(node.sort), Some(node.role)))]
#[invariant(::Predication(node) => node.arguments.keys().all(|place| place.get() > 0))]
#[invariant(::Formula(node) => formula_node_has_valid_shape(node))]
#[invariant(::Sign(node) => node.sign_kind.as_ref().is_some_and(|kind| *kind == SignKind::Quotation) == node.quotation.is_some())]
#[invariant(::DisplayedContent(node) => !node.relation.is_empty())]
#[invariant(::MathExpression(node) => node.subscript.as_ref().is_none_or(|subscript| subscript.value.object_kind() == SemanticObjectKind::MathExpression))]
#[invariant(::Quantity(node) => node.value.math_expression.is_none_or(|expression| expression.object_kind() == SemanticObjectKind::MathExpression))]
#[invariant(::RelationMetadata(node) => !node.relation.is_empty())]
#[invariant(::Question(node) => node.body.object_kind() == SemanticObjectKind::Formula)]
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticObject {
    Utterance(UtteranceNode),
    Sequence(SequenceNode),
    Eventuality(EventualityNode),
    Referent(ReferentNode),
    Parameter(ParameterNode),
    Predication(PredicationNode),
    Formula(FormulaNode),
    Sign(SignNode),
    DisplayedContent(DisplayedContentNode),
    MathExpression(MathExpressionNode),
    Quantity(QuantityNode),
    RelationMetadata(RelationMetadataNode),
    Question(QuestionNode),
}

#[requires(true)]
#[ensures(ret == matches!(id.object_kind(), SemanticObjectKind::Referent) && id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
fn eventuality_is_referent(id: SemanticObjectId) -> bool {
    id.object_kind() == SemanticObjectKind::Referent
        && id
            .referent_sort()
            .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
}

#[requires(true)]
#[ensures(ret == bindings.windows(2).all(|pair| pair[0] < pair[1]))]
fn generated_eventuality_bindings_are_sorted(bindings: &[GeneratedEventualityId]) -> bool {
    bindings.windows(2).all(|pair| pair[0] < pair[1])
}

#[requires(true)]
#[ensures(ret == matches!(id.object_kind(), SemanticObjectKind::Parameter | SemanticObjectKind::Referent))]
fn question_focus_kind_is_allowed(id: SemanticObjectId) -> bool {
    matches!(
        id.object_kind(),
        SemanticObjectKind::Parameter | SemanticObjectKind::Referent
    )
}

#[requires(true)]
#[ensures(ret == !matches!(operator, FormulaOperator::Atom | FormulaOperator::Exists | FormulaOperator::Forall | FormulaOperator::None | FormulaOperator::Cardinality | FormulaOperator::PluralExists | FormulaOperator::PluralForall | FormulaOperator::QuantifierBundle | FormulaOperator::RespectivelyDistribution))]
fn formula_connective_operator_is_allowed(operator: FormulaOperator) -> bool {
    !matches!(
        operator,
        FormulaOperator::Atom
            | FormulaOperator::Exists
            | FormulaOperator::Forall
            | FormulaOperator::None
            | FormulaOperator::Cardinality
            | FormulaOperator::PluralExists
            | FormulaOperator::PluralForall
            | FormulaOperator::QuantifierBundle
            | FormulaOperator::RespectivelyDistribution
    )
}

#[requires(true)]
#[ensures(true)]
fn formula_node_has_valid_shape(node: &FormulaNode) -> bool {
    match node.as_data() {
        data!(FormulaNode::Atom(node)) => {
            node.predication.object_kind() == SemanticObjectKind::Predication
        }
        data!(FormulaNode::Connective(node)) => {
            formula_connective_operator_is_allowed(node.operator) && !node.children.is_empty()
        }
        data!(FormulaNode::Quantified(node)) => {
            quantifier_formula_operator_is_allowed(node.operator)
                && node.body.object_kind() == SemanticObjectKind::Formula
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            !node.bindings.is_empty() && node.body.object_kind() == SemanticObjectKind::Formula
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            !node.streams.is_empty() && node.body.object_kind() == SemanticObjectKind::Formula
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

impl SemanticSort {
    #[requires(true)]
    #[ensures(ret.is_some() == self.is_subsort_of(SemanticSort::eventuality()))]
    fn eventuality_sort(self) -> Option<EventualitySort> {
        match self {
            Self::Eventuality(sort) => Some(sort),
            _ => None,
        }
    }
}

macro_rules! update_node_common {
    ($variant:ident, $node_type:ident, $node:expr, $diagnostic:expr) => {{
        let data = $node.into_data();
        let common = common_with_diagnostic(data.common, $diagnostic);
        new!(SemanticObject::$variant($node_type::from_data(data!(
            $node_type {
                common: common,
                ..data
            }
        ))))
    }};
}

macro_rules! replace_node_diagnostics {
    ($variant:ident, $node_type:ident, $node:expr, $diagnostics:expr) => {{
        let data = $node.into_data();
        let common = common_with_diagnostics(data.common, $diagnostics);
        new!(SemanticObject::$variant($node_type::from_data(data!(
            $node_type {
                common: common,
                ..data
            }
        ))))
    }};
}

macro_rules! replace_node_source {
    ($variant:ident, $node_type:ident, $node:expr, $source:expr) => {{
        let data = $node.into_data();
        let common = common_with_source(data.common, $source);
        new!(SemanticObject::$variant($node_type::from_data(data!(
            $node_type {
                common: common,
                ..data
            }
        ))))
    }};
}

macro_rules! define_variant_access {
    ($as_name:ident, $update_name:ident, $variant:ident, $node_type:ident) => {
        #[requires(true)]
        #[ensures(true)]
        pub fn $as_name(&self) -> Option<&$node_type> {
            match self.as_data() {
                data!(SemanticObject::$variant(node)) => Some(node),
                _ => None,
            }
        }

        #[requires(self.$as_name().is_some())]
        #[ensures(self.$as_name().is_some())]
        pub fn $update_name(&mut self, update: impl FnOnce($node_type) -> $node_type) {
            let owned = self.take_for_update();
            *self = match owned.into_data() {
                data!(SemanticObject::$variant(node)) => {
                    new!(SemanticObject::$variant(update(node)))
                }
                data => SemanticObject::from_data(data),
            };
        }
    };
}

#[requires(!diagnostic.message.is_empty())]
#[ensures(ret.diagnostics.len() == old(common.diagnostics.len()) + 1)]
fn common_with_diagnostic(
    common: SemanticObjectCommon,
    diagnostic: SemanticDiagnostic,
) -> SemanticObjectCommon {
    let data = common.into_data();
    let mut diagnostics = data.diagnostics;
    diagnostics.push(diagnostic);
    SemanticObjectCommon::from_data(data!(SemanticObjectCommon {
        diagnostics: diagnostics,
        ..data
    }))
}

#[requires(diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()))]
#[ensures(ret.diagnostics.len() == old(diagnostics.len()))]
fn common_with_diagnostics(
    common: SemanticObjectCommon,
    diagnostics: Vec<SemanticDiagnostic>,
) -> SemanticObjectCommon {
    let data = common.into_data();
    SemanticObjectCommon::from_data(data!(SemanticObjectCommon {
        diagnostics: diagnostics,
        ..data
    }))
}

#[requires(true)]
#[ensures(true)]
fn common_with_source(
    common: SemanticObjectCommon,
    source: Option<SemanticSource>,
) -> SemanticObjectCommon {
    common.with_data(data! { source: source })
}

#[requires(!diagnostic.message.is_empty())]
#[ensures(true)]
fn formula_with_diagnostic(node: FormulaNode, diagnostic: SemanticDiagnostic) -> FormulaNode {
    match node.into_data() {
        data!(FormulaNode::Atom(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostic(data.common, diagnostic);
            new!(FormulaNode::Atom(AtomFormulaNode::from_data(data!(
                AtomFormulaNode {
                    common: common,
                    ..data
                }
            ))))
        }
        data!(FormulaNode::Connective(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostic(data.common, diagnostic);
            new!(FormulaNode::Connective(ConnectiveFormulaNode::from_data(
                data!(ConnectiveFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::Quantified(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostic(data.common, diagnostic);
            new!(FormulaNode::Quantified(QuantifiedFormulaNode::from_data(
                data!(QuantifiedFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostic(data.common, diagnostic);
            new!(FormulaNode::QuantifierBundle(
                QuantifierBundleFormulaNode::from_data(data!(QuantifierBundleFormulaNode {
                    common: common,
                    ..data
                }))
            ))
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostic(data.common, diagnostic);
            new!(FormulaNode::RespectivelyDistribution(
                RespectivelyDistributionFormulaNode::from_data(data!(
                    RespectivelyDistributionFormulaNode {
                        common: common,
                        ..data
                    }
                ))
            ))
        }
    }
}

#[requires(diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()))]
#[ensures(true)]
fn formula_with_diagnostics(
    node: FormulaNode,
    diagnostics: Vec<SemanticDiagnostic>,
) -> FormulaNode {
    match node.into_data() {
        data!(FormulaNode::Atom(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostics(data.common, diagnostics);
            new!(FormulaNode::Atom(AtomFormulaNode::from_data(data!(
                AtomFormulaNode {
                    common: common,
                    ..data
                }
            ))))
        }
        data!(FormulaNode::Connective(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostics(data.common, diagnostics);
            new!(FormulaNode::Connective(ConnectiveFormulaNode::from_data(
                data!(ConnectiveFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::Quantified(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostics(data.common, diagnostics);
            new!(FormulaNode::Quantified(QuantifiedFormulaNode::from_data(
                data!(QuantifiedFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostics(data.common, diagnostics);
            new!(FormulaNode::QuantifierBundle(
                QuantifierBundleFormulaNode::from_data(data!(QuantifierBundleFormulaNode {
                    common: common,
                    ..data
                }))
            ))
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let data = node.into_data();
            let common = common_with_diagnostics(data.common, diagnostics);
            new!(FormulaNode::RespectivelyDistribution(
                RespectivelyDistributionFormulaNode::from_data(data!(
                    RespectivelyDistributionFormulaNode {
                        common: common,
                        ..data
                    }
                ))
            ))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn formula_with_source(node: FormulaNode, source: Option<SemanticSource>) -> FormulaNode {
    match node.into_data() {
        data!(FormulaNode::Atom(node)) => {
            let data = node.into_data();
            let common = common_with_source(data.common, source);
            new!(FormulaNode::Atom(AtomFormulaNode::from_data(data!(
                AtomFormulaNode {
                    common: common,
                    ..data
                }
            ))))
        }
        data!(FormulaNode::Connective(node)) => {
            let data = node.into_data();
            let common = common_with_source(data.common, source);
            new!(FormulaNode::Connective(ConnectiveFormulaNode::from_data(
                data!(ConnectiveFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::Quantified(node)) => {
            let data = node.into_data();
            let common = common_with_source(data.common, source);
            new!(FormulaNode::Quantified(QuantifiedFormulaNode::from_data(
                data!(QuantifiedFormulaNode {
                    common: common,
                    ..data
                })
            )))
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            let data = node.into_data();
            let common = common_with_source(data.common, source);
            new!(FormulaNode::QuantifierBundle(
                QuantifierBundleFormulaNode::from_data(data!(QuantifierBundleFormulaNode {
                    common: common,
                    ..data
                }))
            ))
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let data = node.into_data();
            let common = common_with_source(data.common, source);
            new!(FormulaNode::RespectivelyDistribution(
                RespectivelyDistributionFormulaNode::from_data(data!(
                    RespectivelyDistributionFormulaNode {
                        common: common,
                        ..data
                    }
                ))
            ))
        }
    }
}

// Constructors and ownership-preserving update/access helpers are kept on the
// enum so callers never handle unchecked node data.
impl SemanticObject {
    define_variant_access!(as_utterance, update_utterance, Utterance, UtteranceNode);
    define_variant_access!(as_sequence, update_sequence, Sequence, SequenceNode);
    define_variant_access!(
        as_eventuality,
        update_eventuality,
        Eventuality,
        EventualityNode
    );
    define_variant_access!(as_referent, update_referent, Referent, ReferentNode);
    define_variant_access!(as_parameter, update_parameter, Parameter, ParameterNode);
    define_variant_access!(
        as_predication,
        update_predication,
        Predication,
        PredicationNode
    );
    define_variant_access!(as_formula, update_formula, Formula, FormulaNode);
    define_variant_access!(as_sign, update_sign, Sign, SignNode);
    define_variant_access!(
        as_displayed_content,
        update_displayed_content,
        DisplayedContent,
        DisplayedContentNode
    );
    define_variant_access!(
        as_math_expression,
        update_math_expression,
        MathExpression,
        MathExpressionNode
    );
    define_variant_access!(as_quantity, update_quantity, Quantity, QuantityNode);
    define_variant_access!(
        as_relation_metadata,
        update_relation_metadata,
        RelationMetadata,
        RelationMetadataNode
    );
    define_variant_access!(as_question, update_question, Question, QuestionNode);

    #[requires(eventuality_is_referent(eventuality))]
    #[requires(speaker.object_kind() == SemanticObjectKind::Referent)]
    #[requires(audience.object_kind() == SemanticObjectKind::Referent)]
    #[requires(eventuality_is_referent(now))]
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
        new!(SemanticObject::Utterance(new!(UtteranceNode {
            force,
            speaker,
            audience,
            eventuality,
            content,
            deictic_ground: DeicticGround {
                time: now,
                place: here
            },
            asides: Vec::new(),
            vocative_kind: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Sequence)]
    pub fn sequence(
        items: Vec<SemanticObjectId>,
        relation: SequenceRelation,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Sequence(new!(SequenceNode {
            force: None,
            items,
            content: None,
            connection_claims: Vec::new(),
            bound_eventualities: Vec::new(),
            ordinal_labels: Vec::new(),
            relation,
            nonlogical_connection: None,
            elided_connection_operand: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
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
        let sequence = match Self::sequence(items, relation, source, diagnostics).into_data() {
            data!(SemanticObject::Sequence(sequence)) => sequence,
            _ => unreachable!("sequence constructor returns a sequence"),
        };
        new!(SemanticObject::Sequence(sequence.with_data(data! {
            nonlogical_connection: Some(nonlogical_connection),
        })))
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
        let sequence = match Self::sequence(items, relation, source, diagnostics).into_data() {
            data!(SemanticObject::Sequence(sequence)) => sequence,
            _ => unreachable!("sequence constructor returns a sequence"),
        };
        new!(SemanticObject::Sequence(sequence.with_data(data! {
            connection_claims: connection_claims,
        })))
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn generated_eventuality(
        class: EventualityClass,
        actuality: Option<Actuality>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::eventuality_with_details(
            EventualityDenotation::generated_bound(),
            class,
            None,
            None,
            None,
            actuality,
            source,
            Vec::new(),
        )
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(ret.referent_category() == Some(ReferentCategory::Constant))]
    pub fn referential_eventuality(
        class: EventualityClass,
        actuality: Option<Actuality>,
        source: Option<SemanticSource>,
    ) -> Self {
        Self::eventuality_with_details(
            EventualityDenotation::referential(ReferentCategory::Constant),
            class,
            None,
            None,
            None,
            actuality,
            source,
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    fn eventuality_with_details(
        denotation: EventualityDenotation,
        class: EventualityClass,
        indexical: Option<IndexicalKind>,
        descriptor: Option<Descriptor>,
        composition: Option<Composition>,
        actuality: Option<Actuality>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let sort = class
            .sort()
            .eventuality_sort()
            .expect("eventuality classes have eventuality sorts");
        new!(SemanticObject::Eventuality(new!(EventualityNode {
            denotation,
            sort,
            class: Some(class),
            indexical,
            descriptor,
            composition,
            relative_clauses: Vec::new(),
            assigned_names: Vec::new(),
            modal_arguments: Vec::new(),
            actuality,
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
            content: None,
            body: None,
            parameters: Vec::new(),
            arity: None,
            embedded_questions: Vec::new(),
            abstraction_kind: None,
            experiencer: None,
            scale: None,
            target: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
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
        let scope_dependence =
            (category == ReferentCategory::Constant).then(ScopeDependence::fixed);
        if let Some(eventuality_sort) = sort.eventuality_sort() {
            return new!(SemanticObject::Eventuality(new!(EventualityNode {
                denotation: EventualityDenotation::referential(category),
                sort: eventuality_sort,
                class: None,
                indexical,
                descriptor,
                composition,
                relative_clauses: Vec::new(),
                assigned_names: Vec::new(),
                modal_arguments: Vec::new(),
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
                content: None,
                body: None,
                parameters: Vec::new(),
                arity: None,
                embedded_questions: Vec::new(),
                abstraction_kind: None,
                experiencer: None,
                scale: None,
                target: None,
                subscript: None,
                common: SemanticObjectCommon::new(source, diagnostics),
            })));
        }
        if sort == SemanticSort::Sign {
            return new!(SemanticObject::Sign(new!(SignNode {
                category,
                scope_dependence,
                sign_kind: None,
                text: None,
                letterals: Vec::new(),
                quotation: None,
                denotes: None,
                descriptor,
                relative_clauses: Vec::new(),
                target: None,
                subscript: None,
                common: SemanticObjectCommon::new(source, diagnostics),
            })));
        }
        new!(SemanticObject::Referent(new!(ReferentNode {
            category,
            scope_dependence,
            sort,
            indexical,
            descriptor,
            composition,
            relative_clauses: Vec::new(),
            assigned_names: Vec::new(),
            body: None,
            parameters: Vec::new(),
            arity: None,
            embedded_questions: Vec::new(),
            abstraction_kind: None,
            abstracted: None,
            experiencer: None,
            scale: None,
            target: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Parameter)]
    pub fn parameter(
        sort: SemanticSort,
        role: ParameterRole,
        introduced_by: String,
        source: Option<SemanticSource>,
    ) -> Self {
        new!(SemanticObject::Parameter(new!(ParameterNode {
            sort,
            role,
            introduced_by,
            subscript: None,
            common: SemanticObjectCommon::new(source, Vec::new()),
        })))
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
        Self::predication_with_relation(
            new!(PredicationRelation::Named { relation }),
            eventuality,
            arguments,
            mode,
            source,
            diagnostics,
        )
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
        Self::predication_with_relation(
            new!(PredicationRelation::Parameter {
                parameter: relation_parameter
            }),
            eventuality,
            arguments,
            mode,
            source,
            diagnostics,
        )
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Predication)]
    fn predication_with_relation(
        relation: PredicationRelation,
        eventuality: Option<SemanticObjectId>,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        mode: PredicationMode,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Predication(new!(PredicationNode {
            relation,
            eventuality,
            tanru_link: None,
            arguments,
            place_questions: Vec::new(),
            modal_arguments: Vec::new(),
            reciprocity: Vec::new(),
            mode,
            scalar_negation: None,
            relation_metadata: None,
            introduced_by: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(!relation.is_empty())]
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
        let predication =
            match Self::predication(relation, eventuality, arguments, mode, source, diagnostics)
                .into_data()
            {
                data!(SemanticObject::Predication(predication)) => predication,
                _ => unreachable!("predication constructor returns a predication"),
            };
        new!(SemanticObject::Predication(predication.with_data(data! {
            tanru_link: Some(tanru_link),
        })))
    }

    #[requires(predication.object_kind() == SemanticObjectKind::Predication)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn atom_formula(
        predication: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Formula(new!(FormulaNode::Atom(new!(
            AtomFormulaNode {
                predication,
                bound_eventualities: Vec::new(),
                common: SemanticObjectCommon::new(source, diagnostics),
            }
        )))))
    }

    #[requires(formula_connective_operator_is_allowed(operator))]
    #[requires(!children.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn connective_formula(
        operator: FormulaOperator,
        children: Vec<SemanticObjectId>,
        connector: Option<Connector>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Formula(new!(FormulaNode::Connective(
            new!(ConnectiveFormulaNode {
                operator,
                children,
                connector,
                eventuality: None,
                bound_eventualities: Vec::new(),
                common: SemanticObjectCommon::new(source, diagnostics),
            })
        ))))
    }

    #[requires(quantifier_formula_operator_is_allowed(operator))]
    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(ret.formula_domain_import() == quantified_formula_domain_import(operator, restriction))]
    pub fn quantified_formula(
        operator: FormulaOperator,
        variable: SemanticObjectId,
        restriction: Option<SemanticObjectId>,
        body: SemanticObjectId,
        quantity: Option<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Formula(new!(FormulaNode::Quantified(
            new!(QuantifiedFormulaNode {
                operator,
                variable,
                source_variable: None,
                selection_source: None,
                restriction,
                body,
                quantity,
                bound_eventualities: Vec::new(),
                common: SemanticObjectCommon::new(source, diagnostics),
            })
        ))))
    }

    #[requires(!bindings.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn quantifier_bundle_formula(
        bindings: Vec<QuantifierBinding>,
        body: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Formula(new!(
            FormulaNode::QuantifierBundle(new!(QuantifierBundleFormulaNode {
                bindings,
                body,
                bound_eventualities: Vec::new(),
                common: SemanticObjectCommon::new(source, diagnostics),
            }))
        )))
    }

    #[requires(!streams.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Formula)]
    pub fn respectively_distribution_formula(
        body: SemanticObjectId,
        streams: Vec<RespectivelyStream>,
        distinct_partition: Option<bool>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Formula(new!(
            FormulaNode::RespectivelyDistribution(new!(RespectivelyDistributionFormulaNode {
                body,
                streams,
                distinct_partition,
                bound_eventualities: Vec::new(),
                common: SemanticObjectCommon::new(source, diagnostics),
            }))
        )))
    }

    #[requires(parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn abstraction(
        kind: AbstractionKind,
        body: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let sort = kind.output_sort();
        let mut object = Self::referent(
            ReferentCategory::Constant,
            sort,
            None,
            None,
            None,
            source,
            diagnostics,
        );
        object.set_abstraction_details(kind, body, parameters);
        object
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
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
        Self::question_with_focus(
            kind, mode, domain, body, slots, None, None, asker, respondent, source,
        )
    }

    #[requires(body.object_kind() == SemanticObjectKind::Formula)]
    #[requires(question_node_shape_is_valid(kind, mode, domain, &slots, focus, presupposed_answer))]
    #[requires(focus.is_none_or(question_focus_kind_is_allowed))]
    #[requires(presupposed_answer.is_none_or(question_focus_kind_is_allowed))]
    #[ensures(ret.object_kind() == SemanticObjectKind::Question)]
    #[allow(clippy::too_many_arguments)]
    pub fn question_with_focus(
        kind: QuestionKind,
        mode: QuestionMode,
        domain: SemanticSort,
        body: SemanticObjectId,
        slots: Vec<QuestionSlot>,
        focus: Option<SemanticObjectId>,
        presupposed_answer: Option<SemanticObjectId>,
        asker: SemanticObjectId,
        respondent: SemanticObjectId,
        source: Option<SemanticSource>,
    ) -> Self {
        new!(SemanticObject::Question(new!(QuestionNode {
            kind,
            mode,
            asker,
            respondent,
            domain,
            body,
            slots,
            focus,
            presupposed_answer,
            common: SemanticObjectCommon::new(source, Vec::new()),
        })))
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn sign(
        sign_kind: SignKind,
        quotation: Option<Quotation>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::Sign(new!(SignNode {
            category: ReferentCategory::Constant,
            scope_dependence: Some(ScopeDependence::fixed()),
            sign_kind: Some(sign_kind),
            text: None,
            letterals: Vec::new(),
            quotation,
            denotes: None,
            descriptor: None,
            relative_clauses: Vec::new(),
            target: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(sign_kind != SignKind::Quotation)]
    #[requires(!text.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::Referent)]
    pub fn text_sign(
        sign_kind: SignKind,
        text: String,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let sign = match Self::sign(sign_kind, None, source, diagnostics).into_data() {
            data!(SemanticObject::Sign(sign)) => sign,
            _ => unreachable!("sign constructor returns a sign"),
        };
        new!(SemanticObject::Sign(
            sign.with_data(data! { text: Some(text) })
        ))
    }

    #[requires(!relation.is_empty())]
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
        new!(SemanticObject::DisplayedContent(new!(
            DisplayedContentNode {
                family,
                relation,
                intensity: None,
                polarity,
                phase: None,
                modifiers: Vec::new(),
                assertion_effect,
                experiencer,
                target,
                target_focus: None,
                anchor,
                common: SemanticObjectCommon::new(source, diagnostics),
            }
        )))
    }

    #[requires(literal.is_some() || operator.is_some())]
    #[requires(literal.is_some() == operands.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression(
        operator: Option<MathOperator>,
        operands: Vec<SemanticObjectId>,
        literal: Option<MathLiteral>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        let kind = match (operator, literal) {
            (Some(operator), None) => new!(MathExpressionNodeKind::Operator {
                operator,
                operands,
                operator_denotes: None,
                endpoint_inclusion: None,
            }),
            (None, Some(literal)) => new!(MathExpressionNodeKind::Literal {
                literal,
                denotes: None,
            }),
            _ => unreachable!("math expression shape is checked by contract"),
        };
        new!(SemanticObject::MathExpression(new!(MathExpressionNode {
            kind,
            scalar_negation: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(operator.is_interval())]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_interval_expression(
        operator: MathOperator,
        operands: Vec<SemanticObjectId>,
        endpoint_inclusion: Option<IntervalEndpointInclusion>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::MathExpression(new!(MathExpressionNode {
            kind: new!(MathExpressionNodeKind::Operator {
                operator,
                operands,
                operator_denotes: None,
                endpoint_inclusion,
            }),
            scalar_negation: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(operator_parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_expression_with_operator_parameter(
        operator_parameter: SemanticObjectId,
        operands: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::MathExpression(new!(MathExpressionNode {
            kind: new!(MathExpressionNodeKind::QuestionedOperator {
                operator_parameter,
                operands,
            }),
            scalar_negation: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(argument_object_kind_can_fill(denotes.object_kind()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_sumti_operand(
        denotes: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        Self::math_operand(
            MathLiteralKind::SumtiOperand,
            "mo'e",
            denotes,
            source,
            diagnostics,
        )
    }

    #[requires(argument_object_kind_can_fill(denotes.object_kind()))]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    pub fn math_selbri_operand(
        denotes: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        Self::math_operand(
            MathLiteralKind::SelbriOperand,
            "ni'e",
            denotes,
            source,
            diagnostics,
        )
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::MathExpression)]
    fn math_operand(
        kind: MathLiteralKind,
        marker: &str,
        denotes: SemanticObjectId,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::MathExpression(new!(MathExpressionNode {
            kind: new!(MathExpressionNodeKind::Literal {
                literal: MathLiteral::text(kind, marker.to_owned()),
                denotes: Some(denotes),
            }),
            scalar_negation: None,
            subscript: None,
            common: SemanticObjectCommon::new(source, diagnostics),
        })))
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == SemanticObjectKind::Quantity)]
    pub fn quantity(
        form: QuantityForm,
        value: QuantityValue,
        scale: QuantityScale,
        source: Option<SemanticSource>,
    ) -> Self {
        new!(SemanticObject::Quantity(new!(QuantityNode {
            form,
            value,
            scale,
            comparison_set: None,
            common: SemanticObjectCommon::new(source, Vec::new()),
        })))
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.object_kind() == SemanticObjectKind::RelationMetadata)]
    pub fn relation_metadata(
        relation: String,
        source_words: Vec<String>,
        place_structure: Vec<PlaceDescription>,
        expansion: Option<RelationExpansion>,
        source: Option<SemanticSource>,
        diagnostics: Vec<SemanticDiagnostic>,
    ) -> Self {
        new!(SemanticObject::RelationMetadata(new!(
            RelationMetadataNode {
                relation,
                source_words,
                place_structure,
                expansion,
                common: SemanticObjectCommon::new(source, diagnostics),
            }
        )))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn object_kind(&self) -> SemanticObjectKind {
        match self.as_data() {
            data!(SemanticObject::Utterance(_)) => SemanticObjectKind::Utterance,
            data!(SemanticObject::Sequence(_)) => SemanticObjectKind::Sequence,
            data!(SemanticObject::Eventuality(_))
            | data!(SemanticObject::Referent(_))
            | data!(SemanticObject::Sign(_)) => SemanticObjectKind::Referent,
            data!(SemanticObject::Parameter(_)) => SemanticObjectKind::Parameter,
            data!(SemanticObject::Predication(_)) => SemanticObjectKind::Predication,
            data!(SemanticObject::Formula(_)) => SemanticObjectKind::Formula,
            data!(SemanticObject::DisplayedContent(_)) => SemanticObjectKind::DisplayedContent,
            data!(SemanticObject::MathExpression(_)) => SemanticObjectKind::MathExpression,
            data!(SemanticObject::Quantity(_)) => SemanticObjectKind::Quantity,
            data!(SemanticObject::RelationMetadata(_)) => SemanticObjectKind::RelationMetadata,
            data!(SemanticObject::Question(_)) => SemanticObjectKind::Question,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.object_kind(), SemanticObjectKind::Referent | SemanticObjectKind::Parameter))]
    pub fn sort(&self) -> Option<SemanticSort> {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => Some(SemanticSort::Eventuality(node.sort)),
            data!(SemanticObject::Referent(node)) => Some(node.sort),
            data!(SemanticObject::Sign(_)) => Some(SemanticSort::Sign),
            data!(SemanticObject::Parameter(node)) => Some(node.sort),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == self.as_formula().is_some())]
    pub fn formula_traversal(&self) -> Option<FormulaTraversal> {
        let formula = self.as_formula()?;
        let traversal = match formula.as_data() {
            data!(FormulaNode::Atom(node)) => new!(FormulaTraversal {
                predication: Some(node.predication),
                children: Vec::new(),
                restriction: None,
                body: None,
            }),
            data!(FormulaNode::Connective(node)) => new!(FormulaTraversal {
                predication: None,
                children: node.children.clone(),
                restriction: None,
                body: None,
            }),
            data!(FormulaNode::Quantified(node)) => new!(FormulaTraversal {
                predication: None,
                children: Vec::new(),
                restriction: node.restriction,
                body: Some(node.body),
            }),
            data!(FormulaNode::QuantifierBundle(node)) => new!(FormulaTraversal {
                predication: None,
                children: Vec::new(),
                restriction: None,
                body: Some(node.body),
            }),
            data!(FormulaNode::RespectivelyDistribution(node)) => new!(FormulaTraversal {
                predication: None,
                children: Vec::new(),
                restriction: None,
                body: Some(node.body),
            }),
        };
        Some(traversal)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn formula_operator(&self) -> Option<FormulaOperator> {
        match self.as_formula()?.as_data() {
            data!(FormulaNode::Atom(_)) => Some(FormulaOperator::Atom),
            data!(FormulaNode::Connective(node)) => Some(node.operator),
            data!(FormulaNode::Quantified(node)) => Some(node.operator),
            data!(FormulaNode::QuantifierBundle(_)) => Some(FormulaOperator::QuantifierBundle),
            data!(FormulaNode::RespectivelyDistribution(_)) => {
                Some(FormulaOperator::RespectivelyDistribution)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| id.object_kind() == SemanticObjectKind::Predication))]
    pub fn formula_predication(&self) -> Option<SemanticObjectId> {
        match self.as_formula()?.as_data() {
            data!(FormulaNode::Atom(node)) => Some(node.predication),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn formula_children(&self) -> &[SemanticObjectId] {
        match self.as_formula().map(FormulaNode::as_data) {
            Some(data!(FormulaNode::Connective(node))) => &node.children,
            _ => &[],
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn formula_restriction(&self) -> Option<SemanticObjectId> {
        match self.as_formula()?.as_data() {
            data!(FormulaNode::Quantified(node)) => node.restriction,
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (matches!(self.formula_operator(), Some(FormulaOperator::Forall | FormulaOperator::PluralForall)) && self.formula_restriction().is_some()))]
    pub fn formula_domain_import(&self) -> Option<DomainImport> {
        match self.as_formula()?.as_data() {
            data!(FormulaNode::Quantified(node)) => {
                quantified_formula_domain_import(node.operator, node.restriction)
            }
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn formula_body(&self) -> Option<SemanticObjectId> {
        match self.as_formula()?.as_data() {
            data!(FormulaNode::Quantified(node)) => Some(node.body),
            data!(FormulaNode::QuantifierBundle(node)) => Some(node.body),
            data!(FormulaNode::RespectivelyDistribution(node)) => Some(node.body),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|eventuality| eventuality.object_id().referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    pub fn bound_eventualities(&self) -> &[GeneratedEventualityId] {
        match self.as_data() {
            data!(SemanticObject::Sequence(node)) => &node.bound_eventualities,
            data!(SemanticObject::Formula(formula)) => match formula.as_data() {
                data!(FormulaNode::Atom(node)) => &node.bound_eventualities,
                data!(FormulaNode::Connective(node)) => &node.bound_eventualities,
                data!(FormulaNode::Quantified(node)) => &node.bound_eventualities,
                data!(FormulaNode::QuantifierBundle(node)) => &node.bound_eventualities,
                data!(FormulaNode::RespectivelyDistribution(node)) => &node.bound_eventualities,
            },
            _ => &[],
        }
    }

    #[requires(matches!(self.object_kind(), SemanticObjectKind::Formula | SemanticObjectKind::Sequence))]
    #[requires(generated_eventuality_bindings_are_sorted(&bound_eventualities))]
    #[ensures(self.bound_eventualities() == old(bound_eventualities.clone()).as_slice())]
    pub(crate) fn set_bound_eventualities(
        &mut self,
        bound_eventualities: Vec<GeneratedEventualityId>,
    ) {
        if self.as_sequence().is_some() {
            self.update_sequence(|node| {
                node.with_data(data! { bound_eventualities: bound_eventualities })
            });
            return;
        }
        self.update_formula(|formula| match formula.into_data() {
            data!(FormulaNode::Atom(node)) => new!(FormulaNode::Atom(
                node.with_data(data! { bound_eventualities: bound_eventualities })
            )),
            data!(FormulaNode::Connective(node)) => new!(FormulaNode::Connective(
                node.with_data(data! { bound_eventualities: bound_eventualities })
            )),
            data!(FormulaNode::Quantified(node)) => new!(FormulaNode::Quantified(
                node.with_data(data! { bound_eventualities: bound_eventualities })
            )),
            data!(FormulaNode::QuantifierBundle(node)) => new!(FormulaNode::QuantifierBundle(
                node.with_data(data! { bound_eventualities: bound_eventualities })
            )),
            data!(FormulaNode::RespectivelyDistribution(node)) => {
                new!(FormulaNode::RespectivelyDistribution(node.with_data(
                    data! { bound_eventualities: bound_eventualities }
                )))
            }
        });
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predication_arguments(&self) -> Option<&BTreeMap<PlaceIndex, ArgumentValue>> {
        Some(&self.as_predication()?.arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predication_modal_arguments(&self) -> Option<&[ModalArgument]> {
        Some(&self.as_predication()?.modal_arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predication_mode(&self) -> Option<PredicationMode> {
        Some(self.as_predication()?.mode)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predication_eventuality(&self) -> Option<SemanticObjectId> {
        self.as_predication()?.eventuality
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predication_tanru_link(&self) -> Option<&TanruLink> {
        self.as_predication()?.tanru_link.as_ref()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn quantity_value(&self) -> Option<&QuantityValue> {
        Some(&self.as_quantity()?.value)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn referent_category(&self) -> Option<ReferentCategory> {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.denotation.category(),
            data!(SemanticObject::Referent(node)) => Some(node.category),
            data!(SemanticObject::Sign(node)) => Some(node.category),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (self.referent_category() == Some(ReferentCategory::Constant)))]
    pub fn scope_dependence(&self) -> Option<&ScopeDependence> {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.denotation.scope_dependence(),
            data!(SemanticObject::Referent(node)) => node.scope_dependence.as_ref(),
            data!(SemanticObject::Sign(node)) => node.scope_dependence.as_ref(),
            _ => None,
        }
    }

    #[requires(self.referent_category() == Some(ReferentCategory::Constant))]
    #[ensures(self.scope_dependence().is_some_and(|derived| derived == &old(scope_dependence.clone())))]
    pub(crate) fn set_scope_dependence(&mut self, scope_dependence: ScopeDependence) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| {
                let denotation = node
                    .denotation
                    .clone()
                    .with_scope_dependence(scope_dependence);
                node.with_data(data! { denotation: denotation })
            });
        } else if self.as_referent().is_some() {
            self.update_referent(|node| {
                node.with_data(data! { scope_dependence: Some(scope_dependence) })
            });
        } else {
            self.update_sign(|node| {
                node.with_data(data! { scope_dependence: Some(scope_dependence) })
            });
        }
    }

    #[requires(true)]
    #[ensures(ret == self.as_eventuality().is_some_and(|node| node.denotation.is_generated_bound()))]
    pub fn is_generated_eventuality(&self) -> bool {
        self.as_eventuality()
            .is_some_and(|node| node.denotation.is_generated_bound())
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn referent_composition(&self) -> Option<&Composition> {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.composition.as_ref(),
            data!(SemanticObject::Referent(node)) => node.composition.as_ref(),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn descriptor(&self) -> Option<&Descriptor> {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => node.descriptor.as_ref(),
            data!(SemanticObject::Referent(node)) => node.descriptor.as_ref(),
            data!(SemanticObject::Sign(node)) => node.descriptor.as_ref(),
            _ => None,
        }
    }

    #[requires(self.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub fn set_descriptor(&mut self, descriptor: Descriptor) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| node.with_data(data! { descriptor: Some(descriptor) }));
        } else if self.as_sign().is_some() {
            self.update_sign(|node| node.with_data(data! { descriptor: Some(descriptor) }));
        } else {
            self.update_referent(|node| node.with_data(data! { descriptor: Some(descriptor) }));
        }
    }

    #[requires(self.object_kind() == SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub fn set_referent_target(&mut self, target: Option<SemanticObjectId>) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| node.with_data(data! { target: target }));
        } else if self.as_sign().is_some() {
            self.update_sign(|node| node.with_data(data! { target: target }));
        } else {
            self.update_referent(|node| node.with_data(data! { target: target }));
        }
    }

    #[requires(self.as_sign().is_some())]
    #[ensures(true)]
    pub fn set_sign_denotes(&mut self, denotes: Option<SemanticObjectId>) {
        self.update_sign(|node| node.with_data(data! { denotes: denotes }));
    }

    #[requires(self.as_eventuality().is_some() || self.as_referent().is_some())]
    #[ensures(true)]
    pub fn set_abstraction_embedded_questions(&mut self, questions: Vec<SemanticObjectId>) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| node.with_data(data! { embedded_questions: questions }));
        } else {
            self.update_referent(|node| node.with_data(data! { embedded_questions: questions }));
        }
    }

    #[requires(self.as_eventuality().is_some())]
    #[requires(sort.is_subsort_of(SemanticSort::eventuality()))]
    #[ensures(true)]
    pub fn configure_eventuality_abstraction(
        &mut self,
        class: EventualityClass,
        sort: SemanticSort,
        body: SemanticObjectId,
        kind: AbstractionKind,
        parameters: Vec<SemanticObjectId>,
        embedded_questions: Vec<SemanticObjectId>,
        source: Option<SemanticSource>,
    ) {
        let sort = sort
            .eventuality_sort()
            .expect("precondition requires an eventuality sort");
        self.update_eventuality(|node| {
            node.with_data(data! {
                denotation: EventualityDenotation::referential(ReferentCategory::Constant),
                class: Some(class),
                sort: sort,
                content: Some(body),
                abstraction_kind: Some(kind),
                parameters: parameters,
                embedded_questions: embedded_questions,
            })
        });
        self.replace_source(source);
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assigned_names(&self) -> &[AssignedName] {
        match self.as_data() {
            data!(SemanticObject::Eventuality(node)) => &node.assigned_names,
            data!(SemanticObject::Referent(node)) => &node.assigned_names,
            _ => &[],
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source(&self) -> Option<&SemanticSource> {
        self.common().source.as_ref()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn diagnostics(&self) -> &[SemanticDiagnostic] {
        &self.common().diagnostics
    }

    #[requires(true)]
    #[ensures(true)]
    fn common(&self) -> &SemanticObjectCommon {
        match self.as_data() {
            data!(SemanticObject::Utterance(node)) => &node.common,
            data!(SemanticObject::Sequence(node)) => &node.common,
            data!(SemanticObject::Eventuality(node)) => &node.common,
            data!(SemanticObject::Referent(node)) => &node.common,
            data!(SemanticObject::Parameter(node)) => &node.common,
            data!(SemanticObject::Predication(node)) => &node.common,
            data!(SemanticObject::Formula(node)) => match node.as_data() {
                data!(FormulaNode::Atom(node)) => &node.common,
                data!(FormulaNode::Connective(node)) => &node.common,
                data!(FormulaNode::Quantified(node)) => &node.common,
                data!(FormulaNode::QuantifierBundle(node)) => &node.common,
                data!(FormulaNode::RespectivelyDistribution(node)) => &node.common,
            },
            data!(SemanticObject::Sign(node)) => &node.common,
            data!(SemanticObject::DisplayedContent(node)) => &node.common,
            data!(SemanticObject::MathExpression(node)) => &node.common,
            data!(SemanticObject::Quantity(node)) => &node.common,
            data!(SemanticObject::RelationMetadata(node)) => &node.common,
            data!(SemanticObject::Question(node)) => &node.common,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn references_into(&self, out: &mut Vec<SemanticObjectId>) {
        references_into(self, out);
    }

    /// Collects semantic references while excluding the derived owner edge itself.
    ///
    /// Binding derivation and unrelated introduction-order traversals use this view so an
    /// already-derived edge cannot become evidence for its own scope or reorder referential
    /// introductions.
    #[requires(true)]
    #[ensures(out.len() >= old(out.len()))]
    pub(crate) fn references_without_event_bindings_into(&self, out: &mut Vec<SemanticObjectId>) {
        let start = out.len();
        references_into(self, out);
        let binding_count = self.bound_eventualities().len();
        if binding_count > 0 {
            out.truncate(out.len() - binding_count);
        }
        debug_assert!(out.len() >= start);
    }

    #[requires(true)]
    #[ensures(self.diagnostics().len() == old(self.diagnostics().len()) + 1)]
    pub fn push_diagnostic(&mut self, diagnostic: SemanticDiagnostic) {
        let owned = self.take_for_update();
        *self = match owned.into_data() {
            data!(SemanticObject::Utterance(node)) => {
                update_node_common!(Utterance, UtteranceNode, node, diagnostic)
            }
            data!(SemanticObject::Sequence(node)) => {
                update_node_common!(Sequence, SequenceNode, node, diagnostic)
            }
            data!(SemanticObject::Eventuality(node)) => {
                update_node_common!(Eventuality, EventualityNode, node, diagnostic)
            }
            data!(SemanticObject::Referent(node)) => {
                update_node_common!(Referent, ReferentNode, node, diagnostic)
            }
            data!(SemanticObject::Parameter(node)) => {
                update_node_common!(Parameter, ParameterNode, node, diagnostic)
            }
            data!(SemanticObject::Predication(node)) => {
                update_node_common!(Predication, PredicationNode, node, diagnostic)
            }
            data!(SemanticObject::Formula(node)) => new!(SemanticObject::Formula(
                formula_with_diagnostic(node, diagnostic)
            )),
            data!(SemanticObject::Sign(node)) => {
                update_node_common!(Sign, SignNode, node, diagnostic)
            }
            data!(SemanticObject::DisplayedContent(node)) => {
                update_node_common!(DisplayedContent, DisplayedContentNode, node, diagnostic)
            }
            data!(SemanticObject::MathExpression(node)) => {
                update_node_common!(MathExpression, MathExpressionNode, node, diagnostic)
            }
            data!(SemanticObject::Quantity(node)) => {
                update_node_common!(Quantity, QuantityNode, node, diagnostic)
            }
            data!(SemanticObject::RelationMetadata(node)) => {
                update_node_common!(RelationMetadata, RelationMetadataNode, node, diagnostic)
            }
            data!(SemanticObject::Question(node)) => {
                update_node_common!(Question, QuestionNode, node, diagnostic)
            }
        };
    }

    #[requires(diagnostics.iter().all(|diagnostic| !diagnostic.message.is_empty()))]
    #[ensures(self.diagnostics().len() == old(diagnostics.len()))]
    pub fn replace_diagnostics(&mut self, diagnostics: Vec<SemanticDiagnostic>) {
        let owned = self.take_for_update();
        *self = match owned.into_data() {
            data!(SemanticObject::Utterance(node)) => {
                replace_node_diagnostics!(Utterance, UtteranceNode, node, diagnostics)
            }
            data!(SemanticObject::Sequence(node)) => {
                replace_node_diagnostics!(Sequence, SequenceNode, node, diagnostics)
            }
            data!(SemanticObject::Eventuality(node)) => {
                replace_node_diagnostics!(Eventuality, EventualityNode, node, diagnostics)
            }
            data!(SemanticObject::Referent(node)) => {
                replace_node_diagnostics!(Referent, ReferentNode, node, diagnostics)
            }
            data!(SemanticObject::Parameter(node)) => {
                replace_node_diagnostics!(Parameter, ParameterNode, node, diagnostics)
            }
            data!(SemanticObject::Predication(node)) => {
                replace_node_diagnostics!(Predication, PredicationNode, node, diagnostics)
            }
            data!(SemanticObject::Formula(node)) => new!(SemanticObject::Formula(
                formula_with_diagnostics(node, diagnostics)
            )),
            data!(SemanticObject::Sign(node)) => {
                replace_node_diagnostics!(Sign, SignNode, node, diagnostics)
            }
            data!(SemanticObject::DisplayedContent(node)) => {
                replace_node_diagnostics!(DisplayedContent, DisplayedContentNode, node, diagnostics)
            }
            data!(SemanticObject::MathExpression(node)) => {
                replace_node_diagnostics!(MathExpression, MathExpressionNode, node, diagnostics)
            }
            data!(SemanticObject::Quantity(node)) => {
                replace_node_diagnostics!(Quantity, QuantityNode, node, diagnostics)
            }
            data!(SemanticObject::RelationMetadata(node)) => {
                replace_node_diagnostics!(RelationMetadata, RelationMetadataNode, node, diagnostics)
            }
            data!(SemanticObject::Question(node)) => {
                replace_node_diagnostics!(Question, QuestionNode, node, diagnostics)
            }
        };
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn replace_source(&mut self, source: Option<SemanticSource>) {
        let owned = self.take_for_update();
        *self = match owned.into_data() {
            data!(SemanticObject::Utterance(node)) => {
                replace_node_source!(Utterance, UtteranceNode, node, source)
            }
            data!(SemanticObject::Sequence(node)) => {
                replace_node_source!(Sequence, SequenceNode, node, source)
            }
            data!(SemanticObject::Eventuality(node)) => {
                replace_node_source!(Eventuality, EventualityNode, node, source)
            }
            data!(SemanticObject::Referent(node)) => {
                replace_node_source!(Referent, ReferentNode, node, source)
            }
            data!(SemanticObject::Parameter(node)) => {
                replace_node_source!(Parameter, ParameterNode, node, source)
            }
            data!(SemanticObject::Predication(node)) => {
                replace_node_source!(Predication, PredicationNode, node, source)
            }
            data!(SemanticObject::Formula(node)) => {
                new!(SemanticObject::Formula(formula_with_source(node, source)))
            }
            data!(SemanticObject::Sign(node)) => replace_node_source!(Sign, SignNode, node, source),
            data!(SemanticObject::DisplayedContent(node)) => {
                replace_node_source!(DisplayedContent, DisplayedContentNode, node, source)
            }
            data!(SemanticObject::MathExpression(node)) => {
                replace_node_source!(MathExpression, MathExpressionNode, node, source)
            }
            data!(SemanticObject::Quantity(node)) => {
                replace_node_source!(Quantity, QuantityNode, node, source)
            }
            data!(SemanticObject::RelationMetadata(node)) => {
                replace_node_source!(RelationMetadata, RelationMetadataNode, node, source)
            }
            data!(SemanticObject::Question(node)) => {
                replace_node_source!(Question, QuestionNode, node, source)
            }
        };
    }

    #[requires(self.as_eventuality().is_some() || self.as_referent().is_some() || self.as_sign().is_some())]
    #[ensures(true)]
    pub fn extend_relative_clauses(&mut self, clauses: Vec<RelativeClause>) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| {
                let mut data = node.into_data();
                data.relative_clauses.extend(clauses);
                EventualityNode::from_data(data)
            });
        } else if self.as_referent().is_some() {
            self.update_referent(|node| {
                let mut data = node.into_data();
                data.relative_clauses.extend(clauses);
                ReferentNode::from_data(data)
            });
        } else {
            self.update_sign(|node| {
                let mut data = node.into_data();
                data.relative_clauses.extend(clauses);
                SignNode::from_data(data)
            });
        }
    }

    #[requires(self.as_eventuality().is_some() || self.as_referent().is_some())]
    #[ensures(true)]
    pub fn push_assigned_name(&mut self, name: AssignedName) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| {
                let mut data = node.into_data();
                data.assigned_names.push(name);
                EventualityNode::from_data(data)
            });
        } else {
            self.update_referent(|node| {
                let mut data = node.into_data();
                data.assigned_names.push(name);
                ReferentNode::from_data(data)
            });
        }
    }

    #[requires(matches!(self.object_kind(), SemanticObjectKind::Referent | SemanticObjectKind::Parameter | SemanticObjectKind::MathExpression))]
    #[ensures(true)]
    pub fn set_subscript(&mut self, subscript: Subscript) {
        if self.as_eventuality().is_some() {
            self.update_eventuality(|node| node.with_data(data! { subscript: Some(subscript) }));
        } else if self.as_referent().is_some() {
            self.update_referent(|node| node.with_data(data! { subscript: Some(subscript) }));
        } else if self.as_sign().is_some() {
            self.update_sign(|node| node.with_data(data! { subscript: Some(subscript) }));
        } else if self.as_parameter().is_some() {
            self.update_parameter(|node| node.with_data(data! { subscript: Some(subscript) }));
        } else {
            self.update_math_expression(|node| {
                node.with_data(data! { subscript: Some(subscript) })
            });
        }
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.as_predication().is_some_and(|node| node.modal_arguments.len() == old(modal_arguments.len()))) ]
    pub fn set_predication_modal_arguments(&mut self, modal_arguments: Vec<ModalArgument>) {
        self.update_predication(|node| {
            node.with_data(data! {
                modal_arguments: modal_arguments,
            })
        });
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.as_predication().is_some_and(|node| node.place_questions.len() == old(place_questions.len()))) ]
    pub fn set_predication_place_questions(&mut self, place_questions: Vec<PlaceQuestionBinding>) {
        self.update_predication(|node| {
            node.with_data(data! {
                place_questions: place_questions,
            })
        });
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.as_predication().is_some())]
    pub fn set_predication_attachments(
        &mut self,
        modal_arguments: Vec<ModalArgument>,
        place_questions: Vec<PlaceQuestionBinding>,
    ) {
        self.update_predication(|node| {
            node.with_data(data! {
                modal_arguments: modal_arguments,
                place_questions: place_questions,
            })
        });
    }

    #[requires(self.as_formula().is_some())]
    #[requires(eventuality.is_none_or(eventuality_is_referent))]
    #[ensures(self.as_formula().is_some())]
    pub fn set_scoped_formula_eventuality(&mut self, eventuality: Option<SemanticObjectId>) {
        self.update_formula(|formula| match formula.into_data() {
            data!(FormulaNode::Connective(node)) => new!(FormulaNode::Connective(
                node.with_data(data! { eventuality: eventuality })
            )),
            data => FormulaNode::from_data(data),
        });
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.as_predication().is_some())]
    pub fn set_predication_relation_metadata(
        &mut self,
        relation_metadata: Option<SemanticObjectId>,
    ) {
        self.update_predication(|node| {
            node.with_data(data! {
                relation_metadata: relation_metadata,
            })
        });
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.predication_mode() == Some(mode))]
    pub fn set_predication_mode(&mut self, mode: PredicationMode) {
        self.update_predication(|node| node.with_data(data! { mode: mode }));
    }

    #[requires(self.as_predication().is_some())]
    #[ensures(self.as_predication().is_some())]
    pub fn set_predication_scalar_negation(&mut self, scalar_negation: ScalarNegation) {
        self.update_predication(|node| {
            node.with_data(data! {
                scalar_negation: Some(scalar_negation),
            })
        });
    }

    #[requires(self.as_formula().is_some())]
    #[requires(source_variable.is_none_or(|variable| variable.object_kind() == SemanticObjectKind::Referent))]
    #[ensures(ret.as_formula().is_some())]
    pub fn with_quantifier_selection(
        mut self,
        source_variable: Option<SemanticObjectId>,
        selection_source: Option<SelectionSource>,
    ) -> Self {
        self.update_formula(|formula| match formula.into_data() {
            data!(FormulaNode::Quantified(node)) => {
                new!(FormulaNode::Quantified(node.with_data(data! {
                    source_variable: source_variable,
                    selection_source: selection_source,
                })))
            }
            data => FormulaNode::from_data(data),
        });
        self
    }

    #[requires(self.as_eventuality().is_some())]
    #[ensures(self.as_eventuality().is_some())]
    pub fn apply_eventuality_aspects(&mut self, mut aspects: Vec<Aspect>, spatial: bool) {
        self.update_eventuality(|node| {
            let data = node.into_data();
            if aspects.len() == 1 {
                if spatial {
                    EventualityNode::from_data(data!(EventualityNode {
                        spatial_aspect: aspects.pop(),
                        ..data
                    }))
                } else {
                    EventualityNode::from_data(data!(EventualityNode {
                        aspect: aspects.pop(),
                        ..data
                    }))
                }
            } else if aspects.is_empty() {
                EventualityNode::from_data(data)
            } else if spatial {
                let mut spatial_aspects = data.spatial_aspects;
                spatial_aspects.extend(aspects);
                EventualityNode::from_data(data!(EventualityNode {
                    spatial_aspects: spatial_aspects,
                    ..data
                }))
            } else {
                let mut current_aspects = data.aspects;
                current_aspects.extend(aspects);
                EventualityNode::from_data(data!(EventualityNode {
                    aspects: current_aspects,
                    ..data
                }))
            }
        });
    }

    #[requires(self.as_eventuality().is_some())]
    #[ensures(self.as_eventuality().is_some())]
    pub fn attach_eventuality_magnitude(&mut self, magnitude: AnchorMagnitude, spatial: bool) {
        self.update_eventuality(|node| {
            let mut data = node.into_data();
            if spatial {
                if let Some(step) = data.space_path.pop() {
                    data.space_path
                        .push(step.with_data(data! { magnitude: Some(magnitude) }));
                    EventualityNode::from_data(data)
                } else if let Some(space) = data.space {
                    data.space = Some(space.with_data(data! { magnitude: Some(magnitude) }));
                    EventualityNode::from_data(data)
                } else {
                    EventualityNode::from_data(data)
                }
            } else {
                if let Some(step) = data.time_path.pop() {
                    data.time_path
                        .push(step.with_data(data! { magnitude: Some(magnitude) }));
                    EventualityNode::from_data(data)
                } else if let Some(time) = data.time {
                    data.time = Some(time.with_data(data! { magnitude: Some(magnitude) }));
                    EventualityNode::from_data(data)
                } else {
                    EventualityNode::from_data(data)
                }
            }
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn take_for_update(&mut self) -> Self {
        std::mem::replace(
            self,
            Self::sequence(
                Vec::new(),
                SequenceRelation::SameTopicContinuation,
                None,
                Vec::new(),
            ),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_abstraction_details(
        &mut self,
        kind: AbstractionKind,
        body: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
    ) {
        let owned = self.take_for_update();
        *self = match owned.into_data() {
            data!(SemanticObject::Referent(node)) => {
                let arity = (kind == AbstractionKind::Property).then_some(parameters.len());
                new!(SemanticObject::Referent(node.with_data(data! {
                    body: Some(body),
                    parameters: parameters,
                    arity: arity,
                    abstraction_kind: Some(kind),
                })))
            }
            data!(SemanticObject::Eventuality(node)) => {
                new!(SemanticObject::Eventuality(node.with_data(data! {
                    body: Some(body),
                    parameters: parameters,
                    abstraction_kind: Some(kind),
                })))
            }
            _ => unreachable!("abstraction outputs are referent variants"),
        };
    }
}

// The serializer deliberately mirrors the old flat field order. Keeping it in
// this module makes the unchecked flat shape a boundary-only concern.
impl Serialize for SemanticObject {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_flat(self, serializer)
    }
}

#[requires(true)]
#[ensures(true)]
fn serialize_flat<S>(object: &SemanticObject, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(None)?;
    map.serialize_entry("type", &object.object_kind())?;
    match object.as_data() {
        data!(SemanticObject::Utterance(node)) => serialize_utterance(&mut map, node)?,
        data!(SemanticObject::Sequence(node)) => serialize_sequence(&mut map, node)?,
        data!(SemanticObject::Eventuality(node)) => serialize_eventuality(&mut map, node)?,
        data!(SemanticObject::Referent(node)) => serialize_referent(&mut map, node)?,
        data!(SemanticObject::Parameter(node)) => serialize_parameter(&mut map, node)?,
        data!(SemanticObject::Predication(node)) => serialize_predication(&mut map, node)?,
        data!(SemanticObject::Formula(node)) => serialize_formula(&mut map, node)?,
        data!(SemanticObject::Sign(node)) => serialize_sign(&mut map, node)?,
        data!(SemanticObject::DisplayedContent(node)) => serialize_displayed(&mut map, node)?,
        data!(SemanticObject::MathExpression(node)) => serialize_math(&mut map, node)?,
        data!(SemanticObject::Quantity(node)) => serialize_quantity(&mut map, node)?,
        data!(SemanticObject::RelationMetadata(node)) => {
            serialize_relation_metadata(&mut map, node)?
        }
        data!(SemanticObject::Question(node)) => serialize_question(&mut map, node)?,
    }
    serialize_common(&mut map, object.common())?;
    map.end()
}

macro_rules! optional_entry {
    ($map:expr, $key:literal, $value:expr) => {
        if let Some(value) = $value {
            $map.serialize_entry($key, value)?;
        }
    };
}

macro_rules! nonempty_entry {
    ($map:expr, $key:literal, $value:expr) => {
        if !$value.is_empty() {
            $map.serialize_entry($key, $value)?;
        }
    };
}

#[requires(true)]
#[ensures(true)]
fn serialize_common<M: SerializeMap>(
    map: &mut M,
    common: &SemanticObjectCommon,
) -> Result<(), M::Error> {
    optional_entry!(map, "source", common.source.as_ref());
    nonempty_entry!(map, "diagnostics", &common.diagnostics);
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_utterance<M: SerializeMap>(map: &mut M, node: &UtteranceNode) -> Result<(), M::Error> {
    map.serialize_entry("force", &node.force)?;
    map.serialize_entry("speaker", &node.speaker)?;
    map.serialize_entry("audience", &node.audience)?;
    map.serialize_entry("eventuality", &node.eventuality)?;
    optional_entry!(map, "content", node.content.as_ref());
    map.serialize_entry("deicticGround", &node.deictic_ground)?;
    nonempty_entry!(map, "asides", &node.asides);
    optional_entry!(map, "vocativeKind", node.vocative_kind.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_sequence<M: SerializeMap>(map: &mut M, node: &SequenceNode) -> Result<(), M::Error> {
    optional_entry!(map, "force", node.force.as_ref());
    optional_entry!(map, "content", node.content.as_ref());
    nonempty_entry!(map, "items", &node.items);
    nonempty_entry!(map, "connectionClaims", &node.connection_claims);
    nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
    nonempty_entry!(map, "ordinalLabels", &node.ordinal_labels);
    map.serialize_entry("relation", &node.relation)?;
    optional_entry!(
        map,
        "nonlogicalConnection",
        node.nonlogical_connection.as_ref()
    );
    optional_entry!(
        map,
        "elidedConnectionOperand",
        node.elided_connection_operand.as_ref()
    );
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_eventuality<M: SerializeMap>(
    map: &mut M,
    node: &EventualityNode,
) -> Result<(), M::Error> {
    map.serialize_entry("denotation", &node.denotation)?;
    optional_entry!(map, "content", node.content.as_ref());
    optional_entry!(map, "actuality", node.actuality.as_ref());
    optional_entry!(map, "tenseModal", node.tense_modal.as_ref());
    optional_entry!(map, "time", node.time.as_ref());
    nonempty_entry!(map, "timePath", &node.time_path);
    optional_entry!(map, "timeInterval", node.time_interval.as_ref());
    optional_entry!(map, "timeSpan", node.time_span.as_ref());
    optional_entry!(map, "aspect", node.aspect.as_ref());
    nonempty_entry!(map, "aspects", &node.aspects);
    nonempty_entry!(map, "recurrence", &node.recurrence);
    nonempty_entry!(map, "intervalModifiers", &node.interval_modifiers);
    optional_entry!(map, "space", node.space.as_ref());
    nonempty_entry!(map, "spacePath", &node.space_path);
    optional_entry!(map, "spaceInterval", node.space_interval.as_ref());
    optional_entry!(map, "spatialAspect", node.spatial_aspect.as_ref());
    nonempty_entry!(map, "spatialAspects", &node.spatial_aspects);
    nonempty_entry!(map, "spatialRecurrence", &node.spatial_recurrence);
    nonempty_entry!(
        map,
        "spatialIntervalModifiers",
        &node.spatial_interval_modifiers
    );
    if let Some(category) = node.denotation.category() {
        map.serialize_entry("category", &category)?;
    }
    optional_entry!(map, "scopeDependence", node.denotation.scope_dependence());
    map.serialize_entry("sort", &SemanticSort::Eventuality(node.sort))?;
    optional_entry!(map, "indexical", node.indexical.as_ref());
    optional_entry!(map, "descriptor", node.descriptor.as_ref());
    optional_entry!(map, "composition", node.composition.as_ref());
    nonempty_entry!(map, "relativeClauses", &node.relative_clauses);
    nonempty_entry!(map, "assignedNames", &node.assigned_names);
    nonempty_entry!(map, "modalArguments", &node.modal_arguments);
    optional_entry!(map, "body", node.body.as_ref());
    nonempty_entry!(map, "parameters", &node.parameters);
    optional_entry!(map, "arity", node.arity.as_ref());
    nonempty_entry!(map, "embeddedQuestions", &node.embedded_questions);
    optional_entry!(map, "experiencer", node.experiencer.as_ref());
    optional_entry!(map, "target", node.target.as_ref());
    optional_entry!(map, "scale", node.scale.as_ref());
    optional_entry!(map, "subscript", node.subscript.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_referent<M: SerializeMap>(map: &mut M, node: &ReferentNode) -> Result<(), M::Error> {
    map.serialize_entry("category", &node.category)?;
    optional_entry!(map, "scopeDependence", node.scope_dependence.as_ref());
    map.serialize_entry("sort", &node.sort)?;
    optional_entry!(map, "indexical", node.indexical.as_ref());
    optional_entry!(map, "descriptor", node.descriptor.as_ref());
    optional_entry!(map, "composition", node.composition.as_ref());
    nonempty_entry!(map, "relativeClauses", &node.relative_clauses);
    nonempty_entry!(map, "assignedNames", &node.assigned_names);
    optional_entry!(map, "body", node.body.as_ref());
    nonempty_entry!(map, "parameters", &node.parameters);
    optional_entry!(map, "arity", node.arity.as_ref());
    nonempty_entry!(map, "embeddedQuestions", &node.embedded_questions);
    optional_entry!(map, "abstracted", node.abstracted.as_ref());
    optional_entry!(map, "experiencer", node.experiencer.as_ref());
    optional_entry!(map, "target", node.target.as_ref());
    optional_entry!(map, "scale", node.scale.as_ref());
    optional_entry!(map, "subscript", node.subscript.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_parameter<M: SerializeMap>(map: &mut M, node: &ParameterNode) -> Result<(), M::Error> {
    map.serialize_entry("sort", &node.sort)?;
    map.serialize_entry("role", &node.role)?;
    map.serialize_entry("introducedBy", &node.introduced_by)?;
    optional_entry!(map, "subscript", node.subscript.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_predication<M: SerializeMap>(
    map: &mut M,
    node: &PredicationNode,
) -> Result<(), M::Error> {
    optional_entry!(map, "eventuality", node.eventuality.as_ref());
    optional_entry!(map, "introducedBy", node.introduced_by.as_ref());
    match node.relation.as_data() {
        data!(PredicationRelation::Named { relation }) => {
            map.serialize_entry("relation", relation)?
        }
        data!(PredicationRelation::Parameter { parameter }) => {
            map.serialize_entry("relationParameter", parameter)?
        }
    }
    optional_entry!(map, "tanruLink", node.tanru_link.as_ref());
    nonempty_entry!(map, "arguments", &node.arguments);
    nonempty_entry!(map, "placeQuestions", &node.place_questions);
    nonempty_entry!(map, "modalArguments", &node.modal_arguments);
    nonempty_entry!(map, "reciprocity", &node.reciprocity);
    map.serialize_entry("mode", &node.mode)?;
    optional_entry!(map, "scalarNegation", node.scalar_negation.as_ref());
    optional_entry!(map, "relationMetadata", node.relation_metadata.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_formula<M: SerializeMap>(map: &mut M, node: &FormulaNode) -> Result<(), M::Error> {
    match node.as_data() {
        data!(FormulaNode::Atom(node)) => {
            map.serialize_entry("operator", &FormulaOperator::Atom)?;
            map.serialize_entry("predication", &node.predication)?;
            nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
        }
        data!(FormulaNode::Connective(node)) => {
            optional_entry!(map, "eventuality", node.eventuality.as_ref());
            map.serialize_entry("operator", &node.operator)?;
            nonempty_entry!(map, "children", &node.children);
            optional_entry!(map, "connector", node.connector.as_ref());
            nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
        }
        data!(FormulaNode::Quantified(node)) => {
            map.serialize_entry("operator", &node.operator)?;
            map.serialize_entry("variable", &node.variable)?;
            optional_entry!(map, "sourceVariable", node.source_variable.as_ref());
            optional_entry!(map, "selectionSource", node.selection_source.as_ref());
            optional_entry!(map, "restriction", node.restriction.as_ref());
            optional_entry!(
                map,
                "domainImport",
                quantified_formula_domain_import(node.operator, node.restriction).as_ref()
            );
            map.serialize_entry("body", &node.body)?;
            optional_entry!(map, "quantity", node.quantity.as_ref());
            nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            map.serialize_entry("operator", &FormulaOperator::QuantifierBundle)?;
            map.serialize_entry("body", &node.body)?;
            nonempty_entry!(map, "bindings", &node.bindings);
            map.serialize_entry("coequalScope", &true)?;
            nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            map.serialize_entry("operator", &FormulaOperator::RespectivelyDistribution)?;
            map.serialize_entry("body", &node.body)?;
            nonempty_entry!(map, "streams", &node.streams);
            optional_entry!(map, "distinctPartition", node.distinct_partition.as_ref());
            nonempty_entry!(map, "boundEventualities", &node.bound_eventualities);
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_sign<M: SerializeMap>(map: &mut M, node: &SignNode) -> Result<(), M::Error> {
    map.serialize_entry("category", &node.category)?;
    optional_entry!(map, "scopeDependence", node.scope_dependence.as_ref());
    map.serialize_entry("sort", &SemanticSort::Sign)?;
    optional_entry!(map, "descriptor", node.descriptor.as_ref());
    optional_entry!(map, "kind", node.sign_kind.as_ref());
    optional_entry!(map, "text", node.text.as_ref());
    nonempty_entry!(map, "letterals", &node.letterals);
    optional_entry!(map, "quotation", node.quotation.as_ref());
    optional_entry!(map, "denotes", node.denotes.as_ref());
    nonempty_entry!(map, "relativeClauses", &node.relative_clauses);
    optional_entry!(map, "target", node.target.as_ref());
    optional_entry!(map, "subscript", node.subscript.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_displayed<M: SerializeMap>(
    map: &mut M,
    node: &DisplayedContentNode,
) -> Result<(), M::Error> {
    map.serialize_entry("relation", &node.relation)?;
    map.serialize_entry("family", &node.family)?;
    optional_entry!(map, "intensity", node.intensity.as_ref());
    map.serialize_entry("polarity", &node.polarity)?;
    optional_entry!(map, "phase", node.phase.as_ref());
    nonempty_entry!(map, "modifiers", &node.modifiers);
    map.serialize_entry("assertionEffect", &node.assertion_effect)?;
    map.serialize_entry("experiencer", &node.experiencer)?;
    map.serialize_entry("target", &node.target)?;
    optional_entry!(map, "targetFocus", node.target_focus.as_ref());
    map.serialize_entry("anchor", &node.anchor)?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_math<M: SerializeMap>(map: &mut M, node: &MathExpressionNode) -> Result<(), M::Error> {
    optional_entry!(map, "scalarNegation", node.scalar_negation.as_ref());
    match node.kind.as_data() {
        data!(MathExpressionNodeKind::Literal { literal, denotes }) => {
            optional_entry!(map, "denotes", denotes.as_ref());
            map.serialize_entry("literal", literal)?;
        }
        data!(MathExpressionNodeKind::Operator {
            operator,
            operands,
            operator_denotes,
            endpoint_inclusion
        }) => {
            map.serialize_entry("operator", operator)?;
            optional_entry!(map, "operatorDenotes", operator_denotes.as_ref());
            optional_entry!(map, "endpointInclusion", endpoint_inclusion.as_ref());
            nonempty_entry!(map, "operands", operands);
        }
        data!(MathExpressionNodeKind::QuestionedOperator {
            operator_parameter,
            operands
        }) => {
            map.serialize_entry("operatorParameter", operator_parameter)?;
            nonempty_entry!(map, "operands", operands);
        }
    }
    optional_entry!(map, "subscript", node.subscript.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_quantity<M: SerializeMap>(map: &mut M, node: &QuantityNode) -> Result<(), M::Error> {
    map.serialize_entry("form", &node.form)?;
    map.serialize_entry("value", &node.value)?;
    map.serialize_entry("scale", &node.scale)?;
    optional_entry!(map, "comparisonSet", node.comparison_set.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_relation_metadata<M: SerializeMap>(
    map: &mut M,
    node: &RelationMetadataNode,
) -> Result<(), M::Error> {
    map.serialize_entry("relation", &node.relation)?;
    nonempty_entry!(map, "sourceWords", &node.source_words);
    nonempty_entry!(map, "placeStructure", &node.place_structure);
    optional_entry!(map, "expansion", node.expansion.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn serialize_question<M: SerializeMap>(map: &mut M, node: &QuestionNode) -> Result<(), M::Error> {
    map.serialize_entry("body", &node.body)?;
    map.serialize_entry("kind", &node.kind)?;
    map.serialize_entry("mode", &node.mode)?;
    map.serialize_entry("asker", &node.asker)?;
    map.serialize_entry("respondent", &node.respondent)?;
    map.serialize_entry("domain", &node.domain)?;
    nonempty_entry!(map, "slots", &node.slots);
    optional_entry!(map, "focus", node.focus.as_ref());
    optional_entry!(map, "presupposedAnswer", node.presupposed_answer.as_ref());
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn references_into(object: &SemanticObject, out: &mut Vec<SemanticObjectId>) {
    // Reference collection is implemented after consumer migration; keeping it
    // centralized here prevents the serde boundary from becoming graph logic.
    match object.as_data() {
        data!(SemanticObject::Utterance(node)) => {
            out.extend([node.speaker, node.audience, node.eventuality]);
            extend_optional(out, node.content);
            out.extend([node.deictic_ground.time, node.deictic_ground.place]);
            out.extend(node.asides.iter().copied());
        }
        data!(SemanticObject::Sequence(node)) => {
            out.extend(node.items.iter().copied());
            extend_optional(out, node.content);
            out.extend(node.connection_claims.iter().copied());
            for label in &node.ordinal_labels {
                label.references_into(out);
            }
            if let Some(connection) = &node.nonlogical_connection {
                connection.references_into(out);
            }
            out.extend(
                node.bound_eventualities
                    .iter()
                    .map(|eventuality| eventuality.object_id()),
            );
        }
        data!(SemanticObject::Eventuality(node)) => {
            extend_optional(out, node.tense_modal);
            extend_optional(out, node.content);
            extend_optional(out, node.body);
            out.extend(node.parameters.iter().copied());
            out.extend(node.embedded_questions.iter().copied());
            extend_optional(out, node.experiencer);
            extend_optional(out, node.scale);
            extend_optional(out, node.target);
            for modal in &node.modal_arguments {
                modal.references_into(out);
            }
            if let Some(time_interval) = &node.time_interval {
                time_interval.references_into(out);
            }
            if let Some(time_span) = &node.time_span {
                time_span.references_into(out);
            }
            if let Some(aspect) = &node.aspect {
                aspect.references_into(out);
            }
            for aspect in &node.aspects {
                aspect.references_into(out);
            }
            for recurrence in &node.recurrence {
                recurrence.references_into(out);
            }
            for modifier in &node.interval_modifiers {
                modifier.references_into(out);
            }
            if let Some(space_interval) = &node.space_interval {
                space_interval.references_into(out);
            }
            if let Some(aspect) = &node.spatial_aspect {
                aspect.references_into(out);
            }
            for aspect in &node.spatial_aspects {
                aspect.references_into(out);
            }
            for recurrence in &node.spatial_recurrence {
                recurrence.references_into(out);
            }
            for modifier in &node.spatial_interval_modifiers {
                modifier.references_into(out);
            }
            if let Some(subscript) = &node.subscript {
                subscript.references_into(out);
            }
            collect_referent_and_event_references(
                node.descriptor.as_ref(),
                node.composition.as_ref(),
                &node.relative_clauses,
                &node.time,
                &node.time_path,
                &node.space,
                &node.space_path,
                out,
            );
        }
        data!(SemanticObject::Referent(node)) => {
            extend_optional(out, node.body);
            out.extend(node.parameters.iter().copied());
            out.extend(node.embedded_questions.iter().copied());
            extend_optional(out, node.abstracted);
            extend_optional(out, node.experiencer);
            extend_optional(out, node.scale);
            extend_optional(out, node.target);
            collect_referent_references(
                node.descriptor.as_ref(),
                node.composition.as_ref(),
                &node.relative_clauses,
                out,
            );
            if let Some(subscript) = &node.subscript {
                subscript.references_into(out);
            }
        }
        data!(SemanticObject::Parameter(node)) => {
            if let Some(subscript) = &node.subscript {
                subscript.references_into(out);
            }
        }
        data!(SemanticObject::Predication(node)) => collect_predication_references(node, out),
        data!(SemanticObject::Formula(node)) => collect_formula_references(node, out),
        data!(SemanticObject::Sign(node)) => {
            if let Some(descriptor) = &node.descriptor {
                descriptor.references_into(out);
            }
            if let Some(quotation) = &node.quotation {
                quotation.references_into(out);
            }
            extend_optional(out, node.denotes);
            out.extend(node.relative_clauses.iter().map(|clause| clause.body));
            extend_optional(out, node.target);
            if let Some(subscript) = &node.subscript {
                subscript.references_into(out);
            }
        }
        data!(SemanticObject::DisplayedContent(node)) => {
            out.extend([node.experiencer, node.target, node.anchor])
        }
        data!(SemanticObject::MathExpression(node)) => {
            match node.kind.as_data() {
                data!(MathExpressionNodeKind::Literal { denotes, .. }) => {
                    extend_optional(out, *denotes)
                }
                data!(MathExpressionNodeKind::Operator {
                    operands,
                    operator_denotes,
                    ..
                }) => {
                    out.extend(operands.iter().copied());
                    extend_optional(out, *operator_denotes);
                }
                data!(MathExpressionNodeKind::QuestionedOperator {
                    operator_parameter,
                    operands
                }) => {
                    out.push(*operator_parameter);
                    out.extend(operands.iter().copied());
                }
            }
            if let Some(scalar) = &node.scalar_negation {
                scalar.references_into(out);
            }
            if let Some(subscript) = &node.subscript {
                subscript.references_into(out);
            }
        }
        data!(SemanticObject::Quantity(node)) => {
            node.value.references_into(out);
            extend_optional(out, node.comparison_set);
        }
        data!(SemanticObject::RelationMetadata(node)) => {
            if let Some(expansion) = &node.expansion {
                expansion.references_into(out);
            }
        }
        data!(SemanticObject::Question(node)) => {
            out.extend([node.asker, node.respondent, node.body]);
            out.extend(node.slots.iter().filter_map(QuestionSlot::parameter));
            extend_optional(out, node.focus);
            extend_optional(out, node.presupposed_answer);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_referent_references(
    descriptor: Option<&Descriptor>,
    composition: Option<&Composition>,
    relative_clauses: &[RelativeClause],
    out: &mut Vec<SemanticObjectId>,
) {
    if let Some(descriptor) = descriptor {
        descriptor.references_into(out);
    }
    if let Some(composition) = composition {
        out.extend(composition.members.iter().copied());
        out.extend(composition.excluded_members.iter().copied());
        extend_optional(out, composition.operator_parameter);
    }
    out.extend(relative_clauses.iter().map(|clause| clause.body));
}

#[requires(true)]
#[ensures(true)]
fn collect_referent_and_event_references(
    descriptor: Option<&Descriptor>,
    composition: Option<&Composition>,
    relative_clauses: &[RelativeClause],
    time: &Option<AnchorRelation>,
    time_path: &[TemporalPathStep],
    space: &Option<AnchorRelation>,
    space_path: &[TemporalPathStep],
    out: &mut Vec<SemanticObjectId>,
) {
    collect_referent_references(descriptor, composition, relative_clauses, out);
    if let Some(time) = time {
        time.references_into(out);
    }
    for step in time_path {
        step.references_into(out);
    }
    if let Some(space) = space {
        space.references_into(out);
    }
    for step in space_path {
        step.references_into(out);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_predication_references(node: &PredicationNode, out: &mut Vec<SemanticObjectId>) {
    if let data!(PredicationRelation::Parameter { parameter }) = node.relation.as_data() {
        out.push(*parameter);
    }
    extend_optional(out, node.eventuality);
    if let Some(tanru) = &node.tanru_link {
        tanru.references_into(out);
    }
    for argument in node.arguments.values() {
        argument.references_into(out);
    }
    for question in &node.place_questions {
        question.references_into(out);
    }
    for modal in &node.modal_arguments {
        modal.references_into(out);
    }
    for exchange in &node.reciprocity {
        exchange.references_into(out);
    }
    if let Some(scalar) = &node.scalar_negation {
        scalar.references_into(out);
    }
    extend_optional(out, node.relation_metadata);
}

#[requires(true)]
#[ensures(true)]
fn collect_formula_references(node: &FormulaNode, out: &mut Vec<SemanticObjectId>) {
    match node.as_data() {
        data!(FormulaNode::Atom(node)) => out.push(node.predication),
        data!(FormulaNode::Connective(node)) => {
            extend_optional(out, node.eventuality);
            out.extend(node.children.iter().copied());
            if let Some(connector) = &node.connector {
                connector.references_into(out);
            }
        }
        data!(FormulaNode::Quantified(node)) => {
            out.push(node.variable);
            extend_optional(out, node.source_variable);
            if let Some(selection) = &node.selection_source {
                selection.references_into(out);
            }
            extend_optional(out, node.restriction);
            out.push(node.body);
            extend_optional(out, node.quantity);
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            for binding in &node.bindings {
                binding.references_into(out);
            }
            out.push(node.body);
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            out.push(node.body);
            for stream in &node.streams {
                stream.references_into(out);
            }
        }
    }
    out.extend(
        match node.as_data() {
            data!(FormulaNode::Atom(node)) => &node.bound_eventualities,
            data!(FormulaNode::Connective(node)) => &node.bound_eventualities,
            data!(FormulaNode::Quantified(node)) => &node.bound_eventualities,
            data!(FormulaNode::QuantifierBundle(node)) => &node.bound_eventualities,
            data!(FormulaNode::RespectivelyDistribution(node)) => &node.bound_eventualities,
        }
        .iter()
        .map(|eventuality| eventuality.object_id()),
    );
}
