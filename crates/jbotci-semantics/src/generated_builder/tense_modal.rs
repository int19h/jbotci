use super::*;

/// Grammar-directed validation that prevents project-specific experimental BAI
/// from reaching canonical semantic lowering. The morphology inventory contains
/// more BAI spellings than the complete 65-member CLL 9.17 table; their
/// predicate mappings are not standardized and must not be guessed.
#[invariant(
    unsupported_marker
        .as_ref()
        .is_none_or(|marker| !marker.is_empty()),
    "captured unsupported BAI markers must be non-empty"
)]
#[derive(Clone, Default)]
struct GeneratedBaiSupportValidator {
    unsupported_marker: Option<String>,
}

impl<'tree> TreeWalker<'tree> for GeneratedBaiSupportValidator {
    #[requires(true)]
    #[ensures(true)]
    fn walk_modal_tense(&mut self, node: &'tree ModalTenseSyntax) {
        // Some project BAI spellings (for example `xei'e`) lower to typed
        // tense/space coordinates rather than to `Adjunct`. They do not
        // need a CLL 9.17 predicate mapping and must retain that established
        // interpretation. Reject only an unsupported BAI that would otherwise
        // reach the generic relation-argument path.
        if self.unsupported_marker.is_none() && !generated_tense_modal_has_event_modifier(node) {
            let mut collector = GeneratedSpanCollector::default();
            node.visit_in_order(&mut collector);
            let marker = collector
                .tokens
                .iter()
                .find(|token| token.is_selmaho(Selmaho::Bai))
                .map(|token| token_text(token));
            if let Some(marker) = marker {
                if modal_relation_for_marker(&marker).is_none() {
                    *self = self.clone().with_data(data! {
                        unsupported_marker: Some(marker)
                    });
                }
            }
        }
        jbotci_syntax::generated_model::walk::modal_tense(self, node);
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn validate_supported_bai_modal_markers(
    syntax: &TextSyntax,
) -> Result<(), SemanticsError> {
    let mut validator = GeneratedBaiSupportValidator::default();
    TreeWalkable::walk_with(syntax, &mut validator);
    if let Some(marker) = validator.into_data().unsupported_marker {
        return Err(invalid_graph(format!(
            "BAI `{marker}` has no standardized predicate mapping in the complete CLL 9.17 table"
        )));
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tagged_sumti_term_has_event_modifier(term: &TaggedSumtiTermSyntax) -> bool {
    generated_tense_modal_has_event_modifier(term.tense_modal.as_ref())
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Fiho) || !generated_node_contains_cmavo(selbri, Cmavo::Fiho)))]
pub(super) fn generated_fiho_tense_selbri(
    tense_modal: &LeadingTermTagTenseModalSyntax,
) -> Option<&SelbriSyntax> {
    match tense_modal {
        LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) => {
            generated_fiho_tense_modal_selbri(tense_modal)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Fiho) || !generated_node_contains_cmavo(selbri, Cmavo::Fiho)))]
pub(super) fn generated_fiho_tense_modal_selbri(
    tense_modal: &TenseModalSyntax,
) -> Option<&SelbriSyntax> {
    match &tense_modal.0 {
        TenseModalBodySyntax::TenseModalAtom(TenseModalAtomSyntax::FihoTense(fiho)) => {
            Some(fiho.selbri.as_ref())
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_has_event_modifier<N: TreeNode>(tense_modal: &N) -> bool {
    generated_tense_modal_anchors_to_speech_time(tense_modal)
        || generated_anchor_relation_for_tense_modal(tense_modal).is_some()
        || generated_tense_question_token_for_tense_modal(tense_modal).is_some()
        || generated_actuality_for_tense_modal(tense_modal).is_some()
        || generated_time_interval_for_tense_modal(tense_modal, None).is_some()
        || generated_space_interval_for_tense_modal(tense_modal, None).is_some()
        || generated_node_contains_recurrence_marker(tense_modal)
        || !generated_temporal_aspect_contours_for_tense_modal(tense_modal).is_empty()
        || !generated_spatial_aspect_contours_for_tense_modal(tense_modal).is_empty()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_is_experimental_fa_tag(tense_modal: &TenseModalSyntax) -> bool {
    let TenseModalSyntax(TenseModalBodySyntax::TenseModalAtom(
        TenseModalAtomSyntax::ExpTagAtomRun(run),
    )) = tense_modal
    else {
        return false;
    };
    generated_single_fa_exp_tag_prefixes(run).is_some_and(|(nahe, se)| !nahe && !se)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_single_fa_exp_tag_prefixes(
    run: &ExpTagAtomRunSyntax,
) -> Option<(bool, bool)> {
    let ExpTagAtomRunSyntax(body) = run;
    let body = body.as_ref();
    if !body.additional.is_empty() {
        return None;
    }
    let first = body.first.as_ref();
    matches!(first.atom.value.as_ref(), ExpTagAtomSyntax::ExpFaTagAtom(_))
        .then_some((first.nahe.is_some(), first.se.is_some()))
}

#[requires(true)]
#[ensures(ret -> generated_tense_modal_has_event_modifier(tense_modal))]
pub(super) fn generated_tense_modal_event_modifier_allocates_objects<N: TreeNode>(
    tense_modal: &N,
) -> bool {
    generated_tense_question_token_for_tense_modal(tense_modal).is_some()
        || generated_node_contains_recurrence_marker(tense_modal)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| token.is_cmavo(Cmavo::Cuhe)))]
pub(super) fn generated_tense_question_token_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<Token> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .into_iter()
        .find(|token| token.is_cmavo(Cmavo::Cuhe))
        .cloned()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_has_story_time_temporal_modifier<N: TreeNode>(
    tense_modal: &N,
) -> bool {
    generated_tense_modal_anchors_to_speech_time(tense_modal)
        || generated_anchor_relations_with_introducers_for_tense_modal(tense_modal)
            .iter()
            .any(|(domain, _, _)| *domain == GeneratedAnchorDomain::Time)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_anchors_to_speech_time<N: TreeNode>(tense_modal: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Nau))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|connection| connection.as_ref().is_none_or(|(_, _, spec)| spec.terms.len() >= 2)) || ret.is_err())]
