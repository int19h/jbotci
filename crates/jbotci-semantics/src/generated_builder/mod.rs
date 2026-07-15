//! Semantic builder that consumes the generated syntax model directly.

use std::collections::{BTreeMap, BTreeSet, HashSet};

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, expensive_ensures, invariant, new, requires};
use jbotci_dictionary::Dictionary;
use jbotci_morphology::{
    Cmavo, LujvoPart, Selmaho, Word, WordData, WordLike, WordLikeData, push_stripped_diacritics_to,
    strip_diacritics,
};
use jbotci_source::SourceSpan;
use jbotci_syntax::generated_model::{
    AbstractionTanruUnitSyntax, AbstractorConnectionSyntax, AfterthoughtBridiTailSyntax,
    AfterthoughtBridiTailWithoutTailTermsSyntax, AfterthoughtMeksoOperatorSyntax,
    ArgumentConnectiveSyntax, ArrayMeksoOperandSyntax, AssignedProBridiTanruUnitSyntax,
    AtomRef as GeneratedAtomRef, BareCuBridiSyntax, BareCuTermsBridiSyntax,
    BoGroupedBridiTailSyntax, BoGroupedBridiTailWithoutTailTermsSyntax, BoOrLinkedTanruUnitSyntax,
    BoundMeksoOperandSyntax, BoundMeksoOperatorSyntax, BoundOrAtomMeksoOperatorSyntax,
    BoundOrSimpleMeksoOperandSyntax, BoundTanruUnitSyntax, BridiRelativeClauseSyntax,
    BridiStatementContinuationSyntax, BridiStatementSyntax, BridiSubbridiSyntax, BridiSyntax,
    BridiTailConnectiveSyntax, BridiTailSyntax, BridiTailWithPossibleTailTermsSyntax,
    BridiWithLeadingTermsSyntax, BridiWithPostCuTermsSyntax, ClosedIntervalConnectiveSyntax,
    CmevlaVocativeSumtiSyntax, CoSelbriSyntax, ConnectedJaiInnerSelbriSyntax,
    ConnectedSelbriSyntax, ConnectedTermSyntax, CuTermsBridiTailSyntax, DescriptionHeadSyntax,
    DescriptionTailBodySyntax, DescriptionTailSyntax, DescriptorWithGadriSumtiSyntax,
    DescriptorWithOuterQuantifierSumtiSyntax, DescriptorWithoutGadriSumtiSyntax,
    DirectForethoughtBridiConnectionSyntax, EkFragmentSyntax, ExplicitXauhaLohoiTextSyntax,
    FollowingParagraphStatementSyntax, ForethoughtBridiConnectionSyntax,
    ForethoughtBridiConnectionWithoutTailTermsSyntax, ForethoughtCallMeksoSyntax,
    ForethoughtMeksoOperandSyntax, ForethoughtMeksoOperatorSyntax,
    ForethoughtSelbriConnectionSyntax, ForethoughtSelbriGroupTanruUnitSyntax,
    ForethoughtStatementSyntax, ForethoughtSumtiSyntax, ForethoughtTermsetSyntax,
    FragmentStatementSyntax, FreeModifierSyntax, GihekConnectiveSyntax, GihekFragmentSyntax,
    GikConnectiveSyntax, GohaWordTanruUnitSyntax, GroupedForethoughtBridiConnectionSyntax,
    GroupedTanruUnitSyntax, GuhekConnectiveSyntax, IParagraphStatementConnectiveSyntax,
    IStatementConnectionSyntax, IStatementConnectionTailSyntax, IStatementConnectiveSyntax,
    InfixMeksoSyntax, JaiInnerTanruUnitSyntax, JaiModalTanruUnitSyntax, JekConnectiveSyntax,
    JoiConnectiveSyntax, JoikConnectiveSyntax, LaheSumtiSyntax, LeadingIStatementSyntax,
    LeadingIndicatorSyntax, LeadingTermTagTenseModalSyntax, LerfuStringMeksoSyntax,
    LerfuStringSumtiSyntax, LetterStringContinuationSyntax, LetterStringSyntax, LetterTokensSyntax,
    LinkargsSyntax, LinkedSumtiContinuationFragmentSyntax, LinkedSumtiFragmentSyntax,
    LinkedSumtiSyntax, LinkedTanruUnitForCeiSyntax, LinkedTanruUnitSyntax, MeksoBaseSyntax,
    MeksoFragmentSyntax, MeksoOperandSyntax, MeksoOperatorSyntax, MeksoPrecedenceSyntax,
    MeksoSyntax, ModalForethoughtConnectiveSyntax, MultipleNaFragmentSyntax, NameSumtiSyntax,
    NegatedForethoughtBridiConnectionSyntax, NihoParagraphSyntax, NodeRef as GeneratedNodeRef,
    NoihaAdverbialTermSyntax, NumberMeksoSyntax, NumberSumtiSyntax, NumberWordContinuationSyntax,
    NumberWordsSyntax, OperatorSelbriTanruUnitSyntax, OrdinalTanruUnitSyntax,
    ParagraphStandardStatementConnectiveSyntax, ParagraphStatementSequenceSyntax, ParagraphSyntax,
    ParenthesizedMeksoOperandSyntax, PeheTermsetConnectionSyntax, PeheTermsetOperandSyntax,
    PendingIConnectiveSyntax, PlainLinkedSumtiSyntax, PlainRelativeSumtiSyntax,
    PrenexFragmentSyntax, PrenexStatementSyntax, PrenexSubbridiSyntax,
    PreposedIStatementConnectionSyntax, ProBridiTanruUnitSyntax, ProSumtiSyntax,
    QualifiedMeksoOperandSyntax, QuantifiedSumtiSyntax, QuantifierRelationDescriptionTailSyntax,
    QuantifierSumtiDescriptionTailSyntax, QuantifierSyntax, QuoteSyntax, QuotedSumtiSyntax,
    RegularTextSyntax, RelationAfterthoughtConnectiveSyntax, RelationDescriptionTailSyntax,
    RelationOnlyBridiSyntax, RelativeClauseAtomSyntax, RelativeClauseFragmentSyntax,
    RelativeClauseListSyntax, RelativeClauseTailSyntax, RelativeSumtiSyntax,
    RestrictiveBridiRelativeClauseSyntax, ReversePolishMeksoSyntax, ReversePolishPartsSyntax,
    ScalarNegatedSumtiSyntax, ScalarNegatedSumtiWithBoSyntax, ScalarNegatedTanruInnerUnitSyntax,
    ScalarNegatedTanruUnitSyntax, SelbriFragmentSyntax, SelbriMeksoOperandSyntax,
    SelbriSimpleBridiTailSyntax, SelbriSyntax, SelbriVocativeSumtiSyntax, SimpleBridiTailSyntax,
    SimpleBridiTailWithoutTailTermsSyntax, SimpleIntervalConnectiveSyntax,
    SimpleMeksoOperandSyntax, SimpleMeksoOperatorSyntax, SimpleParagraphSyntax, SimpleSumtiSyntax,
    SimpleTermSyntax, SingleNaFragmentSyntax, SoiFreeModifierSyntax,
    StandardStatementConnectiveSyntax, StatementAfterIConnectiveSyntax, StatementBaseSyntax,
    StatementConnectiveSyntax, StatementOrFragmentStatementSyntax, StatementOrFragmentSyntax,
    StatementSyntax, SubbridiSyntax, SumtiAfterthoughtSyntax, SumtiAssociationRelativeClauseSyntax,
    SumtiAtomSyntax, SumtiBaseSyntax, SumtiBoundSyntax, SumtiConnectionTailSyntax,
    SumtiForethoughtSyntax, SumtiGroupedSyntax, SumtiMeksoOperandSyntax, SumtiSelbriSumtiSyntax,
    SumtiSelbriTanruUnitSyntax, SumtiSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax,
    TaggedSumtiTermSyntax, TanruJaiInnerSelbriSyntax, TanruSelbriSyntax,
    TanruUnitAtomBaseForCeiSyntax, TanruUnitAtomBaseSyntax, TanruUnitAtomForCeiSyntax,
    TanruUnitAtomSyntax, TanruUnitSyntax, TenseModalAtomSyntax, TenseModalBodySyntax,
    TenseModalSyntax, TenseTaggedRelativeSumtiSyntax, TermSyntax, TermsFragmentSyntax,
    TermsetGroupSyntax, TextGroupStatementSyntax, TextLeadingConnectiveSyntax,
    TextNihoParagraphsSyntax, TextParagraphWithAdditionalNihoSyntax, TextParagraphsSyntax,
    TextSyntax, TreeNode, UntaggedSelbriSyntax, VocativeFreeModifierSyntax,
    VocativeMarkerWordsSyntax, VocativeSumtiSyntax, VuhoSumtiAttachmentTailSyntax,
    WordTanruUnitSyntax, ZantufaBoGroupedMeksoBaseSyntax, ZantufaExtraGikConnectiveSyntax,
    ZantufaGroupedMeksoOperandSequenceSyntax, ZantufaInfixMeksoSyntax, ZantufaMeSelbriBodySyntax,
    ZantufaMeTanruUnitSyntax, ZantufaMeksoFragmentSyntax, ZantufaMexMoiTanruUnitSyntax,
    ZantufaReversePolishMeksoSyntax, ZantufaStatementAbstractionTanruUnitSyntax,
    ZantufaStatementTermsStatementSyntax, ZantufaStatementTermsTailSyntax,
};
use jbotci_syntax::tree::{Token, WithFreeModifiers, WithIndicators, WithIndicatorsData};
use jbotci_tree::TreeVisitor;
use vec1::Vec1;

use crate::facade::{
    SemanticBuildOptions, SemanticsError, SemanticsErrorKind, dictionary_relation_place_count,
};
use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorMagnitude, AnchorRelation, AnchorRelationData,
    ArgumentValue, ArgumentValueData, ArgumentValueKind, Aspect, AssignedName, AssignedNameData,
    CommandTarget, Composition, CompositionOperator, Connector, Descriptor, DescriptorDefiniteness,
    DescriptorKind, DisplayedContentAssertionEffect, DisplayedContentFamily,
    DisplayedContentModifier, DisplayedContentNode, DisplayedContentPolarity,
    DisplayedContentTargetFocus, ElidedConnectionOperand, EventualityClass, EventualityNode,
    EventualityNodeData, EventualitySort, ForethoughtRelationBranch, FormulaNode, FormulaNodeData,
    FormulaOperator, FormulaTraversal, IndexicalKind, IntervalEndpointInclusion, IntervalModifier,
    IntervalModifierData, LetteralUnit, LetteralUnitKind, MathExpressionNode,
    MathExpressionNodeData, MathExpressionNodeKind, MathExpressionNodeKindData, MathLiteral,
    MathLiteralKind, MathOperator, MathOperatorData, MixedRadixComponent, ModalArgument,
    ModalArgumentData, ModalNegation, ModalNegationKind, NonlogicalConnection, ParagraphTransition,
    ParameterRole, PlaceIndex, PlaceQuestionBinding, PlaceQuestionBindingData, PredicationMode,
    PredicationNode, PredicationNodeData, PredicationRelationData, QuantifierBinding,
    QuantifierBundleFormulaNode, QuantityForm, QuantityScale, QuantityValue, QuestionKind,
    QuestionMode, QuestionNode, QuestionSlot, QuestionSlotRole, Quotation, RafsiBinding,
    ReciprocalExchange, ReciprocalExchangeData, Recurrence, RecurrenceConnection,
    RecurrenceConnectionKind, RecurrenceKind, ReferentCategory, ReferentNode, RelationExpansion,
    RelationLabel, RelationLabelData, RelativeClause, RelativeClauseKind, RespectivelyStream,
    ScalarNegation, ScalarNegationKind, SelectionSource, SemanticGraph, SemanticObject,
    SemanticObjectData, SemanticObjectId, SemanticSort, SequenceNode, SequenceRelation, SignKind,
    SignNode, SourceByteSpan, SpaceInterval, SpatialMotion, SpatialMotionKind, Subscript,
    TanruLink, TanruLinkData, TemporalPathAnchor, TemporalPathStep, TemporalPathStepData,
    TimeInterval, TimeSpan, TimeSpanEndpoint, UtteranceForce, UtteranceNode,
    argument_object_kind_can_fill, diagnostic, displayed_content_target_kind_is_allowed,
    source_from_spans,
};

mod connectives;
mod elision;
mod formulas;
mod mekso;
mod numbers_letterals;
mod pro_bridi;
mod relation_dispatch;
mod relations;
mod sources;
mod statements;
mod sumti_connections;
mod sumti_referents;
mod surface;
mod tanru_property;
mod tense_modal;
mod text_plan;

use connectives::*;
use mekso::*;
use numbers_letterals::*;
use relation_dispatch::*;
use relations::*;
use sources::*;
use surface::*;
use tense_modal::*;
use text_plan::*;

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
pub fn build_generated_semantic_graph_with_dictionary(
    syntax: &TextSyntax,
    source_text: Option<&str>,
    dictionary: &Dictionary<'_>,
) -> Result<SemanticGraph, SemanticsError> {
    build_generated_semantic_graph_with_dictionary_and_options(
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
pub fn build_generated_semantic_graph_with_dictionary_and_options<'a>(
    syntax: &TextSyntax,
    options: SemanticBuildOptions<'a>,
    dictionary: &Dictionary<'_>,
) -> Result<SemanticGraph, SemanticsError> {
    let builder = GeneratedGraphBuilder::new(options, dictionary);
    builder.build_text(syntax)
}

#[invariant(true)]
struct GeneratedGraphBuilder<'a, 'dict, 'syntax> {
    options: SemanticBuildOptions<'a>,
    dictionary: &'dict Dictionary<'dict>,
    objects: BTreeMap<SemanticObjectId, SemanticObject>,
    next_index: usize,
    relative_head_stack: Vec<SemanticObjectId>,
    current_utterance: Option<SemanticObjectId>,
    previous_utterance: Option<SemanticObjectId>,
    next_utterance: Option<SemanticObjectId>,
    current_speaker: SemanticObjectId,
    current_audience: SemanticObjectId,
    current_now: SemanticObjectId,
    current_here: SemanticObjectId,
    content_eventualities: BTreeMap<SemanticObjectId, SemanticObjectId>,
    scoped_argument_variables: BTreeMap<(usize, usize), SemanticObjectId>,
    direct_question_slots: Vec<GeneratedDirectQuestionSlot>,
    relation_variable_parameters: BTreeMap<(usize, usize), SemanticObjectId>,
    implicit_existential_variables: Vec<GeneratedImplicitExistential>,
    recorded_implicit_existential_variables: HashSet<SemanticObjectId>,
    implicit_da_series_bindings: BTreeMap<String, SemanticObjectId>,
    quantified_da_series_bindings: BTreeMap<String, GeneratedSemanticDaSeriesScopeBinding>,
    sticky_modal_arguments: BTreeMap<GeneratedStickyModalKey, ModalArgument>,
    host_event_modal_elisions: BTreeMap<SemanticObjectId, Vec<GeneratedHostEventModalElision>>,
    sticky_time_path: Vec<TemporalPathStep>,
    sticky_space_path: Vec<TemporalPathStep>,
    story_time_anchor: Option<SemanticObjectId>,
    pending_event_modifiers: BTreeMap<SemanticObjectId, Vec<GeneratedEventTenseModifier>>,
    deferred_event_modifier_flush_depth: usize,
    prenex_pro_sumti_bindings: BTreeMap<String, Vec<GeneratedPrenexProSumtiBinding>>,
    prenex_relation_variable_bindings:
        BTreeMap<String, Vec<GeneratedPrenexRelationVariableBinding>>,
    abstraction_parameter_stack: Vec<Vec<SemanticObjectId>>,
    indirect_question_stack: Vec<Vec<GeneratedIndirectQuestionFocus>>,
    temporal_context_stack: Vec<SemanticObjectId>,
    pro_bridi_scope_stack: Vec<&'syntax BridiSyntax>,
    completed_pro_bridi_frames: Vec<GeneratedProBridiFrame<'syntax>>,
    current_quote_depth: usize,
    sumti_referents: BTreeMap<(usize, usize), SemanticObjectId>,
    sumti_referent_cache_bypass_depth: usize,
    letter_sumti_referents: BTreeMap<String, Vec<GeneratedLetterSumtiReferent>>,
    pending_sumti_candidates: Vec<GeneratedPendingSumtiCandidate<'syntax>>,
    recent_sumti_referents: Vec<GeneratedRecentSumtiReferent>,
    assigned_referents: BTreeMap<String, SemanticObjectId>,
    math_variable_referents: BTreeMap<String, SemanticObjectId>,
    assigned_pro_bridi_bindings: BTreeMap<String, GeneratedAssignedProBridiBinding<'syntax>>,
    pending_asides: Vec<SemanticObjectId>,
    defer_active_prenex_implicit_existentials: usize,
    deferred_active_prenex_implicit_existentials: Vec<GeneratedImplicitExistential>,
    pending_negated_selbri_argument_scope_reservations: usize,
    suppress_prenex_bound_implicit_existential_recording: usize,
    pending_after_eventuality_reservations: usize,
}

#[invariant(!relation.is_empty(), "modal relation must be named")]
#[invariant(!introduced_by.is_empty(), "modal introducer must be named")]
#[invariant(*place > 0, "modal argument place must be numbered from one")]
#[invariant(argument.kind == ArgumentValueKind::Elided, "stored modal host replacement must be elided")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedHostEventModalElision {
    relation: String,
    introduced_by: String,
    source: Option<crate::model::SemanticSource>,
    place: usize,
    argument: ArgumentValue,
}

#[invariant(source_key.0 <= source_key.1, "recent sumti source key must be ordered")]
#[invariant(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneratedRecentSumtiReferent {
    source_key: (usize, usize),
    referent: SemanticObjectId,
}

#[invariant(crate::model::question_kind_domain_are_coherent(*kind, *domain))]
#[invariant(*kind != QuestionKind::Multiple)]
#[invariant((*kind == QuestionKind::Truth) == parameter.is_none())]
#[invariant(parameter.is_none_or(|parameter| parameter.object_kind() == crate::model::SemanticObjectKind::Parameter))]
#[derive(Debug, Clone)]
struct GeneratedDirectQuestionSlot {
    parameter: Option<SemanticObjectId>,
    kind: QuestionKind,
    domain: SemanticSort,
    source_order: usize,
}

#[invariant(focus.object_kind() == crate::model::SemanticObjectKind::Parameter || focus.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(slots.iter().all(|slot| slot.parameter().is_some_and(|parameter| parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)))]
#[derive(Debug, Clone)]
struct GeneratedIndirectQuestionFocus {
    focus: SemanticObjectId,
    presupposed_answer: Option<SemanticObjectId>,
    slots: Vec<QuestionSlot>,
    kind: QuestionKind,
    domain: SemanticSort,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(source_key.0 <= source_key.1)]
#[derive(Debug, Clone)]
struct GeneratedPendingSumtiCandidate<'syntax> {
    source_key: (usize, usize),
    sumti: &'syntax SumtiSyntax,
}

#[invariant(!key.is_empty())]
#[invariant(source_key.0 <= source_key.1)]
#[invariant(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[derive(Debug, Clone)]
struct GeneratedLetterSumtiReferent {
    key: String,
    source_key: (usize, usize),
    referent: SemanticObjectId,
}

#[invariant(!introduced_by.is_empty(), "sticky modal key must preserve its source marker")]
#[invariant(!relation.is_empty(), "sticky modal key must preserve its source relation")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GeneratedStickyModalKey {
    introduced_by: String,
    relation: String,
}

impl GeneratedStickyModalKey {
    #[requires(!modal_argument.introduced_by.is_empty())]
    #[requires(modal_argument.relation.as_ref().is_some_and(|relation| !relation.is_empty()))]
    #[ensures(ret.introduced_by == modal_argument.introduced_by)]
    fn for_modal_argument(modal_argument: &ModalArgument) -> Self {
        let relation = modal_argument
            .relation
            .as_ref()
            .expect("precondition guarantees relation modal")
            .clone();
        Self::from_data(data!(GeneratedStickyModalKey {
            introduced_by: modal_argument.introduced_by.clone(),
            relation,
        }))
    }
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

#[invariant(::MarkerOnly => true)]
#[invariant(::VisibleArgumentsAndLinkargs => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedScalarNegationScope {
    MarkerOnly,
    VisibleArgumentsAndLinkargs,
}

#[invariant(eventuality.is_none_or(|eventuality| eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
#[derive(Debug, Clone)]
struct ScalarNegationContext {
    eventuality: Option<SemanticObjectId>,
    scalar_negation: ScalarNegation,
}

#[invariant(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[invariant(head_predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
#[derive(Debug, Clone)]
struct GeneratedTanruFormulaForArgument {
    formula: SemanticObjectId,
    x1_argument: ArgumentValue,
    head_predication: SemanticObjectId,
}

#[invariant(relation.as_ref().is_some_and(|relation| relation.is_displayable()) != tanru.is_some(), "assigned pro-bridi binding must have exactly one target")]
#[invariant(visible_arguments.keys().all(|place| *place > 0), "visible places are 1-based")]
#[derive(Debug, Clone)]
struct GeneratedAssignedProBridiBinding<'syntax> {
    relation: Option<RelationLabel>,
    tanru: Option<&'syntax TanruSelbriSyntax>,
    source: Option<crate::model::SemanticSource>,
    visible_arguments: BTreeMap<usize, ArgumentValue>,
}

#[invariant(relation.is_displayable())]
#[invariant(arguments.keys().all(|place| place.get() > 0))]
#[derive(Debug, Clone)]
struct GeneratedProBridiFrame<'syntax> {
    relation: RelationLabel,
    arguments: BTreeMap<PlaceIndex, ArgumentValue>,
    place_count: Option<usize>,
    event_tense: Option<&'syntax TenseModalSyntax>,
    quote_depth: usize,
    replay: Option<GeneratedProBridiReplaySource<'syntax>>,
    predication_source: Option<crate::model::SemanticSource>,
    formula_source: Option<crate::model::SemanticSource>,
    diagnostics: Vec<crate::model::SemanticDiagnostic>,
}

#[invariant(*first_visible_place > 0)]
#[derive(Debug, Clone)]
struct GeneratedProBridiReplaySource<'syntax> {
    selbri: &'syntax SelbriSyntax,
    terms: Vec<&'syntax TermSyntax>,
    first_visible_place: usize,
}

#[invariant(::Bridi(_) => true)]
#[invariant(::Fragment(_) => true)]
#[invariant(::StatementConnection(_) => true)]
#[invariant(::PreposedStatementConnection(_) => true)]
#[invariant(::PrenexStatement(_) => true)]
#[invariant(::TextGroupStatement(_) => true)]
#[invariant(::ForethoughtStatement(_) => true)]
#[invariant(::ZantufaStatementTerms(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedTextRoot<'syntax> {
    Bridi(&'syntax BridiSyntax),
    Fragment(GeneratedFragmentRoot<'syntax>),
    StatementConnection(&'syntax IStatementConnectionSyntax),
    PreposedStatementConnection(&'syntax PreposedIStatementConnectionSyntax),
    PrenexStatement(&'syntax PrenexStatementSyntax),
    TextGroupStatement(&'syntax TextGroupStatementSyntax),
    ForethoughtStatement(&'syntax ForethoughtStatementSyntax),
    ZantufaStatementTerms(&'syntax ZantufaStatementTermsStatementSyntax),
}

#[invariant(::Prenex(_) => true)]
#[invariant(::Selbri(_) => true)]
#[invariant(::Ek(_) => true)]
#[invariant(::Gihek(_) => true)]
#[invariant(::MultipleNa(_) => true)]
#[invariant(::SingleNa(_) => true)]
#[invariant(::Terms(_) => true)]
#[invariant(::Mekso(_) => true)]
#[invariant(::RelativeClause(_) => true)]
#[invariant(::LinkedSumtiContinuation(_) => true)]
#[invariant(::LinkedSumti(_) => true)]
#[invariant(::ZantufaMekso(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedFragmentRoot<'syntax> {
    Prenex(&'syntax PrenexFragmentSyntax),
    Selbri(&'syntax SelbriFragmentSyntax),
    Ek(&'syntax EkFragmentSyntax),
    Gihek(&'syntax GihekFragmentSyntax),
    MultipleNa(&'syntax MultipleNaFragmentSyntax),
    SingleNa(&'syntax SingleNaFragmentSyntax),
    Terms(&'syntax TermsFragmentSyntax),
    Mekso(&'syntax MeksoFragmentSyntax),
    RelativeClause(&'syntax RelativeClauseFragmentSyntax),
    LinkedSumtiContinuation(&'syntax LinkedSumtiContinuationFragmentSyntax),
    LinkedSumti(&'syntax LinkedSumtiFragmentSyntax),
    ZantufaMekso(&'syntax ZantufaMeksoFragmentSyntax),
}

#[invariant(content.is_none_or(|content| crate::model::argument_object_kind_can_fill(content.object_kind())))]
#[derive(Debug, Clone)]
struct GeneratedFragmentSemantics {
    content: Option<SemanticObjectId>,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
#[invariant(last_item.object_kind() == crate::model::SemanticObjectKind::Utterance || last_item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
#[invariant(formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
#[derive(Debug, Clone)]
struct GeneratedStatementConnectionOperand {
    item: SemanticObjectId,
    formula: Option<SemanticObjectId>,
    last_item: SemanticObjectId,
    spans: Vec<SourceSpan>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedStatementConnectionTail<'syntax> {
    left_operand: GeneratedStatementConnectionOperand,
    i: &'syntax Token,
    connective: &'syntax IStatementConnectiveSyntax,
    trailing_statement: &'syntax StatementAfterIConnectiveSyntax,
    spans: Vec<SourceSpan>,
    operand: GeneratedStatementConnectionOperand,
}

#[invariant(true)]
#[derive(Debug)]
struct GeneratedTextPlan<'syntax> {
    leading_nai: &'syntax [Token],
    leading_cmevla: &'syntax [Token],
    leading_indicators: &'syntax [LeadingIndicatorSyntax],
    leading_free_modifiers: Vec<&'syntax FreeModifierSyntax>,
    leading_connective:
        Option<&'syntax jbotci_syntax::generated_model::TextLeadingConnectiveSyntax>,
    leading_i_statements: &'syntax [LeadingIStatementSyntax],
    items: Vec<GeneratedTextPlanItem<'syntax>>,
}

#[invariant(!source.is_empty())]
#[derive(Debug)]
struct GeneratedRelationLabelConnector {
    source: String,
    has_bo: bool,
}

#[invariant(!connector.is_empty())]
#[invariant(trailing.is_displayable())]
#[derive(Debug)]
struct GeneratedPendingRelationLabelConnection {
    connector: String,
    trailing: RelationLabel,
}

#[invariant(::Root { .. } => true)]
#[invariant(::StandaloneParagraphBoundary { .. } => true)]
#[invariant(::StandaloneFreeModifiers(_) => true)]
#[invariant(::PendingStatementConnection { .. } => true)]
#[invariant(::TrailingSeparator { .. } => true)]
#[derive(Debug)]
enum GeneratedTextPlanItem<'syntax> {
    Root {
        root: GeneratedTextRoot<'syntax>,
        free_modifiers: Vec<&'syntax FreeModifierSyntax>,
        separator_i: Option<&'syntax Token>,
    },
    StandaloneParagraphBoundary {
        markers: &'syntax Vec1<Token>,
        free_modifiers: Vec<&'syntax FreeModifierSyntax>,
    },
    StandaloneFreeModifiers(Vec<&'syntax FreeModifierSyntax>),
    PendingStatementConnection {
        i: &'syntax Token,
        connective: &'syntax StatementConnectiveSyntax,
    },
    TrailingSeparator {
        i: &'syntax Token,
        free_modifiers: Vec<&'syntax FreeModifierSyntax>,
    },
}

#[invariant(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
#[invariant(restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
#[derive(Debug, Clone)]
struct GeneratedImplicitExistential {
    variable: SemanticObjectId,
    source: Option<crate::model::SemanticSource>,
    restrictions: Vec<SemanticObjectId>,
}

#[invariant(variable.is_none_or(|variable| variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
#[invariant(!word.is_empty(), "prenex pro-sumti binding word must be present")]
#[invariant(scope_key.is_none_or(|(byte_start, byte_end)| byte_start <= byte_end))]
#[derive(Debug, Clone)]
struct GeneratedPrenexProSumtiBinding {
    variable: Option<SemanticObjectId>,
    word: String,
    source: Option<crate::model::SemanticSource>,
    scope_key: Option<(usize, usize)>,
}

#[invariant(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
#[invariant(!word.is_empty(), "prenex relation-variable binding word must be present")]
#[derive(Debug, Clone)]
struct GeneratedPrenexRelationVariableBinding {
    parameter: SemanticObjectId,
    word: String,
}

#[invariant(::ProSumti(word) => !word.is_empty())]
#[invariant(::RelationVariable(word) => !word.is_empty())]
#[derive(Debug, Clone)]
enum GeneratedPrenexPushedBinding {
    ProSumti(String),
    RelationVariable(String),
}

#[invariant(quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))]
#[invariant(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
#[invariant(quantity.is_some() || *operator == FormulaOperator::Exists, "bare prenex scopes are existential")]
#[derive(Debug, Clone)]
struct GeneratedPrenexQuantifierScope {
    operator: FormulaOperator,
    variable: SemanticObjectId,
    quantity: Option<SemanticObjectId>,
    source: Option<crate::model::SemanticSource>,
    relative_clause_restrictions: Vec<SemanticObjectId>,
}

#[invariant(::ProBridi(_) => true)]
#[invariant(::GohaWord(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedRelationQuestionSyntax<'syntax> {
    ProBridi(&'syntax ProBridiTanruUnitSyntax),
    GohaWord(&'syntax GohaWordTanruUnitSyntax),
}

#[invariant(::ProBridi(_) => true)]
#[invariant(::GohaWord(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedRelationParameterSyntax<'syntax> {
    ProBridi(&'syntax ProBridiTanruUnitSyntax),
    GohaWord(&'syntax GohaWordTanruUnitSyntax),
}

#[invariant(!introduced_by.is_empty())]
#[invariant(!relation.is_empty())]
#[invariant(*visible_place > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedModalStatementConnectionSpec {
    introduced_by: String,
    relation: String,
    visible_place: usize,
    argument_kind: GeneratedModalConnectionArgumentKind,
}

#[invariant(!source.is_empty(), "modal connection source must preserve connector text")]
#[invariant((operator == &FormulaOperator::RespectivelyDistribution && truth_table.is_none()) || (operator != &FormulaOperator::RespectivelyDistribution && truth_table.as_ref().is_some_and(|table| table.len() == 4)), "logical modal connections have either a four-row truth table or respectively distribution")]
#[invariant(terms.len() >= 2, "modal connections need at least two branches")]
#[derive(Debug, Clone)]
struct GeneratedLogicalModalConnectionSpec<'syntax> {
    operator: FormulaOperator,
    source: String,
    truth_table: Option<String>,
    terms: Vec<GeneratedConnectedModalTerm<'syntax>>,
}

#[invariant(generated_tense_modal_has_modal_argument(tense_modal))]
#[derive(Debug, Clone)]
struct GeneratedConnectedModalTerm<'syntax> {
    tense_modal: TenseModalSyntax,
    kind: GeneratedConnectedModalTermKind<'syntax>,
    negated: bool,
}

#[invariant(!source.is_empty())]
#[invariant(!locus.is_empty())]
#[invariant(branches.len() >= 2)]
#[invariant((operator == &FormulaOperator::RespectivelyDistribution && truth_table.is_none() && connector_question.is_none()) || (operator != &FormulaOperator::RespectivelyDistribution && truth_table.is_some() != connector_question.is_some()))]
#[derive(Debug, Clone)]
struct GeneratedLogicalTagConnection<'syntax> {
    operator: FormulaOperator,
    source: String,
    truth_table: Option<String>,
    connector_question: Option<Token>,
    locus: String,
    connected_index: usize,
    branches: Vec<GeneratedLogicalTagConnectionBranch<'syntax>>,
}

#[invariant(::Modal { term, .. } => generated_tense_modal_has_modal_argument(&term.tense_modal))]
#[invariant(::Event { branch, anchor } => generated_tense_modal_has_event_modifier(&branch.tense_modal) && anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[derive(Debug, Clone)]
enum GeneratedLogicalTagConnectionBranch<'syntax> {
    Modal {
        term: GeneratedConnectedModalTerm<'syntax>,
        argument: ArgumentValue,
    },
    Event {
        branch: GeneratedConnectedEventTenseBranch,
        anchor: Option<SemanticObjectId>,
    },
}

#[invariant(::Named { introduced_by, relation, visible_place } => !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0)]
#[invariant(::AdHoc { fiho } => fiho.fiho.value.is_cmavo(Cmavo::Fiho) && fiho.fehu.as_ref().is_none_or(|fehu| fehu.value.is_cmavo(Cmavo::Fehu)))]
#[derive(Debug, Clone)]
enum GeneratedConnectedModalTermKind<'syntax> {
    Named {
        introduced_by: String,
        relation: String,
        visible_place: usize,
    },
    AdHoc {
        fiho: &'syntax jbotci_syntax::generated_model::FihoTenseSyntax,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedModalConnectionArgumentKind {
    Eventuality,
    Formula,
}

#[invariant(crate::model::argument_object_kind_can_fill(value.object_kind()))]
#[derive(Debug, Clone)]
struct GeneratedScalarScaleDefinition {
    value: SemanticObjectId,
    introduced_by: String,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedDescriptionAbstraction<'syntax> {
    abstraction: &'syntax AbstractionTanruUnitSyntax,
    output_sort: SemanticSort,
    link_relation: &'static str,
}

#[invariant(nu.is_selmaho(Selmaho::Nu))]
#[invariant(nai.is_none_or(|nai| nai.is_cmavo(Cmavo::Nai)))]
#[invariant(kei.is_none_or(|kei| kei.is_cmavo(Cmavo::Kei)))]
#[derive(Debug, Clone, Copy)]
struct GeneratedAbstractionBranch<'syntax> {
    abstraction: &'syntax AbstractionTanruUnitSyntax,
    nu: &'syntax WithFreeModifiers<Token, FreeModifierSyntax>,
    nai: Option<&'syntax WithFreeModifiers<Token, FreeModifierSyntax>>,
    subbridi: &'syntax SubbridiSyntax,
    kei: Option<&'syntax WithFreeModifiers<Token, FreeModifierSyntax>>,
}

impl<'syntax> GeneratedAbstractionBranch<'syntax> {
    #[requires(true)]
    #[ensures(ret.nai.is_some() == abstraction.nai.is_some())]
    fn primary(abstraction: &'syntax AbstractionTanruUnitSyntax) -> Self {
        Self::from_data(data!(GeneratedAbstractionBranch {
            abstraction,
            nu: &abstraction.nu,
            nai: abstraction.nai.as_ref(),
            subbridi: abstraction.subbridi.as_ref(),
            kei: abstraction.kei.as_ref(),
        }))
    }

    #[requires(true)]
    #[ensures(ret.nai.is_some() == connection.nai.is_some())]
    fn connected(
        abstraction: &'syntax AbstractionTanruUnitSyntax,
        connection: &'syntax AbstractorConnectionSyntax,
    ) -> Self {
        Self::from_data(data!(GeneratedAbstractionBranch {
            abstraction,
            nu: &connection.nu,
            nai: connection.nai.as_ref(),
            subbridi: abstraction.subbridi.as_ref(),
            kei: abstraction.kei.as_ref(),
        }))
    }
}

#[invariant(::Normal(_) => true)]
#[invariant(::Cei(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedTanruAtomView<'syntax> {
    Normal(&'syntax TanruUnitAtomSyntax),
    Cei(&'syntax TanruUnitAtomForCeiSyntax),
}

impl<'syntax> GeneratedTanruAtomView<'syntax> {
    #[requires(true)]
    #[ensures(true)]
    fn normal(atom: &'syntax TanruUnitAtomSyntax) -> Self {
        Self::Normal(atom)
    }

    #[requires(true)]
    #[ensures(true)]
    fn cei(atom: &'syntax TanruUnitAtomForCeiSyntax) -> Self {
        Self::Cei(atom)
    }

    #[requires(true)]
    #[ensures(true)]
    fn conversions(self) -> &'syntax [WithFreeModifiers<Token, FreeModifierSyntax>] {
        match self {
            Self::Normal(atom) => &atom.conversions,
            Self::Cei(atom) => &atom.conversions,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn base(self) -> GeneratedTanruAtomBaseView<'syntax> {
        match self {
            Self::Normal(atom) => GeneratedTanruAtomBaseView::Normal(atom.base.as_ref()),
            Self::Cei(atom) => GeneratedTanruAtomBaseView::Cei(atom.base.as_ref()),
        }
    }
}

#[invariant(::Normal(_) => true)]
#[invariant(::Cei(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedTanruAtomBaseView<'syntax> {
    Normal(&'syntax TanruUnitAtomBaseSyntax),
    Cei(&'syntax TanruUnitAtomBaseForCeiSyntax),
}

impl<'syntax> GeneratedTanruAtomBaseView<'syntax> {
    #[requires(true)]
    #[ensures(true)]
    fn scalar_negated_base(self) -> Option<&'syntax ScalarNegatedTanruUnitSyntax> {
        match self {
            Self::Normal(TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit))
            | Self::Cei(TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit)) => Some(unit),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn grouped_base(self) -> Option<&'syntax GroupedTanruUnitSyntax> {
        match self {
            Self::Normal(TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped))
            | Self::Cei(TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(grouped)) => Some(grouped),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn sumti_selbri_base(self) -> Option<&'syntax SumtiSelbriTanruUnitSyntax> {
        match self {
            Self::Normal(TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(unit))
            | Self::Cei(TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(unit)) => Some(unit),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn preposed_linkargs_base(self) -> Option<(&'syntax LinkargsSyntax, &'syntax TanruUnitSyntax)> {
        match self {
            Self::Normal(TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit))
            | Self::Cei(TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit)) => {
                Some((&unit.linkargs, &unit.base))
            }
            _ => None,
        }
    }
}

#[invariant(!relation.is_empty(), "aggregate relation must be named")]
#[invariant(!member_word.is_empty(), "aggregate member gadri word must be named")]
#[derive(Debug, Clone, Copy)]
struct AggregateDescriptionSpec {
    sort: SemanticSort,
    relation: &'static str,
    member_cmavo: Cmavo,
    member_word: &'static str,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedTermAssignments<'syntax> {
    visible_arguments: BTreeMap<usize, ArgumentValue>,
    next_visible_place: usize,
    place_questions: Vec<GeneratedPlaceQuestionAssignment>,
    modal_terms: Vec<GeneratedModalTerm<'syntax>>,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    coequal_scope_groups: Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
    implicit_existentials: Vec<GeneratedImplicitExistential>,
    term_formula_scopes: Vec<GeneratedTermFormulaScope>,
}

#[invariant(tagged_sumti.is_none_or(|tagged_sumti| std::ptr::eq(tagged_sumti.tense_modal.as_ref(), *tense_modal)), "a prepared modal term keeps the tag and its optional sumti-bearing wrapper together")]
#[invariant(tagged_sumti.is_some() || argument.is_none(), "a bare tag cannot carry a prepared sumti argument")]
#[invariant(argument.as_ref().is_none_or(|argument| argument.value.is_some() || argument.kind == ArgumentValueKind::Deleted))]
#[derive(Debug, Clone)]
struct GeneratedModalTerm<'syntax> {
    tense_modal: &'syntax LeadingTermTagTenseModalSyntax,
    tagged_sumti: Option<&'syntax TaggedSumtiTermSyntax>,
    argument: Option<ArgumentValue>,
}

#[invariant(true)]
#[derive(Debug)]
struct GeneratedScopedFormula<'syntax> {
    formula: SemanticObjectId,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    coequal_scope_groups: Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
    implicit_existentials: Vec<GeneratedImplicitExistential>,
    term_formula_scopes: Vec<GeneratedTermFormulaScope>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedForethoughtPrefixContext<'syntax> {
    assignments: GeneratedTermAssignments<'syntax>,
    modal_arguments: Vec<ModalArgument>,
}

#[invariant(!introduced_by.is_empty())]
#[invariant(argument.value.is_some())]
#[derive(Debug, Clone)]
struct GeneratedPlaceQuestionAssignment {
    introduced_by: String,
    argument: ArgumentValue,
    parameter_source: Option<crate::model::SemanticSource>,
    binding_source: Option<crate::model::SemanticSource>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedArgumentQuantifierBundleScope<'syntax> {
    scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(true)]
#[derive(Debug)]
struct GeneratedLinkargsAssignments<'syntax> {
    visible_arguments: BTreeMap<usize, ArgumentValue>,
    modal_arguments: Vec<ModalArgument>,
    event_modifiers: Vec<GeneratedLinkedEventModifier<'syntax>>,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    next_visible_place: usize,
}

#[invariant(!visible_argument_branches.is_empty())]
#[invariant(visible_argument_branches.iter().all(|branch| branch.keys().all(|place| *place > 0)))]
#[derive(Debug)]
struct GeneratedLinkargsArgumentBranches<'syntax> {
    visible_argument_branches: Vec<BTreeMap<usize, ArgumentValue>>,
    modal_arguments: Vec<ModalArgument>,
    event_modifiers: Vec<GeneratedLinkedEventModifier<'syntax>>,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
}

#[invariant(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[derive(Debug, Clone)]
struct GeneratedLinkedEventModifier<'syntax> {
    tense_modal: &'syntax TenseModalSyntax,
    anchor: Option<SemanticObjectId>,
}

#[invariant(::Negation { .. } => true)]
#[derive(Debug, Clone)]
enum GeneratedTermFormulaScope {
    Negation {
        source: Option<crate::model::SemanticSource>,
    },
}

#[invariant(true)]
#[derive(Debug, Default)]
struct GeneratedRecurrenceEventModifiers {
    temporal_aspects: Vec<Aspect>,
    temporal_recurrences: Vec<Recurrence>,
    temporal_interval_modifiers: Vec<IntervalModifier>,
    spatial_aspects: Vec<Aspect>,
    spatial_recurrences: Vec<Recurrence>,
    spatial_interval_modifiers: Vec<IntervalModifier>,
}

#[invariant(::Object(quantity) => quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[invariant(::Value(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedRecurrenceQuantity {
    Object(SemanticObjectId),
    Value(QuantityValue),
}

type GeneratedRecurrenceQuantityCache =
    BTreeMap<GeneratedRecurrenceQuantityCacheKey, SemanticObjectId>;

#[invariant(!introduced_by.is_empty(), "recurrence quantity cache marker must be named")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GeneratedRecurrenceQuantityCacheKey {
    kind: RecurrenceKind,
    introduced_by: String,
    connection: Option<RecurrenceConnection>,
    value: GeneratedRecurrenceQuantityCacheValue,
    negation: Option<ModalNegation>,
}

impl GeneratedRecurrenceQuantityCacheKey {
    #[requires(recurrence.value.is_some())]
    #[ensures(!ret.introduced_by.is_empty())]
    fn from_recurrence(recurrence: &Recurrence) -> Self {
        let value = recurrence
            .value
            .as_ref()
            .expect("checked by recurrence cache-key precondition");
        Self::new(
            recurrence.kind,
            recurrence.introduced_by.clone(),
            recurrence.connection.clone(),
            GeneratedRecurrenceQuantityCacheValue::from_quantity_value(value),
            recurrence.negation.clone(),
        )
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.introduced_by == old(introduced_by.clone()))]
    fn new(
        kind: RecurrenceKind,
        introduced_by: String,
        connection: Option<RecurrenceConnection>,
        value: GeneratedRecurrenceQuantityCacheValue,
        negation: Option<ModalNegation>,
    ) -> Self {
        Self::from_data(data!(GeneratedRecurrenceQuantityCacheKey {
            kind,
            introduced_by,
            connection,
            value,
            negation,
        }))
    }
}

#[invariant(::Integer(_) => true)]
#[invariant(::ParsedInteger { text, .. } => !text.is_empty(), "parsed recurrence integers keep their source text in the cache key")]
#[invariant(::Text(text) => !text.is_empty())]
#[invariant(::MathExpression(math_expression) => math_expression.object_kind() == crate::model::SemanticObjectKind::MathExpression)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GeneratedRecurrenceQuantityCacheValue {
    Integer(i64),
    ParsedInteger { text: String, integer: i64 },
    Text(String),
    MathExpression(SemanticObjectId),
}

impl GeneratedRecurrenceQuantityCacheValue {
    #[requires(true)]
    #[ensures(true)]
    fn from_quantity_value(value: &QuantityValue) -> Self {
        if let Some(integer) = value.integer {
            return new!(GeneratedRecurrenceQuantityCacheValue::Integer(integer));
        }
        if let Some(text) = &value.text {
            return new!(GeneratedRecurrenceQuantityCacheValue::Text(text.clone()));
        }
        let math_expression = value
            .math_expression
            .expect("quantity values always carry exactly one payload");
        new!(GeneratedRecurrenceQuantityCacheValue::MathExpression(
            math_expression
        ))
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn parsed_integer(text: String, integer: i64) -> Self {
        Self::from_data(data!(
            GeneratedRecurrenceQuantityCacheValue::ParsedInteger { text, integer }
        ))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Default)]
struct GeneratedStickyEventUpdate {
    reset: bool,
    time_path: Option<Vec<TemporalPathStep>>,
    space_path: Option<Vec<TemporalPathStep>>,
}

#[invariant(::TenseModal(_) => true)]
#[invariant(::LeadingTermTag(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedEventTenseModal {
    TenseModal(TenseModalSyntax),
    LeadingTermTag(LeadingTermTagTenseModalSyntax),
}

#[invariant(*order < usize::MAX)]
#[invariant(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[derive(Debug, Clone)]
struct GeneratedEventTenseModifier {
    order: usize,
    tense_modal: GeneratedEventTenseModal,
    anchor: Option<SemanticObjectId>,
    magnitude: Option<AnchorMagnitude>,
}

#[invariant(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[derive(Debug, Clone)]
struct GeneratedGovernedTermset {
    termset_index: usize,
    anchor: Option<SemanticObjectId>,
    magnitude: Option<AnchorMagnitude>,
}

#[invariant(branches.len() >= 2)]
#[invariant(!source.is_empty())]
#[invariant((operator == &FormulaOperator::RespectivelyDistribution && truth_table.is_none() && connector_question.is_none()) || (operator != &FormulaOperator::RespectivelyDistribution && truth_table.is_some() != connector_question.is_some()))]
#[derive(Debug, Clone)]
struct GeneratedConnectedEventTenseSpec {
    operator: FormulaOperator,
    source: String,
    truth_table: Option<String>,
    connector_question: Option<Token>,
    branches: Vec<GeneratedConnectedEventTenseBranch>,
}

#[invariant(generated_tense_modal_has_event_modifier(tense_modal))]
#[derive(Debug, Clone)]
struct GeneratedConnectedEventTenseBranch {
    tense_modal: TenseModalSyntax,
    negated: bool,
}

#[invariant(::Argument(_) => true)]
#[invariant(::Bundle(_) => true)]
#[invariant(::ImplicitExistential(_) => true)]
#[invariant(::Term(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedOrderedFormulaScope<'syntax> {
    Argument(GeneratedArgumentQuantifierScope<'syntax>),
    Bundle(GeneratedArgumentQuantifierBundleScope<'syntax>),
    ImplicitExistential(GeneratedImplicitExistential),
    Term(GeneratedTermFormulaScope),
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedPreparedArgumentFormulaScope<'syntax> {
    scope: GeneratedArgumentQuantifierScope<'syntax>,
    restriction: Option<SemanticObjectId>,
    quantity: GeneratedPreparedArgumentQuantity,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedPreparedArgumentQuantifierBundleScope<'syntax> {
    scopes: Vec<GeneratedPreparedArgumentFormulaScope<'syntax>>,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[invariant(right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[derive(Debug, Clone)]
struct GeneratedConnectedQuantifierQuantityScope {
    left_quantity: SemanticObjectId,
    right_quantity: SemanticObjectId,
    left_negated: bool,
    right_negated: bool,
    operator: FormulaOperator,
    connector: Connector,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(connector.locus == "mekso-operator")]
#[invariant(!connector.source.is_empty())]
#[invariant(connector.truth_table.as_ref().is_none_or(|table| table.len() == 4))]
#[derive(Debug, Clone)]
struct GeneratedConnectedMeksoOperatorExpansion {
    left_operator: MeksoOperatorSyntax,
    right_operator: MeksoOperatorSyntax,
    operator: FormulaOperator,
    connector: Connector,
}

#[invariant(::Single(quantity) => quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[invariant(::Connected(connection) => connection.left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity && connection.right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
#[derive(Debug, Clone)]
enum GeneratedPreparedArgumentQuantity {
    Single(SemanticObjectId),
    Connected(GeneratedConnectedQuantifierQuantityScope),
}

#[invariant(::Argument(_) => true)]
#[invariant(::Bundle(_) => true)]
#[invariant(::ImplicitExistential(_) => true)]
#[invariant(::Term(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedPreparedOrderedFormulaScope<'syntax> {
    Argument(GeneratedPreparedArgumentFormulaScope<'syntax>),
    Bundle(GeneratedPreparedArgumentQuantifierBundleScope<'syntax>),
    ImplicitExistential(GeneratedImplicitExistential),
    Term(GeneratedTermFormulaScope),
}

#[invariant(::ImplicitExistential(_) => true)]
#[invariant(::Term(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedBridiFormulaScope {
    ImplicitExistential(GeneratedImplicitExistential),
    Term(GeneratedTermFormulaScope),
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedAlternativeArgument<'syntax> {
    argument: ArgumentValue,
    negated: bool,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
}

#[invariant(::Built(_) => true)]
#[invariant(::Sumti { .. } => true)]
#[invariant(::SumtiForethought { .. } => true)]
#[invariant(::SumtiBound { .. } => true)]
#[derive(Debug, Clone)]
enum GeneratedAlternativeArgumentSource<'syntax> {
    Built(GeneratedAlternativeArgument<'syntax>),
    Sumti {
        sumti: &'syntax SumtiSyntax,
        negated: bool,
    },
    SumtiForethought {
        sumti: &'syntax SumtiForethoughtSyntax,
        negated: bool,
    },
    SumtiBound {
        sumti: &'syntax SumtiBoundSyntax,
        negated: bool,
    },
}

impl<'syntax> From<GeneratedAlternativeArgument<'syntax>>
    for GeneratedAlternativeArgumentSource<'syntax>
{
    #[requires(true)]
    #[ensures(matches!(ret, GeneratedAlternativeArgumentSource::Built(_)))]
    fn from(argument: GeneratedAlternativeArgument<'syntax>) -> Self {
        Self::Built(argument)
    }
}

#[invariant(::Argument { .. } => true)]
#[invariant(::Forethought { .. } => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedDistributedSumtiConnective<'syntax> {
    Argument {
        connective: &'syntax ArgumentConnectiveSyntax,
        tense_modal: Option<&'syntax TenseModalSyntax>,
        bo: bool,
    },
    Forethought {
        gek: &'syntax jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
        gik: &'syntax GikConnectiveSyntax,
    },
}

#[invariant(::Connected(_) => true)]
#[invariant(::Bound(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedDirectTermConnective<'syntax> {
    Connected(&'syntax jbotci_syntax::generated_model::ConnectedTermConnectiveSyntax),
    Bound(&'syntax jbotci_syntax::generated_model::BoundTermConnectiveSyntax),
}

#[invariant(*continuation_count <= sumti.continuations.len())]
#[derive(Debug, Clone, Copy)]
struct GeneratedSumtiAfterthoughtPrefix<'syntax> {
    sumti: &'syntax SumtiAfterthoughtSyntax,
    continuation_count: usize,
}

#[invariant(::Sumti(_) => true)]
#[invariant(::SumtiGrouped(_) => true)]
#[invariant(::SumtiAfterthought(_) => true)]
#[invariant(::SumtiBound(_) => true)]
#[invariant(::SumtiForethought(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedDistributedSumtiBranch<'syntax> {
    Sumti(&'syntax SumtiSyntax),
    SumtiGrouped(&'syntax SumtiGroupedSyntax),
    SumtiAfterthought(GeneratedSumtiAfterthoughtPrefix<'syntax>),
    SumtiBound(&'syntax SumtiBoundSyntax),
    SumtiForethought(&'syntax SumtiForethoughtSyntax),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedLogicalSumtiConnection<'syntax> {
    leading: GeneratedDistributedSumtiBranch<'syntax>,
    connective: GeneratedDistributedSumtiConnective<'syntax>,
    trailing: GeneratedDistributedSumtiBranch<'syntax>,
    relative_clauses: Option<&'syntax RelativeClauseListSyntax>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedDaSeriesScopeBinding<'syntax> {
    variable: SemanticObjectId,
    restriction_nodes: Vec<GeneratedArgumentQuantifierScopeNode<'syntax>>,
    restriction_formulas: Vec<SemanticObjectId>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedSemanticDaSeriesScopeBinding {
    variable: SemanticObjectId,
    restriction_formulas: Vec<SemanticObjectId>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct GeneratedArgumentQuantifierScope<'syntax> {
    node: GeneratedArgumentQuantifierScopeNode<'syntax>,
    source: GeneratedArgumentQuantifierSource<'syntax>,
    variable: SemanticObjectId,
    source_variable: Option<SemanticObjectId>,
    selection_source: Option<SelectionSource>,
    source_restriction_nodes: Vec<GeneratedArgumentQuantifierScopeNode<'syntax>>,
    source_restriction_formulas: Vec<SemanticObjectId>,
    inherited_restrictions: Vec<SemanticObjectId>,
    relative_clause_restrictions: Vec<SemanticObjectId>,
}

#[invariant(crate::model::argument_object_kind_can_fill(object.object_kind()))]
#[invariant(formula_scopes.iter().all(|scope| scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
#[invariant(formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
#[derive(Debug, Clone)]
struct GeneratedVocativeTarget<'syntax> {
    object: SemanticObjectId,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    formula: Option<SemanticObjectId>,
    audience_is_target: bool,
}

#[invariant(::Sumti(_) => true)]
#[invariant(::SumtiBound(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedArgumentQuantifierScopeNode<'syntax> {
    Sumti(&'syntax SumtiSyntax),
    SumtiBound(&'syntax SumtiBoundSyntax),
}

#[invariant(::QuantifiedSumti(_) => true)]
#[invariant(::OuterQuantifiedDescription(_) => true)]
#[invariant(::NoGadriDescription(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedArgumentQuantifierSource<'syntax> {
    QuantifiedSumti(&'syntax QuantifiedSumtiSyntax),
    OuterQuantifiedDescription(&'syntax DescriptorWithOuterQuantifierSumtiSyntax),
    NoGadriDescription(&'syntax DescriptorWithoutGadriSumtiSyntax),
}

#[invariant(::Negation { .. } => true)]
#[invariant(::Quantifier(_) => true)]
#[derive(Debug, Clone)]
enum GeneratedPrenexFormulaScope {
    Negation {
        source: Option<crate::model::SemanticSource>,
    },
    Quantifier(GeneratedPrenexQuantifierScope),
}

#[invariant(speaker.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(audience.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(now.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[invariant(here.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[derive(Debug, Clone, Copy)]
struct GeneratedDeicticRoles {
    speaker: SemanticObjectId,
    audience: SemanticObjectId,
    now: SemanticObjectId,
    here: SemanticObjectId,
}

#[invariant(::Description => true)]
#[invariant(::PropertyAbstraction => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedPropertyTanruContext {
    Description,
    PropertyAbstraction,
}

#[invariant(::Absent => true)]
#[invariant(::Fresh => true)]
#[invariant(::Existing(eventuality) => eventuality.object_kind() == crate::model::SemanticObjectKind::Referent && eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[derive(Debug, Clone, Copy)]
enum GeneratedPredicationEventuality {
    Absent,
    Fresh,
    Existing(SemanticObjectId),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedAnchorDomain {
    Time,
    Space,
}

impl GeneratedPropertyTanruContext {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn connector_locus(self) -> &'static str {
        match self {
            Self::Description => "description",
            Self::PropertyAbstraction => "property-abstraction",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn tertau_source(
        self,
        builder: &GeneratedGraphBuilder<'_, '_, '_>,
        tanru: &TanruSelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Option<crate::model::SemanticSource> {
        match self {
            Self::Description => builder.source_for_node(tanru, "restrictive-predication"),
            Self::PropertyAbstraction => source,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn predication_eventuality(
        self,
        eventuality: Option<SemanticObjectId>,
    ) -> GeneratedPredicationEventuality {
        match (self, eventuality) {
            (Self::Description, _) => GeneratedPredicationEventuality::from_data(data!(
                GeneratedPredicationEventuality::Absent
            )),
            (Self::PropertyAbstraction, Some(eventuality)) => {
                GeneratedPredicationEventuality::from_data(data!(
                    GeneratedPredicationEventuality::Existing(eventuality)
                ))
            }
            (Self::PropertyAbstraction, None) => GeneratedPredicationEventuality::from_data(data!(
                GeneratedPredicationEventuality::Fresh
            )),
        }
    }
}

impl GeneratedPredicationEventuality {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    fn resolve(
        self,
        builder: &mut GeneratedGraphBuilder<'_, '_, '_>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match self.as_data() {
            data!(GeneratedPredicationEventuality::Absent) => Ok(None),
            data!(GeneratedPredicationEventuality::Existing(eventuality)) => Ok(Some(*eventuality)),
            data!(GeneratedPredicationEventuality::Fresh) => builder
                .build_generated_predication_eventuality(source)
                .map(Some),
        }
    }
}

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(true)]
    #[ensures(ret.next_index == 5)]
    fn new(options: SemanticBuildOptions<'a>, dictionary: &'dict Dictionary<'dict>) -> Self {
        let mut builder = Self {
            options,
            dictionary,
            objects: BTreeMap::new(),
            next_index: 5,
            relative_head_stack: Vec::new(),
            current_utterance: None,
            previous_utterance: None,
            next_utterance: None,
            current_speaker: SemanticObjectId::speaker(),
            current_audience: SemanticObjectId::addressee(),
            current_now: SemanticObjectId::now(),
            current_here: SemanticObjectId::here(),
            content_eventualities: BTreeMap::new(),
            scoped_argument_variables: BTreeMap::new(),
            direct_question_slots: Vec::new(),
            relation_variable_parameters: BTreeMap::new(),
            implicit_existential_variables: Vec::new(),
            recorded_implicit_existential_variables: HashSet::new(),
            implicit_da_series_bindings: BTreeMap::new(),
            quantified_da_series_bindings: BTreeMap::new(),
            sticky_modal_arguments: BTreeMap::new(),
            host_event_modal_elisions: BTreeMap::new(),
            sticky_time_path: Vec::new(),
            sticky_space_path: Vec::new(),
            story_time_anchor: None,
            pending_event_modifiers: BTreeMap::new(),
            deferred_event_modifier_flush_depth: 0,
            prenex_pro_sumti_bindings: BTreeMap::new(),
            prenex_relation_variable_bindings: BTreeMap::new(),
            abstraction_parameter_stack: Vec::new(),
            indirect_question_stack: Vec::new(),
            temporal_context_stack: Vec::new(),
            pro_bridi_scope_stack: Vec::new(),
            completed_pro_bridi_frames: Vec::new(),
            current_quote_depth: 0,
            sumti_referents: BTreeMap::new(),
            sumti_referent_cache_bypass_depth: 0,
            letter_sumti_referents: BTreeMap::new(),
            pending_sumti_candidates: Vec::new(),
            recent_sumti_referents: Vec::new(),
            assigned_referents: BTreeMap::new(),
            math_variable_referents: BTreeMap::new(),
            assigned_pro_bridi_bindings: BTreeMap::new(),
            pending_asides: Vec::new(),
            defer_active_prenex_implicit_existentials: 0,
            deferred_active_prenex_implicit_existentials: Vec::new(),
            pending_negated_selbri_argument_scope_reservations: 0,
            suppress_prenex_bound_implicit_existential_recording: 0,
            pending_after_eventuality_reservations: 0,
        };
        builder.insert_deictics();
        builder
    }

    #[requires(true)]
    #[ensures(self.objects.contains_key(&SemanticObjectId::speaker()))]
    #[ensures(self.objects.contains_key(&SemanticObjectId::addressee()))]
    #[ensures(self.objects.contains_key(&SemanticObjectId::now()))]
    #[ensures(self.objects.contains_key(&SemanticObjectId::here()))]
    fn insert_deictics(&mut self) {
        self.objects.insert(
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
        self.objects.insert(
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
        self.objects.insert(
            SemanticObjectId::now(),
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::eventuality(),
                Some(IndexicalKind::Now),
                None,
                None,
                None,
                Vec::new(),
            ),
        );
        self.objects.insert(
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

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn current_speaker(&self) -> SemanticObjectId {
        self.current_speaker
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn current_audience(&self) -> SemanticObjectId {
        self.current_audience
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    fn current_now(&self) -> SemanticObjectId {
        self.current_now
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn current_here(&self) -> SemanticObjectId {
        self.current_here
    }

    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(anchor.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
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
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent))]
    fn current_temporal_context(&self) -> Option<SemanticObjectId> {
        self.temporal_context_stack.last().copied()
    }

    #[requires(true)]
    #[ensures(ret.speaker == self.current_speaker)]
    #[ensures(ret.audience == self.current_audience)]
    #[ensures(ret.now == self.current_now)]
    #[ensures(ret.here == self.current_here)]
    fn current_deictic_roles(&self) -> GeneratedDeicticRoles {
        GeneratedDeicticRoles::from_data(data!(GeneratedDeicticRoles {
            speaker: self.current_speaker,
            audience: self.current_audience,
            now: self.current_now,
            here: self.current_here,
        }))
    }

    #[requires(roles.speaker.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(roles.audience.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(roles.now.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(roles.here.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(self.current_speaker == roles.speaker)]
    #[ensures(self.current_audience == roles.audience)]
    #[ensures(self.current_now == roles.now)]
    #[ensures(self.current_here == roles.here)]
    fn set_current_deictic_roles(&mut self, roles: GeneratedDeicticRoles) {
        self.current_speaker = roles.speaker;
        self.current_audience = roles.audience;
        self.current_now = roles.now;
        self.current_here = roles.here;
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|roles| roles.speaker.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_fresh_quote_deictic_roles(
        &mut self,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedDeicticRoles, SemanticsError> {
        let speaker = self.next_referent_id();
        self.insert(
            speaker,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Speaker),
                None,
                None,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let audience = self.next_referent_id();
        self.insert(
            audience,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Audience),
                None,
                None,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let now = self.next_referent_with_sort_id(SemanticSort::eventuality());
        self.insert(
            now,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::eventuality(),
                Some(IndexicalKind::Now),
                None,
                None,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let here = self.next_referent_id();
        self.insert(
            here,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(IndexicalKind::Here),
                None,
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(GeneratedDeicticRoles::from_data(data!(
            GeneratedDeicticRoles {
                speaker,
                audience,
                now,
                here,
            }
        )))
    }

    #[requires(child.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_generated_tense_scope_formula<'syntax: 'tree>(
        &mut self,
        child: SemanticObjectId,
        tense_modal: &'syntax TenseModalSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.build_generated_tense_eventuality(tense_modal, source.clone())?;
        let formula = self.next_formula_id();
        let mut object = SemanticObject::connective_formula(
            FormulaOperator::Scoped,
            vec![child],
            None,
            source,
            Vec::new(),
        );
        object.set_scoped_formula_eventuality(eventuality);
        self.insert(formula, object)?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.as_ref().is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))) || ret.is_err())]
    fn build_generated_tense_eventuality<'syntax: 'tree>(
        &mut self,
        tense_modal: &'syntax TenseModalSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if !generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        let eventuality = self.build_generated_predication_eventuality(source)?;
        self.apply_generated_tense_modal_event_modifier_to_eventuality(
            eventuality,
            tense_modal,
            None,
        )?;
        Ok(Some(eventuality))
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.as_ref().is_none_or(|eventuality| generated_semantic_object_is_eventuality(eventuality))) || ret.is_err())]
    fn take_deferred_generated_eventuality_template(
        &mut self,
        eventuality: Option<SemanticObjectId>,
    ) -> Result<Option<SemanticObject>, SemanticsError> {
        let Some(eventuality) = eventuality else {
            return Ok(None);
        };
        self.flush_generated_event_modifiers(eventuality)?;
        let Some(object) = self.objects.get(&eventuality).cloned() else {
            return Err(invalid_graph(format!(
                "missing generated eventuality template: {eventuality}"
            )));
        };
        if object.object_kind() != crate::model::SemanticObjectKind::Referent
            || !object
                .sort()
                .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
        {
            return Err(invalid_graph(format!(
                "cannot defer non-eventuality object: {eventuality}"
            )));
        }
        if eventuality.index() + 1 == self.next_index {
            self.objects.remove(&eventuality);
            self.next_index = eventuality.index();
        }
        Ok(Some(object))
    }

    #[requires(template.is_none_or(generated_semantic_object_is_eventuality))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    fn build_generated_branch_eventuality_from_template(
        &mut self,
        template: Option<&SemanticObject>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut object = template.cloned().unwrap_or_else(|| {
            SemanticObject::generated_eventuality(EventualityClass::Event, None, None)
        });
        if object.object_kind() != crate::model::SemanticObjectKind::Referent
            || !object
                .sort()
                .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
        {
            return Err(invalid_graph(
                "cannot build branch event from non-eventuality template".to_owned(),
            ));
        }
        object.replace_source(source);
        let sort = object.sort().unwrap_or_else(SemanticSort::eventuality);
        let eventuality = self.next_referent_with_sort_id(sort);
        self.insert(eventuality, object)?;
        Ok(eventuality)
    }

    #[requires(true)]
    #[ensures(ret.anchor.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn with_default_anchor_for_generated_tense(
        &self,
        domain: GeneratedAnchorDomain,
        relation: AnchorRelation,
    ) -> AnchorRelation {
        let default_anchor = match domain {
            GeneratedAnchorDomain::Time => self
                .current_temporal_context()
                .unwrap_or_else(|| self.current_now()),
            GeneratedAnchorDomain::Space => self.current_here(),
        };
        relation.with_data(data! { anchor: default_anchor })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    fn build_generated_predication_eventuality(
        &mut self,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality_id();
        let mut object =
            SemanticObject::generated_eventuality(EventualityClass::Event, None, source);
        self.apply_generated_inherited_sticky_paths_to_event(&mut object);
        self.insert(eventuality, object)?;
        if self.pending_after_eventuality_reservations > 0 {
            self.reserve_generated_semantic_id();
            self.pending_after_eventuality_reservations -= 1;
        }
        Ok(eventuality)
    }

    #[requires(generated_semantic_object_is_eventuality(event))]
    #[ensures(true)]
    fn apply_generated_inherited_sticky_paths_to_event(&self, event: &mut SemanticObject) {
        if !self.sticky_time_path.is_empty()
            && !(self.options.story_time && self.story_time_anchor.is_some())
        {
            let time_path = generated_inherited_temporal_path(&self.sticky_time_path);
            event.update_eventuality(|event| {
                event.with_data(data! {
                    time_path: time_path,
                })
            });
            normalize_generated_event_time_path(event);
        }
        if !self.sticky_space_path.is_empty() {
            let space_path = generated_inherited_temporal_path(&self.sticky_space_path);
            event.update_eventuality(|event| {
                event.with_data(data! {
                    space_path: space_path,
                })
            });
            normalize_generated_event_space_path(event);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn apply_generated_sticky_event_update(&mut self, update: GeneratedStickyEventUpdate) {
        if update.reset {
            self.sticky_time_path.clear();
            self.sticky_space_path.clear();
            self.story_time_anchor = None;
        }
        if let Some(time_path) = update.time_path {
            self.sticky_time_path = time_path;
        }
        if let Some(space_path) = update.space_path {
            self.sticky_space_path = space_path;
        }
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(true)]
    fn record_generated_tense_modal_event_modifier(
        &mut self,
        eventuality: SemanticObjectId,
        tense_modal: &TenseModalSyntax,
        anchor: Option<SemanticObjectId>,
    ) -> Result<bool, SemanticsError> {
        if !generated_tense_modal_has_event_modifier(tense_modal)
            && !generated_tense_modal_makes_tense_sticky(tense_modal)
            && !generated_tense_modal_makes_space_sticky(tense_modal)
            && !generated_tense_modal_resets_sticky_tense(tense_modal)
        {
            return Ok(false);
        }
        let order = self.source_order_for_node(tense_modal);
        self.pending_event_modifiers
            .entry(eventuality)
            .or_default()
            .push(GeneratedEventTenseModifier::from_data(data!(
                GeneratedEventTenseModifier {
                    order,
                    tense_modal: GeneratedEventTenseModal::TenseModal(tense_modal.clone()),
                    anchor,
                    magnitude: None,
                }
            )));
        Ok(true)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(true)]
    fn record_generated_leading_term_tag_event_modifier(
        &mut self,
        eventuality: SemanticObjectId,
        tense_modal: &LeadingTermTagTenseModalSyntax,
        anchor: Option<SemanticObjectId>,
    ) -> Result<bool, SemanticsError> {
        self.record_generated_leading_term_tag_event_modifier_with_magnitude(
            eventuality,
            tense_modal,
            anchor,
            None,
        )
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(true)]
    fn record_generated_leading_term_tag_event_modifier_with_magnitude(
        &mut self,
        eventuality: SemanticObjectId,
        tense_modal: &LeadingTermTagTenseModalSyntax,
        anchor: Option<SemanticObjectId>,
        magnitude: Option<AnchorMagnitude>,
    ) -> Result<bool, SemanticsError> {
        if !generated_tense_modal_has_event_modifier(tense_modal)
            && !generated_tense_modal_makes_tense_sticky(tense_modal)
            && !generated_tense_modal_makes_space_sticky(tense_modal)
            && !generated_tense_modal_resets_sticky_tense(tense_modal)
        {
            return Ok(false);
        }
        let order = self.source_order_for_node(tense_modal);
        self.pending_event_modifiers
            .entry(eventuality)
            .or_default()
            .push(GeneratedEventTenseModifier::from_data(data!(
                GeneratedEventTenseModifier {
                    order,
                    tense_modal: GeneratedEventTenseModal::LeadingTermTag(tense_modal.clone()),
                    anchor,
                    magnitude,
                }
            )));
        Ok(true)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    fn flush_generated_event_modifiers(
        &mut self,
        eventuality: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        let Some(mut modifiers) = self.pending_event_modifiers.remove(&eventuality) else {
            return Ok(());
        };
        modifiers.sort_by_key(|modifier| modifier.order);
        for modifier in modifiers {
            let data!(GeneratedEventTenseModifier {
                tense_modal,
                anchor,
                magnitude,
                ..
            }) = modifier.into_data();
            match tense_modal {
                GeneratedEventTenseModal::TenseModal(tense_modal) => {
                    self.apply_generated_tense_modal_event_modifier_to_eventuality_now(
                        eventuality,
                        &tense_modal,
                        anchor,
                    )?;
                    if let Some(magnitude) = magnitude {
                        let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                            invalid_graph(format!("missing generated eventuality {eventuality}"))
                        })?;
                        attach_generated_magnitude_to_event_modifier(
                            event,
                            &tense_modal,
                            magnitude,
                        );
                    }
                }
                GeneratedEventTenseModal::LeadingTermTag(tense_modal) => {
                    self.apply_generated_tense_modal_event_modifier_to_eventuality_now(
                        eventuality,
                        &tense_modal,
                        anchor,
                    )?;
                    if let Some(magnitude) = magnitude {
                        let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                            invalid_graph(format!("missing generated eventuality {eventuality}"))
                        })?;
                        attach_generated_magnitude_to_event_modifier(
                            event,
                            &tense_modal,
                            magnitude,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    fn flush_generated_event_modifiers_with_recurrence_quantity_promotion(
        &mut self,
        eventuality: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        self.flush_generated_event_modifiers(eventuality)?;
        self.promote_generated_eventuality_recurrence_quantities(eventuality)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    fn promote_generated_eventuality_recurrence_quantities(
        &mut self,
        eventuality: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        let Some(object) = self.objects.remove(&eventuality) else {
            return Err(invalid_graph(format!(
                "missing generated eventuality {eventuality}"
            )));
        };
        let mut event = match object.into_data() {
            data!(SemanticObject::Eventuality(event)) => event.into_data(),
            data => {
                self.objects
                    .insert(eventuality, SemanticObject::from_data(data));
                return Err(invalid_graph(format!(
                    "cannot promote recurrence quantities on non-eventuality {eventuality}"
                )));
            }
        };
        let mut quantity_cache = BTreeMap::new();
        self.promote_generated_recurrence_quantities(&mut event.recurrence, &mut quantity_cache)?;
        self.promote_generated_recurrence_quantities(
            &mut event.spatial_recurrence,
            &mut quantity_cache,
        )?;
        self.promote_generated_interval_modifier_quantities(
            &mut event.interval_modifiers,
            &mut quantity_cache,
        )?;
        self.promote_generated_interval_modifier_quantities(
            &mut event.spatial_interval_modifiers,
            &mut quantity_cache,
        )?;
        self.objects.insert(
            eventuality,
            new!(SemanticObject::Eventuality(EventualityNode::from_data(
                event
            ))),
        );
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn promote_generated_interval_modifier_quantities(
        &mut self,
        modifiers: &mut [IntervalModifier],
        quantity_cache: &mut GeneratedRecurrenceQuantityCache,
    ) -> Result<(), SemanticsError> {
        for modifier in modifiers {
            if let data!(IntervalModifier::Recurrence(recurrence)) = modifier.as_data() {
                let mut recurrence = recurrence.clone();
                self.promote_generated_recurrence_quantity(&mut recurrence, quantity_cache)?;
                *modifier = new!(IntervalModifier::Recurrence(recurrence));
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn promote_generated_recurrence_quantities(
        &mut self,
        recurrences: &mut [Recurrence],
        quantity_cache: &mut GeneratedRecurrenceQuantityCache,
    ) -> Result<(), SemanticsError> {
        for recurrence in recurrences {
            self.promote_generated_recurrence_quantity(recurrence, quantity_cache)?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn promote_generated_recurrence_quantity(
        &mut self,
        recurrence: &mut Recurrence,
        quantity_cache: &mut GeneratedRecurrenceQuantityCache,
    ) -> Result<(), SemanticsError> {
        if recurrence.quantity.is_some() || recurrence.value.is_none() {
            return Ok(());
        }
        let key = GeneratedRecurrenceQuantityCacheKey::from_recurrence(recurrence);
        if let Some(quantity) = quantity_cache.get(&key).copied() {
            *recurrence = recurrence.clone().with_data(data! {
                quantity: Some(quantity),
                value: None,
            });
            return Ok(());
        }
        let value = recurrence
            .value
            .clone()
            .expect("checked above that recurrence has a value");
        let quantity = self.build_recurrence_quantity_for_generated_value(value)?;
        *recurrence = recurrence.clone().with_data(data! {
            quantity: Some(quantity),
            value: None,
        });
        quantity_cache.insert(key, quantity);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    fn build_eventuality(
        &mut self,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_predication_eventuality(source)
    }

    #[requires(id.object_kind() == object.object_kind())]
    #[ensures(true)]
    fn insert(
        &mut self,
        id: SemanticObjectId,
        mut object: SemanticObject,
    ) -> Result<(), SemanticsError> {
        if let Some(predication) = object.as_predication() {
            let eventuality = predication.eventuality;
            let mode = predication.mode;
            if let Some(eventuality) = eventuality {
                object.update_predication(|predication| {
                    let mut data = predication.into_data();
                    for modal_argument in &mut data.modal_arguments {
                        self.bind_generated_modal_argument_to_host_event(
                            modal_argument,
                            eventuality,
                        );
                    }
                    PredicationNode::from_data(data)
                });
                self.finalize_generated_eventuality_for_predication_mode(eventuality, Some(mode))?;
            }
        }
        if self.objects.insert(id, object).is_some() {
            return Err(SemanticsError {
                kind: SemanticsErrorKind::DuplicateObject,
                message: format!("semantic builder attempted to insert duplicate object ID {id}"),
            });
        }
        Ok(())
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    fn finalize_generated_eventuality_for_predication_mode(
        &mut self,
        eventuality: SemanticObjectId,
        mode: Option<PredicationMode>,
    ) -> Result<(), SemanticsError> {
        let story_time = self.options.story_time;
        let story_anchor = self.story_time_anchor;
        let mut advance_story_time = false;
        {
            let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                invalid_graph(format!("missing generated eventuality {eventuality}"))
            })?;
            if mode != Some(PredicationMode::Asserted) {
                clear_generated_inherited_event_time_path(event);
                clear_generated_inherited_event_space_path(event);
                return Ok(());
            }
            let explicit_temporal = generated_event_has_explicit_temporal_marker(event);
            let sticky_temporal = generated_event_has_explicit_sticky_temporal_marker(event);
            if story_time {
                if let Some(anchor) = story_anchor
                    && !explicit_temporal
                {
                    clear_generated_event_time_path(event);
                    let time = new!(AnchorRelation {
                        relation: "after".to_owned(),
                        anchor,
                        sticky: false,
                        inherited: None,
                        distance: None,
                        magnitude: None,
                        scalar_negation: None,
                        motion: None,
                    });
                    event.update_eventuality(|node| node.with_data(data! { time: Some(time) }));
                }
                advance_story_time =
                    !explicit_temporal || sticky_temporal || story_anchor.is_none();
            }
        }
        if advance_story_time {
            self.story_time_anchor = Some(eventuality);
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_semantic_object_source(
        &mut self,
        id: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get_mut(&id)
            .ok_or_else(|| invalid_graph(format!("missing generated semantic object {id}")))?;
        object.replace_source(source);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    fn next_utterance_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::utterance(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(self.next_index == old(self.next_index) + 1)]
    fn reserve_generated_semantic_id(&mut self) {
        self.next_index += 1;
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    fn next_sequence_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::sequence(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    fn next_eventuality_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent_with_sort(SemanticSort::eventuality(), self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn next_locution_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent_with_sort(
            SemanticSort::Eventuality(EventualitySort::Locution),
            self.next_index,
        );
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn next_referent_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(sort))]
    fn next_referent_with_sort_id(&mut self, sort: SemanticSort) -> SemanticObjectId {
        let id = SemanticObjectId::referent_with_sort(sort, self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Predication)]
    fn next_predication_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::predication(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Formula)]
    fn next_formula_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::formula(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    fn next_parameter_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::parameter(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Question)]
    fn next_question_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::question(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::DisplayedContent)]
    fn next_display_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::displayed_content(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::Relation))]
    fn next_relation_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent_with_sort(SemanticSort::Relation, self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::RelationMetadata)]
    fn next_relation_metadata_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::relation_metadata(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::Sign))]
    fn next_sign_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::sign(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::MathExpression)]
    fn next_math_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::math_expression(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Quantity)]
    fn next_quantity_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::quantity(self.next_index);
        self.next_index += 1;
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn tokens_for_node<N: TreeNode>(&self, node: &N) -> Vec<Token> {
        let mut visitor = GeneratedSpanCollector::default();
        node.visit_in_order(&mut visitor);
        visitor.tokens.into_iter().cloned().collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_node<N: TreeNode>(
        &self,
        node: &N,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut visitor = GeneratedSpanCollector::default();
        node.visit_in_order(&mut visitor);
        let spans =
            source_spans_with_following_cmevla_period(&visitor.spans, self.options.source_text);
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_generated_subbridi(
        &self,
        subbridi: &SubbridiSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
                self.source_for_node(bridi, construct)
            }
            SubbridiSyntax::PrenexSubbridi(prenex) => {
                self.source_for_generated_subbridi(&prenex.inner_subbridi, construct)
            }
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_generated_spans(
        &self,
        spans: &[SourceSpan],
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let spans = source_spans_with_following_cmevla_period(spans, self.options.source_text);
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_abstraction_branch(
        &self,
        branch: GeneratedAbstractionBranch<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        self.source_for_node(branch.abstraction, construct)
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_abstraction_branch_tokens(
        &self,
        branch: GeneratedAbstractionBranch<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        collect_generated_node_spans(branch.nu, &mut spans);
        if let Some(nai) = branch.nai {
            collect_generated_node_spans(nai, &mut spans);
        }
        collect_generated_node_spans(branch.subbridi, &mut spans);
        if let Some(kei) = branch.kei {
            collect_generated_node_spans(kei, &mut spans);
        }
        self.source_for_generated_spans(&spans, construct)
    }

    #[requires(true)]
    #[ensures(true)]
    fn exact_source_for_node<N: TreeNode>(
        &self,
        node: &N,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut visitor = GeneratedSpanCollector::default();
        node.visit_in_order(&mut visitor);
        source_from_spans(&visitor.spans, self.options.source_text, Some(construct))
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_outer_quantified_description_domain(
        &self,
        description: &DescriptorWithOuterQuantifierSumtiSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut spans = Vec::new();
        collect_generated_node_spans(&description.description, &mut spans);
        collect_generated_node_spans(&description.tail, &mut spans);
        if let Some(ku) = &description.ku {
            spans.extend(ku.value.source_spans().into_iter().cloned());
            for free_modifier in &ku.free_modifiers {
                collect_generated_node_spans(free_modifier, &mut spans);
            }
        }
        let spans = source_spans_with_following_cmevla_period(&spans, self.options.source_text);
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

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_generated_argument_quantifier_scope_node(
        &self,
        node: GeneratedArgumentQuantifierScopeNode<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        match node {
            GeneratedArgumentQuantifierScopeNode::Sumti(sumti) => {
                self.source_for_node(sumti, construct)
            }
            GeneratedArgumentQuantifierScopeNode::SumtiBound(sumti) => {
                self.source_for_node(sumti, construct)
            }
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    fn source_for_generated_argument_quantifier_scope(
        &self,
        source: GeneratedArgumentQuantifierSource<'_>,
        node: GeneratedArgumentQuantifierScopeNode<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let source = match source {
            GeneratedArgumentQuantifierSource::QuantifiedSumti(quantified) => {
                self.source_for_node(quantified, construct)
            }
            GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description) => self
                .source_for_generated_argument_quantifier_scope_node(node, construct)
                .or_else(|| self.source_for_node(description, construct)),
            GeneratedArgumentQuantifierSource::NoGadriDescription(description) => {
                self.source_for_node(description, construct)
            }
        };
        source.or_else(|| self.source_for_generated_argument_quantifier_scope_node(node, construct))
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|(byte_start, byte_end)| byte_start <= byte_end))]
    fn source_key_for_node<N: TreeNode>(&self, node: &N) -> Option<(usize, usize)> {
        self.source_for_node(node, "source-key")
            .map(|source| (source.span.byte_start, source.span.byte_end))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_order_for_node<N: TreeNode>(&self, node: &N) -> usize {
        self.source_key_for_node(node)
            .map(|(start, _)| start)
            .unwrap_or(usize::MAX - 1)
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_relation_question(
        &self,
        question: GeneratedRelationQuestionSyntax<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        match question {
            GeneratedRelationQuestionSyntax::ProBridi(pro_bridi) => {
                self.source_for_node(pro_bridi, construct)
            }
            GeneratedRelationQuestionSyntax::GohaWord(goha) => {
                self.source_for_node(goha, construct)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_relation_parameter_syntax(
        &self,
        syntax: GeneratedRelationParameterSyntax<'_>,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        match syntax {
            GeneratedRelationParameterSyntax::ProBridi(pro_bridi) => {
                self.source_for_node(pro_bridi, construct)
            }
            GeneratedRelationParameterSyntax::GohaWord(goha) => {
                self.source_for_node(goha, construct)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|(byte_start, byte_end)| byte_start <= byte_end))]
    fn source_key_for_relation_parameter_syntax(
        &self,
        syntax: GeneratedRelationParameterSyntax<'_>,
    ) -> Option<(usize, usize)> {
        self.source_for_relation_parameter_syntax(syntax, "source-key")
            .map(|source| (source.span.byte_start, source.span.byte_end))
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_for_name_sumti(
        &self,
        name: &NameSumtiSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut visitor = GeneratedSpanCollector::default();
        name.visit_in_order(&mut visitor);
        let spans =
            source_spans_with_following_cmevla_period(&visitor.spans, self.options.source_text);
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }
}

#[requires(first_visible_place > 0)]
#[ensures(ret.2 > 0)]
fn split_generated_co_terms<'syntax>(
    terms: Vec<&'syntax TermSyntax>,
    first_visible_place: usize,
) -> (Vec<&'syntax TermSyntax>, Vec<&'syntax TermSyntax>, usize) {
    if first_visible_place != 1 {
        return (Vec::new(), terms, first_visible_place);
    }
    let mut terms = terms.into_iter();
    let Some(head_term) = terms.next() else {
        return (Vec::new(), Vec::new(), first_visible_place + 1);
    };
    (vec![head_term], terms.collect(), first_visible_place + 1)
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_is_interval(connective: &StatementConnectiveSyntax) -> bool {
    matches!(
        generated_statement_connective_primary_cmavo(connective),
        Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_operand_connective_source(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> String {
    match connective {
        jbotci_syntax::generated_model::OperandConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_source(connective)
        }
        jbotci_syntax::generated_model::OperandConnectiveSyntax::EkConnective(connective) => {
            token_text(&connective.a.value)
        }
        jbotci_syntax::generated_model::OperandConnectiveSyntax::JekConnective(connective) => {
            token_text(&connective.ja.value)
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_operand_connective_tokens(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> Vec<Token> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector.tokens.into_iter().cloned().collect()
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_primary_cmavo(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> Option<Cmavo> {
    generated_operand_connective_tokens(connective)
        .into_iter()
        .find_map(|token| {
            let cmavo = token.cmavo()?;
            (!matches!(
                cmavo,
                Cmavo::Se | Cmavo::Na | Cmavo::Nai | Cmavo::Gaho | Cmavo::Kehi
            ))
            .then_some(cmavo)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_is_logical(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> bool {
    matches!(
        generated_operand_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
                | Cmavo::Ga
                | Cmavo::Ge
                | Cmavo::Go
                | Cmavo::Gu
        )
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_is_interval(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> bool {
    matches!(
        generated_operand_connective_primary_cmavo(connective),
        Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_formula_operator(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> FormulaOperator {
    match generated_operand_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja | Cmavo::Ga) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je | Cmavo::Ge) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo | Cmavo::Go) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju | Cmavo::Gu) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_operand_connective_truth_table(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> Option<String> {
    if !generated_operand_connective_is_logical(connective) {
        return None;
    }
    let operator = generated_operand_connective_formula_operator(connective);
    let left_negated = generated_operand_connective_negates_left(connective);
    let right_negated = generated_operand_connective_negates_right(connective);
    let se = generated_operand_connective_has_se(connective);
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_negates_left(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> bool {
    generated_operand_connective_tokens(connective)
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Na))
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_negates_right(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> bool {
    generated_operand_connective_tokens(connective)
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Nai))
}

#[requires(true)]
#[ensures(!ret || generated_operand_connective_tokens(connective).iter().any(|token| token.is_selmaho(Selmaho::Se)))]
fn generated_operand_connective_has_se(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> bool {
    generated_operand_connective_tokens(connective)
        .iter()
        .any(|token| token.is_selmaho(Selmaho::Se))
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_connective_math_operator(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> MathOperator {
    MathOperator::from_label(generated_operand_connective_source(connective))
}

#[requires(generated_operand_connective_is_interval(connective))]
#[ensures(ret.as_ref().is_ok_and(|operator| operator.is_interval()) || ret.is_err())]
fn generated_operand_connective_interval_operator(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
) -> Result<MathOperator, SemanticsError> {
    match generated_operand_connective_primary_cmavo(connective) {
        Some(Cmavo::Bihi) => Ok(new!(MathOperator::UnorderedInterval)),
        Some(Cmavo::Biho) => Ok(new!(MathOperator::OrderedInterval)),
        Some(Cmavo::Mihi) => Ok(new!(MathOperator::CenteredInterval)),
        _ => Err(unsupported("non-interval operand connective")),
    }
}

#[requires(true)]
#[ensures(ret.is_none() || generated_operand_connective_is_interval(connective))]
fn generated_operand_connective_endpoint_inclusion(
    connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
    reverse_members: bool,
) -> Option<IntervalEndpointInclusion> {
    let jbotci_syntax::generated_model::OperandConnectiveSyntax::JoikConnective(
        JoikConnectiveSyntax::ClosedIntervalConnective(connective),
    ) = connective
    else {
        return None;
    };
    let left = endpoint_inclusion_for_generated_cmavo(connective.left_interval.cmavo()?)?;
    let right = endpoint_inclusion_for_generated_cmavo(connective.right_interval.value.cmavo()?)?;
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
fn generated_modal_forethought_connective_source(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> String {
    let tokens = generated_modal_forethought_connective_tokens(connective);
    token_list_text(tokens.iter())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_modal_forethought_pair_source(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    gik: &GikConnectiveSyntax,
) -> String {
    let mut parts = vec![generated_modal_forethought_connective_source(connective)];
    parts.push(token_text(&gik.gi.value));
    if let Some(nai) = &gik.nai {
        parts.push(token_text(&nai.value));
    }
    parts.join(" ")
}

#[requires(true)]
#[ensures(ret.len() >= before_terms.len() + after_terms.len())]
fn generated_forethought_termset_branch_terms<'syntax, I, T>(
    before_terms: &[&'syntax TermSyntax],
    branch_terms: I,
    after_terms: &[&'syntax TermSyntax],
) -> Vec<&'syntax TermSyntax>
where
    I: IntoIterator<Item = &'syntax T>,
    T: AsRef<TermSyntax> + 'syntax,
{
    let branch_terms = branch_terms.into_iter();
    let mut terms =
        Vec::with_capacity(before_terms.len() + branch_terms.size_hint().0 + after_terms.len());
    terms.extend_from_slice(before_terms);
    terms.extend(branch_terms.map(|term| term.as_ref()));
    terms.extend_from_slice(after_terms);
    terms
}

#[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
#[ensures(ret > 0)]
fn next_visible_place_after_generated_assignments(
    assignments: &GeneratedTermAssignments<'_>,
) -> usize {
    assignments.next_visible_place
}

#[requires(true)]
#[ensures(true)]
fn generated_shared_head_term_uses_shared_source(term: &TermSyntax) -> bool {
    !matches!(
        generated_simple_term_for_assignment(term),
        Ok(SimpleTermSyntax::PlaceTaggedSumtiTerm(_))
            | Ok(SimpleTermSyntax::TaggedSumtiTerm(_))
            | Ok(SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_))
    )
}

#[requires(true)]
#[requires(target.visible_arguments.keys().all(|place| *place > 0))]
#[requires(source.visible_arguments.keys().all(|place| *place > 0))]
#[ensures(true)]
fn extend_generated_term_assignments_shifted<'syntax>(
    target: &mut GeneratedTermAssignments<'syntax>,
    source: &GeneratedTermAssignments<'syntax>,
    visible_place_offset: usize,
) -> Result<(), SemanticsError> {
    for (place, argument) in &source.visible_arguments {
        insert_visible_argument(
            &mut target.visible_arguments,
            place + visible_place_offset,
            argument.clone(),
        )?;
    }
    target.next_visible_place = target
        .next_visible_place
        .max(source.next_visible_place + visible_place_offset);
    target
        .place_questions
        .extend(source.place_questions.clone());
    target
        .modal_terms
        .extend(source.modal_terms.iter().cloned());
    target.formula_scopes.extend(source.formula_scopes.clone());
    target
        .coequal_scope_groups
        .extend(source.coequal_scope_groups.clone());
    target
        .implicit_existentials
        .extend(source.implicit_existentials.clone());
    target
        .term_formula_scopes
        .extend(source.term_formula_scopes.clone());
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| *place == 1 || *place == 2) || ret.is_err())]
fn generated_bridi_with_leading_terms_first_visible_place(
    leading_terms: &[&TermSyntax],
) -> Result<usize, SemanticsError> {
    if next_visible_place_after_generated_terms(leading_terms, 1)? == 1 {
        Ok(2)
    } else {
        Ok(1)
    }
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place >= first_visible_place) || ret.is_err())]
fn next_visible_place_after_generated_terms(
    terms: &[&TermSyntax],
    first_visible_place: usize,
) -> Result<usize, SemanticsError> {
    let mut next_visible_place = first_visible_place;
    let mut assigned_places = BTreeSet::new();
    for term in terms {
        advance_next_visible_place_after_generated_term(
            term,
            &mut next_visible_place,
            &mut assigned_places,
        )?;
    }
    Ok(next_visible_place)
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place >= first_visible_place) || ret.is_err())]
fn first_unfilled_visible_place_after_generated_prefix_terms(
    terms: &[&TermSyntax],
    first_visible_place: usize,
) -> Result<usize, SemanticsError> {
    let mut next_visible_place = 1;
    let mut assigned_places = BTreeSet::new();
    for term in terms {
        advance_next_visible_place_after_generated_term(
            term,
            &mut next_visible_place,
            &mut assigned_places,
        )?;
    }
    Ok(first_unfilled_generated_simulated_place(
        &assigned_places,
        first_visible_place,
    ))
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|assignments| assignments.iter().all(|(place, _)| *place > 0)) || ret.is_err())]
fn generated_numbered_sumti_assignments_for_terms<'syntax>(
    terms: &[&'syntax TermSyntax],
    first_visible_place: usize,
) -> Result<Vec<(usize, &'syntax SumtiSyntax)>, SemanticsError> {
    let mut assignments = Vec::new();
    let mut assigned_places = BTreeSet::new();
    let mut next_visible_place = first_visible_place;
    for term in terms {
        generated_numbered_sumti_assignments_for_term(
            term,
            &mut assignments,
            &mut assigned_places,
            &mut next_visible_place,
        )?;
    }
    Ok(assignments)
}

#[requires(*next_visible_place > 0)]
#[requires(assignments.iter().all(|(place, _)| *place > 0))]
#[requires(assigned_places.iter().all(|place| *place > 0))]
#[ensures(true)]
fn generated_numbered_sumti_assignments_for_term<'syntax>(
    term: &'syntax TermSyntax,
    assignments: &mut Vec<(usize, &'syntax SumtiSyntax)>,
    assigned_places: &mut BTreeSet<usize>,
    next_visible_place: &mut usize,
) -> Result<(), SemanticsError> {
    match term {
        TermSyntax::TermsetGroup(termset) => {
            generated_numbered_sumti_assignments_for_simple_term(
                termset.leading_term.as_ref(),
                assignments,
                assigned_places,
                next_visible_place,
            )?;
            for continuation in &termset.continuations {
                generated_numbered_sumti_assignments_for_simple_term(
                    continuation.trailing_term.as_ref(),
                    assignments,
                    assigned_places,
                    next_visible_place,
                )?;
            }
            Ok(())
        }
        TermSyntax::SimpleTerm(simple) => generated_numbered_sumti_assignments_for_simple_term(
            simple,
            assignments,
            assigned_places,
            next_visible_place,
        ),
        TermSyntax::ConnectedTerm(ConnectedTermSyntax {
            leading_term,
            continuations,
        }) if continuations.is_empty() => generated_numbered_sumti_assignments_for_simple_term(
            leading_term.as_ref(),
            assignments,
            assigned_places,
            next_visible_place,
        ),
        _ => Err(unsupported("non-simple term")),
    }
}

#[requires(*next_visible_place > 0)]
#[requires(assignments.iter().all(|(place, _)| *place > 0))]
#[requires(assigned_places.iter().all(|place| *place > 0))]
#[ensures(true)]
fn generated_numbered_sumti_assignments_for_simple_term<'syntax>(
    term: &'syntax SimpleTermSyntax,
    assignments: &mut Vec<(usize, &'syntax SumtiSyntax)>,
    assigned_places: &mut BTreeSet<usize>,
    next_visible_place: &mut usize,
) -> Result<(), SemanticsError> {
    if let Some(description) = generated_undefined_experimental_term_description(term) {
        return Err(undefined_semantics(description));
    }
    match term {
        SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
            let place =
                first_unfilled_generated_simulated_place(assigned_places, *next_visible_place);
            assignments.push((place, sumti));
            assigned_places.insert(place);
            record_generated_simulated_visible_place_assignment(
                assigned_places,
                next_visible_place,
                place,
            );
            Ok(())
        }
        SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
            // FAI restores the displaced argument of a JAI conversion. It is not a numbered
            // place and does not participate in the numbered-assignment prepass; the JAI
            // relation builder extracts and assigns it after establishing the converted frame.
            if term.fa.value.cmavo() == Some(Cmavo::Fai) {
                return Ok(());
            }
            let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() else {
                return Ok(());
            };
            let place = fa_place(&term.fa.value)?;
            assignments.push((place, sumti));
            assigned_places.insert(place);
            record_generated_simulated_visible_place_assignment(
                assigned_places,
                next_visible_place,
                place,
            );
            Ok(())
        }
        SimpleTermSyntax::TaggedSumtiTerm(_)
        | SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | SimpleTermSyntax::NaKuTerm(_)
        | SimpleTermSyntax::BareNaTerm(_) => Ok(()),
        SimpleTermSyntax::NuhiTermset(termset) => {
            for term in &termset.termset {
                generated_numbered_sumti_assignments_for_term(
                    term,
                    assignments,
                    assigned_places,
                    next_visible_place,
                )?;
            }
            Ok(())
        }
        SimpleTermSyntax::KeTermset(termset) => {
            for term in &termset.termset {
                generated_numbered_sumti_assignments_for_term(
                    term,
                    assignments,
                    assigned_places,
                    next_visible_place,
                )?;
            }
            Ok(())
        }
        _ => Err(unsupported("non-sumti term")),
    }
}

#[requires(*next_visible_place > 0)]
#[ensures(true)]
fn advance_next_visible_place_after_generated_term(
    term: &TermSyntax,
    next_visible_place: &mut usize,
    assigned_places: &mut BTreeSet<usize>,
) -> Result<(), SemanticsError> {
    match term {
        TermSyntax::TermsetGroup(termset) => {
            advance_next_visible_place_after_generated_simple_term(
                termset.leading_term.as_ref(),
                next_visible_place,
                assigned_places,
            )?;
            for continuation in &termset.continuations {
                advance_next_visible_place_after_generated_simple_term(
                    continuation.trailing_term.as_ref(),
                    next_visible_place,
                    assigned_places,
                )?;
            }
            Ok(())
        }
        TermSyntax::SimpleTerm(simple) => advance_next_visible_place_after_generated_simple_term(
            simple,
            next_visible_place,
            assigned_places,
        ),
        TermSyntax::ConnectedTerm(ConnectedTermSyntax {
            leading_term,
            continuations,
        }) if continuations.is_empty() => advance_next_visible_place_after_generated_simple_term(
            leading_term.as_ref(),
            next_visible_place,
            assigned_places,
        ),
        _ => Err(unsupported("non-simple term")),
    }
}

#[requires(*next_visible_place > 0)]
#[ensures(true)]
fn advance_next_visible_place_after_generated_simple_term(
    simple: &SimpleTermSyntax,
    next_visible_place: &mut usize,
    assigned_places: &mut BTreeSet<usize>,
) -> Result<(), SemanticsError> {
    if let Some(description) = generated_undefined_experimental_term_description(simple) {
        return Err(undefined_semantics(description));
    }
    match simple {
        SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
            let count = generated_sumti_afterthought_for_termset(sumti)
                .map(|afterthought| 1 + afterthought.continuations.len())
                .unwrap_or(1);
            for _ in 0..count {
                let place =
                    first_unfilled_generated_simulated_place(assigned_places, *next_visible_place);
                assigned_places.insert(place);
                record_generated_simulated_visible_place_assignment(
                    assigned_places,
                    next_visible_place,
                    place,
                );
            }
            Ok(())
        }
        SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
            if term.fa.value.cmavo() == Some(Cmavo::Fiha) {
                *next_visible_place += 1;
                return Ok(());
            }
            if term.fa.value.cmavo() == Some(Cmavo::Fai) {
                return Ok(());
            }
            let place = fa_place(&term.fa.value)?;
            assigned_places.insert(place);
            record_generated_simulated_visible_place_assignment(
                assigned_places,
                next_visible_place,
                place,
            );
            Ok(())
        }
        SimpleTermSyntax::TaggedSumtiTerm(_)
        | SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | SimpleTermSyntax::NaKuTerm(_)
        | SimpleTermSyntax::BareNaTerm(_) => Ok(()),
        SimpleTermSyntax::NuhiTermset(termset) => {
            for term in &termset.termset {
                advance_next_visible_place_after_generated_term(
                    term,
                    next_visible_place,
                    assigned_places,
                )?;
            }
            Ok(())
        }
        SimpleTermSyntax::KeTermset(termset) => {
            for term in &termset.termset {
                advance_next_visible_place_after_generated_term(
                    term,
                    next_visible_place,
                    assigned_places,
                )?;
            }
            Ok(())
        }
        _ => Err(unsupported("non-sumti term")),
    }
}

#[requires(start > 0)]
#[requires(arguments.keys().all(|place| *place > 0))]
#[ensures(ret > 0)]
fn first_unfilled_generated_visible_place(
    arguments: &BTreeMap<usize, ArgumentValue>,
    start: usize,
) -> usize {
    let mut place = start;
    while arguments.contains_key(&place) {
        place += 1;
    }
    place
}

#[requires(place > 0)]
#[requires(*next_visible_place > 0)]
#[requires(arguments.keys().all(|assigned| *assigned > 0))]
#[ensures(*next_visible_place > 0)]
fn record_generated_visible_place_assignment(
    arguments: &BTreeMap<usize, ArgumentValue>,
    next_visible_place: &mut usize,
    place: usize,
) {
    *next_visible_place = place + 1;
    while arguments.contains_key(&*next_visible_place) {
        *next_visible_place += 1;
    }
}

#[requires(start > 0)]
#[requires(assigned_places.iter().all(|place| *place > 0))]
#[ensures(ret > 0)]
fn first_unfilled_generated_simulated_place(
    assigned_places: &BTreeSet<usize>,
    start: usize,
) -> usize {
    let mut place = start;
    while assigned_places.contains(&place) {
        place += 1;
    }
    place
}

#[requires(place > 0)]
#[requires(*next_visible_place > 0)]
#[requires(assigned_places.iter().all(|assigned| *assigned > 0))]
#[ensures(*next_visible_place > 0)]
fn record_generated_simulated_visible_place_assignment(
    assigned_places: &BTreeSet<usize>,
    next_visible_place: &mut usize,
    place: usize,
) {
    *next_visible_place = place + 1;
    while assigned_places.contains(&*next_visible_place) {
        *next_visible_place += 1;
    }
}

#[requires(start > 0)]
#[requires(counts.keys().all(|place| *place > 0))]
#[ensures(ret > 0)]
fn first_unfilled_generated_counted_place(counts: &BTreeMap<usize, usize>, start: usize) -> usize {
    let mut place = start;
    while counts.contains_key(&place) {
        place += 1;
    }
    place
}

#[requires(place > 0)]
#[requires(*next_visible_place > 0)]
#[requires(counts.keys().all(|assigned| *assigned > 0))]
#[ensures(*next_visible_place > 0)]
fn record_generated_counted_visible_place_assignment(
    counts: &BTreeMap<usize, usize>,
    next_visible_place: &mut usize,
    place: usize,
) {
    *next_visible_place = place + 1;
    while counts.contains_key(&*next_visible_place) {
        *next_visible_place += 1;
    }
}

#[requires(true)]
#[ensures(ret.visible_arguments.is_empty())]
fn empty_generated_term_assignments<'syntax>() -> GeneratedTermAssignments<'syntax> {
    GeneratedTermAssignments {
        visible_arguments: BTreeMap::new(),
        next_visible_place: 1,
        place_questions: Vec::new(),
        modal_terms: Vec::new(),
        formula_scopes: Vec::new(),
        coequal_scope_groups: Vec::new(),
        implicit_existentials: Vec::new(),
        term_formula_scopes: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_generated_coequal_scope_group_or_individual_scopes<'syntax>(
    mut scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    source: Option<crate::model::SemanticSource>,
    individual_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
) {
    // CLL 16.5 makes quantifier order scope-bearing; CLL 16.7 makes termsets
    // the explicit equal-scope exception.
    if scopes.len() > 1 {
        coequal_scope_groups.push(GeneratedArgumentQuantifierBundleScope { scopes, source });
    } else {
        individual_scopes.append(&mut scopes);
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_distributed_sumti_connective_formula_operator(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> FormulaOperator {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { connective, .. } => {
            generated_argument_connective_formula_operator(connective)
        }
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            generated_modal_forethought_connective_formula_operator(gek)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
fn generated_distributed_sumti_connective_source(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> Result<String, SemanticsError> {
    match connective {
        GeneratedDistributedSumtiConnective::Argument {
            connective,
            tense_modal,
            bo,
        } => {
            let connective_source = generated_argument_connective_source(connective)?;
            let Some(tense_modal) = tense_modal else {
                return Ok(connective_source);
            };
            let mut collector = GeneratedSpanCollector::default();
            tense_modal.visit_in_order(&mut collector);
            let tense_source = token_list_text(collector.tokens.iter().copied());
            if bo {
                Ok(format!("{connective_source} {tense_source} bo"))
            } else {
                Ok(format!("{connective_source} {tense_source}"))
            }
        }
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            Ok(generated_modal_forethought_connective_source(gek))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|connector| connector.as_ref().is_none_or(|connector| !connector.source.is_empty() && connector.locus == "sumti")) || ret.is_err())]
fn generated_distributed_sumti_connector(
    connective: Option<GeneratedDistributedSumtiConnective<'_>>,
    parameter: Option<SemanticObjectId>,
) -> Result<Option<Connector>, SemanticsError> {
    let Some(connective) = connective else {
        return Ok(None);
    };
    let source = generated_distributed_sumti_connective_source(connective)?;
    Ok(Some(new!(Connector {
        source,
        locus: "sumti".to_owned(),
        truth_table: generated_distributed_sumti_connective_truth_table(connective),
        parameter,
    })))
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_distributed_sumti_connective_truth_table(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> Option<String> {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { connective, .. } => {
            generated_argument_connective_truth_table(connective)
        }
        GeneratedDistributedSumtiConnective::Forethought { gek, gik } => {
            generated_modal_forethought_gik_connective_truth_table(gek, gik)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
fn generated_distributed_sumti_connective_modal_spec(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> Option<GeneratedModalStatementConnectionSpec> {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { tense_modal, .. } => {
            tense_modal.and_then(generated_modal_statement_connection_spec_for_tense_modal)
        }
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            generated_modal_statement_connection_spec_for_tense_modal(gek)
        }
    }
}

#[requires(true)]
#[ensures(ret -> generated_distributed_sumti_connective_modal_spec(connective).is_some())]
fn generated_distributed_sumti_connective_is_pure_modal(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> bool {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { .. } => false,
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            generated_modal_forethought_connective_is_pure_modal(gek)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_distributed_sumti_connective_visible_argument_is_first(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> bool {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { .. } => false,
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            generated_tense_relation_spec_for_tense_modal(gek).is_none()
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_modal_forethought_connective_tokens(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> Vec<Token> {
    let mut visitor = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut visitor);
    visitor.tokens.into_iter().cloned().collect()
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_forethought_connective_primary_cmavo(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> Option<Cmavo> {
    generated_modal_forethought_connective_tokens(connective)
        .into_iter()
        .find_map(|token| {
            let cmavo = token.cmavo()?;
            (!matches!(
                cmavo,
                Cmavo::Gi | Cmavo::Bo | Cmavo::Na | Cmavo::Nai | Cmavo::Gaho | Cmavo::Kehi
            ))
            .then_some(cmavo)
        })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| matches!(token.cmavo(), Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi))))]
fn generated_modal_forethought_connective_question_token(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> Option<Token> {
    generated_modal_forethought_connective_tokens(connective)
        .into_iter()
        .find(|token| {
            matches!(
                token.cmavo(),
                Some(Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi)
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_forethought_connective_is_logical(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    if generated_modal_forethought_connective_question_token(connective).is_some() {
        return true;
    }
    if generated_modal_statement_connection_spec_for_tense_modal(connective).is_some() {
        return true;
    }
    matches!(
        generated_modal_forethought_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
                | Cmavo::Ga
                | Cmavo::Ge
                | Cmavo::Go
                | Cmavo::Gu
                | Cmavo::Jehi
                | Cmavo::Gehi
        )
    )
}

#[requires(true)]
#[ensures(ret -> generated_modal_statement_connection_spec_for_tense_modal(connective).is_some())]
fn generated_modal_forethought_connective_is_pure_modal(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    generated_tense_relation_spec_for_tense_modal(connective).is_some()
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_forethought_connective_formula_operator(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> FormulaOperator {
    if generated_modal_forethought_connective_question_token(connective).is_some() {
        return FormulaOperator::ConnectiveQuestion;
    }
    match generated_modal_forethought_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja | Cmavo::Ga) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je | Cmavo::Ge) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo | Cmavo::Go) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju | Cmavo::Gu) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_forethought_connective_negates_left(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    generated_modal_forethought_connective_tokens(connective)
        .iter()
        .any(|token| matches!(token.cmavo(), Some(Cmavo::Na | Cmavo::Nai)))
}

#[requires(true)]
#[ensures(!ret || generated_modal_forethought_connective_tokens(connective).iter().any(|token| token.is_selmaho(Selmaho::Se)))]
fn generated_modal_forethought_connective_has_se(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    generated_modal_forethought_connective_tokens(connective)
        .iter()
        .any(|token| token.is_selmaho(Selmaho::Se))
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_modal_forethought_gik_connective_truth_table(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    gik: &GikConnectiveSyntax,
) -> Option<String> {
    generated_modal_forethought_connective_truth_table_with_right_negated(
        connective,
        gik.nai.is_some(),
    )
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_modal_forethought_connective_truth_table_with_right_negated(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    right_negated: bool,
) -> Option<String> {
    generated_modal_forethought_connective_truth_table_with_negations(
        connective,
        generated_modal_forethought_connective_negates_left(connective),
        right_negated,
    )
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_modal_forethought_connective_truth_table_with_negations(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    left_negated: bool,
    right_negated: bool,
) -> Option<String> {
    if generated_modal_forethought_connective_question_token(connective).is_some()
        || !generated_modal_forethought_connective_is_logical(connective)
    {
        return None;
    }
    let operator = generated_modal_forethought_connective_formula_operator(connective);
    let se = generated_modal_forethought_connective_tokens(connective)
        .iter()
        .any(|token| token.is_selmaho(Selmaho::Se));
    Some(
        [(true, true), (true, false), (false, true), (false, false)]
            .into_iter()
            .map(|(left, right)| {
                let left = if left_negated { !left } else { left };
                let right = if right_negated { !right } else { right };
                let result = if se {
                    connective_truth_value_for_operator(operator, right, left)
                } else {
                    connective_truth_value_for_operator(operator, left, right)
                };
                if result { 'T' } else { 'F' }
            })
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_forethought_connective_is_interval(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    matches!(
        generated_modal_forethought_connective_primary_cmavo(connective),
        Some(Cmavo::Bihi | Cmavo::Biho | Cmavo::Mihi)
    )
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn generated_nonlogical_modal_forethought_composition_operator(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> Result<CompositionOperator, SemanticsError> {
    match generated_modal_forethought_connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => Ok(CompositionOperator::Joint),
        Some(Cmavo::Joi) => Ok(CompositionOperator::Mass),
        Some(Cmavo::Ce) => Ok(CompositionOperator::Set),
        Some(Cmavo::Ceho) => Ok(CompositionOperator::Sequence),
        Some(Cmavo::Fahu) => Ok(CompositionOperator::Respectively),
        Some(Cmavo::Johe) => Ok(CompositionOperator::Union),
        Some(Cmavo::Kuha) => Ok(CompositionOperator::Intersection),
        Some(Cmavo::Pihu) => Ok(CompositionOperator::CrossProduct),
        Some(Cmavo::Bihi) => Ok(CompositionOperator::UnorderedInterval),
        Some(Cmavo::Biho) => Ok(CompositionOperator::OrderedInterval),
        Some(Cmavo::Mihi) => Ok(CompositionOperator::CenteredInterval),
        _ => Err(unsupported(&format!(
            "unsupported nonlogical forethought connective {}",
            generated_modal_forethought_connective_source(connective)
        ))),
    }
}

#[requires(true)]
#[ensures(!ret || generated_modal_forethought_connective_tokens(connective).iter().any(|token| token.is_selmaho(Selmaho::Se)))]
fn generated_modal_forethought_connective_reverses_composition_members(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
) -> bool {
    generated_modal_forethought_connective_tokens(connective)
        .iter()
        .any(|token| token.is_selmaho(Selmaho::Se))
        && matches!(
            generated_modal_forethought_connective_primary_cmavo(connective),
            Some(Cmavo::Joi | Cmavo::Ce | Cmavo::Ceho)
        )
}

#[requires(true)]
#[ensures(ret.is_none() || generated_modal_forethought_connective_is_interval(connective))]
fn generated_modal_forethought_connective_endpoint_inclusion(
    connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    reverse_members: bool,
) -> Option<IntervalEndpointInclusion> {
    if !generated_modal_forethought_connective_is_interval(connective) {
        return None;
    }
    let endpoints = generated_modal_forethought_connective_tokens(connective)
        .into_iter()
        .filter(|token| matches!(token.cmavo(), Some(Cmavo::Gaho | Cmavo::Kehi)))
        .collect::<Vec<_>>();
    let [left, right] = endpoints.as_slice() else {
        return None;
    };
    let left = endpoint_inclusion_for_generated_cmavo(left.cmavo()?)?;
    let right = endpoint_inclusion_for_generated_cmavo(right.cmavo()?)?;
    Some(if reverse_members {
        IntervalEndpointInclusion {
            left: right,
            right: left,
        }
    } else {
        IntervalEndpointInclusion { left, right }
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_xi_free_modifier(
    free_modifier: &FreeModifierSyntax,
) -> Option<&jbotci_syntax::generated_model::XiFreeModifierSyntax> {
    match free_modifier {
        FreeModifierSyntax::XiFreeModifier(free_modifier) => Some(free_modifier),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|offset| offset > 0))]
fn generated_pro_sumti_positive_xi_offset(pro_sumti: &ProSumtiSyntax) -> Option<usize> {
    let subscript = pro_sumti
        .0
        .free_modifiers
        .iter()
        .find_map(generated_xi_free_modifier)?;
    let jbotci_syntax::generated_model::XiFreeModifierSyntax::XiNumberFreeModifier(subscript) =
        subscript
    else {
        return None;
    };
    let text = generated_number_words_text(&subscript.expression.0.number.value);
    let value = parse_generated_simple_pa_integer(&text)?;
    usize::try_from(value).ok().filter(|offset| *offset > 0)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn abstraction_relation_label_from_generated(
    abstraction: &AbstractionTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let abstractor = token_text(&abstraction.nu.value);
    let relation = relation_label_from_subbridi(&abstraction.subbridi)?;
    Ok(RelationLabel::abstraction(
        abstraction_kind_for_nu(abstraction),
        abstractor,
        relation,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn abstraction_relation_label_from_zantufa_statement(
    abstraction: &ZantufaStatementAbstractionTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let abstractor = token_text(&abstraction.nu.value);
    let relation = relation_label_from_statement(&abstraction.statement)?;
    Ok(RelationLabel::abstraction(
        abstraction_kind_for_cmavo(abstraction.nu.value.cmavo()),
        abstractor,
        relation,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_statement(
    statement: &StatementSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match statement {
        StatementSyntax::StatementBase(statement) => relation_label_from_statement_base(statement),
        StatementSyntax::IStatementConnection(connection) => {
            relation_label_from_i_statement_connection(connection)
        }
        StatementSyntax::PreposedIStatementConnection(connection) => {
            relation_label_from_preposed_i_statement_connection(connection)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_i_statement_connection(
    connection: &IStatementConnectionSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut statements = vec![relation_label_from_statement_base(
        &connection.leading_statement,
    )?];
    let mut connectors = Vec::with_capacity(connection.continuations.len());
    for continuation in &connection.continuations {
        let (pending, _i, connective, trailing_statement) =
            statement_connection_tail_parts(continuation)?;
        if !pending.is_empty() {
            return Err(requires_discourse_context(
                "the elided operand in a chained pending statement-connection relation label",
            ));
        }
        statements.push(relation_label_from_statement_after_i_connective(
            trailing_statement,
        )?);
        connectors.push(new!(GeneratedRelationLabelConnector {
            source: generated_i_statement_connective_token_source(connective),
            has_bo: generated_i_statement_connective_has_bo(connective),
        }));
    }

    let mut right = statements
        .pop()
        .expect("a statement connection has a trailing statement");
    let mut pending_non_bo = Vec::new();
    while let Some(connector) = connectors.pop() {
        let data!(GeneratedRelationLabelConnector { source, has_bo }) = connector.into_data();
        let left = statements
            .pop()
            .expect("each statement connector has a left operand");
        if has_bo {
            right = RelationLabel::statement_connection(left, source, right);
        } else {
            pending_non_bo.push(new!(GeneratedPendingRelationLabelConnection {
                connector: source,
                trailing: right,
            }));
            right = left;
        }
    }
    for pending in pending_non_bo.into_iter().rev() {
        let data!(GeneratedPendingRelationLabelConnection {
            connector,
            trailing,
        }) = pending.into_data();
        right = RelationLabel::statement_connection(right, connector, trailing);
    }
    Ok(right)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_preposed_i_statement_connection(
    connection: &PreposedIStatementConnectionSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let left = relation_label_from_statement_base(&connection.leading_statement)?;
    let right = relation_label_from_statement_after_i_connective(&connection.trailing_statement)?;
    Ok(RelationLabel::statement_connection(
        left,
        generated_statement_connective_core_source(&connection.connective)?,
        right,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_statement_base(
    statement: &StatementBaseSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match statement {
        StatementBaseSyntax::BridiStatement(statement) => {
            if !statement.continuations.is_empty() {
                return Err(unsupported("bridi statement continuation relation label"));
            }
            relation_label_from_bridi(&statement.bridi)
        }
        StatementBaseSyntax::PrenexStatement(statement) => {
            relation_label_from_statement(&statement.inner_statement)
        }
        StatementBaseSyntax::TextGroupStatement(statement) => {
            relation_label_from_text_group_statement(statement)
        }
        StatementBaseSyntax::ForethoughtStatement(statement) => {
            relation_label_from_forethought_statement(statement)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_forethought_statement(
    statement: &ForethoughtStatementSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let first = relation_label_from_statement(&statement.first)?;
    let mut branches = Vec::with_capacity(1 + statement.additional_branches.len());
    branches.push(ForethoughtRelationBranch::new(
        generated_node_surface_text(&statement.first_branch.gik)?,
        relation_label_from_statement(&statement.first_branch.statement)?,
    ));
    for branch in &statement.additional_branches {
        branches.push(ForethoughtRelationBranch::new(
            generated_node_surface_text(&branch.gik)?,
            relation_label_from_statement(&branch.statement)?,
        ));
    }
    Ok(RelationLabel::forethought_statement_connection(
        generated_modal_forethought_connective_source(&statement.gek),
        first,
        branches,
        statement.gihi.as_ref().map(token_text),
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_statement_after_i_connective(
    statement: &StatementAfterIConnectiveSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match statement {
        StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
            if !statement.continuations.is_empty() {
                return Err(unsupported(
                    "bridi statement continuation after statement connective relation label",
                ));
            }
            relation_label_from_bridi(&statement.bridi)
        }
        StatementAfterIConnectiveSyntax::TextGroupStatement(statement) => {
            relation_label_from_text_group_statement(statement)
        }
        StatementAfterIConnectiveSyntax::ForethoughtStatement(statement) => {
            relation_label_from_forethought_statement(statement)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_text_group_statement(
    statement: &TextGroupStatementSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let plan = generated_text_plan_from_text(&statement.text)?;
    if !plan.leading_nai.is_empty()
        || !plan.leading_cmevla.is_empty()
        || !plan.leading_indicators.is_empty()
        || !plan.leading_free_modifiers.is_empty()
        || plan.leading_connective.is_some()
        || !plan.leading_i_statements.is_empty()
        || plan.items.len() != 1
    {
        return Err(requires_discourse_context(
            "a text-group relation label containing discourse-level material",
        ));
    }
    let GeneratedTextPlanItem::Root {
        root,
        free_modifiers,
        separator_i,
    } = &plan.items[0]
    else {
        return Err(requires_discourse_context(
            "a text-group relation label without a single denoting statement",
        ));
    };
    if !free_modifiers.is_empty() || separator_i.is_some() {
        return Err(requires_discourse_context(
            "a text-group relation label with statement-level asides",
        ));
    }
    let relation = relation_label_from_generated_text_root(*root)?;
    Ok(RelationLabel::text_group(
        statement
            .tense_modal
            .as_deref()
            .map(generated_node_surface_text)
            .transpose()?,
        token_text(&statement.tuhe.value),
        relation,
        statement.tuhu.as_ref().map(|tuhu| token_text(&tuhu.value)),
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_generated_text_root(
    root: GeneratedTextRoot<'_>,
) -> Result<RelationLabel, SemanticsError> {
    match root {
        GeneratedTextRoot::Bridi(bridi) => relation_label_from_bridi(bridi),
        GeneratedTextRoot::Fragment(GeneratedFragmentRoot::Selbri(fragment)) => {
            relation_label_from_selbri(fragment.0.as_ref())
        }
        GeneratedTextRoot::Fragment(_) => Err(requires_discourse_context(
            "a non-selbri fragment used as a text-group relation label",
        )),
        GeneratedTextRoot::StatementConnection(connection) => {
            relation_label_from_i_statement_connection(connection)
        }
        GeneratedTextRoot::PreposedStatementConnection(connection) => {
            relation_label_from_preposed_i_statement_connection(connection)
        }
        GeneratedTextRoot::PrenexStatement(statement) => {
            relation_label_from_statement(&statement.inner_statement)
        }
        GeneratedTextRoot::TextGroupStatement(statement) => {
            relation_label_from_text_group_statement(statement)
        }
        GeneratedTextRoot::ForethoughtStatement(statement) => {
            relation_label_from_forethought_statement(statement)
        }
        GeneratedTextRoot::ZantufaStatementTerms(statement) => {
            if !zantufa_statement_terms_tail_terms(&statement.tail).is_empty() {
                return Err(requires_discourse_context(
                    "statement-level terms in a text-group relation label",
                ));
            }
            relation_label_from_statement(&statement.statement)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_subbridi(
    subbridi: &SubbridiSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match subbridi {
        SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
            relation_label_from_bridi(bridi)
        }
        SubbridiSyntax::PrenexSubbridi(prenex) => {
            let terms = prenex
                .prenex_terms
                .iter()
                .map(generated_node_surface_text)
                .collect::<Result<Vec<_>, _>>()?;
            let separator = token_text(&prenex.zohu.value);
            let relation = relation_label_from_subbridi(&prenex.inner_subbridi)?;
            Ok(RelationLabel::prenex(terms, separator, relation))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_bridi(bridi: &BridiSyntax) -> Result<RelationLabel, SemanticsError> {
    let tail = match bridi {
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(tail)) => tail,
        BridiSyntax::BridiWithLeadingTerms(BridiWithLeadingTermsSyntax { bridi_tail, .. }) => {
            bridi_tail
        }
        _ => return Err(unsupported("subbridi relation label")),
    };
    let simple_tail = match simple_tail_from_bridi_tail(tail) {
        Ok(simple_tail) => simple_tail,
        Err(_) => {
            return Ok(RelationLabel::constructed(generated_node_surface_text(
                tail,
            )?));
        }
    };
    match generated_pro_bridi_target_relation_label(&simple_tail.selbri)? {
        Some(relation) => Ok(relation),
        None => Ok(RelationLabel::constructed(generated_node_surface_text(
            simple_tail.selbri.as_ref(),
        )?)),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_relation_name_for_generated_unit_pair(
    leading: &TanruUnitSyntax,
    trailing: &TanruUnitSyntax,
) -> Result<String, SemanticsError> {
    tanru_relation_name_for_generated_unit_run(leading, &[], trailing, true)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_relation_name_for_generated_unit_run(
    first_unit: &TanruUnitSyntax,
    modifier_units: &[TanruUnitSyntax],
    trailing_unit: &TanruUnitSyntax,
    parenthesize_modifier_units: bool,
) -> Result<String, SemanticsError> {
    Ok(format!(
        "{}-{}",
        tanru_unit_label_from_generated_unit_run(
            first_unit,
            modifier_units,
            parenthesize_modifier_units
        )?,
        tanru_operand_label_from_generated_unit(trailing_unit)?
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_generated_unit_run(
    first_unit: &TanruUnitSyntax,
    additional_units: &[TanruUnitSyntax],
    parenthesize_additional_units: bool,
) -> Result<String, SemanticsError> {
    let mut label = tanru_unit_label_from_generated_unit(first_unit)?;
    for unit in additional_units {
        label.push('-');
        if parenthesize_additional_units {
            label.push_str(&tanru_operand_label_from_generated_unit(unit)?);
        } else {
            label.push_str(&tanru_unit_label_from_generated_unit(unit)?);
        }
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_operand_label_from_generated_unit(
    unit: &TanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let label = tanru_unit_label_from_generated_unit(unit)?;
    if generated_tanru_unit_label_needs_parentheses(unit) {
        Ok(format!("({label})"))
    } else {
        Ok(label)
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_label_needs_parentheses(unit: &TanruUnitSyntax) -> bool {
    match unit.0.first.as_ref() {
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => true,
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => match unit.base.base.as_ref() {
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
                grouped_tanru_unit_label_needs_parentheses(grouped)
            }
            base => scalar_negated_tanru_atom_base(base)
                .and_then(scalar_negated_tanru_unit_inner_grouped)
                .is_some_and(|(grouped, _)| grouped_tanru_unit_label_needs_parentheses(grouped)),
        },
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            let base = linked_tanru_unit_from_cei(unit.base.as_ref());
            match base.base.base.as_ref() {
                TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
                    grouped_tanru_unit_label_needs_parentheses(grouped)
                }
                base => scalar_negated_tanru_atom_base(base)
                    .and_then(scalar_negated_tanru_unit_inner_grouped)
                    .is_some_and(|(grouped, _)| {
                        grouped_tanru_unit_label_needs_parentheses(grouped)
                    }),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_is_connected_selbri_formula(unit: &TanruUnitSyntax) -> bool {
    !unit.0.links.is_empty()
        || bo_or_linked_tanru_unit_has_bo_connective(unit.0.first.as_ref())
        || matches!(
            unit.0.first.as_ref(),
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_tanru_formula_source_construct(tanru: &TanruSelbriSyntax) -> &'static str {
    if generated_tanru_unit_has_bo_connective_source(&tanru.first_unit)
        || tanru
            .additional_units
            .iter()
            .any(generated_tanru_unit_has_bo_connective_source)
    {
        "connected-selbri-formula"
    } else {
        "tanru-formula"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_tanru_unit_formula_source_construct(unit: &TanruUnitSyntax) -> &'static str {
    if generated_tanru_unit_has_bo_connective_source(unit) {
        "connected-selbri-formula"
    } else {
        "tanru-formula"
    }
}

#[requires(true)]
#[ensures(ret == bo_or_linked_tanru_unit_has_bo_connective(unit.0.first.as_ref()))]
fn generated_tanru_unit_has_bo_connective_source(unit: &TanruUnitSyntax) -> bool {
    bo_or_linked_tanru_unit_has_bo_connective(unit.0.first.as_ref())
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_preallocates_head_eventuality(unit: &TanruUnitSyntax) -> bool {
    generated_tanru_unit_is_connected_selbri_formula(unit)
        || matches!(
            unit.0.first.as_ref(),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
                | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_)
        )
}

#[requires(true)]
#[ensures(true)]
fn bo_or_linked_tanru_unit_has_bo_connective(unit: &BoOrLinkedTanruUnitSyntax) -> bool {
    match unit {
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
            unit.bo_connective.is_some()
                || bo_or_linked_tanru_unit_has_bo_connective(unit.trailing_unit.as_ref())
        }
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn linked_tanru_unit_from_cei(unit: &LinkedTanruUnitForCeiSyntax) -> LinkedTanruUnitSyntax {
    LinkedTanruUnitSyntax {
        base: std::sync::Arc::new(tanru_unit_atom_from_cei(unit.base.as_ref())),
        linkargs: unit.linkargs.clone(),
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_atom_from_cei(unit: &TanruUnitAtomForCeiSyntax) -> TanruUnitAtomSyntax {
    TanruUnitAtomSyntax {
        conversions: unit.conversions.clone(),
        base: std::sync::Arc::new(tanru_unit_atom_base_from_cei(unit.base.as_ref())),
    }
}

#[requires(true)]
#[ensures(true)]
fn tanru_unit_atom_base_from_cei(unit: &TanruUnitAtomBaseForCeiSyntax) -> TanruUnitAtomBaseSyntax {
    match unit {
        TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::ProBridiTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::OrdinalTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::OrdinalTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::WordTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::AbstractionTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::AbstractionTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaStatementAbstractionTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::ZantufaStatementAbstractionTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaMeTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::ZantufaMeTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaMexMoiTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::ZantufaMexMoiTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::OperatorSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::OperatorSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::QuotedBridiSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::QuotedBridiSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::QuotedTextSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::QuotedTextSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::TextSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::TextSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::TagSelbriTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::TagSelbriTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::GohaWordTanruUnit(unit.clone())
        }
        TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(unit) => {
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(unit.clone())
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_linked_tanru_unit(
    unit: &LinkedTanruUnitForCeiSyntax,
) -> Option<String> {
    if unit.linkargs.is_some() {
        return None;
    }
    let base = tanru_unit_atom_from_cei(unit.base.as_ref());
    assigned_pro_bridi_reference_label_for_tanru_unit_atom(&base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_tanru_unit_atom(
    unit: &TanruUnitAtomSyntax,
) -> Option<String> {
    assigned_pro_bridi_reference_label_for_tanru_unit_atom_base(unit.base.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_tanru_unit_atom_base(
    unit: &TanruUnitAtomBaseSyntax,
) -> Option<String> {
    match unit {
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            assigned_pro_bridi_reference_label_for_token(&word.value)
        }
        TanruUnitAtomBaseSyntax::ProBridiTanruUnit(unit) => {
            assigned_pro_bridi_reference_label_for_pro_bridi_tanru_unit(unit)
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            assigned_pro_bridi_reference_label_for_scalar_negated_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<String> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            assigned_pro_bridi_reference_label_for_tanru_unit_atom(atom)
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(unit) => {
            assigned_pro_bridi_reference_label_for_pro_bridi_tanru_unit(unit)
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_pro_bridi_tanru_unit(
    unit: &ProBridiTanruUnitSyntax,
) -> Option<String> {
    let cmavo = unit.goha.value.cmavo()?;
    matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)
        .then(|| cmavo.canonical_text().to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn assigned_pro_bridi_reference_label_for_token(token: &Token) -> Option<String> {
    let text = token_text(token);
    matches!(
        text.as_str(),
        "broda" | "brode" | "brodi" | "brodo" | "brodu"
    )
    .then_some(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
fn relation_label_from_tanru_unit(unit: &TanruUnitSyntax) -> Result<RelationLabel, SemanticsError> {
    relation_label_from_generated_tanru_unit(unit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
fn relation_label_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            relation_label_from_linked_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
            relation_label_from_bound_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => {
            relation_label_from_forethought_selbri_group_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            let base = linked_tanru_unit_from_cei(unit.base.as_ref());
            relation_label_from_linked_tanru_unit(&base)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
fn relation_label_from_linked_tanru_unit(
    unit: &LinkedTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    relation_label_from_tanru_unit_atom(&unit.base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
fn relation_label_from_bound_tanru_unit(
    unit: &BoundTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let leading = relation_label_from_linked_tanru_unit(&unit.leading_unit)?;
    let trailing = relation_label_from_bo_or_linked_tanru_unit(&unit.trailing_unit)?;
    if let Some(connective) = &unit.bo_connective {
        Ok(RelationLabel::constructed(format!(
            "{} {} {}",
            leading,
            relation_afterthought_connective_label(connective)?,
            trailing
        )))
    } else {
        Ok(RelationLabel::constructed(format!(
            "{leading} bo {trailing}"
        )))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
fn relation_label_from_forethought_selbri_group_tanru_unit(
    unit: &ForethoughtSelbriGroupTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut parts = vec![
        generated_guhek_connective_source(&unit.guhek),
        relation_phrase_label_from_selbri(unit.leading_selbri.as_ref())?,
        token_text(&unit.first_branch.gik.gi.value),
        relation_label_from_bo_or_linked_tanru_unit(unit.first_branch.unit.as_ref())?
            .display_text(),
    ];
    for branch in &unit.additional_branches {
        parts.push(token_text(&branch.gik.0.value));
        parts.push(
            relation_label_from_bo_or_linked_tanru_unit(branch.unit.as_ref())?.display_text(),
        );
    }
    Ok(RelationLabel::constructed(parts.join(" ")))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            tanru_unit_label_from_linked_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
            tanru_unit_label_from_bound_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => {
            relation_label_from_forethought_selbri_group_tanru_unit(unit)
                .map(|label| label.display_text())
        }
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            let base = linked_tanru_unit_from_cei(unit.base.as_ref());
            tanru_unit_label_from_linked_tanru_unit(&base)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_linked_tanru_unit(
    unit: &LinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    tanru_unit_label_from_tanru_unit_atom(&unit.base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_bound_tanru_unit(
    unit: &BoundTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let leading = tanru_unit_label_from_linked_tanru_unit(&unit.leading_unit)?;
    let trailing = tanru_operand_label_from_bo_or_linked_tanru_unit(&unit.trailing_unit)?;
    Ok(format!("{leading}-{trailing}"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_operand_label_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let label = tanru_unit_label_from_bo_or_linked_tanru_unit(unit)?;
    if bo_or_linked_tanru_unit_label_needs_parentheses(unit) {
        Ok(format!("({label})"))
    } else {
        Ok(label)
    }
}

#[requires(true)]
#[ensures(true)]
fn bo_or_linked_tanru_unit_label_needs_parentheses(unit: &BoOrLinkedTanruUnitSyntax) -> bool {
    match unit {
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => true,
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => match unit.base.base.as_ref() {
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
                grouped_tanru_unit_label_needs_parentheses(grouped)
            }
            base => scalar_negated_tanru_atom_base(base)
                .and_then(scalar_negated_tanru_unit_inner_grouped)
                .is_some_and(|(grouped, _)| grouped_tanru_unit_label_needs_parentheses(grouped)),
        },
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            let base = linked_tanru_unit_from_cei(unit.base.as_ref());
            match base.base.base.as_ref() {
                TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
                    grouped_tanru_unit_label_needs_parentheses(grouped)
                }
                base => scalar_negated_tanru_atom_base(base)
                    .and_then(scalar_negated_tanru_unit_inner_grouped)
                    .is_some_and(|(grouped, _)| {
                        grouped_tanru_unit_label_needs_parentheses(grouped)
                    }),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn grouped_tanru_unit_label_needs_parentheses(grouped: &GroupedTanruUnitSyntax) -> bool {
    !grouped.selbri.leading_selbri.additional_units.is_empty()
        || !grouped.selbri.continuations.is_empty()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_tanru_unit_atom(
    unit: &TanruUnitAtomSyntax,
) -> Result<String, SemanticsError> {
    match unit.base.as_ref() {
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            tanru_unit_label_from_scalar_negated_tanru_unit(unit)
        }
        TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
            tanru_label_from_connected_selbri(&grouped.selbri)
        }
        _ => relation_label_from_tanru_unit_atom(unit).map(|label| label.display_text()),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            tanru_unit_label_from_tanru_unit_atom(atom)
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi).display_text())
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => {
            Err(unsupported("tagged scalar-negated tanru unit"))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
fn relation_label_from_tanru_unit_atom(
    unit: &TanruUnitAtomSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match unit.base.as_ref() {
        TanruUnitAtomBaseSyntax::OrdinalTanruUnit(ordinal) => {
            relation_label_from_ordinal_tanru_unit(ordinal)
        }
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word))
        | TanruUnitAtomBaseSyntax::GohaWordTanruUnit(GohaWordTanruUnitSyntax(word)) => {
            Ok(relation_label_from_token(&word.value))
        }
        TanruUnitAtomBaseSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi))
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            relation_label_from_scalar_negated_tanru_unit(unit)
        }
        TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
            relation_label_from_grouped_tanru_unit(grouped)
        }
        TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) => {
            abstraction_relation_label_from_generated(abstraction)
        }
        TanruUnitAtomBaseSyntax::ZantufaStatementAbstractionTanruUnit(abstraction) => {
            abstraction_relation_label_from_zantufa_statement(abstraction)
        }
        TanruUnitAtomBaseSyntax::ZantufaMeTanruUnit(unit) => {
            relation_label_from_zantufa_me_tanru_unit(unit)
        }
        TanruUnitAtomBaseSyntax::ZantufaMexMoiTanruUnit(unit) => {
            relation_label_from_zantufa_mex_moi_tanru_unit(unit)
        }
        TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(_) => {
            Ok(RelationLabel::constructed("referentOf".to_owned()))
        }
        TanruUnitAtomBaseSyntax::OperatorSelbriTanruUnit(operator) => {
            relation_label_from_operator_selbri_tanru_unit(operator)
        }
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) => {
            relation_label_from_jai_inner_tanru_unit(&unit.inner_unit)
        }
        TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit) => {
            relation_label_from_generated_tanru_unit(&unit.base)
        }
        TanruUnitAtomBaseSyntax::QuotedBridiSelbriTanruUnit(unit) => Ok(
            RelationLabel::constructed(generated_node_surface_text(unit)?),
        ),
        TanruUnitAtomBaseSyntax::QuotedTextSelbriTanruUnit(unit) => Ok(RelationLabel::constructed(
            generated_node_surface_text(unit)?,
        )),
        TanruUnitAtomBaseSyntax::TextSelbriTanruUnit(unit) => Ok(RelationLabel::constructed(
            generated_node_surface_text(unit)?,
        )),
        TanruUnitAtomBaseSyntax::TagSelbriTanruUnit(unit) => Ok(RelationLabel::constructed(
            generated_node_surface_text(unit)?,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn simple_sumti_from_term(term: &TermSyntax) -> Result<&SumtiSyntax, SemanticsError> {
    let simple = match term {
        TermSyntax::SimpleTerm(simple) => simple,
        TermSyntax::ConnectedTerm(ConnectedTermSyntax {
            leading_term,
            continuations,
        }) if continuations.is_empty() => leading_term.as_ref(),
        _ => return Err(unsupported("non-simple term")),
    };
    let SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) = simple else {
        return Err(unsupported("non-sumti term"));
    };
    Ok(sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_term_for_assignment(
    term: &TermSyntax,
) -> Result<&SimpleTermSyntax, SemanticsError> {
    match term {
        TermSyntax::SimpleTerm(simple) => Ok(simple),
        TermSyntax::ConnectedTerm(ConnectedTermSyntax {
            leading_term,
            continuations,
        }) if continuations.is_empty() => Ok(leading_term.as_ref()),
        _ => Err(unsupported("non-simple term")),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_undefined_experimental_term_description(
    term: &SimpleTermSyntax,
) -> Option<&'static str> {
    match term {
        SimpleTermSyntax::NoihaAdverbialTerm(_) => Some("an experimental NOIhA adverbial term"),
        SimpleTermSyntax::FihoiAdverbialTerm(_) => Some("an experimental FIhOI adverbial term"),
        SimpleTermSyntax::SoiAdverbialTerm(_) => Some("an experimental SOI/XOI adverbial term"),
        SimpleTermSyntax::JaiTaggedSumtiTerm(_) => Some("an experimental Zantufa JAI tag term"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_governed_termset_indices_for_terms(terms: &[&TermSyntax]) -> BTreeSet<usize> {
    let mut indices = BTreeSet::new();
    for (modifier_index, term) in terms.iter().enumerate() {
        if !generated_tagged_term_governs_following_termset(term) {
            continue;
        }
        if let Some(termset_index) =
            generated_nearest_following_governed_termset_index(terms, modifier_index + 1)
        {
            indices.insert(termset_index);
        }
    }
    indices
}

#[requires(true)]
#[ensures(true)]
fn generated_tagged_term_governs_following_termset(term: &TermSyntax) -> bool {
    let Ok(SimpleTermSyntax::TaggedSumtiTerm(term)) = generated_simple_term_for_assignment(term)
    else {
        return false;
    };
    matches!(
        term.sumti.as_ref(),
        TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_)
    ) && generated_tense_modal_has_event_modifier(term.tense_modal.as_ref())
}

#[requires(start <= terms.len())]
#[ensures(ret.is_none_or(|index| index >= start && index < terms.len()))]
fn generated_nearest_following_governed_termset_index(
    terms: &[&TermSyntax],
    start: usize,
) -> Option<usize> {
    terms
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, term)| generated_term_is_governed_termset(term).then_some(index))
}

#[requires(true)]
#[ensures(true)]
fn generated_term_is_governed_termset(term: &TermSyntax) -> bool {
    match term {
        TermSyntax::TermsetGroup(_) => true,
        _ => {
            let Ok(simple) = generated_simple_term_for_assignment(term) else {
                return false;
            };
            matches!(
                simple,
                SimpleTermSyntax::NuhiTermset(_) | SimpleTermSyntax::KeTermset(_)
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tense_modal_is_lahu_modal<N: TreeNode>(tense_modal: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Lahu))
}

#[requires(true)]
#[ensures(true)]
fn generated_term_has_distributed_sumti_connection(term: &TermSyntax) -> bool {
    let Ok(simple) = generated_simple_term_for_assignment(term) else {
        return false;
    };
    match simple {
        SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
            generated_logical_sumti_connection_for_branch(GeneratedDistributedSumtiBranch::Sumti(
                sumti,
            ))
            .is_ok_and(|connection| connection.is_some())
        }
        SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_logical_sumti_connection_for_branch(
                    GeneratedDistributedSumtiBranch::Sumti(sumti),
                )
                .is_ok_and(|connection| connection.is_some())
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        SimpleTermSyntax::TaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_logical_sumti_connection_for_branch(
                    GeneratedDistributedSumtiBranch::Sumti(sumti),
                )
                .is_ok_and(|connection| connection.is_some())
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        _ => false,
    }
}

#[requires(first_visible_place > 0)]
#[ensures(true)]
fn generated_terms_have_duplicate_numbered_assignments(
    terms: &[&TermSyntax],
    first_visible_place: usize,
) -> Result<bool, SemanticsError> {
    let mut counts = BTreeMap::<usize, usize>::new();
    let mut next_visible_place = first_visible_place;
    for term in terms {
        let Ok(simple) = generated_simple_term_for_assignment(term) else {
            return Ok(false);
        };
        match simple {
            SimpleTermSyntax::SumtiTerm(_) => {
                let place = first_unfilled_generated_counted_place(&counts, next_visible_place);
                *counts.entry(place).or_default() += 1;
                record_generated_counted_visible_place_assignment(
                    &counts,
                    &mut next_visible_place,
                    place,
                );
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                if term.fa.value.cmavo() == Some(Cmavo::Fiha) {
                    next_visible_place += 1;
                    continue;
                }
                if term.fa.value.cmavo() == Some(Cmavo::Fai) {
                    continue;
                }
                let place = fa_place(&term.fa.value)?;
                *counts.entry(place).or_default() += 1;
                record_generated_counted_visible_place_assignment(
                    &counts,
                    &mut next_visible_place,
                    place,
                );
            }
            SimpleTermSyntax::TaggedSumtiTerm(_)
            | SimpleTermSyntax::NaKuTerm(_)
            | SimpleTermSyntax::BareNaTerm(_) => {}
            _ => return Ok(false),
        }
    }
    Ok(counts.values().any(|count| *count > 1))
}

#[requires(true)]
#[ensures(true)]
fn generated_logical_sumti_connection_for_branch(
    branch: GeneratedDistributedSumtiBranch<'_>,
) -> Result<Option<GeneratedLogicalSumtiConnection<'_>>, SemanticsError> {
    match branch {
        GeneratedDistributedSumtiBranch::Sumti(sumti) => {
            if let Some(VuhoSumtiAttachmentTailSyntax::VuhoConnectedSumtiAttachmentTail(tail)) =
                &sumti.vuho_attachment
            {
                let connection = &tail.sumti_connection;
                if generated_argument_connective_is_logical(&connection.connective)
                    && !generated_argument_connective_is_interval(&connection.connective)
                {
                    return Ok(Some(GeneratedLogicalSumtiConnection {
                        leading: GeneratedDistributedSumtiBranch::SumtiGrouped(
                            sumti.base_sumti.as_ref(),
                        ),
                        connective: GeneratedDistributedSumtiConnective::Argument {
                            connective: &connection.connective,
                            tense_modal: None,
                            bo: false,
                        },
                        trailing: GeneratedDistributedSumtiBranch::Sumti(connection.sumti.as_ref()),
                        relative_clauses: None,
                    }));
                }
                return Ok(None);
            }
            let relative_clauses = generated_vuho_relative_clause_list_for_sumti(sumti);
            if sumti.vuho_attachment.is_some() && relative_clauses.is_none() {
                return Ok(None);
            }
            let connection = generated_logical_sumti_connection_for_branch(
                GeneratedDistributedSumtiBranch::SumtiGrouped(sumti.base_sumti.as_ref()),
            )?;
            Ok(
                connection.map(|connection| GeneratedLogicalSumtiConnection {
                    relative_clauses: relative_clauses.or(connection.relative_clauses),
                    ..connection
                }),
            )
        }
        GeneratedDistributedSumtiBranch::SumtiGrouped(sumti) => {
            if let Some(tail) = &sumti.grouped_tail {
                if generated_argument_connective_is_logical(&tail.connective)
                    && !generated_argument_connective_is_interval(&tail.connective)
                {
                    return Ok(Some(GeneratedLogicalSumtiConnection {
                        leading: generated_sumti_afterthought_branch(&sumti.leading_sumti),
                        connective: GeneratedDistributedSumtiConnective::Argument {
                            connective: &tail.connective,
                            tense_modal: tail.tense_modal.as_deref(),
                            bo: false,
                        },
                        trailing: GeneratedDistributedSumtiBranch::Sumti(tail.inner_sumti.as_ref()),
                        relative_clauses: None,
                    }));
                }
                return Ok(None);
            }
            generated_logical_sumti_connection_for_branch(generated_sumti_afterthought_branch(
                &sumti.leading_sumti,
            ))
        }
        GeneratedDistributedSumtiBranch::SumtiAfterthought(prefix) => {
            let sumti = prefix.sumti;
            let continuation_count = prefix.continuation_count;
            if continuation_count == 0 {
                return generated_logical_sumti_connection_for_branch(
                    GeneratedDistributedSumtiBranch::SumtiBound(sumti.leading_sumti.as_ref()),
                );
            }
            let continuation = &sumti.continuations[continuation_count - 1];
            if generated_argument_connective_is_logical(&continuation.connective)
                && !generated_argument_connective_is_interval(&continuation.connective)
            {
                return Ok(Some(GeneratedLogicalSumtiConnection {
                    leading: GeneratedDistributedSumtiBranch::SumtiAfterthought(new!(
                        GeneratedSumtiAfterthoughtPrefix {
                            sumti,
                            continuation_count: continuation_count - 1,
                        }
                    )),
                    connective: GeneratedDistributedSumtiConnective::Argument {
                        connective: &continuation.connective,
                        tense_modal: None,
                        bo: false,
                    },
                    trailing: GeneratedDistributedSumtiBranch::SumtiBound(
                        continuation.sumti.as_ref(),
                    ),
                    relative_clauses: None,
                }));
            }
            Ok(None)
        }
        GeneratedDistributedSumtiBranch::SumtiBound(sumti) => {
            if let Some(tail) = &sumti.bound_tail {
                if generated_argument_connective_is_logical(tail.connective.as_ref())
                    && !generated_argument_connective_is_interval(tail.connective.as_ref())
                {
                    return Ok(Some(GeneratedLogicalSumtiConnection {
                        leading: GeneratedDistributedSumtiBranch::SumtiForethought(
                            sumti.leading_sumti.as_ref(),
                        ),
                        connective: GeneratedDistributedSumtiConnective::Argument {
                            connective: tail.connective.as_ref(),
                            tense_modal: tail.tense_modal.as_deref(),
                            bo: true,
                        },
                        trailing: GeneratedDistributedSumtiBranch::SumtiBound(
                            tail.trailing_sumti.as_ref(),
                        ),
                        relative_clauses: None,
                    }));
                }
                return Ok(None);
            }
            generated_logical_sumti_connection_for_branch(
                GeneratedDistributedSumtiBranch::SumtiForethought(sumti.leading_sumti.as_ref()),
            )
        }
        GeneratedDistributedSumtiBranch::SumtiForethought(sumti) => {
            let SumtiForethoughtSyntax::ForethoughtSumti(forethought) = sumti else {
                return Ok(None);
            };
            if generated_modal_forethought_connective_is_logical(&forethought.gek)
                && !generated_modal_forethought_connective_is_interval(&forethought.gek)
            {
                return Ok(Some(GeneratedLogicalSumtiConnection {
                    leading: GeneratedDistributedSumtiBranch::Sumti(
                        forethought.leading_sumti.as_ref(),
                    ),
                    connective: GeneratedDistributedSumtiConnective::Forethought {
                        gek: &forethought.gek,
                        gik: &forethought.first_branch.gik,
                    },
                    trailing: GeneratedDistributedSumtiBranch::SumtiForethought(
                        forethought.first_branch.sumti.as_ref(),
                    ),
                    relative_clauses: None,
                }));
            }
            Ok(None)
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret, GeneratedDistributedSumtiBranch::SumtiAfterthought(_)))]
fn generated_sumti_afterthought_branch(
    sumti: &SumtiAfterthoughtSyntax,
) -> GeneratedDistributedSumtiBranch<'_> {
    GeneratedDistributedSumtiBranch::SumtiAfterthought(new!(GeneratedSumtiAfterthoughtPrefix {
        sumti,
        continuation_count: sumti.continuations.len(),
    }))
}

#[requires(true)]
#[ensures(true)]
fn generated_distributed_sumti_connective_negates_left(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> bool {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { connective, .. } => {
            generated_argument_connective_negates_left(connective)
        }
        GeneratedDistributedSumtiConnective::Forethought { gek, .. } => {
            generated_modal_forethought_connective_negates_left(gek)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_distributed_sumti_connective_negates_right(
    connective: GeneratedDistributedSumtiConnective<'_>,
) -> bool {
    match connective {
        GeneratedDistributedSumtiConnective::Argument { connective, .. } => {
            generated_argument_connective_negates_right(connective)
        }
        GeneratedDistributedSumtiConnective::Forethought { gik, .. } => gik.nai.is_some(),
    }
}

#[requires(true)]
#[ensures(true)]
fn simple_sumti_base_from_sumti(sumti: &SumtiSyntax) -> Result<&SumtiBaseSyntax, SemanticsError> {
    let SumtiSyntax {
        base_sumti,
        vuho_attachment,
    } = sumti;
    if vuho_attachment.is_some() {
        return Err(unsupported("VUhO attached sumti"));
    }
    let SumtiGroupedSyntax {
        leading_sumti,
        grouped_tail,
    } = base_sumti.as_ref();
    if grouped_tail.is_some() {
        return Err(unsupported("grouped sumti"));
    }
    let SumtiAfterthoughtSyntax {
        leading_sumti,
        continuations,
    } = leading_sumti.as_ref();
    if !continuations.is_empty() {
        return Err(unsupported("afterthought sumti"));
    }
    let SumtiBoundSyntax {
        leading_sumti,
        bound_tail,
    } = leading_sumti.as_ref();
    if bound_tail.is_some() {
        return Err(unsupported("bound sumti"));
    }
    let SumtiForethoughtSyntax::SimpleSumti(SimpleSumtiSyntax {
        base_sumti,
        relative_clauses,
    }) = leading_sumti.as_ref()
    else {
        return Err(unsupported("forethought sumti"));
    };
    if relative_clauses.is_some() {
        return Err(unsupported("relative clauses"));
    }
    let SumtiAtomSyntax::SumtiBase(sumti_base) = base_sumti.as_ref() else {
        return Err(unsupported("quantified sumti"));
    };
    Ok(sumti_base)
}

#[requires(true)]
#[ensures(true)]
fn generated_quantified_sumti_from_sumti(sumti: &SumtiSyntax) -> Option<&QuantifiedSumtiSyntax> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    let SumtiAtomSyntax::QuantifiedSumti(quantified) = simple.base_sumti.as_ref() else {
        return None;
    };
    Some(quantified)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| source.is_none_or(|source| matches!(source, GeneratedArgumentQuantifierSource::QuantifiedSumti(_) | GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(_) | GeneratedArgumentQuantifierSource::NoGadriDescription(_)))) || ret.is_err())]
fn generated_argument_quantifier_source_from_sumti(
    sumti: &SumtiSyntax,
) -> Result<Option<GeneratedArgumentQuantifierSource<'_>>, SemanticsError> {
    if let Some(quantified_sumti) = generated_quantified_sumti_from_sumti(sumti) {
        return Ok(Some(GeneratedArgumentQuantifierSource::QuantifiedSumti(
            quantified_sumti,
        )));
    }
    if let Some(description) = outer_quantified_description_from_sumti(sumti) {
        return Ok(Some(
            GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description),
        ));
    }
    no_gadri_description_from_sumti(sumti)
        .map(|description| description.map(GeneratedArgumentQuantifierSource::NoGadriDescription))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| source.is_none_or(|source| matches!(source, GeneratedArgumentQuantifierSource::QuantifiedSumti(_) | GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(_) | GeneratedArgumentQuantifierSource::NoGadriDescription(_)))) || ret.is_err())]
fn generated_argument_quantifier_source_from_sumti_bound(
    sumti: &SumtiBoundSyntax,
) -> Result<Option<GeneratedArgumentQuantifierSource<'_>>, SemanticsError> {
    if let Some(quantified) = generated_quantified_sumti_from_sumti_bound(sumti) {
        return Ok(Some(GeneratedArgumentQuantifierSource::QuantifiedSumti(
            quantified,
        )));
    }
    if let Some(description) = outer_quantified_description_from_sumti_bound(sumti) {
        return Ok(Some(
            GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description),
        ));
    }
    no_gadri_description_from_sumti_bound(sumti)
        .map(|description| description.map(GeneratedArgumentQuantifierSource::NoGadriDescription))
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_has_argument_formula_scope(sumti: &SumtiSyntax) -> Result<bool, SemanticsError> {
    if generated_argument_quantifier_source_from_sumti(sumti)?.is_some() {
        return Ok(true);
    }
    if let Some(simple) = generated_simple_sumti_from_sumti(sumti)
        && let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::LaheSumti(lahe)) =
            simple.base_sumti.as_ref()
    {
        return generated_sumti_has_argument_formula_scope(&lahe.inner_sumti);
    }
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return Ok(false);
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if afterthought.continuations.is_empty()
        || afterthought
            .continuations
            .iter()
            .any(|continuation| generated_argument_connective_is_logical(&continuation.connective))
    {
        return Ok(false);
    }
    if generated_argument_quantifier_source_from_sumti_bound(&afterthought.leading_sumti)?.is_some()
    {
        return Ok(true);
    }
    afterthought
        .continuations
        .iter()
        .try_fold(false, |has_scope, continuation| {
            Ok::<bool, SemanticsError>(
                has_scope
                    || generated_argument_quantifier_source_from_sumti_bound(&continuation.sumti)?
                        .is_some(),
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_from_sumti(sumti: &SumtiSyntax) -> Option<&SimpleSumtiSyntax> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() || afterthought.leading_sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) =
        afterthought.leading_sumti.leading_sumti.as_ref()
    else {
        return None;
    };
    Some(simple)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_quantified_da_series_pro_sumti_from_sumti(
    sumti: &SumtiSyntax,
) -> Option<&ProSumtiSyntax> {
    generated_quantified_da_series_pro_sumti(generated_quantified_sumti_from_sumti(sumti)?)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_quantified_variable_sort(sumti: &SumtiSyntax) -> SemanticSort {
    if let Some(quantified) = generated_quantified_sumti_from_sumti(sumti) {
        return generated_sumti_base_variable_sort(&quantified.inner_sumti);
    }
    if let Some(description) = outer_quantified_description_from_sumti(sumti) {
        return description
            .description
            .0
            .value
            .cmavo()
            .map(description_sumti_sort_for_cmavo)
            .unwrap_or(SemanticSort::Entity);
    }
    SemanticSort::Entity
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_variable_sort(sumti: &SumtiBaseSyntax) -> SemanticSort {
    match sumti {
        SumtiBaseSyntax::NumberSumti(_) => SemanticSort::Number,
        SumtiBaseSyntax::QuotedSumti(_) => SemanticSort::Sign,
        SumtiBaseSyntax::NameSumti(name) => gadri_name_sort(name.la.value.cmavo()),
        SumtiBaseSyntax::LaheSumti(sumti) => referent_qualifier_sort(sumti.lahe.value.cmavo()),
        SumtiBaseSyntax::DescriptorWithGadriSumti(description) => description
            .description
            .0
            .value
            .cmavo()
            .map(description_sumti_sort_for_cmavo)
            .unwrap_or(SemanticSort::Entity),
        _ => SemanticSort::Entity,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_argument_scope_source_quantifier<'syntax>(
    source: GeneratedArgumentQuantifierSource<'syntax>,
) -> &'syntax QuantifierSyntax {
    match source {
        GeneratedArgumentQuantifierSource::QuantifiedSumti(sumti) => &sumti.quantifier,
        GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description) => {
            &description.outer_quantifier
        }
        GeneratedArgumentQuantifierSource::NoGadriDescription(description) => {
            &description.quantifier
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn description_sumti_sort_for_cmavo(cmavo: Cmavo) -> SemanticSort {
    match cmavo {
        Cmavo::Loi | Cmavo::Lei | Cmavo::Lai => SemanticSort::Mass,
        Cmavo::Lohi | Cmavo::Lehi | Cmavo::Lahi => SemanticSort::Set,
        _ => SemanticSort::Entity,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| !spec.relation.is_empty() && !spec.member_word.is_empty()))]
fn aggregate_description_spec(cmavo: Cmavo) -> Option<AggregateDescriptionSpec> {
    match cmavo {
        Cmavo::Loi => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Mass,
                relation: "gunma",
                member_cmavo: Cmavo::Lo,
                member_word: "lo",
            }
        ))),
        Cmavo::Lei => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Mass,
                relation: "gunma",
                member_cmavo: Cmavo::Le,
                member_word: "le",
            }
        ))),
        Cmavo::Lai => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Mass,
                relation: "gunma",
                member_cmavo: Cmavo::La,
                member_word: "la",
            }
        ))),
        Cmavo::Lohi => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Set,
                relation: "selcmi",
                member_cmavo: Cmavo::Lo,
                member_word: "lo",
            }
        ))),
        Cmavo::Lehi => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Set,
                relation: "selcmi",
                member_cmavo: Cmavo::Le,
                member_word: "le",
            }
        ))),
        Cmavo::Lahi => Some(AggregateDescriptionSpec::from_data(data!(
            AggregateDescriptionSpec {
                sort: SemanticSort::Set,
                relation: "selcmi",
                member_cmavo: Cmavo::La,
                member_word: "la",
            }
        ))),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn afterthought_sumti_from_sumti(
    sumti: &SumtiSyntax,
) -> Result<Option<&SumtiAfterthoughtSyntax>, SemanticsError> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return Ok(None);
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if afterthought.continuations.is_empty() {
        return Ok(None);
    }
    Ok(Some(afterthought))
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_for_distribution(
    sumti: &SumtiSyntax,
) -> Option<&SumtiAfterthoughtSyntax> {
    if !generated_sumti_vuho_attachment_is_distribution_transparent(sumti)
        || sumti.base_sumti.grouped_tail.is_some()
    {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if afterthought.continuations.is_empty() {
        return None;
    }
    afterthought
        .continuations
        .iter()
        .all(|continuation| {
            generated_argument_connective_is_logical(&continuation.connective)
                && !generated_argument_connective_is_interval(&continuation.connective)
        })
        .then_some(afterthought)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_for_distribution(sumti: &SumtiSyntax) -> Option<&SumtiBoundSyntax> {
    if !generated_sumti_vuho_attachment_is_distribution_transparent(sumti)
        || sumti.base_sumti.grouped_tail.is_some()
    {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() {
        return None;
    }
    let bound = afterthought.leading_sumti.as_ref();
    let tail = bound.bound_tail.as_ref()?;
    (generated_argument_connective_is_logical(tail.connective.as_ref())
        && !generated_argument_connective_is_interval(tail.connective.as_ref()))
    .then_some(bound)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_for_termset(
    sumti: &SumtiSyntax,
) -> Option<&SumtiAfterthoughtSyntax> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if afterthought.continuations.is_empty() {
        return None;
    }
    afterthought
        .continuations
        .iter()
        .all(|continuation| {
            generated_argument_connective_primary_cmavo(&continuation.connective)
                == Some(Cmavo::Cehe)
        })
        .then_some(afterthought)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_forethought_for_distribution(
    sumti: &SumtiSyntax,
) -> Option<&ForethoughtSumtiSyntax> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() {
        return None;
    }
    let bound = afterthought.leading_sumti.as_ref();
    if bound.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::ForethoughtSumti(forethought) = bound.leading_sumti.as_ref() else {
        return None;
    };
    if !forethought.additional_branches.is_empty() {
        return None;
    }
    (generated_modal_forethought_connective_is_logical(&forethought.gek)
        && !generated_modal_forethought_connective_is_interval(&forethought.gek))
    .then_some(forethought)
}

#[requires(true)]
#[ensures(true)]
fn no_gadri_description_from_sumti(
    sumti: &SumtiSyntax,
) -> Result<Option<&DescriptorWithoutGadriSumtiSyntax>, SemanticsError> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return Ok(None);
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() {
        return Ok(None);
    }
    no_gadri_description_from_sumti_bound(&afterthought.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn outer_quantified_description_from_sumti(
    sumti: &SumtiSyntax,
) -> Option<&DescriptorWithOuterQuantifierSumtiSyntax> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() || afterthought.leading_sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) =
        afterthought.leading_sumti.leading_sumti.as_ref()
    else {
        return None;
    };
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(
        description,
    )) = simple.base_sumti.as_ref()
    else {
        return None;
    };
    Some(description)
}

#[requires(true)]
#[ensures(true)]
fn generated_quantified_sumti_from_sumti_bound(
    sumti: &SumtiBoundSyntax,
) -> Option<&QuantifiedSumtiSyntax> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) = sumti.leading_sumti.as_ref() else {
        return None;
    };
    let SumtiAtomSyntax::QuantifiedSumti(quantified) = simple.base_sumti.as_ref() else {
        return None;
    };
    Some(quantified)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_quantified_da_series_pro_sumti_from_sumti_bound(
    sumti: &SumtiBoundSyntax,
) -> Option<&ProSumtiSyntax> {
    generated_quantified_da_series_pro_sumti(generated_quantified_sumti_from_sumti_bound(sumti)?)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_quantified_da_series_pro_sumti(
    quantified: &QuantifiedSumtiSyntax,
) -> Option<&ProSumtiSyntax> {
    let SumtiBaseSyntax::ProSumti(pro_sumti) = &*quantified.inner_sumti else {
        return None;
    };
    pro_sumti
        .0
        .value
        .cmavo()
        .is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))
        .then_some(pro_sumti)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
fn generated_da_series_word_for_argument_scope(
    scope: &GeneratedArgumentQuantifierScope<'_>,
) -> Option<String> {
    let GeneratedArgumentQuantifierSource::QuantifiedSumti(quantified) = scope.source else {
        return None;
    };
    Some(token_text(
        &generated_quantified_da_series_pro_sumti(quantified)?
            .0
            .value,
    ))
}

#[requires(scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[ensures(ret.variable == scope.variable)]
fn generated_da_series_scope_binding_from_scope<'syntax>(
    scope: &GeneratedArgumentQuantifierScope<'syntax>,
) -> GeneratedDaSeriesScopeBinding<'syntax> {
    let mut restriction_nodes = scope.source_restriction_nodes.clone();
    restriction_nodes.push(scope.node);
    GeneratedDaSeriesScopeBinding {
        variable: scope.variable,
        restriction_nodes,
        restriction_formulas: Vec::new(),
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

#[requires(!word.is_empty())]
#[ensures(true)]
fn assignable_koha_cmavo_for_word(word: &str) -> Option<Cmavo> {
    let cmavo = Cmavo::from_text(word)?;
    is_assignable_koha(cmavo).then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn outer_quantified_description_from_sumti_bound(
    sumti: &SumtiBoundSyntax,
) -> Option<&DescriptorWithOuterQuantifierSumtiSyntax> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) = sumti.leading_sumti.as_ref() else {
        return None;
    };
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(
        description,
    )) = simple.base_sumti.as_ref()
    else {
        return None;
    };
    Some(description)
}

#[requires(true)]
#[ensures(true)]
fn no_gadri_description_from_sumti_bound(
    sumti: &SumtiBoundSyntax,
) -> Result<Option<&DescriptorWithoutGadriSumtiSyntax>, SemanticsError> {
    if sumti.bound_tail.is_some() {
        return Ok(None);
    }
    let SumtiForethoughtSyntax::SimpleSumti(SimpleSumtiSyntax {
        base_sumti,
        relative_clauses: None,
    }) = sumti.leading_sumti.as_ref()
    else {
        return Ok(None);
    };
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::DescriptorWithoutGadriSumti(description)) =
        base_sumti.as_ref()
    else {
        return Ok(None);
    };
    Ok(Some(description))
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_variable_sort(sumti: &SumtiBoundSyntax) -> SemanticSort {
    if let Some(quantified) = generated_quantified_sumti_from_sumti_bound(sumti) {
        return generated_sumti_base_variable_sort(&quantified.inner_sumti);
    }
    if let Some(description) = outer_quantified_description_from_sumti_bound(sumti) {
        return description
            .description
            .0
            .value
            .cmavo()
            .map(description_sumti_sort_for_cmavo)
            .unwrap_or(SemanticSort::Entity);
    }
    SemanticSort::Entity
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_relative_clause_list(sumti: &SumtiSyntax) -> Option<&RelativeClauseListSyntax> {
    if sumti.vuho_attachment.is_some() {
        return generated_vuho_relative_clause_list_for_sumti(sumti);
    }
    if sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() || afterthought.leading_sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) =
        afterthought.leading_sumti.leading_sumti.as_ref()
    else {
        return None;
    };
    if simple.relative_clauses.is_some() {
        return simple.relative_clauses.as_ref();
    }
    match simple.base_sumti.as_ref() {
        SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::LaheSumti(sumti)) => {
            sumti.relative_clauses.as_ref()
        }
        SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::DescriptorWithoutGadriSumti(description)) => {
            description.relative_clauses.as_ref()
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|clause| clause.association_marker.value.cmavo() == Some(Cmavo::Goi)))]
fn generated_goi_assignment_clause(
    relative_clauses: &RelativeClauseListSyntax,
) -> Option<&SumtiAssociationRelativeClauseSyntax> {
    generated_goi_assignment_clause_atom(&relative_clauses.first).or_else(|| {
        relative_clauses.additional.iter().find_map(|tail| {
            let atom = match tail {
                RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => tail.inner.as_ref(),
                RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => tail.inner.as_ref(),
            };
            generated_goi_assignment_clause_atom(atom)
        })
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|clause| clause.association_marker.value.cmavo() == Some(Cmavo::Goi)))]
fn generated_goi_assignment_clause_atom(
    clause: &RelativeClauseAtomSyntax,
) -> Option<&SumtiAssociationRelativeClauseSyntax> {
    match clause {
        RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause)
            if clause.association_marker.value.cmavo() == Some(Cmavo::Goi) =>
        {
            Some(clause)
        }
        RelativeClauseAtomSyntax::BridiRelativeClause(_) => None,
        RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_is_assignable_reference(sumti: &SumtiSyntax) -> bool {
    let Some(simple) = generated_simple_sumti_from_sumti(sumti) else {
        return false;
    };
    generated_simple_sumti_is_assignable_reference(simple)
}

#[requires(true)]
#[ensures(true)]
fn generated_relative_sumti_is_assignable_reference(sumti: &RelativeSumtiSyntax) -> bool {
    match sumti {
        RelativeSumtiSyntax::PlainRelativeSumti(PlainRelativeSumtiSyntax(sumti)) => {
            generated_sumti_is_assignable_reference(sumti)
        }
        RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => match sumti.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_sumti_is_assignable_reference(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        RelativeSumtiSyntax::NaKuRelativeSumti(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_is_assignable_reference(sumti: &SimpleSumtiSyntax) -> bool {
    let SumtiAtomSyntax::SumtiBase(base_sumti) = sumti.base_sumti.as_ref() else {
        return false;
    };
    match base_sumti {
        SumtiBaseSyntax::ProSumti(pro_sumti) => {
            pro_sumti.0.value.cmavo().is_some_and(is_assignable_koha)
        }
        SumtiBaseSyntax::LerfuStringSumti(_) => true,
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_vuho_relative_clause_list_for_sumti(
    sumti: &SumtiSyntax,
) -> Option<&RelativeClauseListSyntax> {
    let VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(tail) =
        sumti.vuho_attachment.as_ref()?
    else {
        return None;
    };
    tail.sumti_connection
        .is_none()
        .then_some(&tail.relative_clauses)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_vuho_attachment_is_distribution_transparent(sumti: &SumtiSyntax) -> bool {
    sumti.vuho_attachment.is_none()
        || generated_vuho_relative_clause_list_for_sumti(sumti).is_some()
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_relative_clause_list(
    sumti: &SumtiBoundSyntax,
) -> Option<&RelativeClauseListSyntax> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) = sumti.leading_sumti.as_ref() else {
        return None;
    };
    if simple.relative_clauses.is_some() {
        return simple.relative_clauses.as_ref();
    }
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::DescriptorWithoutGadriSumti(description)) =
        simple.base_sumti.as_ref()
    else {
        return None;
    };
    description.relative_clauses.as_ref()
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti<'syntax>(
    sumti: &'syntax SumtiSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    generated_occurrence_relative_clause_lists_for_sumti_grouped(&sumti.base_sumti, out);
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_grouped<'syntax>(
    sumti: &'syntax SumtiGroupedSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    if sumti.grouped_tail.is_none() {
        generated_occurrence_relative_clause_lists_for_sumti_afterthought(
            &sumti.leading_sumti,
            out,
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_afterthought<'syntax>(
    sumti: &'syntax SumtiAfterthoughtSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    if sumti.continuations.is_empty() {
        generated_occurrence_relative_clause_lists_for_sumti_bound(&sumti.leading_sumti, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_bound<'syntax>(
    sumti: &'syntax SumtiBoundSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    if sumti.bound_tail.is_none() {
        generated_occurrence_relative_clause_lists_for_sumti_forethought(&sumti.leading_sumti, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_forethought<'syntax>(
    sumti: &'syntax SumtiForethoughtSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    if let SumtiForethoughtSyntax::SimpleSumti(sumti) = sumti {
        generated_occurrence_relative_clause_lists_for_simple_sumti(sumti, out);
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_simple_sumti<'syntax>(
    sumti: &'syntax SimpleSumtiSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    generated_occurrence_relative_clause_lists_for_sumti_atom(&sumti.base_sumti, out);
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_atom<'syntax>(
    sumti: &'syntax SumtiAtomSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    match sumti {
        SumtiAtomSyntax::SumtiBase(sumti) => {
            generated_occurrence_relative_clause_lists_for_sumti_base(sumti, out);
        }
        SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_occurrence_relative_clause_lists_for_sumti_base(&sumti.inner_sumti, out);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_sumti_base<'syntax>(
    sumti: &'syntax SumtiBaseSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    match sumti {
        SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
            generated_occurrence_relative_clause_lists_for_sumti(&sumti.inner_sumti, out);
        }
        SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
            generated_occurrence_relative_clause_lists_for_sumti(&sumti.inner_sumti, out);
        }
        SumtiBaseSyntax::LaheSumti(sumti) => {
            generated_occurrence_relative_clause_lists_for_sumti(&sumti.inner_sumti, out);
        }
        SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
            generated_occurrence_relative_clause_lists_for_description_tail(&description.tail, out);
        }
        SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(description) => {
            generated_occurrence_relative_clause_lists_for_description_tail(&description.tail, out);
        }
        SumtiBaseSyntax::LaheTermWrapper(_)
        | SumtiBaseSyntax::ScalarNegatedTermWrapperWithBo(_)
        | SumtiBaseSyntax::ScalarNegatedTermWrapper(_)
        | SumtiBaseSyntax::BridiDescriptionSumti(_)
        | SumtiBaseSyntax::NameSumti(_)
        | SumtiBaseSyntax::DescriptionConnectionSumti(_)
        | SumtiBaseSyntax::DescriptorWithoutGadriSumti(_)
        | SumtiBaseSyntax::NumberSumti(_)
        | SumtiBaseSyntax::LerfuStringSumti(_)
        | SumtiBaseSyntax::QuotedSumti(_)
        | SumtiBaseSyntax::ProSumti(_) => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_occurrence_relative_clause_lists_for_description_tail<'syntax>(
    tail: &'syntax DescriptionTailSyntax,
    out: &mut Vec<&'syntax RelativeClauseListSyntax>,
) {
    if tail.leading_tail_elements.tail_sumti.is_none()
        && let Some(relative_clauses) = &tail.leading_tail_elements.relative_clauses
    {
        out.push(relative_clauses);
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_is_command_target(sumti: &SumtiSyntax) -> bool {
    generated_sumti_spine_cmavo(sumti) == Some(Cmavo::Ko)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_is_elided(sumti: &SumtiSyntax) -> bool {
    generated_sumti_spine_cmavo(sumti) == Some(Cmavo::Zohe)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_is_deleted(sumti: &SumtiSyntax) -> bool {
    generated_sumti_spine_cmavo(sumti) == Some(Cmavo::Ziho)
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (1..=5).contains(&place)))]
fn generated_voha_place_for_sumti(sumti: &SumtiSyntax) -> Option<usize> {
    match generated_sumti_spine_cmavo(sumti)? {
        Cmavo::Voha => Some(1),
        Cmavo::Vohe => Some(2),
        Cmavo::Vohi => Some(3),
        Cmavo::Voho => Some(4),
        Cmavo::Vohu => Some(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_is_direct_anaphora_candidate(sumti: &SumtiSyntax) -> bool {
    match generated_unconnected_sumti_atom(sumti) {
        Some(SumtiAtomSyntax::SumtiBase(base)) => {
            generated_sumti_base_records_recent_referent(base)
        }
        Some(SumtiAtomSyntax::QuantifiedSumti(sumti)) => {
            generated_sumti_base_records_recent_referent(&sumti.inner_sumti)
        }
        None => true,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_records_recent_referent(sumti: &SumtiBaseSyntax) -> bool {
    if let SumtiBaseSyntax::ProSumti(pro_sumti) = sumti {
        return pro_sumti.0.value.cmavo() == Some(Cmavo::Ri);
    }
    !matches!(
        sumti,
        SumtiBaseSyntax::ProSumti(_)
            | SumtiBaseSyntax::LaheSumti(_)
            | SumtiBaseSyntax::ScalarNegatedSumti(_)
            | SumtiBaseSyntax::ScalarNegatedSumtiWithBo(_)
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_unconnected_sumti_atom(sumti: &SumtiSyntax) -> Option<&SumtiAtomSyntax> {
    if sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() || afterthought.leading_sumti.bound_tail.is_some() {
        return None;
    }
    let SumtiForethoughtSyntax::SimpleSumti(simple) =
        afterthought.leading_sumti.leading_sumti.as_ref()
    else {
        return None;
    };
    Some(simple.base_sumti.as_ref())
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_spine_free_modifiers(sumti: &SumtiSyntax) -> Option<&[FreeModifierSyntax]> {
    match generated_unconnected_sumti_atom(sumti)? {
        SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::ProSumti(pro_sumti)) => {
            Some(&pro_sumti.0.free_modifiers)
        }
        SumtiAtomSyntax::QuantifiedSumti(quantified) => match quantified.inner_sumti.as_ref() {
            SumtiBaseSyntax::ProSumti(pro_sumti) => Some(&pro_sumti.0.free_modifiers),
            _ => None,
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_spine_cmavo(sumti: &SumtiSyntax) -> Option<Cmavo> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    generated_sumti_afterthought_spine_cmavo(&sumti.base_sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_spine_cmavo(sumti: &SumtiAfterthoughtSyntax) -> Option<Cmavo> {
    if !sumti.continuations.is_empty() {
        return None;
    }
    generated_sumti_bound_spine_cmavo(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_spine_cmavo(sumti: &SumtiBoundSyntax) -> Option<Cmavo> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    generated_sumti_forethought_spine_cmavo(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_forethought_spine_cmavo(sumti: &SumtiForethoughtSyntax) -> Option<Cmavo> {
    match sumti {
        SumtiForethoughtSyntax::SimpleSumti(simple) => generated_simple_sumti_spine_cmavo(simple),
        SumtiForethoughtSyntax::ForethoughtSumti(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_spine_cmavo(sumti: &SimpleSumtiSyntax) -> Option<Cmavo> {
    generated_sumti_atom_spine_cmavo(&sumti.base_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_atom_spine_cmavo(sumti: &SumtiAtomSyntax) -> Option<Cmavo> {
    match sumti {
        SumtiAtomSyntax::SumtiBase(base) => generated_sumti_base_spine_cmavo(base),
        SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_sumti_base_spine_cmavo(&sumti.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_spine_cmavo(sumti: &SumtiBaseSyntax) -> Option<Cmavo> {
    match sumti {
        SumtiBaseSyntax::ProSumti(pro_sumti) => pro_sumti.0.value.cmavo(),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_node_contains_cmavo<N: TreeNode>(node: &N, cmavo: Cmavo) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    node.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| token.cmavo() == Some(cmavo))
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_contains_current_level_keha(statement: &StatementSyntax) -> bool {
    match statement {
        StatementSyntax::StatementBase(statement) => {
            generated_statement_base_contains_current_level_keha(statement)
        }
        StatementSyntax::IStatementConnection(connection) => {
            generated_statement_base_contains_current_level_keha(&connection.leading_statement)
                || connection.continuations.iter().any(|continuation| {
                    generated_i_statement_connection_tail_contains_current_level_keha(continuation)
                })
        }
        StatementSyntax::PreposedIStatementConnection(connection) => {
            generated_statement_base_contains_current_level_keha(&connection.leading_statement)
                || generated_statement_after_i_connective_contains_current_level_keha(
                    &connection.trailing_statement,
                )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_i_statement_connection_tail_contains_current_level_keha(
    tail: &IStatementConnectionTailSyntax,
) -> bool {
    match tail {
        IStatementConnectionTailSyntax::ChainedIConnectiveStatementTail(tail) => {
            generated_statement_after_i_connective_contains_current_level_keha(
                &tail.trailing_statement,
            )
        }
        IStatementConnectionTailSyntax::SimpleIConnectiveStatementTail(tail) => {
            generated_statement_after_i_connective_contains_current_level_keha(
                &tail.trailing_statement,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_after_i_connective_contains_current_level_keha(
    statement: &StatementAfterIConnectiveSyntax,
) -> bool {
    match statement {
        StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
            generated_bridi_statement_contains_current_level_keha(statement)
        }
        StatementAfterIConnectiveSyntax::TextGroupStatement(_) => false,
        StatementAfterIConnectiveSyntax::ForethoughtStatement(statement) => {
            generated_forethought_statement_contains_current_level_keha(statement)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_base_contains_current_level_keha(statement: &StatementBaseSyntax) -> bool {
    match statement {
        StatementBaseSyntax::PrenexStatement(statement) => {
            statement
                .prenex_terms
                .iter()
                .any(generated_term_contains_current_level_keha)
                || generated_statement_contains_current_level_keha(&statement.inner_statement)
        }
        StatementBaseSyntax::BridiStatement(statement) => {
            generated_bridi_statement_contains_current_level_keha(statement)
        }
        StatementBaseSyntax::TextGroupStatement(_) => false,
        StatementBaseSyntax::ForethoughtStatement(statement) => {
            generated_forethought_statement_contains_current_level_keha(statement)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_forethought_statement_contains_current_level_keha(
    statement: &ForethoughtStatementSyntax,
) -> bool {
    generated_statement_contains_current_level_keha(&statement.first)
        || generated_statement_contains_current_level_keha(&statement.first_branch.statement)
        || statement
            .additional_branches
            .iter()
            .any(|branch| generated_statement_contains_current_level_keha(&branch.statement))
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_statement_contains_current_level_keha(statement: &BridiStatementSyntax) -> bool {
    generated_bridi_contains_current_level_keha(&statement.bridi)
        || statement
            .continuations
            .iter()
            .any(generated_bridi_statement_continuation_contains_current_level_keha)
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_statement_continuation_contains_current_level_keha(
    continuation: &BridiStatementContinuationSyntax,
) -> bool {
    match continuation {
        BridiStatementContinuationSyntax::BoBridiStatementContinuation(continuation) => {
            generated_subbridi_contains_current_level_keha(&continuation.trailing_subbridi)
        }
        BridiStatementContinuationSyntax::KeBridiStatementContinuation(continuation) => {
            generated_subbridi_contains_current_level_keha(&continuation.trailing_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_subbridi_contains_current_level_keha(subbridi: &SubbridiSyntax) -> bool {
    match subbridi {
        SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
            generated_bridi_contains_current_level_keha(bridi)
        }
        SubbridiSyntax::PrenexSubbridi(prenex) => {
            prenex
                .prenex_terms
                .iter()
                .any(generated_term_contains_current_level_keha)
                || generated_subbridi_contains_current_level_keha(&prenex.inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_contains_current_level_keha(bridi: &BridiSyntax) -> bool {
    match bridi {
        BridiSyntax::BridiWithLeadingTerms(BridiWithLeadingTermsSyntax {
            leading_terms,
            bridi_tail,
            ..
        }) => {
            leading_terms
                .iter()
                .any(generated_term_contains_current_level_keha)
                || generated_bridi_tail_contains_current_level_keha(bridi_tail)
        }
        BridiSyntax::BridiWithPostCuTerms(BridiWithPostCuTermsSyntax {
            leading_terms,
            bridi_tail,
            ..
        }) => {
            leading_terms
                .iter()
                .any(generated_term_contains_current_level_keha)
                || bridi_tail
                    .terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
                || generated_bridi_tail_contains_current_level_keha(&bridi_tail.bridi_tail)
        }
        BridiSyntax::BareCuBridi(BareCuBridiSyntax { bridi_tail, .. })
        | BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
            generated_bridi_tail_contains_current_level_keha(bridi_tail)
        }
        BridiSyntax::BareCuTermsBridi(BareCuTermsBridiSyntax { bridi_tail, .. }) => {
            bridi_tail
                .terms
                .iter()
                .any(generated_term_contains_current_level_keha)
                || generated_bridi_tail_contains_current_level_keha(&bridi_tail.bridi_tail)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_tail_contains_current_level_keha(tail: &BridiTailSyntax) -> bool {
    match tail {
        BridiTailSyntax::ZantufaGroupedBridiTail(tail) => {
            generated_bridi_tail_contains_current_level_keha(&tail.bridi_tail)
                || tail
                    .tail_terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
        }
        BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
            generated_afterthought_bridi_tail_contains_current_level_keha(&tail.first)
                || tail.ke_continuation.as_ref().is_some_and(|continuation| {
                    generated_bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                        || continuation
                            .tail_terms
                            .iter()
                            .any(generated_term_contains_current_level_keha)
                })
        }
        BridiTailSyntax::BridiTailWithoutTailTerms(tail) => {
            generated_afterthought_bridi_tail_without_tail_terms_contains_current_level_keha(
                &tail.first,
            ) || tail.ke_continuation.as_ref().is_some_and(|continuation| {
                generated_bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                    || continuation
                        .tail_terms
                        .iter()
                        .any(generated_term_contains_current_level_keha)
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_afterthought_bridi_tail_contains_current_level_keha(
    tail: &AfterthoughtBridiTailSyntax,
) -> bool {
    generated_bo_grouped_bridi_tail_contains_current_level_keha(&tail.0.first)
        || tail.0.links.iter().any(|link| {
            generated_bo_grouped_bridi_tail_contains_current_level_keha(&link.bridi_tail)
                || link
                    .tail_terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_afterthought_bridi_tail_without_tail_terms_contains_current_level_keha(
    tail: &AfterthoughtBridiTailWithoutTailTermsSyntax,
) -> bool {
    generated_bo_grouped_bridi_tail_without_tail_terms_contains_current_level_keha(&tail.0.first)
        || tail.0.links.iter().any(|link| {
            generated_bo_grouped_bridi_tail_without_tail_terms_contains_current_level_keha(
                &link.bridi_tail,
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_bo_grouped_bridi_tail_contains_current_level_keha(
    tail: &BoGroupedBridiTailSyntax,
) -> bool {
    generated_simple_bridi_tail_contains_current_level_keha(&tail.first)
        || tail.bo_continuation.as_ref().is_some_and(|continuation| {
            generated_bo_grouped_bridi_tail_contains_current_level_keha(&continuation.bridi_tail)
                || continuation
                    .tail_terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_bo_grouped_bridi_tail_without_tail_terms_contains_current_level_keha(
    tail: &BoGroupedBridiTailWithoutTailTermsSyntax,
) -> bool {
    generated_simple_bridi_tail_without_tail_terms_contains_current_level_keha(&tail.first)
        || tail.bo_continuation.as_ref().is_some_and(|continuation| {
            generated_bo_grouped_bridi_tail_without_tail_terms_contains_current_level_keha(
                &continuation.bridi_tail,
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_bridi_tail_contains_current_level_keha(tail: &SimpleBridiTailSyntax) -> bool {
    match tail {
        SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(forethought) => {
            generated_forethought_bridi_connection_contains_current_level_keha(&forethought.0)
        }
        SimpleBridiTailSyntax::SelbriSimpleBridiTail(simple_tail) => {
            generated_selbri_contains_current_level_keha(&simple_tail.selbri)
                || simple_tail
                    .terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_bridi_tail_without_tail_terms_contains_current_level_keha(
    tail: &SimpleBridiTailWithoutTailTermsSyntax,
) -> bool {
    match tail {
        SimpleBridiTailWithoutTailTermsSyntax::ForethoughtSimpleBridiTailWithoutTailTerms(
            forethought,
        ) => generated_forethought_bridi_connection_without_tail_terms_contains_current_level_keha(
            &forethought.0,
        ),
        SimpleBridiTailWithoutTailTermsSyntax::SelbriSimpleBridiTailWithoutTailTerms(
            simple_tail,
        ) => generated_selbri_contains_current_level_keha(&simple_tail.selbri),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_forethought_bridi_connection_contains_current_level_keha(
    connection: &ForethoughtBridiConnectionSyntax,
) -> bool {
    match connection {
        ForethoughtBridiConnectionSyntax::DirectForethoughtBridiConnection(connection) => {
            generated_subbridi_contains_current_level_keha(&connection.first)
                || generated_subbridi_contains_current_level_keha(&connection.first_branch.branch)
                || connection
                    .additional_branches
                    .iter()
                    .any(|branch| generated_subbridi_contains_current_level_keha(&branch.branch))
                || connection
                    .tail_terms
                    .iter()
                    .any(generated_term_contains_current_level_keha)
        }
        ForethoughtBridiConnectionSyntax::GroupedForethoughtBridiConnection(connection) => {
            generated_forethought_bridi_connection_contains_current_level_keha(&connection.inner)
        }
        ForethoughtBridiConnectionSyntax::NegatedForethoughtBridiConnection(connection) => {
            generated_forethought_bridi_connection_contains_current_level_keha(&connection.inner)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_forethought_bridi_connection_without_tail_terms_contains_current_level_keha(
    connection: &ForethoughtBridiConnectionWithoutTailTermsSyntax,
) -> bool {
    match connection {
        ForethoughtBridiConnectionWithoutTailTermsSyntax::DirectForethoughtBridiConnectionWithoutTailTerms(
            connection,
        ) => {
            generated_subbridi_contains_current_level_keha(&connection.first)
                || generated_subbridi_contains_current_level_keha(&connection.first_branch.branch)
                || connection.additional_branches.iter().any(|branch| {
                    generated_subbridi_contains_current_level_keha(&branch.branch)
                })
        }
        ForethoughtBridiConnectionWithoutTailTermsSyntax::GroupedForethoughtBridiConnectionWithoutTailTerms(
            connection,
        ) => {
            generated_forethought_bridi_connection_without_tail_terms_contains_current_level_keha(
                &connection.inner,
            )
        }
        ForethoughtBridiConnectionWithoutTailTermsSyntax::NegatedForethoughtBridiConnectionWithoutTailTerms(
            connection,
        ) => {
            generated_forethought_bridi_connection_without_tail_terms_contains_current_level_keha(
                &connection.inner,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_term_contains_current_level_keha(term: &TermSyntax) -> bool {
    match term {
        TermSyntax::PeheTermsetConnection(connection) => {
            generated_pehe_termset_operand_contains_current_level_keha(&connection.leading_term)
                || connection.continuations.iter().any(|continuation| {
                    generated_pehe_termset_operand_contains_current_level_keha(
                        &continuation.trailing_term,
                    )
                })
        }
        TermSyntax::BoundTermConnection(connection) => {
            generated_simple_term_contains_current_level_keha(&connection.leading_term)
                || generated_simple_term_contains_current_level_keha(&connection.trailing_term)
        }
        TermSyntax::TermsetGroup(group) => {
            generated_simple_term_contains_current_level_keha(&group.leading_term)
                || group.continuations.iter().any(|continuation| {
                    generated_simple_term_contains_current_level_keha(&continuation.trailing_term)
                })
        }
        TermSyntax::ConnectedTerm(connection) => {
            generated_simple_term_contains_current_level_keha(&connection.leading_term)
                || connection.continuations.iter().any(|continuation| {
                    generated_simple_term_contains_current_level_keha(&continuation.trailing_term)
                })
        }
        TermSyntax::SimpleTerm(term) => generated_simple_term_contains_current_level_keha(term),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_pehe_termset_operand_contains_current_level_keha(
    operand: &PeheTermsetOperandSyntax,
) -> bool {
    match operand {
        PeheTermsetOperandSyntax::BoundTermConnection(connection) => {
            generated_simple_term_contains_current_level_keha(&connection.leading_term)
                || generated_simple_term_contains_current_level_keha(&connection.trailing_term)
        }
        PeheTermsetOperandSyntax::TermsetGroup(group) => {
            generated_simple_term_contains_current_level_keha(&group.leading_term)
                || group.continuations.iter().any(|continuation| {
                    generated_simple_term_contains_current_level_keha(&continuation.trailing_term)
                })
        }
        PeheTermsetOperandSyntax::SimpleTerm(term) => {
            generated_simple_term_contains_current_level_keha(term)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_term_contains_current_level_keha(term: &SimpleTermSyntax) -> bool {
    match term {
        SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
            generated_sumti_contains_current_level_keha(sumti)
        }
        SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_sumti_contains_current_level_keha(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        SimpleTermSyntax::JaiTaggedSumtiTerm(term) => {
            generated_sumti_contains_current_level_keha(&term.sumti)
        }
        SimpleTermSyntax::TaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_sumti_contains_current_level_keha(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        SimpleTermSyntax::NoihaAdverbialTerm(term) => match term {
            NoihaAdverbialTermSyntax::NoihaVariableAdverbialTerm(term) => {
                term.free_modifiers
                    .iter()
                    .any(|free_modifier| generated_node_contains_cmavo(free_modifier, Cmavo::Keha))
                    || generated_selbri_contains_current_level_keha(&term.selbri)
            }
            NoihaAdverbialTermSyntax::NoihaRelativeAdverbialTerm(term) => {
                generated_selbri_contains_current_level_keha(&term.selbri)
            }
        },
        SimpleTermSyntax::FihoiAdverbialTerm(term) => {
            generated_statement_contains_current_level_keha(&term.statement)
        }
        SimpleTermSyntax::SoiAdverbialTerm(term) => {
            generated_statement_contains_current_level_keha(&term.statement)
        }
        SimpleTermSyntax::ForethoughtTermset(termset) => {
            termset
                .terms
                .iter()
                .any(|term| generated_term_contains_current_level_keha(term))
                || termset
                    .first_branch
                    .terms
                    .iter()
                    .any(|term| generated_term_contains_current_level_keha(term))
                || termset.additional_branches.iter().any(|branch| {
                    branch
                        .terms
                        .iter()
                        .any(|term| generated_term_contains_current_level_keha(term))
                })
        }
        SimpleTermSyntax::NuhiTermset(termset) => termset
            .termset
            .iter()
            .any(|term| generated_term_contains_current_level_keha(term)),
        SimpleTermSyntax::KeTermset(termset) => termset
            .termset
            .iter()
            .any(|term| generated_term_contains_current_level_keha(term)),
        SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | SimpleTermSyntax::NaKuTerm(_)
        | SimpleTermSyntax::BareNaTerm(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_contains_current_level_keha(sumti: &SumtiSyntax) -> bool {
    generated_sumti_grouped_contains_current_level_keha(&sumti.base_sumti)
        || sumti
            .vuho_attachment
            .as_ref()
            .is_some_and(|attachment| match attachment {
                VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(_) => false,
                VuhoSumtiAttachmentTailSyntax::VuhoConnectedSumtiAttachmentTail(tail) => {
                    generated_sumti_connection_tail_contains_current_level_keha(
                        &tail.sumti_connection,
                    )
                }
            })
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_grouped_contains_current_level_keha(sumti: &SumtiGroupedSyntax) -> bool {
    generated_sumti_afterthought_contains_current_level_keha(&sumti.leading_sumti)
        || sumti
            .grouped_tail
            .as_ref()
            .is_some_and(|tail| generated_sumti_contains_current_level_keha(&tail.inner_sumti))
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_contains_current_level_keha(
    sumti: &SumtiAfterthoughtSyntax,
) -> bool {
    generated_sumti_bound_contains_current_level_keha(&sumti.leading_sumti)
        || sumti.continuations.iter().any(|continuation| {
            generated_sumti_bound_contains_current_level_keha(&continuation.sumti)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_contains_current_level_keha(sumti: &SumtiBoundSyntax) -> bool {
    generated_sumti_forethought_contains_current_level_keha(&sumti.leading_sumti)
        || sumti.bound_tail.as_ref().is_some_and(|tail| {
            generated_sumti_bound_contains_current_level_keha(&tail.trailing_sumti)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_forethought_contains_current_level_keha(sumti: &SumtiForethoughtSyntax) -> bool {
    match sumti {
        SumtiForethoughtSyntax::ForethoughtSumti(sumti) => {
            generated_sumti_contains_current_level_keha(&sumti.leading_sumti)
                || generated_sumti_forethought_contains_current_level_keha(
                    &sumti.first_branch.sumti,
                )
                || sumti.additional_branches.iter().any(|branch| {
                    generated_sumti_forethought_contains_current_level_keha(&branch.sumti)
                })
        }
        SumtiForethoughtSyntax::SimpleSumti(sumti) => {
            generated_simple_sumti_contains_current_level_keha(sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_contains_current_level_keha(sumti: &SimpleSumtiSyntax) -> bool {
    generated_sumti_atom_contains_current_level_keha(&sumti.base_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_atom_contains_current_level_keha(sumti: &SumtiAtomSyntax) -> bool {
    match sumti {
        SumtiAtomSyntax::SumtiBase(sumti) => {
            generated_sumti_base_contains_current_level_keha(sumti)
        }
        SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_sumti_base_contains_current_level_keha(&sumti.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_contains_current_level_keha(sumti: &SumtiBaseSyntax) -> bool {
    match sumti {
        SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
            generated_sumti_contains_current_level_keha(&sumti.inner_sumti)
        }
        SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
            generated_sumti_contains_current_level_keha(&sumti.inner_sumti)
        }
        SumtiBaseSyntax::LaheSumti(sumti) => {
            generated_sumti_contains_current_level_keha(&sumti.inner_sumti)
        }
        SumtiBaseSyntax::LaheTermWrapper(term) => {
            generated_term_contains_current_level_keha(&term.inner_term)
        }
        SumtiBaseSyntax::ScalarNegatedTermWrapperWithBo(term) => {
            generated_term_contains_current_level_keha(&term.inner_term)
        }
        SumtiBaseSyntax::ScalarNegatedTermWrapper(term) => {
            generated_term_contains_current_level_keha(&term.inner_term)
        }
        SumtiBaseSyntax::ProSumti(pro_sumti) => pro_sumti.0.value.cmavo() == Some(Cmavo::Keha),
        SumtiBaseSyntax::BridiDescriptionSumti(sumti) => {
            generated_statement_contains_current_level_keha(&sumti.statement)
        }
        SumtiBaseSyntax::NameSumti(_)
        | SumtiBaseSyntax::DescriptionConnectionSumti(_)
        | SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(_)
        | SumtiBaseSyntax::DescriptorWithGadriSumti(_)
        | SumtiBaseSyntax::DescriptorWithoutGadriSumti(_)
        | SumtiBaseSyntax::NumberSumti(_)
        | SumtiBaseSyntax::LerfuStringSumti(_)
        | SumtiBaseSyntax::QuotedSumti(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_connection_tail_contains_current_level_keha(
    tail: &SumtiConnectionTailSyntax,
) -> bool {
    generated_sumti_contains_current_level_keha(&tail.sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_selbri_contains_current_level_keha(selbri: &SelbriSyntax) -> bool {
    match selbri {
        SelbriSyntax::TaggedSelbri(selbri) => {
            generated_untagged_selbri_contains_current_level_keha(&selbri.inner_selbri)
        }
        SelbriSyntax::UntaggedSelbri(selbri) => {
            generated_untagged_selbri_contains_current_level_keha(selbri)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_untagged_selbri_contains_current_level_keha(selbri: &UntaggedSelbriSyntax) -> bool {
    match selbri {
        UntaggedSelbriSyntax::NegatedSelbri(selbri) => {
            generated_selbri_contains_current_level_keha(&selbri.inner_selbri)
        }
        UntaggedSelbriSyntax::CoSelbri(selbri) => {
            generated_connected_selbri_contains_current_level_keha(&selbri.leading_selbri)
                || selbri.co_tail.as_ref().is_some_and(|tail| {
                    generated_co_selbri_contains_current_level_keha(&tail.trailing_selbri)
                })
        }
        UntaggedSelbriSyntax::ForethoughtSelbriConnection(connection) => {
            generated_selbri_contains_current_level_keha(&connection.leading_selbri)
                || generated_selbri_contains_current_level_keha(&connection.first_branch.selbri)
                || connection
                    .additional_branches
                    .iter()
                    .any(|branch| generated_selbri_contains_current_level_keha(&branch.selbri))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_co_selbri_contains_current_level_keha(selbri: &CoSelbriSyntax) -> bool {
    generated_connected_selbri_contains_current_level_keha(&selbri.leading_selbri)
        || selbri.co_tail.as_ref().is_some_and(|tail| {
            generated_co_selbri_contains_current_level_keha(&tail.trailing_selbri)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_connected_selbri_contains_current_level_keha(selbri: &ConnectedSelbriSyntax) -> bool {
    generated_tanru_selbri_contains_current_level_keha(&selbri.leading_selbri)
        || selbri.continuations.iter().any(|continuation| {
            generated_tanru_selbri_contains_current_level_keha(&continuation.trailing_selbri)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_selbri_contains_current_level_keha(selbri: &TanruSelbriSyntax) -> bool {
    generated_tanru_unit_contains_current_level_keha(&selbri.first_unit)
        || selbri
            .additional_units
            .iter()
            .any(generated_tanru_unit_contains_current_level_keha)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_contains_current_level_keha(unit: &TanruUnitSyntax) -> bool {
    generated_bo_or_linked_tanru_unit_contains_current_level_keha(&unit.0.first)
        || unit.0.links.iter().any(|link| {
            generated_bo_or_linked_tanru_unit_contains_current_level_keha(&link.trailing_unit)
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_bo_or_linked_tanru_unit_contains_current_level_keha(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> bool {
    match unit {
        BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => {
            generated_selbri_contains_current_level_keha(&unit.leading_selbri)
                || generated_bo_or_linked_tanru_unit_contains_current_level_keha(
                    &unit.first_branch.unit,
                )
                || unit.additional_branches.iter().any(|branch| {
                    generated_bo_or_linked_tanru_unit_contains_current_level_keha(&branch.unit)
                })
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
            generated_linked_tanru_unit_contains_current_level_keha(&unit.leading_unit)
                || generated_bo_or_linked_tanru_unit_contains_current_level_keha(
                    &unit.trailing_unit,
                )
        }
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            generated_linked_tanru_unit_for_cei_contains_current_level_keha(&unit.base)
                || unit.assignments.iter().any(|assignment| {
                    generated_linked_tanru_unit_for_cei_contains_current_level_keha(
                        &assignment.tanru_unit,
                    )
                })
        }
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            generated_linked_tanru_unit_contains_current_level_keha(unit)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_linked_tanru_unit_contains_current_level_keha(unit: &LinkedTanruUnitSyntax) -> bool {
    generated_tanru_unit_atom_contains_current_level_keha(&unit.base)
        || unit
            .linkargs
            .as_ref()
            .is_some_and(|linkargs| generated_linkargs_contains_current_level_keha(linkargs))
}

#[requires(true)]
#[ensures(true)]
fn generated_linked_tanru_unit_for_cei_contains_current_level_keha(
    unit: &LinkedTanruUnitForCeiSyntax,
) -> bool {
    generated_tanru_unit_atom_for_cei_contains_current_level_keha(&unit.base)
        || unit
            .linkargs
            .as_ref()
            .is_some_and(|linkargs| generated_linkargs_contains_current_level_keha(linkargs))
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_contains_current_level_keha(unit: &TanruUnitAtomSyntax) -> bool {
    generated_tanru_unit_atom_base_contains_current_level_keha(&unit.base)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_for_cei_contains_current_level_keha(
    unit: &TanruUnitAtomForCeiSyntax,
) -> bool {
    generated_tanru_unit_atom_base_for_cei_contains_current_level_keha(&unit.base)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_base_contains_current_level_keha(
    unit: &TanruUnitAtomBaseSyntax,
) -> bool {
    match unit {
        TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit) => {
            generated_linkargs_contains_current_level_keha(&unit.linkargs)
                || generated_tanru_unit_contains_current_level_keha(&unit.base)
        }
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) => {
            generated_jai_inner_tanru_unit_contains_current_level_keha(&unit.inner_unit)
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_scalar_negated_tanru_inner_unit_contains_current_level_keha(&unit.inner_unit)
        }
        TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(unit) => {
            generated_sumti_selbri_sumti_contains_current_level_keha(&unit.sumti)
        }
        TanruUnitAtomBaseSyntax::ZantufaStatementAbstractionTanruUnit(unit) => {
            generated_statement_contains_current_level_keha(&unit.statement)
        }
        TanruUnitAtomBaseSyntax::ZantufaMeTanruUnit(unit) => {
            generated_zantufa_me_tanru_unit_contains_current_level_keha(unit)
        }
        TanruUnitAtomBaseSyntax::ZantufaMexMoiTanruUnit(unit) => {
            generated_node_contains_cmavo(unit.expression.as_ref(), Cmavo::Keha)
        }
        TanruUnitAtomBaseSyntax::GroupedTanruUnit(unit) => {
            generated_connected_selbri_contains_current_level_keha(&unit.selbri)
        }
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            word.value.cmavo() == Some(Cmavo::Keha)
        }
        TanruUnitAtomBaseSyntax::ProBridiTanruUnit(_)
        | TanruUnitAtomBaseSyntax::AbstractionTanruUnit(_)
        | TanruUnitAtomBaseSyntax::OrdinalTanruUnit(_)
        | TanruUnitAtomBaseSyntax::OperatorSelbriTanruUnit(_)
        | TanruUnitAtomBaseSyntax::QuotedBridiSelbriTanruUnit(_)
        | TanruUnitAtomBaseSyntax::QuotedTextSelbriTanruUnit(_)
        | TanruUnitAtomBaseSyntax::TextSelbriTanruUnit(_)
        | TanruUnitAtomBaseSyntax::TagSelbriTanruUnit(_)
        | TanruUnitAtomBaseSyntax::GohaWordTanruUnit(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_base_for_cei_contains_current_level_keha(
    unit: &TanruUnitAtomBaseForCeiSyntax,
) -> bool {
    match unit {
        TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit) => {
            generated_linkargs_contains_current_level_keha(&unit.linkargs)
                || generated_tanru_unit_contains_current_level_keha(&unit.base)
        }
        TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit) => {
            generated_jai_inner_tanru_unit_contains_current_level_keha(&unit.inner_unit)
        }
        TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_scalar_negated_tanru_inner_unit_contains_current_level_keha(&unit.inner_unit)
        }
        TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(unit) => {
            generated_sumti_selbri_sumti_contains_current_level_keha(&unit.sumti)
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaStatementAbstractionTanruUnit(unit) => {
            generated_statement_contains_current_level_keha(&unit.statement)
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaMeTanruUnit(unit) => {
            generated_zantufa_me_tanru_unit_contains_current_level_keha(unit)
        }
        TanruUnitAtomBaseForCeiSyntax::ZantufaMexMoiTanruUnit(unit) => {
            generated_node_contains_cmavo(unit.expression.as_ref(), Cmavo::Keha)
        }
        TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(unit) => {
            generated_connected_selbri_contains_current_level_keha(&unit.selbri)
        }
        TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            word.value.cmavo() == Some(Cmavo::Keha)
        }
        TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::AbstractionTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::OrdinalTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::OperatorSelbriTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::QuotedBridiSelbriTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::QuotedTextSelbriTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::TextSelbriTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::TagSelbriTanruUnit(_)
        | TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_zantufa_me_tanru_unit_contains_current_level_keha(
    unit: &ZantufaMeTanruUnitSyntax,
) -> bool {
    match unit.body.as_ref() {
        ZantufaMeSelbriBodySyntax::ZantufaMeMeksoSelbriBody(body) => {
            generated_node_contains_cmavo(body.0.as_ref(), Cmavo::Keha)
        }
        ZantufaMeSelbriBodySyntax::ZantufaMeTagSelbriBody(body) => {
            generated_node_contains_cmavo(body.0.as_ref(), Cmavo::Keha)
        }
        ZantufaMeSelbriBodySyntax::ZantufaMeOperatorSelbriBody(body) => body
            .0
            .iter()
            .any(|operator| generated_node_contains_cmavo(operator, Cmavo::Keha)),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_scalar_negated_tanru_inner_unit_contains_current_level_keha(
    unit: &ScalarNegatedTanruInnerUnitSyntax,
) -> bool {
    match unit {
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(unit) => {
            generated_connected_selbri_contains_current_level_keha(&unit.inner_selbri)
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(_) => false,
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(unit) => {
            generated_tanru_unit_atom_contains_current_level_keha(unit)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_jai_inner_tanru_unit_contains_current_level_keha(
    unit: &JaiInnerTanruUnitSyntax,
) -> bool {
    match unit {
        JaiInnerTanruUnitSyntax::ConvertedJaiInnerTanruUnit(unit) => {
            generated_jai_inner_tanru_unit_contains_current_level_keha(&unit.inner_unit)
        }
        JaiInnerTanruUnitSyntax::ScalarNegatedJaiInnerTanruUnit(unit) => {
            generated_jai_inner_tanru_unit_contains_current_level_keha(&unit.inner_unit)
        }
        JaiInnerTanruUnitSyntax::SumtiSelbriTanruUnit(unit) => {
            generated_sumti_selbri_sumti_contains_current_level_keha(&unit.sumti)
        }
        JaiInnerTanruUnitSyntax::GroupedJaiInnerTanruUnit(unit) => {
            generated_connected_jai_inner_selbri_contains_current_level_keha(&unit.selbri)
        }
        JaiInnerTanruUnitSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            word.value.cmavo() == Some(Cmavo::Keha)
        }
        JaiInnerTanruUnitSyntax::QuotedBridiSelbriTanruUnit(_)
        | JaiInnerTanruUnitSyntax::QuotedTextSelbriTanruUnit(_)
        | JaiInnerTanruUnitSyntax::TextSelbriTanruUnit(_)
        | JaiInnerTanruUnitSyntax::OrdinalTanruUnit(_)
        | JaiInnerTanruUnitSyntax::OperatorSelbriTanruUnit(_)
        | JaiInnerTanruUnitSyntax::ProBridiTanruUnit(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_connected_jai_inner_selbri_contains_current_level_keha(
    selbri: &ConnectedJaiInnerSelbriSyntax,
) -> bool {
    generated_tanru_jai_inner_selbri_contains_current_level_keha(&selbri.leading_selbri)
        || selbri.continuations.iter().any(|continuation| {
            generated_tanru_jai_inner_selbri_contains_current_level_keha(
                &continuation.trailing_selbri,
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_jai_inner_selbri_contains_current_level_keha(
    selbri: &TanruJaiInnerSelbriSyntax,
) -> bool {
    generated_jai_inner_tanru_unit_contains_current_level_keha(&selbri.first_unit)
        || selbri
            .additional_units
            .iter()
            .any(generated_jai_inner_tanru_unit_contains_current_level_keha)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_selbri_sumti_contains_current_level_keha(
    sumti: &SumtiSelbriSumtiSyntax,
) -> bool {
    match sumti {
        SumtiSelbriSumtiSyntax::Sumti(sumti) => generated_sumti_contains_current_level_keha(sumti),
        SumtiSelbriSumtiSyntax::MeLerfuSumti(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_linkargs_contains_current_level_keha(linkargs: &LinkargsSyntax) -> bool {
    generated_linked_sumti_contains_current_level_keha(&linkargs.first_link)
        || linkargs
            .bei_links
            .iter()
            .any(|link| generated_linked_sumti_contains_current_level_keha(&link.link))
}

#[requires(true)]
#[ensures(true)]
fn generated_linked_sumti_contains_current_level_keha(sumti: &LinkedSumtiSyntax) -> bool {
    match sumti {
        LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => match sumti.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_sumti_contains_current_level_keha(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        LinkedSumtiSyntax::TenseTaggedLinkedSumti(sumti) => match sumti.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                generated_sumti_contains_current_level_keha(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => false,
        },
        LinkedSumtiSyntax::PlainLinkedSumti(PlainLinkedSumtiSyntax(sumti)) => {
            generated_sumti_contains_current_level_keha(sumti)
        }
        LinkedSumtiSyntax::EmptyLinkedSumti(_) => false,
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

#[requires(place.get() > 0)]
#[ensures(ret > 0)]
fn argument_place_index(place: &PlaceIndex) -> usize {
    place.get()
}

#[requires(true)]
#[ensures(true)]
fn generated_untagged_selbri_has_formula_scope(selbri: &UntaggedSelbriSyntax) -> bool {
    match selbri {
        UntaggedSelbriSyntax::NegatedSelbri(_) => true,
        UntaggedSelbriSyntax::CoSelbri(_) => false,
        UntaggedSelbriSyntax::ForethoughtSelbriConnection(_) => true,
    }
}

#[requires(true)]
#[ensures(matches!(ret, FormulaOperator::Affirmed | FormulaOperator::Not))]
fn generated_bridi_negation_operator<F>(na: &WithFreeModifiers<Token, F>) -> FormulaOperator {
    if na.value.cmavo() == Some(Cmavo::Jaha) {
        FormulaOperator::Affirmed
    } else {
        FormulaOperator::Not
    }
}

#[requires(matches!(operator, FormulaOperator::Affirmed | FormulaOperator::Not))]
#[ensures(!ret.is_empty())]
fn bridi_negation_source_construct(operator: FormulaOperator) -> &'static str {
    match operator {
        FormulaOperator::Affirmed => "bridi-affirmation",
        FormulaOperator::Not => "bridi-negation",
        _ => unreachable!("precondition restricts bridi NA operators"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
fn generated_time_relation_for_tense_modal(tense_modal: &TenseModalSyntax) -> Option<String> {
    generated_tense_relation_spec_for_tense_modal(tense_modal)
        .map(|(_introduced_by, relation, _visible_place)| relation)
}

#[requires(true)]
#[ensures(ret.iter().all(|variable| matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
fn generated_prenex_formula_scope_variables(
    scopes: &[GeneratedPrenexFormulaScope],
) -> HashSet<SemanticObjectId> {
    scopes
        .iter()
        .filter_map(|scope| match scope {
            GeneratedPrenexFormulaScope::Negation { .. } => None,
            GeneratedPrenexFormulaScope::Quantifier(scope) => Some(scope.variable),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn generated_term_formula_scope_source(
    scope: &GeneratedTermFormulaScope,
) -> Option<crate::model::SemanticSource> {
    match scope {
        GeneratedTermFormulaScope::Negation { source } => source.clone(),
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_formula_option(
    value: &mut Option<SemanticObjectId>,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    if *value == Some(old_id) {
        *value = Some(new_id);
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_formula_vec(
    values: &mut [SemanticObjectId],
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    for value in values {
        if *value == old_id {
            *value = new_id;
        }
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_relative_clause_formula_references(
    clauses: &mut Vec<RelativeClause>,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    for clause in clauses {
        if clause.body == old_id {
            *clause = clause.clone().with_data(data! { body: new_id });
        }
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_descriptor_formula_references(
    descriptor: &mut Option<Descriptor>,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    if let Some(value) = descriptor.take() {
        let mut data = value.into_data();
        replace_generated_formula_option(&mut data.body, old_id, new_id);
        replace_generated_relative_clause_formula_references(
            &mut data.relative_clauses,
            old_id,
            new_id,
        );
        *descriptor = Some(Descriptor::from_data(data));
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_predication_formula_references(
    predication: &mut PredicationNodeData,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    for argument in predication.arguments.values_mut() {
        replace_generated_argument_value_formula_references(argument, old_id, new_id);
    }
    for question in &mut predication.place_questions {
        let mut argument = question.argument.clone();
        replace_generated_argument_value_formula_references(&mut argument, old_id, new_id);
        if argument != question.argument {
            *question = question.clone().with_data(data! { argument: argument });
        }
    }
    for modal_argument in &mut predication.modal_arguments {
        let mut arguments = modal_argument.arguments.clone();
        for argument in arguments.values_mut() {
            replace_generated_argument_value_formula_references(argument, old_id, new_id);
        }
        let mut body = modal_argument.body;
        replace_generated_formula_option(&mut body, old_id, new_id);
        if arguments != modal_argument.arguments || body != modal_argument.body {
            *modal_argument = modal_argument.clone().with_data(data! {
                arguments: arguments,
                body: body,
            });
        }
    }
    for exchange in &mut predication.reciprocity {
        let mut left = exchange.left.clone();
        let mut right = exchange.right.clone();
        replace_generated_argument_value_formula_references(&mut left, old_id, new_id);
        replace_generated_argument_value_formula_references(&mut right, old_id, new_id);
        if left != exchange.left || right != exchange.right {
            *exchange = exchange
                .clone()
                .with_data(data! { left: left, right: right });
        }
    }
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_formula_node_references(
    formula: FormulaNode,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) -> FormulaNode {
    match formula.into_data() {
        data!(FormulaNode::Atom(node)) => new!(FormulaNode::Atom(node)),
        data!(FormulaNode::Connective(node)) => {
            let mut data = node.into_data();
            replace_generated_formula_vec(&mut data.children, old_id, new_id);
            new!(FormulaNode::Connective(
                crate::model::ConnectiveFormulaNode::from_data(data)
            ))
        }
        data!(FormulaNode::Quantified(node)) => {
            let mut data = node.into_data();
            replace_generated_formula_option(&mut data.restriction, old_id, new_id);
            if data.body == old_id {
                data.body = new_id;
            }
            new!(FormulaNode::Quantified(
                crate::model::QuantifiedFormulaNode::from_data(data)
            ))
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            let mut data = node.into_data();
            if data.body == old_id {
                data.body = new_id;
            }
            for binding in &mut data.bindings {
                let mut restriction = binding.restriction;
                replace_generated_formula_option(&mut restriction, old_id, new_id);
                if restriction != binding.restriction {
                    *binding = binding
                        .clone()
                        .with_data(data! { restriction: restriction });
                }
            }
            new!(FormulaNode::QuantifierBundle(
                QuantifierBundleFormulaNode::from_data(data)
            ))
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let mut data = node.into_data();
            if data.body == old_id {
                data.body = new_id;
            }
            new!(FormulaNode::RespectivelyDistribution(
                crate::model::RespectivelyDistributionFormulaNode::from_data(data)
            ))
        }
    }
}

#[requires(true)]
#[ensures(ret.relative_clauses.len() >= old(argument.relative_clauses.len()))]
fn append_generated_relative_clauses_to_argument(
    argument: ArgumentValue,
    relative_clauses: Vec<RelativeClause>,
) -> ArgumentValue {
    if relative_clauses.is_empty() {
        return argument;
    }
    if argument.relative_clauses.is_empty() {
        return argument.with_relative_clauses(relative_clauses);
    }
    let data = argument.into_data();
    let mut all_relative_clauses = data.relative_clauses;
    all_relative_clauses.extend(relative_clauses);
    ArgumentValue::from_data(data!(ArgumentValue {
        relative_clauses: all_relative_clauses,
        ..data
    }))
}

#[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[ensures(true)]
fn replace_generated_argument_value_formula_references(
    argument: &mut ArgumentValue,
    old_id: SemanticObjectId,
    new_id: SemanticObjectId,
) {
    let mut relative_clauses = argument.relative_clauses.clone();
    replace_generated_relative_clause_formula_references(&mut relative_clauses, old_id, new_id);
    if relative_clauses != argument.relative_clauses {
        *argument = argument.clone().with_data(data! {
            relative_clauses: relative_clauses,
        });
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_prenex_scope_binding_key(
    scope: &GeneratedArgumentQuantifierScope<'_>,
) -> Option<String> {
    match scope.source {
        GeneratedArgumentQuantifierSource::QuantifiedSumti(quantified) => {
            generated_pro_sumti_binding_key_from_sumti_base(&quantified.inner_sumti)
        }
        GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(_)
        | GeneratedArgumentQuantifierSource::NoGadriDescription(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_prenex_binding_sumti_for_term(
    term: &TermSyntax,
) -> Result<Option<&SumtiSyntax>, SemanticsError> {
    let simple = generated_simple_term_for_assignment(term)?;
    match simple {
        SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => Ok(Some(sumti)),
        SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => Ok(Some(sumti)),
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => Ok(None),
        },
        SimpleTermSyntax::TaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => Ok(Some(sumti)),
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => Ok(None),
        },
        _ => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_prenex_binding_key_for_sumti(sumti: &SumtiSyntax) -> Option<String> {
    generated_quantified_sumti_from_sumti(sumti).and_then(|quantified| {
        generated_pro_sumti_binding_key_from_sumti_base(&quantified.inner_sumti)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_prenex_binding_pro_sumti_for_sumti(sumti: &SumtiSyntax) -> Option<&ProSumtiSyntax> {
    generated_prenex_da_series_pro_sumti_from_sumti(sumti)
        .or_else(|| {
            generated_quantified_sumti_from_sumti(sumti)
                .and_then(|quantified| generated_pro_sumti_from_sumti_base(&quantified.inner_sumti))
        })
        .filter(|pro_sumti| {
            pro_sumti
                .0
                .value
                .cmavo()
                .is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))
        })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_prenex_da_series_pro_sumti_from_sumti(sumti: &SumtiSyntax) -> Option<&ProSumtiSyntax> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::ProSumti(pro_sumti)) =
        simple.base_sumti.as_ref()
    else {
        return None;
    };
    Some(pro_sumti).filter(|pro_sumti| {
        pro_sumti
            .0
            .value
            .cmavo()
            .is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))))]
fn generated_bare_da_series_pro_sumti_from_sumti(sumti: &SumtiSyntax) -> Option<&ProSumtiSyntax> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    if simple.relative_clauses.is_some() {
        return None;
    }
    let SumtiAtomSyntax::SumtiBase(SumtiBaseSyntax::ProSumti(pro_sumti)) =
        simple.base_sumti.as_ref()
    else {
        return None;
    };
    Some(pro_sumti).filter(|pro_sumti| {
        pro_sumti
            .0
            .value
            .cmavo()
            .is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_pro_sumti_binding_key_from_sumti_base(sumti: &SumtiBaseSyntax) -> Option<String> {
    match sumti {
        SumtiBaseSyntax::ProSumti(pro_sumti) => Some(token_text(&pro_sumti.0.value)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|pro_sumti| pro_sumti.0.value.cmavo().is_some()))]
fn generated_pro_sumti_from_sumti_base(sumti: &SumtiBaseSyntax) -> Option<&ProSumtiSyntax> {
    match sumti {
        SumtiBaseSyntax::ProSumti(pro_sumti) => Some(pro_sumti),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_domain, relation)| !relation.relation.is_empty()))]
fn generated_anchor_relation_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(GeneratedAnchorDomain, AnchorRelation)> {
    generated_anchor_relation_with_introducer_for_tense_modal(tense_modal)
        .map(|(domain, relation, _introduced_by)| (domain, relation))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_domain, relation, introduced_by)| !relation.relation.is_empty() && !introduced_by.is_empty()))]
fn generated_anchor_relation_with_introducer_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(GeneratedAnchorDomain, AnchorRelation, String)> {
    generated_anchor_relations_with_introducers_for_tense_modal(tense_modal)
        .into_iter()
        .next()
}

#[requires(true)]
#[ensures(ret.iter().all(|(_domain, relation, introduced_by)| !relation.relation.is_empty() && !introduced_by.is_empty()))]
fn generated_anchor_relations_with_introducers_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Vec<(GeneratedAnchorDomain, AnchorRelation, String)> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    generated_anchor_relations_with_introducers_for_tokens(collector.tokens)
}

#[requires(true)]
#[ensures(ret.iter().all(|(_domain, relation, introduced_by)| !relation.relation.is_empty() && !introduced_by.is_empty()))]
fn generated_anchor_relations_with_introducers_for_tokens(
    tokens: Vec<&Token>,
) -> Vec<(GeneratedAnchorDomain, AnchorRelation, String)> {
    let mut relations = Vec::new();
    let mut previous_relation_accepts_distance = None::<GeneratedAnchorDomain>;
    let mut interval_accepts_direction = false;
    let mut pending_scalar_negation = None::<ScalarNegation>;
    let mut pending_motion = None::<String>;
    for token in tokens {
        if token.cmavo() == Some(Cmavo::Mohi) {
            previous_relation_accepts_distance = None;
            interval_accepts_direction = false;
            pending_motion = Some(token_text(token));
            continue;
        }
        if matches!(
            token.cmavo(),
            Some(Cmavo::Nahe | Cmavo::Tohe | Cmavo::Nohe | Cmavo::Jeha)
        ) {
            pending_scalar_negation = Some(scalar_negation_for_token(token));
            previous_relation_accepts_distance = None;
            pending_motion = None;
            continue;
        }
        if space_interval_part_accepts_direction(token) {
            interval_accepts_direction = true;
            previous_relation_accepts_distance = None;
            pending_motion = None;
            continue;
        }
        if interval_accepts_direction && space_interval_direction_for_faha_token(token).is_some() {
            interval_accepts_direction = false;
            previous_relation_accepts_distance = None;
            pending_motion = None;
            continue;
        }
        interval_accepts_direction = false;
        if let Some(relation) = time_relation_for_pu_token(token) {
            relations.push((
                GeneratedAnchorDomain::Time,
                new!(AnchorRelation {
                    relation,
                    anchor: SemanticObjectId::now(),
                    sticky: false,
                    inherited: None,
                    distance: None,
                    magnitude: None,
                    scalar_negation: pending_scalar_negation.take(),
                    motion: None,
                }),
                token_text(token),
            ));
            previous_relation_accepts_distance = Some(GeneratedAnchorDomain::Time);
            pending_motion = None;
            continue;
        }
        if let Some(distance) = time_distance_for_zi_token(token) {
            if previous_relation_accepts_distance == Some(GeneratedAnchorDomain::Time)
                && let Some((GeneratedAnchorDomain::Time, relation, _introduced_by)) =
                    relations.last_mut()
                && relation.distance.is_none()
            {
                *relation = relation
                    .clone()
                    .with_data(data! { distance: Some(distance) });
                previous_relation_accepts_distance = None;
                continue;
            }
            if let Some(relation) = time_relation_for_time_distance_token(token) {
                relations.push((
                    GeneratedAnchorDomain::Time,
                    new!(AnchorRelation {
                        relation,
                        anchor: SemanticObjectId::now(),
                        sticky: false,
                        inherited: None,
                        distance: None,
                        magnitude: None,
                        scalar_negation: pending_scalar_negation.take(),
                        motion: None,
                    }),
                    token_text(token),
                ));
            }
            previous_relation_accepts_distance = None;
            pending_motion = None;
            continue;
        }
        if let Some(relation) = space_relation_for_faha_token(token) {
            let motion = pending_motion
                .take()
                .map(|introduced_by| SpatialMotion::new(SpatialMotionKind::Toward, introduced_by));
            relations.push((
                GeneratedAnchorDomain::Space,
                new!(AnchorRelation {
                    relation,
                    anchor: SemanticObjectId::here(),
                    sticky: false,
                    inherited: None,
                    distance: None,
                    magnitude: None,
                    scalar_negation: pending_scalar_negation.take(),
                    motion,
                }),
                token_text(token),
            ));
            previous_relation_accepts_distance = Some(GeneratedAnchorDomain::Space);
            continue;
        }
        if let Some(distance) = space_distance_for_va_token(token) {
            if previous_relation_accepts_distance == Some(GeneratedAnchorDomain::Space)
                && let Some((GeneratedAnchorDomain::Space, relation, _introduced_by)) =
                    relations.last_mut()
                && relation.distance.is_none()
            {
                *relation = relation
                    .clone()
                    .with_data(data! { distance: Some(distance) });
                previous_relation_accepts_distance = None;
                continue;
            }
            relations.push((
                GeneratedAnchorDomain::Space,
                new!(AnchorRelation {
                    relation: "distanceFrom".to_owned(),
                    anchor: SemanticObjectId::here(),
                    sticky: false,
                    inherited: None,
                    distance: Some(distance),
                    magnitude: None,
                    scalar_negation: pending_scalar_negation.take(),
                    motion: None,
                }),
                token_text(token),
            ));
            previous_relation_accepts_distance = None;
            pending_motion = None;
            continue;
        }
        previous_relation_accepts_distance = None;
        pending_motion = None;
    }
    relations
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|span| !span.introduced_by.is_empty()))]
fn generated_time_span_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeSpan> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let tokens = collector.tokens;
    let connector_index = tokens
        .iter()
        .position(|token| matches!(token.cmavo(), Some(Cmavo::Bihi | Cmavo::Biho)))?;
    let connector = tokens.get(connector_index)?;
    let start_tokens = tokens[..connector_index].to_vec();
    let end_tokens = tokens[connector_index + 1..].to_vec();
    if start_tokens.is_empty() || end_tokens.is_empty() {
        return None;
    }
    let anchor = anchor.or(Some(SemanticObjectId::now()));
    Some(TimeSpan::new(
        generated_time_span_endpoint_from_tokens(start_tokens, anchor)?,
        generated_time_span_endpoint_from_tokens(end_tokens, anchor)?,
        token_text(connector),
    ))
}

#[requires(!tokens.is_empty())]
#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|endpoint| !endpoint.relation.is_empty()))]
fn generated_time_span_endpoint_from_tokens(
    tokens: Vec<&Token>,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeSpanEndpoint> {
    let mut relations = generated_anchor_relations_with_introducers_for_tokens(tokens)
        .into_iter()
        .filter(|(domain, _, _)| *domain == GeneratedAnchorDomain::Time)
        .collect::<Vec<_>>();
    if relations.len() != 1 {
        return None;
    }
    let (_, relation, introduced_by) = relations.pop()?;
    let data!(AnchorRelation {
        relation,
        distance,
        scalar_negation,
        ..
    }) = relation.into_data();
    Some(TimeSpanEndpoint::new(
        relation,
        anchor,
        introduced_by,
        distance,
        scalar_negation,
    ))
}
#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn update_generated_eventuality_data(
    event: &mut SemanticObject,
    update: impl FnOnce(&mut EventualityNodeData),
) {
    event.update_eventuality(|event| {
        let mut data = event.into_data();
        update(&mut data);
        EventualityNode::from_data(data)
    });
}

#[requires(!introduced_by.is_empty())]
#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn apply_generated_anchor_relation_to_event(
    event: &mut SemanticObject,
    domain: GeneratedAnchorDomain,
    relation: AnchorRelation,
    introduced_by: String,
    explicit_anchor: bool,
) {
    match domain {
        GeneratedAnchorDomain::Time => {
            apply_generated_time_relation_to_event(event, relation, introduced_by, explicit_anchor);
        }
        GeneratedAnchorDomain::Space => {
            apply_generated_space_relation_to_event(
                event,
                relation,
                introduced_by,
                explicit_anchor,
            );
        }
    }
}

#[requires(!introduced_by.is_empty())]
#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn apply_generated_time_relation_to_event(
    event: &mut SemanticObject,
    relation: AnchorRelation,
    introduced_by: String,
    explicit_anchor: bool,
) {
    update_generated_eventuality_data(event, |event| {
        if let Some(time) = event.time.take() {
            event
                .time_path
                .push(generated_temporal_path_step_from_anchor_relation(
                    time, None,
                ));
        }
        let anchor =
            (!explicit_anchor && !event.time_path.is_empty()).then(TemporalPathAnchor::previous);
        event.time_path.push(
            generated_temporal_path_step_from_anchor_relation_with_anchor(
                relation,
                Some(introduced_by),
                anchor,
            ),
        );
    });
}

#[requires(!introduced_by.is_empty())]
#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn apply_generated_space_relation_to_event(
    event: &mut SemanticObject,
    relation: AnchorRelation,
    introduced_by: String,
    explicit_anchor: bool,
) {
    update_generated_eventuality_data(event, |event| {
        if let Some(space) = event.space.take() {
            event
                .space_path
                .push(generated_temporal_path_step_from_anchor_relation(
                    space, None,
                ));
        }
        let anchor =
            (!explicit_anchor && !event.space_path.is_empty()).then(TemporalPathAnchor::previous);
        event.space_path.push(
            generated_temporal_path_step_from_anchor_relation_with_anchor(
                relation,
                Some(introduced_by),
                anchor,
            ),
        );
    });
}

#[requires(introduced_by.as_ref().is_none_or(|introduced_by| !introduced_by.is_empty()))]
#[ensures(!ret.relation.is_empty())]
fn generated_temporal_path_step_from_anchor_relation(
    relation: AnchorRelation,
    introduced_by: Option<String>,
) -> TemporalPathStep {
    generated_temporal_path_step_from_anchor_relation_with_anchor(relation, introduced_by, None)
}

#[requires(introduced_by.as_ref().is_none_or(|introduced_by| !introduced_by.is_empty()))]
#[ensures(!ret.relation.is_empty())]
fn generated_temporal_path_step_from_anchor_relation_with_anchor(
    relation: AnchorRelation,
    introduced_by: Option<String>,
    anchor_override: Option<TemporalPathAnchor>,
) -> TemporalPathStep {
    let introduced_by = introduced_by
        .unwrap_or_else(|| generated_introduced_by_for_time_relation(&relation.relation));
    let data!(AnchorRelation {
        relation,
        anchor,
        sticky,
        inherited,
        distance,
        magnitude,
        scalar_negation,
        motion,
    }) = relation.into_data();
    let anchor = anchor_override.unwrap_or_else(|| TemporalPathAnchor::object(anchor));
    let mut step = TemporalPathStep::new(
        relation,
        anchor,
        introduced_by,
        distance,
        magnitude,
        scalar_negation,
        motion,
    );
    if sticky {
        step = step.with_data(data! {
            sticky: true,
            inherited: inherited,
        });
    }
    step
}

#[requires(true)]
#[ensures(event.time.is_none() || event.time_path.is_empty())]
fn normalize_generated_event_time_path_data(event: &mut EventualityNodeData) {
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
        sticky,
        inherited,
        distance,
        magnitude,
        scalar_negation,
        motion,
    }) = step.into_data();
    if let Some(anchor) = anchor.object_id() {
        event.time = Some(new!(AnchorRelation {
            relation,
            anchor,
            sticky,
            inherited,
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

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn normalize_generated_event_time_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, normalize_generated_event_time_path_data);
}

#[requires(true)]
#[ensures(event.space.is_none() || event.space_path.is_empty())]
fn normalize_generated_event_space_path_data(event: &mut EventualityNodeData) {
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
        sticky,
        inherited,
        distance,
        magnitude,
        scalar_negation,
        motion,
    }) = step.into_data();
    if let Some(anchor) = anchor.object_id() {
        event.space = Some(new!(AnchorRelation {
            relation,
            anchor,
            sticky,
            inherited,
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

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn normalize_generated_event_space_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, normalize_generated_event_space_path_data);
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn clear_generated_event_time_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, |event| {
        event.time = None;
        event.time_path.clear();
    });
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn clear_generated_event_space_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, |event| {
        event.space = None;
        event.space_path.clear();
    });
}

#[requires(true)]
#[ensures(true)]
fn generated_anchor_relation_is_inherited(relation: &AnchorRelation) -> bool {
    relation.inherited == Some(true)
}

#[requires(true)]
#[ensures(true)]
fn generated_temporal_path_step_is_inherited(step: &TemporalPathStep) -> bool {
    step.inherited == Some(true)
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(true)]
fn generated_event_has_explicit_temporal_marker(event: &SemanticObject) -> bool {
    let event = event
        .as_eventuality()
        .expect("eventuality precondition supplies an eventuality variant");
    event
        .time
        .as_ref()
        .is_some_and(|time| !generated_anchor_relation_is_inherited(time))
        || event
            .time_path
            .iter()
            .any(|step| !generated_temporal_path_step_is_inherited(step))
        || event.time_interval.is_some()
        || event.time_span.is_some()
        || !event.recurrence.is_empty()
        || !event.interval_modifiers.is_empty()
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(true)]
fn generated_event_has_explicit_sticky_temporal_marker(event: &SemanticObject) -> bool {
    let event = event
        .as_eventuality()
        .expect("eventuality precondition supplies an eventuality variant");
    event
        .time
        .as_ref()
        .is_some_and(|time| time.sticky && !generated_anchor_relation_is_inherited(time))
        || event
            .time_path
            .iter()
            .any(|step| step.sticky && !generated_temporal_path_step_is_inherited(step))
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn clear_generated_inherited_event_time_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, |event| {
        if event
            .time
            .as_ref()
            .is_some_and(generated_anchor_relation_is_inherited)
        {
            event.time = None;
        }
        event
            .time_path
            .retain(|step| !generated_temporal_path_step_is_inherited(step));
        normalize_generated_event_time_path_data(event);
    });
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn clear_generated_inherited_event_space_path(event: &mut SemanticObject) {
    update_generated_eventuality_data(event, |event| {
        if event
            .space
            .as_ref()
            .is_some_and(generated_anchor_relation_is_inherited)
        {
            event.space = None;
        }
        event
            .space_path
            .retain(|step| !generated_temporal_path_step_is_inherited(step));
        normalize_generated_event_space_path_data(event);
    });
}

#[requires(true)]
#[ensures(ret.iter().all(|step| step.sticky && step.inherited == Some(true)))]
fn generated_inherited_temporal_path(path: &[TemporalPathStep]) -> Vec<TemporalPathStep> {
    path.iter()
        .cloned()
        .map(|step| mark_generated_temporal_path_step_sticky(step, Some(true)))
        .collect()
}

#[requires(true)]
#[ensures(ret.sticky)]
fn mark_generated_anchor_relation_sticky(
    relation: AnchorRelation,
    inherited: Option<bool>,
) -> AnchorRelation {
    relation.with_data(data! {
        sticky: true,
        inherited: inherited,
    })
}

#[requires(true)]
#[ensures(ret.sticky)]
fn mark_generated_temporal_path_step_sticky(
    step: TemporalPathStep,
    inherited: Option<bool>,
) -> TemporalPathStep {
    step.with_data(data! {
        sticky: true,
        inherited: inherited,
    })
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn mark_generated_event_time_sticky(event: &mut SemanticObject, inherited: Option<bool>) {
    update_generated_eventuality_data(event, |event| {
        if let Some(time) = event.time.take() {
            event.time = Some(mark_generated_anchor_relation_sticky(time, inherited));
        }
        event.time_path = event
            .time_path
            .drain(..)
            .map(|step| mark_generated_temporal_path_step_sticky(step, inherited))
            .collect();
    });
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(generated_semantic_object_is_eventuality(event))]
fn mark_generated_event_space_sticky(event: &mut SemanticObject, inherited: Option<bool>) {
    update_generated_eventuality_data(event, |event| {
        if let Some(space) = event.space.take() {
            event.space = Some(mark_generated_anchor_relation_sticky(space, inherited));
        }
        event.space_path = event
            .space_path
            .drain(..)
            .map(|step| mark_generated_temporal_path_step_sticky(step, inherited))
            .collect();
    });
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(ret.iter().all(|step| step.sticky))]
fn generated_event_time_path_for_sticky_storage(event: &SemanticObject) -> Vec<TemporalPathStep> {
    let event = event
        .as_eventuality()
        .expect("eventuality precondition supplies an eventuality variant");
    if !event.time_path.is_empty() {
        return event.time_path.clone();
    }
    event
        .time
        .iter()
        .cloned()
        .map(|time| generated_temporal_path_step_from_anchor_relation(time, None))
        .collect()
}

#[requires(generated_semantic_object_is_eventuality(event))]
#[ensures(ret.iter().all(|step| step.sticky))]
fn generated_event_space_path_for_sticky_storage(event: &SemanticObject) -> Vec<TemporalPathStep> {
    let event = event
        .as_eventuality()
        .expect("eventuality precondition supplies an eventuality variant");
    if !event.space_path.is_empty() {
        return event.space_path.clone();
    }
    event
        .space
        .iter()
        .cloned()
        .map(|space| generated_temporal_path_step_from_anchor_relation(space, None))
        .collect()
}

#[requires(!relation.is_empty())]
#[ensures(!ret.is_empty())]
fn generated_introduced_by_for_time_relation(relation: &str) -> String {
    match relation {
        "before" => "pu",
        "at" => "ca",
        "after" => "ba",
        _ => "implicit",
    }
    .to_owned()
}

#[requires(true)]
#[ensures(ret == (id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
fn semantic_id_is_eventuality(id: SemanticObjectId) -> bool {
    id.object_kind() == crate::model::SemanticObjectKind::Referent
        && id
            .referent_sort()
            .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
}

#[requires(true)]
#[ensures(ret == (object.object_kind() == crate::model::SemanticObjectKind::Referent && object.sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
fn generated_semantic_object_is_eventuality(object: &SemanticObject) -> bool {
    object.object_kind() == crate::model::SemanticObjectKind::Referent
        && object
            .sort()
            .is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))
}

#[requires(true)]
#[ensures(true)]
fn referent_qualifier_kind(cmavo: Option<Cmavo>) -> DescriptorKind {
    match cmavo {
        Some(Cmavo::Lahe) => DescriptorKind::ReferentOfSymbol,
        Some(Cmavo::Luhe) => DescriptorKind::SymbolForReferent,
        Some(Cmavo::Tuha) => DescriptorKind::AbstractionAbout,
        Some(Cmavo::Luha) => DescriptorKind::MemberOf,
        Some(Cmavo::Luhi) => DescriptorKind::SetFrom,
        Some(Cmavo::Luho) => DescriptorKind::MassFrom,
        Some(Cmavo::Vuhi) => DescriptorKind::SequenceFrom,
        _ => DescriptorKind::QualifiedSumti,
    }
}

#[requires(true)]
#[ensures(true)]
fn referent_qualifier_sort(cmavo: Option<Cmavo>) -> SemanticSort {
    match cmavo {
        Some(Cmavo::Luhe) => SemanticSort::Sign,
        Some(Cmavo::Tuha) => SemanticSort::eventuality(),
        Some(Cmavo::Luhi) => SemanticSort::Set,
        Some(Cmavo::Luho) => SemanticSort::Mass,
        Some(Cmavo::Vuhi) => SemanticSort::Sequence,
        _ => SemanticSort::Entity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ActualityKind, DomainImport, ScopeDependence, ScopeDependenceData, SemanticObjectKind,
    };
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[requires(!source.is_empty())]
    #[ensures(!ret.objects.is_empty())]
    fn semantic_graph_for(source: &str) -> SemanticGraph {
        semantic_result_for(source).expect("source should build semantics")
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
    fn semantic_result_for(source: &str) -> Result<SemanticGraph, SemanticsError> {
        semantic_result_for_with_parse_options(source, &jbotci_syntax::ParseOptions::default())
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|graph| !graph.objects.is_empty()) || ret.is_err())]
    fn semantic_result_for_with_parse_options(
        source: &str,
        parse_options: &jbotci_syntax::ParseOptions,
    ) -> Result<SemanticGraph, SemanticsError> {
        let words = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
            source,
            &jbotci_morphology::MorphologyOptions::default(),
            None,
        )
        .expect("source should segment");
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            parse_options,
        )
        .expect("source should parse");
        build_generated_semantic_graph_with_dictionary(
            &syntax,
            Some(source),
            jbotci_dictionary_data::english(),
        )
    }

    #[requires(graph.objects.values().any(|object| matches!(object.as_formula().map(FormulaNode::as_data), Some(data!(FormulaNode::Quantified(node))) if node.operator == FormulaOperator::Forall)))]
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    fn forall_variable(graph: &SemanticGraph) -> SemanticObjectId {
        graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::Quantified(node))
                    if node.operator == FormulaOperator::Forall =>
                {
                    Some(node.variable)
                }
                _ => None,
            })
            .expect("precondition guarantees a forall formula")
    }

    #[requires(graph.objects.values().any(|object| object.as_predication().is_some_and(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation))))]
    #[ensures(!ret.is_empty())]
    #[ensures(ret.iter().all(|id| graph.objects.get(id).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Constant))))]
    fn constant_argument_ids(graph: &SemanticGraph, relation: &str) -> Vec<SemanticObjectId> {
        let predication = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                    .then_some(predication)
            })
            .expect("precondition guarantees a matching predication");
        predication
            .arguments
            .values()
            .filter_map(|argument| argument.value)
            .filter(|id| {
                graph.objects.get(id).is_some_and(|object| {
                    object.referent_category() == Some(ReferentCategory::Constant)
                })
            })
            .collect()
    }

    #[requires(graph.objects.contains_key(&constant))]
    #[requires(graph.objects.get(&constant).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Constant)))]
    #[requires(binders.iter().all(|binder| graph.objects.contains_key(binder)))]
    #[ensures(graph.objects.get(&constant).is_some_and(|object| matches!(object.scope_dependence().map(ScopeDependence::as_data), Some(data!(ScopeDependence::Underspecified { may_depend_on })) if may_depend_on.iter().copied().eq(binders.iter().copied()))))]
    fn assert_underspecified_scope(
        graph: &SemanticGraph,
        constant: SemanticObjectId,
        binders: &[SemanticObjectId],
    ) {
        let object = graph.objects.get(&constant).expect("precondition checked");
        assert!(matches!(
            object.scope_dependence().map(ScopeDependence::as_data),
            Some(data!(ScopeDependence::Underspecified { may_depend_on }))
                if may_depend_on.iter().copied().eq(binders.iter().copied())
        ));
        assert_eq!(
            serde_json::to_value(object).expect("constant should serialize")["scopeDependence"],
            serde_json::json!({
                "kind": "underspecified",
                "mayDependOn": binders.iter().map(ToString::to_string).collect::<Vec<_>>(),
            })
        );
    }

    #[requires(true)]
    #[ensures(graph.objects.get(&ret).is_some_and(SemanticObject::is_generated_eventuality))]
    fn generated_event_for_relation(graph: &SemanticGraph, relation: &str) -> SemanticObjectId {
        graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                    .then_some(predication.eventuality)
                    .flatten()
            })
            .filter(|eventuality| {
                graph
                    .objects
                    .get(eventuality)
                    .is_some_and(SemanticObject::is_generated_eventuality)
            })
            .expect("precondition guarantees a generated predication eventuality")
    }

    #[requires(graph.objects.get(&eventuality).is_some_and(SemanticObject::is_generated_eventuality))]
    #[ensures(matches!(ret.object_kind(), crate::model::SemanticObjectKind::Formula | crate::model::SemanticObjectKind::Sequence))]
    fn event_binding_owner(
        graph: &SemanticGraph,
        eventuality: SemanticObjectId,
    ) -> SemanticObjectId {
        let owners = graph
            .objects
            .iter()
            .filter_map(|(&id, object)| {
                object
                    .bound_eventualities()
                    .iter()
                    .any(|bound| bound.object_id() == eventuality)
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        assert_eq!(owners.len(), 1, "generated events have exactly one owner");
        owners[0]
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.iter().all(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication))]
    fn named_predication_ids(graph: &SemanticGraph, relation: &str) -> Vec<SemanticObjectId> {
        graph
            .objects
            .iter()
            .filter_map(|(&id, object)| {
                object
                    .as_predication()
                    .is_some_and(|predication| {
                        matches!(
                            predication.relation.as_data(),
                            data!(PredicationRelation::Named { relation: candidate })
                                if candidate == relation
                        )
                    })
                    .then_some(id)
            })
            .collect()
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    fn formula_contains_predication(
        graph: &SemanticGraph,
        formula: SemanticObjectId,
        predication: SemanticObjectId,
    ) -> bool {
        let traversal = graph
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_traversal)
            .expect("formula precondition guarantees traversal");
        if traversal.predication == Some(predication) {
            return true;
        }
        traversal
            .children
            .iter()
            .copied()
            .chain(traversal.restriction)
            .chain(traversal.body)
            .any(|child| formula_contains_predication(graph, child, predication))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_cu_bridi_builds_the_relation_formula_with_elided_arguments() {
        let graph = semantic_graph_for("cu klama");
        let utterance = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .expect("bare-CU bridi is an utterance");
        assert_eq!(utterance.force, UtteranceForce::Assert);
        let formula = utterance.content.expect("bare-CU bridi has content");
        let predication = graph
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("bare-CU content is an atom");
        assert!(matches!(
            predication.relation.as_data(),
            data!(PredicationRelation::Named { relation }) if relation == "klama"
        ));
        assert_eq!(predication.arguments.len(), 5);
        assert!(
            predication
                .arguments
                .values()
                .all(|argument| argument.kind == ArgumentValueKind::Elided)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn post_cu_terms_keep_their_stream_order_before_tail_terms() {
        let graph = semantic_graph_for("mi cu do tavla ti");
        let formula = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("post-CU bridi has formula content");
        let predication = graph
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("post-CU content is an atom");
        assert!(matches!(
            predication.relation.as_data(),
            data!(PredicationRelation::Named { relation }) if relation == "tavla"
        ));
        for (place, indexical) in [
            (1, IndexicalKind::Speaker),
            (2, IndexicalKind::Audience),
            (3, IndexicalKind::ProximalDemonstrative),
        ] {
            let argument = predication
                .arguments
                .get(&argument_key(place))
                .expect("each explicit term has a place");
            assert_eq!(argument.kind, ArgumentValueKind::Filled);
            assert_eq!(
                argument
                    .value
                    .and_then(|id| graph.objects.get(&id))
                    .and_then(SemanticObject::as_referent)
                    .and_then(|referent| referent.indexical),
                Some(indexical)
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_niho_records_ordered_topic_transitions() {
        let new_topic = semantic_graph_for("ni'oni'o");
        let sequence = new_topic
            .objects
            .get(&new_topic.root)
            .and_then(SemanticObject::as_sequence)
            .expect("standalone NIhO is a discourse sequence");
        assert!(sequence.items.is_empty());
        assert!(sequence.content.is_none());
        assert!(sequence.connection_claims.is_empty());
        assert_eq!(
            sequence.relation,
            SequenceRelation::ParagraphBoundary {
                transition: ParagraphTransition::NewTopic,
                additional: vec![ParagraphTransition::NewTopic],
            }
        );

        let resume = semantic_graph_for("no'i");
        let sequence = resume
            .objects
            .get(&resume.root)
            .and_then(SemanticObject::as_sequence)
            .expect("standalone NOhI is a discourse sequence");
        assert_eq!(
            sequence.relation,
            SequenceRelation::ParagraphBoundary {
                transition: ParagraphTransition::ResumePriorTopic,
                additional: Vec::new(),
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_vau_keeps_terms_fragment_denotation() {
        let graph = semantic_graph_for("zo nalslabu vau xu");
        let utterance = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .expect("terms fragment is an utterance");
        assert_eq!(utterance.force, UtteranceForce::Mention);
        let content = utterance.content.expect("terms fragment keeps its term");
        let sign = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_sign)
            .expect("ZO term denotes a sign");
        assert_eq!(sign.sign_kind, Some(SignKind::Quotation));
        assert_eq!(
            sign.quotation
                .as_ref()
                .and_then(|quotation| quotation.text.as_deref()),
            Some("zo nalslabu")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_fragment_families_keep_typed_denotations_or_context_requirements() {
        let number = semantic_graph_for("mu");
        let number_content = number
            .objects
            .get(&number.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("number fragment has content");
        let number_referent = number
            .objects
            .get(&number_content)
            .and_then(SemanticObject::as_referent)
            .expect("number fragment denotes a referent");
        assert_eq!(number_referent.sort, SemanticSort::Number);
        assert_eq!(
            number_referent
                .descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.name.as_deref()),
            Some("mu")
        );
        assert!(
            number_referent
                .descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.quantity)
                .is_some()
        );

        let polarity = semantic_graph_for("na na");
        let polarity_content = polarity
            .objects
            .get(&polarity.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("NA answer has content");
        let polarity_sign = polarity
            .objects
            .get(&polarity_content)
            .and_then(SemanticObject::as_sign)
            .expect("NA answer denotes a connective expression");
        assert_eq!(polarity_sign.sign_kind, Some(SignKind::Connective));
        assert_eq!(polarity_sign.text.as_deref(), Some("na na"));

        let relative = semantic_graph_for("poi remna");
        let property = relative
            .objects
            .get(&relative.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .and_then(|content| relative.objects.get(&content))
            .and_then(SemanticObject::as_referent)
            .expect("relative-clause fragment denotes a property");
        assert_eq!(property.sort, SemanticSort::Relation);
        assert_eq!(property.abstraction_kind, Some(AbstractionKind::Property));
        assert_eq!(property.parameters.len(), 1);
        let parameter = property.parameters[0];
        assert!(relative.objects.get(&parameter).is_some_and(|object| {
            object.as_parameter().is_some_and(|parameter| {
                parameter.sort == SemanticSort::Entity
                    && parameter.role == ParameterRole::RelativeClauseHead
            })
        }));
        let body = property.body.expect("property has a formula body");
        let predication = relative
            .objects
            .get(&body)
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| relative.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("POI body is an atomic restriction");
        assert!(matches!(
            predication.relation.as_data(),
            data!(PredicationRelation::Named { relation }) if relation == "remna"
        ));
        assert_eq!(
            predication
                .arguments
                .get(&argument_key(1))
                .and_then(|argument| argument.value),
            Some(parameter)
        );

        let linked = semantic_result_for("bei le dinju").expect_err("BEI omits its link head");
        assert_eq!(linked.kind, SemanticsErrorKind::RequiresDiscourseContext);
        assert!(linked.message.contains("omitted linked-argument head"));

        let quantified = semantic_graph_for("ro do");
        let quantified_utterance = quantified.objects[&quantified.root]
            .as_utterance()
            .expect("quantified fragment utterance");
        assert_eq!(quantified_utterance.force, UtteranceForce::Mention);
        let quantified_content = quantified_utterance
            .content
            .and_then(|content| quantified.objects[&content].as_formula())
            .expect("quantified fragment formula");
        let data!(FormulaNode::Quantified(scope)) = quantified_content.as_data() else {
            panic!("quantified fragment must preserve its quantifier scope");
        };
        assert_eq!(scope.operator, FormulaOperator::Forall);
        assert_eq!(scope.variable.referent_sort(), Some(SemanticSort::Entity));
        assert_eq!(scope.restriction, Some(scope.body));
        assert!(
            scope
                .quantity
                .and_then(|quantity| quantified.objects[&quantity].as_quantity())
                .is_some_and(|quantity| quantity.form == QuantityForm::All)
        );

        for source in ["fe", "coi mofo", "to be safe", "fi", "fo"] {
            let place_tag = semantic_result_for(source)
                .expect_err("a standalone FA tag has no bridi place structure");
            assert_eq!(place_tag.kind, SemanticsErrorKind::RequiresDiscourseContext);
            assert!(place_tag.message.contains("standalone place-tag fragment"));
        }

        let naku = semantic_result_for("naku")
            .expect_err("standalone naku has no proposition whose scope it can negate");
        assert_eq!(naku.kind, SemanticsErrorKind::RequiresDiscourseContext);
        assert!(naku.message.contains("missing bridi proposition"));

        let termset = semantic_result_for(
            "nu'ige zo by. .a zo beiste nu'ugi zo zy. .a zo zgana toji'a zo viska toi",
        )
        .expect_err("a standalone termset has no bridi place structure");
        assert_eq!(termset.kind, SemanticsErrorKind::RequiresDiscourseContext);
        assert!(termset.message.contains("standalone termset fragment"));

        let deleted = semantic_graph_for("ru'a zi'o");
        let deleted_content = deleted
            .objects
            .get(&deleted.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("deleted sumti fragment keeps its referential mention");
        assert_eq!(
            deleted_content.object_kind(),
            crate::model::SemanticObjectKind::Referent
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pending_statement_connections_keep_the_present_operand_and_missing_side() {
        let trailing = semantic_graph_for("broda i je broda i ja");
        let pending = trailing
            .objects
            .get(&trailing.root)
            .and_then(SemanticObject::as_sequence)
            .expect("trailing connective builds a sequence");
        assert_eq!(
            pending.elided_connection_operand,
            Some(ElidedConnectionOperand::FollowingDiscourse)
        );
        assert_eq!(pending.items.len(), 1);
        let inner = trailing
            .objects
            .get(&pending.items[0])
            .and_then(SemanticObject::as_sequence)
            .expect("the completed JE connection remains nested");
        let inner_formula = inner.content.expect("JE connection has content");
        assert_eq!(
            trailing
                .objects
                .get(&inner_formula)
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::And)
        );
        let pending_formula = pending.content.expect("pending JA has unary content");
        let pending_object = trailing
            .objects
            .get(&pending_formula)
            .expect("pending formula exists");
        assert_eq!(pending_object.formula_operator(), Some(FormulaOperator::Or));
        assert_eq!(pending_object.formula_children(), &[inner_formula]);

        let chained = semantic_graph_for("broda i je i ja broda");
        let outer = chained
            .objects
            .get(&chained.root)
            .and_then(SemanticObject::as_sequence)
            .expect("outer JA builds a sequence");
        assert_eq!(outer.items.len(), 2);
        let pending = chained
            .objects
            .get(&outer.items[0])
            .and_then(SemanticObject::as_sequence)
            .expect("pending JE remains the left operand");
        assert_eq!(
            pending.elided_connection_operand,
            Some(ElidedConnectionOperand::FollowingDiscourse)
        );
        let pending_formula = pending.content.expect("pending JE has unary content");
        let pending_object = chained
            .objects
            .get(&pending_formula)
            .expect("pending formula exists");
        assert_eq!(
            pending_object.formula_operator(),
            Some(FormulaOperator::And)
        );
        assert_eq!(pending_object.formula_children().len(), 1);
        let outer_formula = outer.content.expect("outer JA has binary content");
        let outer_object = chained
            .objects
            .get(&outer_formula)
            .expect("outer formula exists");
        assert_eq!(outer_object.formula_operator(), Some(FormulaOperator::Or));
        assert_eq!(outer_object.formula_children().len(), 2);
        assert_eq!(outer_object.formula_children()[0], pending_formula);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_prenex_and_text_group_connections_keep_their_graph_structure() {
        for source in ["broda i je brode", "broda je i brode"] {
            let graph = semantic_graph_for(source);
            let sequence = graph
                .objects
                .get(&graph.root)
                .and_then(SemanticObject::as_sequence)
                .expect("statement connection is a sequence");
            assert_eq!(sequence.items.len(), 2);
            let formula = sequence.content.expect("statement connection has content");
            let formula = graph.objects.get(&formula).expect("formula exists");
            assert_eq!(formula.formula_operator(), Some(FormulaOperator::And));
            assert_eq!(formula.formula_children().len(), 2);
            for (item, child) in sequence.items.iter().zip(formula.formula_children()) {
                let utterance = graph
                    .objects
                    .get(item)
                    .and_then(SemanticObject::as_utterance)
                    .expect("each connection operand is an utterance");
                assert_eq!(utterance.force, UtteranceForce::Subordinated);
                assert_eq!(utterance.content, Some(*child));
            }
        }

        let prenex = semantic_graph_for("da zo'u broda i je brode");
        let sequence = prenex
            .objects
            .get(&prenex.root)
            .and_then(SemanticObject::as_sequence)
            .expect("prenex connection remains a sequence");
        assert_eq!(sequence.items.len(), 2);
        let quantified = sequence.content.expect("prenex scopes over the connection");
        let quantified = prenex.objects.get(&quantified).expect("formula exists");
        assert_eq!(quantified.formula_operator(), Some(FormulaOperator::Exists));
        let body = quantified.formula_body().expect("quantifier has a body");
        assert_eq!(
            prenex
                .objects
                .get(&body)
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::And)
        );

        let grouped = semantic_graph_for("tu'e broda i je brode tu'u");
        let nested = grouped
            .objects
            .get(&grouped.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("text group denotes nested discourse");
        let nested = grouped
            .objects
            .get(&nested)
            .and_then(SemanticObject::as_sequence)
            .expect("text group content is a sequence");
        assert_eq!(nested.items.len(), 2);
        assert_eq!(
            nested
                .content
                .and_then(|formula| grouped.objects.get(&formula))
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::And)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_prenex_keeps_relation_label_and_quantifier_scope_order() {
        let labeled = semantic_graph_for("cy pa nu ba ku zo'u cy no fliba kei");
        assert_eq!(
            named_predication_ids(&labeled, "nu ba ku zo'u fliba").len(),
            1,
            "the abstraction relation label must retain the embedded prenex"
        );
        assert!(
            named_predication_ids(&labeled, "nu fliba").is_empty(),
            "dropping the prenex must not produce the same relation"
        );

        let scoped = semantic_graph_for("mi djica lo nu ro da su'o de zo'u da dunda de");
        let abstraction_body = scoped
            .objects
            .values()
            .find_map(|object| object.as_eventuality().and_then(|node| node.content))
            .expect("the NU abstraction has a formula body");
        let data!(FormulaNode::Quantified(outer)) = scoped.objects[&abstraction_body]
            .as_formula()
            .expect("abstraction body is a formula")
            .as_data()
        else {
            panic!("first prenex term must introduce the outer quantifier");
        };
        assert_eq!(outer.operator, FormulaOperator::Forall);
        let data!(FormulaNode::Quantified(inner)) = scoped.objects[&outer.body]
            .as_formula()
            .expect("outer quantifier body is a formula")
            .as_data()
        else {
            panic!("second prenex term must introduce the inner quantifier");
        };
        assert_eq!(inner.operator, FormulaOperator::Cardinality);

        let dunda = named_predication_ids(&scoped, "dunda");
        assert_eq!(dunda.len(), 1);
        let dunda_id = dunda[0];
        let dunda = scoped.objects[&dunda_id]
            .as_predication()
            .expect("dunda predication");
        assert_eq!(
            dunda.arguments[&argument_key(1)].value,
            Some(outer.variable)
        );
        assert_eq!(
            dunda.arguments[&argument_key(2)].value,
            Some(inner.variable)
        );
        assert_eq!(
            scoped.objects[&inner.body].formula_predication(),
            Some(dunda_id)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_statement_abstractions_preserve_connection_and_group_labels() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(zantufa)").expect("Zantufa dialect");
        let options = jbotci_syntax::ParseOptions::default().with_dialect_definition(&dialect);
        for (source, relation) in [
            ("nu broda i je brode kei", "nu (broda) je (brode)"),
            ("nu broda je i brode kei", "nu (broda) je (brode)"),
            ("nu ga broda gi brode kei", "nu ga (broda) gi (brode)"),
            (
                "nu tu'e broda i je brode tu'u kei",
                "nu tu'e (broda) je (brode) tu'u",
            ),
        ] {
            let graph = semantic_result_for_with_parse_options(source, &options)
                .expect("statement abstraction has semantics");
            assert!(graph.objects.values().any(|object| {
                object.as_predication().is_some_and(|predication| {
                    matches!(
                        predication.relation.as_data(),
                        data!(PredicationRelation::Named { relation: candidate })
                            if candidate == relation
                    )
                })
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_initial_connections_keep_the_present_operand_and_prior_discourse_slot() {
        let logical = semantic_graph_for(".ija cfipu mi");
        let sequence = logical
            .objects
            .get(&logical.root)
            .and_then(SemanticObject::as_sequence)
            .expect("text-initial JA builds a sequence");
        assert_eq!(
            sequence.elided_connection_operand,
            Some(ElidedConnectionOperand::PriorDiscourse)
        );
        assert_eq!(sequence.items.len(), 1);
        let content = sequence.content.expect("JA has unary connective content");
        let formula = logical.objects.get(&content).expect("JA formula exists");
        assert_eq!(formula.formula_operator(), Some(FormulaOperator::Or));
        assert_eq!(formula.formula_children().len(), 1);
        assert_eq!(sequence.connection_claims.len(), 0);

        let modal = semantic_graph_for(".iseni'ibo zo se se vimcu");
        let sequence = modal
            .objects
            .get(&modal.root)
            .and_then(SemanticObject::as_sequence)
            .expect("text-initial modal builds a sequence");
        assert_eq!(
            sequence.elided_connection_operand,
            Some(ElidedConnectionOperand::PriorDiscourse)
        );
        assert_eq!(sequence.items.len(), 1);
        assert!(sequence.content.is_none());
        assert_eq!(sequence.connection_claims.len(), 1);
        let claim = modal
            .objects
            .get(&sequence.connection_claims[0])
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| modal.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("NIhI connection is an atomic claim");
        assert!(matches!(
            claim.relation.as_data(),
            data!(PredicationRelation::Named { relation }) if relation == "nibli"
        ));
        assert_eq!(claim.arguments.len(), 1);
        assert!(claim.arguments.contains_key(&argument_key(2)));
        assert_eq!(claim.introduced_by.as_deref(), Some("se ni'i"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derived_renderings_descend_into_formula_valued_arguments() {
        let graph = semantic_graph_for("gy prane cukla .i se ni'i bo lo nabmi cu nadycai");
        let sequence = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_sequence)
            .expect("modal statement connection builds a sequence");
        let claim_formula = sequence.connection_claims[0];
        let claim_predication = graph
            .objects
            .get(&claim_formula)
            .and_then(SemanticObject::formula_predication)
            .expect("modal statement connection claim is atomic");
        let claim = graph
            .objects
            .get(&claim_predication)
            .and_then(SemanticObject::as_predication)
            .expect("modal statement connection has a predication");
        let branch_formulas = [
            claim.arguments[&argument_key(1)]
                .value
                .expect("connection x1 is present"),
            claim.arguments[&argument_key(2)]
                .value
                .expect("connection x2 is present"),
        ];
        assert_ne!(branch_formulas[0], branch_formulas[1]);
        assert!(branch_formulas.iter().all(|id| {
            id.object_kind() == SemanticObjectKind::Formula && graph.objects.contains_key(id)
        }));

        for render in [
            crate::render::render_tree_proj as fn(&SemanticGraph) -> String,
            crate::render::render_tree,
        ] {
            let rendered = render(&graph);
            let lines: Vec<_> = rendered.lines().collect();
            let claim_index = lines
                .iter()
                .position(|line| line.contains(&format!("[{claim_predication}]")))
                .expect("connection claim predication is rendered");
            let claim_indent = lines[claim_index]
                .bytes()
                .take_while(|byte| *byte == b' ')
                .count();
            let descendants: Vec<_> = lines[claim_index + 1..]
                .iter()
                .take_while(|line| {
                    line.bytes().take_while(|byte| *byte == b' ').count() > claim_indent
                })
                .copied()
                .collect();
            for branch in branch_formulas {
                assert!(descendants.iter().any(|line| {
                    line.trim_start().starts_with("formula argument: ")
                        && line.ends_with(&format!(" [{branch}]"))
                }));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_atom_event_is_typed_bound_and_not_projected() {
        let graph = semantic_graph_for("mi klama");
        let event = generated_event_for_relation(&graph, "klama");
        let object = graph.objects.get(&event).expect("event exists");
        let owner = event_binding_owner(&graph, event);

        assert_eq!(object.referent_category(), None);
        assert_eq!(object.scope_dependence(), None);
        assert_eq!(
            graph
                .objects
                .get(&owner)
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::Atom)
        );
        let json = serde_json::to_value(object).expect("event serializes");
        assert_eq!(json["denotation"], serde_json::json!("generated-bound"));
        assert!(json.get("category").is_none());
        assert!(json.get("scopeDependence").is_none());
        let tree_proj = crate::render::render_tree_proj(&graph);
        assert!(tree_proj.contains(&format!("binds=exists eventuality[{event}]")));
        assert!(tree_proj.contains(&format!("event=eventuality[{event}]")));
        assert!(!tree_proj.contains(&format!("denotes eventuality[{event}]")));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derived_renderings_project_event_conditions_and_explicit_absence() {
        let tenseless = semantic_graph_for("mi klama");
        let past = semantic_graph_for("mi pu klama");
        let actual = semantic_graph_for("mi ca'a klama");
        let interval = semantic_graph_for("mi pu ze'a klama");
        let aspect = semantic_graph_for("mi ca'o klama");
        let spatial = semantic_graph_for("mi vi klama");

        for render in [
            crate::render::render_tree_proj as fn(&SemanticGraph) -> String,
            crate::render::render_tree,
        ] {
            let tenseless = render(&tenseless);
            let past = render(&past);
            assert_ne!(tenseless, past);
            assert!(tenseless.contains("time=unspecified; actuality=unspecified"));
            assert!(past.contains("time=before(anchor=now[eventuality:3]"));
            assert!(render(&actual).contains("actuality=actual"));
            assert!(render(&interval).contains("time-interval=medium(anchor=unspecified)"));
            assert!(render(&aspect).contains("aspect=continuative(anchor=unspecified"));
            assert!(render(&spatial).contains(
                "space=distanceFrom(anchor=here[entity:4]; sticky=false; details={distance=short"
            ));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_proj_format_partitions_tree_and_displaced_commitments() {
        let graph = semantic_graph_for("mi nitcu lo tanxe");
        let tree_proj = crate::render::render_tree_proj(&graph);
        let (tree, projected) = tree_proj
            .split_once("\n\nprojected:\n")
            .expect("tree+proj format has a tree spine and projected section");

        assert!(tree.starts_with("utterance assert "));
        assert!(tree.contains("descriptor body: atom"));
        assert!(projected.contains("tanxe("));
        assert!(projected.contains("[mode=restrictive]"));
        assert!(!tree_proj.contains("at-issue commitments:"));
        assert!(!tree_proj.contains("presupposed/projected:"));
        assert!(!tree_proj.contains("context="));

        let elided = graph
            .objects
            .iter()
            .filter_map(|(&id, object)| {
                (object.referent_category() == Some(ReferentCategory::Constant)
                    && object
                        .descriptor()
                        .is_some_and(|descriptor| descriptor.kind == DescriptorKind::Elided))
                .then_some(id)
            })
            .collect::<Vec<_>>();
        assert_eq!(elided.len(), 3, "the probe has three implicit places");
        let grouped = projected
            .lines()
            .find(|line| line.contains("descriptor-kind=elided"))
            .expect("elided constants share one projected line");
        assert_eq!(
            projected
                .lines()
                .filter(|line| line.contains("descriptor-kind=elided"))
                .count(),
            1,
            "the shared implicit-constant template must not be split"
        );
        for id in elided {
            assert_eq!(
                grouped.matches(&format!("[{id}]")).count(),
                1,
                "each grouped constant must be retained exactly once"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_proj_format_renders_each_event_condition_block_once() {
        let graph = semantic_graph_for("mi pu klama");
        let tree_proj = crate::render::render_tree_proj(&graph);
        let tree = crate::render::render_tree(&graph);
        let event = generated_event_for_relation(&graph, "klama");

        assert!(tree.matches(&format!("[{event}]; time=")).count() >= 1);
        assert!(tree.matches(&format!("[{event}] {{time=")).count() >= 1);
        for (&id, object) in &graph.objects {
            if object.as_eventuality().is_none() {
                continue;
            }
            let condition_sites = tree_proj.matches(&format!("[{id}]; time=")).count()
                + tree_proj.matches(&format!("[{id}] {{time=")).count();
            assert_eq!(
                condition_sites, 1,
                "event {id} must have exactly one tree+proj condition site"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derived_traversal_surfaces_scoped_connective_event() {
        let graph = semantic_graph_for("mi na pu na ca klama le zarci");
        let eventuality = graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::Connective(node)) if node.eventuality.is_some() => {
                    node.eventuality
                }
                _ => None,
            })
            .expect("scoped tense formula has its own eventuality");
        let expected_use = format!("scoped {{event=eventuality[{eventuality}]");
        let expected_binding = format!(
            "binds=exists eventuality[{eventuality}] {{time=before(anchor=now[eventuality:3]"
        );
        for rendering in [
            crate::render::render_tree_proj(&graph),
            crate::render::render_tree(&graph),
        ] {
            assert!(rendering.contains(&expected_use));
            assert!(rendering.contains(&expected_binding));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn negated_generated_event_binds_inside_not() {
        let graph = semantic_graph_for("mi na klama");
        let event = generated_event_for_relation(&graph, "klama");
        let owner = event_binding_owner(&graph, event);
        assert_eq!(
            graph
                .objects
                .get(&owner)
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::Atom)
        );
        assert!(graph.objects.values().any(|object| {
            object.formula_operator() == Some(FormulaOperator::Not)
                && object.formula_children().contains(&owner)
        }));
        let tree = crate::render::render_tree(&graph);
        assert!(tree.contains(&format!("child: atom binds=exists eventuality[{event}]")));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tanru_head_event_binds_at_shared_conjunction() {
        let graph = semantic_graph_for("barda xunre gerku");
        let event = generated_event_for_relation(&graph, "gerku");
        let owner = event_binding_owner(&graph, event);
        assert_eq!(
            graph
                .objects
                .get(&owner)
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::And)
        );
        assert!(
            crate::render::render_tree(&graph)
                .contains(&format!("and binds=exists eventuality[{event}]"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tanru_and_co_keep_modifier_graphs_and_head_place_structure() {
        let tanru = semantic_graph_for("so'u prenu cu kelci djica");
        let djica = named_predication_ids(&tanru, "djica");
        let kelci = named_predication_ids(&tanru, "kelci");
        assert_eq!(djica.len(), 1);
        assert_eq!(kelci.len(), 1);
        let link = tanru
            .objects
            .values()
            .find_map(SemanticObject::predication_tanru_link)
            .expect("plain tanru keeps an explicit modifier link");
        assert_eq!(link.head, djica[0]);
        let modifier_body = tanru
            .objects
            .get(&link.modifier)
            .and_then(SemanticObject::as_referent)
            .and_then(|referent| referent.body)
            .expect("tanru modifier is a relation abstraction");
        assert!(formula_contains_predication(
            &tanru,
            modifier_body,
            kelci[0]
        ));
        let djica_x1 = tanru.objects[&djica[0]]
            .as_predication()
            .expect("djica")
            .arguments[&argument_key(1)]
            .value;
        let kelci_x1 = tanru.objects[&kelci[0]]
            .as_predication()
            .expect("kelci")
            .arguments[&argument_key(1)]
            .value;
        assert_ne!(djica_x1, kelci_x1, "modifier x1 is its property parameter");

        let co = semantic_graph_for("mi zbasu co fagri do");
        let zbasu = named_predication_ids(&co, "zbasu");
        let fagri = named_predication_ids(&co, "fagri");
        assert_eq!(zbasu.len(), 1);
        assert_eq!(fagri.len(), 1);
        let link = co
            .objects
            .values()
            .find_map(SemanticObject::predication_tanru_link)
            .expect("CO inversion keeps an explicit modifier link");
        assert_eq!(link.head, zbasu[0], "pre-CO tertau remains the head");
        let modifier_body = co
            .objects
            .get(&link.modifier)
            .and_then(SemanticObject::as_referent)
            .and_then(|referent| referent.body)
            .expect("post-CO seltau is a relation abstraction");
        assert!(formula_contains_predication(&co, modifier_body, fagri[0]));
        let zbasu = co.objects[&zbasu[0]].as_predication().expect("zbasu");
        let fagri = co.objects[&fagri[0]].as_predication().expect("fagri");
        assert_eq!(
            zbasu.arguments[&argument_key(1)].value,
            Some(SemanticObjectId::speaker())
        );
        assert_eq!(
            fagri.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::addressee()),
            "post-CO terms start at the modifier's x2"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_tanru_head_shares_x1_but_not_branch_events() {
        let graph = semantic_graph_for("mi brodo ke broda je brode ke'e bau do");
        let broda = named_predication_ids(&graph, "broda");
        let brode = named_predication_ids(&graph, "brode");
        let brodo = named_predication_ids(&graph, "brodo");
        assert_eq!(broda.len(), 1);
        assert_eq!(brode.len(), 1);
        assert_eq!(brodo.len(), 1);
        let broda_node = graph.objects[&broda[0]].as_predication().expect("broda");
        let brode_node = graph.objects[&brode[0]].as_predication().expect("brode");
        assert_eq!(
            broda_node.arguments[&argument_key(1)].value,
            brode_node.arguments[&argument_key(1)].value
        );
        assert_ne!(broda_node.eventuality, brode_node.eventuality);
        for branch in [broda_node, brode_node] {
            let modal = branch
                .modal_arguments
                .iter()
                .find(|modal| modal.relation.as_deref() == Some("bangu"))
                .expect("group-head modal term attaches to every connected branch");
            assert_eq!(
                modal.arguments[&argument_key(1)].value,
                Some(SemanticObjectId::addressee())
            );
        }
        let link = graph
            .objects
            .values()
            .find_map(SemanticObject::predication_tanru_link)
            .expect("outer tanru modifier remains explicit");
        assert_eq!(link.head, broda[0]);
        let modifier_body = graph
            .objects
            .get(&link.modifier)
            .and_then(SemanticObject::as_referent)
            .and_then(|referent| referent.body)
            .expect("outer modifier is a relation abstraction");
        assert!(formula_contains_predication(
            &graph,
            modifier_body,
            brodo[0]
        ));
        let head_event = broda_node.eventuality.expect("head branch has an event");
        assert_eq!(
            event_binding_owner(&graph, head_event).object_kind(),
            SemanticObjectKind::Formula
        );
        assert_eq!(
            graph
                .objects
                .get(&event_binding_owner(&graph, head_event))
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::And)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn me_and_moi_preserve_referents_ordinals_and_linkargs() {
        let me = semantic_graph_for("mi me do");
        let referent_of = named_predication_ids(&me, "referentOf");
        assert_eq!(referent_of.len(), 1);
        let referent_of = me.objects[&referent_of[0]]
            .as_predication()
            .expect("ME predication");
        assert_eq!(referent_of.arguments.len(), 2);
        assert_eq!(
            referent_of.arguments[&argument_key(1)].value,
            Some(SemanticObjectId::speaker())
        );
        assert_eq!(
            referent_of.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::addressee())
        );

        let linked =
            semantic_graph_for("xu le kelvo be li rezeci cu dunli le me la sesius. be li no");
        let linked_referent_of = named_predication_ids(&linked, "referentOf");
        assert_eq!(linked_referent_of.len(), 1);
        let linked_referent_of = linked.objects[&linked_referent_of[0]]
            .as_predication()
            .expect("linked ME predication");
        assert_eq!(linked_referent_of.arguments.len(), 3);
        assert_eq!(
            linked_referent_of.arguments[&argument_key(2)]
                .value
                .and_then(|id| id.referent_sort()),
            Some(SemanticSort::Entity)
        );
        assert_eq!(
            linked_referent_of.arguments[&argument_key(3)]
                .value
                .and_then(|id| id.referent_sort()),
            Some(SemanticSort::Number)
        );

        let moi = semantic_graph_for("ta me li ny. su'i pa me'u moi le'i mi ratcu");
        assert!(named_predication_ids(&moi, "referentOf").is_empty());
        let ordinal = named_predication_ids(&moi, "li ny su'i pa moi");
        assert_eq!(ordinal.len(), 1);
        let ordinal = moi.objects[&ordinal[0]]
            .as_predication()
            .expect("MOI predication");
        assert_eq!(ordinal.arguments.len(), 3);
        assert_eq!(
            ordinal.arguments[&argument_key(1)]
                .value
                .and_then(|id| moi.objects.get(&id))
                .and_then(SemanticObject::as_referent)
                .and_then(|referent| referent.indexical),
            Some(IndexicalKind::MedialDemonstrative)
        );
        assert_eq!(
            ordinal.arguments[&argument_key(2)]
                .value
                .and_then(|id| id.referent_sort()),
            Some(SemanticSort::Set)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn headline_scoped_connected_tanru_keeps_both_property_branches() {
        let graph = semantic_graph_for(
            "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa ro lo jmive kei ta'i lo racli",
        );
        let xendo = named_predication_ids(&graph, "xendo");
        let cnikansa = named_predication_ids(&graph, "cnikansa");
        assert_eq!(xendo.len(), 1);
        assert_eq!(cnikansa.len(), 1);
        let xendo_node = graph.objects[&xendo[0]].as_predication().expect("xendo");
        let cnikansa_node = graph.objects[&cnikansa[0]]
            .as_predication()
            .expect("cnikansa");
        assert_eq!(xendo_node.mode, PredicationMode::Restrictive);
        assert_eq!(cnikansa_node.mode, PredicationMode::Restrictive);
        assert_eq!(
            xendo_node.arguments[&argument_key(1)].value,
            cnikansa_node.arguments[&argument_key(1)].value,
            "connected property branches share ce'u"
        );
        assert_eq!(
            xendo_node.arguments[&argument_key(2)].value,
            cnikansa_node.arguments[&argument_key(2)].value,
            "connected property branches share ro lo jmive"
        );
        assert!(graph.objects.iter().any(|(&formula, object)| {
            object.formula_operator() == Some(FormulaOperator::And)
                && formula_contains_predication(&graph, formula, xendo[0])
                && formula_contains_predication(&graph, formula, cnikansa[0])
        }));
        assert_eq!(named_predication_ids(&graph, "tarti").len(), 1);
        assert_eq!(named_predication_ids(&graph, "troci").len(), 1);
        assert_eq!(named_predication_ids(&graph, "cadga").len(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_word_tanru_units_keep_typed_inner_arguments() {
        let preposed = semantic_graph_for("lo be mi broda cu melbi");
        let broda = named_predication_ids(&preposed, "broda");
        assert_eq!(broda.len(), 1);
        let broda = preposed.objects[&broda[0]]
            .as_predication()
            .expect("preposed-BE inner relation");
        assert_eq!(broda.mode, PredicationMode::Restrictive);
        assert_eq!(
            broda.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::speaker()),
            "preposed BE fills the inner relation's x2"
        );

        let quoted_bridi = semantic_graph_for("go'oi broda");
        let quoted_bridi_relations = quoted_bridi
            .objects
            .values()
            .filter_map(SemanticObject::as_predication)
            .map(|predication| match predication.relation.as_data() {
                data!(crate::model::PredicationRelation::Named { relation }) => relation.as_str(),
                data!(crate::model::PredicationRelation::Parameter { .. }) => {
                    panic!("quoted bridi relation is not a parameter")
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(quoted_bridi_relations, ["cmavo:go'oĭ-\"broda\""]);

        let tag_relation = semantic_graph_for("cy no xo'i ne'i cy pa");
        assert_eq!(named_predication_ids(&tag_relation, "xo'i ne'i").len(), 1);

        let dialect =
            jbotci_dialect::parse_dialect_definition("(zantufa)").expect("Zantufa dialect");
        let options = jbotci_syntax::ParseOptions::default().with_dialect_definition(&dialect);
        let text_relation =
            semantic_result_for_with_parse_options("mi lu'ei do klama li'au", &options)
                .expect("LUhEI relation unit has semantics");
        assert_eq!(
            named_predication_ids(&text_relation, "lu'ei do klama li'au").len(),
            1
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_forethought_bridi_shares_terms_across_connected_branches() {
        let graph = semantic_graph_for(
            ".i ma'i le ci moi ba ku le za'u da ga nai du'e va'e le ka citno kei gi'e ricfu gi snada le ka rivbi kei gi'e troci le ka citka do",
        );
        let mut shared_x1 = None;
        let mut shared_standard = None;
        for relation in ["du'e va'e", "ricfu", "snada", "troci"] {
            let predications = named_predication_ids(&graph, relation);
            assert_eq!(predications.len(), 1);
            let predication = graph.objects[&predications[0]]
                .as_predication()
                .expect("forethought branch predication");
            let x1 = predication.arguments[&argument_key(1)]
                .value
                .expect("shared forethought x1");
            assert_eq!(*shared_x1.get_or_insert(x1), x1);
            let standard = predication
                .modal_arguments
                .iter()
                .find(|modal| modal.relation.as_deref() == Some("manri"))
                .and_then(|modal| modal.arguments[&argument_key(1)].value)
                .expect("MAhI standard attaches to every branch");
            assert_eq!(*shared_standard.get_or_insert(standard), standard);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn discourse_connection_events_bind_on_sequence_owner() {
        let graph = semantic_graph_for("do nelci mi .ibabo mi nelci do");
        let sequence = graph
            .objects
            .iter()
            .find_map(|(&id, object)| object.as_sequence().is_some().then_some(id))
            .expect("statement connection creates a sequence");
        let item_events = graph
            .objects
            .values()
            .filter_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "nelci")
                    .then_some(predication.eventuality)
                    .flatten()
            })
            .collect::<Vec<_>>();
        assert_eq!(item_events.len(), 2);
        for event in item_events {
            assert_eq!(event_binding_owner(&graph, event), sequence);
        }
        assert_eq!(
            graph
                .objects
                .get(&sequence)
                .expect("sequence exists")
                .bound_eventualities()
                .len(),
            2
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reified_formula_connection_events_bind_on_connection_formula() {
        let graph = semantic_graph_for("mi klama pugi le zarci gi le zdani");
        let connection_formula = graph
            .objects
            .iter()
            .find_map(|(&formula, object)| {
                let predication = object
                    .formula_predication()
                    .and_then(|predication| graph.objects.get(&predication))?
                    .as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "before")
                    .then_some(formula)
            })
            .expect("tense connection has an asserted connection formula");
        let branch_events = graph
            .objects
            .values()
            .filter_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                    .then_some(predication.eventuality)
                    .flatten()
            })
            .collect::<Vec<_>>();
        assert_eq!(branch_events.len(), 2);
        for event in branch_events {
            assert_eq!(event_binding_owner(&graph, event), connection_formula);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn promoted_nu_event_is_referential_and_never_bound() {
        let graph = semantic_graph_for("mi nelci do mu'i le nu do nelci mi");
        let abstraction = graph
            .objects
            .iter()
            .find_map(|(&id, object)| {
                object.as_eventuality().and_then(|eventuality| {
                    (eventuality.content.is_some()
                        && eventuality
                            .descriptor
                            .as_ref()
                            .is_some_and(|descriptor| descriptor.word == "le"))
                    .then_some(id)
                })
            })
            .expect("le nu denotes an eventuality");
        let object = graph.objects.get(&abstraction).expect("abstraction exists");
        assert_eq!(object.referent_category(), Some(ReferentCategory::Constant));
        assert!(!object.is_generated_eventuality());
        assert!(graph.objects.values().all(|owner| {
            owner
                .bound_eventualities()
                .iter()
                .all(|bound| bound.object_id() != abstraction)
        }));
        assert_eq!(
            serde_json::to_value(object).expect("abstraction serializes")["denotation"],
            serde_json::json!("referential")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_actuality_constrains_locally_bound_event() {
        let graph = semantic_graph_for("mi ca'a klama");
        let event = generated_event_for_relation(&graph, "klama");
        let eventuality = graph
            .objects
            .get(&event)
            .and_then(SemanticObject::as_eventuality)
            .expect("generated event exists");
        assert_eq!(
            eventuality.actuality.map(|actuality| actuality.kind),
            Some(ActualityKind::Actual)
        );
        assert_eq!(
            graph
                .objects
                .get(&event_binding_owner(&graph, event))
                .and_then(SemanticObject::formula_operator),
            Some(FormulaOperator::Atom)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restricted_universal_emits_projective_domain_import() {
        let graph = semantic_graph_for("ro mlatu cu jbena");
        let quantified = graph
            .objects
            .values()
            .find(|object| {
                object.formula_operator() == Some(FormulaOperator::Forall)
                    && object.formula_restriction().is_some()
            })
            .expect("restricted forall formula");

        assert_eq!(
            quantified.formula_domain_import(),
            Some(DomainImport::Projective)
        );
        assert_eq!(
            serde_json::to_value(quantified).expect("formula should serialize")["domainImport"],
            serde_json::json!("projective")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantified_vocative_scopes_its_addressee_and_target_formula() {
        let graph = semantic_graph_for("coi rodo");
        let utterance = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .expect("root should be the vocative utterance");
        assert_eq!(utterance.force, UtteranceForce::Vocative);
        let content = utterance.content.expect("quantified vocative content");
        let data!(FormulaNode::Quantified(scope)) = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_formula)
            .expect("vocative content should be a formula")
            .as_data()
        else {
            panic!("quantified vocative content should be a quantified formula");
        };
        assert_eq!(scope.operator, FormulaOperator::Forall);
        assert_eq!(utterance.audience, scope.variable);
        assert_eq!(
            graph
                .objects
                .get(&scope.variable)
                .and_then(|object| object.referent_category()),
            Some(ReferentCategory::Variable)
        );
        assert_eq!(
            graph
                .objects
                .get(&content)
                .and_then(SemanticObject::formula_domain_import),
            Some(DomainImport::Projective)
        );
        let quantity = graph
            .objects
            .get(&scope.quantity.expect("ro quantity"))
            .and_then(SemanticObject::as_quantity)
            .expect("ro quantity object");
        assert_eq!(quantity.form, QuantityForm::All);

        let restriction = graph
            .objects
            .get(&scope.restriction.expect("audience membership restriction"))
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("membership restriction predication");
        assert!(matches!(
            restriction.relation.as_data(),
            data!(crate::model::PredicationRelation::Named { relation }) if relation == "memberOf"
        ));
        assert_eq!(
            restriction.arguments[&argument_key(1)].value,
            Some(scope.variable)
        );
        assert_eq!(
            restriction.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::addressee())
        );

        let target = graph
            .objects
            .get(&scope.body)
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("vocative target predication");
        assert!(matches!(
            target.relation.as_data(),
            data!(crate::model::PredicationRelation::Named { relation }) if relation == "vocativeTarget"
        ));
        assert_eq!(target.mode, PredicationMode::Performative);
        assert_eq!(
            target.arguments[&argument_key(1)].value,
            Some(scope.variable)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn outer_quantified_description_restricts_the_matrix_argument() {
        let graph = semantic_graph_for("ro le prenu cu klama");
        let scope = graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::Quantified(scope))
                    if scope.operator == FormulaOperator::Forall =>
                {
                    Some(scope)
                }
                _ => None,
            })
            .expect("outer ro should introduce a forall scope");
        assert_eq!(
            graph
                .objects
                .values()
                .find_map(|object| {
                    let predication = object.as_predication()?;
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                        .then_some(predication.arguments[&argument_key(1)].value)
                })
                .flatten(),
            Some(scope.variable)
        );
        let restriction = graph
            .objects
            .get(
                &scope
                    .restriction
                    .expect("description membership restriction"),
            )
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("description membership predication");
        assert!(matches!(
            restriction.relation.as_data(),
            data!(crate::model::PredicationRelation::Named { relation }) if relation == "memberOf"
        ));
        assert_eq!(
            restriction.arguments[&argument_key(1)].value,
            Some(scope.variable)
        );
        let domain = restriction.arguments[&argument_key(2)]
            .value
            .expect("description referent domain");
        assert_eq!(
            graph
                .objects
                .get(&domain)
                .and_then(SemanticObject::descriptor)
                .map(|descriptor| descriptor.word.as_str()),
            Some("le")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cardinal_quantified_pro_sumti_restricts_the_matrix_argument() {
        let graph = semantic_graph_for("re do klama");
        let scope = graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::Quantified(scope))
                    if scope.operator == FormulaOperator::Cardinality =>
                {
                    Some(scope)
                }
                _ => None,
            })
            .expect("re do should introduce cardinality scope");
        let quantity = graph
            .objects
            .get(&scope.quantity.expect("re quantity"))
            .and_then(SemanticObject::as_quantity)
            .expect("re quantity object");
        assert_eq!(quantity.form, QuantityForm::Exact);
        assert_eq!(quantity.value.integer, Some(2));
        let klama = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                    .then_some(predication)
            })
            .expect("matrix klama predication");
        assert_eq!(
            klama.arguments[&argument_key(1)].value,
            Some(scope.variable)
        );
        let restriction = graph
            .objects
            .get(&scope.restriction.expect("audience membership restriction"))
            .and_then(SemanticObject::formula_predication)
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("audience membership predication");
        assert_eq!(
            restriction.arguments[&argument_key(1)].value,
            Some(scope.variable)
        );
        assert_eq!(
            restriction.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::addressee())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn three_sumti_afterthought_chain_distributes_with_left_grouping() {
        let graph = semantic_graph_for("le glico .e le dotco .e le fraso cu tavla");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let data!(FormulaNode::Connective(outer)) = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_formula)
            .expect("outer connection formula")
            .as_data()
        else {
            panic!("three-branch chain should have an outer connective");
        };
        assert_eq!(outer.operator, FormulaOperator::And);
        assert_eq!(outer.children.len(), 2);
        assert_eq!(
            outer
                .connector
                .as_ref()
                .map(|connector| connector.source.as_str()),
            Some("e")
        );
        let data!(FormulaNode::Connective(inner)) = graph
            .objects
            .get(&outer.children[0])
            .and_then(SemanticObject::as_formula)
            .expect("left-grouped inner connection")
            .as_data()
        else {
            panic!("the first two sumti should form the left branch");
        };
        assert_eq!(inner.operator, FormulaOperator::And);
        assert_eq!(inner.children.len(), 2);
        assert_eq!(
            inner
                .connector
                .as_ref()
                .map(|connector| connector.source.as_str()),
            Some("e")
        );
        assert_eq!(
            graph
                .objects
                .values()
                .filter_map(SemanticObject::as_predication)
                .filter(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "tavla"))
                .count(),
            3
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ke_grouped_sumti_preserves_the_explicit_right_branch() {
        let graph = semantic_graph_for("le klama .e ke le broda .e le brode ke'e cu cadzu");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let data!(FormulaNode::Connective(outer)) = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_formula)
            .expect("outer connection formula")
            .as_data()
        else {
            panic!("grouped sumti should distribute through an outer connective");
        };
        assert_eq!(outer.operator, FormulaOperator::And);
        assert_eq!(outer.children.len(), 2);
        assert!(matches!(
            graph.objects[&outer.children[0]]
                .as_formula()
                .map(FormulaNode::as_data),
            Some(data!(FormulaNode::Atom(_)))
        ));
        let data!(FormulaNode::Connective(group)) = graph
            .objects
            .get(&outer.children[1])
            .and_then(SemanticObject::as_formula)
            .expect("explicit ke group formula")
            .as_data()
        else {
            panic!("ke should keep the trailing pair grouped");
        };
        assert_eq!(group.operator, FormulaOperator::And);
        assert_eq!(group.children.len(), 2);
        assert_eq!(
            graph
                .objects
                .values()
                .filter_map(SemanticObject::as_predication)
                .filter(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "cadzu"))
                .count(),
            3
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vuho_connected_sumti_distributes_the_matrix_predication() {
        let graph = semantic_graph_for("mi viska ko'a vu'o .e ko'e");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let data!(FormulaNode::Connective(connection)) = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_formula)
            .expect("VUhO connection formula")
            .as_data()
        else {
            panic!("VUhO-connected sumti should distribute the bridi");
        };
        assert_eq!(connection.operator, FormulaOperator::And);
        assert_eq!(connection.children.len(), 2);
        assert_eq!(
            connection
                .connector
                .as_ref()
                .map(|connector| (connector.source.as_str(), connector.locus.as_str())),
            Some(("e", "sumti"))
        );
        let viska = connection
            .children
            .iter()
            .map(|formula| {
                graph.objects[formula]
                    .formula_predication()
                    .and_then(|predication| graph.objects.get(&predication))
                    .and_then(SemanticObject::as_predication)
                    .expect("distributed viska predication")
            })
            .collect::<Vec<_>>();
        assert!(viska.iter().all(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "viska")));
        assert_eq!(
            viska[0].arguments[&argument_key(1)].value,
            viska[1].arguments[&argument_key(1)].value
        );
        assert_ne!(
            viska[0].arguments[&argument_key(2)].value,
            viska[1].arguments[&argument_key(2)].value
        );
        assert!(graph.objects.values().all(|object| {
            object
                .referent_composition()
                .is_none_or(|composition| composition.operator != CompositionOperator::Joint)
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ji_sumti_connection_builds_a_connective_question_distribution() {
        let graph = semantic_graph_for("le cecmu ji le velsku cu vajni");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let question = graph
            .objects
            .get(&content)
            .and_then(SemanticObject::as_question)
            .expect("connective question");
        assert_eq!(question.kind, QuestionKind::Connective);
        assert_eq!(question.mode, QuestionMode::Direct);
        let data!(FormulaNode::Connective(connection)) = graph.objects[&question.body]
            .as_formula()
            .expect("connective-question body formula")
            .as_data()
        else {
            panic!("ji should distribute as a connective question");
        };
        assert_eq!(connection.operator, FormulaOperator::ConnectiveQuestion);
        assert_eq!(connection.children.len(), 2);
        assert!(
            connection
                .connector
                .as_ref()
                .is_some_and(|connector| connector.source == "ji" && connector.parameter.is_some())
        );
        assert_eq!(
            graph
                .objects
                .values()
                .filter_map(SemanticObject::as_predication)
                .filter(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "vajni"))
                .count(),
            2
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mixed_argument_relation_question_preserves_every_ordered_answer_slot() {
        let graph = semantic_graph_for("ma mo ma");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("mixed direct question");
        assert_eq!(question.kind, QuestionKind::Multiple);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(question.domain, SemanticSort::ArgumentBundle);
        assert_eq!(
            question
                .slots
                .iter()
                .map(|slot| {
                    let (kind, domain) = slot
                        .kind_and_domain()
                        .expect("mixed slots must carry their own type");
                    (kind, domain, slot.parameter())
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    QuestionKind::Argument,
                    SemanticSort::Entity,
                    question.slots[0].parameter(),
                ),
                (
                    QuestionKind::Relation,
                    SemanticSort::Relation,
                    question.slots[1].parameter(),
                ),
                (
                    QuestionKind::Argument,
                    SemanticSort::Entity,
                    question.slots[2].parameter(),
                ),
            ]
        );
        let first_argument = question.slots[0].parameter().expect("first ma parameter");
        let relation = question.slots[1].parameter().expect("mo parameter");
        let second_argument = question.slots[2].parameter().expect("second ma parameter");
        let predication = graph
            .objects
            .values()
            .filter_map(SemanticObject::as_predication)
            .find(|predication| matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Parameter { parameter }) if **parameter == *relation.as_data()))
            .expect("mo must remain the predication relation");
        assert!(
            predication
                .arguments
                .values()
                .any(|argument| argument.value == Some(first_argument))
        );
        assert!(
            predication
                .arguments
                .values()
                .any(|argument| argument.value == Some(second_argument))
        );
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn truth_quantity_argument_and_relation_question_preserves_all_domains() {
        let graph = semantic_graph_for("pau xo ma mo xu");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("four-domain direct question");
        assert_eq!(question.kind, QuestionKind::Multiple);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(
            question
                .slots
                .iter()
                .map(|slot| {
                    let (kind, domain) = slot
                        .kind_and_domain()
                        .expect("mixed slots must carry their own type");
                    (kind, domain, slot.parameter().is_some())
                })
                .collect::<Vec<_>>(),
            vec![
                (QuestionKind::Quantity, SemanticSort::Number, true),
                (QuestionKind::Argument, SemanticSort::Entity, true),
                (QuestionKind::Relation, SemanticSort::Relation, true),
                (QuestionKind::Truth, SemanticSort::TruthValue, false),
            ]
        );
        let quantity_parameter = question.slots[0].parameter().expect("xo parameter");
        assert!(graph.objects.values().any(|object| {
            object.as_quantity().is_some_and(|quantity| {
                quantity.value.question_parameters == vec![quantity_parameter]
            })
        }));
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn direct_question_parameters_inside_abstraction_remain_outer_answer_slots() {
        let graph = semantic_graph_for("mi djuno le du'u ma mo");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("outer direct question");
        assert_eq!(question.kind, QuestionKind::Multiple);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(question.slots.len(), 2);
        assert_eq!(
            question.slots[0].kind_and_domain(),
            Some((QuestionKind::Argument, SemanticSort::Entity))
        );
        assert_eq!(
            question.slots[1].kind_and_domain(),
            Some((QuestionKind::Relation, SemanticSort::Relation))
        );
        let argument = question.slots[0].parameter().expect("ma parameter");
        let relation = question.slots[1].parameter().expect("mo parameter");
        assert!(graph.objects.values().any(|object| {
            object.as_predication().is_some_and(|predication| {
                predication.mode == PredicationMode::Inert
                    && matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Parameter { parameter }) if **parameter == *relation.as_data())
                    && predication
                        .arguments
                        .values()
                        .any(|value| value.value == Some(argument))
            })
        }));
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_question_composes_connective_truth_and_bridi_answer_slots() {
        let graph = semantic_graph_for(".ije'ibo xu xo ma mo pei");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("composed statement question");
        assert_eq!(question.kind, QuestionKind::Multiple);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(
            question
                .slots
                .iter()
                .map(|slot| {
                    let (kind, domain) = slot
                        .kind_and_domain()
                        .expect("mixed slots must carry their own type");
                    (kind, domain, slot.parameter().is_some())
                })
                .collect::<Vec<_>>(),
            vec![
                (QuestionKind::Connective, SemanticSort::Connective, true),
                (QuestionKind::Truth, SemanticSort::TruthValue, false),
                (QuestionKind::Quantity, SemanticSort::Number, true),
                (QuestionKind::Argument, SemanticSort::Entity, true),
                (QuestionKind::Relation, SemanticSort::Relation, true),
            ]
        );
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn leading_i_indicator_truth_question_composes_with_fragment_domains() {
        let graph = semantic_graph_for(".i pei xu cu'e xo ma mo");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("leading-I mixed question");
        assert_eq!(question.kind, QuestionKind::Multiple);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(
            question
                .slots
                .iter()
                .map(|slot| {
                    let (kind, domain) = slot
                        .kind_and_domain()
                        .expect("mixed slots must carry their own type");
                    (kind, domain, slot.parameter().is_some())
                })
                .collect::<Vec<_>>(),
            vec![
                (QuestionKind::Truth, SemanticSort::TruthValue, false),
                (QuestionKind::Tense, SemanticSort::TenseModal, true),
                (QuestionKind::Quantity, SemanticSort::Number, true),
                (QuestionKind::Argument, SemanticSort::Entity, true),
                (QuestionKind::Relation, SemanticSort::Relation, true),
            ]
        );
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantified_sumti_fragment_keeps_its_question_inside_typed_scope() {
        let graph = semantic_graph_for(".i pa mlana be ma");
        let question = graph
            .objects
            .values()
            .find_map(SemanticObject::as_question)
            .expect("quantified fragment direct question");
        assert_eq!(question.kind, QuestionKind::Argument);
        assert_eq!(question.mode, QuestionMode::Direct);
        assert_eq!(question.slots.len(), 1);
        let parameter = question.slots[0].parameter().expect("ma parameter");
        let data!(FormulaNode::Quantified(scope)) = graph.objects[&question.body]
            .as_formula()
            .expect("quantified fragment formula")
            .as_data()
        else {
            panic!("fragment content must retain its cardinality scope");
        };
        assert_eq!(scope.operator, FormulaOperator::Cardinality);
        assert!(graph.objects.values().any(|object| {
            object.as_predication().is_some_and(|predication| {
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "mlana")
                    && predication
                        .arguments
                        .values()
                        .any(|argument| argument.value == Some(parameter))
            })
        }));
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_and_connected_quantified_sumti_fragments_keep_their_scopes() {
        for source in ["tu'a so'i da", "mi joi noda"] {
            let graph = semantic_graph_for(source);
            let content = graph
                .objects
                .get(&graph.root)
                .and_then(SemanticObject::as_utterance)
                .and_then(|utterance| utterance.content)
                .expect("sumti fragment content");
            assert_eq!(content.object_kind(), SemanticObjectKind::Formula);
            assert!(graph.objects.values().any(|object| {
                matches!(
                    object.as_formula().map(FormulaNode::as_data),
                    Some(data!(FormulaNode::Quantified(_)))
                )
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pro_bridi_replay_drops_an_overridden_question_argument_slot() {
        let graph = semantic_graph_for("lu ma do tavla .i mi go'i li'u");
        let questions = graph
            .objects
            .values()
            .filter_map(SemanticObject::as_question)
            .collect::<Vec<_>>();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].kind, QuestionKind::Argument);
        assert_eq!(questions[0].slots.len(), 1);
        assert_eq!(
            questions[0]
                .common
                .source
                .as_ref()
                .and_then(|source| source.text.as_deref()),
            Some("ma do tavla")
        );
        assert!(crate::model::semantic_object_question_slots_are_valid(
            &graph.objects
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_ki_cancels_a_prior_tense_anchor_question() {
        let graph = semantic_graph_for("ca ma ki klama");
        let utterance = graph.objects[&graph.root]
            .as_utterance()
            .expect("single bridi utterance");
        assert_eq!(utterance.force, UtteranceForce::Assert);
        assert_eq!(
            utterance.content.map(SemanticObjectId::object_kind),
            Some(SemanticObjectKind::Formula)
        );
        assert!(
            graph
                .objects
                .values()
                .all(|object| object.as_question().is_none())
        );
        assert!(graph.objects.values().all(|object| {
            object
                .as_parameter()
                .is_none_or(|parameter| parameter.role != ParameterRole::ArgumentQuestion)
        }));
        let klama = graph
            .objects
            .values()
            .filter_map(SemanticObject::as_predication)
            .find(|predication| {
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
            })
            .expect("klama predication");
        let event = klama
            .eventuality
            .and_then(|event| graph.objects[&event].as_eventuality())
            .expect("klama eventuality");
        assert!(event.time.is_none());
        assert!(event.time_path.is_empty());
        assert!(event.space.is_none());
        assert!(event.space_path.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keha_is_typed_outside_relatives_and_crosses_abstraction_boundaries() {
        let property = semantic_graph_for("ka ke'a pilno ce'u");
        let pilno = named_predication_ids(&property, "pilno");
        assert_eq!(pilno.len(), 1);
        let pilno = property.objects[&pilno[0]]
            .as_predication()
            .expect("pilno predication");
        let relative_head = pilno.arguments[&argument_key(1)]
            .value
            .expect("ke'a fills pilno x1");
        let property_slot = pilno.arguments[&argument_key(2)]
            .value
            .expect("ce'u fills pilno x2");
        assert_ne!(relative_head, property_slot);
        assert_eq!(
            property.objects[&relative_head]
                .as_parameter()
                .map(|parameter| parameter.role),
            Some(ParameterRole::RelativeClauseHead)
        );
        assert_eq!(
            property.objects[&property_slot]
                .as_parameter()
                .map(|parameter| parameter.role),
            Some(ParameterRole::PropertySlot)
        );

        let nested = semantic_graph_for("lo mlatu poi mi djica lo nu do viska ke'a ku'o cu melbi");
        let mlatu = named_predication_ids(&nested, "mlatu");
        assert_eq!(mlatu.len(), 1);
        let head = nested.objects[&mlatu[0]]
            .as_predication()
            .and_then(|predication| predication.arguments[&argument_key(1)].value)
            .expect("mlatu description head");
        let viska = named_predication_ids(&nested, "viska");
        assert_eq!(viska.len(), 1);
        let nested_keha = nested.objects[&viska[0]]
            .as_predication()
            .and_then(|predication| predication.arguments[&argument_key(2)].value)
            .expect("nested ke'a fills viska x2");
        assert_eq!(
            nested_keha, head,
            "CLL 8.1 ke'a must retain the concrete relative head through NU"
        );
        assert_eq!(nested_keha.object_kind(), SemanticObjectKind::Referent);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modal_statement_relative_keeps_both_branches_and_connection_claim() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(zantufa)").expect("Zantufa dialect");
        let options = jbotci_syntax::ParseOptions::default().with_dialect_definition(&dialect);
        let graph = semantic_result_for_with_parse_options(
            "lo sinxa noi cukla milxe .i ba bo vi fa'u va punji lo ro mei lo pluta ku'o cu se finti",
            &options,
        )
        .expect("modal statement relative should build semantics");
        let sinxa = named_predication_ids(&graph, "sinxa");
        assert_eq!(sinxa.len(), 1);
        let head = graph.objects[&sinxa[0]]
            .as_predication()
            .and_then(|predication| predication.arguments[&argument_key(1)].value)
            .expect("sinxa description head");
        let relative = graph.objects[&head]
            .descriptor()
            .and_then(|descriptor| descriptor.relative_clauses.first())
            .expect("NOI statement relative clause");
        let data!(FormulaNode::Connective(body)) = graph.objects[&relative.body]
            .as_formula()
            .expect("relative body formula")
            .as_data()
        else {
            panic!("modal statement relative must have a connective formula body");
        };
        assert_eq!(body.operator, FormulaOperator::And);
        assert_eq!(body.children.len(), 3);

        let milxe = named_predication_ids(&graph, "milxe");
        let punji = named_predication_ids(&graph, "punji");
        let after = named_predication_ids(&graph, "after");
        assert_eq!((milxe.len(), punji.len(), after.len()), (1, 1, 1));
        assert!(formula_contains_predication(
            &graph,
            body.children[0],
            milxe[0]
        ));
        assert!(formula_contains_predication(
            &graph,
            body.children[1],
            punji[0]
        ));
        assert_eq!(
            graph.objects[&body.children[2]].formula_predication(),
            Some(after[0])
        );
        assert_eq!(
            graph.objects[&milxe[0]]
                .as_predication()
                .and_then(|predication| predication.arguments[&argument_key(1)].value),
            Some(head),
            "the relative head fills the first branch rather than the connection claim"
        );

        let leading_event = graph.objects[&milxe[0]]
            .as_predication()
            .and_then(|predication| predication.eventuality)
            .expect("milxe event");
        let trailing_event = graph.objects[&punji[0]]
            .as_predication()
            .and_then(|predication| predication.eventuality)
            .expect("punji event");
        let after = graph.objects[&after[0]]
            .as_predication()
            .expect("after connection claim");
        assert_eq!(
            after.arguments[&argument_key(1)].value,
            Some(trailing_event)
        );
        assert_eq!(after.arguments[&argument_key(2)].value, Some(leading_event));
        assert_eq!(
            graph.objects[&trailing_event]
                .as_eventuality()
                .map(|event| event.space_path.len()),
            Some(2),
            "vi fa'u va remains attached to the trailing branch event"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn naku_relative_phrase_negates_the_association_restriction() {
        let graph = semantic_graph_for("le gerku pe naku cu klama ti");
        let klama = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                    .then_some(predication)
            })
            .expect("matrix klama predication");
        let dog = klama.arguments[&argument_key(1)]
            .value
            .expect("description argument");
        let descriptor = graph.objects[&dog]
            .descriptor()
            .expect("dog description descriptor");
        let relative = descriptor
            .relative_clauses
            .first()
            .expect("pe naku relative phrase");
        let data!(FormulaNode::Connective(negation)) = graph.objects[&relative.body]
            .as_formula()
            .expect("relative phrase formula")
            .as_data()
        else {
            panic!("pe naku should wrap the association in negation");
        };
        assert_eq!(negation.operator, FormulaOperator::Not);
        assert_eq!(negation.children.len(), 1);
        let association = graph.objects[&negation.children[0]]
            .formula_predication()
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("negated association predication");
        assert!(matches!(
            association.relation.as_data(),
            data!(crate::model::PredicationRelation::Named { relation }) if relation == "associatedWith"
        ));
        assert_eq!(association.mode, PredicationMode::Restrictive);
        assert_eq!(association.arguments[&argument_key(1)].value, Some(dog));
        let associated = association.arguments[&argument_key(2)]
            .value
            .expect("elided associated object");
        assert_eq!(
            graph.objects[&associated].referent_category(),
            Some(ReferentCategory::Constant)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restricted_universal_elisions_may_depend_on_the_quantifier() {
        let graph = semantic_graph_for("ro mlatu cu jbena");
        let variable = forall_variable(&graph);
        let constants = constant_argument_ids(&graph, "jbena");

        assert!(
            constants.len() >= 1,
            "jbena should have elided constant arguments"
        );
        for constant in &constants {
            assert_underspecified_scope(&graph, *constant, &[variable]);
        }
        let tree_proj = crate::render::render_tree_proj(&graph);
        assert!(tree_proj.contains("binder-dependence=underspecified; may-depend-on="));
        for constant in constants {
            assert!(!tree_proj.lines().any(|line| {
                line.starts_with("- exists ") && line.contains(&format!("[{constant}]"))
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn negated_quantified_elisions_keep_the_enclosing_binder() {
        let graph = semantic_graph_for("naku ro da poi mlatu cu klama");
        let variable = forall_variable(&graph);
        let constants = constant_argument_ids(&graph, "klama");

        assert!(
            constants.len() >= 1,
            "klama should have elided constant arguments"
        );
        for constant in &constants {
            assert_underspecified_scope(&graph, *constant, &[variable]);
        }
        let tree_proj = crate::render::render_tree_proj(&graph);
        for constant in constants {
            assert!(!tree_proj.lines().any(|line| {
                line.starts_with("- exists ") && line.contains(&format!("[{constant}]"))
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn top_level_explicit_and_elided_constants_are_explicitly_fixed() {
        let graph = semantic_graph_for("mi klama zo'e");
        let constants = constant_argument_ids(&graph, "klama");

        assert!(
            constants.len() >= 1,
            "klama should include the explicit zo'e"
        );
        for constant in &constants {
            let object = graph.objects.get(constant).expect("argument exists");
            assert!(matches!(
                object.scope_dependence().map(ScopeDependence::as_data),
                Some(data!(ScopeDependence::Fixed))
            ));
            assert_eq!(
                serde_json::to_value(object).expect("constant should serialize")["scopeDependence"],
                serde_json::json!({ "kind": "fixed" })
            );
        }
        let tree_proj = crate::render::render_tree_proj(&graph);
        assert!(tree_proj.contains("binder-dependence=fixed; constant"));
        for constant in constants {
            assert!(!tree_proj.lines().any(|line| {
                line.starts_with("- exists ") && line.contains(&format!("[{constant}]"))
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn description_introduced_under_quantifier_may_depend_on_it() {
        let graph = semantic_graph_for("ro da cu viska lo mlatu");
        let variable = forall_variable(&graph);
        let descriptions = graph
            .objects
            .iter()
            .filter_map(|(id, object)| {
                (object.referent_category() == Some(ReferentCategory::Constant)
                    && object
                        .descriptor()
                        .is_some_and(|descriptor| descriptor.word == "lo"))
                .then_some(*id)
            })
            .collect::<Vec<_>>();

        assert_eq!(descriptions.len(), 1, "the sentence has one lo description");
        assert_underspecified_scope(&graph, descriptions[0], &[variable]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_universal_does_not_emit_domain_import() {
        let graph = semantic_graph_for("ro da zo'u da go broda gi brode");
        let quantified = graph
            .objects
            .values()
            .find(|object| object.formula_operator() == Some(FormulaOperator::Forall))
            .expect("bare forall formula");

        assert_eq!(quantified.formula_restriction(), None);
        assert_eq!(quantified.formula_domain_import(), None);
        assert!(graph.objects.values().all(|object| {
            serde_json::to_value(object)
                .expect("object should serialize")
                .get("domainImport")
                .is_none()
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restricted_existential_does_not_emit_domain_import() {
        let graph = semantic_graph_for("su'o da poi mlatu cu klama");
        let quantified = graph
            .objects
            .values()
            .find(|object| {
                object.formula_operator() == Some(FormulaOperator::Cardinality)
                    && object.formula_restriction().is_some()
            })
            .expect("restricted su'o cardinality formula");

        assert_eq!(quantified.formula_domain_import(), None);
        assert!(
            serde_json::to_value(quantified)
                .expect("formula should serialize")
                .get("domainImport")
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn restricted_plural_universal_emits_projective_domain_import() {
        let quantified = SemanticObject::quantified_formula(
            FormulaOperator::PluralForall,
            SemanticObjectId::referent(1),
            Some(SemanticObjectId::formula(2)),
            SemanticObjectId::formula(3),
            None,
            None,
            Vec::new(),
        );

        assert_eq!(
            quantified.formula_domain_import(),
            Some(DomainImport::Projective)
        );
        assert_eq!(
            serde_json::to_value(quantified).expect("formula should serialize")["domainImport"],
            serde_json::json!("projective")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn token_list_text_allows_empty_token_lists_explicitly() {
        assert_eq!(token_list_text(std::iter::empty::<&Token>()), "");
        assert_eq!(
            non_empty_token_list_text(std::iter::empty::<&Token>()),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nu_initial_zei_compound_relation_keeps_unknown_place_structure() {
        const SOURCE: &str = "lo nu zei broda cu brode";
        const UNKNOWN_PLACE_STRUCTURE_WARNING: &str = "relation place structure is unavailable; only places required by explicit assignments are represented";

        let words = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
            SOURCE,
            &jbotci_morphology::MorphologyOptions::default(),
            None,
        )
        .expect("source should segment");
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            SOURCE,
            &jbotci_syntax::ParseOptions::default(),
        )
        .expect("source should parse");
        let graph = build_generated_semantic_graph_with_dictionary(
            &syntax,
            Some(SOURCE),
            jbotci_dictionary_data::english(),
        )
        .expect("source should build semantics");

        let compound_predication = graph
            .objects
            .values()
            .find(|object| {
                object.as_predication().is_some_and(|predication| {
                    match predication.relation.as_data() {
                        data!(crate::model::PredicationRelation::Named { relation }) => {
                            relation.starts_with("cmavo:nu-") && relation.contains("-gismu:")
                        }
                        data!(crate::model::PredicationRelation::Parameter { .. }) => false,
                    }
                })
            })
            .expect("nu-initial ZEI compound relation should be present");

        let argument_places = compound_predication
            .predication_arguments()
            .expect("compound object should be a predication")
            .keys()
            .map(|place| place.get())
            .collect::<Vec<_>>();
        assert_eq!(argument_places, vec![1]);
        assert!(
            compound_predication
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message == UNKNOWN_PLACE_STRUCTURE_WARNING),
            "ZEI compound should not inherit the one-place nu abstraction structure",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_abstraction_implicit_property_slot_uses_branch_source() {
        const SOURCE: &str = "lo nu je ka broda cu brode";

        let words = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
            SOURCE,
            &jbotci_morphology::MorphologyOptions::default(),
            None,
        )
        .expect("source should segment");
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            SOURCE,
            &jbotci_syntax::ParseOptions::default(),
        )
        .expect("source should parse");
        let graph = build_generated_semantic_graph_with_dictionary(
            &syntax,
            Some(SOURCE),
            jbotci_dictionary_data::english(),
        )
        .expect("source should build semantics");

        let parameter = graph
            .objects
            .values()
            .find(|object| {
                object.as_parameter().is_some_and(|parameter| {
                    parameter.role == ParameterRole::PropertySlot
                        && parameter.introduced_by == "implicit ce'u"
                })
            })
            .expect("connected ka branch should synthesize an implicit property slot");
        let source = parameter
            .source()
            .expect("implicit property slot should have source");
        assert_eq!(source.text.as_deref(), Some("ka broda"));
        assert_eq!(source.span.byte_start, 9);
        assert_eq!(source.span.byte_end, 17);
        assert_eq!(source.construct.as_deref(), Some("implicit-property-slot"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn question_sumti_survive_relative_goi_and_fragment_lowering() {
        let relative = semantic_graph_for("ma poi cinri ku'o vi do fasnu");
        let parameter = relative
            .objects
            .iter()
            .find_map(|(&id, object)| {
                object
                    .as_parameter()
                    .is_some_and(|parameter| parameter.role == ParameterRole::ArgumentQuestion)
                    .then_some(id)
            })
            .expect("ma should produce an argument-question parameter");
        for relation in ["cinri", "fasnu"] {
            assert!(relative.objects.values().any(|object| {
                object.as_predication().is_some_and(|predication| {
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                        && predication
                            .arguments
                            .values()
                            .any(|argument| argument.value == Some(parameter))
                })
            }));
        }

        let goi = semantic_graph_for("ma goi ko'a cu klama ko'a");
        let question = goi
            .objects
            .iter()
            .find_map(|(&id, object)| {
                object
                    .as_parameter()
                    .is_some_and(|parameter| parameter.role == ParameterRole::ArgumentQuestion)
                    .then_some(id)
            })
            .expect("goi should preserve the question parameter");
        let klama = goi
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                    .then_some(predication)
            })
            .expect("klama predication should exist");
        assert_eq!(klama.arguments[&argument_key(1)].value, Some(question));
        assert_eq!(klama.arguments[&argument_key(2)].value, Some(question));

        let fragment = semantic_graph_for("ma");
        let parameter = fragment
            .objects
            .iter()
            .find_map(|(&id, object)| object.as_parameter().is_some().then_some(id))
            .expect("bare ma should remain a parameter");
        assert_eq!(
            fragment
                .objects
                .get(&fragment.root)
                .and_then(SemanticObject::as_utterance)
                .and_then(|utterance| utterance.content),
            Some(parameter)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn question_and_deleted_arguments_keep_their_typed_semantics() {
        let respectively = semantic_graph_for("ma fa'u ma klama ma fa'u ma");
        assert_eq!(
            respectively
                .objects
                .values()
                .filter(|object| {
                    object
                        .as_parameter()
                        .is_some_and(|parameter| parameter.role == ParameterRole::ArgumentQuestion)
                })
                .count(),
            4
        );
        assert_eq!(
            respectively
                .objects
                .values()
                .filter_map(SemanticObject::referent_composition)
                .filter(|composition| {
                    composition.operator == CompositionOperator::Respectively
                        && composition.members.len() == 2
                        && composition.members.iter().all(|member| {
                            respectively.objects.get(member).is_some_and(|object| {
                                object.as_parameter().is_some_and(|parameter| {
                                    parameter.role == ParameterRole::ArgumentQuestion
                                })
                            })
                        })
                })
                .count(),
            2
        );

        let indirect = semantic_graph_for("mi na djuno le makau mukti");
        let makau = indirect
            .objects
            .iter()
            .find_map(|(&id, object)| {
                object
                    .as_parameter()
                    .is_some_and(|parameter| {
                        parameter.role == ParameterRole::ArgumentQuestion
                            && parameter.introduced_by == "ma"
                    })
                    .then_some(id)
            })
            .expect("makau should produce an argument-question parameter");
        let associated_with = indirect
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "associatedWith")
                    .then_some(predication)
            })
            .expect("possessive makau should produce an association restriction");
        assert_eq!(
            associated_with.arguments[&argument_key(2)].value,
            Some(makau)
        );
        assert!(indirect.objects.values().any(|object| {
            object.as_question().is_some_and(|question| {
                question.slots.iter().any(|slot| {
                    slot.parameter() == Some(makau) && slot.role() == QuestionSlotRole::Answer
                })
            })
        }));

        let deleted = semantic_graph_for("gugde fi zi'o");
        let gugde = deleted
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "gugde")
                    .then_some(predication)
            })
            .expect("gugde predication should exist");
        let x3 = &gugde.arguments[&argument_key(3)];
        assert_eq!(x3.kind, ArgumentValueKind::Deleted);
        assert_eq!(x3.value, None);
        assert_eq!(x3.introduced_by.as_deref(), Some("zi'o"));

        let me = semantic_graph_for("me ma");
        let referent_of = me
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "referentOf")
                    .then_some(predication)
            })
            .expect("me should lower to referentOf");
        let source = referent_of.arguments[&argument_key(2)]
            .value
            .expect("referentOf source is filled");
        assert!(me.objects.get(&source).is_some_and(|object| {
            object
                .as_parameter()
                .is_some_and(|parameter| parameter.role == ParameterRole::ArgumentQuestion)
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multi_item_lu_quote_has_an_utterance_anchor() {
        let graph = semantic_graph_for("lu mi klama i do cadzu li'u cu se cusku mi");
        let quoted_utterance = graph
            .objects
            .values()
            .find_map(|object| object.as_sign()?.quotation.as_ref()?.utterance)
            .expect("LU quotation should point at an utterance");
        let quoted_content = graph
            .objects
            .get(&quoted_utterance)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("quoted utterance should contain its discourse");
        assert_eq!(quoted_content.object_kind(), SemanticObjectKind::Sequence);
        assert_eq!(
            graph
                .objects
                .get(&quoted_content)
                .and_then(SemanticObject::as_sequence)
                .map(|sequence| sequence.items.len()),
            Some(2)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sign_relative_clauses_and_restrictive_termsets_are_preserved() {
        let sign_graph = semantic_graph_for("xu zo irc poi lojbo cmene");
        let sign = sign_graph
            .objects
            .values()
            .find_map(SemanticObject::as_sign)
            .expect("zo should produce a sign");
        assert_eq!(sign.relative_clauses.len(), 1);
        assert_eq!(
            sign.relative_clauses[0].body.object_kind(),
            SemanticObjectKind::Formula
        );

        let termset = semantic_graph_for(
            "la blabi ractu noi jgari nu'i ge lo tabra lo xance gi lo skapi te ciska clanu le drata",
        );
        for relation in ["tabra", "skapi"] {
            assert!(termset.objects.values().any(|object| {
                object.as_predication().is_some_and(|predication| {
                    predication.mode == PredicationMode::Restrictive
                        && matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                })
            }));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_modal_terms_attach_to_every_connected_bridi_branch() {
        let graph = semantic_graph_for("va'o le nu do klama ku mi cu cadzu gi'e tavla");
        let mut modal_values = Vec::new();
        for relation in ["cadzu", "tavla"] {
            let predication = graph
                .objects
                .values()
                .find_map(|object| {
                    let predication = object.as_predication()?;
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                        .then_some(predication)
                })
                .expect("connected branch predication should exist");
            let modal = predication
                .modal_arguments
                .iter()
                .find(|modal| modal.relation.as_deref() == Some("va'o"))
                .expect("shared va'o term should attach to every branch");
            modal_values.push(
                modal.arguments[&argument_key(1)]
                    .value
                    .expect("va'o condition should be filled"),
            );
        }
        assert_eq!(modal_values[0], modal_values[1]);
        assert_eq!(
            modal_values[0].referent_sort(),
            Some(SemanticSort::Eventuality(EventualitySort::General))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_tense_term_constrains_every_connected_bridi_branch() {
        let graph = semantic_graph_for("i caku do zvati le zdani gi'i gunka");
        let mut events = Vec::new();
        for relation in ["zvati", "gunka"] {
            let predication = graph
                .objects
                .values()
                .find_map(|object| {
                    let predication = object.as_predication()?;
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                        .then_some(predication)
                })
                .expect("connected branch predication should exist");
            let event_id = predication
                .eventuality
                .expect("connected branch should have an eventuality");
            let event = graph.objects[&event_id]
                .as_eventuality()
                .expect("branch eventuality should be typed");
            let time = event
                .time
                .as_ref()
                .expect("shared ca ku must constrain every branch event");
            assert_eq!(time.relation, "at");
            assert_eq!(time.anchor, SemanticObjectId::now());
            events.push(event_id);
        }
        assert_ne!(
            events[0], events[1],
            "the shared condition must be copied onto distinct branch events"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_forethought_term_survives_nested_bo_bridi_grouping() {
        let graph = semantic_graph_for("mi ga broda gi brode gi'e ba bo brodi");
        for relation in ["broda", "brode", "brodi"] {
            let predication = graph
                .objects
                .values()
                .find_map(|object| {
                    let predication = object.as_predication()?;
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                        .then_some(predication)
                })
                .expect("every forethought/BO branch should retain its predication");
            assert_eq!(
                predication.arguments[&argument_key(1)].value,
                Some(SemanticObjectId::speaker()),
                "the shared mi term must fill x1 in {relation}"
            );
        }
        let content = graph.objects[&graph.root]
            .as_utterance()
            .and_then(|utterance| utterance.content)
            .expect("forethought bridi assertion content");
        let data!(FormulaNode::Connective(outer)) = graph.objects[&content]
            .as_formula()
            .expect("forethought connection formula")
            .as_data()
        else {
            panic!("GA forethought bridi must remain a connective formula");
        };
        assert_eq!(outer.operator, FormulaOperator::Or);
        assert_eq!(outer.children.len(), 2);
        assert_eq!(
            outer
                .connector
                .as_ref()
                .map(|connector| (connector.source.as_str(), connector.locus.as_str())),
            Some(("ga gi", "bridi"))
        );
        assert_eq!(
            graph.objects[&outer.children[0]].formula_predication(),
            Some(named_predication_ids(&graph, "broda")[0])
        );

        let data!(FormulaNode::Connective(grouped_tail)) = graph.objects[&outer.children[1]]
            .as_formula()
            .expect("nested GIhE/BO branch formula")
            .as_data()
        else {
            panic!("the right forethought branch must retain GIhE/BO grouping");
        };
        assert_eq!(grouped_tail.operator, FormulaOperator::And);
        assert_eq!(grouped_tail.children.len(), 3);
        assert_eq!(
            grouped_tail
                .connector
                .as_ref()
                .map(|connector| (connector.source.as_str(), connector.locus.as_str())),
            Some(("gi'e ba bo", "bridiTail"))
        );
        assert_eq!(
            graph.objects[&grouped_tail.children[0]].formula_predication(),
            Some(named_predication_ids(&graph, "brode")[0])
        );
        assert_eq!(
            graph.objects[&grouped_tail.children[1]].formula_predication(),
            Some(named_predication_ids(&graph, "brodi")[0])
        );
        assert_eq!(
            graph.objects[&grouped_tail.children[2]]
                .formula_predication()
                .and_then(|predication| graph.objects.get(&predication))
                .and_then(SemanticObject::as_predication)
                .and_then(|predication| match predication.relation.as_data() {
                    data!(PredicationRelation::Named { relation }) => Some(relation.as_str()),
                    data!(PredicationRelation::Parameter { .. }) => None,
                }),
            Some("after")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_tense_terms_compose_with_the_following_tag_on_the_same_event() {
        let spatial = semantic_graph_for("vi bai broda");
        let broda = spatial
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "broda")
                    .then_some(predication)
            })
            .expect("broda predication should exist");
        let eventuality = broda.eventuality.expect("broda should have an eventuality");
        let event = spatial.objects[&eventuality]
            .as_eventuality()
            .expect("broda eventuality should be an event");
        let space = event
            .space
            .as_ref()
            .expect("the leading vi term must constrain the event");
        assert_eq!(space.relation, "distanceFrom");
        assert_eq!(space.anchor, SemanticObjectId::here());
        assert_eq!(space.distance.as_deref(), Some("short"));
        let bai = broda
            .modal_arguments
            .iter()
            .find(|argument| argument.relation.as_deref() == Some("bapli"))
            .expect("the following bai tag must remain attached");
        assert_eq!(bai.arguments[&argument_key(2)].value, Some(eventuality));

        let aspectual = semantic_graph_for("do ka'e ro roi tavla");
        let tavla_event = aspectual
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "tavla")
                    .then_some(predication.eventuality)
                    .flatten()
            })
            .and_then(|eventuality| aspectual.objects.get(&eventuality))
            .and_then(SemanticObject::as_eventuality)
            .expect("tavla should have an eventuality");
        assert_eq!(
            tavla_event
                .actuality
                .as_ref()
                .map(|actuality| actuality.kind),
            Some(ActualityKind::Capable),
            "the leading ka'e term must not be dropped"
        );
        assert_eq!(tavla_event.recurrence.len(), 1);
        assert_eq!(
            tavla_event.recurrence[0].kind,
            RecurrenceKind::OccurrenceCount,
            "the following ro roi tag must compose with ka'e"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_adverbial_terms_report_principled_undefined_semantics() {
        for (source, construct) in [
            (
                "mi klama noi'a broda",
                "an experimental NOIhA adverbial term",
            ),
            (
                "mi klama fi'oi broda",
                "an experimental FIhOI adverbial term",
            ),
            (
                "mi klama xoi mutce",
                "an experimental SOI/XOI adverbial term",
            ),
        ] {
            let error = semantic_result_for(source)
                .expect_err("experimental adverbials have no defined semantic lowering");
            assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
            assert_eq!(
                error.message,
                format!("semantic interpretation is undefined for {construct}")
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_sumti_bases_report_principled_undefined_semantics() {
        for (source, construct) in [
            (
                "mi tavla la'e fa do",
                "an experimental LAhE/NAhE term wrapper",
            ),
            (
                "mi tavla na'e bo fa do",
                "an experimental LAhE/NAhE term wrapper",
            ),
            (
                "mi tavla na'e fa do",
                "an experimental NAhE-prefixed FA tag",
            ),
            (
                "lo'oi mi klama ku'au cu melbi",
                "an experimental LOhOI/KUhAU bridi-description sumti",
            ),
            (
                "lo je le broda",
                "an experimental JA connection between descriptor heads",
            ),
        ] {
            let error = semantic_result_for(source)
                .expect_err("experimental sumti base has no defined semantic lowering");
            assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
            assert_eq!(
                error.message,
                format!("semantic interpretation is undefined for {construct}")
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_online_location_tag_anchors_the_host_event() {
        let graph = semantic_graph_for("xei'e lo kibro mi klama");
        let predication = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "klama")
                    .then_some(predication)
            })
            .expect("klama predication should exist");
        let speaker = graph
            .objects
            .iter()
            .find_map(|(id, object)| {
                (object.as_referent().and_then(|referent| referent.indexical)
                    == Some(IndexicalKind::Speaker))
                .then_some(*id)
            })
            .expect("speaker referent should exist");
        assert_eq!(
            predication.arguments[&argument_key(1)].value,
            Some(speaker),
            "the ordinary mi term must retain klama x1"
        );
        let event = graph
            .objects
            .get(&predication.eventuality.expect("klama should have an event"))
            .and_then(SemanticObject::as_eventuality)
            .expect("klama eventuality should exist");
        let space = event
            .space
            .as_ref()
            .expect("xei'e must create a spatial condition");
        assert_eq!(space.relation, "onlineWith");
        assert_eq!(
            graph
                .objects
                .get(&space.anchor)
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("lo kibro"),
            "the xei'e sumti must be the online-location anchor"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ki_does_not_silently_discard_a_tagged_sumti() {
        let error = semantic_result_for("i xu do gunka ki le do zdani vu ma doi tsali")
            .expect_err("KI alone defines a reset, not a relation to a following sumti");
        assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
        assert_eq!(
            error.message,
            "semantic interpretation is undefined for a KI reset tag applied to a sumti argument"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicate_linkarg_x1_expands_to_conjoined_restrictions() {
        let graph = semantic_graph_for("le gadri be fa zo le");
        let gadri = graph
            .objects
            .values()
            .filter_map(SemanticObject::as_predication)
            .filter(|predication| {
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "gadri")
            })
            .collect::<Vec<_>>();
        assert_eq!(gadri.len(), 2);
        assert!(
            gadri
                .iter()
                .all(|predication| predication.mode == PredicationMode::Restrictive)
        );
        let x1s = gadri
            .iter()
            .map(|predication| {
                predication.arguments[&argument_key(1)]
                    .value
                    .expect("both restrictions have x1")
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(x1s.len(), 2);
        assert!(graph.objects.values().any(|object| {
            object.formula_operator() == Some(FormulaOperator::And)
                && object.formula_children().len() == 2
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fai_restores_jai_displaced_argument_without_consuming_a_numbered_place() {
        let graph = semantic_graph_for("le panka cu jai vi citka le cirla fai le ratcu");
        let citka = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "citka")
                    .then_some(predication)
            })
            .expect("citka predication should exist");
        let restored_x1 = &citka.arguments[&argument_key(1)];
        let ordinary_x2 = &citka.arguments[&argument_key(2)];
        assert_eq!(restored_x1.kind, ArgumentValueKind::Filled);
        assert_eq!(ordinary_x2.kind, ArgumentValueKind::Filled);
        assert_eq!(
            restored_x1
                .value
                .and_then(|value| graph.objects.get(&value))
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("le ratcu")
        );
        assert_eq!(
            ordinary_x2
                .value
                .and_then(|value| graph.objects.get(&value))
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("le cirla")
        );

        let event = citka
            .eventuality
            .and_then(|event| graph.objects.get(&event))
            .and_then(SemanticObject::as_eventuality)
            .expect("JAI tense conversion should retain the citka eventuality");
        let anchor = event
            .space
            .as_ref()
            .map(|relation| relation.anchor)
            .expect("vi should retain the raised location as an event anchor");
        assert_eq!(
            graph
                .objects
                .get(&anchor)
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("le panka")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_linkargs_do_not_consume_or_drop_following_slots() {
        let graph = semantic_graph_for("lo broda be bei mi cu melbi");
        let broda = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "broda")
                    .then_some(predication)
            })
            .expect("broda restriction should exist");
        assert_eq!(broda.arguments.len(), 2);
        assert_eq!(
            broda.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::speaker())
        );
        assert_eq!(
            broda.arguments[&argument_key(2)].kind,
            ArgumentValueKind::Filled
        );

        for source in ["lo broda be cu melbi", "lo broda be mi bei cu melbi"] {
            semantic_result_for(source).expect("a trailing empty linkarg should be zero-width");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_tagged_linkarg_constrains_the_relation_event_without_consuming_a_place() {
        let graph = semantic_graph_for("le viska be mi bei ca le nu do klama cu melbi");
        let viska = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "viska")
                    .then_some(predication)
            })
            .expect("viska restriction should exist");
        assert_eq!(
            viska.arguments[&argument_key(2)].value,
            Some(SemanticObjectId::speaker()),
            "the ordinary BE argument must remain x2"
        );
        assert_eq!(
            viska.arguments[&argument_key(2)].kind,
            ArgumentValueKind::Filled
        );

        let event = viska
            .eventuality
            .and_then(|eventuality| graph.objects.get(&eventuality))
            .and_then(SemanticObject::as_eventuality)
            .expect("the tense-tagged link must allocate a viska eventuality");
        let anchor = event
            .time
            .as_ref()
            .map(|time| time.anchor)
            .expect("ca must constrain the viska eventuality");
        assert_eq!(
            graph
                .objects
                .get(&anchor)
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("le nu do klama"),
            "the linked sumti must be the temporal anchor rather than being dropped"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_tense_terms_copy_the_bridi_and_keep_both_event_anchors() {
        let graph = semantic_graph_for("pu ko'a .e ba ko'e broda");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let data!(FormulaNode::Connective(connection)) = graph.objects[&content]
            .as_formula()
            .expect("direct term connection formula")
            .as_data()
        else {
            panic!("connected terms should distribute the bridi");
        };
        assert_eq!(connection.operator, FormulaOperator::And);
        assert_eq!(connection.children.len(), 2);
        assert_eq!(
            connection
                .connector
                .as_ref()
                .map(|connector| (connector.source.as_str(), connector.locus.as_str())),
            Some(("e", "term"))
        );

        let branches = connection
            .children
            .iter()
            .map(|formula| {
                let predication = graph.objects[formula]
                    .formula_predication()
                    .and_then(|predication| graph.objects.get(&predication))
                    .and_then(SemanticObject::as_predication)
                    .expect("distributed broda predication");
                assert!(matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "broda"));
                let event = predication
                    .eventuality
                    .and_then(|event| graph.objects.get(&event))
                    .and_then(SemanticObject::as_eventuality)
                    .expect("each branch has its own eventuality");
                let time = event.time.as_ref().expect("each term constrains its branch");
                let anchor_source = graph
                    .objects
                    .get(&time.anchor)
                    .and_then(SemanticObject::source)
                    .and_then(|source| source.text.as_deref())
                    .expect("the tense anchor retains its source");
                (time.relation.as_str(), anchor_source)
            })
            .collect::<Vec<_>>();
        assert_eq!(branches, [("before", "ko'a"), ("after", "ko'e")]);
        assert_ne!(
            graph.objects[&connection.children[0]]
                .formula_predication()
                .and_then(|predication| graph.objects[&predication].as_predication())
                .and_then(|predication| predication.eventuality),
            graph.objects[&connection.children[1]]
                .formula_predication()
                .and_then(|predication| graph.objects[&predication].as_predication())
                .and_then(|predication| predication.eventuality),
            "the two event conditions must not collapse onto one copied predication"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_modal_anchor_sumti_copy_the_bridi_and_relate_branch_events() {
        let graph = semantic_graph_for("pu ko'a .e ba bo ko'e broda");
        let content = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .expect("assertion content");
        let data!(FormulaNode::Connective(connection)) = graph.objects[&content]
            .as_formula()
            .expect("sumti connection formula")
            .as_data()
        else {
            panic!("the connected modal anchor should distribute the bridi");
        };
        assert_eq!(connection.operator, FormulaOperator::And);
        assert_eq!(connection.children.len(), 3);
        assert_eq!(
            connection
                .connector
                .as_ref()
                .map(|connector| (connector.source.as_str(), connector.locus.as_str())),
            Some(("e ba bo", "sumti"))
        );

        let branch_events = connection.children[..2]
            .iter()
            .map(|formula| {
                let predication = graph.objects[formula]
                    .formula_predication()
                    .and_then(|predication| graph.objects.get(&predication))
                    .and_then(SemanticObject::as_predication)
                    .expect("distributed broda predication");
                assert!(matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "broda"));
                let event_id = predication
                    .eventuality
                    .expect("each copied bridi has its own event");
                let event = graph.objects[&event_id]
                    .as_eventuality()
                    .expect("branch event");
                let time = event
                    .time
                    .as_ref()
                    .expect("pu must constrain every copied branch");
                assert_eq!(time.relation, "before");
                let anchor_source = graph.objects[&time.anchor]
                    .source()
                    .and_then(|source| source.text.as_deref())
                    .expect("branch anchor source");
                (event_id, anchor_source)
            })
            .collect::<Vec<_>>();
        assert_eq!(branch_events[0].1, "ko'a");
        assert_eq!(branch_events[1].1, "ko'e");
        assert_ne!(branch_events[0].0, branch_events[1].0);

        let connection_claim = graph.objects[&connection.children[2]]
            .formula_predication()
            .and_then(|predication| graph.objects.get(&predication))
            .and_then(SemanticObject::as_predication)
            .expect("ba must become a claim relating the branch events");
        assert!(
            matches!(connection_claim.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "after")
        );
        assert_eq!(
            connection_claim.arguments[&argument_key(1)].value,
            Some(branch_events[1].0)
        );
        assert_eq!(
            connection_claim.arguments[&argument_key(2)].value,
            Some(branch_events[0].0)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recurrent_connected_sumti_copy_a_compound_tanru_and_constrain_the_second_event() {
        let graph = semantic_graph_for(
            "mi lifri gi'e di'a senva sezysku lu broda li'u .e su'o roi bo lu brode li'u",
        );
        let lifri_ids = named_predication_ids(&graph, "lifri");
        let [lifri] = lifri_ids.as_slice() else {
            panic!("the shared branch must contain exactly one lifri predication");
        };
        let lifri = graph.objects[lifri]
            .as_predication()
            .expect("lifri predication");
        assert_eq!(
            lifri.arguments[&argument_key(1)].value,
            Some(SemanticObjectId::speaker())
        );
        assert_eq!(
            lifri.arguments[&argument_key(2)].kind,
            ArgumentValueKind::Elided,
            "the linked quotations are local to sezysku and must not leak into lifri"
        );
        let mut branches = named_predication_ids(&graph, "sezysku")
            .into_iter()
            .map(|predication| {
                let predication = graph.objects[&predication]
                    .as_predication()
                    .expect("sezysku branch predication");
                assert_eq!(
                    predication.arguments[&argument_key(1)].value,
                    Some(SemanticObjectId::speaker()),
                    "the shared leading mi must reach every linked sezysku branch"
                );
                let sign = predication.arguments[&argument_key(2)]
                    .value
                    .expect("each copied tanru retains its quote argument");
                let sign_source = graph.objects[&sign]
                    .source()
                    .and_then(|source| source.text.as_deref())
                    .expect("quote source");
                let event = predication
                    .eventuality
                    .expect("each copied tanru head has an event");
                (sign_source, event)
            })
            .collect::<Vec<_>>();
        branches.sort_by_key(|(source, _)| *source);
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].0, "lu broda li'u");
        assert_eq!(branches[1].0, "lu brode li'u");
        assert_ne!(branches[0].1, branches[1].1);

        let first_event = graph.objects[&branches[0].1]
            .as_eventuality()
            .expect("first tanru event");
        let second_event = graph.objects[&branches[1].1]
            .as_eventuality()
            .expect("second tanru event");
        for (branch_index, event) in [first_event, second_event].into_iter().enumerate() {
            assert!(
                event
                    .aspect
                    .as_ref()
                    .is_some_and(|aspect| aspect.contour == "resumptive"),
                "the selbri-scoped di'a must reach linked tanru branch {branch_index}: {:?}",
                (branches[branch_index].1, &event.aspect)
            );
        }
        assert!(first_event.recurrence.is_empty());
        let [recurrence] = second_event.recurrence.as_slice() else {
            panic!("su'o roi must become exactly one recurrence condition");
        };
        assert_eq!(recurrence.kind, RecurrenceKind::OccurrenceCount);
        assert_eq!(recurrence.introduced_by, "roi");
        assert_eq!(recurrence.interval, Some(branches[0].1));
        let quantity = recurrence.quantity.expect("su'o supplies a quantity");
        let quantity = graph.objects[&quantity]
            .as_quantity()
            .expect("recurrence quantity is typed");
        assert_eq!(quantity.form, QuantityForm::AtLeast);
        assert_eq!(quantity.scale, QuantityScale::Frequency);
        assert_eq!(quantity.value, QuantityValue::text("su'o".to_owned()));

        assert!(graph.objects.values().any(|object| {
            object.formula_operator() == Some(FormulaOperator::And)
                && object.as_formula().is_some_and(|formula| {
                    matches!(formula.as_data(), data!(FormulaNode::Connective(formula)) if formula.connector.as_ref().is_some_and(|connector| connector.source == "e su'o roi bo" && connector.locus == "sumti"))
                })
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_fa_sumti_connection_tag_is_a_principled_error() {
        let error = semantic_result_for("ko'a .e fa bo ko'e")
            .expect_err("experimental FA tag semantics are undefined");
        assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
        assert_eq!(
            error.message,
            "semantic interpretation is undefined for an experimental FA tag in a sumti connection"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ji_connected_tense_terms_build_a_connective_question() {
        let graph = semantic_graph_for("pu ko'a ji ba ko'e broda");
        let question = graph
            .objects
            .get(&graph.root)
            .and_then(SemanticObject::as_utterance)
            .and_then(|utterance| utterance.content)
            .and_then(|content| graph.objects.get(&content))
            .and_then(SemanticObject::as_question)
            .expect("JI term connection should produce a direct connective question");
        let data!(FormulaNode::Connective(connection)) = graph.objects[&question.body]
            .as_formula()
            .expect("connective-question body")
            .as_data()
        else {
            panic!("JI term connection should distribute the bridi");
        };
        assert_eq!(connection.operator, FormulaOperator::ConnectiveQuestion);
        assert_eq!(connection.children.len(), 2);
        assert!(connection.connector.as_ref().is_some_and(|connector| {
            connector.source == "ji"
                && connector.locus == "term"
                && connector.truth_table.is_none()
                && connector.parameter.is_some()
        }));
        assert_eq!(named_predication_ids(&graph, "broda").len(), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_fiho_modals_copy_the_bridi_and_keep_typed_modal_bodies() {
        let graph = semantic_graph_for(".e'a casnu fi'o selsnu ja fi'o bangu la lojban");
        let casnu = named_predication_ids(&graph, "casnu");
        assert_eq!(casnu.len(), 2, "JA-connected FIhO tags copy the bridi");
        let mut modal_relations = BTreeSet::new();
        let mut shared_topic = None;
        for predication in casnu {
            let predication = graph.objects[&predication]
                .as_predication()
                .expect("casnu branch predication");
            let [modal] = predication.modal_arguments.as_slice() else {
                panic!("each casnu branch must retain exactly one FIhO modal body");
            };
            assert_eq!(modal.introduced_by, "fi'o");
            let body = modal.body.expect("FIhO modal keeps a typed body formula");
            assert_eq!(
                modal.component, predication.eventuality,
                "the FIhO modal component must be its copied casnu event"
            );
            let body = graph.objects[&body]
                .formula_predication()
                .and_then(|body| graph.objects.get(&body))
                .and_then(SemanticObject::as_predication)
                .expect("FIhO body is an atomic selbri formula");
            let data!(PredicationRelation::Named { relation }) = body.relation.as_data() else {
                panic!("FIhO body relation must stay named");
            };
            modal_relations.insert(relation.clone());
            let topic = body.arguments[&argument_key(1)]
                .value
                .expect("la lojban fills the FIhO relation x1");
            assert_eq!(*shared_topic.get_or_insert(topic), topic);
        }
        assert_eq!(
            modal_relations,
            BTreeSet::from(["bangu".to_owned(), "selsnu".to_owned()])
        );
        assert!(graph.objects.values().any(|object| {
            matches!(
                object.as_formula().map(FormulaNode::as_data),
                Some(data!(FormulaNode::Connective(connection)))
                    if connection.operator == FormulaOperator::Or
                        && connection.connector.as_ref().is_some_and(|connector| {
                            connector.locus == "modal"
                                && connector.source == "fi'o selsnu ja fi'o bangu"
                        })
            )
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connected_event_tense_terms_copy_the_bridi_and_constrain_each_event() {
        let graph = semantic_graph_for("mi pu je ba zu cu zasti");
        let mut zasti = named_predication_ids(&graph, "zasti")
            .into_iter()
            .map(|id| {
                let predication = graph.objects[&id]
                    .as_predication()
                    .expect("zasti branch predication");
                assert_eq!(
                    predication.arguments[&argument_key(1)].value,
                    Some(SemanticObjectId::speaker()),
                    "the leading mi term must be shared by both tense branches"
                );
                let eventuality = predication
                    .eventuality
                    .expect("each zasti branch has an eventuality");
                (
                    eventuality,
                    graph.objects[&eventuality].as_eventuality().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        zasti.sort_by_key(|(eventuality, _)| *eventuality);
        let [(first_id, first), (second_id, second)] = zasti.as_slice() else {
            panic!("JE-connected tense terms must build exactly two zasti events");
        };
        let first_time = first.time.as_ref().expect("pu constrains the first event");
        let second_time = second
            .time
            .as_ref()
            .expect("ba zu constrains the second event");
        assert_eq!(first_time.relation, "before");
        assert_eq!(second_time.relation, "after");
        assert_eq!(first_time.anchor, second_time.anchor);
        assert_eq!(first_time.distance, None);
        assert_eq!(second_time.distance.as_deref(), Some("long"));
        assert_ne!(
            first_id, second_id,
            "the branches need distinct eventualities"
        );
        assert!(graph.objects.values().any(|object| {
            matches!(
                object.as_formula().map(FormulaNode::as_data),
                Some(data!(FormulaNode::Connective(connection)))
                    if connection.operator == FormulaOperator::And
                        && connection.children.len() == 2
                        && connection.connector.as_ref().is_some_and(|connector| {
                            connector.locus == "tense"
                                && connector.source == "pu je ba zu"
                                && connector.truth_table.as_deref() == Some("TFFF")
                        })
            )
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fahu_connected_spatial_terms_preserve_parallel_event_conditions() {
        let graph = semantic_graph_for("punji le romei le pluta vi fa'u va ku");
        let mut punji = named_predication_ids(&graph, "punji")
            .into_iter()
            .map(|id| {
                let predication = graph.objects[&id]
                    .as_predication()
                    .expect("punji branch predication");
                let eventuality = predication
                    .eventuality
                    .expect("each punji branch has an eventuality");
                (
                    predication,
                    graph.objects[&eventuality].as_eventuality().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        punji.sort_by_key(|(predication, _)| predication.eventuality);
        let [(near_predication, near), (medium_predication, medium)] = punji.as_slice() else {
            panic!("FAhU-connected spatial terms must build exactly two punji events");
        };
        let near_space = near.space.as_ref().expect("vi constrains the first event");
        let medium_space = medium
            .space
            .as_ref()
            .expect("va constrains the second event");
        assert_eq!(near_space.relation, "distanceFrom");
        assert_eq!(medium_space.relation, "distanceFrom");
        assert_eq!(near_space.anchor, medium_space.anchor);
        assert_eq!(near_space.distance.as_deref(), Some("short"));
        assert_eq!(medium_space.distance.as_deref(), Some("medium"));
        for place in [2, 3] {
            assert_eq!(
                near_predication.arguments[&argument_key(place)].value,
                medium_predication.arguments[&argument_key(place)].value,
                "explicit place x{place} must be shared by both spatial branches"
            );
        }
        let distribution = graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::RespectivelyDistribution(distribution)) => Some(distribution),
                _ => None,
            })
            .expect("fa'u must remain a typed respectively distribution");
        let [branch_stream] = distribution.streams.as_slice() else {
            panic!("a tag-only fa'u connection has one parallel formula stream");
        };
        assert_eq!(branch_stream.items.len(), 2);
        let data!(FormulaNode::Connective(body)) = graph.objects[&distribution.body]
            .as_formula()
            .expect("respectively body is a formula")
            .as_data()
        else {
            panic!("respectively body must retain both spatial branches");
        };
        assert_eq!(body.operator, FormulaOperator::And);
        assert_eq!(body.children, branch_stream.items);
        assert!(body.connector.as_ref().is_some_and(|connector| {
            connector.locus == "tense"
                && connector.source == "vi fa'u va"
                && connector.truth_table.is_none()
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_termsets_fill_each_branch_place_without_dropping_shared_terms() {
        let graph = semantic_graph_for(
            "le pamoi zgana cu du la mapypre noi nerkla gi'e jgari nu'i ge lo tcati kabri le xance gi lo nanba joi matne le drata",
        );
        let jgari = named_predication_ids(&graph, "jgari");
        let [first, second] = jgari.as_slice() else {
            panic!("GE/GI termsets must build exactly two jgari branches");
        };
        let first = graph.objects[first].as_predication().unwrap();
        let second = graph.objects[second].as_predication().unwrap();
        assert_eq!(
            first.arguments[&argument_key(1)].value,
            second.arguments[&argument_key(1)].value,
            "the leading observer must be shared across termset branches"
        );
        let branch_sources = |predication: &crate::model::PredicationNode| {
            [2, 3]
                .map(|place| {
                    let value = predication.arguments[&argument_key(place)]
                        .value
                        .expect("termset branch fills both explicit places");
                    graph.objects[&value]
                        .source()
                        .and_then(|source| source.text.clone())
                        .expect("explicit sumti retains its source")
                })
                .to_vec()
        };
        let sources = [branch_sources(first), branch_sources(second)];
        assert!(sources.contains(&vec!["lo tcati kabri".to_owned(), "le xance".to_owned()]));
        assert!(sources.contains(&vec![
            "lo nanba joi matne".to_owned(),
            "le drata".to_owned(),
        ]));
        assert!(graph.objects.values().any(|object| {
            matches!(
                object.as_formula().map(FormulaNode::as_data),
                Some(data!(FormulaNode::Connective(connection)))
                    if connection.operator == FormulaOperator::And
                        && connection.children.len() == 2
                        && connection.connector.as_ref().is_some_and(|connector| {
                            connector.locus == "termset"
                                && connector.source == "ge gi"
                                && connector.truth_table.as_deref() == Some("TFFF")
                        })
            )
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_connected_term_fragments_require_the_missing_bridi() {
        let error = semantic_result_for("lu da .o de li'u du lu da .e de .a ke nada .e nade li'u")
            .expect_err(
                "the connected terms inside the second quotation have no bridi to distribute",
            );
        assert_eq!(error.kind, SemanticsErrorKind::RequiresDiscourseContext);
        assert_eq!(
            error.message,
            "semantic analysis of the missing bridi proposition distributed by standalone connected-term fragment `da e de a ke na da e na de` requires discourse context"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nonlogical_direct_term_connections_are_principled_errors() {
        for source in ["pu ko'a joi ba ko'e broda", "pu ko'a su'i ba ko'e broda"] {
            let error = semantic_result_for(source)
                .expect_err("experimental nonlogical direct term semantics are undefined");
            assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
            assert_eq!(
                error.message,
                "semantic interpretation is undefined for an experimental nonlogical direct term connection"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_jai_fai_preserves_its_argument_and_quantified_event_anchor() {
        let graph = semantic_graph_for("jai frili fai le nu krobi'o fa'a roda");
        let frili = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "frili")
                    .then_some(predication)
            })
            .expect("frili predication should exist");
        let restored = frili.arguments[&argument_key(1)]
            .value
            .expect("FAI should restore the explicit eventuality into x1");
        assert_eq!(
            graph
                .objects
                .get(&restored)
                .and_then(SemanticObject::source)
                .and_then(|source| source.text.as_deref()),
            Some("le nu krobi'o fa'a roda")
        );

        let quantified = graph
            .objects
            .values()
            .find_map(|object| match object.as_formula()?.as_data() {
                data!(FormulaNode::Quantified(node))
                    if node.operator == FormulaOperator::Forall =>
                {
                    Some(node.variable)
                }
                _ => None,
            })
            .expect("roda should retain a universal scope");
        let krobiho_event = graph
            .objects
            .values()
            .find_map(|object| {
                let predication = object.as_predication()?;
                matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation }) if relation == "krobi'o")
                    .then_some(predication.eventuality)
                    .flatten()
            })
            .expect("krobi'o predication should have an eventuality");
        assert_eq!(
            graph
                .objects
                .get(&krobiho_event)
                .and_then(SemanticObject::as_eventuality)
                .and_then(|event| event.space.as_ref())
                .map(|space| space.anchor),
            Some(quantified)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fai_does_not_cross_a_nested_bridi_scope_to_find_jai() {
        for source in [
            ".iku'i li'a la rab,n. poi jai mutce lakne fai zo'epeleka ce'u zvati cu ca cando",
            "mi pu pa roi jai mukti le nu vitke le mikce fai lo trixe nu cortu",
            ".i ni'o mi mutce jai mukti le nu cirke la tikik fai le nu mi cikre le ralju samselpla vreji",
        ] {
            let error = semantic_result_for(source)
                .expect_err("FAI without a JAI target in its own bridi must be rejected");
            assert_eq!(error.kind, SemanticsErrorKind::InvalidGraph);
            assert_eq!(
                error.message,
                "semantic interpretation is undefined for a fai term without a local JAI conversion target"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn abstraction_retargets_only_inherited_body_eventuality() {
        let graph = semantic_graph_for("le za'i mi lenku ki'u le nu le glavacri minji na banzu");
        let relation_event = |relation: &str| {
            graph
                .objects
                .values()
                .find_map(|object| {
                    let predication = object.as_predication()?;
                    matches!(predication.relation.as_data(), data!(crate::model::PredicationRelation::Named { relation: candidate }) if candidate == relation)
                        .then_some(predication.eventuality)
                        .flatten()
                })
                .expect("named predication should have an eventuality")
        };
        let state = relation_event("lenku");
        let event = relation_event("banzu");
        assert_ne!(state, event);
        assert_eq!(
            state.referent_sort(),
            Some(SemanticSort::Eventuality(EventualitySort::State))
        );
        assert!(graph.objects.contains_key(&state));
        assert!(graph.objects.contains_key(&event));
    }
}
