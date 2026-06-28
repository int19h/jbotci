//! Semantic builder that consumes the generated syntax model directly.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_dictionary::Dictionary;
use jbotci_morphology::{Cmavo, Word, strip_diacritics};
use jbotci_source::SourceSpan;
use jbotci_syntax::generated_model::{
    AbstractionTanruUnitSyntax, AtomRef as GeneratedAtomRef, BoGroupedBridiTailSyntax,
    BoOrLinkedTanruUnitSyntax, BridiStatementSyntax, BridiSubbridiSyntax, BridiSyntax,
    BridiTailSyntax, BridiTailWithPossibleTailTermsSyntax, BridiWithLeadingTermsSyntax,
    CoSelbriSyntax, ConnectedSelbriSyntax, ConnectedTermSyntax, DescriptionTailBodySyntax,
    DescriptorWithGadriSumtiSyntax, NameSumtiSyntax, ParagraphSyntax, ProSumtiSyntax,
    RegularTextSyntax, RelationDescriptionTailSyntax, RelationOnlyBridiSyntax,
    SelbriSimpleBridiTailSyntax, SelbriSyntax, SimpleBridiTailSyntax, SimpleParagraphSyntax,
    SimpleSumtiSyntax, SimpleTermSyntax, StatementBaseSyntax, StatementOrFragmentStatementSyntax,
    StatementOrFragmentSyntax, StatementSyntax, SubbridiSyntax, SumtiAfterthoughtSyntax,
    SumtiAtomSyntax, SumtiBaseSyntax, SumtiBoundSyntax, SumtiForethoughtSyntax, SumtiGroupedSyntax,
    SumtiSyntax, SumtiTermSyntax, TanruSelbriSyntax, TanruUnitAtomBaseSyntax, TanruUnitAtomSyntax,
    TanruUnitSyntax, TermSyntax, TextParagraphWithAdditionalNihoSyntax, TextParagraphsSyntax,
    TextSyntax, TreeNode, UntaggedSelbriSyntax, WordTanruUnitSyntax,
};
use jbotci_syntax::tree::Token;
use jbotci_tree::TreeVisitor;