pub(super) fn generated_logical_modal_connection_assignment_in_terms<'syntax>(
    terms: &[&'syntax TermSyntax],
) -> Result<
    Option<(
        usize,
        &'syntax TaggedSumtiTermSyntax,
        GeneratedLogicalModalConnectionSpec<'syntax>,
    )>,
    SemanticsError,
> {
    let mut connection = None;
    for (index, term) in terms.iter().enumerate() {
        let Ok(GeneratedSimpleTermRef::TaggedSumtiTerm(term)) =
            generated_simple_term_for_assignment(term)
        else {
            continue;
        };
        let LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) = term.tense_modal.as_ref()
        else {
            continue;
        };
        let Some(spec) = generated_logical_modal_connection_spec_for_tense_modal(tense_modal)?
        else {
            continue;
        };
        if connection.is_some() {
            return Ok(None);
        }
        connection = Some((index, term, spec));
    }
    Ok(connection)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, _, spec)| spec.branches.len() >= 2))]
pub(super) fn generated_logical_event_tense_connection_assignment_in_terms<'syntax>(
    terms: &[&'syntax TermSyntax],
) -> Option<(
    usize,
    &'syntax TaggedSumtiTermSyntax,
    GeneratedConnectedEventTenseSpec,
)> {
    let mut connection = None;
    for (index, term) in terms.iter().enumerate() {
        let Ok(GeneratedSimpleTermRef::TaggedSumtiTerm(term)) =
            generated_simple_term_for_assignment(term)
        else {
            continue;
        };
        let LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) = term.tense_modal.as_ref()
        else {
            continue;
        };
        let Some(spec) = generated_connected_event_tense_spec_for_term_connection(tense_modal)
        else {
            continue;
        };
        if connection.is_some() {
            return None;
        }
        connection = Some((index, term, spec));
    }
    connection
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|spec| spec.as_ref().is_none_or(|spec| spec.terms.len() >= 2)) || ret.is_err())]
pub(super) fn generated_logical_modal_connection_spec_for_tense_modal<'syntax>(
    tense_modal: &'syntax TenseModalSyntax,
) -> Result<Option<GeneratedLogicalModalConnectionSpec<'syntax>>, SemanticsError> {
    // CLL 10.23 keeps modal and tense connection grammar parallel, but their
    // event-role order differs, so modal connection lowering stays separate.
    let TenseModalBodySyntax::ConnectedTenseModal(connected) = &tense_modal.0 else {
        return Ok(None);
    };
    let [continuation] = connected.continuations.as_slice() else {
        return Ok(None);
    };
    if generated_connected_event_tense_connective_question_token(&continuation.connective).is_some()
    {
        return Ok(None);
    }
    let Some(operator) =
        generated_connected_event_tense_connective_formula_operator(&continuation.connective)
    else {
        return Ok(None);
    };
    let mut terms = Vec::with_capacity(2);
    let Some(mut first) = generated_connected_modal_term_from_atom(connected.first.as_ref()) else {
        return Ok(None);
    };
    if generated_connected_event_tense_connective_negates_left(&continuation.connective) {
        first = first.with_data(data! { negated: true });
    }
    terms.push(first);
    let Some(mut second) =
        generated_connected_modal_term_from_atom(continuation.tense_modal.as_ref())
    else {
        return Ok(None);
    };
    if generated_connected_event_tense_connective_negates_right(&continuation.connective) {
        second = second.with_data(data! { negated: true });
    }
    terms.push(second);
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    Ok(Some(new!(GeneratedLogicalModalConnectionSpec {
        operator,
        source: token_list_text(collector.tokens.iter().copied()),
        truth_table: (operator != FormulaOperator::RespectivelyDistribution)
            .then(|| generated_truth_table_for_formula_operator(operator)),
        terms,
    })))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|term| generated_tense_modal_has_adjunct(&term.tense_modal)))]
pub(super) fn generated_connected_modal_term_from_atom<'syntax>(
    atom: &'syntax TenseModalAtomSyntax,
) -> Option<GeneratedConnectedModalTerm<'syntax>> {
    let tense_modal = TenseModalSyntax(TenseModalBodySyntax::TenseModalAtom(atom.clone()));
    if generated_tense_modal_has_event_modifier(&tense_modal) {
        return None;
    }
    let kind = if let TenseModalAtomSyntax::FihoTense(fiho) = atom {
        new!(GeneratedConnectedModalTermKind::AdHoc { fiho })
    } else {
        let (introduced_by, relation, visible_place) =
            generated_modal_relation_spec_for_tense_modal(&tense_modal)?;
        new!(GeneratedConnectedModalTermKind::Named {
            introduced_by,
            relation,
            visible_place,
        })
    };
    Some(new!(GeneratedConnectedModalTerm {
        tense_modal,
        kind,
        negated: false,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| spec.branches.len() >= 2))]
