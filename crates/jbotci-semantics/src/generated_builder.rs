//! Semantic builder that consumes the generated syntax model directly.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dictionary::Dictionary;
use jbotci_morphology::{Cmavo, Word, strip_diacritics};
use jbotci_source::SourceSpan;
use jbotci_syntax::generated_model::{
    AbstractionTanruUnitSyntax, ArgumentConnectiveSyntax, AtomRef as GeneratedAtomRef,
    BoGroupedBridiTailSyntax, BoOrLinkedTanruUnitSyntax, BoundTanruUnitSyntax,
    BridiStatementSyntax, BridiSubbridiSyntax, BridiSyntax, BridiTailSyntax,
    BridiTailWithPossibleTailTermsSyntax, BridiWithLeadingTermsSyntax, CoSelbriSyntax,
    ConnectedSelbriSyntax, ConnectedTermSyntax, DescriptionTailBodySyntax,
    DescriptorWithGadriSumtiSyntax, DescriptorWithoutGadriSumtiSyntax, EkConnectiveSyntax,
    ForethoughtSelbriConnectionSyntax, ForethoughtSelbriGroupTanruUnitSyntax,
    FragmentStatementSyntax, FreeModifierSyntax, GikConnectiveSyntax, GohaWordTanruUnitSyntax,
    GroupedTanruUnitSyntax, GuhekConnectiveSyntax, JoikConnectiveSyntax, LaheSumtiSyntax,
    LinkargsSyntax, LinkedSumtiSyntax, LinkedTanruUnitSyntax, NameSumtiSyntax,
    OrdinalTanruUnitSyntax, ParagraphSyntax, ProBridiTanruUnitSyntax, ProSumtiSyntax,
    QuantifierRelationDescriptionTailSyntax, QuantifierSyntax, RegularTextSyntax,
    RelationAfterthoughtConnectiveSyntax, RelationDescriptionTailSyntax, RelationOnlyBridiSyntax,
    ScalarNegatedTanruInnerUnitSyntax, ScalarNegatedTanruUnitSyntax, SelbriSimpleBridiTailSyntax,
    SelbriSyntax, SimpleBridiTailSyntax, SimpleParagraphSyntax, SimpleSumtiSyntax,
    SimpleTermSyntax, StatementBaseSyntax, StatementOrFragmentStatementSyntax,
    StatementOrFragmentSyntax, StatementSyntax, SubbridiSyntax, SumtiAfterthoughtSyntax,
    SumtiAtomSyntax, SumtiBaseSyntax, SumtiBoundSyntax, SumtiForethoughtSyntax, SumtiGroupedSyntax,
    SumtiSelbriSumtiSyntax, SumtiSelbriTanruUnitSyntax, SumtiSyntax, SumtiTermSyntax,
    TaggedOrElidedSumtiSyntax, TanruSelbriSyntax, TanruUnitAtomBaseSyntax, TanruUnitAtomSyntax,
    TanruUnitSyntax, TenseModalSyntax, TermSyntax, TermsFragmentSyntax,
    TextParagraphWithAdditionalNihoSyntax, TextParagraphsSyntax, TextSyntax, TreeNode,
    UntaggedSelbriSyntax, WordTanruUnitSyntax,
};
use jbotci_syntax::tree::{Token, WithFreeModifiers};
use jbotci_tree::TreeVisitor;

