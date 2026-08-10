use super::*;

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_selbri(
    selbri: &SelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(CoSelbriSyntax {
        leading_selbri: _,
        co_tail: _,
    })) = selbri
    else {
        return Err(invalid_graph(
            "atomic relation label requested for a tagged or connected selbri".to_owned(),
        ));
    };
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
        unreachable!("previous pattern requires a co selbri")
    };
    relation_label_from_co_selbri(co_selbri)
}

/// Extract the flat relation and converted tagged place from a `fi'o` selbri
/// that contains exactly one lexical predicate.
///
/// Grouping is transparent only while every enclosed grammar layer still has
/// one child. Any construct that contributes semantic structure of its own
/// (tanru composition, connectives, `NU`, linked arguments, negation, and so
/// on) returns `None` and therefore retains the body/component representation.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|spec| spec.as_ref().is_none_or(|spec| !spec.relation.is_empty() && spec.visible_place > 0)) || ret.is_err())]
pub(super) fn generated_simple_fiho_relation_spec(
    selbri: &SelbriSyntax,
) -> Result<Option<GeneratedSimpleFihoRelationSpec>, SemanticsError> {
    let mut inspector = GeneratedSimpleFihoRelationInspector::default();
    TreeWalkable::walk_with(selbri, &mut inspector);
    let data!(GeneratedSimpleFihoRelationInspector {
        lexical_word,
        composite,
    }) = inspector.into_data();
    let Some(word) = lexical_word.filter(|_| !composite) else {
        return Ok(None);
    };
    let token = &word.0.value;
    let relation = relation_label_from_token(token).display_text();
    let visible_place = generated_raw_place_visible_rank_for_selbri(selbri, 1)?;
    Ok(Some(new!(GeneratedSimpleFihoRelationSpec {
        relation: relation,
        visible_place: visible_place,
    })))
}

#[invariant(!*composite || lexical_word.is_none(), "composite inspection state does not retain an irrelevant lexical word")]
#[derive(Default)]
struct GeneratedSimpleFihoRelationInspector<'tree> {
    lexical_word: Option<&'tree WordTanruUnitSyntax>,
    composite: bool,
}

impl<'tree> GeneratedSimpleFihoRelationInspector<'tree> {
    #[requires(true)]
    #[ensures(self.composite)]
    fn mark_composite(&mut self) {
        *self = new!(GeneratedSimpleFihoRelationInspector {
            lexical_word: None,
            composite: true,
        });
    }

    #[requires(true)]
    #[ensures(self.lexical_word == Some(word) || self.composite)]
    fn capture_lexical_word(&mut self, word: &'tree WordTanruUnitSyntax) {
        if self.lexical_word.is_some() {
            self.mark_composite();
        } else {
            *self = new!(GeneratedSimpleFihoRelationInspector {
                lexical_word: Some(word),
                composite: false,
            });
        }
    }
}

impl<'tree> TreeWalker<'tree> for GeneratedSimpleFihoRelationInspector<'tree> {
    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_tagged_selbri(&mut self, _node: &'tree TaggedSelbriSyntax) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_negated_selbri(&mut self, _node: &'tree NegatedSelbriSyntax) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_forethought_selbri_connection(
        &mut self,
        _node: &'tree ForethoughtSelbriConnectionSyntax,
    ) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite || node.co_tail.is_none())]
    fn walk_co_selbri(&mut self, node: &'tree CoSelbriSyntax) {
        if node.co_tail.is_some() {
            self.mark_composite();
        } else {
            jbotci_syntax::generated_model::walk::co_selbri(self, node);
        }
    }

    #[requires(true)]
    #[ensures(self.composite || node.continuations.is_empty())]
    fn walk_connected_selbri(&mut self, node: &'tree ConnectedSelbriSyntax) {
        if node.continuations.is_empty() {
            jbotci_syntax::generated_model::walk::connected_selbri(self, node);
        } else {
            self.mark_composite();
        }
    }

    #[requires(true)]
    #[ensures(self.composite || node.additional_units.is_empty())]
    fn walk_tanru_selbri(&mut self, node: &'tree TanruSelbriSyntax) {
        if node.additional_units.is_empty() {
            jbotci_syntax::generated_model::walk::tanru_selbri(self, node);
        } else {
            self.mark_composite();
        }
    }

    #[requires(true)]
    #[ensures(self.composite || node.0.links.is_empty())]
    fn walk_tanru_unit(&mut self, node: &'tree TanruUnitSyntax) {
        if node.0.links.is_empty() {
            jbotci_syntax::generated_model::walk::tanru_unit(self, node);
        } else {
            self.mark_composite();
        }
    }

    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_forethought_selbri_group_tanru_unit(
        &mut self,
        _node: &'tree ForethoughtSelbriGroupTanruUnitSyntax,
    ) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_bound_tanru_unit(&mut self, _node: &'tree BoundTanruUnitSyntax) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite)]
    fn walk_assigned_pro_bridi_tanru_unit(
        &mut self,
        _node: &'tree AssignedProBridiTanruUnitSyntax,
    ) {
        self.mark_composite();
    }

    #[requires(true)]
    #[ensures(self.composite || node.linkargs.is_none())]
    fn walk_linked_tanru_unit(&mut self, node: &'tree LinkedTanruUnitSyntax) {
        if node.linkargs.is_some() {
            self.mark_composite();
        } else {
            jbotci_syntax::generated_model::walk::linked_tanru_unit(self, node);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tanru_unit_atom_base(&mut self, node: &'tree TanruUnitAtomBaseSyntax) {
        match node {
            TanruUnitAtomBaseSyntax::WordTanruUnit(word) => {
                TreeWalkable::walk_with(word, self);
            }
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(grouped) => {
                TreeWalkable::walk_with(grouped, self);
            }
            _ => self.mark_composite(),
        }
    }

    #[requires(true)]
    #[ensures(self.lexical_word == Some(node) || self.composite)]
    fn walk_word_tanru_unit(&mut self, node: &'tree WordTanruUnitSyntax) {
        self.capture_lexical_word(node);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_cmavo_is_resolvable_pro_bridi(cmavo: Cmavo) -> bool {
    matches!(cmavo, Cmavo::Gohi | Cmavo::Gohe | Cmavo::Nei | Cmavo::Noha)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_is_pro_bridi_label(relation: &str) -> bool {
    matches!(relation, "go'i" | "go'e" | "nei" | "no'a")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.as_ref().is_none_or(|relation| relation.is_displayable())) || ret.is_err())]
pub(super) fn generated_pro_bridi_target_relation_label(
    selbri: &SelbriSyntax,
) -> Result<Option<RelationLabel>, SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => {
            generated_pro_bridi_target_relation_label_for_untagged(tagged.inner_selbri.as_ref())
        }
        SelbriSyntax::UntaggedSelbri(untagged) => {
            generated_pro_bridi_target_relation_label_for_untagged(untagged)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|source| source.as_ref().is_none_or(|source| source.first_visible_place > 0)) || ret.is_err())]
pub(super) fn generated_pro_bridi_replay_source_from_bridi<'syntax>(
    bridi: &'syntax BridiSyntax,
) -> Result<Option<GeneratedProBridiReplaySource<'syntax>>, SemanticsError> {
    match bridi {
        BridiSyntax::BridiWithLeadingTerms(bridi) => {
            let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
            let terms = bridi
                .leading_terms
                .iter()
                .chain(simple_tail.terms.iter())
                .collect::<Vec<_>>();
            Ok(Some(new!(GeneratedProBridiReplaySource {
                selbri: simple_tail.selbri.as_ref(),
                terms,
                first_visible_place: 1,
            })))
        }
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
            let simple_tail = simple_tail_from_bridi_tail(bridi_tail)?;
            Ok(Some(new!(GeneratedProBridiReplaySource {
                selbri: simple_tail.selbri.as_ref(),
                terms: Vec::new(),
                first_visible_place: 2,
            })))
        }
        _ => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_pro_bridi_event_tense_from_selbri(
    selbri: &SelbriSyntax,
) -> Option<&TenseModalSyntax> {
    let SelbriSyntax::TaggedSelbri(tagged) = selbri else {
        return None;
    };
    generated_tense_modal_has_event_modifier(tagged.tense_modal.as_ref())
        .then_some(tagged.tense_modal.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.as_ref().is_none_or(|relation| relation.is_displayable())) || ret.is_err())]