pub(super) fn generated_connected_event_tense_spec_for_tense_modal(
    tense_modal: &TenseModalSyntax,
) -> Option<GeneratedConnectedEventTenseSpec> {
    generated_connected_event_tense_spec(tense_modal, false)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| spec.branches.len() >= 2))]
pub(super) fn generated_connected_event_tense_spec_for_term_connection(
    tense_modal: &TenseModalSyntax,
) -> Option<GeneratedConnectedEventTenseSpec> {
    generated_connected_event_tense_spec(tense_modal, true)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|spec| spec.branches.len() >= 2))]
fn generated_connected_event_tense_spec(
    tense_modal: &TenseModalSyntax,
    allow_respectively: bool,
) -> Option<GeneratedConnectedEventTenseSpec> {
    let TenseModalBodySyntax::ConnectedTenseModal(connected) = &tense_modal.0 else {
        return None;
    };
    let mut operator = None::<FormulaOperator>;
    let mut connector_question = None::<Token>;
    let mut branches = Vec::with_capacity(connected.continuations.len() + 1);
    branches.push(generated_connected_event_tense_branch_from_atom(
        connected.first.as_ref(),
    )?);
    for continuation in &connected.continuations {
        if generated_connected_event_tense_connective_negates_left(&continuation.connective)
            && let Some(branch) = branches.last_mut()
        {
            *branch = branch.clone().with_data(data! { negated: true });
        }
        let next_branch_negated =
            generated_connected_event_tense_connective_negates_right(&continuation.connective);
        if let Some(token) =
            generated_connected_event_tense_connective_question_token(&continuation.connective)
        {
            if operator.is_some() || connector_question.is_some() {
                return None;
            }
            operator = Some(FormulaOperator::ConnectiveQuestion);
            connector_question = Some(token);
        } else {
            if connector_question.is_some() {
                return None;
            }
            let next_operator = if allow_respectively {
                generated_connected_event_tense_connective_formula_operator_for_term_connection(
                    &continuation.connective,
                )?
            } else {
                generated_connected_event_tense_connective_formula_operator(
                    &continuation.connective,
                )?
            };
            if let Some(operator) = operator
                && operator != next_operator
            {
                return None;
            }
            operator = Some(next_operator);
        }
        let mut next_branch =
            generated_connected_event_tense_branch_from_atom(continuation.tense_modal.as_ref())?;
        if next_branch_negated {
            next_branch = next_branch.with_data(data! { negated: true });
        }
        branches.push(next_branch);
    }
    let operator = operator?;
    let truth_table = (connector_question.is_none()
        && operator != FormulaOperator::RespectivelyDistribution)
        .then(|| generated_truth_table_for_formula_operator(operator));
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    Some(GeneratedConnectedEventTenseSpec::from_data(data!(
        GeneratedConnectedEventTenseSpec {
            operator,
            source: token_list_text(collector.tokens.iter().copied()),
            truth_table,
            connector_question,
            branches,
        }
    )))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|branch| generated_tense_modal_has_event_modifier(&branch.tense_modal)))]
pub(super) fn generated_connected_event_tense_branch_from_atom(
    atom: &TenseModalAtomSyntax,
) -> Option<GeneratedConnectedEventTenseBranch> {
    let tense_modal = TenseModalSyntax(TenseModalBodySyntax::TenseModalAtom(atom.clone()));
    generated_tense_modal_has_event_modifier(&tense_modal).then(|| {
        GeneratedConnectedEventTenseBranch::from_data(data!(GeneratedConnectedEventTenseBranch {
            negated: generated_adjunct_negation_for_tense_modal(&tense_modal).is_some(),
            tense_modal,
        }))
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|token| token.is_cmavo(Cmavo::Jehi)))]
pub(super) fn generated_connected_event_tense_connective_question_token<N: TreeNode>(
    connective: &N,
) -> Option<Token> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .into_iter()
        .find(|token| token.is_cmavo(Cmavo::Jehi))
        .cloned()
}

#[requires(true)]
#[ensures(ret.is_none_or(|operator| matches!(operator, FormulaOperator::And | FormulaOperator::Or | FormulaOperator::Iff | FormulaOperator::WhetherOrNot)))]
pub(super) fn generated_connected_event_tense_connective_formula_operator<N: TreeNode>(
    connective: &N,
) -> Option<FormulaOperator> {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .into_iter()
        .find_map(|token| match token.cmavo() {
            Some(Cmavo::Ja) => Some(FormulaOperator::Or),
            Some(Cmavo::Je) => Some(FormulaOperator::And),
            Some(Cmavo::Jo) => Some(FormulaOperator::Iff),
            Some(Cmavo::Ju) => Some(FormulaOperator::WhetherOrNot),
            _ => None,
        })
}

#[requires(true)]
#[ensures(ret.is_none_or(|operator| matches!(operator, FormulaOperator::And | FormulaOperator::Or | FormulaOperator::Iff | FormulaOperator::WhetherOrNot | FormulaOperator::RespectivelyDistribution)))]
pub(super) fn generated_connected_event_tense_connective_formula_operator_for_term_connection<
    N: TreeNode,
>(
    connective: &N,
) -> Option<FormulaOperator> {
    generated_connected_event_tense_connective_formula_operator(connective).or_else(|| {
        let mut collector = GeneratedSpanCollector::default();
        connective.visit_in_order(&mut collector);
        collector
            .tokens
            .into_iter()
            .any(|token| token.cmavo() == Some(Cmavo::Fahu))
            .then_some(FormulaOperator::RespectivelyDistribution)
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_connected_event_tense_connective_negates_left<N: TreeNode>(
    connective: &N,
) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Na))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_connected_event_tense_connective_negates_right<N: TreeNode>(
    connective: &N,
) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    connective.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| token.cmavo() == Some(Cmavo::Nai))
}

