use super::*;

impl<'a, 'dict> GeneratedGraphBuilder<'a, 'dict> {
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_formula_for_generated_tanru_unit_terms(
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
            let connected_formula = generated_tanru_unit_is_connected_selbri_formula(unit);
            let connected_source = connected_formula.then(|| {
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    "connected-selbri-formula",
                )
            });
            let leading_eventuality =
                if !terms.is_empty() && generated_tanru_unit_preallocates_head_eventuality(unit) {
                    Some(
                        self.build_eventuality(
                            connected_source
                                .clone()
                                .flatten()
                                .or_else(|| predication_source.clone()),
                        )?,
                    )
                } else {
                    None
                };
            let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
            self.record_generated_assigned_pro_bridi_bindings_for_tanru_unit(
                unit,
                &assignments.visible_arguments,
            )?;
            let result = self.build_tanru_unit_formula_for_visible_arguments(
                unit,
                assignments.visible_arguments,
                connected_source
                    .clone()
                    .flatten()
                    .or_else(|| formula_source.clone()),
                "selbri",
                leading_eventuality,
            )?;
            self.set_semantic_object_source(
                result.head_predication,
                connected_source
                    .flatten()
                    .or_else(|| predication_source.clone()),
            )?;
            self.attach_generated_modal_terms_to_formula(result.formula, &assignments.modal_terms)?;
            return self.wrap_formula_with_generated_assignment_scopes(
                result.formula,
                assignments.formula_scopes,
                assignments.coequal_scope_groups,
                assignments.implicit_existentials,
                assignments.term_formula_scopes,
            );
        }
        let (atom, linkargs) = generated_linked_tanru_unit_parts(unit)?;
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        if let Some(scalar_unit) = scalar_unit
            && linkargs.is_none()
            && atom.conversions.is_empty()
            && let Some(cmavo) =
                resolvable_generated_pro_bridi_cmavo_from_scalar_negated_tanru_unit(scalar_unit)
        {
            let predication_source = source_with_construct(
                predication_source
                    .clone()
                    .or_else(|| formula_source.clone()),
                "predication",
            );
            let formula_source = source_with_construct(
                formula_source
                    .clone()
                    .or_else(|| predication_source.clone()),
                "bridi-formula",
            );
            if let Some(formula) = self.build_resolved_generated_pro_bridi_formula_for_terms(
                cmavo,
                terms.clone(),
                first_visible_place,
                eventuality,
                mode,
                Some(scalar_negation_for_marker(&scalar_unit.nahe)),
                predication_source,
                formula_source,
            )? {
                return Ok(formula);
            }
        }
        if let Some(scalar_unit) = scalar_unit
            && let Some((grouped, inner_conversions)) =
                scalar_negated_tanru_unit_inner_grouped(scalar_unit)
        {
            let relation =
                semantic_relation_label(relation_label_from_grouped_tanru_unit(grouped)?);
            let relation_text = relation.display_text();
            let mut conversions = atom.conversions.clone();
            conversions.extend(inner_conversions.iter().cloned());
            if let Some(formula) = self
                .build_scalar_generated_logical_sumti_connection_formula_for_terms(
                    &relation_text,
                    &terms,
                    first_visible_place,
                    relation_place_count(self.dictionary, &relation)
                        .unwrap_or_else(|| terms.len().max(1)),
                    &conversions,
                    mode,
                    eventuality,
                    predication_source.clone(),
                    formula_source.clone(),
                    scalar_negation_for_generated_scalar_tanru_unit_atom(
                        atom,
                        scalar_unit,
                        None,
                        GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs,
                    )?,
                )?
            {
                return Ok(formula);
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
            let result = self
                .build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
                    &grouped.selbri,
                    visible_arguments,
                    formula_source,
                    eventuality,
                )?;
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            let formula = result.formula;
            self.attach_generated_modal_terms_to_formula(formula, &assignments.modal_terms)?;
            self.apply_scalar_negation_to_tanru_links(
                formula,
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    atom,
                    scalar_unit,
                    None,
                    GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs,
                )?,
            )?;
            let formula = self
                .detach_tanru_relation_formula_without_positive_head(formula)
                .unwrap_or(formula);
            return self.wrap_formula_with_generated_assignment_scopes(
                formula,
                assignments.formula_scopes,
                assignments.coequal_scope_groups,
                assignments.implicit_existentials,
                assignments.term_formula_scopes,
            );
        }
        if let TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) = atom.base.as_ref() {
            let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
            let visible_arguments = map_visible_arguments_for_generated_conversions(
                assignments.visible_arguments,
                &atom.conversions,
            )?;
            let result = self
                .build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
                    &grouped.selbri,
                    visible_arguments,
                    formula_source,
                    eventuality,
                )?;
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            let formula = result.formula;
            self.attach_generated_modal_terms_to_formula(formula, &assignments.modal_terms)?;
            return self.wrap_formula_with_generated_assignment_scopes(
                formula,
                assignments.formula_scopes,
                assignments.coequal_scope_groups,
                assignments.implicit_existentials,
                assignments.term_formula_scopes,
            );
        }
        let relation = semantic_relation_label(relation_label_from_tanru_unit_atom_base(
            atom.base.as_ref(),
        )?);
        let relation_text = relation.display_text();
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let jai_unit = generated_jai_modal_tanru_unit(atom.base.as_ref());
        let (terms, fai_sumti) = if jai_unit.is_some() {
            self.split_generated_fai_terms(terms)?
        } else {
            (terms, Vec::new())
        };
        if linkargs.is_none()
            && let Some(formula) = self.build_generated_logical_sumti_connection_formula_for_terms(
                &relation_text,
                &terms,
                first_visible_place,
                place_count.unwrap_or_else(|| terms.len().max(1)),
                &atom.conversions,
                mode,
                predication_source.clone(),
                formula_source.clone(),
            )?
        {
            return Ok(formula);
        }
        if linkargs.is_none()
            && let Some(formula) = self.build_generated_logical_modal_connection_formula_for_terms(
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    "modal-branch-formula",
                ),
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    "modal-connection-formula",
                ),
                &relation_text,
                place_count,
                place_count.unwrap_or_else(|| terms.len().max(1)),
                &[],
                false,
                &terms,
                None,
                first_visible_place,
                &atom.conversions,
                mode,
                predication_source.clone(),
            )?
        {
            return Ok(formula);
        }
        let prebuild_linkarg_assignments = match linkargs {
            Some(linkargs) => !generated_linkargs_visible_places(linkargs, 2)?.is_empty(),
            None => false,
        };
        let (eventuality, prebuilt_linkarg_assignments, assignments) = match eventuality {
            Some(eventuality) => {
                let prebuilt_linkarg_assignments = if prebuild_linkarg_assignments {
                    Some(self.build_linkargs_assignments(
                        linkargs.expect("prebuild flag requires linkargs"),
                        2,
                    )?)
                } else {
                    None
                };
                self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
                let assignments = self.with_temporal_context(eventuality, |builder| {
                    builder.build_term_assignments_for_terms(terms.clone(), first_visible_place)
                })?;
                (eventuality, prebuilt_linkarg_assignments, assignments)
            }
            None if scalar_unit.is_some() => {
                let prebuilt_linkarg_assignments = if prebuild_linkarg_assignments {
                    Some(self.build_linkargs_assignments(
                        linkargs.expect("prebuild flag requires linkargs"),
                        2,
                    )?)
                } else {
                    None
                };
                let assignments =
                    self.build_term_assignments_for_terms(terms.clone(), first_visible_place)?;
                let eventuality =
                    self.build_generated_predication_eventuality(predication_source.clone())?;
                self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
                (eventuality, prebuilt_linkarg_assignments, assignments)
            }
            None => {
                let eventuality =
                    self.build_generated_predication_eventuality(predication_source.clone())?;
                self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
                let prebuilt_linkarg_assignments = if prebuild_linkarg_assignments {
                    Some(self.build_linkargs_assignments(
                        linkargs.expect("prebuild flag requires linkargs"),
                        2,
                    )?)
                } else {
                    None
                };
                let assignments = self.with_temporal_context(eventuality, |builder| {
                    builder.build_term_assignments_for_terms(terms.clone(), first_visible_place)
                })?;
                (eventuality, prebuilt_linkarg_assignments, assignments)
            }
        };
        let place_question_assignments = assignments.place_questions.clone();
        let mut visible_arguments = assignments.visible_arguments;
        let mut linkarg_modal_arguments = Vec::new();
        let mut bare_jai_raised_participant = None;
        let mut jai_modal_visible_arguments = None;
        if let Some(jai_unit) = jai_unit {
            let moved_place = generated_jai_moved_relation_place(jai_unit)?;
            if jai_unit.tense_modal.is_some() {
                let raised_argument = visible_arguments.remove(&1);
                for sumti in fai_sumti {
                    let argument = self.build_tagged_or_elided_sumti_argument(sumti)?;
                    insert_visible_argument(&mut visible_arguments, moved_place, argument)?;
                }
                if let Some(raised_argument) = raised_argument {
                    jai_modal_visible_arguments =
                        Some(BTreeMap::from([(1, raised_argument.clone())]));
                }
            } else if moved_place > 1 {
                let raised_argument = visible_arguments.remove(&1);
                visible_arguments =
                    shift_generated_visible_arguments_after_jai_raised_argument(visible_arguments)?;
                for sumti in fai_sumti {
                    let argument = self.build_tagged_or_elided_sumti_argument(sumti)?;
                    insert_visible_argument(&mut visible_arguments, moved_place, argument)?;
                }
                if let Some(raised_argument) = raised_argument {
                    jai_modal_visible_arguments =
                        Some(BTreeMap::from([(1, raised_argument.clone())]));
                    if jai_unit.tense_modal.is_none()
                        && let Some(raised_operand) = raised_argument.value
                    {
                        bare_jai_raised_participant = Some((jai_unit, moved_place, raised_operand));
                    }
                }
            } else {
                apply_generated_bare_jai_visible_argument_with_source(
                    self,
                    &mut visible_arguments,
                    Some(jai_unit),
                    self.source_for_node(generated_linked_tanru_unit(unit)?, "abstraction-about"),
                )?;
            }
        }
        if let Some(linkargs) = linkargs {
            let adjusted = if let Some(linkarg_assignments) = prebuilt_linkarg_assignments {
                Self::visible_arguments_adjusted_for_linkarg_assignments(
                    visible_arguments,
                    linkarg_assignments,
                    2,
                )?
            } else {
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?
            };
            visible_arguments = adjusted.visible_arguments;
            linkarg_modal_arguments = adjusted.modal_arguments;
        }
        if let Some(unit) = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref()) {
            let raised_referent = jai_modal_visible_arguments
                .as_ref()
                .and_then(|arguments| arguments.get(&1))
                .and_then(|argument| argument.value);
            if let Some(tense_modal) = unit.tense_modal.as_deref()
                && generated_tense_modal_has_event_modifier(tense_modal)
            {
                self.apply_generated_tense_modal_event_modifier_to_eventuality(
                    eventuality,
                    tense_modal,
                    raised_referent,
                )?;
            } else if let Some(raised_referent) = raised_referent
                && let Some(modal_argument) =
                    self.build_generated_jai_modal_argument_for_referent(unit, raised_referent)?
            {
                linkarg_modal_arguments.push(modal_argument);
            }
        }
        if linkargs.is_none()
            && let Some(label) = assigned_pro_bridi_reference_label_for_tanru_unit_atom(atom)
            && let Some(binding) = self.assigned_pro_bridi_bindings.get(&label).cloned()
        {
            let current_visible_arguments = map_visible_arguments_for_generated_conversions(
                visible_arguments.clone(),
                &atom.conversions,
            )?;
            let formula = self.build_generated_assigned_pro_bridi_reference_formula(
                &binding,
                current_visible_arguments,
                &place_question_assignments,
                &assignments.modal_terms,
                eventuality,
                mode,
                predication_source,
                formula_source,
            )?;
            return self.wrap_formula_with_generated_assignment_scopes(
                formula,
                assignments.formula_scopes,
                assignments.coequal_scope_groups,
                assignments.implicit_existentials,
                assignments.term_formula_scopes,
            );
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let mut modal_arguments = self
            .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                eventuality,
                &assignments.modal_terms,
                Some(&arguments),
            )?;
        modal_arguments.extend(linkarg_modal_arguments);
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
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
            &arguments,
            place_count,
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
        let predication_mode = predication_mode_for_relation(&relation, mode);
        let predication = self.next_predication_id();
        let bare_jai_involvement =
            bare_jai_raised_participant.and_then(|(jai_unit, moved_place, raised_operand)| {
                arguments
                    .get(&argument_key(moved_place))
                    .and_then(|argument| argument.value)
                    .map(|moved_operand| (jai_unit, moved_operand, raised_operand))
            });
        let mut predication_object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            predication_mode,
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.place_questions = place_questions;
        self.insert(predication, predication_object)?;
        if let Some(scalar_unit) = scalar_unit {
            let scalar_negation_scope =
                if linkargs.is_some_and(generated_linkargs_provide_scalar_scale_context) {
                    GeneratedScalarNegationScope::MarkerOnly
                } else {
                    GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs
                };
            let scalar_negation = scalar_negation_for_generated_scalar_tanru_unit_atom(
                atom,
                scalar_unit,
                linkargs,
                scalar_negation_scope,
            )?;
            self.set_scalar_negation(predication, scalar_negation)?;
        }
        self.attach_generated_reciprocity_to_predication_for_terms(predication, &terms)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        let formula = match bare_jai_involvement {
            Some((jai_unit, moved_operand, raised_operand))
                if moved_operand != raised_operand
                    && !self.generated_referent_is_abstraction_about_operand(
                        moved_operand,
                        "jai",
                        raised_operand,
                    ) =>
            {
                self.conjoin_generated_bare_jai_involvement_formula(
                    formula,
                    moved_operand,
                    raised_operand,
                    self.source_for_node(jai_unit, "bare-jai-raised-participant"),
                )?
            }
            _ => formula,
        };
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_no_gadri_quantified_argument_formula(
        &mut self,
        simple_tail: &SelbriSimpleBridiTailSyntax,
        description: &DescriptorWithoutGadriSumtiSyntax,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality =
            self.build_generated_predication_eventuality(predication_source.clone())?;
        let variable = self.build_bound_argument_variable(description)?;
        let body = if matches!(simple_tail.selbri.as_ref(), SelbriSyntax::TaggedSelbri(_)) {
            let mut visible_arguments = BTreeMap::new();
            insert_visible_argument(
                &mut visible_arguments,
                1,
                ArgumentValue::filled(variable, None),
            )?;
            let result = self.build_selbri_formula_for_visible_arguments(
                &simple_tail.selbri,
                visible_arguments,
                formula_source,
                "selbri",
                Some(eventuality),
            )?;
            self.set_semantic_object_source(result.head_predication, predication_source)?;
            result.formula
        } else {
            let relation =
                semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
            self.build_relation_formula_for_argument(
                relation,
                ArgumentValue::filled(variable, None),
                Some(eventuality),
                PredicationMode::Asserted,
                predication_source,
                formula_source,
            )?
        };
        self.wrap_formula_with_no_gadri_quantifier(description, variable, body)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_afterthought_sumti_argument_formula(
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
        if connector != CompositionOperator::Joint {
            return Err(unsupported("non-joint afterthought sumti distribution"));
        }
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let mut leading_is_quantified = false;
        let leading = match no_gadri_description_from_sumti_bound(&sumti.leading_sumti)? {
            Some(description) => {
                leading_is_quantified = true;
                let variable = self.build_bound_argument_variable(description)?;
                let leading_eventuality = self.build_generated_predication_eventuality(
                    distributed_predication_source.clone(),
                )?;
                let leading_body = self.build_relation_formula_for_argument(
                    relation.clone(),
                    ArgumentValue::filled(variable, None),
                    Some(leading_eventuality),
                    PredicationMode::Asserted,
                    distributed_predication_source.clone(),
                    distributed_formula_source.clone(),
                )?;
                self.wrap_formula_with_no_gadri_quantifier(description, variable, leading_body)?
            }
            None => {
                let leading_referent = self.build_sumti_bound_referent(&sumti.leading_sumti)?;
                self.build_distributed_relation_formula_for_argument(
                    relation.clone(),
                    ArgumentValue::filled(leading_referent, None),
                    PredicationMode::Asserted,
                    distributed_predication_source.clone(),
                    distributed_formula_source.clone(),
                )?
            }
        };
        let trailing_referent = self.build_sumti_bound_referent(&continuation.sumti)?;
        let trailing = if leading_is_quantified {
            self.build_relation_formula_for_argument(
                relation,
                ArgumentValue::filled(trailing_referent, None),
                None,
                PredicationMode::Asserted,
                distributed_predication_source,
                distributed_formula_source,
            )?
        } else {
            self.build_distributed_relation_formula_for_argument(
                relation,
                ArgumentValue::filled(trailing_referent, None),
                PredicationMode::Asserted,
                distributed_predication_source,
                distributed_formula_source,
            )?
        };
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![leading, trailing],
                Some(new!(Connector {
                    source: "e".to_owned(),
                    locus: "sumti".to_owned(),
                    truth_table: Some("TFFF".to_owned()),
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_distributed_relation_formula_for_argument(
        &mut self,
        relation: RelationLabel,
        argument: ArgumentValue,
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
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), argument);
        for place in 2..=place_limit {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(place),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        let eventuality =
            self.build_generated_predication_eventuality(predication_source.clone())?;
        let predication_mode = predication_mode_for_relation(&relation, mode);
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::predication(
                relation.display_text(),
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

    #[requires(argument.value.is_some_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent || id.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_formula_for_argument(
        &mut self,
        relation: RelationLabel,
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
            None => self.build_generated_predication_eventuality(predication_source.clone())?,
        };
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), argument);
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
                relation.display_text(),
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
    pub(super) fn build_bound_argument_variable<N: TreeNode>(
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

    #[requires(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_no_gadri_quantifier(
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
                generated_quantifier_formula_operator(&description.quantifier),
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
    pub(super) fn wrap_formula_with_generated_assignment_scopes<'syntax>(
        &mut self,
        formula: SemanticObjectId,
        argument_scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        implicit_existentials: Vec<GeneratedImplicitExistential>,
        term_scopes: Vec<GeneratedTermFormulaScope>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if self.pending_negated_selbri_argument_scope_reservations > 0
            && !argument_scopes.is_empty()
        {
            self.reserve_generated_semantic_id();
            self.pending_negated_selbri_argument_scope_reservations -= 1;
        }
        let mut scopes = argument_scopes
            .into_iter()
            .map(|scope| {
                let order = self
                    .source_for_generated_argument_quantifier_scope(
                        scope.source,
                        scope.node,
                        "quantifier-scope",
                    )
                    .map(|source| source.span.byte_start)
                    .unwrap_or(usize::MAX);
                (order, GeneratedOrderedFormulaScope::Argument(scope))
            })
            .collect::<Vec<_>>();
        scopes.extend(coequal_scope_groups.into_iter().map(|scope| {
            let order = scope
                .source
                .as_ref()
                .map(|source| source.span.byte_start)
                .unwrap_or(usize::MAX);
            (order, GeneratedOrderedFormulaScope::Bundle(scope))
        }));
        for existential in implicit_existentials {
            if self.defer_active_prenex_implicit_existentials > 0
                && self.generated_variable_has_active_prenex_binding(existential.variable)
            {
                self.deferred_active_prenex_implicit_existentials
                    .push(existential);
                continue;
            }
            let order = existential
                .source
                .as_ref()
                .map(|source| source.span.byte_start)
                .unwrap_or(usize::MAX);
            scopes.push((
                order,
                GeneratedOrderedFormulaScope::ImplicitExistential(existential),
            ));
        }
        scopes.extend(term_scopes.into_iter().map(|scope| {
            let order = generated_term_formula_scope_source(&scope)
                .map(|source| source.span.byte_start)
                .unwrap_or(usize::MAX);
            (order, GeneratedOrderedFormulaScope::Term(scope))
        }));
        scopes.sort_by_key(|(order, _scope)| *order);

        let mut prepared_scopes = Vec::with_capacity(scopes.len());
        for (_order, scope) in scopes {
            prepared_scopes.push(match scope {
                GeneratedOrderedFormulaScope::Argument(scope) => {
                    let mut restrictions = self
                        .generated_argument_restrictions_for_scope_source_in_formula_context(
                            formula,
                            scope.source,
                            scope.variable,
                        )?;
                    restrictions.extend(
                        self.lower_generated_argument_scope_source_restriction_nodes(
                            &scope,
                            scope.variable,
                        )?,
                    );
                    restrictions.extend(self.clone_generated_restriction_formulas_for_variable(
                        &scope.source_restriction_formulas,
                        scope.source_variable,
                        scope.variable,
                    )?);
                    restrictions.extend(scope.inherited_restrictions.iter().copied());
                    restrictions.extend(scope.relative_clause_restrictions.iter().copied());
                    self.record_generated_quantified_da_series_binding(&scope, &restrictions);
                    let quantifier = generated_argument_scope_source_quantifier(scope.source);
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
                    let restriction = self.combine_generated_restriction_formulas(restrictions)?;
                    GeneratedPreparedOrderedFormulaScope::Argument(
                        GeneratedPreparedArgumentFormulaScope {
                            scope,
                            restriction,
                            quantity,
                        },
                    )
                }
                GeneratedOrderedFormulaScope::Bundle(bundle) => {
                    let GeneratedArgumentQuantifierBundleScope { scopes, source } = bundle;
                    let mut prepared = Vec::with_capacity(scopes.len());
                    for scope in scopes {
                        let mut restrictions = self
                            .generated_argument_restrictions_for_scope_source_in_formula_context(
                                formula,
                                scope.source,
                                scope.variable,
                            )?;
                        restrictions.extend(
                            self.lower_generated_argument_scope_source_restriction_nodes(
                                &scope,
                                scope.variable,
                            )?,
                        );
                        restrictions.extend(
                            self.clone_generated_restriction_formulas_for_variable(
                                &scope.source_restriction_formulas,
                                scope.source_variable,
                                scope.variable,
                            )?,
                        );
                        restrictions.extend(scope.inherited_restrictions.iter().copied());
                        restrictions.extend(scope.relative_clause_restrictions.iter().copied());
                        self.record_generated_quantified_da_series_binding(&scope, &restrictions);
                        let quantifier = generated_argument_scope_source_quantifier(scope.source);
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
                        let restriction =
                            self.combine_generated_restriction_formulas(restrictions)?;
                        prepared.push(GeneratedPreparedArgumentFormulaScope {
                            scope,
                            restriction,
                            quantity,
                        });
                    }
                    GeneratedPreparedOrderedFormulaScope::Bundle(
                        GeneratedPreparedArgumentQuantifierBundleScope {
                            scopes: prepared,
                            source,
                        },
                    )
                }
                GeneratedOrderedFormulaScope::ImplicitExistential(existential) => {
                    GeneratedPreparedOrderedFormulaScope::ImplicitExistential(existential)
                }
                GeneratedOrderedFormulaScope::Term(scope) => {
                    GeneratedPreparedOrderedFormulaScope::Term(scope)
                }
            });
        }

        let mut body = formula;
        for scope in prepared_scopes.into_iter().rev() {
            body = match scope {
                GeneratedPreparedOrderedFormulaScope::Argument(argument_scope) => {
                    let GeneratedPreparedArgumentFormulaScope {
                        scope,
                        restriction,
                        quantity,
                    } = argument_scope;
                    self.wrap_formula_with_generated_argument_scope(
                        body,
                        scope,
                        restriction,
                        quantity,
                    )?
                }
                GeneratedPreparedOrderedFormulaScope::Bundle(bundle) => {
                    self.wrap_formula_with_generated_argument_bundle_scope(body, bundle)?
                }
                GeneratedPreparedOrderedFormulaScope::ImplicitExistential(existential) => {
                    self.build_generated_implicit_existential_formula(body, existential)?
                }
                GeneratedPreparedOrderedFormulaScope::Term(
                    GeneratedTermFormulaScope::Negation { source },
                ) => self.build_unary_formula(FormulaOperator::Not, body, source)?,
            };
        }
        Ok(body)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_argument_scopes<'syntax>(
        &mut self,
        formula: SemanticObjectId,
        scopes: Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut prepared_scopes = Vec::with_capacity(scopes.len());
        for scope in scopes {
            let mut restrictions = self
                .generated_argument_restrictions_for_scope_source_in_formula_context(
                    formula,
                    scope.source,
                    scope.variable,
                )?;
            restrictions.extend(
                self.lower_generated_argument_scope_source_restriction_nodes(
                    &scope,
                    scope.variable,
                )?,
            );
            restrictions.extend(self.clone_generated_restriction_formulas_for_variable(
                &scope.source_restriction_formulas,
                scope.source_variable,
                scope.variable,
            )?);
            restrictions.extend(scope.inherited_restrictions.iter().copied());
            restrictions.extend(scope.relative_clause_restrictions.iter().copied());
            self.record_generated_quantified_da_series_binding(&scope, &restrictions);
            let quantifier = generated_argument_scope_source_quantifier(scope.source);
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
            let restriction = self.combine_generated_restriction_formulas(restrictions)?;
            prepared_scopes.push((scope, restriction, quantity));
        }
        let mut body = formula;
        for (scope, restriction, quantity) in prepared_scopes.into_iter().rev() {
            body = self.wrap_formula_with_generated_argument_scope(
                body,
                scope,
                restriction,
                quantity,
            )?;
        }
        Ok(body)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_argument_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: GeneratedArgumentQuantifierScope<'_>,
        restriction: Option<SemanticObjectId>,
        quantity: GeneratedPreparedArgumentQuantity,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let quantifier = generated_argument_scope_source_quantifier(scope.source);
        let source = self.source_for_generated_argument_quantifier_scope(
            scope.source,
            scope.node,
            "quantifier-scope",
        );
        let distribution_quantity = match quantity.as_data() {
            data!(GeneratedPreparedArgumentQuantity::Single(quantity)) => Some(*quantity),
            data!(GeneratedPreparedArgumentQuantity::Connected(_)) => None,
        };
        if let Some(distribution) = self
            .build_generated_quantified_respectively_distribution_formula(
                formula,
                scope.variable,
                distribution_quantity,
                restriction,
                source.clone(),
            )?
        {
            return Ok(distribution);
        }
        if let data!(GeneratedPreparedArgumentQuantity::Connected(connection)) =
            quantity.clone().into_data()
        {
            let data!(GeneratedConnectedQuantifierQuantityScope {
                left_quantity,
                right_quantity,
                left_negated,
                right_negated,
                operator: connection_operator,
                connector,
                source: connection_source,
            }) = connection.into_data();
            let left = self.next_formula_id();
            self.insert(
                left,
                SemanticObject::quantified_formula(
                    generated_quantifier_formula_operator(quantifier),
                    scope.variable,
                    restriction,
                    formula,
                    Some(left_quantity),
                    source.clone(),
                    Vec::new(),
                )
                .with_quantifier_selection(scope.source_variable, scope.selection_source.clone()),
            )?;
            let left = if left_negated {
                self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
            } else {
                left
            };
            let right = self.next_formula_id();
            self.insert(
                right,
                SemanticObject::quantified_formula(
                    generated_quantifier_formula_operator(quantifier),
                    scope.variable,
                    restriction,
                    formula,
                    Some(right_quantity),
                    source.clone(),
                    Vec::new(),
                )
                .with_quantifier_selection(scope.source_variable, scope.selection_source),
            )?;
            let right = if right_negated {
                self.build_unary_formula(FormulaOperator::Not, right, source)?
            } else {
                right
            };
            let connected = self.next_formula_id();
            self.insert(
                connected,
                SemanticObject::connective_formula(
                    connection_operator,
                    vec![left, right],
                    Some(connector),
                    connection_source,
                    Vec::new(),
                ),
            )?;
            return Ok(connected);
        }
        let quantity = match quantity.into_data() {
            data!(GeneratedPreparedArgumentQuantity::Single(quantity)) => quantity,
            data!(GeneratedPreparedArgumentQuantity::Connected(_)) => {
                unreachable!("connected generated quantifier quantity handled above");
            }
        };
        let scoped = self.next_formula_id();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                generated_quantifier_formula_operator(quantifier),
                scope.variable,
                restriction,
                formula,
                Some(quantity),
                source,
                Vec::new(),
            )
            .with_quantifier_selection(scope.source_variable, scope.selection_source),
        )?;
        Ok(scoped)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))]
    #[requires(restriction.is_none_or(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_quantified_respectively_distribution_formula(
        &mut self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
        quantity: Option<SemanticObjectId>,
        restriction: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(quantity) = quantity else {
            return Ok(None);
        };
        let Some(count) = self.exact_generated_integer_quantity_value(quantity) else {
            return Ok(None);
        };
        let Some((composite, members)) =
            self.generated_respectively_composite_argument_paired_with_variable(formula, variable)
        else {
            return Ok(None);
        };
        if members.len() != count {
            return Ok(None);
        }

        let composite_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Entity,
            ParameterRole::RespectiveSlot,
        )?;
        let witness_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Entity,
            ParameterRole::RespectiveSlot,
        )?;
        let replacements = BTreeMap::from([(composite, composite_slot), (variable, witness_slot)]);
        let Some(body) =
            self.clone_generated_formula_with_argument_replacements(formula, &replacements)?
        else {
            return Ok(None);
        };

        let restriction = restriction
            .map(|restriction| {
                let replacements = BTreeMap::from([(variable, witness_slot)]);
                self.clone_generated_formula_with_argument_replacements(restriction, &replacements)
                    .and_then(|cloned| {
                        cloned.ok_or_else(|| {
                            invalid_graph(
                                "generated respectively witness restriction could not be templated"
                                    .to_owned(),
                            )
                        })
                    })
            })
            .transpose()?;

        let witness_items = (0..count)
            .map(|_| self.build_generated_respective_witness_referent(source.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let streams = vec![
            RespectivelyStream::new(composite_slot, members),
            RespectivelyStream::new_with_details(
                witness_slot,
                witness_items,
                restriction,
                Some(quantity),
            ),
        ];
        let distribution = self.next_formula_id();
        self.insert(
            distribution,
            SemanticObject::respectively_distribution_formula(
                body,
                streams,
                Some(true),
                source,
                Vec::new(),
            ),
        )?;
        Ok(Some(distribution))
    }

    #[requires(quantity.object_kind() == crate::model::SemanticObjectKind::Quantity)]
    #[ensures(true)]
    pub(super) fn exact_generated_integer_quantity_value(
        &self,
        quantity: SemanticObjectId,
    ) -> Option<usize> {
        let object = self.objects.get(&quantity)?;
        if object.object_kind() != crate::model::SemanticObjectKind::Quantity {
            return None;
        }
        object
            .value
            .as_ref()?
            .integer
            .and_then(|integer| usize::try_from(integer).ok())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variable.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_none_or(|(_composite, members)| !members.is_empty()))]
    pub(super) fn generated_respectively_composite_argument_paired_with_variable(
        &self,
        formula: SemanticObjectId,
        variable: SemanticObjectId,
    ) -> Option<(SemanticObjectId, Vec<SemanticObjectId>)> {
        let formula_object = self.objects.get(&formula)?;
        if !matches!(
            formula_object.operator.as_ref()?.as_data(),
            SemanticOperatorData::Formula(FormulaOperator::Atom)
        ) {
            return None;
        }
        let predication = self.objects.get(&formula_object.predication?)?;
        let has_variable_argument = predication
            .arguments
            .values()
            .any(|argument| argument.value == Some(variable));
        if !has_variable_argument {
            return None;
        }
        predication.arguments.values().find_map(|argument| {
            let value = argument.value?;
            self.generated_respectively_composite_members(value)
                .map(|members| (value, members))
        })
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(ret.as_ref().is_none_or(|members| !members.is_empty()))]
    pub(super) fn generated_respectively_composite_members(
        &self,
        referent: SemanticObjectId,
    ) -> Option<Vec<SemanticObjectId>> {
        let object = self.objects.get(&referent)?;
        if object.object_kind() != crate::model::SemanticObjectKind::Referent
            || object.category != Some(ReferentCategory::Composite)
        {
            return None;
        }
        let composition = object.composition.as_ref()?;
        (composition.operator == CompositionOperator::Respectively
            && !composition.members.is_empty())
        .then(|| composition.members.clone())
    }

    #[requires(!introduced_by.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_generated_parameter_with_source(
        &mut self,
        introduced_by: String,
        source: Option<crate::model::SemanticSource>,
        sort: SemanticSort,
        role: ParameterRole,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(sort, role, introduced_by, source),
        )?;
        Ok(parameter)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_respective_witness_referent(
        &mut self,
        source: Option<crate::model::SemanticSource>,
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
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(bundle.scopes.len() > 1)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_argument_bundle_scope(
        &mut self,
        formula: SemanticObjectId,
        bundle: GeneratedPreparedArgumentQuantifierBundleScope<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let GeneratedPreparedArgumentQuantifierBundleScope { scopes, source } = bundle;
        let mut bindings = Vec::with_capacity(scopes.len());
        for prepared in scopes {
            let GeneratedPreparedArgumentFormulaScope {
                scope,
                restriction,
                quantity,
            } = prepared;
            let quantity = match quantity.into_data() {
                data!(GeneratedPreparedArgumentQuantity::Single(quantity)) => quantity,
                data!(GeneratedPreparedArgumentQuantity::Connected(_)) => {
                    return Err(invalid_graph(
                        "generated quantifier bundle binding cannot carry a connected quantity"
                            .to_owned(),
                    ));
                }
            };
            let quantifier = generated_argument_scope_source_quantifier(scope.source);
            bindings.push(QuantifierBinding::new(
                generated_quantifier_formula_operator(quantifier),
                scope.variable,
                scope.source_variable,
                scope.selection_source,
                restriction,
                Some(quantity),
                self.source_for_generated_argument_quantifier_scope(
                    scope.source,
                    scope.node,
                    "quantifier-scope",
                ),
            ));
        }
        self.reserve_generated_argument_bundle_scope_ids(bindings.len());
        let scoped = self.next_formula_id();
        self.insert(
            scoped,
            SemanticObject::quantifier_bundle_formula(bindings, formula, source, Vec::new()),
        )?;
        Ok(scoped)
    }

    #[requires(scope_count > 1)]
    #[ensures(self.next_index == old(self.next_index) + scope_count * 2 + 1)]
    pub(super) fn reserve_generated_argument_bundle_scope_ids(&mut self, scope_count: usize) {
        for _ in 0..(scope_count * 2 + 1) {
            self.reserve_generated_semantic_id();
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn generated_argument_restrictions_for_scope_source_in_formula_context(
        &mut self,
        formula: SemanticObjectId,
        source: GeneratedArgumentQuantifierSource<'_>,
        variable: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let Some(eventuality) = self.primary_eventuality_for_generated_formula(formula) else {
            return self.generated_argument_restrictions_for_scope_source(source, variable);
        };
        self.with_temporal_context(eventuality, |builder| {
            builder.generated_argument_restrictions_for_scope_source(source, variable)
        })
    }

    #[requires(matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|restrictions| restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn generated_argument_restrictions_for_scope_source(
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
    pub(super) fn generated_argument_restrictions_for_quantified_sumti(
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
    pub(super) fn combine_generated_restriction_formulas(
        &mut self,
        mut restrictions: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        restrictions.sort_unstable();
        restrictions.dedup();
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
    pub(super) fn build_membership_restriction_formula(
        &mut self,
        variable: SemanticObjectId,
        base: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(variable, None));
        arguments.insert(argument_key(2), ArgumentValue::filled(base, None));
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
    pub(super) fn build_no_gadri_restriction_formula(
        &mut self,
        description: &DescriptorWithoutGadriSumtiSyntax,
        variable: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_restrictive_formula(&description.selbri, variable)
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_formula_for_terms(
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
    pub(super) fn build_tanru_formula_for_terms_with_head_eventuality_order(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        head_eventuality_before_terms: bool,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_tanru_formula_for_terms_with_head_eventuality_order_and_mode(
            tanru,
            terms,
            first_visible_place,
            None,
            PredicationMode::Asserted,
            head_eventuality_before_terms,
            source,
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[requires(first_visible_place > 0)]
    #[requires(head_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_formula_for_terms_with_head_eventuality_order_and_mode(
        &mut self,
        tanru: &TanruSelbriSyntax,
        terms: Vec<&TermSyntax>,
        first_visible_place: usize,
        head_eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        head_eventuality_before_terms: bool,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let head_eventuality = if head_eventuality_before_terms {
            match head_eventuality {
                Some(head_eventuality) => Some(head_eventuality),
                None => Some(self.build_generated_predication_eventuality(source.clone())?),
            }
        } else {
            head_eventuality
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        self.record_generated_assigned_pro_bridi_bindings_for_tanru_selbri(
            tanru,
            &assignments.visible_arguments,
        )?;
        let result = self
            .build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
                tanru,
                assignments.visible_arguments,
                head_eventuality,
                source,
                &assignments.modal_terms,
            )?;
        if mode != PredicationMode::Asserted {
            self.set_formula_predication_mode(result.formula, mode);
        }
        self.wrap_formula_with_generated_assignment_scopes(
            result.formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(!tanru.additional_units.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_formula_for_visible_arguments(
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
    pub(super) fn build_tanru_formula_for_visible_arguments_with_head_eventuality(
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
    pub(super) fn build_tanru_formula_result_for_visible_arguments_with_head_eventuality(
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
    pub(super) fn build_tanru_formula_result_for_visible_arguments_with_head_eventuality_and_modal_terms(
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
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: "selbri".to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
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
    pub(super) fn build_tanru_formula_for_connected_selbri_with_visible_arguments(
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
    pub(super) fn build_connected_selbri_tanru_formula_for_visible_arguments(
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
    pub(super) fn build_connected_selbri_tanru_formula_for_visible_arguments_with_leading_eventuality(
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
    pub(super) fn build_tanru_selbri_formula_for_visible_arguments(
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
    pub(super) fn build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
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
    pub(super) fn build_selbri_formula_for_visible_arguments(
        &mut self,
        selbri: &SelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        match selbri {
            SelbriSyntax::TaggedSelbri(tagged) => {
                let inner = SelbriSyntax::UntaggedSelbri(tagged.inner_selbri.as_ref().clone());
                if generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref()) {
                    if leading_eventuality.is_some() {
                        return Err(unsupported(
                            "eventuality on scoped tagged visible-argument selbri",
                        ));
                    }
                    let result = self.build_selbri_formula_for_visible_arguments(
                        &inner,
                        visible_arguments,
                        source.clone(),
                        connector_locus,
                        None,
                    )?;
                    let formula = self.build_generated_tense_scope_formula(
                        result.formula,
                        tagged.tense_modal.as_ref(),
                        self.source_for_node(tagged, "tense-scope"),
                    )?;
                    Ok(GeneratedTanruFormulaForArgument::from_data(data!(
                        GeneratedTanruFormulaForArgument {
                            formula,
                            x1_argument: result.x1_argument.clone(),
                            head_predication: result.head_predication,
                        }
                    )))
                } else {
                    let leading_eventuality = match leading_eventuality {
                        Some(eventuality) => {
                            if generated_tense_modal_has_event_modifier(tagged.tense_modal.as_ref())
                            {
                                self.apply_generated_tense_modal_event_modifier_to_eventuality(
                                    eventuality,
                                    tagged.tense_modal.as_ref(),
                                    None,
                                )?;
                            }
                            Some(eventuality)
                        }
                        None => self.build_generated_tense_eventuality(
                            tagged.tense_modal.as_ref(),
                            source.clone(),
                        )?,
                    };
                    self.build_selbri_formula_for_visible_arguments(
                        &inner,
                        visible_arguments,
                        source,
                        connector_locus,
                        leading_eventuality,
                    )
                }
            }
            SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) => self
                .build_co_selbri_inversion_formula_for_visible_arguments(
                    co_selbri,
                    visible_arguments,
                    source,
                    leading_eventuality,
                ),
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
    pub(super) fn build_forethought_selbri_connection_formula_for_visible_arguments(
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
            connection.first_branch.selbri.as_ref(),
            visible_arguments.clone(),
            source.clone(),
            connector_locus,
            None,
        )?;
        let mut formula = self.build_binary_formula_for_generated_forethought_selbri_connective(
            &connection.guhek,
            &connection.first_branch.gik,
            connector_locus,
            leading.formula,
            trailing.formula,
            source.clone(),
        )?;
        for branch in &connection.additional_branches {
            let trailing = self.build_selbri_formula_for_visible_arguments(
                branch.selbri.as_ref(),
                visible_arguments.clone(),
                source.clone(),
                connector_locus,
                None,
            )?;
            formula = self.build_binary_formula_for_generated_extra_forethought_selbri_connective(
                &connection.guhek,
                &branch.gik,
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

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
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
            unit.first_branch.unit.as_ref(),
            visible_arguments.clone(),
            None,
            source.clone(),
            connector_locus,
        )?;
        let mut formula = self.build_binary_formula_for_generated_forethought_selbri_connective(
            &unit.guhek,
            &unit.first_branch.gik,
            connector_locus,
            leading.formula,
            trailing.formula,
            source.clone(),
        )?;
        for branch in &unit.additional_branches {
            let trailing = self.build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
                branch.unit.as_ref(),
                visible_arguments.clone(),
                None,
                source.clone(),
                connector_locus,
            )?;
            formula = self.build_binary_formula_for_generated_extra_forethought_selbri_connective(
                &unit.guhek,
                &branch.gik,
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

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_unit_formula_for_visible_arguments(
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
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => self
                .build_bound_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                    leading_eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                    leading_eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => self
                .build_assigned_pro_bridi_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    leading_eventuality,
                ),
        }
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bound_tanru_unit_formula_for_visible_arguments(
        &mut self,
        unit: &BoundTanruUnitSyntax,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        connector_locus: &str,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        if let Some(connective) = &unit.bo_connective {
            if leading_eventuality.is_some() {
                return Err(unsupported(
                    "preallocated connected BO-bound tanru unit eventuality",
                ));
            }
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
            leading_eventuality,
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
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: connector_locus.to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
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
    pub(super) fn build_tanru_head_relation_formula(
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
    pub(super) fn build_tanru_head_relation_formula_with_modal_terms(
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
                self.build_bound_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    "tanru-unit",
                    eventuality,
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
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => self
                .build_assigned_pro_bridi_tanru_unit_head_relation_formula_with_modal_terms(
                    unit,
                    visible_arguments,
                    eventuality,
                    source,
                    modal_terms,
                ),
        }
    }

    #[requires(!unit.0.links.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_tanru_unit_head_formula(
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
    pub(super) fn build_tanru_head_relation_formula_for_bo_or_linked_tanru_unit(
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
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => self
                .build_forethought_selbri_group_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    connector_locus,
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => self
                .build_assigned_pro_bridi_tanru_unit_formula_for_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    eventuality,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_head_relation_formula_for_linked_tanru_unit(
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
    pub(super) fn build_tanru_head_relation_formula_from_parts(
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
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    atom,
                    scalar_unit,
                    None,
                    GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs,
                )?,
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
        let linkarg_visible_places = match linkargs {
            Some(linkargs) => generated_linkargs_visible_places(linkargs, 2)?,
            None => BTreeSet::new(),
        };
        let should_prebuild_linkarg_assignments =
            eventuality.is_none() && !linkarg_visible_places.is_empty();
        let should_prebuild_linkarg_assignments_before_event =
            should_prebuild_linkarg_assignments && linkarg_visible_places.contains(&2);
        let prebuilt_linkarg_assignments = match linkargs {
            Some(linkargs) if should_prebuild_linkarg_assignments_before_event => {
                Some(self.build_linkargs_assignments(linkargs, 2)?)
            }
            _ => None,
        };
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(source.clone())?,
        };
        let prebuilt_linkarg_assignments = match linkargs {
            Some(linkargs)
                if should_prebuild_linkarg_assignments
                    && !should_prebuild_linkarg_assignments_before_event =>
            {
                Some(self.build_linkargs_assignments(linkargs, 2)?)
            }
            _ => prebuilt_linkarg_assignments,
        };
        apply_generated_bare_jai_visible_argument(
            self,
            &mut visible_arguments,
            bare_generated_jai_modal_tanru_unit(atom.base.as_ref()),
        )?;
        let visible_x1_argument = visible_arguments.get(&1).cloned();
        let mut linkarg_modal_arguments = Vec::new();
        if let Some(linkargs) = linkargs {
            let adjusted = if let Some(linkarg_assignments) = prebuilt_linkarg_assignments {
                Self::visible_arguments_adjusted_for_linkarg_assignments(
                    visible_arguments,
                    linkarg_assignments,
                    2,
                )?
            } else {
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?
            };
            visible_arguments = adjusted.visible_arguments;
            linkarg_modal_arguments = adjusted.modal_arguments;
        }
        if let Some(unit) = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref()) {
            linkarg_modal_arguments.push(self.build_generated_jai_modal_argument(
                unit,
                &visible_arguments,
                eventuality,
            )?);
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
        self.apply_generated_tagged_term_event_modifiers(eventuality, modal_terms)?;
        let mut modal_arguments = self
            .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                eventuality,
                modal_terms,
                Some(&arguments),
            )?;
        modal_arguments.extend(linkarg_modal_arguments);
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
            .or_else(|| arguments.get(&argument_key(1)).cloned())
            .ok_or_else(|| unsupported("tanru without visible x1"))?;
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source.clone(),
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        if let Some(scalar_unit) = scalar_unit {
            let scalar_negation_scope =
                if linkargs.is_some_and(generated_linkargs_provide_scalar_scale_context) {
                    GeneratedScalarNegationScope::MarkerOnly
                } else {
                    GeneratedScalarNegationScope::VisibleArgumentsAndLinkargs
                };
            let scalar_negation = scalar_negation_for_generated_scalar_tanru_unit_atom(
                atom,
                scalar_unit,
                linkargs,
                scalar_negation_scope,
            )?;
            self.set_scalar_negation(predication, scalar_negation)?;
        }
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
    pub(super) fn build_term_assignments_for_terms<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        self.build_term_assignments_for_terms_with_shared_tail_source(
            terms,
            first_visible_place,
            None,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_term_assignments_for_terms_with_shared_tail_source<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
        shared_tail_start: Option<usize>,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        if shared_tail_start.is_none() {
            return self.build_term_assignments_for_terms_without_shared_tail_source(
                terms,
                first_visible_place,
            );
        }
        self.build_term_assignments_for_terms_with_shared_tail_source_core(
            terms,
            first_visible_place,
            shared_tail_start,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_term_assignments_for_terms_without_shared_tail_source<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        self.build_term_assignments_for_terms_excluding_source(terms, first_visible_place, None)
            .map(|(assignments, _)| assignments)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_term_assignments_for_terms_with_shared_tail_source_core<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
        shared_tail_start: Option<usize>,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        let mut assignments = empty_generated_term_assignments();
        assignments.next_visible_place = first_visible_place;
        let governed_termsets = generated_governed_termset_indices_for_terms(&terms);
        for (index, term) in terms.into_iter().enumerate() {
            if governed_termsets.contains(&index) {
                continue;
            }
            let existing_places = assignments
                .visible_arguments
                .keys()
                .copied()
                .collect::<BTreeSet<_>>();
            let existential_start = self.implicit_existential_variables.len();
            self.insert_generated_term_assignment(
                &mut assignments.visible_arguments,
                &mut assignments.place_questions,
                &mut assignments.modal_terms,
                &mut assignments.formula_scopes,
                &mut assignments.coequal_scope_groups,
                &mut assignments.term_formula_scopes,
                &mut assignments.next_visible_place,
                term,
            )?;
            assignments.implicit_existentials.extend(
                self.implicit_existential_variables
                    .split_off(existential_start),
            );
            if shared_tail_start.is_some_and(|start| index >= start)
                && generated_shared_head_term_uses_shared_source(term)
            {
                let source = self.source_for_node(term, "shared-tail-term");
                for (place, argument) in &mut assignments.visible_arguments {
                    if !existing_places.contains(place) {
                        *argument = argument.clone().with_data(data! {
                            source: source.clone(),
                        });
                    }
                }
            }
        }
        Ok(assignments)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(assignments, _)| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_term_assignments_for_terms_excluding_source<'syntax>(
        &mut self,
        terms: Vec<&'syntax TermSyntax>,
        first_visible_place: usize,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<(GeneratedTermAssignments<'syntax>, bool), SemanticsError> {
        let mut visible_arguments = BTreeMap::new();
        let mut place_questions = Vec::new();
        let mut modal_terms = Vec::new();
        let mut formula_scopes = Vec::new();
        let mut coequal_scope_groups = Vec::new();
        let mut implicit_existentials = Vec::new();
        let mut term_formula_scopes = Vec::new();
        let mut next_visible_place = first_visible_place;
        let mut skipped_excluded_source = false;
        let mut assigned_places_for_skipped = BTreeSet::new();
        let governed_termsets = generated_governed_termset_indices_for_terms(&terms);
        for (index, term) in terms.into_iter().enumerate() {
            if governed_termsets.contains(&index) {
                continue;
            }
            if let Some(excluded_source) = excluded_source
                && generated_node_contains_byte_span(term, excluded_source)
            {
                let previous_next_visible_place = next_visible_place;
                let previous_assigned_places = assigned_places_for_skipped.clone();
                advance_next_visible_place_after_generated_term(
                    term,
                    &mut next_visible_place,
                    &mut assigned_places_for_skipped,
                )?;
                if previous_next_visible_place != next_visible_place
                    || previous_assigned_places != assigned_places_for_skipped
                {
                    skipped_excluded_source = true;
                }
                continue;
            }
            let existential_start = self.implicit_existential_variables.len();
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
            implicit_existentials.extend(
                self.implicit_existential_variables
                    .split_off(existential_start),
            );
            assigned_places_for_skipped.extend(visible_arguments.keys().copied());
        }
        Ok((
            GeneratedTermAssignments {
                visible_arguments,
                next_visible_place,
                place_questions,
                modal_terms,
                formula_scopes,
                coequal_scope_groups,
                implicit_existentials,
                term_formula_scopes,
            },
            skipped_excluded_source,
        ))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_sumti_selbri_formula_for_terms(
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
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        self.apply_generated_tagged_term_event_modifiers(eventuality, &assignments.modal_terms)?;
        let source_operand = self.build_sumti_selbri_source_operand(&sumti_selbri.sumti)?;
        if !arguments.contains_key(&argument_key(1)) {
            let referent = self.build_elided_referent("zo'e".to_owned())?;
            arguments.insert(
                argument_key(1),
                ArgumentValue::elided(referent, "zo'e".to_owned(), None),
            );
        }
        arguments.insert(argument_key(2), ArgumentValue::filled(source_operand, None));
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            "referentOf".to_owned(),
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            source.clone(),
            Vec::new(),
        );
        predication_object.modal_arguments = self
            .build_modal_arguments_for_generated_tagged_terms_for_event(
                eventuality,
                &assignments.modal_terms,
            )?;
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
    pub(super) fn build_sumti_selbri_formula_for_argument(
        &mut self,
        sumti_selbri: &SumtiSelbriTanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if sumti_selbri.moi_marker.is_some() {
            return Err(unsupported("MOI sumti selbri"));
        }
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let source_operand = self.build_sumti_selbri_source_operand(&sumti_selbri.sumti)?;
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), argument);
        arguments.insert(argument_key(2), ArgumentValue::filled(source_operand, None));
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
    pub(super) fn build_sumti_selbri_source_operand(
        &mut self,
        sumti: &SumtiSelbriSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match sumti {
            SumtiSelbriSumtiSyntax::Sumti(sumti) => {
                if let Some(sign) = self.build_generated_letteral_sign_for_sumti(sumti)? {
                    Ok(sign)
                } else {
                    self.build_sumti_referent(sumti)
                }
            }
            SumtiSelbriSumtiSyntax::MeLerfuSumti(_) => Err(unsupported("ME lerfu sumti")),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_property_abstraction_for_tanru_unit(
        &mut self,
        unit: &TanruUnitSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_abstraction_for_tanru_run(unit, &[], source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_property_abstraction_for_tanru_run(
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
    pub(super) fn build_property_abstraction_for_single_tanru_unit(
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
    pub(super) fn build_property_abstraction_output(
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

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_property_abstraction_for_co_selbri(
        &mut self,
        selbri: &CoSelbriSyntax,
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
        let body = self.build_property_formula_for_co_selbri(
            selbri,
            parameter,
            source.clone(),
            GeneratedPropertyTanruContext::PropertyAbstraction,
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_property_abstraction_for_co_selbri_with_visible_arguments(
        &mut self,
        selbri: &CoSelbriSyntax,
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
        let body = self.build_property_formula_for_co_selbri_with_visible_arguments(
            selbri,
            visible_arguments,
            source.clone(),
            GeneratedPropertyTanruContext::PropertyAbstraction,
            None,
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_co_selbri(
        &mut self,
        selbri: &CoSelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(co_tail) = &selbri.co_tail else {
            return self.build_property_formula_for_connected_selbri(
                &selbri.leading_selbri,
                parameter,
                source,
                context,
            );
        };
        let head_formula = self.build_property_formula_for_connected_selbri(
            &selbri.leading_selbri,
            parameter,
            source.clone(),
            context,
        )?;
        let head_predication = self.primary_predication_for_formula(head_formula)?;
        let modifier = self
            .build_property_abstraction_for_co_selbri(&co_tail.trailing_selbri, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            ArgumentValue::filled(parameter, None),
            modifier,
            tanru_relation_name_for_generated_co_pair(
                &co_tail.trailing_selbri,
                &selbri.leading_selbri,
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
                vec![head_formula, relation_formula],
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: "property-inversion".to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_co_selbri_with_visible_arguments(
        &mut self,
        selbri: &CoSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(co_tail) = &selbri.co_tail else {
            return self.build_property_formula_for_connected_selbri_with_visible_arguments(
                &selbri.leading_selbri,
                visible_arguments,
                source,
                context,
                leading_eventuality,
            );
        };
        let head_formula = self
            .build_property_formula_for_connected_selbri_with_visible_arguments(
                &selbri.leading_selbri,
                visible_arguments.clone(),
                source.clone(),
                context,
                leading_eventuality,
            )?;
        let head_predication = self.primary_predication_for_formula(head_formula)?;
        let modifier = self
            .build_property_abstraction_for_co_selbri(&co_tail.trailing_selbri, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            visible_arguments.get(&1).cloned().ok_or_else(|| {
                invalid_graph("generated CO property formula is missing x1".to_owned())
            })?,
            modifier,
            tanru_relation_name_for_generated_co_pair(
                &co_tail.trailing_selbri,
                &selbri.leading_selbri,
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
                vec![head_formula, relation_formula],
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: "property-inversion".to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_selbri(
        &mut self,
        selbri: &SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_formula_for_selbri_with_context(
            selbri,
            parameter,
            source,
            GeneratedPropertyTanruContext::Description,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_selbri_with_context(
        &mut self,
        selbri: &SelbriSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri
            && co_selbri.co_tail.is_some()
        {
            return self
                .build_property_formula_for_co_selbri(co_selbri, parameter, source, context);
        }
        if let Some(tanru) = tanru_selbri_from_selbri(selbri)?
            && !tanru.additional_units.is_empty()
        {
            return self.build_property_formula_for_tanru_selbri(tanru, parameter, source, context);
        }
        let relation = semantic_relation_label(relation_label_from_selbri(selbri)?);
        self.build_property_atom_for_relation_with_eventuality(
            relation,
            parameter,
            source,
            context.predication_eventuality(None),
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_tanru_selbri(
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
    pub(super) fn build_property_formula_for_connected_selbri(
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

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_connected_selbri_with_visible_arguments(
        &mut self,
        selbri: &ConnectedSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = self.build_property_formula_for_tanru_selbri_with_visible_arguments(
            &selbri.leading_selbri,
            visible_arguments.clone(),
            source.clone(),
            context,
            leading_eventuality,
        )?;
        for continuation in &selbri.continuations {
            let trailing = self.build_property_formula_for_tanru_selbri_with_visible_arguments(
                &continuation.trailing_selbri,
                visible_arguments.clone(),
                source.clone(),
                context,
                None,
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

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_tanru_selbri_with_visible_arguments(
        &mut self,
        tanru: &TanruSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        context: GeneratedPropertyTanruContext,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some((trailing_unit, modifier_units)) = tanru.additional_units.split_last() else {
            return self.build_property_formula_for_tanru_unit_with_visible_arguments(
                &tanru.first_unit,
                visible_arguments,
                source,
                context.predication_eventuality(leading_eventuality),
            );
        };
        let tertau_source = context.tertau_source(self, tanru, source.clone());
        let tertau_eventuality = context.predication_eventuality(leading_eventuality);
        let tertau_formula = self.build_property_formula_for_tanru_unit_with_visible_arguments(
            trailing_unit,
            visible_arguments.clone(),
            tertau_source,
            tertau_eventuality,
        )?;
        let head_predication = self.primary_predication_for_formula(tertau_formula)?;
        let modifier = self.build_property_abstraction_for_tanru_run(
            &tanru.first_unit,
            modifier_units,
            source.clone(),
        )?;
        let relation_formula = self.build_tanru_relation_formula(
            visible_arguments.get(&1).cloned().ok_or_else(|| {
                invalid_graph("generated tanru property formula is missing x1".to_owned())
            })?,
            modifier,
            tanru_relation_name_for_generated_unit_run(
                &tanru.first_unit,
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
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: context.connector_locus().to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_tanru_unit_with_visible_arguments(
        &mut self,
        unit: &TanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if !unit.0.links.is_empty() {
            return self
                .build_connected_property_formula_for_tanru_unit_chain_with_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    eventuality,
                );
        }
        match unit.0.first.as_ref() {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_property_formula_for_linked_tanru_unit_with_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let base = linked_tanru_unit_from_cei(unit.base.as_ref());
                self.build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
                    &base.base,
                    base.linkargs.as_ref(),
                    visible_arguments,
                    PredicationMode::Restrictive,
                    self.source_for_node(unit, "restrictive-predication"),
                    self.source_for_node(unit, "restrictive-formula"),
                    eventuality,
                )
            }
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => {
                Err(unsupported("non-atomic tanru unit property arguments"))
            }
        }
    }

    #[requires(!unit.0.links.is_empty())]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_property_formula_for_tanru_unit_chain_with_visible_arguments(
        &mut self,
        unit: &TanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = self
            .build_connected_branch_property_formula_for_bo_or_linked_tanru_unit(
                &unit.0.first,
                visible_arguments.clone(),
                eventuality,
            )?;
        for link in &unit.0.links {
            let trailing = self
                .build_connected_branch_property_formula_for_bo_or_linked_tanru_unit(
                    &link.trailing_unit,
                    visible_arguments.clone(),
                    eventuality,
                )?;
            formula = self.build_binary_formula_for_relation_afterthought_connective(
                &link.connective,
                "selbri",
                formula,
                trailing,
                source.clone(),
            )?;
        }
        Ok(formula)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_branch_property_formula_for_bo_or_linked_tanru_unit(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
                    &unit.base,
                    unit.linkargs.as_ref(),
                    visible_arguments,
                    PredicationMode::Restrictive,
                    self.source_for_node(unit, "restrictive-predication"),
                    self.source_for_node(unit, "restrictive-formula"),
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let base = linked_tanru_unit_from_cei(unit.base.as_ref());
                self.build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
                    &base.base,
                    base.linkargs.as_ref(),
                    visible_arguments,
                    PredicationMode::Restrictive,
                    self.source_for_node(unit, "restrictive-predication"),
                    self.source_for_node(unit, "restrictive-formula"),
                    eventuality,
                )
            }
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => {
                Err(unsupported("non-atomic tanru unit property arguments"))
            }
        }
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_bo_or_linked_tanru_unit_with_visible_arguments(
        &mut self,
        unit: &BoOrLinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match unit {
            BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => self
                .build_property_formula_for_linked_tanru_unit_with_visible_arguments(
                    unit,
                    visible_arguments,
                    source,
                    eventuality,
                ),
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let base = linked_tanru_unit_from_cei(unit.base.as_ref());
                self.build_property_formula_for_linked_tanru_unit_with_visible_arguments(
                    &base,
                    visible_arguments,
                    source,
                    eventuality,
                )
            }
            BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
            | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => {
                Err(unsupported("non-atomic tanru unit property arguments"))
            }
        }
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_tanru_run(
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
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: context.connector_locus().to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_tanru_unit(
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
        if let Some(question) = relation_question_syntax_from_generated_tanru_unit(unit)? {
            return self
                .build_property_atom_for_generated_relation_question(question, parameter, source);
        }
        if let Some(cmavo) = resolvable_generated_pro_bridi_cmavo_from_tanru_unit(unit)?
            && let Some(formula) = self.build_restrictive_formula_for_generated_pro_bridi_frame(
                cmavo,
                parameter,
                source.clone(),
            )?
        {
            return Ok(formula);
        }
        self.build_property_formula_for_bo_or_linked_tanru_unit(&unit.0.first, parameter, source)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_description_property_formula_for_tanru_unit(
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
        if let Some(question) = relation_question_syntax_from_generated_tanru_unit(unit)? {
            return self
                .build_property_atom_for_generated_relation_question(question, parameter, source);
        }
        if let Some(cmavo) = resolvable_generated_pro_bridi_cmavo_from_tanru_unit(unit)?
            && let Some(formula) = self.build_restrictive_formula_for_generated_pro_bridi_frame(
                cmavo,
                parameter,
                source.clone(),
            )?
        {
            return Ok(formula);
        }
        self.build_description_property_formula_for_bo_or_linked_tanru_unit(
            &unit.0.first,
            parameter,
            source,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_description_property_formula_for_bo_or_linked_tanru_unit(
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
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let base = linked_tanru_unit_from_cei(unit.base.as_ref());
                self.build_description_property_formula_for_linked_tanru_unit(
                    &base, parameter, source,
                )
            }
        }
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_description_property_formula_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if unit.linkargs.is_none()
            && let Some(label) = assigned_pro_bridi_reference_label_for_tanru_unit_atom(&unit.base)
            && let Some(binding) = self.assigned_pro_bridi_bindings.get(&label).cloned()
        {
            return self.build_property_formula_for_assigned_pro_bridi_binding(
                &binding, parameter, source,
            );
        }
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
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    &unit.base,
                    scalar_unit,
                    unit.linkargs.as_ref(),
                    GeneratedScalarNegationScope::MarkerOnly,
                )?,
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
    pub(super) fn build_connected_property_formula_for_tanru_unit_chain(
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
    pub(super) fn build_connected_property_formula_for_tanru_unit_chain_with_locus(
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
    pub(super) fn build_property_composition_for_generated_tanru_unit(
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
    pub(super) fn build_property_abstraction_for_bo_or_linked_tanru_unit(
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
    pub(super) fn build_property_composition_from_generated_connective(
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
        let collective = operator.is_mass().then_some(true);
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
    pub(super) fn build_property_formula_for_bo_or_linked_tanru_unit(
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
            BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let base = linked_tanru_unit_from_cei(unit.base.as_ref());
                self.build_property_formula_for_linked_tanru_unit(&base, parameter, source)
            }
        }
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_forethought_selbri_group_tanru_unit(
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
            unit.first_branch.unit.as_ref(),
            parameter,
            source.clone(),
        )?;
        let mut formula = self.build_binary_formula_for_generated_forethought_selbri_connective(
            &unit.guhek,
            &unit.first_branch.gik,
            "property-abstraction",
            leading,
            trailing,
            source.clone(),
        )?;
        for branch in &unit.additional_branches {
            let trailing = self.build_property_formula_for_bo_or_linked_tanru_unit(
                branch.unit.as_ref(),
                parameter,
                source.clone(),
            )?;
            formula = self.build_binary_formula_for_generated_extra_forethought_selbri_connective(
                &unit.guhek,
                &branch.gik,
                "property-abstraction",
                formula,
                trailing,
                source.clone(),
            )?;
        }
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_forethought_tanru_branch_selbri(
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
    pub(super) fn build_property_formula_for_bound_tanru_unit(
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
                Some(new!(Connector {
                    source: "tanru".to_owned(),
                    locus: "property-abstraction".to_owned(),
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_linked_tanru_unit(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if unit.linkargs.is_none()
            && let Some(label) = assigned_pro_bridi_reference_label_for_tanru_unit_atom(&unit.base)
            && let Some(binding) = self.assigned_pro_bridi_bindings.get(&label).cloned()
        {
            return self.build_property_formula_for_assigned_pro_bridi_binding(
                &binding, parameter, source,
            );
        }
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
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    &unit.base,
                    scalar_unit,
                    unit.linkargs.as_ref(),
                    GeneratedScalarNegationScope::MarkerOnly,
                )?,
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
    pub(super) fn build_property_abstraction_for_linked_tanru_unit(
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
    pub(super) fn build_property_abstraction_for_linked_tanru_unit_with_visible_arguments(
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
            GeneratedPredicationEventuality::from_data(data!(
                GeneratedPredicationEventuality::Fresh
            )),
        )?;
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_formula_for_linked_tanru_unit_with_visible_arguments(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
            &unit.base,
            unit.linkargs.as_ref(),
            visible_arguments,
            PredicationMode::Restrictive,
            source.clone(),
            source,
            eventuality,
        )
    }

    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_formula_for_tanru_unit_atom_with_visible_arguments(
        &mut self,
        atom: &TanruUnitAtomSyntax,
        linkargs: Option<&LinkargsSyntax>,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut modal_arguments = Vec::new();
        apply_generated_bare_jai_visible_argument(
            self,
            &mut visible_arguments,
            bare_generated_jai_modal_tanru_unit(atom.base.as_ref()),
        )?;
        if let Some(linkargs) = linkargs {
            let adjusted =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted.visible_arguments;
            modal_arguments = adjusted.modal_arguments;
        }
        let jai_modal = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref());
        let scalar_unit = scalar_negated_tanru_atom_base(atom.base.as_ref());
        let relation = semantic_relation_label(match scalar_unit {
            Some(unit) => relation_label_from_scalar_negated_tanru_unit(unit)?,
            None => relation_label_from_tanru_unit_atom_base(atom.base.as_ref())?,
        });
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        let eventuality = eventuality.resolve(self, predication_source.clone())?;
        let eventuality = if jai_modal.is_some() && eventuality.is_none() {
            Some(self.build_eventuality(predication_source.clone())?)
        } else {
            eventuality
        };
        if let (Some(unit), Some(eventuality)) = (jai_modal, eventuality) {
            modal_arguments.push(self.build_generated_jai_modal_argument(
                unit,
                &visible_arguments,
                eventuality,
            )?);
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            eventuality,
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        if let Some(scalar_negation) = scalar_unit
            .map(|unit| {
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    atom,
                    unit,
                    linkargs,
                    GeneratedScalarNegationScope::MarkerOnly,
                )
            })
            .transpose()?
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
    pub(super) fn build_description_relation_formula_for_tanru_unit_atom_with_visible_arguments(
        &mut self,
        atom: &TanruUnitAtomSyntax,
        linkargs: Option<&LinkargsSyntax>,
        mut visible_arguments: BTreeMap<usize, ArgumentValue>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut modal_arguments = Vec::new();
        if let Some(linkargs) = linkargs {
            let adjusted =
                self.visible_arguments_adjusted_for_linkargs(visible_arguments, linkargs, 2)?;
            visible_arguments = adjusted.visible_arguments;
            modal_arguments = adjusted.modal_arguments;
        }
        let jai_modal = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref());
        let predication_eventuality = if jai_modal.is_some() {
            Some(self.build_eventuality(predication_source.clone())?)
        } else {
            None
        };
        if let (Some(unit), Some(eventuality)) = (jai_modal, predication_eventuality) {
            modal_arguments.push(self.build_generated_jai_modal_argument(
                unit,
                &visible_arguments,
                eventuality,
            )?);
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            predication_eventuality,
            arguments,
            PredicationMode::Restrictive,
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        if let Some(scalar_negation) = scalar_unit
            .map(|unit| {
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    atom,
                    unit,
                    linkargs,
                    GeneratedScalarNegationScope::MarkerOnly,
                )
            })
            .transpose()?
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
    pub(super) fn build_property_formula_for_grouped_tanru_unit(
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
    pub(super) fn build_relation_formula_for_generated_tanru_unit_argument(
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
    pub(super) fn build_eventful_relation_formula_for_generated_tanru_unit_argument(
        &mut self,
        unit: &TanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality =
            self.build_generated_predication_eventuality(predication_source.clone())?;
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
    pub(super) fn build_eventful_relation_formula_for_linked_tanru_unit_argument(
        &mut self,
        unit: &LinkedTanruUnitSyntax,
        argument: ArgumentValue,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality =
            self.build_generated_predication_eventuality(predication_source.clone())?;
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
    pub(super) fn build_relation_formula_for_linked_tanru_unit_argument_with_eventuality(
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
        let mut modal_arguments = Vec::new();
        apply_generated_bare_jai_visible_argument_with_source(
            self,
            &mut visible_arguments,
            bare_generated_jai_modal_tanru_unit(atom.base.as_ref()),
            self.source_for_node(unit, "abstraction-about"),
        )?;
        if let Some(linkargs) = linkargs {
            modal_arguments =
                self.extend_visible_arguments_with_linkargs(&mut visible_arguments, linkargs, 2)?;
        }
        let jai_modal = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref());
        let eventuality = if jai_modal.is_some() && eventuality.is_none() {
            Some(self.build_eventuality(predication_source.clone())?)
        } else {
            eventuality
        };
        if let (Some(unit), Some(eventuality)) = (jai_modal, eventuality) {
            modal_arguments.push(self.build_generated_jai_modal_argument(
                unit,
                &visible_arguments,
                eventuality,
            )?);
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_limit = match place_count {
            Some(place_count) => place_count,
            None => {
                if !relation_has_open_place_structure(&relation) {
                    diagnostics.push(diagnostic(
                        "relation place structure is unavailable; only explicit assigned places are represented",
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
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            eventuality,
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        let scalar_negation = match (scalar_negation, scalar_unit) {
            (Some(scalar_negation), _) => Some(scalar_negation),
            (None, Some(unit)) => Some(scalar_negation_for_generated_scalar_tanru_unit_atom(
                atom,
                unit,
                linkargs,
                GeneratedScalarNegationScope::MarkerOnly,
            )?),
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
    pub(super) fn build_relation_formula_for_generated_tanru_unit_argument_with_eventuality(
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
        let mut modal_arguments = Vec::new();
        apply_generated_bare_jai_visible_argument_with_source(
            self,
            &mut visible_arguments,
            bare_generated_jai_modal_tanru_unit(atom.base.as_ref()),
            self.source_for_node(generated_linked_tanru_unit(unit)?, "abstraction-about"),
        )?;
        if let Some(linkargs) = linkargs {
            modal_arguments =
                self.extend_visible_arguments_with_linkargs(&mut visible_arguments, linkargs, 2)?;
        }
        let jai_modal = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref());
        let eventuality = if jai_modal.is_some() && eventuality.is_none() {
            Some(self.build_eventuality(predication_source.clone())?)
        } else {
            eventuality
        };
        if let (Some(unit), Some(eventuality)) = (jai_modal, eventuality) {
            modal_arguments.push(self.build_generated_jai_modal_argument(
                unit,
                &visible_arguments,
                eventuality,
            )?);
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
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            eventuality,
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        let scalar_negation = match (scalar_negation, scalar_unit) {
            (Some(scalar_negation), _) => Some(scalar_negation),
            (None, Some(unit)) => Some(scalar_negation_for_generated_scalar_tanru_unit_atom(
                atom,
                unit,
                linkargs,
                GeneratedScalarNegationScope::MarkerOnly,
            )?),
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tagged_relation_formula_for_generated_tanru_unit_argument(
        &mut self,
        unit: &TanruUnitSyntax,
        argument: ArgumentValue,
        tense_modal: &TenseModalSyntax,
        mode: PredicationMode,
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
        let mut modal_arguments = Vec::new();
        if let Some(linkargs) = linkargs {
            modal_arguments =
                self.extend_visible_arguments_with_linkargs(&mut visible_arguments, linkargs, 2)?;
        }
        let jai_modal = generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref());
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
                    "multiple generated tagged tanru arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
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
        let eventuality =
            self.build_generated_tense_eventuality(tense_modal, predication_source.clone())?;
        let eventuality = if jai_modal.is_some() && eventuality.is_none() {
            Some(self.build_eventuality(predication_source.clone())?)
        } else {
            eventuality
        };
        if let (Some(unit), Some(eventuality_id)) = (jai_modal, eventuality) {
            modal_arguments.push(
                self.build_generated_jai_modal_argument(
                    unit,
                    &arguments
                        .iter()
                        .filter_map(|(place, argument)| Some((place.get(), argument.clone())))
                        .collect(),
                    eventuality_id,
                )?,
            );
        }
        let relation_text = relation.display_text();
        let relation_metadata = self.build_generated_relation_metadata_for_tanru_atom_base(
            atom.base.as_ref(),
            &relation_text,
            predication_source.clone(),
        )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            eventuality,
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.modal_arguments = modal_arguments;
        predication_object.relation_metadata = relation_metadata;
        self.insert(predication, predication_object)?;
        if let Some(unit) = scalar_unit {
            self.set_scalar_negation(
                predication,
                scalar_negation_for_generated_scalar_tanru_unit_atom(
                    atom,
                    unit,
                    linkargs,
                    GeneratedScalarNegationScope::MarkerOnly,
                )?,
            )?;
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.next_visible_place >= first_visible_place && assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_linkargs_assignments(
        &mut self,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<GeneratedLinkargsAssignments, SemanticsError> {
        let mut assignments = GeneratedLinkargsAssignments {
            visible_arguments: BTreeMap::new(),
            modal_arguments: Vec::new(),
            next_visible_place: first_visible_place,
        };
        self.add_linked_sumti_assignment(&mut assignments, &linkargs.first_link)?;
        for link in &linkargs.bei_links {
            self.add_linked_sumti_assignment(&mut assignments, &link.link)?;
        }
        Ok(assignments)
    }

    #[requires(first_visible_place > 0)]
    #[requires(arguments.keys().all(|place| *place > 0))]
    #[ensures(true)]
    pub(super) fn extend_visible_arguments_with_linkargs(
        &mut self,
        arguments: &mut BTreeMap<usize, ArgumentValue>,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<Vec<ModalArgument>, SemanticsError> {
        let linkargs_assignments =
            self.build_linkargs_assignments(linkargs, first_visible_place)?;
        for (place, argument) in linkargs_assignments.visible_arguments {
            insert_visible_argument(arguments, place, argument)?;
        }
        Ok(linkargs_assignments.modal_arguments)
    }

    #[requires(first_visible_place > 0)]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.next_visible_place >= first_visible_place && assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn visible_arguments_adjusted_for_linkargs(
        &mut self,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        linkargs: &LinkargsSyntax,
        first_visible_place: usize,
    ) -> Result<GeneratedLinkargsAssignments, SemanticsError> {
        let linkarg_assignments = self.build_linkargs_assignments(linkargs, first_visible_place)?;
        Self::visible_arguments_adjusted_for_linkarg_assignments(
            visible_arguments,
            linkarg_assignments,
            first_visible_place,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[requires(linkarg_assignments.next_visible_place >= first_visible_place)]
    #[requires(linkarg_assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.next_visible_place >= first_visible_place && assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn visible_arguments_adjusted_for_linkarg_assignments(
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        linkarg_assignments: GeneratedLinkargsAssignments,
        first_visible_place: usize,
    ) -> Result<GeneratedLinkargsAssignments, SemanticsError> {
        let mut next_tail_place = linkarg_assignments.next_visible_place;
        let mut adjusted_arguments = BTreeMap::new();
        let mut displaced_arguments = Vec::new();
        for (place, argument) in visible_arguments
            .iter()
            .filter(|(place, _)| **place < first_visible_place)
        {
            if linkarg_assignments.visible_arguments.contains_key(place) {
                displaced_arguments.push((*place, argument.clone()));
            } else {
                insert_visible_argument(&mut adjusted_arguments, *place, argument.clone())?;
            }
        }
        for (place, argument) in linkarg_assignments.visible_arguments {
            insert_visible_argument(&mut adjusted_arguments, place, argument)?;
        }
        for (_, argument) in displaced_arguments.into_iter().chain(
            visible_arguments
                .into_iter()
                .filter(|(place, _)| *place >= first_visible_place),
        ) {
            while adjusted_arguments.contains_key(&next_tail_place) {
                next_tail_place += 1;
            }
            insert_visible_argument(&mut adjusted_arguments, next_tail_place, argument)?;
            next_tail_place += 1;
        }
        Ok(GeneratedLinkargsAssignments {
            visible_arguments: adjusted_arguments,
            modal_arguments: linkarg_assignments.modal_arguments,
            next_visible_place: next_tail_place,
        })
    }

    #[requires(first_visible_place > 0)]
    #[requires(visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|(_, arguments)| arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn visible_arguments_shifted_after_linkargs(
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

    #[requires(assignments.next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn add_linked_sumti_assignment(
        &mut self,
        assignments: &mut GeneratedLinkargsAssignments,
        link: &LinkedSumtiSyntax,
    ) -> Result<(), SemanticsError> {
        match link {
            LinkedSumtiSyntax::PlainLinkedSumti(sumti) => {
                let argument = self.build_argument_for_generated_sumti(&sumti.0)?;
                insert_visible_argument(
                    &mut assignments.visible_arguments,
                    assignments.next_visible_place,
                    argument,
                )?;
                assignments.next_visible_place += 1;
            }
            LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
                let place = linked_sumti_place(&sumti.fa.value)?;
                let argument = self.build_tagged_or_elided_sumti_argument(&sumti.sumti)?;
                insert_visible_argument(&mut assignments.visible_arguments, place, argument)?;
                assignments.next_visible_place = assignments.next_visible_place.max(place + 1);
            }
            LinkedSumtiSyntax::TenseTaggedLinkedSumti(sumti) => {
                let modal_argument = self
                    .build_modal_argument_for_generated_tense_tagged_linked_sumti(
                        sumti.tense_modal.as_ref(),
                        sumti.sumti.as_ref(),
                    )?;
                assignments.modal_arguments.push(modal_argument);
            }
            LinkedSumtiSyntax::EmptyLinkedSumti(_) => {
                return Err(unsupported("empty linked sumti"));
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_modal_argument_for_generated_tense_tagged_linked_sumti(
        &mut self,
        tense_modal: &TenseModalSyntax,
        sumti: &TaggedOrElidedSumtiSyntax,
    ) -> Result<ModalArgument, SemanticsError> {
        let Some((introduced_by, relation, visible_place)) =
            generated_modal_relation_spec_for_tense_modal(tense_modal)
        else {
            return Err(unsupported("tense-tagged linked sumti tense modal"));
        };
        let argument = self.build_tagged_or_elided_sumti_argument(sumti)?;
        let arguments = self.modal_argument_map_for_visible_place(
            argument,
            visible_place,
            relation_place_count(self.dictionary, &relation),
        )?;
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

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some()) || ret.is_err())]
    pub(super) fn build_tagged_or_elided_sumti_argument(
        &mut self,
        sumti: &TaggedOrElidedSumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        self.build_tagged_or_elided_sumti_argument_with_visible_arguments(sumti, None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some()) || ret.is_err())]
    pub(super) fn build_tagged_or_elided_sumti_argument_with_visible_arguments(
        &mut self,
        sumti: &TaggedOrElidedSumtiSyntax,
        visible_arguments: Option<&BTreeMap<usize, ArgumentValue>>,
    ) -> Result<ArgumentValue, SemanticsError> {
        match sumti {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                match visible_arguments.and_then(|visible_arguments| {
                    generated_voha_place_for_sumti(sumti)
                        .and_then(|place| visible_arguments.get(&place).cloned())
                }) {
                    Some(argument) => Ok(argument),
                    None => self.build_argument_for_generated_sumti(sumti),
                }
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(sumti) => {
                let source = self.source_for_node(sumti, "elided-sumti");
                let referent =
                    self.build_elided_referent_with_source("zo'e".to_owned(), source.clone())?;
                Ok(ArgumentValue::elided(
                    referent,
                    "zo'e".to_owned(),
                    source_with_construct(source, "elided-place"),
                ))
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.value.is_some()) || ret.is_err())]
    pub(super) fn build_tagged_or_elided_sumti_argument_with_predication_arguments(
        &mut self,
        sumti: &TaggedOrElidedSumtiSyntax,
        arguments: Option<&BTreeMap<PlaceIndex, ArgumentValue>>,
    ) -> Result<ArgumentValue, SemanticsError> {
        match sumti {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                match arguments.and_then(|arguments| {
                    generated_voha_place_for_sumti(sumti)
                        .and_then(|place| arguments.get(&argument_key(place)).cloned())
                }) {
                    Some(argument) => Ok(argument),
                    None => self.build_argument_for_generated_sumti(sumti),
                }
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => {
                self.build_tagged_or_elided_sumti_argument(sumti)
            }
        }
    }

    #[requires(relation.is_displayable())]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_atom_for_relation(
        &mut self,
        relation: RelationLabel,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_property_atom_for_relation_with_eventuality(
            relation,
            parameter,
            source,
            GeneratedPredicationEventuality::from_data(data!(
                GeneratedPredicationEventuality::Absent
            )),
        )
    }

    #[requires(relation.is_displayable())]
    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_atom_for_relation_with_eventuality(
        &mut self,
        relation: RelationLabel,
        parameter: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
        eventuality: GeneratedPredicationEventuality,
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
        arguments.insert(argument_key(1), ArgumentValue::filled(parameter, None));
        let eventuality = eventuality.resolve(self, source.clone())?;
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
                relation.display_text(),
                eventuality,
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

    #[requires(x1.object_kind() == crate::model::SemanticObjectKind::Referent || x1.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_property_atom_for_generated_relation_question(
        &mut self,
        question: GeneratedRelationQuestionSyntax<'_>,
        x1: SemanticObjectId,
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
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(x1, None));
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::relation_parameter_predication(
                parameter,
                Some(eventuality),
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

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication) || ret.is_err())]
    pub(super) fn primary_predication_for_atom_formula(
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
    pub(super) fn primary_predication_for_formula(
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

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))) || ret.is_err())]
    pub(super) fn eventuality_for_generated_formula(
        &self,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let predication = self.primary_predication_for_formula(formula)?;
        self.objects
            .get(&predication)
            .and_then(|object| object.eventuality)
            .ok_or_else(|| {
                invalid_graph(format!(
                    "formula {formula} primary predication {predication} has no eventuality"
                ))
            })
    }

    #[requires(!relation_label.is_empty())]
    #[requires(head_predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tanru_relation_formula(
        &mut self,
        x1_argument: ArgumentValue,
        modifier: SemanticObjectId,
        relation_label: String,
        head_predication: SemanticObjectId,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), x1_argument);
        arguments.insert(argument_key(2), ArgumentValue::filled(modifier, None));
        let predication = self.next_predication_id();
        self.insert(
            predication,
            SemanticObject::tanru_link_predication(
                "tanru".to_owned(),
                None,
                arguments,
                TanruLink::new(
                    head_predication,
                    modifier,
                    RelationLabel::constructed(relation_label),
                ),
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
    pub(super) fn build_binary_formula_for_relation_afterthought_connective(
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
                Some(new!(Connector {
                    source: connector_source,
                    locus: locus.to_owned(),
                    truth_table: generated_relation_afterthought_connective_truth_table(connective),
                    parameter: None,
                })),
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
    pub(super) fn build_binary_formula_for_generated_forethought_selbri_connective(
        &mut self,
        guhek: &GuhekConnectiveSyntax,
        gik: &GikConnectiveSyntax,
        locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_formula_for_generated_forethought_selbri_connective_core(
            guhek,
            generated_guhek_connective_negates_left(guhek),
            generated_gik_connective_negates_right(gik),
            generated_guhek_connective_source(guhek),
            generated_guhek_gik_connective_truth_table(guhek, gik),
            locus,
            left,
            right,
            source,
        )
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_extra_forethought_selbri_connective(
        &mut self,
        guhek: &GuhekConnectiveSyntax,
        gik: &ZantufaExtraGikConnectiveSyntax,
        locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let connector_source = format!(
            "{} {}",
            generated_guhek_connective_source(guhek),
            token_text(&gik.0.value)
        );
        self.build_formula_for_generated_forethought_selbri_connective_core(
            guhek,
            false,
            false,
            connector_source,
            generated_guhek_connective_truth_table_with_negations(guhek, false, false),
            locus,
            left,
            right,
            source,
        )
    }

    #[requires(!connector_source.is_empty())]
    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!locus.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_formula_for_generated_forethought_selbri_connective_core(
        &mut self,
        guhek: &GuhekConnectiveSyntax,
        left_negated: bool,
        right_negated: bool,
        connector_source: String,
        truth_table: Option<String>,
        locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_guhek_connective_formula_operator(guhek);
        let left_formula = if left_negated {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right_formula = if right_negated {
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
                Some(new!(Connector {
                    source: connector_source,
                    locus: locus.to_owned(),
                    truth_table,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(child.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_unary_formula(
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
    pub(super) fn set_formula_predication_mode(
        &mut self,
        formula: SemanticObjectId,
        mode: PredicationMode,
    ) {
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
    pub(super) fn mark_generated_whether_or_not_inert_operand(
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
    pub(super) fn mark_generated_statement_whether_or_not_inert_operand(
        &mut self,
        connective: &StatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
    ) {
        if generated_statement_connective_formula_operator_for_core(connective)
            != FormulaOperator::WhetherOrNot
        {
            return;
        }
        let inert = if generated_statement_connective_has_se(connective) {
            left
        } else {
            right
        };
        self.set_formula_predication_mode(inert, PredicationMode::Inert);
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn mark_generated_bridi_tail_whether_or_not_inert_operand(
        &mut self,
        connective: &BridiTailConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
    ) {
        if generated_bridi_tail_connective_formula_operator(connective)
            != FormulaOperator::WhetherOrNot
        {
            return;
        }
        let inert = if generated_bridi_tail_connective_has_se(connective) {
            left
        } else {
            right
        };
        self.set_formula_predication_mode(inert, PredicationMode::Inert);
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn mark_generated_forethought_whether_or_not_inert_operand(
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

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn mark_generated_modal_forethought_whether_or_not_inert_operand(
        &mut self,
        connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
    ) {
        if generated_modal_forethought_connective_formula_operator(connective)
            != FormulaOperator::WhetherOrNot
        {
            return;
        }
        let inert = if generated_modal_forethought_connective_has_se(connective) {
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
    pub(super) fn insert_scalar_negation_scale_referent(
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
                Some(new!(Descriptor {
                    kind: DescriptorKind::Scale,
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
                })),
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|negation| negation.scale.is_some()) || ret.is_err())]
    pub(super) fn scalar_negation_with_scale_for_modal_arguments(
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
    #[ensures(true)]
    pub(super) fn set_scalar_negation(
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
    #[ensures(true)]
    pub(super) fn apply_scalar_negation_to_tanru_links(
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
    #[ensures(true)]
    pub(super) fn apply_scalar_negation_to_formula_predications(
        &mut self,
        formula: SemanticObjectId,
        scalar_negation: ScalarNegation,
    ) -> Result<(), SemanticsError> {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return Ok(());
        };
        match object.operator.as_ref().map(|operator| operator.as_data()) {
            Some(data!(SemanticOperator::Formula(FormulaOperator::Atom))) => {
                if let Some(predication) = object.predication {
                    self.set_scalar_negation(predication, scalar_negation)?;
                }
            }
            Some(data!(SemanticOperator::Formula(_))) => {
                for child in object.children {
                    self.apply_scalar_negation_to_formula_predications(
                        child,
                        scalar_negation.clone(),
                    )?;
                }
            }
            Some(data!(SemanticOperator::Math(_))) | None => {}
        }
        Ok(())
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
    pub(super) fn tanru_relation_formula_without_positive_head(
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
    pub(super) fn detach_tanru_relation_formula_without_positive_head(
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
    pub(super) fn formula_is_tanru_relation_for_head(
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
}