use crate::builder::{
    SemanticBuildOptions, SemanticsError, SemanticsErrorKind, dictionary_relation_place_count,
};
use crate::model::{
    AbstractionKind, Actuality, ActualityKind, AnchorRelation, ArgumentValue, Composition,
    Connector, Descriptor, EventualityClass, EventualitySort, FormulaOperator, IndexicalKind,
    ModalArgument, ParameterRole, PredicationMode, QuantityForm, QuantityScale, QuantityValue,
    ReferentCategory, ScalarNegation, ScalarNegationKind, SemanticGraph, SemanticObject,
    SemanticObjectId, SemanticOperatorData, SemanticSort, TanruLink, UtteranceForce, diagnostic,
    source_from_spans,
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
#[derive(Debug, Clone, Copy)]
enum GeneratedTextRoot<'syntax> {
    Bridi(&'syntax BridiSyntax),
    TermsFragment(&'syntax TermsFragmentSyntax),
}

#[invariant(crate::model::argument_object_kind_can_fill(value.object_kind()))]
#[derive(Debug, Clone)]
struct GeneratedScalarScaleDefinition {
    value: SemanticObjectId,
    introduced_by: String,
    source: Option<crate::model::SemanticSource>,
}

#[invariant(::Description => true)]
#[invariant(::PropertyAbstraction => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedPropertyTanruContext {
    Description,
    PropertyAbstraction,
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
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_text(mut self, syntax: &TextSyntax) -> Result<SemanticGraph, SemanticsError> {
        let root = single_semantic_root_from_text(syntax)?;
        let utterance_id = self.next_utterance_id();
        let (force, content, source) = match root {
            GeneratedTextRoot::Bridi(bridi) => (
                UtteranceForce::Assert,
                Some(self.build_bridi_formula(bridi)?),
                self.source_for_node(bridi, "bridi"),
            ),
            GeneratedTextRoot::TermsFragment(fragment) => {
                let referent = self.build_terms_fragment_referent(fragment)?;
                (
                    UtteranceForce::Mention,
                    Some(referent),
                    self.source_for_node(fragment, "fragment"),
                )
            }
        };
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
                SemanticObjectId::speaker(),
                SemanticObjectId::addressee(),
                SemanticObjectId::now(),
                SemanticObjectId::here(),
                source,
                Vec::new(),
            ),
        )?;
        SemanticGraph::new(utterance_id, self.objects).map_err(|message| SemanticsError {
            kind: SemanticsErrorKind::InvalidGraph,
            message: format!("semantic graph invariant failed: {message}"),
        })
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

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_relation_only_bridi_formula_with_options(
        &mut self,
        bridi: &RelationOnlyBridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let simple_tail = simple_tail_from_bridi_tail(&bridi.0)?;
        let abstraction = if simple_tail.terms.is_empty() && eventuality.is_none() {
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
                Vec::new(),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(&simple_tail.selbri)? {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped sumti selbri"));
            }
            return self.build_sumti_selbri_formula_for_terms(
                sumti_selbri,
                Vec::new(),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && tanru.additional_units.is_empty()
            && generated_tanru_unit_is_grouped(&tanru.first_unit)?
        {
            return self.build_relation_formula_for_generated_tanru_unit_terms(
                &tanru.first_unit,
                Vec::new(),
                eventuality,
                mode,
                self.source_for_node(bridi, "tanru-formula"),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        self.build_simple_tail_formula_with_options(
            simple_tail,
            Vec::new(),
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
                eventuality,
                mode,
                self.source_for_node(bridi, "tanru-formula"),
                self.source_for_node(bridi, "tanru-formula"),
            );
        }
        self.build_simple_tail_formula_with_options(
            simple_tail,
            terms,
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
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_formula_with_options(
            &simple_tail.selbri,
            terms,
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
                let mut visible_arguments = self.build_visible_arguments_for_terms(terms)?;
                if !visible_arguments.contains_key(&1) {
                    let referent = self.build_elided_referent("zo'e".to_owned())?;
                    insert_visible_argument(
                        &mut visible_arguments,
                        1,
                        ArgumentValue::elided(referent, "zo'e".to_owned(), None),
                    )?;
                }
                self.build_forethought_selbri_connection_formula_for_visible_arguments(
                    connection,
                    visible_arguments,
                    source_with_construct(
                        formula_source.or(predication_source),
                        "connected-selbri-formula",
                    ),
                    "selbri",
                    None,
                )
                .map(|result| result.formula)
            }
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_co_selbri_formula_with_options(
        &mut self,
        selbri: &CoSelbriSyntax,
        terms: Vec<&TermSyntax>,
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
                formula_scope_child,
                formula_source,
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
        let mut arguments = BTreeMap::new();
        let mut visible_place = 1usize;
        for term in terms {
            let referent = self.build_term_referent(term)?;
            arguments.insert(
                argument_key(visible_place),
                ArgumentValue::filled(referent, None),
            );
            visible_place += 1;
        }
        for place in visible_place..=place_limit {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
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
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
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
        let Some(relation) = generated_time_relation_for_tense_modal(tense_modal) else {
            return Ok(None);
        };
        let eventuality = self.next_eventuality_id();
        let mut object = SemanticObject::eventuality(EventualityClass::Event, None, source);
        object.time = Some(new!(AnchorRelation {
            relation: relation,
            anchor: SemanticObjectId::now(),
            sticky: false,
            inherited: None,
            distance: None,
            magnitude: None,
            scalar_negation: None,
            motion: None,
        }));
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
            let visible_arguments = self.build_visible_arguments_for_terms(terms)?;
            return self
                .build_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    formula_source,
                    "selbri",
                    leading_eventuality,
                )
                .map(|result| result.formula);
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
            let visible_arguments = self.build_visible_arguments_for_terms(terms)?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
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
            self.apply_scalar_negation_to_tanru_links(
                formula,
                scalar_negation_for_marker(&scalar_unit.nahe)
                    .with_argument_scope(vec!["x1".to_owned()]),
            )?;
            return Ok(self
                .detach_tanru_relation_formula_without_positive_head(formula)
                .unwrap_or(formula));
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = atom.base.as_ref() {
            if eventuality.is_some() || mode != PredicationMode::Asserted {
                return Err(unsupported("scoped grouped tanru unit"));
            }
            let visible_arguments = self.build_visible_arguments_for_terms(terms)?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments,
                &atom.conversions,
            )?;
            return self.build_tanru_formula_for_connected_selbri_with_visible_arguments(
                &grouped.selbri,
                visible_arguments,
                formula_source,
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
                    SemanticObject::eventuality(
                        EventualityClass::Event,
                        None,
                        predication_source.clone(),
                    ),
                )?;
                eventuality
            }
        };
        let mut visible_arguments = self.build_visible_arguments_for_terms(terms)?;
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
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(&description.selbri)? {
            return self.build_sumti_selbri_formula_for_argument(
                sumti_selbri,
                ArgumentValue::filled(variable, None),
                PredicationMode::Restrictive,
                self.source_for_node(&description.selbri, "restrictive-predication"),
            );
        }
        let relation = semantic_relation_label(relation_label_from_selbri(&description.selbri)?);
        self.build_relation_formula_for_argument(
            relation,
            ArgumentValue::filled(variable, None),
            None,
            PredicationMode::Restrictive,
            self.source_for_node(&description.selbri, "restrictive-predication"),
            self.source_for_node(&description.selbri, "restrictive-predication"),
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_terms(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_tanru_formula_for_terms_with_head_eventuality_order(tanru, terms, false, source)
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_terms_with_head_eventuality_order(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
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
        let visible_arguments = self.build_visible_arguments_for_terms(terms)?;
        self.build_tanru_formula_for_visible_arguments_with_head_eventuality(
            tanru,
            visible_arguments,
            head_eventuality,
            source,
        )
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
        let Some((trailing_unit, modifier_units)) = tanru.additional_units.split_last() else {
            return Err(unsupported("empty tanru continuation"));
        };
        let head = self.build_tanru_head_relation_formula(
            trailing_unit,
            visible_arguments,
            head_eventuality,
            source.clone(),
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
        if !unit.0.links.is_empty() {
            if eventuality.is_some() {
                return Err(unsupported(
                    "preallocated connected tanru unit head eventuality",
                ));
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
                ),
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
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
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        if let Some(scalar_unit) = scalar_unit
            && let Some((grouped, inner_conversions)) =
                scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
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
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                PredicationMode::Asserted,
                source.clone(),
                diagnostics,
            ),
        )?;
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
    #[ensures(ret.as_ref().is_ok_and(|arguments| arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    fn build_visible_arguments_for_terms(
        &mut self,
        terms: Vec<&TermSyntax>,
    ) -> Result<BTreeMap<usize, ArgumentValue>, SemanticsError> {
        let mut arguments = BTreeMap::new();
        let mut next_visible_place = 1usize;
        for term in terms {
            self.insert_visible_argument_for_generated_term(
                &mut arguments,
                &mut next_visible_place,
                term,
            )?;
        }
        Ok(arguments)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_sumti_selbri_formula_for_terms(
        &mut self,
        sumti_selbri: &SumtiSelbriTanruUnitSyntax,
        terms: Vec<&TermSyntax>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti_selbri.moi_marker.is_some() {
            return Err(unsupported("MOI sumti selbri"));
        }
        let mut arguments = BTreeMap::new();
        let mut visible_place = 1usize;
        for term in terms {
            let referent = self.build_term_referent(term)?;
            arguments.insert(
                argument_key(visible_place),
                ArgumentValue::filled(referent, None),
            );
            visible_place += 1;
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
        self.insert(
            predication,
            SemanticObject::predication(
                "referentOf".to_owned(),
                Some(eventuality),
                arguments,
                PredicationMode::Asserted,
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
                let argument = ArgumentValue::filled(self.build_sumti_referent(&sumti.0)?, None);
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
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => Ok(ArgumentValue::filled(
                self.build_sumti_referent(sumti)?,
                None,
            )),
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
                    speaker: Some(SemanticObjectId::speaker()),
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
        let argument = self.build_argument_for_generated_term(term)?;
        argument
            .value
            .ok_or_else(|| unsupported("non-referential term argument"))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn insert_visible_argument_for_generated_term(
        &mut self,
        arguments: &mut BTreeMap<usize, ArgumentValue>,
        next_visible_place: &mut usize,
        term: &TermSyntax,
    ) -> Result<(), SemanticsError> {
        let (place, argument) =
            self.build_visible_place_and_argument_for_generated_term(*next_visible_place, term)?;
        insert_visible_argument(arguments, place, argument)?;
        *next_visible_place = (*next_visible_place).max(place + 1);
        Ok(())
    }

    #[requires(next_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|(place, _)| *place > 0) || ret.is_err())]
    fn build_visible_place_and_argument_for_generated_term(
        &mut self,
        next_visible_place: usize,
        term: &TermSyntax,
    ) -> Result<(usize, ArgumentValue), SemanticsError> {
        let simple = match term {
            TermSyntax::SimpleTerm(simple) => simple,
            TermSyntax::ConnectedTerm(ConnectedTermSyntax {
                leading_term,
                continuations,
            }) if continuations.is_empty() => leading_term.as_ref(),
            _ => return Err(unsupported("non-simple term")),
        };
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => Ok((
                next_visible_place,
                ArgumentValue::filled(self.build_sumti_referent(sumti)?, None),
            )),
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => Ok((
                fa_place(&term.fa.value)?,
                self.build_tagged_or_elided_sumti_argument(&term.sumti)?,
            )),
            _ => Err(unsupported("non-sumti term")),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_argument_for_generated_term(
        &mut self,
        term: &TermSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        let (_place, argument) =
            self.build_visible_place_and_argument_for_generated_term(1, term)?;
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_simple_sumti_referent(
        &mut self,
        sumti: &SimpleSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti.relative_clauses.is_some() {
            return Err(unsupported("relative clauses"));
        }
        match sumti.base_sumti.as_ref() {
            SumtiAtomSyntax::SumtiBase(base) => self.build_sumti_base_referent(base),
            SumtiAtomSyntax::QuantifiedSumti(_) => Err(unsupported("quantified sumti")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_sumti_base_referent(
        &mut self,
        sumti: &SumtiBaseSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti {
            SumtiBaseSyntax::ProSumti(pro_sumti) => self.build_pro_sumti_referent(pro_sumti),
            SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
                self.build_description_referent(description)
            }
            SumtiBaseSyntax::DescriptorWithoutGadriSumti(description) => {
                self.build_no_gadri_description_referent(description)
            }
            SumtiBaseSyntax::NameSumti(name) => self.build_name_sumti_referent(name),
            SumtiBaseSyntax::LaheSumti(sumti) => self.build_lahe_sumti_referent(sumti),
            _ => Err(unsupported("sumti base")),
        }
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
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                referent_qualifier_sort(sumti.lahe.value.cmavo()),
                None,
                Some(Descriptor {
                    kind: referent_qualifier_kind(sumti.lahe.value.cmavo()).to_owned(),
                    word: token_text(&sumti.lahe.value),
                    speaker: Some(SemanticObjectId::speaker()),
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
            Some(Cmavo::Mi) => Ok(SemanticObjectId::speaker()),
            Some(Cmavo::Do) => Ok(SemanticObjectId::addressee()),
            Some(Cmavo::Ko) => Ok(SemanticObjectId::addressee()),
            Some(Cmavo::Ti) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::ProximalDemonstrative)
            }
            Some(Cmavo::Ta) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::MedialDemonstrative)
            }
            Some(Cmavo::Tu) => {
                self.build_demonstrative_referent(pro_sumti, IndexicalKind::DistalDemonstrative)
            }
            _ => Err(unsupported(&format!("pro-sumti {word}"))),
        }
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
                    speaker: Some(SemanticObjectId::speaker()),
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
        if description.tail.leading_tail_elements.tail_sumti.is_some()
            || description
                .tail
                .leading_tail_elements
                .relative_clauses
                .is_some()
        {
            return Err(unsupported("description leading tail elements"));
        }
        let (selbri, relative_clauses, quantity) = match description.tail.tail.as_ref() {
            DescriptionTailBodySyntax::RelationDescriptionTail(RelationDescriptionTailSyntax {
                selbri,
                relative_clauses,
            }) => (selbri.as_ref(), relative_clauses.as_ref(), None),
            DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(
                QuantifierRelationDescriptionTailSyntax {
                    quantifier,
                    selbri,
                    relative_clauses,
                },
            ) => (selbri.as_ref(), relative_clauses.as_ref(), Some(quantifier)),
            _ => return Err(unsupported("non-relation description tail")),
        };
        if relative_clauses.is_some() {
            return Err(unsupported("description relative clauses"));
        }
        let cmavo = description.description.0.value.cmavo();
        let word = token_text(&description.description.0.value);
        let kind = description_kind_for_cmavo(cmavo).to_owned();
        let abstraction = self.single_abstraction_from_selbri(selbri)?.cloned();
        if let Some(abstraction) = abstraction {
            return self.build_abstraction_description_output(
                description,
                &abstraction,
                kind,
                word,
            );
        }
        let id = self.next_referent_id();
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
                self.source_for_node(description, "description"),
                Vec::new(),
            ),
        )?;
        let body = match description_characterization_for_cmavo(cmavo) {
            DescriptionCharacterization::SpeakerDescribed => {
                self.build_speaker_description_formula(description, selbri, id)?
            }
            DescriptionCharacterization::Veridical => self.build_restrictive_formula(selbri, id)?,
        };
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
        descriptor.body = Some(body);
        descriptor.quantity = quantity;
        Ok(id)
    }

    #[requires(!kind.is_empty())]
    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_abstraction_description_output(
        &mut self,
        description: &DescriptorWithGadriSumtiSyntax,
        abstraction: &AbstractionTanruUnitSyntax,
        kind: String,
        word: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let cmavo = description.description.0.value.cmavo();
        let source = self.source_for_node(description, "description");
        let id = self.build_abstraction_output(abstraction, source.clone())?;
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction description output {id}"
            ))
        })?;
        object.descriptor = Some(Descriptor {
            kind,
            word,
            speaker: Some(SemanticObjectId::speaker()),
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_speaker_description_formula(
        &mut self,
        description: &DescriptorWithGadriSumtiSyntax,
        selbri: &SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let property = self.build_description_property_abstraction_for_selbri(selbri)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(
            "x1".to_owned(),
            ArgumentValue::filled(SemanticObjectId::speaker(), None),
        );
        arguments.insert("x2".to_owned(), ArgumentValue::filled(referent, None));
        arguments.insert(
            "x3".to_owned(),
            ArgumentValue::filled(SemanticObjectId::addressee(), None),
        );
        arguments.insert("x4".to_owned(), ArgumentValue::filled(property, None));
        self.build_structural_formula_from_arguments(
            "skicu",
            arguments,
            PredicationMode::Incidental,
            self.source_for_node(description, "speaker-description"),
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
        let value = simple_pa_integer_from_tokens(&words)
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
        mut arguments: BTreeMap<String, ArgumentValue>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
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
        let Some(class) = abstraction_eventuality_class(kind) else {
            return Err(unsupported("non-event abstraction"));
        };
        let id = self.next_referent_with_sort_id(sort);
        let body = self.build_subbridi_formula_with_eventuality(
            &abstraction.subbridi,
            id,
            abstraction_body_mode(kind),
        )?;
        let mut object = SemanticObject::eventuality(class, None, source);
        object.sort = Some(sort);
        object.content = Some(body);
        object.abstraction_kind = Some(kind);
        self.insert(id, object)?;
        Ok(id)
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

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent(&mut self, label: String) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort(label, SemanticSort::Entity)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    fn build_elided_referent_with_sort(
        &mut self,
        label: String,
        sort: SemanticSort,
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
                None,
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
#[ensures(ret.is_ok() || ret.is_err())]
fn single_semantic_root_from_text(
    syntax: &TextSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
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
        || !leading_i_statements.is_empty()
    {
        return Err(unsupported("text leading material"));
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
    let ParagraphSyntax::SimpleParagraph(SimpleParagraphSyntax(sequence)) = first else {
        return Err(unsupported("NIhO paragraph"));
    };
    if !sequence.following.is_empty() || !sequence.trailing.is_empty() {
        return Err(unsupported("paragraph statement continuations"));
    }
    match sequence.initial.0.as_ref() {
        StatementOrFragmentSyntax::StatementOrFragmentStatement(
            StatementOrFragmentStatementSyntax(statement),
        ) => {
            let StatementSyntax::StatementBase(StatementBaseSyntax::BridiStatement(
                BridiStatementSyntax {
                    bridi,
                    continuations,
                },
            )) = statement
            else {
                return Err(unsupported("non-simple statement"));
            };
            if !continuations.is_empty() {
                return Err(unsupported("statement connective continuations"));
            }
            Ok(GeneratedTextRoot::Bridi(bridi))
        }
        StatementOrFragmentSyntax::FragmentStatement(FragmentStatementSyntax::TermsFragment(
            fragment,
        )) => Ok(GeneratedTextRoot::TermsFragment(fragment)),
        StatementOrFragmentSyntax::FragmentStatement(_) => Err(unsupported("non-terms fragment")),
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
    if simple_tail.vau.is_some() {
        return Err(unsupported("explicit vau in generated semantic checkpoint"));
    }
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
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector.tokens.iter().find_map(time_relation_for_pu_token)
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