#[requires(true)]
#[ensures(!generated_tense_modal_has_event_modifier(tense_modal) -> !ret)]
pub(super) fn generated_tense_modal_has_contradictory_event_negation(
    tense_modal: &TenseModalSyntax,
) -> bool {
    generated_adjunct_negation_for_tense_modal(tense_modal).is_some()
        && !matches!(
            &tense_modal.0,
            TenseModalBodySyntax::TenseModalAtom(
                TenseModalAtomSyntax::ModalTense(_) | TenseModalAtomSyntax::FihoTense(_)
            ) | TenseModalBodySyntax::ConnectedTenseModal(_)
        )
        && generated_tense_modal_has_event_modifier(tense_modal)
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_bridi(
    bridi: &BridiSyntax,
) -> Option<&TenseModalSyntax> {
    let leading = match bridi {
        BridiSyntax::BridiWithLeadingTerms(bridi) => bridi
            .leading_terms
            .iter()
            .find_map(first_generated_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                simple_tail_from_bridi_tail(&bridi.bridi_tail)
                    .ok()
                    .and_then(|tail| {
                        tail.terms
                            .iter()
                            .find_map(first_generated_contradictory_event_tense_modal_for_term)
                    })
            }),
        BridiSyntax::BridiWithPostCuTerms(bridi) => bridi
            .leading_terms
            .iter()
            .find_map(first_generated_contradictory_event_tense_modal_for_term)
            .or_else(|| {
                bridi
                    .bridi_tail
                    .terms
                    .iter()
                    .find_map(first_generated_contradictory_event_tense_modal_for_term)
            }),
        BridiSyntax::BareCuTermsBridi(bridi) => bridi
            .bridi_tail
            .terms
            .iter()
            .find_map(first_generated_contradictory_event_tense_modal_for_term),
        BridiSyntax::BareCuBridi(_) | BridiSyntax::RelationOnlyBridi(_) => None,
    };
    leading.or_else(|| {
        main_generated_selbri_for_bridi(bridi)
            .and_then(first_generated_contradictory_event_tense_modal_for_selbri)
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_selbri(
    selbri: &SelbriSyntax,
) -> Option<&TenseModalSyntax> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => {
            if generated_tense_modal_has_contradictory_event_negation(&tagged.tense_modal) {
                Some(tagged.tense_modal.as_ref())
            } else {
                first_generated_contradictory_event_tense_modal_for_untagged_selbri(
                    tagged.inner_selbri.as_ref(),
                )
            }
        }
        SelbriSyntax::UntaggedSelbri(untagged) => {
            first_generated_contradictory_event_tense_modal_for_untagged_selbri(untagged)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_untagged_selbri(
    selbri: &UntaggedSelbriSyntax,
) -> Option<&TenseModalSyntax> {
    match selbri {
        UntaggedSelbriSyntax::NegatedSelbri(negated) => {
            first_generated_contradictory_event_tense_modal_for_selbri(&negated.inner_selbri)
        }
        UntaggedSelbriSyntax::CoSelbri(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_co_selbri(
    _selbri: &CoSelbriSyntax,
) -> Option<&TenseModalSyntax> {
    None
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_tanru_selbri(
    _selbri: &TanruSelbriSyntax,
) -> Option<&TenseModalSyntax> {
    None
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_tanru_unit(
    _unit: &TanruUnitSyntax,
) -> Option<&TenseModalSyntax> {
    None
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_linked_sumti(
    _link: &LinkedSumtiSyntax,
) -> Option<&TenseModalSyntax> {
    None
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_term(
    term: &TermSyntax,
) -> Option<&TenseModalSyntax> {
    match generated_simple_term_for_assignment(term).ok()? {
        GeneratedSimpleTermRef::TaggedSumtiTerm(term) => {
            let tense_modal = match term.tense_modal.as_ref() {
                LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) => Some(tense_modal),
                _ => None,
            };
            tense_modal.filter(|tense_modal| {
                generated_tense_modal_has_contradictory_event_negation(tense_modal)
            })
        }
        GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(term) => {
            let tense_modal = match term.0.as_ref() {
                LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) => Some(tense_modal),
                _ => None,
            };
            tense_modal.filter(|tense_modal| {
                generated_tense_modal_has_contradictory_event_negation(tense_modal)
            })
        }
        GeneratedSimpleTermRef::JaiTaggedSumtiTerm(term) => {
            let Some(tag) = &term.tag else {
                return None;
            };
            match tag.as_ref() {
                tense_modal
                    if generated_tense_modal_has_contradictory_event_negation(tense_modal) =>
                {
                    Some(tense_modal)
                }
                _ => None,
            }
        }
        GeneratedSimpleTermRef::SumtiTerm(SumtiTermSyntax(sumti)) => {
            first_generated_contradictory_event_tense_modal_for_sumti(sumti)
        }
        GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => match term.sumti.as_ref() {
            TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                first_generated_contradictory_event_tense_modal_for_sumti(sumti)
            }
            TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => None,
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(generated_tense_modal_has_contradictory_event_negation))]
pub(super) fn first_generated_contradictory_event_tense_modal_for_sumti(
    sumti: &SumtiSyntax,
) -> Option<&TenseModalSyntax> {
    let grouped = sumti.base_sumti.as_ref();
    grouped.grouped_tail.as_ref().and_then(|tail| {
        tail.tense_modal.as_deref().filter(|tense_modal| {
            generated_tense_modal_has_contradictory_event_negation(tense_modal)
        })
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_makes_modal_sticky<N: TreeNode>(tense_modal: &N) -> bool {
    // CLL 9.14 and 10.13 both use KI for stickiness, but modal stickiness
    // replays tagged arguments while tense stickiness shifts the event origin.
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut saw_modal = false;
    let mut saw_ki = false;
    for token in collector.tokens {
        saw_modal |= token.is_selmaho(Selmaho::Bai);
        saw_ki |= token.cmavo() == Some(Cmavo::Ki);
    }
    saw_modal && saw_ki
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_makes_tense_sticky<N: TreeNode>(tense_modal: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut saw_time_relation = false;
    let mut saw_ki = false;
    for token in collector.tokens {
        saw_time_relation |= time_relation_for_pu_token(&token).is_some();
        saw_ki |= token.cmavo() == Some(Cmavo::Ki);
    }
    saw_time_relation && saw_ki
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_makes_space_sticky<N: TreeNode>(tense_modal: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut saw_space_relation = false;
    let mut saw_ki = false;
    let mut interval_accepts_direction = false;
    for token in collector.tokens {
        if token.cmavo() == Some(Cmavo::Ki) {
            saw_ki = true;
            interval_accepts_direction = false;
            continue;
        }
        if space_interval_part_accepts_direction(&token) {
            interval_accepts_direction = true;
            continue;
        }
        if interval_accepts_direction && space_interval_direction_for_faha_token(&token).is_some() {
            interval_accepts_direction = false;
            continue;
        }
        interval_accepts_direction = false;
        saw_space_relation |= space_relation_for_faha_token(&token).is_some()
            || space_distance_for_va_token(&token).is_some();
    }
    saw_space_relation && saw_ki
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_resets_sticky_tense<N: TreeNode>(tense_modal: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector.tokens.len() == 1 && collector.tokens[0].cmavo() == Some(Cmavo::Ki)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_tense_modal_resets_sticky_modals(tense_modal: &TenseModalSyntax) -> bool {
    matches!(
        &tense_modal.0,
        TenseModalBodySyntax::TenseModalAtom(TenseModalAtomSyntax::StickyTense(_))
    )
}

#[requires(true)]
#[ensures(generated_tense_modal_has_event_modifier(tense_modal) -> !ret)]
pub(super) fn generated_tense_modal_has_adjunct(tense_modal: &TenseModalSyntax) -> bool {
    if generated_tense_modal_has_event_modifier(tense_modal) {
        return false;
    }
    generated_fiho_tense_modal_selbri(tense_modal).is_some()
        || generated_modal_relation_spec_for_tense_modal(tense_modal).is_some()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_node_contains_recurrence_marker<N: TreeNode>(node: &N) -> bool {
    let mut collector = GeneratedSpanCollector::default();
    node.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .any(|token| generated_recurrence_kind_for_interval_marker(token).is_some())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_recurrence_kind_for_interval_marker(
    token: &Token,
) -> Option<RecurrenceKind> {
    match token.cmavo() {
        Some(Cmavo::Roi) => Some(RecurrenceKind::OccurrenceCount),
        Some(Cmavo::Rehu) => Some(RecurrenceKind::OrdinalOccurrence),
        Some(Cmavo::Dihi) => Some(RecurrenceKind::Regular),
        Some(Cmavo::Naho) => Some(RecurrenceKind::Typically),
        Some(Cmavo::Ruhi) => Some(RecurrenceKind::Continuously),
        Some(Cmavo::Tahe) => Some(RecurrenceKind::Habitually),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_recurrence_event_modifier(
    modifiers: &mut GeneratedRecurrenceEventModifiers,
    recurrence: Recurrence,
    spatial: bool,
) -> (bool, usize, usize) {
    if spatial {
        modifiers.spatial_recurrences.push(recurrence);
        let recurrence_index = modifiers.spatial_recurrences.len() - 1;
        modifiers
            .spatial_interval_modifiers
            .push(new!(IntervalModifier::Recurrence(
                modifiers.spatial_recurrences[recurrence_index].clone(),
            )));
        let modifier_index = modifiers.spatial_interval_modifiers.len() - 1;
        (true, recurrence_index, modifier_index)
    } else {
        modifiers.temporal_recurrences.push(recurrence);
        let recurrence_index = modifiers.temporal_recurrences.len() - 1;
        modifiers
            .temporal_interval_modifiers
            .push(new!(IntervalModifier::Recurrence(
                modifiers.temporal_recurrences[recurrence_index].clone(),
            )));
        let modifier_index = modifiers.temporal_interval_modifiers.len() - 1;
        (false, recurrence_index, modifier_index)
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn apply_generated_recurrence_negation(
    modifiers: &mut GeneratedRecurrenceEventModifiers,
    spatial: bool,
    recurrence_index: usize,
    modifier_index: usize,
    negation: TaggedNegation,
) {
    let recurrence = if spatial {
        modifiers.spatial_recurrences.get_mut(recurrence_index)
    } else {
        modifiers.temporal_recurrences.get_mut(recurrence_index)
    };
    let Some(recurrence) = recurrence else {
        return;
    };
    *recurrence = recurrence.clone().with_data(data! {
        negation: Some(negation),
    });
    let modifier = if spatial {
        modifiers.spatial_interval_modifiers.get_mut(modifier_index)
    } else {
        modifiers
            .temporal_interval_modifiers
            .get_mut(modifier_index)
    };
    if let Some(modifier) = modifier {
        *modifier = new!(IntervalModifier::Recurrence(recurrence.clone()));
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn apply_generated_aspects_to_event(
    event: &mut SemanticObject,
    aspects: Vec<Aspect>,
    spatial: bool,
) {
    event.apply_eventuality_aspects(aspects, spatial);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn attach_generated_magnitude_to_event_modifier<N: TreeNode>(
    event: &mut SemanticObject,
    tense_modal: &N,
    magnitude: AnchorMagnitude,
) {
    let relations = generated_anchor_relations_with_introducers_for_tense_modal(tense_modal);
    if relations
        .iter()
        .any(|(domain, _, _)| *domain == GeneratedAnchorDomain::Space)
    {
        event.attach_eventuality_magnitude(magnitude, true);
        return;
    }
    if relations
        .iter()
        .any(|(domain, _, _)| *domain == GeneratedAnchorDomain::Time)
    {
        event.attach_eventuality_magnitude(magnitude, false);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_actuality_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<Actuality> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .copied()
        .find_map(generated_actuality_for_caha_token)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_actuality_for_caha_token(token: &Token) -> Option<Actuality> {
    let kind = match token.cmavo() {
        Some(Cmavo::Caha) => ActualityKind::Actual,
        Some(Cmavo::Kahe) => ActualityKind::Capable,
        Some(Cmavo::Nuho) => ActualityKind::Potential,
        Some(Cmavo::Puhi) => ActualityKind::Demonstrated,
        _ => return None,
    };
    Some(Actuality { kind })
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|interval| !interval.extent.is_empty()))]
pub(super) fn generated_time_interval_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
    anchor: Option<SemanticObjectId>,
) -> Option<TimeInterval> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector
        .tokens
        .iter()
        .copied()
        .find_map(time_interval_extent_for_zeha_token)
        .map(|extent| TimeInterval::new(extent, anchor))
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|interval| interval.extent.is_some() || !interval.directions.is_empty() || !interval.dimensions.is_empty()))]
pub(super) fn generated_space_interval_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
    anchor: Option<SemanticObjectId>,
) -> Option<SpaceInterval> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    space_interval_for_generated_tokens(collector.tokens.iter().copied(), anchor)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_temporal_aspect_contours_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Vec<String> {
    generated_scoped_aspect_contours_for_tense_modal(tense_modal).0
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_spatial_aspect_contours_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Vec<String> {
    generated_scoped_aspect_contours_for_tense_modal(tense_modal).1
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_scoped_aspect_contours_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> (Vec<String>, Vec<String>) {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut temporal = Vec::new();
    let mut spatial = Vec::new();
    let mut next_interval_property_is_spatial = false;
    for token in &collector.tokens {
        if token.is_cmavo(Cmavo::Fehe) {
            next_interval_property_is_spatial = true;
            continue;
        }
        if let Some(contour) = aspect_contour_for_zaho_token(token) {
            if next_interval_property_is_spatial {
                spatial.push(contour);
            } else {
                temporal.push(contour);
            }
            next_interval_property_is_spatial = false;
            continue;
        }
        if !token.is_selmaho(Selmaho::Pa) {
            next_interval_property_is_spatial = false;
        }
    }
    (temporal, spatial)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|contour| !contour.is_empty()))]
pub(super) fn aspect_contour_for_zaho_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Puho) => Some("prospective".to_owned()),
        Some(Cmavo::Caho) => Some("continuative".to_owned()),
        Some(Cmavo::Baho) => Some("retrospective".to_owned()),
        Some(Cmavo::Coha) => Some("initiative".to_owned()),
        Some(Cmavo::Cohu) => Some("cessative".to_owned()),
        Some(Cmavo::Mohu) => Some("completive".to_owned()),
        Some(Cmavo::Zaho) => Some("superfective".to_owned()),
        Some(Cmavo::Cohi) => Some("achievative".to_owned()),
        Some(Cmavo::Deha) => Some("pausative".to_owned()),
        Some(Cmavo::Diha) => Some("resumptive".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
pub(super) fn generated_modal_relation_spec_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(String, String, usize)> {
    // CLL 9.5-9.6/9.17 make the modal sumti fill the selected relation
    // role; SE conversion chooses which underlying BAI place is selected.
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    for (index, token) in collector.tokens.iter().enumerate() {
        if !token.is_selmaho(Selmaho::Bai) {
            continue;
        }
        let marker = token_text(token);
        let conversion = index
            .checked_sub(1)
            .and_then(|previous| collector.tokens.get(previous))
            .copied()
            .filter(|previous| previous.is_selmaho(Selmaho::Se));
        let visible_place = conversion
            .and_then(generated_se_token_conversion_place)
            .unwrap_or(1);
        let introduced_by = conversion
            .map(|conversion| format!("{} {marker}", token_text(conversion)))
            .unwrap_or_else(|| marker.clone());
        let relation = modal_relation_for_marker(&marker)?;
        return Some((introduced_by, relation.to_owned(), visible_place));
    }
    generated_tense_relation_spec_for_tokens(&collector.tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
pub(super) fn generated_tense_relation_spec_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<(String, String, usize)> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    generated_tense_relation_spec_for_tokens(&collector.tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(introduced_by, relation, visible_place)| !introduced_by.is_empty() && !relation.is_empty() && *visible_place > 0))]
pub(super) fn generated_tense_relation_spec_for_tokens(
    tokens: &[&Token],
) -> Option<(String, String, usize)> {
    for token in tokens {
        if let Some(relation) = time_relation_for_pu_token(token)
            .or_else(|| space_relation_for_faha_token(token))
            .or_else(|| space_distance_for_va_token(token).map(|_| "distanceFrom".to_owned()))
        {
            return Some((token_text(token), relation, 1));
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|extent| !extent.is_empty()))]
pub(super) fn time_interval_extent_for_zeha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zehi) => Some("short".to_owned()),
        Some(Cmavo::Zeha) => Some("medium".to_owned()),
        Some(Cmavo::Zehu) => Some("long".to_owned()),
        Some(Cmavo::Zehe) => Some("whole".to_owned()),
        _ => None,
    }
}

#[requires(anchor.is_none_or(|anchor| crate::model::argument_object_kind_can_fill(anchor.object_kind())))]
#[ensures(ret.as_ref().is_none_or(|interval| interval.extent.is_some() || !interval.directions.is_empty() || !interval.dimensions.is_empty()))]
pub(super) fn space_interval_for_generated_tokens<'a>(
    tokens: impl Iterator<Item = &'a Token>,
    anchor: Option<SemanticObjectId>,
) -> Option<SpaceInterval> {
    let mut extent = None;
    let mut directions = Vec::new();
    let mut dimensions = Vec::new();
    let mut accepts_direction = false;
    for token in tokens {
        if extent.is_none()
            && let Some(interval_extent) = space_interval_extent_for_veha_token(token)
        {
            extent = Some(interval_extent);
            accepts_direction = true;
            continue;
        }
        if let Some(dimension) = space_interval_dimension_for_viha_token(token) {
            dimensions.push(dimension);
            accepts_direction = true;
            continue;
        }
        if accepts_direction && let Some(direction) = space_interval_direction_for_faha_token(token)
        {
            directions.push(direction);
            accepts_direction = false;
            continue;
        }
        accepts_direction = space_interval_part_accepts_direction(token);
    }
    (extent.is_some() || !directions.is_empty() || !dimensions.is_empty())
        .then(|| SpaceInterval::new(extent, directions, dimensions, anchor))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|extent| !extent.is_empty()))]
pub(super) fn space_interval_extent_for_veha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vehi) => Some("short".to_owned()),
        Some(Cmavo::Veha) => Some("medium".to_owned()),
        Some(Cmavo::Vehu) => Some("long".to_owned()),
        Some(Cmavo::Vehe) => Some("whole".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|dimension| !dimension.is_empty()))]
pub(super) fn space_interval_dimension_for_viha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vihi) => Some("line".to_owned()),
        Some(Cmavo::Viha) => Some("area".to_owned()),
        Some(Cmavo::Vihu) => Some("volume".to_owned()),
        Some(Cmavo::Vihe) => Some("spaceTime".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|direction| !direction.is_empty()))]
pub(super) fn space_interval_direction_for_faha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Beha) => Some("north".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret == (space_interval_extent_for_veha_token(token).is_some() || space_interval_dimension_for_viha_token(token).is_some()))]
pub(super) fn space_interval_part_accepts_direction(token: &Token) -> bool {
    space_interval_extent_for_veha_token(token).is_some()
        || space_interval_dimension_for_viha_token(token).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
pub(super) fn time_relation_for_pu_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Pu) => Some("before".to_owned()),
        Some(Cmavo::Ca) => Some("at".to_owned()),
        Some(Cmavo::Ba) => Some("after".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|distance| !distance.is_empty()))]
pub(super) fn time_distance_for_zi_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zi) => Some("short".to_owned()),
        Some(Cmavo::Za) => Some("medium".to_owned()),
        Some(Cmavo::Zu) => Some("long".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
pub(super) fn time_relation_for_time_distance_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Zi) => Some("near".to_owned()),
        Some(Cmavo::Za) => Some("mediumDistance".to_owned()),
        Some(Cmavo::Zu) => Some("far".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
pub(super) fn space_distance_for_va_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Vi) => Some("short".to_owned()),
        Some(Cmavo::Va) => Some("medium".to_owned()),
        Some(Cmavo::Vu) => Some("long".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|relation| !relation.is_empty()))]
pub(super) fn space_relation_for_faha_token(token: &Token) -> Option<String> {
    match token.cmavo() {
        Some(Cmavo::Buhu) => Some("coincidentWith".to_owned()),
        Some(Cmavo::Cahu) => Some("inFrontOf".to_owned()),
        Some(Cmavo::Tiha) => Some("behind".to_owned()),
        Some(Cmavo::Zuha) => Some("leftOf".to_owned()),
        Some(Cmavo::Rihu) => Some("rightOf".to_owned()),
        Some(Cmavo::Gahu) => Some("above".to_owned()),
        Some(Cmavo::Niha) => Some("below".to_owned()),
        Some(Cmavo::Nehi) => Some("within".to_owned()),
        Some(Cmavo::Ruhu) => Some("surrounding".to_owned()),
        Some(Cmavo::Paho) => Some("through".to_owned()),
        Some(Cmavo::Neha) => Some("nextTo".to_owned()),
        Some(Cmavo::Tehe) => Some("bordering".to_owned()),
        Some(Cmavo::Reho) => Some("adjacentTo".to_owned()),
        Some(Cmavo::Faha) => Some("toward".to_owned()),
        Some(Cmavo::Toho) => Some("awayFrom".to_owned()),
        Some(Cmavo::Zohi) => Some("inwardFrom".to_owned()),
        Some(Cmavo::Zeho) => Some("outwardFrom".to_owned()),
        Some(Cmavo::Zoha) => Some("tangentialTo".to_owned()),
        Some(Cmavo::Beha) => Some("northOf".to_owned()),
        Some(Cmavo::Nehu) => Some("southOf".to_owned()),
        Some(Cmavo::Duha) => Some("eastOf".to_owned()),
        Some(Cmavo::Vuha) => Some("westOf".to_owned()),
        // The jbovlaste definition makes experimental FAhA3 `xei'e` the
        // online-location counterpart of `bu'u`, not a generic place word.
        Some(Cmavo::Xeihe) => Some("onlineWith".to_owned()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
pub(super) fn generated_se_token_conversion_place(se: &Token) -> Option<usize> {
    match se.cmavo() {
        Some(Cmavo::Se) => Some(2),
        Some(Cmavo::Te) => Some(3),
        Some(Cmavo::Ve) => Some(4),
        Some(Cmavo::Xe) => Some(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
pub(super) fn generated_adjunct_negation_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<TaggedNegation> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    let mut previous_recurrence_marker = false;
    for token in &collector.tokens {
        if token.cmavo() == Some(Cmavo::Nai) {
            if !previous_recurrence_marker {
                return Some(TaggedNegation::new(
                    TaggedNegationKind::Contradictory,
                    token_text(token),
                ));
            }
            previous_recurrence_marker = false;
            continue;
        }
        previous_recurrence_marker = token_is_recurrence_interval_marker(token);
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub(super) fn token_is_recurrence_interval_marker(token: &Token) -> bool {
    matches!(
        token.cmavo(),
        Some(Cmavo::Roi | Cmavo::Rehu | Cmavo::Dihi | Cmavo::Naho | Cmavo::Ruhi | Cmavo::Tahe)
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
pub(super) fn generated_modal_scalar_negation_for_tense_modal<N: TreeNode>(
    tense_modal: &N,
) -> Option<ScalarNegation> {
    let mut collector = GeneratedSpanCollector::default();
    tense_modal.visit_in_order(&mut collector);
    collector.tokens.iter().find_map(|token| {
        matches!(
            token.cmavo(),
            Some(Cmavo::Nahe | Cmavo::Tohe | Cmavo::Nohe | Cmavo::Jeha)
        )
        .then(|| scalar_negation_for_token(token))
    })
}

/// The complete CLL 9.17 BAI table. This intentionally excludes the project's
/// experimental/dialect BAI inventory: those markers have no CLL-defined
/// desugaring and are rejected before semantic lowering.
#[requires(!marker.is_empty())]
#[ensures(ret.is_none_or(|relation| !relation.is_empty()))]
pub(super) fn modal_relation_for_marker(marker: &str) -> Option<&'static str> {
    match marker {
        "ba'i" => Some("basti"),
        "bai" => Some("bapli"),
        "bau" => Some("bangu"),
        "be'i" => Some("benji"),
        "ca'i" => Some("catni"),
        "cau" => Some("claxu"),
        "ci'e" => Some("ciste"),
        "ci'o" => Some("cinmo"),
        "ci'u" => Some("ckilu"),
        "cu'u" => Some("cusku"),
        "de'i" => Some("detri"),
        "di'o" => Some("diklo"),
        "do'e" => Some("unspecified-role"),
        "du'i" => Some("dunli"),
        "du'o" => Some("djuno"),
        "fa'e" => Some("fatne"),
        "fau" => Some("fasnu"),
        "fi'e" => Some("finti"),
        "ga'a" => Some("zgana"),
        "gau" => Some("gasnu"),
        "ja'e" => Some("jalge"),
        "ja'i" => Some("javni"),
        "ji'e" => Some("jimte"),
        "ji'o" => Some("jitro"),
        "ji'u" => Some("jicmu"),
        "ka'a" => Some("klama"),
        "ka'i" => Some("krati"),
        "kai" => Some("ckaji"),
        "ki'i" => Some("ckini"),
        "ki'u" => Some("krinu"),
        "koi" => Some("korbi"),
        "ku'u" => Some("kulnu"),
        "la'u" => Some("klani"),
        "le'a" => Some("klesi"),
        "li'e" => Some("lidne"),
        "ma'e" => Some("marji"),
        "ma'i" => Some("manri"),
        "mau" => Some("zmadu"),
        "me'a" => Some("mleca"),
        "me'e" => Some("cmene"),
        "mu'i" => Some("mukti"),
        "mu'u" => Some("mupli"),
        "ni'i" => Some("nibli"),
        "pa'a" => Some("panra"),
        "pa'u" => Some("pagbu"),
        "pi'o" => Some("pilno"),
        "po'i" => Some("porsi"),
        "pu'a" => Some("pluka"),
        "pu'e" => Some("pruce"),
        "ra'a" => Some("srana"),
        "ra'i" => Some("krasi"),
        "rai" => Some("traji"),
        "ri'a" => Some("rinka"),
        "ri'i" => Some("lifri"),
        "sau" => Some("sarcu"),
        "si'u" => Some("sidju"),
        "ta'i" => Some("tadji"),
        "tai" => Some("tamsmi"),
        "ti'i" => Some("stidi"),
        "ti'u" => Some("tcika"),
        "tu'i" => Some("stuzi"),
        "va'o" => Some("vanbi"),
        "va'u" => Some("xamgu"),
        "zau" => Some("zanru"),
        "zu'e" => Some("zukte"),
        _ => None,
    }
}