use crate::builder::{
    SemanticBuildOptions, SemanticsError, SemanticsErrorKind, dictionary_relation_place_count,
};
use crate::model::{
    AbstractionKind, Actuality, ActualityKind, ArgumentValue, Connector, Descriptor,
    EventualityClass, EventualitySort, FormulaOperator, IndexicalKind, ParameterRole,
    PredicationMode, ReferentCategory, SemanticGraph, SemanticObject, SemanticObjectId,
    SemanticSort, TanruLink, UtteranceForce, diagnostic, source_from_spans,
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
        let bridi = single_bridi_from_text(syntax)?;
        let utterance_id = self.next_utterance_id();
        let formula = self.build_bridi_formula(bridi)?;
        let locution = self.next_locution_id();
        self.insert(
            locution,
            SemanticObject::eventuality(
                EventualityClass::Locution,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                self.source_for_node(bridi, "bridi"),
            ),
        )?;
        self.insert(
            utterance_id,
            SemanticObject::utterance(
                UtteranceForce::Assert,
                locution,
                Some(formula),
                SemanticObjectId::speaker(),
                SemanticObjectId::addressee(),
                SemanticObjectId::now(),
                SemanticObjectId::here(),
                self.source_for_node(bridi, "bridi"),
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
        let relation = relation_label_from_selbri(&simple_tail.selbri)?;
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

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_formula_for_terms(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let [trailing_unit] = tanru.additional_units.as_slice() else {
            return Err(unsupported("multi-unit tanru"));
        };
        let head = self.build_tanru_head_relation_formula(trailing_unit, terms, source.clone())?;
        let modifier =
            self.build_property_abstraction_for_tanru_unit(&tanru.first_unit, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            head.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_generated_unit_pair(&tanru.first_unit, trailing_unit)?,
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
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_tanru_head_relation_formula(
        &mut self,
        unit: &TanruUnitSyntax,
        terms: Vec<&TermSyntax>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        let relation = semantic_relation_label(relation_label_from_generated_tanru_unit(unit)?);
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
        for place in visible_place..=place_limit {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let x1_argument = arguments
            .get("x1")
            .cloned()
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    fn build_property_abstraction_for_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let abstraction = abstraction_from_generated_tanru_unit(unit)?.cloned();
        let Some(abstraction) = abstraction else {
            return Err(unsupported("non-abstraction tanru modifier"));
        };
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
        let relation = self.next_relation_id();
        self.insert(
            relation,
            SemanticObject::abstraction(
                AbstractionKind::Property,
                body,
                vec![parameter],
                source,
                Vec::new(),
            ),
        )?;
        Ok(relation)
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

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_term_referent(
        &mut self,
        term: &TermSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sumti = simple_sumti_from_term(term)?;
        self.build_sumti_referent(sumti)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn build_sumti_referent(
        &mut self,
        sumti: &SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match simple_sumti_base_from_sumti(sumti)? {
            SumtiBaseSyntax::ProSumti(pro_sumti) => self.build_pro_sumti_referent(pro_sumti),
            SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
                self.build_description_referent(description)
            }
            SumtiBaseSyntax::NameSumti(name) => self.build_name_sumti_referent(name),
            _ => Err(unsupported("non-pro-sumti")),
        }
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
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                gadri_name_sort(name.la.value.cmavo()),
                None,
                Some(Descriptor {
                    kind: description_kind_for_cmavo(name.la.value.cmavo()).to_owned(),
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
        if description.ku.is_some() {
            return Err(unsupported(
                "explicit KU in generated description checkpoint",
            ));
        }
        if description.tail.leading_tail_elements.tail_sumti.is_some()
            || description
                .tail
                .leading_tail_elements
                .relative_clauses
                .is_some()
        {
            return Err(unsupported("description leading tail elements"));
        }
        let DescriptionTailBodySyntax::RelationDescriptionTail(RelationDescriptionTailSyntax {
            selbri,
            relative_clauses,
        }) = description.tail.tail.as_ref()
        else {
            return Err(unsupported("non-relation description tail"));
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
        let body = self.build_restrictive_formula(selbri, parameter)?;
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
    #[ensures(true)]
    fn source_for_node<N: TreeNode>(
        &self,
        node: &N,
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut visitor = GeneratedSpanCollector::default();
        node.visit_in_order(&mut visitor);
        source_from_spans(&visitor.spans, self.options.source_text, Some(construct))
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
}

impl<'tree> TreeVisitor<'tree> for GeneratedSpanCollector {
    type Atom = GeneratedAtomRef<'tree>;
    type Node = jbotci_syntax::generated_model::NodeRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let GeneratedAtomRef::Token(token) = atom;
        self.spans.extend(token.source_spans().into_iter().cloned());
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn single_bridi_from_text(syntax: &TextSyntax) -> Result<&BridiSyntax, SemanticsError> {
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
    let StatementOrFragmentSyntax::StatementOrFragmentStatement(
        StatementOrFragmentStatementSyntax(statement),
    ) = sequence.initial.0.as_ref()
    else {
        return Err(unsupported("fragment statement"));
    };
    let StatementSyntax::StatementBase(StatementBaseSyntax::BridiStatement(BridiStatementSyntax {
        bridi,
        continuations,
    })) = statement
    else {
        return Err(unsupported("non-simple statement"));
    };
    if !continuations.is_empty() {
        return Err(unsupported("statement connective continuations"));
    }
    Ok(bridi)
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
        leading_selbri,
        co_tail,
    })) = selbri
    else {
        return Err(unsupported("tagged or connected selbri"));
    };
    if co_tail.is_some() {
        return Err(unsupported("CO selbri"));
    }
    let ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = leading_selbri.as_ref();
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
    Ok(Some(leading_selbri.as_ref()))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn abstraction_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<&AbstractionTanruUnitSyntax>, SemanticsError> {
    let atom = generated_tanru_unit_atom(unit)?;
    let TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) = atom.base.as_ref() else {
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
#[ensures(ret.as_ref().is_ok_and(|atom| atom.conversions.is_empty()) || ret.is_err())]
fn generated_tanru_unit_atom(
    unit: &TanruUnitSyntax,
) -> Result<&TanruUnitAtomSyntax, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Err(unsupported("connected tanru unit"));
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Err(unsupported("non-atomic tanru unit"));
    };
    if unit.linkargs.is_some() {
        return Err(unsupported("linkargs tanru unit"));
    }
    if !unit.base.conversions.is_empty() {
        return Err(unsupported("converted tanru unit"));
    }
    Ok(&unit.base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
fn relation_label_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let atom = generated_tanru_unit_atom(unit)?;
    match atom.base.as_ref() {
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            Ok(token_text(&word.value))
        }
        TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) => {
            abstraction_relation_label_from_generated(abstraction)
        }
        _ => Err(unsupported("non-word tanru unit")),
    }
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
    Ok(format!(
        "{}-{}",
        relation_label_from_generated_tanru_unit(leading)?,
        relation_label_from_generated_tanru_unit(trailing)?
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_tanru_unit(unit: &TanruUnitSyntax) -> Result<String, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Err(unsupported("connected tanru unit"));
    }
    relation_label_from_bo_or_linked_tanru_unit(&unit.0.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<String, SemanticsError> {
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = unit else {
        return Err(unsupported("non-atomic tanru unit"));
    };
    if unit.linkargs.is_some() {
        return Err(unsupported("linkargs tanru unit"));
    }
    relation_label_from_tanru_unit_atom(&unit.base)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| !relation.is_empty()) || ret.is_err())]
fn relation_label_from_tanru_unit_atom(
    unit: &TanruUnitAtomSyntax,
) -> Result<String, SemanticsError> {
    if !unit.conversions.is_empty() {
        return Err(unsupported("converted tanru unit"));
    }
    let TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) = unit.base.as_ref()
    else {
        return Err(unsupported("non-word tanru unit"));
    };
    Ok(token_text(&word.value))
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

#[requires(!relation.is_empty())]
#[ensures(true)]
fn relation_place_count(dictionary: &Dictionary<'_>, relation: &str) -> Option<usize> {
    if relation_has_open_place_structure(relation) {
        return None;
    }
    dictionary_relation_place_count(dictionary, relation)
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
#[ensures(true)]
fn gadri_name_sort(cmavo: Option<Cmavo>) -> SemanticSort {
    match cmavo {
        Some(Cmavo::Lai) => SemanticSort::Mass,
        Some(Cmavo::Lahi) => SemanticSort::Set,
        _ => SemanticSort::Entity,
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
