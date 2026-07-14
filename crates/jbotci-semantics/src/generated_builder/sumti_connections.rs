use super::*;

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_vocative_sumti_connection_formula(
        &mut self,
        sumti: &'tree SumtiSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let branch = GeneratedDistributedSumtiBranch::Sumti(sumti);
        if generated_logical_sumti_connection_for_branch(branch)?.is_none() {
            return Ok(None);
        }
        self.build_generated_sumti_connection_formula_for_place::<()>(
            "vocativeTarget",
            1,
            &BTreeMap::new(),
            branch,
            1,
            &[],
            PredicationMode::Performative,
            source.clone(),
            source,
            &[],
            &[],
        )
        .map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|argument| argument.as_ref().is_none_or(|argument| argument.value.is_some() || argument.kind == ArgumentValueKind::Deleted)) || ret.is_err())]
    pub(super) fn build_nonlogical_connected_sumti_argument_with_formula_scopes<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
    ) -> Result<Option<ArgumentValue>, SemanticsError> {
        if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
            return Ok(None);
        }
        let afterthought = sumti.base_sumti.leading_sumti.as_ref();
        if afterthought.continuations.is_empty()
            || afterthought.continuations.iter().any(|continuation| {
                generated_argument_connective_is_logical(&continuation.connective)
            })
        {
            return Ok(None);
        }
        let mut has_quantifier =
            generated_argument_quantifier_source_from_sumti_bound(&afterthought.leading_sumti)?
                .is_some();
        for continuation in &afterthought.continuations {
            has_quantifier |=
                generated_argument_quantifier_source_from_sumti_bound(&continuation.sumti)?
                    .is_some();
        }
        if !has_quantifier {
            return Ok(None);
        }
        let leading = self.build_generated_alternative_argument_for_sumti_bound(
            &afterthought.leading_sumti,
            false,
        )?;
        let mut referent = leading
            .argument
            .value
            .ok_or_else(|| unsupported("deleted operand in nonlogical sumti connection"))?;
        formula_scopes.extend(leading.formula_scopes);
        for continuation in &afterthought.continuations {
            let trailing = self
                .build_generated_alternative_argument_for_sumti_bound(&continuation.sumti, false)?;
            let trailing_referent = trailing
                .argument
                .value
                .ok_or_else(|| unsupported("deleted operand in nonlogical sumti connection"))?;
            formula_scopes.extend(trailing.formula_scopes);
            referent = self.build_connected_generated_sumti_referent(
                sumti,
                referent,
                &continuation.connective,
                trailing_referent,
            )?;
        }
        self.finish_generated_sumti_argument(sumti, referent)
            .map(Some)
    }

    #[requires(!relation.is_empty())]
    #[requires(first_visible_place > 0)]
    #[requires(place_limit > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_logical_sumti_connection_formula_for_terms<'syntax: 'tree, F>(
        &mut self,
        relation: &str,
        terms: &[&'syntax TermSyntax],
        first_visible_place: usize,
        place_limit: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        self.build_generated_logical_sumti_connection_formula_for_terms_with_scalar_negation_context(
            relation,
            terms,
            first_visible_place,
            place_limit,
            conversions,
            mode,
            predication_source,
            formula_source,
            None,
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(first_visible_place > 0)]
    #[requires(place_limit > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_logical_sumti_connection_formula_for_terms_with_scalar_negation_context<
        'syntax: 'tree,
        F,
    >(
        &mut self,
        relation: &str,
        terms: &[&'syntax TermSyntax],
        first_visible_place: usize,
        place_limit: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        scalar_negation_context: Option<ScalarNegationContext>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let has_distributed_sumti_connection = terms
            .iter()
            .any(|term| generated_term_has_distributed_sumti_connection(term));
        let has_duplicate_numbered_assignments =
            generated_terms_have_duplicate_numbered_assignments(terms, first_visible_place)?;
        if !has_distributed_sumti_connection && !has_duplicate_numbered_assignments {
            return Ok(None);
        }
        if scalar_negation_context.is_none() && !has_duplicate_numbered_assignments {
            if let Some(formula) = self
                .build_recursive_generated_logical_sumti_connection_formula_for_terms(
                    relation,
                    terms,
                    first_visible_place,
                    place_limit,
                    conversions,
                    mode,
                    predication_source.clone(),
                    formula_source.clone(),
                )?
            {
                return Ok(Some(formula));
            }
        }

        let mut alternatives = BTreeMap::<usize, Vec<GeneratedAlternativeArgumentSource>>::new();
        let mut modal_terms = Vec::new();
        let mut modal_formula_scopes = Vec::new();
        let mut term_formula_scopes = Vec::new();
        let mut connective = None;
        let mut pending_connections = Vec::<(usize, &'syntax SumtiAfterthoughtSyntax)>::new();
        let mut pending_bound_connections = Vec::<(usize, &'syntax SumtiBoundSyntax)>::new();
        let mut pending_forethought_connections =
            Vec::<(usize, &'syntax ForethoughtSumtiSyntax)>::new();
        let mut next_visible_place = first_visible_place;
        let mut highest_assigned_place = 0usize;
        for term in terms {
            let simple = generated_simple_term_for_assignment(term)?;
            match simple {
                SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                    let place = next_visible_place;
                    next_visible_place += 1;
                    highest_assigned_place = highest_assigned_place.max(place);
                    if let Some(afterthought) = generated_sumti_afterthought_for_distribution(sumti)
                    {
                        let [continuation] = afterthought.continuations.as_slice() else {
                            return Err(unsupported(
                                "multi-continuation generated sumti distribution",
                            ));
                        };
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Argument {
                                connective: &continuation.connective,
                                tense_modal: None,
                                bo: false,
                            }));
                        pending_connections.push((place, afterthought));
                    } else if let Some(bound) = generated_sumti_bound_for_distribution(sumti) {
                        let tail = bound
                            .bound_tail
                            .as_ref()
                            .expect("bound distribution has tail");
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Argument {
                                connective: tail.connective.as_ref(),
                                tense_modal: tail.tense_modal.as_deref(),
                                bo: true,
                            }));
                        pending_bound_connections.push((place, bound));
                    } else if let Some(forethought) =
                        generated_sumti_forethought_for_distribution(sumti)
                    {
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Forethought {
                                gek: &forethought.gek,
                                gik: &forethought.first_branch.gik,
                            }));
                        pending_forethought_connections.push((place, forethought));
                    } else {
                        self.insert_generated_sumti_distribution_alternatives(
                            &mut alternatives,
                            place,
                            sumti,
                        )?;
                    }
                }
                SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                    let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() else {
                        let place = fa_place(&term.fa.value)?;
                        insert_generated_alternative_argument(
                            &mut alternatives,
                            place,
                            GeneratedAlternativeArgument {
                                argument: self
                                    .build_tagged_or_elided_sumti_argument(&term.sumti)?,
                                negated: false,
                                formula_scopes: Vec::new(),
                            }
                            .into(),
                        )?;
                        next_visible_place = next_visible_place.max(place + 1);
                        highest_assigned_place = highest_assigned_place.max(place);
                        continue;
                    };
                    let place = fa_place(&term.fa.value)?;
                    highest_assigned_place = highest_assigned_place.max(place);
                    next_visible_place = next_visible_place.max(place + 1);
                    if let Some(afterthought) = generated_sumti_afterthought_for_distribution(sumti)
                    {
                        let [continuation] = afterthought.continuations.as_slice() else {
                            return Err(unsupported(
                                "multi-continuation generated sumti distribution",
                            ));
                        };
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Argument {
                                connective: &continuation.connective,
                                tense_modal: None,
                                bo: false,
                            }));
                        pending_connections.push((place, afterthought));
                    } else if let Some(bound) = generated_sumti_bound_for_distribution(sumti) {
                        let tail = bound
                            .bound_tail
                            .as_ref()
                            .expect("bound distribution has tail");
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Argument {
                                connective: tail.connective.as_ref(),
                                tense_modal: tail.tense_modal.as_deref(),
                                bo: true,
                            }));
                        pending_bound_connections.push((place, bound));
                    } else if let Some(forethought) =
                        generated_sumti_forethought_for_distribution(sumti)
                    {
                        connective =
                            connective.or(Some(GeneratedDistributedSumtiConnective::Forethought {
                                gek: &forethought.gek,
                                gik: &forethought.first_branch.gik,
                            }));
                        pending_forethought_connections.push((place, forethought));
                    } else {
                        self.insert_generated_sumti_distribution_alternatives(
                            &mut alternatives,
                            place,
                            sumti,
                        )?;
                    }
                }
                SimpleTermSyntax::TaggedSumtiTerm(term) => {
                    modal_terms
                        .push(self.prepare_generated_modal_term(term, &mut modal_formula_scopes)?);
                }
                SimpleTermSyntax::NaKuTerm(_) | SimpleTermSyntax::BareNaTerm(_) => {
                    self.collect_generated_term_formula_scopes_for_simple_term(
                        *term,
                        simple,
                        &mut term_formula_scopes,
                    )?;
                }
                _ => return Err(unsupported("non-sumti term")),
            }
        }
        for (place, afterthought) in pending_connections {
            let [continuation] = afterthought.continuations.as_slice() else {
                return Err(unsupported(
                    "multi-continuation generated sumti distribution",
                ));
            };
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::SumtiBound {
                    sumti: &afterthought.leading_sumti,
                    negated: generated_argument_connective_negates_left(&continuation.connective),
                },
            )?;
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::SumtiBound {
                    sumti: &continuation.sumti,
                    negated: generated_argument_connective_negates_right(&continuation.connective),
                },
            )?;
        }
        for (place, bound) in pending_bound_connections {
            let tail = bound
                .bound_tail
                .as_ref()
                .expect("bound distribution has tail");
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::SumtiForethought {
                    sumti: &bound.leading_sumti,
                    negated: generated_argument_connective_negates_left(tail.connective.as_ref()),
                },
            )?;
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::SumtiBound {
                    sumti: &tail.trailing_sumti,
                    negated: generated_argument_connective_negates_right(tail.connective.as_ref()),
                },
            )?;
        }
        for (place, forethought) in pending_forethought_connections {
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::Sumti {
                    sumti: &forethought.leading_sumti,
                    negated: generated_modal_forethought_connective_negates_left(&forethought.gek),
                },
            )?;
            insert_generated_alternative_argument(
                &mut alternatives,
                place,
                GeneratedAlternativeArgumentSource::SumtiForethought {
                    sumti: &forethought.first_branch.sumti,
                    negated: forethought.first_branch.gik.nai.is_some(),
                },
            )?;
            for branch in &forethought.additional_branches {
                insert_generated_alternative_argument(
                    &mut alternatives,
                    place,
                    GeneratedAlternativeArgumentSource::SumtiForethought {
                        sumti: &branch.sumti,
                        negated: false,
                    },
                )?;
            }
        }

        if connective.is_none() && !has_duplicate_numbered_assignments {
            return Ok(None);
        }
        let fill_through = place_limit.max(highest_assigned_place);
        let alternatives = self.prebuild_generated_alternative_arguments_by_place(alternatives)?;
        let mut shared_arguments = BTreeMap::<usize, GeneratedAlternativeArgument>::new();
        let mut branch_alternatives = BTreeMap::<usize, Vec<GeneratedAlternativeArgument>>::new();
        for (place, mut values) in alternatives {
            if values.len() == 1 {
                let value = values.pop().expect("single value just checked");
                shared_arguments.insert(place, value);
            } else {
                branch_alternatives.insert(place, values);
            }
        }
        let mut branches = vec![BTreeMap::<usize, GeneratedAlternativeArgument>::new()];
        for (place, values) in branch_alternatives {
            let mut next = Vec::new();
            for branch in &branches {
                for value in &values {
                    let mut branch = branch.clone();
                    branch.insert(place, value.clone());
                    next.push(branch);
                }
            }
            branches = next;
        }

        let source = predication_source
            .clone()
            .or_else(|| formula_source.clone());
        let mut outer_scopes = shared_arguments
            .values()
            .flat_map(|value| value.formula_scopes.iter().cloned())
            .collect::<Vec<_>>();
        outer_scopes.extend(modal_formula_scopes);
        let connection_formula_source =
            source_with_construct(source.clone(), "sumti-connection-formula");
        let pure_modal_connection =
            connective.is_some_and(generated_distributed_sumti_connective_is_pure_modal);
        let mut children = Vec::new();
        if let Some(scalar_negation_context) = scalar_negation_context {
            let event_template = self.take_deferred_generated_eventuality_template(
                scalar_negation_context.eventuality,
            )?;
            let mut prebuilt_branches = Vec::with_capacity(branches.len());
            for branch in branches {
                let mut values = shared_arguments.clone();
                for (place, value) in branch {
                    if values.insert(place, value).is_some() {
                        return Err(invalid_graph(format!(
                            "multiple generated bridi arguments map to x{place}"
                        )));
                    }
                }
                prebuilt_branches.push(values);
            }

            let mut reserved_deferred_event_compatibility_id = false;
            let mut reserved_scalar_branch_compatibility_id = false;
            for mut branch in prebuilt_branches {
                let eventuality = self.build_generated_branch_eventuality_from_template(
                    event_template.as_ref(),
                    connection_formula_source.clone(),
                )?;
                self.apply_generated_tagged_term_event_modifiers(eventuality, &modal_terms)?;
                if event_template.is_some() && !reserved_deferred_event_compatibility_id {
                    self.reserve_generated_semantic_id();
                    reserved_deferred_event_compatibility_id = true;
                }
                if event_template.is_none() && !reserved_scalar_branch_compatibility_id {
                    self.reserve_generated_semantic_id();
                    reserved_scalar_branch_compatibility_id = true;
                }
                for place in 1..=fill_through {
                    if !branch.contains_key(&place) {
                        branch.insert(
                            place,
                            GeneratedAlternativeArgument {
                                argument: self.build_elided_argument_for_place(place)?,
                                negated: false,
                                formula_scopes: Vec::new(),
                            },
                        );
                    }
                }
                let mut arguments = BTreeMap::new();
                let mut branch_negated = false;
                let mut branch_scopes = Vec::new();
                for (place, value) in branch {
                    branch_negated |= value.negated;
                    branch_scopes.extend(value.formula_scopes);
                    let place = mapped_place_for_generated_conversions(place, conversions)?;
                    let key = argument_key(place);
                    if arguments.insert(key.clone(), value.argument).is_some() {
                        return Err(invalid_graph(format!(
                            "multiple generated bridi arguments map to {key}"
                        )));
                    }
                }
                let modal_arguments = self
                    .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                        eventuality,
                        &modal_terms,
                        Some(&arguments),
                    )?;
                let mut predication_object = SemanticObject::predication(
                    relation.to_owned(),
                    Some(eventuality),
                    arguments,
                    predication_mode_for_relation(relation, mode),
                    connection_formula_source.clone(),
                    Vec::new(),
                );
                predication_object.set_predication_modal_arguments(modal_arguments);
                let predication = self.next_predication_id();
                self.insert(predication, predication_object)?;
                self.set_scalar_negation(
                    predication,
                    scalar_negation_context.scalar_negation.clone(),
                )?;
                let formula = self.next_formula_id();
                self.insert(
                    formula,
                    SemanticObject::atom_formula(
                        predication,
                        connection_formula_source.clone(),
                        Vec::new(),
                    ),
                )?;
                let formula =
                    self.wrap_formula_with_generated_argument_scopes(formula, branch_scopes)?;
                let formula = if branch_negated {
                    self.build_unary_formula(
                        FormulaOperator::Not,
                        formula,
                        source_with_construct(source.clone(), "distributed-negation"),
                    )?
                } else {
                    formula
                };
                children.push(formula);
            }
        } else {
            for mut branch in branches {
                for place in 1..=fill_through {
                    if !branch.contains_key(&place) && !shared_arguments.contains_key(&place) {
                        branch.insert(
                            place,
                            GeneratedAlternativeArgument {
                                argument: self.build_elided_argument_for_place(place)?,
                                negated: false,
                                formula_scopes: Vec::new(),
                            },
                        );
                    }
                }
                let mut arguments = BTreeMap::new();
                let mut branch_negated = false;
                let mut branch_scopes = Vec::new();
                for (place, value) in &shared_arguments {
                    branch_negated |= value.negated;
                    let place = mapped_place_for_generated_conversions(*place, conversions)?;
                    arguments.insert(argument_key(place), value.argument.clone());
                }
                for (place, value) in branch {
                    branch_negated |= value.negated;
                    branch_scopes.extend(value.formula_scopes);
                    let place = mapped_place_for_generated_conversions(place, conversions)?;
                    arguments.insert(argument_key(place), value.argument);
                }
                let eventuality = self.build_generated_predication_eventuality(
                    source_with_construct(source.clone(), "distributed-predication"),
                )?;
                self.apply_generated_tagged_term_event_modifiers(eventuality, &modal_terms)?;
                let modal_arguments = self
                    .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                        eventuality,
                        &modal_terms,
                        Some(&arguments),
                    )?;
                let mut predication_object = SemanticObject::predication(
                    relation.to_owned(),
                    Some(eventuality),
                    arguments,
                    predication_mode_for_relation(relation, mode),
                    source_with_construct(source.clone(), "distributed-predication"),
                    Vec::new(),
                );
                predication_object.set_predication_modal_arguments(modal_arguments);
                let predication = self.next_predication_id();
                self.insert(predication, predication_object)?;
                let formula = self.next_formula_id();
                self.insert(
                    formula,
                    SemanticObject::atom_formula(
                        predication,
                        source_with_construct(source.clone(), "distributed-formula"),
                        Vec::new(),
                    ),
                )?;
                let formula =
                    self.wrap_formula_with_generated_argument_scopes(formula, branch_scopes)?;
                let formula = if branch_negated {
                    self.build_unary_formula(
                        FormulaOperator::Not,
                        formula,
                        source_with_construct(source.clone(), "distributed-negation"),
                    )?
                } else {
                    formula
                };
                children.push(formula);
            }
        }

        let mut diagnostics = Vec::new();
        let mut modal_claim = None;
        if let Some(connective) = connective
            && let Some(spec) = generated_distributed_sumti_connective_modal_spec(connective)
        {
            if let [first_formula, second_formula] = children.as_slice() {
                let (visible_formula, other_formula) =
                    if generated_distributed_sumti_connective_visible_argument_is_first(connective)
                    {
                        (*first_formula, *second_formula)
                    } else {
                        (*second_formula, *first_formula)
                    };
                match self.build_generated_modal_formula_connection_claim(
                    visible_formula,
                    other_formula,
                    &spec,
                    source_with_construct(source.clone(), "sumti-connection-claim"),
                )?
                {
                    Some(claim) => {
                        if pure_modal_connection {
                            self.set_formula_predication_mode(
                                *first_formula,
                                PredicationMode::Inert,
                            );
                            self.set_formula_predication_mode(
                                *second_formula,
                                PredicationMode::Inert,
                            );
                            modal_claim = Some(claim);
                        } else {
                            children.push(claim);
                        }
                    }
                    None => diagnostics.push(diagnostic(
                        "modal sumti connection could not find formula-bearing bridi events to relate",
                    )),
                }
            } else {
                diagnostics.push(diagnostic(
                    "modal sumti connection with more than two distributed branches is not fully lowered yet",
                ));
            }
        }
        if let Some(claim) = modal_claim {
            let formula = self.wrap_formula_with_generated_assignment_scopes(
                claim,
                outer_scopes,
                Vec::new(),
                Vec::new(),
                term_formula_scopes,
            )?;
            return Ok(Some(formula));
        }

        let formula = self.next_formula_id();
        let connector_parameter = self
            .build_generated_connective_question_parameter_for_distributed_sumti_connective_option(
                connective,
            )?;
        self.insert(
            formula,
            SemanticObject::connective_formula(
                connective
                    .map(generated_distributed_sumti_connective_formula_operator)
                    .unwrap_or(FormulaOperator::And),
                children,
                generated_distributed_sumti_connector(connective, connector_parameter)?,
                connection_formula_source,
                diagnostics,
            ),
        )?;
        let formula = self.wrap_formula_with_generated_assignment_scopes(
            formula,
            outer_scopes,
            Vec::new(),
            Vec::new(),
            term_formula_scopes,
        )?;
        Ok(Some(formula))
    }

    #[requires(!relation.is_empty())]
    #[requires(first_visible_place > 0)]
    #[requires(place_limit > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_recursive_generated_logical_sumti_connection_formula_for_terms<
        'syntax: 'tree,
        F,
    >(
        &mut self,
        relation: &str,
        terms: &[&'syntax TermSyntax],
        first_visible_place: usize,
        place_limit: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let mut connected_place = None;
        let mut base_arguments = BTreeMap::<usize, ArgumentValue>::new();
        let mut outer_scopes = Vec::new();
        let mut modal_terms = Vec::new();
        let mut term_formula_scopes = Vec::new();
        let mut assignment_counts = BTreeMap::<usize, usize>::new();
        let mut next_visible_place = first_visible_place;
        let mut highest_assigned_place = 0usize;

        for term in terms {
            let simple = generated_simple_term_for_assignment(term)?;
            match simple {
                SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                    let place = next_visible_place;
                    next_visible_place += 1;
                    highest_assigned_place = highest_assigned_place.max(place);
                    *assignment_counts.entry(place).or_default() += 1;
                    let branch = GeneratedDistributedSumtiBranch::Sumti(sumti);
                    if generated_logical_sumti_connection_for_branch(branch)?.is_some() {
                        if connected_place.replace((place, branch)).is_some() {
                            return Ok(None);
                        }
                    } else {
                        let mut formula_scopes = Vec::new();
                        let argument = self
                            .build_argument_for_generated_sumti_with_formula_scopes(
                                sumti,
                                &mut formula_scopes,
                            )?;
                        outer_scopes.extend(formula_scopes);
                        if base_arguments.insert(place, argument).is_some() {
                            return Ok(None);
                        }
                    }
                }
                SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                    let place = fa_place(&term.fa.value)?;
                    next_visible_place = next_visible_place.max(place + 1);
                    highest_assigned_place = highest_assigned_place.max(place);
                    *assignment_counts.entry(place).or_default() += 1;
                    match term.sumti.as_ref() {
                        TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                            let branch = GeneratedDistributedSumtiBranch::Sumti(sumti);
                            if generated_logical_sumti_connection_for_branch(branch)?.is_some() {
                                if connected_place.replace((place, branch)).is_some() {
                                    return Ok(None);
                                }
                            } else {
                                let mut formula_scopes = Vec::new();
                                let argument = self
                                    .build_argument_for_generated_sumti_with_formula_scopes(
                                        sumti,
                                        &mut formula_scopes,
                                    )?;
                                outer_scopes.extend(formula_scopes);
                                if base_arguments.insert(place, argument).is_some() {
                                    return Ok(None);
                                }
                            }
                        }
                        TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => {
                            let argument =
                                self.build_tagged_or_elided_sumti_argument(&term.sumti)?;
                            if base_arguments.insert(place, argument).is_some() {
                                return Ok(None);
                            }
                        }
                    }
                }
                SimpleTermSyntax::TaggedSumtiTerm(term) => {
                    modal_terms.push(self.prepare_generated_modal_term(term, &mut outer_scopes)?);
                }
                SimpleTermSyntax::NaKuTerm(_) | SimpleTermSyntax::BareNaTerm(_) => {
                    self.collect_generated_term_formula_scopes_for_simple_term(
                        *term,
                        simple,
                        &mut term_formula_scopes,
                    )?;
                }
                _ => return Ok(None),
            }
        }

        if assignment_counts.values().any(|count| *count > 1) {
            return Ok(None);
        }
        let Some((connected_place, connected_sumti)) = connected_place else {
            return Ok(None);
        };
        let fill_through = place_limit.max(highest_assigned_place);
        let formula = self.build_generated_sumti_connection_formula_for_place(
            relation,
            connected_place,
            &base_arguments,
            connected_sumti,
            fill_through,
            conversions,
            mode,
            predication_source,
            formula_source,
            &modal_terms,
            &[],
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            outer_scopes,
            Vec::new(),
            Vec::new(),
            term_formula_scopes,
        )
        .map(Some)
    }

    #[requires(!relation.is_empty())]
    #[requires(first_visible_place > 0)]
    #[requires(place_limit > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_scalar_generated_logical_sumti_connection_formula_for_terms<
        'syntax: 'tree,
        F,
    >(
        &mut self,
        relation: &str,
        terms: &[&'syntax TermSyntax],
        first_visible_place: usize,
        place_limit: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        eventuality: Option<SemanticObjectId>,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        scalar_negation: ScalarNegation,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        self.build_generated_logical_sumti_connection_formula_for_terms_with_scalar_negation_context(
            relation,
            terms,
            first_visible_place,
            place_limit,
            conversions,
            mode,
            predication_source,
            formula_source,
            Some(new!(ScalarNegationContext {
                eventuality,
                scalar_negation,
            })),
        )
    }

    #[requires(!relation.is_empty())]
    #[requires(connected_place > 0)]
    #[requires(fill_through > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_sumti_connection_formula_for_place<'syntax: 'tree, F>(
        &mut self,
        relation: &str,
        connected_place: usize,
        base_arguments: &BTreeMap<usize, ArgumentValue>,
        sumti: GeneratedDistributedSumtiBranch<'syntax>,
        fill_through: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        modal_terms: &[GeneratedModalTerm<'tree>],
        additional_relative_clause_lists: &[&'syntax RelativeClauseListSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(connection) = generated_logical_sumti_connection_for_branch(sumti)? else {
            return self.build_generated_sumti_connection_branch_formula(
                relation,
                connected_place,
                base_arguments,
                sumti,
                fill_through,
                conversions,
                mode,
                predication_source,
                formula_source,
                modal_terms,
                false,
                additional_relative_clause_lists,
            );
        };
        let mut relative_clause_lists = additional_relative_clause_lists.to_vec();
        if let Some(relative_clauses) = connection.relative_clauses {
            relative_clause_lists.push(relative_clauses);
        }
        let leading_formula = self.build_generated_sumti_connection_branch_formula(
            relation,
            connected_place,
            base_arguments,
            connection.leading,
            fill_through,
            conversions,
            mode,
            predication_source.clone(),
            formula_source.clone(),
            modal_terms,
            generated_distributed_sumti_connective_negates_left(connection.connective),
            &relative_clause_lists,
        )?;
        let trailing_formula = self.build_generated_sumti_connection_branch_formula(
            relation,
            connected_place,
            base_arguments,
            connection.trailing,
            fill_through,
            conversions,
            mode,
            predication_source.clone(),
            formula_source.clone(),
            modal_terms,
            generated_distributed_sumti_connective_negates_right(connection.connective),
            &relative_clause_lists,
        )?;
        let mut children = vec![leading_formula, trailing_formula];
        let mut diagnostics = Vec::new();
        let pure_modal_connection =
            generated_distributed_sumti_connective_is_pure_modal(connection.connective);
        if let Some(spec) = generated_distributed_sumti_connective_modal_spec(connection.connective)
        {
            let (visible_formula, other_formula) =
                if generated_distributed_sumti_connective_visible_argument_is_first(
                    connection.connective,
                ) {
                    (leading_formula, trailing_formula)
                } else {
                    (trailing_formula, leading_formula)
                };
            match self.build_generated_modal_formula_connection_claim(
                visible_formula,
                other_formula,
                &spec,
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    "sumti-connection-claim",
                ),
            )? {
                Some(claim) => {
                    if pure_modal_connection {
                        self.set_formula_predication_mode(leading_formula, PredicationMode::Inert);
                        self.set_formula_predication_mode(trailing_formula, PredicationMode::Inert);
                        return Ok(claim);
                    }
                    children.push(claim);
                }
                None => diagnostics.push(diagnostic(
                    "modal sumti connection could not find formula-bearing bridi events to relate",
                )),
            }
        }
        let connector_source =
            generated_distributed_sumti_connective_source(connection.connective)?;
        let formula = self.next_formula_id();
        let connector_parameter = self
            .build_generated_connective_question_parameter_for_distributed_sumti_connective(
                connection.connective,
            )?;
        self.insert(
            formula,
            SemanticObject::connective_formula(
                generated_distributed_sumti_connective_formula_operator(connection.connective),
                children,
                Some(new!(Connector {
                    source: connector_source,
                    locus: "sumti".to_owned(),
                    truth_table: generated_distributed_sumti_connective_truth_table(
                        connection.connective,
                    ),
                    parameter: connector_parameter,
                })),
                source_with_construct(
                    formula_source.or(predication_source),
                    "sumti-connection-formula",
                ),
                diagnostics,
            ),
        )?;
        Ok(formula)
    }

    #[requires(!relation.is_empty())]
    #[requires(connected_place > 0)]
    #[requires(fill_through > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_sumti_connection_branch_formula<'syntax: 'tree, F>(
        &mut self,
        relation: &str,
        connected_place: usize,
        base_arguments: &BTreeMap<usize, ArgumentValue>,
        sumti: GeneratedDistributedSumtiBranch<'syntax>,
        fill_through: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
        modal_terms: &[GeneratedModalTerm<'tree>],
        negated: bool,
        additional_relative_clause_lists: &[&'syntax RelativeClauseListSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut formula = if generated_logical_sumti_connection_for_branch(sumti)?.is_some() {
            self.build_generated_sumti_connection_formula_for_place(
                relation,
                connected_place,
                base_arguments,
                sumti,
                fill_through,
                conversions,
                mode,
                predication_source.clone(),
                formula_source.clone(),
                modal_terms,
                additional_relative_clause_lists,
            )?
        } else {
            let mut raw_arguments = base_arguments.clone();
            let mut argument =
                self.build_generated_alternative_argument_for_sumti_branch(sumti, false)?;
            for relative_clauses in additional_relative_clause_lists {
                argument.argument = self.attach_generated_relative_clauses_to_argument(
                    argument.argument,
                    relative_clauses,
                )?;
            }
            if raw_arguments
                .insert(connected_place, argument.argument)
                .is_some()
            {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to x{connected_place}"
                )));
            }
            for place in 1..=fill_through {
                if !raw_arguments.contains_key(&place) {
                    raw_arguments.insert(place, self.build_elided_argument_for_place(place)?);
                }
            }
            let mut arguments = BTreeMap::new();
            for (place, argument) in raw_arguments {
                let mapped_place = mapped_place_for_generated_conversions(place, conversions)?;
                let key = argument_key(mapped_place);
                if arguments.insert(key.clone(), argument).is_some() {
                    return Err(invalid_graph(format!(
                        "multiple generated bridi arguments map to {key}"
                    )));
                }
            }
            let eventuality = self.build_generated_predication_eventuality(
                source_with_construct(predication_source.clone(), "distributed-predication"),
            )?;
            self.apply_generated_tagged_term_event_modifiers(eventuality, modal_terms)?;
            let modal_arguments = self
                .build_modal_arguments_for_generated_tagged_terms_for_event_with_predication_arguments(
                    eventuality,
                    modal_terms,
                    Some(&arguments),
                )?;
            let mut predication_object = SemanticObject::predication(
                relation.to_owned(),
                Some(eventuality),
                arguments,
                predication_mode_for_relation(relation, mode),
                source_with_construct(predication_source.clone(), "distributed-predication"),
                Vec::new(),
            );
            predication_object.set_predication_modal_arguments(modal_arguments);
            let predication = self.next_predication_id();
            self.insert(predication, predication_object)?;
            let formula = self.next_formula_id();
            self.insert(
                formula,
                SemanticObject::atom_formula(
                    predication,
                    source_with_construct(formula_source.clone(), "distributed-formula"),
                    Vec::new(),
                ),
            )?;
            self.wrap_formula_with_generated_argument_scopes(formula, argument.formula_scopes)?
        };
        if negated {
            formula = self.build_unary_formula(
                FormulaOperator::Not,
                formula,
                source_with_construct(formula_source, "distributed-negation"),
            )?;
        }
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_alternative_argument_for_sumti_branch<'syntax: 'tree>(
        &mut self,
        sumti: GeneratedDistributedSumtiBranch<'syntax>,
        negated: bool,
    ) -> Result<GeneratedAlternativeArgument<'syntax>, SemanticsError> {
        match sumti {
            GeneratedDistributedSumtiBranch::Sumti(sumti) => self
                .build_generated_alternative_argument_source(
                    GeneratedAlternativeArgumentSource::Sumti { sumti, negated },
                ),
            GeneratedDistributedSumtiBranch::SumtiGrouped(sumti) => {
                if sumti.grouped_tail.is_some() {
                    let referent = self.build_sumti_grouped_referent(sumti)?;
                    Ok(GeneratedAlternativeArgument {
                        argument: ArgumentValue::filled(referent, None),
                        negated,
                        formula_scopes: Vec::new(),
                    })
                } else {
                    self.build_generated_alternative_argument_for_sumti_branch(
                        GeneratedDistributedSumtiBranch::SumtiAfterthought(&sumti.leading_sumti),
                        negated,
                    )
                }
            }
            GeneratedDistributedSumtiBranch::SumtiAfterthought(sumti) => {
                if sumti.continuations.is_empty() {
                    self.build_generated_alternative_argument_for_sumti_branch(
                        GeneratedDistributedSumtiBranch::SumtiBound(&sumti.leading_sumti),
                        negated,
                    )
                } else {
                    let referent = self.build_sumti_afterthought_referent(sumti)?;
                    Ok(GeneratedAlternativeArgument {
                        argument: ArgumentValue::filled(referent, None),
                        negated,
                        formula_scopes: Vec::new(),
                    })
                }
            }
            GeneratedDistributedSumtiBranch::SumtiAfterthoughtPrefix(prefix) => {
                if prefix.continuation_count == 0 {
                    return self.build_generated_alternative_argument_for_sumti_branch(
                        GeneratedDistributedSumtiBranch::SumtiBound(&prefix.sumti.leading_sumti),
                        negated,
                    );
                }
                self.build_generated_alternative_argument_for_sumti_afterthought_prefix(
                    prefix, negated,
                )
            }
            GeneratedDistributedSumtiBranch::SumtiBound(sumti) => {
                self.build_generated_alternative_argument_for_sumti_bound(sumti, negated)
            }
            GeneratedDistributedSumtiBranch::SumtiForethought(sumti) => {
                self.build_generated_alternative_argument_for_sumti_forethought(sumti, negated)
            }
        }
    }

    #[requires(prefix.continuation_count > 0)]
    #[ensures(true)]
    pub(super) fn build_generated_alternative_argument_for_sumti_afterthought_prefix<
        'syntax: 'tree,
    >(
        &mut self,
        prefix: GeneratedSumtiAfterthoughtPrefix<'syntax>,
        negated: bool,
    ) -> Result<GeneratedAlternativeArgument<'syntax>, SemanticsError> {
        let leading = self.build_generated_alternative_argument_for_sumti_bound(
            &prefix.sumti.leading_sumti,
            false,
        )?;
        let mut referent = leading
            .argument
            .value
            .ok_or_else(|| unsupported("deleted operand in afterthought sumti prefix"))?;
        let mut formula_scopes = leading.formula_scopes;
        for continuation in prefix
            .sumti
            .continuations
            .iter()
            .take(prefix.continuation_count)
        {
            let trailing = self
                .build_generated_alternative_argument_for_sumti_bound(&continuation.sumti, false)?;
            let trailing_referent = trailing
                .argument
                .value
                .ok_or_else(|| unsupported("deleted operand in afterthought sumti prefix"))?;
            formula_scopes.extend(trailing.formula_scopes);
            referent = self.build_connected_generated_sumti_referent(
                prefix.sumti,
                referent,
                &continuation.connective,
                trailing_referent,
            )?;
        }
        Ok(GeneratedAlternativeArgument {
            argument: ArgumentValue::filled(referent, None),
            negated,
            formula_scopes,
        })
    }

    #[requires(place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|connective| connective.is_none_or(|_| !arguments.get(&place).is_none_or(|values| values.is_empty()))) || ret.is_err())]
    pub(super) fn insert_generated_sumti_distribution_alternatives<'syntax: 'tree>(
        &mut self,
        arguments: &mut BTreeMap<usize, Vec<GeneratedAlternativeArgumentSource<'syntax>>>,
        place: usize,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<Option<GeneratedDistributedSumtiConnective<'syntax>>, SemanticsError> {
        let Some(afterthought) = generated_sumti_afterthought_for_distribution(sumti) else {
            if let Some(bound) = generated_sumti_bound_for_distribution(sumti) {
                let tail = bound
                    .bound_tail
                    .as_ref()
                    .expect("bound distribution has tail");
                insert_generated_alternative_argument(
                    arguments,
                    place,
                    GeneratedAlternativeArgumentSource::SumtiForethought {
                        sumti: &bound.leading_sumti,
                        negated: generated_argument_connective_negates_left(
                            tail.connective.as_ref(),
                        ),
                    },
                )?;
                insert_generated_alternative_argument(
                    arguments,
                    place,
                    GeneratedAlternativeArgumentSource::SumtiBound {
                        sumti: &tail.trailing_sumti,
                        negated: generated_argument_connective_negates_right(
                            tail.connective.as_ref(),
                        ),
                    },
                )?;
                return Ok(Some(GeneratedDistributedSumtiConnective::Argument {
                    connective: tail.connective.as_ref(),
                    tense_modal: tail.tense_modal.as_deref(),
                    bo: true,
                }));
            }
            if let Some(forethought) = generated_sumti_forethought_for_distribution(sumti) {
                insert_generated_alternative_argument(
                    arguments,
                    place,
                    GeneratedAlternativeArgumentSource::Sumti {
                        sumti: &forethought.leading_sumti,
                        negated: generated_modal_forethought_connective_negates_left(
                            &forethought.gek,
                        ),
                    },
                )?;
                insert_generated_alternative_argument(
                    arguments,
                    place,
                    GeneratedAlternativeArgumentSource::SumtiForethought {
                        sumti: &forethought.first_branch.sumti,
                        negated: forethought.first_branch.gik.nai.is_some(),
                    },
                )?;
                return Ok(Some(GeneratedDistributedSumtiConnective::Forethought {
                    gek: &forethought.gek,
                    gik: &forethought.first_branch.gik,
                }));
            }
            let mut formula_scopes = Vec::new();
            let argument = self.build_argument_for_generated_sumti_with_formula_scopes(
                sumti,
                &mut formula_scopes,
            )?;
            insert_generated_alternative_argument(
                arguments,
                place,
                GeneratedAlternativeArgument {
                    argument,
                    negated: false,
                    formula_scopes,
                }
                .into(),
            )?;
            return Ok(None);
        };
        let [continuation] = afterthought.continuations.as_slice() else {
            return Err(unsupported(
                "multi-continuation generated sumti distribution",
            ));
        };
        insert_generated_alternative_argument(
            arguments,
            place,
            GeneratedAlternativeArgumentSource::SumtiBound {
                sumti: &afterthought.leading_sumti,
                negated: generated_argument_connective_negates_left(&continuation.connective),
            },
        )?;
        insert_generated_alternative_argument(
            arguments,
            place,
            GeneratedAlternativeArgumentSource::SumtiBound {
                sumti: &continuation.sumti,
                negated: generated_argument_connective_negates_right(&continuation.connective),
            },
        )?;
        Ok(Some(GeneratedDistributedSumtiConnective::Argument {
            connective: &continuation.connective,
            tense_modal: None,
            bo: false,
        }))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|arguments| arguments.values().all(|values| !values.is_empty())) || ret.is_err())]
    pub(super) fn prebuild_generated_alternative_arguments_by_place<'syntax: 'tree>(
        &mut self,
        alternatives: BTreeMap<usize, Vec<GeneratedAlternativeArgumentSource<'syntax>>>,
    ) -> Result<BTreeMap<usize, Vec<GeneratedAlternativeArgument<'syntax>>>, SemanticsError> {
        let mut prebuilt =
            BTreeMap::<usize, Vec<Option<GeneratedAlternativeArgument<'syntax>>>>::new();
        let mut slots = Vec::new();
        for (place, values) in alternatives {
            let len = values.len();
            if len == 0 {
                continue;
            }
            prebuilt.insert(place, std::iter::repeat_with(|| None).take(len).collect());
            for (index, source) in values.into_iter().enumerate() {
                let source_order =
                    self.source_order_for_generated_alternative_argument_source(&source);
                slots.push((source_order, place, index, source));
            }
        }
        slots.sort_by_key(|(source_order, place, index, _source)| (*source_order, *place, *index));
        for (_source_order, place, index, source) in slots {
            let value = self.build_generated_alternative_argument_source(source)?;
            let values = prebuilt.get_mut(&place).ok_or_else(|| {
                invalid_graph(format!(
                    "generated alternative argument prebuild lost place x{place}"
                ))
            })?;
            values[index] = Some(value);
        }

        let mut arguments = BTreeMap::new();
        for (place, values) in prebuilt {
            let mut built_values = Vec::with_capacity(values.len());
            for value in values {
                built_values.push(value.ok_or_else(|| {
                    invalid_graph(format!(
                        "generated alternative argument prebuild lost a value for x{place}"
                    ))
                })?);
            }
            arguments.insert(place, built_values);
        }
        Ok(arguments)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn source_order_for_generated_alternative_argument_source(
        &self,
        source: &GeneratedAlternativeArgumentSource<'_>,
    ) -> usize {
        match source {
            GeneratedAlternativeArgumentSource::Built(_) => usize::MAX - 3,
            GeneratedAlternativeArgumentSource::Sumti { sumti, .. } => {
                self.source_order_for_node(*sumti)
            }
            GeneratedAlternativeArgumentSource::SumtiForethought { sumti, .. } => {
                self.source_order_for_node(*sumti)
            }
            GeneratedAlternativeArgumentSource::SumtiBound { sumti, .. } => {
                self.source_order_for_node(*sumti)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_alternative_argument_source<'syntax: 'tree>(
        &mut self,
        source: GeneratedAlternativeArgumentSource<'syntax>,
    ) -> Result<GeneratedAlternativeArgument<'syntax>, SemanticsError> {
        match source {
            GeneratedAlternativeArgumentSource::Built(argument) => Ok(argument),
            GeneratedAlternativeArgumentSource::Sumti { sumti, negated } => {
                let mut formula_scopes = Vec::new();
                let argument = self.build_argument_for_generated_sumti_with_formula_scopes(
                    sumti,
                    &mut formula_scopes,
                )?;
                Ok(GeneratedAlternativeArgument {
                    argument,
                    negated,
                    formula_scopes,
                })
            }
            GeneratedAlternativeArgumentSource::SumtiForethought { sumti, negated } => {
                self.build_generated_alternative_argument_for_sumti_forethought(sumti, negated)
            }
            GeneratedAlternativeArgumentSource::SumtiBound { sumti, negated } => {
                self.build_generated_alternative_argument_for_sumti_bound(sumti, negated)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_alternative_argument_for_sumti_bound<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiBoundSyntax,
        negated: bool,
    ) -> Result<GeneratedAlternativeArgument<'syntax>, SemanticsError> {
        if generated_sumti_bound_spine_cmavo(sumti) == Some(Cmavo::Ziho) {
            return Ok(GeneratedAlternativeArgument {
                argument: ArgumentValue::deleted(
                    "zi'o".to_owned(),
                    self.source_for_node(sumti, "deleted-place"),
                ),
                negated,
                formula_scopes: Vec::new(),
            });
        }
        let mut formula_scopes = Vec::new();
        let scope_source = generated_argument_quantifier_source_from_sumti_bound(sumti)?;
        let referent = if scope_source.is_some() {
            if self
                .generated_requantified_da_source_for_sumti_bound(sumti, &formula_scopes)
                .is_some()
            {
                self.build_plain_scoped_argument_variable_for_generated_sumti_bound(sumti)?
            } else {
                self.build_scoped_argument_variable_for_generated_sumti_bound(sumti)?
            }
        } else {
            self.build_sumti_bound_referent(sumti)?
        };
        let selection =
            self.generated_requantified_da_source_for_sumti_bound(sumti, &formula_scopes);
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
        let mut argument = if generated_sumti_bound_spine_cmavo(sumti) == Some(Cmavo::Zohe) {
            ArgumentValue::elided(
                referent,
                "zo'e".to_owned(),
                self.source_for_node(sumti, "elided-place"),
            )
        } else {
            ArgumentValue::filled(referent, None)
        };
        if generated_sumti_bound_spine_cmavo(sumti) == Some(Cmavo::Ko) {
            argument = argument.with_command_target(CommandTarget::new("ko".to_owned()));
        }
        let mut relative_clause_restrictions = Vec::new();
        if let Some(relative_clauses) = generated_sumti_bound_relative_clause_list(sumti) {
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
        if let Some(scope_source) = scope_source {
            formula_scopes.push(GeneratedArgumentQuantifierScope {
                node: GeneratedArgumentQuantifierScopeNode::SumtiBound(sumti),
                source: scope_source,
                variable: referent,
                source_variable,
                selection_source,
                source_restriction_nodes,
                source_restriction_formulas,
                inherited_restrictions,
                relative_clause_restrictions,
            });
        }
        Ok(GeneratedAlternativeArgument {
            argument,
            negated,
            formula_scopes,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_alternative_argument_for_sumti_forethought<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiForethoughtSyntax,
        negated: bool,
    ) -> Result<GeneratedAlternativeArgument<'syntax>, SemanticsError> {
        let referent = self.build_sumti_forethought_referent(sumti)?;
        let mut argument = ArgumentValue::filled(referent, None);
        if let SumtiForethoughtSyntax::SimpleSumti(simple) = sumti {
            if let Some(relative_clauses) = &simple.relative_clauses {
                let relative_clauses =
                    self.lower_generated_relative_clause_list(relative_clauses, referent)?;
                if !relative_clauses.is_empty() {
                    argument = argument.with_relative_clauses(relative_clauses);
                }
            }
        }
        Ok(GeneratedAlternativeArgument {
            argument,
            negated,
            formula_scopes: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_scoped_argument_variable_for_generated_sumti_bound(
        &mut self,
        sumti: &'tree SumtiBoundSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(pro_sumti) = generated_quantified_da_series_pro_sumti_from_sumti_bound(sumti) {
            return self.build_scoped_generated_pro_sumti_variable(
                pro_sumti,
                generated_sumti_bound_variable_sort(sumti),
            );
        }
        let sort = generated_sumti_bound_variable_sort(sumti);
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
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_plain_scoped_argument_variable_for_generated_sumti_bound(
        &mut self,
        sumti: &'tree SumtiBoundSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sort = generated_sumti_bound_variable_sort(sumti);
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
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_connective_question_parameter_for_argument_connective(
        &mut self,
        connective: &'tree ArgumentConnectiveSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(token) = generated_argument_connective_question_token(connective) else {
            return Ok(None);
        };
        self.build_generated_connective_question_parameter_for_token(&token)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_connective_question_parameter_for_modal_forethought_connective(
        &mut self,
        connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(token) = generated_modal_forethought_connective_question_token(connective) else {
            return Ok(None);
        };
        self.build_generated_connective_question_parameter_for_token(&token)
    }

    #[requires(token.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Ji | Cmavo::Gehi | Cmavo::Gihi | Cmavo::Guhi | Cmavo::Jehi)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_connective_question_parameter_for_token(
        &mut self,
        token: &Token,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Connective,
                ParameterRole::ConnectiveQuestion,
                token_text(&token),
                self.source_for_token(&token, "parameter"),
            ),
        )?;
        if token_has_indicator_cmavo(&token, Cmavo::Kau)
            && self.record_generated_indirect_question_focus(
                GeneratedIndirectQuestionFocus::from_data(data!(GeneratedIndirectQuestionFocus {
                    focus: parameter,
                    presupposed_answer: None,
                    slots: vec![new!(QuestionSlot {
                        parameter,
                        role: QuestionSlotRole::Answer,
                    })],
                    kind: QuestionKind::Connective,
                    domain: SemanticSort::Connective,
                    source: self.source_for_token(&token, "indirect-question"),
                })),
            )
        {
            return Ok(Some(parameter));
        }
        self.connective_question_parameters.push(parameter);
        Ok(Some(parameter))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_connective_question_parameter_for_distributed_sumti_connective(
        &mut self,
        connective: GeneratedDistributedSumtiConnective<'tree>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match connective {
            GeneratedDistributedSumtiConnective::Argument { connective, .. } => self
                .build_generated_connective_question_parameter_for_argument_connective(connective),
            GeneratedDistributedSumtiConnective::Forethought { gek, .. } => self
                .build_generated_connective_question_parameter_for_modal_forethought_connective(
                    gek,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_connective_question_parameter_for_distributed_sumti_connective_option(
        &mut self,
        connective: Option<GeneratedDistributedSumtiConnective<'tree>>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match connective {
            Some(connective) => self
                .build_generated_connective_question_parameter_for_distributed_sumti_connective(
                    connective,
                ),
            None => Ok(None),
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_question_formula_for_terms(
        &mut self,
        question: GeneratedRelationQuestionSyntax<'_>,
        terms: Vec<&'tree TermSyntax>,
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
            None => self.build_generated_predication_eventuality(source.clone())?,
        };
        let assignments = self.build_term_assignments_for_terms(terms, first_visible_place)?;
        self.apply_generated_tagged_term_event_modifiers(eventuality, &assignments.modal_terms)?;
        let modal_arguments = self.build_modal_arguments_for_generated_tagged_terms_for_event(
            eventuality,
            &assignments.modal_terms,
        )?;
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
        object.set_predication_modal_arguments(modal_arguments);
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
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
    pub(super) fn build_relation_variable_formula_for_terms(
        &mut self,
        relation_variable: GeneratedRelationParameterSyntax<'_>,
        terms: Vec<&'tree TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (parameter, prenex_bound) = self
            .build_relation_variable_parameter_for_generated_relation_parameter_syntax(
                relation_variable,
            )?;
        let atom = self.build_relation_parameter_atom_formula_for_terms(
            parameter,
            terms,
            first_visible_place,
            eventuality,
            mode,
            source.clone(),
        )?;
        if prenex_bound {
            self.record_generated_implicit_existential_once(
                parameter,
                self.source_for_relation_parameter_syntax(relation_variable, "quantifier-scope"),
            );
            return Ok(atom);
        }
        let scoped = self.next_formula_id();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                FormulaOperator::Exists,
                parameter,
                None,
                atom,
                None,
                self.source_for_relation_parameter_syntax(relation_variable, "quantifier-scope"),
                Vec::new(),
            ),
        )?;
        Ok(scoped)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_unspecified_relation_formula_for_terms(
        &mut self,
        unspecified_relation: GeneratedRelationParameterSyntax<'_>,
        terms: Vec<&'tree TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self
            .build_unspecified_relation_parameter_for_generated_relation_parameter_syntax(
                unspecified_relation,
            )?;
        self.build_relation_parameter_atom_formula_for_terms(
            parameter,
            terms,
            first_visible_place,
            eventuality,
            mode,
            source,
        )
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_parameter_atom_formula_for_terms(
        &mut self,
        parameter: SemanticObjectId,
        terms: Vec<&'tree TermSyntax>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(source.clone())?,
        };
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
        let assignments = self.with_temporal_context(eventuality, |builder| {
            builder.build_term_assignments_for_terms(terms, first_visible_place)
        })?;
        let modal_arguments = self.build_modal_arguments_for_generated_tagged_terms_for_event(
            eventuality,
            &assignments.modal_terms,
        )?;
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
        object.set_predication_modal_arguments(modal_arguments);
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(id, _)| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_relation_variable_parameter_for_generated_relation_parameter_syntax(
        &mut self,
        relation_variable: GeneratedRelationParameterSyntax<'_>,
    ) -> Result<(SemanticObjectId, bool), SemanticsError> {
        let word = token_text(generated_relation_parameter_token(relation_variable));
        if let Some(parameter) = self
            .prenex_relation_variable_bindings
            .get(&word)
            .and_then(|bindings| bindings.last())
            .map(|binding| binding.parameter)
        {
            return Ok((parameter, true));
        }
        let key = self
            .source_key_for_relation_parameter_syntax(relation_variable)
            .ok_or_else(|| {
                invalid_graph("missing generated relation-variable source".to_owned())
            })?;
        if let Some(parameter) = self.relation_variable_parameters.get(&key).copied() {
            return Ok((parameter, false));
        }
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Relation,
                ParameterRole::RelationVariable,
                word,
                self.source_for_relation_parameter_syntax(relation_variable, "parameter"),
            ),
        )?;
        self.relation_variable_parameters.insert(key, parameter);
        Ok((parameter, false))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter) || ret.is_err())]
    pub(super) fn build_unspecified_relation_parameter_for_generated_relation_parameter_syntax(
        &mut self,
        unspecified_relation: GeneratedRelationParameterSyntax<'_>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Relation,
                ParameterRole::UnspecifiedRelation,
                token_text(generated_relation_parameter_token(unspecified_relation)),
                self.source_for_relation_parameter_syntax(unspecified_relation, "parameter"),
            ),
        )?;
        Ok(parameter)
    }
}
