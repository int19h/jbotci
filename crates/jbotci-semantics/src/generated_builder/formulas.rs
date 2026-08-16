use super::*;

/// A direct-connection operand at the BO-bound level of the term hierarchy.
#[invariant(::Bound(_) => true)]
#[derive(Debug, Clone, Copy)]
enum GeneratedDirectTermOperand<'syntax> {
    Bound(&'syntax BoundTermSyntax),
}

impl<'syntax> GeneratedDirectTermOperand<'syntax> {
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Bound(term) if GeneratedSimpleTermRef::from_bound(term).is_none()))]
    fn is_grouped_connection(self) -> bool {
        matches!(self, Self::Bound(term) if GeneratedSimpleTermRef::from_bound(term).is_none())
    }
}

impl<'a, 'dict, 'tree> GeneratedGraphBuilder<'a, 'dict, 'tree> {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_bridi_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_bridi_formula_with_options(bridi, None, PredicationMode::Asserted)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bridi_formula_with_suffix_terms<N: TreeNode>(
        &mut self,
        source_node: &N,
        bridi: &'tree BridiSyntax,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.pro_bridi_scope_stack.push(bridi);
        let result = match bridi {
            BridiSyntax::BridiWithLeadingTerms(bridi) => self
                .build_bridi_with_leading_terms_formula_with_suffix_terms(
                    source_node,
                    bridi,
                    suffix_terms,
                ),
            BridiSyntax::RelationOnlyBridi(bridi) => self
                .build_relation_only_bridi_formula_with_suffix_terms(
                    source_node,
                    &bridi.0,
                    suffix_terms,
                ),
            BridiSyntax::BareCuBridi(bridi) => self
                .build_relation_only_bridi_formula_with_suffix_terms(
                    source_node,
                    &bridi.bridi_tail,
                    suffix_terms,
                ),
            BridiSyntax::BridiWithPostCuTerms(_) | BridiSyntax::BareCuTermsBridi(_) => {
                Err(undefined_semantics(
                    "experimental Zantufa post-CU terms combined with statement-level suffix terms",
                ))
            }
        };
        self.pro_bridi_scope_stack.pop();
        result
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scopes| scopes.iter().all(|scope| matches!(scope, GeneratedTermFormulaScope::Negation { .. }))) || ret.is_err())]
    pub(super) fn generated_bridi_term_formula_scopes(
        &self,
        bridi: &'tree BridiSyntax,
    ) -> Result<Vec<GeneratedTermFormulaScope>, SemanticsError> {
        let mut scopes = Vec::new();
        self.collect_generated_bridi_term_formula_scopes(bridi, &mut scopes)?;
        Ok(scopes)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|_| scopes.iter().all(|scope| matches!(scope, GeneratedTermFormulaScope::Negation { .. }))) || ret.is_err())]
    pub(super) fn collect_generated_bridi_term_formula_scopes(
        &self,
        bridi: &'tree BridiSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        match bridi {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                for term in &bridi.leading_terms {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
                self.collect_generated_bridi_tail_term_formula_scopes(&bridi.bridi_tail, scopes)?;
            }
            BridiSyntax::BridiWithPostCuTerms(bridi) => {
                for term in bridi
                    .leading_terms
                    .iter()
                    .chain(bridi.bridi_tail.terms.iter())
                {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
                self.collect_generated_bridi_tail_term_formula_scopes(
                    &bridi.bridi_tail.bridi_tail,
                    scopes,
                )?;
            }
            BridiSyntax::BareCuBridi(bridi) => {
                self.collect_generated_bridi_tail_term_formula_scopes(&bridi.bridi_tail, scopes)?;
            }
            BridiSyntax::BareCuTermsBridi(bridi) => {
                for term in &bridi.bridi_tail.terms {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
                self.collect_generated_bridi_tail_term_formula_scopes(
                    &bridi.bridi_tail.bridi_tail,
                    scopes,
                )?;
            }
            BridiSyntax::RelationOnlyBridi(bridi) => {
                self.collect_generated_bridi_tail_term_formula_scopes(&bridi.0, scopes)?;
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|_| scopes.iter().all(|scope| matches!(scope, GeneratedTermFormulaScope::Negation { .. }))) || ret.is_err())]
    pub(super) fn collect_generated_bridi_tail_term_formula_scopes(
        &self,
        tail: &'tree BridiTailSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        if let Some(connection) = forethought_connection_from_bridi_tail(tail)? {
            return self.collect_generated_forethought_bridi_connection_term_formula_scopes(
                connection, scopes,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(tail)?;
        for term in &simple_tail.terms {
            self.collect_generated_term_formula_scopes_for_bridi_term(
                GeneratedBridiTermRef::Term(term),
                scopes,
            )?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|_| scopes.iter().all(|scope| matches!(scope, GeneratedTermFormulaScope::Negation { .. }))) || ret.is_err())]
    pub(super) fn collect_generated_forethought_bridi_connection_term_formula_scopes(
        &self,
        connection: &'tree ForethoughtBridiConnectionSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        match connection {
            ForethoughtBridiConnectionSyntax::DirectForethoughtBridiConnection(connection) => {
                self.collect_generated_subbridi_term_formula_scopes(&connection.first, scopes)?;
                self.collect_generated_subbridi_term_formula_scopes(
                    &connection.first_branch.branch,
                    scopes,
                )?;
                for branch in &connection.additional_branches {
                    self.collect_generated_subbridi_term_formula_scopes(&branch.branch, scopes)?;
                }
                for term in &connection.tail_terms {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
                Ok(())
            }
            ForethoughtBridiConnectionSyntax::GroupedForethoughtBridiConnection(connection) => self
                .collect_generated_forethought_bridi_connection_term_formula_scopes(
                    &connection.inner,
                    scopes,
                ),
            ForethoughtBridiConnectionSyntax::NegatedForethoughtBridiConnection(connection) => self
                .collect_generated_forethought_bridi_connection_term_formula_scopes(
                    &connection.inner,
                    scopes,
                ),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|_| scopes.iter().all(|scope| matches!(scope, GeneratedTermFormulaScope::Negation { .. }))) || ret.is_err())]
    pub(super) fn collect_generated_subbridi_term_formula_scopes(
        &self,
        subbridi: &'tree SubbridiSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
                self.collect_generated_bridi_term_formula_scopes(bridi, scopes)
            }
            SubbridiSyntax::PrenexSubbridi(prenex) => {
                self.collect_generated_subbridi_term_formula_scopes(&prenex.inner_subbridi, scopes)
            }
        }
    }

    /// Collect the formula scopes a term opens, at whichever level of the hierarchy it sits.
    ///
    /// The five levels used to have five near-identical copies of this walk. They differ only in
    /// which grouping tiers they admit, and the view answers that uniformly, so there is one.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_term_formula_scopes_for_bridi_term(
        &self,
        term: GeneratedBridiTermRef<'tree>,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        if let Some(leaf) = term.simple() {
            return self.collect_generated_term_formula_scopes_for_simple_term(term, leaf, scopes);
        }
        match term.grouping() {
            Some(GeneratedTermGroupingRef::PeheTermsetConnection(connection)) => {
                self.collect_generated_term_formula_scopes_for_bridi_term(
                    GeneratedBridiTermRef::Cehe(&connection.leading_term),
                    scopes,
                )?;
                for continuation in &connection.continuations {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Cehe(&continuation.trailing_term),
                        scopes,
                    )?;
                }
                Ok(())
            }
            Some(GeneratedTermGroupingRef::TermsetGroup(group)) => {
                self.collect_generated_term_formula_scopes_for_termset_group(group, scopes)
            }
            Some(GeneratedTermGroupingRef::ConnectedTerm(connection)) => {
                self.collect_generated_term_formula_scopes_for_connected_term(connection, scopes)
            }
            Some(GeneratedTermGroupingRef::StagBoundTermConnection(connection)) => {
                self.collect_generated_term_formula_scopes_for_stag_bound_term(connection, scopes)
            }
            None => unreachable!("a term that is neither a leaf nor a grouping node"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_term_formula_scopes_for_termset_group(
        &self,
        group: &'tree TermsetGroupSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        self.collect_generated_term_formula_scopes_for_bridi_term(
            GeneratedBridiTermRef::Loose(&group.leading_term),
            scopes,
        )?;
        for continuation in &group.continuations {
            self.collect_generated_term_formula_scopes_for_bridi_term(
                GeneratedBridiTermRef::Nonabs(&continuation.trailing_term),
                scopes,
            )?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_term_formula_scopes_for_connected_term(
        &self,
        connection: &'tree ConnectedTermSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        self.collect_generated_term_formula_scopes_for_bridi_term(
            GeneratedBridiTermRef::Bound(&connection.leading_term),
            scopes,
        )?;
        for continuation in &connection.continuations {
            self.collect_generated_term_formula_scopes_for_bridi_term(
                GeneratedBridiTermRef::Bound(&continuation.trailing_term),
                scopes,
            )?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_generated_term_formula_scopes_for_stag_bound_term(
        &self,
        connection: &'tree StagBoundTermConnectionSyntax,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        self.collect_generated_term_formula_scopes_for_bridi_term(
            GeneratedBridiTermRef::Simple(&connection.leading_term),
            scopes,
        )?;
        for continuation in &connection.continuations {
            self.collect_generated_term_formula_scopes_for_bridi_term(
                GeneratedBridiTermRef::Simple(&continuation.trailing_term),
                scopes,
            )?;
        }
        Ok(())
    }

    #[requires(term.simple().is_some(), "the caller has already resolved this term to its leaf")]
    #[ensures(true)]
    pub(super) fn collect_generated_term_formula_scopes_for_simple_term(
        &self,
        term: GeneratedBridiTermRef<'tree>,
        simple: GeneratedSimpleTermRef<'tree>,
        scopes: &mut Vec<GeneratedTermFormulaScope>,
    ) -> Result<(), SemanticsError> {
        match simple {
            GeneratedSimpleTermRef::NaKuTerm(_) | GeneratedSimpleTermRef::BareNaTerm(_) => {
                scopes.push(GeneratedTermFormulaScope::Negation {
                    source: self.source_for_bridi_term(term, "bridi-negation-boundary"),
                });
            }
            GeneratedSimpleTermRef::NuhiTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
            }
            GeneratedSimpleTermRef::KeTermset(termset) => {
                for term in &termset.termset {
                    self.collect_generated_term_formula_scopes_for_bridi_term(
                        GeneratedBridiTermRef::Term(term),
                        scopes,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_bridi_formula_with_options(
        &mut self,
        bridi: &'tree BridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.pro_bridi_scope_stack.push(bridi);
        let result = match bridi {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                self.build_bridi_with_leading_terms_formula_with_options(bridi, eventuality, mode)
            }
            BridiSyntax::BridiWithPostCuTerms(bridi) => {
                let leading_terms = bridi
                    .leading_terms
                    .iter()
                    .chain(bridi.bridi_tail.terms.iter())
                    .map(GeneratedBridiTermRef::Term)
                    .collect::<Vec<_>>();
                self.build_bridi_tail_formula_with_prefix_terms(
                    bridi,
                    &bridi.bridi_tail.bridi_tail,
                    &leading_terms,
                    eventuality,
                    mode,
                )
            }
            BridiSyntax::BareCuBridi(bridi) => self.build_bridi_tail_formula_with_prefix_terms(
                bridi,
                &bridi.bridi_tail,
                &[],
                eventuality,
                mode,
            ),
            BridiSyntax::BareCuTermsBridi(bridi) => {
                let leading_terms = bridi
                    .bridi_tail
                    .terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term)
                    .collect::<Vec<_>>();
                self.build_bridi_tail_formula_with_prefix_terms(
                    bridi,
                    &bridi.bridi_tail.bridi_tail,
                    &leading_terms,
                    eventuality,
                    mode,
                )
            }
            BridiSyntax::RelationOnlyBridi(bridi) => {
                self.build_relation_only_bridi_formula_with_options(bridi, eventuality, mode)
            }
        };
        self.pro_bridi_scope_stack.pop();
        result
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bridi_tail_formula_with_prefix_terms<N: TreeNode>(
        &mut self,
        source_node: &N,
        tail: &'tree BridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let first_visible_place = if prefix_terms.is_empty() {
            2
        } else {
            generated_bridi_with_leading_terms_first_visible_place(prefix_terms)?
        };
        if generated_bridi_tail_is_connected(tail) {
            return self.build_connected_bridi_tail_formula_with_shared_terms(
                source_node,
                tail,
                prefix_terms,
                &[],
                first_visible_place,
                eventuality,
                mode,
                !prefix_terms.is_empty(),
                true,
                true,
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(tail)? {
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                prefix_terms,
                &[],
                eventuality,
                mode,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(tail)?;
        let terms = prefix_terms
            .iter()
            .copied()
            .chain(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term))
            .collect::<Vec<_>>();
        if let Some(formula) = self.build_generated_forethought_termset_connection_formula(
            source_node,
            simple_tail,
            &terms,
            &BTreeMap::new(),
            &[],
            first_visible_place,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        if let Some(formula) = self.build_generated_pehe_termset_connection_formula(
            simple_tail,
            &terms,
            first_visible_place,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        self.build_selbri_simple_bridi_tail_formula_from_terms(
            source_node,
            simple_tail,
            terms,
            first_visible_place,
            eventuality,
            mode,
            !prefix_terms.is_empty(),
        )
    }

    #[requires(eventuality.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality())))]
    #[ensures(true)]
    pub(super) fn build_subbridi_formula_with_eventuality(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        eventuality: SemanticObjectId,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_generated_subbridi_formula_with_options(subbridi, Some(eventuality), mode)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_subbridi_formula_with_options(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
                self.build_bridi_formula_with_options(bridi, eventuality, mode)
            }
            SubbridiSyntax::PrenexSubbridi(prenex) => {
                self.build_generated_prenex_subbridi_formula_with_options(prenex, eventuality, mode)
            }
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_prenex_subbridi_formula_with_options(
        &mut self,
        prenex: &'tree PrenexSubbridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let context = self.push_generated_prenex_term_bindings(&prenex.prenex_terms)?;
        let result = self.build_generated_subbridi_formula_with_options(
            &prenex.inner_subbridi,
            eventuality,
            mode,
        );
        let data!(GeneratedPrenexContext {
            pushed_bindings,
            topics,
        }) = context.into_data();
        self.pop_generated_prenex_scope_bindings(pushed_bindings);
        let formula = result?;
        self.wrap_formula_with_generated_prenex_terms(
            formula,
            &prenex.prenex_terms,
            topics,
            self.source_for_node(prenex, "topic-comment"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| abstraction.abstractor_connections.is_empty())) || ret.is_err())]
    pub(super) fn single_abstraction_from_selbri<'syntax>(
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
        let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(leading_selbri) else {
            return Ok(None);
        };
        if !unit.assignments.is_empty() {
            return Ok(None);
        }
        let unit = unit.base.as_ref();
        if unit.linkargs.is_some() || !unit.base.conversions.is_empty() {
            return Ok(None);
        }
        let TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) = unit.base.base.as_ref()
        else {
            return Ok(None);
        };
        if abstraction.nai.is_some() || !abstraction.abstractor_connections.is_empty() {
            return Ok(None);
        }
        Ok(Some(abstraction))
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_selbri(
        selbri: &'tree SelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        match selbri {
            SelbriSyntax::ReinterpretZantufaAssignedSelbri(_)
            | SelbriSyntax::ZantufaRelativeSelbri(_)
            | SelbriSyntax::ZantufaPriorityAssignedSelbri(_) => Ok(None),
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
    pub(super) fn generated_description_abstraction_for_untagged_selbri(
        selbri: &'tree UntaggedSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        match selbri {
            UntaggedSelbriSyntax::CoSelbri(co_selbri) if co_selbri.co_tail.is_none() => {
                Self::generated_description_abstraction_for_tanru_selbri(&co_selbri.leading_selbri)
            }
            UntaggedSelbriSyntax::NegatedSelbri(_) | UntaggedSelbriSyntax::CoSelbri(_) => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_connected_selbri(
        selbri: &'tree ConnectedSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        if !selbri.continuations.is_empty() {
            return Ok(None);
        }
        let bound = selbri.leading_selbri.as_ref();
        if bound.bo_tail.is_some() {
            return Ok(None);
        }
        let PlainBoSelbriSyntax::PlainBoTanruUnit(unit) = bound.leading_selbri.as_ref() else {
            return Ok(None);
        };
        if unit.bo_tail.is_some() {
            return Ok(None);
        }
        Self::generated_description_abstraction_for_tanru_unit(&unit.leading_unit)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_tanru_selbri(
        selbri: &'tree TanruSelbriSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(selbri) else {
            return Ok(None);
        };
        Self::generated_description_abstraction_for_tanru_unit(unit)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_tanru_unit(
        unit: &'tree TanruUnitSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        if !unit.assignments.is_empty() {
            return Ok(None);
        }
        // The `be` links belong to this level, not to the NU unit, and
        // they carry the abstractor's CLL 11.13 trailing place.
        Ok(
            Self::generated_description_abstraction_for_tanru_atom(&unit.base.base)?.map(
                |abstraction| GeneratedDescriptionAbstraction {
                    linkargs: unit.base.linkargs.as_ref(),
                    ..abstraction
                },
            ),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_tanru_atom(
        atom: &'tree TanruUnitAtomSyntax,
    ) -> Result<Option<GeneratedDescriptionAbstraction<'tree>>, SemanticsError> {
        match atom.base.as_ref() {
            TanruUnitAtomBaseSyntax::AbstractionTanruUnit(abstraction) => {
                Self::generated_description_abstraction_for_nu_with_conversions(
                    abstraction,
                    &atom.conversions,
                )
            }
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) if atom.conversions.is_empty() => {
                Self::generated_description_abstraction_for_tanru_selbri(&grouped.selbri)
            }
            _ => Ok(None),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|abstraction| abstraction.is_none_or(|abstraction| !abstraction.link_relation.is_empty())) || ret.is_err())]
    pub(super) fn generated_description_abstraction_for_nu_with_conversions<'syntax: 'tree, F>(
        abstraction: &'syntax AbstractionTanruUnitSyntax,
        conversions: &[WithFreeModifiers<Token, F>],
    ) -> Result<Option<GeneratedDescriptionAbstraction<'syntax>>, SemanticsError> {
        if abstraction.nai.is_some() {
            return Ok(None);
        }
        let kind = abstraction_kind_for_nu(abstraction);
        if conversions.is_empty() {
            return Ok(Some(GeneratedDescriptionAbstraction {
                abstraction,
                output_sort: abstraction_output_sort(kind),
                link_relation: abstraction_link_relation(kind),
                linkargs: None,
            }));
        }
        if !abstraction.abstractor_connections.is_empty() {
            return Ok(None);
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
                linkargs: None,
            }));
        }
        Ok(None)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_relation_only_bridi_formula_with_options(
        &mut self,
        bridi: &'tree RelationOnlyBridiSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_bridi_tail_is_connected(&bridi.0) {
            return self.build_connected_bridi_tail_formula_with_shared_terms(
                bridi,
                &bridi.0,
                &[],
                &[],
                2,
                eventuality,
                mode,
                false,
                true,
                true,
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(&bridi.0)? {
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                &[],
                &[],
                eventuality,
                mode,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(&bridi.0)?;
        let terms: Vec<GeneratedBridiTermRef<'tree>> = simple_tail
            .terms
            .iter()
            .map(GeneratedBridiTermRef::Term)
            .collect();
        if let Some(formula) = self.build_generated_forethought_termset_connection_formula(
            bridi,
            simple_tail,
            &terms,
            &BTreeMap::new(),
            &[],
            2,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        self.build_selbri_simple_bridi_tail_formula_from_terms(
            bridi,
            simple_tail,
            terms,
            2,
            eventuality,
            mode,
            false,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_relation_only_bridi_formula_with_suffix_terms<N: TreeNode>(
        &mut self,
        source_node: &N,
        tail: &'tree BridiTailSyntax,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
    ) -> Result<SemanticObjectId, SemanticsError> {
        if generated_bridi_tail_is_connected(tail) {
            return self.build_connected_bridi_tail_formula_with_shared_terms(
                source_node,
                tail,
                &[],
                suffix_terms,
                2,
                None,
                PredicationMode::Asserted,
                false,
                true,
                true,
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(tail)? {
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                &[],
                suffix_terms,
                None,
                PredicationMode::Asserted,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(tail)?;
        let mut terms = Vec::with_capacity(simple_tail.terms.len() + suffix_terms.len());
        terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend_from_slice(suffix_terms);
        let shared_tail_start = (!suffix_terms.is_empty()).then_some(simple_tail.terms.len());
        if let Some(formula) = self.build_generated_forethought_termset_connection_formula(
            source_node,
            simple_tail,
            &terms,
            &BTreeMap::new(),
            &[],
            2,
            None,
            PredicationMode::Asserted,
        )? {
            return Ok(formula);
        }
        self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
            source_node,
            simple_tail,
            &BTreeMap::new(),
            &[],
            terms,
            shared_tail_start,
            2,
            None,
            PredicationMode::Asserted,
            false,
            "bridi-formula",
        )
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn build_bridi_with_leading_terms_formula(
        &mut self,
        bridi: &'tree BridiWithLeadingTermsSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_bridi_with_leading_terms_formula_with_options(
            bridi,
            None,
            PredicationMode::Asserted,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_bridi_with_leading_terms_formula_with_options(
        &mut self,
        bridi: &'tree BridiWithLeadingTermsSyntax,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_terms: Vec<GeneratedBridiTermRef<'tree>> = bridi
            .leading_terms
            .iter()
            .map(GeneratedBridiTermRef::Term)
            .collect();
        if generated_bridi_tail_is_connected(&bridi.bridi_tail) {
            return self.build_connected_bridi_tail_formula_with_shared_terms(
                bridi,
                &bridi.bridi_tail,
                &leading_terms,
                &[],
                1,
                eventuality,
                mode,
                true,
                true,
                true,
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(&bridi.bridi_tail)? {
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                &leading_terms,
                &[],
                eventuality,
                mode,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
        let terms: Vec<GeneratedBridiTermRef<'tree>> = leading_terms
            .iter()
            .copied()
            .chain(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term))
            .collect();
        if let Some(formula) = self.build_generated_forethought_termset_connection_formula(
            bridi,
            simple_tail,
            &terms,
            &BTreeMap::new(),
            &[],
            1,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        if let Some(formula) = self.build_generated_pehe_termset_connection_formula(
            simple_tail,
            &terms,
            1,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        // Guard direct term connections before computing the first visible place: the
        // place-advancement helper below cannot walk a connected leading term and would trip the
        // "connected term reached simple visible-place advancement" invariant. This mirrors the
        // shared guard at `build_selbri_simple_bridi_tail_formula_from_terms_with_source_constructs`
        // (which covers the relation-only / prefix-term paths whose first visible place does not
        // depend on scanning the terms); routing through the same handler keeps behavior uniform.
        if let Some(formula) = self.build_generated_direct_term_connection_formula(
            simple_tail,
            &terms,
            1,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        let first_visible_place =
            generated_bridi_with_leading_terms_first_visible_place(&leading_terms)?;
        self.build_selbri_simple_bridi_tail_formula_from_terms(
            bridi,
            simple_tail,
            terms,
            first_visible_place,
            eventuality,
            mode,
            true,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_direct_term_connection_formula<'syntax: 'tree>(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((position, connection)) =
            terms
                .iter()
                .enumerate()
                .find_map(|(position, term)| match term.grouping() {
                    Some(GeneratedTermGroupingRef::ConnectedTerm(connection)) => {
                        Some((position, connection))
                    }
                    _ => None,
                })
        else {
            return Ok(None);
        };

        let (leading_term, continuations): (
            GeneratedDirectTermOperand<'syntax>,
            Vec<(
                &'syntax TermAfterthoughtConnectiveSyntax,
                GeneratedDirectTermOperand<'syntax>,
            )>,
        ) = (
            GeneratedDirectTermOperand::Bound(&connection.leading_term),
            connection
                .continuations
                .iter()
                .map(|continuation| {
                    (
                        &continuation.connective,
                        GeneratedDirectTermOperand::Bound(&continuation.trailing_term),
                    )
                })
                .collect(),
        );

        if leading_term.is_grouped_connection()
            || continuations
                .iter()
                .any(|(_, term)| term.is_grouped_connection())
        {
            return Err(undefined_semantics(
                "a grouped direct term connection in the term-hierarchy dialect",
            ));
        }

        // Nonlogical direct term connections (e.g. `bi'o`) are unsupported in every context.
        // Detect them here, before any context-specific gating, so that all lowering paths that
        // funnel through this shared guard report the same graceful unsupported-construct error
        // rather than falling through to simple-term assignment lowering (which trips a graph
        // invariant) or the explicit-eventuality guard below.
        if continuations
            .iter()
            .any(|(connective, _)| !generated_direct_term_connective_is_logical(*connective))
        {
            return Err(undefined_semantics(
                "an experimental nonlogical direct term connection",
            ));
        }

        if eventuality.is_some() {
            return Err(invalid_graph(
                "direct term connection reached branch lowering with an explicit eventuality"
                    .to_owned(),
            ));
        }

        let before_terms = &terms[..position];
        let after_terms = &terms[position + 1..];
        let prefix_assignments =
            self.build_term_assignments_for_terms(before_terms.to_vec(), first_visible_place)?;
        let suffix_assignments = self.build_term_assignments_for_terms(after_terms.to_vec(), 1)?;
        let source = self.source_for_node(connection, "term-connection-formula");

        let mut formula = self.build_generated_direct_term_branch_formula_in_mode(
            simple_tail,
            &prefix_assignments,
            leading_term,
            &suffix_assignments,
            first_visible_place,
            mode,
            source.clone(),
        )?;
        for (connective, trailing_term) in continuations {
            let right = self.build_generated_direct_term_branch_formula_in_mode(
                simple_tail,
                &prefix_assignments,
                trailing_term,
                &suffix_assignments,
                first_visible_place,
                mode,
                source.clone(),
            )?;
            formula = self.build_generated_direct_term_pair_formula(
                connective,
                formula,
                right,
                source.clone(),
            )?;
        }
        Ok(Some(formula))
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    fn build_generated_direct_term_branch_formula_in_mode<'syntax: 'tree>(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        prefix_assignments: &GeneratedTermAssignments<'syntax>,
        term: GeneratedDirectTermOperand<'syntax>,
        suffix_assignments: &GeneratedTermAssignments<'syntax>,
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut assignments = prefix_assignments.clone();
        let mut next_visible_place =
            next_visible_place_after_generated_assignments(&assignments).max(first_visible_place);
        let existential_start = self.implicit_existential_variables.len();
        match term {
            GeneratedDirectTermOperand::Bound(term) => {
                let Some(simple) = GeneratedSimpleTermRef::from_bound(term) else {
                    return Err(undefined_semantics(
                        "a grouped direct term connection in the term-hierarchy dialect",
                    ));
                };
                self.insert_generated_simple_term_assignment(
                    &mut assignments.visible_arguments,
                    &mut assignments.place_questions,
                    &mut assignments.modal_terms,
                    &mut assignments.formula_scopes,
                    &mut assignments.coequal_scope_groups,
                    &mut assignments.term_formula_scopes,
                    &mut next_visible_place,
                    GeneratedBridiTermRef::Bound(term),
                    simple,
                )?;
            }
        }
        assignments.implicit_existentials.extend(
            self.implicit_existential_variables
                .split_off(existential_start),
        );
        assignments.next_visible_place = next_visible_place;
        extend_generated_term_assignments_shifted(
            &mut assignments,
            suffix_assignments,
            next_visible_place.saturating_sub(1),
        )?;
        self.build_generated_termset_branch_formula_from_assignments_in_mode(
            simple_tail,
            assignments,
            mode,
            source,
        )
    }

    #[requires(generated_direct_term_connective_is_logical(connective))]
    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_direct_term_pair_formula(
        &mut self,
        connective: &TermAfterthoughtConnectiveSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let left = if generated_direct_term_connective_negates_left(connective) {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right = if generated_direct_term_connective_negates_right(connective) {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        let operator = generated_direct_term_connective_formula_operator(connective);
        let children = if generated_direct_term_connective_has_se(connective)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let parameter = generated_direct_term_connective_question_token(connective)
            .map(|token| self.build_generated_connective_question_parameter_for_token(&token))
            .transpose()?
            .flatten();
        let connector_source = generated_direct_term_connective_source(connective)?;
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
                    source: ConnectorSource::surface_word(connector_source),
                    locus: ConnectorLocus::Term,
                    truth_table: generated_direct_term_connective_truth_table(connective),
                    parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bridi_with_leading_terms_formula_with_suffix_terms<N: TreeNode>(
        &mut self,
        source_node: &N,
        bridi: &'tree BridiWithLeadingTermsSyntax,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_terms: Vec<GeneratedBridiTermRef<'tree>> = bridi
            .leading_terms
            .iter()
            .map(GeneratedBridiTermRef::Term)
            .collect();
        if generated_bridi_tail_is_connected(&bridi.bridi_tail) {
            return self.build_connected_bridi_tail_formula_with_shared_terms(
                source_node,
                &bridi.bridi_tail,
                &leading_terms,
                suffix_terms,
                1,
                None,
                PredicationMode::Asserted,
                true,
                true,
                true,
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(&bridi.bridi_tail)? {
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                &leading_terms,
                suffix_terms,
                None,
                PredicationMode::Asserted,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
        let mut terms =
            Vec::with_capacity(leading_terms.len() + simple_tail.terms.len() + suffix_terms.len());
        terms.extend(leading_terms.iter().copied());
        terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend_from_slice(suffix_terms);
        let shared_tail_start =
            (!suffix_terms.is_empty()).then_some(leading_terms.len() + simple_tail.terms.len());
        if let Some(formula) = self.build_generated_forethought_termset_connection_formula(
            source_node,
            simple_tail,
            &terms,
            &BTreeMap::new(),
            &[],
            1,
            None,
            PredicationMode::Asserted,
        )? {
            return Ok(formula);
        }
        if let Some(formula) = self.build_generated_pehe_termset_connection_formula(
            simple_tail,
            &terms,
            1,
            None,
            PredicationMode::Asserted,
        )? {
            return Ok(formula);
        }
        let first_visible_place =
            generated_bridi_with_leading_terms_first_visible_place(&leading_terms)?;
        self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
            source_node,
            simple_tail,
            &BTreeMap::new(),
            &[],
            terms,
            shared_tail_start,
            first_visible_place,
            None,
            PredicationMode::Asserted,
            true,
            "bridi-formula",
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_forethought_termset_connection_formula<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        _source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: &[GeneratedBridiTermRef<'syntax>],
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        preassigned_place_questions: &[GeneratedPlaceQuestionAssignment],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((position, termset)) = terms.iter().enumerate().find_map(|(position, term)| {
            generated_forethought_termset_in_term(*term).map(|termset| (position, termset))
        }) else {
            return Ok(None);
        };
        if eventuality.is_some() {
            return Err(invalid_graph(
                "forethought termset reached branch lowering with an explicit eventuality"
                    .to_owned(),
            ));
        }

        let before_terms = &terms[..position];
        let after_terms = &terms[position + 1..];
        let (leading_branch, trailing_branch) = termset.branches();
        let leading_terms =
            generated_forethought_termset_branch_terms(before_terms, leading_branch, after_terms);
        let trailing_terms =
            generated_forethought_termset_branch_terms(before_terms, trailing_branch, after_terms);
        let gek = termset.gek();
        let gik = termset.gik();
        let source = match termset {
            GeneratedForethoughtTermsetRef::Nuhi(termset) => {
                self.source_for_node(termset, "termset-connection-formula")
            }
            GeneratedForethoughtTermsetRef::Gek(termset) => {
                self.source_for_node(termset, "termset-connection-formula")
            }
            GeneratedForethoughtTermsetRef::Zantufa(termset) => {
                self.source_for_node(termset, "termset-connection-formula")
            }
        };
        let modal_connection_spec = generated_modal_statement_connection_spec_for_tense_modal(gek);
        if !termset.additional_branches().is_empty()
            && (!generated_modal_forethought_connective_is_logical(gek)
                || generated_modal_forethought_connective_primary_cmavo(gek) == Some(Cmavo::Fahu)
                || modal_connection_spec.is_some())
        {
            return Err(undefined_semantics(
                "an experimental n-ary modal, nonlogical, or FAhU forethought termset connection",
            ));
        }
        if !generated_modal_forethought_connective_is_logical(gek)
            && generated_modal_forethought_connective_primary_cmavo(gek) != Some(Cmavo::Fahu)
            && modal_connection_spec.is_none()
        {
            if !preassigned_visible_arguments.is_empty() || !preassigned_place_questions.is_empty()
            {
                return Err(invalid_graph(
                    "nonlogical forethought termset with shared arguments reached unshared branch lowering"
                        .to_owned(),
                ));
            }
            return self
                .build_generated_nonlogical_forethought_termset_connection_formula(
                    gek,
                    gik,
                    simple_tail,
                    leading_terms,
                    trailing_terms,
                    first_visible_place,
                    mode,
                    source,
                )
                .map(Some);
        }
        let leading = self.build_generated_termset_branch_formula_in_mode(
            simple_tail,
            leading_terms,
            preassigned_visible_arguments,
            preassigned_place_questions,
            first_visible_place,
            mode,
            source.clone(),
        )?;
        let trailing = self.build_generated_termset_branch_formula_in_mode(
            simple_tail,
            trailing_terms,
            preassigned_visible_arguments,
            preassigned_place_questions,
            first_visible_place,
            mode,
            source.clone(),
        )?;
        if let Some(formula) = self.build_generated_fahu_forethought_termset_distribution_formula(
            gek,
            gik,
            leading,
            trailing,
            source.clone(),
        )? {
            return Ok(Some(formula));
        }
        self.mark_generated_modal_forethought_whether_or_not_inert_operand(gek, leading, trailing);
        let left = if generated_modal_forethought_connective_negates_left(gek) {
            self.build_unary_formula(FormulaOperator::Not, leading, source.clone())?
        } else {
            leading
        };
        let right = if generated_gik_connective_negates_right(gik) {
            self.build_unary_formula(FormulaOperator::Not, trailing, source.clone())?
        } else {
            trailing
        };
        let operator = generated_modal_forethought_connective_formula_operator(gek);
        let mut children = if generated_modal_forethought_connective_has_se(gek)
            && operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let mut diagnostics = Vec::new();
        if let Some(spec) = &modal_connection_spec {
            match self.build_generated_modal_formula_connection_claim(
                leading,
                trailing,
                spec,
                source.clone(),
            )? {
                Some(claim) => children.push(claim),
                None => diagnostics.push(diagnostic(
                    "modal termset connection could not find formula-bearing bridi events or propositions to relate",
                )),
            }
        } else if !generated_modal_forethought_connective_is_logical(gek) {
            diagnostics.push(diagnostic(
                "nonlogical forethought termset connection composition is not fully lowered yet",
            ));
        }
        let connector_parameter = self
            .build_generated_connective_question_parameter_for_modal_forethought_connective(gek)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(generated_modal_forethought_pair_source(
                        gek, gik,
                    ),),
                    locus: ConnectorLocus::TermSet,
                    truth_table: generated_modal_forethought_gik_connective_truth_table(gek, gik)
                        .or_else(|| modal_connection_spec.map(|_| "TFFF".to_owned())),
                    parameter: connector_parameter,
                })),
                source.clone(),
                diagnostics,
            ),
        )?;
        let mut formula = formula;
        for branch in termset.additional_branches() {
            let branch_terms = generated_forethought_termset_branch_terms(
                before_terms,
                branch
                    .terms
                    .iter()
                    .map(|term| GeneratedBridiTermRef::Term(term.as_ref()))
                    .collect(),
                after_terms,
            );
            let branch_formula = self.build_generated_termset_branch_formula_in_mode(
                simple_tail,
                branch_terms,
                preassigned_visible_arguments,
                preassigned_place_questions,
                first_visible_place,
                mode,
                source.clone(),
            )?;
            let connector_source = format!(
                "{} {}",
                generated_modal_forethought_connective_source(gek),
                token_text(&branch.gik.0.value)
            );
            formula = self
                .build_binary_formula_for_generated_forethought_statement_connective_core(
                    gek,
                    false,
                    false,
                    connector_source,
                    ConnectorLocus::TermSet,
                    formula,
                    branch_formula,
                    source.clone(),
                )?;
        }
        Ok(Some(formula))
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.is_none_or(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_pehe_termset_connection_formula<'syntax: 'tree>(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Some((position, connection)) =
            terms
                .iter()
                .enumerate()
                .find_map(|(position, term)| match term.grouping() {
                    Some(GeneratedTermGroupingRef::PeheTermsetConnection(connection)) => {
                        Some((position, connection))
                    }
                    _ => None,
                })
        else {
            return Ok(None);
        };
        if eventuality.is_some() {
            return Err(invalid_graph(
                "PEhE termset reached branch lowering with an explicit eventuality".to_owned(),
            ));
        }
        let before_terms = &terms[..position];
        let after_terms = &terms[position + 1..];
        let source = self.source_for_node(connection, "termset-connection-formula");
        let prefix_assignments =
            self.build_term_assignments_for_terms(before_terms.to_vec(), first_visible_place)?;
        let suffix_assignments = self.build_term_assignments_for_terms(after_terms.to_vec(), 1)?;
        if connection.continuations.len() == 1
            && !generated_statement_connective_is_logical(&statement_connective_from_standard(
                &connection.continuations[0].connective,
            ))
        {
            return self
                .build_generated_nonlogical_pehe_termset_connection_formula(
                    connection,
                    simple_tail,
                    &prefix_assignments,
                    &connection.continuations[0],
                    &suffix_assignments,
                    first_visible_place,
                    mode,
                    source,
                )
                .map(Some);
        }

        let mut formula = self.build_generated_pehe_termset_branch_formula_in_mode(
            simple_tail,
            &prefix_assignments,
            &connection.leading_term,
            &suffix_assignments,
            first_visible_place,
            mode,
            source.clone(),
        )?;
        for continuation in &connection.continuations {
            if !generated_statement_connective_is_logical(&statement_connective_from_standard(
                &continuation.connective,
            )) {
                return Err(invalid_graph(
                    "mixed logical and nonlogical PEhE chain reached logical branch folding"
                        .to_owned(),
                ));
            }
            let right = self.build_generated_pehe_termset_branch_formula_in_mode(
                simple_tail,
                &prefix_assignments,
                &continuation.trailing_term,
                &suffix_assignments,
                first_visible_place,
                mode,
                source.clone(),
            )?;
            formula = self.build_generated_pehe_termset_pair_formula(
                continuation,
                formula,
                right,
                source.clone(),
            )?;
        }
        Ok(Some(formula))
    }

    #[requires(left.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(right.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_pehe_termset_pair_formula(
        &mut self,
        continuation: &jbotci_syntax::generated_model::PeheTermsetConnectionContinuationSyntax,
        left: SemanticObjectId,
        right: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let left = if generated_statement_connective_negates_left(
            &statement_connective_from_standard(&continuation.connective),
        ) {
            self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
        } else {
            left
        };
        let right = if generated_statement_connective_negates_right(
            &statement_connective_from_standard(&continuation.connective),
        ) {
            self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
        } else {
            right
        };
        let operator = generated_statement_connective_formula_operator_for_core(
            &statement_connective_from_standard(&continuation.connective),
        );
        let children = if generated_statement_connective_has_se(
            &statement_connective_from_standard(&continuation.connective),
        ) && operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let connector_parameter =
            build_generated_connective_question_parameter_for_statement_connective(
                self,
                &statement_connective_from_standard(&continuation.connective),
            )?;
        let connector_source = generated_statement_connective_core_source(
            &statement_connective_from_standard(&continuation.connective),
        )?;
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
                    source: ConnectorSource::surface_word(format!("pe'e {connector_source}")),
                    locus: ConnectorLocus::TermSet,
                    truth_table: generated_statement_connective_core_truth_table(
                        &statement_connective_from_standard(&continuation.connective),
                    ),
                    parameter: connector_parameter,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(mode == PredicationMode::Asserted)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_fahu_forethought_termset_distribution_formula_from_terms<
        'syntax: 'tree,
    >(
        &mut self,
        termset: &'tree ForethoughtTermsetSyntax,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        leading_terms: &[GeneratedBridiTermRef<'syntax>],
        trailing_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if generated_modal_forethought_connective_primary_cmavo(&termset.gek) != Some(Cmavo::Fahu) {
            return Ok(None);
        }
        let leading_assignments =
            self.build_term_assignments_for_terms(leading_terms.to_vec(), first_visible_place)?;
        let trailing_assignments =
            self.build_term_assignments_for_terms(trailing_terms.to_vec(), first_visible_place)?;
        if !Self::generated_term_assignments_are_unscoped(&leading_assignments)
            || !Self::generated_term_assignments_are_unscoped(&trailing_assignments)
        {
            return Err(invalid_graph(
                "scoped FAhU termset reached unscoped respectively lowering".to_owned(),
            ));
        }

        let Some((composite, members)) = self
            .generated_first_respectively_composite_argument_in_assignments(&leading_assignments)
        else {
            return Ok(None);
        };
        if members.len() != 2 {
            return Ok(None);
        }
        let reverse_members =
            generated_modal_forethought_connective_reverses_composition_members(&termset.gek);
        let (first, second) = if reverse_members {
            (members[1], members[0])
        } else {
            (members[0], members[1])
        };
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let relation_text = relation.display_text();
        let place_count = relation_place_count(self.dictionary, &relation);
        let place_limit = place_count.unwrap_or_else(|| {
            leading_assignments
                .visible_arguments
                .keys()
                .chain(trailing_assignments.visible_arguments.keys())
                .copied()
                .max()
                .unwrap_or(1)
        });
        let leading_replaced = self.build_generated_fahu_termset_branch_formula_from_assignments(
            &relation_text,
            &leading_assignments,
            &BTreeMap::from([(composite, first)]),
            place_limit,
            mode,
            source.clone(),
        )?;
        let trailing_replaced = self.build_generated_fahu_termset_branch_formula_from_assignments(
            &relation_text,
            &trailing_assignments,
            &BTreeMap::from([(composite, second)]),
            place_limit,
            mode,
            source.clone(),
        )?;
        let body = self.next_formula_id();
        self.insert(
            body,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![leading_replaced, trailing_replaced],
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(
                        generated_modal_forethought_connective_source(&termset.gek),
                    ),
                    locus: ConnectorLocus::TermSet,
                    truth_table: None,
                    parameter: None,
                })),
                source.clone(),
                Vec::new(),
            ),
        )?;
        let subject_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Entity,
            ParameterRole::RespectiveSlot,
        )?;
        let branch_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Proposition,
            ParameterRole::RespectiveSlot,
        )?;
        let stream_members = if reverse_members {
            vec![members[1], members[0]]
        } else {
            members
        };
        let distribution = self.next_formula_id();
        self.insert(
            distribution,
            SemanticObject::respectively_distribution_formula(
                body,
                vec![
                    RespectivelyStream::new(subject_slot, stream_members),
                    RespectivelyStream::new(branch_slot, vec![leading_replaced, trailing_replaced]),
                ],
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(Some(distribution))
    }

    #[requires(!relation.is_empty())]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[requires(replacements.values().all(|replacement| crate::model::argument_object_kind_can_fill(replacement.object_kind())))]
    #[requires(place_limit > 0)]
    #[requires(mode == PredicationMode::Asserted)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_fahu_termset_branch_formula_from_assignments(
        &mut self,
        relation: &str,
        assignments: &GeneratedTermAssignments<'tree>,
        replacements: &BTreeMap<SemanticObjectId, SemanticObjectId>,
        place_limit: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&assignments.modal_terms)?;
        for adjunct in &mut adjuncts {
            let mut arguments = adjunct.arguments.clone();
            for argument in arguments.values_mut() {
                replace_generated_argument_value_object(argument, replacements);
            }
            if arguments != adjunct.arguments {
                *adjunct = adjunct.clone().with_data(data! { arguments: arguments });
            }
        }

        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in &assignments.visible_arguments {
            let key = argument_key(*visible_place);
            let mut argument = argument.clone();
            replace_generated_argument_value_object(&mut argument, replacements);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated fa'u termset branch arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        for place in 1..=place_limit.max(highest_argument).max(1) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }

        let predication = self.next_predication_id();
        let mut object = SemanticObject::predication(
            relation.to_owned(),
            None,
            arguments,
            predication_mode_for_relation(relation, mode),
            source.clone(),
            Vec::new(),
        );
        object.set_predication_adjuncts(adjuncts);
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn generated_term_assignments_are_unscoped(
        assignments: &GeneratedTermAssignments<'tree>,
    ) -> bool {
        assignments.formula_scopes.is_empty()
            && assignments.coequal_scope_groups.is_empty()
            && assignments.implicit_existentials.is_empty()
            && assignments.term_formula_scopes.is_empty()
    }

    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_none_or(|(_composite, members)| !members.is_empty()))]
    pub(super) fn generated_first_respectively_composite_argument_in_assignments(
        &self,
        assignments: &GeneratedTermAssignments<'tree>,
    ) -> Option<(SemanticObjectId, Vec<SemanticObjectId>)> {
        assignments.visible_arguments.values().find_map(|argument| {
            let value = argument.value?;
            self.generated_respectively_composite_members(value)
                .map(|members| (value, members))
        })
    }

    #[requires(leading.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(trailing.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_fahu_forethought_termset_distribution_formula(
        &mut self,
        connective: &jbotci_syntax::generated_model::ModalForethoughtConnectiveSyntax,
        gik: &'tree GikConnectiveSyntax,
        leading: SemanticObjectId,
        trailing: SemanticObjectId,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if generated_modal_forethought_connective_primary_cmavo(connective) != Some(Cmavo::Fahu) {
            return Ok(None);
        }
        let Some((composite, members)) =
            self.generated_first_respectively_composite_argument(leading)
        else {
            return Ok(None);
        };
        if members.len() != 2 {
            return Ok(None);
        }
        let reverse_members =
            generated_modal_forethought_connective_reverses_composition_members(connective);
        let (first, second) = if reverse_members {
            (members[1], members[0])
        } else {
            (members[0], members[1])
        };
        let leading_replaced = self
            .clone_generated_formula_with_argument_replacements(
                leading,
                &BTreeMap::from([(composite, first)]),
            )?
            .ok_or_else(|| {
                invalid_graph(
                    "generated fa'u termset leading branch could not be distributed".to_owned(),
                )
            })?;
        let trailing_replaced = self
            .clone_generated_formula_with_argument_replacements(
                trailing,
                &BTreeMap::from([(composite, second)]),
            )?
            .ok_or_else(|| {
                invalid_graph(
                    "generated fa'u termset trailing branch could not be distributed".to_owned(),
                )
            })?;
        let body = self.next_formula_id();
        self.insert(
            body,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                vec![leading_replaced, trailing_replaced],
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(
                        if generated_modal_forethought_connective_primary_cmavo(connective)
                            == Some(Cmavo::Fahu)
                        {
                            generated_modal_forethought_connective_source(connective)
                        } else {
                            generated_modal_forethought_pair_source(connective, gik)
                        },
                    ),
                    locus: ConnectorLocus::TermSet,
                    truth_table: None,
                    parameter: None,
                })),
                source.clone(),
                Vec::new(),
            ),
        )?;
        let subject_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Entity,
            ParameterRole::RespectiveSlot,
        )?;
        let branch_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Proposition,
            ParameterRole::RespectiveSlot,
        )?;
        let stream_members = if reverse_members {
            vec![members[1], members[0]]
        } else {
            members
        };
        let distribution = self.next_formula_id();
        self.insert(
            distribution,
            SemanticObject::respectively_distribution_formula(
                body,
                vec![
                    RespectivelyStream::new(subject_slot, stream_members),
                    RespectivelyStream::new(branch_slot, vec![leading_replaced, trailing_replaced]),
                ],
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(Some(distribution))
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_none_or(|(_composite, members)| !members.is_empty()))]
    pub(super) fn generated_first_respectively_composite_argument(
        &self,
        formula: SemanticObjectId,
    ) -> Option<(SemanticObjectId, Vec<SemanticObjectId>)> {
        let object = self.objects.get(&formula)?;
        match object.as_formula()?.as_data() {
            data!(FormulaNode::Atom(formula)) => {
                let predication = self.objects.get(&formula.predication)?.as_predication()?;
                predication.arguments.values().find_map(|argument| {
                    let value = argument.value?;
                    self.generated_respectively_composite_members(value)
                        .map(|members| (value, members))
                })
            }
            data!(FormulaNode::Connective(formula)) => formula
                .children
                .iter()
                .find_map(|child| self.generated_first_respectively_composite_argument(*child)),
            data!(FormulaNode::Quantified(_))
            | data!(FormulaNode::QuantifierBundle(_))
            | data!(FormulaNode::RespectivelyDistribution(_)) => None,
        }
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_pehe_termset_branch_formula_in_mode<'syntax: 'tree>(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        prefix_assignments: &GeneratedTermAssignments<'syntax>,
        operand: &'syntax CeheTermSyntax,
        suffix_assignments: &GeneratedTermAssignments<'syntax>,
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let assignments = self.build_generated_pehe_termset_branch_assignments(
            prefix_assignments,
            operand,
            suffix_assignments,
            first_visible_place,
        )?;
        self.build_generated_termset_branch_formula_from_assignments_in_mode(
            simple_tail,
            assignments,
            mode,
            source,
        )
    }

    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_termset_branch_formula_from_assignments_in_mode(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        assignments: GeneratedTermAssignments<'tree>,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut diagnostics = Vec::new();
        if place_count.is_none() && !relation_has_open_place_structure(&relation) {
            diagnostics.push(diagnostic(
                "relation place structure is unavailable; only places required by explicit assignments are represented",
            ));
        }
        let place_question_assignments = assignments.place_questions.clone();
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated termset branch arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
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
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        self.apply_generated_tagged_term_event_modifiers(eventuality, &assignments.modal_terms)?;
        let adjuncts = self
            .build_adjuncts_for_generated_tagged_terms_for_event_with_predication_arguments(
                eventuality,
                &assignments.modal_terms,
                Some(&arguments),
            )?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            source.clone(),
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
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

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_generated_pehe_termset_branch_assignments<'syntax: 'tree>(
        &mut self,
        prefix_assignments: &GeneratedTermAssignments<'syntax>,
        operand: &'syntax CeheTermSyntax,
        suffix_assignments: &GeneratedTermAssignments<'syntax>,
        first_visible_place: usize,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        let mut assignments = prefix_assignments.clone();
        let mut next_visible_place =
            next_visible_place_after_generated_assignments(&assignments).max(first_visible_place);
        let existential_start = self.implicit_existential_variables.len();
        self.insert_generated_pehe_termset_operand_assignment(
            &mut assignments.visible_arguments,
            &mut assignments.place_questions,
            &mut assignments.modal_terms,
            &mut assignments.formula_scopes,
            &mut assignments.coequal_scope_groups,
            &mut assignments.term_formula_scopes,
            &mut next_visible_place,
            operand,
        )?;
        assignments.implicit_existentials.extend(
            self.implicit_existential_variables
                .split_off(existential_start),
        );
        extend_generated_term_assignments_shifted(
            &mut assignments,
            suffix_assignments,
            next_visible_place.saturating_sub(1),
        )?;
        Ok(assignments)
    }

    #[requires(*next_visible_place > 0)]
    #[ensures(true)]
    pub(super) fn insert_generated_pehe_termset_operand_assignment<'syntax: 'tree>(
        &mut self,
        visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
        place_questions: &mut Vec<GeneratedPlaceQuestionAssignment>,
        modal_terms: &mut Vec<GeneratedModalTerm<'syntax>>,
        formula_scopes: &mut Vec<GeneratedArgumentQuantifierScope<'syntax>>,
        coequal_scope_groups: &mut Vec<GeneratedArgumentQuantifierBundleScope<'syntax>>,
        term_formula_scopes: &mut Vec<GeneratedTermFormulaScope>,
        next_visible_place: &mut usize,
        operand: &'syntax CeheTermSyntax,
    ) -> Result<(), SemanticsError> {
        if let Some(simple) = GeneratedSimpleTermRef::from_cehe(operand) {
            return self.insert_generated_simple_term_assignment(
                visible_arguments,
                place_questions,
                modal_terms,
                formula_scopes,
                coequal_scope_groups,
                term_formula_scopes,
                next_visible_place,
                GeneratedBridiTermRef::Cehe(operand),
                simple,
            );
        }
        match operand {
            CeheTermSyntax::TermsetGroup(termset) => self
                .insert_generated_termset_group_assignment(
                    visible_arguments,
                    place_questions,
                    modal_terms,
                    formula_scopes,
                    coequal_scope_groups,
                    term_formula_scopes,
                    next_visible_place,
                    GeneratedBridiTermRef::Cehe(operand),
                    termset,
                ),
            CeheTermSyntax::ConnectedTerm(_) | CeheTermSyntax::StagBoundTermConnection(_) => Err(
                undefined_semantics("a grouped term connection inside a PEhE termset branch"),
            ),
            _ => unreachable!("CEhE-level leaf conversion rejected a non-connection variant"),
        }
    }

    #[requires(first_visible_place > 0)]
    #[requires(mode == PredicationMode::Asserted)]
    #[requires(!generated_statement_connective_is_logical(&statement_connective_from_standard(&continuation.connective)))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_nonlogical_pehe_termset_connection_formula<'syntax: 'tree>(
        &mut self,
        connection: &'syntax PeheTermsetConnectionSyntax,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        prefix_assignments: &GeneratedTermAssignments<'syntax>,
        continuation: &'syntax jbotci_syntax::generated_model::PeheTermsetConnectionContinuationSyntax,
        suffix_assignments: &GeneratedTermAssignments<'syntax>,
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_assignments = self.build_generated_pehe_termset_branch_assignments(
            prefix_assignments,
            &connection.leading_term,
            suffix_assignments,
            first_visible_place,
        )?;
        let trailing_assignments = self.build_generated_pehe_termset_branch_assignments(
            prefix_assignments,
            &continuation.trailing_term,
            suffix_assignments,
            first_visible_place,
        )?;
        if !leading_assignments.formula_scopes.is_empty()
            || !leading_assignments.coequal_scope_groups.is_empty()
            || !leading_assignments.implicit_existentials.is_empty()
            || !leading_assignments.term_formula_scopes.is_empty()
            || !trailing_assignments.formula_scopes.is_empty()
            || !trailing_assignments.coequal_scope_groups.is_empty()
            || !trailing_assignments.implicit_existentials.is_empty()
            || !trailing_assignments.term_formula_scopes.is_empty()
        {
            return Err(invalid_graph(
                "scoped nonlogical PEhE branch reached unscoped composite lowering".to_owned(),
            ));
        }

        let mut leading_adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&leading_assignments.modal_terms)?;
        let mut trailing_adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&trailing_assignments.modal_terms)?;
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let mut arguments = BTreeMap::new();
        let mut places = leading_assignments
            .visible_arguments
            .keys()
            .chain(trailing_assignments.visible_arguments.keys())
            .copied()
            .collect::<Vec<_>>();
        places.sort_unstable();
        places.dedup();
        for place in places {
            match (
                leading_assignments.visible_arguments.get(&place),
                trailing_assignments.visible_arguments.get(&place),
            ) {
                (Some(leading), Some(trailing)) if leading.value == trailing.value => {
                    arguments.insert(argument_key(place), leading.clone());
                }
                (Some(leading), Some(trailing)) => {
                    let Some(leading_value) = leading.value else {
                        continue;
                    };
                    let Some(trailing_value) = trailing.value else {
                        continue;
                    };
                    let reverse_members =
                        generated_statement_connective_reverses_composition_members(
                            &statement_connective_from_standard(&continuation.connective),
                        );
                    let (first, second) = if reverse_members {
                        (trailing_value, leading_value)
                    } else {
                        (leading_value, trailing_value)
                    };
                    let operator = generated_nonlogical_statement_composition_operator(
                        &statement_connective_from_standard(&continuation.connective),
                    )?;
                    let collective = operator.is_mass().then_some(true);
                    let endpoint_inclusion = generated_statement_connective_endpoint_inclusion(
                        &statement_connective_from_standard(&continuation.connective),
                        reverse_members,
                    );
                    let composite = self.next_referent_id();
                    self.insert(
                        composite,
                        SemanticObject::referent(
                            ReferentCategory::Composite,
                            SemanticSort::Entity,
                            None,
                            None,
                            Some(new!(Composition {
                                operator,
                                operator_parameter: None,
                                members: vec![first, second],
                                excluded_members: Vec::new(),
                                collective,
                                scalar_negated: generated_statement_connective_negates_right(
                                    &statement_connective_from_standard(&continuation.connective),
                                )
                                .then_some(true),
                                complement: None,
                                endpoint_inclusion,
                            })),
                            source_with_construct(source.clone(), "connected-sumti"),
                            Vec::new(),
                        ),
                    )?;
                    arguments.insert(argument_key(place), ArgumentValue::filled(composite, None));
                }
                (Some(argument), None) | (None, Some(argument)) => {
                    arguments.insert(argument_key(place), argument.clone());
                }
                (None, None) => {}
            }
        }
        let leading_component = leading_assignments
            .visible_arguments
            .get(&1)
            .and_then(|argument| argument.value);
        let trailing_component = trailing_assignments
            .visible_arguments
            .get(&1)
            .and_then(|argument| argument.value);
        let mut adjuncts = Vec::new();
        adjuncts.extend(leading_adjuncts.drain(..).map(|argument| {
            leading_component.map_or(argument.clone(), |component| {
                argument.with_component(component)
            })
        }));
        adjuncts.extend(trailing_adjuncts.drain(..).map(|argument| {
            trailing_component.map_or(argument.clone(), |component| {
                argument.with_component(component)
            })
        }));
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let predication = self.next_predication_id();
        let mut object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            source.clone(),
            Vec::new(),
        );
        object.set_predication_adjuncts(adjuncts);
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(mode == PredicationMode::Asserted)]
    #[requires(!generated_modal_forethought_connective_is_logical(gek))]
    #[requires(generated_modal_forethought_connective_primary_cmavo(gek) != Some(Cmavo::Fahu))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_nonlogical_forethought_termset_connection_formula(
        &mut self,
        gek: &'tree ModalForethoughtConnectiveSyntax,
        gik: &'tree GikConnectiveSyntax,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        leading_terms: Vec<GeneratedBridiTermRef<'tree>>,
        trailing_terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let leading_assignments =
            self.build_term_assignments_for_terms(leading_terms, first_visible_place)?;
        let trailing_assignments =
            self.build_term_assignments_for_terms(trailing_terms, first_visible_place)?;
        if !leading_assignments.formula_scopes.is_empty()
            || !leading_assignments.coequal_scope_groups.is_empty()
            || !leading_assignments.implicit_existentials.is_empty()
            || !leading_assignments.term_formula_scopes.is_empty()
            || !trailing_assignments.formula_scopes.is_empty()
            || !trailing_assignments.coequal_scope_groups.is_empty()
            || !trailing_assignments.implicit_existentials.is_empty()
            || !trailing_assignments.term_formula_scopes.is_empty()
        {
            return Err(invalid_graph(
                "scoped nonlogical forethought termset reached unscoped composite lowering"
                    .to_owned(),
            ));
        }

        let mut leading_adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&leading_assignments.modal_terms)?;
        let mut trailing_adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&trailing_assignments.modal_terms)?;
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let mut arguments = BTreeMap::new();
        let mut places = leading_assignments
            .visible_arguments
            .keys()
            .chain(trailing_assignments.visible_arguments.keys())
            .copied()
            .collect::<Vec<_>>();
        places.sort_unstable();
        places.dedup();
        for place in places {
            match (
                leading_assignments.visible_arguments.get(&place),
                trailing_assignments.visible_arguments.get(&place),
            ) {
                (Some(leading), Some(trailing)) if leading.value == trailing.value => {
                    arguments.insert(argument_key(place), leading.clone());
                }
                (Some(leading), Some(trailing)) => {
                    let Some(leading_value) = leading.value else {
                        continue;
                    };
                    let Some(trailing_value) = trailing.value else {
                        continue;
                    };
                    let reverse_members =
                        generated_modal_forethought_connective_reverses_composition_members(gek);
                    let (first, second) = if reverse_members {
                        (trailing_value, leading_value)
                    } else {
                        (leading_value, trailing_value)
                    };
                    let operator =
                        generated_nonlogical_modal_forethought_composition_operator(gek)?;
                    let collective = operator.is_mass().then_some(true);
                    let endpoint_inclusion =
                        generated_modal_forethought_connective_endpoint_inclusion(
                            gek,
                            reverse_members,
                        );
                    let composite = self.next_referent_id();
                    self.insert(
                        composite,
                        SemanticObject::referent(
                            ReferentCategory::Composite,
                            SemanticSort::Entity,
                            None,
                            None,
                            Some(new!(Composition {
                                operator,
                                operator_parameter: None,
                                members: vec![first, second],
                                excluded_members: Vec::new(),
                                collective,
                                scalar_negated: gik.nai.is_some().then_some(true),
                                complement: None,
                                endpoint_inclusion,
                            })),
                            source_with_construct(source.clone(), "connected-sumti"),
                            Vec::new(),
                        ),
                    )?;
                    arguments.insert(argument_key(place), ArgumentValue::filled(composite, None));
                }
                (Some(argument), None) | (None, Some(argument)) => {
                    arguments.insert(argument_key(place), argument.clone());
                }
                (None, None) => {}
            }
        }
        let leading_component = leading_assignments
            .visible_arguments
            .get(&1)
            .and_then(|argument| argument.value);
        let trailing_component = trailing_assignments
            .visible_arguments
            .get(&1)
            .and_then(|argument| argument.value);
        let mut adjuncts = Vec::new();
        adjuncts.extend(leading_adjuncts.drain(..).map(|argument| {
            leading_component.map_or(argument.clone(), |component| {
                argument.with_component(component)
            })
        }));
        adjuncts.extend(trailing_adjuncts.drain(..).map(|argument| {
            trailing_component.map_or(argument.clone(), |component| {
                argument.with_component(component)
            })
        }));
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let predication = self.next_predication_id();
        let mut object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            source.clone(),
            Vec::new(),
        );
        object.set_predication_adjuncts(adjuncts);
        self.insert(predication, object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(preassigned_visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_termset_branch_formula_in_mode(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        preassigned_place_questions: &[GeneratedPlaceQuestionAssignment],
        first_visible_place: usize,
        mode: PredicationMode,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let relation = match relation_label_from_selbri(&simple_tail.selbri) {
            Ok(relation) => semantic_relation_label(relation),
            Err(_) => {
                return self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments(
                    &simple_tail.selbri,
                    simple_tail,
                    preassigned_visible_arguments,
                    preassigned_place_questions,
                    terms,
                    first_visible_place,
                    None,
                    mode,
                    false,
                );
            }
        };
        let relation_text = relation.display_text();
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
                terms.len().max(1).max(
                    preassigned_visible_arguments
                        .keys()
                        .copied()
                        .max()
                        .unwrap_or(0),
                )
            }
        };
        if preassigned_place_questions.is_empty() {
            if let Some(formula) = self
                .build_generated_logical_sumti_connection_formula_for_terms_with_preassigned_arguments(
                    &relation_text,
                    &terms,
                    preassigned_visible_arguments,
                    first_visible_place,
                    place_limit,
                    &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
                    mode,
                    source.clone(),
                    source.clone(),
                )?
            {
                return Ok(formula);
            }
        }
        let assignments =
            self.build_term_assignments_for_terms(terms.clone(), first_visible_place)?;
        let adjuncts = self.build_adjuncts_for_generated_tagged_terms(&assignments.modal_terms)?;
        let mut arguments = preassigned_visible_arguments
            .iter()
            .map(|(place, argument)| (argument_key(*place), argument.clone()))
            .collect::<BTreeMap<_, _>>();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated termset branch arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let mut place_question_assignments = preassigned_place_questions.to_vec();
        place_question_assignments.extend(assignments.place_questions.clone());
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
            &arguments,
            place_count,
            highest_argument,
        )?;
        for place in 1..=place_limit.max(highest_argument) {
            let key = argument_key(place);
            if !arguments.contains_key(&key) {
                arguments.insert(key, self.build_elided_argument_for_place(place)?);
            }
        }
        let eventuality = self.build_generated_predication_eventuality(source.clone())?;
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            source.clone(),
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
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

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_from_terms<N: TreeNode>(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_simple_bridi_tail_formula_from_terms_with_formula_construct(
            source_node,
            simple_tail,
            terms,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            "bridi-formula",
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_from_terms_with_formula_construct<
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        formula_construct: &'static str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_simple_bridi_tail_formula_from_terms_with_source_constructs(
            source_node,
            simple_tail,
            terms,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            "predication",
            formula_construct,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|scoped| scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_selbri_simple_bridi_tail_scoped_formula_from_terms<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'syntax>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        formula_construct: &'static str,
    ) -> Result<GeneratedScopedFormula<'syntax>, SemanticsError> {
        if let Some(scoped) = self.build_direct_relation_scoped_formula_from_terms(
            source_node,
            &simple_tail.selbri,
            &[],
            false,
            terms.clone(),
            None,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            formula_construct,
        )? {
            return Ok(scoped);
        }
        let formula = self
            .build_selbri_simple_bridi_tail_formula_from_terms_with_formula_construct(
                source_node,
                simple_tail,
                terms,
                first_visible_place,
                eventuality,
                mode,
                allow_single_argument_distribution,
                formula_construct,
            )?;
        Ok(self.generated_scoped_formula_without_scopes(formula))
    }

    #[requires(first_visible_place > 0)]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|scoped| scoped.as_ref().is_none_or(|scoped| scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_direct_relation_scoped_formula_from_terms<'syntax: 'tree, N: TreeNode>(
        &mut self,
        source_node: &N,
        selbri: &'tree SelbriSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        annotate_shared_head_source: bool,
        mut terms: Vec<GeneratedBridiTermRef<'syntax>>,
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        formula_construct: &'static str,
    ) -> Result<Option<GeneratedScopedFormula<'syntax>>, SemanticsError> {
        let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
            return Ok(None);
        };
        if co_selbri.co_tail.is_some()
            || relation_question_syntax_from_co_selbri(co_selbri)?.is_some()
            || relation_variable_syntax_from_co_selbri(co_selbri)?.is_some()
            || unspecified_relation_syntax_from_co_selbri(co_selbri)?.is_some()
            || single_relation_parameter_syntax_from_co_selbri(co_selbri)?.is_some()
        {
            return Ok(None);
        }
        let tanru = tanru_selbri_from_co_selbri(co_selbri)?;
        if let Some(tanru) = tanru {
            let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(tanru) else {
                return Ok(None);
            };
            if sumti_selbri_from_generated_tanru_unit(unit)?.is_some()
                || generated_tanru_unit_is_grouped(unit)?
            {
                return Ok(None);
            }
        }
        let (_, fai_sumti) = self.split_generated_fai_terms(terms.clone())?;
        if !fai_sumti.is_empty() {
            let jai_unit = match tanru {
                Some(tanru) => {
                    let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(tanru) else {
                        return Ok(None);
                    };
                    let (atom, _) = generated_linked_tanru_unit_parts(unit)?;
                    generated_jai_modal_tanru_unit(atom.base.as_ref())
                }
                None => None,
            };
            if jai_unit.is_some() {
                // The full tanru path extracts FAI after establishing the JAI-converted frame.
                return Ok(None);
            }
            if let Some(shared_tail_start) = shared_tail_start {
                let (_, local_fai_sumti) =
                    self.split_generated_fai_terms(terms[..shared_tail_start].to_vec())?;
                if local_fai_sumti.is_empty() {
                    // A shared FAI is meaningful only to connected branches containing JAI. It
                    // contributes no numbered assignment to their ordinary sibling branches.
                    terms = self.split_generated_fai_terms(terms)?.0;
                }
            }
        }
        if eventuality.is_none()
            && mode == PredicationMode::Asserted
            && allow_single_argument_distribution
            && let [term] = terms.as_slice()
            && let Some(sumti) = simple_sumti_from_term(*term)
            && no_gadri_description_from_sumti(sumti)?.is_some()
        {
            return Ok(None);
        }
        let relation = match relation_label_from_co_selbri(co_selbri) {
            Ok(relation) => semantic_relation_label(relation),
            Err(_) => return Ok(None),
        };
        let relation_text = relation.display_text();
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
        let predication_source = self.source_for_node(source_node, "predication");
        let formula_source = self.source_for_node(source_node, formula_construct);
        if let Some(formula) = self.build_generated_logical_sumti_connection_formula_for_terms(
            &relation_text,
            &terms,
            first_visible_place,
            place_limit,
            &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
            mode,
            predication_source.clone(),
            formula_source.clone(),
        )? {
            return Ok(Some(self.generated_scoped_formula_without_scopes(formula)));
        }
        if let Some(formula) = self.build_generated_logical_modal_connection_formula_for_terms(
            self.source_for_node(source_node, "modal-branch-formula"),
            self.source_for_node(source_node, "modal-connection-formula"),
            &relation_text,
            place_count,
            place_limit,
            prefix_terms,
            annotate_shared_head_source,
            &terms,
            shared_tail_start,
            first_visible_place,
            &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
            mode,
            predication_source.clone(),
        )? {
            return Ok(Some(self.generated_scoped_formula_without_scopes(formula)));
        }
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(predication_source.clone())?,
        };
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
        let local_first_visible_place = first_unfilled_visible_place_after_generated_prefix_terms(
            prefix_terms,
            first_visible_place,
        )?;
        let assignments = self.with_temporal_context(eventuality, |builder| {
            builder.build_term_assignments_for_terms_with_shared_tail_source(
                terms.clone(),
                local_first_visible_place,
                shared_tail_start,
            )
        })?;
        let shared_head_assignments = if prefix_terms.is_empty() {
            empty_generated_term_assignments()
        } else {
            self.build_generated_shared_head_assignments(prefix_terms, annotate_shared_head_source)?
        };
        let mut place_question_assignments = shared_head_assignments.place_questions.clone();
        place_question_assignments.extend(assignments.place_questions.clone());
        let mut visible_arguments_for_modal_terms =
            shared_head_assignments.visible_arguments.clone();
        visible_arguments_for_modal_terms.extend(assignments.visible_arguments.clone());
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in &shared_head_assignments.visible_arguments {
            arguments.insert(argument_key(*visible_place), argument.clone());
        }
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let adjuncts = self
            .build_adjuncts_for_generated_tagged_terms_for_event_with_visible_arguments(
                eventuality,
                &assignments.modal_terms,
                Some(&visible_arguments_for_modal_terms),
            )?;
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
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
        self.attach_generated_reciprocity_to_predication_for_terms(predication, &terms)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        self.attach_generated_modal_terms_to_formula(
            formula,
            &shared_head_assignments.modal_terms,
        )?;
        let mut scoped = GeneratedScopedFormula {
            formula,
            formula_scopes: assignments.formula_scopes,
            coequal_scope_groups: assignments.coequal_scope_groups,
            implicit_existentials: assignments.implicit_existentials,
            term_formula_scopes: assignments.term_formula_scopes,
        };
        scoped = self.append_generated_term_assignment_scopes(scoped, shared_head_assignments);
        Ok(Some(scoped))
    }

    #[requires(place_limit > 0)]
    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_logical_modal_connection_formula_for_terms<'syntax: 'tree, F>(
        &mut self,
        branch_formula_source: Option<crate::model::SemanticSource>,
        connection_formula_source: Option<crate::model::SemanticSource>,
        relation: &str,
        direct_relation_place_count: Option<usize>,
        place_limit: usize,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        annotate_shared_head_source: bool,
        terms: &[GeneratedBridiTermRef<'syntax>],
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        if mode != PredicationMode::Asserted {
            return Ok(None);
        }
        let connection = if let Some((connected_index, connected_term, spec)) =
            generated_logical_event_tense_connection_assignment_in_terms(terms)
        {
            let anchor = self
                .build_tagged_or_elided_sumti_argument(&connected_term.sumti)?
                .value;
            let data!(GeneratedConnectedEventTenseSpec {
                operator,
                source,
                truth_table,
                connector_question,
                branches,
            }) = spec.into_data();
            new!(GeneratedLogicalTagConnection {
                operator,
                source,
                truth_table,
                connector_question,
                locus: ConnectorLocus::Tense,
                connected_index,
                branches: branches
                    .into_iter()
                    .map(|branch| {
                        new!(GeneratedLogicalTagConnectionBranch::Event { branch, anchor })
                    })
                    .collect(),
            })
        } else if let Some((connected_index, connected_term, spec)) =
            generated_logical_modal_connection_assignment_in_terms(terms)?
        {
            let argument = self.build_tagged_or_elided_sumti_argument(&connected_term.sumti)?;
            let data!(GeneratedLogicalModalConnectionSpec {
                operator,
                source,
                truth_table,
                terms: modal_terms,
            }) = spec.into_data();
            new!(GeneratedLogicalTagConnection {
                operator,
                source,
                truth_table,
                connector_question: None,
                locus: ConnectorLocus::Tag,
                connected_index,
                branches: modal_terms
                    .into_iter()
                    .map(|term| {
                        new!(GeneratedLogicalTagConnectionBranch::Modal {
                            term,
                            argument: argument.clone(),
                        })
                    })
                    .collect(),
            })
        } else {
            return Ok(None);
        };
        self.build_generated_logical_tag_connection_formula_for_terms(
            branch_formula_source,
            connection_formula_source,
            relation,
            direct_relation_place_count,
            place_limit,
            prefix_terms,
            annotate_shared_head_source,
            terms,
            shared_tail_start,
            first_visible_place,
            conversions,
            connection,
            predication_source,
        )
        .map(Some)
    }

    #[requires(place_limit > 0)]
    #[requires(first_visible_place > 0)]
    #[requires(connection.connected_index < terms.len())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_logical_tag_connection_formula_for_terms<'syntax: 'tree, F>(
        &mut self,
        branch_formula_source: Option<crate::model::SemanticSource>,
        connection_formula_source: Option<crate::model::SemanticSource>,
        relation: &str,
        direct_relation_place_count: Option<usize>,
        place_limit: usize,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        annotate_shared_head_source: bool,
        terms: &[GeneratedBridiTermRef<'syntax>],
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        connection: GeneratedLogicalTagConnection<'syntax>,
        predication_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let base_terms = terms
            .iter()
            .enumerate()
            .filter_map(|(index, term)| (index != connection.connected_index).then_some(*term))
            .collect::<Vec<_>>();
        let adjusted_shared_tail_start = shared_tail_start.map(|start| {
            if connection.connected_index < start {
                start.saturating_sub(1)
            } else {
                start
            }
        });
        let local_first_visible_place =
            next_visible_place_after_generated_terms(prefix_terms, first_visible_place)?;
        let shared_head_assignments = if prefix_terms.is_empty() {
            empty_generated_term_assignments()
        } else {
            self.build_generated_shared_head_assignments(prefix_terms, annotate_shared_head_source)?
        };
        let mut children = Vec::with_capacity(connection.branches.len());
        for branch in &connection.branches {
            children.push(
                self.build_generated_logical_modal_connection_branch_formula(
                    branch_formula_source.clone(),
                    relation,
                    direct_relation_place_count,
                    place_limit,
                    &base_terms,
                    adjusted_shared_tail_start,
                    local_first_visible_place,
                    conversions,
                    shared_head_assignments.clone(),
                    branch,
                    predication_source.clone(),
                )?,
            );
        }

        if connection.operator == FormulaOperator::RespectivelyDistribution {
            return self.build_generated_respectively_tag_connection_formula(
                children,
                &connection,
                connection_formula_source,
            );
        }
        let connector_parameter = connection
            .connector_question
            .as_ref()
            .map(|token| self.build_generated_connective_question_parameter_for_token(token))
            .transpose()?
            .flatten();
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                connection.operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(connection.source.clone()),
                    locus: connection.locus,
                    truth_table: connection.truth_table.clone(),
                    parameter: connector_parameter,
                })),
                connection_formula_source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(children.len() == connection.branches.len())]
    #[requires(children.len() >= 2)]
    #[requires(children.iter().all(|child| child.object_kind() == crate::model::SemanticObjectKind::Formula))]
    #[requires(connection.operator == FormulaOperator::RespectivelyDistribution)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_respectively_tag_connection_formula(
        &mut self,
        children: Vec<SemanticObjectId>,
        connection: &GeneratedLogicalTagConnection<'tree>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let composites = children
            .iter()
            .map(|child| self.generated_first_respectively_composite_argument(*child))
            .collect::<Vec<_>>();
        let has_composites = composites.iter().any(Option::is_some);
        if has_composites && composites.iter().any(Option::is_none) {
            return Err(invalid_graph(
                "generated fa'u tag connection branches disagree about their composite argument"
                    .to_owned(),
            ));
        }
        let mut distributed_children = Vec::with_capacity(children.len());
        let mut subject_members = Vec::with_capacity(children.len());
        for (branch_index, (child, composite)) in children.into_iter().zip(composites).enumerate() {
            let Some((composite, members)) = composite else {
                distributed_children.push(child);
                continue;
            };
            if members.len() != connection.branches.len() {
                return Err(invalid_graph(format!(
                    "generated fa'u tag connection has {} branches but its composite argument has {} members",
                    connection.branches.len(),
                    members.len(),
                )));
            }
            let member = members[branch_index];
            let distributed = self
                .clone_generated_formula_with_argument_replacements(
                    child,
                    &BTreeMap::from([(composite, member)]),
                )?
                .ok_or_else(|| {
                    invalid_graph(format!(
                        "generated fa'u tag connection branch {} could not be distributed",
                        branch_index + 1,
                    ))
                })?;
            distributed_children.push(distributed);
            subject_members.push(member);
        }

        let body = self.next_formula_id();
        self.insert(
            body,
            SemanticObject::connective_formula(
                FormulaOperator::And,
                distributed_children.clone(),
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(connection.source.clone()),
                    locus: connection.locus,
                    truth_table: None,
                    parameter: None,
                })),
                source.clone(),
                Vec::new(),
            ),
        )?;
        let branch_slot = self.build_generated_parameter_with_source(
            "fa'u".to_owned(),
            source.clone(),
            SemanticSort::Proposition,
            ParameterRole::RespectiveSlot,
        )?;
        let mut streams = Vec::with_capacity(usize::from(has_composites) + 1);
        if has_composites {
            let subject_slot = self.build_generated_parameter_with_source(
                "fa'u".to_owned(),
                source.clone(),
                SemanticSort::Entity,
                ParameterRole::RespectiveSlot,
            )?;
            streams.push(RespectivelyStream::new(subject_slot, subject_members));
        }
        streams.push(RespectivelyStream::new(
            branch_slot,
            distributed_children.clone(),
        ));
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::respectively_distribution_formula(
                body,
                streams,
                None,
                source,
                Vec::new(),
            ),
        )?;
        Ok(formula)
    }

    #[requires(place_limit > 0)]
    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_logical_modal_connection_branch_formula<'syntax: 'tree, F>(
        &mut self,
        branch_formula_source: Option<crate::model::SemanticSource>,
        relation: &str,
        direct_relation_place_count: Option<usize>,
        place_limit: usize,
        terms: &[GeneratedBridiTermRef<'syntax>],
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        conversions: &[WithFreeModifiers<Token, F>],
        shared_head_assignments: GeneratedTermAssignments<'syntax>,
        branch: &GeneratedLogicalTagConnectionBranch<'syntax>,
        predication_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut branch_adjunct = match branch.as_data() {
            data!(GeneratedLogicalTagConnectionBranch::Modal { term, argument }) => {
                Some(match term.kind.as_data() {
                    data!(GeneratedConnectedModalTermKind::Named {
                        introduced_by,
                        relation,
                        visible_place,
                    }) => {
                        let adjuncts_for_relation = self.adjunct_map_for_visible_place(
                            argument.clone(),
                            *visible_place,
                            relation_place_count(self.dictionary, relation),
                        )?;
                        self.generated_adjunct_with_tense_modal_modifiers(
                            &term.tense_modal,
                            relation.clone(),
                            introduced_by.clone(),
                            adjuncts_for_relation,
                            generated_adjunct_negation_for_tense_modal(&term.tense_modal),
                            generated_modal_scalar_negation_for_tense_modal(&term.tense_modal),
                            "modal-argument",
                        )
                    }
                    data!(GeneratedConnectedModalTermKind::AdHoc { fiho }) => self
                        .build_generated_fiho_adjunct_for_selbri(
                            &term.tense_modal,
                            fiho.selbri.as_ref(),
                            argument.clone(),
                            "modal-argument",
                        )?,
                })
            }
            data!(GeneratedLogicalTagConnectionBranch::Event { .. }) => None,
        };
        let eventuality =
            self.build_generated_predication_eventuality(predication_source.clone())?;
        if let Some(adjunct) = &mut branch_adjunct {
            self.bind_generated_adjunct_to_host_event(adjunct, eventuality);
        }
        if let data!(GeneratedLogicalTagConnectionBranch::Event { branch, anchor }) =
            branch.as_data()
        {
            self.record_generated_tense_modal_event_modifier(
                eventuality,
                &branch.tense_modal,
                *anchor,
            )?;
            self.flush_generated_event_modifiers(eventuality)?;
        }
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, terms)?;
        let assignments = self.with_temporal_context(eventuality, |builder| {
            builder.build_term_assignments_for_terms_with_shared_tail_source(
                terms.to_vec(),
                first_visible_place,
                shared_tail_start,
            )
        })?;
        let mut place_question_assignments = shared_head_assignments.place_questions.clone();
        place_question_assignments.extend(assignments.place_questions.clone());
        let mut visible_arguments = BTreeMap::new();
        for (visible_place, argument) in &shared_head_assignments.visible_arguments {
            insert_visible_argument(&mut visible_arguments, *visible_place, argument.clone())?;
        }
        for (visible_place, argument) in assignments.visible_arguments {
            insert_visible_argument(&mut visible_arguments, visible_place, argument)?;
        }
        let visible_arguments =
            map_visible_arguments_for_generated_conversions(visible_arguments, conversions)?;
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in visible_arguments {
            arguments.insert(argument_key(visible_place), argument);
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
            &arguments,
            direct_relation_place_count,
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
        let mut diagnostics = Vec::new();
        if direct_relation_place_count.is_none() && !relation_has_open_place_structure(relation) {
            diagnostics.push(diagnostic(
                "relation place structure is unavailable; only places required by explicit assignments are represented",
            ));
        }
        let inherited_adjuncts = self.sticky_adjuncts.clone();
        let mut adjuncts = branch_adjunct.into_iter().collect::<Vec<_>>();
        self.append_generated_sticky_adjuncts(
            &inherited_adjuncts,
            &mut adjuncts,
            Some(eventuality),
        );
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation.to_owned(),
            Some(eventuality),
            arguments,
            predication_mode_for_relation(relation, PredicationMode::Asserted),
            predication_source,
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
        self.attach_generated_reciprocity_to_predication_for_terms(predication, terms)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, branch_formula_source, Vec::new()),
        )?;
        self.attach_generated_modal_terms_to_formula(
            formula,
            &shared_head_assignments.modal_terms,
        )?;
        let mut scoped = GeneratedScopedFormula {
            formula,
            formula_scopes: assignments.formula_scopes,
            coequal_scope_groups: assignments.coequal_scope_groups,
            implicit_existentials: assignments.implicit_existentials,
            term_formula_scopes: assignments.term_formula_scopes,
        };
        scoped = self.append_generated_term_assignment_scopes(scoped, shared_head_assignments);
        let formula = self.wrap_generated_scoped_formula(scoped)?;
        let (negated, tense_modal) = match branch.as_data() {
            data!(GeneratedLogicalTagConnectionBranch::Modal { term, .. }) => {
                (term.negated, &term.tense_modal)
            }
            data!(GeneratedLogicalTagConnectionBranch::Event { branch, .. }) => {
                (branch.negated, &branch.tense_modal)
            }
        };
        if negated {
            return self.build_unary_formula(
                FormulaOperator::Not,
                formula,
                self.source_for_node(tense_modal, "tag-negation"),
            );
        }
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(!predication_construct.is_empty())]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_from_terms_with_source_constructs<
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        predication_construct: &'static str,
        formula_construct: &'static str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        // Direct term connections (`ti'u li so bi'o ti'u li pano`, `broda .e brode` in argument
        // position) must be intercepted before the terms reach simple-term assignment lowering,
        // which cannot represent a connected term and would trip a graph invariant. This is the
        // shared choke point every bridi-tail lowering path funnels through, so guarding here
        // routes top-level bridi, relation-only bridi (relative-clause / abstraction bodies), and
        // prefix-term bridi uniformly through the same handler instead of relying on each caller
        // to add its own check.
        if let Some(formula) = self.build_generated_direct_term_connection_formula(
            simple_tail,
            &terms,
            first_visible_place,
            eventuality,
            mode,
        )? {
            return Ok(formula);
        }
        let predication_source = self.source_for_node(source_node, predication_construct);
        let formula_source = self.source_for_node(source_node, formula_construct);
        let abstraction = if terms.is_empty() && eventuality.is_none() {
            self.single_abstraction_from_selbri(&simple_tail.selbri)?
        } else {
            None
        };
        if let Some(abstraction) = abstraction {
            return self.build_abstraction_link_formula_for_visible_argument(
                abstraction,
                None,
                formula_source,
                mode,
            );
        }
        if eventuality.is_none()
            && mode == PredicationMode::Asserted
            && allow_single_argument_distribution
            && let [term] = terms.as_slice()
            && let Some(sumti) = simple_sumti_from_term(*term)
        {
            if let Some(description) = no_gadri_description_from_sumti(sumti)? {
                return self.build_no_gadri_quantified_argument_formula(
                    simple_tail,
                    description,
                    predication_source,
                    formula_source,
                );
            }
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && generated_single_tanru_unit_from_tanru_selbri(tanru).is_none()
        {
            return self.build_tanru_formula_for_terms_with_head_eventuality_order_and_mode(
                tanru,
                terms,
                first_visible_place,
                eventuality,
                mode,
                false,
                self.source_for_node(source_node, generated_tanru_formula_source_construct(tanru)),
            );
        }
        if let Some(sumti_selbri) = sumti_selbri_from_selbri(&simple_tail.selbri)? {
            return self.build_sumti_selbri_formula_for_terms(
                sumti_selbri,
                terms,
                first_visible_place,
                eventuality,
                mode,
                self.source_for_node(source_node, "tanru-formula"),
            );
        }
        if eventuality.is_none()
            && mode == PredicationMode::Asserted
            && let Some(formula) = self.build_generated_connected_mekso_identity_formula(
                simple_tail,
                terms.clone(),
                first_visible_place,
            )?
        {
            return Ok(formula);
        }
        if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
            && let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(tanru)
            && (generated_tanru_unit_is_grouped(unit)?
                || generated_tanru_unit_has_scalar_negated_base(unit))
        {
            return self.build_relation_formula_for_generated_tanru_unit_terms(
                unit,
                terms,
                first_visible_place,
                eventuality,
                mode,
                self.source_for_node(
                    source_node,
                    generated_tanru_unit_formula_source_construct(unit),
                ),
                self.source_for_node(
                    source_node,
                    generated_tanru_unit_formula_source_construct(unit),
                ),
            );
        }
        self.build_simple_tail_formula_with_options(
            simple_tail,
            terms,
            first_visible_place,
            eventuality,
            mode,
            predication_source,
            formula_source,
        )
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.is_none_or(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula)) || ret.is_err())]
    pub(super) fn build_generated_connected_mekso_identity_formula(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
    ) -> Result<Option<SemanticObjectId>, SemanticsError> {
        let Ok(relation) = relation_label_from_selbri(&simple_tail.selbri) else {
            return Ok(None);
        };
        if !matches!(
            semantic_relation_label(relation).as_data(),
            data!(RelationLabel::Identity)
        ) {
            return Ok(None);
        }
        let Ok(assignments) =
            generated_numbered_sumti_assignments_for_terms(&terms, first_visible_place)
        else {
            return Ok(None);
        };
        for (connected_index, (connected_place, connected_sumti)) in assignments.iter().enumerate()
        {
            let Some(SumtiBaseSyntax::NumberSumti(number)) =
                simple_sumti_base_from_sumti(connected_sumti)
            else {
                continue;
            };
            if let Some(expansion) =
                first_generated_connected_mekso_operator(number.expression.as_ref())?
            {
                let data!(GeneratedConnectedMeksoOperatorExpansion {
                    left_operator,
                    right_operator,
                    operator,
                    connector,
                }) = expansion.into_data();
                let source = self.source_for_node(number, "operator-connection-formula");
                let left = self.build_generated_connected_mekso_operator_identity_branch_formula(
                    &assignments,
                    connected_index,
                    *connected_place,
                    number.expression.as_ref(),
                    &left_operator,
                    number,
                    source.clone(),
                )?;
                let right = self.build_generated_connected_mekso_operator_identity_branch_formula(
                    &assignments,
                    connected_index,
                    *connected_place,
                    number.expression.as_ref(),
                    &right_operator,
                    number,
                    source.clone(),
                )?;
                let formula = self.next_formula_id();
                self.insert(
                    formula,
                    SemanticObject::connective_formula(
                        operator,
                        vec![left, right],
                        Some(connector),
                        source,
                        Vec::new(),
                    ),
                )?;
                return Ok(Some(formula));
            }
            let Some(operand) =
                generated_forethought_mekso_operand_from_mekso(number.expression.as_ref())
            else {
                continue;
            };
            let source = self.source_for_node(number, "operand-connection-formula");
            let left = self.build_generated_connected_mekso_identity_branch_formula(
                &assignments,
                connected_index,
                *connected_place,
                &operand.left_expression,
                number,
                source.clone(),
            )?;
            let right = self.build_generated_connected_simple_mekso_identity_branch_formula(
                &assignments,
                connected_index,
                *connected_place,
                &operand.right_expression,
                number,
                source.clone(),
            )?;
            let left = if generated_modal_forethought_connective_negates_left(&operand.gek) {
                self.build_unary_formula(FormulaOperator::Not, left, source.clone())?
            } else {
                left
            };
            let right = if generated_gik_connective_negates_right(&operand.gik) {
                self.build_unary_formula(FormulaOperator::Not, right, source.clone())?
            } else {
                right
            };
            self.mark_generated_modal_forethought_whether_or_not_inert_operand(
                &operand.gek,
                left,
                right,
            );
            let base_operator =
                generated_modal_forethought_connective_formula_operator(&operand.gek);
            let modal_connection_spec =
                generated_modal_statement_connection_spec_for_tense_modal(&operand.gek);
            let mut children = if modal_connection_spec.is_some() {
                vec![left, right]
            } else if generated_modal_forethought_connective_has_se(&operand.gek)
                && base_operator != FormulaOperator::WhetherOrNot
            {
                vec![right, left]
            } else {
                vec![left, right]
            };
            let mut diagnostics = Vec::new();
            let operator = if let Some(spec) = modal_connection_spec {
                match self.build_generated_modal_formula_connection_claim(
                    left,
                    right,
                    &spec,
                    source.clone(),
                )? {
                    Some(claim) => children.push(claim),
                    None => diagnostics.push(diagnostic(
                        "modal operand connection could not find formulas to relate",
                    )),
                }
                FormulaOperator::And
            } else {
                base_operator
            };
            let formula = self.next_formula_id();
            self.insert(
                formula,
                SemanticObject::connective_formula(
                    operator,
                    children,
                    Some(new!(Connector {
                        source: ConnectorSource::surface_word(
                            generated_modal_forethought_pair_source(&operand.gek, &operand.gik),
                        ),
                        locus: ConnectorLocus::Operand,
                        truth_table: generated_modal_forethought_gik_connective_truth_table(
                            &operand.gek,
                            &operand.gik,
                        ),
                        parameter: None,
                    })),
                    source,
                    diagnostics,
                ),
            )?;
            return Ok(Some(formula));
        }
        Ok(None)
    }

    #[requires(connected_place > 0)]
    #[requires(connected_index < assignments.len())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_connected_mekso_identity_branch_formula(
        &mut self,
        assignments: &[(usize, &'tree SumtiSyntax)],
        connected_index: usize,
        connected_place: usize,
        expression: &'tree MeksoOperandSyntax,
        number: &'tree NumberSumtiSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        for (index, (place, sumti)) in assignments.iter().enumerate() {
            if index == connected_index {
                continue;
            }
            arguments.insert(
                argument_key(*place),
                self.build_argument_for_generated_sumti(sumti)?,
            );
        }
        let branch_referent =
            self.build_number_referent_for_generated_mekso_operand(expression, number)?;
        arguments.insert(
            argument_key(connected_place),
            ArgumentValue::filled(branch_referent, None),
        );
        let predication = self.build_generated_predication_from_arguments(
            "identity".to_owned(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(connected_place > 0)]
    #[requires(connected_index < assignments.len())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_connected_simple_mekso_identity_branch_formula(
        &mut self,
        assignments: &[(usize, &'tree SumtiSyntax)],
        connected_index: usize,
        connected_place: usize,
        expression: &'tree SimpleMeksoOperandSyntax,
        number: &'tree NumberSumtiSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        for (index, (place, sumti)) in assignments.iter().enumerate() {
            if index == connected_index {
                continue;
            }
            arguments.insert(
                argument_key(*place),
                self.build_argument_for_generated_sumti(sumti)?,
            );
        }
        let branch_referent =
            self.build_number_referent_for_generated_simple_mekso_operand(expression, number)?;
        arguments.insert(
            argument_key(connected_place),
            ArgumentValue::filled(branch_referent, None),
        );
        let predication = self.build_generated_predication_from_arguments(
            "identity".to_owned(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(connected_place > 0)]
    #[requires(connected_index < assignments.len())]
    #[ensures(ret.as_ref().is_ok_and(|formula| formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_connected_mekso_operator_identity_branch_formula(
        &mut self,
        assignments: &[(usize, &'tree SumtiSyntax)],
        connected_index: usize,
        connected_place: usize,
        expression: &'tree MeksoSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        number: &'tree NumberSumtiSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut arguments = BTreeMap::new();
        for (index, (place, sumti)) in assignments.iter().enumerate() {
            if index == connected_index {
                continue;
            }
            arguments.insert(
                argument_key(*place),
                self.build_argument_for_generated_sumti(sumti)?,
            );
        }
        let branch_referent = self.build_number_referent_for_generated_connected_operator_branch(
            expression,
            replacement_operator,
            number,
        )?;
        arguments.insert(
            argument_key(connected_place),
            ArgumentValue::filled(branch_referent, None),
        );
        let predication = self.build_generated_predication_from_arguments(
            "identity".to_owned(),
            source.clone(),
            arguments,
            Vec::new(),
        )?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, source, Vec::new()),
        )?;
        Ok(formula)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_number_referent_for_generated_connected_operator_branch(
        &mut self,
        expression: &'tree MeksoSyntax,
        replacement_operator: &MeksoOperatorSyntax,
        number: &'tree NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let (math, replaced) = self
            .build_generated_math_expression_with_connected_operator_replacement(
                expression,
                replacement_operator,
                self.source_for_node(number, "math-expression"),
            )?;
        let name = generated_mekso_surface_text_with_connected_operator_replacement(
            expression,
            replacement_operator,
        )?
        .unwrap_or_else(|| {
            generated_mekso_surface_text(expression).unwrap_or_else(|_| "mekso".to_owned())
        });
        let quantity = self.next_quantity_id();
        let mut diagnostics = Vec::new();
        if !replaced {
            diagnostics.push(diagnostic(
                "connected mekso operator branch did not replace an operator",
            ));
        }
        self.insert(quantity, {
            let mut object = SemanticObject::quantity(
                quantity_form_for_text(&name),
                QuantityValue::math_expression(math),
                QuantityScale::Count,
                self.source_for_node(number, "quantity"),
            );
            object.replace_diagnostics(diagnostics);
            object
        })?;
        self.build_number_referent_with_quantity(
            &number.li,
            name,
            quantity,
            self.source_for_node(number, "number-sumti"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_number_referent_for_generated_mekso_operand(
        &mut self,
        expression: &'tree MeksoOperandSyntax,
        number: &'tree NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_mekso_operand_surface_text(expression)?;
        let quantity = self.build_quantity_for_generated_mekso_operand(
            expression,
            self.source_for_node(number, "quantity"),
        )?;
        self.build_number_referent_with_quantity(
            &number.li,
            text,
            quantity,
            self.source_for_node(number, "number-sumti"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Referent && id.referent_sort() == Some(SemanticSort::Number)) || ret.is_err())]
    pub(super) fn build_number_referent_for_generated_simple_mekso_operand(
        &mut self,
        expression: &'tree SimpleMeksoOperandSyntax,
        number: &'tree NumberSumtiSyntax,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_simple_mekso_operand_surface_text(expression)?;
        let quantity = self.build_quantity_for_generated_simple_mekso_operand(
            expression,
            self.source_for_node(number, "quantity"),
        )?;
        self.build_number_referent_with_quantity(
            &number.li,
            text,
            quantity,
            self.source_for_node(number, "number-sumti"),
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_mekso_operand(
        &mut self,
        expression: &'tree MeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_mekso_operand_surface_text(expression)?;
        let mut value = generated_simple_pa_quantity_value_for_mekso_operand(expression)
            .map_or_else(
                || {
                    self.build_generated_mekso_operand(
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
        let question_parameters =
            self.build_quantity_question_parameters_for_generated_node(expression)?;
        if !question_parameters.is_empty() {
            value = value.with_question_parameters(question_parameters);
        }
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
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Quantity) || ret.is_err())]
    pub(super) fn build_quantity_for_generated_simple_mekso_operand(
        &mut self,
        expression: &'tree SimpleMeksoOperandSyntax,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let text = generated_simple_mekso_operand_surface_text(expression)?;
        let mut value = generated_simple_pa_quantity_value_for_simple_mekso_operand(expression)
            .map_or_else(
                || {
                    self.build_generated_simple_mekso_operand(
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
        let question_parameters =
            self.build_quantity_question_parameters_for_generated_node(expression)?;
        if !question_parameters.is_empty() {
            value = value.with_question_parameters(question_parameters);
        }
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

    #[requires(first_visible_place > 0)]
    #[requires(preassigned_visible_arguments.keys().all(|place| *place > 0))]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_with_preassigned_arguments<N: TreeNode>(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        preassigned_place_questions: &[GeneratedPlaceQuestionAssignment],
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
            source_node,
            simple_tail,
            preassigned_visible_arguments,
            preassigned_place_questions,
            terms,
            None,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            "bridi-formula",
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(preassigned_visible_arguments.keys().all(|place| *place > 0))]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct<
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        preassigned_place_questions: &[GeneratedPlaceQuestionAssignment],
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        formula_construct: &'static str,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct_excluding_source(
            source_node,
            simple_tail,
            preassigned_visible_arguments,
            preassigned_place_questions,
            terms,
            shared_tail_start,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            formula_construct,
            None,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(preassigned_visible_arguments.keys().all(|place| *place > 0))]
    #[requires(!formula_construct.is_empty())]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct_excluding_source<
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        preassigned_place_questions: &[GeneratedPlaceQuestionAssignment],
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        formula_construct: &'static str,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if preassigned_visible_arguments.is_empty()
            && preassigned_place_questions.is_empty()
            && shared_tail_start.is_none()
            && excluded_source.is_none()
        {
            return self.build_selbri_simple_bridi_tail_formula_from_terms_with_formula_construct(
                source_node,
                simple_tail,
                terms,
                first_visible_place,
                eventuality,
                mode,
                allow_single_argument_distribution,
                formula_construct,
            );
        }
        // The shared-tail / excluded-source branches below bypass
        // `build_selbri_simple_bridi_tail_formula_from_terms_with_source_constructs`, so when there
        // are no preassigned arguments to thread through the connection, route any direct term
        // connection through the same guard the other paths use so it is built (logical) or reported
        // gracefully (nonlogical) rather than reaching simple-term assignment lowering. Connections
        // that arrive here with preassigned arguments (or on any other unguarded path) are caught at
        // the shared choke point in `insert_generated_term_assignment`.
        if preassigned_visible_arguments.is_empty()
            && preassigned_place_questions.is_empty()
            && let Some(formula) = self.build_generated_direct_term_connection_formula(
                simple_tail,
                &terms,
                first_visible_place,
                eventuality,
                mode,
            )?
        {
            return Ok(formula);
        }
        let predication_source = self.source_for_node(source_node, "predication");
        let formula_source = self.source_for_node(source_node, formula_construct);
        if excluded_source.is_none()
            && let SelbriSyntax::TaggedSelbri(tagged) = simple_tail.selbri.as_ref()
            && generated_connected_event_tense_spec_for_tense_modal(tagged.tense_modal.as_ref())
                .is_none()
            && !generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref())
            && let UntaggedSelbriSyntax::CoSelbri(co_selbri) = tagged.inner_selbri.as_ref()
            && let Some(tanru) = tanru_selbri_from_co_selbri(co_selbri)?
            && generated_single_tanru_unit_from_tanru_selbri(tanru).is_none()
            && terms
                .iter()
                .any(|term| generated_term_has_distributed_sumti_connection(*term))
        {
            if generated_tense_modal_resets_sticky_modals(tagged.tense_modal.as_ref()) {
                self.sticky_adjuncts.clear();
            }
            if generated_tense_modal_resets_sticky_tense(tagged.tense_modal.as_ref()) {
                self.sticky_time_path.clear();
                self.sticky_space_path.clear();
            }
            let mut base_assignments = empty_generated_term_assignments();
            base_assignments.visible_arguments = preassigned_visible_arguments.clone();
            base_assignments.place_questions = preassigned_place_questions.to_vec();
            base_assignments.next_visible_place = first_visible_place;
            let source =
                self.source_for_node(source_node, generated_tanru_formula_source_construct(tanru));
            return self.build_tagged_selbri_formula_around_child(
                tagged,
                eventuality,
                predication_source.clone(),
                |builder, child_eventuality| {
                    let Some(formula) = builder
                        .build_generated_logical_sumti_connection_formula_for_tanru_terms_with_preassigned(
                            tanru,
                            &terms,
                            &base_assignments,
                            first_visible_place,
                            child_eventuality,
                            mode,
                            source,
                        )?
                    else {
                        return Err(invalid_graph(
                            "connected tanru argument disappeared after tagged-selbri dispatch"
                                .to_owned(),
                        ));
                    };
                    Ok(formula)
                },
            );
        }
        if excluded_source.is_none()
            && let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) =
                simple_tail.selbri.as_ref()
            && co_selbri.co_tail.is_none()
            && let Some(tanru) = tanru_selbri_from_co_selbri(co_selbri)?
            && let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(tanru)
            && let (atom, _) = generated_linked_tanru_unit_parts(unit)?
            && generated_jai_modal_tanru_unit(atom.base.as_ref()).is_some()
            && !self.split_generated_fai_terms(terms.clone())?.1.is_empty()
        {
            return self
                .build_relation_formula_for_generated_tanru_unit_terms_with_preassigned_arguments(
                    unit,
                    preassigned_visible_arguments,
                    preassigned_place_questions,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    predication_source,
                    formula_source,
                );
        }
        if preassigned_place_questions.is_empty() && excluded_source.is_none() {
            if eventuality.is_none()
                && let Ok(relation) = relation_label_from_selbri(&simple_tail.selbri)
            {
                let relation = semantic_relation_label(relation);
                let place_count = relation_place_count(self.dictionary, &relation);
                let place_limit = place_count.unwrap_or_else(|| {
                    preassigned_visible_arguments
                        .keys()
                        .copied()
                        .max()
                        .unwrap_or(0)
                        .max(terms.len())
                        .max(1)
                });
                if let Some(formula) = self
                    .build_generated_logical_sumti_connection_formula_for_terms_with_preassigned_arguments(
                        &relation.display_text(),
                        &terms,
                        preassigned_visible_arguments,
                        first_visible_place,
                        place_limit,
                        &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
                        mode,
                        predication_source.clone(),
                        formula_source.clone(),
                    )?
                {
                    return Ok(formula);
                }
            }
            if let Some(tanru) = tanru_selbri_from_selbri(&simple_tail.selbri)?
                && generated_single_tanru_unit_from_tanru_selbri(tanru).is_none()
            {
                let mut base_assignments = empty_generated_term_assignments();
                base_assignments.visible_arguments = preassigned_visible_arguments.clone();
                base_assignments.place_questions = preassigned_place_questions.to_vec();
                base_assignments.next_visible_place = first_visible_place;
                if let Some(formula) = self
                    .build_generated_logical_sumti_connection_formula_for_tanru_terms_with_preassigned(
                        tanru,
                        &terms,
                        &base_assignments,
                        first_visible_place,
                        eventuality,
                        mode,
                        self.source_for_node(
                            source_node,
                            generated_tanru_formula_source_construct(tanru),
                        ),
                    )?
                {
                    return Ok(formula);
                }
            }
        }
        let assignments = match excluded_source {
            Some(excluded_source) => {
                self.build_term_assignments_for_terms_excluding_source(
                    terms,
                    first_visible_place,
                    Some(excluded_source),
                )?
                .0
            }
            None => self.build_term_assignments_for_terms_with_shared_tail_source(
                terms,
                first_visible_place,
                shared_tail_start,
            )?,
        };
        if relation_label_from_selbri(&simple_tail.selbri).is_err()
            && preassigned_place_questions.is_empty()
            && assignments.place_questions.is_empty()
        {
            let mut visible_arguments = preassigned_visible_arguments.clone();
            for (visible_place, argument) in assignments.visible_arguments {
                if visible_arguments.insert(visible_place, argument).is_some() {
                    return Err(invalid_graph(format!(
                        "multiple generated structural selbri arguments map to x{visible_place}"
                    )));
                }
            }
            let result = self.build_selbri_formula_for_visible_arguments(
                &simple_tail.selbri,
                visible_arguments,
                formula_source,
                ConnectorLocus::Predicate,
                eventuality,
            )?;
            self.set_semantic_object_source(result.head_predication, predication_source)?;
            self.attach_generated_modal_terms_to_formula(result.formula, &assignments.modal_terms)?;
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            return self.wrap_formula_with_generated_assignment_scopes(
                result.formula,
                assignments.formula_scopes,
                assignments.coequal_scope_groups,
                assignments.implicit_existentials,
                assignments.term_formula_scopes,
            );
        }
        let relation = semantic_relation_label(relation_label_from_selbri(&simple_tail.selbri)?);
        let place_count = relation_place_count(self.dictionary, &relation);
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in preassigned_visible_arguments {
            arguments.insert(argument_key(*visible_place), argument.clone());
        }
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated forethought branch arguments map to {key}"
                )));
            }
        }
        let mut place_question_assignments = preassigned_place_questions.to_vec();
        place_question_assignments.extend(assignments.place_questions.clone());
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let mut diagnostics = Vec::new();
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
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(predication_source.clone())?,
        };
        self.apply_generated_tagged_term_event_modifiers(eventuality, &assignments.modal_terms)?;
        let adjuncts = self
            .build_adjuncts_for_generated_tagged_terms_for_event_with_predication_arguments(
                eventuality,
                &assignments.modal_terms,
                Some(&arguments),
            )?;
        let predication_mode = predication_mode_for_relation(&relation, mode);
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            predication_mode,
            predication_source.clone(),
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|(id, context)| id.object_kind() == crate::model::SemanticObjectKind::Formula && context.assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_selbri_simple_bridi_tail_formula_with_deferred_prefix_assignments<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        shared_tail_start: Option<usize>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
    ) -> Result<(SemanticObjectId, GeneratedForethoughtPrefixContext<'syntax>), SemanticsError>
    {
        if prefix_terms.is_empty() {
            let formula = if let Some(shared_tail_start) = shared_tail_start {
                self.build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
                    source_node,
                    simple_tail,
                    &BTreeMap::new(),
                    &[],
                    terms,
                    Some(shared_tail_start),
                    first_visible_place,
                    eventuality,
                    mode,
                    allow_single_argument_distribution,
                    "bridi-formula",
                )?
            } else {
                self.build_selbri_simple_bridi_tail_formula_from_terms(
                    source_node,
                    simple_tail,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    allow_single_argument_distribution,
                )?
            };
            return Ok((
                formula,
                GeneratedForethoughtPrefixContext {
                    assignments: empty_generated_term_assignments(),
                    adjuncts: Vec::new(),
                },
            ));
        }
        if mode != PredicationMode::Asserted {
            return Err(invalid_graph(
                "deferred prefix terms reached asserted-only branch lowering".to_owned(),
            ));
        }
        let local_assignments = self.build_term_assignments_for_terms_with_shared_tail_source(
            terms,
            first_visible_place,
            shared_tail_start,
        )?;
        let prefix_assignments = self.build_term_assignments_for_terms(prefix_terms.to_vec(), 1)?;
        let shared_adjuncts =
            self.build_adjuncts_for_generated_tagged_terms(&prefix_assignments.modal_terms)?;
        let mut place_question_assignments = prefix_assignments.place_questions.clone();
        place_question_assignments.extend(local_assignments.place_questions.clone());

        let relation = match relation_label_from_selbri(&simple_tail.selbri) {
            Ok(relation) => semantic_relation_label(relation),
            Err(_) if place_question_assignments.is_empty() => {
                let mut visible_arguments = prefix_assignments.visible_arguments.clone();
                for (visible_place, argument) in &local_assignments.visible_arguments {
                    insert_visible_argument(
                        &mut visible_arguments,
                        *visible_place,
                        argument.clone(),
                    )?;
                }
                let result = self.build_selbri_formula_for_visible_arguments(
                    &simple_tail.selbri,
                    visible_arguments,
                    self.source_for_node(source_node, "bridi-formula"),
                    ConnectorLocus::Predicate,
                    eventuality,
                )?;
                self.attach_generated_modal_terms_to_formula(
                    result.formula,
                    &prefix_assignments.modal_terms,
                )?;
                self.attach_generated_modal_terms_to_formula(
                    result.formula,
                    &local_assignments.modal_terms,
                )?;
                let formula = self.wrap_formula_with_generated_assignment_scopes(
                    result.formula,
                    local_assignments.formula_scopes,
                    local_assignments.coequal_scope_groups,
                    local_assignments.implicit_existentials,
                    local_assignments.term_formula_scopes,
                )?;
                let branch_prenex_existentials = self
                    .generated_implicit_existentials_for_active_prenex_bindings(
                        &prefix_assignments.implicit_existentials,
                    );
                let formula = self
                    .wrap_formula_with_implicit_existentials(formula, branch_prenex_existentials)?;
                return Ok((
                    formula,
                    GeneratedForethoughtPrefixContext {
                        assignments: prefix_assignments,
                        adjuncts: shared_adjuncts,
                    },
                ));
            }
            Err(error) => return Err(error),
        };
        let place_count = relation_place_count(self.dictionary, &relation);

        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in &prefix_assignments.visible_arguments {
            arguments.insert(argument_key(*visible_place), argument.clone());
        }
        for (visible_place, argument) in local_assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated forethought branch arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let mut diagnostics = Vec::new();
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
        let predication_source = self.source_for_node(source_node, "predication");
        let formula_source = self.source_for_node(source_node, "bridi-formula");
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(predication_source.clone())?,
        };
        self.apply_generated_tagged_term_event_modifiers(
            eventuality,
            &prefix_assignments.modal_terms,
        )?;
        self.apply_generated_tagged_term_event_modifiers(
            eventuality,
            &local_assignments.modal_terms,
        )?;
        let mut adjuncts = shared_adjuncts.clone();
        for adjunct in &mut adjuncts {
            self.bind_generated_adjunct_to_host_event(adjunct, eventuality);
        }
        adjuncts.extend(self.build_adjuncts_for_generated_tagged_terms_for_event(
            eventuality,
            &local_assignments.modal_terms,
        )?);
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation.display_text(),
            Some(eventuality),
            arguments,
            PredicationMode::Asserted,
            predication_source,
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        let formula = self.wrap_formula_with_generated_assignment_scopes(
            formula,
            local_assignments.formula_scopes,
            local_assignments.coequal_scope_groups,
            local_assignments.implicit_existentials,
            local_assignments.term_formula_scopes,
        )?;
        let branch_prenex_existentials = self
            .generated_implicit_existentials_for_active_prenex_bindings(
                &prefix_assignments.implicit_existentials,
            );
        let formula =
            self.wrap_formula_with_implicit_existentials(formula, branch_prenex_existentials)?;
        Ok((
            formula,
            GeneratedForethoughtPrefixContext {
                assignments: prefix_assignments,
                adjuncts: shared_adjuncts,
            },
        ))
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_forethought_bridi_connection_formula_with_shared_terms(
        &mut self,
        connection: &'tree ForethoughtBridiConnectionSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'tree>],
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match connection {
            ForethoughtBridiConnectionSyntax::DirectForethoughtBridiConnection(connection) => self
                .build_direct_forethought_bridi_connection_formula(
                    connection,
                    prefix_terms,
                    suffix_terms,
                    eventuality,
                    mode,
                ),
            ForethoughtBridiConnectionSyntax::GroupedForethoughtBridiConnection(connection) => self
                .build_grouped_forethought_bridi_connection_formula(
                    connection,
                    prefix_terms,
                    suffix_terms,
                    eventuality,
                    mode,
                ),
            ForethoughtBridiConnectionSyntax::NegatedForethoughtBridiConnection(connection) => self
                .build_negated_forethought_bridi_connection_formula(
                    connection,
                    prefix_terms,
                    suffix_terms,
                    eventuality,
                    mode,
                ),
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_direct_forethought_bridi_connection_formula(
        &mut self,
        connection: &'tree DirectForethoughtBridiConnectionSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'tree>],
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let modal_connection_spec =
            generated_modal_statement_connection_spec_for_tense_modal(&connection.gek);
        if !connection.additional_branches.is_empty()
            && (!generated_modal_forethought_connective_is_logical(&connection.gek)
                || modal_connection_spec.is_some())
        {
            return Err(undefined_semantics(
                "an experimental n-ary modal or nonlogical forethought bridi connection",
            ));
        }
        let first_eventuality = match eventuality {
            Some(eventuality) => Some(eventuality),
            None if prefix_terms.is_empty()
                || generated_subbridi_is_connected_bridi_tail(&connection.first) =>
            {
                None
            }
            None => Some(
                self.build_eventuality(self.source_for_node(&connection.first, "predication"))?,
            ),
        };
        let first_visible_place = next_visible_place_after_generated_terms(prefix_terms, 1)?;
        let mut branch_suffix_terms =
            Vec::with_capacity(connection.tail_terms.len() + suffix_terms.len());
        branch_suffix_terms.extend(
            connection
                .tail_terms
                .iter()
                .map(GeneratedBridiTermRef::Term),
        );
        branch_suffix_terms.extend_from_slice(suffix_terms);

        let (first_formula, prefix_context) = self
            .build_forethought_subbridi_branch_formula_with_deferred_prefix(
                &connection.first,
                prefix_terms,
                first_visible_place,
                &branch_suffix_terms,
                first_eventuality,
            )?;

        let second_eventuality = if prefix_terms.is_empty() {
            None
        } else {
            Some(self.build_eventuality(
                self.source_for_node(&connection.first_branch.branch, "predication"),
            )?)
        };
        let branch_prenex_existentials = self
            .generated_implicit_existentials_for_active_prenex_bindings(
                &prefix_context.assignments.implicit_existentials,
            );
        let second_formula = self.build_forethought_subbridi_branch_formula(
            &connection.first_branch.branch,
            &prefix_context.assignments.visible_arguments,
            first_visible_place,
            &branch_suffix_terms,
            second_eventuality,
            &branch_prenex_existentials,
        )?;
        for adjunct in &prefix_context.adjuncts {
            self.attach_adjunct_to_generated_formula(second_formula, adjunct)?;
        }
        if let Some(anchor) = self.current_utterance {
            self.attach_generated_indicator_displays_with_target_focus(
                indicator_parts_for_generated_node(&connection.gek),
                first_formula,
                anchor,
                "indicator",
                None,
                false,
            )?;
            self.attach_generated_indicator_displays_with_target_focus(
                indicator_parts_for_generated_node(&connection.first_branch.gik),
                second_formula,
                anchor,
                "indicator",
                None,
                false,
            )?;
        }

        self.mark_generated_modal_forethought_whether_or_not_inert_operand(
            &connection.gek,
            first_formula,
            second_formula,
        );
        let left = if generated_modal_forethought_connective_negates_left(&connection.gek) {
            self.build_unary_formula(FormulaOperator::Not, first_formula, None)?
        } else {
            first_formula
        };
        let right = if generated_gik_connective_negates_right(&connection.first_branch.gik) {
            self.build_unary_formula(FormulaOperator::Not, second_formula, None)?
        } else {
            second_formula
        };
        let base_operator =
            generated_modal_forethought_connective_formula_operator(&connection.gek);
        let pure_modal_connection = !prefix_terms.is_empty()
            && modal_connection_spec.is_some()
            && generated_modal_forethought_connective_is_pure_modal(&connection.gek);
        let mut children = if modal_connection_spec.is_some() {
            vec![left, right]
        } else if generated_modal_forethought_connective_has_se(&connection.gek)
            && base_operator != FormulaOperator::WhetherOrNot
        {
            vec![right, left]
        } else {
            vec![left, right]
        };
        let mut diagnostics = Vec::new();
        let mut modal_claim = None;
        let operator = if let Some(spec) = modal_connection_spec {
            let (visible_formula, other_formula) =
                if generated_tense_relation_spec_for_tense_modal(&connection.gek).is_none() {
                    (left, right)
                } else {
                    (right, left)
                };
            match self.build_generated_modal_formula_connection_claim(
                    visible_formula,
                    other_formula,
                    &spec,
                    None,
                )? {
                    Some(claim) => {
                        if pure_modal_connection {
                            self.set_formula_predication_mode(
                                first_formula,
                                PredicationMode::Inert,
                            );
                            self.set_formula_predication_mode(
                                second_formula,
                                PredicationMode::Inert,
                            );
                            modal_claim = Some(claim);
                        } else {
                            children.push(claim);
                        }
                    }
                    None => diagnostics.push(diagnostic(
                        "modal forethought connection could not find formula-bearing bridi events to relate",
                    )),
                }
            FormulaOperator::And
        } else {
            base_operator
        };
        if let Some(claim) = modal_claim {
            let formula = self.wrap_formula_with_generated_assignment_scopes(
                claim,
                prefix_context.assignments.formula_scopes,
                prefix_context.assignments.coequal_scope_groups,
                prefix_context.assignments.implicit_existentials,
                prefix_context.assignments.term_formula_scopes,
            )?;
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(formula, mode);
            }
            return Ok(formula);
        }
        let connector_parameter = self
            .build_generated_connective_question_parameter_for_modal_forethought_connective(
                &connection.gek,
            )?;
        let mut formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(generated_modal_forethought_pair_source(
                        &connection.gek,
                        &connection.first_branch.gik,
                    ),),
                    locus: ConnectorLocus::Clause,
                    truth_table: generated_modal_forethought_gik_connective_truth_table(
                        &connection.gek,
                        &connection.first_branch.gik,
                    ),
                    parameter: connector_parameter,
                })),
                None,
                diagnostics,
            ),
        )?;
        for branch in &connection.additional_branches {
            let branch_eventuality = if prefix_terms.is_empty() {
                None
            } else {
                Some(self.build_eventuality(self.source_for_node(&branch.branch, "predication"))?)
            };
            let branch_formula = self.build_forethought_subbridi_branch_formula(
                &branch.branch,
                &prefix_context.assignments.visible_arguments,
                first_visible_place,
                &branch_suffix_terms,
                branch_eventuality,
                &branch_prenex_existentials,
            )?;
            for adjunct in &prefix_context.adjuncts {
                self.attach_adjunct_to_generated_formula(branch_formula, adjunct)?;
            }
            if let Some(anchor) = self.current_utterance {
                self.attach_generated_indicator_displays_with_target_focus(
                    indicator_parts_for_generated_node(&branch.gik),
                    branch_formula,
                    anchor,
                    "indicator",
                    None,
                    false,
                )?;
            }
            let connector_source = format!(
                "{} {}",
                generated_modal_forethought_connective_source(&connection.gek),
                token_text(&branch.gik.0.value)
            );
            formula = self
                .build_binary_formula_for_generated_forethought_statement_connective_core(
                    &connection.gek,
                    false,
                    false,
                    connector_source,
                    ConnectorLocus::Clause,
                    formula,
                    branch_formula,
                    None,
                )?;
        }
        let formula = self.wrap_formula_with_generated_assignment_scopes(
            formula,
            prefix_context.assignments.formula_scopes,
            prefix_context.assignments.coequal_scope_groups,
            prefix_context.assignments.implicit_existentials,
            prefix_context.assignments.term_formula_scopes,
        )?;
        if mode != PredicationMode::Asserted {
            self.set_formula_predication_mode(formula, mode);
        }
        Ok(formula)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_grouped_forethought_bridi_connection_formula(
        &mut self,
        connection: &'tree GroupedForethoughtBridiConnectionSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'tree>],
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.build_forethought_bridi_connection_formula_with_shared_terms(
            &connection.inner,
            prefix_terms,
            suffix_terms,
            eventuality,
            mode,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_negated_forethought_bridi_connection_formula(
        &mut self,
        connection: &'tree NegatedForethoughtBridiConnectionSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'tree>],
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let child = self.build_forethought_bridi_connection_formula_with_shared_terms(
            &connection.inner,
            prefix_terms,
            suffix_terms,
            eventuality,
            mode,
        )?;
        self.build_unary_formula(FormulaOperator::Not, child, None)
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_bridi_tail_formula_with_shared_terms<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        tail: &'syntax BridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        annotate_shared_head_source: bool,
        annotate_compound_source: bool,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let BridiTailSyntax::BridiTailWithPossibleTailTerms(BridiTailWithPossibleTailTermsSyntax {
            first,
            ke_continuation,
        }) = tail
        else {
            return Err(invalid_graph(
                "connected-tail lowering received a bridi tail without possible terms".to_owned(),
            ));
        };
        let source = annotate_compound_source
            .then(|| self.source_for_node(source_node, "compound-bridi-formula"))
            .flatten();
        let leading_suffix_terms = if let Some(first_continuation) = first.0.links.first() {
            let mut leading_suffix_terms =
                Vec::with_capacity(first_continuation.tail_terms.len() + suffix_terms.len());
            leading_suffix_terms.extend(
                first_continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            leading_suffix_terms.extend_from_slice(suffix_terms);
            leading_suffix_terms
        } else {
            suffix_terms.to_vec()
        };
        let mut current = self.build_bo_grouped_bridi_tail_formula_with_shared_terms(
            first.0.first.as_ref(),
            prefix_terms,
            &leading_suffix_terms,
            first_visible_place,
            mode,
            allow_single_argument_distribution,
            annotate_shared_head_source,
        )?;
        for continuation in &first.0.links {
            let mut branch_suffix_terms =
                Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
            branch_suffix_terms.extend(
                continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            branch_suffix_terms.extend_from_slice(suffix_terms);
            let next = self.build_bo_grouped_bridi_tail_formula_with_shared_terms(
                &continuation.bridi_tail,
                prefix_terms,
                &branch_suffix_terms,
                first_visible_place,
                mode,
                allow_single_argument_distribution,
                annotate_shared_head_source,
            )?;
            let formula = self.build_binary_generated_bridi_tail_connection_formula(
                current.formula,
                next.formula,
                &continuation.connective,
                None,
                source.clone(),
            )?;
            current = self.with_generated_scoped_formula(current, formula);
            current = self.append_generated_scoped_formula_scopes(current, next);
        }
        if let Some(continuation) = ke_continuation.as_deref() {
            let mut branch_suffix_terms =
                Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
            branch_suffix_terms.extend(
                continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            branch_suffix_terms.extend_from_slice(suffix_terms);
            let next = self.build_connected_bridi_tail_formula_with_shared_terms(
                continuation,
                &continuation.bridi_tail,
                prefix_terms,
                &branch_suffix_terms,
                first_visible_place,
                None,
                mode,
                allow_single_argument_distribution,
                annotate_shared_head_source,
                false,
            )?;
            let connective =
                BridiTailConnectiveSyntax::GihekConnective(continuation.connective.clone());
            let formula = self.build_binary_generated_bridi_tail_connection_formula(
                current.formula,
                next,
                &connective,
                continuation.tense_modal.as_deref(),
                source,
            )?;
            current = self.with_generated_scoped_formula(current, formula);
        }
        let formula = self.wrap_generated_scoped_formula(current)?;
        let Some(eventuality) = eventuality else {
            return Ok(formula);
        };
        let scoped = self.next_formula_id();
        let mut object = SemanticObject::connective_formula(
            FormulaOperator::Scoped,
            vec![formula],
            None,
            self.source_for_node(source_node, "compound-event-scope"),
            Vec::new(),
        );
        object.set_scoped_formula_eventuality(Some(eventuality));
        self.insert(scoped, object)?;
        Ok(scoped)
    }

    #[requires(first_visible_place > 0)]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_connected_bridi_tail_formula_with_preassigned_shared_terms<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        tail: &'syntax BridiTailSyntax,
        assignments: &GeneratedTermAssignments<'syntax>,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        leading_eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let BridiTailSyntax::BridiTailWithPossibleTailTerms(BridiTailWithPossibleTailTermsSyntax {
            first,
            ke_continuation,
        }) = tail
        else {
            return Err(invalid_graph(
                "connected-tail lowering received a bridi tail without possible terms".to_owned(),
            ));
        };
        let source = self.source_for_node(source_node, "compound-bridi-formula");
        let leading_suffix_terms = if let Some(first_continuation) = first.0.links.first() {
            let mut leading_suffix_terms =
                Vec::with_capacity(first_continuation.tail_terms.len() + suffix_terms.len());
            leading_suffix_terms.extend(
                first_continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            leading_suffix_terms.extend_from_slice(suffix_terms);
            leading_suffix_terms
        } else {
            suffix_terms.to_vec()
        };
        let mut formula = self.build_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms(
            first.0.first.as_ref(),
            assignments,
            &leading_suffix_terms,
            first_visible_place,
            leading_eventuality,
            mode,
            allow_single_argument_distribution,
            excluded_source,
        )?;
        for continuation in &first.0.links {
            let mut branch_suffix_terms =
                Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
            branch_suffix_terms.extend(
                continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            branch_suffix_terms.extend_from_slice(suffix_terms);
            let next = self.build_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms(
                &continuation.bridi_tail,
                assignments,
                &branch_suffix_terms,
                first_visible_place,
                None,
                mode,
                allow_single_argument_distribution,
                excluded_source,
            )?;
            formula = self.build_binary_generated_bridi_tail_connection_formula(
                formula,
                next,
                &continuation.connective,
                None,
                source.clone(),
            )?;
        }
        if let Some(continuation) = ke_continuation.as_deref() {
            let mut branch_suffix_terms =
                Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
            branch_suffix_terms.extend(
                continuation
                    .tail_terms
                    .iter()
                    .map(GeneratedBridiTermRef::Term),
            );
            branch_suffix_terms.extend_from_slice(suffix_terms);
            let next = self.build_connected_bridi_tail_formula_with_preassigned_shared_terms(
                continuation,
                &continuation.bridi_tail,
                assignments,
                &branch_suffix_terms,
                first_visible_place,
                None,
                mode,
                allow_single_argument_distribution,
                excluded_source,
            )?;
            let connective =
                BridiTailConnectiveSyntax::GihekConnective(continuation.connective.clone());
            formula = self.build_binary_generated_bridi_tail_connection_formula(
                formula,
                next,
                &connective,
                continuation.tense_modal.as_deref(),
                source,
            )?;
        }
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms<
        'syntax: 'tree,
    >(
        &mut self,
        tail: &'syntax BoGroupedBridiTailSyntax,
        assignments: &GeneratedTermAssignments<'syntax>,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            return self.build_bound_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms(
                tail,
                continuation,
                assignments,
                suffix_terms,
                first_visible_place,
                eventuality,
                mode,
                allow_single_argument_distribution,
                excluded_source,
            );
        }
        self.build_bo_grouped_bridi_tail_formula_core_with_preassigned_shared_terms(
            tail,
            assignments,
            suffix_terms,
            first_visible_place,
            eventuality,
            mode,
            allow_single_argument_distribution,
            excluded_source,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bo_grouped_bridi_tail_formula_core_with_preassigned_shared_terms<
        'syntax: 'tree,
    >(
        &mut self,
        tail: &'syntax BoGroupedBridiTailSyntax,
        assignments: &GeneratedTermAssignments<'syntax>,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let SimpleBridiTailSyntax::SelbriSimpleBridiTail(simple_tail) = tail.first.as_ref() else {
            return Err(invalid_graph(
                "forethought bridi tail reached preassigned selbri-tail lowering".to_owned(),
            ));
        };
        let mut terms = Vec::with_capacity(simple_tail.terms.len() + suffix_terms.len());
        terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend_from_slice(suffix_terms);
        let shared_tail_start = (!suffix_terms.is_empty()).then_some(simple_tail.terms.len());
        let formula = self
            .build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct_excluding_source(
                &simple_tail.selbri,
                simple_tail,
                &assignments.visible_arguments,
                &assignments.place_questions,
                terms,
                shared_tail_start,
                first_visible_place.max(assignments.next_visible_place),
                eventuality,
                mode,
                allow_single_argument_distribution,
                "bridi-tail-formula",
                excluded_source,
            )?;
        self.attach_generated_modal_terms_to_formula(formula, &assignments.modal_terms)?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bound_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms<
        'syntax: 'tree,
    >(
        &mut self,
        leading_tail: &'syntax BoGroupedBridiTailSyntax,
        continuation: &'syntax jbotci_syntax::generated_model::BridiTailBoContinuationSyntax,
        assignments: &GeneratedTermAssignments<'syntax>,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        leading_eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        excluded_source: Option<&SourceByteSpan>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let mut branch_suffix_terms =
            Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
        branch_suffix_terms.extend(
            continuation
                .tail_terms
                .iter()
                .map(GeneratedBridiTermRef::Term),
        );
        branch_suffix_terms.extend_from_slice(suffix_terms);
        let first = self.build_bo_grouped_bridi_tail_formula_core_with_preassigned_shared_terms(
            leading_tail,
            assignments,
            &branch_suffix_terms,
            first_visible_place,
            leading_eventuality,
            mode,
            allow_single_argument_distribution,
            excluded_source,
        )?;
        let second = self.build_bo_grouped_bridi_tail_formula_with_preassigned_shared_terms(
            &continuation.bridi_tail,
            assignments,
            &branch_suffix_terms,
            first_visible_place,
            None,
            mode,
            allow_single_argument_distribution,
            excluded_source,
        )?;
        let source = continuation.tense_modal.as_deref().and_then(|tense_modal| {
            self.source_for_node(tense_modal, "bridi-tail-connection-formula")
        });
        self.build_binary_generated_bridi_tail_connection_formula(
            first,
            second,
            &continuation.connective,
            continuation.tense_modal.as_deref(),
            source,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|assignments| assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_generated_shared_head_assignments<'syntax: 'tree>(
        &mut self,
        terms: &[GeneratedBridiTermRef<'syntax>],
        annotate_shared_head_source: bool,
    ) -> Result<GeneratedTermAssignments<'syntax>, SemanticsError> {
        let mut assignments = empty_generated_term_assignments();
        for term in terms {
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
                *term,
            )?;
            assignments.implicit_existentials.extend(
                self.implicit_existential_variables
                    .split_off(existential_start),
            );
            if annotate_shared_head_source && generated_shared_head_term_uses_shared_source(*term) {
                let source = self.source_for_bridi_term(*term, "shared-head-term");
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

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.formula == formula)]
    pub(super) fn generated_scoped_formula_without_scopes<'syntax>(
        &self,
        formula: SemanticObjectId,
    ) -> GeneratedScopedFormula<'syntax> {
        GeneratedScopedFormula {
            formula,
            formula_scopes: Vec::new(),
            coequal_scope_groups: Vec::new(),
            implicit_existentials: Vec::new(),
            term_formula_scopes: Vec::new(),
        }
    }

    #[requires(scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn wrap_generated_scoped_formula(
        &mut self,
        scoped: GeneratedScopedFormula<'tree>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        self.wrap_formula_with_generated_assignment_scopes(
            scoped.formula,
            scoped.formula_scopes,
            scoped.coequal_scope_groups,
            scoped.implicit_existentials,
            scoped.term_formula_scopes,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn with_generated_scoped_formula<'syntax>(
        &self,
        scoped: GeneratedScopedFormula<'syntax>,
        formula: SemanticObjectId,
    ) -> GeneratedScopedFormula<'syntax> {
        GeneratedScopedFormula { formula, ..scoped }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn append_generated_scoped_formula_scopes<'syntax>(
        &self,
        target: GeneratedScopedFormula<'syntax>,
        source: GeneratedScopedFormula<'syntax>,
    ) -> GeneratedScopedFormula<'syntax> {
        let mut target = target;
        target.formula_scopes.extend(source.formula_scopes);
        target
            .coequal_scope_groups
            .extend(source.coequal_scope_groups);
        target
            .implicit_existentials
            .extend(source.implicit_existentials);
        target
            .term_formula_scopes
            .extend(source.term_formula_scopes);
        target
    }

    #[requires(true)]
    #[requires(assignments.visible_arguments.keys().all(|place| *place > 0))]
    #[ensures(true)]
    pub(super) fn append_generated_term_assignment_scopes<'syntax>(
        &self,
        target: GeneratedScopedFormula<'syntax>,
        assignments: GeneratedTermAssignments<'syntax>,
    ) -> GeneratedScopedFormula<'syntax> {
        let mut target = target;
        target.formula_scopes.extend(assignments.formula_scopes);
        target
            .coequal_scope_groups
            .extend(assignments.coequal_scope_groups);
        target
            .implicit_existentials
            .extend(assignments.implicit_existentials);
        target
            .term_formula_scopes
            .extend(assignments.term_formula_scopes);
        target
    }

    #[requires(formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(arguments.iter().all(|argument| argument.value.is_some()))]
    #[ensures(true)]
    pub(super) fn apply_generated_shared_head_arguments(
        &mut self,
        formula: SemanticObjectId,
        arguments: &[ArgumentValue],
    ) -> Result<(), SemanticsError> {
        for argument in arguments {
            self.fill_first_elided_generated_formula_argument_with_argument(formula, argument)?;
        }
        Ok(())
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|scoped| scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bo_grouped_bridi_tail_formula_with_shared_terms<'syntax: 'tree>(
        &mut self,
        tail: &'syntax BoGroupedBridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        annotate_shared_head_source: bool,
    ) -> Result<GeneratedScopedFormula<'syntax>, SemanticsError> {
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            return self.build_bound_bo_grouped_bridi_tail_formula_with_shared_terms(
                tail,
                continuation,
                prefix_terms,
                suffix_terms,
                first_visible_place,
                mode,
                allow_single_argument_distribution,
                annotate_shared_head_source,
            );
        }
        self.build_bo_grouped_bridi_tail_formula_core_with_shared_terms(
            tail,
            prefix_terms,
            suffix_terms,
            first_visible_place,
            mode,
            allow_single_argument_distribution,
            annotate_shared_head_source,
        )
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|scoped| scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bo_grouped_bridi_tail_formula_core_with_shared_terms<'syntax: 'tree>(
        &mut self,
        tail: &'syntax BoGroupedBridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        annotate_shared_head_source: bool,
    ) -> Result<GeneratedScopedFormula<'syntax>, SemanticsError> {
        match tail.first.as_ref() {
            SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(forethought) => {
                let formula = self.build_forethought_bridi_connection_formula_with_shared_terms(
                    &forethought.0,
                    prefix_terms,
                    suffix_terms,
                    None,
                    mode,
                )?;
                Ok(self.generated_scoped_formula_without_scopes(formula))
            }
            SimpleBridiTailSyntax::SelbriSimpleBridiTail(simple_tail) => {
                let mut terms = Vec::with_capacity(simple_tail.terms.len() + suffix_terms.len());
                terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
                terms.extend_from_slice(suffix_terms);
                let shared_tail_start =
                    (!suffix_terms.is_empty()).then_some(simple_tail.terms.len());
                let eventuality = None;
                if terms
                    .iter()
                    .any(|term| generated_forethought_termset_in_term(*term).is_some())
                {
                    let shared_head_assignments = if prefix_terms.is_empty() {
                        empty_generated_term_assignments()
                    } else {
                        self.build_generated_shared_head_assignments(
                            prefix_terms,
                            annotate_shared_head_source,
                        )?
                    };
                    let local_first_visible_place =
                        first_visible_place.max(shared_head_assignments.next_visible_place);
                    if let Some(formula) = self
                        .build_generated_forethought_termset_connection_formula(
                            &simple_tail.selbri,
                            simple_tail,
                            &terms,
                            &shared_head_assignments.visible_arguments,
                            &shared_head_assignments.place_questions,
                            local_first_visible_place,
                            eventuality,
                            mode,
                        )?
                    {
                        self.attach_generated_modal_terms_to_formula(
                            formula,
                            &shared_head_assignments.modal_terms,
                        )?;
                        return Ok(GeneratedScopedFormula {
                            formula,
                            formula_scopes: shared_head_assignments.formula_scopes,
                            coequal_scope_groups: shared_head_assignments.coequal_scope_groups,
                            implicit_existentials: shared_head_assignments.implicit_existentials,
                            term_formula_scopes: shared_head_assignments.term_formula_scopes,
                        });
                    }
                }
                if let Some(scoped) = self.build_direct_relation_scoped_formula_from_terms(
                    &simple_tail.selbri,
                    &simple_tail.selbri,
                    prefix_terms,
                    annotate_shared_head_source,
                    terms.clone(),
                    shared_tail_start,
                    first_visible_place,
                    eventuality,
                    mode,
                    allow_single_argument_distribution,
                    "bridi-tail-formula",
                )? {
                    return Ok(scoped);
                }
                if prefix_terms.is_empty() {
                    return self.build_selbri_simple_bridi_tail_scoped_formula_from_terms(
                        &simple_tail.selbri,
                        simple_tail,
                        terms,
                        first_visible_place,
                        eventuality,
                        mode,
                        allow_single_argument_distribution,
                        "bridi-tail-formula",
                    );
                }
                let shared_head_assignments = self.build_generated_shared_head_assignments(
                    prefix_terms,
                    annotate_shared_head_source,
                )?;
                let preassigned_visible_arguments =
                    shared_head_assignments.visible_arguments.clone();
                let local_first_visible_place =
                    first_visible_place.max(shared_head_assignments.next_visible_place);
                let formula = self
                    .build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
                        &simple_tail.selbri,
                        simple_tail,
                        &preassigned_visible_arguments,
                        &shared_head_assignments.place_questions,
                        terms,
                        shared_tail_start,
                        local_first_visible_place,
                        eventuality,
                        mode,
                        allow_single_argument_distribution,
                        "bridi-tail-formula",
                    )?;
                self.attach_generated_modal_terms_to_formula(
                    formula,
                    &shared_head_assignments.modal_terms,
                )?;
                let formula = self.wrap_formula_with_generated_assignment_scopes(
                    formula,
                    shared_head_assignments.formula_scopes,
                    shared_head_assignments.coequal_scope_groups,
                    shared_head_assignments.implicit_existentials,
                    shared_head_assignments.term_formula_scopes,
                )?;
                Ok(self.generated_scoped_formula_without_scopes(formula))
            }
        }
    }

    #[requires(first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|scoped| scoped.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_bound_bo_grouped_bridi_tail_formula_with_shared_terms<'syntax: 'tree>(
        &mut self,
        leading_tail: &'syntax BoGroupedBridiTailSyntax,
        continuation: &'syntax jbotci_syntax::generated_model::BridiTailBoContinuationSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        mode: PredicationMode,
        allow_single_argument_distribution: bool,
        annotate_shared_head_source: bool,
    ) -> Result<GeneratedScopedFormula<'syntax>, SemanticsError> {
        let mut branch_suffix_terms =
            Vec::with_capacity(continuation.tail_terms.len() + suffix_terms.len());
        branch_suffix_terms.extend(
            continuation
                .tail_terms
                .iter()
                .map(GeneratedBridiTermRef::Term),
        );
        branch_suffix_terms.extend_from_slice(suffix_terms);
        let mut first = self.build_bo_grouped_bridi_tail_formula_core_with_shared_terms(
            leading_tail,
            prefix_terms,
            &branch_suffix_terms,
            first_visible_place,
            mode,
            allow_single_argument_distribution,
            annotate_shared_head_source,
        )?;
        let second = self.build_bo_grouped_bridi_tail_formula_with_shared_terms(
            &continuation.bridi_tail,
            prefix_terms,
            &branch_suffix_terms,
            first_visible_place,
            mode,
            allow_single_argument_distribution,
            annotate_shared_head_source,
        )?;
        let source = continuation.tense_modal.as_deref().and_then(|tense_modal| {
            self.source_for_node(tense_modal, "bridi-tail-connection-formula")
        });
        let formula = self.build_binary_generated_bridi_tail_connection_formula(
            first.formula,
            second.formula,
            &continuation.connective,
            continuation.tense_modal.as_deref(),
            source,
        )?;
        first = self.with_generated_scoped_formula(first, formula);
        first = self.append_generated_scoped_formula_scopes(first, second);
        Ok(first)
    }

    #[requires(previous_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[requires(next_formula.object_kind() == crate::model::SemanticObjectKind::Formula)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_binary_generated_bridi_tail_connection_formula(
        &mut self,
        previous_formula: SemanticObjectId,
        next_formula: SemanticObjectId,
        connective: &BridiTailConnectiveSyntax,
        tense_modal: Option<&TenseModalSyntax>,
        source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let left_formula = if generated_bridi_tail_connective_negates_left(connective) {
            self.build_unary_formula(FormulaOperator::Not, previous_formula, source.clone())?
        } else {
            previous_formula
        };
        let right_formula = if generated_bridi_tail_connective_negates_right(connective) {
            self.build_unary_formula(FormulaOperator::Not, next_formula, source.clone())?
        } else {
            next_formula
        };
        self.mark_generated_bridi_tail_whether_or_not_inert_operand(
            connective,
            previous_formula,
            next_formula,
        );
        let connector_parameter = match generated_bridi_tail_connective_question_token(connective) {
            Some(token) => self.build_generated_connective_question_parameter_for_token(&token)?,
            None => None,
        };
        let base_operator = generated_bridi_tail_connective_formula_operator(connective);
        let operator = if connector_parameter.is_some() {
            FormulaOperator::ConnectiveQuestion
        } else {
            base_operator
        };
        let mut children = if generated_bridi_tail_connective_has_se(connective)
            && base_operator != FormulaOperator::WhetherOrNot
        {
            vec![right_formula, left_formula]
        } else {
            vec![left_formula, right_formula]
        };
        let mut diagnostics = Vec::new();
        if let Some(spec) =
            generated_modal_statement_connection_spec_for_optional_tense_modal(tense_modal)
        {
            let claim_source = tense_modal.and_then(|tense_modal| {
                self.source_for_node(tense_modal, "bridi-tail-connection-claim")
            });
            match self.build_generated_modal_formula_connection_claim(
                next_formula,
                previous_formula,
                &spec,
                claim_source,
            )? {
                Some(claim) => children.push(claim),
                None => diagnostics.push(diagnostic(
                    "modal bridi-tail connection could not find formula-bearing bridi events to relate",
                )),
            }
        }
        let connector_source =
            generated_bridi_tail_connective_source_with_tense_modal(connective, tense_modal)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(connector_source),
                    locus: ConnectorLocus::PredicatePhrase,
                    truth_table: connector_parameter
                        .is_none()
                        .then(|| generated_bridi_tail_connective_truth_table(connective))
                        .flatten(),
                    parameter: connector_parameter,
                })),
                source,
                diagnostics,
            ),
        )?;
        Ok(formula)
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|(id, context)| id.object_kind() == crate::model::SemanticObjectKind::Formula && context.assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_forethought_subbridi_branch_formula_with_deferred_prefix<'syntax: 'tree>(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        eventuality: Option<SemanticObjectId>,
    ) -> Result<(SemanticObjectId, GeneratedForethoughtPrefixContext<'syntax>), SemanticsError>
    {
        if prefix_terms.is_empty() {
            let formula = self.build_forethought_subbridi_branch_formula(
                subbridi,
                &BTreeMap::new(),
                first_visible_place,
                suffix_terms,
                eventuality,
                &[],
            )?;
            return Ok((
                formula,
                GeneratedForethoughtPrefixContext {
                    assignments: empty_generated_term_assignments(),
                    adjuncts: Vec::new(),
                },
            ));
        }
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => self
                .build_forethought_bridi_branch_formula_with_deferred_prefix(
                    bridi,
                    prefix_terms,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                ),
            SubbridiSyntax::PrenexSubbridi(prenex) => {
                let prenex_context =
                    self.push_generated_prenex_term_bindings(&prenex.prenex_terms)?;
                let result = self.build_forethought_subbridi_branch_formula_with_deferred_prefix(
                    &prenex.inner_subbridi,
                    prefix_terms,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                );
                let data!(GeneratedPrenexContext {
                    pushed_bindings,
                    topics,
                }) = prenex_context.into_data();
                self.pop_generated_prenex_scope_bindings(pushed_bindings);
                let (formula, context) = result?;
                let formula = self.wrap_formula_with_generated_prenex_terms(
                    formula,
                    &prenex.prenex_terms,
                    topics,
                    self.source_for_node(prenex, "topic-comment"),
                )?;
                Ok((formula, context))
            }
        }
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|(id, context)| id.object_kind() == crate::model::SemanticObjectKind::Formula && context.assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_forethought_bridi_branch_formula_with_deferred_prefix<'syntax: 'tree>(
        &mut self,
        bridi: &'tree BridiSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        eventuality: Option<SemanticObjectId>,
    ) -> Result<(SemanticObjectId, GeneratedForethoughtPrefixContext<'syntax>), SemanticsError>
    {
        match bridi {
            BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(tail)) => self
                .build_forethought_bridi_branch_tail_formula_with_deferred_prefix(
                    bridi,
                    &[],
                    tail,
                    prefix_terms,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    false,
                ),
            BridiSyntax::BridiWithLeadingTerms(bridi_with_terms) => self
                .build_forethought_bridi_branch_tail_formula_with_deferred_prefix(
                    bridi,
                    bridi_with_terms.leading_terms.as_slice(),
                    &bridi_with_terms.bridi_tail,
                    prefix_terms,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    true,
                ),
            BridiSyntax::BareCuBridi(bridi) => self
                .build_forethought_bridi_branch_tail_formula_with_deferred_prefix(
                    bridi,
                    &[],
                    &bridi.bridi_tail,
                    prefix_terms,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    false,
                ),
            BridiSyntax::BridiWithPostCuTerms(_) | BridiSyntax::BareCuTermsBridi(_) => {
                Err(undefined_semantics(
                    "experimental Zantufa post-CU terms inside a forethought bridi branch",
                ))
            }
        }
    }

    #[requires(first_visible_place > 0)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|(id, context)| id.object_kind() == crate::model::SemanticObjectKind::Formula && context.assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_forethought_bridi_branch_tail_formula_with_deferred_prefix<
        'syntax: 'tree,
        N: TreeNode,
    >(
        &mut self,
        source_node: &N,
        branch_leading_terms: &'tree [TermSyntax],
        tail: &'tree BridiTailSyntax,
        prefix_terms: &[GeneratedBridiTermRef<'syntax>],
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'syntax>],
        eventuality: Option<SemanticObjectId>,
        allow_single_argument_distribution: bool,
    ) -> Result<(SemanticObjectId, GeneratedForethoughtPrefixContext<'syntax>), SemanticsError>
    {
        let branch_first_visible_place =
            if prefix_terms.is_empty() && branch_leading_terms.is_empty() {
                2
            } else {
                first_visible_place
            };
        if generated_bridi_tail_is_connected(tail) {
            let prefix_assignments = if prefix_terms.is_empty() {
                empty_generated_term_assignments()
            } else {
                self.build_term_assignments_for_terms(prefix_terms.to_vec(), 1)?
            };
            if !prefix_terms.is_empty() {
                let adjuncts = self
                    .build_adjuncts_for_generated_tagged_terms(&prefix_assignments.modal_terms)?;
                let formula = self
                    .build_connected_bridi_tail_formula_with_preassigned_shared_terms(
                        source_node,
                        tail,
                        &prefix_assignments,
                        suffix_terms,
                        branch_first_visible_place,
                        eventuality,
                        PredicationMode::Asserted,
                        allow_single_argument_distribution,
                        None,
                    )?;
                return Ok((
                    formula,
                    GeneratedForethoughtPrefixContext {
                        assignments: prefix_assignments,
                        adjuncts,
                    },
                ));
            }
            let formula = self.build_connected_bridi_tail_formula_with_shared_terms(
                source_node,
                tail,
                prefix_terms,
                suffix_terms,
                branch_first_visible_place,
                None,
                PredicationMode::Asserted,
                allow_single_argument_distribution,
                false,
                true,
            )?;
            return Ok((
                formula,
                GeneratedForethoughtPrefixContext {
                    assignments: prefix_assignments,
                    adjuncts: Vec::new(),
                },
            ));
        }
        if forethought_connection_from_bridi_tail(tail)?.is_some() {
            return Err(invalid_graph(
                "nested forethought bridi reached simple shared-term branch lowering".to_owned(),
            ));
        }
        let simple_tail = simple_tail_from_bridi_tail(tail)?;
        let mut terms = Vec::with_capacity(
            branch_leading_terms.len() + simple_tail.terms.len() + suffix_terms.len(),
        );
        terms.extend(branch_leading_terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend_from_slice(suffix_terms);
        let shared_tail_start = (!suffix_terms.is_empty())
            .then_some(branch_leading_terms.len() + simple_tail.terms.len());
        self.build_selbri_simple_bridi_tail_formula_with_deferred_prefix_assignments(
            source_node,
            simple_tail,
            prefix_terms,
            terms,
            shared_tail_start,
            branch_first_visible_place,
            eventuality,
            PredicationMode::Asserted,
            allow_single_argument_distribution,
        )
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_forethought_subbridi_branch_formula(
        &mut self,
        subbridi: &'tree SubbridiSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        branch_prenex_existentials: &[GeneratedImplicitExistential],
    ) -> Result<SemanticObjectId, SemanticsError> {
        match subbridi {
            SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => self
                .build_forethought_bridi_branch_formula(
                    bridi,
                    preassigned_visible_arguments,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    branch_prenex_existentials,
                ),
            SubbridiSyntax::PrenexSubbridi(prenex) => {
                let context = self.push_generated_prenex_term_bindings(&prenex.prenex_terms)?;
                let result = self.build_forethought_subbridi_branch_formula(
                    &prenex.inner_subbridi,
                    preassigned_visible_arguments,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    branch_prenex_existentials,
                );
                let data!(GeneratedPrenexContext {
                    pushed_bindings,
                    topics,
                }) = context.into_data();
                self.pop_generated_prenex_scope_bindings(pushed_bindings);
                let formula = result?;
                self.wrap_formula_with_generated_prenex_terms(
                    formula,
                    &prenex.prenex_terms,
                    topics,
                    self.source_for_node(prenex, "topic-comment"),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_forethought_bridi_branch_formula(
        &mut self,
        bridi: &'tree BridiSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        branch_prenex_existentials: &[GeneratedImplicitExistential],
    ) -> Result<SemanticObjectId, SemanticsError> {
        match bridi {
            BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(tail)) => self
                .build_forethought_bridi_branch_tail_formula(
                    bridi,
                    &[],
                    tail,
                    preassigned_visible_arguments,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    false,
                    branch_prenex_existentials,
                ),
            BridiSyntax::BridiWithLeadingTerms(bridi_with_terms) => self
                .build_forethought_bridi_branch_tail_formula(
                    bridi,
                    bridi_with_terms.leading_terms.as_slice(),
                    &bridi_with_terms.bridi_tail,
                    preassigned_visible_arguments,
                    first_visible_place,
                    suffix_terms,
                    eventuality,
                    true,
                    branch_prenex_existentials,
                ),
            BridiSyntax::BareCuBridi(bridi) => self.build_forethought_bridi_branch_tail_formula(
                bridi,
                &[],
                &bridi.bridi_tail,
                preassigned_visible_arguments,
                first_visible_place,
                suffix_terms,
                eventuality,
                false,
                branch_prenex_existentials,
            ),
            BridiSyntax::BridiWithPostCuTerms(_) | BridiSyntax::BareCuTermsBridi(_) => {
                Err(undefined_semantics(
                    "experimental Zantufa post-CU terms inside a forethought bridi branch",
                ))
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_forethought_bridi_branch_tail_formula<N: TreeNode>(
        &mut self,
        source_node: &N,
        branch_leading_terms: &'tree [TermSyntax],
        tail: &'tree BridiTailSyntax,
        preassigned_visible_arguments: &BTreeMap<usize, ArgumentValue>,
        first_visible_place: usize,
        suffix_terms: &[GeneratedBridiTermRef<'tree>],
        eventuality: Option<SemanticObjectId>,
        allow_single_argument_distribution: bool,
        branch_prenex_existentials: &[GeneratedImplicitExistential],
    ) -> Result<SemanticObjectId, SemanticsError> {
        let branch_first_visible_place =
            if preassigned_visible_arguments.is_empty() && branch_leading_terms.is_empty() {
                2
            } else {
                first_visible_place
            };
        if generated_bridi_tail_is_connected(tail) {
            if !preassigned_visible_arguments.is_empty() {
                if !branch_leading_terms.is_empty() {
                    return Err(invalid_graph(
                        "connected forethought branch with local terms reached shared-only lowering"
                            .to_owned(),
                    ));
                }
                let mut assignments = empty_generated_term_assignments();
                assignments.visible_arguments = preassigned_visible_arguments.clone();
                assignments.next_visible_place = first_visible_place;
                let formula = self
                    .build_connected_bridi_tail_formula_with_preassigned_shared_terms(
                        source_node,
                        tail,
                        &assignments,
                        suffix_terms,
                        branch_first_visible_place,
                        eventuality,
                        PredicationMode::Asserted,
                        allow_single_argument_distribution,
                        None,
                    )?;
                return self.wrap_formula_with_implicit_existentials(
                    formula,
                    branch_prenex_existentials.to_vec(),
                );
            }
            let leading_terms = branch_leading_terms
                .iter()
                .map(GeneratedBridiTermRef::Term)
                .collect::<Vec<_>>();
            let formula = self.build_connected_bridi_tail_formula_with_shared_terms(
                source_node,
                tail,
                &leading_terms,
                suffix_terms,
                branch_first_visible_place,
                None,
                PredicationMode::Asserted,
                allow_single_argument_distribution,
                false,
                true,
            )?;
            return self.wrap_formula_with_implicit_existentials(
                formula,
                branch_prenex_existentials.to_vec(),
            );
        }
        if let Some(connection) = forethought_connection_from_bridi_tail(tail)? {
            if !branch_leading_terms.is_empty() || !preassigned_visible_arguments.is_empty() {
                return Err(invalid_graph(
                    "nested forethought bridi reached simple shared-term branch lowering"
                        .to_owned(),
                ));
            }
            return self.build_forethought_bridi_connection_formula_with_shared_terms(
                connection,
                &[],
                suffix_terms,
                eventuality,
                PredicationMode::Asserted,
            );
        }
        let simple_tail = simple_tail_from_bridi_tail(tail)?;
        let mut terms = Vec::with_capacity(
            branch_leading_terms.len() + simple_tail.terms.len() + suffix_terms.len(),
        );
        terms.extend(branch_leading_terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend(simple_tail.terms.iter().map(GeneratedBridiTermRef::Term));
        terms.extend_from_slice(suffix_terms);
        let shared_tail_start = (!suffix_terms.is_empty())
            .then_some(branch_leading_terms.len() + simple_tail.terms.len());
        let formula = self
            .build_selbri_simple_bridi_tail_formula_with_preassigned_arguments_and_formula_construct(
            source_node,
            simple_tail,
            preassigned_visible_arguments,
            &[],
            terms,
            shared_tail_start,
            branch_first_visible_place,
            eventuality,
            PredicationMode::Asserted,
            allow_single_argument_distribution,
            "bridi-formula",
        )?;
        self.wrap_formula_with_implicit_existentials(formula, branch_prenex_existentials.to_vec())
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(true)]
    pub(super) fn build_simple_tail_formula_with_options(
        &mut self,
        simple_tail: &'tree SelbriSimpleBridiTailSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
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
    pub(super) fn build_selbri_formula_with_options(
        &mut self,
        selbri: &'tree SelbriSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        match selbri {
            SelbriSyntax::ReinterpretZantufaAssignedSelbri(assigned) => self
                .build_co_selbri_formula_with_options(
                    &assigned.0.leading_selbri,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    formula_scope_child,
                    predication_source,
                    formula_source,
                ),
            SelbriSyntax::ZantufaRelativeSelbri(relative) => self
                .build_co_selbri_formula_with_options(
                    &relative.leading_selbri,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    formula_scope_child,
                    predication_source,
                    formula_source,
                ),
            SelbriSyntax::ZantufaPriorityAssignedSelbri(assigned) => self
                .build_co_selbri_formula_with_options(
                    &assigned.0.leading_selbri,
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    formula_scope_child,
                    predication_source,
                    formula_source,
                ),
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

    #[requires(first_visible_place > 0)]
    #[requires(spec.branches.len() >= 2)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_connected_event_tense_formula_for_tagged_selbri(
        &mut self,
        tagged: &'tree jbotci_syntax::generated_model::TaggedSelbriSyntax,
        spec: GeneratedConnectedEventTenseSpec,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let structurally_lowered = match tagged.inner_selbri.as_ref() {
            UntaggedSelbriSyntax::CoSelbri(co_selbri) => {
                relation_label_from_co_selbri(co_selbri).is_err()
            }
            UntaggedSelbriSyntax::NegatedSelbri(_) => true,
        };
        if structurally_lowered {
            return self.build_generated_connected_event_tense_formula_for_structural_selbri(
                tagged,
                spec,
                terms,
                first_visible_place,
                eventuality,
                mode,
                predication_source,
                formula_source,
            );
        }
        let data!(GeneratedConnectedEventTenseSpec {
            operator,
            source,
            truth_table,
            connector_question,
            branches,
        }) = spec.into_data();
        let UntaggedSelbriSyntax::CoSelbri(co_selbri) = tagged.inner_selbri.as_ref() else {
            return Err(invalid_graph(
                "connected event tense reached atomic-relation lowering for a compound selbri"
                    .to_owned(),
            ));
        };
        let relation = semantic_relation_label(relation_label_from_co_selbri(co_selbri)?);
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
        let explicit_template = self.take_deferred_generated_eventuality_template(eventuality)?;
        let template = match explicit_template {
            Some(template) => template,
            None => {
                let mut template =
                    SemanticObject::generated_eventuality(EventualityClass::Event, None, None);
                self.apply_generated_inherited_sticky_paths_to_event(&mut template);
                template
            }
        };
        let first_eventuality = self.build_generated_branch_eventuality_from_template(
            Some(&template),
            predication_source.clone(),
        )?;
        let assignments = self.with_temporal_context(first_eventuality, |builder| {
            builder.build_term_assignments_for_terms(terms.clone(), first_visible_place)
        })?;
        let place_question_assignments = assignments.place_questions.clone();
        let mut explicit_arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if explicit_arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated connected-tense arguments map to {key}"
                )));
            }
        }
        let highest_argument = explicit_arguments
            .keys()
            .map(|place| place.get())
            .max()
            .unwrap_or(0);
        let place_questions = self.build_generated_place_question_bindings(
            &place_question_assignments,
            &explicit_arguments,
            place_count,
            highest_argument,
        )?;
        let modal_terms = assignments.modal_terms;
        let formula_scopes = assignments.formula_scopes;
        let coequal_scope_groups = assignments.coequal_scope_groups;
        let implicit_existentials = assignments.implicit_existentials;
        let term_formula_scopes = assignments.term_formula_scopes;
        let mut children = Vec::with_capacity(branches.len());
        for (branch_index, branch) in branches.iter().enumerate() {
            let branch_eventuality = if branch_index == 0 {
                first_eventuality
            } else {
                self.build_generated_branch_eventuality_from_template(
                    Some(&template),
                    predication_source.clone(),
                )?
            };
            self.apply_generated_tagged_term_event_modifiers(branch_eventuality, &modal_terms)?;
            let adjuncts = self.build_adjuncts_for_generated_tagged_terms_for_event(
                branch_eventuality,
                &modal_terms,
            )?;
            let mut arguments = explicit_arguments.clone();
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
                relation.display_text(),
                Some(branch_eventuality),
                arguments,
                predication_mode_for_relation(&relation, mode),
                predication_source.clone(),
                diagnostics.clone(),
            );
            predication_object.set_predication_attachments(adjuncts, place_questions.clone());
            self.insert(predication, predication_object)?;
            self.attach_generated_reciprocity_to_predication_for_terms(predication, &terms)?;
            self.record_generated_tense_modal_event_modifier(
                branch_eventuality,
                &branch.tense_modal,
                None,
            )?;
            self.flush_generated_event_modifiers(branch_eventuality)?;
            let atom = self.next_formula_id();
            self.insert(
                atom,
                SemanticObject::atom_formula(predication, formula_source.clone(), Vec::new()),
            )?;
            let branch_formula = if branch.negated {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    atom,
                    self.source_for_node(&branch.tense_modal, "tense-negation"),
                )?
            } else {
                atom
            };
            children.push(branch_formula);
        }
        let connector_parameter = connector_question
            .as_ref()
            .map(|token| self.build_generated_connective_question_parameter_for_token(token))
            .transpose()?
            .flatten();
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(source),
                    locus: ConnectorLocus::Tense,
                    truth_table,
                    parameter: connector_parameter,
                })),
                formula_source,
                Vec::new(),
            ),
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            formula_scopes,
            coequal_scope_groups,
            implicit_existentials,
            term_formula_scopes,
        )
    }

    #[requires(first_visible_place > 0)]
    #[requires(spec.branches.len() >= 2)]
    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_generated_connected_event_tense_formula_for_structural_selbri(
        &mut self,
        tagged: &'tree jbotci_syntax::generated_model::TaggedSelbriSyntax,
        spec: GeneratedConnectedEventTenseSpec,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        let data!(GeneratedConnectedEventTenseSpec {
            operator,
            source,
            truth_table,
            connector_question,
            branches,
        }) = spec.into_data();
        let explicit_template = self.take_deferred_generated_eventuality_template(eventuality)?;
        let template = match explicit_template {
            Some(template) => template,
            None => {
                let mut template =
                    SemanticObject::generated_eventuality(EventualityClass::Event, None, None);
                self.apply_generated_inherited_sticky_paths_to_event(&mut template);
                template
            }
        };
        let first_eventuality = self.build_generated_branch_eventuality_from_template(
            Some(&template),
            predication_source.clone(),
        )?;
        let assignments = self.with_temporal_context(first_eventuality, |builder| {
            builder.build_term_assignments_for_terms(terms.clone(), first_visible_place)
        })?;
        let explicit_arguments = assignments
            .visible_arguments
            .iter()
            .map(|(place, argument)| (argument_key(*place), argument.clone()))
            .collect::<BTreeMap<_, _>>();
        let highest_argument = explicit_arguments
            .keys()
            .map(|place| place.get())
            .max()
            .unwrap_or(0);
        let place_questions = self.build_generated_place_question_bindings(
            &assignments.place_questions,
            &explicit_arguments,
            None,
            highest_argument,
        )?;
        let mut children = Vec::with_capacity(branches.len());
        for (branch_index, branch) in branches.iter().enumerate() {
            let branch_eventuality = if branch_index == 0 {
                first_eventuality
            } else {
                self.build_generated_branch_eventuality_from_template(
                    Some(&template),
                    predication_source.clone(),
                )?
            };
            let result = self.build_untagged_selbri_formula_for_visible_arguments(
                tagged.inner_selbri.as_ref(),
                assignments.visible_arguments.clone(),
                formula_source.clone(),
                ConnectorLocus::Tense,
                Some(branch_eventuality),
            )?;
            self.attach_generated_modal_terms_to_formula(result.formula, &assignments.modal_terms)?;
            if !place_questions.is_empty()
                && let Some(predication) = self.objects.get_mut(&result.head_predication)
            {
                predication.set_predication_place_questions(place_questions.clone());
            }
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            self.attach_generated_reciprocity_to_predication_for_terms(
                result.head_predication,
                &terms,
            )?;
            self.record_generated_tense_modal_event_modifier(
                branch_eventuality,
                &branch.tense_modal,
                None,
            )?;
            self.flush_generated_event_modifiers(branch_eventuality)?;
            let branch_formula = if branch.negated {
                self.build_unary_formula(
                    FormulaOperator::Not,
                    result.formula,
                    self.source_for_node(&branch.tense_modal, "tense-negation"),
                )?
            } else {
                result.formula
            };
            children.push(branch_formula);
        }
        let connector_parameter = connector_question
            .as_ref()
            .map(|token| self.build_generated_connective_question_parameter_for_token(token))
            .transpose()?
            .flatten();
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::connective_formula(
                operator,
                children,
                Some(new!(Connector {
                    source: ConnectorSource::surface_word(source),
                    locus: ConnectorLocus::Tense,
                    truth_table,
                    parameter: connector_parameter,
                })),
                formula_source,
                Vec::new(),
            ),
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tagged_selbri_formula_with_options(
        &mut self,
        tagged: &'tree jbotci_syntax::generated_model::TaggedSelbriSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if let Some(spec) =
            generated_connected_event_tense_spec_for_tense_modal(tagged.tense_modal.as_ref())
        {
            return self.build_generated_connected_event_tense_formula_for_tagged_selbri(
                tagged,
                spec,
                terms,
                first_visible_place,
                eventuality,
                mode,
                predication_source,
                formula_source,
            );
        }
        if generated_tense_modal_resets_sticky_modals(tagged.tense_modal.as_ref()) {
            self.sticky_adjuncts.clear();
        }
        if generated_tense_modal_resets_sticky_tense(tagged.tense_modal.as_ref()) {
            self.sticky_time_path.clear();
            self.sticky_space_path.clear();
        }
        if generated_untagged_selbri_has_formula_scope(tagged.inner_selbri.as_ref()) {
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
            let scope_source = self.source_for_node(tagged, "tense-scope");
            if let Some(eventuality) = eventuality {
                if generated_tense_modal_has_event_modifier(tagged.tense_modal.as_ref()) {
                    self.apply_generated_tense_modal_event_modifier_to_eventuality(
                        eventuality,
                        tagged.tense_modal.as_ref(),
                        None,
                    )?;
                }
                let formula = self.next_formula_id();
                let mut object = SemanticObject::connective_formula(
                    FormulaOperator::Scoped,
                    vec![child],
                    None,
                    scope_source,
                    Vec::new(),
                );
                object.set_scoped_formula_eventuality(Some(eventuality));
                self.insert(formula, object)?;
                return Ok(formula);
            }
            return self.build_generated_tense_scope_formula(
                child,
                tagged.tense_modal.as_ref(),
                scope_source,
            );
        }

        self.build_tagged_selbri_formula_around_child(
            tagged,
            eventuality,
            predication_source.clone(),
            |builder, child_eventuality| {
                builder.build_untagged_selbri_formula_with_options(
                    tagged.inner_selbri.as_ref(),
                    terms,
                    first_visible_place,
                    child_eventuality,
                    mode,
                    formula_scope_child,
                    predication_source,
                    formula_source,
                )
            },
        )
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_tagged_selbri_formula_around_child<F>(
        &mut self,
        tagged: &'tree jbotci_syntax::generated_model::TaggedSelbriSyntax,
        eventuality: Option<SemanticObjectId>,
        predication_source: Option<crate::model::SemanticSource>,
        build_child: F,
    ) -> Result<SemanticObjectId, SemanticsError>
    where
        F: FnOnce(&mut Self, Option<SemanticObjectId>) -> Result<SemanticObjectId, SemanticsError>,
    {
        let tagged_tense_modifies_event =
            generated_tense_modal_has_event_modifier(tagged.tense_modal.as_ref())
                || generated_tense_modal_makes_tense_sticky(tagged.tense_modal.as_ref())
                || generated_tense_modal_makes_space_sticky(tagged.tense_modal.as_ref())
                || generated_tense_modal_resets_sticky_tense(tagged.tense_modal.as_ref());
        let tagged_temporal_modifier =
            generated_tense_modal_has_story_time_temporal_modifier(tagged.tense_modal.as_ref());
        let previous_story_time_anchor = self.story_time_anchor;
        let preapply_tagged_event_modifier = tagged_tense_modifies_event
            && !self.options.story_time
            && generated_tense_modal_event_modifier_allocates_objects(tagged.tense_modal.as_ref());
        let child_eventuality = if eventuality.is_none()
            && (generated_tense_modal_has_adjunct(tagged.tense_modal.as_ref())
                || preapply_tagged_event_modifier)
        {
            Some(self.build_generated_predication_eventuality(predication_source.clone())?)
        } else {
            eventuality
        };
        let tagged_adjunct = self.build_adjunct_for_generated_tense_modal(
            tagged.tense_modal.as_ref(),
            "modal-argument",
        )?;
        if let Some(eventuality) = child_eventuality
            && preapply_tagged_event_modifier
        {
            self.record_generated_tense_modal_event_modifier(
                eventuality,
                tagged.tense_modal.as_ref(),
                None,
            )?;
            if self.objects.contains_key(&eventuality) {
                self.flush_generated_event_modifiers_with_recurrence_quantity_promotion(
                    eventuality,
                )?;
            }
        }
        self.deferred_event_modifier_flush_depth += 1;
        let formula_result = build_child(self, child_eventuality);
        self.deferred_event_modifier_flush_depth -= 1;
        let formula = formula_result?;
        let mut host_eventualities =
            self.eventualities_for_generated_formula_predications(formula)?;
        if let Some(eventuality) = child_eventuality
            && !host_eventualities.contains(&eventuality)
        {
            host_eventualities.insert(0, eventuality);
        }
        for eventuality in &host_eventualities {
            if self.pending_event_modifiers.contains_key(eventuality) {
                self.flush_generated_event_modifiers_with_recurrence_quantity_promotion(
                    *eventuality,
                )?;
            }
        }
        if tagged_tense_modifies_event && !preapply_tagged_event_modifier {
            if self.options.story_time && tagged_temporal_modifier {
                self.story_time_anchor = previous_story_time_anchor;
                for eventuality in &host_eventualities {
                    if let Some(event) = self.objects.get_mut(eventuality) {
                        clear_generated_event_time_path(event);
                    }
                }
            }
            for eventuality in &host_eventualities {
                self.record_generated_tense_modal_event_modifier(
                    *eventuality,
                    tagged.tense_modal.as_ref(),
                    None,
                )?;
                if self.objects.contains_key(eventuality) {
                    self.flush_generated_event_modifiers_with_recurrence_quantity_promotion(
                        *eventuality,
                    )?;
                }
            }
            if self.options.story_time
                && tagged_temporal_modifier
                && (generated_tense_modal_makes_tense_sticky(tagged.tense_modal.as_ref())
                    || previous_story_time_anchor.is_none())
                && let Some(eventuality) = host_eventualities.first()
            {
                self.story_time_anchor = Some(*eventuality);
            }
        }
        if let Some(adjunct) = &tagged_adjunct {
            self.record_generated_sticky_adjunct_if_needed(tagged.tense_modal.as_ref(), adjunct);
        }
        if let Some(adjunct) = tagged_adjunct {
            self.prepend_adjunct_to_generated_formula(formula, &adjunct)?;
        }
        Ok(formula)
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_untagged_selbri_formula_with_options(
        &mut self,
        selbri: &'tree UntaggedSelbriSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
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
                let deferred_start = self.deferred_active_prenex_implicit_existentials.len();
                let reservation_start = self.pending_negated_selbri_argument_scope_reservations;
                self.defer_active_prenex_implicit_existentials += 1;
                self.pending_negated_selbri_argument_scope_reservations += 1;
                let child = self.build_selbri_formula_with_options(
                    negated.inner_selbri.as_ref(),
                    terms,
                    first_visible_place,
                    eventuality,
                    mode,
                    true,
                    formula_source.clone(),
                    formula_source.clone(),
                );
                self.defer_active_prenex_implicit_existentials -= 1;
                if self.pending_negated_selbri_argument_scope_reservations > reservation_start {
                    self.pending_negated_selbri_argument_scope_reservations -= 1;
                }
                let child = child?;
                let deferred_existentials = self
                    .deferred_active_prenex_implicit_existentials
                    .split_off(deferred_start);
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
                self.wrap_formula_with_implicit_existentials(formula, deferred_existentials)
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
        }
    }

    #[requires(eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|id| id.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_co_selbri_formula_with_options(
        &mut self,
        selbri: &'tree CoSelbriSyntax,
        terms: Vec<GeneratedBridiTermRef<'tree>>,
        first_visible_place: usize,
        eventuality: Option<SemanticObjectId>,
        mode: PredicationMode,
        formula_scope_child: bool,
        predication_source: Option<crate::model::SemanticSource>,
        formula_source: Option<crate::model::SemanticSource>,
    ) -> Result<SemanticObjectId, SemanticsError> {
        if selbri.co_tail.is_some() {
            let source = if formula_scope_child {
                formula_source
                    .clone()
                    .or_else(|| predication_source.clone())
            } else {
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    "tanru-inversion-formula",
                )
            };
            let (head_terms, modifier_terms, modifier_first_visible_place) =
                split_generated_co_terms(selbri, terms)?;
            let leading_eventuality = match eventuality {
                Some(eventuality) => Some(eventuality),
                None if !head_terms.is_empty() => {
                    Some(self.build_generated_predication_eventuality(source.clone())?)
                }
                None => None,
            };
            let head_assignments =
                self.build_term_assignments_for_terms(head_terms, first_visible_place)?;
            let (result, modifier_assignments) = self
                .build_co_selbri_inversion_formula_for_visible_arguments_with_modifier_terms(
                    selbri,
                    head_assignments.visible_arguments,
                    modifier_terms,
                    modifier_first_visible_place,
                    source,
                    leading_eventuality,
                )?;
            self.attach_generated_modal_terms_to_formula(
                result.formula,
                &head_assignments.modal_terms,
            )?;
            self.attach_generated_modal_terms_to_formula(
                result.formula,
                &modifier_assignments.modal_terms,
            )?;
            if mode != PredicationMode::Asserted {
                self.set_formula_predication_mode(result.formula, mode);
            }
            let mut formula_scopes = head_assignments.formula_scopes;
            formula_scopes.extend(modifier_assignments.formula_scopes);
            let mut coequal_scope_groups = head_assignments.coequal_scope_groups;
            coequal_scope_groups.extend(modifier_assignments.coequal_scope_groups);
            let mut implicit_existentials = head_assignments.implicit_existentials;
            implicit_existentials.extend(modifier_assignments.implicit_existentials);
            let mut term_formula_scopes = head_assignments.term_formula_scopes;
            term_formula_scopes.extend(modifier_assignments.term_formula_scopes);
            return self.wrap_formula_with_generated_assignment_scopes(
                result.formula,
                formula_scopes,
                coequal_scope_groups,
                implicit_existentials,
                term_formula_scopes,
            );
        }
        if let Some(tanru) = tanru_selbri_from_co_selbri(selbri)?
            && generated_single_tanru_unit_from_tanru_selbri(tanru).is_none()
        {
            let source = if formula_scope_child {
                formula_source
                    .clone()
                    .or_else(|| predication_source.clone())
            } else {
                source_with_construct(
                    formula_source
                        .clone()
                        .or_else(|| predication_source.clone()),
                    generated_tanru_formula_source_construct(tanru),
                )
            };
            return self.build_tanru_formula_for_terms_with_head_eventuality_order_and_mode(
                tanru,
                terms,
                first_visible_place,
                eventuality,
                mode,
                formula_scope_child,
                source,
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
        if let Some(relation_variable) = relation_variable_syntax_from_co_selbri(selbri)? {
            return self.build_relation_variable_formula_for_terms(
                relation_variable,
                terms,
                first_visible_place,
                eventuality,
                mode,
                source_with_construct(
                    predication_source.or(formula_source),
                    "relation-variable-formula",
                ),
            );
        }
        if let Some(unspecified_relation) = unspecified_relation_syntax_from_co_selbri(selbri)? {
            return self.build_unspecified_relation_formula_for_terms(
                unspecified_relation,
                terms,
                first_visible_place,
                eventuality,
                mode,
                source_with_construct(
                    predication_source.or(formula_source),
                    "unspecified-relation-formula",
                ),
            );
        }
        if let Some(pro_bridi) = single_relation_parameter_syntax_from_co_selbri(selbri)?
            && let Some(cmavo) = generated_relation_parameter_token(pro_bridi).cmavo()
            && generated_cmavo_is_resolvable_pro_bridi(cmavo)
        {
            let formula = self.build_resolved_generated_pro_bridi_formula_for_terms(
                cmavo,
                terms.clone(),
                first_visible_place,
                eventuality,
                mode,
                None,
                predication_source.clone(),
                formula_source.clone(),
            )?;
            if let Some(formula) = formula {
                return Ok(formula);
            }
        }
        if let Some(tanru) = tanru_selbri_from_co_selbri(selbri)?
            && let Some(unit) = generated_single_tanru_unit_from_tanru_selbri(tanru)
            && sumti_selbri_from_generated_tanru_unit(unit)?.is_none()
        {
            let (predication_source, formula_source) =
                if generated_tanru_unit_is_connected_selbri_formula(unit) {
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
                unit,
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
        let relation_text = relation.display_text();
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
        if let Some(formula) = self.build_generated_logical_sumti_connection_formula_for_terms(
            &relation_text,
            &terms,
            first_visible_place,
            place_limit,
            &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
            mode,
            predication_source.clone(),
            formula_source.clone(),
        )? {
            return Ok(formula);
        }
        if let Some(formula) = self.build_generated_logical_modal_connection_formula_for_terms(
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
            place_limit,
            &[],
            false,
            &terms,
            None,
            first_visible_place,
            &[] as &[WithFreeModifiers<Token, FreeModifierSyntax>],
            mode,
            predication_source.clone(),
        )? {
            return Ok(formula);
        }
        let eventuality = match eventuality {
            Some(eventuality) => eventuality,
            None => self.build_generated_predication_eventuality(predication_source.clone())?,
        };
        self.apply_generated_tagged_term_event_modifiers_in_terms(eventuality, &terms)?;
        let assignments = self.with_temporal_context(eventuality, |builder| {
            builder.build_term_assignments_for_terms(terms.clone(), first_visible_place)
        })?;
        let place_question_assignments = assignments.place_questions.clone();
        let visible_arguments_for_modal_terms = assignments.visible_arguments.clone();
        let mut arguments = BTreeMap::new();
        for (visible_place, argument) in assignments.visible_arguments {
            let key = argument_key(visible_place);
            if arguments.insert(key.clone(), argument).is_some() {
                return Err(invalid_graph(format!(
                    "multiple generated bridi arguments map to {key}"
                )));
            }
        }
        let highest_argument = arguments.keys().map(|place| place.get()).max().unwrap_or(0);
        let adjuncts = self
            .build_adjuncts_for_generated_tagged_terms_for_event_with_visible_arguments(
                eventuality,
                &assignments.modal_terms,
                Some(&visible_arguments_for_modal_terms),
            )?;
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
        let predication = self.next_predication_id();
        let mut predication_object = SemanticObject::predication(
            relation_text,
            Some(eventuality),
            arguments,
            predication_mode_for_relation(&relation, mode),
            predication_source,
            diagnostics,
        );
        predication_object.set_predication_attachments(adjuncts, place_questions);
        self.insert(predication, predication_object)?;
        self.attach_generated_reciprocity_to_predication_for_terms(predication, &terms)?;
        let formula = self.next_formula_id();
        self.insert(
            formula,
            SemanticObject::atom_formula(predication, formula_source, Vec::new()),
        )?;
        self.wrap_formula_with_generated_assignment_scopes(
            formula,
            assignments.formula_scopes,
            assignments.coequal_scope_groups,
            assignments.implicit_existentials,
            assignments.term_formula_scopes,
        )
    }

    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[ensures(ret.as_ref().is_ok_and(|result| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula) || ret.is_err())]
    pub(super) fn build_co_selbri_inversion_formula_for_visible_arguments<'syntax: 'tree>(
        &mut self,
        selbri: &'syntax CoSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        source: Option<crate::model::SemanticSource>,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<GeneratedTanruFormulaForArgument, SemanticsError> {
        let Some(co_tail) = &selbri.co_tail else {
            return self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
                &selbri.leading_selbri,
                visible_arguments,
                leading_eventuality,
                source,
            );
        };
        let head = self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
            &selbri.leading_selbri,
            visible_arguments,
            leading_eventuality,
            source.clone(),
        )?;
        let modifier = self
            .build_property_abstraction_for_co_selbri(&co_tail.trailing_selbri, source.clone())?;
        let relation_formula = self.build_tanru_relation_formula(
            head.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_generated_co_pair(
                &co_tail.trailing_selbri,
                &selbri.leading_selbri,
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
                    source: new!(ConnectorSource::ImplicitJuxtaposition),
                    locus: ConnectorLocus::PredicateInversion,
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

    #[requires(leading_eventuality.is_none_or(|id| id.referent_sort().is_some_and(|sort| sort.is_subsort_of(SemanticSort::eventuality()))))]
    #[requires(modifier_first_visible_place > 0)]
    #[ensures(ret.as_ref().is_ok_and(|(result, assignments)| result.formula.object_kind() == crate::model::SemanticObjectKind::Formula && assignments.visible_arguments.keys().all(|place| *place > 0)) || ret.is_err())]
    pub(super) fn build_co_selbri_inversion_formula_for_visible_arguments_with_modifier_terms<
        'syntax: 'tree,
    >(
        &mut self,
        selbri: &'tree CoSelbriSyntax,
        visible_arguments: BTreeMap<usize, ArgumentValue>,
        modifier_terms: Vec<GeneratedBridiTermRef<'syntax>>,
        modifier_first_visible_place: usize,
        source: Option<crate::model::SemanticSource>,
        leading_eventuality: Option<SemanticObjectId>,
    ) -> Result<
        (
            GeneratedTanruFormulaForArgument,
            GeneratedTermAssignments<'syntax>,
        ),
        SemanticsError,
    > {
        let Some(co_tail) = &selbri.co_tail else {
            let result = self
                .build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
                    &selbri.leading_selbri,
                    visible_arguments,
                    leading_eventuality,
                    source,
                )?;
            return Ok((result, empty_generated_term_assignments()));
        };
        let head = self.build_tanru_selbri_formula_for_visible_arguments_with_head_eventuality(
            &selbri.leading_selbri,
            visible_arguments,
            leading_eventuality,
            source.clone(),
        )?;
        let modifier_parameter = self.next_parameter_id();
        self.insert(
            modifier_parameter,
            SemanticObject::parameter(
                SemanticSort::Entity,
                ParameterRole::PropertySlot,
                "ce'u".to_owned(),
                source.clone(),
            ),
        )?;
        let modifier_eventuality = self.build_generated_predication_eventuality(source.clone())?;
        let modifier_assignments =
            self.build_term_assignments_for_terms(modifier_terms, modifier_first_visible_place)?;
        let mut modifier_visible_arguments = modifier_assignments.visible_arguments.clone();
        insert_visible_argument(
            &mut modifier_visible_arguments,
            1,
            ArgumentValue::filled(modifier_parameter, None),
        )?;
        let modifier_body = self.build_property_formula_for_co_selbri_with_visible_arguments(
            &co_tail.trailing_selbri,
            modifier_visible_arguments,
            source.clone(),
            GeneratedPropertyTanruContext::PropertyAbstraction,
            Some(modifier_eventuality),
        )?;
        let modifier = self.build_property_abstraction_output(
            modifier_body,
            vec![modifier_parameter],
            source.clone(),
        )?;
        let relation_formula = self.build_tanru_relation_formula(
            head.x1_argument.clone(),
            modifier,
            tanru_relation_name_for_generated_co_pair(
                &co_tail.trailing_selbri,
                &selbri.leading_selbri,
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
                    source: new!(ConnectorSource::ImplicitJuxtaposition),
                    locus: ConnectorLocus::PredicateInversion,
                    truth_table: None,
                    parameter: None,
                })),
                source,
                Vec::new(),
            ),
        )?;
        Ok((
            GeneratedTanruFormulaForArgument::from_data(data!(GeneratedTanruFormulaForArgument {
                formula,
                x1_argument: head.x1_argument.clone(),
                head_predication: head.head_predication,
            })),
            modifier_assignments,
        ))
    }
}
