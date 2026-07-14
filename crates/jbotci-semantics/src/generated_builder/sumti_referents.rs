use super::*;

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_term_argument_object(
        &mut self,
        term: &'tree TermSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let argument = self.build_argument_for_generated_term(term)?.into_data();
        let argument_object = argument
            .value
            .ok_or_else(|| unsupported("non-referential term argument"))?;
        if !argument.relative_clauses.is_empty()
            && matches!(
                argument_object.object_kind(),
                crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Sign
            )
        {
            let object = self.objects.get_mut(&argument_object).ok_or_else(|| {
                invalid_graph(format!(
                    "semantic builder could not find generated term argument object {argument_object}"
                ))
            })?;
            object.extend_relative_clauses(argument.relative_clauses);
        }
        Ok(argument_object)
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn insert_generated_term_assignment<'syntax: 'tree>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        place_questions: &mut Vec<GeneratedPlaceQuestionAssignment>,
        modal_terms: &mut Vec<&'syntax TaggedSumtiTermSyntax>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        term_formula_scopes: &mut Vec<GeneratedTermFormulaScope>,
        next_visible_place: &mut usize,
        term: &'syntax TermSyntax,
    ) -> Result<(), SemanticsError> {
        match term {
            TermSyntax::TermsetGroup(termset) => self.insert_generated_termset_group_assignment(
                visible_arguments,
                place_questions,
                modal_terms,
                formula_scopes,
                coequal_scope_groups,
                term_formula_scopes,
                next_visible_place,
                term,
                termset,
            ),
            TermSyntax::SimpleTerm(simple) => self.insert_generated_simple_term_assignment(
                visible_arguments,
                place_questions,
                modal_terms,
                formula_scopes,
                coequal_scope_groups,
                term_formula_scopes,
                next_visible_place,
                term,
                simple,
            ),
            TermSyntax::ConnectedTerm(ConnectedTermSyntax {
                leading_term,
                continuations,
            }) if continuations.is_empty() => self.insert_generated_simple_term_assignment(
                visible_arguments,
                place_questions,
                modal_terms,
                formula_scopes,
                coequal_scope_groups,
                term_formula_scopes,
                next_visible_place,
                term,
                leading_term.as_ref(),
            ),
            _ => Err(unsupported("non-simple term")),
        }
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn insert_generated_termset_group_assignment<'syntax: 'tree, N: TreeNode>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        place_questions: &mut Vec<GeneratedPlaceQuestionAssignment>,
        modal_terms: &mut Vec<&'syntax TaggedSumtiTermSyntax>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        term_formula_scopes: &mut Vec<GeneratedTermFormulaScope>,
        next_visible_place: &mut usize,
        node: &N,
        termset: &'syntax TermsetGroupSyntax,
    ) -> Result<(), SemanticsError> {
        let mut local_formula_scopes = Vec::new();
        let mut local_coequal_scope_groups = Vec::new();
        self.insert_generated_simple_term_assignment(
            visible_arguments,
            place_questions,
            modal_terms,
            &mut local_formula_scopes,
            &mut local_coequal_scope_groups,
            term_formula_scopes,
            next_visible_place,
            termset.leading_term.as_ref(),
            termset.leading_term.as_ref(),
        )?;
        for continuation in &termset.continuations {
            self.insert_generated_simple_term_assignment(
                visible_arguments,
                place_questions,
                modal_terms,
                &mut local_formula_scopes,
                &mut local_coequal_scope_groups,
                term_formula_scopes,
                next_visible_place,
                continuation.trailing_term.as_ref(),
                continuation.trailing_term.as_ref(),
            )?;
        }
        push_generated_coequal_scope_group_or_individual_scopes(
            local_formula_scopes,
            self.source_for_node(node, "quantifier-bundle"),
            formula_scopes,
            coequal_scope_groups,
        );
        coequal_scope_groups.extend(local_coequal_scope_groups);
        Ok(())
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn insert_generated_simple_term_assignment<'syntax: 'tree, N: TreeNode>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        place_questions: &mut Vec<GeneratedPlaceQuestionAssignment>,
        modal_terms: &mut Vec<&'syntax TaggedSumtiTermSyntax>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        term_formula_scopes: &mut Vec<GeneratedTermFormulaScope>,
        next_visible_place: &mut usize,
        node: &N,
        simple: &'syntax SimpleTermSyntax,
    ) -> Result<(), SemanticsError> {
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                if self.insert_generated_termset_sumti_assignment(
                    visible_arguments,
                    formula_scopes,
                    coequal_scope_groups,
                    next_visible_place,
                    self.source_for_node(node, "quantifier-bundle"),
                    sumti,
                )? {
                    return Ok(());
                }
                let argument = match generated_voha_place_for_sumti(sumti)
                    .and_then(|place| visible_arguments.get(&place).cloned())
                {
                    Some(argument) => argument,
                    None => self.build_argument_for_generated_sumti_with_formula_scopes(
                        sumti,
                        formula_scopes,
                    )?,
                };
                let place =
                    first_unfilled_generated_visible_place(visible_arguments, *next_visible_place);
                insert_visible_argument(visible_arguments, place, argument)?;
                record_generated_visible_place_assignment(
                    visible_arguments,
                    next_visible_place,
                    place,
                );
                Ok(())
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                if term.fa.value.cmavo() == Some(Cmavo::Fiha) {
                    let argument = self.build_tagged_or_elided_sumti_argument(&term.sumti)?;
                    place_questions.push(new!(GeneratedPlaceQuestionAssignment {
                        introduced_by: token_text(&term.fa.value),
                        argument,
                        parameter_source: self.source_for_node(node, "parameter"),
                        binding_source: self.source_for_node(node, "place-question"),
                    }));
                    *next_visible_place += 1;
                    return Ok(());
                }
                let place = fa_place(&term.fa.value)?;
                let argument = match term.sumti.as_ref() {
                    TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                        match generated_voha_place_for_sumti(sumti).and_then(|referenced_place| {
                            visible_arguments.get(&referenced_place).cloned()
                        }) {
                            Some(argument) => argument,
                            None => self.build_tagged_or_elided_sumti_argument(&term.sumti)?,
                        }
                    }
                    TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => {
                        self.build_tagged_or_elided_sumti_argument(&term.sumti)?
                    }
                };
                insert_visible_argument(visible_arguments, place, argument)?;
                record_generated_visible_place_assignment(
                    visible_arguments,
                    next_visible_place,
                    place,
                );
                Ok(())
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                modal_terms.push(term);
                Ok(())
            }
            SimpleTermSyntax::NaKuTerm(_) | SimpleTermSyntax::BareNaTerm(_) => {
                term_formula_scopes.push(GeneratedTermFormulaScope::Negation {
                    source: self.source_for_node(node, "bridi-negation-boundary"),
                });
                Ok(())
            }
            SimpleTermSyntax::NuhiTermset(termset) => {
                let mut local_formula_scopes = Vec::new();
                let mut local_coequal_scope_groups = Vec::new();
                for term in &termset.termset {
                    self.insert_generated_term_assignment(
                        visible_arguments,
                        place_questions,
                        modal_terms,
                        &mut local_formula_scopes,
                        &mut local_coequal_scope_groups,
                        term_formula_scopes,
                        next_visible_place,
                        term,
                    )?;
                }
                push_generated_coequal_scope_group_or_individual_scopes(
                    local_formula_scopes,
                    self.source_for_node(node, "quantifier-bundle"),
                    formula_scopes,
                    coequal_scope_groups,
                );
                coequal_scope_groups.extend(local_coequal_scope_groups);
                Ok(())
            }
            SimpleTermSyntax::KeTermset(termset) => {
                let mut local_formula_scopes = Vec::new();
                let mut local_coequal_scope_groups = Vec::new();
                for term in &termset.termset {
                    self.insert_generated_term_assignment(
                        visible_arguments,
                        place_questions,
                        modal_terms,
                        &mut local_formula_scopes,
                        &mut local_coequal_scope_groups,
                        term_formula_scopes,
                        next_visible_place,
                        term,
                    )?;
                }
                push_generated_coequal_scope_group_or_individual_scopes(
                    local_formula_scopes,
                    self.source_for_node(node, "quantifier-bundle"),
                    formula_scopes,
                    coequal_scope_groups,
                );
                coequal_scope_groups.extend(local_coequal_scope_groups);
                Ok(())
            }
            _ => Err(unsupported("non-sumti term")),
        }
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|handled| !*handled || *next_visible_place > 1) || ret.is_err())]
    pub(super) fn insert_generated_termset_sumti_assignment<'syntax: 'tree>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        next_visible_place: &mut usize,
        source: Option<crate::model::SemanticSource>,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<bool, SemanticsError> {
        let Some(afterthought) = generated_sumti_afterthought_for_termset(sumti) else {
            return Ok(false);
        };
        let mut local_formula_scopes = Vec::new();
        self.insert_generated_sumti_bound_termset_assignment(
            visible_arguments,
            &mut local_formula_scopes,
            next_visible_place,
            &afterthought.leading_sumti,
        )?;
        for continuation in &afterthought.continuations {
            self.insert_generated_sumti_bound_termset_assignment(
                visible_arguments,
                &mut local_formula_scopes,
                next_visible_place,
                &continuation.sumti,
            )?;
        }
        push_generated_coequal_scope_group_or_individual_scopes(
            local_formula_scopes,
            source,
            formula_scopes,
            coequal_scope_groups,
        );
        Ok(true)
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn insert_generated_sumti_bound_termset_assignment<'syntax: 'tree>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        next_visible_place: &mut usize,
        sumti: &'syntax SumtiBoundSyntax,
    ) -> Result<(), SemanticsError> {
        let argument = self.build_generated_alternative_argument_for_sumti_bound(sumti, false)?;
        formula_scopes.extend(argument.formula_scopes);
        insert_visible_argument(visible_arguments, *next_visible_place, argument.argument)?;
        *next_visible_place += 1;
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_modal_argument_for_generated_tagged_sumti(
        &mut self,
        term: &'tree TaggedSumtiTermSyntax,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        self.build_modal_argument_for_generated_tagged_sumti_with_visible_arguments(term, None)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_modal_argument_for_generated_tagged_sumti_with_visible_arguments(
        &mut self,
        term: &'tree TaggedSumtiTermSyntax,
        visible_arguments: Option<&BTreeMap<usize, ArgumentValue>>,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        let tense_modal = term.tense_modal.as_ref();
        if generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        if let Some(selbri) = generated_fiho_tense_selbri(tense_modal) {
            let argument = self.build_tagged_or_elided_sumti_argument_with_visible_arguments(
                &term.sumti,
                visible_arguments,
            )?;
            return self
                .build_generated_ad_hoc_modal_argument_for_selbri(
                    tense_modal,
                    selbri,
                    argument,
                    "modal-argument",
                )
                .map(Some);
        }
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("tagged sumti tense modal"));
        };
        let argument = self.build_tagged_or_elided_sumti_argument_with_visible_arguments(
            &term.sumti,
            visible_arguments,
        )?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
        Ok(Some(
            self.generated_modal_argument_with_tense_modal_modifiers(
                tense_modal,
                relation,
                introduced_by,
                arguments,
                generated_modal_negation_for_tense_modal(tense_modal),
                generated_modal_scalar_negation_for_tense_modal(tense_modal),
                "modal-argument",
            ),
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_modal_argument_for_generated_tagged_sumti_with_predication_arguments(
        &mut self,
        term: &'tree TaggedSumtiTermSyntax,
        arguments: Option<&BTreeMap<PlaceIndex, ArgumentValue>>,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        let tense_modal = term.tense_modal.as_ref();
        if generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        if let Some(selbri) = generated_fiho_tense_selbri(tense_modal) {
            let argument = self.build_tagged_or_elided_sumti_argument_with_predication_arguments(
                &term.sumti,
                arguments,
            )?;
            return self
                .build_generated_ad_hoc_modal_argument_for_selbri(
                    tense_modal,
                    selbri,
                    argument,
                    "modal-argument",
                )
                .map(Some);
        }
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("tagged sumti tense modal"));
        };
        let argument = self.build_tagged_or_elided_sumti_argument_with_predication_arguments(
            &term.sumti,
            arguments,
        )?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
        Ok(Some(
            self.generated_modal_argument_with_tense_modal_modifiers(
                tense_modal,
                relation,
                introduced_by,
                arguments,
                generated_modal_negation_for_tense_modal(tense_modal),
                generated_modal_scalar_negation_for_tense_modal(tense_modal),
                "modal-argument",
            ),
        ))
    }

    #[requires(!construct.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|modal_argument| modal_argument.body.is_some() && modal_argument.relation.is_none()) || ret.is_err())]
    pub(super) fn build_generated_ad_hoc_modal_argument_for_selbri<N: TreeNode>(
        &mut self,
        tense_modal: &N,
        selbri: &'tree SelbriSyntax,
        argument: ArgumentValue,
        construct: &str,
    ) -> Result<ModalArgument, SemanticsError> {
        let mut visible_arguments = BTreeMap::new();
        insert_visible_argument(&mut visible_arguments, 1, argument)?;
        let source = self.source_for_node(tense_modal, construct);
        let lowered = self.build_selbri_formula_for_visible_arguments(
            selbri,
            visible_arguments,
            source.clone(),
            "modal-argument",
            None,
        )?;
        self.set_formula_predication_mode(lowered.formula, PredicationMode::Incidental);
        let mut modal_argument = ModalArgument::body("fi'o".to_owned(), lowered.formula, source);
        let modifiers = self.modal_argument_modifiers_for_generated_tense_modal(tense_modal);
        if !modifiers.is_empty() {
            modal_argument = modal_argument.with_data(data! { modifiers: modifiers });
        }
        Ok(modal_argument)
    }

    #[requires(!relation.is_empty())]
    #[requires(!introduced_by.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[requires(!construct.is_empty())]
    #[ensures(true)]
    pub(super) fn generated_modal_argument_with_tense_modal_modifiers<N: TreeNode>(
        &self,
        tense_modal: &N,
        relation: String,
        introduced_by: String,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
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
            self.source_for_node(tense_modal, construct),
        );
        let modifiers = self.modal_argument_modifiers_for_generated_tense_modal(tense_modal);
        if !modifiers.is_empty() {
            modal_argument = modal_argument.with_data(data! { modifiers: modifiers });
        }
        modal_argument
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn modal_argument_modifiers_for_generated_tense_modal<N: TreeNode>(
        &self,
        tense_modal: &N,
    ) -> Vec<DisplayedContentModifier> {
        indicator_display_drafts(indicator_parts_for_generated_node(tense_modal))
            .into_iter()
            .map(|draft| {
                let source = if draft.source_tokens.is_empty() {
                    None
                } else {
                    self.source_for_tokens(&draft.source_tokens, "modal-indicator")
                }
                .or_else(|| self.source_for_node(tense_modal, "modal-indicator"));
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

    #[requires(visible_x1_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|arguments| !arguments.is_empty()) || ret.is_err())]
    pub(super) fn modal_argument_map_for_visible_place(
        &mut self,
        argument: ArgumentValue,
        visible_x1_place: usize,
        place_count: Option<usize>,
    ) -> Result<BTreeMap<PlaceIndex, ArgumentValue>, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(visible_x1_place), argument);
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

    #[requires(eventuality.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.relation.is_some()) || ret.is_err())]
    pub(super) fn build_generated_jai_modal_argument(
        &mut self,
        unit: &JaiModalTanruUnitSyntax,
        visible_arguments: &BTreeMap<usize, ArgumentValue>,
        eventuality: SemanticObjectId,
    ) -> Result<ModalArgument, SemanticsError> {
        let Some(tense_modal) = unit.tense_modal.as_deref() else {
            return Err(unsupported("bare jai modal argument"));
        };
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("jai modal tanru unit tense modal"));
        };
        let other_place = convert_numbered_place(2, visible_place);
        let highest_place = relation_place_count(self.dictionary, &relation)
            .unwrap_or(visible_place.max(other_place))
            .max(visible_place)
            .max(other_place);
        let visible_argument = visible_arguments
            .get(&1)
            .cloned()
            .unwrap_or(self.build_elided_argument_for_place(1)?);
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(visible_place), visible_argument);
        arguments.insert(
            argument_key(other_place),
            ArgumentValue::filled(eventuality, None),
        );
        for place in 1..=highest_place {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        Ok(self.generated_modal_argument_with_tense_modal_modifiers(
            tense_modal,
            relation,
            introduced_by,
            arguments,
            generated_modal_negation_for_tense_modal(tense_modal),
            generated_modal_scalar_negation_for_tense_modal(tense_modal),
            "modal-argument",
        ))
    }

    #[requires(crate::model::argument_object_kind_can_fill(argument_object.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.as_ref().is_none_or(|argument| argument.relation.is_some())) || ret.is_err())]
    pub(super) fn build_generated_jai_modal_argument_for_argument_object(
        &mut self,
        unit: &JaiModalTanruUnitSyntax,
        argument_object: SemanticObjectId,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        let Some(tense_modal) = unit.tense_modal.as_deref() else {
            return Ok(None);
        };
        if generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("jai modal tanru unit tense modal"));
        };
        let arguments = self.modal_argument_map_for_visible_place(
            ArgumentValue::filled(argument_object, None),
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
        Ok(Some(
            self.generated_modal_argument_with_tense_modal_modifiers(
                tense_modal,
                relation,
                introduced_by,
                arguments,
                generated_modal_negation_for_tense_modal(tense_modal),
                generated_modal_scalar_negation_for_tense_modal(tense_modal),
                "modal-argument",
            ),
        ))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn attach_generated_modal_terms_to_formula(
        &mut self,
        formula: SemanticObjectId,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
    ) -> Result<(), SemanticsError> {
        for modal_term in modal_terms {
            self.attach_generated_modal_term_to_formula(formula, modal_term)?;
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn attach_generated_modal_term_to_formula(
        &mut self,
        formula: SemanticObjectId,
        modal_term: &'tree TaggedSumtiTermSyntax,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&formula)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing generated formula {formula}")))?;
        if let Some(predication) = object.formula_predication() {
            self.attach_generated_modal_term_to_predication(predication, modal_term)?;
        }
        for child in object.formula_children().to_vec() {
            self.attach_generated_modal_term_to_formula(child, modal_term)?;
        }
        if let Some(body) = object.formula_body() {
            self.attach_generated_modal_term_to_formula(body, modal_term)?;
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn attach_generated_modal_term_to_predication(
        &mut self,
        predication: SemanticObjectId,
        modal_term: &'tree TaggedSumtiTermSyntax,
    ) -> Result<(), SemanticsError> {
        let (mode, eventuality) = {
            let object = self.objects.get(&predication).ok_or_else(|| {
                invalid_graph(format!("missing generated predication {predication}"))
            })?;
            (object.predication_mode(), object.predication_eventuality())
        };
        if mode != Some(PredicationMode::Asserted) {
            return Ok(());
        }
        let Some(mut modal_argument) =
            self.build_modal_argument_for_generated_tagged_sumti(modal_term)?
        else {
            return Ok(());
        };
        if let Some(eventuality) = eventuality {
            self.bind_generated_modal_argument_to_host_event(&mut modal_argument, eventuality);
        }
        let object = self
            .objects
            .get_mut(&predication)
            .ok_or_else(|| invalid_graph(format!("missing generated predication {predication}")))?;
        if object
            .predication_modal_arguments()
            .is_some_and(|arguments| !arguments.contains(&modal_argument))
        {
            object.update_predication(|node| {
                let mut data = node.into_data();
                data.modal_arguments.push(modal_argument);
                PredicationNode::from_data(data)
            });
        }
        Ok(())
    }

    #[requires(!construct.is_empty())]
    #[ensures(true)]
    pub(super) fn build_modal_argument_for_generated_tense_modal(
        &mut self,
        tense_modal: &'tree TenseModalSyntax,
        construct: &str,
    ) -> Result<Option<ModalArgument>, SemanticsError> {
        if generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(None);
        }
        if let Some(selbri) = generated_fiho_tense_modal_selbri(tense_modal) {
            let visible_x1_place = generated_raw_place_visible_rank_for_selbri(selbri, 1)?;
            let argument = self.build_elided_argument_for_place(visible_x1_place)?;
            return self
                .build_generated_ad_hoc_modal_argument_for_selbri(
                    tense_modal,
                    selbri,
                    argument,
                    construct,
                )
                .map(Some);
        }
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Ok(None);
        };
        let argument = self.build_elided_argument_for_place(visible_place)?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
        Ok(Some(
            self.generated_modal_argument_with_tense_modal_modifiers(
                tense_modal,
                relation,
                introduced_by,
                arguments,
                generated_modal_negation_for_tense_modal(tense_modal),
                generated_modal_scalar_negation_for_tense_modal(tense_modal),
                construct,
            ),
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn attach_modal_argument_to_generated_discourse_item(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&id)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing generated discourse item {id}")))?;
        match object.object_kind() {
            crate::model::SemanticObjectKind::Utterance => {
                if let Some(content) = object.as_utterance().and_then(|node| node.content) {
                    self.attach_modal_argument_to_generated_content(content, modal_argument)?;
                }
            }
            crate::model::SemanticObjectKind::Sequence => {
                for item in object
                    .as_sequence()
                    .map(|node| node.items.clone())
                    .unwrap_or_default()
                {
                    self.attach_modal_argument_to_generated_discourse_item(item, modal_argument)?;
                }
            }
            crate::model::SemanticObjectKind::Formula => {
                self.attach_modal_argument_to_generated_formula(id, modal_argument)?;
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn attach_modal_argument_to_generated_content(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        match id.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.attach_modal_argument_to_generated_formula(id, modal_argument)
            }
            crate::model::SemanticObjectKind::Sequence => {
                self.attach_modal_argument_to_generated_discourse_item(id, modal_argument)
            }
            crate::model::SemanticObjectKind::Question => Ok(()),
            _ => Ok(()),
        }
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn attach_modal_argument_to_generated_formula(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&id)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing generated formula {id}")))?;
        if let Some(predication) = object.formula_predication() {
            self.attach_modal_argument_to_generated_predication(predication, modal_argument)?;
        }
        for child in object.formula_children().to_vec() {
            self.attach_modal_argument_to_generated_formula(child, modal_argument)?;
        }
        if let Some(body) = object.formula_body() {
            self.attach_modal_argument_to_generated_formula(body, modal_argument)?;
        }
        Ok(())
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn prepend_modal_argument_to_generated_formula(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let object = self
            .objects
            .get(&id)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing generated formula {id}")))?;
        if let Some(predication) = object.formula_predication() {
            self.prepend_modal_argument_to_generated_predication(predication, modal_argument)?;
        }
        for child in object.formula_children().to_vec() {
            self.prepend_modal_argument_to_generated_formula(child, modal_argument)?;
        }
        if let Some(body) = object.formula_body() {
            self.prepend_modal_argument_to_generated_formula(body, modal_argument)?;
        }
        Ok(())
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn attach_modal_argument_to_generated_predication(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let (mode, eventuality) = {
            let object = self
                .objects
                .get(&id)
                .ok_or_else(|| invalid_graph(format!("missing generated predication {id}")))?;
            (object.predication_mode(), object.predication_eventuality())
        };
        if mode == Some(PredicationMode::Asserted) {
            let mut modal_argument = modal_argument.clone();
            if let Some(eventuality) = eventuality {
                self.bind_generated_modal_argument_to_host_event(&mut modal_argument, eventuality);
            }
            let object = self
                .objects
                .get_mut(&id)
                .ok_or_else(|| invalid_graph(format!("missing generated predication {id}")))?;
            if object
                .predication_modal_arguments()
                .is_some_and(|arguments| !arguments.contains(&modal_argument))
            {
                object.update_predication(|node| {
                    let mut data = node.into_data();
                    data.modal_arguments.push(modal_argument);
                    PredicationNode::from_data(data)
                });
            }
        }
        Ok(())
    }

    #[requires(id.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn prepend_modal_argument_to_generated_predication(
        &mut self,
        id: SemanticObjectId,
        modal_argument: &ModalArgument,
    ) -> Result<(), SemanticsError> {
        let (mode, eventuality) = {
            let object = self
                .objects
                .get(&id)
                .ok_or_else(|| invalid_graph(format!("missing generated predication {id}")))?;
            (object.predication_mode(), object.predication_eventuality())
        };
        if mode == Some(PredicationMode::Asserted) {
            let mut modal_argument = modal_argument.clone();
            if let Some(eventuality) = eventuality {
                self.bind_generated_modal_argument_to_host_event(&mut modal_argument, eventuality);
            }
            let object = self
                .objects
                .get_mut(&id)
                .ok_or_else(|| invalid_graph(format!("missing generated predication {id}")))?;
            if object
                .predication_modal_arguments()
                .is_some_and(|arguments| !arguments.contains(&modal_argument))
            {
                object.update_predication(|node| {
                    let mut data = node.into_data();
                    data.modal_arguments.insert(0, modal_argument);
                    PredicationNode::from_data(data)
                });
            }
        }
        Ok(())
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn bind_generated_modal_argument_to_host_event(
        &mut self,
        modal_argument: &mut ModalArgument,
        eventuality: SemanticObjectId,
    ) {
        if let Some(elision) = bind_generated_modal_argument_to_host_event_preserving_elision(
            modal_argument,
            eventuality,
        ) {
            self.host_event_modal_elisions
                .entry(eventuality)
                .or_default()
                .push(elision);
        }
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(place > 0)]
    #[ensures(ret.as_ref().is_none_or(|argument| argument.kind == ArgumentValueKind::Elided))]
    pub(super) fn generated_host_event_modal_elision(
        &mut self,
        eventuality: SemanticObjectId,
        modal_argument: &ModalArgument,
        place: usize,
    ) -> Option<ArgumentValue> {
        let relation = modal_argument.relation.as_ref()?;
        self.host_event_modal_elisions
            .get(&eventuality)?
            .iter()
            .find(|elision| {
                elision.place == place
                    && &elision.relation == relation
                    && elision.introduced_by == modal_argument.introduced_by
                    && elision.source == modal_argument.source
            })
            .map(|elision| elision.argument.clone())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_modal_arguments_for_generated_tagged_terms(
        &mut self,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let inherited_modal_arguments = self.sticky_modal_arguments.clone();
        let mut modal_arguments = Vec::new();
        for term in modal_terms {
            if let Some(argument) = self.build_modal_argument_for_generated_tagged_sumti(term)? {
                self.record_generated_sticky_modal_argument_if_needed(
                    term.tense_modal.as_ref(),
                    &argument,
                );
                modal_arguments.push(argument);
            }
        }
        self.append_generated_sticky_modal_arguments(
            &inherited_modal_arguments,
            &mut modal_arguments,
            None,
        );
        Ok(modal_arguments)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn build_modal_arguments_for_generated_tagged_terms_for_event(
        &mut self,
        eventuality: SemanticObjectId,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        self.build_modal_arguments_for_generated_tagged_terms_for_event_with_visible_arguments(
            eventuality,
            modal_terms,
            None,
        )
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn build_modal_arguments_for_generated_tagged_terms_for_event_with_visible_arguments(
        &mut self,
        eventuality: SemanticObjectId,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
        visible_arguments: Option<&BTreeMap<usize, ArgumentValue>>,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let inherited_modal_arguments = self.sticky_modal_arguments.clone();
        let mut modal_arguments = Vec::new();
        for term in modal_terms {
            if let Some(mut argument) = self
                .build_modal_argument_for_generated_tagged_sumti_with_visible_arguments(
                    term,
                    visible_arguments,
                )?
            {
                self.record_generated_sticky_modal_argument_if_needed(
                    term.tense_modal.as_ref(),
                    &argument,
                );
                self.bind_generated_modal_argument_to_host_event(&mut argument, eventuality);
                modal_arguments.push(argument);
            }
        }
        self.append_generated_sticky_modal_arguments(
            &inherited_modal_arguments,
            &mut modal_arguments,
            Some(eventuality),
        );
        Ok(modal_arguments)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
        &mut self,
        eventuality: SemanticObjectId,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
        arguments: Option<&BTreeMap<PlaceIndex, ArgumentValue>>,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let inherited_modal_arguments = self.sticky_modal_arguments.clone();
        let mut modal_arguments = Vec::new();
        for term in modal_terms {
            if let Some(mut argument) = self
                .build_modal_argument_for_generated_tagged_sumti_with_predication_arguments(
                    term, arguments,
                )?
            {
                self.record_generated_sticky_modal_argument_if_needed(
                    term.tense_modal.as_ref(),
                    &argument,
                );
                self.bind_generated_modal_argument_to_host_event(&mut argument, eventuality);
                modal_arguments.push(argument);
            }
        }
        self.append_generated_sticky_modal_arguments(
            &inherited_modal_arguments,
            &mut modal_arguments,
            Some(eventuality),
        );
        Ok(modal_arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_generated_sticky_modal_argument_if_needed<N: TreeNode>(
        &mut self,
        tense_modal: &N,
        modal_argument: &ModalArgument,
    ) {
        if !generated_tense_modal_makes_modal_sticky(tense_modal) {
            return;
        }
        if modal_argument.relation.is_none() {
            return;
        }
        self.sticky_modal_arguments.insert(
            GeneratedStickyModalKey::for_modal_argument(modal_argument),
            modal_argument.clone(),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn append_generated_sticky_modal_arguments(
        &mut self,
        inherited_modal_arguments: &BTreeMap<GeneratedStickyModalKey, ModalArgument>,
        modal_arguments: &mut Vec<ModalArgument>,
        eventuality: Option<SemanticObjectId>,
    ) {
        for (key, sticky_modal) in inherited_modal_arguments {
            if modal_arguments
                .iter()
                .filter(|modal_argument| modal_argument.relation.is_some())
                .map(GeneratedStickyModalKey::for_modal_argument)
                .any(|modal_key| modal_key == *key)
            {
                continue;
            }
            let mut sticky_modal = sticky_modal.clone();
            if let Some(eventuality) = eventuality {
                self.bind_generated_modal_argument_to_host_event(&mut sticky_modal, eventuality);
            }
            if modal_arguments.contains(&sticky_modal) {
                continue;
            }
            modal_arguments.push(sticky_modal);
        }
    }

    #[requires(true)]
    #[ensures(self.pending_sumti_candidates.len() == old(self.pending_sumti_candidates.len()))]
    pub(super) fn with_pending_sumti_candidates_for_terms<T>(
        &mut self,
        terms: &[&'tree TermSyntax],
        body: impl FnOnce(&mut Self) -> Result<T, SemanticsError>,
    ) -> Result<T, SemanticsError> {
        let old_len = self.pending_sumti_candidates.len();
        let mut candidates = Vec::new();
        for term in terms {
            self.collect_pending_sumti_candidates_for_term(term, &mut candidates)?;
        }
        self.pending_sumti_candidates.extend(candidates);
        let result = body(self);
        self.pending_sumti_candidates.truncate(old_len);
        result
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_pending_sumti_candidates_for_term(
        &self,
        term: &'tree TermSyntax,
        candidates: &mut Vec<GeneratedPendingSumtiCandidate<'tree>>,
    ) -> Result<(), SemanticsError> {
        match term {
            TermSyntax::TermsetGroup(termset) => {
                self.collect_pending_sumti_candidates_for_simple_term(
                    termset.leading_term.as_ref(),
                    candidates,
                )?;
                for continuation in &termset.continuations {
                    self.collect_pending_sumti_candidates_for_simple_term(
                        continuation.trailing_term.as_ref(),
                        candidates,
                    )?;
                }
                Ok(())
            }
            TermSyntax::SimpleTerm(simple) => {
                self.collect_pending_sumti_candidates_for_simple_term(simple, candidates)
            }
            TermSyntax::ConnectedTerm(ConnectedTermSyntax {
                leading_term,
                continuations,
            }) if continuations.is_empty() => self
                .collect_pending_sumti_candidates_for_simple_term(
                    leading_term.as_ref(),
                    candidates,
                ),
            _ => Ok(()),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_pending_sumti_candidates_for_simple_term(
        &self,
        simple: &'tree SimpleTermSyntax,
        candidates: &mut Vec<GeneratedPendingSumtiCandidate<'tree>>,
    ) -> Result<(), SemanticsError> {
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                self.push_pending_sumti_candidate(sumti, candidates);
                Ok(())
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                if let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() {
                    self.push_pending_sumti_candidate(sumti, candidates);
                }
                Ok(())
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                if let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() {
                    self.push_pending_sumti_candidate(sumti, candidates);
                }
                Ok(())
            }
            SimpleTermSyntax::NuhiTermset(termset) => {
                for term in &termset.termset {
                    self.collect_pending_sumti_candidates_for_term(term, candidates)?;
                }
                Ok(())
            }
            SimpleTermSyntax::KeTermset(termset) => {
                for term in &termset.termset {
                    self.collect_pending_sumti_candidates_for_term(term, candidates)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn push_pending_sumti_candidate(
        &self,
        sumti: &'tree SumtiSyntax,
        candidates: &mut Vec<GeneratedPendingSumtiCandidate<'tree>>,
    ) {
        let Some(source_key) = self.source_key_for_node(sumti) else {
            return;
        };
        candidates.push(new!(GeneratedPendingSumtiCandidate { source_key, sumti }));
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    pub(super) fn build_pending_previous_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((node_start, _)) = self.source_key_for_node(node) else {
            return Ok(None);
        };
        let candidate = self
            .pending_sumti_candidates
            .iter()
            .filter(|candidate| candidate.source_key.1 <= node_start)
            .max_by_key(|candidate| candidate.source_key.1);
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        self.build_sumti_referent(candidate.sumti).map(Some)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tagged_term_event_modifiers_in_terms<'syntax: 'tree>(
        &mut self,
        eventuality: SemanticObjectId,
        terms: &[&'syntax TermSyntax],
    ) -> Result<(), SemanticsError> {
        self.with_pending_sumti_candidates_for_terms(terms, |builder| {
            let governed_termsets = builder.build_generated_governed_termsets_for_terms(terms)?;
            for (index, term) in terms.iter().enumerate() {
                let Ok(SimpleTermSyntax::TaggedSumtiTerm(term)) =
                    generated_simple_term_for_assignment(term)
                else {
                    continue;
                };
                if let Some(governed) = governed_termsets.get(&index) {
                    builder.apply_generated_tagged_term_event_modifier_with_governed_termset(
                        eventuality,
                        term,
                        governed,
                    )?;
                } else {
                    builder.apply_generated_tagged_term_event_modifier(eventuality, term)?;
                }
            }
            if builder.deferred_event_modifier_flush_depth == 0 {
                builder.flush_generated_event_modifiers_with_recurrence_quantity_promotion(
                    eventuality,
                )?;
            }
            Ok(())
        })
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tagged_term_event_modifiers(
        &mut self,
        eventuality: SemanticObjectId,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
    ) -> Result<(), SemanticsError> {
        for term in modal_terms {
            self.apply_generated_tagged_term_event_modifier(eventuality, term)?;
        }
        if self.deferred_event_modifier_flush_depth == 0 {
            self.flush_generated_event_modifiers_with_recurrence_quantity_promotion(eventuality)?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|governed| governed.values().all(|termset| termset.anchor.is_some() || termset.magnitude.is_some())) || ret.is_err())]
    pub(super) fn build_generated_governed_termsets_for_terms<'syntax: 'tree>(
        &mut self,
        terms: &[&'syntax TermSyntax],
    ) -> Result<BTreeMap<usize, GeneratedGovernedTermset>, SemanticsError> {
        let mut by_termset = BTreeMap::<usize, GeneratedGovernedTermset>::new();
        let mut by_modifier = BTreeMap::<usize, GeneratedGovernedTermset>::new();
        for (modifier_index, term) in terms.iter().enumerate() {
            if !generated_tagged_term_governs_following_termset(term) {
                continue;
            }
            let Some(termset_index) =
                generated_nearest_following_governed_termset_index(terms, modifier_index + 1)
            else {
                continue;
            };
            if let std::collections::btree_map::Entry::Vacant(entry) =
                by_termset.entry(termset_index)
            {
                if let Some(governed) =
                    self.build_generated_governed_termset(termset_index, terms[termset_index])?
                {
                    entry.insert(governed);
                }
            }
            if let Some(governed) = by_termset.get(&termset_index) {
                by_modifier.insert(modifier_index, governed.clone());
            }
        }
        Ok(by_modifier)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|termset| termset.as_ref().is_none_or(|termset| termset.anchor.is_some() || termset.magnitude.is_some())) || ret.is_err())]
    pub(super) fn build_generated_governed_termset<'syntax: 'tree>(
        &mut self,
        termset_index: usize,
        termset: &'syntax TermSyntax,
    ) -> Result<Option<GeneratedGovernedTermset>, SemanticsError> {
        let mut anchor = None;
        let mut magnitude = None;
        self.collect_generated_governed_termset_members(termset, &mut anchor, &mut magnitude)?;
        if anchor.is_none() && magnitude.is_none() {
            Ok(None)
        } else {
            Ok(Some(new!(GeneratedGovernedTermset {
                termset_index,
                anchor,
                magnitude,
            })))
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_governed_termset_members<'syntax: 'tree>(
        &mut self,
        term: &'syntax TermSyntax,
        anchor: &mut Option<SemanticObjectId>,
        magnitude: &mut Option<AnchorMagnitude>,
    ) -> Result<(), SemanticsError> {
        match term {
            TermSyntax::TermsetGroup(termset) => {
                self.collect_generated_governed_termset_simple_member(
                    termset.leading_term.as_ref(),
                    anchor,
                    magnitude,
                )?;
                for continuation in &termset.continuations {
                    self.collect_generated_governed_termset_simple_member(
                        continuation.trailing_term.as_ref(),
                        anchor,
                        magnitude,
                    )?;
                }
                Ok(())
            }
            _ => {
                let Ok(simple) = generated_simple_term_for_assignment(term) else {
                    return Ok(());
                };
                self.collect_generated_governed_termset_simple_member(simple, anchor, magnitude)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_governed_termset_simple_member<'syntax: 'tree>(
        &mut self,
        simple: &'syntax SimpleTermSyntax,
        anchor: &mut Option<SemanticObjectId>,
        magnitude: &mut Option<AnchorMagnitude>,
    ) -> Result<(), SemanticsError> {
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) if anchor.is_none() => {
                *anchor = self.build_argument_for_generated_sumti(sumti)?.value;
            }
            SimpleTermSyntax::TaggedSumtiTerm(term)
                if magnitude.is_none()
                    && generated_tense_modal_is_lahu_modal(term.tense_modal.as_ref()) =>
            {
                if let Some(value) = self
                    .build_tagged_or_elided_sumti_argument(&term.sumti)?
                    .value
                {
                    *magnitude = Some(AnchorMagnitude::new(
                        value,
                        "la'u".to_owned(),
                        self.source_for_node(term.tense_modal.as_ref(), "exact-magnitude"),
                    ));
                }
            }
            SimpleTermSyntax::NuhiTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_governed_termset_members(
                        term.as_ref(),
                        anchor,
                        magnitude,
                    )?;
                }
            }
            SimpleTermSyntax::KeTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_governed_termset_members(
                        term.as_ref(),
                        anchor,
                        magnitude,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tagged_term_event_modifier(
        &mut self,
        eventuality: SemanticObjectId,
        term: &'tree TaggedSumtiTermSyntax,
    ) -> Result<bool, SemanticsError> {
        let tense_modal = term.tense_modal.as_ref();
        if generated_tense_modal_resets_sticky_tense(tense_modal) {
            self.record_generated_leading_term_tag_event_modifier(eventuality, tense_modal, None)?;
            return Ok(true);
        }
        if !generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(false);
        }
        let anchor = match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.build_argument_for_generated_sumti(sumti)?.value
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => None,
        };
        self.record_generated_leading_term_tag_event_modifier(eventuality, tense_modal, anchor)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tagged_term_event_modifier_with_governed_termset(
        &mut self,
        eventuality: SemanticObjectId,
        term: &'tree TaggedSumtiTermSyntax,
        governed: &GeneratedGovernedTermset,
    ) -> Result<bool, SemanticsError> {
        let tense_modal = term.tense_modal.as_ref();
        if generated_tense_modal_resets_sticky_tense(tense_modal) {
            self.record_generated_leading_term_tag_event_modifier_with_magnitude(
                eventuality,
                tense_modal,
                None,
                governed.magnitude.clone(),
            )?;
            return Ok(true);
        }
        if !generated_tense_modal_has_event_modifier(tense_modal) {
            return Ok(false);
        }
        let anchor = match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.build_argument_for_generated_sumti(sumti)?.value
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => governed.anchor,
        };
        self.record_generated_leading_term_tag_event_modifier_with_magnitude(
            eventuality,
            tense_modal,
            anchor,
            governed.magnitude.clone(),
        )
    }

    #[requires(eventuality_interval.is_none_or(|interval| crate::model::argument_object_kind_can_fill(interval.object_kind())))]
    #[requires(spatial_eventuality_interval.is_none_or(|interval| crate::model::argument_object_kind_can_fill(interval.object_kind())))]
    #[ensures(true)]
    pub(super) fn build_generated_recurrence_event_modifiers<N: TreeNode>(
        &mut self,
        tense_modal: &N,
        eventuality_interval: Option<SemanticObjectId>,
        spatial_eventuality_interval: Option<SemanticObjectId>,
        scalar_negation: Option<ScalarNegation>,
    ) -> Result<GeneratedRecurrenceEventModifiers, SemanticsError> {
        let tokens = self.tokens_for_node(tense_modal);
        let mut modifiers = GeneratedRecurrenceEventModifiers::default();
        let mut pending_number_tokens = Vec::<Token>::new();
        let mut pending_number_is_spatial = false;
        let mut next_property_is_spatial = false;
        let mut pending_recurrence_index = None::<(bool, usize, usize)>;
        let mut pending_recurrence_connection = None::<RecurrenceConnection>;
        let mut quantity_cache = GeneratedRecurrenceQuantityCache::new();
        for token in tokens {
            if token.is_cmavo(Cmavo::Fehe) {
                pending_number_tokens.clear();
                pending_number_is_spatial = false;
                pending_recurrence_index = None;
                pending_recurrence_connection = None;
                next_property_is_spatial = true;
                continue;
            }
            if token.is_cmavo(Cmavo::Pihu) {
                pending_number_tokens.clear();
                pending_number_is_spatial = false;
                pending_recurrence_index = None;
                pending_recurrence_connection = Some(RecurrenceConnection::new(
                    RecurrenceConnectionKind::Product,
                    token_text(&token),
                ));
                continue;
            }
            if token.is_cmavo(Cmavo::Nai) {
                if let Some((spatial, recurrence_index, modifier_index)) =
                    pending_recurrence_index.take()
                {
                    apply_generated_recurrence_negation(
                        &mut modifiers,
                        spatial,
                        recurrence_index,
                        modifier_index,
                        ModalNegation::new(ModalNegationKind::OtherThan, token_text(&token)),
                    );
                    continue;
                }
                pending_number_tokens.clear();
                pending_number_is_spatial = false;
                pending_recurrence_connection = None;
                next_property_is_spatial = false;
                continue;
            }
            if token.is_selmaho(Selmaho::Pa) {
                if pending_number_tokens.is_empty() {
                    pending_number_is_spatial = next_property_is_spatial;
                }
                pending_number_tokens.push(token);
                pending_recurrence_index = None;
                continue;
            }
            let number_tokens = std::mem::take(&mut pending_number_tokens);
            if let Some(kind) = generated_recurrence_kind_for_interval_marker(&token) {
                let introduced_by = token_text(&token);
                let connection = pending_recurrence_connection.take();
                let quantity = if number_tokens.is_empty() {
                    None
                } else {
                    let text = token_list_text(number_tokens.iter());
                    let value = parse_generated_recurrence_integer(&number_tokens, &text);
                    let quantity = if let Some(value) = value {
                        let cache_key = GeneratedRecurrenceQuantityCacheKey::new(
                            kind,
                            introduced_by.clone(),
                            connection.clone(),
                            GeneratedRecurrenceQuantityCacheValue::parsed_integer(
                                text.clone(),
                                value,
                            ),
                            None,
                        );
                        let quantity = if let Some(quantity) =
                            quantity_cache.get(&cache_key).copied()
                        {
                            quantity
                        } else {
                            let quantity =
                                self.build_recurrence_quantity_for_generated_integer(&text, value)?;
                            quantity_cache.insert(cache_key, quantity);
                            quantity
                        };
                        new!(GeneratedRecurrenceQuantity::Object(quantity))
                    } else {
                        new!(GeneratedRecurrenceQuantity::Value(QuantityValue::text(
                            text
                        )))
                    };
                    Some(quantity)
                };
                let spatial = pending_number_is_spatial || next_property_is_spatial;
                let recurrence_interval = if spatial {
                    spatial_eventuality_interval
                } else {
                    eventuality_interval
                };
                let recurrence = match quantity.map(GeneratedRecurrenceQuantity::into_data) {
                    Some(data!(GeneratedRecurrenceQuantity::Object(quantity))) => {
                        Recurrence::new_with_quantity(
                            kind,
                            introduced_by,
                            connection,
                            quantity,
                            recurrence_interval,
                            None,
                            None,
                        )
                    }
                    Some(data!(GeneratedRecurrenceQuantity::Value(value))) => Recurrence::new(
                        kind,
                        introduced_by,
                        connection,
                        Some(value),
                        recurrence_interval,
                        None,
                        None,
                    ),
                    None => Recurrence::new(
                        kind,
                        introduced_by,
                        connection,
                        None,
                        recurrence_interval,
                        None,
                        None,
                    ),
                };
                pending_recurrence_index = Some(push_generated_recurrence_event_modifier(
                    &mut modifiers,
                    recurrence,
                    spatial,
                ));
                pending_number_is_spatial = false;
                next_property_is_spatial = false;
                continue;
            }
            if let Some(contour) = aspect_contour_for_zaho_token(&token) {
                pending_recurrence_index = None;
                pending_recurrence_connection = None;
                let spatial = next_property_is_spatial;
                let anchor = if spatial {
                    spatial_eventuality_interval
                } else {
                    eventuality_interval
                };
                let aspect =
                    Aspect::new_with_polarity(contour.clone(), anchor, scalar_negation.clone());
                let interval_modifier =
                    new!(IntervalModifier::Aspect(Aspect::new(contour, anchor)));
                if spatial {
                    modifiers.spatial_aspects.push(aspect);
                    modifiers.spatial_interval_modifiers.push(interval_modifier);
                } else {
                    modifiers.temporal_aspects.push(aspect);
                    modifiers
                        .temporal_interval_modifiers
                        .push(interval_modifier);
                }
                pending_number_is_spatial = false;
                next_property_is_spatial = false;
                continue;
            }
            pending_number_is_spatial = false;
            pending_recurrence_index = None;
            pending_recurrence_connection = None;
            if !token.is_selmaho(Selmaho::Pa) {
                next_property_is_spatial = false;
            }
        }
        Ok(modifiers)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_argument_for_generated_term(
        &mut self,
        term: &'tree TermSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        let mut visible_arguments = BTreeMap::new();
        let mut place_questions = Vec::new();
        let mut modal_terms = Vec::new();
        let mut formula_scopes = Vec::new();
        let mut coequal_scope_groups = Vec::new();
        let mut term_formula_scopes = Vec::new();
        let mut next_visible_place = 1;
        self.insert_generated_term_assignment(
            &mut visible_arguments,
            &mut place_questions,
            &mut modal_terms,
            &mut formula_scopes,
            &mut coequal_scope_groups,
            &mut term_formula_scopes,
            &mut next_visible_place,
            term,
        )?;
        if !modal_terms.is_empty() {
            return Err(unsupported("modal term as referential argument"));
        }
        if !place_questions.is_empty() {
            return Err(unsupported("place-question term as referential argument"));
        }
        if !formula_scopes.is_empty() {
            return Err(unsupported("scoped term as referential argument"));
        }
        if !coequal_scope_groups.is_empty() {
            return Err(unsupported("coequal-scoped term as referential argument"));
        }
        if !term_formula_scopes.is_empty() {
            return Err(unsupported("formula-scoped term as referential argument"));
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| crate::model::argument_object_kind_can_fill(id.object_kind()))) || ret.is_err())]
    pub(super) fn build_terms_fragment_content(
        &mut self,
        fragment: &'tree TermsFragmentSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let [term] = fragment.terms.as_slice() else {
            return Ok(None);
        };
        if let Ok(SimpleTermSyntax::TaggedSumtiTerm(term)) =
            generated_simple_term_for_assignment(term)
        {
            return self
                .build_generated_tense_modal_fragment_content(
                    term.tense_modal.as_ref(),
                    term.sumti.as_ref(),
                )
                .map(Some);
        }
        match generated_simple_term_for_assignment(term) {
            Ok(SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti))) => {
                if let Some(sign) = self.build_generated_letteral_sign_for_sumti(sumti)? {
                    return Ok(Some(sign));
                }
                let argument_object = self.build_sumti_referent(sumti)?;
                if argument_object.object_kind() == crate::model::SemanticObjectKind::Referent {
                    self.attach_generated_relative_clauses_to_referent(argument_object, sumti)?;
                }
                return Ok(Some(argument_object));
            }
            Ok(SimpleTermSyntax::PlaceTaggedSumtiTerm(term)) => {
                if let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() {
                    if let Some(sign) = self.build_generated_letteral_sign_for_sumti(sumti)? {
                        return Ok(Some(sign));
                    }
                    let argument_object = self.build_sumti_referent(sumti)?;
                    if argument_object.object_kind() == crate::model::SemanticObjectKind::Referent {
                        self.attach_generated_relative_clauses_to_referent(argument_object, sumti)?;
                    }
                    return Ok(Some(argument_object));
                }
            }
            _ => {}
        }
        self.build_term_argument_object(term).map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.as_ref().is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign))) || ret.is_err())]
    pub(super) fn build_generated_letteral_sign_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Ok(SumtiBaseSyntax::LerfuStringSumti(letters)) = simple_sumti_base_from_sumti(sumti)
        else {
            return Ok(None);
        };
        let tokens = generated_letter_string_tokens(&letters.words);
        if tokens.is_empty() {
            return Ok(None);
        }
        let sign =
            self.build_generated_letteral_sign(&tokens, self.source_for_node(sumti, "letteral"))?;
        self.attach_subscript_from_free_modifiers(sign, &letters.free_modifiers)?;
        Ok(Some(sign))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_generated_connective_fragment_sign<N: TreeNode>(
        &mut self,
        fragment: &N,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_connective_sign(fragment, "connective-fragment")
    }

    #[requires(!source_construct.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_generated_connective_sign<N: TreeNode>(
        &mut self,
        fragment: &N,
        source_construct: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let tokens = self.tokens_for_node(fragment);
        if tokens.is_empty() {
            return Err(unsupported("empty connective fragment"));
        }
        let sign = self.next_sign_id();
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Connective,
                token_list_text(tokens.iter()),
                self.source_for_tokens(&tokens, source_construct),
                Vec::new(),
            ),
        )?;
        Ok(sign)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| semantic_id_is_eventuality(*id)) || ret.is_err())]
    pub(super) fn build_generated_tense_modal_fragment_content<N: TreeNode>(
        &mut self,
        tense_modal: &N,
        sumti: &'tree TaggedOrElidedSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_eventuality_id();
        let mut event = SemanticObject::referential_eventuality(
            EventualityClass::Event,
            None,
            self.source_for_node(tense_modal, "tense-modal-fragment"),
        );
        if generated_tense_modal_has_event_modifier(tense_modal) {
            let anchor = match sumti {
                TaggedOrElidedSumtiSyntax::Sumti(sumti) => Some(self.build_sumti_referent(sumti)?),
                TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => None,
            };
            let time_span = generated_time_span_for_tense_modal(tense_modal, anchor);
            for (domain, relation, introduced_by) in
                generated_anchor_relations_with_introducers_for_tense_modal(tense_modal)
            {
                if time_span.is_some() && domain == GeneratedAnchorDomain::Time {
                    continue;
                }
                let mut relation = self.with_default_anchor_for_generated_tense(domain, relation);
                if let Some(anchor) = anchor {
                    relation = relation.with_data(data! { anchor: anchor });
                }
                apply_generated_anchor_relation_to_event(
                    &mut event,
                    domain,
                    relation,
                    introduced_by,
                    anchor.is_some(),
                );
            }
            if let Some(time_span) = time_span {
                event.update_eventuality(|node| {
                    node.with_data(data! { time_span: Some(time_span) })
                });
            }
            let scalar_negation = generated_modal_scalar_negation_for_tense_modal(tense_modal);
            if let Some(actuality) = generated_actuality_for_tense_modal(tense_modal) {
                event.update_eventuality(|node| {
                    node.with_data(data! { actuality: Some(actuality) })
                });
            }
            if let Some(parameter) =
                self.build_generated_tense_question_parameter_for_tense_modal(tense_modal)?
            {
                event.update_eventuality(|node| {
                    node.with_data(data! { tense_modal: Some(parameter) })
                });
            }
            if let Some(time_interval) =
                generated_time_interval_for_tense_modal(tense_modal, anchor)
            {
                event.update_eventuality(|node| {
                    node.with_data(data! { time_interval: Some(time_interval) })
                });
            }
            if let Some(space_interval) =
                generated_space_interval_for_tense_modal(tense_modal, anchor)
            {
                event.update_eventuality(|node| {
                    node.with_data(data! { space_interval: Some(space_interval) })
                });
            }
            let recurrence_modifiers = self.build_generated_recurrence_event_modifiers(
                tense_modal,
                anchor,
                anchor,
                scalar_negation,
            )?;
            apply_generated_aspects_to_event(
                &mut event,
                recurrence_modifiers.temporal_aspects,
                false,
            );
            event.update_eventuality(|node| {
                let mut data = node.into_data();
                data.recurrence
                    .extend(recurrence_modifiers.temporal_recurrences);
                data.interval_modifiers
                    .extend(recurrence_modifiers.temporal_interval_modifiers);
                data.spatial_recurrence
                    .extend(recurrence_modifiers.spatial_recurrences);
                data.spatial_interval_modifiers
                    .extend(recurrence_modifiers.spatial_interval_modifiers);
                EventualityNode::from_data(data)
            });
            apply_generated_aspects_to_event(
                &mut event,
                recurrence_modifiers.spatial_aspects,
                true,
            );
        } else if let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        {
            let argument = self.build_tagged_or_elided_sumti_argument(sumti)?;
            let arguments = self.modal_argument_map_for_visible_place(
                argument,
                visible_place,
                relation_place_count(self.dictionary, &relation),
            )?;
            let modal_argument = self.generated_modal_argument_with_tense_modal_modifiers(
                tense_modal,
                relation,
                introduced_by,
                arguments,
                generated_modal_negation_for_tense_modal(tense_modal),
                generated_modal_scalar_negation_for_tense_modal(tense_modal),
                "modal-fragment",
            );
            event.update_eventuality(|node| {
                let mut data = node.into_data();
                data.modal_arguments.push(modal_argument);
                EventualityNode::from_data(data)
            });
        } else {
            event.push_diagnostic(diagnostic(
                "tense/modal fragment has no implemented semantic value",
            ));
        }
        normalize_generated_event_time_path(&mut event);
        normalize_generated_event_space_path(&mut event);
        self.insert(id, event)?;
        Ok(id)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tense_modal_event_modifier_to_eventuality(
        &mut self,
        eventuality: SemanticObjectId,
        tense_modal: &'tree TenseModalSyntax,
        anchor: Option<SemanticObjectId>,
    ) -> Result<(), SemanticsError> {
        self.record_generated_tense_modal_event_modifier(eventuality, tense_modal, anchor)?;
        self.flush_generated_event_modifiers_with_recurrence_quantity_promotion(eventuality)
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
    #[ensures(true)]
    pub(super) fn apply_generated_tense_modal_event_modifier_to_eventuality_now<N: TreeNode>(
        &mut self,
        eventuality: SemanticObjectId,
        tense_modal: &N,
        anchor: Option<SemanticObjectId>,
    ) -> Result<(), SemanticsError> {
        if generated_tense_modal_resets_sticky_tense(tense_modal) {
            let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                invalid_graph(format!("missing generated eventuality {eventuality}"))
            })?;
            clear_generated_event_time_path(event);
            clear_generated_event_space_path(event);
            self.apply_generated_sticky_event_update(GeneratedStickyEventUpdate {
                reset: true,
                ..GeneratedStickyEventUpdate::default()
            });
            return Ok(());
        }
        if generated_tense_modal_anchors_to_speech_time(tense_modal) {
            let actuality = generated_actuality_for_tense_modal(tense_modal);
            let current_now = self.current_now();
            let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                invalid_graph(format!("missing generated eventuality {eventuality}"))
            })?;
            clear_generated_event_time_path(event);
            let time = new!(AnchorRelation {
                relation: "at".to_owned(),
                anchor: current_now,
                sticky: false,
                inherited: None,
                distance: None,
                magnitude: None,
                scalar_negation: None,
                motion: None,
            });
            event.update_eventuality(|node| {
                let mut data = node.into_data();
                data.time = Some(time);
                if actuality.is_some() {
                    data.actuality = actuality;
                }
                EventualityNode::from_data(data)
            });
            return Ok(());
        }
        let anchor_was_explicit = anchor.is_some();
        let story_temporal_anchor = if anchor.is_none()
            && self.options.story_time
            && generated_tense_modal_has_story_time_temporal_modifier(tense_modal)
        {
            self.story_time_anchor
        } else {
            None
        };
        if story_temporal_anchor.is_some() {
            let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                invalid_graph(format!("missing generated eventuality {eventuality}"))
            })?;
            clear_generated_inherited_event_time_path(event);
        }
        let temporal_anchor = anchor
            .or(story_temporal_anchor)
            .or_else(|| self.current_temporal_context());
        let spatial_anchor = anchor;
        let time_span = generated_time_span_for_tense_modal(tense_modal, temporal_anchor);
        let anchor_relations =
            generated_anchor_relations_with_introducers_for_tense_modal(tense_modal)
                .into_iter()
                .filter(|(domain, _, _)| {
                    !(time_span.is_some() && *domain == GeneratedAnchorDomain::Time)
                })
                .map(|(domain, relation, introduced_by)| {
                    let mut relation =
                        self.with_default_anchor_for_generated_tense(domain, relation);
                    let explicit_anchor = match domain {
                        GeneratedAnchorDomain::Time => temporal_anchor,
                        GeneratedAnchorDomain::Space => spatial_anchor,
                    };
                    if let Some(anchor) = explicit_anchor {
                        relation = relation.with_data(data! { anchor: anchor });
                    }
                    (domain, relation, introduced_by, anchor_was_explicit)
                })
                .collect::<Vec<_>>();
        let scalar_negation = generated_modal_scalar_negation_for_tense_modal(tense_modal);
        let actuality = generated_actuality_for_tense_modal(tense_modal);
        let time_interval = generated_time_interval_for_tense_modal(tense_modal, temporal_anchor);
        let space_interval = generated_space_interval_for_tense_modal(tense_modal, spatial_anchor);
        let tense_question_parameter =
            self.build_generated_tense_question_parameter_for_tense_modal(tense_modal)?;
        let recurrence_modifiers = self.build_generated_recurrence_event_modifiers(
            tense_modal,
            temporal_anchor,
            spatial_anchor,
            scalar_negation,
        )?;
        let sticky_update = {
            let event = self.objects.get_mut(&eventuality).ok_or_else(|| {
                invalid_graph(format!("missing generated eventuality {eventuality}"))
            })?;
            if let Some(actuality) = actuality {
                event.update_eventuality(|node| {
                    node.with_data(data! { actuality: Some(actuality) })
                });
            }
            if let Some(parameter) = tense_question_parameter {
                event.update_eventuality(|node| {
                    node.with_data(data! { tense_modal: Some(parameter) })
                });
            }
            for (domain, relation, introduced_by, explicit_anchor) in anchor_relations {
                apply_generated_anchor_relation_to_event(
                    event,
                    domain,
                    relation,
                    introduced_by,
                    explicit_anchor,
                );
            }
            if let Some(time_interval) = time_interval {
                event.update_eventuality(|node| {
                    node.with_data(data! { time_interval: Some(time_interval) })
                });
            }
            if let Some(time_span) = time_span {
                event.update_eventuality(|node| {
                    node.with_data(data! { time_span: Some(time_span) })
                });
            }
            if let Some(space_interval) = space_interval {
                event.update_eventuality(|node| {
                    node.with_data(data! { space_interval: Some(space_interval) })
                });
            }
            apply_generated_aspects_to_event(event, recurrence_modifiers.temporal_aspects, false);
            event.update_eventuality(|node| {
                let mut data = node.into_data();
                data.recurrence
                    .extend(recurrence_modifiers.temporal_recurrences);
                data.interval_modifiers
                    .extend(recurrence_modifiers.temporal_interval_modifiers);
                data.spatial_recurrence
                    .extend(recurrence_modifiers.spatial_recurrences);
                data.spatial_interval_modifiers
                    .extend(recurrence_modifiers.spatial_interval_modifiers);
                EventualityNode::from_data(data)
            });
            apply_generated_aspects_to_event(event, recurrence_modifiers.spatial_aspects, true);
            let mut update = GeneratedStickyEventUpdate::default();
            if generated_tense_modal_makes_tense_sticky(tense_modal) {
                mark_generated_event_time_sticky(event, None);
                update.time_path = Some(generated_event_time_path_for_sticky_storage(event));
            }
            if generated_tense_modal_makes_space_sticky(tense_modal) {
                mark_generated_event_space_sticky(event, None);
                update.space_path = Some(generated_event_space_path_for_sticky_storage(event));
            }
            normalize_generated_event_time_path(event);
            normalize_generated_event_space_path(event);
            update
        };
        self.apply_generated_sticky_event_update(sticky_update);
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn split_generated_fai_terms<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
    ) -> Result<
        (
            Vec<&'syntax TermSyntax>,
            Vec<&'syntax TaggedOrElidedSumtiSyntax>,
        ),
        SemanticsError,
    > {
        let mut ordinary_terms = Vec::new();
        let mut fai_sumti = Vec::new();
        for term in terms {
            if let Ok(SimpleTermSyntax::PlaceTaggedSumtiTerm(place_tagged)) =
                generated_simple_term_for_assignment(term)
                && place_tagged.fa.value.cmavo() == Some(Cmavo::Fai)
            {
                fai_sumti.push(place_tagged.sumti.as_ref());
                continue;
            }
            ordinary_terms.push(term);
        }
        Ok((ordinary_terms, fai_sumti))
    }

    #[requires(atom_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(crate::model::argument_object_kind_can_fill(moved_operand.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(raised_operand.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn conjoin_generated_bare_jai_involvement_formula(
        &mut self,
        atom_formula: SemanticObjectId,
        moved_operand: SemanticObjectId,
        raised_operand: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let involvement = self.build_generated_binary_constructed_relation_formula(
            "involves",
            moved_operand,
            raised_operand,
            source.clone(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![atom_formula, involvement],
                Some(new!(Connector {
                    source: "jai".to_owned(),
                    locus: "bare-jai-raised-participant".to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(!relation.is_empty())]
    #[requires(crate::model::argument_object_kind_can_fill(x1.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(x2.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_binary_constructed_relation_formula(
        &mut self,
        relation: &str,
        x1: SemanticObjectId,
        x2: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(x1, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(x2, None));
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.to_owned(),
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

    #[requires(crate::model::argument_object_kind_can_fill(referent.object_kind()))]
    #[ensures(true)]
    pub(super) fn generated_referent_is_abstraction_about_operand(
        &self,
        referent: SemanticObjectId,
        word: &str,
        operand: SemanticObjectId,
    ) -> bool {
        self.objects.get(&referent).is_some_and(|object| {
            object.descriptor().is_some_and(|descriptor| {
                descriptor.kind == DescriptorKind::AbstractionAbout
                    && descriptor.word == word
                    && descriptor.operand == Some(operand)
            })
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_cached_sumti_referent_for_node<N: TreeNode>(
        &mut self,
        node: &N,
        build: impl FnOnce(&mut Self) -> Result<SemanticObjectId, SemanticsError>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let cache_key = (self.sumti_referent_cache_bypass_depth == 0)
            .then(|| self.source_key_for_node(node))
            .flatten();
        if let Some(cache_key) = cache_key
            && let Some(referent) = self.sumti_referents.get(&cache_key).copied()
        {
            return Ok((referent, false));
        }
        let referent = build(self)?;
        if let Some(cache_key) = cache_key {
            self.sumti_referents.insert(cache_key, referent);
        }
        Ok((referent, true))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_sumti_referent<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(vuho_attachment) = &sumti.vuho_attachment {
            match vuho_attachment {
                VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(tail)
                    if tail.sumti_connection.is_some() =>
                {
                    return Err(unsupported("VUhO sumti relative connection"));
                }
                VuhoSumtiAttachmentTailSyntax::VuhoConnectedSumtiAttachmentTail(_) => {
                    return Err(unsupported("VUhO connected sumti"));
                }
                VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(_) => {}
            }
        }
        let (id, built) = self.build_cached_sumti_referent_for_node(sumti, |builder| {
            if let Some(referent) = builder.build_generated_goi_associated_referent(sumti)? {
                Ok(referent)
            } else {
                builder.build_sumti_grouped_referent(&sumti.base_sumti)
            }
        })?;
        if !built {
            return Ok(id);
        }
        if id.object_kind() == crate::model::SemanticObjectKind::Referent
            && generated_sumti_has_current_kau_focus(sumti)
        {
            self.record_generated_indirect_question_focus(
                GeneratedIndirectQuestionFocus::from_data(data!(GeneratedIndirectQuestionFocus {
                    focus: id,
                    presupposed_answer: Some(id),
                    slots: Vec::new(),
                    kind: QuestionKind::Argument,
                    domain: SemanticSort::Entity,
                    source: self.source_for_node(sumti, "indirect-question"),
                })),
            );
        }
        if let Some(anchor) = self.current_utterance
            && displayed_content_target_kind_is_allowed(id.object_kind())
            && !generated_sumti_connection_has_branch_indicator_attachment(sumti)
        {
            self.attach_generated_indicator_displays_with_target_focus(
                indicator_parts_for_generated_node(sumti),
                id,
                anchor,
                "indicator",
                None,
                false,
            )?;
        }
        if id.object_kind() == crate::model::SemanticObjectKind::Referent
            && generated_sumti_is_direct_anaphora_candidate(sumti)
        {
            self.record_generated_letter_sumti_antecedent(sumti, id);
            if let Some(source_key) = self.source_key_for_node(sumti) {
                self.recent_sumti_referents
                    .push(new!(GeneratedRecentSumtiReferent {
                        source_key,
                        referent: id,
                    }));
            }
        }
        Ok(id)
    }

    #[requires(offset > 0)]
    #[ensures(ret.as_ref().is_none_or(|referent| referent.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn recent_sumti_referent_before_node<N: TreeNode>(
        &self,
        node: &N,
        offset: usize,
    ) -> Option<SemanticObjectId> {
        let (node_start, _) = self.source_key_for_node(node)?;
        let mut candidates = self
            .recent_sumti_referents
            .iter()
            .enumerate()
            .filter(|(_, mention)| mention.source_key.1 <= node_start)
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(index, mention)| (mention.source_key, *index));
        candidates
            .into_iter()
            .rev()
            .nth(offset - 1)
            .map(|(_, mention)| mention.referent)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn record_generated_letter_sumti_antecedent<'syntax>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
        referent: SemanticObjectId,
    ) {
        let Some(source_key) = self.source_key_for_node(sumti) else {
            return;
        };
        for key in generated_argument_letter_keys(sumti) {
            self.letter_sumti_referents
                .entry(key.clone())
                .or_default()
                .push(new!(GeneratedLetterSumtiReferent {
                    key,
                    source_key,
                    referent,
                }));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_argument_for_generated_sumti<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
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
            if generated_prenex_da_series_pro_sumti_from_sumti(sumti).is_some() {
                self.add_generated_implicit_existential_restrictions(
                    referent,
                    relative_clauses
                        .iter()
                        .map(|relative_clause| relative_clause.body)
                        .collect(),
                );
            }
            if !relative_clauses.is_empty() {
                argument = argument.with_relative_clauses(relative_clauses);
            }
        }
        if referent.object_kind() == crate::model::SemanticObjectKind::Referent {
            let occurrence_relative_clauses =
                self.lower_generated_occurrence_relative_clauses_for_sumti(sumti, referent)?;
            if !occurrence_relative_clauses.is_empty() {
                argument = append_generated_relative_clauses_to_argument(
                    argument,
                    occurrence_relative_clauses,
                );
            }
        }
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.kind != ArgumentValueKind::Deleted) || ret.is_err())]
    pub(super) fn attach_generated_relative_clauses_to_argument(
        &mut self,
        argument: ArgumentValue,
        relative_clauses: &'tree RelativeClauseListSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        if argument.kind == ArgumentValueKind::Deleted {
            return Err(invalid_graph(
                "cannot attach generated relative clauses to a deleted argument".to_owned(),
            ));
        }
        let Some(head) = argument.value else {
            return Err(invalid_graph(
                "cannot attach generated relative clauses to an argument with no referent"
                    .to_owned(),
            ));
        };
        if !crate::model::argument_object_kind_can_fill(head.object_kind()) {
            return Err(invalid_graph(format!(
                "cannot attach generated relative clauses to non-argument object {head}"
            )));
        }
        let lowered = self.lower_generated_relative_clause_list(relative_clauses, head)?;
        if lowered.is_empty() {
            return Ok(argument);
        }
        if argument.relative_clauses.is_empty() {
            return Ok(argument.with_relative_clauses(lowered));
        }
        let data = argument.into_data();
        let mut all_relative_clauses = data.relative_clauses;
        all_relative_clauses.extend(lowered);
        Ok(ArgumentValue::from_data(data!(ArgumentValue {
            relative_clauses: all_relative_clauses,
            ..data
        })))
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn lower_generated_occurrence_relative_clauses_for_sumti<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError> {
        let mut lists = Vec::new();
        generated_occurrence_relative_clause_lists_for_sumti(sumti, &mut lists);
        let mut lowered = Vec::new();
        for list in lists {
            lowered.extend(self.lower_generated_occurrence_relative_clause_list(list, head)?);
        }
        Ok(lowered)
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn lower_generated_occurrence_relative_clause_list<'syntax: 'tree>(
        &mut self,
        relative_clauses: &'syntax RelativeClauseListSyntax,
        head: SemanticObjectId,
    ) -> Result<Vec<RelativeClause>, SemanticsError> {
        let mut lowered = Vec::new();
        if let RelativeClauseAtomSyntax::BridiRelativeClause(clause) = &relative_clauses.first {
            lowered.push(self.lower_generated_bridi_relative_clause(clause, head)?);
        }
        for tail in &relative_clauses.additional {
            let atom = match tail {
                RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => tail.inner.as_ref(),
                RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => tail.inner.as_ref(),
            };
            if let RelativeClauseAtomSyntax::BridiRelativeClause(clause) = atom {
                lowered.push(self.lower_generated_bridi_relative_clause(clause, head)?);
            }
        }
        Ok(lowered)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn attach_generated_relative_clauses_to_referent(
        &mut self,
        head: SemanticObjectId,
        sumti: &'tree SumtiSyntax,
    ) -> Result<(), SemanticsError> {
        let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) else {
            return Ok(());
        };
        let lowered = self.lower_generated_relative_clause_list(relative_clauses, head)?;
        if lowered.is_empty() {
            return Ok(());
        }
        let object = self.objects.get_mut(&head).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find relative-clause head {head}"
            ))
        })?;
        object.extend_relative_clauses(lowered);
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_argument_for_generated_sumti_with_formula_scopes<'syntax: 'tree>(
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
        let selection = self.generated_requantified_da_source_for_sumti(sumti, formula_scopes);
        let referent = if selection.is_some() {
            self.build_plain_scoped_argument_variable_for_generated_sumti(sumti)?
        } else {
            self.build_scoped_argument_variable_for_generated_sumti(sumti)?
        };
        let (
            source_variable,
            selection_source,
            source_restriction_nodes,
            source_restriction_formulas,
            inherited_restrictions,
        ) = if let Some(selection) = selection {
            (
                Some(selection.variable),
                Some(SelectionSource::witness_set(selection.variable)),
                selection.restriction_nodes,
                selection.restriction_formulas,
                Vec::new(),
            )
        } else {
            (None, None, Vec::new(), Vec::new(), Vec::new())
        };
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
        let mut relative_clause_restrictions = Vec::new();
        if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
            let relative_clauses =
                self.lower_generated_relative_clause_list(relative_clauses, referent)?;
            relative_clause_restrictions.extend(
                relative_clauses
                    .iter()
                    .map(|relative_clause| relative_clause.body),
            );
            if !relative_clauses.is_empty() {
                argument = argument.with_relative_clauses(relative_clauses);
            }
        }
        if referent.object_kind() == crate::model::SemanticObjectKind::Referent {
            let occurrence_relative_clauses =
                self.lower_generated_occurrence_relative_clauses_for_sumti(sumti, referent)?;
            relative_clause_restrictions.extend(
                occurrence_relative_clauses
                    .iter()
                    .map(|relative_clause| relative_clause.body),
            );
            if !occurrence_relative_clauses.is_empty() {
                argument = append_generated_relative_clauses_to_argument(
                    argument,
                    occurrence_relative_clauses,
                );
            }
        }
        formula_scopes.push(GeneratedArgumentQuantifierScope {
            node: GeneratedArgumentQuantifierScopeNode::Sumti(sumti),
            source: scope_source,
            variable: referent,
            source_variable,
            selection_source,
            source_restriction_nodes,
            source_restriction_formulas,
            inherited_restrictions,
            relative_clause_restrictions,
        });
        Ok(argument)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_scoped_argument_variable_for_generated_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(key) = self.source_key_for_node(sumti)
            && let Some(id) = self.scoped_argument_variables.get(&key)
        {
            return Ok(*id);
        }
        if let Some(pro_sumti) = generated_quantified_da_series_pro_sumti_from_sumti(sumti) {
            let id = self.build_scoped_generated_pro_sumti_variable(
                pro_sumti,
                generated_sumti_quantified_variable_sort(sumti),
            )?;
            if let Some(key) = self.source_key_for_node(sumti) {
                self.scoped_argument_variables.insert(key, id);
            }
            return Ok(id);
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

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_plain_scoped_argument_variable_for_generated_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
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

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|binding| binding.variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn generated_requantified_da_source_for_sumti<'syntax>(
        &self,
        sumti: &'syntax SumtiSyntax,
        formula_scopes: &[GeneratedArgumentQuantifierScope<'syntax>],
    ) -> Option<GeneratedDaSeriesScopeBinding<'syntax>> {
        let word = token_text(
            &generated_quantified_da_series_pro_sumti_from_sumti(sumti)?
                .0
                .value,
        );
        self.generated_requantified_da_source_for_word(&word, formula_scopes)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|binding| binding.variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn generated_requantified_da_source_for_sumti_bound<'syntax>(
        &self,
        sumti: &'syntax SumtiBoundSyntax,
        formula_scopes: &[GeneratedArgumentQuantifierScope<'syntax>],
    ) -> Option<GeneratedDaSeriesScopeBinding<'syntax>> {
        let word = token_text(
            &generated_quantified_da_series_pro_sumti_from_sumti_bound(sumti)?
                .0
                .value,
        );
        self.generated_requantified_da_source_for_word(&word, formula_scopes)
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_none_or(|binding| binding.variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn generated_requantified_da_source_for_word<'syntax>(
        &self,
        word: &str,
        formula_scopes: &[GeneratedArgumentQuantifierScope<'syntax>],
    ) -> Option<GeneratedDaSeriesScopeBinding<'syntax>> {
        formula_scopes
            .iter()
            .rev()
            .find_map(|scope| {
                let scope_word = generated_da_series_word_for_argument_scope(scope)?;
                (scope_word == word).then(|| generated_da_series_scope_binding_from_scope(scope))
            })
            .or_else(|| {
                self.quantified_da_series_bindings
                    .get(word)
                    .cloned()
                    .map(|binding| GeneratedDaSeriesScopeBinding {
                        variable: binding.variable,
                        restriction_nodes: Vec::new(),
                        restriction_formulas: binding.restriction_formulas,
                    })
            })
            .or_else(|| {
                self.implicit_da_series_bindings
                    .get(word)
                    .copied()
                    .map(|variable| GeneratedDaSeriesScopeBinding {
                        variable,
                        restriction_nodes: Vec::new(),
                        restriction_formulas: Vec::new(),
                    })
            })
    }

    #[requires(restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(true)]
    pub(super) fn record_generated_quantified_da_series_binding(
        &mut self,
        scope: &GeneratedArgumentQuantifierScope<'_>,
        restrictions: &[SemanticObjectId],
    ) {
        let Some(word) = generated_da_series_word_for_argument_scope(scope) else {
            return;
        };
        self.quantified_da_series_bindings.insert(
            word,
            GeneratedSemanticDaSeriesScopeBinding {
                variable: scope.variable,
                restriction_formulas: restrictions.to_vec(),
            },
        );
    }

    #[requires(binding.variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn lower_generated_da_series_scope_binding_restrictions(
        &mut self,
        binding: &GeneratedDaSeriesScopeBinding<'tree>,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let reservation_start = self.pending_after_eventuality_reservations;
        if !binding.restriction_nodes.is_empty() {
            self.pending_after_eventuality_reservations += 1;
        }
        let restrictions = self.lower_generated_argument_scope_node_relative_restrictions(
            &binding.restriction_nodes,
            variable,
        );
        if self.pending_after_eventuality_reservations > reservation_start {
            self.pending_after_eventuality_reservations -= 1;
        }
        restrictions
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn lower_generated_argument_scope_source_restriction_nodes(
        &mut self,
        scope: &GeneratedArgumentQuantifierScope<'tree>,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        self.lower_generated_argument_scope_node_relative_restrictions(
            &scope.source_restriction_nodes,
            variable,
        )
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn lower_generated_argument_scope_node_relative_restrictions(
        &mut self,
        nodes: &[GeneratedArgumentQuantifierScopeNode<'tree>],
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut restrictions = Vec::new();
        for node in nodes {
            match *node {
                GeneratedArgumentQuantifierScopeNode::Sumti(sumti) => {
                    if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
                        restrictions.extend(
                            self.lower_generated_relative_clause_list(relative_clauses, variable)?
                                .into_iter()
                                .map(|relative_clause| relative_clause.body),
                        );
                    }
                }
                GeneratedArgumentQuantifierScopeNode::SumtiBound(sumti) => {
                    if let Some(relative_clauses) =
                        generated_sumti_bound_relative_clause_list(sumti)
                    {
                        restrictions.extend(
                            self.lower_generated_relative_clause_list(relative_clauses, variable)?
                                .into_iter()
                                .map(|relative_clause| relative_clause.body),
                        );
                    }
                }
            }
        }
        Ok(restrictions)
    }

    #[requires(restriction_formulas.iter().all(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[requires(source_variable.is_none_or(|variable| variable.object_kind() == crate::model::SemanticObjectKind::Referent))]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|formulas| formulas.iter().all(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn clone_generated_restriction_formulas_for_variable(
        &mut self,
        restriction_formulas: &[SemanticObjectId],
        source_variable: Option<SemanticObjectId>,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        if restriction_formulas.is_empty() {
            return Ok(Vec::new());
        }
        let Some(source_variable) = source_variable else {
            return Err(invalid_graph(
                "generated da-series restriction formulas require a source variable".to_owned(),
            ));
        };
        let mut cloned = BTreeMap::new();
        let reservation_start = self.pending_after_eventuality_reservations;
        self.pending_after_eventuality_reservations += 1;
        let formulas = restriction_formulas
            .iter()
            .map(|formula| {
                self.clone_generated_formula_replacing_referent(
                    *formula,
                    source_variable,
                    variable,
                    &mut cloned,
                )
            })
            .collect();
        if self.pending_after_eventuality_reservations > reservation_start {
            self.pending_after_eventuality_reservations -= 1;
        }
        formulas
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(replacements.values().all(|replacement| crate::model::argument_object_kind_can_fill(replacement.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn clone_generated_formula_with_argument_replacements(
        &mut self,
        formula: SemanticObjectId,
        replacements: &BTreeMap<SemanticObjectId, SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(None);
        };
        match object.as_formula().map(FormulaNode::as_data) {
            Some(data!(FormulaNode::Atom(atom))) => {
                let predication = atom.predication;
                let Some(cloned_predication) = self
                    .clone_generated_predication_with_argument_replacements(
                        predication,
                        replacements,
                    )?
                else {
                    return Ok(None);
                };
                let cloned_formula = self.next_formula_id();
                self.insert(
                    cloned_formula,
                    SemanticObject::atom_formula(
                        cloned_predication,
                        object.source().cloned(),
                        object.diagnostics().to_vec(),
                    ),
                )?;
                Ok(Some(cloned_formula))
            }
            Some(data!(FormulaNode::Connective(connective)))
                if matches!(
                    connective.operator,
                    FormulaOperator::And
                        | FormulaOperator::Or
                        | FormulaOperator::Iff
                        | FormulaOperator::WhetherOrNot
                ) =>
            {
                let mut children = Vec::with_capacity(connective.children.len());
                for child in &connective.children {
                    let Some(cloned_child) = self
                        .clone_generated_formula_with_argument_replacements(*child, replacements)?
                    else {
                        return Ok(None);
                    };
                    children.push(cloned_child);
                }
                let cloned_formula = self.next_formula_id();
                self.insert(
                    cloned_formula,
                    SemanticObject::connective_formula(
                        connective.operator,
                        children,
                        connective.connector.clone(),
                        object.source().cloned(),
                        object.diagnostics().to_vec(),
                    ),
                )?;
                Ok(Some(cloned_formula))
            }
            _ => Ok(None),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(replacements.values().all(|replacement| crate::model::argument_object_kind_can_fill(replacement.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|predication| predication.is_none_or(|predication| predication.object_kind() == crate::model::SemanticObjectKind::Predication)) || ret.is_err())]
    pub(super) fn clone_generated_predication_with_argument_replacements(
        &mut self,
        predication: SemanticObjectId,
        replacements: &BTreeMap<SemanticObjectId, SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(object) = self.objects.get(&predication).cloned() else {
            return Ok(None);
        };
        let Some(node) = object.as_predication() else {
            return Ok(None);
        };
        let relation = match node.relation.as_data() {
            data!(PredicationRelation::Named { relation }) => relation.clone(),
            data!(PredicationRelation::Parameter { .. }) => return Ok(None),
        };
        let source_eventuality = node.eventuality;
        let mut arguments = node.arguments.clone();
        for argument in arguments.values_mut() {
            replace_generated_argument_value_object(argument, replacements);
        }
        let mut modal_arguments = node.modal_arguments.clone();
        for modal_argument in &mut modal_arguments {
            let mut arguments = modal_argument.arguments.clone();
            for (place_key, argument) in &mut arguments {
                replace_generated_argument_value_object(argument, replacements);
                let place = argument_place_index(place_key);
                if let Some(eventuality) = source_eventuality
                    && argument.value == Some(eventuality)
                    && let Some(elision) =
                        self.generated_host_event_modal_elision(eventuality, modal_argument, place)
                {
                    *argument = elision;
                }
            }
            if arguments != modal_argument.arguments {
                *modal_argument = modal_argument
                    .clone()
                    .with_data(data! { arguments: arguments });
            }
        }
        let id = self.next_predication_id();
        let mut cloned = SemanticObject::predication(
            relation,
            None,
            arguments,
            node.mode,
            object.source().cloned(),
            object.diagnostics().to_vec(),
        );
        cloned.set_predication_modal_arguments(modal_arguments);
        self.insert(id, cloned)?;
        Ok(Some(id))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(source_variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn clone_generated_formula_replacing_referent(
        &mut self,
        formula: SemanticObjectId,
        source_variable: SemanticObjectId,
        variable: SemanticObjectId,
        cloned: &mut BTreeMap<SemanticObjectId, SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(id) = cloned.get(&formula) {
            return Ok(*id);
        }
        let object = self
            .objects
            .get(&formula)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing formula to clone: {formula}")))?;
        if object.object_kind() != crate::model::SemanticObjectKind::Formula {
            return Err(invalid_graph(format!(
                "cannot clone non-formula restriction object: {formula}"
            )));
        }
        let formula_node = match object.into_data() {
            data!(SemanticObject::Formula(node)) => node,
            _ => unreachable!("formula kind has formula variant"),
        };
        let formula_node = match formula_node.into_data() {
            data!(FormulaNode::Atom(node)) => {
                let predication = self.clone_generated_predication_replacing_referent(
                    node.predication,
                    source_variable,
                    variable,
                    cloned,
                )?;
                new!(FormulaNode::Atom(
                    node.with_data(data! { predication: predication })
                ))
            }
            data!(FormulaNode::Connective(node)) => {
                let mut children = Vec::with_capacity(node.children.len());
                for child in &node.children {
                    children.push(self.clone_generated_formula_replacing_referent(
                        *child,
                        source_variable,
                        variable,
                        cloned,
                    )?);
                }
                new!(FormulaNode::Connective(
                    node.with_data(data! { children: children })
                ))
            }
            data!(FormulaNode::Quantified(node)) => {
                let restriction = match node.restriction {
                    Some(restriction) => Some(self.clone_generated_formula_replacing_referent(
                        restriction,
                        source_variable,
                        variable,
                        cloned,
                    )?),
                    None => None,
                };
                let body = self.clone_generated_formula_replacing_referent(
                    node.body,
                    source_variable,
                    variable,
                    cloned,
                )?;
                let selected_variable = if node.variable == source_variable {
                    variable
                } else {
                    node.variable
                };
                let selected_source_variable = if node.source_variable == Some(source_variable) {
                    Some(variable)
                } else {
                    node.source_variable
                };
                let selection_source = if node
                    .selection_source
                    .as_ref()
                    .is_some_and(|source| source.variable == source_variable)
                {
                    Some(SelectionSource::witness_set(variable))
                } else {
                    node.selection_source.clone()
                };
                new!(FormulaNode::Quantified(node.with_data(data! {
                    variable: selected_variable,
                    source_variable: selected_source_variable,
                    selection_source: selection_source,
                    restriction: restriction,
                    body: body,
                })))
            }
            data!(FormulaNode::QuantifierBundle(node)) => {
                let mut bindings = Vec::with_capacity(node.bindings.len());
                for binding in &node.bindings {
                    let mut data = binding.clone().into_data();
                    if data.variable == source_variable {
                        data.variable = variable;
                    }
                    if data.source_variable == Some(source_variable) {
                        data.source_variable = Some(variable);
                    }
                    if data
                        .selection_source
                        .as_ref()
                        .is_some_and(|source| source.variable == source_variable)
                    {
                        data.selection_source = Some(SelectionSource::witness_set(variable));
                    }
                    if let Some(restriction) = data.restriction {
                        data.restriction = Some(self.clone_generated_formula_replacing_referent(
                            restriction,
                            source_variable,
                            variable,
                            cloned,
                        )?);
                    }
                    bindings.push(QuantifierBinding::from_data(data));
                }
                let body = self.clone_generated_formula_replacing_referent(
                    node.body,
                    source_variable,
                    variable,
                    cloned,
                )?;
                new!(FormulaNode::QuantifierBundle(
                    node.with_data(data! { bindings: bindings, body: body })
                ))
            }
            data!(FormulaNode::RespectivelyDistribution(node)) => {
                let body = self.clone_generated_formula_replacing_referent(
                    node.body,
                    source_variable,
                    variable,
                    cloned,
                )?;
                new!(FormulaNode::RespectivelyDistribution(
                    node.with_data(data! { body: body })
                ))
            }
        };
        let object = new!(SemanticObject::Formula(formula_node));

        let cloned_formula = self.next_formula_id();
        cloned.insert(formula, cloned_formula);
        self.insert(cloned_formula, object)?;
        Ok(cloned_formula)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(source_variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication) || ret.is_err())]
    pub(super) fn clone_generated_predication_replacing_referent(
        &mut self,
        predication: SemanticObjectId,
        source_variable: SemanticObjectId,
        variable: SemanticObjectId,
        cloned: &mut BTreeMap<SemanticObjectId, SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(id) = cloned.get(&predication) {
            return Ok(*id);
        }
        let object =
            self.objects.get(&predication).cloned().ok_or_else(|| {
                invalid_graph(format!("missing predication to clone: {predication}"))
            })?;
        if object.object_kind() != crate::model::SemanticObjectKind::Predication {
            return Err(invalid_graph(format!(
                "cannot clone non-predication restriction object: {predication}"
            )));
        }
        let mut data = match object.into_data() {
            data!(SemanticObject::Predication(node)) => node.into_data(),
            _ => unreachable!("predication kind has predication variant"),
        };
        if let Some(eventuality) = data.eventuality {
            data.eventuality = Some(self.clone_generated_eventuality_for_predication(eventuality)?);
        }
        for argument in data.arguments.values_mut() {
            *argument = self.clone_generated_argument_value_replacing_referent(
                argument,
                source_variable,
                variable,
                cloned,
            )?;
        }
        let cloned_predication = self.next_predication_id();
        cloned.insert(predication, cloned_predication);
        self.insert(
            cloned_predication,
            new!(SemanticObject::Predication(PredicationNode::from_data(
                data
            ))),
        )?;
        Ok(cloned_predication)
    }

    #[requires(eventuality.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn clone_generated_eventuality_for_predication(
        &mut self,
        eventuality: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let object =
            self.objects.get(&eventuality).cloned().ok_or_else(|| {
                invalid_graph(format!("missing eventuality to clone: {eventuality}"))
            })?;
        if object.as_eventuality().is_none() {
            return Err(invalid_graph(format!(
                "cannot clone non-eventuality predication event: {eventuality}"
            )));
        }
        let sort = object.sort().unwrap_or_else(SemanticSort::eventuality);
        let cloned_eventuality = self.next_referent_with_sort_id(sort);
        self.insert(cloned_eventuality, object)?;
        if self.pending_after_eventuality_reservations > 0 {
            self.reserve_generated_semantic_id();
            self.pending_after_eventuality_reservations -= 1;
        }
        Ok(cloned_eventuality)
    }

    #[requires(argument.value.is_none_or(|value| crate::model::argument_object_kind_can_fill(value.object_kind())))]
    #[requires(source_variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_none_or(|value| crate::model::argument_object_kind_can_fill(value.object_kind()))) || ret.is_err())]
    pub(super) fn clone_generated_argument_value_replacing_referent(
        &mut self,
        argument: &ArgumentValue,
        source_variable: SemanticObjectId,
        variable: SemanticObjectId,
        cloned: &mut BTreeMap<SemanticObjectId, SemanticObjectId>,
    ) -> Result<ArgumentValue, SemanticsError> {
        let mut data = argument.clone().into_data();
        if data.value == Some(source_variable) {
            data.value = Some(variable);
        } else if let Some(value) = data.value
            && self.generated_referent_is_elided_constant(value)
        {
            data.value = Some(self.clone_generated_referent_for_restriction(value)?);
        }
        for clause in &mut data.relative_clauses {
            let body = self.clone_generated_formula_replacing_referent(
                clause.body,
                source_variable,
                variable,
                cloned,
            )?;
            *clause = clause.clone().with_data(data! { body: body });
        }
        Ok(ArgumentValue::from_data(data))
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn generated_referent_is_elided_constant(&self, referent: SemanticObjectId) -> bool {
        self.objects.get(&referent).is_some_and(|object| {
            object.object_kind() == crate::model::SemanticObjectKind::Referent
                && object.referent_category() == Some(ReferentCategory::Constant)
                && object
                    .descriptor()
                    .is_some_and(|descriptor| descriptor.kind == DescriptorKind::Elided)
        })
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn clone_generated_referent_for_restriction(
        &mut self,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let object = self
            .objects
            .get(&referent)
            .cloned()
            .ok_or_else(|| invalid_graph(format!("missing referent to clone: {referent}")))?;
        if object.object_kind() != crate::model::SemanticObjectKind::Referent {
            return Err(invalid_graph(format!(
                "cannot clone non-referent restriction object: {referent}"
            )));
        }
        let sort = object.sort().unwrap_or(SemanticSort::Entity);
        let cloned = self.next_referent_with_sort_id(sort);
        self.insert(cloned, object)?;
        Ok(cloned)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_scoped_generated_pro_sumti_variable(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
        sort: SemanticSort,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Variable,
                sort,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::ProSumti,
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
                })),
                None,
                self.source_for_node(pro_sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_relative_clause_list<'syntax: 'tree>(
        &mut self,
        relative_clauses: &'syntax RelativeClauseListSyntax,
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

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_relative_clause_atom(
        &mut self,
        clause: &'tree RelativeClauseAtomSyntax,
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
    #[ensures(true)]
    pub(super) fn lower_generated_descriptor_relative_clause_list(
        &mut self,
        relative_clauses: &'tree RelativeClauseListSyntax,
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

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_sumti_association_relative_clause(
        &mut self,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<Option<RelativeClause>, SemanticsError> {
        let marker_text = token_text(&clause.association_marker.value);
        if clause.association_marker.value.cmavo() == Some(Cmavo::Goi) {
            return Ok(None);
        }
        let source = self.exact_source_for_node(clause, "relative-phrase");
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
        arguments.insert(argument_key(1), ArgumentValue::filled(head, None));
        arguments.insert(argument_key(2), associated_argument);
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

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_goi_associated_referent<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) else {
            return Ok(None);
        };
        let Some(clause) = generated_goi_assignment_clause(relative_clauses) else {
            return Ok(None);
        };
        if generated_sumti_is_assignable_reference(sumti) {
            let Some(argument_object) =
                self.build_generated_relative_sumti_argument_object(&clause.sumti)?
            else {
                return Ok(None);
            };
            if let Some(assigned_name) = self.assigned_name_for_generated_sumti(sumti, clause) {
                self.assign_generated_name_to_argument_object(argument_object, assigned_name)?;
            }
            return Ok(Some(argument_object));
        }
        if generated_relative_sumti_is_assignable_reference(&clause.sumti) {
            let argument_object = self.build_sumti_grouped_referent(&sumti.base_sumti)?;
            if let Some(assigned_name) =
                self.assigned_name_for_generated_relative_sumti(&clause.sumti, clause)
            {
                self.assign_generated_name_to_argument_object(argument_object, assigned_name)?;
            }
            return Ok(Some(argument_object));
        }
        if let Some(assigned_name) =
            self.assigned_name_for_generated_relative_sumti(&clause.sumti, clause)
        {
            let argument_object = self.build_sumti_grouped_referent(&sumti.base_sumti)?;
            self.assign_generated_name_to_argument_object(argument_object, assigned_name)?;
            return Ok(Some(argument_object));
        }
        Ok(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| crate::model::argument_object_kind_can_fill(id.object_kind()))) || ret.is_err())]
    pub(super) fn build_generated_relative_sumti_argument_object<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax RelativeSumtiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match sumti {
            RelativeSumtiSyntax::PlainRelativeSumti(PlainRelativeSumtiSyntax(sumti)) => {
                Ok(Some(self.build_sumti_referent(sumti)?))
            }
            RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
                let argument = self.build_tagged_or_elided_sumti_argument(&sumti.sumti)?;
                Ok(argument.value)
            }
            RelativeSumtiSyntax::NaKuRelativeSumti(_) => Ok(None),
        }
    }

    #[requires(crate::model::argument_object_kind_can_fill(argument_object.object_kind()))]
    #[ensures(true)]
    pub(super) fn assign_generated_name_to_argument_object(
        &mut self,
        argument_object: SemanticObjectId,
        assigned_name: AssignedName,
    ) -> Result<(), SemanticsError> {
        let key = assigned_name.name.clone();
        let object = self.objects.get_mut(&argument_object).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find assigned-name argument object {argument_object}"
            ))
        })?;
        if argument_object.object_kind() == crate::model::SemanticObjectKind::Referent
            && !object
                .assigned_names()
                .iter()
                .any(|existing| existing == &assigned_name)
        {
            object.push_assigned_name(assigned_name);
        }
        self.assigned_referents.insert(key, argument_object);
        Ok(())
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn push_generated_goi_assigned_names_to_referent(
        &mut self,
        referent: SemanticObjectId,
        relative_clauses: &'tree RelativeClauseListSyntax,
    ) -> Result<(), SemanticsError> {
        if let Some(clause) = generated_goi_assignment_clause(relative_clauses)
            && let Some(assigned_name) =
                self.assigned_name_for_generated_relative_sumti(&clause.sumti, clause)
        {
            self.assign_generated_name_to_argument_object(referent, assigned_name)?;
        }
        Ok(())
    }

    #[requires(head.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn add_generated_goi_assignment_to_referent(
        &mut self,
        head: SemanticObjectId,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
    ) -> Result<(), SemanticsError> {
        let Some(assigned_name) =
            self.assigned_name_for_generated_relative_sumti(&clause.sumti, clause)
        else {
            return Ok(());
        };
        self.assign_generated_name_to_argument_object(head, assigned_name)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn assigned_name_for_generated_relative_sumti(
        &self,
        sumti: &'tree RelativeSumtiSyntax,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
    ) -> Option<AssignedName> {
        match sumti {
            RelativeSumtiSyntax::PlainRelativeSumti(PlainRelativeSumtiSyntax(sumti)) => {
                self.assigned_name_for_generated_sumti(sumti, clause)
            }
            RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
                self.assigned_name_for_generated_tagged_or_elided_sumti(&sumti.sumti, clause)
            }
            RelativeSumtiSyntax::NaKuRelativeSumti(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn assigned_name_for_generated_tagged_or_elided_sumti(
        &self,
        sumti: &'tree TaggedOrElidedSumtiSyntax,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
    ) -> Option<AssignedName> {
        match sumti {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.assigned_name_for_generated_sumti(sumti, clause)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn assigned_name_for_generated_sumti(
        &self,
        sumti: &'tree SumtiSyntax,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
    ) -> Option<AssignedName> {
        let simple = generated_simple_sumti_from_sumti(sumti)?;
        let SumtiAtomSyntax::SumtiBase(base_sumti) = simple.base_sumti.as_ref() else {
            return None;
        };
        let (word, name) = match base_sumti {
            SumtiBaseSyntax::ProSumti(pro_sumti)
                if pro_sumti.0.value.cmavo().is_some_and(is_assignable_koha) =>
            {
                let handle = token_text(&pro_sumti.0.value);
                (handle.clone(), handle)
            }
            SumtiBaseSyntax::LerfuStringSumti(letters) => {
                let handle = generated_letter_string_text(&letters.words);
                (handle.clone(), handle)
            }
            SumtiBaseSyntax::NameSumti(name) => (
                token_text(&name.la.value),
                token_list_text(name.names.value.iter()),
            ),
            _ => return None,
        };
        Some(AssignedName::from_data(data!(AssignedName {
            name,
            word,
            introduced_by: token_text(&clause.association_marker.value),
            source: self.source_for_generated_assigned_name_clause(clause),
        })))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn source_for_generated_assigned_name_clause(
        &self,
        clause: &'tree SumtiAssociationRelativeClauseSyntax,
    ) -> Option<crate::model::SemanticSource> {
        let mut tokens = vec![clause.association_marker.value.clone()];
        tokens.extend(self.tokens_for_node(clause.sumti.as_ref()));
        self.source_for_tokens(&tokens, "assigned-name")
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[requires(!marker_text.is_empty())]
    #[ensures(true)]
    pub(super) fn build_generated_modal_sumti_association_clause(
        &mut self,
        sumti: &'tree TenseTaggedRelativeSumtiSyntax,
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
        arguments.insert(argument_key(head_place), ArgumentValue::filled(head, None));
        arguments.insert(argument_key(visible_place), associated_argument);
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
        object.update_predication(|node| {
            node.with_data(data! { introduced_by: Some(introduced_by) })
        });
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
    pub(super) fn build_argument_for_generated_relative_sumti(
        &mut self,
        sumti: &'tree RelativeSumtiSyntax,
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

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_bridi_relative_clause(
        &mut self,
        clause: &'tree BridiRelativeClauseSyntax,
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
            BridiRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(clause) => self
                .lower_generated_relative_statement(
                    clause.statement.as_ref(),
                    head,
                    RelativeClauseKind::Restrictive,
                ),
            BridiRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(clause) => self
                .lower_generated_relative_statement(
                    clause.statement.as_ref(),
                    head,
                    RelativeClauseKind::Incidental,
                ),
        }
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_restrictive_bridi_relative_clause(
        &mut self,
        clause: &'tree RestrictiveBridiRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<RelativeClause, SemanticsError> {
        if clause
            .poi
            .value
            .cmavo()
            .is_some_and(cmavo_is_nonveridical_relative_marker)
        {
            return self.lower_generated_nonveridical_relative_bridi_clause(clause, head);
        }
        self.lower_generated_relative_subbridi(
            clause.subbridi.as_ref(),
            head,
            RelativeClauseKind::Restrictive,
        )
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_nonveridical_relative_bridi_clause(
        &mut self,
        clause: &'tree RestrictiveBridiRelativeClauseSyntax,
        head: SemanticObjectId,
    ) -> Result<RelativeClause, SemanticsError> {
        let marker_text = token_text(&clause.poi.value);
        let source = self.source_for_generated_subbridi(&clause.subbridi, "relative-clause");
        let formula = if let Some(selbri) = main_generated_selbri_for_subbridi(&clause.subbridi) {
            self.build_generated_nonveridical_relative_formula_for_selbri(
                selbri,
                head,
                source.clone(),
            )?
        } else {
            let formula = self
                .build_generated_subbridi_formula(&clause.subbridi, PredicationMode::Restrictive)?;
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

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_nonveridical_relative_formula_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
        head: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let property = self
            .build_generated_property_abstraction_for_selbri_with_source(selbri, source.clone())?;
        let mut arguments = BTreeMap::new();
        arguments.insert(
            argument_key(1),
            ArgumentValue::filled(self.current_speaker(), None),
        );
        arguments.insert(argument_key(2), ArgumentValue::filled(head, None));
        arguments.insert(argument_key(3), ArgumentValue::filled(property, None));
        let predication = self.next_predication_id();
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
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_generated_property_abstraction_for_selbri_with_source(
        &mut self,
        selbri: &'tree SelbriSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
            return self
                .build_description_property_abstraction_for_selbri_with_source(selbri, source);
        };
        self.build_property_abstraction_for_co_selbri(co_selbri, source)
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_relative_subbridi(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
    ) -> Result<RelativeClause, SemanticsError> {
        let mode = predication_mode_for_relative_clause_kind(kind);
        let contains_keha = generated_subbridi_contains_current_level_keha(subbridi);
        let previous_relative_head = self.relative_head;
        self.relative_head = Some(head);
        self.relative_head_stack.push(head);
        let result = self.build_generated_subbridi_formula(subbridi, mode);
        self.relative_head_stack.pop();
        self.relative_head = previous_relative_head;
        let formula = result?;
        if !contains_keha {
            self.fill_first_elided_generated_formula_argument_with_object(formula, head)?;
        }
        self.set_formula_predication_mode(formula, mode);
        Ok(RelativeClause::new(
            kind,
            formula,
            self.source_for_generated_subbridi(subbridi, "relative-clause"),
        ))
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[ensures(true)]
    pub(super) fn lower_generated_relative_statement(
        &mut self,
        statement: &'tree StatementSyntax,
        head: SemanticObjectId,
        kind: RelativeClauseKind,
    ) -> Result<RelativeClause, SemanticsError> {
        let mode = predication_mode_for_relative_clause_kind(kind);
        let contains_keha = generated_statement_contains_current_level_keha(statement);
        let previous_relative_head = self.relative_head;
        self.relative_head = Some(head);
        self.relative_head_stack.push(head);
        let result = self
            .build_generated_statement_connection_item(statement, UtteranceForce::Subordinated)
            .and_then(|(_item, formula)| {
                formula.ok_or_else(|| unsupported("relative statement without formula"))
            });
        self.relative_head_stack.pop();
        self.relative_head = previous_relative_head;
        let formula = result?;
        if !contains_keha {
            self.fill_first_elided_generated_formula_argument_with_object(formula, head)?;
        }
        self.set_formula_predication_mode(formula, mode);
        Ok(RelativeClause::new(
            kind,
            formula,
            self.source_for_node(statement, "relative-clause"),
        ))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_subbridi_formula(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_subbridi_formula_with_options(subbridi, None, mode)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_sumti_grouped_referent<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiGroupedSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| {
            if sumti.grouped_tail.is_some() {
                return Err(unsupported("grouped sumti"));
            }
            builder.build_sumti_afterthought_referent(&sumti.leading_sumti)
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_sumti_afterthought_referent<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiAfterthoughtSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| {
            let leading = builder.build_sumti_bound_referent(&sumti.leading_sumti)?;
            let [] = sumti.continuations.as_slice() else {
                let [continuation] = sumti.continuations.as_slice() else {
                    return Err(unsupported("multi-continuation afterthought sumti"));
                };
                let trailing = builder.build_sumti_bound_referent(&continuation.sumti)?;
                return builder.build_connected_generated_sumti_referent(
                    sumti,
                    leading,
                    &continuation.connective,
                    trailing,
                );
            };
            Ok(leading)
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_sumti_bound_referent(
        &mut self,
        sumti: &'tree SumtiBoundSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| {
            let leading = builder.build_sumti_forethought_referent(&sumti.leading_sumti)?;
            let Some(tail) = &sumti.bound_tail else {
                return Ok(leading);
            };
            if tail.tense_modal.is_some() {
                return Err(unsupported("tense-modal bound sumti"));
            }
            let trailing = builder.build_sumti_bound_referent(&tail.trailing_sumti)?;
            builder.build_connected_generated_sumti_referent(
                sumti,
                leading,
                tail.connective.as_ref(),
                trailing,
            )
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_sumti_forethought_referent(
        &mut self,
        sumti: &'tree SumtiForethoughtSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| match sumti {
            SumtiForethoughtSyntax::SimpleSumti(simple) => {
                builder.build_simple_sumti_referent(simple)
            }
            SumtiForethoughtSyntax::ForethoughtSumti(sumti) => {
                let mut leading = builder.build_sumti_referent(&sumti.leading_sumti)?;
                let trailing =
                    builder.build_sumti_forethought_referent(&sumti.first_branch.sumti)?;
                leading = builder.build_connected_generated_forethought_sumti_referent(
                    sumti,
                    leading,
                    &sumti.gek,
                    &sumti.first_branch.gik,
                    trailing,
                )?;
                for branch in &sumti.additional_branches {
                    let trailing = builder.build_sumti_forethought_referent(&branch.sumti)?;
                    leading = builder.build_connected_generated_extra_forethought_sumti_referent(
                        sumti, leading, &sumti.gek, trailing,
                    )?;
                }
                Ok(leading)
            }
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_simple_sumti_referent(
        &mut self,
        sumti: &'tree SimpleSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| {
            match sumti.base_sumti.as_ref() {
                SumtiAtomSyntax::SumtiBase(base) => builder.build_sumti_base_referent(base),
                SumtiAtomSyntax::QuantifiedSumti(_) => Err(unsupported("quantified sumti")),
            }
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_sumti_base_referent(
        &mut self,
        sumti: &'tree SumtiBaseSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_cached_sumti_referent_for_node(sumti, |builder| match sumti {
            SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
                builder.build_scalar_negated_generated_sumti_with_bo_referent(sumti)
            }
            SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
                builder.build_scalar_negated_generated_sumti_referent(sumti)
            }
            SumtiBaseSyntax::ProSumti(pro_sumti) => builder.build_pro_sumti_referent(pro_sumti),
            SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
                builder.build_description_referent(description)
            }
            SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(description) => {
                builder.build_outer_quantified_description_referent(description)
            }
            SumtiBaseSyntax::DescriptorWithoutGadriSumti(description) => {
                builder.build_no_gadri_description_referent(description)
            }
            SumtiBaseSyntax::NameSumti(name) => builder.build_name_sumti_referent(name),
            SumtiBaseSyntax::NumberSumti(number) => builder.build_number_sumti_referent(number),
            SumtiBaseSyntax::LerfuStringSumti(sumti) => {
                builder.build_lerfu_string_sumti_referent(sumti)
            }
            SumtiBaseSyntax::LaheSumti(sumti) => builder.build_lahe_sumti_referent(sumti),
            SumtiBaseSyntax::QuotedSumti(sumti) => builder.build_quoted_sumti_sign(sumti),
            _ => Err(unsupported("sumti base")),
        })
        .map(|(referent, _built)| referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_lerfu_string_sumti_referent(
        &mut self,
        sumti: &'tree LerfuStringSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(referent) = self.resolved_generated_lerfu_sumti_referent(sumti) {
            return Ok(referent);
        }
        self.build_generated_diagnostic_sumti_referent(
            sumti,
            "letteral pro-sumti did not resolve to an antecedent",
        )
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn resolved_generated_lerfu_sumti_referent(
        &self,
        sumti: &'tree LerfuStringSumtiSyntax,
    ) -> Option<SemanticObjectId> {
        let key = generated_letter_string_initial_key(&sumti.words)?;
        let (node_start, _) = self.source_key_for_node(sumti)?;
        self.letter_sumti_referents
            .get(&key)?
            .iter()
            .rev()
            .find(|candidate| candidate.source_key.1 <= node_start)
            .map(|candidate| candidate.referent)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_quoted_sumti_sign(
        &mut self,
        sumti: &'tree QuotedSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(sumti, "quotation");
        let source_text = source.as_ref().and_then(|source| source.text.clone());
        let quotation = match sumti.0.as_ref() {
            QuoteSyntax::TextQuote(quote) => {
                let utterance = self.build_generated_quoted_text_group(
                    &quote.text,
                    &quote.lu.free_modifiers,
                    source.clone(),
                )?;
                new!(Quotation {
                    mode: "parsed".to_owned(),
                    utterance,
                    delimiter: None,
                    text: source_text,
                })
            }
            _ => {
                let delimiter = self
                    .tokens_for_node(sumti)
                    .first()
                    .map(quote_delimiter_text);
                new!(Quotation {
                    mode: "opaque".to_owned(),
                    utterance: None,
                    delimiter,
                    text: source_text,
                })
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance)) || ret.is_err())]
    pub(super) fn build_generated_quoted_text_group(
        &mut self,
        text: &'tree TextSyntax,
        marker_free_modifiers: &'tree [FreeModifierSyntax],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let plan = generated_text_plan_from_text(text)?;
        let has_semantic_text = generated_text_plan_has_semantic_content(&plan);
        if !has_semantic_text && !free_modifiers_have_generated_vocative(marker_free_modifiers) {
            return Ok(None);
        }
        let previous_roles = self.current_deictic_roles();
        let previous_current_utterance = self.current_utterance;
        let previous_previous_utterance = self.previous_utterance;
        let previous_next_utterance = self.next_utterance;
        let previous_quote_depth = self.current_quote_depth;
        let quote_roles = self.build_fresh_quote_deictic_roles(source.clone())?;
        self.set_current_deictic_roles(quote_roles);
        self.current_utterance = None;
        self.previous_utterance = None;
        self.next_utterance = None;
        self.current_quote_depth = self.current_quote_depth.checked_add(1).ok_or_else(|| {
            invalid_graph("generated quotation nesting depth overflowed".to_owned())
        })?;
        let result = (|| {
            let mut marker_asides =
                self.build_generated_vocative_asides_from_slice(marker_free_modifiers)?;
            if has_semantic_text {
                let items = self.build_generated_text_plan_items(plan)?;
                if items.is_empty() {
                    return Ok(None);
                }
                let discourse_item = if let [single] = items.as_slice() {
                    *single
                } else {
                    let id = self.next_sequence_id();
                    self.insert(
                        id,
                        SemanticObject::sequence(
                            items,
                            SequenceRelation::SameTopicContinuation,
                            None,
                            Vec::new(),
                        ),
                    )?;
                    id
                };
                let root =
                    self.wrap_generated_quoted_discourse_item_in_utterance(discourse_item, source)?;
                if !marker_asides.is_empty() {
                    self.add_asides_to_generated_discourse_item(
                        root,
                        std::mem::take(&mut marker_asides),
                    );
                }
                Ok(Some(root))
            } else {
                self.build_generated_standalone_asides(marker_asides)?
                    .map(|item| {
                        self.wrap_generated_quoted_discourse_item_in_utterance(item, source)
                    })
                    .transpose()
            }
        })();
        self.set_current_deictic_roles(previous_roles);
        self.current_utterance = previous_current_utterance;
        self.previous_utterance = previous_previous_utterance;
        self.next_utterance = previous_next_utterance;
        self.current_quote_depth = previous_quote_depth;
        result
    }

    #[requires(matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn wrap_generated_quoted_discourse_item_in_utterance(
        &mut self,
        item: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if item.object_kind() == crate::model::SemanticObjectKind::Utterance {
            return Ok(item);
        }
        let utterance = self.next_utterance_id();
        self.insert_generated_utterance(utterance, UtteranceForce::Quote, Some(item), source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_text_group_statement(
        &mut self,
        statement: &'tree TextGroupStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let utterance = self.next_utterance_id();
        self.build_generated_text_group_statement_with_id(utterance, statement)
    }

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance) || ret.is_err())]
    pub(super) fn build_generated_text_group_statement_with_id(
        &mut self,
        utterance: SemanticObjectId,
        statement: &'tree TextGroupStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.current_utterance = Some(utterance);
        let source = self.source_for_node(statement, "statement");
        self.insert_generated_utterance(utterance, UtteranceForce::Assert, None, source)?;
        let nested = self.build_generated_text_group_sequence(&statement.text)?;
        let nested = self.ensure_generated_text_group_sequence_content(nested, &statement.text)?;
        if let Some(tense_modal) = &statement.tense_modal
            && generated_tense_relation_spec_for_tense_modal(tense_modal).is_none()
            && let Some(modal_argument) =
                self.build_modal_argument_for_generated_tense_modal(tense_modal, "modal-argument")?
        {
            self.record_generated_sticky_modal_argument_if_needed(tense_modal, &modal_argument);
            self.attach_modal_argument_to_generated_discourse_item(nested, &modal_argument)?;
        }
        let object = self.objects.get_mut(&utterance).ok_or_else(|| {
            invalid_graph(format!(
                "missing generated text-group utterance {utterance}"
            ))
        })?;
        object.update_utterance(|node| node.with_data(data! { content: Some(nested) }));
        object.push_diagnostic(diagnostic(
            "tu'e text group is represented as a nested discourse sequence",
        ));
        Ok(utterance)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_generated_text_group_sequence(
        &mut self,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let plan = generated_text_plan_from_text(text)?;
        let previous_current_utterance = self.current_utterance;
        let previous_previous_utterance = self.previous_utterance;
        let previous_next_utterance = self.next_utterance;
        self.current_utterance = None;
        self.previous_utterance = None;
        self.next_utterance = None;
        let result = (|| {
            let items = self.build_generated_text_plan_items(plan)?;
            if let [single] = items.as_slice() {
                Ok(*single)
            } else {
                let sequence = self.next_sequence_id();
                self.insert(
                    sequence,
                    SemanticObject::sequence(
                        items,
                        SequenceRelation::SameTopicContinuation,
                        self.source_for_node(text, "text"),
                        Vec::new(),
                    ),
                )?;
                Ok(sequence)
            }
        })();
        self.current_utterance = previous_current_utterance;
        self.previous_utterance = previous_previous_utterance;
        self.next_utterance = previous_next_utterance;
        result
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn ensure_generated_text_group_sequence_content(
        &mut self,
        item: SemanticObjectId,
        text: &'tree TextSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            return Ok(item);
        }
        let sequence = self.next_sequence_id();
        self.insert(
            sequence,
            SemanticObject::sequence(
                vec![item],
                SequenceRelation::SameTopicContinuation,
                self.source_for_node(text, "text-group-sequence"),
                Vec::new(),
            ),
        )?;
        Ok(sequence)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_scalar_negated_generated_sumti_with_bo_referent(
        &mut self,
        sumti: &'tree ScalarNegatedSumtiWithBoSyntax,
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
    pub(super) fn build_scalar_negated_generated_sumti_referent(
        &mut self,
        sumti: &'tree ScalarNegatedSumtiSyntax,
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
    pub(super) fn build_scalar_negated_generated_sumti_referent_with_marker<N: TreeNode>(
        &mut self,
        node: &N,
        cmavo: Option<Cmavo>,
        word: String,
        inner_sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operand = self.build_sumti_referent(inner_sumti)?;
        let sort = self
            .objects
            .get(&operand)
            .and_then(SemanticObject::sort)
            .unwrap_or(SemanticSort::Entity);
        let scale = self.build_generated_scalar_negation_scale_referent(node, &word)?;
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(new!(Descriptor {
                    kind: scalar_negated_sumti_qualifier_kind(cmavo),
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
                })),
                None,
                self.source_for_node(node, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_scalar_negation_scale_referent<N: TreeNode>(
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
    pub(super) fn build_generated_diagnostic_sumti_referent<N: TreeNode>(
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
                Some(new!(Descriptor {
                    kind: DescriptorKind::UnloweredSumti,
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
                })),
                None,
                self.source_for_node(node, "sumti"),
                vec![diagnostic(message)],
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| crate::model::argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_number_sumti_referent(
        &mut self,
        number: &'tree NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if number.li.value.cmavo() == Some(Cmavo::Meho) {
            let id = self.build_generated_math_expression_sign(number)?;
            self.attach_subscript_from_free_modifiers(id, &number.li.free_modifiers)?;
            return Ok(id);
        }
        let variable_name = generated_math_variable_name(number.expression.as_ref());
        if let Some(variable_name) = &variable_name
            && let Some(referent) = self.math_variable_referents.get(variable_name)
        {
            return Ok(*referent);
        }
        let text = generated_number_descriptor_mekso_surface_text(number.expression.as_ref())?;
        let quantity = self.build_quantity_for_generated_mekso(
            number.expression.as_ref(),
            self.source_for_node(number, "quantity"),
        )?;
        let id = self.build_number_referent_with_quantity(
            &number.li,
            text,
            quantity,
            self.source_for_node(number, "number-sumti"),
        )?;
        if let Some(variable_name) = variable_name {
            self.math_variable_referents.insert(variable_name, id);
        }
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_number_referent_with_quantity(
        &mut self,
        li: &WithFreeModifiers<Token, FreeModifierSyntax>,
        text: String,
        quantity: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_with_sort_id(SemanticSort::Number);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Number,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::Number,
                    word: token_text(&li.value),
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: Some(quantity),
                    name: Some(text),
                    scale: None,
                    definiteness: None,
                    operand: None,
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        self.attach_subscript_from_free_modifiers(id, &li.free_modifiers)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_generated_math_expression_sign(
        &mut self,
        number: &'tree NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(letteral) = generated_mekso_letteral_tokens(number.expression.as_ref()) {
            let id = self.build_generated_letteral_sign(
                &letteral.0,
                self.source_for_node(number.expression.as_ref(), "letteral"),
            )?;
            if let Some(free_modifiers) = letteral.1 {
                self.attach_subscript_from_free_modifiers(id, free_modifiers)?;
            }
            return Ok(id);
        }
        let expression = self.build_generated_math_expression(
            number.expression.as_ref(),
            self.source_for_node(number, "math-expression"),
        )?;
        let mut sign = SemanticObject::text_sign(
            SignKind::MathExpression,
            generated_mekso_surface_text(number.expression.as_ref())?,
            self.source_for_node(number, "number-sumti"),
            Vec::new(),
        );
        sign.update_sign(|node| node.with_data(data! { denotes: Some(expression) }));
        let id = self.next_sign_id();
        self.insert(id, sign)?;
        Ok(id)
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_generated_letteral_sign(
        &mut self,
        tokens: &[Token],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let letterals = letteral_units_for_tokens(tokens);
        let text = letteral_display_text(&letterals)
            .or_else(|| source.as_ref().and_then(|source| source.text.clone()))
            .unwrap_or_else(|| token_list_text(tokens.iter()));
        let mut sign = SemanticObject::text_sign(SignKind::Letteral, text, source, Vec::new());
        sign.update_sign(|node| node.with_data(data! { letterals: letterals }));
        let id = self.next_sign_id();
        self.insert(id, sign)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_mekso(
        &mut self,
        expression: &'tree MeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_mekso_surface_text(expression)?;
        let value = generated_simple_pa_quantity_value_for_mekso(expression).map_or_else(
            || {
                self.build_generated_math_expression(
                    expression,
                    source.clone().map(|source| crate::model::SemanticSource {
                        construct: Some("math-expression".to_owned()),
                        ..source
                    }),
                )
                .map(QuantityValue::math_expression)
            },
            Ok,
        )?;
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                source,
            ),
        )?;
        Ok(quantity)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_expression(
        &mut self,
        expression: &'tree MeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match expression {
            MeksoSyntax::ZantufaReversePolishMekso(reverse_polish) => {
                self.build_generated_zantufa_reverse_polish_mekso(reverse_polish, source)
            }
            MeksoSyntax::ZantufaInfixMekso(infix) => {
                self.build_generated_zantufa_infix_mekso(infix, source)
            }
            MeksoSyntax::InfixMekso(infix) => self.build_generated_infix_mekso(infix, source),
            MeksoSyntax::ReversePolishMekso(reverse_polish) => {
                self.build_generated_reverse_polish_mekso(reverse_polish, source)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_expression_with_connected_operator_replacement(
        &mut self,
        expression: &'tree MeksoSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        match expression {
            MeksoSyntax::ZantufaReversePolishMekso(reverse_polish) => self
                .build_generated_zantufa_reverse_polish_mekso(reverse_polish, source)
                .map(|id| (id, false)),
            MeksoSyntax::ZantufaInfixMekso(infix) => self
                .build_generated_zantufa_infix_mekso_with_connected_operator_replacement(
                    infix,
                    replacement_operator,
                    source,
                ),
            MeksoSyntax::InfixMekso(infix) => self
                .build_generated_infix_mekso_with_connected_operator_replacement(
                    infix,
                    replacement_operator,
                    source,
                ),
            MeksoSyntax::ReversePolishMekso(reverse_polish) => self
                .build_generated_reverse_polish_mekso(reverse_polish, source)
                .map(|id| (id, false)),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_infix_mekso_with_connected_operator_replacement(
        &mut self,
        infix: &'tree InfixMeksoSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        if infix.continuations.is_empty() {
            return self.build_generated_mekso_precedence_with_connected_operator_replacement(
                &infix.first_expression,
                replacement_operator,
                source,
            );
        }
        let (mut expression, mut replaced) = self
            .build_generated_mekso_precedence_with_connected_operator_replacement(
                &infix.first_expression,
                replacement_operator,
                None,
            )?;
        let last_index = infix.continuations.len() - 1;
        for (index, continuation) in infix.continuations.iter().enumerate() {
            let expression_source = (index == last_index).then(|| source.clone()).flatten();
            if !replaced && connected_generated_mekso_operator(&continuation.operator)?.is_some() {
                let right =
                    self.build_generated_mekso_precedence(&continuation.right_expression, None)?;
                expression = self.build_generated_math_operator_expression_for_temporary_operator(
                    replacement_operator,
                    vec![expression, right],
                    expression_source,
                )?;
                replaced = true;
            } else {
                let (right, right_replaced) = if replaced {
                    (
                        self.build_generated_mekso_precedence(
                            &continuation.right_expression,
                            None,
                        )?,
                        false,
                    )
                } else {
                    self.build_generated_mekso_precedence_with_connected_operator_replacement(
                        &continuation.right_expression,
                        replacement_operator,
                        None,
                    )?
                };
                expression = self.build_generated_math_operator_expression_for_operator(
                    &continuation.operator,
                    vec![expression, right],
                    expression_source,
                )?;
                replaced |= right_replaced;
            }
        }
        Ok((expression, replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_infix_mekso_with_connected_operator_replacement(
        &mut self,
        infix: &'tree ZantufaInfixMeksoSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        if infix.continuations.is_empty() {
            return self.build_generated_mekso_precedence_with_connected_operator_replacement(
                &infix.first_expression,
                replacement_operator,
                source,
            );
        }
        let (mut expression, mut replaced) = self
            .build_generated_mekso_precedence_with_connected_operator_replacement(
                &infix.first_expression,
                replacement_operator,
                None,
            )?;
        let last_index = infix.continuations.len() - 1;
        for (index, continuation) in infix.continuations.iter().enumerate() {
            let expression_source = (index == last_index).then(|| source.clone()).flatten();
            let (operands, operand_replaced) = match &continuation.right_expression {
                Some(right_expression) if replaced => (
                    vec![
                        expression,
                        self.build_generated_mekso_precedence(right_expression, None)?,
                    ],
                    false,
                ),
                Some(right_expression) => {
                    let (right, right_replaced) = self
                        .build_generated_mekso_precedence_with_connected_operator_replacement(
                            right_expression,
                            replacement_operator,
                            None,
                        )?;
                    (vec![expression, right], right_replaced)
                }
                None => (vec![expression], false),
            };
            let (next_expression, operator_replaced) = if replaced {
                (
                    self.build_generated_zantufa_operator_sequence_expression(
                        &continuation.operators,
                        operands,
                        expression_source,
                    )?,
                    false,
                )
            } else {
                self.build_generated_zantufa_operator_sequence_expression_with_connected_operator_replacement(
                    &continuation.operators,
                    replacement_operator,
                    operands,
                    expression_source,
                )?
            };
            expression = next_expression;
            replaced |= operand_replaced || operator_replaced;
        }
        Ok((expression, replaced))
    }

    #[requires(!operators.is_empty())]
    #[requires(!operands.is_empty())]
    #[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_operator_sequence_expression<
        O: AsRef<MeksoOperatorSyntax>,
    >(
        &mut self,
        operators: &'tree [O],
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let [operator] = operators {
            return self.build_generated_math_operator_expression_for_operator(
                operator.as_ref(),
                operands,
                source,
            );
        }
        self.build_generated_math_operator_expression(
            MathOperator::from_label(generated_zantufa_mekso_operator_sequence_label(operators)?),
            operands,
            source,
        )
    }

    #[requires(!operators.is_empty())]
    #[requires(!operands.is_empty())]
    #[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_operator_sequence_expression_with_connected_operator_replacement<
        O: AsRef<MeksoOperatorSyntax>,
    >(
        &mut self,
        operators: &'tree [O],
        replacement_operator: &MeksoOperatorSyntax,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        if let [operator] = operators {
            if connected_generated_mekso_operator(operator.as_ref())?.is_some() {
                return self
                    .build_generated_math_operator_expression_for_temporary_operator(
                        replacement_operator,
                        operands,
                        source,
                    )
                    .map(|id| (id, true));
            }
            return self
                .build_generated_math_operator_expression_for_operator(
                    operator.as_ref(),
                    operands,
                    source,
                )
                .map(|id| (id, false));
        }
        let (label, replaced) = generated_zantufa_mekso_operator_sequence_label_with_replacement(
            operators,
            replacement_operator,
        )?;
        self.build_generated_math_operator_expression(
            MathOperator::from_label(label),
            operands,
            source,
        )
        .map(|id| (id, replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_precedence_with_connected_operator_replacement(
        &mut self,
        expression: &'tree MeksoPrecedenceSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let Some(tail) = &expression.tail else {
            return self.build_generated_mekso_base_with_connected_operator_replacement(
                &expression.left_expression,
                replacement_operator,
                source,
            );
        };
        if connected_generated_mekso_operator(&tail.operator)?.is_some() {
            let left = self.build_generated_mekso_base(&expression.left_expression, None)?;
            let right = self.build_generated_mekso_precedence(&tail.right_expression, None)?;
            return self
                .build_generated_math_operator_expression_for_temporary_operator(
                    replacement_operator,
                    vec![left, right],
                    source,
                )
                .map(|id| (id, true));
        }
        let (left, left_replaced) = self
            .build_generated_mekso_base_with_connected_operator_replacement(
                &expression.left_expression,
                replacement_operator,
                None,
            )?;
        let (right, right_replaced) = if left_replaced {
            (
                self.build_generated_mekso_precedence(&tail.right_expression, None)?,
                false,
            )
        } else {
            self.build_generated_mekso_precedence_with_connected_operator_replacement(
                &tail.right_expression,
                replacement_operator,
                None,
            )?
        };
        self.build_generated_math_operator_expression_for_operator(
            &tail.operator,
            vec![left, right],
            source,
        )
        .map(|id| (id, left_replaced || right_replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_base_with_connected_operator_replacement(
        &mut self,
        expression: &'tree MeksoBaseSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        match expression {
            MeksoBaseSyntax::MeksoOperand(operand) => self
                .build_generated_mekso_operand_with_connected_operator_replacement(
                    operand,
                    replacement_operator,
                    source,
                ),
            MeksoBaseSyntax::ForethoughtCallMekso(call) => {
                if connected_generated_mekso_operator(&call.operator)?.is_some() {
                    let operands = call
                        .operands
                        .iter()
                        .map(|operand| self.build_generated_mekso_base(operand, None))
                        .collect::<Result<Vec<_>, _>>()?;
                    return self
                        .build_generated_math_operator_expression_for_temporary_operator(
                            replacement_operator,
                            operands,
                            source,
                        )
                        .map(|id| (id, true));
                }
                let mut built_operands = Vec::with_capacity(call.operands.len());
                let mut replaced = false;
                for operand in &call.operands {
                    if replaced {
                        built_operands.push(self.build_generated_mekso_base(operand, None)?);
                    } else {
                        let (id, operand_replaced) = self
                            .build_generated_mekso_base_with_connected_operator_replacement(
                                operand,
                                replacement_operator,
                                None,
                            )?;
                        replaced = operand_replaced;
                        built_operands.push(id);
                    }
                }
                self.build_generated_math_operator_expression_for_operator(
                    &call.operator,
                    built_operands,
                    source,
                )
                .map(|id| (id, replaced))
            }
            MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => self
                .build_generated_zantufa_bo_grouped_mekso_base_with_connected_operator_replacement(
                    group,
                    replacement_operator,
                    source,
                ),
            MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => self
                .build_generated_zantufa_grouped_mekso_operand_sequence_with_connected_operator_replacement(
                    group,
                    replacement_operator,
                    source,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_bo_grouped_mekso_base_with_connected_operator_replacement(
        &mut self,
        group: &'tree ZantufaBoGroupedMeksoBaseSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let (first, mut replaced) = self
            .build_generated_mekso_operand_with_connected_operator_replacement(
                &group.first,
                replacement_operator,
                None,
            )?;
        let mut operands = Vec::with_capacity(group.continuations.len() + 1);
        operands.push(first);
        for continuation in &group.continuations {
            if replaced {
                operands.push(self.build_generated_mekso_operand(&continuation.expression, None)?);
            } else {
                let (operand, operand_replaced) = self
                    .build_generated_mekso_operand_with_connected_operator_replacement(
                        &continuation.expression,
                        replacement_operator,
                        None,
                    )?;
                replaced = operand_replaced;
                operands.push(operand);
            }
        }
        self.build_generated_math_operator_expression(new!(MathOperator::BoGroup), operands, source)
            .map(|id| (id, replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_grouped_mekso_operand_sequence_with_connected_operator_replacement(
        &mut self,
        group: &'tree ZantufaGroupedMeksoOperandSequenceSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let mut operands = Vec::with_capacity(group.operands.len());
        let mut replaced = false;
        for operand in &group.operands {
            if replaced {
                operands.push(self.build_generated_mekso_operand(operand, None)?);
            } else {
                let (operand, operand_replaced) = self
                    .build_generated_mekso_operand_with_connected_operator_replacement(
                        operand,
                        replacement_operator,
                        None,
                    )?;
                replaced = operand_replaced;
                operands.push(operand);
            }
        }
        self.build_generated_math_operator_expression(
            new!(MathOperator::OperandGroup),
            operands,
            source,
        )
        .map(|id| (id, replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_operand_with_connected_operator_replacement(
        &mut self,
        operand: &'tree MeksoOperandSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        match operand {
            MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
                let chain = &operand.0;
                let mut expression = self
                    .build_generated_bound_or_simple_mekso_operand_with_connected_operator_replacement(
                        &chain.first,
                        replacement_operator,
                        if chain.links.is_empty() { source.clone() } else { None },
                    )?;
                if chain.links.is_empty() {
                    return Ok(expression);
                }
                let replaced = expression.1;
                let last_index = chain.links.len() - 1;
                for (index, link) in chain.links.iter().enumerate() {
                    let right = self.build_generated_bound_or_simple_mekso_operand(
                        &link.trailing_expression,
                        None,
                    )?;
                    let expression_source = (index == last_index).then(|| source.clone()).flatten();
                    expression.0 = self.build_generated_operand_connective_math_expression(
                        &link.operand_connective,
                        vec![expression.0, right],
                        expression_source,
                    )?;
                }
                Ok((expression.0, replaced))
            }
            MeksoOperandSyntax::BoundMeksoOperand(operand) => self
                .build_generated_bound_mekso_operand_with_connected_operator_replacement(
                    operand,
                    replacement_operator,
                    source,
                ),
            MeksoOperandSyntax::SimpleMeksoOperand(operand) => self
                .build_generated_simple_mekso_operand_with_connected_operator_replacement(
                    operand,
                    replacement_operator,
                    source,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_bound_or_simple_mekso_operand_with_connected_operator_replacement(
        &mut self,
        operand: &'tree BoundOrSimpleMeksoOperandSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        match operand {
            BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => self
                .build_generated_bound_mekso_operand_with_connected_operator_replacement(
                    operand,
                    replacement_operator,
                    source,
                ),
            BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => self
                .build_generated_simple_mekso_operand_with_connected_operator_replacement(
                    operand,
                    replacement_operator,
                    source,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_bound_mekso_operand_with_connected_operator_replacement(
        &mut self,
        operand: &'tree BoundMeksoOperandSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let (left, left_replaced) = self
            .build_generated_simple_mekso_operand_with_connected_operator_replacement(
                &operand.left_expression,
                replacement_operator,
                None,
            )?;
        let (right, right_replaced) = if left_replaced {
            (
                self.build_generated_mekso_operand(&operand.right_expression, None)?,
                false,
            )
        } else {
            self.build_generated_mekso_operand_with_connected_operator_replacement(
                &operand.right_expression,
                replacement_operator,
                None,
            )?
        };
        self.build_generated_operand_connective_math_expression(
            &operand.operand_connective,
            vec![left, right],
            source,
        )
        .map(|id| (id, left_replaced || right_replaced))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_simple_mekso_operand_with_connected_operator_replacement(
        &mut self,
        operand: &'tree SimpleMeksoOperandSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        match operand {
            SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
                let (left, left_replaced) = self
                    .build_generated_mekso_operand_with_connected_operator_replacement(
                        &operand.left_expression,
                        replacement_operator,
                        None,
                    )?;
                let (right, right_replaced) = if left_replaced {
                    (
                        self.build_generated_mekso_operand(&operand.right_expression, None)?,
                        false,
                    )
                } else {
                    self.build_generated_mekso_operand_with_connected_operator_replacement(
                        &operand.right_expression,
                        replacement_operator,
                        None,
                    )?
                };
                self.build_generated_math_operator_expression(
                    MathOperator::from_label(generated_modal_forethought_connective_source(
                        &operand.gek,
                    )),
                    vec![left, right],
                    source,
                )
                .map(|id| (id, left_replaced || right_replaced))
            }
            SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
                let (id, replaced) = self
                    .build_generated_mekso_operand_with_connected_operator_replacement(
                        &operand.inner_expression,
                        replacement_operator,
                        source,
                    )?;
                self.set_math_scalar_negation(id, scalar_negation_for_token(&operand.nahe));
                Ok((id, replaced))
            }
            SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => self
                .build_generated_math_expression_with_connected_operator_replacement(
                    &operand.inner_expression,
                    replacement_operator,
                    source,
                ),
            SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => {
                let mut built_expressions = Vec::with_capacity(operand.expressions.len());
                let mut replaced = false;
                for expression in &operand.expressions {
                    if replaced {
                        built_expressions
                            .push(self.build_generated_math_expression(expression, None)?);
                    } else {
                        let (id, expression_replaced) = self
                            .build_generated_math_expression_with_connected_operator_replacement(
                                expression,
                                replacement_operator,
                                None,
                            )?;
                        replaced = expression_replaced;
                        built_expressions.push(id);
                    }
                }
                self.build_generated_math_operator_expression(
                    new!(MathOperator::Array),
                    built_expressions,
                    source,
                )
                .map(|id| (id, replaced))
            }
            _ => self
                .build_generated_simple_mekso_operand(operand, source)
                .map(|id| (id, false)),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_infix_mekso(
        &mut self,
        infix: &'tree InfixMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if infix.continuations.is_empty() {
            return self.build_generated_mekso_precedence(&infix.first_expression, source);
        }
        let mut expression =
            self.build_generated_mekso_precedence(&infix.first_expression, None)?;
        let last_index = infix.continuations.len() - 1;
        for (index, continuation) in infix.continuations.iter().enumerate() {
            let right =
                self.build_generated_mekso_precedence(&continuation.right_expression, None)?;
            let expression_source = (index == last_index).then(|| source.clone()).flatten();
            expression = self.build_generated_math_operator_expression_for_operator(
                &continuation.operator,
                vec![expression, right],
                expression_source,
            )?;
        }
        Ok(expression)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_infix_mekso(
        &mut self,
        infix: &'tree ZantufaInfixMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if infix.continuations.is_empty() {
            return self.build_generated_mekso_precedence(&infix.first_expression, source);
        }
        let mut expression =
            self.build_generated_mekso_precedence(&infix.first_expression, None)?;
        let last_index = infix.continuations.len() - 1;
        for (index, continuation) in infix.continuations.iter().enumerate() {
            let expression_source = (index == last_index).then(|| source.clone()).flatten();
            let operands = match &continuation.right_expression {
                Some(right_expression) => vec![
                    expression,
                    self.build_generated_mekso_precedence(right_expression, None)?,
                ],
                None => vec![expression],
            };
            expression = self.build_generated_zantufa_operator_sequence_expression(
                &continuation.operators,
                operands,
                expression_source,
            )?;
        }
        Ok(expression)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_precedence(
        &mut self,
        expression: &'tree MeksoPrecedenceSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(tail) = &expression.tail else {
            return self.build_generated_mekso_base(&expression.left_expression, source);
        };
        let left = self.build_generated_mekso_base(&expression.left_expression, None)?;
        let right = self.build_generated_mekso_precedence(&tail.right_expression, None)?;
        self.build_generated_math_operator_expression_for_operator(
            &tail.operator,
            vec![left, right],
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_base(
        &mut self,
        expression: &'tree MeksoBaseSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match expression {
            MeksoBaseSyntax::MeksoOperand(operand) => {
                self.build_generated_mekso_operand(operand, source)
            }
            MeksoBaseSyntax::ForethoughtCallMekso(call) => {
                self.build_generated_forethought_call_mekso(call, source)
            }
            MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
                self.build_generated_zantufa_bo_grouped_mekso_base(group, source)
            }
            MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => {
                self.build_generated_zantufa_grouped_mekso_operand_sequence(group, source)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_bo_grouped_mekso_base(
        &mut self,
        group: &'tree ZantufaBoGroupedMeksoBaseSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut operands = Vec::with_capacity(group.continuations.len() + 1);
        operands.push(self.build_generated_mekso_operand(&group.first, None)?);
        for continuation in &group.continuations {
            operands.push(self.build_generated_mekso_operand(&continuation.expression, None)?);
        }
        self.build_generated_math_operator_expression(new!(MathOperator::BoGroup), operands, source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_grouped_mekso_operand_sequence(
        &mut self,
        group: &'tree ZantufaGroupedMeksoOperandSequenceSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operands = group
            .operands
            .iter()
            .map(|operand| self.build_generated_mekso_operand(operand, None))
            .collect::<Result<Vec<_>, _>>()?;
        self.build_generated_math_operator_expression(
            new!(MathOperator::OperandGroup),
            operands,
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_mekso_operand(
        &mut self,
        operand: &'tree MeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match operand {
            MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
                let chain = &operand.0;
                if chain.links.is_empty() {
                    return self
                        .build_generated_bound_or_simple_mekso_operand(&chain.first, source);
                }
                let mut expression =
                    self.build_generated_bound_or_simple_mekso_operand(&chain.first, None)?;
                let last_index = chain.links.len() - 1;
                for (index, link) in chain.links.iter().enumerate() {
                    let right = self.build_generated_bound_or_simple_mekso_operand(
                        &link.trailing_expression,
                        None,
                    )?;
                    let expression_source = (index == last_index).then(|| source.clone()).flatten();
                    expression = self.build_generated_operand_connective_math_expression(
                        &link.operand_connective,
                        vec![expression, right],
                        expression_source,
                    )?;
                }
                Ok(expression)
            }
            MeksoOperandSyntax::BoundMeksoOperand(operand) => {
                self.build_generated_bound_mekso_operand(operand, source)
            }
            MeksoOperandSyntax::SimpleMeksoOperand(operand) => {
                self.build_generated_simple_mekso_operand(operand, source)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_bound_or_simple_mekso_operand(
        &mut self,
        operand: &'tree BoundOrSimpleMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match operand {
            BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
                self.build_generated_bound_mekso_operand(operand, source)
            }
            BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
                self.build_generated_simple_mekso_operand(operand, source)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_bound_mekso_operand(
        &mut self,
        operand: &'tree BoundMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let left = self.build_generated_simple_mekso_operand(&operand.left_expression, None)?;
        let right = self.build_generated_mekso_operand(&operand.right_expression, None)?;
        self.build_generated_operand_connective_math_expression(
            &operand.operand_connective,
            vec![left, right],
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_simple_mekso_operand(
        &mut self,
        operand: &'tree SimpleMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match operand {
            SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
                self.build_generated_forethought_mekso_operand(operand, source)
            }
            SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
                self.build_generated_qualified_mekso_operand(operand, source)
            }
            SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => {
                let id = self.build_generated_mekso_operand(&operand.inner_expression, source)?;
                self.set_math_scalar_negation(id, scalar_negation_for_token(&operand.nahe.value));
                Ok(id)
            }
            SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
                self.build_generated_math_expression(&operand.inner_expression, source)
            }
            SimpleMeksoOperandSyntax::SumtiMeksoOperand(operand) => {
                self.build_generated_sumti_mekso_operand(operand, source)
            }
            SimpleMeksoOperandSyntax::SelbriMeksoOperand(operand) => {
                self.build_generated_selbri_mekso_operand(operand, source)
            }
            SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(operand) => {
                let source = source.or_else(|| self.source_for_node(operand, "selbri-operand"));
                let Some(abstraction) = self.single_abstraction_from_selbri(&operand.selbri)?
                else {
                    return self.build_generated_math_literal(
                        MathLiteral::text(
                            MathLiteralKind::SelbriOperand,
                            generated_selbri_surface_text(&operand.selbri)?,
                        ),
                        source,
                    );
                };
                let denotation = self.build_abstraction_output(
                    abstraction,
                    self.source_for_node(abstraction, "abstraction"),
                )?;
                let id = self.next_math_id();
                self.insert(
                    id,
                    SemanticObject::math_selbri_operand(denotation, source, Vec::new()),
                )?;
                Ok(id)
            }
            SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => {
                self.build_generated_array_mekso_operand(operand, source)
            }
            SimpleMeksoOperandSyntax::NumberMekso(number) => {
                self.build_generated_math_expression_for_number(number, source)
            }
            SimpleMeksoOperandSyntax::LerfuStringMekso(letter) => {
                self.build_generated_math_expression_for_lerfu_string(letter, source)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_forethought_mekso_operand(
        &mut self,
        operand: &'tree ForethoughtMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let left = self.build_generated_mekso_operand(&operand.left_expression, None)?;
        let right = self.build_generated_mekso_operand(&operand.right_expression, None)?;
        self.build_generated_math_operator_expression(
            MathOperator::from_label(generated_modal_forethought_connective_source(&operand.gek)),
            vec![left, right],
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_qualified_mekso_operand(
        &mut self,
        operand: &'tree QualifiedMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_generated_mekso_operand(&operand.inner_expression, source)?;
        self.set_math_scalar_negation(id, scalar_negation_for_token(&operand.nahe));
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_sumti_mekso_operand(
        &mut self,
        operand: &'tree SumtiMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let denotation = self.build_generated_sumti_operand_denotation(&operand.sumti)?;
        let source = source.or_else(|| self.source_for_node(operand, "sumti-operand"));
        let id = self.next_math_id();
        self.insert(
            id,
            SemanticObject::math_sumti_operand(denotation, source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| argument_object_kind_can_fill(id.object_kind())) || ret.is_err())]
    pub(super) fn build_generated_sumti_operand_denotation(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
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
            return self.build_sumti_referent(sumti);
        };

        let variable = self.build_scoped_argument_variable_for_generated_sumti(sumti)?;
        let mut restrictions =
            self.generated_argument_restrictions_for_scope_source(scope_source, variable)?;
        if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
            restrictions.extend(
                self.lower_generated_relative_clause_list(relative_clauses, variable)?
                    .into_iter()
                    .map(|clause| clause.body),
            );
        }
        let body = self.generated_sumti_operand_body_formula(variable, &restrictions)?;
        let restriction = self.combine_generated_restriction_formulas(restrictions)?;
        let quantifier = generated_argument_scope_source_quantifier(scope_source);
        let quantity_connection = self
            .connected_quantifier_quantity_scope_for_generated_quantifier(
                quantifier,
                "mekso-operand",
            )?;
        let quantity = if let Some(connection) = quantity_connection {
            new!(GeneratedPreparedArgumentQuantity::Connected(connection))
        } else {
            new!(GeneratedPreparedArgumentQuantity::Single(
                self.build_quantity_for_quantifier(quantifier)?,
            ))
        };
        let scope = GeneratedArgumentQuantifierScope {
            node: GeneratedArgumentQuantifierScopeNode::Sumti(sumti),
            source: scope_source,
            variable,
            source_variable: None,
            selection_source: None,
            source_restriction_nodes: Vec::new(),
            source_restriction_formulas: Vec::new(),
            inherited_restrictions: Vec::new(),
            relative_clause_restrictions: Vec::new(),
        };
        self.wrap_formula_with_generated_argument_scope(body, scope, restriction, quantity)
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn generated_sumti_operand_body_formula(
        &mut self,
        variable: SemanticObjectId,
        restrictions: &[SemanticObjectId],
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(body) = self.combine_generated_restriction_formulas(restrictions.to_vec())? {
            return Ok(body);
        }
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(variable, None));
        self.build_structural_formula_from_arguments(
            "sumtiOperand",
            arguments,
            PredicationMode::Restrictive,
            None,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_selbri_mekso_operand(
        &mut self,
        operand: &'tree SelbriMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = source.or_else(|| self.source_for_node(operand, "selbri-operand"));
        let Some(abstraction) = self.single_abstraction_from_selbri(&operand.selbri)? else {
            return self.build_generated_math_literal(
                MathLiteral::text(
                    MathLiteralKind::SelbriOperand,
                    generated_selbri_surface_text(&operand.selbri)?,
                ),
                source,
            );
        };
        let denotation = self.build_abstraction_output(
            abstraction,
            self.source_for_node(abstraction, "abstraction"),
        )?;
        let id = self.next_math_id();
        self.insert(
            id,
            SemanticObject::math_selbri_operand(denotation, source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_array_mekso_operand(
        &mut self,
        operand: &'tree ArrayMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operands = operand
            .expressions
            .iter()
            .map(|expression| self.build_generated_math_expression(expression, None))
            .collect::<Result<Vec<_>, _>>()?;
        self.build_generated_math_operator_expression(new!(MathOperator::Array), operands, source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_forethought_call_mekso(
        &mut self,
        call: &'tree ForethoughtCallMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operands = call
            .operands
            .iter()
            .map(|operand| self.build_generated_mekso_base(operand, None))
            .collect::<Result<Vec<_>, _>>()?;
        self.build_generated_math_operator_expression_for_operator(&call.operator, operands, source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_zantufa_reverse_polish_mekso(
        &mut self,
        reverse_polish: &'tree ZantufaReversePolishMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut stack = Vec::with_capacity(reverse_polish.operands.len() + 1);
        for operand in &reverse_polish.operands {
            stack.push(self.build_generated_mekso_base(operand, None)?);
        }

        let first_operator_source = reverse_polish
            .tails
            .is_empty()
            .then(|| source.clone())
            .flatten();
        self.apply_generated_zantufa_reverse_polish_operator(
            &mut stack,
            &reverse_polish.operator,
            first_operator_source,
        )?;

        let last_tail_index = reverse_polish.tails.len().saturating_sub(1);
        for (index, tail) in reverse_polish.tails.iter().enumerate() {
            for operand in &tail.operands {
                stack.push(self.build_generated_mekso_base(operand, None)?);
            }
            let operator_source = (index == last_tail_index).then(|| source.clone()).flatten();
            self.apply_generated_zantufa_reverse_polish_operator(
                &mut stack,
                &tail.operator,
                operator_source,
            )?;
        }

        if stack.len() != 1 {
            return Err(unsupported(
                "Zantufa reverse Polish mex stack did not reduce to one expression",
            ));
        }
        stack.pop().ok_or_else(|| {
            unsupported("Zantufa reverse Polish mex stack did not produce an expression")
        })
    }

    #[requires(stack.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(stack.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    pub(super) fn apply_generated_zantufa_reverse_polish_operator(
        &mut self,
        stack: &mut Vec<SemanticObjectId>,
        operator: &'tree MeksoOperatorSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        if stack.len() < 2 {
            return Err(unsupported(
                "Zantufa reverse Polish mex operator without two operands",
            ));
        }
        let right = stack
            .pop()
            .ok_or_else(|| unsupported("Zantufa reverse Polish mex missing right operand"))?;
        let left = stack
            .pop()
            .ok_or_else(|| unsupported("Zantufa reverse Polish mex missing left operand"))?;
        let expression = self.build_generated_math_operator_expression_for_operator(
            operator,
            vec![left, right],
            source,
        )?;
        stack.push(expression);
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_reverse_polish_mekso(
        &mut self,
        reverse_polish: &'tree ReversePolishMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_math_literal(
            MathLiteral::text(
                MathLiteralKind::Expression,
                generated_reverse_polish_surface_text(reverse_polish)?,
            ),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_expression_for_number(
        &mut self,
        number: &'tree NumberMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_number_words_text(&number.0.number.value);
        self.build_generated_math_literal(math_literal_for_pa_text(text), source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_expression_for_lerfu_string(
        &mut self,
        letter: &'tree LerfuStringMeksoSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let tokens = generated_letter_string_tokens(&letter.letters);
        let value = generated_math_letteral_text(&tokens);
        let id = self.build_generated_math_literal(
            MathLiteral::text(MathLiteralKind::Variable, value),
            source,
        )?;
        self.attach_subscript_from_free_modifiers(id, &letter.free_modifiers)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_operator_expression_for_operator(
        &mut self,
        operator: &'tree MeksoOperatorSyntax,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.build_generated_math_operator_expression_for_operator_core(
            operator, operands, source,
        )?;
        if let Some(denotation) = self.generated_math_operator_denotation_for_operator(operator)? {
            self.set_math_operator_denotation(id, denotation);
        }
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_operator_expression_for_temporary_operator(
        &mut self,
        operator: &MeksoOperatorSyntax,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_math_operator_expression_for_operator_core(operator, operands, source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    fn build_generated_math_operator_expression_for_operator_core(
        &mut self,
        operator: &MeksoOperatorSyntax,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operands = generated_math_operands_for_operator(operator, operands);
        let id = if let Some(parameter) =
            self.generated_math_operator_question_parameter_for_operator(operator)?
        {
            let id = self.next_math_id();
            self.insert(
                id,
                SemanticObject::math_expression_with_operator_parameter(
                    parameter,
                    operands,
                    source,
                    Vec::new(),
                ),
            )?;
            id
        } else {
            self.build_generated_math_operator_expression(
                generated_math_operator_label(operator)?,
                operands,
                source,
            )?
        };
        if let Some(scalar_negation) = scalar_negation_for_generated_mekso_operator(operator) {
            self.set_math_scalar_negation(id, scalar_negation);
        }
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn generated_math_operator_question_parameter_for_operator(
        &mut self,
        operator: &MeksoOperatorSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(token) = generated_math_operator_question_token_for_operator(operator)? else {
            return Ok(None);
        };
        self.build_generated_math_operator_question_parameter_for_token(token)
            .map(Some)
    }

    #[requires(token.cmavo() == Some(Cmavo::Mo))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_generated_math_operator_question_parameter_for_token(
        &mut self,
        token: &Token,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::MathOperator,
                ParameterRole::MathOperatorQuestion,
                token_text(token),
                self.source_for_token(token, "parameter"),
            ),
        )?;
        self.math_operator_question_parameters.push(parameter);
        Ok(parameter)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| argument_object_kind_can_fill(id.object_kind()))) || ret.is_err())]
    pub(super) fn generated_math_operator_denotation_for_operator(
        &mut self,
        operator: &'tree MeksoOperatorSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match operator {
            MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
                if !operator.0.links.is_empty() {
                    return Ok(None);
                }
                self.generated_math_operator_denotation_for_bound_or_atom(operator.0.first.as_ref())
            }
            MeksoOperatorSyntax::BoundMeksoOperator(_) => Ok(None),
            MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
                self.generated_math_operator_denotation_for_simple_operator(operator)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| argument_object_kind_can_fill(id.object_kind()))) || ret.is_err())]
    pub(super) fn generated_math_operator_denotation_for_bound_or_atom(
        &mut self,
        operator: &'tree BoundOrAtomMeksoOperatorSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match operator {
            BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(_) => Ok(None),
            BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
                self.generated_math_operator_denotation_for_simple_operator(operator)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| argument_object_kind_can_fill(id.object_kind()))) || ret.is_err())]
    pub(super) fn generated_math_operator_denotation_for_simple_operator(
        &mut self,
        operator: &'tree SimpleMeksoOperatorSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match operator {
            SimpleMeksoOperatorSyntax::SelbriMeksoOperator(operator) => {
                if generated_math_operator_question_token_for_selbri(&operator.selbri)?.is_some() {
                    return Ok(None);
                }
                self.build_generated_math_operator_property_abstraction_for_selbri(
                    &operator.selbri,
                    self.source_for_node(operator.selbri.as_ref(), "math-operator-denotation"),
                )
                .map(Some)
            }
            SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
                self.generated_math_operator_denotation_for_operator(&operator.inner_operator)
            }
            SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
                self.generated_math_operator_denotation_for_operator(&operator.inner_operator)
            }
            SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
                self.generated_math_operator_denotation_for_operator(&operator.inner_operator)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_generated_math_operator_property_abstraction_for_selbri<'syntax: 'tree>(
        &mut self,
        selbri: &'syntax SelbriSyntax,
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
        let body = self.build_property_formula_for_selbri_with_context(
            selbri,
            parameter,
            source.clone(),
            GeneratedPropertyTanruContext::PropertyAbstraction,
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(true)]
    #[requires(!operands.is_empty())]
    #[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_operator_expression(
        &mut self,
        operator: MathOperator,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math_id();
        self.insert(
            id,
            SemanticObject::math_expression(Some(operator), operands, None, source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(!operands.is_empty())]
    #[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_operand_connective_math_expression(
        &mut self,
        connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
        operands: Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_operand_connective_is_interval(connective) {
            return self.build_generated_math_interval_expression(
                generated_operand_connective_interval_operator(connective)?,
                operands,
                generated_operand_connective_endpoint_inclusion(connective, false),
                source,
            );
        }
        self.build_generated_math_operator_expression(
            generated_operand_connective_math_operator(connective),
            operands,
            source,
        )
    }

    #[requires(operator.is_interval())]
    #[requires(!operands.is_empty())]
    #[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_interval_expression(
        &mut self,
        operator: MathOperator,
        operands: Vec<SemanticObjectId>,
        endpoint_inclusion: Option<IntervalEndpointInclusion>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math_id();
        self.insert(
            id,
            SemanticObject::math_interval_expression(
                operator,
                operands,
                endpoint_inclusion,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::MathExpression) || ret.is_err())]
    pub(super) fn build_generated_math_literal(
        &mut self,
        literal: MathLiteral,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_math_id();
        self.insert(
            id,
            SemanticObject::math_expression(None, Vec::new(), Some(literal), source, Vec::new()),
        )?;
        Ok(id)
    }

    #[requires(math_expression.object_kind() == crate::model::SemanticObjectKind::MathExpression)]
    #[ensures(true)]
    pub(super) fn set_math_scalar_negation(
        &mut self,
        math_expression: SemanticObjectId,
        scalar_negation: ScalarNegation,
    ) {
        if let Some(object) = self.objects.get_mut(&math_expression) {
            object.update_math_expression(|node| {
                node.with_data(data! { scalar_negation: Some(scalar_negation) })
            });
        };
    }

    #[requires(math_expression.object_kind() == crate::model::SemanticObjectKind::MathExpression)]
    #[requires(argument_object_kind_can_fill(denotation.object_kind()))]
    #[ensures(true)]
    pub(super) fn set_math_operator_denotation(
        &mut self,
        math_expression: SemanticObjectId,
        denotation: SemanticObjectId,
    ) {
        if let Some(object) = self.objects.get_mut(&math_expression) {
            object.update_math_expression(|node| {
                let data = node.into_data();
                let kind = match data.kind.into_data() {
                    data!(MathExpressionNodeKind::Operator {
                        operator,
                        operands,
                        endpoint_inclusion,
                        ..
                    }) => new!(MathExpressionNodeKind::Operator {
                        operator,
                        operands,
                        operator_denotes: Some(denotation),
                        endpoint_inclusion,
                    }),
                    kind => MathExpressionNodeKind::from_data(kind),
                };
                MathExpressionNode::from_data(data!(MathExpressionNode { kind: kind, ..data }))
            });
        }
    }

    #[requires(matches!(
        object.object_kind(),
        crate::model::SemanticObjectKind::Referent
            | crate::model::SemanticObjectKind::Parameter
            | crate::model::SemanticObjectKind::MathExpression
    ))]
    #[ensures(true)]
    pub(super) fn attach_subscript_from_free_modifiers(
        &mut self,
        object: SemanticObjectId,
        free_modifiers: &[FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        let Some(subscript) = self.subscript_from_free_modifiers(free_modifiers)? else {
            return Ok(());
        };
        if let Some(object) = self.objects.get_mut(&object) {
            object.set_subscript(subscript);
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|subscript| subscript.as_ref().is_none_or(|subscript| subscript.value.object_kind() == crate::model::SemanticObjectKind::MathExpression)) || ret.is_err())]
    pub(super) fn subscript_from_free_modifiers(
        &mut self,
        free_modifiers: &[FreeModifierSyntax],
    ) -> Result<Option<Subscript>, SemanticsError> {
        let Some(free_modifier) = free_modifiers.iter().find_map(generated_xi_free_modifier) else {
            return Ok(None);
        };
        let (xi, expression_text) = match free_modifier {
            jbotci_syntax::generated_model::XiFreeModifierSyntax::XiNumberFreeModifier(
                subscript,
            ) => (
                &subscript.xi,
                generated_number_words_text(&subscript.expression.0.number.value),
            ),
            jbotci_syntax::generated_model::XiFreeModifierSyntax::XiLerfuStringFreeModifier(
                subscript,
            ) => (
                &subscript.xi,
                generated_letter_string_text(&subscript.expression.letters),
            ),
            jbotci_syntax::generated_model::XiFreeModifierSyntax::XiParenthesizedFreeModifier(
                subscript,
            ) => (
                &subscript.xi,
                generated_subscript_mekso_surface_text(&subscript.expression.inner_expression)?,
            ),
        };
        let value = self.build_generated_math_literal(
            math_literal_for_pa_text(expression_text),
            self.exact_source_for_node(free_modifier, "subscript-value"),
        )?;
        Ok(Some(Subscript::new(
            value,
            token_text(&xi.value),
            self.exact_source_for_node(free_modifier, "subscript"),
        )))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_lahe_sumti_referent(
        &mut self,
        sumti: &'tree LaheSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operand = self.build_sumti_referent(&sumti.inner_sumti)?;
        let sort = referent_qualifier_sort(sumti.lahe.value.cmavo());
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(new!(Descriptor {
                    kind: referent_qualifier_kind(sumti.lahe.value.cmavo()),
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
                })),
                None,
                self.source_for_node(sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_no_gadri_description_referent(
        &mut self,
        description: &'tree DescriptorWithoutGadriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        let quantity = self.build_quantity_for_quantifier(&description.quantifier)?;
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::Description,
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
                })),
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
        let Some(descriptor) = object.descriptor().cloned() else {
            return Err(invalid_graph(format!(
                "semantic builder no-gadri description referent {id} has no descriptor"
            )));
        };
        object.set_descriptor(descriptor.with_data(data! {
            body: Some(body),
        }));
        Ok(id)
    }

    #[requires(crate::model::argument_object_kind_can_fill(source.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(trailing.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_connected_generated_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
        source: SemanticObjectId,
        connective: &'tree ArgumentConnectiveSyntax,
        trailing: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let interval_connective = generated_argument_connective_is_interval(connective);
        let logical_connective = generated_argument_connective_is_logical(connective);
        let operator_parameter =
            self.build_generated_connective_question_parameter_for_argument_connective(connective)?;
        let right_negated = operator_parameter.is_none()
            && generated_argument_connective_negates_right(connective)
            && logical_connective;
        let complement = (operator_parameter.is_none()
            && interval_connective
            && generated_argument_connective_negates_right(connective))
        .then_some(true);
        let scalar_negated = (operator_parameter.is_none()
            && !logical_connective
            && !interval_connective
            && generated_argument_connective_negates_right(connective))
        .then_some(true);
        let operator = if operator_parameter.is_some() {
            CompositionOperator::ConnectiveQuestion
        } else if logical_connective {
            CompositionOperator::Joint
        } else {
            generated_nonlogical_argument_composition_operator(connective)?
        };
        let reverse_members =
            generated_argument_connective_reverses_composition_members(connective);
        let (first, second) = if reverse_members {
            (trailing, source)
        } else {
            (source, trailing)
        };
        let members = if right_negated {
            vec![source]
        } else {
            vec![first, second]
        };
        let excluded_members = if right_negated {
            vec![trailing]
        } else {
            Vec::new()
        };
        let collective = operator.is_mass().then_some(true);
        let endpoint_inclusion =
            generated_argument_connective_endpoint_inclusion(connective, reverse_members);
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
                    operator_parameter,
                    members,
                    excluded_members,
                    collective,
                    scalar_negated,
                    complement,
                    endpoint_inclusion,
                })),
                self.source_for_node(node, "connected-sumti"),
                Vec::new(),
            ),
        )?;
        if let Some(anchor) = self.current_utterance {
            let trailing_parts = generated_argument_connective_head_indicator_parts(connective);
            if !trailing_parts.is_empty()
                && displayed_content_target_kind_is_allowed(trailing.object_kind())
            {
                self.attach_generated_indicator_displays_with_target_focus(
                    trailing_parts,
                    trailing,
                    anchor,
                    "indicator",
                    None,
                    false,
                )?;
            }
            let composite_parts =
                generated_argument_connective_modifier_indicator_parts(connective);
            if !composite_parts.is_empty()
                && displayed_content_target_kind_is_allowed(id.object_kind())
            {
                self.attach_generated_indicator_displays_with_target_focus(
                    composite_parts,
                    id,
                    anchor,
                    "indicator",
                    None,
                    false,
                )?;
            }
        }
        Ok(id)
    }

    #[requires(crate::model::argument_object_kind_can_fill(source.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(trailing.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_connected_generated_forethought_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
        source: SemanticObjectId,
        connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
        gik: &'tree GikConnectiveSyntax,
        trailing: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let interval_connective = generated_modal_forethought_connective_is_interval(connective);
        let logical_connective = generated_modal_forethought_connective_is_logical(connective);
        let operator_parameter = self
            .build_generated_connective_question_parameter_for_modal_forethought_connective(
                connective,
            )?;
        let right_negated = operator_parameter.is_none() && gik.nai.is_some() && logical_connective;
        let complement = (operator_parameter.is_none() && interval_connective && gik.nai.is_some())
            .then_some(true);
        let scalar_negated = (operator_parameter.is_none()
            && !logical_connective
            && !interval_connective
            && gik.nai.is_some())
        .then_some(true);
        let operator = if operator_parameter.is_some() {
            CompositionOperator::ConnectiveQuestion
        } else if logical_connective {
            CompositionOperator::Joint
        } else {
            generated_nonlogical_modal_forethought_composition_operator(connective)?
        };
        let reverse_members =
            generated_modal_forethought_connective_reverses_composition_members(connective);
        let (first, second) = if reverse_members {
            (trailing, source)
        } else {
            (source, trailing)
        };
        let members = if right_negated {
            vec![source]
        } else {
            vec![first, second]
        };
        let excluded_members = if right_negated {
            vec![trailing]
        } else {
            Vec::new()
        };
        let collective = operator.is_mass().then_some(true);
        let endpoint_inclusion =
            generated_modal_forethought_connective_endpoint_inclusion(connective, reverse_members);
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
                    operator_parameter,
                    members,
                    excluded_members,
                    collective,
                    scalar_negated,
                    complement,
                    endpoint_inclusion,
                })),
                self.source_for_node(node, "connected-sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(crate::model::argument_object_kind_can_fill(source.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(trailing.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_connected_generated_extra_forethought_sumti_referent<N: TreeNode>(
        &mut self,
        node: &N,
        source: SemanticObjectId,
        connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
        trailing: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let logical_connective = generated_modal_forethought_connective_is_logical(connective);
        let operator_parameter = self
            .build_generated_connective_question_parameter_for_modal_forethought_connective(
                connective,
            )?;
        let operator = if operator_parameter.is_some() {
            CompositionOperator::ConnectiveQuestion
        } else if logical_connective {
            CompositionOperator::Joint
        } else {
            generated_nonlogical_modal_forethought_composition_operator(connective)?
        };
        let reverse_members =
            generated_modal_forethought_connective_reverses_composition_members(connective);
        let (first, second) = if reverse_members {
            (trailing, source)
        } else {
            (source, trailing)
        };
        let collective = operator.is_mass().then_some(true);
        let endpoint_inclusion =
            generated_modal_forethought_connective_endpoint_inclusion(connective, reverse_members);
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
                    operator_parameter,
                    members: vec![first, second],
                    excluded_members: Vec::new(),
                    collective,
                    scalar_negated: None,
                    complement: None,
                    endpoint_inclusion,
                })),
                self.source_for_node(node, "connected-sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_pro_sumti_referent(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.queue_generated_vocative_asides(&pro_sumti.0.free_modifiers)?;
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
            Some(Cmavo::Ri) => {
                if let Some(referent) = self.build_pending_previous_sumti_referent(pro_sumti)? {
                    return Ok(referent);
                }
                let recent_offset = generated_pro_sumti_positive_xi_offset(pro_sumti).unwrap_or(1);
                self.recent_sumti_referent_before_node(pro_sumti, recent_offset)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        self.build_generated_pro_sumti_fallback_referent(
                            pro_sumti,
                            ReferentCategory::Constant,
                        )
                    })
            }
            Some(Cmavo::Keha) => {
                let offset = generated_pro_sumti_positive_xi_offset(pro_sumti).unwrap_or(1);
                self.relative_head_stack
                    .iter()
                    .rev()
                    .nth(offset - 1)
                    .copied()
                    .or(self.relative_head)
                    .ok_or_else(|| unsupported("relative head pro-sumti outside relative clause"))
            }
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
            Some(cmavo) if is_assignable_koha(cmavo) => self
                .assigned_referents
                .get(&token_text(&pro_sumti.0.value))
                .copied()
                .map(Ok)
                .unwrap_or_else(|| {
                    self.build_generated_pro_sumti_fallback_referent(
                        pro_sumti,
                        ReferentCategory::Constant,
                    )
                }),
            Some(Cmavo::Da | Cmavo::De | Cmavo::Di)
                if with_free_modifiers_has_indicator_cmavo(&pro_sumti.0, Cmavo::Kau) =>
            {
                self.build_indefinite_kau_argument_parameter(pro_sumti)
            }
            Some(Cmavo::Da | Cmavo::De | Cmavo::Di) => {
                if let Some(variable) = self.generated_prenex_bound_pro_sumti(pro_sumti)? {
                    Ok(variable)
                } else if let Some(variable) =
                    self.generated_implicit_da_series_bound_pro_sumti(pro_sumti)
                {
                    Ok(variable)
                } else {
                    self.build_implicit_existential_variable(pro_sumti)
                }
            }
            _ => self
                .build_generated_pro_sumti_fallback_referent(pro_sumti, ReferentCategory::Constant),
        }
    }

    #[requires(pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent)) || ret.is_err())]
    pub(super) fn generated_prenex_bound_pro_sumti(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let key = token_text(&pro_sumti.0.value);
        let Some(index) = self
            .prenex_pro_sumti_bindings
            .get(&key)
            .and_then(|bindings| bindings.len().checked_sub(1))
        else {
            return Ok(None);
        };
        if let Some(variable) = self
            .prenex_pro_sumti_bindings
            .get(&key)
            .and_then(|bindings| bindings.get(index))
            .and_then(|binding| binding.variable)
        {
            if self.suppress_prenex_bound_implicit_existential_recording == 0 {
                self.record_generated_implicit_existential_once(
                    variable,
                    self.source_for_node(pro_sumti, "quantifier-scope"),
                );
            }
            return Ok(Some(variable));
        }
        let (word, source, scope_key) = {
            let binding = self
                .prenex_pro_sumti_bindings
                .get(&key)
                .and_then(|bindings| bindings.get(index))
                .ok_or_else(|| invalid_graph(format!("missing generated prenex binding {key}")))?;
            (
                binding.word.clone(),
                binding.source.clone(),
                binding.scope_key,
            )
        };
        let variable = self.next_referent_id();
        self.insert(
            variable,
            SemanticObject::referent(
                ReferentCategory::Variable,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::ProSumti,
                    word,
                    speaker: None,
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        if let Some(scope_key) = scope_key {
            self.scoped_argument_variables.insert(scope_key, variable);
        }
        let bindings = self
            .prenex_pro_sumti_bindings
            .get_mut(&key)
            .ok_or_else(|| invalid_graph(format!("missing generated prenex binding {key}")))?;
        bindings[index] = bindings[index]
            .clone()
            .with_data(data! { variable: Some(variable) });
        if self.suppress_prenex_bound_implicit_existential_recording == 0 {
            self.record_generated_implicit_existential_once(
                variable,
                self.source_for_node(pro_sumti, "quantifier-scope"),
            );
        }
        Ok(Some(variable))
    }

    #[requires(existentials.iter().all(|existential| matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(ret.iter().all(|existential| matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    pub(super) fn generated_implicit_existentials_for_active_prenex_bindings(
        &self,
        existentials: &[GeneratedImplicitExistential],
    ) -> Vec<GeneratedImplicitExistential> {
        existentials
            .iter()
            .filter(|existential| {
                self.generated_variable_has_active_prenex_binding(existential.variable)
            })
            .cloned()
            .collect()
    }

    #[requires(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(true)]
    pub(super) fn generated_variable_has_active_prenex_binding(
        &self,
        variable: SemanticObjectId,
    ) -> bool {
        self.prenex_pro_sumti_bindings.values().any(|bindings| {
            bindings
                .iter()
                .any(|binding| binding.variable == Some(variable))
        }) || self
            .prenex_relation_variable_bindings
            .values()
            .any(|bindings| bindings.iter().any(|binding| binding.parameter == variable))
    }

    #[requires(pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di)))]
    #[ensures(ret.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent))]
    pub(super) fn generated_implicit_da_series_bound_pro_sumti(
        &self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Option<SemanticObjectId> {
        self.implicit_da_series_bindings
            .get(&token_text(&pro_sumti.0.value))
            .copied()
    }

    #[requires(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(self.implicit_existential_variables.iter().any(|existential| existential.variable == variable) || self.recorded_implicit_existential_variables.contains(&variable))]
    pub(super) fn record_generated_implicit_existential_once(
        &mut self,
        variable: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) {
        if self
            .implicit_existential_variables
            .iter()
            .any(|existential| existential.variable == variable)
            || self
                .recorded_implicit_existential_variables
                .contains(&variable)
        {
            return;
        }
        self.recorded_implicit_existential_variables
            .insert(variable);
        self.implicit_existential_variables
            .push(new!(GeneratedImplicitExistential {
                variable,
                source,
                restrictions: Vec::new(),
            }));
    }

    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(true)]
    pub(super) fn add_generated_implicit_existential_restrictions(
        &mut self,
        variable: SemanticObjectId,
        restrictions: Vec<SemanticObjectId>,
    ) {
        if restrictions.is_empty() {
            return;
        }
        if let Some(existential) = self
            .implicit_existential_variables
            .iter_mut()
            .find(|existential| existential.variable == variable)
        {
            let mut data = existential.clone().into_data();
            data.restrictions.extend(restrictions);
            *existential = GeneratedImplicitExistential::from_data(data);
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_pro_sumti_fallback_referent(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
        category: ReferentCategory,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                category,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::ProSumti,
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
                })),
                None,
                self.source_for_node(pro_sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_argument_question_parameter(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter =
            self.build_generated_parameter(pro_sumti, ParameterRole::ArgumentQuestion)?;
        if with_free_modifiers_has_indicator_cmavo(&pro_sumti.0, Cmavo::Kau)
            && self.record_generated_indirect_question_focus(
                GeneratedIndirectQuestionFocus::from_data(data!(GeneratedIndirectQuestionFocus {
                    focus: parameter,
                    presupposed_answer: None,
                    slots: vec![new!(QuestionSlot {
                        parameter,
                        role: QuestionSlotRole::Answer,
                    })],
                    kind: QuestionKind::Argument,
                    domain: SemanticSort::Entity,
                    source: self.source_for_node(pro_sumti, "indirect-question"),
                })),
            )
        {
            return Ok(parameter);
        }
        self.argument_question_parameters.push(parameter);
        Ok(parameter)
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_generated_place_question_parameter(
        &mut self,
        introduced_by: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Place,
                ParameterRole::PlaceQuestion,
                introduced_by,
                source,
            ),
        )?;
        self.place_question_parameters.push(parameter);
        Ok(parameter)
    }

    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(ret.as_ref().is_ok_and(|bindings| bindings.iter().all(|question| !question.candidate_places.is_empty())) || ret.is_err())]
    pub(super) fn build_generated_place_question_bindings(
        &mut self,
        place_questions: &[GeneratedPlaceQuestionAssignment],
        arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
        place_count: Option<usize>,
        highest_assigned_place: usize,
    ) -> Result<Vec<PlaceQuestionBinding>, SemanticsError> {
        if place_questions.is_empty() {
            return Ok(Vec::new());
        }
        let occupied = arguments
            .keys()
            .map(|place| argument_place_index(place))
            .collect::<HashSet<_>>();
        let candidate_limit = place_count.unwrap_or_else(|| highest_assigned_place.max(1));
        let candidate_places = (1..=candidate_limit)
            .filter(|place| !occupied.contains(place))
            .map(argument_key)
            .collect::<Vec<_>>();
        if candidate_places.is_empty() {
            return Ok(Vec::new());
        }
        let mut bindings = Vec::with_capacity(place_questions.len());
        for question in place_questions {
            let parameter = self.build_generated_place_question_parameter(
                question.introduced_by.clone(),
                question.parameter_source.clone(),
            )?;
            bindings.push(PlaceQuestionBinding::new(
                parameter,
                question.argument.clone(),
                candidate_places.clone(),
                question.binding_source.clone(),
            ));
        }
        Ok(bindings)
    }

    #[requires(pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di)))]
    #[requires(with_free_modifiers_has_indicator_cmavo(&pro_sumti.0, Cmavo::Kau))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_indefinite_kau_argument_parameter(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter =
            self.build_generated_parameter(pro_sumti, ParameterRole::ArgumentQuestion)?;
        if self.record_generated_indirect_question_focus(GeneratedIndirectQuestionFocus::from_data(
            data!(GeneratedIndirectQuestionFocus {
                focus: parameter,
                presupposed_answer: None,
                slots: vec![new!(QuestionSlot {
                    parameter,
                    role: QuestionSlotRole::Answer,
                })],
                kind: QuestionKind::Argument,
                domain: SemanticSort::Entity,
                source: self.source_for_node(pro_sumti, "indirect-question"),
            }),
        )) {
            return Ok(parameter);
        }
        self.argument_question_parameters.push(parameter);
        Ok(parameter)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_generated_parameter(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
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
    pub(super) fn build_implicit_existential_variable(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(pro_sumti, "sumti");
        let variable = self.next_referent_id();
        self.insert(
            variable,
            SemanticObject::referent(
                ReferentCategory::Variable,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::ProSumti,
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
                })),
                None,
                source.clone(),
                Vec::new(),
            ),
        )?;
        self.implicit_existential_variables
            .push(new!(GeneratedImplicitExistential {
                variable,
                source: self.source_for_node(pro_sumti, "quantifier-scope"),
                restrictions: Vec::new(),
            }));
        self.implicit_da_series_bindings
            .insert(token_text(&pro_sumti.0.value), variable);
        Ok(variable)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_typical_place_value_referent(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::TypicalPlaceValue,
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
                })),
                None,
                self.source_for_node(pro_sumti, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_utterance_reference_referent(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
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
            Some(new!(Descriptor {
                kind: DescriptorKind::UtteranceReference,
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
            })),
            None,
            self.source_for_node(pro_sumti, "sumti"),
            diagnostics,
        );
        object.set_referent_target(target);
        self.insert(id, object)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_demonstrative_referent(
        &mut self,
        pro_sumti: &'tree ProSumtiSyntax,
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
    pub(super) fn build_name_sumti_referent(
        &mut self,
        name: &'tree NameSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.queue_generated_vocative_asides(&name.names.free_modifiers)?;
        let sort = gadri_name_sort(name.la.value.cmavo());
        let id = self.next_referent_with_sort_id(sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                sort,
                None,
                Some(new!(Descriptor {
                    kind: name_description_kind_for_cmavo(name.la.value.cmavo()),
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
                })),
                None,
                self.source_for_name_sumti(name, "sumti"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_description_referent(
        &mut self,
        description: &'tree DescriptorWithGadriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_gadri_description_referent(
            description,
            &description.description,
            &description.tail,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_outer_quantified_description_referent(
        &mut self,
        description: &'tree DescriptorWithOuterQuantifierSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let description_source =
            self.source_for_outer_quantified_description_domain(description, "description");
        self.build_gadri_description_referent_with_source(
            description,
            &description.description,
            &description.tail,
            description_source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_gadri_description_referent<N: TreeNode>(
        &mut self,
        description_node: &N,
        description_head: &'tree DescriptionHeadSyntax,
        tail: &'tree DescriptionTailSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let description_source = self.exact_source_for_node(description_node, "description");
        self.build_gadri_description_referent_with_source(
            description_node,
            description_head,
            tail,
            description_source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_gadri_description_referent_with_source<N: TreeNode>(
        &mut self,
        description_node: &N,
        description_head: &'tree DescriptionHeadSyntax,
        tail: &'tree DescriptionTailSyntax,
        description_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (selbri, relative_clauses, tail_quantity, body_operand_sumti) = match tail.tail.as_ref()
        {
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
        let kind = description_kind_for_cmavo(cmavo);
        if let Some(spec) = cmavo.and_then(aggregate_description_spec) {
            return self.build_generated_aggregate_description_referent(
                description_node,
                tail,
                spec,
                kind,
                word,
            );
        }
        let abstraction = selbri
            .map(Self::generated_description_abstraction_for_selbri)
            .transpose()?
            .flatten();
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
                Some(new!(Descriptor {
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
                })),
                None,
                description_source.clone(),
                Vec::new(),
            ),
        )?;
        if leading_operand_sumti.is_none()
            && let Some(relative_clauses) = &leading_tail_elements.relative_clauses
        {
            self.push_generated_goi_assigned_names_to_referent(id, relative_clauses)?;
        }
        if let Some(relative_clauses) = relative_clauses {
            self.push_generated_goi_assigned_names_to_referent(id, relative_clauses)?;
        }
        let body = if let Some(selbri) = selbri {
            self.suppress_prenex_bound_implicit_existential_recording += 1;
            let body = self.build_generated_description_body_formula_for_cmavo(
                description_node,
                selbri,
                id,
                cmavo,
                abstraction,
                description_source.clone(),
            );
            self.suppress_prenex_bound_implicit_existential_recording -= 1;
            Some(body?)
        } else {
            None
        };
        let mut descriptor_operand = None;
        let mut lowered_relative_clauses = Vec::new();
        let quantity_before_descriptor_relative_clauses =
            leading_operand_sumti.is_none() && body_operand_sumti.is_none();
        let mut quantity = None;
        if quantity_before_descriptor_relative_clauses {
            quantity = tail_quantity
                .map(|quantifier| self.build_quantity_for_quantifier(quantifier))
                .transpose()?;
        }
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
            self.suppress_prenex_bound_implicit_existential_recording += 1;
            let operand = self.build_sumti_base_referent(operand_sumti);
            self.suppress_prenex_bound_implicit_existential_recording -= 1;
            let operand = operand?;
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
        if !quantity_before_descriptor_relative_clauses {
            quantity = tail_quantity
                .map(|quantifier| self.build_quantity_for_quantifier(quantifier))
                .transpose()?;
        }
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find description referent {id}"
            ))
        })?;
        let Some(descriptor) = object.descriptor().cloned() else {
            return Err(invalid_graph(format!(
                "semantic builder description referent {id} has no descriptor"
            )));
        };
        object.set_descriptor(descriptor.with_data(data! {
            body: body,
            operand: descriptor_operand,
            quantity: quantity,
            relative_clauses: lowered_relative_clauses,
        }));
        Ok(id)
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_aggregate_description_referent<N: TreeNode>(
        &mut self,
        description_node: &N,
        tail: &'tree DescriptionTailSyntax,
        spec: AggregateDescriptionSpec,
        kind: DescriptorKind,
        word: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relative_clauses = match tail.tail.as_ref() {
            DescriptionTailBodySyntax::RelationDescriptionTail(RelationDescriptionTailSyntax {
                relative_clauses,
                ..
            })
            | DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(
                QuantifierRelationDescriptionTailSyntax {
                    relative_clauses, ..
                },
            ) => relative_clauses.as_ref(),
            DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(_) => None,
        };
        let id = self.next_referent_with_sort_id(spec.sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                spec.sort,
                None,
                Some(new!(Descriptor {
                    kind,
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
                })),
                None,
                self.source_for_node(description_node, "description"),
                Vec::new(),
            ),
        )?;
        if let Some(relative_clauses) = relative_clauses {
            self.push_generated_goi_assigned_names_to_referent(id, relative_clauses)?;
        }

        let member =
            self.build_generated_aggregate_member_referent(description_node, tail, spec)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(id, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(member, None));
        let body = self.build_structural_formula_from_arguments(
            spec.relation,
            arguments,
            PredicationMode::Restrictive,
            self.source_for_node(description_node, "aggregate-description"),
        )?;
        let lowered_relative_clauses = relative_clauses
            .map(|relative_clauses| self.lower_generated_relative_clause_list(relative_clauses, id))
            .transpose()?
            .unwrap_or_default();
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find aggregate description referent {id}"
            ))
        })?;
        let Some(descriptor) = object.descriptor().cloned() else {
            return Err(invalid_graph(format!(
                "semantic builder aggregate description referent {id} has no descriptor"
            )));
        };
        object.set_descriptor(descriptor.with_data(data! {
            body: Some(body),
            relative_clauses: lowered_relative_clauses,
        }));
        Ok(id)
    }

    #[requires(!spec.member_word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_aggregate_member_referent<N: TreeNode>(
        &mut self,
        description_node: &N,
        tail: &'tree DescriptionTailSyntax,
        spec: AggregateDescriptionSpec,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (selbri, quantity, body_operand_sumti) = match tail.tail.as_ref() {
            DescriptionTailBodySyntax::RelationDescriptionTail(RelationDescriptionTailSyntax {
                selbri,
                ..
            }) => (Some(selbri.as_ref()), None, None),
            DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(
                QuantifierRelationDescriptionTailSyntax {
                    quantifier, selbri, ..
                },
            ) => (Some(selbri.as_ref()), Some(quantifier), None),
            DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(
                QuantifierSumtiDescriptionTailSyntax { quantifier, sumti },
            ) => (None, Some(quantifier), Some(sumti.as_ref())),
        };
        let leading_tail_elements = &tail.leading_tail_elements;
        let leading_operand_sumti = leading_tail_elements
            .tail_sumti
            .as_ref()
            .map(|tail_sumti| tail_sumti.0.as_ref());
        if leading_operand_sumti.is_some() && body_operand_sumti.is_some() {
            return Err(unsupported("multiple aggregate description operands"));
        }
        let id = self.next_referent_id();
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: description_kind_for_cmavo(Some(spec.member_cmavo)),
                    word: spec.member_word.to_owned(),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                })),
                None,
                self.source_for_node(description_node, "aggregate-member-description"),
                Vec::new(),
            ),
        )?;
        let abstraction = selbri
            .map(Self::generated_description_abstraction_for_selbri)
            .transpose()?
            .flatten();
        let body = selbri
            .map(|selbri| {
                self.build_generated_description_body_formula_for_cmavo(
                    description_node,
                    selbri,
                    id,
                    Some(spec.member_cmavo),
                    abstraction,
                    self.source_for_node(description_node, "description"),
                )
            })
            .transpose()?;
        let mut descriptor_operand = None;
        if let Some(operand_sumti) = leading_operand_sumti {
            let operand = self.build_sumti_base_referent(operand_sumti)?;
            if selbri.is_none() {
                descriptor_operand = Some(operand);
            }
        }
        if let Some(operand_sumti) = body_operand_sumti {
            descriptor_operand = Some(self.build_sumti_referent(operand_sumti)?);
        }
        let quantity = quantity
            .map(|quantifier| self.build_quantity_for_quantifier(quantifier))
            .transpose()?;
        let lowered_relative_clauses = leading_tail_elements
            .relative_clauses
            .as_ref()
            .map(|relative_clauses| {
                self.lower_generated_descriptor_relative_clause_list(relative_clauses, id)
            })
            .transpose()?
            .unwrap_or_default();
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find aggregate member referent {id}"
            ))
        })?;
        let Some(descriptor) = object.descriptor().cloned() else {
            return Err(invalid_graph(format!(
                "semantic builder aggregate member referent {id} has no descriptor"
            )));
        };
        object.set_descriptor(descriptor.with_data(data! {
            body: body,
            operand: descriptor_operand,
            quantity: quantity,
            relative_clauses: lowered_relative_clauses,
        }));
        Ok(id)
    }

    #[requires(crate::model::argument_object_kind_can_fill(head.object_kind()))]
    #[requires(crate::model::argument_object_kind_can_fill(operand.object_kind()))]
    #[ensures(true)]
    pub(super) fn build_generated_possessive_association_clause<N: TreeNode>(
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
        arguments.insert(argument_key(1), ArgumentValue::filled(head, None));
        arguments.insert(argument_key(2), associated_argument);
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

    #[requires(!word.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_abstraction_description_output(
        &mut self,
        source: Option<crate::model::SemanticSource>,
        cmavo: Option<Cmavo>,
        abstraction: &'tree AbstractionTanruUnitSyntax,
        kind: DescriptorKind,
        word: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !abstraction.abstractor_connections.is_empty() {
            return self.build_connected_abstraction_description_output(
                source,
                cmavo,
                abstraction,
                kind,
                word,
            );
        }
        let id = self.build_abstraction_output(abstraction, source.clone())?;
        let speaker = self.current_speaker();
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find abstraction description output {id}"
            ))
        })?;
        object.set_descriptor(new!(Descriptor {
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
        }));
        object.replace_source(source);
        Ok(id)
    }

    #[requires(!word.is_empty())]
    #[requires(!abstraction.abstractor_connections.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_connected_abstraction_description_output(
        &mut self,
        source: Option<crate::model::SemanticSource>,
        cmavo: Option<Cmavo>,
        abstraction: &'tree AbstractionTanruUnitSyntax,
        kind: DescriptorKind,
        word: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if abstraction.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        let output_sort = abstraction_output_sort(abstraction_kind_for_nu(abstraction));
        let id = self.next_referent_with_sort_id(output_sort);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                output_sort,
                None,
                Some(new!(Descriptor {
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
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        let body = self.build_connected_abstraction_description_body(id, abstraction)?;
        let object = self.objects.get_mut(&id).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find connected abstraction description output {id}"
            ))
        })?;
        if let Some(descriptor) = object.descriptor().cloned() {
            object.set_descriptor(descriptor.with_data(data! {
                body: Some(body),
            }));
        }
        Ok(id)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(!abstraction.abstractor_connections.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_abstraction_description_body(
        &mut self,
        referent: SemanticObjectId,
        abstraction: &'tree AbstractionTanruUnitSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut current = self.build_single_abstraction_description_link_formula(
            referent,
            GeneratedAbstractionBranch::primary(abstraction),
        )?;
        for connection in &abstraction.abstractor_connections {
            let next = self.build_single_abstraction_description_link_formula(
                referent,
                GeneratedAbstractionBranch::connected(abstraction, connection),
            )?;
            let connective = statement_connective_from_standard(&connection.connective);
            current = self.build_binary_formula_for_generated_abstraction_connective(
                &connective,
                current,
                next,
                self.source_for_node(abstraction, "abstraction-connection-formula"),
            )?;
        }
        Ok(current)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_single_abstraction_description_link_formula(
        &mut self,
        referent: SemanticObjectId,
        branch: GeneratedAbstractionBranch<'tree>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let output = self.build_abstraction_output_for_branch(
            branch,
            self.source_for_abstraction_branch(branch, "abstraction"),
        )?;
        let kind = abstraction_kind_for_cmavo(branch.nu.value.cmavo());
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(referent, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(output, None));
        let predication = self.next_predication_id();
        self.insert_generated_abstraction_link_extra_argument(kind, &mut arguments)?;
        self.insert(
            predication,
            SemanticObject::predication(
                abstraction_link_relation(kind).to_owned(),
                None,
                arguments,
                PredicationMode::Restrictive,
                self.source_for_abstraction_branch(branch, "abstraction-description"),
                Vec::new(),
            ),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(
                predication,
                self.source_for_abstraction_branch(branch, "restrictive-formula"),
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(arguments.contains_key(&argument_key(1)))]
    #[requires(arguments.contains_key(&argument_key(2)))]
    #[ensures(true)]
    pub(super) fn insert_generated_abstraction_link_extra_argument(
        &mut self,
        kind: AbstractionKind,
        arguments: &mut BTreeMap<PlaceIndex, ArgumentValue>,
    ) -> Result<(), SemanticsError> {
        let Some(_surface_place) = abstraction_extra_surface_place(kind) else {
            return Ok(());
        };
        arguments.insert(
            argument_key(3),
            self.build_elided_argument_with_sort("zo'e".to_owned(), SemanticSort::Entity)?,
        );
        Ok(())
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_abstraction_connective(
        &mut self,
        connective: &StatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_statement_connective_formula_operator_for_core(connective);
        let Some(truth_table) = generated_statement_connective_core_truth_table(connective) else {
            return Err(unsupported("nonlogical abstraction connective"));
        };
        self.mark_generated_statement_whether_or_not_inert_operand(connective, left, right);
        let left = if generated_statement_connective_negates_left(connective) {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right = if generated_statement_connective_negates_right(connective) {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        let children = if generated_statement_connective_has_se(connective)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let connector_parameter =
            build_generated_connective_question_parameter_for_statement_connective(
                self, connective,
            )?;
        let connector_source = generated_statement_connective_core_source(connective)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                if connector_parameter.is_some() {
                    FormulaOperator::ConnectiveQuestion
                } else {
                    operator
                },
                children,
                Some(new!(Connector {
                    source: connector_source,
                    locus: "abstraction".to_owned(),
                    truth_table: Some(truth_table),
                    parameter: connector_parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_description_body_formula_for_cmavo<N: TreeNode>(
        &mut self,
        description_node: &N,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
        cmavo: Option<Cmavo>,
        abstraction: Option<GeneratedDescriptionAbstraction<'tree>>,
        description_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(abstraction) = abstraction {
            return self.build_generated_abstraction_description_formula(
                selbri,
                referent,
                abstraction,
            );
        }
        if matches!(
            description_characterization_for_cmavo(cmavo),
            DescriptionCharacterization::SpeakerDescribed
        ) && generated_selbri_requires_direct_description_body(selbri)?
        {
            return self.build_restrictive_formula(selbri, referent);
        }
        match description_characterization_for_cmavo(cmavo) {
            DescriptionCharacterization::SpeakerDescribed => {
                let source = source_with_construct(description_source, "speaker-description")
                    .or_else(|| self.source_for_node(description_node, "speaker-description"));
                self.build_speaker_description_formula(source, selbri, referent)
            }
            DescriptionCharacterization::Named => self
                .build_generated_selbri_name_description_formula(
                    description_node,
                    selbri,
                    referent,
                ),
            DescriptionCharacterization::Veridical => {
                self.build_restrictive_formula(selbri, referent)
            }
        }
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(!abstraction.link_relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_abstraction_description_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
        abstraction: GeneratedDescriptionAbstraction<'tree>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let output = self.build_abstraction_output(
            abstraction.abstraction,
            self.source_for_node(abstraction.abstraction, "abstraction"),
        )?;
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(referent, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(output, None));
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
    pub(super) fn build_speaker_description_formula(
        &mut self,
        source: Option<crate::model::SemanticSource>,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let property = self.build_description_property_abstraction_for_selbri(selbri)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(
            argument_key(1),
            ArgumentValue::filled(self.current_speaker(), None),
        );
        arguments.insert(argument_key(2), ArgumentValue::filled(referent, None));
        arguments.insert(
            argument_key(3),
            ArgumentValue::filled(self.current_audience(), None),
        );
        arguments.insert(argument_key(4), ArgumentValue::filled(property, None));
        self.build_structural_formula_from_arguments(
            "skicu",
            arguments,
            PredicationMode::Incidental,
            source,
        )
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_selbri_name_description_formula<N: TreeNode>(
        &mut self,
        description_node: &N,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sign = self.build_generated_selbri_name_sign(selbri)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(sign, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(referent, None));
        arguments.insert(
            argument_key(3),
            ArgumentValue::filled(self.current_speaker(), None),
        );
        self.build_structural_formula_from_arguments(
            "cmene",
            arguments,
            PredicationMode::Incidental,
            self.exact_source_for_node(description_node, "name-description"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Sign)) || ret.is_err())]
    pub(super) fn build_generated_selbri_name_sign(
        &mut self,
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(selbri, "name-sign");
        let denotation = self.build_description_property_abstraction_for_selbri_with_source(
            selbri,
            source_as_name_denotation(source.clone()),
        )?;
        let id = self.next_sign_id();
        let mut object = SemanticObject::text_sign(
            SignKind::Word,
            generated_selbri_surface_text(selbri)?,
            source,
            Vec::new(),
        );
        object.set_sign_denotes(Some(denotation));
        self.insert(id, object)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_description_property_abstraction_for_selbri(
        &mut self,
        selbri: &'tree SelbriSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_description_property_abstraction_for_selbri_with_source(
            selbri,
            self.source_for_node(selbri, "speaker-description-property"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_description_property_abstraction_for_selbri_with_source(
        &mut self,
        selbri: &'tree SelbriSyntax,
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
    pub(super) fn build_restrictive_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::NegatedSelbri(negated)) = selbri {
            let child = self.build_restrictive_formula(&negated.inner_selbri, referent)?;
            return self.build_unary_formula(
                FormulaOperator::Not,
                child,
                self.source_for_node(selbri, "bridi-negation"),
            );
        }
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
            let source = self.source_for_node(selbri, "restrictive-predication");
            if let Some(question) =
                relation_question_syntax_from_generated_tanru_unit(&tanru.first_unit)?
            {
                return self.build_property_atom_for_generated_relation_question(
                    question,
                    referent,
                    self.source_for_node(selbri, "restrictive-formula"),
                );
            }
            if let Some(cmavo) =
                resolvable_generated_pro_bridi_cmavo_from_tanru_unit(&tanru.first_unit)?
                && let Some(formula) = self
                    .build_restrictive_formula_for_generated_pro_bridi_frame(
                        cmavo,
                        referent,
                        self.source_for_node(selbri, "restrictive-formula"),
                    )?
            {
                return Ok(formula);
            }
            if generated_bare_jai_modal_tanru_unit_from_tanru_unit(&tanru.first_unit)?.is_some() {
                return self
                    .build_relation_formula_for_generated_tanru_unit_argument_with_eventuality(
                        &tanru.first_unit,
                        ArgumentValue::filled(referent, None),
                        None,
                        PredicationMode::Restrictive,
                        None,
                        source.clone(),
                        source,
                    );
            }
            if let Some(unit) =
                generated_jai_modal_tanru_unit_with_tense_from_tanru_unit(&tanru.first_unit)?
            {
                return self.build_restrictive_generated_jai_modal_conversion_formula(
                    selbri,
                    &tanru.first_unit,
                    unit,
                    referent,
                );
            }
            if !tanru.first_unit.0.links.is_empty() {
                let mut visible_arguments = BTreeMap::new();
                insert_visible_argument(
                    &mut visible_arguments,
                    1,
                    ArgumentValue::filled(referent, None),
                )?;
                return self.build_property_formula_for_tanru_selbri_with_visible_arguments(
                    tanru,
                    visible_arguments,
                    self.source_for_node(selbri, "restrictive-selbri-formula"),
                    GeneratedPropertyTanruContext::Description,
                    None,
                );
            }
            return self.build_relation_formula_for_generated_tanru_unit_argument(
                &tanru.first_unit,
                ArgumentValue::filled(referent, None),
                PredicationMode::Restrictive,
                self.source_for_node(selbri, "restrictive-predication"),
                self.source_for_node(selbri, "restrictive-formula"),
            );
        }
        if let Some(tanru) = tanru_selbri_from_selbri(selbri)?
            && !tanru.additional_units.is_empty()
        {
            let mut visible_arguments = BTreeMap::new();
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::filled(referent, None),
            )?;
            return self.build_property_formula_for_tanru_selbri_with_visible_arguments(
                tanru,
                visible_arguments,
                self.source_for_node(selbri, "restrictive-tanru-formula"),
                GeneratedPropertyTanruContext::Description,
                None,
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
        arguments.insert(argument_key(1), ArgumentValue::filled(referent, None));
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
                relation.display_text(),
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

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_restrictive_generated_jai_modal_conversion_formula(
        &mut self,
        selbri: &'tree SelbriSyntax,
        unit: &'tree TanruUnitSyntax,
        jai_unit: &'tree JaiModalTanruUnitSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (atom, linkargs) = generated_linked_tanru_unit_parts(unit)?;
        let relation = semantic_relation_label(relation_label_from_tanru_unit_atom_base(
            atom.base.as_ref(),
        )?);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let mut visible_arguments = BTreeMap::new();
        let mut modal_arguments = Vec::new();
        if let Some(linkargs) = linkargs {
            modal_arguments =
                self.extend_visible_arguments_with_linkargs(&mut visible_arguments, linkargs, 2)?;
        }
        let event_modifier_anchor = jai_unit
            .tense_modal
            .as_deref()
            .filter(|tense_modal| generated_tense_modal_has_event_modifier(*tense_modal))
            .map(|_| referent);
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let place = mapped_place_for_generated_conversions(visible_place, &atom.conversions)?;
            let key = argument_key(place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated restrictive jai arguments map to {key}"
                )));
            }
        }
        let visible_x1_place = mapped_place_for_generated_conversions(1, &atom.conversions)?;
        let visible_x1_key = argument_key(visible_x1_place);
        if !arguments.contains_key(&visible_x1_key) {
            arguments.insert(
                visible_x1_key,
                self.build_elided_argument_for_place(visible_x1_place)?,
            );
        }
        let jai_modal_argument =
            self.build_generated_jai_modal_argument_for_argument_object(jai_unit, referent)?;
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only places required by explicit assignments are represented",
                    ));
                }
                highest_argument.max(visible_x1_place)
            }
        };
        for place in 1..=place_limit.max(highest_argument).max(visible_x1_place) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let source = self.source_for_node(selbri, "restrictive-predication");
        let eventuality = self.build_eventuality(source.clone())?;
        if let Some(tense_modal) = jai_unit.tense_modal.as_deref()
            && generated_tense_modal_has_event_modifier(tense_modal)
        {
            self.apply_generated_tense_modal_event_modifier_to_eventuality(
                eventuality,
                tense_modal,
                event_modifier_anchor,
            )?;
        }
        if let Some(mut modal_argument) = jai_modal_argument {
            self.bind_generated_modal_argument_to_host_event(&mut modal_argument, eventuality);
            modal_arguments.push(modal_argument);
        }
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, PredicationMode::Restrictive),
            source,
            diagnostics,
        );
        object.set_predication_attachments(modal_arguments, Vec::new());
        object.set_predication_relation_metadata(relation_metadata);
        self.insert(predication, object)?;
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
    pub(super) fn build_restrictive_formula_for_tagged_selbri(
        &mut self,
        tagged: &'tree jbotci_syntax::generated_model::TaggedSelbriSyntax,
        referent: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref()) {
            return Err(unsupported("scoped tagged restrictive selbri"));
        }
        let UntaggedSelbriSyntax::CoSelbri(co_selbri) = tagged.inner_selbri.as_ref() else {
            return Err(unsupported("non-CO tagged restrictive selbri"));
        };
        let predication_source = self.source_for_node(tagged, "restrictive-predication");
        let formula_source = self.source_for_node(tagged, "restrictive-formula");
        if let Some(tanru) = tanru_selbri_from_co_selbri(co_selbri)?
            && tanru.additional_units.is_empty()
        {
            return self.build_tagged_relation_formula_for_generated_tanru_unit_argument(
                &tanru.first_unit,
                ArgumentValue::filled(referent, None),
                tagged.tense_modal.as_ref(),
                PredicationMode::Restrictive,
                predication_source,
                formula_source,
            );
        }
        let relation = semantic_relation_label(relation_label_from_co_selbri(co_selbri)?);
        let place_count = relation_place_count(self.dictionary, &relation).unwrap_or(1);
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(referent, None));
        for place in 2..=place_count {
            let key = argument_key(place);
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                key,
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let eventuality = self.build_generated_tense_eventuality(
            tagged.tense_modal.as_ref(),
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.display_text(),
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
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_quantifier(
        &mut self,
        quantifier: &'tree QuantifierSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match quantifier {
            QuantifierSyntax::PaRunQuantifier(quantifier) => {
                let words = self.tokens_for_node(&quantifier.number);
                if words.is_empty() {
                    return Err(unsupported("empty quantifier"));
                }
                let text = token_list_text(words.iter());
                self.build_quantity_for_generated_tokens(
                    &words,
                    &text,
                    self.source_for_node(quantifier, "quantity"),
                )
            }
            QuantifierSyntax::MeksoQuantifier(quantifier) => self
                .build_quantity_for_generated_mekso(
                    quantifier.mekso.as_ref(),
                    self.source_for_node(quantifier, "quantity"),
                ),
            QuantifierSyntax::ZantufaRawMeksoQuantifier(quantifier) => self
                .build_quantity_for_generated_mekso(
                    quantifier.0.as_ref(),
                    self.source_for_node(quantifier, "quantity"),
                ),
            QuantifierSyntax::ZantufaPriorityRawMeksoQuantifier(quantifier) => self
                .build_quantity_for_generated_mekso(
                    quantifier.0.as_ref(),
                    self.source_for_node(quantifier, "quantity"),
                ),
        }
    }

    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|connection| connection.as_ref().is_none_or(|connection| connection.left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity && connection.right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)) || ret.is_err())]
    pub(super) fn connected_quantifier_quantity_scope_for_generated_quantifier(
        &mut self,
        quantifier: &'tree QuantifierSyntax,
        locus: &str,
    ) -> Result<Option<GeneratedConnectedQuantifierQuantityScope>, SemanticsError> {
        let QuantifierSyntax::MeksoQuantifier(quantifier) = quantifier else {
            return Ok(None);
        };
        self.connected_quantifier_quantity_scope_for_generated_mekso(&quantifier.mekso, locus)
    }

    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|connection| connection.as_ref().is_none_or(|connection| connection.left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity && connection.right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)) || ret.is_err())]
    pub(super) fn connected_quantifier_quantity_scope_for_generated_mekso(
        &mut self,
        expression: &'tree MeksoSyntax,
        locus: &str,
    ) -> Result<Option<GeneratedConnectedQuantifierQuantityScope>, SemanticsError> {
        if let Some(parenthesized) = generated_parenthesized_mekso_operand_from_mekso(expression) {
            return self.connected_quantifier_quantity_scope_for_generated_mekso(
                &parenthesized.inner_expression,
                locus,
            );
        }
        if let Some(operand) = generated_forethought_mekso_operand_from_mekso(expression)
            && generated_modal_forethought_connective_is_logical(&operand.gek)
            && !generated_modal_forethought_connective_is_interval(&operand.gek)
        {
            return self
                .build_generated_forethought_connected_quantifier_quantity_scope(
                    operand,
                    locus,
                    self.source_for_node(expression, "mekso-operand-connection"),
                )
                .map(Some);
        }
        let Some(MeksoOperandSyntax::AfterthoughtMeksoOperand(operand)) =
            generated_single_mekso_operand_from_mekso(expression)
        else {
            return Ok(None);
        };
        let chain = &operand.0;
        let [link] = chain.links.as_slice() else {
            return Ok(None);
        };
        if !generated_operand_connective_is_logical(&link.operand_connective)
            || generated_operand_connective_is_interval(&link.operand_connective)
        {
            return Ok(None);
        }
        self.build_generated_afterthought_connected_quantifier_quantity_scope(
            &chain.first,
            &link.operand_connective,
            &link.trailing_expression,
            locus,
            self.source_for_node(operand, "mekso-operand-connection"),
        )
        .map(Some)
    }

    #[requires(generated_modal_forethought_connective_is_logical(&operand.gek))]
    #[requires(!generated_modal_forethought_connective_is_interval(&operand.gek))]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|connection| connection.left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity && connection.right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_generated_forethought_connected_quantifier_quantity_scope(
        &mut self,
        operand: &'tree ForethoughtMeksoOperandSyntax,
        locus: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedConnectedQuantifierQuantityScope, SemanticsError> {
        let left_quantity = self.build_quantity_for_generated_mekso_operand(
            &operand.left_expression,
            self.source_for_node(&operand.left_expression, "quantity"),
        )?;
        let right_quantity = self.build_quantity_for_generated_mekso_operand(
            &operand.right_expression,
            self.source_for_node(&operand.right_expression, "quantity"),
        )?;
        Ok(new!(GeneratedConnectedQuantifierQuantityScope {
            left_quantity,
            right_quantity,
            left_negated: generated_modal_forethought_connective_negates_left(&operand.gek),
            right_negated: generated_gik_connective_negates_right(&operand.gik),
            operator: generated_modal_forethought_connective_formula_operator(&operand.gek),
            connector: new!(Connector {
                source: generated_modal_forethought_connective_source(&operand.gek),
                locus: locus.to_owned(),
                truth_table: generated_modal_forethought_gik_connective_truth_table(
                    &operand.gek,
                    &operand.gik,
                ),
                parameter: None,
            }),
            source,
        }))
    }

    #[requires(generated_operand_connective_is_logical(connective))]
    #[requires(!generated_operand_connective_is_interval(connective))]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|connection| connection.left_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity && connection.right_quantity.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_generated_afterthought_connected_quantifier_quantity_scope(
        &mut self,
        left_operand: &'tree BoundOrSimpleMeksoOperandSyntax,
        connective: &jbotci_syntax::generated_model::OperandConnectiveSyntax,
        right_operand: &'tree BoundOrSimpleMeksoOperandSyntax,
        locus: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<GeneratedConnectedQuantifierQuantityScope, SemanticsError> {
        let left_quantity = self.build_quantity_for_generated_bound_or_simple_mekso_operand(
            left_operand,
            self.source_for_node(left_operand, "quantity"),
        )?;
        let right_quantity = self.build_quantity_for_generated_bound_or_simple_mekso_operand(
            right_operand,
            self.source_for_node(right_operand, "quantity"),
        )?;
        Ok(new!(GeneratedConnectedQuantifierQuantityScope {
            left_quantity,
            right_quantity,
            left_negated: generated_operand_connective_negates_left(connective),
            right_negated: generated_operand_connective_negates_right(connective),
            operator: generated_operand_connective_formula_operator(connective),
            connector: new!(Connector {
                source: generated_operand_connective_source(connective),
                locus: locus.to_owned(),
                truth_table: generated_operand_connective_truth_table(connective),
                parameter: None,
            }),
            source,
        }))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_bound_or_simple_mekso_operand(
        &mut self,
        operand: &'tree BoundOrSimpleMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_bound_or_simple_mekso_operand_surface_text(operand)?;
        let value = generated_simple_pa_quantity_value_for_bound_or_simple_mekso_operand(operand)
            .map_or_else(
            || {
                self.build_generated_bound_or_simple_mekso_operand(
                    operand,
                    source.clone().map(|source| crate::model::SemanticSource {
                        construct: Some("math-expression".to_owned()),
                        ..source
                    }),
                )
                .map(QuantityValue::math_expression)
            },
            Ok,
        )?;
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                source,
            ),
        )?;
        Ok(quantity)
    }

    #[requires(!construct.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_node<N: TreeNode>(
        &mut self,
        node: &N,
        construct: &str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let words = self.tokens_for_node(node);
        if words.is_empty() {
            return Err(unsupported("empty quantifier"));
        }
        let text = token_list_text(words.iter());
        self.build_quantity_for_generated_tokens(
            &words,
            &text,
            self.source_for_node(node, construct),
        )
    }

    #[requires(!words.is_empty())]
    #[requires(!text.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_tokens(
        &mut self,
        words: &[Token],
        text: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let value = parse_generated_relational_pa_integer(text)
            .or_else(|| simple_pa_integer_from_tokens(&words))
            .map(QuantityValue::integer)
            .unwrap_or_else(|| QuantityValue::text(text.to_owned()));
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(
                quantity_form_for_text(&text),
                value,
                QuantityScale::Count,
                source,
            ),
        )?;
        Ok(quantity)
    }

    #[requires(!text.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_recurrence_quantity_for_generated_integer(
        &mut self,
        text: &str,
        value: i64,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_recurrence_quantity_for_generated_value_with_form(
            quantity_form_for_text(text),
            QuantityValue::integer(value),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_recurrence_quantity_for_generated_value(
        &mut self,
        value: QuantityValue,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_recurrence_quantity_for_generated_value_with_form(
            quantity_form_for_value(&value),
            value,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_recurrence_quantity_for_generated_value_with_form(
        &mut self,
        form: QuantityForm,
        value: QuantityValue,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let quantity = self.next_quantity_id();
        self.insert(
            quantity,
            SemanticObject::quantity(form, value, QuantityScale::Frequency, None),
        )?;
        Ok(quantity)
    }

    #[requires(!relation.is_empty())]
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_structural_formula_from_arguments(
        &mut self,
        relation: &str,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
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
    #[requires(arguments.keys().all(|place| place.get() > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_structural_formula_from_arguments_with_formula_source(
        &mut self,
        relation: &str,
        mut arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
    pub(super) fn build_abstraction_link_formula_for_visible_argument<'syntax: 'tree>(
        &mut self,
        abstraction: &'syntax AbstractionTanruUnitSyntax,
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
        arguments.insert(argument_key(1), x1);
        arguments.insert(argument_key(2), ArgumentValue::filled(output, None));
        self.build_structural_formula_from_arguments(
            abstraction_link_relation(kind),
            arguments,
            mode,
            source,
        )
    }

    #[requires(source.as_ref().is_none_or(|source| source.construct.is_some()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_abstraction_output<'syntax: 'tree>(
        &mut self,
        abstraction: &'syntax AbstractionTanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if abstraction.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        if !abstraction.abstractor_connections.is_empty() {
            return Err(unsupported("connected abstraction"));
        }
        self.build_abstraction_output_for_branch(
            GeneratedAbstractionBranch::primary(abstraction),
            source,
        )
    }

    #[requires(source.as_ref().is_none_or(|source| source.construct.is_some()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_abstraction_output_for_branch(
        &mut self,
        branch: GeneratedAbstractionBranch<'tree>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if branch.nai.is_some() {
            return Err(unsupported("negated abstraction"));
        }
        let kind = abstraction_kind_for_cmavo(branch.nu.value.cmavo());
        let sort = abstraction_output_sort(kind);
        let first_body_object_index = self.next_index;
        self.abstraction_parameter_stack.push(Vec::new());
        self.indirect_question_stack.push(Vec::new());
        let body = match self
            .build_generated_subbridi_formula(branch.subbridi, abstraction_body_mode(kind))
        {
            Ok(result) => result,
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
        let mut parameters = self
            .abstraction_parameter_stack
            .pop()
            .expect("abstraction parameter stack was just pushed");
        if kind == AbstractionKind::Property && parameters.is_empty() {
            self.insert_implicit_generated_property_slot_parameter(
                body,
                &mut parameters,
                self.source_for_abstraction_branch_tokens(branch, "implicit-property-slot"),
                main_generated_selbri_for_subbridi(branch.subbridi),
            )?;
        }
        self.set_formula_predication_mode(body, abstraction_body_mode(kind));
        let embedded_questions =
            self.build_generated_embedded_indirect_questions(body, indirect_questions)?;

        if let Some(class) = abstraction_eventuality_class(kind) {
            let body_eventuality = self.single_generated_formula_eventuality(body);
            let owned_body_eventuality = body_eventuality.filter(|eventuality| {
                eventuality.index() >= first_body_object_index
                    && self
                        .objects
                        .get(eventuality)
                        .is_some_and(SemanticObject::is_generated_eventuality)
            });
            let mut object = owned_body_eventuality
                .and_then(|eventuality| self.objects.remove(&eventuality))
                .unwrap_or_else(|| {
                    SemanticObject::referential_eventuality(class, None, source.clone())
                });
            let id = match owned_body_eventuality {
                Some(eventuality) if eventuality.referent_sort() == Some(sort) => eventuality,
                Some(eventuality) => {
                    let specialized = self.next_referent_with_sort_id(sort);
                    self.replace_generated_formula_eventuality(body, eventuality, specialized);
                    specialized
                }
                None => {
                    let specialized = self.next_referent_with_sort_id(sort);
                    if let Some(inherited) = body_eventuality {
                        self.replace_generated_formula_eventuality(body, inherited, specialized);
                    }
                    specialized
                }
            };
            object.configure_eventuality_abstraction(
                class,
                sort,
                body,
                kind,
                parameters,
                embedded_questions,
                source,
            );
            self.insert(id, object)?;
            return Ok(id);
        }

        let id = self.next_referent_with_sort_id(sort);
        let mut object = SemanticObject::abstraction(kind, body, parameters, source, Vec::new());
        object.set_abstraction_embedded_questions(embedded_questions);
        self.insert(id, object)?;
        Ok(id)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    pub(super) fn single_generated_formula_eventuality(
        &self,
        formula: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let predication = self.objects.get(&formula)?.formula_predication()?;
        self.objects
            .get(&predication)
            .and_then(SemanticObject::predication_eventuality)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn replace_generated_formula_eventuality(
        &mut self,
        formula: SemanticObjectId,
        old_eventuality: SemanticObjectId,
        new_eventuality: SemanticObjectId,
    ) {
        if old_eventuality == new_eventuality {
            return;
        }
        let Some(traversal) = self
            .objects
            .get(&formula)
            .and_then(SemanticObject::formula_traversal)
            .map(FormulaTraversal::into_data)
        else {
            return;
        };
        if let Some(predication) = traversal.predication {
            self.replace_generated_predication_eventuality(
                predication,
                old_eventuality,
                new_eventuality,
            );
        }
        let formula_uses_old_eventuality = self
            .objects
            .get(&formula)
            .and_then(SemanticObject::as_formula)
            .is_some_and(|node| {
                matches!(
                    node.as_data(),
                    data!(FormulaNode::Connective(node))
                        if node.eventuality == Some(old_eventuality)
                )
            });
        if formula_uses_old_eventuality && let Some(object) = self.objects.get_mut(&formula) {
            object.set_scoped_formula_eventuality(Some(new_eventuality));
        }
        for child in traversal.children {
            self.replace_generated_formula_eventuality(child, old_eventuality, new_eventuality);
        }
        if let Some(restriction) = traversal.restriction {
            self.replace_generated_formula_eventuality(
                restriction,
                old_eventuality,
                new_eventuality,
            );
        }
        if let Some(body) = traversal.body {
            self.replace_generated_formula_eventuality(body, old_eventuality, new_eventuality);
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn replace_generated_predication_eventuality(
        &mut self,
        predication: SemanticObjectId,
        old_eventuality: SemanticObjectId,
        new_eventuality: SemanticObjectId,
    ) {
        if old_eventuality == new_eventuality {
            return;
        }
        let Some(node) = self
            .objects
            .get(&predication)
            .and_then(SemanticObject::as_predication)
        else {
            return;
        };
        let tanru_head = node.tanru_link.as_ref().map(|link| link.head);
        let modal_bodies = node
            .modal_arguments
            .iter()
            .filter_map(|argument| argument.body)
            .collect::<Vec<_>>();
        let Some(object) = self.objects.get_mut(&predication) else {
            return;
        };
        object.update_predication(|node| {
            replace_generated_predication_eventuality_references(
                node,
                old_eventuality,
                new_eventuality,
            )
        });
        if let Some(head) = tanru_head {
            self.replace_generated_predication_eventuality(head, old_eventuality, new_eventuality);
        }
        for body in modal_bodies {
            self.replace_generated_formula_eventuality(body, old_eventuality, new_eventuality);
        }
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn insert_implicit_generated_property_slot_parameter(
        &mut self,
        body: SemanticObjectId,
        parameters: &mut Vec<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        preferred_selbri: Option<&'tree SelbriSyntax>,
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
    pub(super) fn build_elided_argument_with_sort(
        &mut self,
        label: String,
        sort: SemanticSort,
    ) -> Result<ArgumentValue, SemanticsError> {
        let referent = self.build_elided_referent_with_sort(label.clone(), sort)?;
        Ok(ArgumentValue::elided(referent, label, None))
    }

    #[requires(place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.kind == ArgumentValueKind::Elided) || ret.is_err())]
    pub(super) fn build_elided_argument_for_place(
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
    pub(super) fn build_elided_referent(
        &mut self,
        label: String,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, SemanticSort::Entity, None)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_elided_referent_with_source(
        &mut self,
        label: String,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, SemanticSort::Entity, source)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_elided_referent_with_sort(
        &mut self,
        label: String,
        sort: SemanticSort,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_elided_referent_with_sort_and_source(label, sort, None)
    }

    #[requires(!label.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_elided_referent_with_sort_and_source(
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
                Some(new!(Descriptor {
                    kind: DescriptorKind::Elided,
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
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(!word.is_empty())]
    #[requires(crate::model::argument_object_kind_can_fill(operand.object_kind()))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_abstraction_about_referent(
        &mut self,
        word: &str,
        operand: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_with_sort_id(SemanticSort::eventuality());
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::eventuality(),
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::AbstractionAbout,
                    word: word.to_owned(),
                    speaker: Some(self.current_speaker()),
                    body: None,
                    veridical: None,
                    relative_clauses: Vec::new(),
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: Some(operand),
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }
}

#[requires(old_eventuality != new_eventuality)]
#[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(ret.value != Some(old_eventuality))]
fn replace_generated_argument_eventuality_reference(
    argument: ArgumentValue,
    old_eventuality: SemanticObjectId,
    new_eventuality: SemanticObjectId,
) -> ArgumentValue {
    if argument.value != Some(old_eventuality) {
        return argument;
    }
    argument.with_data(data! { value: Some(new_eventuality) })
}

#[requires(old_eventuality != new_eventuality)]
#[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(ret.scale != Some(old_eventuality))]
fn replace_generated_scalar_negation_eventuality_reference(
    scalar_negation: ScalarNegation,
    old_eventuality: SemanticObjectId,
    new_eventuality: SemanticObjectId,
) -> ScalarNegation {
    if scalar_negation.scale != Some(old_eventuality) {
        return scalar_negation;
    }
    scalar_negation.with_data(data! { scale: Some(new_eventuality) })
}

#[requires(old_eventuality != new_eventuality)]
#[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(ret.arguments.values().all(|argument| argument.value != Some(old_eventuality)))]
#[ensures(ret.component != Some(old_eventuality))]
fn replace_generated_modal_eventuality_references(
    modal: ModalArgument,
    old_eventuality: SemanticObjectId,
    new_eventuality: SemanticObjectId,
) -> ModalArgument {
    let data = modal.into_data();
    let arguments = data
        .arguments
        .into_iter()
        .map(|(place, argument)| {
            (
                place,
                replace_generated_argument_eventuality_reference(
                    argument,
                    old_eventuality,
                    new_eventuality,
                ),
            )
        })
        .collect();
    let component = data.component.map(|component| {
        if component == old_eventuality {
            new_eventuality
        } else {
            component
        }
    });
    let scalar_negation = data.scalar_negation.map(|scalar_negation| {
        replace_generated_scalar_negation_eventuality_reference(
            scalar_negation,
            old_eventuality,
            new_eventuality,
        )
    });
    ModalArgument::from_data(data!(ModalArgument {
        arguments,
        component,
        scalar_negation,
        ..data
    }))
}

#[requires(old_eventuality != new_eventuality)]
#[requires(old_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[requires(new_eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
#[ensures(ret.eventuality != Some(old_eventuality))]
#[ensures(ret.arguments.values().all(|argument| argument.value != Some(old_eventuality)))]
#[ensures(ret.place_questions.iter().all(|binding| binding.argument.value != Some(old_eventuality)))]
#[ensures(ret.modal_arguments.iter().all(|modal| modal.arguments.values().all(|argument| argument.value != Some(old_eventuality)) && modal.component != Some(old_eventuality)))]
#[ensures(ret.reciprocity.iter().all(|exchange| exchange.left.value != Some(old_eventuality) && exchange.right.value != Some(old_eventuality)))]
fn replace_generated_predication_eventuality_references(
    predication: PredicationNode,
    old_eventuality: SemanticObjectId,
    new_eventuality: SemanticObjectId,
) -> PredicationNode {
    let data = predication.into_data();
    let eventuality = data.eventuality.map(|eventuality| {
        if eventuality == old_eventuality {
            new_eventuality
        } else {
            eventuality
        }
    });
    let tanru_link = data.tanru_link.map(|link| {
        let link_data = link.into_data();
        let modifier = if link_data.modifier == old_eventuality {
            new_eventuality
        } else {
            link_data.modifier
        };
        TanruLink::from_data(data!(TanruLink {
            modifier,
            ..link_data
        }))
    });
    let arguments = data
        .arguments
        .into_iter()
        .map(|(place, argument)| {
            (
                place,
                replace_generated_argument_eventuality_reference(
                    argument,
                    old_eventuality,
                    new_eventuality,
                ),
            )
        })
        .collect();
    let place_questions = data
        .place_questions
        .into_iter()
        .map(|binding| {
            let binding_data = binding.into_data();
            PlaceQuestionBinding::from_data(data!(PlaceQuestionBinding {
                argument: replace_generated_argument_eventuality_reference(
                    binding_data.argument,
                    old_eventuality,
                    new_eventuality,
                ),
                ..binding_data
            }))
        })
        .collect();
    let modal_arguments = data
        .modal_arguments
        .into_iter()
        .map(|modal| {
            replace_generated_modal_eventuality_references(modal, old_eventuality, new_eventuality)
        })
        .collect();
    let reciprocity = data
        .reciprocity
        .into_iter()
        .map(|exchange| {
            let exchange_data = exchange.into_data();
            ReciprocalExchange::from_data(data!(ReciprocalExchange {
                left: replace_generated_argument_eventuality_reference(
                    exchange_data.left,
                    old_eventuality,
                    new_eventuality,
                ),
                right: replace_generated_argument_eventuality_reference(
                    exchange_data.right,
                    old_eventuality,
                    new_eventuality,
                ),
                ..exchange_data
            }))
        })
        .collect();
    let scalar_negation = data.scalar_negation.map(|scalar_negation| {
        replace_generated_scalar_negation_eventuality_reference(
            scalar_negation,
            old_eventuality,
            new_eventuality,
        )
    });
    PredicationNode::from_data(data!(PredicationNode {
        eventuality,
        tanru_link,
        arguments,
        place_questions,
        modal_arguments,
        reciprocity,
        scalar_negation,
        ..data
    }))
}
