use super::*;

#[requires(!markers.is_empty())]
#[ensures(ret.is_ok())]
fn generated_paragraph_relation(markers: &Vec1<Token>) -> Result<SequenceRelation, SemanticsError> {
    let mut transitions = markers.iter().map(|marker| match marker.cmavo() {
        Some(Cmavo::Niho) => Ok(ParagraphTransition::NewTopic),
        Some(Cmavo::Nohi) => Ok(ParagraphTransition::ResumePriorTopic),
        _ => Err(invalid_graph(
            "NIhO paragraph boundary contains a non-NIhO token".to_owned(),
        )),
    });
    let transition = transitions
        .next()
        .expect("precondition guarantees a first paragraph marker")?;
    let additional = transitions.collect::<Result<Vec<_>, _>>()?;
    Ok(SequenceRelation::ParagraphBoundary {
        transition,
        additional,
    })
}

#[requires(content.is_none_or(|content| crate::model::argument_object_kind_can_fill(content.object_kind())))]
#[ensures(ret.content == content)]
fn generated_fragment_semantics(
    content: Option<SemanticObjectId>,
    source: Option<crate::model::SemanticSource>,
) -> GeneratedFragmentSemantics {
    new!(GeneratedFragmentSemantics { content, source })
}

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    pub(super) fn build_generated_fragment_utterance(
        &mut self,
        utterance_id: SemanticObjectId,
        fragment: GeneratedFragmentRoot<'tree>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let question_start = self.direct_question_slots.len();
        let previous_asides = std::mem::take(&mut self.pending_asides);
        let semantics = self.build_generated_fragment_semantics(fragment);
        let asides = std::mem::replace(&mut self.pending_asides, previous_asides);
        let data!(GeneratedFragmentSemantics { content, source }) = semantics?.into_data();
        let question_slots = self.direct_question_slots.split_off(question_start);
        let (force, content) = if !question_slots.is_empty()
            && let Some(formula) = content
            && formula.object_kind() == crate::model::SemanticObjectKind::Formula
        {
            (
                UtteranceForce::Ask,
                Some(self.build_generated_direct_question(
                    formula,
                    question_slots,
                    source.clone(),
                )?),
            )
        } else {
            (UtteranceForce::Mention, content)
        };
        self.insert_generated_utterance(utterance_id, force, content, source)?;
        if content.is_none() {
            let object = self.objects.get_mut(&utterance_id).ok_or_else(|| {
                invalid_graph(format!(
                    "missing generated fragment utterance {utterance_id}"
                ))
            })?;
            object.push_diagnostic(diagnostic("fragment has no truth-bearing semantic formula"));
        }
        self.add_generated_utterance_asides(utterance_id, asides);
        Ok(utterance_id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|semantics| semantics.content.is_none_or(|content| crate::model::argument_object_kind_can_fill(content.object_kind()))) || ret.is_err())]
    pub(super) fn build_generated_fragment_semantics(
        &mut self,
        fragment: GeneratedFragmentRoot<'tree>,
    ) -> Result<GeneratedFragmentSemantics, SemanticsError> {
        let semantics = match fragment {
            GeneratedFragmentRoot::Prenex(fragment) => {
                let text = token_list_text(self.tokens_for_node(fragment).iter());
                return Err(requires_discourse_context(&format!(
                    "the following scope of prenex fragment `{text}`"
                )));
            }
            GeneratedFragmentRoot::Selbri(fragment) => {
                let source = self.source_for_node(fragment, "fragment");
                let content = self.build_generated_math_operator_property_abstraction_for_selbri(
                    fragment.0.as_ref(),
                    source.clone(),
                )?;
                generated_fragment_semantics(Some(content), source)
            }
            GeneratedFragmentRoot::Ek(fragment) => generated_fragment_semantics(
                Some(self.build_generated_connective_fragment_sign(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::Gihek(fragment) => generated_fragment_semantics(
                Some(self.build_generated_connective_fragment_sign(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::MultipleNa(fragment) => generated_fragment_semantics(
                Some(self.build_generated_connective_fragment_sign(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::SingleNa(fragment) => generated_fragment_semantics(
                Some(self.build_generated_connective_fragment_sign(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::Terms(fragment) => generated_fragment_semantics(
                self.build_terms_fragment_content(fragment)?,
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::Mekso(fragment) => generated_fragment_semantics(
                Some(self.build_mekso_fragment_referent(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::RelativeClause(fragment) => generated_fragment_semantics(
                Some(self.build_relative_clause_fragment_property(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
            GeneratedFragmentRoot::LinkedSumtiContinuation(fragment) => {
                let text = token_list_text(self.tokens_for_node(fragment).iter());
                return Err(requires_discourse_context(&format!(
                    "the omitted linked-argument head of BEI fragment `{text}`"
                )));
            }
            GeneratedFragmentRoot::LinkedSumti(fragment) => {
                let text = token_list_text(self.tokens_for_node(fragment).iter());
                return Err(requires_discourse_context(&format!(
                    "the omitted relation head of BE fragment `{text}`"
                )));
            }
            GeneratedFragmentRoot::ZantufaMekso(fragment) => generated_fragment_semantics(
                Some(self.build_zantufa_mekso_fragment_referent(fragment)?),
                self.source_for_node(fragment, "fragment"),
            ),
        };
        Ok(semantics)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_mekso_fragment_referent(
        &mut self,
        fragment: &'tree MeksoFragmentSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let words = quantifier_tokens(fragment.0.as_ref());
        if words.is_empty() {
            return Err(invalid_graph(
                "generated mekso fragment has no expression tokens".to_owned(),
            ));
        }
        let text = token_list_text(words.iter());
        let quantity = self.build_quantity_for_quantifier(fragment.0.as_ref())?;
        let id = self.next_referent_with_sort_id(SemanticSort::Number);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Number,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::Number,
                    word: "mex".to_owned(),
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
                self.source_for_node(fragment, "number-fragment"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Relation)) || ret.is_err())]
    pub(super) fn build_relative_clause_fragment_property(
        &mut self,
        fragment: &'tree RelativeClauseFragmentSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let source = self.source_for_node(fragment, "relative-clause-fragment");
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::RelativeClauseHead,
                "ke'a".to_owned(),
                source.clone(),
            ),
        )?;
        let clauses = self.lower_generated_relative_clause_list(&fragment.0, parameter)?;
        let bodies = clauses.into_iter().map(|clause| clause.body).collect();
        let Some(body) = self.combine_generated_restriction_formulas(bodies)? else {
            return Err(requires_discourse_context(
                "a relative-clause fragment whose association does not assert a property",
            ));
        };
        self.build_property_abstraction_output(body, vec![parameter], source)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(object.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(ret.is_err() || self.objects.contains_key(&predication))]
    pub(super) fn insert_converted_predication_with_voha_place_map(
        &mut self,
        predication: SemanticObjectId,
        object: SemanticObject,
        mut mapper: impl FnMut(usize) -> Result<usize, SemanticsError>,
    ) -> Result<(), SemanticsError> {
        let needs_place_map = object.as_predication().is_some_and(|node| {
            node.arguments
                .values()
                .chain(
                    node.adjuncts
                        .iter()
                        .flat_map(|modal| modal.arguments.values()),
                )
                .filter_map(|argument| argument.value)
                .any(|value| self.pending_voha_places.contains_key(&value))
        });
        let outer_place_map = self.active_voha_place_maps.last().copied();
        let place_map = needs_place_map
            .then(|| {
                GeneratedVohaPlaceMap::try_from_mapper(|surface_place| {
                    let place = outer_place_map
                        .as_ref()
                        .map(|mapping| mapping.underlying_place(surface_place))
                        .unwrap_or(surface_place);
                    mapper(place)
                })
            })
            .transpose()?;
        self.insert(predication, object)?;
        if let Some(place_map) = place_map {
            self.pending_voha_place_maps.insert(predication, place_map);
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires((1..=5).contains(&surface_place))]
    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Referent || target.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.is_err() || self.pending_voha_direct_targets.get(&(predication, surface_place)) == Some(&target))]
    pub(super) fn record_voha_direct_target(
        &mut self,
        predication: SemanticObjectId,
        surface_place: usize,
        target: SemanticObjectId,
    ) -> Result<(), SemanticsError> {
        if let Some(existing) = self
            .pending_voha_direct_targets
            .insert((predication, surface_place), target)
            && existing != target
        {
            return Err(invalid_graph(format!(
                "surface x{surface_place} of {predication} has conflicting direct vo'a targets"
            )));
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(self.active_voha_place_maps.len() == old(self.active_voha_place_maps.len()))]
    pub(super) fn with_voha_place_map<R>(
        &mut self,
        mut mapper: impl FnMut(usize) -> Result<usize, SemanticsError>,
        build: impl FnOnce(&mut Self) -> Result<R, SemanticsError>,
    ) -> Result<R, SemanticsError> {
        let outer_place_map = self.active_voha_place_maps.last().copied();
        let place_map = GeneratedVohaPlaceMap::try_from_mapper(|surface_place| {
            let place = outer_place_map
                .as_ref()
                .map(|mapping| mapping.underlying_place(surface_place))
                .unwrap_or(surface_place);
            mapper(place)
        })?;
        self.active_voha_place_maps.push(place_map);
        let result = build(self);
        self.active_voha_place_maps
            .pop()
            .expect("vo'a place-map context was just pushed");
        result
    }

    /// Resolve vo'a-series (CLL 7.8) placeholders that could not be resolved inline. Each
    /// placeholder refers to a place of the bridi it appears in; by the time the whole graph is
    /// built, that place has been filled (an explicit term, an elided `zo'e`, a relative-clause
    /// `ke'a` head, a description's `ce'u` slot, an abstraction subject). Point each placeholder's
    /// argument at that filler so the vo'a-series shares its referent, as the same-bridi control
    /// (`su'o lo mlatu cu terpa vo'a`) already does inline. Runs before `prune_unreachable_objects`,
    /// so a placeholder that is no longer referenced afterwards is pruned.
    #[requires(true)]
    #[ensures(self.pending_voha_places.is_empty() && self.pending_voha_place_maps.is_empty() && self.pending_voha_direct_targets.is_empty())]
    fn resolve_pending_voha_references(&mut self) {
        if self.pending_voha_places.is_empty() {
            self.pending_voha_place_maps.clear();
            self.pending_voha_direct_targets.clear();
            return;
        }
        let mut updates = Vec::new();
        for (id, object) in &self.objects {
            let Some(predication) = object.as_predication() else {
                continue;
            };
            for (place, argument) in &predication.arguments {
                let Some(value) = argument.value else {
                    continue;
                };
                if !self.pending_voha_places.contains_key(&value) {
                    continue;
                }
                if let Some(resolved) = resolve_voha_placeholder(
                    &predication.arguments,
                    *id,
                    value,
                    &self.pending_voha_places,
                    self.pending_voha_place_maps.get(id),
                    &self.pending_voha_direct_targets,
                    &mut BTreeSet::new(),
                ) && resolved != value
                {
                    updates.push(new!(GeneratedVohaUpdate {
                        predication: *id,
                        location: new!(GeneratedVohaUpdateLocation::Numbered { place: *place }),
                        resolved,
                    }));
                }
            }
            for (modal_index, modal) in predication.adjuncts.iter().enumerate() {
                for (place, argument) in &modal.arguments {
                    let Some(value) = argument.value else {
                        continue;
                    };
                    if !self.pending_voha_places.contains_key(&value) {
                        continue;
                    }
                    if let Some(resolved) = resolve_voha_placeholder(
                        &predication.arguments,
                        *id,
                        value,
                        &self.pending_voha_places,
                        self.pending_voha_place_maps.get(id),
                        &self.pending_voha_direct_targets,
                        &mut BTreeSet::new(),
                    ) && resolved != value
                    {
                        updates.push(new!(GeneratedVohaUpdate {
                            predication: *id,
                            location: new!(GeneratedVohaUpdateLocation::Modal {
                                modal_index,
                                place: *place,
                            }),
                            resolved,
                        }));
                    }
                }
            }
        }
        for update in updates {
            if let Some(object) = self.objects.get_mut(&update.predication) {
                object.update_predication(|node| {
                    let data = node.into_data();
                    match update.location.as_data() {
                        data!(GeneratedVohaUpdateLocation::Numbered { place }) => {
                            let mut arguments = data.arguments;
                            if let Some(argument) = arguments.get(place).cloned() {
                                arguments.insert(
                                    *place,
                                    argument.with_data(data! { value: Some(update.resolved) }),
                                );
                            }
                            PredicationNode::from_data(data!(PredicationNode { arguments, ..data }))
                        }
                        data!(GeneratedVohaUpdateLocation::Modal { modal_index, place }) => {
                            let mut adjuncts = data.adjuncts;
                            if let Some(modal) = adjuncts.get(*modal_index).cloned() {
                                let modal_data = modal.into_data();
                                let mut arguments = modal_data.arguments;
                                if let Some(argument) = arguments.get(place).cloned() {
                                    arguments.insert(
                                        *place,
                                        argument.with_data(data! { value: Some(update.resolved) }),
                                    );
                                }
                                adjuncts[*modal_index] = Adjunct::from_data(data!(Adjunct {
                                    arguments,
                                    ..modal_data
                                }));
                            }
                            PredicationNode::from_data(data!(PredicationNode { adjuncts, ..data }))
                        }
                    }
                });
            }
        }
        self.pending_voha_places.clear();
        self.pending_voha_place_maps.clear();
        self.pending_voha_direct_targets.clear();
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_text(
        mut self,
        syntax: &'tree TextSyntax,
    ) -> Result<SemanticGraph, SemanticsError> {
        let plan = generated_text_plan_from_text(syntax)?;
        let items = self.build_generated_text_plan_items(plan)?;
        let root = if let [single] = items.as_slice() {
            *single
        } else if items.is_empty() {
            let sequence = self.next_sequence_id();
            self.insert(
                sequence,
                SemanticObject::sequence(
                    Vec::new(),
                    SequenceRelation::SameTopicContinuation,
                    self.source_for_node(syntax, "text"),
                    Vec::new(),
                ),
            )?;
            sequence
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
        self.resolve_pending_voha_references();
        self.prune_unreachable_objects(root);
        if let Some(error) =
            crate::model::semantic_object_question_slots_validation_error(&self.objects)
        {
            return Err(invalid_graph(error));
        }
        SemanticGraph::new(root, self.objects).map_err(|message| SemanticsError {
            kind: SemanticsErrorKind::InvalidGraph,
            message: format!("semantic graph invariant failed: {message}"),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_generated_text_plan_items(
        &mut self,
        plan: GeneratedTextPlan<'tree>,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut items = Vec::new();
        let mut paragraph_boundaries = Vec::new();
        let mut leading_asides =
            self.build_generated_vocative_asides_from_refs(&plan.leading_free_modifiers)?;
        let truth_question = plan.leading_indicators.iter().any(|indicator| {
            let parts = indicator_parts_for_leading_indicator(indicator);
            parts.iter().any(|part| part.cmavo == Cmavo::Xu)
                && !parts.iter().any(|part| part.cmavo == Cmavo::Kau)
        });
        if !plan.leading_nai.is_empty() {
            items.push(self.build_generated_leading_nai_utterance(plan.leading_nai)?);
        }
        if !plan.leading_cmevla.is_empty() {
            items.push(self.build_generated_leading_cmevla_utterance(plan.leading_cmevla)?);
        }

        let utterance_ids = plan
            .items
            .iter()
            .filter_map(|item| match item {
                GeneratedTextPlanItem::Root { root, .. }
                    if generated_text_root_is_utterance(root) =>
                {
                    Some(self.next_utterance_id())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut utterance_index = 0;
        let mut leading_indicators_attached = false;
        let mut truth_question_pending = truth_question;
        for item in plan.items {
            match item {
                GeneratedTextPlanItem::Root {
                    root,
                    free_modifiers,
                    separator_i,
                } if generated_text_root_is_utterance(&root) => {
                    let separator_truth_question = separator_i.is_some_and(|separator| {
                        let parts = indicator_parts_for_token(separator);
                        parts.iter().any(|part| part.cmavo == Cmavo::Xu)
                            && !parts.iter().any(|part| part.cmavo == Cmavo::Kau)
                    });
                    let statement_truth_question =
                        truth_question_pending || separator_truth_question;
                    let mut asides =
                        self.build_generated_vocative_asides_from_refs(&free_modifiers)?;
                    let utterance_id = utterance_ids[utterance_index];
                    self.previous_utterance = utterance_index
                        .checked_sub(1)
                        .and_then(|previous| utterance_ids.get(previous).copied());
                    self.current_utterance = Some(utterance_id);
                    self.next_utterance = utterance_ids.get(utterance_index + 1).copied();
                    utterance_index += 1;
                    let item = self.build_utterance_for_generated_text_root(
                        utterance_id,
                        root,
                        statement_truth_question,
                    )?;
                    if let Some(separator_i) = separator_i {
                        self.attach_generated_statement_separator_indicators_to_discourse_item(
                            item,
                            separator_i,
                            None,
                            false,
                        )?;
                    }
                    if !leading_indicators_attached {
                        self.attach_generated_leading_indicators_to_discourse_item(
                            item,
                            plan.leading_indicators,
                            statement_truth_question,
                        )?;
                        leading_indicators_attached = true;
                    }
                    self.add_asides_to_generated_discourse_item(item, std::mem::take(&mut asides));
                    self.attach_generated_statement_reciprocity_to_discourse_item(
                        item,
                        &free_modifiers,
                    )?;
                    items.push(item);
                    if statement_truth_question {
                        truth_question_pending = false;
                    }
                }
                GeneratedTextPlanItem::Root {
                    root,
                    free_modifiers,
                    separator_i,
                } => {
                    let mut asides =
                        self.build_generated_vocative_asides_from_refs(&free_modifiers)?;
                    let item = self.build_discourse_item_for_generated_text_root(root)?;
                    if truth_question_pending {
                        self.add_generated_truth_question_to_discourse_item(item, 0)?;
                        truth_question_pending = false;
                    }
                    if let Some(separator_i) = separator_i {
                        self.attach_generated_statement_separator_indicators_to_discourse_item(
                            item,
                            separator_i,
                            None,
                            false,
                        )?;
                    }
                    self.add_asides_to_generated_discourse_item(item, std::mem::take(&mut asides));
                    self.attach_generated_statement_reciprocity_to_discourse_item(
                        item,
                        &free_modifiers,
                    )?;
                    items.push(item);
                }
                GeneratedTextPlanItem::StandaloneFreeModifiers(free_modifiers) => {
                    let asides = self.build_generated_vocative_asides_from_refs(&free_modifiers)?;
                    if let Some(item) = self.build_generated_standalone_asides(asides)? {
                        items.push(item);
                    }
                }
                GeneratedTextPlanItem::ParagraphBoundary { markers } => {
                    paragraph_boundaries.push(GeneratedBuiltParagraphBoundary {
                        item_index: items.len(),
                        markers,
                    });
                }
                GeneratedTextPlanItem::StandaloneParagraphBoundary {
                    markers,
                    free_modifiers,
                } => {
                    let relation = generated_paragraph_relation(markers)?;
                    let boundary = self.next_sequence_id();
                    self.insert(
                        boundary,
                        SemanticObject::sequence(
                            Vec::new(),
                            relation,
                            self.source_for_tokens(markers.as_slice(), "paragraph-boundary"),
                            Vec::new(),
                        ),
                    )?;
                    items.push(boundary);
                    let asides = self.build_generated_vocative_asides_from_refs(&free_modifiers)?;
                    if let Some(item) = self.build_generated_standalone_asides(asides)? {
                        items.push(item);
                    }
                }
                GeneratedTextPlanItem::PendingStatementConnection { i, connective } => {
                    let Some(item) = items.pop() else {
                        return Err(invalid_graph(
                            "pending statement connection has no preceding discourse item"
                                .to_owned(),
                        ));
                    };
                    items.push(self.wrap_generated_pending_statement_connection_item(
                        item,
                        i,
                        connective,
                        ElidedConnectionOperand::FollowingDiscourse,
                    )?);
                }
                GeneratedTextPlanItem::TrailingSeparator { i, free_modifiers } => {
                    let asides = self.build_generated_vocative_asides_from_refs(&free_modifiers)?;
                    if let Some(item) = items.last().copied() {
                        self.attach_generated_statement_separator_indicators_to_discourse_item(
                            item, i, None, true,
                        )?;
                        self.add_asides_to_generated_discourse_item(item, asides);
                    } else if let Some(item) = self.build_generated_standalone_asides(asides)? {
                        self.attach_generated_statement_separator_indicators_to_discourse_item(
                            item, i, None, true,
                        )?;
                        items.push(item);
                    }
                }
            }
        }

        for leading_i in plan.leading_i_statements.iter().rev() {
            if leading_i.connective.is_none() {
                if let Some(first_item) = items.first().copied() {
                    for source_order in self.direct_truth_question_source_orders(&leading_i.i) {
                        self.add_generated_truth_question_to_discourse_item(
                            first_item,
                            source_order,
                        )?;
                    }
                    self.attach_generated_statement_separator_indicators_to_discourse_item(
                        first_item,
                        &leading_i.i,
                        None,
                        false,
                    )?;
                }
                continue;
            }
            if let Some(first_item) = items.first_mut() {
                *first_item = self
                    .wrap_generated_leading_i_statement_connection_item(*first_item, leading_i)?;
            } else if let Some(connective) = leading_i.connective.as_deref() {
                items.push(self.build_generated_standalone_connective_utterance(connective)?);
            }
        }

        if let Some(connective) = plan.leading_connective {
            if let Some(first_item) = items.first_mut() {
                *first_item =
                    self.wrap_generated_text_leading_connective_item(*first_item, connective)?;
            } else {
                items.push(self.build_generated_standalone_connective_utterance(connective)?);
            }
        }

        if items.is_empty()
            && !leading_asides.is_empty()
            && let Some(item) =
                self.build_generated_standalone_asides(std::mem::take(&mut leading_asides))?
        {
            items.push(item);
        }

        if items.is_empty() && !plan.leading_indicators.is_empty() {
            items.push(
                self.build_generated_standalone_indicator_utterance(plan.leading_indicators)?,
            );
            leading_indicators_attached = true;
        }

        if let Some(first_item) = items.first().copied() {
            self.add_asides_to_generated_discourse_item(first_item, leading_asides);
            self.attach_generated_statement_reciprocity_to_discourse_item(
                first_item,
                &plan.leading_free_modifiers,
            )?;
            if !leading_indicators_attached && !plan.leading_indicators.is_empty() {
                self.attach_generated_leading_indicators_to_discourse_item(
                    first_item,
                    plan.leading_indicators,
                    truth_question,
                )?;
            }
        } else if let Some(item) = self.build_generated_standalone_asides(leading_asides)? {
            items.push(item);
        }

        items = self.group_generated_paragraph_items(items, &paragraph_boundaries)?;

        self.previous_utterance = None;
        self.current_utterance = None;
        self.next_utterance = None;
        Ok(items)
    }

    #[requires(items.iter().all(|item| matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent)))]
    #[requires(boundaries.iter().all(|boundary| boundary.item_index <= items.len()))]
    #[requires(boundaries.windows(2).all(|pair| pair[0].item_index <= pair[1].item_index))]
    #[ensures(ret.as_ref().is_ok_and(|result| (boundaries.is_empty() || result.len() <= 1) && result.iter().all(|item| matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent))) || ret.is_err())]
    fn group_generated_paragraph_items(
        &mut self,
        items: Vec<SemanticObjectId>,
        boundaries: &[GeneratedBuiltParagraphBoundary<'tree>],
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let Some(first_boundary) = boundaries.first() else {
            return Ok(items);
        };

        let mut grouped =
            self.collapse_generated_paragraph_items(&items[..first_boundary.item_index])?;
        for (boundary_index, boundary) in boundaries.iter().enumerate() {
            let following_item_index = boundaries
                .get(boundary_index + 1)
                .map_or(items.len(), |following| following.item_index);
            let paragraph = self.collapse_generated_paragraph_items(
                &items[boundary.item_index..following_item_index],
            )?;
            let mut boundary_items = Vec::with_capacity(2);
            boundary_items.extend(grouped);
            boundary_items.extend(paragraph);

            let boundary_id = self.next_sequence_id();
            self.insert(
                boundary_id,
                SemanticObject::sequence(
                    boundary_items,
                    generated_paragraph_relation(boundary.markers)?,
                    self.source_for_tokens(boundary.markers.as_slice(), "paragraph-boundary"),
                    Vec::new(),
                ),
            )?;
            grouped = Some(boundary_id);
        }
        Ok(grouped.into_iter().collect())
    }

    #[requires(items.iter().all(|item| matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent)))]
    #[ensures(ret.as_ref().is_ok_and(|item| item.is_none_or(|item| matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent))) || ret.is_err())]
    fn collapse_generated_paragraph_items(
        &mut self,
        items: &[SemanticObjectId],
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match items {
            [] => Ok(None),
            [item] => Ok(Some(*item)),
            _ => {
                let paragraph = self.next_sequence_id();
                self.insert(
                    paragraph,
                    SemanticObject::sequence(
                        items.to_vec(),
                        SequenceRelation::SameTopicContinuation,
                        self.source_for_generated_discourse_items(items, "paragraph"),
                        Vec::new(),
                    ),
                )?;
                Ok(Some(paragraph))
            }
        }
    }

    #[requires(!items.is_empty())]
    #[requires(!construct.is_empty())]
    #[ensures(ret.as_ref().is_none_or(|source| source.construct.as_deref() == Some(construct)))]
    fn source_for_generated_discourse_items(
        &self,
        items: &[SemanticObjectId],
        construct: &str,
    ) -> Option<crate::model::SemanticSource> {
        let mut sources = items
            .iter()
            .filter_map(|item| self.objects.get(item))
            .filter_map(SemanticObject::source);
        let first = sources.next()?;
        let mut byte_start = first.span.byte_start;
        let mut byte_end = first.span.byte_end;
        for source in sources {
            byte_start = byte_start.min(source.span.byte_start);
            byte_end = byte_end.max(source.span.byte_end);
        }
        let text = self
            .options
            .source_text
            .and_then(|text| text.get(byte_start..byte_end))
            .map(str::to_owned);
        Some(crate::model::SemanticSource {
            span: SourceByteSpan {
                byte_start,
                byte_end,
            },
            text,
            construct: Some(construct.to_owned()),
        })
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    pub(super) fn build_utterance_for_generated_text_root(
        &mut self,
        utterance_id: SemanticObjectId,
        root: GeneratedTextRoot<'tree>,
        truth_question: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match root {
            GeneratedTextRoot::Bridi(bridi) => self
                .build_bridi_utterance_with_force(
                    utterance_id,
                    bridi,
                    generated_bridi_force(bridi, truth_question),
                )
                .map(|(utterance, _formula)| utterance),
            GeneratedTextRoot::Fragment(fragment) => {
                let item = self.build_generated_fragment_utterance(utterance_id, fragment)?;
                if truth_question {
                    self.add_generated_truth_question_to_discourse_item(item, 0)?;
                }
                Ok(item)
            }
            GeneratedTextRoot::PrenexStatement(statement) => self
                .build_utterance_for_generated_prenex_statement(
                    utterance_id,
                    statement,
                    truth_question,
                ),
            GeneratedTextRoot::StatementConnection(_)
            | GeneratedTextRoot::PreposedStatementConnection(_)
            | GeneratedTextRoot::ForethoughtStatement(_) => Err(invalid_graph(
                "statement connection reached single-utterance lowering".to_owned(),
            )),
            GeneratedTextRoot::TextGroupStatement(statement) => {
                let item =
                    self.build_generated_text_group_statement_with_id(utterance_id, statement)?;
                if truth_question {
                    self.add_generated_truth_question_to_discourse_item(item, 0)?;
                }
                Ok(item)
            }
            GeneratedTextRoot::ZantufaStatementTerms(statement) => self
                .build_utterance_for_generated_zantufa_statement_terms(
                    utterance_id,
                    statement,
                    truth_question,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_discourse_item_for_generated_text_root(
        &mut self,
        root: GeneratedTextRoot<'tree>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match root {
            GeneratedTextRoot::Bridi(bridi) => {
                let utterance_id = self.next_utterance_id();
                self.current_utterance = Some(utterance_id);
                self.build_bridi_utterance_with_force(
                    utterance_id,
                    bridi,
                    generated_bridi_force(bridi, false),
                )
                .map(|(utterance, _formula)| utterance)
            }
            GeneratedTextRoot::Fragment(fragment) => {
                let utterance_id = self.next_utterance_id();
                self.current_utterance = Some(utterance_id);
                self.build_generated_fragment_utterance(utterance_id, fragment)
            }
            GeneratedTextRoot::StatementConnection(connection) => {
                self.build_i_statement_connection_sequence(connection)
            }
            GeneratedTextRoot::PreposedStatementConnection(connection) => {
                self.build_preposed_i_statement_connection_sequence(connection)
            }
            GeneratedTextRoot::PrenexStatement(statement) => {
                self.build_discourse_item_for_generated_prenex_statement(statement)
            }
            GeneratedTextRoot::TextGroupStatement(statement) => {
                self.build_generated_text_group_statement(statement)
            }
            GeneratedTextRoot::ForethoughtStatement(statement) => {
                self.build_forethought_statement_connection_sequence(statement)
            }
            GeneratedTextRoot::ZantufaStatementTerms(statement) => {
                self.build_discourse_item_for_generated_zantufa_statement_terms(statement)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_zantufa_mekso_fragment_referent(
        &mut self,
        fragment: &'tree ZantufaMeksoFragmentSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_number_descriptor_mekso_surface_text(fragment.0.as_ref())?;
        let quantity = self.build_quantity_for_generated_mekso(
            fragment.0.as_ref(),
            self.source_for_node(fragment.0.as_ref(), "quantity"),
        )?;
        let id = self.next_referent_with_sort_id(SemanticSort::Number);
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Number,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::Number,
                    word: "mex".to_owned(),
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
                self.source_for_node(fragment, "number-fragment"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    pub(super) fn build_utterance_for_generated_zantufa_statement_terms(
        &mut self,
        utterance_id: SemanticObjectId,
        statement: &'tree ZantufaStatementTermsStatementSyntax,
        truth_question: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let root = semantic_root_from_statement(&statement.statement)?;
        let suffix_terms = zantufa_statement_terms_tail_terms(&statement.tail);
        if !suffix_terms.is_empty() {
            let GeneratedTextRoot::Bridi(bridi) = root else {
                return Err(invalid_graph(
                    "Zantufa statement-level trailing terms reached a non-bridi statement"
                        .to_owned(),
                ));
            };
            return self
                .build_bridi_utterance_with_force_and_suffix_terms(
                    utterance_id,
                    bridi,
                    generated_bridi_force(bridi, truth_question),
                    &suffix_terms,
                    statement,
                )
                .map(|(utterance, _formula)| utterance);
        }
        if !generated_text_root_is_utterance(&root) {
            return Err(invalid_graph(
                "Zantufa statement-level reset reached a statement connection in single-utterance lowering"
                    .to_owned(),
            ));
        }
        self.build_utterance_for_generated_text_root(utterance_id, root, truth_question)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_discourse_item_for_generated_zantufa_statement_terms(
        &mut self,
        statement: &'tree ZantufaStatementTermsStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let root = semantic_root_from_statement(&statement.statement)?;
        let suffix_terms = zantufa_statement_terms_tail_terms(&statement.tail);
        if !suffix_terms.is_empty() {
            let GeneratedTextRoot::Bridi(bridi) = root else {
                return Err(invalid_graph(
                    "Zantufa statement-level trailing terms reached a non-bridi statement"
                        .to_owned(),
                ));
            };
            let utterance_id = self.next_utterance_id();
            self.current_utterance = Some(utterance_id);
            return self
                .build_bridi_utterance_with_force_and_suffix_terms(
                    utterance_id,
                    bridi,
                    generated_bridi_force(bridi, false),
                    &suffix_terms,
                    statement,
                )
                .map(|(utterance, _formula)| utterance);
        }
        self.build_discourse_item_for_generated_text_root(root)
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    pub(super) fn build_utterance_for_generated_prenex_statement(
        &mut self,
        utterance_id: SemanticObjectId,
        statement: &'tree PrenexStatementSyntax,
        truth_question: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let context = self.push_generated_prenex_term_bindings(&statement.prenex_terms)?;
        let root = match semantic_root_from_statement(&statement.inner_statement) {
            Ok(root) => root,
            Err(error) => {
                self.pop_generated_prenex_scope_bindings(context.into_data().pushed_bindings);
                return Err(error);
            }
        };
        if !generated_text_root_is_utterance(&root) {
            self.pop_generated_prenex_scope_bindings(context.into_data().pushed_bindings);
            return Err(invalid_graph(
                "statement connection reached single-utterance prenex lowering".to_owned(),
            ));
        }
        let result =
            self.build_utterance_for_generated_text_root(utterance_id, root, truth_question);
        let data!(GeneratedPrenexContext {
            pushed_bindings,
            topics,
        }) = context.into_data();
        self.pop_generated_prenex_scope_bindings(pushed_bindings);
        let item = result?;
        self.apply_generated_prenex_terms_to_discourse_item(
            item,
            &statement.prenex_terms,
            topics,
            self.source_for_node(statement, "topic-comment"),
        )?;
        Ok(item)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance || id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_discourse_item_for_generated_prenex_statement(
        &mut self,
        statement: &'tree PrenexStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let context = self.push_generated_prenex_term_bindings(&statement.prenex_terms)?;
        let root = match semantic_root_from_statement(&statement.inner_statement) {
            Ok(root) => root,
            Err(error) => {
                self.pop_generated_prenex_scope_bindings(context.into_data().pushed_bindings);
                return Err(error);
            }
        };
        let result = self.build_discourse_item_for_generated_text_root(root);
        let data!(GeneratedPrenexContext {
            pushed_bindings,
            topics,
        }) = context.into_data();
        self.pop_generated_prenex_scope_bindings(pushed_bindings);
        let item = result?;
        self.apply_generated_prenex_terms_to_discourse_item(
            item,
            &statement.prenex_terms,
            topics,
            self.source_for_node(statement, "topic-comment"),
        )?;
        Ok(item)
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_leading_cmevla_utterance(
        &mut self,
        tokens: &[Token],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sign = self.next_sign_id();
        let source = self.source_for_tokens(tokens, "name-words");
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Text,
                token_list_text(tokens.iter()),
                source.clone(),
                Vec::new(),
            ),
        )?;
        self.insert_generated_utterance_after_locution(UtteranceForce::Mention, Some(sign), source)
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_leading_nai_utterance(
        &mut self,
        tokens: &[Token],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sign = self.next_sign_id();
        let source = self.source_for_tokens(tokens, "connective-answer");
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Connective,
                token_list_text(tokens.iter()),
                source.clone(),
                Vec::new(),
            ),
        )?;
        self.insert_generated_utterance_after_locution(UtteranceForce::Mention, Some(sign), source)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_standalone_connective_utterance<N: TreeNode>(
        &mut self,
        connective: &N,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let sign = self.build_generated_connective_sign(connective, "connective-expression")?;
        self.insert_generated_utterance_after_locution(
            UtteranceForce::Mention,
            Some(sign),
            self.source_for_node(connective, "connective-utterance"),
        )
    }

    #[requires(!indicators.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_standalone_indicator_utterance(
        &mut self,
        indicators: &[LeadingIndicatorSyntax],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let utterance = self.next_utterance_id();
        let parts = leading_indicator_parts(indicators, false);
        let source_tokens = parts
            .iter()
            .flat_map(|part| part.tokens.iter().cloned())
            .collect::<Vec<_>>();
        if source_tokens.is_empty() {
            return Err(invalid_graph(
                "standalone generated indicator has no source tokens".to_owned(),
            ));
        }
        let sign = self.next_sign_id();
        let source = self.source_for_tokens(&source_tokens, "indicator-expression");
        self.insert(
            sign,
            SemanticObject::text_sign(
                SignKind::Text,
                token_list_text(source_tokens.iter()),
                source.clone(),
                Vec::new(),
            ),
        )?;
        let mut displays = Vec::new();
        for draft in indicator_display_drafts(parts) {
            displays.push(self.insert_generated_indicator_display(
                draft,
                sign,
                utterance,
                "indicator",
                None,
                false,
            )?);
        }
        let content = if let [display] = displays.as_slice() {
            *display
        } else {
            let sequence = self.next_sequence_id();
            self.insert(
                sequence,
                SemanticObject::sequence(
                    displays,
                    SequenceRelation::SameTopicContinuation,
                    source.clone(),
                    Vec::new(),
                ),
            )?;
            sequence
        };
        self.insert_generated_utterance(
            utterance,
            UtteranceForce::Mention,
            Some(content),
            self.source_for_tokens(&source_tokens, "indicator-utterance"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|ids| ids.iter().all(|id| matches!(id.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::DisplayedContent))) || ret.is_err())]
    pub(super) fn build_generated_vocative_asides_from_refs(
        &mut self,
        free_modifiers: &[&'tree FreeModifierSyntax],
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut asides = Vec::new();
        for free_modifier in free_modifiers {
            if let Some(aside) = self.build_generated_vocative_aside(free_modifier)? {
                asides.push(aside);
            }
        }
        Ok(asides)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|ids| ids.iter().all(|id| matches!(id.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::DisplayedContent))) || ret.is_err())]
    pub(super) fn build_generated_vocative_asides_from_slice(
        &mut self,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut asides = Vec::new();
        for free_modifier in free_modifiers {
            if let Some(aside) = self.build_generated_vocative_aside(free_modifier)? {
                asides.push(aside);
            }
        }
        Ok(asides)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn queue_generated_vocative_asides(
        &mut self,
        free_modifiers: &'tree [FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        let asides = self.build_generated_vocative_asides_from_slice(free_modifiers)?;
        self.pending_asides.extend(asides);
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn attach_generated_statement_reciprocity_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        free_modifiers: &[&'tree FreeModifierSyntax],
    ) -> Result<(), SemanticsError> {
        if !free_modifier_refs_have_generated_reciprocity(free_modifiers) {
            return Ok(());
        }
        let Some(formula) = self.content_formula_for_generated_discourse_item(item) else {
            self.add_generated_object_diagnostic(
                item,
                "statement-level soi has no formula-bearing statement to modify",
            );
            return Ok(());
        };
        let predication = match self.primary_predication_for_formula(formula) {
            Ok(predication) => predication,
            Err(_) => {
                self.add_generated_object_diagnostic(
                    item,
                    "statement-level soi has no primary predication to modify",
                );
                return Ok(());
            }
        };
        self.attach_generated_reciprocity_from_free_modifier_refs(predication, free_modifiers, None)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn attach_generated_reciprocity_to_predication_for_terms(
        &mut self,
        predication: SemanticObjectId,
        terms: &[&'tree TermSyntax],
    ) -> Result<(), SemanticsError> {
        let mut exchanges = Vec::new();
        for term in terms {
            self.collect_generated_reciprocal_exchanges_from_term(
                predication,
                term,
                &mut exchanges,
            )?;
        }
        if exchanges.is_empty() {
            return Ok(());
        }
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find generated SOI predication {predication}"
            ))
        })?;
        object.update_predication(|node| {
            let mut data = node.into_data();
            data.reciprocity.extend(exchanges);
            PredicationNode::from_data(data)
        });
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn attach_generated_reciprocity_from_free_modifier_refs(
        &mut self,
        predication: SemanticObjectId,
        free_modifiers: &[&'tree FreeModifierSyntax],
        host_sumti: Option<&'tree SumtiSyntax>,
    ) -> Result<(), SemanticsError> {
        let mut exchanges = Vec::new();
        self.collect_generated_reciprocal_exchanges_from_free_modifier_refs(
            predication,
            free_modifiers,
            host_sumti,
            &mut exchanges,
        )?;
        if exchanges.is_empty() {
            return Ok(());
        }
        let object = self.objects.get_mut(&predication).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find generated SOI predication {predication}"
            ))
        })?;
        object.update_predication(|node| {
            let mut data = node.into_data();
            data.reciprocity.extend(exchanges);
            PredicationNode::from_data(data)
        });
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchanges_from_term(
        &mut self,
        predication: SemanticObjectId,
        term: &'tree TermSyntax,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match term {
            TermSyntax::TermsetGroup(termset) => {
                self.collect_generated_reciprocal_exchanges_from_simple_term(
                    predication,
                    termset.leading_term.as_ref(),
                    out,
                )?;
                for continuation in &termset.continuations {
                    self.collect_generated_reciprocal_exchanges_from_simple_term(
                        predication,
                        continuation.trailing_term.as_ref(),
                        out,
                    )?;
                }
                Ok(())
            }
            TermSyntax::SimpleTerm(simple) => self
                .collect_generated_reciprocal_exchanges_from_simple_term(predication, simple, out),
            TermSyntax::ConnectedTerm(ConnectedTermSyntax {
                leading_term,
                continuations,
            }) if continuations.is_empty() => self
                .collect_generated_reciprocal_exchanges_from_simple_term(
                    predication,
                    leading_term.as_ref(),
                    out,
                ),
            _ => Ok(()),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchanges_from_simple_term(
        &mut self,
        predication: SemanticObjectId,
        simple: &'tree SimpleTermSyntax,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        match simple {
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                self.collect_generated_reciprocal_exchanges_from_sumti(predication, sumti, out)
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                if let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() {
                    self.collect_generated_reciprocal_exchanges_from_sumti(
                        predication,
                        sumti,
                        out,
                    )?;
                }
                Ok(())
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                if let TaggedOrElidedSumtiSyntax::Sumti(sumti) = term.sumti.as_ref() {
                    self.collect_generated_reciprocal_exchanges_from_sumti(
                        predication,
                        sumti,
                        out,
                    )?;
                }
                Ok(())
            }
            SimpleTermSyntax::NuhiTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_reciprocal_exchanges_from_term(predication, term, out)?;
                }
                Ok(())
            }
            SimpleTermSyntax::KeTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_reciprocal_exchanges_from_term(predication, term, out)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchanges_from_sumti(
        &mut self,
        predication: SemanticObjectId,
        sumti: &'tree SumtiSyntax,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        let Some(free_modifiers) = generated_sumti_spine_free_modifiers(sumti) else {
            return Ok(());
        };
        self.collect_generated_reciprocal_exchanges_from_free_modifiers(
            predication,
            free_modifiers,
            Some(sumti),
            out,
        )
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchanges_from_free_modifier_refs(
        &mut self,
        predication: SemanticObjectId,
        free_modifiers: &[&'tree FreeModifierSyntax],
        host_sumti: Option<&'tree SumtiSyntax>,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        for free_modifier in free_modifiers {
            if let Some(soi) = generated_soi_free_modifier(free_modifier) {
                self.collect_generated_reciprocal_exchange_from_soi(
                    predication,
                    soi,
                    host_sumti,
                    out,
                )?;
            }
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchanges_from_free_modifiers(
        &mut self,
        predication: SemanticObjectId,
        free_modifiers: &'tree [FreeModifierSyntax],
        host_sumti: Option<&'tree SumtiSyntax>,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        for free_modifier in free_modifiers {
            if let Some(soi) = generated_soi_free_modifier(free_modifier) {
                self.collect_generated_reciprocal_exchange_from_soi(
                    predication,
                    soi,
                    host_sumti,
                    out,
                )?;
            }
        }
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn collect_generated_reciprocal_exchange_from_soi(
        &mut self,
        predication: SemanticObjectId,
        soi: &'tree SoiFreeModifierSyntax,
        host_sumti: Option<&'tree SumtiSyntax>,
        out: &mut Vec<ReciprocalExchange>,
    ) -> Result<(), SemanticsError> {
        let left = self.build_generated_reciprocal_argument_for_sumti(
            predication,
            soi.leading_sumti.as_ref(),
        )?;
        let right = if let Some(trailing_sumti) = soi.trailing_sumti.as_deref() {
            self.build_generated_reciprocal_argument_for_sumti(predication, trailing_sumti)?
        } else if let Some(host_sumti) = host_sumti {
            self.build_generated_reciprocal_argument_for_sumti(predication, host_sumti)?
        } else {
            self.add_generated_object_diagnostic(
                predication,
                "soi with one explicit participant has no preceding sumti in this position",
            );
            return Ok(());
        };
        if left.kind == ArgumentValueKind::Deleted || right.kind == ArgumentValueKind::Deleted {
            self.add_generated_object_diagnostic(
                predication,
                "soi reciprocity participant was deleted; exchange omitted",
            );
            return Ok(());
        }
        out.push(ReciprocalExchange::new(
            left,
            right,
            "soi".to_owned(),
            self.source_for_node(soi, "reciprocity"),
        ));
        Ok(())
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[ensures(true)]
    pub(super) fn build_generated_reciprocal_argument_for_sumti(
        &mut self,
        predication: SemanticObjectId,
        sumti: &'tree SumtiSyntax,
    ) -> Result<ArgumentValue, SemanticsError> {
        if let Some(place) = generated_voha_place_for_sumti(sumti) {
            return self.generated_predication_argument(predication, place);
        }
        self.build_argument_for_generated_sumti(sumti)
    }

    #[requires(predication.object_kind() == crate::model::SemanticObjectKind::Predication)]
    #[requires(place > 0)]
    #[ensures(true)]
    pub(super) fn generated_predication_argument(
        &mut self,
        predication: SemanticObjectId,
        place: usize,
    ) -> Result<ArgumentValue, SemanticsError> {
        self.objects
            .get(&predication)
            .and_then(|object| object.predication_arguments())
            .and_then(|arguments| arguments.get(&argument_key(place)))
            .cloned()
            .ok_or_else(|| {
                invalid_graph(format!(
                    "generated predication {predication} has no visible argument at x{place}"
                ))
            })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn add_generated_object_diagnostic(
        &mut self,
        object: SemanticObjectId,
        message: &str,
    ) {
        if let Some(object) = self.objects.get_mut(&object) {
            object.push_diagnostic(diagnostic(message));
        }
    }

    #[requires(asides.iter().all(|aside| matches!(aside.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::DisplayedContent)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| matches!(id.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence | crate::model::SemanticObjectKind::DisplayedContent))) || ret.is_err())]
    pub(super) fn build_generated_standalone_asides(
        &mut self,
        asides: Vec<SemanticObjectId>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match asides.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            _ => {
                let id = self.next_sequence_id();
                self.insert(
                    id,
                    SemanticObject::sequence(
                        asides,
                        SequenceRelation::SameTopicContinuation,
                        None,
                        Vec::new(),
                    ),
                )?;
                Ok(Some(id))
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn add_asides_to_generated_discourse_item(
        &mut self,
        item: SemanticObjectId,
        asides: Vec<SemanticObjectId>,
    ) {
        if asides.is_empty() {
            return;
        }
        match item.object_kind() {
            crate::model::SemanticObjectKind::Utterance => {
                self.add_generated_utterance_asides(item, asides);
            }
            crate::model::SemanticObjectKind::Sequence => {
                let first_item = self
                    .objects
                    .get(&item)
                    .and_then(|object| object.as_sequence())
                    .and_then(|sequence| sequence.items.first().copied());
                if let Some(first_item) = first_item {
                    self.add_asides_to_generated_discourse_item(first_item, asides);
                }
            }
            _ => {}
        }
    }

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(asides.iter().all(|aside| matches!(aside.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::DisplayedContent)))]
    #[ensures(true)]
    pub(super) fn add_generated_utterance_asides(
        &mut self,
        utterance: SemanticObjectId,
        asides: Vec<SemanticObjectId>,
    ) {
        if let Some(object) = self.objects.get_mut(&utterance) {
            object.update_utterance(|node| {
                let mut data = node.into_data();
                data.asides.extend(asides);
                UtteranceNode::from_data(data)
            });
        }
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(true)]
    pub(super) fn attach_generated_leading_indicators_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        indicators: &[LeadingIndicatorSyntax],
        truth_question_consumed: bool,
    ) -> Result<(), SemanticsError> {
        if indicators.is_empty() {
            return Ok(());
        }
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            let first_item = self
                .objects
                .get(&item)
                .and_then(|object| object.as_sequence())
                .and_then(|sequence| sequence.items.first().copied());
            if let Some(first_item) = first_item {
                self.attach_generated_leading_indicators_to_discourse_item(
                    first_item,
                    indicators,
                    truth_question_consumed,
                )?;
            }
            return Ok(());
        }
        let parts = leading_indicator_parts(indicators, truth_question_consumed);
        if parts.is_empty() {
            return Ok(());
        }
        let Some(target) = self.displayed_content_target_for_generated_utterance(item) else {
            return Ok(());
        };
        self.attach_generated_indicator_displays_with_target_focus(
            parts,
            target,
            item,
            "indicator",
            Some(DisplayedContentTargetFocus::Bridi),
            false,
        )
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(true)]
    pub(super) fn attach_generated_statement_separator_indicators_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        i: &Token,
        connective: Option<&StatementConnectiveSyntax>,
        force_assertion_effect_none: bool,
    ) -> Result<(), SemanticsError> {
        self.attach_generated_statement_separator_indicators_to_discourse_item_with_target(
            item,
            i,
            connective,
            force_assertion_effect_none,
            None,
        )
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(target_override.is_none_or(|target| displayed_content_target_kind_is_allowed(target.object_kind())))]
    #[ensures(true)]
    pub(super) fn attach_generated_statement_separator_indicators_to_discourse_item_with_target(
        &mut self,
        item: SemanticObjectId,
        i: &Token,
        connective: Option<&StatementConnectiveSyntax>,
        force_assertion_effect_none: bool,
        target_override: Option<SemanticObjectId>,
    ) -> Result<(), SemanticsError> {
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            let first_item = self
                .objects
                .get(&item)
                .and_then(|object| object.as_sequence())
                .and_then(|sequence| sequence.items.first().copied());
            if let Some(first_item) = first_item {
                self.attach_generated_statement_separator_indicators_to_discourse_item_with_target(
                    first_item,
                    i,
                    connective,
                    force_assertion_effect_none,
                    target_override,
                )?;
            }
            return Ok(());
        }
        let mut parts = indicator_parts_for_token(i);
        if let Some(connective) = connective {
            parts.extend(indicator_parts_for_generated_node(connective));
        }
        if parts.is_empty() {
            return Ok(());
        }
        let Some(target) =
            target_override.or_else(|| self.displayed_content_target_for_generated_utterance(item))
        else {
            return Ok(());
        };
        self.attach_generated_indicator_displays_with_target_focus(
            parts,
            target,
            item,
            "indicator",
            Some(DisplayedContentTargetFocus::Bridi),
            force_assertion_effect_none,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn content_formula_for_generated_discourse_item(
        &self,
        item: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&item)?;
        let content = match object.as_data() {
            data!(SemanticObject::Utterance(node)) => node.content?,
            data!(SemanticObject::Sequence(node)) => node.content?,
            _ => return None,
        };
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => Some(content),
            crate::model::SemanticObjectKind::Sequence => self
                .objects
                .get(&content)
                .and_then(|sequence| sequence.as_sequence())
                .and_then(|sequence| sequence.content)
                .filter(|content| {
                    content.object_kind() == crate::model::SemanticObjectKind::Formula
                }),
            crate::model::SemanticObjectKind::Question => self
                .objects
                .get(&content)
                .and_then(|question| question.as_question())
                .map(|question| question.body),
            _ => None,
        }
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn subordinate_formula_for_generated_discourse_item(
        &mut self,
        item: SemanticObjectId,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if let Some(formula) = self.content_formula_for_generated_discourse_item(item) {
            self.mark_generated_discourse_item_subordinated(item);
            return Ok(Some(formula));
        }
        let Some(sequence) = self
            .objects
            .get(&item)
            .and_then(SemanticObject::as_sequence)
            .cloned()
        else {
            return Ok(None);
        };
        if sequence.connection_claims.is_empty() {
            return Ok(None);
        }

        let mut children =
            Vec::with_capacity(sequence.items.len() + sequence.connection_claims.len());
        for child_item in sequence.items.iter().copied() {
            let Some(formula) =
                self.subordinate_formula_for_generated_discourse_item(child_item)?
            else {
                return Ok(None);
            };
            children.push(formula);
        }
        children.extend(sequence.connection_claims.iter().copied());

        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                children,
                None,
                sequence.common.source.clone(),
                Vec::new(),
            ),
        )?;
        if let Some(object) = self.objects.get_mut(&item) {
            object.update_sequence(|node| {
                node.with_data(data! {
                    force: Some(UtteranceForce::Subordinated),
                    content: Some(formula),
                })
            });
        }
        self.set_generated_pro_bridi_formula_predication_mode(formula, PredicationMode::Inert);
        Ok(Some(formula))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.is_none_or(semantic_id_is_eventuality))]
    pub(super) fn primary_eventuality_for_generated_formula(
        &self,
        formula: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&formula)?;
        if let Some(eventuality) = object
            .as_formula()
            .and_then(|formula| match formula.as_data() {
                data!(FormulaNode::Connective(node)) => node.eventuality,
                _ => None,
            })
        {
            return Some(eventuality);
        }
        match object.formula_operator()? {
            FormulaOperator::Atom => {
                let predication = self.objects.get(&object.formula_predication()?)?;
                (predication.predication_mode() == Some(PredicationMode::Asserted))
                    .then(|| predication.predication_eventuality())
                    .flatten()
            }
            _ if object
                .as_formula()
                .and_then(|formula| match formula.as_data() {
                    data!(FormulaNode::Connective(node)) => node.connector.as_ref(),
                    _ => None,
                })
                .as_ref()
                .is_some_and(|connector| connector.source == "tanru") =>
            {
                object
                    .formula_children()
                    .first()
                    .and_then(|child| self.primary_eventuality_for_generated_formula(*child))
            }
            _ => None,
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|eventualities| eventualities.iter().all(|id| semantic_id_is_eventuality(*id))) || ret.is_err())]
    pub(super) fn eventualities_for_generated_formula_predications(
        &self,
        formula: SemanticObjectId,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut eventualities = Vec::new();
        let mut seen = BTreeSet::new();
        self.collect_eventualities_for_generated_formula_predications(
            formula,
            &mut eventualities,
            &mut seen,
        )?;
        if eventualities.is_empty() {
            eventualities.push(self.eventuality_for_generated_formula(formula)?);
        }
        Ok(eventualities)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(eventualities.iter().all(|id| semantic_id_is_eventuality(*id)))]
    #[requires(seen.iter().all(|id| semantic_id_is_eventuality(*id)))]
    #[ensures(eventualities.iter().all(|id| semantic_id_is_eventuality(*id)))]
    #[ensures(seen.iter().all(|id| semantic_id_is_eventuality(*id)))]
    pub(super) fn collect_eventualities_for_generated_formula_predications(
        &self,
        formula: SemanticObjectId,
        eventualities: &mut Vec<SemanticObjectId>,
        seen: &mut BTreeSet<SemanticObjectId>,
    ) -> Result<(), SemanticsError> {
        let object = self.objects.get(&formula).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find formula {formula} for eventuality traversal"
            ))
        })?;
        if let Some(predication) = object.formula_predication()
            && let Some(eventuality) = self
                .objects
                .get(&predication)
                .and_then(SemanticObject::predication_eventuality)
            && seen.insert(eventuality)
        {
            eventualities.push(eventuality);
        }
        for child in object.formula_children() {
            self.collect_eventualities_for_generated_formula_predications(
                *child,
                eventualities,
                seen,
            )?;
        }
        if let Some(restriction) = object.formula_restriction() {
            self.collect_eventualities_for_generated_formula_predications(
                restriction,
                eventualities,
                seen,
            )?;
        }
        if let Some(body) = object.formula_body() {
            self.collect_eventualities_for_generated_formula_predications(
                body,
                eventualities,
                seen,
            )?;
        }
        Ok(())
    }

    #[requires(matches!(
        content.object_kind(),
        crate::model::SemanticObjectKind::Formula | crate::model::SemanticObjectKind::Sequence
    ))]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| semantic_id_is_eventuality(*eventuality)) || ret.is_err())]
    pub(super) fn reified_eventuality_for_generated_content(
        &mut self,
        content: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(eventuality) = self.content_eventualities.get(&content) {
            return Ok(*eventuality);
        }
        let eventuality = self.next_eventuality_id();
        let mut event =
            SemanticObject::generated_eventuality(EventualityClass::Event, None, source);
        event.update_eventuality(|node| node.with_data(data! { content: Some(content) }));
        self.insert(eventuality, event)?;
        self.content_eventualities.insert(content, eventuality);
        Ok(eventuality)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| semantic_id_is_eventuality(*eventuality)) || ret.is_err())]
    pub(super) fn modal_eventuality_argument_for_generated_formula(
        &mut self,
        formula: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(eventuality) = self.primary_eventuality_for_generated_formula(formula) {
            self.set_generated_eventuality_content_if_absent(eventuality, formula);
            return Ok(eventuality);
        }
        self.reified_eventuality_for_generated_content(formula, source)
    }

    #[requires(semantic_id_is_eventuality(eventuality))]
    #[requires(content.object_kind() == crate::model::SemanticObjectKind::Formula || content.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(true)]
    pub(super) fn set_generated_eventuality_content_if_absent(
        &mut self,
        eventuality: SemanticObjectId,
        content: SemanticObjectId,
    ) {
        if let Some(object) = self.objects.get_mut(&eventuality)
            && object
                .as_eventuality()
                .is_some_and(|node| node.content.is_none())
        {
            object.update_eventuality(|node| node.with_data(data! { content: Some(content) }));
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|eventuality| eventuality.is_none_or(semantic_id_is_eventuality)) || ret.is_err())]
    pub(super) fn modal_eventuality_argument_for_generated_discourse_item(
        &mut self,
        item: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(object) = self.objects.get(&item) else {
            return Ok(None);
        };
        let object_kind = object.object_kind();
        let content = object
            .as_utterance()
            .and_then(|utterance| utterance.content);
        match object_kind {
            crate::model::SemanticObjectKind::Sequence => self
                .reified_eventuality_for_generated_content(item, source)
                .map(Some),
            crate::model::SemanticObjectKind::Utterance => {
                let Some(content) = content else {
                    return Ok(None);
                };
                match content.object_kind() {
                    crate::model::SemanticObjectKind::Formula => self
                        .modal_eventuality_argument_for_generated_formula(content, source)
                        .map(Some),
                    crate::model::SemanticObjectKind::Sequence => self
                        .reified_eventuality_for_generated_content(content, source)
                        .map(Some),
                    crate::model::SemanticObjectKind::Question => {
                        let body = self
                            .objects
                            .get(&content)
                            .and_then(|question| question.as_question())
                            .map(|question| question.body);
                        match body {
                            Some(body) => self
                                .modal_eventuality_argument_for_generated_formula(body, source)
                                .map(Some),
                            None => Ok(None),
                        }
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_contextual_modal_statement_connection_claim(
        &mut self,
        item: SemanticObjectId,
        spec: &GeneratedModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let visible_argument = match spec.argument_kind {
            GeneratedModalConnectionArgumentKind::Eventuality => {
                self.modal_eventuality_argument_for_generated_discourse_item(item, source.clone())?
            }
            GeneratedModalConnectionArgumentKind::Formula => {
                self.content_formula_for_generated_discourse_item(item)
            }
        };
        let Some(visible_argument) = visible_argument else {
            return Ok(None);
        };
        let mut arguments = BTreeMap::new();
        arguments.insert(
            argument_key(spec.visible_place),
            ArgumentValue::filled(visible_argument, None),
        );
        let predication = self.build_generated_predication_from_arguments(
            spec.relation.clone(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        if let Some(object) = self.objects.get_mut(&predication) {
            object.update_predication(|node| {
                node.with_data(data! { introduced_by: Some(spec.introduced_by.clone()) })
            });
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(Some(formula))
    }

    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_modal_statement_connection_claim(
        &mut self,
        leading_item: SemanticObjectId,
        trailing_item: SemanticObjectId,
        spec: &GeneratedModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        match spec.argument_kind {
            GeneratedModalConnectionArgumentKind::Eventuality => {
                let Some(leading_eventuality) = self
                    .modal_eventuality_argument_for_generated_discourse_item(
                        leading_item,
                        source.clone(),
                    )?
                else {
                    return Ok(None);
                };
                let Some(trailing_eventuality) = self
                    .modal_eventuality_argument_for_generated_discourse_item(
                        trailing_item,
                        source.clone(),
                    )?
                else {
                    return Ok(None);
                };
                self.build_generated_modal_connection_claim_from_arguments(
                    trailing_eventuality,
                    leading_eventuality,
                    spec,
                    source,
                )
            }
            GeneratedModalConnectionArgumentKind::Formula => {
                let Some(leading_formula) =
                    self.content_formula_for_generated_discourse_item(leading_item)
                else {
                    return Ok(None);
                };
                let Some(trailing_formula) =
                    self.content_formula_for_generated_discourse_item(trailing_item)
                else {
                    return Ok(None);
                };
                self.build_generated_modal_formula_connection_claim(
                    trailing_formula,
                    leading_formula,
                    spec,
                    source,
                )
            }
        }
    }

    #[requires(visible_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(other_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_modal_formula_connection_claim(
        &mut self,
        visible_formula: SemanticObjectId,
        other_formula: SemanticObjectId,
        spec: &GeneratedModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let (visible_argument, other_argument) = match spec.argument_kind {
            GeneratedModalConnectionArgumentKind::Eventuality => {
                let visible_eventuality = self.modal_eventuality_argument_for_generated_formula(
                    visible_formula,
                    source.clone(),
                )?;
                let other_eventuality = self.modal_eventuality_argument_for_generated_formula(
                    other_formula,
                    source.clone(),
                )?;
                (visible_eventuality, other_eventuality)
            }
            GeneratedModalConnectionArgumentKind::Formula => (visible_formula, other_formula),
        };
        self.build_generated_modal_connection_claim_from_arguments(
            visible_argument,
            other_argument,
            spec,
            source,
        )
    }

    #[requires(semantic_id_is_eventuality(visible_argument) || visible_argument.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(semantic_id_is_eventuality(other_argument) || other_argument.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(spec.visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|claim| claim.is_none_or(|claim| claim.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_modal_connection_claim_from_arguments(
        &mut self,
        visible_argument: SemanticObjectId,
        other_argument: SemanticObjectId,
        spec: &GeneratedModalStatementConnectionSpec,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let other_place = convert_numbered_place(2, spec.visible_place);
        let highest_place = relation_place_count(self.dictionary, &spec.relation)
            .unwrap_or(spec.visible_place.max(other_place))
            .max(spec.visible_place)
            .max(other_place);
        let mut arguments = BTreeMap::new();
        arguments.insert(
            argument_key(spec.visible_place),
            ArgumentValue::filled(visible_argument, None),
        );
        arguments.insert(
            argument_key(other_place),
            ArgumentValue::filled(other_argument, None),
        );
        for place in 1..=highest_place {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let predication = self.build_generated_predication_from_arguments(
            spec.relation.clone(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        if let Some(object) = self.objects.get_mut(&predication) {
            object.update_predication(|node| {
                node.with_data(data! { introduced_by: Some(spec.introduced_by.clone()) })
            });
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(Some(formula))
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Predication) || ret.is_err())]
    pub(super) fn build_generated_predication_from_arguments(
        &mut self,
        relation: String,
        source: Option<crate::model::SemanticSource>,
        arguments: BTreeMap<PlaceIndex, ArgumentValue>,
        diagnostics: Vec<crate::model::SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let predication = self.next_predication_id();
        let mode = asserted_predication_mode_for_relation(&relation);
        self.insert(
            predication,
            SemanticObject::predication(
                relation,
                Some(eventuality),
                arguments,
                mode,
                source,
                diagnostics,
            ),
        )?;
        Ok(predication)
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::RelationMetadata)) || ret.is_err())]
    pub(super) fn build_generated_relation_metadata_for_tanru_atom_base(
        &mut self,
        base: &TanruUnitAtomBaseSyntax,
        relation: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        self.build_generated_relation_metadata_for_tanru_atom_base_view(
            GeneratedTanruAtomBaseView::Normal(base),
            relation,
            source,
        )
    }

    #[requires(!relation.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::RelationMetadata)) || ret.is_err())]
    pub(super) fn build_generated_relation_metadata_for_tanru_atom_base_view(
        &mut self,
        base: GeneratedTanruAtomBaseView<'_>,
        relation: &str,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if self.dictionary.lookup_word(relation).is_some() {
            return Ok(None);
        }
        let Some(rafsis) = generated_lujvo_rafsi_parts_for_tanru_atom_base_view(base) else {
            return Ok(None);
        };
        let mut source_words = Vec::new();
        let mut rafsi_bindings = Vec::new();
        for rafsi in &rafsis {
            let Some(source_word) = self.source_word_for_generated_lujvo_rafsi(rafsi) else {
                return Ok(None);
            };
            if assignable_koha_cmavo_for_word(&source_word).is_some()
                && let Some(referent) =
                    self.assigned_referents
                        .get(&source_word)
                        .copied()
                        .filter(|referent| {
                            referent.object_kind() == crate::model::SemanticObjectKind::Referent
                        })
            {
                rafsi_bindings.push(RafsiBinding::new(
                    rafsi.clone(),
                    Some(source_word.clone()),
                    Some(referent),
                ));
            }
            source_words.push(source_word);
        }
        let id = self.next_relation_metadata_id();
        self.insert(
            id,
            SemanticObject::relation_metadata(
                relation.to_owned(),
                source_words,
                Vec::new(),
                Some(RelationExpansion {
                    kind: "lujvo".to_owned(),
                    source_words: rafsis,
                    rafsi_bindings,
                }),
                source,
                Vec::new(),
            ),
        )?;
        Ok(Some(id))
    }

    #[requires(!rafsi.is_empty())]
    #[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
    pub(super) fn source_word_for_generated_lujvo_rafsi(&self, rafsi: &str) -> Option<String> {
        if let Some(source_word) = self
            .dictionary
            .lookup_rafsi(rafsi)
            .next()
            .map(|rafsi_match| rafsi_match.entry.word.to_owned())
        {
            return Some(source_word);
        }
        let stripped = rafsi
            .strip_suffix('r')
            .or_else(|| rafsi.strip_suffix('n'))?;
        if stripped.is_empty() {
            return None;
        }
        self.dictionary
            .lookup_rafsi(stripped)
            .next()
            .map(|rafsi_match| rafsi_match.entry.word.to_owned())
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.is_some() || !self.objects.contains_key(&item))]
    pub(super) fn displayed_content_target_for_generated_utterance(
        &self,
        item: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&item)?;
        let content = object
            .as_utterance()
            .and_then(|utterance| utterance.content)
            .unwrap_or(item);
        let content_object = self.objects.get(&content)?;
        if content_object.object_kind() == crate::model::SemanticObjectKind::Question {
            return content_object.as_question().map(|question| question.body);
        }
        Some(content)
    }

    #[requires(displayed_content_target_kind_is_allowed(target.object_kind()))]
    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(!source_construct.is_empty())]
    #[ensures(true)]
    pub(super) fn attach_generated_indicator_displays_with_target_focus(
        &mut self,
        parts: Vec<IndicatorPart>,
        target: SemanticObjectId,
        anchor: SemanticObjectId,
        source_construct: &str,
        target_focus: Option<DisplayedContentTargetFocus>,
        force_assertion_effect_none: bool,
    ) -> Result<(), SemanticsError> {
        for draft in indicator_display_drafts(parts) {
            self.insert_generated_indicator_display(
                draft,
                target,
                anchor,
                source_construct,
                target_focus,
                force_assertion_effect_none,
            )?;
        }
        Ok(())
    }

    #[requires(displayed_content_target_kind_is_allowed(target.object_kind()))]
    #[requires(anchor.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(!source_construct.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::DisplayedContent) || ret.is_err())]
    pub(super) fn insert_generated_indicator_display(
        &mut self,
        draft: IndicatorDisplayDraft,
        target: SemanticObjectId,
        anchor: SemanticObjectId,
        source_construct: &str,
        target_focus: Option<DisplayedContentTargetFocus>,
        force_assertion_effect_none: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let assertion_effect = displayed_assertion_effect_for_target(
            draft.assertion_effect,
            target.object_kind(),
            force_assertion_effect_none,
        );
        if matches!(
            assertion_effect,
            DisplayedContentAssertionEffect::HostSubordinated
                | DisplayedContentAssertionEffect::MetalinguisticallyVoided
        ) && target.object_kind() == crate::model::SemanticObjectKind::Formula
        {
            self.set_formula_predication_mode(target, PredicationMode::Inert);
        }
        let id = self.next_display_id();
        let source = self.source_for_tokens(&draft.source_tokens, source_construct);
        let experiencer = if draft.empathy {
            if target.object_kind() == crate::model::SemanticObjectKind::Referent {
                target
            } else {
                self.build_elided_referent("dai experiencer".to_owned())?
            }
        } else {
            self.current_speaker()
        };
        let family = if draft.question {
            DisplayedContentFamily::QuestionPrompt
        } else {
            draft.family
        };
        let relation = if draft.question {
            attitude_question_relation(&draft.relation)
        } else {
            draft.relation
        };
        let mut object = SemanticObject::displayed_content(
            family,
            relation,
            draft.polarity,
            assertion_effect,
            experiencer,
            target,
            anchor,
            source,
            Vec::new(),
        );
        object.update_displayed_content(|node| {
            node.with_data(data! {
                target_focus: target_focus,
                intensity: draft.intensity,
                phase: draft.phase,
                modifiers: draft.modifiers,
            })
        });
        self.insert(id, object)?;
        if self.objects.contains_key(&anchor) {
            self.add_generated_utterance_asides(anchor, vec![id]);
        } else {
            self.pending_asides.push(id);
        }
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance)) || ret.is_err())]
    pub(super) fn build_generated_vocative_aside(
        &mut self,
        free_modifier: &'tree FreeModifierSyntax,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let FreeModifierSyntax::VocativeFreeModifier(vocative) = free_modifier else {
            return Ok(None);
        };
        self.build_generated_vocative_free_modifier_aside(vocative)
            .map(Some)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn build_generated_vocative_free_modifier_aside(
        &mut self,
        vocative: &'tree VocativeFreeModifierSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let vocative_kind = generated_vocative_kind_for_markers(&vocative.vocative_markers);
        let question_start = self.direct_question_slots.len();
        let previous_pending_asides = std::mem::take(&mut self.pending_asides);
        let target = if let Some(sumti) = vocative.sumti.as_deref() {
            self.build_generated_vocative_target(sumti)?
        } else {
            new!(GeneratedVocativeTarget {
                object: self.current_audience(),
                formula_scopes: Vec::new(),
                formula: None,
                audience_is_target: true,
            })
        };
        let data!(GeneratedVocativeTarget {
            object: addressed_or_identified,
            formula_scopes,
            formula: prebuilt_target_formula,
            audience_is_target,
        }) = target.into_data();
        let nested_asides = std::mem::replace(&mut self.pending_asides, previous_pending_asides);
        let question_slots = self.direct_question_slots.split_off(question_start);
        let target_formula_source = self.exact_source_for_node(vocative, "vocative-question");
        let target_formula = prebuilt_target_formula.or((!formula_scopes.is_empty()
            || !question_slots.is_empty())
        .then(|| {
            self.build_generated_vocative_target_formula(
                addressed_or_identified,
                target_formula_source.clone(),
            )
        })
        .transpose()?);
        let target_formula = target_formula
            .map(|formula| {
                self.wrap_formula_with_generated_argument_scopes(formula, formula_scopes)
            })
            .transpose()?;
        let content = if question_slots.is_empty() {
            target_formula.or_else(|| {
                (vocative_kind == "selfIdentification"
                    && addressed_or_identified.object_kind()
                        == crate::model::SemanticObjectKind::Referent)
                    .then_some(addressed_or_identified)
            })
        } else {
            Some(self.build_generated_direct_question(
                target_formula.expect("question targets always build a formula"),
                question_slots,
                target_formula_source,
            )?)
        };
        let diagnostics = if addressed_or_identified.object_kind()
            == crate::model::SemanticObjectKind::Referent
            || addressed_or_identified.object_kind() == crate::model::SemanticObjectKind::Parameter
        {
            Vec::new()
        } else {
            vec![diagnostic(
                "vocative target is not referent-valued; audience remains contextual",
            )]
        };
        let locution = self.next_locution_id();
        self.insert(
            locution,
            SemanticObject::referential_eventuality(
                EventualityClass::Locution,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                self.exact_source_for_node(vocative, "vocative"),
            ),
        )?;
        let utterance = self.next_utterance_id();
        self.insert(
            utterance,
            SemanticObject::utterance(
                UtteranceForce::Vocative,
                locution,
                content,
                self.current_speaker(),
                self.current_audience(),
                self.current_now(),
                self.current_here(),
                self.exact_source_for_node(vocative, "vocative"),
                diagnostics,
            ),
        )?;
        if let Some(object) = self.objects.get_mut(&utterance) {
            object.update_utterance(|node| {
                node.with_data(data! { vocative_kind: Some(vocative_kind.clone()) })
            });
        }
        if audience_is_target
            && addressed_or_identified.object_kind() == crate::model::SemanticObjectKind::Referent
        {
            if vocative_kind == "selfIdentification" {
                self.set_generated_referent_target(addressed_or_identified, self.current_speaker());
            } else {
                self.set_generated_utterance_audience(utterance, addressed_or_identified);
                self.set_generated_referent_target(
                    addressed_or_identified,
                    self.current_audience(),
                );
            }
        }
        self.add_generated_utterance_asides(utterance, nested_asides);
        Ok(utterance)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|target| crate::model::argument_object_kind_can_fill(target.object.object_kind())) || ret.is_err())]
    pub(super) fn build_generated_vocative_target(
        &mut self,
        target: &'tree VocativeSumtiSyntax,
    ) -> Result<GeneratedVocativeTarget<'tree>, SemanticsError> {
        match target {
            VocativeSumtiSyntax::Sumti(sumti) => {
                let source = self.exact_source_for_node(sumti, "vocative-question");
                if let Some(formula) =
                    self.build_generated_vocative_sumti_connection_formula(sumti, source)?
                {
                    return Ok(new!(GeneratedVocativeTarget {
                        object: self.current_audience(),
                        formula_scopes: Vec::new(),
                        formula: Some(formula),
                        audience_is_target: false,
                    }));
                }
                if generated_argument_quantifier_source_from_sumti(sumti)?.is_some() {
                    let mut formula_scopes = Vec::new();
                    let argument = self.build_argument_for_generated_sumti_with_formula_scopes(
                        sumti,
                        &mut formula_scopes,
                    )?;
                    let object = argument.value.ok_or_else(|| {
                        invalid_graph(
                            "quantified vocative target has no argument object".to_owned(),
                        )
                    })?;
                    return Ok(new!(GeneratedVocativeTarget {
                        object,
                        formula_scopes,
                        formula: None,
                        audience_is_target: true,
                    }));
                }
                let referent = self.build_sumti_referent(sumti)?;
                if referent.object_kind() == crate::model::SemanticObjectKind::Referent {
                    self.attach_generated_relative_clauses_to_referent(referent, sumti)?;
                }
                Ok(new!(GeneratedVocativeTarget {
                    object: referent,
                    formula_scopes: Vec::new(),
                    formula: None,
                    audience_is_target: true,
                }))
            }
            VocativeSumtiSyntax::CmevlaVocativeSumti(sumti) => self
                .build_generated_cmevla_vocative_referent(sumti)
                .map(|object| {
                    new!(GeneratedVocativeTarget {
                        object,
                        formula_scopes: Vec::new(),
                        formula: None,
                        audience_is_target: true,
                    })
                }),
            VocativeSumtiSyntax::SelbriVocativeSumti(sumti) => self
                .build_generated_selbri_vocative_referent(sumti)
                .map(|object| {
                    new!(GeneratedVocativeTarget {
                        object,
                        formula_scopes: Vec::new(),
                        formula: None,
                        audience_is_target: true,
                    })
                }),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_cmevla_vocative_referent(
        &mut self,
        sumti: &'tree CmevlaVocativeSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.queue_generated_vocative_asides(&sumti.names.free_modifiers)?;
        let id = self.next_referent_id();
        let mut object = SemanticObject::referent(
            ReferentCategory::Constant,
            SemanticSort::Entity,
            None,
            Some(new!(Descriptor {
                kind: DescriptorKind::Name,
                word: "la".to_owned(),
                speaker: Some(self.current_speaker()),
                body: None,
                veridical: None,
                relative_clauses: Vec::new(),
                quantity: None,
                name: Some(token_list_text(sumti.names.value.iter())),
                scale: None,
                definiteness: None,
                operand: None,
            })),
            None,
            self.source_for_node(&sumti.names, "sumti"),
            Vec::new(),
        );
        if let Some(relative_clauses) = &sumti.leading_relative_clauses {
            let clauses = self.lower_generated_relative_clause_list(relative_clauses, id)?;
            object.update_referent(|node| {
                let mut data = node.into_data();
                data.relative_clauses.extend(clauses);
                ReferentNode::from_data(data)
            });
        }
        if let Some(relative_clauses) = &sumti.trailing_relative_clauses {
            let clauses = self.lower_generated_relative_clause_list(relative_clauses, id)?;
            object.update_referent(|node| {
                let mut data = node.into_data();
                data.relative_clauses.extend(clauses);
                ReferentNode::from_data(data)
            });
        }
        self.insert(id, object)?;
        Ok(id)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_generated_selbri_vocative_referent(
        &mut self,
        sumti: &'tree SelbriVocativeSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let id = self.next_referent_id();
        let body = self.build_restrictive_formula(&sumti.selbri, id)?;
        let mut descriptor_relative_clauses = Vec::new();
        if let Some(relative_clauses) = &sumti.leading_relative_clauses {
            descriptor_relative_clauses
                .extend(self.lower_generated_relative_clause_list(relative_clauses, id)?);
        }
        if let Some(relative_clauses) = &sumti.trailing_relative_clauses {
            descriptor_relative_clauses
                .extend(self.lower_generated_relative_clause_list(relative_clauses, id)?);
        }
        self.insert(
            id,
            SemanticObject::referent(
                ReferentCategory::Constant,
                SemanticSort::Entity,
                None,
                Some(new!(Descriptor {
                    kind: DescriptorKind::SpeakerDescription,
                    word: "le".to_owned(),
                    speaker: Some(self.current_speaker()),
                    body: Some(body),
                    veridical: None,
                    relative_clauses: descriptor_relative_clauses,
                    quantity: None,
                    name: None,
                    scale: None,
                    definiteness: None,
                    operand: None,
                })),
                None,
                self.exact_source_for_node(sumti, "vocative-description"),
                Vec::new(),
            ),
        )?;
        Ok(id)
    }

    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Referent || target.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_vocative_target_formula(
        &mut self,
        target: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let predication = self.next_predication_id();
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(target, None));
        self.insert(
            predication,
            SemanticObject::predication(
                "vocativeTarget".to_owned(),
                None,
                arguments,
                PredicationMode::Performative,
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

    #[requires(utterance.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(audience.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn set_generated_utterance_audience(
        &mut self,
        utterance: SemanticObjectId,
        audience: SemanticObjectId,
    ) {
        if let Some(object) = self.objects.get_mut(&utterance) {
            object.update_utterance(|node| node.with_data(data! { audience: audience }));
        }
    }

    #[requires(referent.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[requires(target.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn set_generated_referent_target(
        &mut self,
        referent: SemanticObjectId,
        target: SemanticObjectId,
    ) {
        if let Some(object) = self.objects.get_mut(&referent) {
            match object.object_kind() {
                crate::model::SemanticObjectKind::Referent => {
                    if object.as_eventuality().is_some() {
                        object.update_eventuality(|node| {
                            node.with_data(data! { target: Some(target) })
                        });
                    } else if object.as_sign().is_some() {
                        object.update_sign(|node| node.with_data(data! { target: Some(target) }));
                    } else {
                        object
                            .update_referent(|node| node.with_data(data! { target: Some(target) }));
                    }
                }
                _ => unreachable!("referent id must identify a referent variant"),
            }
        }
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|(utterance, formula)| *utterance == utterance_id && formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bridi_utterance_with_force(
        &mut self,
        utterance_id: SemanticObjectId,
        bridi: &'tree BridiSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, SemanticObjectId), SemanticsError> {
        self.build_bridi_utterance_with_force_and_suffix_terms(
            utterance_id,
            bridi,
            force,
            &[],
            bridi,
        )
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|(utterance, formula)| *utterance == utterance_id && formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bridi_utterance_with_force_and_suffix_terms<N: TreeNode>(
        &mut self,
        utterance_id: SemanticObjectId,
        bridi: &'tree BridiSyntax,
        force: UtteranceForce,
        suffix_terms: &[&'tree TermSyntax],
        source_node: &N,
    ) -> Result<(SemanticObjectId, SemanticObjectId), SemanticsError> {
        let question_start = self.direct_question_slots.len();
        let existential_start = self.implicit_existential_variables.len();
        let previous_asides = std::mem::take(&mut self.pending_asides);
        let previous_da_series_bindings = std::mem::take(&mut self.implicit_da_series_bindings);
        let previous_recorded_implicit_existentials =
            std::mem::take(&mut self.recorded_implicit_existential_variables);
        let formula = if suffix_terms.is_empty() {
            self.build_bridi_formula(bridi)
        } else {
            self.build_bridi_formula_with_suffix_terms(source_node, bridi, suffix_terms)
        };
        self.implicit_da_series_bindings = previous_da_series_bindings;
        self.recorded_implicit_existential_variables = previous_recorded_implicit_existentials;
        let formula = formula?;
        self.record_completed_generated_pro_bridi_frame_from_bridi(
            bridi,
            formula,
            self.source_for_node(source_node, "predication"),
        )?;
        let asides = std::mem::replace(&mut self.pending_asides, previous_asides);
        let existentials = self
            .implicit_existential_variables
            .split_off(existential_start);
        let formula = self.wrap_formula_with_generated_bridi_formula_scopes(
            formula,
            existentials,
            Vec::new(),
        )?;
        let formula = self
            .wrap_generated_bridi_formula_with_contradictory_event_tense_negation(bridi, formula)?;
        let mut question_slots = self.direct_question_slots.split_off(question_start);
        question_slots.extend(
            self.direct_truth_question_source_orders(source_node)
                .into_iter()
                .map(|source_order| {
                    GeneratedDirectQuestionSlot::from_data(data!(GeneratedDirectQuestionSlot {
                        parameter: None,
                        kind: QuestionKind::Truth,
                        domain: SemanticSort::TruthValue,
                        source_order,
                    }))
                }),
        );
        if force == UtteranceForce::Ask {
            let source_order = self
                .source_for_node(source_node, "truth-question")
                .map(|source| source.span.byte_start.saturating_sub(1))
                .unwrap_or(0);
            question_slots.push(GeneratedDirectQuestionSlot::from_data(data!(
                GeneratedDirectQuestionSlot {
                    parameter: None,
                    kind: QuestionKind::Truth,
                    domain: SemanticSort::TruthValue,
                    source_order,
                }
            )));
        }
        let (force, content) = if question_slots.is_empty() {
            (force, formula)
        } else {
            (
                UtteranceForce::Ask,
                self.build_generated_direct_question(
                    formula,
                    question_slots,
                    self.source_for_node(source_node, "question"),
                )?,
            )
        };
        self.insert_generated_utterance(
            utterance_id,
            force,
            Some(content),
            self.source_for_node(source_node, "bridi"),
        )?;
        if let Some(selbri) = main_generated_selbri_for_bridi(bridi) {
            self.attach_generated_indicator_displays_with_target_focus(
                indicator_parts_for_generated_node(selbri),
                formula,
                utterance_id,
                "indicator",
                Some(DisplayedContentTargetFocus::Selbri),
                false,
            )?;
        }
        self.add_generated_utterance_asides(utterance_id, asides);
        Ok((utterance_id, formula))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(parameters.iter().all(|parameter| parameter.object_kind() == crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Question) || ret.is_err())]
    pub(super) fn build_direct_question(
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
            .map(|parameter| QuestionSlot::homogeneous(parameter, QuestionSlotRole::Answer))
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
    #[requires(!slots.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Question) || ret.is_err())]
    fn build_generated_direct_question(
        &mut self,
        formula: SemanticObjectId,
        mut slots: Vec<GeneratedDirectQuestionSlot>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        slots.sort_by_key(|slot| slot.source_order);
        let first_kind = slots[0].kind;
        let first_domain = slots[0].domain;
        let homogeneous = slots
            .iter()
            .all(|slot| slot.kind == first_kind && slot.domain == first_domain);
        if homogeneous {
            let parameters = slots
                .into_iter()
                .filter_map(|slot| slot.parameter)
                .collect::<Vec<_>>();
            return self.build_direct_question(
                first_kind,
                first_domain,
                formula,
                parameters,
                source,
            );
        }
        let question = SemanticObjectId::question(self.next_index);
        self.next_index += 1;
        let slots = slots
            .into_iter()
            .map(|slot| {
                QuestionSlot::typed(
                    slot.parameter,
                    QuestionSlotRole::Answer,
                    slot.kind,
                    slot.domain,
                )
            })
            .collect();
        self.insert(
            question,
            SemanticObject::question(
                QuestionKind::Multiple,
                QuestionMode::Direct,
                SemanticSort::ArgumentBundle,
                formula,
                slots,
                self.current_speaker(),
                self.current_audience(),
                source,
            ),
        )?;
        Ok(question)
    }

    #[requires(question.object_kind() == crate::model::SemanticObjectKind::Question)]
    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!additional_slots.is_empty())]
    #[ensures(self.objects.get(&question).is_some_and(|object| object.as_question().is_some_and(|node| node.body == formula)))]
    fn extend_generated_direct_question(
        &mut self,
        question: SemanticObjectId,
        formula: SemanticObjectId,
        additional_slots: Vec<GeneratedDirectQuestionSlot>,
    ) {
        let existing = self.objects[&question]
            .as_question()
            .expect("question ID must identify a question")
            .clone();
        let fallback_order = existing
            .common
            .source
            .as_ref()
            .map(|source| source.span.byte_start)
            .unwrap_or(usize::MAX);
        let mut slots = if existing.kind == QuestionKind::Truth {
            vec![GeneratedDirectQuestionSlot::from_data(data!(
                GeneratedDirectQuestionSlot {
                    parameter: None,
                    kind: QuestionKind::Truth,
                    domain: SemanticSort::TruthValue,
                    source_order: fallback_order,
                }
            ))]
        } else {
            existing
                .slots
                .iter()
                .map(|slot| {
                    let (kind, domain) = slot
                        .kind_and_domain()
                        .unwrap_or((existing.kind, existing.domain));
                    let parameter = slot.parameter();
                    let source_order = parameter
                        .and_then(|parameter| self.objects.get(&parameter))
                        .and_then(SemanticObject::source)
                        .map(|source| source.span.byte_start)
                        .unwrap_or(fallback_order);
                    GeneratedDirectQuestionSlot::from_data(data!(GeneratedDirectQuestionSlot {
                        parameter,
                        kind,
                        domain,
                        source_order,
                    }))
                })
                .collect()
        };
        slots.extend(additional_slots);
        slots.sort_by_key(|slot| slot.source_order);
        let first_kind = slots[0].kind;
        let first_domain = slots[0].domain;
        let homogeneous = slots
            .iter()
            .all(|slot| slot.kind == first_kind && slot.domain == first_domain);
        let (kind, domain, slots) = if homogeneous {
            (
                first_kind,
                first_domain,
                slots
                    .into_iter()
                    .filter_map(|slot| slot.parameter)
                    .map(|parameter| QuestionSlot::homogeneous(parameter, QuestionSlotRole::Answer))
                    .collect(),
            )
        } else {
            (
                QuestionKind::Multiple,
                SemanticSort::ArgumentBundle,
                slots
                    .into_iter()
                    .map(|slot| {
                        QuestionSlot::typed(
                            slot.parameter,
                            QuestionSlotRole::Answer,
                            slot.kind,
                            slot.domain,
                        )
                    })
                    .collect(),
            )
        };
        self.objects
            .get_mut(&question)
            .expect("question must remain present")
            .update_question(|node| {
                node.with_data(data! {
                    kind: kind,
                    domain: domain,
                    body: formula,
                    slots: slots,
                })
            });
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.is_none_or(|question| question.object_kind() == crate::model::SemanticObjectKind::Question))]
    fn direct_question_for_generated_discourse_item(
        &self,
        item: SemanticObjectId,
    ) -> Option<SemanticObjectId> {
        let object = self.objects.get(&item)?;
        let content = object
            .as_utterance()
            .and_then(|node| node.content)
            .or_else(|| object.as_sequence().and_then(|node| node.content));
        if let Some(content) = content
            && content.object_kind() == crate::model::SemanticObjectKind::Question
        {
            return Some(content);
        }
        object
            .as_sequence()?
            .items
            .iter()
            .find_map(|item| self.direct_question_for_generated_discourse_item(*item))
    }

    #[requires(sequence.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!slots.is_empty())]
    #[requires(items.iter().all(|item| matches!(item.object_kind(), crate::model::SemanticObjectKind::Utterance | crate::model::SemanticObjectKind::Sequence)))]
    #[ensures(self.objects.get(&sequence).is_some_and(|object| object.as_sequence().is_some_and(|node| node.content.is_some_and(|content| content.object_kind() == crate::model::SemanticObjectKind::Question))))]
    fn set_generated_statement_question_content(
        &mut self,
        sequence: SemanticObjectId,
        formula: SemanticObjectId,
        slots: Vec<GeneratedDirectQuestionSlot>,
        items: &[SemanticObjectId],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        let question = items
            .iter()
            .find_map(|item| self.direct_question_for_generated_discourse_item(*item));
        let question = match question {
            Some(question) => {
                self.extend_generated_direct_question(question, formula, slots);
                question
            }
            None => self.build_generated_direct_question(formula, slots, source)?,
        };
        self.objects
            .get_mut(&sequence)
            .expect("sequence must remain present")
            .update_sequence(|node| node.with_data(data! { content: Some(question) }));
        Ok(())
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn add_generated_truth_question_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        source_order: usize,
    ) -> Result<(), SemanticsError> {
        let Some(formula) = self.content_formula_for_generated_discourse_item(item) else {
            return Ok(());
        };
        let slot = GeneratedDirectQuestionSlot::from_data(data!(GeneratedDirectQuestionSlot {
            parameter: None,
            kind: QuestionKind::Truth,
            domain: SemanticSort::TruthValue,
            source_order,
        }));
        let question = match self.direct_question_for_generated_discourse_item(item) {
            Some(question) => {
                self.extend_generated_direct_question(question, formula, vec![slot]);
                question
            }
            None => self.build_generated_direct_question(
                formula,
                vec![slot],
                self.objects
                    .get(&item)
                    .and_then(SemanticObject::source)
                    .cloned(),
            )?,
        };
        if let Some(object) = self.objects.get_mut(&item) {
            if object.as_sequence().is_some() {
                object.update_sequence(|node| node.with_data(data! { content: Some(question) }));
            } else if object.as_utterance().is_some() {
                object.update_utterance(|node| {
                    node.with_data(data! {
                        force: UtteranceForce::Ask,
                        content: Some(question),
                    })
                });
            }
        }
        Ok(())
    }

    #[requires(parameter.object_kind() == crate::model::SemanticObjectKind::Parameter)]
    #[requires(kind != QuestionKind::Truth && kind != QuestionKind::Multiple)]
    #[requires(crate::model::question_kind_domain_are_coherent(kind, domain))]
    #[ensures(self.direct_question_slots.len() == old(self.direct_question_slots.len()) + 1)]
    pub(super) fn record_generated_direct_question_parameter(
        &mut self,
        parameter: SemanticObjectId,
        kind: QuestionKind,
        domain: SemanticSort,
    ) {
        let source_order = self
            .objects
            .get(&parameter)
            .and_then(SemanticObject::source)
            .map(|source| source.span.byte_start)
            .unwrap_or(usize::MAX);
        self.direct_question_slots
            .push(GeneratedDirectQuestionSlot::from_data(data!(
                GeneratedDirectQuestionSlot {
                    parameter: Some(parameter),
                    kind,
                    domain,
                    source_order,
                }
            )));
    }

    #[requires(true)]
    #[ensures(true)]
    fn direct_truth_question_source_orders<N: TreeNode>(&self, node: &N) -> Vec<usize> {
        let mut collector = GeneratedSpanCollector::default();
        node.visit_in_order(&mut collector);
        let mut orders = Vec::new();
        for token in collector.tokens {
            let parts = indicator_parts_for_token(token);
            if parts.iter().any(|part| part.cmavo == Cmavo::Kau) {
                continue;
            }
            orders.extend(parts.into_iter().filter_map(|part| {
                (part.cmavo == Cmavo::Xu)
                    .then(|| self.source_for_tokens(&part.tokens, "truth-question"))
                    .flatten()
                    .map(|source| source.span.byte_start)
            }));
        }
        orders
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.as_ref().is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Parameter)) || ret.is_err())]
    pub(super) fn build_generated_tense_question_parameter_for_tense_modal<N: TreeNode>(
        &mut self,
        tense_modal: &N,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some(token) = generated_tense_question_token_for_tense_modal(tense_modal) else {
            return Ok(None);
        };
        let parameter = self.next_parameter_id();
        self.insert(
            parameter,
            SemanticObject::parameter(
                SemanticSort::TenseModal,
                ParameterRole::TenseQuestion,
                token_text(&token),
                self.source_for_token(&token, "parameter"),
            ),
        )?;
        self.record_generated_direct_question_parameter(
            parameter,
            QuestionKind::Tense,
            SemanticSort::TenseModal,
        );
        Ok(Some(parameter))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_generated_bridi_formula_with_contradictory_event_tense_negation(
        &mut self,
        bridi: &'tree BridiSyntax,
        formula: SemanticObjectId,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(tense_modal) = first_generated_contradictory_event_tense_modal_for_bridi(bridi)
        else {
            return Ok(formula);
        };
        self.build_unary_formula(
            FormulaOperator::Not,
            formula,
            self.source_for_node(tense_modal, "tense-negation"),
        )
    }

    #[requires(focus.focus.object_kind() == crate::model::SemanticObjectKind::Parameter || focus.focus.object_kind() == crate::model::SemanticObjectKind::Referent)]
    #[ensures(true)]
    pub(super) fn record_generated_indirect_question_focus(
        &mut self,
        focus: GeneratedIndirectQuestionFocus,
    ) -> bool {
        let Some(foci) = self.indirect_question_stack.last_mut() else {
            return false;
        };
        foci.push(focus);
        true
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|questions| questions.iter().all(|question| question.object_kind() == crate::model::SemanticObjectKind::Question)) || ret.is_err())]
    pub(super) fn build_generated_embedded_indirect_questions(
        &mut self,
        body: SemanticObjectId,
        foci: Vec<GeneratedIndirectQuestionFocus>,
    ) -> Result<Vec<SemanticObjectId>, SemanticsError> {
        let mut questions = Vec::new();
        for focus in foci {
            let data!(GeneratedIndirectQuestionFocus {
                focus,
                presupposed_answer,
                slots,
                kind,
                domain,
                source,
            }) = focus.into_data();
            let id = self.next_question_id();
            let object = SemanticObject::question_with_focus(
                kind,
                QuestionMode::Indirect,
                domain,
                body,
                slots,
                Some(focus),
                presupposed_answer,
                self.current_speaker(),
                self.current_audience(),
                source,
            );
            self.insert(id, object)?;
            questions.push(id);
        }
        Ok(questions)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(existentials.iter().all(|existential| matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_implicit_existentials(
        &mut self,
        formula: SemanticObjectId,
        existentials: Vec<GeneratedImplicitExistential>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut body = formula;
        for existential in existentials.into_iter().rev() {
            body =
                self.wrap_formula_with_generated_implicit_existential_ordered(body, existential)?;
        }
        Ok(body)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(existentials.iter().all(|existential| matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_bridi_formula_scopes(
        &mut self,
        formula: SemanticObjectId,
        existentials: Vec<GeneratedImplicitExistential>,
        term_scopes: Vec<GeneratedTermFormulaScope>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut scopes = existentials
            .into_iter()
            .map(|existential| {
                let order = existential
                    .source
                    .as_ref()
                    .map(|source| source.span.byte_start)
                    .unwrap_or(usize::MAX);
                (
                    order,
                    GeneratedBridiFormulaScope::ImplicitExistential(existential),
                )
            })
            .collect::<Vec<_>>();
        scopes.extend(term_scopes.into_iter().map(|scope| {
            let order = generated_term_formula_scope_source(&scope)
                .map(|source| source.span.byte_start)
                .unwrap_or(usize::MAX);
            (order, GeneratedBridiFormulaScope::Term(scope))
        }));
        scopes.sort_by_key(|(order, _scope)| *order);

        let mut body = formula;
        for (_order, scope) in scopes.into_iter().rev() {
            body = match scope {
                GeneratedBridiFormulaScope::ImplicitExistential(existential) => self
                    .wrap_formula_with_generated_implicit_existential_ordered(body, existential)?,
                GeneratedBridiFormulaScope::Term(GeneratedTermFormulaScope::Negation {
                    source,
                }) => self.build_unary_formula(FormulaOperator::Not, body, source)?,
            };
        }
        Ok(body)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_implicit_existential_ordered(
        &mut self,
        body: SemanticObjectId,
        existential: GeneratedImplicitExistential,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let existential_order = existential
            .source
            .as_ref()
            .map(|source| source.span.byte_start)
            .unwrap_or(usize::MAX);
        if let Some(outer_order) = self.generated_quantifier_formula_source_order(body)
            && outer_order <= existential_order
            && let Some(inner_body) = self
                .objects
                .get(&body)
                .and_then(SemanticObject::formula_body)
        {
            let wrapped_inner = self.wrap_formula_with_generated_implicit_existential_ordered(
                inner_body,
                existential,
            )?;
            if wrapped_inner != inner_body
                && let Some(object) = self.objects.get_mut(&body)
            {
                object.update_formula(|formula| match formula.into_data() {
                    data!(FormulaNode::Quantified(node)) => new!(FormulaNode::Quantified(
                        node.with_data(data! { body: wrapped_inner })
                    )),
                    data => FormulaNode::from_data(data),
                });
            }
            return Ok(body);
        }
        self.build_generated_implicit_existential_formula(body, existential)
    }

    #[requires(body.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(matches!(existential.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_implicit_existential_formula(
        &mut self,
        body: SemanticObjectId,
        existential: GeneratedImplicitExistential,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let data!(GeneratedImplicitExistential {
            variable,
            source,
            restrictions,
        }) = existential.into_data();
        let restriction = self.combine_generated_restriction_formulas(restrictions)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::quantified_formula(
                FormulaOperator::Exists,
                variable,
                restriction,
                body,
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn generated_quantifier_formula_source_order(
        &self,
        formula: SemanticObjectId,
    ) -> Option<usize> {
        let object = self.objects.get(&formula)?;
        let is_quantifier = object.formula_operator().is_some_and(|operator| {
            matches!(
                operator,
                FormulaOperator::Exists
                    | FormulaOperator::Forall
                    | FormulaOperator::None
                    | FormulaOperator::Cardinality
                    | FormulaOperator::PluralExists
                    | FormulaOperator::PluralForall
            )
        });
        is_quantifier.then(|| object.source().map(|source| source.span.byte_start))?
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(true)]
    pub(super) fn apply_generated_prenex_terms_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        terms: &'tree [TermSyntax],
        topics: Vec<SemanticObjectId>,
        topic_source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        let scopes = self.generated_prenex_formula_scopes_for_terms(terms)?;
        self.apply_generated_prenex_semantics_to_discourse_item(item, scopes, topics, topic_source)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(true)]
    pub(super) fn apply_generated_prenex_semantics_to_discourse_item(
        &mut self,
        item: SemanticObjectId,
        scopes: Vec<GeneratedPrenexFormulaScope>,
        topics: Vec<SemanticObjectId>,
        topic_source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        if scopes.is_empty() && topics.is_empty() {
            return Ok(());
        }
        let Some(content) = self
            .objects
            .get(&item)
            .and_then(|object| match object.as_data() {
                data!(SemanticObject::Utterance(node)) => node.content,
                data!(SemanticObject::Sequence(node)) => node.content,
                _ => None,
            })
        else {
            return Ok(());
        };
        let variables = generated_prenex_formula_scope_variables(&scopes);
        let content =
            self.strip_generated_implicit_quantifiers_from_content(content, &variables)?;
        self.strip_generated_implicit_quantifiers_for_variables_everywhere(&variables)?;
        let content =
            self.add_generated_topic_comment_links_to_content(content, &topics, topic_source)?;
        let wrapped = self.wrap_generated_content_with_prenex_scopes(content, scopes)?;
        if let Some(object) = self.objects.get_mut(&item) {
            match object.object_kind() {
                crate::model::SemanticObjectKind::Utterance => {
                    object.update_utterance(|node| node.with_data(data! { content: Some(wrapped) }))
                }
                crate::model::SemanticObjectKind::Sequence => {
                    object.update_sequence(|node| node.with_data(data! { content: Some(wrapped) }))
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[requires(content.object_kind() == crate::model::SemanticObjectKind::Formula || content.object_kind() == crate::model::SemanticObjectKind::Question || content.object_kind() == crate::model::SemanticObjectKind::Referent || content.object_kind() == crate::model::SemanticObjectKind::DisplayedContent || content.object_kind() == crate::model::SemanticObjectKind::Sequence || content.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == content.object_kind()) || ret.is_err())]
    pub(super) fn add_generated_topic_comment_links_to_content(
        &mut self,
        content: SemanticObjectId,
        topics: &[SemanticObjectId],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if topics.is_empty() {
            return Ok(content);
        }
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.add_generated_topic_comment_links_to_formula(content, topics, source)
            }
            crate::model::SemanticObjectKind::Question => {
                let body = self
                    .objects
                    .get(&content)
                    .and_then(SemanticObject::as_question)
                    .map(|question| question.body)
                    .ok_or_else(|| {
                        invalid_graph(format!(
                            "semantic builder could not find topic/comment question {content}"
                        ))
                    })?;
                let linked =
                    self.add_generated_topic_comment_links_to_formula(body, topics, source)?;
                if let Some(object) = self.objects.get_mut(&content) {
                    object.update_question(|node| node.with_data(data! { body: linked }));
                }
                Ok(content)
            }
            crate::model::SemanticObjectKind::Sequence => {
                self.add_generated_topic_comment_links_to_sequence(content, topics, source)?;
                Ok(content)
            }
            _ => Err(invalid_graph(
                "topic/comment prenex has no formula-, question-, or sequence-valued comment"
                    .to_owned(),
            )),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn add_generated_topic_comment_links_to_formula(
        &mut self,
        formula: SemanticObjectId,
        topics: &[SemanticObjectId],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if topics.is_empty() {
            return Ok(formula);
        }
        let comment_eventuality = match self.primary_eventuality_for_generated_formula(formula) {
            Some(eventuality) => eventuality,
            None => self.reified_eventuality_for_generated_content(formula, source.clone())?,
        };
        let mut children = Vec::with_capacity(topics.len() + 1);
        children.push(formula);
        for topic in topics {
            children.push(self.build_generated_topic_comment_formula(
                *topic,
                comment_eventuality,
                source.clone(),
            )?);
        }
        let linked = self.next_formula_id();
        self.insert(
            linked,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                children,
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(linked)
    }

    #[requires(sequence.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(true)]
    pub(super) fn add_generated_topic_comment_links_to_sequence(
        &mut self,
        sequence: SemanticObjectId,
        topics: &[SemanticObjectId],
        source: Option<crate::model::SemanticSource>,
    ) -> Result<(), SemanticsError> {
        if topics.is_empty() {
            return Ok(());
        }
        let comment_eventuality =
            self.reified_eventuality_for_generated_content(sequence, source.clone())?;
        let mut claims = Vec::with_capacity(topics.len());
        for topic in topics {
            claims.push(self.build_generated_topic_comment_formula(
                *topic,
                comment_eventuality,
                source.clone(),
            )?);
        }
        let object = self.objects.get_mut(&sequence).ok_or_else(|| {
            invalid_graph(format!(
                "semantic builder could not find topic/comment sequence {sequence}"
            ))
        })?;
        object.update_sequence(|node| {
            let mut data = node.into_data();
            data.connection_claims.extend(claims);
            SequenceNode::from_data(data)
        });
        Ok(())
    }

    #[requires(crate::model::argument_object_kind_can_fill(topic.object_kind()))]
    #[requires(semantic_id_is_eventuality(comment_eventuality))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_topic_comment_formula(
        &mut self,
        topic: SemanticObjectId,
        comment_eventuality: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        arguments.insert(argument_key(1), ArgumentValue::filled(topic, None));
        arguments.insert(
            argument_key(2),
            ArgumentValue::filled(comment_eventuality, None),
        );
        let predication = self.build_generated_predication_from_arguments(
            "topicOf".to_owned(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        if let Some(object) = self.objects.get_mut(&predication) {
            object.update_predication(|node| {
                node.with_data(data! { introduced_by: Some("zo'u".to_owned()) })
            });
        }
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(content.object_kind() == crate::model::SemanticObjectKind::Formula || content.object_kind() == crate::model::SemanticObjectKind::Question || content.object_kind() == crate::model::SemanticObjectKind::Referent || content.object_kind() == crate::model::SemanticObjectKind::DisplayedContent || content.object_kind() == crate::model::SemanticObjectKind::Sequence || content.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == content.object_kind()) || ret.is_err())]
    pub(super) fn wrap_generated_content_with_prenex_scopes(
        &mut self,
        content: SemanticObjectId,
        scopes: Vec<GeneratedPrenexFormulaScope>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.wrap_formula_with_generated_prenex_scopes(content, scopes)
            }
            crate::model::SemanticObjectKind::Question => {
                let body = self
                    .objects
                    .get(&content)
                    .and_then(|object| object.as_question())
                    .map(|question| question.body);
                if let Some(body) = body {
                    let wrapped = self.wrap_formula_with_generated_prenex_scopes(body, scopes)?;
                    if wrapped != body
                        && let Some(object) = self.objects.get_mut(&content)
                    {
                        object.update_question(|node| node.with_data(data! { body: wrapped }));
                    }
                }
                Ok(content)
            }
            _ => Ok(content),
        }
    }

    #[requires(content.object_kind() == crate::model::SemanticObjectKind::Formula || content.object_kind() == crate::model::SemanticObjectKind::Question || content.object_kind() == crate::model::SemanticObjectKind::Referent || content.object_kind() == crate::model::SemanticObjectKind::DisplayedContent || content.object_kind() == crate::model::SemanticObjectKind::Sequence || content.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[requires(variables.iter().all(|variable| matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == content.object_kind()) || ret.is_err())]
    pub(super) fn strip_generated_implicit_quantifiers_from_content(
        &mut self,
        content: SemanticObjectId,
        variables: &HashSet<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if variables.is_empty() {
            return Ok(content);
        }
        match content.object_kind() {
            crate::model::SemanticObjectKind::Formula => {
                self.strip_generated_implicit_quantifiers_for_variables(content, variables)
            }
            crate::model::SemanticObjectKind::Question => {
                let body = self
                    .objects
                    .get(&content)
                    .and_then(|object| object.as_question())
                    .map(|question| question.body);
                if let Some(body) = body {
                    let stripped =
                        self.strip_generated_implicit_quantifiers_for_variables(body, variables)?;
                    if stripped != body
                        && let Some(object) = self.objects.get_mut(&content)
                    {
                        object.update_question(|node| node.with_data(data! { body: stripped }));
                    }
                }
                Ok(content)
            }
            _ => Ok(content),
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(variables.iter().all(|variable| matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn strip_generated_implicit_quantifiers_for_variables(
        &mut self,
        formula: SemanticObjectId,
        variables: &HashSet<SemanticObjectId>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if variables.is_empty() {
            return Ok(formula);
        }
        let Some(object) = self.objects.get(&formula) else {
            return Ok(formula);
        };
        let quantified = object
            .as_formula()
            .and_then(|formula| match formula.as_data() {
                data!(FormulaNode::Quantified(node)) => Some(node),
                _ => None,
            });
        let is_implicit_target = quantified.is_some_and(|node| {
            node.operator == FormulaOperator::Exists
                && node.quantity.is_none()
                && variables.contains(&node.variable)
        });
        if is_implicit_target && let Some(body) = object.formula_body() {
            let stripped =
                self.strip_generated_implicit_quantifiers_for_variables(body, variables)?;
            self.replace_generated_formula_reference_everywhere(formula, stripped);
            self.objects.remove(&formula);
            return Ok(stripped);
        }
        let body = object.formula_body();
        let restriction = object.formula_restriction();
        let children = object.formula_children().to_vec();
        let binding_restrictions = object
            .as_formula()
            .and_then(|formula| match formula.as_data() {
                data!(FormulaNode::QuantifierBundle(node)) => Some(&node.bindings),
                _ => None,
            })
            .into_iter()
            .flatten()
            .map(|binding| binding.restriction)
            .collect::<Vec<_>>();
        if let Some(body) = body {
            let stripped =
                self.strip_generated_implicit_quantifiers_for_variables(body, variables)?;
            if stripped != body
                && let Some(object) = self.objects.get_mut(&formula)
            {
                object.update_formula(|formula| match formula.into_data() {
                    data!(FormulaNode::Quantified(node)) => new!(FormulaNode::Quantified(
                        node.with_data(data! { body: stripped })
                    )),
                    data!(FormulaNode::QuantifierBundle(node)) => new!(
                        FormulaNode::QuantifierBundle(node.with_data(data! { body: stripped }))
                    ),
                    data!(FormulaNode::RespectivelyDistribution(node)) => {
                        new!(FormulaNode::RespectivelyDistribution(
                            node.with_data(data! { body: stripped })
                        ))
                    }
                    data => FormulaNode::from_data(data),
                });
            }
        }
        if let Some(restriction) = restriction {
            let stripped =
                self.strip_generated_implicit_quantifiers_for_variables(restriction, variables)?;
            if stripped != restriction
                && let Some(object) = self.objects.get_mut(&formula)
            {
                object.update_formula(|formula| match formula.into_data() {
                    data!(FormulaNode::Quantified(node)) => new!(FormulaNode::Quantified(
                        node.with_data(data! { restriction: Some(stripped) })
                    )),
                    data => FormulaNode::from_data(data),
                });
            }
        }
        if !binding_restrictions.is_empty() {
            let mut stripped_restrictions = Vec::with_capacity(binding_restrictions.len());
            let mut changed = false;
            for restriction in binding_restrictions {
                let Some(restriction) = restriction else {
                    stripped_restrictions.push(None);
                    continue;
                };
                let stripped = self
                    .strip_generated_implicit_quantifiers_for_variables(restriction, variables)?;
                changed |= stripped != restriction;
                stripped_restrictions.push(Some(stripped));
            }
            if changed && let Some(object) = self.objects.get_mut(&formula) {
                object.update_formula(|formula| match formula.into_data() {
                    data!(FormulaNode::QuantifierBundle(node)) => {
                        let mut node_data = node.into_data();
                        for (binding, restriction) in
                            node_data.bindings.iter_mut().zip(stripped_restrictions)
                        {
                            *binding = binding
                                .clone()
                                .with_data(data! { restriction: restriction });
                        }
                        new!(FormulaNode::QuantifierBundle(
                            QuantifierBundleFormulaNode::from_data(node_data)
                        ))
                    }
                    data => FormulaNode::from_data(data),
                });
            }
        }
        if !children.is_empty() {
            let mut stripped_children = Vec::with_capacity(children.len());
            let mut changed = false;
            for child in children {
                let stripped =
                    self.strip_generated_implicit_quantifiers_for_variables(child, variables)?;
                changed |= stripped != child;
                stripped_children.push(stripped);
            }
            if changed && let Some(object) = self.objects.get_mut(&formula) {
                object.update_formula(|formula| match formula.into_data() {
                    data!(FormulaNode::Connective(node)) => new!(FormulaNode::Connective(
                        node.with_data(data! { children: stripped_children })
                    )),
                    data => FormulaNode::from_data(data),
                });
            }
        }
        Ok(formula)
    }

    #[requires(variables.iter().all(|variable| matches!(variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter)))]
    #[ensures(true)]
    pub(super) fn strip_generated_implicit_quantifiers_for_variables_everywhere(
        &mut self,
        variables: &HashSet<SemanticObjectId>,
    ) -> Result<(), SemanticsError> {
        if variables.is_empty() {
            return Ok(());
        }
        let formula_ids = self
            .objects
            .keys()
            .copied()
            .filter(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)
            .collect::<Vec<_>>();
        for formula in formula_ids {
            if self.objects.contains_key(&formula) {
                self.strip_generated_implicit_quantifiers_for_variables(formula, variables)?;
            }
        }
        self.implicit_existential_variables
            .retain(|existential| !variables.contains(&existential.variable));
        self.recorded_implicit_existential_variables
            .retain(|variable| !variables.contains(variable));
        Ok(())
    }

    #[requires(old_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(new_id.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn replace_generated_formula_reference_everywhere(
        &mut self,
        old_id: SemanticObjectId,
        new_id: SemanticObjectId,
    ) {
        if old_id == new_id {
            return;
        }
        for object in self.objects.values_mut() {
            if object.as_utterance().is_some() {
                object.update_utterance(|node| {
                    let mut data = node.into_data();
                    replace_generated_formula_option(&mut data.content, old_id, new_id);
                    UtteranceNode::from_data(data)
                });
            } else if object.as_sequence().is_some() {
                object.update_sequence(|node| {
                    let mut data = node.into_data();
                    replace_generated_formula_option(&mut data.content, old_id, new_id);
                    replace_generated_formula_vec(&mut data.connection_claims, old_id, new_id);
                    SequenceNode::from_data(data)
                });
            } else if object.as_eventuality().is_some() {
                object.update_eventuality(|node| {
                    let mut data = node.into_data();
                    replace_generated_descriptor_formula_references(
                        &mut data.descriptor,
                        old_id,
                        new_id,
                    );
                    replace_generated_relative_clause_formula_references(
                        &mut data.relative_clauses,
                        old_id,
                        new_id,
                    );
                    replace_generated_formula_option(&mut data.content, old_id, new_id);
                    replace_generated_formula_option(&mut data.body, old_id, new_id);
                    replace_generated_formula_option(&mut data.target, old_id, new_id);
                    EventualityNode::from_data(data)
                });
            } else if object.as_referent().is_some() {
                object.update_referent(|node| {
                    let mut data = node.into_data();
                    replace_generated_descriptor_formula_references(
                        &mut data.descriptor,
                        old_id,
                        new_id,
                    );
                    replace_generated_relative_clause_formula_references(
                        &mut data.relative_clauses,
                        old_id,
                        new_id,
                    );
                    replace_generated_formula_option(&mut data.body, old_id, new_id);
                    replace_generated_formula_option(&mut data.abstracted, old_id, new_id);
                    replace_generated_formula_option(&mut data.target, old_id, new_id);
                    ReferentNode::from_data(data)
                });
            } else if object.as_predication().is_some() {
                object.update_predication(|node| {
                    let mut data = node.into_data();
                    replace_generated_predication_formula_references(&mut data, old_id, new_id);
                    PredicationNode::from_data(data)
                });
            } else if object.as_formula().is_some() {
                object.update_formula(|formula| {
                    replace_generated_formula_node_references(formula, old_id, new_id)
                });
            } else if object.as_sign().is_some() {
                object.update_sign(|node| {
                    let mut data = node.into_data();
                    replace_generated_descriptor_formula_references(
                        &mut data.descriptor,
                        old_id,
                        new_id,
                    );
                    replace_generated_formula_option(&mut data.target, old_id, new_id);
                    SignNode::from_data(data)
                });
            } else if object.as_displayed_content().is_some() {
                object.update_displayed_content(|node| {
                    let mut data = node.into_data();
                    if data.target == old_id {
                        data.target = new_id;
                    }
                    DisplayedContentNode::from_data(data)
                });
            } else if object.as_question().is_some() {
                object.update_question(|node| {
                    let mut data = node.into_data();
                    if data.body == old_id {
                        data.body = new_id;
                    }
                    replace_generated_formula_option(&mut data.presupposed_answer, old_id, new_id);
                    QuestionNode::from_data(data)
                });
            }
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_prenex_scopes(
        &mut self,
        formula: SemanticObjectId,
        scopes: Vec<GeneratedPrenexFormulaScope>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut body = formula;
        for scope in scopes.into_iter().rev() {
            body = self.wrap_formula_with_generated_prenex_scope(body, scope)?;
        }
        Ok(body)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(topics.iter().all(|topic| crate::model::argument_object_kind_can_fill(topic.object_kind())))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_prenex_terms(
        &mut self,
        formula: SemanticObjectId,
        terms: &'tree [TermSyntax],
        topics: Vec<SemanticObjectId>,
        topic_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let scopes = self.generated_prenex_formula_scopes_for_terms(terms)?;
        let variables = generated_prenex_formula_scope_variables(&scopes);
        let body = self.strip_generated_implicit_quantifiers_for_variables(formula, &variables)?;
        self.strip_generated_implicit_quantifiers_for_variables_everywhere(&variables)?;
        let body =
            self.add_generated_topic_comment_links_to_formula(body, &topics, topic_source)?;
        self.wrap_formula_with_generated_prenex_scopes(body, scopes)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn generated_prenex_formula_scopes_for_terms<'syntax: 'tree>(
        &mut self,
        terms: &'syntax [TermSyntax],
    ) -> Result<Vec<GeneratedPrenexFormulaScope>, SemanticsError> {
        let mut scopes = Vec::new();
        for term in terms {
            if let Some(scope) = self.generated_prenex_formula_scope_for_term(term)? {
                scopes.push(scope);
            }
        }
        Ok(scopes)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn push_generated_prenex_term_bindings(
        &mut self,
        terms: &'tree [TermSyntax],
    ) -> Result<GeneratedPrenexContext, SemanticsError> {
        let mut pushed = Vec::new();
        let mut topics = Vec::new();
        for term in terms {
            let Some(sumti) = generated_prenex_binding_sumti_for_term(term)? else {
                continue;
            };
            self.apply_generated_prenex_goi_assignments_for_sumti(sumti)?;
            if let Some(pro_sumti) = generated_prenex_binding_pro_sumti_for_sumti(sumti) {
                let key = token_text(&pro_sumti.0.value);
                let source = self.source_for_node(pro_sumti, "sumti");
                let scope_key = self.source_key_for_node(sumti);
                self.prenex_pro_sumti_bindings
                    .entry(key.clone())
                    .or_default()
                    .push(new!(GeneratedPrenexProSumtiBinding {
                        variable: None,
                        word: key.clone(),
                        source,
                        scope_key,
                    }));
                pushed.push(new!(GeneratedPrenexPushedBinding::ProSumti(key)));
                continue;
            }
            if let Some(relation_variable) =
                relation_variable_syntax_from_no_gadri_prenex_sumti(sumti)?
            {
                let parameter = self
                    .build_relation_variable_parameter_for_generated_relation_parameter_syntax(
                        relation_variable,
                    )?
                    .0;
                let key = token_text(generated_relation_parameter_token(relation_variable));
                self.prenex_relation_variable_bindings
                    .entry(key.clone())
                    .or_default()
                    .push(new!(GeneratedPrenexRelationVariableBinding {
                        parameter,
                        word: key.clone(),
                    }));
                pushed.push(new!(GeneratedPrenexPushedBinding::RelationVariable(key)));
                continue;
            }
            if generated_prenex_topic_sumti_for_term(term)?.is_some() {
                topics.push(self.build_sumti_referent(sumti)?);
            }
        }
        Ok(new!(GeneratedPrenexContext {
            pushed_bindings: pushed,
            topics,
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn apply_generated_prenex_goi_assignments_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<(), SemanticsError> {
        let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) else {
            return Ok(());
        };
        if generated_goi_assignment_clause(relative_clauses).is_none() {
            return Ok(());
        }
        let _ = self.build_generated_goi_associated_referent(sumti)?;
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn pop_generated_prenex_scope_bindings(
        &mut self,
        pushed: Vec<GeneratedPrenexPushedBinding>,
    ) {
        for binding in pushed.into_iter().rev() {
            match binding.into_data() {
                data!(GeneratedPrenexPushedBinding::ProSumti(key)) => {
                    if let Some(bindings) = self.prenex_pro_sumti_bindings.get_mut(&key) {
                        bindings.pop();
                        if bindings.is_empty() {
                            self.prenex_pro_sumti_bindings.remove(&key);
                        }
                    }
                }
                data!(GeneratedPrenexPushedBinding::RelationVariable(key)) => {
                    if let Some(bindings) = self.prenex_relation_variable_bindings.get_mut(&key) {
                        bindings.pop();
                        if bindings.is_empty() {
                            self.prenex_relation_variable_bindings.remove(&key);
                        }
                    }
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn generated_prenex_formula_scope_for_term<'syntax: 'tree>(
        &mut self,
        term: &'syntax TermSyntax,
    ) -> Result<Option<GeneratedPrenexFormulaScope>, SemanticsError> {
        let simple = generated_simple_term_for_assignment(term)?;
        match simple {
            SimpleTermSyntax::NaKuTerm(_) | SimpleTermSyntax::BareNaTerm(_) => {
                Ok(Some(GeneratedPrenexFormulaScope::Negation {
                    source: self.source_for_node(term, "prenex-negation"),
                }))
            }
            SimpleTermSyntax::SumtiTerm(SumtiTermSyntax(sumti)) => {
                self.generated_prenex_formula_scope_for_sumti(sumti)
            }
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => match term.sumti.as_ref() {
                TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                    self.generated_prenex_formula_scope_for_sumti(sumti)
                }
                TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => Ok(None),
            },
            SimpleTermSyntax::TaggedSumtiTerm(term) => match term.sumti.as_ref() {
                TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                    self.generated_prenex_formula_scope_for_sumti(sumti)
                }
                TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => Ok(None),
            },
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn generated_prenex_formula_scope_for_sumti<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<Option<GeneratedPrenexFormulaScope>, SemanticsError> {
        if let Some(scope) = self.build_generated_prenex_relation_variable_scope_for_sumti(sumti)? {
            return Ok(Some(GeneratedPrenexFormulaScope::Quantifier(scope)));
        }
        self.build_generated_prenex_quantifier_scope_for_sumti(sumti)
            .map(|scope| scope.map(GeneratedPrenexFormulaScope::Quantifier))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| scope.as_ref().is_none_or(|scope| scope.variable.object_kind() == crate::model::SemanticObjectKind::Parameter && scope.quantity.is_some_and(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))) || ret.is_err())]
    pub(super) fn build_generated_prenex_relation_variable_scope_for_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
    ) -> Result<Option<GeneratedPrenexQuantifierScope>, SemanticsError> {
        let Some(description) = no_gadri_description_from_sumti(sumti)? else {
            return Ok(None);
        };
        if description.relative_clauses.is_some() {
            return Ok(None);
        }
        let Some(relation_variable) =
            relation_variable_syntax_from_generated_selbri(&description.selbri)?
        else {
            return Ok(None);
        };
        let (parameter, _) = self
            .build_relation_variable_parameter_for_generated_relation_parameter_syntax(
                relation_variable,
            )?;
        let quantity = self.build_quantity_for_quantifier(&description.quantifier)?;
        Ok(Some(new!(GeneratedPrenexQuantifierScope {
            operator: generated_quantifier_formula_operator(&description.quantifier),
            variable: parameter,
            quantity: Some(quantity),
            source: self.source_for_node(description, "quantifier-scope"),
            relative_clause_restrictions: Vec::new(),
        })))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| scope.as_ref().is_none_or(|scope| scope.variable.object_kind() == crate::model::SemanticObjectKind::Referent && scope.quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))) || ret.is_err())]
    pub(super) fn build_generated_prenex_quantifier_scope_for_sumti<'syntax: 'tree>(
        &mut self,
        sumti: &'syntax SumtiSyntax,
    ) -> Result<Option<GeneratedPrenexQuantifierScope>, SemanticsError> {
        let quantified_sumti = generated_quantified_sumti_from_sumti(sumti);
        let mut bare_pro_sumti = None;
        let (operator, quantity, source) = if let Some(quantified_sumti) = quantified_sumti {
            if generated_quantified_da_series_pro_sumti(quantified_sumti).is_none() {
                return Ok(None);
            }
            (
                generated_quantifier_formula_operator(&quantified_sumti.quantifier),
                Some(self.build_quantity_for_quantifier(&quantified_sumti.quantifier)?),
                self.source_for_node(quantified_sumti, "quantifier-scope"),
            )
        } else {
            let Some(pro_sumti) = generated_prenex_da_series_pro_sumti_from_sumti(sumti) else {
                return Ok(None);
            };
            bare_pro_sumti = Some(pro_sumti);
            (
                FormulaOperator::Exists,
                None,
                self.source_for_node(pro_sumti, "quantifier-scope"),
            )
        };
        let variable = if let Some(pro_sumti) = bare_pro_sumti {
            self.build_scoped_prenex_da_series_variable_for_generated_sumti(sumti, pro_sumti)?
        } else {
            self.build_scoped_argument_variable_for_generated_sumti(sumti)?
        };
        let relative_clause_restrictions =
            if let Some(relative_clauses) = generated_sumti_relative_clause_list(sumti) {
                self.lower_generated_relative_clause_list(relative_clauses, variable)?
                    .into_iter()
                    .map(|clause| clause.body)
                    .collect()
            } else {
                Vec::new()
            };
        Ok(Some(new!(GeneratedPrenexQuantifierScope {
            operator,
            variable,
            quantity,
            source,
            relative_clause_restrictions,
        })))
    }

    #[requires(pro_sumti.0.value.cmavo().is_some_and(|cmavo| matches!(cmavo, Cmavo::Da | Cmavo::De | Cmavo::Di)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent) || ret.is_err())]
    pub(super) fn build_scoped_prenex_da_series_variable_for_generated_sumti(
        &mut self,
        sumti: &'tree SumtiSyntax,
        pro_sumti: &'tree ProSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(key) = self.source_key_for_node(sumti)
            && let Some(id) = self.scoped_argument_variables.get(&key)
        {
            return Ok(*id);
        }
        let id = self.build_scoped_generated_pro_sumti_variable(
            pro_sumti,
            generated_sumti_quantified_variable_sort(sumti),
        )?;
        if let Some(key) = self.source_key_for_node(sumti) {
            self.scoped_argument_variables.insert(key, id);
        }
        Ok(id)
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_prenex_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: GeneratedPrenexFormulaScope,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match scope {
            GeneratedPrenexFormulaScope::Negation { source } => {
                self.build_unary_formula(FormulaOperator::Not, formula, source)
            }
            GeneratedPrenexFormulaScope::Quantifier(scope) => {
                self.wrap_formula_with_generated_prenex_quantifier_scope(formula, scope)
            }
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(matches!(scope.variable.object_kind(), crate::model::SemanticObjectKind::Referent | crate::model::SemanticObjectKind::Parameter))]
    #[requires(scope.quantity.is_none_or(|quantity| quantity.object_kind() == crate::model::SemanticObjectKind::Quantity))]
    #[requires(scope.relative_clause_restrictions.iter().all(|restriction| restriction.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_formula_with_generated_prenex_quantifier_scope(
        &mut self,
        formula: SemanticObjectId,
        scope: GeneratedPrenexQuantifierScope,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let scope = scope.into_data();
        let restriction =
            self.combine_generated_restriction_formulas(scope.relative_clause_restrictions)?;
        let scoped = self.next_formula_id();
        self.insert(
            scoped,
            SemanticObject::quantified_formula(
                scope.operator,
                scope.variable,
                restriction,
                formula,
                scope.quantity,
                scope.source,
                Vec::new(),
            ),
        )?;
        Ok(scoped)
    }

    #[requires(utterance_id.object_kind() == crate::model::SemanticObjectKind::Utterance)]
    #[ensures(ret.as_ref().is_ok_and(|id| *id == utterance_id) || ret.is_err())]
    pub(super) fn insert_generated_utterance(
        &mut self,
        utterance_id: SemanticObjectId,
        force: UtteranceForce,
        content: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let locution = self.next_locution_id();
        self.insert(
            locution,
            SemanticObject::referential_eventuality(
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Utterance) || ret.is_err())]
    pub(super) fn insert_generated_utterance_after_locution(
        &mut self,
        force: UtteranceForce,
        content: Option<SemanticObjectId>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let locution = self.next_locution_id();
        self.insert(
            locution,
            SemanticObject::referential_eventuality(
                EventualityClass::Locution,
                Some(Actuality {
                    kind: ActualityKind::Actual,
                }),
                source.clone(),
            ),
        )?;
        let utterance = self.next_utterance_id();
        self.insert(
            utterance,
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
        Ok(utterance)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_i_statement_connection_sequence(
        &mut self,
        connection: &'tree IStatementConnectionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut leading_spans = Vec::new();
        collect_generated_node_spans(&connection.leading_statement, &mut leading_spans);
        let (leading_item, leading_formula) = self.build_generated_statement_base_connection_item(
            &connection.leading_statement,
            UtteranceForce::Assert,
        )?;
        let leading = new!(GeneratedStatementConnectionOperand {
            item: leading_item,
            formula: leading_formula,
            last_item: leading_item,
            spans: leading_spans,
        });
        let (mut current, mut index) =
            self.build_generated_statement_bo_connection_group(connection, leading, 0)?;
        while index < connection.continuations.len() {
            let continuation = &connection.continuations[index];
            let tail =
                self.build_generated_statement_connection_tail_operand(continuation, current)?;
            index += 1;
            let (right, next_index) = self.build_generated_statement_bo_connection_group(
                connection,
                tail.operand.clone(),
                index,
            )?;
            index = next_index;
            current = self.combine_generated_statement_connection_operands(
                tail.left_operand,
                tail.i,
                tail.connective,
                tail.trailing_statement,
                tail.spans.clone(),
                right,
            )?;
        }
        Ok(current.item)
    }

    #[requires(start <= connection.continuations.len())]
    #[ensures(ret.as_ref().is_ok_and(|(_, index)| *index <= connection.continuations.len()) || ret.is_err())]
    pub(super) fn build_generated_statement_bo_connection_group(
        &mut self,
        connection: &'tree IStatementConnectionSyntax,
        left: GeneratedStatementConnectionOperand,
        start: usize,
    ) -> Result<(GeneratedStatementConnectionOperand, usize), SemanticsError> {
        if start >= connection.continuations.len() {
            return Ok((left, start));
        }
        let continuation = &connection.continuations[start];
        let (_, _, connective, _) = statement_connection_tail_parts(continuation)?;
        if !generated_i_statement_connective_has_bo(connective) {
            return Ok((left, start));
        }
        let tail = self.build_generated_statement_connection_tail_operand(continuation, left)?;
        let (right, index) = self.build_generated_statement_bo_connection_group(
            connection,
            tail.operand.clone(),
            start + 1,
        )?;
        let combined = self.combine_generated_statement_connection_operands(
            tail.left_operand,
            tail.i,
            tail.connective,
            tail.trailing_statement,
            tail.spans.clone(),
            right,
        )?;
        Ok((combined, index))
    }

    #[requires(left_operand.item.object_kind() == crate::model::SemanticObjectKind::Utterance || left_operand.item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|tail| (tail.left_operand.item.object_kind() == crate::model::SemanticObjectKind::Utterance || tail.left_operand.item.object_kind() == crate::model::SemanticObjectKind::Sequence) && (tail.operand.item.object_kind() == crate::model::SemanticObjectKind::Utterance || tail.operand.item.object_kind() == crate::model::SemanticObjectKind::Sequence)) || ret.is_err())]
    pub(super) fn build_generated_statement_connection_tail_operand<'syntax: 'tree>(
        &mut self,
        continuation: &'syntax IStatementConnectionTailSyntax,
        mut left_operand: GeneratedStatementConnectionOperand,
    ) -> Result<GeneratedStatementConnectionTail<'syntax>, SemanticsError> {
        let (pending, i, connective, trailing_statement) =
            statement_connection_tail_parts(continuation)?;
        for pending in pending {
            let item = self.wrap_generated_pending_statement_connection_item(
                left_operand.item,
                &pending.i,
                &pending.connective,
                ElidedConnectionOperand::FollowingDiscourse,
            )?;
            let formula = self.content_formula_for_generated_discourse_item(item);
            let left_data = left_operand.into_data();
            left_operand = new!(GeneratedStatementConnectionOperand {
                item,
                formula,
                last_item: left_data.last_item,
                spans: left_data.spans,
            });
        }
        let mut spans = Vec::new();
        collect_generated_node_spans(continuation, &mut spans);
        let mut trailing_spans = Vec::new();
        collect_generated_node_spans(trailing_statement, &mut trailing_spans);
        self.previous_utterance = Some(left_operand.item);
        self.next_utterance = None;
        let (trailing_item, trailing_formula) = self
            .build_generated_statement_after_i_connection_item(
                trailing_statement,
                UtteranceForce::Assert,
            )?;
        Ok(GeneratedStatementConnectionTail {
            left_operand,
            i,
            connective,
            trailing_statement,
            spans,
            operand: new!(GeneratedStatementConnectionOperand {
                item: trailing_item,
                formula: trailing_formula,
                last_item: trailing_item,
                spans: trailing_spans,
            }),
        })
    }

    #[requires(left.item.object_kind() == crate::model::SemanticObjectKind::Utterance || left.item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(right.item.object_kind() == crate::model::SemanticObjectKind::Utterance || right.item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|operand| operand.item.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn combine_generated_statement_connection_operands(
        &mut self,
        left: GeneratedStatementConnectionOperand,
        _i: &Token,
        connective: &'tree IStatementConnectiveSyntax,
        trailing_statement: &'tree StatementAfterIConnectiveSyntax,
        tail_spans: Vec<SourceSpan>,
        right: GeneratedStatementConnectionOperand,
    ) -> Result<GeneratedStatementConnectionOperand, SemanticsError> {
        let mut spans = Vec::with_capacity(left.spans.len() + tail_spans.len() + right.spans.len());
        spans.extend(left.spans.iter().cloned());
        spans.extend(tail_spans);
        spans.extend(right.spans.iter().cloned());
        let source = self.source_for_generated_spans(&spans, "statement-connection");
        let mut diagnostics = Vec::new();
        let has_logical_component =
            generated_i_statement_connective_has_logical_component(connective);
        let question_start = self.direct_question_slots.len();
        let content = if has_logical_component {
            if let (Some(left_formula), Some(right_formula)) = (left.formula, right.formula) {
                Some(
                    self.build_binary_formula_for_generated_statement_connective(
                        connective,
                        left_formula,
                        right_formula,
                        source.clone(),
                    )?,
                )
            } else {
                diagnostics.push(diagnostic(
                    "logical generated statement connection could not find formula-bearing statements to connect",
                ));
                None
            }
        } else {
            None
        };
        let question_slots = self.direct_question_slots.split_off(question_start);
        let mut connection_claims = Vec::new();
        let claim_spec = generated_modal_statement_connection_spec(connective)
            .map(|spec| (spec, source.clone()))
            .or_else(|| {
                generated_text_group_statement_connection_spec(trailing_statement).map(
                    |(tense_modal, spec)| {
                        (
                            spec,
                            self.source_for_node(tense_modal, "statement-connection-claim"),
                        )
                    },
                )
            });
        if let Some((spec, claim_source)) = claim_spec {
            match self.build_generated_modal_statement_connection_claim(
                left.item,
                right.item,
                &spec,
                claim_source,
            )? {
                Some(claim) => connection_claims.push(claim),
                None => diagnostics.push(diagnostic(
                    "modal generated statement connection could not find formula-bearing bridi events to relate",
                )),
            }
        }
        let nonlogical_connection = if has_logical_component {
            None
        } else {
            Some(generated_i_statement_nonlogical_connection(connective)?)
        };
        let question_source = source.clone();
        let item = self.insert_generated_statement_connection_sequence(
            left.item,
            right.item,
            content,
            connection_claims,
            nonlogical_connection,
            source,
            diagnostics,
        )?;
        if !question_slots.is_empty()
            && let Some(formula) = content
        {
            self.set_generated_statement_question_content(
                item,
                formula,
                question_slots,
                &[left.item, right.item],
                question_source,
            )?;
        }
        self.attach_generated_statement_separator_indicators_to_discourse_item_with_target(
            right.last_item,
            _i,
            generated_i_statement_connective_core(connective)?,
            false,
            content,
        )?;
        Ok(new!(GeneratedStatementConnectionOperand {
            item,
            formula: content,
            last_item: right.last_item,
            spans,
        }))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_forethought_statement_connection_sequence(
        &mut self,
        connection: &'tree ForethoughtStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (item, _formula) =
            self.build_forethought_statement_connection_item(connection, UtteranceForce::Assert)?;
        if item.object_kind() == crate::model::SemanticObjectKind::Sequence {
            Ok(item)
        } else {
            Err(invalid_graph(format!(
                "forethought statement connection built non-sequence item {item}"
            )))
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(item, formula)| (item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence) && formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_forethought_statement_connection_item(
        &mut self,
        connection: &'tree ForethoughtStatementSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, Option<SemanticObjectId>), SemanticsError> {
        let source = self.source_for_node(connection, "forethought-statement-connection");
        let (first_item, first_formula) =
            self.build_generated_statement_connection_item(&connection.first, force)?;
        let (branch_item, branch_formula) = self.build_generated_statement_connection_item(
            &connection.first_branch.statement,
            UtteranceForce::Assert,
        )?;
        let mut items = vec![first_item, branch_item];
        let logical = generated_modal_forethought_connective_is_logical(&connection.gek);
        let mut formula = if logical {
            match (first_formula, branch_formula) {
                (Some(left), Some(right)) => Some(
                    self.build_binary_formula_for_generated_forethought_statement_connective(
                        &connection.gek,
                        &connection.first_branch.gik,
                        left,
                        right,
                        source.clone(),
                    )?,
                ),
                _ => None,
            }
        } else {
            None
        };
        let mut connection_claims = Vec::new();
        let mut diagnostics = Vec::new();
        if !logical {
            if let Some(spec) =
                generated_modal_statement_connection_spec_for_tense_modal(&connection.gek)
            {
                match self.build_generated_modal_statement_connection_claim(
                    first_item,
                    branch_item,
                    &spec,
                    source.clone(),
                )? {
                    Some(claim) => connection_claims.push(claim),
                    None => diagnostics.push(diagnostic(
                        "modal forethought statement connection could not find discourse items to relate",
                    )),
                }
            } else {
                diagnostics.push(diagnostic(
                    "nonlogical forethought statement connection is not fully lowered",
                ));
            }
        } else if formula.is_none() {
            diagnostics.push(diagnostic(
                "logical forethought statement connection could not find formula-bearing statements to connect",
            ));
        }

        let mut previous_item = branch_item;
        for branch in &connection.additional_branches {
            let (next_item, next_formula) = self.build_generated_statement_connection_item(
                &branch.statement,
                UtteranceForce::Assert,
            )?;
            if logical {
                if let (Some(left), Some(right)) = (formula, next_formula) {
                    formula = Some(
                        self.build_binary_formula_for_generated_extra_forethought_statement_connective(
                            &connection.gek,
                            &branch.gik,
                            left,
                            right,
                            source.clone(),
                        )?,
                    );
                } else {
                    diagnostics.push(diagnostic(
                        "logical forethought statement connection could not find formula-bearing statements to connect",
                    ));
                    formula = None;
                }
            } else if let Some(spec) =
                generated_modal_statement_connection_spec_for_tense_modal(&connection.gek)
            {
                match self.build_generated_modal_statement_connection_claim(
                    previous_item,
                    next_item,
                    &spec,
                    source.clone(),
                )? {
                    Some(claim) => connection_claims.push(claim),
                    None => diagnostics.push(diagnostic(
                        "modal forethought statement connection could not find discourse items to relate",
                    )),
                }
            }
            items.push(next_item);
            previous_item = next_item;
        }

        if formula.is_some() {
            for item in &items {
                self.mark_generated_discourse_item_subordinated(*item);
            }
        }
        let sequence = self.next_sequence_id();
        let mut object = SemanticObject::sequence_with_connection_claims(
            items,
            SequenceRelation::SameTopicContinuation,
            connection_claims,
            source,
            diagnostics,
        );
        object.update_sequence(|node| node.with_data(data! { content: formula }));
        self.insert(sequence, object)?;
        Ok((sequence, formula))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(item, formula)| (item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence) && formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_statement_connection_item(
        &mut self,
        statement: &'tree StatementSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, Option<SemanticObjectId>), SemanticsError> {
        match statement {
            StatementSyntax::StatementBase(statement) => {
                self.build_generated_statement_base_connection_item(statement, force)
            }
            StatementSyntax::IStatementConnection(statement) => {
                let item = self.build_i_statement_connection_sequence(statement)?;
                let formula = self.content_formula_for_generated_discourse_item(item);
                Ok((item, formula))
            }
            StatementSyntax::PreposedIStatementConnection(statement) => {
                let item = self.build_preposed_i_statement_connection_sequence(statement)?;
                let formula = self.content_formula_for_generated_discourse_item(item);
                Ok((item, formula))
            }
        }
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_forethought_statement_connective(
        &mut self,
        connective: &'tree ModalForethoughtConnectiveSyntax,
        gik: &'tree GikConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_binary_formula_for_generated_forethought_statement_connective_core(
            connective,
            generated_modal_forethought_connective_negates_left(connective),
            generated_gik_connective_negates_right(gik),
            generated_modal_forethought_pair_source(connective, gik),
            "statement",
            left,
            right,
            source,
        )
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_extra_forethought_statement_connective(
        &mut self,
        connective: &'tree ModalForethoughtConnectiveSyntax,
        gik: &'tree ZantufaExtraGikConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let connector_source = format!(
            "{} {}",
            generated_modal_forethought_connective_source(connective),
            token_text(&gik.0.value)
        );
        self.build_binary_formula_for_generated_forethought_statement_connective_core(
            connective,
            false,
            false,
            connector_source,
            "statement",
            left,
            right,
            source,
        )
    }

    #[requires(!connector_source.is_empty())]
    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_forethought_statement_connective_core(
        &mut self,
        connective: &'tree ModalForethoughtConnectiveSyntax,
        left_negated: bool,
        right_negated: bool,
        connector_source: String,
        connector_locus: &str,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.mark_generated_modal_forethought_whether_or_not_inert_operand(connective, left, right);
        let left = if left_negated {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right = if right_negated {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        let operator = generated_modal_forethought_connective_formula_operator(connective);
        let children = if generated_modal_forethought_connective_has_se(connective)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let parameter = self
            .build_generated_connective_question_parameter_for_modal_forethought_connective(
                connective,
            )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                if parameter.is_some() {
                    FormulaOperator::ConnectiveQuestion
                } else {
                    operator
                },
                children,
                Some(new!(Connector {
                    source: connector_source,
                    locus: connector_locus.to_owned(),
                    truth_table: generated_modal_forethought_connective_truth_table_with_negations(
                        connective,
                        left_negated,
                        right_negated,
                    ),
                    parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(item, formula)| (item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence) && formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_statement_base_connection_item(
        &mut self,
        statement: &'tree StatementBaseSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, Option<SemanticObjectId>), SemanticsError> {
        match statement {
            StatementBaseSyntax::BridiStatement(statement) => {
                let utterance = self.next_utterance_id();
                self.current_utterance = Some(utterance);
                let (item, formula) = self.build_bridi_utterance_with_force(
                    utterance,
                    bridi_from_bridi_statement(statement)?,
                    force,
                )?;
                Ok((item, Some(formula)))
            }
            StatementBaseSyntax::TextGroupStatement(statement) => {
                let item = self.build_generated_text_group_statement(statement)?;
                let formula = self.content_formula_for_generated_discourse_item(item);
                Ok((item, formula))
            }
            StatementBaseSyntax::PrenexStatement(statement) => {
                let item = self.build_discourse_item_for_generated_prenex_statement(statement)?;
                let formula = self.content_formula_for_generated_discourse_item(item);
                Ok((item, formula))
            }
            StatementBaseSyntax::ForethoughtStatement(statement) => {
                self.build_forethought_statement_connection_item(statement, force)
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|(item, formula)| (item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence) && formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_statement_after_i_connection_item(
        &mut self,
        statement: &'tree StatementAfterIConnectiveSyntax,
        force: UtteranceForce,
    ) -> Result<(SemanticObjectId, Option<SemanticObjectId>), SemanticsError> {
        match statement {
            StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
                let utterance = self.next_utterance_id();
                self.current_utterance = Some(utterance);
                let (item, formula) = self.build_bridi_utterance_with_force(
                    utterance,
                    bridi_from_bridi_statement(statement)?,
                    force,
                )?;
                Ok((item, Some(formula)))
            }
            StatementAfterIConnectiveSyntax::TextGroupStatement(statement) => {
                let item = self.build_generated_text_group_statement(statement)?;
                let formula = self.content_formula_for_generated_discourse_item(item);
                Ok((item, formula))
            }
            StatementAfterIConnectiveSyntax::ForethoughtStatement(statement) => {
                self.build_forethought_statement_connection_item(statement, force)
            }
        }
    }

    #[requires(left_item.object_kind() == crate::model::SemanticObjectKind::Utterance || left_item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(right_item.object_kind() == crate::model::SemanticObjectKind::Utterance || right_item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(content.is_none_or(|content| content.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn insert_generated_statement_connection_sequence(
        &mut self,
        left_item: SemanticObjectId,
        right_item: SemanticObjectId,
        content: Option<SemanticObjectId>,
        connection_claims: Vec<SemanticObjectId>,
        nonlogical_connection: Option<NonlogicalConnection>,
        source: Option<crate::model::SemanticSource>,
        diagnostics: Vec<crate::model::SemanticDiagnostic>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if content.is_some() {
            self.mark_generated_discourse_item_subordinated(left_item);
            self.mark_generated_discourse_item_subordinated(right_item);
        }
        let sequence = self.next_sequence_id();
        let mut object = SemanticObject::sequence_with_connection_claims(
            vec![left_item, right_item],
            SequenceRelation::SameTopicContinuation,
            connection_claims,
            source,
            diagnostics,
        );
        object.update_sequence(|node| {
            node.with_data(data! {
                content: content,
                nonlogical_connection: nonlogical_connection,
            })
        });
        self.insert(sequence, object)?;
        Ok(sequence)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(true)]
    pub(super) fn mark_generated_discourse_item_subordinated(&mut self, item: SemanticObjectId) {
        let content = if let Some(object) = self.objects.get_mut(&item) {
            if let Some(content) = object.as_utterance().and_then(|node| node.content) {
                object.update_utterance(|node| {
                    node.with_data(data! { force: UtteranceForce::Subordinated })
                });
                Some(content)
            } else {
                let content = object.as_sequence().and_then(|node| node.content);
                if object.as_sequence().is_some() {
                    object.update_sequence(|node| {
                        node.with_data(data! { force: Some(UtteranceForce::Subordinated) })
                    });
                }
                content
            }
        } else {
            None
        };
        if let Some(content) = content
            && content.object_kind() == crate::model::SemanticObjectKind::Formula
        {
            self.set_generated_pro_bridi_formula_predication_mode(content, PredicationMode::Inert);
        }
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(true)]
    pub(super) fn set_generated_pro_bridi_formula_predication_mode(
        &mut self,
        formula: SemanticObjectId,
        mode: PredicationMode,
    ) {
        let Some(object) = self.objects.get(&formula).cloned() else {
            return;
        };
        if let Some(predication) = object.formula_predication()
            && self
                .objects
                .get(&predication)
                .and_then(|object| object.as_predication())
                .and_then(|node| match node.relation.as_data() {
                    data!(PredicationRelation::Named { relation }) => Some(relation.as_str()),
                    data!(PredicationRelation::Parameter { .. }) => None,
                })
                .is_some_and(generated_relation_is_pro_bridi_label)
            && let Some(object) = self.objects.get_mut(&predication)
        {
            object.set_predication_mode(mode);
        }
        for child in object.formula_children().to_vec() {
            self.set_generated_pro_bridi_formula_predication_mode(child, mode);
        }
        if let Some(restriction) = object.formula_restriction() {
            self.set_generated_pro_bridi_formula_predication_mode(restriction, mode);
        }
        if let Some(body) = object.formula_body() {
            self.set_generated_pro_bridi_formula_predication_mode(body, mode);
        }
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_statement_connective(
        &mut self,
        connective: &'tree IStatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(connective_core) = generated_i_statement_connective_core(connective)? else {
            return Err(invalid_graph(
                "modal-only statement connective reached logical formula lowering".to_owned(),
            ));
        };
        let connector_source = generated_statement_connective_source(connective)?;
        self.build_binary_formula_for_generated_statement_connective_core_with_connector_source(
            connective_core,
            connector_source,
            left,
            right,
            source,
        )
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_statement_connective_core(
        &mut self,
        connective: &'tree StatementConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let connector_source = generated_statement_connective_core_source(connective)?;
        self.build_binary_formula_for_generated_statement_connective_core_with_connector_source(
            connective,
            connector_source,
            left,
            right,
            source,
        )
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(!connector_source.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_formula_for_generated_statement_connective_core_with_connector_source(
        &mut self,
        connective: &'tree StatementConnectiveSyntax,
        connector_source: String,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_statement_connective_formula_operator_for_core(connective);
        let truth_table = generated_statement_connective_core_truth_table(connective);
        if operator != FormulaOperator::ConnectiveQuestion && truth_table.is_none() {
            return Err(invalid_graph(
                "nonlogical statement connective reached logical formula lowering".to_owned(),
            ));
        }
        let parameter = build_generated_connective_question_parameter_for_statement_connective(
            self, connective,
        )?;
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
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: connector_source,
                    locus: "statement".to_owned(),
                    truth_table,
                    parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(child.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(generated_statement_connective_is_logical(connective))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_unary_formula_for_generated_statement_connective_core(
        &mut self,
        connective: &StatementConnectiveSyntax,
        child: SemanticObjectId,
        elided_operand: ElidedConnectionOperand,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let operator = generated_statement_connective_formula_operator_for_core(connective);
        let truth_table = generated_statement_connective_core_truth_table(connective);
        if operator != FormulaOperator::ConnectiveQuestion && truth_table.is_none() {
            return Err(invalid_graph(
                "nonlogical statement connective reached unary logical formula lowering".to_owned(),
            ));
        }
        let negates_present_operand = match elided_operand {
            ElidedConnectionOperand::PriorDiscourse => {
                generated_statement_connective_negates_right(connective)
            }
            ElidedConnectionOperand::FollowingDiscourse => {
                generated_statement_connective_negates_left(connective)
            }
        };
        let child = if negates_present_operand {
            self.build_unary_formula(FormulaOperator::Not, child, source.clone())?
        } else {
            child
        };
        let parameter = build_generated_connective_question_parameter_for_statement_connective(
            self, connective,
        )?;
        let connector_source = generated_statement_connective_core_source(connective)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                vec![child],
                Some(new!(Connector {
                    source: connector_source,
                    locus: "statement".to_owned(),
                    truth_table,
                    parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[requires(formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn insert_generated_contextual_statement_connection_sequence(
        &mut self,
        item: SemanticObjectId,
        formula: Option<SemanticObjectId>,
        connective: Option<&StatementConnectiveSyntax>,
        connection_claims: Vec<SemanticObjectId>,
        elided_operand: ElidedConnectionOperand,
        truth_question_source_orders: Vec<usize>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let logical = connective.is_some_and(generated_statement_connective_is_logical);
        let question_start = self.direct_question_slots.len();
        let mut diagnostics = Vec::new();
        let content = if logical {
            match formula {
                Some(formula) => Some(
                    self.build_unary_formula_for_generated_statement_connective_core(
                        connective.expect("logical flag requires a connective"),
                        formula,
                        elided_operand,
                        source.clone(),
                    )?,
                ),
                None => {
                    diagnostics.push(diagnostic(
                        "contextual logical statement connection has no formula-bearing present operand",
                    ));
                    None
                }
            }
        } else {
            None
        };
        let mut question_slots = self.direct_question_slots.split_off(question_start);
        question_slots.extend(
            truth_question_source_orders
                .into_iter()
                .map(|source_order| {
                    GeneratedDirectQuestionSlot::from_data(data!(GeneratedDirectQuestionSlot {
                        parameter: None,
                        kind: QuestionKind::Truth,
                        domain: SemanticSort::TruthValue,
                        source_order,
                    }))
                }),
        );
        let nonlogical_connection = connective
            .filter(|connective| !generated_statement_connective_is_logical(connective))
            .map(generated_statement_core_nonlogical_connection)
            .transpose()?;
        if content.is_none() && connection_claims.is_empty() && nonlogical_connection.is_none() {
            return Err(requires_discourse_context(
                "a logical statement connection whose present operand is non-propositional",
            ));
        }
        if content.is_some() {
            self.mark_generated_discourse_item_subordinated(item);
        }
        let sequence = self.next_sequence_id();
        let question_source = source.clone();
        let mut object = SemanticObject::sequence_with_connection_claims(
            vec![item],
            SequenceRelation::SameTopicContinuation,
            connection_claims,
            source,
            diagnostics,
        );
        object.update_sequence(|node| {
            node.with_data(data! {
                content: content,
                nonlogical_connection: nonlogical_connection,
                elided_connection_operand: Some(elided_operand),
            })
        });
        self.insert(sequence, object)?;
        if !question_slots.is_empty()
            && let Some(formula) = content
        {
            self.set_generated_statement_question_content(
                sequence,
                formula,
                question_slots,
                &[item],
                question_source,
            )?;
        }
        Ok(sequence)
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn wrap_generated_pending_statement_connection_item(
        &mut self,
        item: SemanticObjectId,
        i: &Token,
        connective: &'tree StatementConnectiveSyntax,
        elided_operand: ElidedConnectionOperand,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut spans = Vec::new();
        collect_generated_node_spans(i, &mut spans);
        collect_generated_node_spans(connective, &mut spans);
        let source = self.source_for_generated_spans(&spans, "statement-connection");
        let formula = self.content_formula_for_generated_discourse_item(item);
        self.attach_generated_statement_separator_indicators_to_discourse_item(
            item,
            i,
            Some(connective),
            false,
        )?;
        self.insert_generated_contextual_statement_connection_sequence(
            item,
            formula,
            Some(connective),
            Vec::new(),
            elided_operand,
            self.direct_truth_question_source_orders(i)
                .into_iter()
                .chain(self.direct_truth_question_source_orders(connective))
                .collect(),
            source,
        )
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn wrap_generated_leading_i_statement_connection_item(
        &mut self,
        item: SemanticObjectId,
        leading_i: &LeadingIStatementSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let Some(connective) = leading_i.connective.as_deref() else {
            self.attach_generated_statement_separator_indicators_to_discourse_item(
                item,
                &leading_i.i,
                None,
                false,
            )?;
            return Ok(item);
        };
        let core = match connective {
            IParagraphStatementConnectiveSyntax::IStandardParagraphStatementConnective(
                connective,
            ) => Some(statement_connective_from_paragraph_standard(
                &connective.connective,
            )),
            IParagraphStatementConnectiveSyntax::ITagBoParagraphStatementConnective(_) => None,
        };
        let modal_spec = generated_paragraph_modal_statement_connection_spec(connective);
        if core.is_none() && modal_spec.is_none() {
            return Err(requires_discourse_context(
                "text-initial BO grouping without a modal relation",
            ));
        }
        let mut spans = Vec::new();
        collect_generated_node_spans(&leading_i.i, &mut spans);
        collect_generated_node_spans(connective, &mut spans);
        let source = self.source_for_generated_spans(&spans, "statement-connection");
        let claims = match modal_spec.as_ref() {
            Some(spec) => self
                .build_generated_contextual_modal_statement_connection_claim(
                    item,
                    spec,
                    source.clone(),
                )?
                .into_iter()
                .collect(),
            None => Vec::new(),
        };
        self.attach_generated_statement_separator_indicators_to_discourse_item(
            item,
            &leading_i.i,
            core.as_ref(),
            false,
        )?;
        let formula = self.content_formula_for_generated_discourse_item(item);
        self.insert_generated_contextual_statement_connection_sequence(
            item,
            formula,
            core.as_ref(),
            claims,
            ElidedConnectionOperand::PriorDiscourse,
            self.direct_truth_question_source_orders(leading_i),
            source,
        )
    }

    #[requires(item.object_kind() == crate::model::SemanticObjectKind::Utterance || item.object_kind() == crate::model::SemanticObjectKind::Sequence)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn wrap_generated_text_leading_connective_item(
        &mut self,
        item: SemanticObjectId,
        connective: &TextLeadingConnectiveSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let TextLeadingConnectiveSyntax::StandardStatementConnective(connective) = connective
        else {
            return Err(requires_discourse_context(
                "text-initial CEhE termset connection",
            ));
        };
        let core = statement_connective_from_standard(connective);
        let source = self.source_for_node(connective, "statement-connection");
        let formula = self.content_formula_for_generated_discourse_item(item);
        self.insert_generated_contextual_statement_connection_sequence(
            item,
            formula,
            Some(&core),
            Vec::new(),
            ElidedConnectionOperand::PriorDiscourse,
            self.direct_truth_question_source_orders(connective),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Sequence) || ret.is_err())]
    pub(super) fn build_preposed_i_statement_connection_sequence(
        &mut self,
        connection: &'tree PreposedIStatementConnectionSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let logical = generated_statement_connective_is_logical(&connection.connective);
        let leading_bridi = bridi_from_statement_base(&connection.leading_statement)?;
        let trailing_bridi =
            bridi_from_statement_after_i_connective(&connection.trailing_statement)?;
        let leading_utterance = self.next_utterance_id();
        self.current_utterance = Some(leading_utterance);
        let (leading_item, leading_formula) = self.build_bridi_utterance_with_force(
            leading_utterance,
            leading_bridi,
            UtteranceForce::Assert,
        )?;
        let trailing_utterance = self.next_utterance_id();
        self.previous_utterance = Some(leading_item);
        self.current_utterance = Some(trailing_utterance);
        self.next_utterance = None;
        let (trailing_item, trailing_formula) = self.build_bridi_utterance_with_force(
            trailing_utterance,
            trailing_bridi,
            UtteranceForce::Assert,
        )?;
        self.attach_generated_statement_separator_indicators_to_discourse_item(
            trailing_item,
            &connection.i,
            Some(&connection.connective),
            false,
        )?;
        let question_start = self.direct_question_slots.len();
        let source = self.source_for_node(connection, "statement-connection");
        let formula = logical
            .then(|| {
                self.build_binary_formula_for_generated_statement_connective_core(
                    &connection.connective,
                    leading_formula,
                    trailing_formula,
                    source.clone(),
                )
            })
            .transpose()?;
        let mut question_slots = self.direct_question_slots.split_off(question_start);
        question_slots.extend(
            self.direct_truth_question_source_orders(&connection.i)
                .into_iter()
                .chain(self.direct_truth_question_source_orders(&connection.connective))
                .map(|source_order| {
                    GeneratedDirectQuestionSlot::from_data(data!(GeneratedDirectQuestionSlot {
                        parameter: None,
                        kind: QuestionKind::Truth,
                        domain: SemanticSort::TruthValue,
                        source_order,
                    }))
                }),
        );
        let question_source = self.source_for_node(connection, "statement-question");
        let nonlogical_connection = (!logical)
            .then(|| generated_statement_core_nonlogical_connection(&connection.connective))
            .transpose()?;
        let sequence = self.insert_generated_statement_connection_sequence(
            leading_item,
            trailing_item,
            formula,
            Vec::new(),
            nonlogical_connection,
            source,
            Vec::new(),
        )?;
        if !question_slots.is_empty()
            && let Some(formula) = formula
        {
            self.set_generated_statement_question_content(
                sequence,
                formula,
                question_slots,
                &[leading_item, trailing_item],
                question_source,
            )?;
        }
        Ok(sequence)
    }

    #[requires(self.objects.contains_key(&root))]
    #[ensures(self.objects.contains_key(&root))]
    #[expensive_ensures(reachable_generated_object_ids(&self.objects, root).len() == self.objects.len())]
    pub(super) fn prune_unreachable_objects(&mut self, root: SemanticObjectId) {
        let reachable = reachable_generated_object_ids(&self.objects, root);
        self.objects.retain(|id, _object| reachable.contains(id));
    }
}

#[requires(objects.contains_key(&root))]
#[ensures(ret.contains(&root))]
fn reachable_generated_object_ids(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    root: SemanticObjectId,
) -> HashSet<SemanticObjectId> {
    let mut reachable = HashSet::new();
    let mut stack = vec![root];
    while let Some(next) = stack.pop() {
        if reachable.insert(next)
            && let Some(object) = objects.get(&next)
        {
            let mut references = Vec::new();
            object.references_into(&mut references);
            stack.extend(references);
        }
    }
    reachable
}
