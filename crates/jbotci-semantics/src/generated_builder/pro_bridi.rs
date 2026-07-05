use super::*;

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(true)]
    pub(super) fn record_generated_assigned_pro_bridi_bindings_for_tanru_unit(
        &mut self,
        unit: &'tree TanruUnitSyntax,
        visible_arguments: &BTreeMap<usize, ArgumentValue>,
    ) -> Result<(), SemanticsError> {
        if !unit.0.links.is_empty() {
            return Ok(());
        }
        let BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(assigned) = unit.0.first.as_ref()
        else {
            return Ok(());
        };
        let base = linked_tanru_unit_from_cei(assigned.base.as_ref());
        let relation = semantic_relation_label(relation_label_from_linked_tanru_unit(&base)?);
        for assignment in &assigned.assignments {
            let Some(label) =
                assigned_pro_bridi_reference_label_for_linked_tanru_unit(&assignment.tanru_unit)
            else {
                continue;
            };
            self.assigned_pro_bridi_bindings.insert(
                label,
                GeneratedAssignedProBridiBinding::from_data(data!(
                    GeneratedAssignedProBridiBinding {
                        relation: Some(relation.clone()),
                        tanru: None,
                        source: None,
                        visible_arguments: visible_arguments.clone(),
                    }
                )),
            );
        }
        Ok(())
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(true)]
    pub(super) fn record_generated_assigned_pro_bridi_bindings_for_tanru_selbri(
        &mut self,
        tanru: &'tree TanruSelbriSyntax,
        visible_arguments: &BTreeMap<usize, ArgumentValue>,
    ) -> Result<(), SemanticsError> {
        let source = self.source_for_node(tanru, "restrictive-tanru-formula");
        for unit in std::iter::once(&tanru.first_unit).chain(tanru.additional_units.iter()) {
            if !unit.0.links.is_empty() {
                continue;
            }
            let BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(assigned) =
                unit.0.first.as_ref()
            else {
                continue;
            };
            for assignment in &assigned.assignments {
                let Some(label) = assigned_pro_bridi_reference_label_for_linked_tanru_unit(
                    &assignment.tanru_unit,
                ) else {
                    continue;
                };
                self.assigned_pro_bridi_bindings.insert(
                    label,
                    GeneratedAssignedProBridiBinding::from_data(data!(
                        GeneratedAssignedProBridiBinding {
                            relation: None,
                            tanru: Some(tanru),
                            source: source.clone(),
                            visible_arguments: visible_arguments.clone(),
                        }
                    )),
                );
            }
        }
        Ok(())
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[requires(current_visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_assigned_pro_bridi_reference_formula(
        &mut self,
        binding: &GeneratedAssignedProBridiBinding<'tree>,
        current_visible_arguments: BTreeMap<usize, ArgumentValue>,
        place_question_assignments: &[GeneratedPlaceQuestionAssignment],
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
        eventuality: SemanticObjectId,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut visible_arguments = binding.visible_arguments.clone();
        for place in current_visible_arguments.keys() {
            visible_arguments.remove(place);
        }
        for (place, argument) in current_visible_arguments {
            insert_visible_argument(&mut visible_arguments, place, argument)?;
        }
        if let Some(tanru) = binding.tanru {
            let result = if tanru.additional_units.is_empty() {
                self.build_tanru_unit_formula_for_visible_arguments(
                    &tanru.first_unit,
                    visible_arguments,
                    formula_source,
                    "selbri",
                    Some(eventuality),
                )?
            } else {
                self.build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
                    tanru,
                    visible_arguments,
                    Some(eventuality),
                    formula_source,
                    modal_terms,
                )?
            };
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            return Ok(result.formula);
        }
        let relation = binding.relation.clone().ok_or_else(|| {
            invalid_graph("assigned pro-bridi binding has no relation target".to_owned())
        })?;
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated pro-bridi arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        if place_count.is_none() && !relation_has_open_place_structure(&relation) {
            diagnostics.push(diagnostic(
                "relation place structure is unavailable; only places required by explicit assignments are represented",
            ));
        }
        let place_questions = self.build_generated_place_question_bindings(
            place_question_assignments,
            &arguments,
            place_count,
            highest_argument,
        )?;
        for place in 1..=place_count.unwrap_or(0).max(highest_argument).max(1) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let modal_arguments = self
            .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                eventuality,
                modal_terms,
                Some(&arguments),
            )?;
        let mut predication_object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.place_questions = place_questions;
        let predication = self.next_predication_id();
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_assigned_pro_bridi_binding(
        &mut self,
        binding: &GeneratedAssignedProBridiBinding<'tree>,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(tanru) = binding.tanru {
            let source = binding.source.clone().or_else(|| source.clone());
            if tanru.additional_units.is_empty() {
                return self.build_description_property_formula_for_tanru_unit(
                    &tanru.first_unit,
                    parameter,
                    source,
                );
            }
            return self.build_property_formula_for_tanru_run(
                &tanru.first_unit,
                &tanru.additional_units,
                parameter,
                source,
                GeneratedPropertyTanruContext::Description,
            );
        }
        let relation = binding.relation.clone().ok_or_else(|| {
            invalid_graph("assigned pro-bridi binding has no relation target".to_owned())
        })?;
        self.build_relation_formula_for_argument(
            relation,
            ArgumentValue::filled(parameter, None),
            None,
            PredicationMode::Restrictive,
            source.clone(),
            source,
        )
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_assigned_pro_bridi_tanru_unit_formula_for_visible_arguments<
        'syntax: 'tree,
    >(
        &mut self,
        unit: &'syntax AssignedProBridiTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_head_relation_formula_from_parts(
            GeneratedTanruAtomView::cei(unit.base.base.as_ref()),
            unit.base.linkargs.as_ref(),
            visible_arguments,
            eventuality,
            source,
            &[],
        )
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_assigned_pro_bridi_tanru_unit_head_relation_formula_with_modal_terms<
        'syntax: 'tree,
    >(
        &mut self,
        unit: &'syntax AssignedProBridiTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
        modal_terms: &[&'tree TaggedSumtiTermSyntax],
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        self.build_tanru_head_relation_formula_from_parts(
            GeneratedTanruAtomView::cei(unit.base.base.as_ref()),
            unit.base.linkargs.as_ref(),
            visible_arguments,
            eventuality,
            source,
            modal_terms,
        )
    }

    #[requires(x1.object_kind() == crate::model::SemanticObjectKind::Referent || x1.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_restrictive_formula_for_generated_pro_bridi_frame(
        &mut self,
        cmavo: Cmavo,
        x1: SemanticObjectId,
        fallback_source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(target) = self.generated_pro_bridi_target_frame(cmavo, fallback_source.clone())?
        else {
            return Ok(None);
        };
        let eventuality = target
            .event_tense
            .as_ref()
            .map(|tense_modal| {
                self.build_generated_tense_eventuality(
                    tense_modal,
                    target.predication_source.clone(),
                )
            })
            .transpose()?
            .flatten();
        let mut arguments = BTreeMap::new();
        for (place, argument) in &target.arguments {
            if argument.kind != ArgumentValueKind::Elided {
                arguments.insert(place.clone(), argument.clone());
            }
        }
        arguments.insert(argument_key(1), ArgumentValue::filled(x1, None));
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_limit = target.place_count.unwrap_or(highest_argument.max(1));
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
        let predication_source = target
            .predication_source
            .clone()
            .or_else(|| fallback_source.clone());
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                target.relation.display_text(),
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
                target.formula_source.clone().or(fallback_source),
                Vec::new(),
            ),
        )?;
        Ok(Some(formula))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_completed_generated_pro_bridi_frame_from_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        if let Some(frame) =
            self.generated_completed_pro_bridi_frame_from_formula(bridi, formula, source)?
        {
            self.completed_pro_bridi_frames.push(frame);
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|frame| frame.as_ref().is_none_or(|frame| frame.relation.is_displayable())) || ret.is_err())]
    pub(super) fn generated_completed_pro_bridi_frame_from_formula(
        &self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
        _source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<GeneratedProBridiFrame<'tree>>, SemanticsError> {
        let Some(selbri) = main_generated_selbri_for_bridi(bridi) else {
            return Ok(None);
        };
        let Some(relation) = generated_pro_bridi_target_relation_label(selbri)? else {
            return Ok(None);
        };
        let relation = semantic_relation_label(relation);
        let predication = self.primary_predication_for_formula(formula)?;
        let object = self.objects.get(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find generated completed predication {predication}"
            ))
        })?;
        let replay = generated_pro_bridi_replay_source_from_bridi(bridi)?;
        let place_count = relation_place_count(self.dictionary, &relation);
        Ok(Some(new!(GeneratedProBridiFrame {
            relation,
            arguments: object.arguments.clone(),
            place_count,
            event_tense: generated_pro_bridi_event_tense_from_selbri(selbri),
            quote_depth: self.current_quote_depth,
            replay,
            predication_source: self.source_for_node(selbri, "restrictive-predication"),
            formula_source: self.source_for_node(selbri, "restrictive-formula"),
            diagnostics: Vec::new(),
        })))
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_resolved_generated_pro_bridi_formula_for_terms(
        &mut self,
        cmavo: Cmavo,
        terms: Vec<&'tree TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        scalar_negation: Option<ScalarNegation>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(target) =
            self.generated_pro_bridi_target_frame(cmavo, predication_source.clone())?
        else {
            return Ok(None);
        };
        let excluded_source = predication_source.as_ref().map(|source| &source.span);
        let target_arguments =
            self.generated_pro_bridi_replayed_arguments(&target, excluded_source)?;
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_pro_bridi_eventuality_from_frame(
                &target,
                predication_source.clone(),
            )?,
        };
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
        let assignments = self.with_temporal_context(eventuality, |builder| {
            builder.build_term_assignments_for_terms(terms, first_visible_place)
        })?;
        let place_question_assignments = assignments.place_questions.clone();
        let mut arguments = BTreeMap::new();
        for (place, argument) in target_arguments {
            if argument.kind != ArgumentValueKind::Elided {
                arguments.insert(place, argument);
            }
        }
        for (visible_place, argument) in assignments.visible_arguments {
            arguments.insert(argument_key(visible_place), argument);
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_limit = target.place_count.unwrap_or(highest_argument.max(1));
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
            &arguments,
            target.place_count,
            highest_argument,
        )?;
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
        let modal_arguments = self.build_modal_arguments_for_generated_tagged_terms_for_event(
            eventuality,
            &assignments.modal_terms,
        )?;
        let mut diagnostics = target.diagnostics.clone();
        if target.place_count.is_none() && !relation_has_open_place_structure(&target.relation) {
            diagnostics.push(diagnostic(
                "relation place structure is unavailable; only places required by explicit assignments are represented",
            ));
        }
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            target.relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&target.relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.place_questions = place_questions;
        self.insert(predication, predication_object)?;
        if let Some(scalar_negation) = scalar_negation {
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        let formula = self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )?;
        Ok(Some(formula))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|frame| frame.as_ref().is_none_or(|frame| frame.relation.is_displayable())) || ret.is_err())]
    pub(super) fn generated_pro_bridi_target_frame(
        &mut self,
        cmavo: Cmavo,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<GeneratedProBridiFrame<'tree>>, SemanticsError> {
        match cmavo {
            Cmavo::Gohi => Ok(self
                .completed_pro_bridi_frames
                .iter()
                .rev()
                .find(|frame| frame.quote_depth == self.current_quote_depth)
                .cloned()),
            Cmavo::Gohe => Ok(self
                .completed_pro_bridi_frames
                .iter()
                .rev()
                .filter(|frame| frame.quote_depth == self.current_quote_depth)
                .nth(1)
                .cloned()),
            Cmavo::Nei => {
                let bridi = self.pro_bridi_scope_stack.first().copied();
                bridi
                    .map(|bridi| self.generated_pro_bridi_frame_from_bridi(bridi, source))
                    .transpose()
                    .map(Option::flatten)
            }
            Cmavo::Noha => {
                let bridi = self.pro_bridi_scope_stack.iter().rev().nth(1).copied();
                bridi
                    .map(|bridi| self.generated_pro_bridi_frame_from_bridi(bridi, source))
                    .transpose()
                    .map(Option::flatten)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    pub(super) fn build_generated_pro_bridi_eventuality_from_frame(
        &mut self,
        frame: &GeneratedProBridiFrame<'tree>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(tense_modal) = &frame.event_tense
            && let Some(eventuality) =
                self.build_generated_tense_eventuality(tense_modal, source.clone())?
        {
            return Ok(eventuality);
        }
        self.build_eventuality(source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|frame| frame.as_ref().is_none_or(|frame| frame.relation.is_displayable())) || ret.is_err())]
    pub(super) fn generated_pro_bridi_frame_from_bridi(
        &mut self,
        bridi: &'tree BridiSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<GeneratedProBridiFrame<'tree>>, SemanticsError> {
        match bridi {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
                let terms = bridi
                    .leading_terms
                    .iter()
                    .chain(simple_tail.terms.iter())
                    .collect::<Vec<_>>();
                self.generated_pro_bridi_frame_from_selbri_and_terms(
                    &simple_tail.selbri,
                    terms,
                    1,
                    source,
                )
            }
            BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
                let simple_tail = simple_tail_from_bridi_tail(bridi_tail)?;
                self.generated_pro_bridi_frame_from_selbri_and_terms(
                    &simple_tail.selbri,
                    Vec::new(),
                    2,
                    source,
                )
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(self.sumti_referent_cache_bypass_depth == old(self.sumti_referent_cache_bypass_depth))]
    pub(super) fn with_sumti_referent_cache_bypassed<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, SemanticsError>,
    ) -> Result<T, SemanticsError> {
        let previous_depth = self.sumti_referent_cache_bypass_depth;
        self.sumti_referent_cache_bypass_depth =
            previous_depth.checked_add(1).ok_or_else(|| {
                invalid_graph("generated sumti referent cache bypass depth overflowed".to_owned())
            })?;
        let result = body(self);
        self.sumti_referent_cache_bypass_depth = previous_depth;
        result
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|arguments| arguments.keys().all(|place| place.get() > 0)) || ret.is_err())]
    pub(super) fn generated_pro_bridi_replayed_arguments(
        &mut self,
        frame: &GeneratedProBridiFrame<'tree>,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<BTreeMap<PlaceIndex, ArgumentValue>, SemanticsError> {
        if self.current_quote_depth == 0 {
            return Ok(frame.arguments.clone());
        }
        let Some(replay) = &frame.replay else {
            return Ok(frame.arguments.clone());
        };
        let terms = replay.terms.clone();
        let (assignments, _) = self.with_sumti_referent_cache_bypassed(|builder| {
            builder.build_term_assignments_for_terms_excluding_source(
                terms,
                replay.first_visible_place,
                excluded_source,
            )
        })?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let place = generated_raw_place_visible_rank_for_selbri(&replay.selbri, visible_place)?;
            arguments.insert(argument_key(place), argument);
        }
        Ok(arguments)
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|frame| frame.as_ref().is_none_or(|frame| frame.relation.is_displayable())) || ret.is_err())]
    pub(super) fn generated_pro_bridi_frame_from_selbri_and_terms(
        &mut self,
        selbri: &'tree SelbriSyntax,
        terms: Vec<&'tree TermSyntax>,
        first_visible_place: usize,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<GeneratedProBridiFrame<'tree>>, SemanticsError> {
        let Some(relation) = generated_pro_bridi_target_relation_label(selbri)? else {
            return Ok(None);
        };
        let relation = semantic_relation_label(relation);
        let excluded_source = source.as_ref().map(|source| &source.span);
        let (assignments, skipped_recursive_argument) = self
            .build_term_assignments_for_terms_excluding_source(
                terms,
                first_visible_place,
                excluded_source,
            )?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let place = generated_raw_place_visible_rank_for_selbri(selbri, visible_place)?;
            arguments.insert(argument_key(place), argument);
        }
        let diagnostics = if skipped_recursive_argument {
            vec![diagnostic(
                "recursive inherited pro-bridi argument was elided to keep the semantic graph finite",
            )]
        } else {
            Vec::new()
        };
        let place_count = relation_place_count(self.dictionary, &relation);
        Ok(Some(new!(GeneratedProBridiFrame {
            relation,
            arguments,
            place_count,
            event_tense: generated_pro_bridi_event_tense_from_selbri(selbri),
            quote_depth: self.current_quote_depth,
            replay: None,
            predication_source: self.source_for_node(selbri, "restrictive-predication"),
            formula_source: self.source_for_node(selbri, "restrictive-formula"),
            diagnostics,
        })))
    }
}