pub(super) fn generated_pro_bridi_target_relation_label_for_untagged(
    selbri: &UntaggedSelbriSyntax,
) -> Result<Option<RelationLabel>, SemanticsError> {
    match selbri {
        UntaggedSelbriSyntax::CoSelbri(co_selbri) => {
            generated_pro_bridi_target_relation_label_from_co_selbri(co_selbri).map(Some)
        }
        UntaggedSelbriSyntax::NegatedSelbri(negated) => {
            generated_pro_bridi_target_relation_label(&negated.inner_selbri)
        }
        UntaggedSelbriSyntax::ForethoughtSelbriConnection(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
pub(super) fn generated_pro_bridi_target_relation_label_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    relation_label_from_co_selbri(selbri)
        .or_else(|_| tanru_label_from_co_selbri(selbri).map(RelationLabel::constructed))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    if selbri.co_tail.is_some() {
        return Err(invalid_graph(
            "atomic relation label requested for a CO selbri".to_owned(),
        ));
    }
    let ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = selbri.leading_selbri.as_ref();
    if !continuations.is_empty() {
        return Err(invalid_graph(
            "atomic relation label requested for a connected selbri".to_owned(),
        ));
    }
    let TanruSelbriSyntax {
        first_unit,
        additional_units,
    } = leading_selbri.as_ref();
    if !additional_units.is_empty() {
        return Err(invalid_graph(
            "atomic relation label requested for a tanru".to_owned(),
        ));
    }
    relation_label_from_tanru_unit(first_unit)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_question_syntax_from_co_selbri(
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
    relation_question_syntax_from_generated_tanru_unit(first_unit)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_question_syntax_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<GeneratedRelationQuestionSyntax<'_>>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    };
    relation_question_syntax_from_bo_or_linked_tanru_unit(unit.0.first.as_ref())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_question_syntax_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<Option<GeneratedRelationQuestionSyntax<'_>>, SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            relation_question_syntax_from_linked_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_question_syntax_from_linked_tanru_unit(
    unit: &LinkedTanruUnitSyntax,
) -> Result<Option<GeneratedRelationQuestionSyntax<'_>>, SemanticsError> {
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
pub(super) fn generated_relation_question_token(
    question: GeneratedRelationQuestionSyntax<'_>,
) -> &Token {
    match question {
        GeneratedRelationQuestionSyntax::ProBridi(pro_bridi) => &pro_bridi.goha.value,
        GeneratedRelationQuestionSyntax::GohaWord(goha) => {
            generated_goha_word_tanru_unit_token(goha)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_variable_syntax_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    let Some(syntax) = single_relation_parameter_syntax_from_co_selbri(selbri)? else {
        return Ok(None);
    };
    if generated_relation_parameter_token(syntax)
        .cmavo()
        .is_some_and(|cmavo| matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi))
    {
        Ok(Some(syntax))
    } else {
        Ok(None)
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn unspecified_relation_syntax_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    let Some(syntax) = single_relation_parameter_syntax_from_co_selbri(selbri)? else {
        return Ok(None);
    };
    if generated_relation_parameter_token(syntax).cmavo() == Some(Cmavo::Cohe) {
        Ok(Some(syntax))
    } else {
        Ok(None)
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_variable_syntax_from_generated_selbri(
    selbri: &SelbriSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
        return Ok(None);
    };
    relation_variable_syntax_from_co_selbri(co_selbri)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn relation_variable_syntax_from_no_gadri_prenex_sumti(
    sumti: &SumtiSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    let Some(description) = no_gadri_description_from_sumti(sumti)? else {
        return Ok(None);
    };
    if description.relative_clauses.is_some() {
        return Ok(None);
    }
    relation_variable_syntax_from_generated_selbri(&description.selbri)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn single_relation_parameter_syntax_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
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
    single_relation_parameter_syntax_from_generated_tanru_unit(first_unit)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn single_relation_parameter_syntax_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    };
    single_relation_parameter_syntax_from_bo_or_linked_tanru_unit(unit.0.first.as_ref())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn single_relation_parameter_syntax_from_bo_or_linked_tanru_unit(
    unit: &BoOrLinkedTanruUnitSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            single_relation_parameter_syntax_from_linked_tanru_unit(unit)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn single_relation_parameter_syntax_from_linked_tanru_unit(
    unit: &LinkedTanruUnitSyntax,
) -> Result<Option<GeneratedRelationParameterSyntax<'_>>, SemanticsError> {
    if unit.linkargs.is_some() || !unit.base.conversions.is_empty() {
        return Ok(None);
    }
    match unit.base.base.as_ref() {
        TanruUnitAtomBaseSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(Some(GeneratedRelationParameterSyntax::ProBridi(pro_bridi)))
        }
        TanruUnitAtomBaseSyntax::GohaWordTanruUnit(goha) => {
            Ok(Some(GeneratedRelationParameterSyntax::GohaWord(goha)))
        }
        _ => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn resolvable_generated_pro_bridi_cmavo_from_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<Cmavo>, SemanticsError> {
    let Some(parameter) = single_relation_parameter_syntax_from_generated_tanru_unit(unit)? else {
        return Ok(None);
    };
    Ok(generated_relation_parameter_token(parameter)
        .cmavo()
        .filter(|cmavo| generated_cmavo_is_resolvable_pro_bridi(*cmavo)))
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_cmavo_is_resolvable_pro_bridi))]
pub(super) fn resolvable_generated_pro_bridi_cmavo_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<Cmavo> {
    let ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(pro_bridi) = unit.inner_unit.as_ref()
    else {
        return None;
    };
    pro_bridi
        .goha
        .value
        .cmavo()
        .filter(|cmavo| generated_cmavo_is_resolvable_pro_bridi(*cmavo))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_relation_parameter_token(
    syntax: GeneratedRelationParameterSyntax<'_>,
) -> &Token {
    match syntax {
        GeneratedRelationParameterSyntax::ProBridi(pro_bridi) => &pro_bridi.goha.value,
        GeneratedRelationParameterSyntax::GohaWord(goha) => {
            generated_goha_word_tanru_unit_token(goha)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_goha_word_tanru_unit_token(unit: &GohaWordTanruUnitSyntax) -> &Token {
    let GohaWordTanruUnitSyntax(word) = unit;
    &word.value
}

#[requires(true)]
#[ensures(true)]
pub(super) fn tanru_selbri_from_selbri(
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
#[ensures(true)]
pub(super) fn tanru_selbri_from_co_selbri(
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
#[ensures(true)]
pub(super) fn sumti_selbri_from_selbri(
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
#[ensures(true)]
pub(super) fn sumti_selbri_from_generated_tanru_unit(
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
    if linkargs.is_some() || !atom.conversions.is_empty() {
        return Ok(None);
    }
    Ok(Some(sumti_selbri))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_selbri_requires_direct_description_body(
    selbri: &SelbriSyntax,
) -> Result<bool, SemanticsError> {
    let Some(tanru) = tanru_selbri_from_selbri(selbri)? else {
        return Ok(false);
    };
    if !tanru.additional_units.is_empty() {
        return Ok(false);
    }
    generated_tanru_unit_is_jai_conversion(&tanru.first_unit)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_unit_is_jai_conversion(
    unit: &TanruUnitSyntax,
) -> Result<bool, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(false);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = unit.0.first.as_ref() else {
        return Ok(false);
    };
    Ok(generated_tanru_unit_atom_base_is_jai_conversion(
        unit.base.base.as_ref(),
    ))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_unit_atom_base_is_jai_conversion(
    base: &TanruUnitAtomBaseSyntax,
) -> bool {
    match base {
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(_) => true,
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            let ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) = unit.inner_unit.as_ref()
            else {
                return false;
            };
            generated_tanru_unit_atom_base_is_jai_conversion(atom.base.as_ref())
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_unit_is_grouped(
    unit: &TanruUnitSyntax,
) -> Result<bool, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(false);
    }
    if matches!(
        unit.0.first.as_ref(),
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
    ) {
        return Ok(true);
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
#[ensures(true)]
pub(super) fn abstraction_from_generated_tanru_unit(
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
        return Ok(None);
    }
    if !atom.conversions.is_empty() {
        return Ok(None);
    }
    if abstraction.nai.is_some() {
        return Ok(None);
    }
    if !abstraction.abstractor_connections.is_empty() {
        return Ok(None);
    }
    Ok(Some(abstraction))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_linked_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<&LinkedTanruUnitSyntax, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Err(invalid_graph(
            "atomic tanru unit requested for a connected tanru unit".to_owned(),
        ));
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Err(invalid_graph(
            "atomic tanru unit requested for a BO-grouped tanru unit".to_owned(),
        ));
    };
    Ok(unit)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_linked_tanru_unit_parts(
    unit: &TanruUnitSyntax,
) -> Result<(&TanruUnitAtomSyntax, Option<&LinkargsSyntax>), SemanticsError> {
    let unit = generated_linked_tanru_unit(unit)?;
    Ok((&unit.base, unit.linkargs.as_ref()))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_selbri_is_single_converted_group(tanru: &TanruSelbriSyntax) -> bool {
    if !tanru.additional_units.is_empty() || !tanru.first_unit.0.links.is_empty() {
        return false;
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = tanru.first_unit.0.first.as_ref() else {
        return false;
    };
    !unit.base.conversions.is_empty()
        && matches!(
            unit.base.base.as_ref(),
            TanruUnitAtomBaseSyntax::GroupedTanruUnit(_)
        )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|unit| unit.is_none_or(|unit| unit.tense_modal.is_none())) || ret.is_err())]
pub(super) fn generated_bare_jai_modal_tanru_unit_from_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<&JaiModalTanruUnitSyntax>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = unit.0.first.as_ref() else {
        return Ok(None);
    };
    let atom = &unit.base;
    Ok(bare_generated_jai_modal_tanru_unit(atom.base.as_ref()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|unit| unit.is_none_or(|unit| unit.tense_modal.is_some())) || ret.is_err())]
pub(super) fn generated_jai_modal_tanru_unit_with_tense_from_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<Option<&JaiModalTanruUnitSyntax>, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Ok(None);
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = unit.0.first.as_ref() else {
        return Ok(None);
    };
    let atom = &unit.base;
    Ok(generated_jai_modal_tanru_unit_with_tense(
        atom.base.as_ref(),
    ))
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn generated_raw_place_visible_rank_for_selbri(
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
pub(super) fn generated_raw_place_visible_rank_for_untagged_selbri(
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
pub(super) fn generated_raw_place_visible_rank_for_connected_selbri(
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
pub(super) fn generated_raw_place_visible_rank_for_tanru_selbri(
    selbri: &TanruSelbriSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    let unit = selbri.additional_units.last().unwrap_or(&selbri.first_unit);
    generated_raw_place_visible_rank_for_tanru_unit(unit, place)
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn generated_raw_place_visible_rank_for_tanru_unit(
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
pub(super) fn generated_raw_place_visible_rank_for_bo_or_linked_tanru_unit(
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
        BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            let base = linked_tanru_unit_from_cei(unit.base.as_ref());
            generated_raw_place_visible_rank_for_tanru_unit_atom(&base.base, place)
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => Ok(place),
    }
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn generated_raw_place_visible_rank_for_tanru_unit_atom(
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
pub(super) fn generated_raw_place_visible_rank_for_scalar_negated_tanru_unit(
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
pub(super) fn mapped_place_for_generated_conversions<F>(
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
pub(super) fn map_visible_arguments_for_generated_conversions<F>(
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
pub(super) fn convert_numbered_place(place: usize, converted_place: usize) -> usize {
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
pub(super) fn se_conversion_place(token: &Token) -> Result<Option<usize>, SemanticsError> {
    match token.cmavo() {
        Some(Cmavo::Se) => Ok(Some(2)),
        Some(Cmavo::Te) => Ok(Some(3)),
        Some(Cmavo::Ve) => Ok(Some(4)),
        Some(Cmavo::Xe) => Ok(Some(5)),
        Some(cmavo) => Err(invalid_graph(format!(
            "generated SE conversion contains unexpected cmavo {cmavo:?}"
        ))),
        None => Err(invalid_graph(
            "generated SE conversion contains a non-cmavo token".to_owned(),
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| (1..=5).contains(place)) || ret.is_err())]
pub(super) fn fa_place(token: &Token) -> Result<usize, SemanticsError> {
    match token.cmavo() {
        Some(Cmavo::Fa) => Ok(1),
        Some(Cmavo::Fe) => Ok(2),
        Some(Cmavo::Fi) => Ok(3),
        Some(Cmavo::Fo) => Ok(4),
        Some(Cmavo::Fu) => Ok(5),
        Some(Cmavo::Fiha) => Err(invalid_graph(
            "place-question tag has no fixed place index".to_owned(),
        )),
        Some(Cmavo::Fai) => Err(invalid_graph(
            "FAI tag requires JAI-specific place resolution".to_owned(),
        )),
        Some(cmavo) => Err(invalid_graph(format!(
            "generated FA tag contains unexpected cmavo {cmavo:?}"
        ))),
        None => Err(invalid_graph(
            "generated FA tag contains a non-cmavo token".to_owned(),
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn linked_sumti_place(token: &Token) -> Result<usize, SemanticsError> {
    match token.cmavo() {
        Some(Cmavo::Fai) => Ok(1),
        _ => fa_place(token),
    }
}

#[requires(first_visible_place > 0)]
#[ensures(true)]
pub(super) fn linkargs_assign_visible_place_before(
    linkargs: &LinkargsSyntax,
    first_visible_place: usize,
) -> Result<bool, SemanticsError> {
    if linked_sumti_assigns_visible_place_before(
        generated_linked_term_leaf(&linkargs.first_link)?,
        first_visible_place,
    )? {
        return Ok(true);
    }
    for link in &linkargs.bei_links {
        if linked_sumti_assigns_visible_place_before(
            generated_linked_term_leaf(&link.link)?,
            first_visible_place,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[requires(first_visible_place > 0)]
#[ensures(true)]
pub(super) fn linked_sumti_assigns_visible_place_before(
    link: GeneratedLinkedSumtiRef<'_>,
    first_visible_place: usize,
) -> Result<bool, SemanticsError> {
    match link {
        GeneratedLinkedSumtiRef::PlaceTagged(sumti) => {
            Ok(linked_sumti_place(&sumti.fa.value)? < first_visible_place)
        }
        GeneratedLinkedSumtiRef::Plain(_)
        | GeneratedLinkedSumtiRef::TenseTagged(_)
        | GeneratedLinkedSumtiRef::Empty => Ok(false),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() == GeneratedLinkedSumtiRef::from_linked_term(link).is_some())]
pub(super) fn generated_linked_term_leaf(
    link: &LinkedTermSyntax,
) -> Result<GeneratedLinkedSumtiRef<'_>, SemanticsError> {
    GeneratedLinkedSumtiRef::from_linked_term(link).ok_or_else(|| {
        undefined_semantics("a grouped linked-term connection in the term-hierarchy dialect")
    })
}

#[requires(occupied_places.iter().all(|place| *place > 0))]
#[requires(*next_visible_place > 0)]
#[ensures(*next_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| place.is_none_or(|place| place > 0)) || ret.is_err())]
pub(super) fn generated_linked_sumti_numbered_place(
    occupied_places: &BTreeSet<usize>,
    next_visible_place: &mut usize,
    link: GeneratedLinkedSumtiRef<'_>,
) -> Result<Option<usize>, SemanticsError> {
    match link {
        GeneratedLinkedSumtiRef::Plain(_) => {
            while occupied_places.contains(next_visible_place) {
                *next_visible_place += 1;
            }
            let place = *next_visible_place;
            *next_visible_place = place + 1;
            Ok(Some(place))
        }
        GeneratedLinkedSumtiRef::PlaceTagged(sumti) => {
            let place = linked_sumti_place(&sumti.fa.value)?;
            *next_visible_place = place + 1;
            Ok(Some(place))
        }
        GeneratedLinkedSumtiRef::TenseTagged(_) | GeneratedLinkedSumtiRef::Empty => Ok(None),
    }
}

#[requires(place > 0)]
#[ensures(true)]
pub(super) fn insert_visible_argument(
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

#[requires(place > 0)]
#[ensures(!arguments.get(&place).is_none_or(|values| values.is_empty()))]
pub(super) fn insert_generated_alternative_argument<'syntax>(
    arguments: &mut BTreeMap<usize, Vec<GeneratedAlternativeArgumentSource<'syntax>>>,
    place: usize,
    argument: GeneratedAlternativeArgumentSource<'syntax>,
) -> Result<(), SemanticsError> {
    arguments.entry(place).or_default().push(argument);
    Ok(())
}

#[requires(replacements.values().all(|replacement| crate::model::argument_object_kind_can_fill(replacement.object_kind())))]
#[ensures(true)]
pub(super) fn replace_generated_argument_value_object(
    argument: &mut ArgumentValue,
    replacements: &BTreeMap<SemanticObjectId, SemanticObjectId>,
) {
    if let Some(value) = argument.value
        && let Some(replacement) = replacements.get(&value)
    {
        let mut data = argument.clone().into_data();
        data.value = Some(*replacement);
        *argument = ArgumentValue::from_data(data);
    }
}

#[requires(visible_arguments.keys().all(|place| *place > 0))]
#[ensures(true)]
pub(super) fn apply_generated_bare_jai_visible_argument(
    builder: &mut GeneratedGraphBuilder<'_, '_, '_>,
    visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
    unit: Option<&JaiModalTanruUnitSyntax>,
) -> Result<(), SemanticsError> {
    let source = unit.and_then(|unit| builder.source_for_node(unit, "abstraction-about"));
    apply_generated_bare_jai_visible_argument_with_source(builder, visible_arguments, unit, source)
}

#[requires(visible_arguments.keys().all(|place| *place > 0))]
#[ensures(true)]
pub(super) fn apply_generated_bare_jai_visible_argument_with_source(
    builder: &mut GeneratedGraphBuilder<'_, '_, '_>,
    visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
    unit: Option<&JaiModalTanruUnitSyntax>,
    source: Option<crate::model::SemanticSource>,
) -> Result<(), SemanticsError> {
    let Some(_unit) = unit else {
        return Ok(());
    };
    let Some(argument) = visible_arguments.remove(&1) else {
        return Ok(());
    };
    let Some(operand) = argument.value else {
        visible_arguments.insert(1, argument);
        return Ok(());
    };
    let referent = builder.build_generated_abstraction_about_referent("jai", operand, source)?;
    visible_arguments.insert(1, ArgumentValue::filled(referent, None));
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_jai_modal_tanru_unit(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    match base {
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) => Some(unit),
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_jai_modal_tanru_unit_from_scalar_negated_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_jai_modal_tanru_unit_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    let ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) = unit.inner_unit.as_ref() else {
        return None;
    };
    generated_jai_modal_tanru_unit(atom.base.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn generated_jai_moved_relation_place(
    unit: &JaiModalTanruUnitSyntax,
) -> Result<usize, SemanticsError> {
    generated_jai_inner_moved_relation_place(unit.inner_unit.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn generated_jai_inner_moved_relation_place(
    unit: &JaiInnerTanruUnitSyntax,
) -> Result<usize, SemanticsError> {
    match unit {
        JaiInnerTanruUnitSyntax::ConvertedJaiInnerTanruUnit(unit) => {
            Ok(se_conversion_place(&unit.se.value)?.unwrap_or(1))
        }
        JaiInnerTanruUnitSyntax::ScalarNegatedJaiInnerTanruUnit(unit) => {
            generated_jai_inner_moved_relation_place(unit.inner_unit.as_ref())
        }
        _ => Ok(1),
    }
}

#[requires(arguments.keys().all(|place| *place > 0))]
#[ensures(ret.as_ref().is_ok_and(|shifted| shifted.keys().all(|place| *place > 0)) || ret.is_err())]
pub(super) fn shift_generated_visible_arguments_after_jai_raised_argument(
    arguments: BTreeMap<usize, ArgumentValue>,
) -> Result<BTreeMap<usize, ArgumentValue>, SemanticsError> {
    let mut shifted = BTreeMap::new();
    for (place, argument) in arguments {
        let shifted_place = place.saturating_sub(1).max(1);
        if shifted.insert(shifted_place, argument).is_some() {
            return Err(invalid_graph(format!(
                "multiple generated JAI arguments map to visible place x{shifted_place}"
            )));
        }
    }
    Ok(shifted)
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_none()))]
pub(super) fn bare_generated_jai_modal_tanru_unit(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    match base {
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) if unit.tense_modal.is_none() => {
            Some(unit)
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            bare_generated_jai_from_scalar_negated_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_none()))]
pub(super) fn bare_generated_jai_modal_tanru_atom_base_view(
    base: GeneratedTanruAtomBaseView<'_>,
) -> Option<&JaiModalTanruUnitSyntax> {
    match base {
        GeneratedTanruAtomBaseView::Normal(base) => bare_generated_jai_modal_tanru_unit(base),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit))
            if unit.tense_modal.is_none() =>
        {
            Some(unit)
        }
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(
            unit,
        )) => bare_generated_jai_from_scalar_negated_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_none()))]
pub(super) fn bare_generated_jai_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    let ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) = unit.inner_unit.as_ref() else {
        return None;
    };
    bare_generated_jai_modal_tanru_unit(atom.base.as_ref())
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_some()))]
pub(super) fn generated_jai_modal_tanru_unit_with_tense(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    match base {
        TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) if unit.tense_modal.is_some() => {
            Some(unit)
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_jai_modal_tanru_unit_with_tense_from_scalar_negated_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_some()))]
pub(super) fn generated_jai_modal_tanru_atom_base_view_with_tense(
    base: GeneratedTanruAtomBaseView<'_>,
) -> Option<&JaiModalTanruUnitSyntax> {
    match base {
        GeneratedTanruAtomBaseView::Normal(base) => generated_jai_modal_tanru_unit_with_tense(base),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit))
            if unit.tense_modal.is_some() =>
        {
            Some(unit)
        }
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(
            unit,
        )) => generated_jai_modal_tanru_unit_with_tense_from_scalar_negated_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|unit| unit.tense_modal.is_some()))]
pub(super) fn generated_jai_modal_tanru_unit_with_tense_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<&JaiModalTanruUnitSyntax> {
    let ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) = unit.inner_unit.as_ref() else {
        return None;
    };
    generated_jai_modal_tanru_unit_with_tense(atom.base.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_generated_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut label = relation_label_from_bo_or_linked_tanru_unit(&unit.0.first)?;
    for link in &unit.0.links {
        label = RelationLabel::constructed(format!(
            "{} {} {}",
            label,
            relation_afterthought_connective_label(&link.connective)?,
            relation_label_from_bo_or_linked_tanru_unit(&link.trailing_unit)?
        ));
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn tanru_unit_label_from_generated_unit(
    unit: &TanruUnitSyntax,
) -> Result<String, SemanticsError> {
    if !unit.0.links.is_empty() {
        return relation_label_from_generated_tanru_unit(unit).map(|label| label.display_text());
    }
    tanru_unit_label_from_bo_or_linked_tanru_unit(&unit.0.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|relation| relation.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_tanru_unit_atom_base(
    base: &TanruUnitAtomBaseSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match base {
        TanruUnitAtomBaseSyntax::OrdinalTanruUnit(ordinal) => {
            relation_label_from_ordinal_tanru_unit(ordinal)
        }
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            Ok(relation_label_from_token(&word.value))
        }
        TanruUnitAtomBaseSyntax::GohaWordTanruUnit(GohaWordTanruUnitSyntax(word)) => {
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
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_tanru_atom_base_view(
    base: GeneratedTanruAtomBaseView<'_>,
) -> Result<RelationLabel, SemanticsError> {
    match base {
        GeneratedTanruAtomBaseView::Normal(base) => relation_label_from_tanru_unit_atom_base(base),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::OrdinalTanruUnit(
            ordinal,
        )) => relation_label_from_ordinal_tanru_unit(ordinal),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(
            WordTanruUnitSyntax(word),
        )) => Ok(relation_label_from_token(&word.value)),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(
            GohaWordTanruUnitSyntax(word),
        )) => Ok(relation_label_from_token(&word.value)),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(
            pro_bridi,
        )) => Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi)),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(
            unit,
        )) => relation_label_from_scalar_negated_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(
            grouped,
        )) => relation_label_from_grouped_tanru_unit(grouped),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::AbstractionTanruUnit(
            abstraction,
        )) => abstraction_relation_label_from_generated(abstraction),
        GeneratedTanruAtomBaseView::Cei(
            TanruUnitAtomBaseForCeiSyntax::ZantufaStatementAbstractionTanruUnit(abstraction),
        ) => abstraction_relation_label_from_zantufa_statement(abstraction),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ZantufaMeTanruUnit(
            unit,
        )) => relation_label_from_zantufa_me_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ZantufaMexMoiTanruUnit(
            unit,
        )) => relation_label_from_zantufa_mex_moi_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(_)) => {
            Ok(RelationLabel::constructed("referentOf".to_owned()))
        }
        GeneratedTanruAtomBaseView::Cei(
            TanruUnitAtomBaseForCeiSyntax::OperatorSelbriTanruUnit(operator),
        ) => relation_label_from_operator_selbri_tanru_unit(operator),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit)) => {
            relation_label_from_jai_inner_tanru_unit(&unit.inner_unit)
        }
        GeneratedTanruAtomBaseView::Cei(
            TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit),
        ) => relation_label_from_generated_tanru_unit(&unit.base),
        GeneratedTanruAtomBaseView::Cei(
            TanruUnitAtomBaseForCeiSyntax::QuotedBridiSelbriTanruUnit(unit),
        ) => Ok(RelationLabel::constructed(generated_node_surface_text(
            unit,
        )?)),
        GeneratedTanruAtomBaseView::Cei(
            TanruUnitAtomBaseForCeiSyntax::QuotedTextSelbriTanruUnit(unit),
        ) => Ok(RelationLabel::constructed(generated_node_surface_text(
            unit,
        )?)),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::TextSelbriTanruUnit(
            unit,
        )) => Ok(RelationLabel::constructed(generated_node_surface_text(
            unit,
        )?)),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::TagSelbriTanruUnit(
            unit,
        )) => Ok(RelationLabel::constructed(generated_node_surface_text(
            unit,
        )?)),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_lujvo_rafsi_parts_for_tanru_unit_atom_base(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<Vec<String>> {
    match base {
        TanruUnitAtomBaseSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            generated_lujvo_rafsi_parts_for_token(&word.value)
        }
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_lujvo_rafsi_parts_for_scalar_negated_tanru_unit(unit)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_lujvo_rafsi_parts_for_tanru_atom_base_view(
    base: GeneratedTanruAtomBaseView<'_>,
) -> Option<Vec<String>> {
    match base {
        GeneratedTanruAtomBaseView::Normal(base) => {
            generated_lujvo_rafsi_parts_for_tanru_unit_atom_base(base)
        }
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(
            WordTanruUnitSyntax(word),
        )) => generated_lujvo_rafsi_parts_for_token(&word.value),
        GeneratedTanruAtomBaseView::Cei(TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(
            unit,
        )) => generated_lujvo_rafsi_parts_for_scalar_negated_tanru_unit(unit),
        GeneratedTanruAtomBaseView::Cei(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_lujvo_rafsi_parts_for_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<Vec<String>> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            generated_lujvo_rafsi_parts_for_tanru_unit_atom_base(atom.base.as_ref())
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(_)
        | ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_lujvo_rafsi_parts_for_token(token: &Token) -> Option<Vec<String>> {
    let word = token.core_word().bare_word()?;
    match word.as_data() {
        data!(Word::Lujvo { parts, .. }) => Some(
            parts
                .iter()
                .filter_map(|part| match part {
                    LujvoPart::Rafsi(phonemes) => Some(strip_diacritics(phonemes.as_str())),
                    LujvoPart::Hyphen(_) => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_jai_inner_tanru_unit(
    unit: &JaiInnerTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match unit {
        JaiInnerTanruUnitSyntax::WordTanruUnit(WordTanruUnitSyntax(word)) => {
            Ok(relation_label_from_token(&word.value))
        }
        JaiInnerTanruUnitSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi))
        }
        JaiInnerTanruUnitSyntax::OrdinalTanruUnit(ordinal) => {
            relation_label_from_ordinal_tanru_unit(ordinal)
        }
        JaiInnerTanruUnitSyntax::OperatorSelbriTanruUnit(operator) => {
            relation_label_from_operator_selbri_tanru_unit(operator)
        }
        JaiInnerTanruUnitSyntax::SumtiSelbriTanruUnit(_) => {
            Ok(RelationLabel::constructed("referentOf".to_owned()))
        }
        JaiInnerTanruUnitSyntax::ConvertedJaiInnerTanruUnit(unit) => {
            relation_label_from_jai_inner_tanru_unit(&unit.inner_unit)
        }
        JaiInnerTanruUnitSyntax::ScalarNegatedJaiInnerTanruUnit(unit) => {
            relation_label_from_jai_inner_tanru_unit(&unit.inner_unit)
        }
        JaiInnerTanruUnitSyntax::GroupedJaiInnerTanruUnit(grouped) => {
            relation_label_from_connected_jai_inner_selbri(&grouped.selbri)
        }
        JaiInnerTanruUnitSyntax::QuotedBridiSelbriTanruUnit(unit) => Ok(
            RelationLabel::constructed(generated_node_surface_text(unit)?),
        ),
        JaiInnerTanruUnitSyntax::QuotedTextSelbriTanruUnit(unit) => Ok(RelationLabel::constructed(
            generated_node_surface_text(unit)?,
        )),
        JaiInnerTanruUnitSyntax::TextSelbriTanruUnit(unit) => Ok(RelationLabel::constructed(
            generated_node_surface_text(unit)?,
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_connected_jai_inner_selbri(
    selbri: &jbotci_syntax::generated_model::ConnectedJaiInnerSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut label = relation_label_from_tanru_jai_inner_selbri(&selbri.leading_selbri)?;
    for continuation in &selbri.continuations {
        label = RelationLabel::constructed(format!(
            "{} {} {}",
            label,
            relation_afterthought_connective_label(&continuation.connective)?,
            relation_label_from_tanru_jai_inner_selbri(&continuation.trailing_selbri)?
        ));
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_tanru_jai_inner_selbri(
    selbri: &jbotci_syntax::generated_model::TanruJaiInnerSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut labels =
        vec![relation_label_from_jai_inner_tanru_unit(&selbri.first_unit)?.display_text()];
    for unit in &selbri.additional_units {
        labels.push(relation_label_from_jai_inner_tanru_unit(unit)?.display_text());
    }
    Ok(RelationLabel::constructed(labels.join("-")))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn scalar_negated_tanru_atom_base(
    base: &TanruUnitAtomBaseSyntax,
) -> Option<&ScalarNegatedTanruUnitSyntax> {
    match base {
        TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => Some(unit),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_unit_has_scalar_negated_base(unit: &TanruUnitSyntax) -> bool {
    if !unit.0.links.is_empty() {
        return false;
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = unit.0.first.as_ref() else {
        return false;
    };
    scalar_negated_tanru_atom_base(unit.base.base.as_ref()).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|negation| negation.argument_scope.iter().all(|place| place.get() > 0)) || ret.is_err())]
pub(super) fn scalar_negation_for_generated_scalar_tanru_unit_atom(
    atom: &TanruUnitAtomSyntax,
    unit: &ScalarNegatedTanruUnitSyntax,
    linkargs: Option<&LinkargsSyntax>,
    scope: GeneratedScalarNegationScope,
) -> Result<ScalarNegation, SemanticsError> {
    scalar_negation_for_generated_scalar_tanru_atom_view(
        GeneratedTanruAtomView::normal(atom),
        unit,
        linkargs,
        scope,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|negation| negation.argument_scope.iter().all(|place| place.get() > 0)) || ret.is_err())]
pub(super) fn scalar_negation_for_generated_scalar_tanru_atom_view(
    atom: GeneratedTanruAtomView<'_>,
    unit: &ScalarNegatedTanruUnitSyntax,
    linkargs: Option<&LinkargsSyntax>,
    scope: GeneratedScalarNegationScope,
) -> Result<ScalarNegation, SemanticsError> {
    if scope == GeneratedScalarNegationScope::MarkerOnly {
        return Ok(scalar_negation_for_marker(&unit.nahe));
    }
    let mut places = BTreeSet::new();
    places.insert(mapped_visible_place_for_generated_scalar_tanru_atom_view(
        atom, unit, 1,
    )?);
    if let Some(linkargs) = linkargs {
        for place in generated_linkargs_visible_places(linkargs, 2)? {
            places.insert(mapped_visible_place_for_generated_scalar_tanru_atom_view(
                atom, unit, place,
            )?);
        }
    }
    if let Some((grouped, _)) = scalar_negated_tanru_unit_inner_grouped(unit) {
        let mut grouped_places = BTreeSet::new();
        add_generated_connected_selbri_visible_linkarg_places(
            &mut grouped_places,
            &grouped.selbri,
            2,
        )?;
        for place in grouped_places {
            places.insert(mapped_visible_place_for_generated_scalar_tanru_atom_view(
                atom, unit, place,
            )?);
        }
    }
    Ok(scalar_negation_for_marker(&unit.nahe)
        .with_argument_scope(places.into_iter().map(argument_key).collect()))
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn mapped_visible_place_for_generated_scalar_tanru_unit_atom(
    atom: &TanruUnitAtomSyntax,
    unit: &ScalarNegatedTanruUnitSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    mapped_visible_place_for_generated_scalar_tanru_atom_view(
        GeneratedTanruAtomView::normal(atom),
        unit,
        place,
    )
}

#[requires(place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place > 0) || ret.is_err())]
pub(super) fn mapped_visible_place_for_generated_scalar_tanru_atom_view(
    atom: GeneratedTanruAtomView<'_>,
    unit: &ScalarNegatedTanruUnitSyntax,
    place: usize,
) -> Result<usize, SemanticsError> {
    let place = mapped_place_for_generated_conversions(place, atom.conversions())?;
    match scalar_negated_tanru_unit_inner_atom(unit) {
        Some(inner_atom) => mapped_place_for_generated_conversions(place, &inner_atom.conversions),
        None => Ok(place),
    }
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|places| places.iter().all(|place| *place > 0)) || ret.is_err())]
pub(super) fn generated_linkargs_visible_places(
    linkargs: &LinkargsSyntax,
    first_visible_place: usize,
) -> Result<BTreeSet<usize>, SemanticsError> {
    let mut places = BTreeSet::new();
    let mut next_visible_place = first_visible_place;
    add_generated_linked_sumti_visible_places(
        &mut places,
        &mut next_visible_place,
        generated_linked_term_leaf(&linkargs.first_link)?,
    )?;
    for link in &linkargs.bei_links {
        add_generated_linked_sumti_visible_places(
            &mut places,
            &mut next_visible_place,
            generated_linked_term_leaf(&link.link)?,
        )?;
    }
    Ok(places)
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_selbri_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    selbri: &SelbriSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => add_generated_untagged_selbri_visible_linkarg_places(
            places,
            tagged.inner_selbri.as_ref(),
            first_visible_place,
        ),
        SelbriSyntax::UntaggedSelbri(untagged) => {
            add_generated_untagged_selbri_visible_linkarg_places(
                places,
                untagged,
                first_visible_place,
            )
        }
    }
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_untagged_selbri_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    selbri: &UntaggedSelbriSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    match selbri {
        UntaggedSelbriSyntax::CoSelbri(co_selbri) => {
            add_generated_co_selbri_visible_linkarg_places(places, co_selbri, first_visible_place)
        }
        UntaggedSelbriSyntax::NegatedSelbri(negated) => {
            add_generated_selbri_visible_linkarg_places(
                places,
                negated.inner_selbri.as_ref(),
                first_visible_place,
            )
        }
        UntaggedSelbriSyntax::ForethoughtSelbriConnection(connection) => {
            add_generated_selbri_visible_linkarg_places(
                places,
                connection.leading_selbri.as_ref(),
                first_visible_place,
            )?;
            add_generated_selbri_visible_linkarg_places(
                places,
                connection.first_branch.selbri.as_ref(),
                first_visible_place,
            )?;
            for branch in &connection.additional_branches {
                add_generated_selbri_visible_linkarg_places(
                    places,
                    branch.selbri.as_ref(),
                    first_visible_place,
                )?;
            }
            Ok(())
        }
    }
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_co_selbri_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    selbri: &CoSelbriSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    add_generated_connected_selbri_visible_linkarg_places(
        places,
        selbri.leading_selbri.as_ref(),
        first_visible_place,
    )?;
    if let Some(co_tail) = &selbri.co_tail {
        add_generated_co_selbri_visible_linkarg_places(
            places,
            co_tail.trailing_selbri.as_ref(),
            first_visible_place,
        )?;
    }
    Ok(())
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_connected_selbri_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    selbri: &ConnectedSelbriSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    add_generated_tanru_selbri_visible_linkarg_places(
        places,
        &selbri.leading_selbri,
        first_visible_place,
    )?;
    for continuation in &selbri.continuations {
        add_generated_tanru_selbri_visible_linkarg_places(
            places,
            &continuation.trailing_selbri,
            first_visible_place,
        )?;
    }
    Ok(())
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_tanru_selbri_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    selbri: &TanruSelbriSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    add_generated_tanru_unit_visible_linkarg_places(
        places,
        &selbri.first_unit,
        first_visible_place,
    )?;
    for unit in &selbri.additional_units {
        add_generated_tanru_unit_visible_linkarg_places(places, unit, first_visible_place)?;
    }
    Ok(())
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_tanru_unit_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    unit: &TanruUnitSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    add_generated_bo_or_linked_tanru_unit_visible_linkarg_places(
        places,
        unit.0.first.as_ref(),
        first_visible_place,
    )?;
    for link in &unit.0.links {
        add_generated_bo_or_linked_tanru_unit_visible_linkarg_places(
            places,
            link.trailing_unit.as_ref(),
            first_visible_place,
        )?;
    }
    Ok(())
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_bo_or_linked_tanru_unit_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    unit: &BoOrLinkedTanruUnitSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    match unit {
        BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            add_generated_linked_tanru_unit_visible_linkarg_places(
                places,
                unit,
                first_visible_place,
            )
        }
        BoOrLinkedTanruUnitSyntax::BoundTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_)
        | BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(_) => Ok(()),
    }
}

#[requires(places.iter().all(|place| *place > 0))]
#[requires(first_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_linked_tanru_unit_visible_linkarg_places(
    places: &mut BTreeSet<usize>,
    unit: &LinkedTanruUnitSyntax,
    first_visible_place: usize,
) -> Result<(), SemanticsError> {
    if let Some(linkargs) = &unit.linkargs {
        places.extend(generated_linkargs_visible_places(
            linkargs,
            first_visible_place,
        )?);
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_linkargs_provide_scalar_scale_context(linkargs: &LinkargsSyntax) -> bool {
    GeneratedLinkedSumtiRef::from_linked_term(&linkargs.first_link)
        .is_some_and(generated_linked_sumti_provides_scalar_scale_context)
        || linkargs.bei_links.iter().any(|link| {
            GeneratedLinkedSumtiRef::from_linked_term(&link.link)
                .is_some_and(generated_linked_sumti_provides_scalar_scale_context)
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_linked_sumti_provides_scalar_scale_context(
    link: GeneratedLinkedSumtiRef<'_>,
) -> bool {
    let GeneratedLinkedSumtiRef::TenseTagged(sumti) = link else {
        return false;
    };
    generated_modal_relation_spec_for_tense_modal(sumti.tense_modal.as_ref())
        .is_some_and(|(_, relation, _)| matches!(relation.as_str(), "ckilu" | "ciste" | "klesi"))
}

#[requires(*next_visible_place > 0)]
#[requires(places.iter().all(|place| *place > 0))]
#[ensures(*next_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_linked_sumti_visible_places(
    places: &mut BTreeSet<usize>,
    next_visible_place: &mut usize,
    link: GeneratedLinkedSumtiRef<'_>,
) -> Result<(), SemanticsError> {
    if matches!(link, GeneratedLinkedSumtiRef::Empty) {
        return Err(invalid_graph(
            "generated empty linked sumti has no visible place".to_owned(),
        ));
    }
    if let Some(place) = generated_linked_sumti_numbered_place(places, next_visible_place, link)? {
        places.insert(place);
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn scalar_negated_tanru_unit_inner_atom(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Option<&TanruUnitAtomSyntax> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => Some(atom),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn scalar_negated_tanru_unit_inner_grouped(
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
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_scalar_negated_tanru_unit(
    unit: &ScalarNegatedTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    match unit.inner_unit.as_ref() {
        ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(atom) => {
            relation_label_from_tanru_unit_atom_base(atom.base.as_ref())
        }
        ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(pro_bridi) => {
            Ok(relation_label_from_pro_bridi_tanru_unit(pro_bridi))
        }
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(tagged) => {
            relation_label_from_connected_selbri(&tagged.inner_selbri)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_grouped_tanru_unit(
    grouped: &GroupedTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    relation_label_from_connected_selbri(&grouped.selbri)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn relation_phrase_label_from_selbri(
    selbri: &SelbriSyntax,
) -> Result<String, SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => generated_node_surface_text(tagged),
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) => {
            if co_selbri.co_tail.is_some() {
                return generated_node_surface_text(co_selbri);
            }
            relation_phrase_label_from_connected_selbri(co_selbri.leading_selbri.as_ref())
        }
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::NegatedSelbri(negated)) => {
            generated_node_surface_text(negated)
        }
        SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::ForethoughtSelbriConnection(
            connection,
        )) => {
            let mut parts = vec![
                generated_guhek_connective_source(&connection.guhek),
                relation_phrase_label_from_selbri(connection.leading_selbri.as_ref())?,
                token_text(&connection.first_branch.gik.gi.value),
                relation_phrase_label_from_selbri(connection.first_branch.selbri.as_ref())?,
            ];
            for branch in &connection.additional_branches {
                parts.push(token_text(&branch.gik.0.value));
                parts.push(relation_phrase_label_from_selbri(branch.selbri.as_ref())?);
            }
            Ok(parts.join(" "))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_connected_selbri(
    selbri: &ConnectedSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    if selbri.continuations.is_empty() {
        return relation_label_from_tanru_selbri(&selbri.leading_selbri);
    }
    relation_phrase_label_from_connected_selbri(selbri).map(RelationLabel::constructed)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn relation_phrase_label_from_connected_selbri(
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
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_tanru_selbri(
    tanru: &TanruSelbriSyntax,
) -> Result<RelationLabel, SemanticsError> {
    if tanru.additional_units.is_empty() {
        return relation_label_from_generated_tanru_unit(&tanru.first_unit);
    }
    relation_phrase_label_from_tanru_selbri(tanru).map(RelationLabel::constructed)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn relation_phrase_label_from_tanru_selbri(
    tanru: &TanruSelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut label = relation_label_from_generated_tanru_unit(&tanru.first_unit)?.display_text();
    for unit in &tanru.additional_units {
        label = format!(
            "{label} {}",
            relation_label_from_generated_tanru_unit(unit)?.display_text()
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn tanru_label_from_tanru_selbri(
    tanru: &TanruSelbriSyntax,
) -> Result<String, SemanticsError> {
    let mut label = tanru_unit_label_from_generated_unit(&tanru.first_unit)?;
    for (index, unit) in tanru.additional_units.iter().enumerate() {
        let is_trailing_unit = index + 1 == tanru.additional_units.len();
        let unit_label = if is_trailing_unit && generated_tanru_unit_label_needs_parentheses(unit) {
            tanru_operand_label_from_generated_unit(unit)?
        } else {
            tanru_unit_label_from_generated_unit(unit)?
        };
        label = format!("{label}-{unit_label}");
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn tanru_label_from_connected_selbri(
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
pub(super) fn tanru_label_from_co_selbri(
    selbri: &CoSelbriSyntax,
) -> Result<String, SemanticsError> {
    let Some(co_tail) = &selbri.co_tail else {
        return tanru_label_from_connected_selbri(&selbri.leading_selbri);
    };
    Ok(format!(
        "{}-{}",
        tanru_label_from_co_selbri(&co_tail.trailing_selbri)?,
        tanru_label_from_connected_selbri(&selbri.leading_selbri)?,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn tanru_relation_name_for_generated_co_pair(
    leading_modifier: &CoSelbriSyntax,
    trailing_head: &ConnectedSelbriSyntax,
) -> Result<String, SemanticsError> {
    let trailing_label = tanru_label_from_connected_selbri(trailing_head)?;
    let trailing_label = if generated_connected_selbri_label_needs_parentheses(trailing_head) {
        format!("({trailing_label})")
    } else {
        trailing_label
    };
    Ok(format!(
        "{}-{trailing_label}",
        tanru_label_from_co_selbri(leading_modifier)?
    ))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_connected_selbri_label_needs_parentheses(
    selbri: &ConnectedSelbriSyntax,
) -> bool {
    !selbri.continuations.is_empty()
        || generated_tanru_selbri_label_needs_parentheses(&selbri.leading_selbri)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tanru_selbri_label_needs_parentheses(selbri: &TanruSelbriSyntax) -> bool {
    if selbri.additional_units.is_empty() {
        return generated_tanru_unit_label_needs_parentheses(&selbri.first_unit);
    }
    selbri
        .additional_units
        .last()
        .is_some_and(generated_tanru_unit_label_needs_parentheses)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn relation_afterthought_connective_label(
    connective: &RelationAfterthoughtConnectiveSyntax,
) -> Result<String, SemanticsError> {
    Ok(generated_relation_afterthought_connective_source(connective)?.replace(' ', "-"))
}

#[requires(true)]
#[ensures(ret.is_displayable())]
pub(super) fn relation_label_from_pro_bridi_tanru_unit(
    unit: &ProBridiTanruUnitSyntax,
) -> RelationLabel {
    RelationLabel::pro_bridi(token_text(&unit.goha.value))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_ordinal_tanru_unit(
    ordinal: &OrdinalTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    ordinal.visit_in_order(&mut visitor);
    if visitor.tokens.len() < 2 {
        return Err(invalid_graph(
            "generated ordinal tanru unit has fewer than two tokens".to_owned(),
        ));
    }
    let moi = token_text(
        visitor
            .tokens
            .last()
            .expect("checked above that ordinal has tokens"),
    );
    let expression = token_list_text(visitor.tokens[..visitor.tokens.len() - 1].iter().copied());
    Ok(RelationLabel::mekso_moi(expression, moi))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_operator_selbri_tanru_unit(
    unit: &OperatorSelbriTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    Ok(RelationLabel::nuha_operator(
        generated_mekso_operator_label(&unit.mekso_operator)?,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_zantufa_me_tanru_unit(
    unit: &ZantufaMeTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    let mut parts = vec![token_text(&unit.me.value)];
    match unit.body.as_ref() {
        ZantufaMeSelbriBodySyntax::ZantufaMeOperatorSelbriBody(body) => {
            for operator in &body.0 {
                parts.push(generated_mekso_operator_surface_label(operator)?);
            }
        }
        ZantufaMeSelbriBodySyntax::ZantufaMeMeksoSelbriBody(body) => {
            parts.push(generated_mekso_surface_text(body.0.as_ref())?);
        }
        ZantufaMeSelbriBodySyntax::ZantufaMeTagSelbriBody(body) => {
            parts.push(generated_node_surface_text(body.0.as_ref())?);
        }
    }
    Ok(RelationLabel::constructed(parts.join(" ")))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| label.is_displayable()) || ret.is_err())]
pub(super) fn relation_label_from_zantufa_mex_moi_tanru_unit(
    unit: &ZantufaMexMoiTanruUnitSyntax,
) -> Result<RelationLabel, SemanticsError> {
    Ok(RelationLabel::mekso_moi(
        generated_mekso_surface_text(&unit.expression)?,
        token_text(&unit.moi.value),
    ))
}
