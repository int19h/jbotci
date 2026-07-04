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
        return Err(unsupported("tagged or connected selbri"));
    };
    let SelbriSyntax::UntaggedSelbri(UntaggedSelbriSyntax::CoSelbri(co_selbri)) = selbri else {
        unreachable!("previous pattern requires a co selbri")
    };
    relation_label_from_co_selbri(co_selbri)
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
pub(super) fn generated_pro_bridi_replay_source_from_bridi(
    bridi: &BridiSyntax,
) -> Result<Option<GeneratedProBridiReplaySource>, SemanticsError> {
    match bridi {
        BridiSyntax::BridiWithLeadingTerms(bridi) => {
            let simple_tail = simple_tail_from_bridi_tail(&bridi.bridi_tail)?;
            let terms = bridi
                .leading_terms
                .iter()
                .chain(simple_tail.terms.iter())
                .cloned()
                .collect::<Vec<_>>();
            Ok(Some(new!(GeneratedProBridiReplaySource {
                selbri: simple_tail.selbri.as_ref().clone(),
                terms,
                first_visible_place: 1,
            })))
        }
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
            let simple_tail = simple_tail_from_bridi_tail(bridi_tail)?;
            Ok(Some(new!(GeneratedProBridiReplaySource {
                selbri: simple_tail.selbri.as_ref().clone(),
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
) -> Option<TenseModalSyntax> {
    let SelbriSyntax::TaggedSelbri(tagged) = selbri else {
        return None;
    };
    generated_tense_modal_has_event_modifier(tagged.tense_modal.as_ref())
        .then(|| tagged.tense_modal.as_ref().clone())
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
pub(super) fn connected_selbri_as_tanru(
    selbri: &ConnectedSelbriSyntax,
) -> Result<&TanruSelbriSyntax, SemanticsError> {
    if !selbri.continuations.is_empty() {
        return Err(unsupported("connected grouped tanru unit"));
    }
    Ok(selbri.leading_selbri.as_ref())
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
    if linkargs.is_some() {
        return Err(unsupported("linkargs sumti selbri"));
    }
    if !atom.conversions.is_empty() {
        return Err(unsupported("converted sumti selbri"));
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
pub(super) fn generated_tanru_unit_atom(
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
#[ensures(true)]
pub(super) fn generated_linked_tanru_unit(
    unit: &TanruUnitSyntax,
) -> Result<&LinkedTanruUnitSyntax, SemanticsError> {
    if !unit.0.links.is_empty() {
        return Err(unsupported("connected tanru unit"));
    }
    let BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) = &*unit.0.first else {
        return Err(unsupported("non-atomic tanru unit"));
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
        Some(cmavo) => Err(unsupported(&format!("SE conversion cmavo {cmavo:?}"))),
        None => Err(unsupported("non-cmavo SE conversion")),
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
        Some(Cmavo::Fiha) => Err(unsupported("place-question linked sumti")),
        Some(Cmavo::Fai) => Err(unsupported("FAI linked sumti")),
        Some(cmavo) => Err(unsupported(&format!("FA linked sumti cmavo {cmavo:?}"))),
        None => Err(unsupported("non-cmavo FA linked sumti")),
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
    if linked_sumti_assigns_visible_place_before(&linkargs.first_link, first_visible_place)? {
        return Ok(true);
    }
    for link in &linkargs.bei_links {
        if linked_sumti_assigns_visible_place_before(&link.link, first_visible_place)? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[requires(first_visible_place > 0)]
#[ensures(true)]
pub(super) fn linked_sumti_assigns_visible_place_before(
    link: &LinkedSumtiSyntax,
    first_visible_place: usize,
) -> Result<bool, SemanticsError> {
    match link {
        LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
            Ok(linked_sumti_place(&sumti.fa.value)? < first_visible_place)
        }
        LinkedSumtiSyntax::PlainLinkedSumti(_)
        | LinkedSumtiSyntax::TenseTaggedLinkedSumti(_)
        | LinkedSumtiSyntax::EmptyLinkedSumti(_) => Ok(false),
    }
}

#[requires(first_visible_place > 0)]
#[ensures(ret.as_ref().is_ok_and(|place| *place >= first_visible_place) || ret.is_err())]
pub(super) fn next_visible_place_after_linkargs(
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
#[ensures(true)]
pub(super) fn advance_visible_place_after_linked_sumti(
    next_visible_place: &mut usize,
    link: &LinkedSumtiSyntax,
) -> Result<(), SemanticsError> {
    match link {
        LinkedSumtiSyntax::PlainLinkedSumti(_) => {
            *next_visible_place += 1;
        }
        LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
            let place = linked_sumti_place(&sumti.fa.value)?;
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
    builder: &mut GeneratedGraphBuilder<'_, '_>,
    visible_arguments: &mut BTreeMap<usize, ArgumentValue>,
    unit: Option<&JaiModalTanruUnitSyntax>,
) -> Result<(), SemanticsError> {
    let source = unit.and_then(|unit| builder.source_for_node(unit, "abstraction-about"));
    apply_generated_bare_jai_visible_argument_with_source(builder, visible_arguments, unit, source)
}

#[requires(visible_arguments.keys().all(|place| *place > 0))]
#[ensures(true)]
pub(super) fn apply_generated_bare_jai_visible_argument_with_source(
    builder: &mut GeneratedGraphBuilder<'_, '_>,
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
        _ => Err(unsupported("non-word tanru unit")),
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
        _ => Err(unsupported("non-word jai inner tanru unit")),
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
    if scope == GeneratedScalarNegationScope::MarkerOnly {
        return Ok(scalar_negation_for_marker(&unit.nahe));
    }
    let mut places = BTreeSet::new();
    places.insert(mapped_visible_place_for_generated_scalar_tanru_unit_atom(
        atom, unit, 1,
    )?);
    if let Some(linkargs) = linkargs {
        for place in generated_linkargs_visible_places(linkargs, 2)? {
            places.insert(mapped_visible_place_for_generated_scalar_tanru_unit_atom(
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
            places.insert(mapped_visible_place_for_generated_scalar_tanru_unit_atom(
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
    let place = mapped_place_for_generated_conversions(place, &atom.conversions)?;
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
        &linkargs.first_link,
    )?;
    for link in &linkargs.bei_links {
        add_generated_linked_sumti_visible_places(
            &mut places,
            &mut next_visible_place,
            &link.link,
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
    generated_linked_sumti_provides_scalar_scale_context(&linkargs.first_link)
        || linkargs
            .bei_links
            .iter()
            .any(|link| generated_linked_sumti_provides_scalar_scale_context(&link.link))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_linked_sumti_provides_scalar_scale_context(
    link: &LinkedSumtiSyntax,
) -> bool {
    let LinkedSumtiSyntax::TenseTaggedLinkedSumti(sumti) = link else {
        return false;
    };
    generated_modal_relation_spec_for_tense_modal(sumti.tense_modal.as_ref())
        .is_some_and(|(_, relation, _)| matches!(relation.as_str(), "ci'u" | "ci'e" | "le'a"))
}

#[requires(*next_visible_place > 0)]
#[requires(places.iter().all(|place| *place > 0))]
#[ensures(*next_visible_place > 0)]
#[ensures(places.iter().all(|place| *place > 0))]
pub(super) fn add_generated_linked_sumti_visible_places(
    places: &mut BTreeSet<usize>,
    next_visible_place: &mut usize,
    link: &LinkedSumtiSyntax,
) -> Result<(), SemanticsError> {
    match link {
        LinkedSumtiSyntax::PlainLinkedSumti(_) => {
            places.insert(*next_visible_place);
            *next_visible_place += 1;
        }
        LinkedSumtiSyntax::PlaceTaggedLinkedSumti(sumti) => {
            let place = linked_sumti_place(&sumti.fa.value)?;
            places.insert(place);
            *next_visible_place = (*next_visible_place).max(place + 1);
        }
        LinkedSumtiSyntax::TenseTaggedLinkedSumti(_) => {}
        LinkedSumtiSyntax::EmptyLinkedSumti(_) => {
            return Err(unsupported("empty linked sumti"));
        }
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
        ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(_) => {
            Err(unsupported("tagged scalar-negated tanru unit"))
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
        return Err(unsupported("empty ordinal tanru unit"));
    }
    let moi = token_text(
        visitor
            .tokens
            .last()
            .expect("checked above that ordinal has tokens"),
    );
    let expression = token_list_text(visitor.tokens[..visitor.tokens.len() - 1].iter());
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
