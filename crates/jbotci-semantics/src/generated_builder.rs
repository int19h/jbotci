//! Semantic builder that consumes the generated syntax model directly.

use std::collections::{BTreeMap, HashSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dictionary::Dictionary;
use jbotci_morphology::{Cmavo, Selmaho, Word, strip_diacritics};
use jbotci_source::SourceSpan;
use jbotci_syntax::generated_model::{
    AbstractionTanruUnitSyntax, ArgumentConnectiveSyntax, AtomRef as GeneratedAtomRef,
    BareCuBridiSyntax, BareCuTermsBridiSyntax, BoGroupedBridiTailSyntax, BoOrLinkedTanruUnitSyntax,
    BoundTanruUnitSyntax, BridiRelativeClauseSyntax, BridiStatementSyntax, BridiSubbridiSyntax,
    BridiSyntax, BridiTailSyntax, BridiTailWithPossibleTailTermsSyntax,
    BridiWithLeadingTermsSyntax, BridiWithPostCuTermsSyntax, CoSelbriSyntax, ConnectedSelbriSyntax,
    ConnectedTermSyntax, CuTermsBridiTailSyntax, DescriptionHeadSyntax, DescriptionTailBodySyntax,
    DescriptionTailSyntax, DescriptorWithGadriSumtiSyntax,
    DescriptorWithOuterQuantifierSumtiSyntax, DescriptorWithoutGadriSumtiSyntax,
    EkConnectiveSyntax, ForethoughtSelbriConnectionSyntax, ForethoughtSelbriGroupTanruUnitSyntax,
    FragmentStatementSyntax, FreeModifierSyntax, GikConnectiveSyntax, GohaWordTanruUnitSyntax,
    GroupedTanruUnitSyntax, GuhekConnectiveSyntax, IStatementConnectionSyntax,
    IStatementConnectionTailSyntax, IStatementConnectiveSyntax, JoikConnectiveSyntax,
    LaheSumtiSyntax, LerfuStringSumtiSyntax, LinkargsSyntax, LinkedSumtiSyntax,
    LinkedTanruUnitSyntax, NameSumtiSyntax, NumberSumtiSyntax, OrdinalTanruUnitSyntax,
    ParagraphSyntax, PlainRelativeSumtiSyntax, PreposedIStatementConnectionSyntax,
    ProBridiTanruUnitSyntax, ProSumtiSyntax, QuantifiedSumtiSyntax,
    QuantifierRelationDescriptionTailSyntax, QuantifierSumtiDescriptionTailSyntax,
    QuantifierSyntax, QuoteSyntax, QuotedSumtiSyntax, RegularTextSyntax,
    RelationAfterthoughtConnectiveSyntax, RelationDescriptionTailSyntax, RelationOnlyBridiSyntax,
    RelativeClauseAtomSyntax, RelativeClauseListSyntax, RelativeClauseTailSyntax,
    RelativeSumtiSyntax, RestrictiveBridiRelativeClauseSyntax, ScalarNegatedSumtiSyntax,
    ScalarNegatedSumtiWithBoSyntax, ScalarNegatedTanruInnerUnitSyntax,
    ScalarNegatedTanruUnitSyntax, SelbriSimpleBridiTailSyntax, SelbriSyntax, SimpleBridiTailSyntax,
    SimpleParagraphSyntax, SimpleSumtiSyntax, SimpleTermSyntax, StatementAfterIConnectiveSyntax,
    StatementBaseSyntax, StatementConnectiveSyntax, StatementOrFragmentStatementSyntax,
    StatementOrFragmentSyntax, StatementSyntax, SubbridiSyntax, SumtiAfterthoughtSyntax,
    SumtiAssociationRelativeClauseSyntax, SumtiAtomSyntax, SumtiBaseSyntax, SumtiBoundSyntax,
    SumtiForethoughtSyntax, SumtiGroupedSyntax, SumtiSelbriSumtiSyntax, SumtiSelbriTanruUnitSyntax,
    SumtiSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax, TaggedSumtiTermSyntax,
    TanruSelbriSyntax, TanruUnitAtomBaseSyntax, TanruUnitAtomSyntax, TanruUnitSyntax,
    TenseModalSyntax, TenseTaggedRelativeSumtiSyntax, TermSyntax, TermsFragmentSyntax,
    TextParagraphWithAdditionalNihoSyntax, TextParagraphsSyntax, TextSyntax, TreeNode,
    UntaggedSelbriSyntax, WordTanruUnitSyntax,
};
use jbotci_syntax::tree::{Token, WithFreeModifiers};
use jbotci_tree::TreeVisitor;

use crate::builder::{
    SemanticBuildOptions, SemanticsError, SemanticsErrorKind, dictionary_relation_place_count,
};
use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorRelation, ArgumentValue, ArgumentValueKind,
    CommandTarget, Composition, Connector, Descriptor, DescriptorDefiniteness, EventualityClass,
    EventualitySort, FormulaOperator, IndexicalKind, ModalArgument, ModalNegation,
    ModalNegationKind, ParameterRole, PredicationMode, QuantityForm, QuantityScale, QuantityValue,
    QuestionKind, QuestionMode, QuestionSlot, QuestionSlotRole, Quotation, ReferentCategory,
    RelativeClause, RelativeClauseKind, ScalarNegation, ScalarNegationKind, SemanticGraph,
    SemanticObject, SemanticObjectId, SemanticOperatorData, SemanticSort, SequenceRelation,
    SignKind, TanruLink, UtteranceForce, diagnostic, source_from_spans,
};

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
struct GeneratedGraphBuilder<'a, 'dict> {
    options: SemanticBuildOptions<'a>,
    dictionary: &'dict Dictionary<'dict>,
    objects: BTreeMap<SemanticObjectId, SemanticObject>,
    next_index: usize,
    relative_head: Option<SemanticObjectId>,
    current_utterance: Option<SemanticObjectId>,
    previous_utterance: Option<SemanticObjectId>,
    next_utterance: Option<SemanticObjectId>,
    current_speaker: SemanticObjectId,
    current_audience: SemanticObjectId,
    current_now: SemanticObjectId,
    current_here: SemanticObjectId,
    scoped_argument_variables: BTreeMap<(usize, usize), SemanticObjectId>,
    argument_question_parameters: Vec<SemanticObjectId>,
    relation_question_parameters: Vec<SemanticObjectId>,
    implicit_existential_variables: Vec<GeneratedImplicitExistential>,
    abstraction_parameter_stack: Vec<Vec<SemanticObjectId>>,
}

#[invariant(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
#[invariant(head_predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
#[derive(Debug, Clone)]
struct GeneratedTanruFormulaForArgument {
    formula: SemanticObjectId,
    x1_argument: ArgumentValue,
    head_predication: SemanticObjectId,
}

#[invariant(::Bridi(_) => true)]
#[invariant(::TermsFragment(_) => true)]
#[invariant(::StatementConnection(_) => true)]
#[invariant(::PreposedStatementConnection(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedTextRoot<'syntax> {
    Bridi(&'syntax BridiSyntax),
    TermsFragment(&'syntax TermsFragmentSyntax),
    StatementConnection(&'syntax IStatementConnectionSyntax),
    PreposedStatementConnection(&'syntax PreposedIStatementConnectionSyntax),
}

#[invariant(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
#[derive(Debug, Clone)]
struct GeneratedImplicitExistential {
    variable: SemanticObjectId,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(::ProBridi(_) => true)]
#[invariant(::GohaWord(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedRelationQuestionSyntax<'syntax> {
    ProBridi(&'syntax ProBridiTanruUnitSyntax),
    GohaWord(&'syntax GohaWordTanruUnitSyntax),
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

#[invariant(true)]
#[derive(Debug)]
struct GeneratedTermAssignments<'syntax> {
    visible_arguments: BTreeMap<usize, ArgumentValue>,
    modal_terms: Vec<TaggedSumtiTermSyntax>,
    formula_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct GeneratedArgumentQuantifierScope<'syntax> {
    sumti: &'syntax SumtiSyntax,
    source: GeneratedArgumentQuantifierSource<'syntax>,
    variable: SemanticObjectId,
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
        builder: &GeneratedGraphBuilder<'_, '_>,
        tanru: &TanruSelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Option<crate::model::SemanticSource> {
        match self {
            Self::Description => builder.source_for_node(tanru, "restrictive-predication"),
            Self::PropertyAbstraction => source,
        }
    }
}

impl<'a, 'dict> GeneratedGraphBuilder<'a, 'dict> {
    #[requires(true)]
    #[ensures(ret.next_index == 5)]
    fn new(options: SemanticBuildOptions<'a>, dictionary: &'dict Dictionary<'dict>) -> Self {
        let mut builder = Self {
            options,
            dictionary,
            objects: BTreeMap::new(),
            next_index: 5,
            relative_head: None,
            current_utterance: None,
            previous_utterance: None,
            next_utterance: None,
            current_speaker: SemanticObjectId::speaker(),
            current_audience: SemanticObjectId::addressee(),
            current_now: SemanticObjectId::now(),
            current_here: SemanticObjectId::here(),
            scoped_argument_variables: BTreeMap::new(),
            argument_question_parameters: Vec::new(),
            relation_question_parameters: Vec::new(),
            implicit_existential_variables: Vec::new(),
            abstraction_parameter_stack: Vec::new(),
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

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_text(mut self, syntax: &TextSyntax) -> Result<SemanticGraph, SemanticsError> {
        let roots = semantic_roots_from_text(syntax)?;
        let items = if roots.iter().all(generated_text_root_is_utterance) {
            let utterance_ids = (0..roots.len())
                .map(|_| self.next_utterance_id())
                .collect::<Vec<_>>();
            let mut items = Vec::new();
            for (index, (utterance_id, root)) in
                utterance_ids.iter().copied().zip(roots).enumerate()
            {
                self.previous_utterance = index
                    .checked_sub(1)
                    .and_then(|previous| utterance_ids.get(previous).copied());
                self.current_utterance = Some(utterance_id);
                self.next_utterance = utterance_ids.get(index + 1).copied();
                items.push(self.build_utterance_for_generated_text_root(utterance_id, root)?);
            }
            items
        } else {
            let mut items = Vec::new();
            for root in roots {
                items.push(self.build_discourse_item_for_generated_text_root(root)?);
            }
            items
        };
        self.previous_utterance = None;
        self.current_utterance = None;
        self.next_utterance = None;
        let root = if let [single] = items.as_slice() {
            *single
        } else {
            let sequence = self.next_sequence_id();
            self.insert(
                sequence,
                SemanticObject::sequence(
                    items,
                    SequenceRelation::SameTopicContinuation,
                    self.source_for_node(syntax, "text"),
                    Vec::new(),
                ),
            )?;
            sequence
        };
        self.prune_unreachable_objects(root);
        SemanticGraph::new(root, self.objects).map_err(|message| SemanticsError {
            kind: SemanticsErrorKind::InvalidGraph,
            message: format!("semantic graph invariant failed: {message}"),
        })
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    fn build_utterance_for_generated_text_root(
        &mut self,
        utterance_id: SemanticObjectId,
        root: GeneratedTextRoot<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match root {
            GeneratedTextRoot::Bridi(bridi) => self
                .build_bridi_utterance_with_force(utterance_id, bridi, generated_bridi_force(bridi))
                .map(|(utterance, _formula)| utterance),
            GeneratedTextRoot::TermsFragment(fragment) => {
                let referent = self.build_terms_fragment_referent(fragment)?;
                self.insert_generated_utterance(
                    utterance_id,
                    UtteranceForce::Mention,
                    Some(referent),
                    self.source_for_node(fragment, "fragment"),
                )
            }
            GeneratedTextRoot::StatementConnection(_)
            | GeneratedTextRoot::PreposedStatementConnection(_) => {
                Err(unsupported("statement connection as utterance"))
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    fn build_discourse_item_for_generated_text_root(
        &mut self,
        root: GeneratedTextRoot<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match root {
            GeneratedTextRoot::Bridi(bridi) => {
                let utterance_id = self.next_utterance_id();
                self.current_utterance = Some(utterance_id);
                self.build_bridi_utterance_with_force(
                    utterance_id,
                    bridi,
                    generated_bridi_force(bridi),
                )
                .map(|(utterance, _formula)| utterance)
            }
            GeneratedTextRoot::TermsFragment(fragment) => {
                let utterance_id = self.next_utterance_id();
                self.current_utterance = Some(utterance_id);
                let referent = self.build_terms_fragment_referent(fragment)?;
                self.insert_generated_utterance(
                    utterance_id,
                    UtteranceForce::Mention,
                    Some(referent),
                    self.source_for_node(fragment, "fragment"),
                )
            }
            GeneratedTextRoot::StatementConnection(connection) => {
                self.build_i_statement_connection_sequence(connection)
            }
            GeneratedTextRoot::PreposedStatementConnection(connection) => {
                self.build_preposed_i_statement_connection_sequence(connection)
            }
        }
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|(utterance, formula)| *utterance == utterance_id && formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_bridi_utterance_with_force(
        &mut self,
        utterance_id: SemanticObjectId,
        bridi: &BridiSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, SemanticObjectId), SemanticsError> {
        let question_start = self.argument_question_parameters.len();
        let relation_question_start = self.relation_question_parameters.len();
        let existential_start = self.implicit_existential_variables.len();
        let formula = self.build_bridi_formula(bridi)?;
        let existentials = self
            .implicit_existential_variables
            .split_off(existential_start);
        let formula = self.wrap_formula_with_implicit_existentials(formula, existentials)?;
        let question_parameters = self.argument_question_parameters.split_off(question_start);
        let relation_question_parameters = self
            .relation_question_parameters
            .split_off(relation_question_start);
        let (force, content) =
            if question_parameters.is_empty() && relation_question_parameters.is_empty() {
                (force, formula)
            } else if question_parameters.is_empty() {
                (
                    UtteranceForce::Ask,
                    self.build_direct_question(
                        QuestionKind::Relation,
                        SemanticSort::Relation,
                        formula,
                        relation_question_parameters,
                        self.source_for_node(bridi, "question"),
                    )?,
                )
            } else if relation_question_parameters.is_empty() {
                (
                    UtteranceForce::Ask,
                    self.build_direct_question(
                        QuestionKind::Argument,
                        SemanticSort::Entity,
                        formula,
                        question_parameters,
                        self.source_for_node(bridi, "question"),
                    )?,
                )
            } else {
                return Err(unsupported("mixed direct argument and relation question"));
            };
        self.insert_generated_utterance(
            utterance_id,
            force,
            Some(content),
            self.source_for_node(bridi, "bridi"),
        )?;
        Ok((utterance_id, formula))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameters.iter().all(|parameter| parameter.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Question) || ret.is_err())]
    fn build_direct_question(
        &mut self,
        kind: QuestionKind,
        domain: SemanticSort,
        formula: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let question = SemanticObjectId::question(self.next_index);
        self.next_index += 1;
        let slots = parameters
            .into_iter()
            .map(|parameter| QuestionSlot {
                parameter,
                role: QuestionSlotRole::Answer,
            })
            .collect::<Vec<_>>();
        self.insert(
            question,
            SemanticObject::question(
                kind,
                QuestionMode::Direct,
                domain,
                formula,
                slots,
                self.current_speaker(),
                self.current_audience(),
                source,
            ),
        )?;
        Ok(question)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(existentials.iter().all(|existential| existential.variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_implicit_existentials(
        &mut self,
        formula: SemanticObjectId,
        existentials: Vec<GeneratedImplicitExistential>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut body = formula;
        for existential in existentials.into_iter().rev() {
            let data!(GeneratedImplicitExistential { variable, source }) = existential.into_data();
            let formula = self.next_formula_id();
            self.insert(
                formula,
                SemanticObject::quantified_formula(
                    FormulaOperator::Exists,
                    variable,
                    None,
                    body,
                    None,
                    source,
                    Vec::new(),
                ),
            )?;
            body = formula;
        }
        Ok(body)
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    fn insert_generated_utterance(
        &mut self,
        utterance_id: SemanticObjectId,
        force: UtteranceForce,
        content: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let locution = self.next_locution_id();
        self.insert(
            locution,
            SemanticObject::eventuality(
                EventualityClass::Locution,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                source.clone(),
            ),
        )?;
        self.insert(
            utterance_id,
            SemanticObject::utterance(
                force,
                locution,
                content,
                self.current_speaker(),
                self.current_audience(),
                self.current_now(),
                self.current_here(),
                source,
                Vec::new(),
            ),
        )?;
        Ok(utterance_id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    fn build_i_statement_connection_sequence(
        &mut self,
        connection: &IStatementConnectionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_bridi = bridi_from_statement_base(&connection.leading_statement)?;
        let leading_utterance = self.next_utterance_id();
        self.current_utterance = Some(leading_utterance);
        let (leading_item, mut formula) = self.build_bridi_utterance_with_force(
            leading_utterance,
            leading_bridi,
            UtteranceForce::Subordinated,
        )?;
        let mut items = vec![leading_item];
        for continuation in &connection.continuations {
            let (connective, trailing_bridi) = statement_connection_tail_parts(continuation)?;
            let trailing_utterance = self.next_utterance_id();
            self.previous_utterance = items.last().copied();
            self.current_utterance = Some(trailing_utterance);
            self.next_utterance = None;
            let (trailing_item, trailing_formula) = self.build_bridi_utterance_with_force(
                trailing_utterance,
                trailing_bridi,
                UtteranceForce::Subordinated,
            )?;
            items.push(trailing_item);
            formula = self.build_binary_formula_for_generated_statement_connective(
                connective,
                formula,
                trailing_formula,
                self.source_for_node(connection, "statement-connection"),
            )?;
        }
        let sequence = self.next_sequence_id();
        let mut object = SemanticObject::sequence(
            items,
            SequenceRelation::SameTopicContinuation,
            self.source_for_node(connection, "statement-connection"),
            Vec::new(),
        );
        object.content = Some(formula);
        self.insert(sequence, object)?;
        Ok(sequence)
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_binary_formula_for_generated_statement_connective(
        &mut self,
        connective: &IStatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_binary_formula_for_generated_statement_connective_core(
            generated_i_statement_connective_core(connective)?,
            left,
            right,
            source,
        )
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_binary_formula_for_generated_statement_connective_core(
        &mut self,
        connective: &StatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_statement_connective_formula_operator_for_core(connective);
        let Some(truth_table) = generated_statement_connective_core_truth_table(connective) else {
            return Err(unsupported("nonlogical generated statement connective"));
        };
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                vec![left, right],
                Some(Connector {
                    source: generated_statement_connective_core_source(connective)?,
                    locus: "statement".to_owned(),
                    truth_table: Some(truth_table),
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    fn build_preposed_i_statement_connection_sequence(
        &mut self,
        connection: &PreposedIStatementConnectionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_bridi = bridi_from_statement_base(&connection.leading_statement)?;
        let trailing_bridi =
            bridi_from_statement_after_i_connective(&connection.trailing_statement)?;
        let leading_utterance = self.next_utterance_id();
        self.current_utterance = Some(leading_utterance);
        let (leading_item, leading_formula) = self.build_bridi_utterance_with_force(
            leading_utterance,
            leading_bridi,
            UtteranceForce::Subordinated,
        )?;
        let trailing_utterance = self.next_utterance_id();
        self.previous_utterance = Some(leading_item);
        self.current_utterance = Some(trailing_utterance);
        self.next_utterance = None;
        let (trailing_item, trailing_formula) = self.build_bridi_utterance_with_force(
            trailing_utterance,
            trailing_bridi,
            UtteranceForce::Subordinated,
        )?;
        let formula = self.build_binary_formula_for_generated_statement_connective_core(
            &connection.connective,
            leading_formula,
            trailing_formula,
            self.source_for_node(connection, "statement-connection"),
        )?;
        let sequence = self.next_sequence_id();
        let mut object = SemanticObject::sequence(
            vec![leading_item, trailing_item],
            SequenceRelation::SameTopicContinuation,
            self.source_for_node(connection, "statement-connection"),
            Vec::new(),
        );
        object.content = Some(formula);
        self.insert(sequence, object)?;
        Ok(sequence)
    }

    #[requires(self.objects.contains_key(&root))]
    #[ensures(self.objects.contains_key(&root))]
    #[ensures(self.objects.keys().all(|id| {
        let mut reachable = HashSet::new();
        let mut stack = vec![root];
        while let Some(next) = stack.pop() {
            if reachable.insert(next)
                && let Some(object) = self.objects.get(&next)
            {
                let mut references = Vec::new();
                object.references_into(&mut references);
                stack.extend(references);
            }
        }
        reachable.contains(id)
    }))]
    fn prune_unreachable_objects(&mut self, root: SemanticObjectId) {
        let mut reachable = HashSet::new();
        let mut stack = vec![root];
        while let Some(next) = stack.pop() {
            if reachable.insert(next)
                && let Some(object) = self.objects.get(&next)
            {
                let mut references = Vec::new();
                object.references_into(&mut references);
                stack.extend(references);
            }
        }
        self.objects.retain(|id, _object| reachable.contains(id));
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_formula(
        &mut self,
        bridi: &BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_bridi_formula_with_options(bridi, None, PredicationMode::Asserted)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_formula_with_options(
        &mut self,
        bridi: &BridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match bridi {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                self.build_bridi_with_leading_terms_formula_with_options(bridi, eventuality, mode)
            }
            BridiSyntax::RelationOnlyBridi(bridi) => {
                self.build_relation_only_bridi_formula_with_options(bridi, eventuality, mode)
            }
            _ => Err(unsupported("bridi shape")),
        }
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_subbridi_formula_with_eventuality(
        &mut self,
        subbridi: &SubbridiSyntax,
        eventuality: SemanticObjectId,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
                self.build_bridi_formula_with_options(bridi, Some(eventuality), mode)
            }
            SubbridiSyntax::PrenexSubbridi(_) => Err(unsupported("prenex subbridi")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| abstraction.abstractor_connections.is_empty())) || ret.is_err())]
    fn single_abstraction_from_selbri<'syntax>(
        &self,
        selbri: &'syntax SelbriSyntax,
    ) -> Result<Option<&'syntax AbstractionTanruUnitSyntax>, SemanticsError> {
        let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(CoSelbriSyntax {
            leading_selbri,
            co_tail,
        })) = selbri
        else {
            return Ok(None);
        };
        if co_tail.is_some() {
            return Ok(None);
        }
        let ConnectedSelbriSyntax {
            leading_selbri,
            continuations,
        } = leading_selbri.as_ref();
        if !continuations.is_empty() {
            return Ok(None);
        }
        let TanruSelbriSyntax {
            first_unit,
            additional_units,
        } = leading_selbri.as_ref();
        if !additional_units.is_empty() || !first_unit.0.links.is_empty() {
            return Ok(None);
        }
        let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*first_unit.0.first else {
            return Ok(None);
        };
        if unit.linkargs.is_some() || !unit.base.conversions.is_empty() {
            return Ok(None);
        }
        let TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) = unit.base.base.as_ref()
        else {
            return Ok(None);
        };
        if abstraction.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        if !abstraction.abstractor_connections.is_empty() {
            return Err(unsupported("connected abstraction"));
        }
        Ok(Some(abstraction))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_selbri(
        selbri: &SelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        match selbri {
            SelbriSyntax::TaggedSelbri(tagged) => {
                Self::generated_description_abstraction_for_untagged_selbri(&tagged.inner_selbri)
            }
            SelbriSyntax::UntaggedSelbri(untagged) => {
                Self::generated_description_abstraction_for_untagged_selbri(untagged)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_untagged_selbri(
        selbri: &UntaggedSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        match selbri {
            UntaggedSelbriSyntax::CoSelbri(co_selbri) if co_selbri.co_tail.is_none() => {
                Self::generated_description_abstraction_for_connected_selbri(
                    &co_selbri.leading_selbri,
                )
            }
            UntaggedSelbriSyntax::NegatedSelbri(_)
            | UntaggedSelbriSyntax::CoSelbri(_)
            | UntaggedSelbriSyntax::ForethoughtSelbriConnection(_) => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_connected_selbri(
        selbri: &ConnectedSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        if !selbri.continuations.is_empty() {
            return Ok(None);
        }
        Self::generated_description_abstraction_for_tanru_selbri(&selbri.leading_selbri)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_tanru_selbri(
        selbri: &TanruSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        if !selbri.additional_units.is_empty() {
            return Ok(None);
        }
        Self::generated_description_abstraction_for_tanru_unit(&selbri.first_unit)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_tanru_unit(
        unit: &TanruUnitSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        if !unit.0.links.is_empty() {
            return Ok(None);
        }
        Self::generated_description_abstraction_for_bo_or_linked_tanru_unit(&unit.0.first)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_bo_or_linked_tanru_unit(
        unit: &BoOrLinkedTanruUnitSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) if unit.linkargs.is_none() => {
                Self::generated_description_abstraction_for_tanru_atom(&unit.base)
            }
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_tanru_atom(
        atom: &TanruUnitAtomSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'_>>, SemanticsError> {
        match atom.base.as_ref() {
            TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) => {
                Self::generated_description_abstraction_for_nu_with_conversions(
                    abstraction,
                    &atom.conversions,
                )
            }
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) if atom.conversions.is_empty() => {
                Self::generated_description_abstraction_for_connected_selbri(&grouped.selbri)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    fn generated_description_abstraction_for_nu_with_conversions<'syntax, F>(
        abstraction: &'syntax AbstractionTanruUnitSyntax,
        conversions: &[WithFreeModifiers<Token, F>],
    ) -> Result<Option<GeneratedDescriptionAbstraction<'syntax>>, SemanticsError> {
        if abstraction.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        if !abstraction.abstractor_connections.is_empty() {
            return Err(unsupported("connected abstraction"));
        }
        let kind = abstraction_kind_for_nu(abstraction);
        if conversions.is_empty() {
            return Ok(Some(GeneratedDescriptionAbstraction {
                abstraction,
                output_sort: abstraction_output_sort(kind),
                link_relation: abstraction_link_relation(kind),
            }));
        }
        let [conversion] = conversions else {
            return Ok(None);
        };
        if se_conversion_place(&conversion.value)? == Some(2)
            && kind == AbstractionKind::Proposition
        {
            return Ok(Some(GeneratedDescriptionAbstraction {
                abstraction,
                output_sort: SemanticSort::Text,
                link_relation: "sentenceExpresses",
            }));
        }
        Ok(None)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_only_bridi_formula_with_options(
        &mut self,
        bridi: &RelationOnlyBridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let simple_tail = simple_tail_from_bridi_tail(&bridi.0)?;
        let terms: Vec<&TermSyntax> = simple_tail.terms.iter().collect();
        let abstraction = if terms.is_empty() && eventuality.is_none() {
            self.single_abstraction_from_selbri(&simple_tail.selbri)?
                .cloned()
        } else {
            None
        };
        if let Some(abstraction) = abstraction {
            return self.build_abstraction_link_formula_for_visible_argument(
                &abstraction,
                None,
                self.source_for_node(bridi, "bridi-formula"),
                mode,
            );
        };
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && !tanru.additional_units.is_empty()
        {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped tanru bridi"));
            }
            return self.build_tanru_formula_for_terms(
                tanru,
                terms,
                2,
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(&simple_tail.selbri)? {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped sumti selbri"));
            }
            return self.build_sumti_selbri_formula_for_terms(
                sumti_selbri,
                terms,
                2,
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && tanru.additional_units.is_empty()
            && generated_tanru_unit_is_grouped(&tanru.first_unit)?
        {
            return self.build_relation_formula_for_generated_tanru_unit_terms(
                &tanru.first_unit,
                terms,
                2,
                eventuality,
                mode,
                self.source_for_node(bridi, "tanru-formula"),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        self.build_simple_tail_formula_with_options(
            simple_tail,
            terms,
            2,
            eventuality,
            mode,
            self.source_for_node(bridi, "predication"),
            self.source_for_node(bridi, "bridi-formula"),
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_with_leading_terms_formula(
        &mut self,
        bridi: &BridiWithLeadingTermsSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_bridi_with_leading_terms_formula_with_options(
            bridi,
            None,
            PredicationMode::Asserted,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_bridi_with_leading_terms_formula_with_options(
        &mut self,
        bridi: &BridiWithLeadingTermsSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
        let terms: Vec<&TermSyntax> = bridi
            .leading_terms
            .iter()
            .chain(simple_tail.terms.iter())
            .collect();
        if eventuality.is_none()
            && mode == PredicationMode::Asserted
            && let [term] = terms.as_slice()
            && let Some(sumti) = simple_sumti_from_term(term).ok()
        {
            if let Some(description) = no_gadri_description_from_sumti(sumti)? {
                return self.build_no_gadri_quantified_argument_formula(
                    simple_tail,
                    description,
                    self.source_for_node(bridi, "predication"),
                    self.source_for_node(bridi, "bridi-formula"),
                );
            }
            if let Some(afterthought) = afterthought_sumti_from_sumti(sumti)? {
                return self.build_afterthought_sumti_argument_formula(
                    simple_tail,
                    afterthought,
                    self.source_for_node(bridi, "distributed-predication"),
                    self.source_for_node(bridi, "distributed-formula"),
                    self.source_for_node(bridi, "sumti-connection-formula"),
                );
            }
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && !tanru.additional_units.is_empty()
        {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped tanru bridi"));
            }
            return self.build_tanru_formula_for_terms(
                tanru,
                terms,
                1,
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(&simple_tail.selbri)? {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped sumti selbri"));
            }
            return self.build_sumti_selbri_formula_for_terms(
                sumti_selbri,
                terms,
                1,
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && tanru.additional_units.is_empty()
            && generated_tanru_unit_is_grouped(&tanru.first_unit)?
        {
            return self.build_relation_formula_for_generated_tanru_unit_terms(
                &tanru.first_unit,
                terms,
                1,
                eventuality,
                mode,
                self.source_for_node(bridi, "tanru-formula"),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        self.build_simple_tail_formula_with_options(
            simple_tail,
            terms,
            1,
            eventuality,
            mode,
            self.source_for_node(bridi, "predication"),
            self.source_for_node(bridi, "bridi-formula"),
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_simple_tail_formula_with_options(
        &mut self,
        simple_tail: &SelbriSimpleBridiTailSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_formula_with_options(
            &simple_tail.selbri,
            terms,
            first_visible_place,
            eventuality,
            mode,
            false,
            predication_source,
            formula_source,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_selbri_formula_with_options(
        &mut self,
        selbri: &SelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match selbri {
            SelbriSyntax::TaggedSelbri(tagged) => self.build_tagged_selbri_formula_with_options(
                tagged,
                terms,
                first_visible_place,
                eventuality,
                mode,
                formula_scope_child,
                predication_source,
                formula_source,
            ),
            SelbriSyntax::UntaggedSelbri(untagged) => self
                .build_untagged_selbri_formula_with_options(
                    untagged,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    formula_scope_child,
                    predication_source,
                    formula_source,
                ),
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tagged_selbri_formula_with_options(
        &mut self,
        tagged: &jbotci_syntax::generated_model::TaggedSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref()) {
            if eventuality.is_some() {
                return Err(unsupported("eventuality on scoped tagged selbri"));
            }
            let child = self.build_untagged_selbri_formula_with_options(
                tagged.inner_selbri.as_ref(),
                terms,
                first_visible_place,
                None,
                mode,
                true,
                formula_source.clone(),
                formula_source,
            )?;
            return self.build_generated_tense_scope_formula(
                child,
                tagged.tense_modal.as_ref(),
                self.source_for_node(tagged, "tense-scope"),
            );
        }

        let tense_eventuality = self.build_generated_tense_eventuality(
            tagged.tense_modal.as_ref(),
            predication_source.clone(),
        )?;
        let eventuality = match (eventuality, tense_eventuality) {
            (None, Some(tense_eventuality)) => Some(tense_eventuality),
            (Some(eventuality), None) => Some(eventuality),
            (None, None) => None,
            (Some(_), Some(_)) => return Err(unsupported("stacked tagged selbri eventuality")),
        };
        self.build_untagged_selbri_formula_with_options(
            tagged.inner_selbri.as_ref(),
            terms,
            first_visible_place,
            eventuality,
            mode,
            formula_scope_child,
            predication_source,
            formula_source,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_untagged_selbri_formula_with_options(
        &mut self,
        selbri: &UntaggedSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match selbri {
            UntaggedSelbriSyntax::NegatedSelbri(negated) => {
                let operator = generated_bridi_negation_operator(&negated.na);
                let source_construct = bridi_negation_source_construct(operator);
                let child = self.build_selbri_formula_with_options(
                    negated.inner_selbri.as_ref(),
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    true,
                    formula_source.clone(),
                    formula_source.clone(),
                )?;
                let formula = self.next_formula_id();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        operator,
                        vec![child],
                        None,
                        self.source_for_node(negated, source_construct),
                        Vec::new(),
                    ),
                )?;
                Ok(formula)
            }
            UntaggedSelbriSyntax::CoSelbri(co_selbri) => self.build_co_selbri_formula_with_options(
                co_selbri,
                terms,
                first_visible_place,
                eventuality,
                mode,
                formula_scope_child,
                predication_source,
                formula_source,
            ),
            UntaggedSelbriSyntax::ForethoughtSelbriConnection(connection) => {
                if eventuality.is_some() || mode != PredicationMode::Asserted {
                    return Err(unsupported("scoped forethought selbri connection"));
                }
                let assignments =
                    self.build_term_assignments_for_terms(terms, first_visible_place)?;
                let mut visible_arguments = assignments.visible_arguments;
                if !visible_arguments.contains_key(&1) {
                    let referent = self.build_elided_referent("zo'e".to_owned())?;
                    insert_visible_argument(
                        &mut visible_arguments,
                        1,
                        ArgumentValue::elided(referent, "zo'e".to_owned(), None),
                    )?;
                }
                let result = self
                    .build_forethought_selbri_connection_formula_for_visible_arguments(
                        connection,
                        visible_arguments,
                        source_with_construct(
                            formula_source.or(predication_source),
                            "connected-selbri-formula",
                        ),
                        "selbri",
                        None,
                    )?;
                self.attach_generated_modal_terms_to_formula(
                    result.formula,
                    &assignments.modal_terms,
                )?;
                Ok(result.formula)
            }
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_co_selbri_formula_with_options(
        &mut self,
        selbri: &CoSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(tanru) = tanru_selbri_from_co_selbri(selbri)?
            && !tanru.additional_units.is_empty()
        {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped tanru bridi"));
            }
            return self.build_tanru_formula_for_terms_with_head_eventuality_order(
                tanru,
                terms,
                first_visible_place,
                formula_scope_child,
                formula_source,
            );
        }
        if let Some(question) = relation_question_syntax_from_co_selbri(selbri)? {
            return self.build_relation_question_formula_for_terms(
                question,
                terms,
                first_visible_place,
                eventuality,
                mode,
                source_with_construct(
                    predication_source.or(formula_source),
                    "relation-question-formula",
                ),
            );
        }
        if let Some(tanru) = tanru_selbri_from_co_selbri(selbri)?
            && tanru.additional_units.is_empty()
            && sumti_selbri_from_generated_tanru_unit(&tanru.first_unit)?.is_none()
        {
            let (predication_source, formula_source) =
                if generated_tanru_unit_is_connected_selbri_formula(&tanru.first_unit) {
                    let source = source_with_construct(
                        formula_source
                            .clone()
                            .or_else(|| predication_source.clone()),
                        "connected-selbri-formula",
                    );
                    (source.clone(), source)
                } else {
                    (predication_source, formula_source)
                };
            return self.build_relation_formula_for_generated_tanru_unit_terms(
                &tanru.first_unit,
                terms,
                first_visible_place,
                eventuality,
                mode,
                predication_source,
                formula_source,
            );
        }
        let relation = relation_label_from_co_selbri(selbri)?;
        let relation = semantic_relation_label(relation);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                terms.len().max(1)
            }
        };
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => {
                let eventuality = self.next_eventuality_id();
                self.insert(
                    eventuality,
                    SemanticObject::eventuality(
                        EventualityClass::Event,
                        None,
                        predication_source.clone(),
                    ),
                )?;
                eventuality
            }
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let modal_arguments =
            self.build_modal_arguments_for_generated_tagged_terms(&assignments.modal_terms)?;
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let referent = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(
                    key,
                    ArgumentValue::elided(referent, "zo'e".to_owned(), None),
                );
            }
        }
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation.clone(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_argument_scopes(formula, assignments.formula_scopes)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_question_formula_for_terms(
        &mut self,
        question: GeneratedRelationQuestionSyntax<'_>,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Relation,
                ParameterRole::RelationQuestion,
                token_text(generated_relation_question_token(question)),
                self.source_for_relation_question(question, "parameter"),
            ),
        )?;
        self.relation_question_parameters.push(parameter);
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => {
                let eventuality = self.next_eventuality_id();
                self.insert(
                    eventuality,
                    SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
                )?;
                eventuality
            }
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        let modal_arguments =
            self.build_modal_arguments_for_generated_tagged_terms(&assignments.modal_terms)?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to {key}"
                )));
            }
        }
        let predication = self.next_predication_id();
        let mut object = SemanticObject::relation_parameter_predication(
            parameter,
            Some(eventuality),
            arguments,
            mode,
            source.clone(),
            Vec::new(),
        );
        object.modal_arguments = modal_arguments;
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_argument_scopes(formula, assignments.formula_scopes)
    }

    #[requires(child.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_generated_tense_scope_formula(
        &mut self,
        child: SemanticObjectId,
        tense_modal: &TenseModalSyntax,
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
        object.eventuality = eventuality;
        self.insert(formula, object)?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.as_ref().is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))) || ret.is_err())]
    fn build_generated_tense_eventuality(
        &mut self,
        tense_modal: &TenseModalSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((domain, relation)) = generated_anchor_relation_for_tense_modal(tense_modal)
        else {
            return Ok(None);
        };
        let eventuality = self.next_eventuality_id();
        let mut object = SemanticObject::eventuality(EventualityClass::Event, None, source);
        match domain {
            GeneratedAnchorDomain::Time => object.time = Some(relation),
            GeneratedAnchorDomain::Space => object.space = Some(relation),
        }
        self.insert(eventuality, object)?;
        Ok(Some(eventuality))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    fn build_eventuality(
        &mut self,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source),
        )?;
        Ok(eventuality)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_generated_tanru_unit_terms(
        &mut self,
        unit: &TanruUnitSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !unit.0.links.is_empty()
            || !matches!(
                unit.0.first.as_ref(),
                BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(_)
            )
        {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped connected tanru unit"));
            }
            let leading_eventuality =
                if !terms.is_empty() && generated_tanru_unit_is_connected_selbri_formula(unit) {
                    Some(self.build_eventuality(formula_source.clone())?)
                } else {
                    None
                };
            let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
            let result = self.build_tanru_unit_formula_for_visible_arguments(
                unit,
                assignments.visible_arguments,
                formula_source,
                "selbri",
                leading_eventuality,
            )?;
            self.attach_generated_modal_terms_to_formula(result.formula, &assignments.modal_terms)?;
            return self.wrap_formula_with_generated_argument_scopes(
                result.formula,
                assignments.formula_scopes,
            );
        }
        let (atom, linkargs) = generated_linked_tanru_unit_parts(unit)?;
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        if let Some(scalar_unit) = scalar_unit
            && let Some((grouped, inner_conversions)) =
                scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped scalar grouped tanru unit"));
            }
            let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                assignments.visible_arguments,
                &atom.conversions,
            )?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
                inner_conversions,
            )?;
            let formula = self.build_tanru_formula_for_connected_selbri_with_visible_arguments(
                &grouped.selbri,
                visible_arguments,
                formula_source,
            )?;
            self.attach_generated_modal_terms_to_formula(formula, &assignments.modal_terms)?;
            self.apply_scalar_negation_to_tanru_links(
                formula,
                scalar_negation_for_marker(&scalar_unit.nahe)
                    .with_argument_scope(vec!["x1".to_owned()]),
            )?;
            let formula = self
                .detach_tanru_relation_formula_without_positive_head(formula)
                .unwrap_or(formula);
            return self
                .wrap_formula_with_generated_argument_scopes(formula, assignments.formula_scopes);
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = atom.base.as_ref() {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped grouped tanru unit"));
            }
            let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                assignments.visible_arguments,
                &atom.conversions,
            )?;
            let formula = self.build_tanru_formula_for_connected_selbri_with_visible_arguments(
                &grouped.selbri,
                visible_arguments,
                formula_source,
            )?;
            self.attach_generated_modal_terms_to_formula(formula, &assignments.modal_terms)?;
            return self
                .wrap_formula_with_generated_argument_scopes(formula, assignments.formula_scopes);
        }
        let relation = semantic_relation_label(relation_label_from_tanru_unit_atom_base(
            atom.base.as_ref(),
        )?);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => {
                let eventuality = self.next_eventuality_id();
                self.insert(
                    eventuality,
                    SemanticObject::eventuality(
                        EventualityClass::Event,
                        None,
                        predication_source.clone(),
                    ),
                )?;
                eventuality
            }
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        let mut visible_arguments = assignments.visible_arguments;
        if let Some(linkargs) = linkargs {
            let (_, adjusted_arguments) =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted_arguments;
        }
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let modal_arguments =
            self.build_modal_arguments_for_generated_tagged_terms(&assignments.modal_terms)?;
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let referent = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(
                    key,
                    ArgumentValue::elided(referent, "zo'e".to_owned(), None),
                );
            }
        }
        let predication_mode = predication_mode_for_relation(&relation, mode);
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            predication_mode,
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_argument_scopes(formula, assignments.formula_scopes)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_no_gadri_quantified_argument_formula(
        &mut self,
        simple_tail: &SelbriSimpleBridiTailSyntax,
        description: &DescriptorWithoutGadriSumtiSyntax,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, predication_source.clone()),
        )?;
        let variable = self.build_bound_argument_variable(description)?;
        let body = self.build_relation_formula_for_argument(
            relation,
            ArgumentValue::filled(variable, None),
            Some(eventuality),
            PredicationMode::Asserted,
            predication_source,
            formula_source,
        )?;
        self.wrap_formula_with_no_gadri_quantifier(description, variable, body)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_afterthought_sumti_argument_formula(
        &mut self,
        simple_tail: &SelbriSimpleBridiTailSyntax,
        sumti: &SumtiAfterthoughtSyntax,
        distributed_predication_source: Option<crate::model::SemanticSource>,
        distributed_formula_source: Option<crate::model::SemanticSource>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let [continuation] = sumti.continuations.as_slice() else {
            return Err(unsupported("non-binary afterthought sumti distribution"));
        };
        let connector = generated_argument_connective_operator(&continuation.connective)?;
        if connector != "joint" {
            return Err(unsupported("non-joint afterthought sumti distribution"));
        }
        let Some(description) = no_gadri_description_from_sumti_bound(&sumti.leading_sumti)? else {
            return Err(unsupported("non-quantified leading afterthought sumti"));
        };
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let variable = self.build_bound_argument_variable(description)?;
        let leading_eventuality = self.next_eventuality_id();
        self.insert(
            leading_eventuality,
            SemanticObject::eventuality(
                EventualityClass::Event,
                None,
                distributed_predication_source.clone(),
            ),
        )?;
        let leading_body = self.build_relation_formula_for_argument(
            relation.clone(),
            ArgumentValue::filled(variable, None),
            Some(leading_eventuality),
            PredicationMode::Asserted,
            distributed_predication_source.clone(),
            distributed_formula_source.clone(),
        )?;
        let leading =
            self.wrap_formula_with_no_gadri_quantifier(description, variable, leading_body)?;
        let trailing_referent = self.build_sumti_bound_referent(&continuation.sumti)?;
        let trailing = self.build_relation_formula_for_argument(
            relation,
            ArgumentValue::filled(trailing_referent, None),
            None,
            PredicationMode::Asserted,
            distributed_predication_source,
            distributed_formula_source,
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![leading, trailing],
                Some(Connector {
                    source: "e".to_owned(),
                    locus: "sumti".to_owned(),
                    truth_table: Some("TFFF".to_owned()),
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_argument(
        &mut self,
        relation: String,
        argument: ArgumentValue,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                1
            }
        };
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => {
                let eventuality = self.next_eventuality_id();
                self.insert(
                    eventuality,
                    SemanticObject::eventuality(
                        EventualityClass::Event,
                        None,
                        predication_source.clone(),
                    ),
                )?;
                eventuality
            }
        };
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), argument);
        for place in 2..=place_limit {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let predication_mode = predication_mode_for_relation(&relation, mode);
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                predication_mode,
                predication_source,
                diagnostics,
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_bound_argument_variable<N: TreeNode>(
        &mut self,
        node: &N,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Variable,
                SemanticSort::Entity,
                None,
                None,
                None,
                self.source_for_node(node, "bound-argument"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_no_gadri_quantifier(
        &mut self,
        description: &DescriptorWithoutGadriSumtiSyntax,
        variable: SemanticObjectId,
        body: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let restriction = self.build_no_gadri_restriction_formula(description, variable)?;
        let quantity = self.build_quantity_for_quantifier(&description.quantifier)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::quantified_formula(
                FormulaOperator::Cardinality,
                variable,
                Some(restriction),
                body,
                Some(quantity),
                self.source_for_node(description, "quantifier-scope"),
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_generated_argument_scopes<'syntax>(
        &mut self,
        formula: SemanticObjectId,
        scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut body = formula;
        for scope in scopes.into_iter().rev() {
            body = self.wrap_formula_with_generated_argument_scope(body, scope)?;
        }
        Ok(body)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn wrap_formula_with_generated_argument_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: GeneratedArgumentQuantifierScope<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let restrictions =
            self.generated_argument_restrictions_for_scope_source(scope.source, scope.variable)?;
        let restriction = self.combine_generated_restriction_formulas(restrictions)?;
        let quantifier = generated_argument_scope_source_quantifier(scope.source);
        let quantity = self.build_quantity_for_quantifier(quantifier)?;
        let scoped = self.next_formula_id();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                generated_quantifier_formula_operator(quantifier),
                scope.variable,
                restriction,
                formula,
                Some(quantity),
                self.source_for_node(scope.sumti, "quantifier-scope"),
                Vec::new(),
            ),
        )?;
        Ok(scoped)
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn generated_argument_restrictions_for_scope_source(
        &mut self,
        source: GeneratedArgumentQuantifierSource<'_>,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        match source {
            GeneratedArgumentQuantifierSource::QuantifiedSumti(quantified) => {
                self.generated_argument_restrictions_for_quantified_sumti(quantified, variable)
            }
            GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description) => {
                let base = self.build_outer_quantified_description_referent(description)?;
                self.build_membership_restriction_formula(variable, base)
                    .map(|restriction| vec![restriction])
            }
            GeneratedArgumentQuantifierSource::NoGadriDescription(description) => self
                .build_no_gadri_restriction_formula(description, variable)
                .map(|restriction| vec![restriction]),
        }
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn generated_argument_restrictions_for_quantified_sumti(
        &mut self,
        quantified: &QuantifiedSumtiSyntax,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        if generated_sumti_base_spine_cmavo(&quantified.inner_sumti)
            .is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di))
        {
            return Ok(Vec::new());
        }
        let base = self.build_sumti_base_referent(&quantified.inner_sumti)?;
        self.build_membership_restriction_formula(variable, base)
            .map(|restriction| vec![restriction])
    }

    #[requires(restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|restriction| restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    fn combine_generated_restriction_formulas(
        &mut self,
        restrictions: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match restrictions.as_slice() {
            [] => Ok(None),
            [restriction] => Ok(Some(*restriction)),
            _ => {
                let formula = self.next_formula_id();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        FormulaOperator::And,
                        restrictions,
                        None,
                        None,
                        Vec::new(),
                    ),
                )?;
                Ok(Some(formula))
            }
        }
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(crate::model::argument_object_kind_can_fill(base.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_membership_restriction_formula(
        &mut self,
        variable: SemanticObjectId,
        base: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(variable, None));
        arguments.insert("x2".to_owned(), ArgumentValue::filled(base, None));
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                "memberOf".to_owned(),
                None,
                arguments,
                PredicationMode::Restrictive,
                None,
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, None, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_no_gadri_restriction_formula(
        &mut self,
        description: &DescriptorWithoutGadriSumtiSyntax,
        variable: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if description.relative_clauses.is_some() {
            return Err(unsupported("description relative clauses"));
        }
        self.build_restrictive_formula(&description.selbri, variable)
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_terms(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_tanru_formula_for_terms_with_head_eventuality_order(
            tanru,
            terms,
            first_visible_place,
            false,
            source,
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_terms_with_head_eventuality_order(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        head_eventuality_before_terms: bool,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let head_eventuality = if head_eventuality_before_terms {
            let head_eventuality = self.next_eventuality_id();
            self.insert(
                head_eventuality,
                SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
            )?;
            Some(head_eventuality)
        } else {
            None
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        let result = self
            .build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
                tanru,
                assignments.visible_arguments,
                head_eventuality,
                source,
                &assignments.modal_terms,
            )?;
        self.wrap_formula_with_generated_argument_scopes(result.formula, assignments.formula_scopes)
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_visible_arguments(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_tanru_formula_for_visible_arguments_with_head_eventuality(
            tanru,
            visible_arguments,
            None,
            source,
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[requires(head_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_visible_arguments_with_head_eventuality(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        head_eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_tanru_formula_result_for_visible_arguments_with_head_eventuality(
            tanru,
            visible_arguments,
            head_eventuality,
            source,
        )
        .map(|result| result.formula)
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[requires(head_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_result_for_visible_arguments_with_head_eventuality(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        head_eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
            tanru,
            visible_arguments,
            head_eventuality,
            source,
            &[],
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[requires(head_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        head_eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        modal_terms: &[TaggedSumtiTermSyntax],
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        let Some((trailing_unit, modifier_units)) = tanru.additional_units.split_last() else {
            return Err(unsupported("empty tanru continuation"));
        };
        let head = self.build_tanru_head_relation_formula_with_modal_terms(
            trailing_unit,
            visible_arguments,
            head_eventuality,
            source.clone(),
            modal_terms,
        )?;
        let modifier = self.build_property_abstraction_for_tanru_run(
            &tanru.first_unit,
            modifier_units,
            source.clone(),
        )?;
        let relation_formula = self.build_tanru_relation_formula(
            head.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_generated_unit_run(
                &tanru.first_unit,
                modifier_units,
                trailing_unit,
                false,
            )?,
            head.head_predication,
            PredicationMode::Asserted,
            source.clone(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![head.formula, relation_formula],
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
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: head.x1_argument.clone(),
                head_predication: head.head_predication,
            }
        )))
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_connected_selbri_with_visible_arguments(
        &mut self,
        selbri: &ConnectedSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_connected_selbri_tanru_formula_for_visible_arguments(
            selbri,
            visible_arguments,
            source,
        )
        .map(|result| result.formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_selbri_tanru_formula_for_visible_arguments(
        &mut self,
        selbri: &ConnectedSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
            selbri,
            visible_arguments,
            source,
            None,
        )
    }

    #[requires(true)]
    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
        &mut self,
        selbri: &ConnectedSelbriSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if selbri.continuations.is_empty() {
            return self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
                &selbri.leading_selbri,
                visible_arguments,
                leading_eventuality,
                source,
            );
        }
        if !visible_arguments.contains_key(&1) {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            )?;
        }
        let leading = self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
            &selbri.leading_selbri,
            visible_arguments.clone(),
            leading_eventuality,
            source.clone(),
        )?;
        let mut formula = leading.formula;
        for continuation in &selbri.continuations {
            let trailing = self.build_tanru_selbri_formula_for_visible_arguments(
                &continuation.trailing_selbri,
                visible_arguments.clone(),
                source.clone(),
            )?;
            formula = self.build_binary_formula_for_relation_afterthought_connective(
                &continuation.connective,
                "selbri",
                formula,
                trailing.formula,
                source.clone(),
            )?;
        }
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: leading.x1_argument.clone(),
                head_predication: leading.head_predication,
            }
        )))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_selbri_formula_for_visible_arguments(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
            tanru,
            visible_arguments,
            None,
            source,
        )
    }

    #[requires(true)]
    #[requires(head_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        head_eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if tanru.additional_units.is_empty() {
            return self.build_tanru_unit_formula_for_visible_arguments(
                &tanru.first_unit,
                visible_arguments,
                source,
                "selbri",
                head_eventuality,
            );
        }
        self.build_tanru_formula_result_for_visible_arguments_with_head_eventuality(
            tanru,
            visible_arguments,
            head_eventuality,
            source,
        )
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_selbri_formula_for_visible_arguments(
        &mut self,
        selbri: &SelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        match selbri {
            SelbriSyntax::TaggedSelbri(_) => Err(unsupported("tagged forethought tanru branch")),
            SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) => {
                if co_selbri.co_tail.is_some() {
                    return Err(unsupported("CO forethought tanru branch"));
                }
                self.build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
                    co_selbri.leading_selbri.as_ref(),
                    visible_arguments,
                    source,
                    leading_eventuality,
                )
            }
            SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::NegatedSelbri(negated)) => {
                let result = self.build_selbri_formula_for_visible_arguments(
                    negated.inner_selbri.as_ref(),
                    visible_arguments,
                    source.clone(),
                    connector_locus,
                    leading_eventuality,
                )?;
                let formula = self.build_unary_formula(
                    generated_bridi_negation_operator(&negated.na),
                    result.formula,
                    self.source_for_node(negated, "negated-selbri"),
                )?;
                Ok(GeneratedTanruFormulaForArgument::from_data(data!(
                    GeneratedTanruFormulaForArgument {
                        formula,
                        x1_argument: result.x1_argument.clone(),
                        head_predication: result.head_predication,
                    }
                )))
            }
            SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::ForethoughtSelbriConnection(
                connection,
            )) => self.build_forethought_selbri_connection_formula_for_visible_arguments(
                connection,
                visible_arguments,
                source,
                connector_locus,
                leading_eventuality,
            ),
        }
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_forethought_selbri_connection_formula_for_visible_arguments(
        &mut self,
        connection: &ForethoughtSelbriConnectionSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if !visible_arguments.contains_key(&1) {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            )?;
        }
        let leading = self.build_selbri_formula_for_visible_arguments(
            connection.leading_selbri.as_ref(),
            visible_arguments.clone(),
            source.clone(),
            connector_locus,
            leading_eventuality,
        )?;
        let trailing = self.build_selbri_formula_for_visible_arguments(
            connection.trailing_selbri.as_ref(),
            visible_arguments,
            source.clone(),
            connector_locus,
            None,
        )?;
        let formula = self.build_binary_formula_for_generated_forethought_selbri_connective(
            &connection.guhek,
            &connection.gik,
            connector_locus,
            leading.formula,
            trailing.formula,
            source,
        )?;
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: leading.x1_argument.clone(),
                head_predication: leading.head_predication,
            }
        )))
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
        &mut self,
        unit: &ForethoughtSelbriGroupTanruUnitSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if !visible_arguments.contains_key(&1) {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            )?;
        }
        let leading = self.build_selbri_formula_for_visible_arguments(
            unit.leading_selbri.as_ref(),
            visible_arguments.clone(),
            source.clone(),
            connector_locus,
            leading_eventuality,
        )?;
        let trailing = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
            unit.trailing_unit.as_ref(),
            visible_arguments,
            None,
            source.clone(),
            connector_locus,
        )?;
        let formula = self.build_binary_formula_for_generated_forethought_selbri_connective(
            &unit.guhek,
            &unit.gik,
            connector_locus,
            leading.formula,
            trailing.formula,
            source,
        )?;
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: leading.x1_argument.clone(),
                head_predication: leading.head_predication,
            }
        )))
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_unit_formula_for_visible_arguments(
        &mut self,
        unit: &TanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if !unit.0.links.is_empty() {
            return self.build_connected_tanru_unit_head_formula(
                unit,
                visible_arguments,
                source,
                connector_locus,
                leading_eventuality,
            );
        }
        match unit.0.first.as_ref() {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_tanru_head_relation_formula_for_linked_tanru_unit(
                    unit,
                    visible_arguments,
                    leading_eventuality,
                    source,
                ),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                if leading_eventuality.is_some() {
                    return Err(unsupported("preallocated BO-bound tanru unit eventuality"));
                }
                self.build_bound_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                )
            }
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                    leading_eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
                Err(unsupported("assigned pro-bridi tanru unit formula"))
            }
        }
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_bound_tanru_unit_formula_for_visible_arguments(
        &mut self,
        unit: &BoundTanruUnitSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if let Some(connective) = &unit.bo_connective {
            if !visible_arguments.contains_key(&1) {
                let referent = self.build_elided_referent("zo'e".to_owned())?;
                insert_visible_argument(
                    &mut visible_arguments,
                    1,
                    ArgumentValue::elided(referent, "zo'e".to_owned(), None),
                )?;
            }
            let leading = self.build_tanru_head_relation_formula_for_linked_tanru_unit(
                &unit.leading_unit,
                visible_arguments.clone(),
                None,
                source.clone(),
            )?;
            let trailing = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
                &unit.trailing_unit,
                visible_arguments,
                None,
                source.clone(),
                connector_locus,
            )?;
            let formula = self.build_binary_formula_for_relation_afterthought_connective(
                connective,
                connector_locus,
                leading.formula,
                trailing.formula,
                source,
            )?;
            return Ok(GeneratedTanruFormulaForArgument::from_data(data!(
                GeneratedTanruFormulaForArgument {
                    formula,
                    x1_argument: leading.x1_argument.clone(),
                    head_predication: leading.head_predication,
                }
            )));
        }
        let head = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
            &unit.trailing_unit,
            visible_arguments.clone(),
            None,
            source.clone(),
            connector_locus,
        )?;
        let modifier_arguments = match unit.trailing_unit.as_ref() {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(trailing) => {
                if let Some(linkargs) = &trailing.linkargs {
                    let (_, shifted_arguments) = self.visible_arguments_shifted_after_linkargs(
                        visible_arguments.clone(),
                        linkargs,
                        2,
                    )?;
                    Some(shifted_arguments)
                } else {
                    None
                }
            }
            _ => None,
        };
        let modifier = match modifier_arguments {
            Some(arguments) => self
                .build_property_abstraction_for_linked_tanru_unit_with_visible_arguments(
                    &unit.leading_unit,
                    arguments,
                    source.clone(),
                )?,
            None => self.build_property_abstraction_for_linked_tanru_unit(
                &unit.leading_unit,
                source.clone(),
            )?,
        };
        let relation_formula = self.build_tanru_relation_formula(
            head.x1_argument.clone(),
            modifier,
            tanru_unit_label_from_bound_tanru_unit(unit)?,
            head.head_predication,
            PredicationMode::Asserted,
            source.clone(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![head.formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: connector_locus.to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: head.x1_argument.clone(),
                head_predication: head.head_predication,
            }
        )))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula(
        &mut self,
        unit: &TanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_head_relation_formula_with_modal_terms(
            unit,
            visible_arguments,
            eventuality,
            source,
            &[],
        )
    }

    #[requires(true)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula_with_modal_terms(
        &mut self,
        unit: &TanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        modal_terms: &[TaggedSumtiTermSyntax],
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if !unit.0.links.is_empty() {
            if eventuality.is_some() {
                return Err(unsupported(
                    "preallocated connected tanru unit head eventuality",
                ));
            }
            if !modal_terms.is_empty() {
                return Err(unsupported("modal terms on connected tanru unit head"));
            }
            return self.build_connected_tanru_unit_head_formula(
                unit,
                visible_arguments,
                source,
                "tanru-unit",
                None,
            );
        }
        match unit.0.first.as_ref() {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_tanru_head_relation_formula_from_parts(
                    &unit.base,
                    unit.linkargs.as_ref(),
                    visible_arguments,
                    eventuality,
                    source,
                    modal_terms,
                ),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                if !modal_terms.is_empty() {
                    return Err(unsupported("modal terms on BO-bound tanru head"));
                }
                if eventuality.is_some() {
                    return Err(unsupported("preallocated BO-bound tanru head eventuality"));
                }
                self.build_bound_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    "tanru-unit",
                )
            }
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    "tanru-unit",
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
                Err(unsupported("assigned pro-bridi tanru unit head"))
            }
        }
    }

    #[requires(!unit.0.links.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_tanru_unit_head_formula(
        &mut self,
        unit: &TanruUnitSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if !visible_arguments.contains_key(&1) {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            )?;
        }
        let leading = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
            &unit.0.first,
            visible_arguments.clone(),
            leading_eventuality,
            source.clone(),
            connector_locus,
        )?;
        let mut formula = leading.formula;
        for link in &unit.0.links {
            let trailing = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
                &link.trailing_unit,
                visible_arguments.clone(),
                None,
                source.clone(),
                connector_locus,
            )?;
            formula = self.build_binary_formula_for_relation_afterthought_connective(
                &link.connective,
                connector_locus,
                formula,
                trailing.formula,
                source.clone(),
            )?;
        }
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: leading.x1_argument.clone(),
                head_predication: leading.head_predication,
            }
        )))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_tanru_head_relation_formula_for_linked_tanru_unit(
                    unit,
                    visible_arguments,
                    eventuality,
                    source,
                ),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => self
                .build_bound_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                ),
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
                Err(unsupported("assigned pro-bridi tanru unit head"))
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_head_relation_formula_from_parts(
            &unit.base,
            unit.linkargs.as_ref(),
            visible_arguments,
            eventuality,
            source,
            &[],
        )
    }

    #[requires(true)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula_from_parts(
        &mut self,
        atom: &TanruUnitAtomSyntax,
        linkargs: Option<&LinkargsSyntax>,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        modal_terms: &[TaggedSumtiTermSyntax],
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        if let Some(scalar_unit) = scalar_unit
            && let Some((grouped, inner_conversions)) =
                scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
            if !modal_terms.is_empty() {
                return Err(unsupported("modal terms on grouped scalar tanru head"));
            }
            if linkargs.is_some() {
                return Err(unsupported("scoped scalar grouped tanru unit head"));
            }
            visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
                &atom.conversions,
            )?;
            visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
                inner_conversions,
            )?;
            let result = self
                .build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
                &grouped.selbri,
                visible_arguments,
                source.clone(),
                eventuality,
            )?;
            self.apply_scalar_negation_to_tanru_links(
                result.formula,
                scalar_negation_for_marker(&scalar_unit.nahe)
                    .with_argument_scope(vec!["x1".to_owned()]),
            )?;
            return Ok(GeneratedTanruFormulaForArgument::from_data(data!(
                GeneratedTanruFormulaForArgument {
                    formula: self
                        .detach_tanru_relation_formula_without_positive_head(result.formula)
                        .unwrap_or(result.formula),
                    x1_argument: result.x1_argument.clone(),
                    head_predication: result.head_predication,
                }
            )));
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = atom.base.as_ref() {
            if !modal_terms.is_empty() {
                return Err(unsupported("modal terms on grouped tanru head"));
            }
            if linkargs.is_some() {
                return Err(unsupported("scoped grouped tanru unit head"));
            }
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
                &atom.conversions,
            )?;
            return self.build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
                &grouped.selbri,
                visible_arguments,
                source,
                eventuality,
            );
        }
        let relation = semantic_relation_label(relation_label_from_tanru_unit_atom_base(
            atom.base.as_ref(),
        )?);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => {
                let eventuality = self.next_eventuality_id();
                self.insert(
                    eventuality,
                    SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
                )?;
                eventuality
            }
        };
        let visible_x1_argument = visible_arguments.get(&1).cloned();
        if let Some(linkargs) = linkargs {
            let (_, adjusted_arguments) =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted_arguments;
        }
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated tanru head arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        let modal_arguments = self.build_modal_arguments_for_generated_tagged_terms(modal_terms)?;
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if arguments.contains_key(&key) {
                continue;
            }
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                key,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let x1_argument = visible_x1_argument
            .or_else(|| arguments.get("x1").cloned())
            .ok_or_else(|| unsupported("tanru without visible x1"))?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation,
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source.clone(),
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(GeneratedTanruFormulaForArgument::from_data(data!(
            GeneratedTanruFormulaForArgument {
                formula,
                x1_argument,
                head_predication: predication,
            }
        )))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    fn build_term_assignments_for_terms<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        let mut visible_arguments = BTreeMap::new();
        let mut modal_terms = Vec::new();
        let mut formula_scopes = Vec::new();
        let mut next_visible_place = first_visible_place;
        for term in terms {
            self.insert_generated_term_assignment(
                &mut visible_arguments,
                &mut modal_terms,
                &mut formula_scopes,
                &mut next_visible_place,
                term,
            )?;
        }
        Ok(GeneratedTermAssignments {
            visible_arguments,
            modal_terms,
            formula_scopes,
        })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_sumti_selbri_formula_for_terms(
        &mut self,
        sumti_selbri: &SumtiSelbriTanruUnitSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti_selbri.moi_marker.is_some() {
            return Err(unsupported("MOI sumti selbri"));
        }
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            arguments.insert(argument_key(visible_place), argument);
        }
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
        )?;
        let source_operand = self.build_sumti_selbri_source_operand(&sumti_selbri.sumti)?;
        if !arguments.contains_key("x1") {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                "x1".to_owned(),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        arguments.insert("x2".to_owned(), ArgumentValue::filled(source_operand, None));
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            "referentOf".to_owned(),
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source.clone(),
            Vec::new(),
        );
        predication_object.modal_arguments =
            self.build_modal_arguments_for_generated_tagged_terms(&assignments.modal_terms)?;
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_sumti_selbri_formula_for_argument(
        &mut self,
        sumti_selbri: &SumtiSelbriTanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti_selbri.moi_marker.is_some() {
            return Err(unsupported("MOI sumti selbri"));
        }
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, source.clone()),
        )?;
        let source_operand = self.build_sumti_selbri_source_operand(&sumti_selbri.sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), argument);
        arguments.insert("x2".to_owned(), ArgumentValue::filled(source_operand, None));
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                "referentOf".to_owned(),
                Some(eventuality),
                arguments,
                mode,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_sumti_selbri_source_operand(
        &mut self,
        sumti: &SumtiSelbriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti {
            SumtiSelbriSumtiSyntax::Sumti(sumti) => self.build_sumti_referent(sumti),
            SumtiSelbriSumtiSyntax::MeLerfuSumti(_) => Err(unsupported("ME lerfu sumti")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_property_abstraction_for_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_abstraction_for_tanru_run(unit, &[], source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_property_abstraction_for_tanru_run(
        &mut self,
        first_unit: &TanruUnitSyntax,
        additional_units: &[TanruUnitSyntax],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if additional_units.is_empty() {
            return self.build_property_abstraction_for_single_tanru_unit(first_unit, source);
        }
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_property_formula_for_tanru_run(
            first_unit,
            additional_units,
            parameter,
            source.clone(),
            GeneratedPropertyTanruContext::PropertyAbstraction,
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_property_abstraction_for_single_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(composition) =
            self.build_property_composition_for_generated_tanru_unit(unit, source.clone())?
        {
            return Ok(composition);
        }
        let abstraction = abstraction_from_generated_tanru_unit(unit)?.cloned();
        if let Some(abstraction) = abstraction {
            let kind = abstraction_kind_for_nu(&abstraction);
            let parameter = self.next_parameter_id();
            self.insert(
                parameter,
                SemanticObject::parameter(
                    abstraction_output_sort(kind),
                    ParameterRole::PropertySlot,
                    "ce'u".to_owned(),
                    source.clone(),
                ),
            )?;
            let body = self.build_abstraction_link_formula_for_visible_argument(
                &abstraction,
                Some(ArgumentValue::filled(parameter, None)),
                source.clone(),
                PredicationMode::Restrictive,
            )?;
            return self.build_property_abstraction_output(body, vec![parameter], source);
        }
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_property_formula_for_tanru_unit(unit, parameter, source.clone())?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameters.iter().all(|parameter| parameter.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    fn build_property_abstraction_output(
        &mut self,
        body: SemanticObjectId,
        parameters: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = self.next_relation_id();
        self.insert(
            relation,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                parameters,
                source,
                Vec::new(),
            ),
        )?;
        Ok(relation)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_selbri(
        &mut self,
        selbri: &SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(tanru) = tanru_selbri_from_selbri(selbri)?
            && !tanru.additional_units.is_empty()
        {
            return self.build_property_formula_for_tanru_selbri(
                tanru,
                parameter,
                source,
                GeneratedPropertyTanruContext::Description,
            );
        }
        let relation = semantic_relation_label(relation_label_from_selbri(selbri)?);
        self.build_property_atom_for_relation(relation, parameter, source)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_tanru_selbri(
        &mut self,
        tanru: &TanruSelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_formula_for_tanru_run(
            &tanru.first_unit,
            &tanru.additional_units,
            parameter,
            source,
            context,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_connected_selbri(
        &mut self,
        selbri: &ConnectedSelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = self.build_property_formula_for_tanru_selbri(
            &selbri.leading_selbri,
            parameter,
            source.clone(),
            context,
        )?;
        for continuation in &selbri.continuations {
            let trailing = self.build_property_formula_for_tanru_selbri(
                &continuation.trailing_selbri,
                parameter,
                source.clone(),
                context,
            )?;
            formula = self.build_binary_formula_for_relation_afterthought_connective(
                &continuation.connective,
                context.connector_locus(),
                formula,
                trailing,
                source.clone(),
            )?;
        }
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_tanru_run(
        &mut self,
        first_unit: &TanruUnitSyntax,
        additional_units: &[TanruUnitSyntax],
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some((trailing_unit, modifier_units)) = additional_units.split_last() else {
            return match context {
                GeneratedPropertyTanruContext::Description => self
                    .build_description_property_formula_for_tanru_unit(
                        first_unit, parameter, source,
                    ),
                GeneratedPropertyTanruContext::PropertyAbstraction => {
                    self.build_property_formula_for_tanru_unit(first_unit, parameter, source)
                }
            };
        };
        let tertau_source = match context {
            GeneratedPropertyTanruContext::Description => {
                source_with_construct(source.clone(), "restrictive-predication")
            }
            GeneratedPropertyTanruContext::PropertyAbstraction => source.clone(),
        };
        let tertau_formula = match context {
            GeneratedPropertyTanruContext::Description => self
                .build_description_property_formula_for_tanru_unit(
                    trailing_unit,
                    parameter,
                    tertau_source,
                )?,
            GeneratedPropertyTanruContext::PropertyAbstraction => {
                self.build_property_formula_for_tanru_unit(trailing_unit, parameter, tertau_source)?
            }
        };
        let head_predication = self.primary_predication_for_formula(tertau_formula)?;
        let modifier = self.build_property_abstraction_for_tanru_run(
            first_unit,
            modifier_units,
            source.clone(),
        )?;
        let relation_formula = self.build_tanru_relation_formula(
            ArgumentValue::filled(parameter, None),
            modifier,
            tanru_relation_name_for_generated_unit_run(
                first_unit,
                modifier_units,
                trailing_unit,
                true,
            )?,
            head_predication,
            PredicationMode::Restrictive,
            source.clone(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![tertau_formula, relation_formula],
                Some(Connector {
                    source: "tanru".to_owned(),
                    locus: context.connector_locus().to_owned(),
                    truth_table: None,
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !unit.0.links.is_empty() {
            return self
                .build_connected_property_formula_for_tanru_unit_chain(unit, parameter, source);
        }
        if let Some(sumti_selbri) = sumti_selbri_from_generated_tanru_unit(unit)? {
            return self.build_sumti_selbri_formula_for_argument(
                sumti_selbri,
                ArgumentValue::filled(parameter, None),
                PredicationMode::Restrictive,
                source,
            );
        }
        self.build_property_formula_for_bo_or_linked_tanru_unit(&unit.0.first, parameter, source)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_description_property_formula_for_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !unit.0.links.is_empty() {
            return self.build_connected_property_formula_for_tanru_unit_chain_with_locus(
                unit,
                parameter,
                source,
                "tanru-unit",
            );
        }
        if let Some(sumti_selbri) = sumti_selbri_from_generated_tanru_unit(unit)? {
            return self.build_sumti_selbri_formula_for_argument(
                sumti_selbri,
                ArgumentValue::filled(parameter, None),
                PredicationMode::Restrictive,
                source,
            );
        }
        self.build_description_property_formula_for_bo_or_linked_tanru_unit(
            &unit.0.first,
            parameter,
            source,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_description_property_formula_for_bo_or_linked_tanru_unit(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_description_property_formula_for_linked_tanru_unit(unit, parameter, source),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                self.build_property_formula_for_bound_tanru_unit(unit, parameter, source)
            }
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_property_formula_for_forethought_selbri_group_tanru_unit(
                    unit, parameter, source,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
                Err(unsupported("assigned pro-bridi description tanru unit"))
            }
        }
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_description_property_formula_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(scalar_unit) = scalar_negated_tanru_atom_base(unit.base.base.as_ref())
            && let Some((grouped, _)) = scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
            let formula = self.build_property_formula_for_grouped_tanru_unit(
                grouped,
                parameter,
                source.clone(),
            )?;
            self.apply_scalar_negation_to_tanru_links(
                formula,
                scalar_negation_for_marker(&scalar_unit.nahe),
            )?;
            return Ok(self
                .detach_tanru_relation_formula_without_positive_head(formula)
                .unwrap_or(formula));
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = unit.base.base.as_ref() {
            return self.build_property_formula_for_grouped_tanru_unit(grouped, parameter, source);
        }
        let mut visible_arguments = BTreeMap::new();
        insert_visible_argument(
            &mut visible_arguments,
            1,
            ArgumentValue::filled(parameter, None),
        )?;
        self.build_description_relation_formula_for_tanru_unit_atom_with_visible_arguments(
            &unit.base,
            unit.linkargs.as_ref(),
            visible_arguments,
            source.clone(),
            source,
        )
    }

    #[requires(!unit.0.links.is_empty())]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_property_formula_for_tanru_unit_chain(
        &mut self,
        unit: &TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_connected_property_formula_for_tanru_unit_chain_with_locus(
            unit,
            parameter,
            source,
            "property-abstraction",
        )
    }

    #[requires(!unit.0.links.is_empty())]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_connected_property_formula_for_tanru_unit_chain_with_locus(
        &mut self,
        unit: &TanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        locus: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = self.build_property_formula_for_bo_or_linked_tanru_unit(
            &unit.0.first,
            parameter,
            source.clone(),
        )?;
        for link in &unit.0.links {
            let trailing = self.build_property_formula_for_bo_or_linked_tanru_unit(
                &link.trailing_unit,
                parameter,
                source.clone(),
            )?;
            formula = self.build_binary_formula_for_relation_afterthought_connective(
                &link.connective,
                locus,
                formula,
                trailing,
                source.clone(),
            )?;
        }
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    fn build_property_composition_for_generated_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if unit.0.links.is_empty()
            || unit
                .0
                .links
                .iter()
                .any(|link| generated_relation_afterthought_connective_is_logical(&link.connective))
        {
            return Ok(None);
        }
        let mut current = self.build_property_abstraction_for_bo_or_linked_tanru_unit(
            unit.0.first.as_ref(),
            source.clone(),
        )?;
        for link in &unit.0.links {
            let trailing = self.build_property_abstraction_for_bo_or_linked_tanru_unit(
                link.trailing_unit.as_ref(),
                source.clone(),
            )?;
            current = self.build_property_composition_from_generated_connective(
                current,
                &link.connective,
                trailing,
                source.clone(),
            )?;
        }
        Ok(Some(current))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_property_abstraction_for_bo_or_linked_tanru_unit(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = self.build_property_formula_for_bo_or_linked_tanru_unit(
            unit,
            parameter,
            source.clone(),
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_property_composition_from_generated_connective(
        &mut self,
        left: SemanticObjectId,
        connective: &RelationAfterthoughtConnectiveSyntax,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut members = vec![left, right];
        if generated_relation_afterthought_connective_reverses_composition_members(connective) {
            members.reverse();
        }
        let operator = generated_nonlogical_composition_operator(connective)?;
        let collective = (operator == "mass").then_some(true);
        let id = self.next_referent_with_sort_id(SemanticSort::Concept);
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
                    members,
                    excluded_members: Vec::new(),
                    collective,
                    scalar_negated: None,
                    complement: None,
                    endpoint_inclusion: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_bo_or_linked_tanru_unit(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
                self.build_property_formula_for_linked_tanru_unit(unit, parameter, source)
            }
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                self.build_property_formula_for_bound_tanru_unit(unit, parameter, source)
            }
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_property_formula_for_forethought_selbri_group_tanru_unit(
                    unit, parameter, source,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
                Err(unsupported("assigned pro-bridi tanru unit"))
            }
        }
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_forethought_selbri_group_tanru_unit(
        &mut self,
        unit: &ForethoughtSelbriGroupTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading = self.build_property_formula_for_forethought_tanru_branch_selbri(
            unit.leading_selbri.as_ref(),
            parameter,
            source.clone(),
        )?;
        let trailing = self.build_property_formula_for_bo_or_linked_tanru_unit(
            unit.trailing_unit.as_ref(),
            parameter,
            source.clone(),
        )?;
        self.build_binary_formula_for_generated_forethought_selbri_connective(
            &unit.guhek,
            &unit.gik,
            "property-abstraction",
            leading,
            trailing,
            source,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_forethought_tanru_branch_selbri(
        &mut self,
        selbri: &SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(tanru) = tanru_selbri_from_selbri(selbri)? {
            return self.build_property_formula_for_tanru_selbri(
                tanru,
                parameter,
                source,
                GeneratedPropertyTanruContext::PropertyAbstraction,
            );
        }
        self.build_property_formula_for_selbri(selbri, parameter, source)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_bound_tanru_unit(
        &mut self,
        unit: &BoundTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(connective) = &unit.bo_connective {
            let leading = self.build_property_formula_for_linked_tanru_unit(
                &unit.leading_unit,
                parameter,
                source.clone(),
            )?;
            let trailing = self.build_property_formula_for_bo_or_linked_tanru_unit(
                &unit.trailing_unit,
                parameter,
                source.clone(),
            )?;
            return self.build_binary_formula_for_relation_afterthought_connective(
                connective,
                "property-abstraction",
                leading,
                trailing,
                source,
            );
        }
        let tertau_formula = self.build_property_formula_for_bo_or_linked_tanru_unit(
            &unit.trailing_unit,
            parameter,
            source.clone(),
        )?;
        let head_predication = self.primary_predication_for_formula(tertau_formula)?;
        let modifier = self
            .build_property_abstraction_for_linked_tanru_unit(&unit.leading_unit, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            ArgumentValue::filled(parameter, None),
            modifier,
            tanru_unit_label_from_bound_tanru_unit(unit)?,
            head_predication,
            PredicationMode::Restrictive,
            source.clone(),
        )?;
        let formula = self.next_formula_id();
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
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(scalar_unit) = scalar_negated_tanru_atom_base(unit.base.base.as_ref())
            && let Some((grouped, _)) = scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
            let formula = self.build_property_formula_for_grouped_tanru_unit(
                grouped,
                parameter,
                source.clone(),
            )?;
            self.apply_scalar_negation_to_tanru_links(
                formula,
                scalar_negation_for_marker(&scalar_unit.nahe),
            )?;
            return Ok(self
                .detach_tanru_relation_formula_without_positive_head(formula)
                .unwrap_or(formula));
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = unit.base.base.as_ref() {
            return self.build_property_formula_for_grouped_tanru_unit(grouped, parameter, source);
        }
        self.build_eventful_relation_formula_for_linked_tanru_unit_argument(
            unit,
            ArgumentValue::filled(parameter, None),
            PredicationMode::Restrictive,
            source.clone(),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    fn build_property_abstraction_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body =
            self.build_property_formula_for_linked_tanru_unit(unit, parameter, source.clone())?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    fn build_property_abstraction_for_linked_tanru_unit_with_visible_arguments(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        visible_arguments.insert(1, ArgumentValue::filled(parameter, None));
        let body = self.build_property_formula_for_linked_tanru_unit_with_visible_arguments(
            unit,
            visible_arguments,
            source.clone(),
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_linked_tanru_unit_with_visible_arguments(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
            &unit.base,
            unit.linkargs.as_ref(),
            visible_arguments,
            PredicationMode::Restrictive,
            source.clone(),
            source,
        )
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
        &mut self,
        atom: &TanruUnitAtomSyntax,
        linkargs: Option<&LinkargsSyntax>,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(linkargs) = linkargs {
            let (_, adjusted_arguments) =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted_arguments;
        }
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        let relation = semantic_relation_label(match scalar_unit {
            Some(unit) => relation_label_from_scalar_negated_tanru_unit(unit)?,
            None => relation_label_from_tanru_unit_atom_base(atom.base.as_ref())?,
        });
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, predication_source.clone()),
        )?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let place = match scalar_unit.and_then(scalar_negated_tanru_unit_inner_atom) {
                Some(inner_atom) => {
                    mapped_place_for_generated_conversions(place, &inner_atom.conversions)?
                }
                None => place,
            };
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated tanru arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        for place in 1..=place_limit {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let elided = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(key, ArgumentValue::elided(elided, "zo'e".to_owned(), None));
            }
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.clone(),
                Some(eventuality),
                arguments,
                predication_mode_for_relation(&relation, mode),
                predication_source,
                diagnostics,
            ),
        )?;
        if let Some(scalar_negation) =
            scalar_unit.map(|unit| scalar_negation_for_marker(&unit.nahe))
        {
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_description_relation_formula_for_tanru_unit_atom_with_visible_arguments(
        &mut self,
        atom: &TanruUnitAtomSyntax,
        linkargs: Option<&LinkargsSyntax>,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(linkargs) = linkargs {
            let (_, adjusted_arguments) =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted_arguments;
        }
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        let relation = semantic_relation_label(match scalar_unit {
            Some(unit) => relation_label_from_scalar_negated_tanru_unit(unit)?,
            None => relation_label_from_tanru_unit_atom_base(atom.base.as_ref())?,
        });
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let place = match scalar_unit.and_then(scalar_negated_tanru_unit_inner_atom) {
                Some(inner_atom) => {
                    mapped_place_for_generated_conversions(place, &inner_atom.conversions)?
                }
                None => place,
            };
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated description tanru arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let elided = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(key, ArgumentValue::elided(elided, "zo'e".to_owned(), None));
            }
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                PredicationMode::Restrictive,
                predication_source,
                diagnostics,
            ),
        )?;
        if let Some(scalar_negation) =
            scalar_unit.map(|unit| scalar_negation_for_marker(&unit.nahe))
        {
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_formula_for_grouped_tanru_unit(
        &mut self,
        grouped: &GroupedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_formula_for_connected_selbri(
            &grouped.selbri,
            parameter,
            source,
            GeneratedPropertyTanruContext::PropertyAbstraction,
        )
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_generated_tanru_unit_argument(
        &mut self,
        unit: &TanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_relation_formula_for_generated_tanru_unit_argument_with_eventuality(
            unit,
            argument,
            None,
            mode,
            None,
            predication_source,
            formula_source,
        )
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_eventful_relation_formula_for_generated_tanru_unit_argument(
        &mut self,
        unit: &TanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, predication_source.clone()),
        )?;
        self.build_relation_formula_for_generated_tanru_unit_argument_with_eventuality(
            unit,
            argument,
            Some(eventuality),
            mode,
            None,
            predication_source,
            formula_source,
        )
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_eventful_relation_formula_for_linked_tanru_unit_argument(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.next_eventuality_id();
        self.insert(
            eventuality,
            SemanticObject::eventuality(EventualityClass::Event, None, predication_source.clone()),
        )?;
        self.build_relation_formula_for_linked_tanru_unit_argument_with_eventuality(
            unit,
            argument,
            Some(eventuality),
            mode,
            None,
            predication_source,
            formula_source,
        )
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[requires(eventuality.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_linked_tanru_unit_argument_with_eventuality(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        argument: ArgumentValue,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        scalar_negation: Option<ScalarNegation>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let atom = unit.base.as_ref();
        let linkargs = unit.linkargs.as_ref();
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        let relation = semantic_relation_label(match scalar_unit {
            Some(unit) => relation_label_from_scalar_negated_tanru_unit(unit)?,
            None => relation_label_from_tanru_unit_atom_base(atom.base.as_ref())?,
        });
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let mut visible_arguments = BTreeMap::new();
        insert_visible_argument(&mut visible_arguments, 1, argument)?;
        if let Some(linkargs) = linkargs {
            self.add_linkargs_arguments(&mut visible_arguments, linkargs, 2)?;
        }
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let place = match scalar_unit.and_then(scalar_negated_tanru_unit_inner_atom) {
                Some(inner_atom) => {
                    mapped_place_for_generated_conversions(place, &inner_atom.conversions)?
                }
                None => place,
            };
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated tanru arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let elided = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(key, ArgumentValue::elided(elided, "zo'e".to_owned(), None));
            }
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.clone(),
                eventuality,
                arguments,
                predication_mode_for_relation(&relation, mode),
                predication_source,
                diagnostics,
            ),
        )?;
        let scalar_negation = match (scalar_negation, scalar_unit) {
            (Some(scalar_negation), _) => Some(scalar_negation),
            (None, Some(unit)) => Some(scalar_negation_for_marker(&unit.nahe)),
            (None, None) => None,
        };
        if let Some(scalar_negation) = scalar_negation {
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[requires(eventuality.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_relation_formula_for_generated_tanru_unit_argument_with_eventuality(
        &mut self,
        unit: &TanruUnitSyntax,
        argument: ArgumentValue,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        scalar_negation: Option<ScalarNegation>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (atom, linkargs) = generated_linked_tanru_unit_parts(unit)?;
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        let relation = semantic_relation_label(match scalar_unit {
            Some(unit) => relation_label_from_scalar_negated_tanru_unit(unit)?,
            None => relation_label_from_tanru_unit_atom_base(atom.base.as_ref())?,
        });
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let mut visible_arguments = BTreeMap::new();
        insert_visible_argument(&mut visible_arguments, 1, argument)?;
        if let Some(linkargs) = linkargs {
            self.add_linkargs_arguments(&mut visible_arguments, linkargs, 2)?;
        }
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let place = match scalar_unit.and_then(scalar_negated_tanru_unit_inner_atom) {
                Some(inner_atom) => {
                    mapped_place_for_generated_conversions(place, &inner_atom.conversions)?
                }
                None => place,
            };
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated tanru arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(1)
            }
        };
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let elided = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(key, ArgumentValue::elided(elided, "zo'e".to_owned(), None));
            }
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.clone(),
                eventuality,
                arguments,
                predication_mode_for_relation(&relation, mode),
                predication_source,
                diagnostics,
            ),
        )?;
        let scalar_negation = match (scalar_negation, scalar_unit) {
            (Some(scalar_negation), _) => Some(scalar_negation),
            (None, Some(unit)) => Some(scalar_negation_for_marker(&unit.nahe)),
            (None, None) => None,
        };
        if let Some(scalar_negation) = scalar_negation {
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|place| *place >= first_visible_place) || ret.is_err())]
    fn add_linkargs_arguments(
        &mut self,
        arguments: &mut BTreeMap<usize, ArgumentValue>,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<usize, SemanticsError> {
        let mut next_visible_place = first_visible_place;
        self.add_linked_sumti_argument(arguments, &mut next_visible_place, &linkargs.first_link)?;
        for link in &linkargs.bei_links {
            self.add_linked_sumti_argument(arguments, &mut next_visible_place, &link.link)?;
        }
        Ok(next_visible_place)
    }

    #[requires(first_visible_place > 0)]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|(_, arguments)| arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    fn visible_arguments_adjusted_for_linkargs(
        &mut self,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<(usize, BTreeMap<usize, ArgumentValue>), SemanticsError> {
        let mut linkarg_arguments = BTreeMap::new();
        let mut next_tail_place =
            self.add_linkargs_arguments(&mut linkarg_arguments, linkargs, first_visible_place)?;
        let mut adjusted_arguments = BTreeMap::new();
        for (place, argument) in visible_arguments
            .iter()
            .filter(|(place, _)| **place < first_visible_place)
        {
            insert_visible_argument(&mut adjusted_arguments, *place, argument.clone())?;
        }
        for (place, argument) in linkarg_arguments {
            insert_visible_argument(&mut adjusted_arguments, place, argument)?;
        }
        for (_, argument) in visible_arguments
            .into_iter()
            .filter(|(place, _)| *place >= first_visible_place)
        {
            while adjusted_arguments.contains_key(&next_tail_place) {
                next_tail_place += 1;
            }
            insert_visible_argument(&mut adjusted_arguments, next_tail_place, argument)?;
            next_tail_place += 1;
        }
        Ok((next_tail_place, adjusted_arguments))
    }

    #[requires(first_visible_place > 0)]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|(_, arguments)| arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    fn visible_arguments_shifted_after_linkargs(
        &self,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<(usize, BTreeMap<usize, ArgumentValue>), SemanticsError> {
        let mut next_tail_place = next_visible_place_after_linkargs(linkargs, first_visible_place)?;
        let mut adjusted_arguments = BTreeMap::new();
        for (place, argument) in visible_arguments
            .iter()
            .filter(|(place, _)| **place < first_visible_place)
        {
            insert_visible_argument(&mut adjusted_arguments, *place, argument.clone())?;
        }
        for (_, argument) in visible_arguments
            .into_iter()
            .filter(|(place, _)| *place >= first_visible_place)
        {
            insert_visible_argument(&mut adjusted_arguments, next_tail_place, argument)?;
            next_tail_place += 1;
        }
        Ok((next_tail_place, adjusted_arguments))
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn add_linked_sumti_argument(
        &mut self,
        arguments: &mut BTreeMap<usize, ArgumentValue>,
        next_visible_place: &mut usize,
        link: &LinkedSumtiSyntax,
    ) -> Result<(), SemanticsError> {
        match link {
            LinkedSumtiSyntax::PlainLinkedSumti(sumti) => {
                let argument = self.build_argument_for_generated_sumti(&sumti.0)?;
                insert_visible_argument(arguments, *next_visible_place, argument)?;
                *next_visible_place += 1;
            }
            LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
                let place = fa_place(&sumti.fa.value)?;
                let argument = self.build_tagged_or_elided_sumti_argument(&sumti.sumti)?;
                insert_visible_argument(arguments, place, argument)?;
                *next_visible_place = (*next_visible_place).max(place + 1);
            }
            LinkedSumtiSyntax::TenseTaggedLinkedSumti(_) => {
                return Err(unsupported("tense-tagged linked sumti"));
            }
            LinkedSumtiSyntax::EmptyLinkedSumti(_) => {
                return Err(unsupported("empty linked sumti"));
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some()) || ret.is_err())]
    fn build_tagged_or_elided_sumti_argument(
        &mut self,
        sumti: &TaggedOrElidedSumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        match sumti {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.build_argument_for_generated_sumti(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => {
                let referent = self.build_elided_referent("zo'e".to_owned())?;
                Ok(ArgumentValue::elided(referent, "zo'e".to_owned(), None))
            }
        }
    }

    #[requires(!relation.is_empty())]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_property_atom_for_relation(
        &mut self,
        relation: String,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                1
            }
        };
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(parameter, None));
        for place in 2..=place_limit {
            let elided = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(elided, "zo'e".to_owned(), None),
            );
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                PredicationMode::Restrictive,
                source.clone(),
                diagnostics,
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication) || ret.is_err())]
    fn primary_predication_for_atom_formula(
        &self,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let object = self.objects.get(&formula).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find formula {formula} for predication lookup"
            ))
        })?;
        object
            .predication
            .ok_or_else(|| unsupported("property formula without a primary predication"))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication) || ret.is_err())]
    fn primary_predication_for_formula(
        &self,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let object = self.objects.get(&formula).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find formula {formula} for predication lookup"
            ))
        })?;
        if let Some(predication) = object.predication {
            return Ok(predication);
        }
        for child in &object.children {
            if let Ok(predication) = self.primary_predication_for_formula(*child) {
                return Ok(predication);
            }
        }
        if let Some(restriction) = object.restriction
            && let Ok(predication) = self.primary_predication_for_formula(restriction)
        {
            return Ok(predication);
        }
        if let Some(body) = object.body
            && let Ok(predication) = self.primary_predication_for_formula(body)
        {
            return Ok(predication);
        }
        Err(invalid_graph(format!(
            "formula {formula} has no primary predication"
        )))
    }

    #[requires(!relation_label.is_empty())]
    #[requires(head_predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_relation_formula(
        &mut self,
        x1_argument: ArgumentValue,
        modifier: SemanticObjectId,
        relation_label: String,
        head_predication: SemanticObjectId,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), x1_argument);
        arguments.insert("x2".to_owned(), ArgumentValue::filled(modifier, None));
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::tanru_link_predication(
                "tanru".to_owned(),
                None,
                arguments,
                TanruLink::new(head_predication, modifier, relation_label),
                mode,
                source.clone(),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_binary_formula_for_relation_afterthought_connective(
        &mut self,
        connective: &RelationAfterthoughtConnectiveSyntax,
        locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_relation_afterthought_connective_formula_operator(connective);
        let left_formula = if generated_relation_afterthought_connective_negates_left(connective) {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right_formula = if generated_relation_afterthought_connective_negates_right(connective)
        {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        self.mark_generated_whether_or_not_inert_operand(connective, left, right);
        let children = if generated_relation_afterthought_connective_has_se(connective)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right_formula, left_formula]
        } else {
            vec![left_formula, right_formula]
        };
        let connector_source = generated_relation_afterthought_connective_source(connective)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(Connector {
                    source: connector_source,
                    locus: locus.to_owned(),
                    truth_table: generated_relation_afterthought_connective_truth_table(connective),
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_binary_formula_for_generated_forethought_selbri_connective(
        &mut self,
        guhek: &GuhekConnectiveSyntax,
        gik: &GikConnectiveSyntax,
        locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_guhek_connective_formula_operator(guhek);
        let left_formula = if generated_guhek_connective_negates_left(guhek) {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right_formula = if generated_gik_connective_negates_right(gik) {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        self.mark_generated_forethought_whether_or_not_inert_operand(guhek, left, right);
        let children = if generated_guhek_connective_has_se(guhek)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right_formula, left_formula]
        } else {
            vec![left_formula, right_formula]
        };
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(Connector {
                    source: generated_guhek_connective_source(guhek),
                    locus: locus.to_owned(),
                    truth_table: generated_guhek_gik_connective_truth_table(guhek, gik),
                    parameter: None,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(child.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_unary_formula(
        &mut self,
        operator: FormulaOperator,
        child: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(operator, vec![child], None, source, Vec::new()),
        )?;
        Ok(formula)
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

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn mark_generated_whether_or_not_inert_operand(
        &mut self,
        connective: &RelationAfterthoughtConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
    ) {
        if generated_relation_afterthought_connective_formula_operator(connective)
            != FormulaOperator::WhetherOrNot
        {
            return;
        }
        let inert = if generated_relation_afterthought_connective_has_se(connective) {
            left
        } else {
            right
        };
        self.set_formula_predication_mode(inert, PredicationMode::Inert);
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn mark_generated_forethought_whether_or_not_inert_operand(
        &mut self,
        guhek: &GuhekConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
    ) {
        if generated_guhek_connective_formula_operator(guhek) != FormulaOperator::WhetherOrNot {
            return;
        }
        let inert = if generated_guhek_connective_has_se(guhek) {
            left
        } else {
            right
        };
        self.set_formula_predication_mode(inert, PredicationMode::Inert);
    }

    #[requires(!introduced_by.is_empty())]
    #[requires(!word.is_empty())]
    #[requires(definition.is_none_or(|id| crate::model::argument_object_kind_can_fill(id.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn insert_scalar_negation_scale_referent(
        &mut self,
        introduced_by: &str,
        word: &str,
        definition: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_with_sort_id(SemanticSort::Scale);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Scale,
                None,
                Some(Descriptor {
                    kind: "scale".to_owned(),
                    word: word.to_owned(),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: Some(introduced_by.to_owned()),
                    scale: None,
                    definiteness: None,
                    operand: definition,
                }),
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|negation| negation.scale.is_some()) || ret.is_err())]
    fn scalar_negation_with_scale_for_modal_arguments(
        &mut self,
        scalar_negation: ScalarNegation,
        modal_arguments: &[ModalArgument],
        fallback_source: Option<crate::model::SemanticSource>,
    ) -> Result<ScalarNegation, SemanticsError> {
        if scalar_negation.scale.is_some() {
            return Ok(scalar_negation);
        }
        let scale_definition = modal_arguments
            .iter()
            .find_map(scalar_scale_definition_for_modal_argument);
        let definition = scale_definition.as_ref().map(|definition| definition.value);
        let word = scale_definition
            .as_ref()
            .map(|definition| definition.introduced_by.as_str())
            .unwrap_or("implicit scalar scale");
        let source = scale_definition
            .as_ref()
            .and_then(|definition| definition.source.clone())
            .or(fallback_source)
            .map(source_as_scalar_scale);
        let scale = self.insert_scalar_negation_scale_referent(
            &scalar_negation.introduced_by,
            word,
            definition,
            source,
        )?;
        Ok(scalar_negation.with_scale(scale))
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn set_scalar_negation(
        &mut self,
        predication: SemanticObjectId,
        scalar_negation: ScalarNegation,
    ) -> Result<(), SemanticsError> {
        let Some((modal_arguments, source)) = self
            .objects
            .get(&predication)
            .map(|object| (object.modal_arguments.clone(), object.source.clone()))
        else {
            return Ok(());
        };
        let scalar_negation = self.scalar_negation_with_scale_for_modal_arguments(
            scalar_negation,
            &modal_arguments,
            source,
        )?;
        if let Some(object) = self.objects.get_mut(&predication) {
            object.scalar_negation = Some(scalar_negation);
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn apply_scalar_negation_to_tanru_links(
        &mut self,
        formula: SemanticObjectId,
        scalar_negation: ScalarNegation,
    ) -> Result<bool, SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(false);
        };
        match object.operator.as_ref().map(|operator| operator.as_data()) {
            Some(data!(SemanticOperator::Formula(FormulaOperator::Atom))) => {
                let Some(predication) = object.predication else {
                    return Ok(false);
                };
                if self
                    .objects
                    .get(&predication)
                    .is_some_and(|object| object.tanru_link.is_some())
                {
                    self.set_scalar_negation(predication, scalar_negation)?;
                    return Ok(true);
                }
                Ok(false)
            }
            Some(data!(SemanticOperator::Formula(_))) => {
                let mut changed = false;
                for child in object.children {
                    changed |=
                        self.apply_scalar_negation_to_tanru_links(child, scalar_negation.clone())?;
                }
                Ok(changed)
            }
            Some(data!(SemanticOperator::Math(_))) | None => Ok(false),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
    fn tanru_relation_formula_without_positive_head(
        &self,
        formula: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&formula)?;
        if !matches!(
            object.operator.as_ref()?.as_data(),
            data!(SemanticOperator::Formula(FormulaOperator::And))
        ) || object.children.len() != 2
        {
            return None;
        }
        let head_formula = object.children[0];
        let relation_formula = object.children[1];
        self.formula_is_tanru_relation_for_head(relation_formula, head_formula)
            .then_some(relation_formula)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
    fn detach_tanru_relation_formula_without_positive_head(
        &mut self,
        formula: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&formula)?;
        if !matches!(
            object.operator.as_ref()?.as_data(),
            data!(SemanticOperator::Formula(FormulaOperator::And))
        ) || object.children.len() != 2
        {
            return None;
        }
        let head_formula = object.children[0];
        let relation_formula = object.children[1];
        if !self.formula_is_tanru_relation_for_head(relation_formula, head_formula) {
            return None;
        }
        self.objects.remove(&formula);
        self.objects.remove(&head_formula);
        Some(relation_formula)
    }

    #[requires(relation_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(head_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn formula_is_tanru_relation_for_head(
        &self,
        relation_formula: SemanticObjectId,
        head_formula: SemanticObjectId,
    ) -> bool {
        let Some(relation) = self.objects.get(&relation_formula) else {
            return false;
        };
        if !matches!(
            relation
                .operator
                .as_ref()
                .map(|operator| operator.as_data()),
            Some(data!(SemanticOperator::Formula(FormulaOperator::Atom)))
        ) {
            return false;
        }
        let Some(relation_predication) = relation.predication else {
            return false;
        };
        let Some(head) = self.objects.get(&head_formula) else {
            return false;
        };
        if !matches!(
            head.operator.as_ref().map(|operator| operator.as_data()),
            Some(data!(SemanticOperator::Formula(FormulaOperator::Atom)))
        ) {
            return false;
        }
        let Some(head_predication) = head.predication else {
            return false;
        };
        self.objects
            .get(&relation_predication)
            .and_then(|predication| predication.tanru_link.as_ref())
            .is_some_and(|tanru_link| tanru_link.head == head_predication)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_term_referent(
        &mut self,
        term: &TermSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let argument = self.build_argument_for_generated_term(term)?.into_data();
        let referent = argument
            .value
            .ok_or_else(|| unsupported("non-referential term argument"))?;
        if !argument.relative_clauses.is_empty() {
            let object = self.objects.get_mut(&referent).ok_or_else(|| {
                invalid_graph(format!(
                    "semantic builder could not find generated term referent {referent}"
                ))
            })?;
            object.extend_relative_clauses(argument.relative_clauses);
        }
        Ok(referent)
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_generated_term_assignment<'syntax>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        modal_terms: &mut Vec<TaggedSumtiTermSyntax>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        next_visible_place: &mut usize,
        term: &'syntax TermSyntax,
    ) -> Result<(), SemanticsError> {
        let simple = match term {
            TermSyntax::SimpleTerm(simple) => simple,
            TermSyntax::ConnectedTerm(ConnectedTermSyntax {
                leading_term,
                continuations,
            }) if continuations.is_empty() => leading_term.as_ref(),
            _ => return Err(unsupported("non-simple term")),
        };
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                insert_visible_argument(
                    visible_arguments,
                    *next_visible_place,
                    self.build_argument_for_generated_sumti_with_formula_scopes(
                        sumti,
                        formula_scopes,
                    )?,
                )?;
                *next_visible_place += 1;
                Ok(())
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                let place = fa_place(&term.fa.value)?;
                insert_visible_argument(
                    visible_arguments,
                    place,
                    self.build_tagged_or_elided_sumti_argument(&term.sumti)?,
                )?;
                *next_visible_place = (*next_visible_place).max(place + 1);
                Ok(())
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                modal_terms.push(term.clone());
                Ok(())
            }
            _ => Err(unsupported("non-sumti term")),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_modal_argument_for_generated_tagged_sumti(
        &mut self,
        term: &TaggedSumtiTermSyntax,
    ) -> Result<ModalArgument, SemanticsError> {
        let tense_modal = term.tense_modal.as_ref();
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("tagged sumti tense modal"));
        };
        let argument = self.build_tagged_or_elided_sumti_argument(&term.sumti)?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
        Ok(ModalArgument::new_with_polarity(
            relation,
            introduced_by,
            arguments,
            generated_modal_negation_for_tense_modal(tense_modal),
            generated_modal_scalar_negation_for_tense_modal(tense_modal),
            self.source_for_node(tense_modal, "modal-argument"),
        ))
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
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        Ok(arguments)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_generated_modal_terms_to_formula(
        &mut self,
        formula: SemanticObjectId,
        modal_terms: &[TaggedSumtiTermSyntax],
    ) -> Result<(), SemanticsError> {
        for modal_term in modal_terms {
            self.attach_generated_modal_term_to_formula(formula, modal_term)?;
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_generated_modal_term_to_formula(
        &mut self,
        formula: SemanticObjectId,
        modal_term: &TaggedSumtiTermSyntax,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&formula)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing generated formula {formula}")))?;
        if let Some(predication) = object.predication {
            self.attach_generated_modal_term_to_predication(predication, modal_term)?;
        }
        for child in object.children {
            self.attach_generated_modal_term_to_formula(child, modal_term)?;
        }
        if let Some(body) = object.body {
            self.attach_generated_modal_term_to_formula(body, modal_term)?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn attach_generated_modal_term_to_predication(
        &mut self,
        predication: SemanticObjectId,
        modal_term: &TaggedSumtiTermSyntax,
    ) -> Result<(), SemanticsError> {
        let (mode, eventuality) = {
            let object = self.objects.get(&predication).ok_or_else(|| {
                invalid_graph(format!("missing generated predication {predication}"))
            })?;
            (object.mode, object.eventuality)
        };
        if mode != Some(PredicationMode::Asserted) {
            return Ok(());
        }
        let mut modal_argument =
            self.build_modal_argument_for_generated_tagged_sumti(modal_term)?;
        if let Some(eventuality) = eventuality {
            bind_generated_modal_argument_to_host_event(&mut modal_argument, eventuality);
        }
        let object = self
            .objects
            .get_mut(&predication)
            .ok_or_else(|| invalid_graph(format!("missing generated predication {predication}")))?;
        if !object.modal_arguments.contains(&modal_argument) {
            object.modal_arguments.push(modal_argument);
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_modal_arguments_for_generated_tagged_terms(
        &mut self,
        modal_terms: &[TaggedSumtiTermSyntax],
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        modal_terms
            .iter()
            .map(|term| self.build_modal_argument_for_generated_tagged_sumti(term))
            .collect()
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_for_generated_term(
        &mut self,
        term: &TermSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        let mut visible_arguments = BTreeMap::new();
        let mut modal_terms = Vec::new();
        let mut formula_scopes = Vec::new();
        let mut next_visible_place = 1;
        self.insert_generated_term_assignment(
            &mut visible_arguments,
            &mut modal_terms,
            &mut formula_scopes,
            &mut next_visible_place,
            term,
        )?;
        if !modal_terms.is_empty() {
            return Err(unsupported("modal term as referential argument"));
        }
        if !formula_scopes.is_empty() {
            return Err(unsupported("scoped term as referential argument"));
        }
        let Some(argument) = visible_arguments.remove(&1) else {
            return Err(unsupported("non-referential term argument"));
        };
        if !visible_arguments.is_empty() {
            return Err(unsupported("multi-place term as referential argument"));
        }
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_terms_fragment_referent(
        &mut self,
        fragment: &TermsFragmentSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if fragment.vau.is_some() {
            return Err(unsupported("terms fragment VAU"));
        }
        let [term] = fragment.terms.as_slice() else {
            return Err(unsupported("multi-term fragment"));
        };
        self.build_term_referent(term)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_sumti_referent(
        &mut self,
        sumti: &SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti.vuho_attachment.is_some() {
            return Err(unsupported("VUhO attached sumti"));
        }
        self.build_sumti_grouped_referent(&sumti.base_sumti)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_for_generated_sumti(
        &mut self,
        sumti: &SumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        if generated_sumti_is_deleted(sumti) {
            return Ok(ArgumentValue::deleted(
                "zi'o".to_owned(),
                self.source_for_node(sumti, "deleted-place"),
            ));
        }
        let referent = self.build_sumti_referent(sumti)?;
        let mut argument = if generated_sumti_is_elided(sumti) {
            ArgumentValue::elided(
                referent,
                "zo'e".to_owned(),
                self.source_for_node(sumti, "elided-place"),
            )
        } else {
            ArgumentValue::filled(referent, None)
        };
        if generated_sumti_is_command_target(sumti) {
            argument = argument.with_command_target(CommandTarget::new("ko".to_owned()));
        }
        if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
            let relative_clauses =
                self.lower_generated_relative_clause_list(relative_clauses, referent)?;
            if !relative_clauses.is_empty() {
                argument = argument.with_relative_clauses(relative_clauses);
            }
        }
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_for_generated_sumti_with_formula_scopes<'syntax>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    ) -> Result<ArgumentValue, SemanticsError> {
        if generated_sumti_is_deleted(sumti) {
            return Ok(ArgumentValue::deleted(
                "zi'o".to_owned(),
                self.source_for_node(sumti, "deleted-place"),
            ));
        }
        let scope_source =
            if let Some(quantified_sumti) = generated_quantified_sumti_from_sumti(sumti) {
                Some(GeneratedArgumentQuantifierSource::QuantifiedSumti(
                    quantified_sumti,
                ))
            } else if let Some(description) = outer_quantified_description_from_sumti(sumti) {
                Some(GeneratedArgumentQuantifierSource::OuterQuantifiedDescription(description))
            } else {
                no_gadri_description_from_sumti(sumti)?
                    .map(GeneratedArgumentQuantifierSource::NoGadriDescription)
            };
        let Some(scope_source) = scope_source else {
            return self.build_argument_for_generated_sumti(sumti);
        };
        let referent = self.build_scoped_argument_variable_for_generated_sumti(sumti)?;
        let mut argument = if generated_sumti_is_elided(sumti) {
            ArgumentValue::elided(
                referent,
                "zo'e".to_owned(),
                self.source_for_node(sumti, "elided-place"),
            )
        } else {
            ArgumentValue::filled(referent, None)
        };
        if generated_sumti_is_command_target(sumti) {
            argument = argument.with_command_target(CommandTarget::new("ko".to_owned()));
        }
        if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
            let relative_clauses =
                self.lower_generated_relative_clause_list(relative_clauses, referent)?;
            if !relative_clauses.is_empty() {
                argument = argument.with_relative_clauses(relative_clauses);
            }
        }
        formula_scopes.push(GeneratedArgumentQuantifierScope {
            sumti,
            source: scope_source,
            variable: referent,
        });
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_scoped_argument_variable_for_generated_sumti(
        &mut self,
        sumti: &SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(key) = self.source_key_for_node(sumti)
            && let Some(id) = self.scoped_argument_variables.get(&key)
        {
            return Ok(*id);
        }
        let sort = generated_sumti_quantified_variable_sort(sumti);
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Variable,
                sort,
                None,
                None,
                None,
                self.source_for_node(sumti, "bound-argument"),
                Vec::new(),
            ),
        )?;
        if let Some(key) = self.source_key_for_node(sumti) {
            self.scoped_argument_variables.insert(key, id);
        }
        Ok(id)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_relative_clause_list(
        &mut self,
        relative_clauses: &RelativeClauseListSyntax,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError> {
        let mut lowered = Vec::new();
        if let Some(clause) =
            self.lower_generated_relative_clause_atom(&relative_clauses.first, head)?
        {
            lowered.push(clause);
        }
        for tail in &relative_clauses.additional {
            let atom = match tail {
                RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => tail.inner.as_ref(),
                RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => tail.inner.as_ref(),
            };
            if let Some(clause) = self.lower_generated_relative_clause_atom(atom, head)? {
                lowered.push(clause);
            }
        }
        Ok(lowered)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_relative_clause_atom(
        &mut self,
        clause: &RelativeClauseAtomSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        match clause {
            RelativeClauseAtomSyntax::BridiRelativeClause(clause) => self
                .lower_generated_bridi_relative_clause(clause, head)
                .map(Some),
            RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) => {
                self.lower_generated_sumti_association_relative_clause(clause, head)
            }
        }
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_descriptor_relative_clause_list(
        &mut self,
        relative_clauses: &RelativeClauseListSyntax,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError> {
        let mut lowered = Vec::new();
        if let RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) =
            &relative_clauses.first
            && let Some(clause) =
                self.lower_generated_sumti_association_relative_clause(clause, head)?
        {
            lowered.push(clause);
        }
        for tail in &relative_clauses.additional {
            let atom = match tail {
                RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => tail.inner.as_ref(),
                RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => tail.inner.as_ref(),
            };
            if let RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) = atom
                && let Some(clause) =
                    self.lower_generated_sumti_association_relative_clause(clause, head)?
            {
                lowered.push(clause);
            }
        }
        Ok(lowered)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_sumti_association_relative_clause(
        &mut self,
        clause: &SumtiAssociationRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        let marker_text = token_text(&clause.association_marker.value);
        if clause.association_marker.value.cmavo() == Some(Cmavo::Goi) {
            return Ok(None);
        }
        let source = self.source_for_node(clause, "relative-phrase");
        let marker = clause.association_marker.value.cmavo();
        let kind = marker
            .and_then(relative_phrase_kind_for_marker)
            .unwrap_or(RelativeClauseKind::Restrictive);
        let mode = predication_mode_for_relative_clause_kind(kind);
        if let RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) = clause.sumti.as_ref()
            && let Some(clause) = self.build_generated_modal_sumti_association_clause(
                sumti,
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
        if matches!(
            clause.sumti.as_ref(),
            RelativeSumtiSyntax::TenseTaggedRelativeSumti(_)
        ) {
            diagnostics.push(diagnostic(
                "modal relative phrase source relation is not semantically lowered yet",
            ));
        }
        let associated_argument =
            self.build_argument_for_generated_relative_sumti(&clause.sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(head, None));
        arguments.insert("x2".to_owned(), associated_argument);
        let predication = self.next_predication_id();
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
        let formula = self.next_formula_id();
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
    fn build_generated_modal_sumti_association_clause(
        &mut self,
        sumti: &TenseTaggedRelativeSumtiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
        marker_text: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(sumti.tense_modal.as_ref())
        else {
            return Ok(None);
        };
        let Some(head_place) = modal_relative_phrase_head_place(&relation, visible_place) else {
            return Ok(None);
        };
        let mode = predication_mode_for_relative_clause_kind(kind);
        let associated_argument = self.build_tagged_or_elided_sumti_argument(&sumti.sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(format!("x{head_place}"), ArgumentValue::filled(head, None));
        arguments.insert(format!("x{visible_place}"), associated_argument);
        let mut diagnostics = Vec::new();
        match relation_place_count(self.dictionary, &relation) {
            Some(place_count) => {
                for place in 1..=place_count.max(head_place).max(visible_place) {
                    let key = argument_key(place);
                    if !arguments.contains_key(&key) {
                        arguments.insert(key, self.build_elided_argument_for_place(place)?);
                    }
                }
            }
            None => {
                for place in 1..=head_place.max(visible_place) {
                    let key = argument_key(place);
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
        let predication = self.next_predication_id();
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
        let formula = self.next_formula_id();
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
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some()) || ret.is_err())]
    fn build_argument_for_generated_relative_sumti(
        &mut self,
        sumti: &RelativeSumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        match sumti {
            RelativeSumtiSyntax::PlainRelativeSumti(PlainRelativeSumtiSyntax(sumti)) => {
                self.build_argument_for_generated_sumti(sumti)
            }
            RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
                self.build_tagged_or_elided_sumti_argument(&sumti.sumti)
            }
            RelativeSumtiSyntax::NaKuRelativeSumti(_) => {
                Err(unsupported("negative relative phrase sumti"))
            }
        }
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_bridi_relative_clause(
        &mut self,
        clause: &BridiRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<RelativeClause, SemanticsError> {
        match clause {
            BridiRelativeClauseSyntax::RestrictiveBridiRelativeClause(clause) => {
                self.lower_generated_restrictive_bridi_relative_clause(clause, head)
            }
            BridiRelativeClauseSyntax::IncidentalBridiRelativeClause(clause) => self
                .lower_generated_relative_subbridi(
                    clause.subbridi.as_ref(),
                    head,
                    RelativeClauseKind::Incidental,
                ),
        }
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_restrictive_bridi_relative_clause(
        &mut self,
        clause: &RestrictiveBridiRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<RelativeClause, SemanticsError> {
        if clause
            .poi
            .value
            .cmavo()
            .is_some_and(cmavo_is_nonveridical_relative_marker)
        {
            return Err(unsupported("nonveridical relative clause"));
        }
        self.lower_generated_relative_subbridi(
            clause.subbridi.as_ref(),
            head,
            RelativeClauseKind::Restrictive,
        )
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn lower_generated_relative_subbridi(
        &mut self,
        subbridi: &SubbridiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
    ) -> Result<RelativeClause, SemanticsError> {
        let mode = predication_mode_for_relative_clause_kind(kind);
        let contains_keha = generated_subbridi_contains_cmavo(subbridi, Cmavo::Keha);
        let previous_relative_head = self.relative_head;
        self.relative_head = Some(head);
        let result = self.build_generated_subbridi_formula(subbridi, mode);
        self.relative_head = previous_relative_head;
        let formula = result?;
        if !contains_keha {
            self.fill_first_elided_generated_formula_argument(formula, head)?;
        }
        self.set_formula_predication_mode(formula, mode);
        Ok(RelativeClause::new(
            kind,
            formula,
            self.source_for_node(subbridi, "relative-clause"),
        ))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_generated_subbridi_formula(
        &mut self,
        subbridi: &SubbridiSyntax,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
                self.build_bridi_formula_with_options(bridi, None, mode)
            }
            SubbridiSyntax::PrenexSubbridi(_) => Err(unsupported("prenex subbridi")),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn fill_first_elided_generated_formula_argument(
        &mut self,
        formula: SemanticObjectId,
        head: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(());
        };
        if let Some(predication) = object.predication {
            self.fill_first_elided_generated_predication_argument(predication, head)?;
        }
        for child in object.children {
            self.fill_first_elided_generated_formula_argument(child, head)?;
        }
        if let Some(restriction) = object.restriction {
            self.fill_first_elided_generated_formula_argument(restriction, head)?;
        }
        if let Some(body) = object.body {
            self.fill_first_elided_generated_formula_argument(body, head)?;
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn replace_first_elided_generated_formula_argument(
        &mut self,
        formula: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(false);
        };
        if let Some(predication) = object.predication
            && self.replace_first_elided_generated_predication_argument(
                predication,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        for child in object.children {
            if self.replace_first_elided_generated_formula_argument(
                child,
                parameter,
                preferred_selbri,
            )? {
                return Ok(true);
            }
        }
        if let Some(restriction) = object.restriction
            && self.replace_first_elided_generated_formula_argument(
                restriction,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        if let Some(body) = object.body
            && self.replace_first_elided_generated_formula_argument(
                body,
                parameter,
                preferred_selbri,
            )?
        {
            return Ok(true);
        }
        Ok(false)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn replace_first_elided_generated_predication_argument(
        &mut self,
        predication: SemanticObjectId,
        parameter: SemanticObjectId,
        preferred_selbri: Option<&SelbriSyntax>,
    ) -> Result<bool, SemanticsError> {
        let object = self.objects.get(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction predication {predication}"
            ))
        })?;
        let mut selected_place: Option<(usize, usize, String)> = None;
        for (place, argument) in &object.arguments {
            if argument.kind != ArgumentValueKind::Elided {
                continue;
            }
            let Some(index) = argument_place_index(place) else {
                continue;
            };
            let visible_rank = preferred_selbri
                .map(|selbri| generated_raw_place_visible_rank_for_selbri(selbri, index))
                .transpose()?
                .unwrap_or(index);
            if selected_place
                .as_ref()
                .is_none_or(|(best_visible, best_index, _)| {
                    (visible_rank, index) < (*best_visible, *best_index)
                })
            {
                selected_place = Some((visible_rank, index, place.clone()));
            }
        }
        let Some((_visible_rank, _index, place)) = selected_place else {
            return Ok(false);
        };
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction predication {predication}"
            ))
        })?;
        if let Some(argument) = object.arguments.get_mut(&place) {
            let source = argument.source.clone();
            *argument = ArgumentValue::filled(parameter, source);
        }
        Ok(true)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn fill_first_elided_generated_predication_argument(
        &mut self,
        predication: SemanticObjectId,
        head: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find relative-clause predication {predication}"
            ))
        })?;
        let Some(place) = object
            .arguments
            .iter()
            .filter(|(_place, argument)| argument.kind == ArgumentValueKind::Elided)
            .filter_map(|(place, _argument)| {
                argument_place_index(place).map(|index| (index, place))
            })
            .min_by_key(|(index, _place)| *index)
            .map(|(_index, place)| place.clone())
        else {
            return Ok(());
        };
        if let Some(argument) = object.arguments.get_mut(&place) {
            let source = argument.source.clone();
            *argument = ArgumentValue::filled(head, source);
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_sumti_grouped_referent(
        &mut self,
        sumti: &SumtiGroupedSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti.grouped_tail.is_some() {
            return Err(unsupported("grouped sumti"));
        }
        self.build_sumti_afterthought_referent(&sumti.leading_sumti)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_sumti_afterthought_referent(
        &mut self,
        sumti: &SumtiAfterthoughtSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading = self.build_sumti_bound_referent(&sumti.leading_sumti)?;
        let [] = sumti.continuations.as_slice() else {
            let [continuation] = sumti.continuations.as_slice() else {
                return Err(unsupported("multi-continuation afterthought sumti"));
            };
            let trailing = self.build_sumti_bound_referent(&continuation.sumti)?;
            return self.build_connected_generated_sumti_referent(
                sumti,
                leading,
                &continuation.connective,
                trailing,
            );
        };
        Ok(leading)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_sumti_bound_referent(
        &mut self,
        sumti: &SumtiBoundSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti.bound_tail.is_some() {
            return Err(unsupported("bound sumti"));
        }
        self.build_sumti_forethought_referent(&sumti.leading_sumti)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_sumti_forethought_referent(
        &mut self,
        sumti: &SumtiForethoughtSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti {
            SumtiForethoughtSyntax::SimpleSumti(simple) => self.build_simple_sumti_referent(simple),
            SumtiForethoughtSyntax::ForethoughtSumti(_) => Err(unsupported("forethought sumti")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_simple_sumti_referent(
        &mut self,
        sumti: &SimpleSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti.base_sumti.as_ref() {
            SumtiAtomSyntax::SumtiBase(base) => self.build_sumti_base_referent(base),
            SumtiAtomSyntax::QuantifiedSumti(_) => Err(unsupported("quantified sumti")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_sumti_base_referent(
        &mut self,
        sumti: &SumtiBaseSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti {
            SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
                self.build_scalar_negated_generated_sumti_with_bo_referent(sumti)
            }
            SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
                self.build_scalar_negated_generated_sumti_referent(sumti)
            }
            SumtiBaseSyntax::ProSumti(pro_sumti) => self.build_pro_sumti_referent(pro_sumti),
            SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
                self.build_description_referent(description)
            }
            SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(description) => {
                self.build_outer_quantified_description_referent(description)
            }
            SumtiBaseSyntax::DescriptorWithoutGadriSumti(description) => {
                self.build_no_gadri_description_referent(description)
            }
            SumtiBaseSyntax::NameSumti(name) => self.build_name_sumti_referent(name),
            SumtiBaseSyntax::NumberSumti(number) => self.build_number_sumti_referent(number),
            SumtiBaseSyntax::LerfuStringSumti(sumti) => {
                self.build_lerfu_string_sumti_referent(sumti)
            }
            SumtiBaseSyntax::LaheSumti(sumti) => self.build_lahe_sumti_referent(sumti),
            SumtiBaseSyntax::QuotedSumti(sumti) => self.build_quoted_sumti_sign(sumti),
            _ => Err(unsupported("sumti base")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_lerfu_string_sumti_referent(
        &mut self,
        sumti: &LerfuStringSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_diagnostic_sumti_referent(
            sumti,
            "letteral pro-sumti did not resolve to an antecedent",
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    fn build_quoted_sumti_sign(
        &mut self,
        sumti: &QuotedSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(sumti, "quotation");
        let source_text = source.as_ref().and_then(|source| source.text.clone());
        let quotation = match sumti.0.as_ref() {
            QuoteSyntax::TextQuote(quote) => Quotation {
                mode: "parsed".to_owned(),
                utterance: self.build_generated_quoted_text_group(&quote.text, source.clone())?,
                delimiter: None,
                text: source_text,
            },
            _ => {
                let delimiter = self.tokens_for_node(sumti).first().map(token_text);
                Quotation {
                    mode: "opaque".to_owned(),
                    utterance: None,
                    delimiter,
                    text: source_text,
                }
            }
        };
        let id = self.next_sign_id();
        self.insert(
            id,
            SemanticObject::sign(SignKind::Quotation, Some(quotation), source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence)) || ret.is_err())]
    fn build_generated_quoted_text_group(
        &mut self,
        text: &TextSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let roots = semantic_roots_from_text(text)?;
        if roots.is_empty() {
            return Ok(None);
        }
        let previous_roles = self.current_deictic_roles();
        let previous_current_utterance = self.current_utterance;
        let previous_previous_utterance = self.previous_utterance;
        let previous_next_utterance = self.next_utterance;
        let quote_roles = self.build_fresh_quote_deictic_roles(source)?;
        self.set_current_deictic_roles(quote_roles);
        self.current_utterance = None;
        self.previous_utterance = None;
        self.next_utterance = None;
        let result = if let [root] = roots.as_slice() {
            let utterance_id = self.next_utterance_id();
            self.current_utterance = Some(utterance_id);
            self.build_utterance_for_generated_text_root(utterance_id, *root)
        } else {
            let mut items = Vec::with_capacity(roots.len());
            for root in roots {
                items.push(self.build_discourse_item_for_generated_text_root(root)?);
            }
            let sequence = self.next_sequence_id();
            self.insert(
                sequence,
                SemanticObject::sequence(
                    items,
                    SequenceRelation::SameTopicContinuation,
                    None,
                    Vec::new(),
                ),
            )?;
            Ok(sequence)
        };
        self.set_current_deictic_roles(previous_roles);
        self.current_utterance = previous_current_utterance;
        self.previous_utterance = previous_previous_utterance;
        self.next_utterance = previous_next_utterance;
        result.map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_scalar_negated_generated_sumti_with_bo_referent(
        &mut self,
        sumti: &ScalarNegatedSumtiWithBoSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_scalar_negated_generated_sumti_referent_with_marker(
            sumti,
            sumti.nahe.cmavo(),
            format!("{} bo", token_text(&sumti.nahe)),
            &sumti.inner_sumti,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    fn build_scalar_negated_generated_sumti_referent(
        &mut self,
        sumti: &ScalarNegatedSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_scalar_negated_generated_sumti_referent_with_marker(
            sumti,
            sumti.nahe.value.cmavo(),
            token_text(&sumti.nahe.value),
            &sumti.inner_sumti,
        )
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_scalar_negated_generated_sumti_referent_with_marker<N: TreeNode>(
        &mut self,
        node: &N,
        cmavo: Option<Cmavo>,
        word: String,
        inner_sumti: &SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operand = self.build_sumti_referent(inner_sumti)?;
        let sort = self
            .objects
            .get(&operand)
            .and_then(|object| object.sort)
            .unwrap_or(SemanticSort::Entity);
        let scale = self.build_generated_scalar_negation_scale_referent(node, &word)?;
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind: scalar_negated_sumti_qualifier_kind(cmavo).to_owned(),
                    word,
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: Some(scale),
                    definiteness: descriptor_definiteness_for_scalar_negated_sumti(cmavo),
                    operand: Some(operand),
                }),
                None,
                self.source_for_node(node, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_generated_scalar_negation_scale_referent<N: TreeNode>(
        &mut self,
        node: &N,
        introduced_by: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.insert_scalar_negation_scale_referent(
            introduced_by,
            "implicit scalar scale",
            None,
            self.source_for_node(node, "scalar-scale"),
        )
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_generated_diagnostic_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
        message: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "unloweredSumti".to_owned(),
                    word: "sumti".to_owned(),
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                self.source_for_node(node, "sumti"),
                vec![diagnostic(message)],
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    fn build_number_sumti_referent(
        &mut self,
        number: &NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if number.li.value.cmavo() == Some(Cmavo::Meho) {
            return Err(unsupported("MEhO number sumti"));
        }
        let words = self.tokens_for_node(number.expression.as_ref());
        let Some(value) = simple_pa_integer_from_tokens(&words) else {
            return Err(unsupported("complex number sumti"));
        };
        let text = token_list_text(words.iter());
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                QuantityValue::integer(value),
                QuantityScale::Count,
                self.source_for_node(number, "quantity"),
            ),
        )?;
        let id = self.next_referent_with_sort_id(SemanticSort::Number);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Number,
                None,
                Some(Descriptor {
                    kind: "number".to_owned(),
                    word: token_text(&number.li.value),
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: Some(quantity),
                    name: Some(text),
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                self.source_for_node(number, "number-sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_lahe_sumti_referent(
        &mut self,
        sumti: &LaheSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti.relative_clauses.is_some() || sumti.luhu.is_some() {
            return Err(unsupported("qualified sumti modifiers"));
        }
        let operand = self.build_sumti_referent(&sumti.inner_sumti)?;
        let sort = referent_qualifier_sort(sumti.lahe.value.cmavo());
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind: referent_qualifier_kind(sumti.lahe.value.cmavo()).to_owned(),
                    word: token_text(&sumti.lahe.value),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: Some(operand),
                }),
                None,
                self.source_for_node(sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_no_gadri_description_referent(
        &mut self,
        description: &DescriptorWithoutGadriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if description.relative_clauses.is_some() {
            return Err(unsupported("description relative clauses"));
        }
        let id = self.next_referent_id();
        let quantity = self.build_quantity_for_quantifier(&description.quantifier)?;
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "description".to_owned(),
                    word: String::new(),
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: Some(quantity),
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                self.source_for_node(description, "description"),
                Vec::new(),
            ),
        )?;
        let body = self.build_restrictive_formula(&description.selbri, id)?;
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find no-gadri description referent {id}"
            ))
        })?;
        let Some(descriptor) = object.descriptor.as_mut() else {
            return Err(invalid_graph(format!(
                "semantic builder no-gadri description referent {id} has no descriptor"
            )));
        };
        descriptor.body = Some(body);
        Ok(id)
    }

    #[requires(source.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(trailing.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_connected_generated_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
        source: SemanticObjectId,
        connective: &ArgumentConnectiveSyntax,
        trailing: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_argument_connective_operator(connective)?;
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Composite,
                SemanticSort::Entity,
                None,
                None,
                Some(new!(Composition {
                    operator,
                    operator_parameter: None,
                    members: vec![source, trailing],
                    excluded_members: Vec::new(),
                    collective: None,
                    scalar_negated: None,
                    complement: None,
                    endpoint_inclusion: None,
                })),
                self.source_for_node(node, "connected-sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_pro_sumti_referent(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let word = token_text(&pro_sumti.0.value);
        match pro_sumti.0.value.cmavo() {
            Some(Cmavo::Mi) => Ok(self.current_speaker()),
            Some(Cmavo::Do) => Ok(self.current_audience()),
            Some(Cmavo::Ko) => Ok(self.current_audience()),
            Some(Cmavo::Ma) => self.build_argument_question_parameter(pro_sumti),
            Some(Cmavo::Cehu) => {
                self.build_generated_parameter(pro_sumti, ParameterRole::PropertySlot)
            }
            Some(Cmavo::Zohe) => self.build_elided_referent_with_source(
                "zo'e".to_owned(),
                self.source_for_node(pro_sumti, "elided-sumti"),
            ),
            Some(Cmavo::Zuhi) => self.build_typical_place_value_referent(pro_sumti),
            Some(Cmavo::Keha) => self
                .relative_head
                .ok_or_else(|| unsupported("relative head pro-sumti outside relative clause")),
            Some(
                Cmavo::Dei
                | Cmavo::Dihu
                | Cmavo::Dehu
                | Cmavo::Dahu
                | Cmavo::Dihe
                | Cmavo::Dehe
                | Cmavo::Dahe
                | Cmavo::Dohi,
            ) => self.build_utterance_reference_referent(pro_sumti),
            Some(Cmavo::Ti) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::ProximalDemonstrative)
            }
            Some(Cmavo::Ta) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::MedialDemonstrative)
            }
            Some(Cmavo::Tu) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::DistalDemonstrative)
            }
            Some(Cmavo::Da | Cmavo::De | Cmavo::Di) => {
                self.build_implicit_existential_variable(pro_sumti)
            }
            _ => Err(unsupported(&format!("pro-sumti {word}"))),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_argument_question_parameter(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter =
            self.build_generated_parameter(pro_sumti, ParameterRole::ArgumentQuestion)?;
        self.argument_question_parameters.push(parameter);
        Ok(parameter)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    fn build_generated_parameter(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
        role: ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                role,
                token_text(&pro_sumti.0.value),
                self.source_for_node(pro_sumti, "parameter"),
            ),
        )?;
        if role == ParameterRole::PropertySlot
            && pro_sumti.0.value.cmavo() == Some(Cmavo::Cehu)
            && let Some(parameters) = self.abstraction_parameter_stack.last_mut()
        {
            parameters.push(parameter);
        }
        Ok(parameter)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_implicit_existential_variable(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(pro_sumti, "sumti");
        let variable = self.next_referent_id();
        self.insert(
            variable,
            SemanticObject::referent(
                ReferentCategory::Variable,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "proSumti".to_owned(),
                    word: token_text(&pro_sumti.0.value),
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                source.clone(),
                Vec::new(),
            ),
        )?;
        self.implicit_existential_variables
            .push(new!(GeneratedImplicitExistential {
                variable,
                source: self.source_for_node(pro_sumti, "quantifier-scope"),
            }));
        Ok(variable)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_typical_place_value_referent(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(Descriptor {
                    kind: "typicalPlaceValue".to_owned(),
                    word: token_text(&pro_sumti.0.value),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                self.source_for_node(pro_sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_utterance_reference_referent(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let token = &pro_sumti.0.value;
        let word = token_text(token);
        let target = match token.cmavo() {
            Some(Cmavo::Dei) => self.current_utterance,
            Some(Cmavo::Dihu) => self.previous_utterance,
            Some(Cmavo::Dihe) => self.next_utterance,
            Some(Cmavo::Dohi) => None,
            Some(Cmavo::Dehu | Cmavo::Dahu | Cmavo::Dehe | Cmavo::Dahe) => None,
            _ => return Err(unsupported(&format!("utterance pro-sumti {word}"))),
        };
        let mut diagnostics = Vec::new();
        if target.is_none() && token.cmavo() != Some(Cmavo::Dohi) {
            diagnostics.push(diagnostic(
                "utterance pro-sumti did not resolve to a concrete discourse item",
            ));
        }
        let id = self.next_referent_with_sort_id(SemanticSort::Sign);
        let mut object = SemanticObject::referent(
            ReferentCategory::Constant,
            SemanticSort::Sign,
            None,
            Some(Descriptor {
                kind: "utteranceReference".to_owned(),
                word,
                speaker: Some(self.current_speaker()),
                body: None,
                veridical: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: None,
                scale: None,
                definiteness: None,
                operand: None,
            }),
            None,
            self.source_for_node(pro_sumti, "sumti"),
            diagnostics,
        );
        object.target = target;
        self.insert(id, object)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_demonstrative_referent(
        &mut self,
        pro_sumti: &ProSumtiSyntax,
        indexical: IndexicalKind,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Indexical,
                SemanticSort::Entity,
                Some(indexical),
                None,
                None,
                self.source_for_node(pro_sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_name_sumti_referent(
        &mut self,
        name: &NameSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sort = gadri_name_sort(name.la.value.cmavo());
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind: name_description_kind_for_cmavo(name.la.value.cmavo()).to_owned(),
                    word: token_text(&name.la.value),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: Some(token_list_text(name.names.value.iter())),
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                self.source_for_name_sumti(name, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_description_referent(
        &mut self,
        description: &DescriptorWithGadriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_gadri_description_referent(&description.description, &description.tail)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_outer_quantified_description_referent(
        &mut self,
        description: &DescriptorWithOuterQuantifierSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_gadri_description_referent(&description.description, &description.tail)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_gadri_description_referent(
        &mut self,
        description_head: &DescriptionHeadSyntax,
        tail: &DescriptionTailSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (selbri, relative_clauses, quantity, body_operand_sumti) = match tail.tail.as_ref() {
            DescriptionTailBodySyntax::RelationDescriptionTail(RelationDescriptionTailSyntax {
                selbri,
                relative_clauses,
            }) => (Some(selbri.as_ref()), relative_clauses.as_ref(), None, None),
            DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(
                QuantifierRelationDescriptionTailSyntax {
                    quantifier,
                    selbri,
                    relative_clauses,
                },
            ) => (
                Some(selbri.as_ref()),
                relative_clauses.as_ref(),
                Some(quantifier),
                None,
            ),
            DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(
                QuantifierSumtiDescriptionTailSyntax { quantifier, sumti },
            ) => (None, None, Some(quantifier), Some(sumti.as_ref())),
        };
        let leading_tail_elements = &tail.leading_tail_elements;
        let leading_operand_sumti = leading_tail_elements
            .tail_sumti
            .as_ref()
            .map(|tail_sumti| tail_sumti.0.as_ref());
        if leading_operand_sumti.is_some() && body_operand_sumti.is_some() {
            return Err(unsupported("multiple description operands"));
        }
        let cmavo = description_head.0.value.cmavo();
        let word = token_text(&description_head.0.value);
        let kind = description_kind_for_cmavo(cmavo).to_owned();
        let abstraction = selbri
            .map(Self::generated_description_abstraction_for_selbri)
            .transpose()?
            .flatten();
        let description_source =
            self.source_for_gadri_description(description_head, tail, "description");
        if let Some(abstraction) = abstraction
            && abstraction.link_relation
                == abstraction_link_relation(abstraction_kind_for_nu(abstraction.abstraction))
        {
            return self.build_abstraction_description_output(
                description_source,
                cmavo,
                abstraction.abstraction,
                kind,
                word,
            );
        }
        let sort = abstraction
            .map(|abstraction| abstraction.output_sort)
            .unwrap_or(SemanticSort::Entity);
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(Descriptor {
                    kind,
                    word,
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: cmavo
                        .is_some_and(|cmavo| matches!(cmavo, Cmavo::Lohe | Cmavo::Lehe))
                        .then_some(false),
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                description_source.clone(),
                Vec::new(),
            ),
        )?;
        let body = selbri
            .map(|selbri| {
                if let Some(abstraction) = abstraction {
                    return self.build_generated_abstraction_description_formula(
                        selbri,
                        id,
                        abstraction,
                    );
                }
                match description_characterization_for_cmavo(cmavo) {
                    DescriptionCharacterization::SpeakerDescribed => {
                        let source = self.source_for_gadri_description(
                            description_head,
                            tail,
                            "speaker-description",
                        );
                        self.build_speaker_description_formula(source, selbri, id)
                    }
                    DescriptionCharacterization::Veridical => {
                        self.build_restrictive_formula(selbri, id)
                    }
                }
            })
            .transpose()?;
        let mut descriptor_operand = None;
        let mut lowered_relative_clauses = Vec::new();
        if leading_operand_sumti.is_none()
            && let Some(relative_clauses) = &leading_tail_elements.relative_clauses
        {
            lowered_relative_clauses.extend(
                self.lower_generated_descriptor_relative_clause_list(relative_clauses, id)?,
            );
        }
        lowered_relative_clauses.extend(
            relative_clauses
                .map(|relative_clauses| {
                    self.lower_generated_relative_clause_list(relative_clauses, id)
                })
                .transpose()?
                .unwrap_or_default(),
        );
        if let Some(operand_sumti) = leading_operand_sumti {
            let operand = self.build_sumti_base_referent(operand_sumti)?;
            let operand_relative_clauses = leading_tail_elements
                .relative_clauses
                .as_ref()
                .map(|relative_clauses| {
                    self.lower_generated_relative_clause_list(relative_clauses, operand)
                })
                .transpose()?
                .unwrap_or_default();
            if selbri.is_some() {
                lowered_relative_clauses.push(self.build_generated_possessive_association_clause(
                    id,
                    operand,
                    operand_sumti,
                    operand_relative_clauses,
                )?);
            } else {
                descriptor_operand = Some(operand);
                lowered_relative_clauses.extend(operand_relative_clauses);
            }
        }
        if let Some(operand_sumti) = body_operand_sumti {
            descriptor_operand = Some(self.build_sumti_referent(operand_sumti)?);
        }
        let quantity = quantity
            .map(|quantifier| self.build_quantity_for_quantifier(quantifier))
            .transpose()?;
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find description referent {id}"
            ))
        })?;
        let Some(descriptor) = object.descriptor.as_mut() else {
            return Err(invalid_graph(format!(
                "semantic builder description referent {id} has no descriptor"
            )));
        };
        descriptor.body = body;
        descriptor.operand = descriptor_operand;
        descriptor.quantity = quantity;
        descriptor.relative_clauses = lowered_relative_clauses;
        Ok(id)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(operand.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_generated_possessive_association_clause<N: TreeNode>(
        &mut self,
        head: SemanticObjectId,
        operand: SemanticObjectId,
        operand_sumti: &N,
        operand_relative_clauses: Vec<RelativeClause>,
    ) -> Result<RelativeClause, SemanticsError> {
        let source = self.source_for_node(operand_sumti, "possessive-sumti");
        let mut associated_argument = ArgumentValue::filled(operand, None);
        if !operand_relative_clauses.is_empty() {
            associated_argument =
                associated_argument.with_relative_clauses(operand_relative_clauses);
        }
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(head, None));
        arguments.insert("x2".to_owned(), associated_argument);
        let predication = self.next_predication_id();
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
        let formula = self.next_formula_id();
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

    #[requires(!kind.is_empty())]
    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_abstraction_description_output(
        &mut self,
        source: Option<crate::model::SemanticSource>,
        cmavo: Option<Cmavo>,
        abstraction: &AbstractionTanruUnitSyntax,
        kind: String,
        word: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_abstraction_output(abstraction, source.clone())?;
        let speaker = self.current_speaker();
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction description output {id}"
            ))
        })?;
        object.descriptor = Some(Descriptor {
            kind,
            word,
            speaker: Some(speaker),
            body: None,
            veridical: cmavo
                .is_some_and(|cmavo| matches!(cmavo, Cmavo::Lohe | Cmavo::Lehe))
                .then_some(false),
            relative_clauses: Vec::new(),
            quantity: None,
            name: None,
            scale: None,
            definiteness: None,
            operand: None,
        });
        object.source = source;
        Ok(id)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(!abstraction.link_relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_generated_abstraction_description_formula(
        &mut self,
        selbri: &SelbriSyntax,
        referent: SemanticObjectId,
        abstraction: GeneratedDescriptionAbstraction<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let output = self.build_abstraction_output(
            abstraction.abstraction,
            self.source_for_node(abstraction.abstraction, "abstraction"),
        )?;
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(referent, None));
        arguments.insert("x2".to_owned(), ArgumentValue::filled(output, None));
        self.build_structural_formula_from_arguments_with_formula_source(
            abstraction.link_relation,
            arguments,
            PredicationMode::Restrictive,
            self.source_for_node(selbri, "abstraction-description"),
            self.source_for_node(selbri, "restrictive-formula"),
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_speaker_description_formula(
        &mut self,
        source: Option<crate::model::SemanticSource>,
        selbri: &SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let property = self.build_description_property_abstraction_for_selbri(selbri)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(
            "x1".to_owned(),
            ArgumentValue::filled(self.current_speaker(), None),
        );
        arguments.insert("x2".to_owned(), ArgumentValue::filled(referent, None));
        arguments.insert(
            "x3".to_owned(),
            ArgumentValue::filled(self.current_audience(), None),
        );
        arguments.insert("x4".to_owned(), ArgumentValue::filled(property, None));
        self.build_structural_formula_from_arguments(
            "skicu",
            arguments,
            PredicationMode::Incidental,
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    fn build_description_property_abstraction_for_selbri(
        &mut self,
        selbri: &SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(selbri, "speaker-description-property");
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let body = if let Some(tanru) = tanru_selbri_from_selbri(selbri)?
            && !tanru.additional_units.is_empty()
        {
            self.build_property_formula_for_tanru_selbri(
                tanru,
                parameter,
                self.source_for_node(selbri, "restrictive-tanru-formula"),
                GeneratedPropertyTanruContext::Description,
            )?
        } else {
            self.build_restrictive_formula(selbri, parameter)?
        };
        let abstraction = self.next_relation_id();
        self.insert(
            abstraction,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                vec![parameter],
                source,
                Vec::new(),
            ),
        )?;
        Ok(abstraction)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent || referent.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_restrictive_formula(
        &mut self,
        selbri: &SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(selbri)? {
            return self.build_sumti_selbri_formula_for_argument(
                sumti_selbri,
                ArgumentValue::filled(referent, None),
                PredicationMode::Restrictive,
                self.source_for_node(selbri, "restrictive-predication"),
            );
        }
        if let Some(tanru) = tanru_selbri_from_selbri(selbri)?
            && tanru.additional_units.is_empty()
        {
            return self.build_relation_formula_for_generated_tanru_unit_argument(
                &tanru.first_unit,
                ArgumentValue::filled(referent, None),
                PredicationMode::Restrictive,
                self.source_for_node(selbri, "restrictive-predication"),
                self.source_for_node(selbri, "restrictive-formula"),
            );
        }
        if matches!(selbri, SelbriSyntax::TaggedSelbri(_)) {
            let SelbriSyntax::TaggedSelbri(tagged) = selbri else {
                unreachable!("previous pattern requires a tagged selbri");
            };
            return self.build_restrictive_formula_for_tagged_selbri(tagged, referent);
        }
        let relation = semantic_relation_label(relation_label_from_selbri(selbri)?);
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(referent, None));
        let place_count = relation_place_count(self.dictionary, &relation).unwrap_or(1);
        for place in 2..=place_count {
            let elided = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(elided, "zo'e".to_owned(), None),
            );
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                None,
                arguments,
                PredicationMode::Restrictive,
                self.source_for_node(selbri, "restrictive-predication"),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.source_for_node(selbri, "restrictive-formula"),
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent || referent.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_restrictive_formula_for_tagged_selbri(
        &mut self,
        tagged: &jbotci_syntax::generated_model::TaggedSelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref()) {
            return Err(unsupported("scoped tagged restrictive selbri"));
        }
        let UntaggedSelbriSyntax::CoSelbri(co_selbri) = tagged.inner_selbri.as_ref() else {
            return Err(unsupported("non-CO tagged restrictive selbri"));
        };
        let relation = semantic_relation_label(relation_label_from_co_selbri(co_selbri)?);
        let place_count = relation_place_count(self.dictionary, &relation).unwrap_or(1);
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), ArgumentValue::filled(referent, None));
        for place in 2..=place_count {
            let key = argument_key(place);
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                key,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let predication_source = self.source_for_node(tagged, "restrictive-predication");
        let eventuality = self.build_generated_tense_eventuality(
            tagged.tense_modal.as_ref(),
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                eventuality,
                arguments,
                PredicationMode::Restrictive,
                predication_source,
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.source_for_node(tagged, "restrictive-formula"),
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    fn build_quantity_for_quantifier(
        &mut self,
        quantifier: &QuantifierSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let words = self.tokens_for_node(quantifier);
        if words.is_empty() {
            return Err(unsupported("empty quantifier"));
        }
        let text = token_list_text(words.iter());
        let value = parse_generated_relational_pa_integer(&text)
            .or_else(|| simple_pa_integer_from_tokens(&words))
            .map(QuantityValue::integer)
            .unwrap_or_else(|| QuantityValue::text(text.clone()));
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                self.source_for_node(quantifier, "quantity"),
            ),
        )?;
        Ok(quantity)
    }

    #[requires(!relation.is_empty())]
    #[requires(arguments.keys().all(|place| place.starts_with('x')))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_structural_formula_from_arguments(
        &mut self,
        relation: &str,
        arguments: BTreeMap<String, ArgumentValue>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_structural_formula_from_arguments_with_formula_source(
            relation,
            arguments,
            mode,
            source.clone(),
            source,
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(arguments.keys().all(|place| place.starts_with('x')))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_structural_formula_from_arguments_with_formula_source(
        &mut self,
        relation: &str,
        mut arguments: BTreeMap<String, ArgumentValue>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let highest_argument = arguments
            .keys()
            .filter_map(|place| place.strip_prefix('x'))
            .filter_map(|place| place.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        let place_count =
            relation_place_count(self.dictionary, relation).unwrap_or(highest_argument);
        for place in 1..=place_count.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                let elided = self.build_elided_referent("zo'e".to_owned())?;
                arguments.insert(key, ArgumentValue::elided(elided, "zo'e".to_owned(), None));
            }
        }
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.to_owned(),
                None,
                arguments,
                mode,
                predication_source,
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(source.as_ref().is_none_or(|source| source.construct.is_some()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_abstraction_link_formula_for_visible_argument(
        &mut self,
        abstraction: &AbstractionTanruUnitSyntax,
        visible_argument: Option<ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let kind = abstraction_kind_for_nu(abstraction);
        let x1 = match visible_argument {
            Some(argument) => argument,
            None => self.build_elided_argument_with_sort(
                "zo'e".to_owned(),
                abstraction_output_sort(kind),
            )?,
        };
        let output = self.build_abstraction_output(
            abstraction,
            self.source_for_node(abstraction, "abstraction"),
        )?;
        let mut arguments = BTreeMap::new();
        arguments.insert("x1".to_owned(), x1);
        arguments.insert("x2".to_owned(), ArgumentValue::filled(output, None));
        self.build_structural_formula_from_arguments(
            abstraction_link_relation(kind),
            arguments,
            mode,
            source,
        )
    }

    #[requires(source.as_ref().is_none_or(|source| source.construct.is_some()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_abstraction_output(
        &mut self,
        abstraction: &AbstractionTanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if abstraction.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        if !abstraction.abstractor_connections.is_empty() {
            return Err(unsupported("connected abstraction"));
        }
        let kind = abstraction_kind_for_nu(abstraction);
        let sort = abstraction_output_sort(kind);
        self.abstraction_parameter_stack.push(Vec::new());
        let body_result = if let Some(class) = abstraction_eventuality_class(kind) {
            let id = self.next_referent_with_sort_id(sort);
            let body = self.build_subbridi_formula_with_eventuality(
                &abstraction.subbridi,
                id,
                abstraction_body_mode(kind),
            );
            body.map(|body| (Some((id, class)), body))
        } else {
            self.build_generated_subbridi_formula(
                &abstraction.subbridi,
                abstraction_body_mode(kind),
            )
            .map(|body| (None, body))
        };
        let (eventuality, body) = match body_result {
            Ok(result) => result,
            Err(error) => {
                let _ = self.abstraction_parameter_stack.pop();
                return Err(error);
            }
        };
        let mut parameters = self
            .abstraction_parameter_stack
            .pop()
            .expect("abstraction parameter stack was just pushed");
        if kind == AbstractionKind::Property && parameters.is_empty() {
            self.insert_implicit_generated_property_slot_parameter(
                body,
                &mut parameters,
                self.source_for_node(abstraction, "implicit-property-slot"),
                main_generated_selbri_for_subbridi(&abstraction.subbridi),
            )?;
        }
        self.set_formula_predication_mode(body, abstraction_body_mode(kind));

        if let Some((id, class)) = eventuality {
            let mut object = SemanticObject::eventuality(class, None, source);
            object.sort = Some(sort);
            object.content = Some(body);
            object.abstraction_kind = Some(kind);
            object.parameters = parameters;
            self.insert(id, object)?;
            return Ok(id);
        }

        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::abstraction(kind, body, parameters, source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_implicit_generated_property_slot_parameter(
        &mut self,
        body: SemanticObjectId,
        parameters: &mut Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        preferred_selbri: Option<&SelbriSyntax>,
    ) -> Result<(), SemanticsError> {
        if !parameters.is_empty() {
            return Ok(());
        }
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "implicit ce'u".to_owned(),
                source,
            ),
        )?;
        if self.replace_first_elided_generated_formula_argument(
            body,
            parameter,
            preferred_selbri,
        )? {
            parameters.push(parameter);
        } else {
            self.objects.remove(&parameter);
        }
        Ok(())
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    fn build_elided_argument_with_sort(
        &mut self,
        label: String,
        sort: SemanticSort,
    ) -> Result<ArgumentValue, SemanticsError> {
        let referent = self.build_elided_referent_with_sort(label.clone(), sort)?;
        Ok(ArgumentValue::elided(referent, label, None))
    }

    #[requires(place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.kind == ArgumentValueKind::Elided) || ret.is_err())]
    fn build_elided_argument_for_place(
        &mut self,
        place: usize,
    ) -> Result<ArgumentValue, SemanticsError> {
        let _ = place;
        let label = "zo'e".to_owned();
        let referent = self.build_elided_referent(label.clone())?;
        Ok(ArgumentValue::elided(referent, label, None))
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent(&mut self, label: String) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, SemanticSort::Entity, None)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent_with_source(
        &mut self,
        label: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, SemanticSort::Entity, source)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent_with_sort(
        &mut self,
        label: String,
        sort: SemanticSort,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, sort, None)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent_with_sort_and_source(
        &mut self,
        label: String,
        sort: SemanticSort,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_with_sort_id(sort);
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
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                }),
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(id.object_kind() == object.object_kind())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert(
        &mut self,
        id: SemanticObjectId,
        object: SemanticObject,
    ) -> Result<(), SemanticsError> {
        if self.objects.insert(id, object).is_some() {
            return Err(SemanticsError {
                kind: SemanticsErrorKind::DuplicateObject,
                message: format!("semantic builder attempted to insert duplicate object ID {id}"),
            });
        }
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
    #[ensures(ret.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.referent_sort() == Some(SemanticSort::Relation))]
    fn next_relation_id(&mut self) -> SemanticObjectId {
        let id = SemanticObjectId::referent_with_sort(SemanticSort::Relation, self.next_index);
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
        visitor.tokens
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

    #[requires(true)]
    #[ensures(true)]
    fn source_for_gadri_description(
        &self,
        description_head: &DescriptionHeadSyntax,
        tail: &DescriptionTailSyntax,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut visitor = GeneratedSpanCollector::default();
        description_head.visit_in_order(&mut visitor);
        tail.visit_in_order(&mut visitor);
        let spans =
            source_spans_with_following_cmevla_period(&visitor.spans, self.options.source_text);
        source_from_spans(&spans, self.options.source_text, Some(construct))
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|(byte_start, byte_end)| byte_start <= byte_end))]
    fn source_key_for_node<N: TreeNode>(&self, node: &N) -> Option<(usize, usize)> {
        self.source_for_node(node, "source-key")
            .map(|source| (source.span.byte_start, source.span.byte_end))
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

#[derive(Default)]
#[invariant(true)]
struct GeneratedSpanCollector {
    spans: Vec<SourceSpan>,
    tokens: Vec<Token>,
}

impl<'tree> TreeVisitor<'tree> for GeneratedSpanCollector {
    type Atom = GeneratedAtomRef<'tree>;
    type Node = jbotci_syntax::generated_model::NodeRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let GeneratedAtomRef::Token(token) = atom;
        self.spans.extend(token.source_spans().into_iter().cloned());
        self.tokens.push(token.clone());
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|roots| !roots.is_empty()) || ret.is_err())]
fn semantic_roots_from_text(
    syntax: &TextSyntax,
) -> Result<Vec<GeneratedTextRoot<'_>>, SemanticsError> {
    let TextSyntax::RegularText(RegularTextSyntax {
        leading_nai,
        leading_cmevla,
        leading_indicators,
        leading_free_modifiers,
        leading_connective,
        leading_i_statements,
        paragraphs: Some(paragraphs),
    }) = syntax
    else {
        return Err(unsupported("non-regular generated text"));
    };
    if !leading_nai.is_empty()
        || !leading_cmevla.is_empty()
        || !leading_indicators.is_empty()
        || !leading_free_modifiers.is_empty()
        || leading_connective.is_some()
    {
        return Err(unsupported("text leading material"));
    }
    for leading_i in leading_i_statements {
        if leading_i.connective.is_some() || !leading_i.free_modifiers.is_empty() {
            return Err(unsupported("text leading material"));
        }
    }
    let TextParagraphsSyntax::TextParagraphWithAdditionalNiho(
        TextParagraphWithAdditionalNihoSyntax {
            first,
            additional_niho,
        },
    ) = paragraphs.as_ref()
    else {
        return Err(unsupported("NIhO paragraph text"));
    };
    if !additional_niho.is_empty() {
        return Err(unsupported("additional paragraphs"));
    }
    semantic_roots_from_paragraph(first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|roots| !roots.is_empty()) || ret.is_err())]
fn semantic_roots_from_paragraph(
    paragraph: &ParagraphSyntax,
) -> Result<Vec<GeneratedTextRoot<'_>>, SemanticsError> {
    let sequence = match paragraph {
        ParagraphSyntax::SimpleParagraph(SimpleParagraphSyntax(sequence)) => sequence,
        ParagraphSyntax::INihoParagraph(paragraph) => {
            if !paragraph.free_modifiers.is_empty() {
                return Err(unsupported("NIhO paragraph"));
            }
            let Some(sequence) = paragraph.statements.as_deref() else {
                return Err(unsupported("empty NIhO paragraph"));
            };
            sequence
        }
    };
    if !sequence.trailing.is_empty() {
        return Err(unsupported("paragraph statement continuations"));
    }
    let mut roots = vec![semantic_root_from_statement_or_fragment(
        sequence.initial.0.as_ref(),
    )?];
    for following in &sequence.following {
        if !following.free_modifiers.is_empty() {
            return Err(unsupported("paragraph statement continuations"));
        }
        let Some(statement) = following.statement.as_deref() else {
            continue;
        };
        roots.push(semantic_root_from_statement_or_fragment(statement)?);
    }
    Ok(roots)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn semantic_root_from_statement_or_fragment(
    statement_or_fragment: &StatementOrFragmentSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
    match statement_or_fragment {
        StatementOrFragmentSyntax::StatementOrFragmentStatement(
            StatementOrFragmentStatementSyntax(statement),
        ) => semantic_root_from_statement(statement),
        StatementOrFragmentSyntax::FragmentStatement(FragmentStatementSyntax::TermsFragment(
            fragment,
        )) => Ok(GeneratedTextRoot::TermsFragment(fragment)),
        StatementOrFragmentSyntax::FragmentStatement(_) => Err(unsupported("non-terms fragment")),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn semantic_root_from_statement(
    statement: &StatementSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
    match statement {
        StatementSyntax::IStatementConnection(connection) => {
            Ok(GeneratedTextRoot::StatementConnection(connection))
        }
        StatementSyntax::PreposedIStatementConnection(connection) => {
            Ok(GeneratedTextRoot::PreposedStatementConnection(connection))
        }
        StatementSyntax::StatementBase(base) => {
            Ok(GeneratedTextRoot::Bridi(bridi_from_statement_base(base)?))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_text_root_is_utterance(root: &GeneratedTextRoot<'_>) -> bool {
    matches!(
        root,
        GeneratedTextRoot::Bridi(_) | GeneratedTextRoot::TermsFragment(_)
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_force(bridi: &BridiSyntax) -> UtteranceForce {
    if generated_node_contains_cmavo(bridi, Cmavo::Ko) {
        UtteranceForce::Command
    } else {
        UtteranceForce::Assert
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
fn bridi_from_statement_base(base: &StatementBaseSyntax) -> Result<&BridiSyntax, SemanticsError> {
    match base {
        StatementBaseSyntax::BridiStatement(statement) => bridi_from_bridi_statement(statement),
        StatementBaseSyntax::PrenexStatement(_) => Err(unsupported("prenex statement")),
        StatementBaseSyntax::TextGroupStatement(_) => Err(unsupported("text group statement")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
fn bridi_from_bridi_statement(
    statement: &BridiStatementSyntax,
) -> Result<&BridiSyntax, SemanticsError> {
    if !statement.continuations.is_empty() {
        return Err(unsupported("bridi statement continuations"));
    }
    Ok(&statement.bridi)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
fn bridi_from_statement_after_i_connective(
    statement: &StatementAfterIConnectiveSyntax,
) -> Result<&BridiSyntax, SemanticsError> {
    match statement {
        StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
            bridi_from_bridi_statement(statement)
        }
        StatementAfterIConnectiveSyntax::TextGroupStatement(_) => {
            Err(unsupported("text group statement connection"))
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
fn main_generated_selbri_for_subbridi(subbridi: &SubbridiSyntax) -> Option<&SelbriSyntax> {
    match subbridi {
        SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
            main_generated_selbri_for_bridi(bridi)
        }
        SubbridiSyntax::PrenexSubbridi(prenex) => {
            main_generated_selbri_for_subbridi(&prenex.inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
fn main_generated_selbri_for_bridi(bridi: &BridiSyntax) -> Option<&SelbriSyntax> {
    match bridi {
        BridiSyntax::BridiWithLeadingTerms(BridiWithLeadingTermsSyntax { bridi_tail, .. })
        | BridiSyntax::BareCuBridi(BareCuBridiSyntax { bridi_tail, .. }) => {
            main_generated_selbri_for_bridi_tail(bridi_tail)
        }
        BridiSyntax::BridiWithPostCuTerms(BridiWithPostCuTermsSyntax { bridi_tail, .. })
        | BridiSyntax::BareCuTermsBridi(BareCuTermsBridiSyntax { bridi_tail, .. }) => {
            main_generated_selbri_for_cu_terms_bridi_tail(bridi_tail)
        }
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
            main_generated_selbri_for_bridi_tail(bridi_tail)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
fn main_generated_selbri_for_cu_terms_bridi_tail(
    tail: &CuTermsBridiTailSyntax,
) -> Option<&SelbriSyntax> {
    main_generated_selbri_for_bridi_tail(&tail.bridi_tail)
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
fn main_generated_selbri_for_bridi_tail(tail: &BridiTailSyntax) -> Option<&SelbriSyntax> {
    simple_tail_from_bridi_tail(tail)
        .ok()
        .map(|simple_tail| simple_tail.selbri.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|(_connective, bridi)| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
fn statement_connection_tail_parts(
    tail: &IStatementConnectionTailSyntax,
) -> Result<(&IStatementConnectiveSyntax, &BridiSyntax), SemanticsError> {
    match tail {
        IStatementConnectionTailSyntax::SimpleIConnectiveStatementTail(tail) => Ok((
            &tail.connective,
            bridi_from_statement_after_i_connective(&tail.trailing_statement)?,
        )),
        IStatementConnectionTailSyntax::ChainedIConnectiveStatementTail(_) => {
            Err(unsupported("chained pending statement connective"))
        }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn simple_tail_from_bridi_tail(
    tail: &BridiTailSyntax,
) -> Result<&SelbriSimpleBridiTailSyntax, SemanticsError> {
    let BridiTailSyntax::BridiTailWithPossibleTailTerms(BridiTailWithPossibleTailTermsSyntax {
        first,
        ke_continuation,
    }) = tail
    else {
        return Err(unsupported("bridi tail without possible terms"));
    };
    if ke_continuation.is_some() || !first.0.links.is_empty() {
        return Err(unsupported("connected bridi tail"));
    }
    let BoGroupedBridiTailSyntax {
        first,
        bo_continuation,
    } = first.0.first.as_ref();
    if bo_continuation.is_some() {
        return Err(unsupported("BO grouped bridi tail"));
    }
    let SimpleBridiTailSyntax::SelbriSimpleBridiTail(simple_tail) = first.as_ref() else {
        return Err(unsupported("forethought simple bridi tail"));
    };
    Ok(simple_tail)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_selbri(selbri: &SelbriSyntax) -> Result<String, SemanticsError> {
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(CoSelbriSyntax {
        leading_selbri: _,
        co_tail: _,
    })) = selbri
    else {
        return Err(unsupported("tagged or connected selbri"));
    };
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
        unreachable!("previous pattern requires a co selbri")
    };
    relation_label_from_co_selbri(co_selbri)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_co_selbri(selbri: &CoSelbriSyntax) -> Result<String, SemanticsError> {
    if selbri.co_tail.is_some() {
        return Err(unsupported("CO selbri"));
    }
    let ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = selbri.leading_selbri.as_ref();
    if !continuations.is_empty() {
        return Err(unsupported("connected selbri"));
    }
    let TanruSelbriSyntax {
        first_unit,
        additional_units,
    } = leading_selbri.as_ref();
    if !additional_units.is_empty() {
        return Err(unsupported("tanru"));
    }
    relation_label_from_tanru_unit(first_unit)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn relation_question_syntax_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<Option<GeneratedRelationQuestionSyntax<'_>>, SemanticsError> {
    if selbri.co_tail.is_some() {
        return Ok(None);
    }
    let ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = selbri.leading_selbri.as_ref();
    if !continuations.is_empty() {
        return Ok(None);
    }
    let TanruSelbriSyntax {
        first_unit,
        additional_units,
    } = leading_selbri.as_ref();
    if !additional_units.is_empty() || !first_unit.0.links.is_empty() {
        return Ok(None);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = first_unit.0.first.as_ref() else {
        return Ok(None);
    };
    if unit.linkargs.is_some() || !unit.base.conversions.is_empty() {
        return Ok(None);
    }
    match unit.base.base.as_ref() {
        TanruUnitAtomBaseSyntax::ProBridiTanruUnit(pro_bridi)
            if pro_bridi.goha.value.cmavo() == Some(Cmavo::Mo) =>
        {
            Ok(Some(GeneratedRelationQuestionSyntax::ProBridi(pro_bridi)))
        }
        TanruUnitAtomBaseSyntax::GohaWordTanruUnit(goha)
            if generated_goha_word_tanru_unit_token(goha).cmavo() == Some(Cmavo::Mo) =>
        {
            Ok(Some(GeneratedRelationQuestionSyntax::GohaWord(goha)))
        }
        _ => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_question_token(question: GeneratedRelationQuestionSyntax<'_>) -> &Token {
    match question {
        GeneratedRelationQuestionSyntax::ProBridi(pro_bridi) => &pro_bridi.goha.value,
        GeneratedRelationQuestionSyntax::GohaWord(goha) => {
            generated_goha_word_tanru_unit_token(goha)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_goha_word_tanru_unit_token(unit: &GohaWordTanruUnitSyntax) -> &Token {
    let GohaWordTanruUnitSyntax(word) = unit;
    &word.value
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn tanru_selbri_from_selbri(
    selbri: &SelbriSyntax,
) -> Result<Option<&TanruSelbriSyntax>, SemanticsError> {
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(CoSelbriSyntax {
        leading_selbri: _,
        co_tail: _,
    })) = selbri
    else {
        return Ok(None);
    };
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
        unreachable!("previous pattern requires a co selbri")
    };
    tanru_selbri_from_co_selbri(co_selbri)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn tanru_selbri_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<Option<&TanruSelbriSyntax>, SemanticsError> {
    if selbri.co_tail.is_some() {
        return Ok(None);
    }
    let ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = selbri.leading_selbri.as_ref();
    if !continuations.is_empty() {
        return Ok(None);
    }
    Ok(Some(leading_selbri.as_ref()))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn connected_selbri_as_tanru(
    selbri: &ConnectedSelbriSyntax,
) -> Result<&TanruSelbriSyntax, SemanticsError> {
    if !selbri.continuations.is_empty() {
        return Err(unsupported("connected grouped tanru unit"));
    }
    Ok(selbri.leading_selbri.as_ref())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn sumti_selbri_from_selbri(
    selbri: &SelbriSyntax,
) -> Result<Option<&SumtiSelbriTanruUnitSyntax>, SemanticsError> {
    let Some(tanru) = tanru_selbri_from_selbri(selbri)? else {
        return Ok(None);
    };
    if !tanru.additional_units.is_empty() {
        return Ok(None);
    }
    sumti_selbri_from_generated_tanru_unit(&tanru.first_unit)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn sumti_selbri_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<&SumtiSelbriTanruUnitSyntax>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Ok(None);
    };
    let atom = &unit.base;
    let linkargs = unit.linkargs.as_ref();
    let TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(sumti_selbri) = atom.base.as_ref() else {
        return Ok(None);
    };
    if linkargs.is_some() {
        return Err(unsupported("linkargs sumti selbri"));
    }
    if !atom.conversions.is_empty() {
        return Err(unsupported("converted sumti selbri"));
    }
    Ok(Some(sumti_selbri))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn generated_tanru_unit_is_grouped(unit: &TanruUnitSyntax) -> Result<bool, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(false);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Ok(false);
    };
    let atom = &unit.base;
    if matches!(
        atom.base.as_ref(),
        TanruUnitAtomBaseSyntax::GroupedTanruUnit(_)
    ) {
        return Ok(true);
    }
    Ok(scalar_negated_tanru_atom_base(atom.base.as_ref())
        .and_then(scalar_negated_tanru_unit_inner_grouped)
        .is_some())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn abstraction_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<&AbstractionTanruUnitSyntax>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Ok(None);
    };
    let atom = &unit.base;
    let linkargs = unit.linkargs.as_ref();
    let TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) = atom.base.as_ref() else {
        return Ok(None);
    };
    if linkargs.is_some() {
        return Err(unsupported("linkargs abstraction"));
    }
    if !atom.conversions.is_empty() {
        return Err(unsupported("converted abstraction"));
    }
    if abstraction.nai.is_some() {
        return Err(unsupported("negated abstraction"));
    }
    if !abstraction.abstractor_connections.is_empty() {
        return Err(unsupported("connected abstraction"));
    }
    Ok(Some(abstraction))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|atom| atom.conversions.is_empty()) || ret.is_err())]
fn generated_tanru_unit_atom(
    unit: &TanruUnitSyntax,
) -> Result<&TanruUnitAtomSyntax, SemanticsError> {
    let (atom, linkargs) = generated_linked_tanru_unit_parts(unit)?;
    if linkargs.is_some() {
        return Err(unsupported("linkargs tanru unit"));
    }
    if !atom.conversions.is_empty() {
        return Err(unsupported("converted tanru unit"));
    }
    Ok(atom)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn generated_linked_tanru_unit_parts(
    unit: &TanruUnitSyntax,
) -> Result<(&TanruUnitAtomSyntax, Option<&LinkargsSyntax>), SemanticsError> {
    if !unit.0.links.is_empty() {
        return Err(unsupported("connected tanru unit"));
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Err(unsupported("non-atomic tanru unit"));
    };
    Ok((&unit.base, unit.linkargs.as_ref()))
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_selbri(
    selbri: &SelbriSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => {
            generated_raw_place_visible_rank_for_untagged_selbri(&tagged.inner_selbri, place)
        }
        SelbriSyntax::UntaggedSelbri(untagged) => {
            generated_raw_place_visible_rank_for_untagged_selbri(untagged, place)
        }
    }
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_untagged_selbri(
    selbri: &UntaggedSelbriSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    match selbri {
        UntaggedSelbriSyntax::NegatedSelbri(negated) => {
            generated_raw_place_visible_rank_for_selbri(&negated.inner_selbri, place)
        }
        UntaggedSelbriSyntax::CoSelbri(co_selbri) if co_selbri.co_tail.is_none() => {
            generated_raw_place_visible_rank_for_connected_selbri(&co_selbri.leading_selbri, place)
        }
        UntaggedSelbriSyntax::CoSelbri(_)
        | UntaggedSelbriSyntax::ForethoughtSelbriConnection(_) => Ok(place),
    }
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_connected_selbri(
    selbri: &ConnectedSelbriSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    if !selbri.continuations.is_empty() {
        return Ok(place);
    }
    generated_raw_place_visible_rank_for_tanru_selbri(&selbri.leading_selbri, place)
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_tanru_selbri(
    selbri: &TanruSelbriSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    let unit = selbri.additional_units.last().unwrap_or(&selbri.first_unit);
    generated_raw_place_visible_rank_for_tanru_unit(unit, place)
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_tanru_unit(
    unit: &TanruUnitSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(place);
    }
    generated_raw_place_visible_rank_for_bo_or_linked_tanru_unit(&unit.0.first, place)
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            generated_raw_place_visible_rank_for_tanru_unit_atom(&unit.base, place)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) if unit.bo_connective.is_none() => {
            generated_raw_place_visible_rank_for_bo_or_linked_tanru_unit(&unit.trailing_unit, place)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => Ok(place),
    }
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_tanru_unit_atom(
    atom: &TanruUnitAtomSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    let place = match atom.base.as_ref() {
        TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
            generated_raw_place_visible_rank_for_connected_selbri(&grouped.selbri, place)?
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_raw_place_visible_rank_for_scalar_negated_tanru_unit(unit, place)?
        }
        _ => place,
    };
    mapped_place_for_generated_conversions(place, &atom.conversions)
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn generated_raw_place_visible_rank_for_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            generated_raw_place_visible_rank_for_tanru_unit_atom(atom, place)
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(grouped) => {
            generated_raw_place_visible_rank_for_connected_selbri(&grouped.inner_selbri, place)
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(_) => Ok(place),
    }
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
fn mapped_place_for_generated_conversions<F>(
    place: usize,
    conversions: &[WithFreeModifiers<Token, F>],
) -> Result<usize, SemanticsError> {
    let mut place = place;
    for conversion in conversions {
        if let Some(converted_place) = se_conversion_place(&conversion.value)? {
            place = convert_numbered_place(place, converted_place);
        }
    }
    Ok(place)
}

#[requires(arguments.keys().all(|place| *place > 0))]
#[ensures(ret.as_ref().is_ok_and(|arguments| arguments.keys().all(|place| *place > 0)) || ret.is_err())]
fn map_visible_arguments_for_generated_conversions<F>(
    arguments: BTreeMap<usize, ArgumentValue>,
    conversions: &[WithFreeModifiers<Token, F>],
) -> Result<BTreeMap<usize, ArgumentValue>, SemanticsError> {
    if conversions.is_empty() {
        return Ok(arguments);
    }
    let mut mapped_arguments = BTreeMap::new();
    for (visible_place, argument) in arguments {
        let place = mapped_place_for_generated_conversions(visible_place, conversions)?;
        if mapped_arguments.insert(place, argument).is_some() {
            return Err(invalid_graph(format!(
                "multiple grouped tanru arguments map to visible place x{place}"
            )));
        }
    }
    Ok(mapped_arguments)
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
#[ensures(ret.as_ref().is_ok_and(|place| match place { None => true, Some(place) => (2..=5).contains(place) }) || ret.is_err())]
fn se_conversion_place(token: &Token) -> Result<Option<usize>, SemanticsError> {
    match token.cmavo() {
        Some(Cmavo::Se) => Ok(Some(2)),
        Some(Cmavo::Te) => Ok(Some(3)),
        Some(Cmavo::Ve) => Ok(Some(4)),
        Some(Cmavo::Xe) => Ok(Some(5)),
        Some(cmavo) => Err(unsupported(&format!("SE conversion cmavo {cmavo:?}"))),
        None => Err(unsupported("non-cmavo SE conversion")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| (1..=5).contains(place)) || ret.is_err())]
fn fa_place(token: &Token) -> Result<usize, SemanticsError> {
    match token.cmavo() {
        Some(Cmavo::Fa) => Ok(1),
        Some(Cmavo::Fe) => Ok(2),
        Some(Cmavo::Fi) => Ok(3),
        Some(Cmavo::Fo) => Ok(4),
        Some(Cmavo::Fu) => Ok(5),
        Some(Cmavo::Fiha) => Err(unsupported("place-question linked sumti")),
        Some(Cmavo::Fai) => Err(unsupported("FAI linked sumti")),
        Some(cmavo) => Err(unsupported(&format!("FA linked sumti cmavo {cmavo:?}"))),
        None => Err(unsupported("non-cmavo FA linked sumti")),
    }
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place >= first_visible_place) || ret.is_err())]
fn next_visible_place_after_linkargs(
    linkargs: &LinkargsSyntax,
    first_visible_place: usize,
) -> Result<usize, SemanticsError> {
    let mut next_visible_place = first_visible_place;
    advance_visible_place_after_linked_sumti(&mut next_visible_place, &linkargs.first_link)?;
    for link in &linkargs.bei_links {
        advance_visible_place_after_linked_sumti(&mut next_visible_place, &link.link)?;
    }
    Ok(next_visible_place)
}

#[requires(*next_visible_place > 0)]
#[ensures(ret.is_ok() || ret.is_err())]
fn advance_visible_place_after_linked_sumti(
    next_visible_place: &mut usize,
    link: &LinkedSumtiSyntax,
) -> Result<(), SemanticsError> {
    match link {
        LinkedSumtiSyntax::PlainLinkedSumti(_) => {
            *next_visible_place += 1;
        }
        LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
            let place = fa_place(&sumti.fa.value)?;
            *next_visible_place = (*next_visible_place).max(place + 1);
        }
        LinkedSumtiSyntax::TenseTaggedLinkedSumti(_) => {
            return Err(unsupported("tense-tagged linked sumti"));
        }
        LinkedSumtiSyntax::EmptyLinkedSumti(_) => {
            return Err(unsupported("empty linked sumti"));
        }
    }
    Ok(())
}

#[requires(place > 0)]
#[ensures(ret.is_ok() || ret.is_err())]
fn insert_visible_argument(
    arguments: &mut BTreeMap<usize, ArgumentValue>,
    place: usize,
    argument: ArgumentValue,
) -> Result<(), SemanticsError> {
    if arguments.insert(place, argument).is_some() {
        return Err(invalid_graph(format!(
            "multiple generated tanru arguments assign visible place x{place}"
        )));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let mut label = relation_label_from_bo_or_linked_tanru_unit(&unit.0.first)?;
    for link in &unit.0.links {
        label = format!(
            "{} {} {}",
            label,
            relation_afterthought_connective_label(&link.connective)?,
            relation_label_from_bo_or_linked_tanru_unit(&link.trailing_unit)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_unit_label_from_generated_unit(unit: &TanruUnitSyntax) -> Result<String, SemanticsError> {
    if !unit.0.links.is_empty() {
        return relation_label_from_generated_tanru_unit(unit);
    }
    tanru_unit_label_from_bo_or_linked_tanru_unit(&unit.0.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_tanru_unit_atom_base(
    base: &TanruUnitAtomBaseSyntax,
) -> Result<String, SemanticsError> {
    match base {
        TanruUnitAtomBaseSyntax::OrdinalTanruUnit(ordinal) => {
            relation_label_from_ordinal_tanru_unit(ordinal)
        }
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            Ok(token_text(&word.value))
        }
        TanruUnitAtomBaseSyntax::GohaWordTanruUnit(GohaWordTanruUnitSyntax(word)) => {
            Ok(token_text(&word.value))
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
        TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(_) => Ok("referentOf".to_owned()),
        _ => Err(unsupported("non-word tanru unit")),
    }
}

#[requires(true)]
#[ensures(true)]
fn scalar_negated_tanru_atom_base(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<&ScalarNegatedTanruUnitSyntax> {
    match base {
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => Some(unit),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn scalar_negated_tanru_unit_inner_atom(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<&TanruUnitAtomSyntax> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => Some(atom),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn scalar_negated_tanru_unit_inner_grouped(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<(
    &GroupedTanruUnitSyntax,
    &[WithFreeModifiers<Token, FreeModifierSyntax>],
)> {
    let ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) = unit.inner_unit.as_ref() else {
        return None;
    };
    let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = atom.base.as_ref() else {
        return None;
    };
    Some((grouped, atom.conversions.as_slice()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            relation_label_from_tanru_unit_atom_base(atom.base.as_ref())
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi))
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => {
            Err(unsupported("tagged scalar-negated tanru unit"))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_grouped_tanru_unit(
    grouped: &GroupedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    relation_phrase_label_from_connected_selbri(&grouped.selbri)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_phrase_label_from_selbri(selbri: &SelbriSyntax) -> Result<String, SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(_) => Err(unsupported("tagged selbri relation phrase label")),
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) => {
            if co_selbri.co_tail.is_some() {
                return Err(unsupported("CO selbri relation phrase label"));
            }
            relation_phrase_label_from_connected_selbri(co_selbri.leading_selbri.as_ref())
        }
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::NegatedSelbri(_)) => {
            Err(unsupported("negated selbri relation phrase label"))
        }
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::ForethoughtSelbriConnection(
            connection,
        )) => Ok(format!(
            "{} {} {}",
            generated_guhek_connective_source(&connection.guhek),
            relation_phrase_label_from_selbri(connection.leading_selbri.as_ref())?,
            relation_phrase_label_from_selbri(connection.trailing_selbri.as_ref())?,
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_phrase_label_from_connected_selbri(
    selbri: &ConnectedSelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut label = relation_phrase_label_from_tanru_selbri(&selbri.leading_selbri)?;
    for continuation in &selbri.continuations {
        label = format!(
            "{label} {} {}",
            relation_afterthought_connective_label(&continuation.connective)?,
            relation_phrase_label_from_tanru_selbri(&continuation.trailing_selbri)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_phrase_label_from_tanru_selbri(
    tanru: &TanruSelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut label = relation_label_from_generated_tanru_unit(&tanru.first_unit)?;
    for unit in &tanru.additional_units {
        label = format!(
            "{label} {}",
            relation_label_from_generated_tanru_unit(unit)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_label_from_tanru_selbri(tanru: &TanruSelbriSyntax) -> Result<String, SemanticsError> {
    let mut label = tanru_unit_label_from_generated_unit(&tanru.first_unit)?;
    for unit in &tanru.additional_units {
        label = format!("{label}-{}", tanru_unit_label_from_generated_unit(unit)?);
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn tanru_label_from_connected_selbri(
    selbri: &ConnectedSelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut label = tanru_label_from_tanru_selbri(&selbri.leading_selbri)?;
    for continuation in &selbri.continuations {
        label = format!(
            "{label} {} {}",
            relation_afterthought_connective_label(&continuation.connective)?,
            tanru_label_from_tanru_selbri(&continuation.trailing_selbri)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_afterthought_connective_label(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<String, SemanticsError> {
    Ok(generated_relation_afterthought_connective_source(connective)?.replace(' ', "-"))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relation_label_from_pro_bridi_tanru_unit(unit: &ProBridiTanruUnitSyntax) -> String {
    token_text(&unit.goha.value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_ordinal_tanru_unit(
    ordinal: &OrdinalTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    ordinal.visit_in_order(&mut visitor);
    if visitor.tokens.is_empty() {
        return Err(unsupported("empty ordinal tanru unit"));
    }
    Ok(token_list_text(visitor.tokens.iter()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn abstraction_relation_label_from_generated(
    abstraction: &AbstractionTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let abstractor = token_text(&abstraction.nu.value);
    let relation = relation_label_from_subbridi(&abstraction.subbridi)?;
    Ok(format!("{abstractor} {relation}"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_subbridi(subbridi: &SubbridiSyntax) -> Result<String, SemanticsError> {
    let SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) = subbridi else {
        return Err(unsupported("prenex subbridi relation label"));
    };
    let tail = match bridi.as_ref() {
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(tail)) => tail,
        BridiSyntax::BridiWithLeadingTerms(BridiWithLeadingTermsSyntax { bridi_tail, .. }) => {
            bridi_tail
        }
        _ => return Err(unsupported("subbridi relation label")),
    };
    let simple_tail = simple_tail_from_bridi_tail(tail)?;
    relation_label_from_selbri(&simple_tail.selbri)
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
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_is_connected_selbri_formula(unit: &TanruUnitSyntax) -> bool {
    !unit.0.links.is_empty() || bo_or_linked_tanru_unit_has_bo_connective(unit.0.first.as_ref())
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
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_tanru_unit(unit: &TanruUnitSyntax) -> Result<String, SemanticsError> {
    relation_label_from_generated_tanru_unit(unit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
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
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
            Err(unsupported("assigned pro-bridi tanru unit label"))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_linked_tanru_unit(
    unit: &LinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    relation_label_from_tanru_unit_atom(&unit.base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_bound_tanru_unit(
    unit: &BoundTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let leading = relation_label_from_linked_tanru_unit(&unit.leading_unit)?;
    let trailing = relation_label_from_bo_or_linked_tanru_unit(&unit.trailing_unit)?;
    if let Some(connective) = &unit.bo_connective {
        Ok(format!(
            "{} {} {}",
            leading,
            relation_afterthought_connective_label(connective)?,
            trailing
        ))
    } else {
        Ok(format!("{leading} bo {trailing}"))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_forethought_selbri_group_tanru_unit(
    unit: &ForethoughtSelbriGroupTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    Ok(format!(
        "{} {} {}",
        generated_guhek_connective_source(&unit.guhek),
        relation_phrase_label_from_selbri(unit.leading_selbri.as_ref())?,
        relation_label_from_bo_or_linked_tanru_unit(unit.trailing_unit.as_ref())?,
    ))
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
        }
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => {
            Err(unsupported("assigned pro-bridi tanru unit label"))
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
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => false,
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
        _ => relation_label_from_tanru_unit_atom(unit),
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
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi))
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => {
            Err(unsupported("tagged scalar-negated tanru unit"))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_tanru_unit_atom(
    unit: &TanruUnitAtomSyntax,
) -> Result<String, SemanticsError> {
    match unit.base.as_ref() {
        TanruUnitAtomBaseSyntax::OrdinalTanruUnit(ordinal) => {
            relation_label_from_ordinal_tanru_unit(ordinal)
        }
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word))
        | TanruUnitAtomBaseSyntax::GohaWordTanruUnit(GohaWordTanruUnitSyntax(word)) => {
            Ok(token_text(&word.value))
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
        TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(_) => Ok("referentOf".to_owned()),
        _ => Err(unsupported("non-word tanru unit")),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
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
#[ensures(ret.is_ok() || ret.is_err())]
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
    let SumtiAtomSyntax::QuantifiedSumti(quantified) = simple.base_sumti.as_ref() else {
        return None;
    };
    Some(quantified)
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
#[ensures(ret.is_ok() || ret.is_err())]
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
#[ensures(ret.is_ok() || ret.is_err())]
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
#[ensures(ret.is_ok() || ret.is_err())]
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
fn generated_sumti_relative_clause_list(sumti: &SumtiSyntax) -> Option<&RelativeClauseListSyntax> {
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
    simple.relative_clauses.as_ref()
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
fn generated_subbridi_contains_cmavo(subbridi: &SubbridiSyntax, cmavo: Cmavo) -> bool {
    generated_node_contains_cmavo(subbridi, cmavo)
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

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
fn argument_place_index(place: &str) -> Option<usize> {
    place
        .strip_prefix('x')
        .and_then(|suffix| suffix.parse::<usize>().ok())
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
#[ensures(ret.as_ref().is_none_or(|(_domain, relation)| !relation.relation.is_empty()))]
fn generated_anchor_relation_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<(GeneratedAnchorDomain, AnchorRelation)> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    for token in collector.tokens {
        if let Some(relation) = time_relation_for_pu_token(&token) {
            return Some((
                GeneratedAnchorDomain::Time,
                new!(AnchorRelation {
                    relation,
                    anchor: SemanticObjectId::now(),
                    sticky: false,
                    inherited: None,
                    distance: None,
                    magnitude: None,
                    scalar_negation: None,
                    motion: None,
                }),
            ));
        }
        if let Some(relation) = space_relation_for_faha_token(&token) {
            return Some((
                GeneratedAnchorDomain::Space,
                new!(AnchorRelation {
                    relation,
                    anchor: SemanticObjectId::here(),
                    sticky: false,
                    inherited: None,
                    distance: None,
                    magnitude: None,
                    scalar_negation: None,
                    motion: None,
                }),
            ));
        }
        if let Some(distance) = space_distance_for_va_token(&token) {
            return Some((
                GeneratedAnchorDomain::Space,
                new!(AnchorRelation {
                    relation: "distanceFrom".to_owned(),
                    anchor: SemanticObjectId::here(),
                    sticky: false,
                    inherited: None,
                    distance: Some(distance),
                    magnitude: None,
                    scalar_negation: None,
                    motion: None,
                }),
            ));
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn generated_modal_relation_spec_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(String, String, usize)> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    for (index, token) in collector.tokens.iter().enumerate() {
        if !token.is_selmaho(Selmaho::Bai) {
            continue;
        }
        let marker = token_text(token);
        let conversion = index
            .checked_sub(1)
            .and_then(|previous| collector.tokens.get(previous))
            .filter(|previous| previous.is_selmaho(Selmaho::Se));
        let visible_place = conversion
            .and_then(generated_se_token_conversion_place)
            .unwrap_or(1);
        let introduced_by = conversion
            .map(|conversion| format!("{} {marker}", token_text(conversion)))
            .unwrap_or_else(|| marker.clone());
        return Some((
            introduced_by,
            modal_relation_for_marker(&marker),
            visible_place,
        ));
    }
    generated_tense_relation_spec_for_tokens(&collector.tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn generated_tense_relation_spec_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(String, String, usize)> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    generated_tense_relation_spec_for_tokens(&collector.tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
fn generated_tense_relation_spec_for_tokens(tokens: &[Token]) -> Option<(String, String, usize)> {
    for token in tokens {
        if let Some(relation) = time_relation_for_pu_token(token)
            .or_else(|| space_relation_for_faha_token(token))
            .or_else(|| space_distance_for_va_token(token).map(|_| "distanceFrom".to_owned()))
        {
            return Some((token_text(token), relation, 1));
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
fn time_relation_for_pu_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Pu) => Some("before".to_owned()),
        Some(Cmavo::Ca) => Some("at".to_owned()),
        Some(Cmavo::Ba) => Some("after".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
fn space_distance_for_va_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vi) => Some("short".to_owned()),
        Some(Cmavo::Va) => Some("medium".to_owned()),
        Some(Cmavo::Vu) => Some("long".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
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
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
fn generated_se_token_conversion_place(se: &Token) -> Option<usize> {
    match se.cmavo() {
        Some(Cmavo::Se) => Some(2),
        Some(Cmavo::Te) => Some(3),
        Some(Cmavo::Ve) => Some(4),
        Some(Cmavo::Xe) => Some(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
fn generated_modal_negation_for_tense_modal<N: TreeNode>(tense_modal: &N) -> Option<ModalNegation> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut previous_recurrence_marker = false;
    for token in &collector.tokens {
        if token.cmavo() == Some(Cmavo::Nai) {
            if !previous_recurrence_marker {
                return Some(ModalNegation::new(
                    ModalNegationKind::Contradictory,
                    token_text(token),
                ));
            }
            previous_recurrence_marker = false;
            continue;
        }
        previous_recurrence_marker = token_is_recurrence_interval_marker(token);
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn token_is_recurrence_interval_marker(token: &Token) -> bool {
    matches!(
        token.cmavo(),
        Some(Cmavo::Roi | Cmavo::Rehu | Cmavo::Dihi | Cmavo::Naho | Cmavo::Ruhi | Cmavo::Tahe)
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
fn generated_modal_scalar_negation_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<ScalarNegation> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector.tokens.iter().find_map(|token| {
        matches!(
            token.cmavo(),
            Some(Cmavo::Nahe | Cmavo::Tohe | Cmavo::Nohe | Cmavo::Jeha)
        )
        .then(|| scalar_negation_for_token(token))
    })
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

#[requires(!relation.is_empty())]
#[ensures(true)]
fn relation_place_count(dictionary: &Dictionary<'_>, relation: &str) -> Option<usize> {
    if relation_has_open_place_structure(relation) {
        return None;
    }
    if let Some(place_count) = constructed_relation_place_count(relation) {
        return Some(place_count);
    }
    dictionary_relation_place_count(dictionary, relation)
}

#[requires(!relation.is_empty())]
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
            | "involves"
            | "memberOf"
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

#[requires(!relation.is_empty())]
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
#[ensures(true)]
fn predication_mode_for_relation(relation: &str, mode: PredicationMode) -> PredicationMode {
    if mode == PredicationMode::Asserted {
        asserted_predication_mode_for_relation(relation)
    } else {
        mode
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_kind_for_nu(abstraction: &AbstractionTanruUnitSyntax) -> AbstractionKind {
    abstraction_kind_for_cmavo(abstraction.nu.value.cmavo())
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
fn abstraction_body_mode(kind: AbstractionKind) -> PredicationMode {
    if kind == AbstractionKind::Property {
        PredicationMode::Restrictive
    } else {
        PredicationMode::Inert
    }
}

#[requires(true)]
#[ensures(ret.is_none() || matches!(kind, AbstractionKind::Event | AbstractionKind::Achievement | AbstractionKind::Process | AbstractionKind::Activity | AbstractionKind::State | AbstractionKind::Experience))]
fn abstraction_eventuality_class(kind: AbstractionKind) -> Option<EventualityClass> {
    match kind {
        AbstractionKind::Event | AbstractionKind::Experience => Some(EventualityClass::Event),
        AbstractionKind::Achievement => Some(EventualityClass::Achievement),
        AbstractionKind::Process => Some(EventualityClass::Process),
        AbstractionKind::Activity => Some(EventualityClass::Activity),
        AbstractionKind::State => Some(EventualityClass::State),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_output_sort(kind: AbstractionKind) -> SemanticSort {
    kind.output_sort()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum DescriptionCharacterization {
    SpeakerDescribed,
    Veridical,
}

#[requires(true)]
#[ensures(true)]
fn description_characterization_for_cmavo(cmavo: Option<Cmavo>) -> DescriptionCharacterization {
    match cmavo {
        Some(Cmavo::Le | Cmavo::Lei | Cmavo::Lehi | Cmavo::Lehe) => {
            DescriptionCharacterization::SpeakerDescribed
        }
        _ => DescriptionCharacterization::Veridical,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn description_kind_for_cmavo(cmavo: Option<Cmavo>) -> &'static str {
    match cmavo {
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
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn name_description_kind_for_cmavo(cmavo: Option<Cmavo>) -> &'static str {
    match cmavo {
        Some(Cmavo::Lai) => "massName",
        Some(Cmavo::Lahi) => "setName",
        _ => "name",
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
#[ensures(ret.as_ref().is_ok_and(|operator| !operator.is_empty()) || ret.is_err())]
fn generated_argument_connective_operator(
    connective: &ArgumentConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        ArgumentConnectiveSyntax::EkConnective(EkConnectiveSyntax {
            na: None,
            se: None,
            a,
            nai: None,
        }) if a.value.cmavo() == Some(Cmavo::E) => Ok("joint".to_owned()),
        _ => Err(unsupported("generated argument connective")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
fn generated_relation_afterthought_connective_source(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.a.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.ja.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            Ok(generated_joik_connective_source(connective))
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            Ok(token_text(&connective.0.value))
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_joik_connective_source(connective: &JoikConnectiveSyntax) -> String {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.joi.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.bihi.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            connective_source_from_tokens(tokens)
        }
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => {
            let mut tokens = Vec::new();
            tokens.push(&connective.left_interval);
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.bihi);
            if let Some(token) = &connective.nai {
                tokens.push(token);
            }
            tokens.push(&connective.right_interval.value);
            connective_source_from_tokens(tokens)
        }
    }
}

#[requires(!tokens.is_empty())]
#[ensures(!ret.is_empty())]
fn connective_source_from_tokens(tokens: Vec<&Token>) -> String {
    tokens
        .into_iter()
        .map(token_text)
        .collect::<Vec<_>>()
        .join(" ")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|operator| matches!(operator, FormulaOperator::And | FormulaOperator::Or | FormulaOperator::Iff | FormulaOperator::WhetherOrNot)) || ret.is_err())]
fn generated_statement_connective_formula_operator(
    connective: &IStatementConnectiveSyntax,
) -> Result<FormulaOperator, SemanticsError> {
    let connective = generated_i_statement_connective_core(connective)?;
    Ok(
        match generated_statement_connective_primary_cmavo(connective) {
            Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
            Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
            Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
            Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
            _ => FormulaOperator::And,
        },
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
fn generated_statement_connective_source(
    connective: &IStatementConnectiveSyntax,
) -> Result<String, SemanticsError> {
    generated_statement_connective_core_source(generated_i_statement_connective_core(connective)?)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|table| table.is_none() || table.as_ref().is_some_and(|table| table.len() == 4)) || ret.is_err())]
fn generated_statement_connective_truth_table(
    connective: &IStatementConnectiveSyntax,
) -> Result<Option<String>, SemanticsError> {
    Ok(generated_statement_connective_core_truth_table(
        generated_i_statement_connective_core(connective)?,
    ))
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_statement_connective_core_truth_table(
    connective: &StatementConnectiveSyntax,
) -> Option<String> {
    if !generated_statement_connective_is_logical(connective) {
        return None;
    }
    let operator = generated_statement_connective_formula_operator_for_core(connective);
    let left_negated = generated_statement_connective_negates_left(connective);
    let right_negated = generated_statement_connective_negates_right(connective);
    let se = generated_statement_connective_has_se(connective);
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
#[ensures(ret.as_ref().is_ok_and(|_| true) || ret.is_err())]
fn generated_i_statement_connective_core(
    connective: &IStatementConnectiveSyntax,
) -> Result<&StatementConnectiveSyntax, SemanticsError> {
    match connective {
        IStatementConnectiveSyntax::IStandardStatementConnective(connective) => {
            if connective.tag_bo.is_some() {
                return Err(unsupported("tagged BO statement connective"));
            }
            Ok(&connective.connective)
        }
        IStatementConnectiveSyntax::ITagBoStatementConnective(_) => {
            Err(unsupported("modal statement connective"))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_formula_operator_for_core(
    connective: &StatementConnectiveSyntax,
) -> FormulaOperator {
    match generated_statement_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_primary_cmavo(
    connective: &StatementConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.a.value.cmavo(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.ja.value.cmavo(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_primary_cmavo(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            connective.0.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| !source.is_empty()) || ret.is_err())]
fn generated_statement_connective_core_source(
    connective: &StatementConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.a.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        StatementConnectiveSyntax::JekConnective(connective) => {
            let mut tokens = Vec::new();
            if let Some(token) = &connective.na {
                tokens.push(token);
            }
            if let Some(token) = &connective.se {
                tokens.push(token);
            }
            tokens.push(&connective.ja.value);
            if let Some(token) = &connective.nai {
                tokens.push(&token.value);
            }
            Ok(connective_source_from_tokens(tokens))
        }
        StatementConnectiveSyntax::JoikConnective(connective) => {
            Ok(generated_joik_connective_source(connective))
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            Ok(token_text(&connective.0.value))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_has_se(connective: &StatementConnectiveSyntax) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.se.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.se.is_some(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_has_se(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_negates_left(connective: &StatementConnectiveSyntax) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.na.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.na.is_some(),
        StatementConnectiveSyntax::JoikConnective(_) => false,
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_negates_right(connective: &StatementConnectiveSyntax) -> bool {
    match connective {
        StatementConnectiveSyntax::EkConnective(connective) => connective.nai.is_some(),
        StatementConnectiveSyntax::JekConnective(connective) => connective.nai.is_some(),
        StatementConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_negates_right(connective)
        }
        StatementConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_statement_connective_is_logical(connective: &StatementConnectiveSyntax) -> bool {
    matches!(
        generated_statement_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
        )
    )
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_afterthought_connective_formula_operator(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> FormulaOperator {
    match generated_relation_afterthought_connective_primary_cmavo(connective) {
        Some(Cmavo::A | Cmavo::Ja) => FormulaOperator::Or,
        Some(Cmavo::E | Cmavo::Je) => FormulaOperator::And,
        Some(Cmavo::O | Cmavo::Jo) => FormulaOperator::Iff,
        Some(Cmavo::U | Cmavo::Ju) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_afterthought_connective_primary_cmavo(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Option<Cmavo> {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => {
            connective.a.value.cmavo()
        }
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => {
            connective.ja.value.cmavo()
        }
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_primary_cmavo(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(connective) => {
            connective.0.value.cmavo()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_joik_connective_primary_cmavo(connective: &JoikConnectiveSyntax) -> Option<Cmavo> {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.joi.value.cmavo(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.bihi.value.cmavo(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.bihi.cmavo(),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_afterthought_connective_has_se(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.se.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.se.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_has_se(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_joik_connective_has_se(connective: &JoikConnectiveSyntax) -> bool {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.se.is_some(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.se.is_some(),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_afterthought_connective_negates_left(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.na.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.na.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(_) => false,
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_afterthought_connective_negates_right(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    match connective {
        RelationAfterthoughtConnectiveSyntax::EkConnective(connective) => connective.nai.is_some(),
        RelationAfterthoughtConnectiveSyntax::JekConnective(connective) => connective.nai.is_some(),
        RelationAfterthoughtConnectiveSyntax::JoikConnective(connective) => {
            generated_joik_connective_negates_right(connective)
        }
        RelationAfterthoughtConnectiveSyntax::VuhuNonlogicalConnective(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_joik_connective_negates_right(connective: &JoikConnectiveSyntax) -> bool {
    match connective {
        JoikConnectiveSyntax::JoiConnective(connective) => connective.nai.is_some(),
        JoikConnectiveSyntax::SimpleIntervalConnective(connective) => connective.nai.is_some(),
        JoikConnectiveSyntax::ClosedIntervalConnective(connective) => connective.nai.is_some(),
    }
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_relation_afterthought_connective_truth_table(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Option<String> {
    if !generated_relation_afterthought_connective_is_logical(connective) {
        return None;
    }
    let operator = generated_relation_afterthought_connective_formula_operator(connective);
    let left_negated = generated_relation_afterthought_connective_negates_left(connective);
    let right_negated = generated_relation_afterthought_connective_negates_right(connective);
    let se = generated_relation_afterthought_connective_has_se(connective);
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
fn generated_relation_afterthought_connective_is_logical(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    matches!(
        generated_relation_afterthought_connective_primary_cmavo(connective),
        Some(
            Cmavo::A
                | Cmavo::E
                | Cmavo::O
                | Cmavo::U
                | Cmavo::Ja
                | Cmavo::Je
                | Cmavo::Jo
                | Cmavo::Ju
        )
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|operator| !operator.is_empty()) || ret.is_err())]
fn generated_nonlogical_composition_operator(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<String, SemanticsError> {
    match generated_relation_afterthought_connective_primary_cmavo(connective) {
        Some(Cmavo::Johu) => Ok("joint".to_owned()),
        Some(Cmavo::Joi) => Ok("mass".to_owned()),
        Some(Cmavo::Ce) => Ok("set".to_owned()),
        Some(Cmavo::Ceho) => Ok("sequence".to_owned()),
        Some(Cmavo::Fahu) => Ok("respectively".to_owned()),
        Some(Cmavo::Johe) => Ok("union".to_owned()),
        Some(Cmavo::Kuha) => Ok("intersection".to_owned()),
        Some(Cmavo::Pihu) => Ok("crossProduct".to_owned()),
        Some(Cmavo::Bihi) => Ok("unorderedInterval".to_owned()),
        Some(Cmavo::Biho) => Ok("orderedInterval".to_owned()),
        Some(Cmavo::Mihi) => Ok("centeredInterval".to_owned()),
        _ => Ok(format!(
            "nonlogical:{}",
            generated_relation_afterthought_connective_source(connective)?
        )),
    }
}

#[requires(true)]
#[ensures(!ret || generated_relation_afterthought_connective_has_se(connective))]
fn generated_relation_afterthought_connective_reverses_composition_members(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> bool {
    generated_relation_afterthought_connective_has_se(connective)
        && matches!(
            generated_relation_afterthought_connective_primary_cmavo(connective),
            Some(Cmavo::Ceho | Cmavo::Fahu | Cmavo::Pihu | Cmavo::Biho | Cmavo::Mihi)
        )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_guhek_connective_source(connective: &GuhekConnectiveSyntax) -> String {
    let mut tokens = Vec::new();
    if let Some(token) = &connective.nahe {
        tokens.push(token);
    }
    if let Some(token) = &connective.se {
        tokens.push(token);
    }
    tokens.push(&connective.guha.value);
    if let Some(token) = &connective.nai {
        tokens.push(&token.value);
    }
    connective_source_from_tokens(tokens)
}

#[requires(true)]
#[ensures(true)]
fn generated_guhek_connective_formula_operator(
    connective: &GuhekConnectiveSyntax,
) -> FormulaOperator {
    match connective.guha.value.cmavo() {
        Some(Cmavo::Guha) => FormulaOperator::Or,
        Some(Cmavo::Guhe) => FormulaOperator::And,
        Some(Cmavo::Guho) => FormulaOperator::Iff,
        Some(Cmavo::Guhu) => FormulaOperator::WhetherOrNot,
        _ => FormulaOperator::And,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_guhek_connective_has_se(connective: &GuhekConnectiveSyntax) -> bool {
    connective.se.is_some()
}

#[requires(true)]
#[ensures(true)]
fn generated_guhek_connective_negates_left(connective: &GuhekConnectiveSyntax) -> bool {
    connective.nai.is_some()
}

#[requires(true)]
#[ensures(true)]
fn generated_gik_connective_negates_right(connective: &GikConnectiveSyntax) -> bool {
    connective.nai.is_some()
}

#[requires(true)]
#[ensures(ret.is_none() || ret.as_ref().is_some_and(|table| table.len() == 4))]
fn generated_guhek_gik_connective_truth_table(
    guhek: &GuhekConnectiveSyntax,
    gik: &GikConnectiveSyntax,
) -> Option<String> {
    let operator = generated_guhek_connective_formula_operator(guhek);
    let left_negated = generated_guhek_connective_negates_left(guhek);
    let right_negated = generated_gik_connective_negates_right(gik);
    let se = generated_guhek_connective_has_se(guhek);
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

#[requires(matches!(
    operator,
    FormulaOperator::And
        | FormulaOperator::Or
        | FormulaOperator::Iff
        | FormulaOperator::WhetherOrNot
))]
#[ensures(true)]
fn connective_truth_value_for_operator(operator: FormulaOperator, left: bool, right: bool) -> bool {
    match operator {
        FormulaOperator::And => left && right,
        FormulaOperator::Or => left || right,
        FormulaOperator::Iff => left == right,
        FormulaOperator::WhetherOrNot => left,
        _ => unreachable!("precondition restricts connective truth operators"),
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
        Some(Cmavo::Tuha) => SemanticSort::eventuality(),
        Some(Cmavo::Luhi) => SemanticSort::Set,
        Some(Cmavo::Luho) => SemanticSort::Mass,
        Some(Cmavo::Vuhi) => SemanticSort::Sequence,
        _ => SemanticSort::Entity,
    }
}

#[requires(!tokens.is_empty())]
#[ensures(true)]
fn simple_pa_integer_from_tokens(tokens: &[Token]) -> Option<i64> {
    let mut value = 0i64;
    for token in tokens {
        value = value.checked_mul(10)?;
        value = value.checked_add(pa_digit_value(token.cmavo()?)?)?;
    }
    Some(value)
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn parse_generated_relational_pa_integer(text: &str) -> Option<i64> {
    let (prefix, rest) = text.split_once(char::is_whitespace)?;
    if !matches!(prefix, "su'o" | "su'e" | "za'u" | "me'i" | "su'a") {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    parse_generated_simple_pa_integer(rest)
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn parse_generated_simple_pa_integer(text: &str) -> Option<i64> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    let (sign, first_digit) = match first {
        "ni'u" => (-1_i64, words.next()?),
        "ma'u" => (1_i64, words.next()?),
        _ => (1_i64, first),
    };
    let mut value = i64::from(pa_digit_value_for_text(first_digit)?);
    for word in words {
        let digit = i64::from(pa_digit_value_for_text(word)?);
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some(sign * value)
}

#[requires(!word.is_empty())]
#[ensures(ret.is_none_or(|digit| digit <= 9))]
fn pa_digit_value_for_text(word: &str) -> Option<u8> {
    match word {
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
#[ensures(ret.is_none_or(|digit| (0..=9).contains(&digit)))]
fn pa_digit_value(cmavo: Cmavo) -> Option<i64> {
    match cmavo {
        Cmavo::No => Some(0),
        Cmavo::Pa => Some(1),
        Cmavo::Re => Some(2),
        Cmavo::Ci => Some(3),
        Cmavo::Vo => Some(4),
        Cmavo::Mu => Some(5),
        Cmavo::Xa => Some(6),
        Cmavo::Ze => Some(7),
        Cmavo::Bi => Some(8),
        Cmavo::So => Some(9),
        _ => None,
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn quantity_form_for_text(text: &str) -> QuantityForm {
    match text {
        "ro" => QuantityForm::All,
        text if text.starts_with("su'o") => QuantityForm::AtLeast,
        text if text.starts_with("su'e") => QuantityForm::AtMost,
        text if text.starts_with("za'u") => QuantityForm::MoreThan,
        text if text.starts_with("me'i") => QuantityForm::LessThan,
        text if text.starts_with("ji'i") => QuantityForm::Approximate,
        "so'a" => QuantityForm::TooFew,
        "so'e" => QuantityForm::Enough,
        "so'i" | "so'o" | "so'u" => QuantityForm::Indefinite,
        "du'e" => QuantityForm::TooMany,
        _ => QuantityForm::Exact,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_quantifier_formula_operator(quantifier: &QuantifierSyntax) -> FormulaOperator {
    match token_list_text(quantifier_tokens(quantifier).iter()).as_str() {
        "ro" => FormulaOperator::Forall,
        "no" => FormulaOperator::None,
        _ => FormulaOperator::Cardinality,
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_tokens(quantifier: &QuantifierSyntax) -> Vec<Token> {
    let mut visitor = GeneratedSpanCollector::default();
    quantifier.visit_in_order(&mut visitor);
    visitor.tokens
}

#[requires(true)]
#[ensures(source_text.is_none() -> ret.as_slice() == spans)]
fn source_spans_with_following_cmevla_period(
    spans: &[SourceSpan],
    source_text: Option<&str>,
) -> Vec<SourceSpan> {
    let Some(source_text) = source_text else {
        return spans.to_vec();
    };
    let Some((last_index, last_span)) = spans
        .iter()
        .enumerate()
        .max_by_key(|(_, span)| span.byte_end)
    else {
        return Vec::new();
    };
    let Some(period) = source_text
        .get(last_span.byte_end..)
        .and_then(|tail| tail.chars().next())
        .filter(|period| is_lojban_period(*period))
    else {
        return spans.to_vec();
    };
    let Ok(expanded) = SourceSpan::new(
        last_span.source_id.clone(),
        last_span.byte_start,
        last_span.byte_end + period.len_utf8(),
        last_span.char_start,
        last_span.char_end + 1,
    ) else {
        return spans.to_vec();
    };
    let mut expanded_spans = spans.to_vec();
    expanded_spans[last_index] = expanded;
    expanded_spans
}

#[requires(true)]
#[ensures(ret == matches!(value, '.' | 'ӏ' | 'Ӏ' | '\u{ed89}'))]
fn is_lojban_period(value: char) -> bool {
    matches!(value, '.' | 'ӏ' | 'Ӏ' | '\u{ed89}')
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn token_list_text<'a>(tokens: impl Iterator<Item = &'a Token>) -> String {
    tokens.map(token_text).collect::<Vec<_>>().join(" ")
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|definition| crate::model::argument_object_kind_can_fill(definition.value.object_kind())))]
fn scalar_scale_definition_for_modal_argument(
    modal_argument: &ModalArgument,
) -> Option<GeneratedScalarScaleDefinition> {
    modal_argument.relation.as_ref()?;
    if modal_argument.introduced_by != "ci'u" {
        return None;
    }
    let value = modal_argument.arguments.get("x1")?.value?;
    Some(GeneratedScalarScaleDefinition::from_data(data!(
        GeneratedScalarScaleDefinition {
            value,
            introduced_by: modal_argument.introduced_by.clone(),
            source: modal_argument.source.clone(),
        }
    )))
}

#[requires(true)]
#[ensures(ret.construct.as_deref() == Some("scalar-scale"))]
fn source_as_scalar_scale(source: crate::model::SemanticSource) -> crate::model::SemanticSource {
    crate::model::SemanticSource {
        construct: Some("scalar-scale".to_owned()),
        ..source
    }
}

#[requires(!construct.is_empty())]
#[ensures(ret.as_ref().is_none_or(|source| source.construct.as_deref() == Some(construct)))]
fn source_with_construct(
    source: Option<crate::model::SemanticSource>,
    construct: &str,
) -> Option<crate::model::SemanticSource> {
    source.map(|source| crate::model::SemanticSource {
        construct: Some(construct.to_owned()),
        ..source
    })
}

#[requires(true)]
#[ensures(!ret.introduced_by.is_empty())]
fn scalar_negation_for_marker<F>(marker: &WithFreeModifiers<Token, F>) -> ScalarNegation {
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
fn descriptor_definiteness_for_scalar_negated_sumti(
    cmavo: Option<Cmavo>,
) -> Option<DescriptorDefiniteness> {
    match cmavo {
        Some(Cmavo::Tohe) => Some(DescriptorDefiniteness::UniqueExtreme),
        Some(Cmavo::Nohe) => Some(DescriptorDefiniteness::NeutralPoint),
        Some(Cmavo::Jeha) => Some(DescriptorDefiniteness::AffirmedPoint),
        _ => Some(DescriptorDefiniteness::IndefiniteAlternative),
    }
}

#[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(true)]
fn bind_generated_modal_argument_to_host_event(
    modal_argument: &mut ModalArgument,
    eventuality: SemanticObjectId,
) {
    if modal_argument.relation.is_none() {
        return;
    }
    let Some(place) = generated_modal_relation_host_event_place_for_argument(modal_argument) else {
        return;
    };
    let key = argument_key(place);
    if modal_argument
        .arguments
        .get(&key)
        .is_some_and(|argument| argument.kind != ArgumentValueKind::Elided)
    {
        return;
    }
    let mut data = modal_argument.clone().into_data();
    data.arguments
        .insert(key, ArgumentValue::filled(eventuality, None));
    *modal_argument = ModalArgument::from_data(data);
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
fn generated_modal_relation_host_event_place_for_argument(
    modal_argument: &ModalArgument,
) -> Option<usize> {
    let relation = modal_argument.relation.as_deref()?;
    if generated_modal_relation_has_complementary_event_places(relation)
        && generated_modal_argument_place_is_filled(modal_argument, 2)
        && !generated_modal_argument_place_is_filled(modal_argument, 1)
    {
        return Some(1);
    }
    generated_modal_relation_host_event_place(relation)
}

#[requires(place > 0)]
#[ensures(true)]
fn generated_modal_argument_place_is_filled(modal_argument: &ModalArgument, place: usize) -> bool {
    modal_argument
        .arguments
        .get(&argument_key(place))
        .is_some_and(|argument| argument.kind != ArgumentValueKind::Elided)
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_relation_has_complementary_event_places(relation: &str) -> bool {
    matches!(relation, "krinu" | "mukti" | "nibli" | "rinka")
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| place > 0))]
fn generated_modal_relation_host_event_place(relation: &str) -> Option<usize> {
    match relation {
        "bapli" | "gasnu" | "krinu" | "mukti" | "nibli" | "rinka" => Some(2),
        "pilno" => Some(3),
        _ => None,
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
fn invalid_graph(message: String) -> SemanticsError {
    SemanticsError {
        kind: SemanticsErrorKind::InvalidGraph,
        message: format!("semantic graph invariant failed: {message}"),
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

#[requires(place > 0)]
#[ensures(!ret.is_empty())]
fn argument_key(place: usize) -> String {
    format!("x{place}")
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

#[requires(!what.is_empty())]
#[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
fn unsupported(what: &str) -> SemanticsError {
    SemanticsError {
        kind: SemanticsErrorKind::InvalidGraph,
        message: format!("generated semantic builder does not yet support {what}"),
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::{
        ParseOptions, parse_syntax_tree_generated_model_with_source_and_options,
        parse_syntax_tree_with_source_and_options,
    };

    use super::*;
    use crate::builder::build_semantic_graph_with_dictionary_and_options;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_simple_bridi() {
        assert_generated_builder_matches_legacy("mi tavla do");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_description_sumti() {
        assert_generated_builder_matches_legacy("mi klama le zarci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_explicit_cu() {
        assert_generated_builder_matches_legacy("mi cu klama le zarci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_demonstrative_sumti() {
        assert_generated_builder_matches_legacy("ta bloti");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_unknown_place_structure() {
        assert_generated_builder_matches_legacy("ta blotrskunri");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_identity_goha() {
        assert_generated_builder_matches_legacy("do du la .djan.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_sumti_selbri_name() {
        assert_generated_builder_matches_legacy("do me la .djan.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_quantified_sumti_selbri_description() {
        assert_generated_builder_matches_legacy("la .BALtazar. cu me le ci nolraitru");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_no_gadri_me_sumti_distribution() {
        assert_generated_builder_matches_legacy("re me le ci nolraitru me'u .e la .djan. cu blabi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_linkargs_with_conversion_in_description() {
        assert_generated_builder_matches_legacy("ta me la'e le se cusku be do me'u cukta");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_simple_conversion() {
        assert_generated_builder_matches_legacy("do se prami mi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_grouped_tanru_conversion() {
        assert_generated_builder_matches_legacy("le zarci cu se ke cadzu klama ke'e la .alis.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_scalar_negated_tanru_modifier() {
        assert_generated_builder_matches_legacy("la .alis. cu na'e cadzu klama le zarci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_scalar_negated_ordinal_tanru() {
        assert_generated_builder_matches_legacy("la .djonz. cu na'e pamoi cusku");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_name_sumti() {
        assert_generated_builder_matches_legacy("la .djan. cu klama le zarci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_bare_event_abstraction() {
        assert_generated_builder_matches_legacy("nu mi klama le zarci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_abstraction_tanru() {
        assert_generated_builder_matches_legacy("la .djan. cu nu sonci kei djica");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_event_abstraction_description() {
        assert_generated_builder_matches_legacy("la .djan. cu djica le nu sonci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_converted_property_abstraction_description() {
        assert_generated_builder_matches_legacy("la .djan. cu ckaji le ka se risna");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_builder_matches_legacy_for_converted_proposition_description() {
        assert_generated_builder_matches_legacy(
            "la .djan. cusku le se du'u la .djordj. klama le zarci",
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_generated_builder_matches_legacy(source: &str) {
        let dictionary = jbotci_dictionary_data::english();
        let old_syntax = legacy_syntax(source);
        let generated_syntax = generated_syntax(source);
        let options = SemanticBuildOptions {
            source_text: Some(source),
            story_time: false,
        };
        let old_graph = build_semantic_graph_with_dictionary_and_options(
            &old_syntax.parse_tree,
            options,
            dictionary,
        )
        .expect("legacy graph");
        let generated_graph = build_generated_semantic_graph_with_dictionary_and_options(
            &generated_syntax,
            options,
            dictionary,
        )
        .expect("generated graph");
        assert_eq!(
            graph_json(&generated_graph),
            graph_json(&old_graph),
            "generated semantic graph must match the handwritten AST builder"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn legacy_syntax(source: &str) -> jbotci_syntax::SyntaxParse {
        let words = segment_words_with_modifiers(source).expect("morphology");
        parse_syntax_tree_with_source_and_options(&words, source, &ParseOptions::default())
            .expect("legacy syntax")
    }

    #[requires(true)]
    #[ensures(true)]
    fn generated_syntax(source: &str) -> Box<TextSyntax> {
        let words = segment_words_with_modifiers(source).expect("morphology");
        parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
        .expect("generated syntax")
    }

    #[requires(true)]
    #[ensures(true)]
    fn graph_json(graph: &SemanticGraph) -> serde_json::Value {
        serde_json::from_str(&graph.to_json_string(0).expect("semantic JSON")).expect("valid JSON")
    }
}
