use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_text_plan_from_text(
    syntax: &TextSyntax,
) -> Result<GeneratedTextPlan<'_>, SemanticsError> {
    match syntax {
        TextSyntax::RegularText(RegularTextSyntax {
            leading_nai,
            leading_cmevla,
            leading_indicators,
            leading_free_modifiers,
            leading_connective,
            leading_i_statements,
            paragraphs,
        }) => {
            let mut plan = GeneratedTextPlan {
                leading_nai,
                leading_cmevla,
                leading_indicators,
                leading_free_modifiers: leading_free_modifiers.iter().collect(),
                leading_connective: leading_connective.as_ref(),
                leading_i_statements,
                items: Vec::new(),
            };
            for leading_i in leading_i_statements {
                plan.leading_free_modifiers
                    .extend(leading_i.free_modifiers.iter());
            }
            if let Some(paragraphs) = paragraphs {
                push_generated_text_paragraphs(&mut plan.items, paragraphs)?;
            }
            Ok(plan)
        }
        TextSyntax::ExplicitXauhaLohoiText(ExplicitXauhaLohoiTextSyntax(paragraphs)) => {
            let mut plan = GeneratedTextPlan {
                leading_nai: &[],
                leading_cmevla: &[],
                leading_indicators: &[],
                leading_free_modifiers: Vec::new(),
                leading_connective: None,
                leading_i_statements: &[],
                items: Vec::new(),
            };
            push_generated_text_paragraph_with_additional_niho(&mut plan.items, paragraphs)?;
            Ok(plan)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_text_plan_has_semantic_content(plan: &GeneratedTextPlan<'_>) -> bool {
    !plan.leading_nai.is_empty()
        || !plan.leading_cmevla.is_empty()
        || !plan.leading_indicators.is_empty()
        || !plan.leading_free_modifiers.is_empty()
        || plan.leading_connective.is_some()
        || plan
            .leading_i_statements
            .iter()
            .any(|statement| statement.connective.is_some())
        || !plan.items.is_empty()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn free_modifiers_have_generated_vocative(
    free_modifiers: &[FreeModifierSyntax],
) -> bool {
    free_modifiers
        .iter()
        .any(|free_modifier| matches!(free_modifier, FreeModifierSyntax::VocativeFreeModifier(_)))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn free_modifier_refs_have_generated_reciprocity(
    free_modifiers: &[&FreeModifierSyntax],
) -> bool {
    free_modifiers
        .iter()
        .any(|free_modifier| generated_soi_free_modifier(free_modifier).is_some())
}

#[requires(true)]
#[ensures(ret.is_some() == matches!(free_modifier, FreeModifierSyntax::SoiFreeModifier(_)))]
pub(super) fn generated_soi_free_modifier(
    free_modifier: &FreeModifierSyntax,
) -> Option<&SoiFreeModifierSyntax> {
    match free_modifier {
        FreeModifierSyntax::SoiFreeModifier(free_modifier) => Some(free_modifier),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_text_paragraphs<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    paragraphs: &'syntax TextParagraphsSyntax,
) -> Result<(), SemanticsError> {
    match paragraphs {
        TextParagraphsSyntax::TextParagraphWithAdditionalNiho(paragraphs) => {
            push_generated_text_paragraph_with_additional_niho(items, paragraphs)
        }
        TextParagraphsSyntax::TextNihoParagraphs(TextNihoParagraphsSyntax(paragraphs)) => {
            for paragraph in paragraphs {
                push_generated_niho_paragraph_items(items, paragraph)?;
            }
            Ok(())
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_text_paragraph_with_additional_niho<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    paragraphs: &'syntax TextParagraphWithAdditionalNihoSyntax,
) -> Result<(), SemanticsError> {
    push_generated_paragraph_items(items, &paragraphs.first)?;
    for paragraph in &paragraphs.additional_niho {
        push_generated_niho_paragraph_items(items, paragraph)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_paragraph_items<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    paragraph: &'syntax ParagraphSyntax,
) -> Result<(), SemanticsError> {
    match paragraph {
        ParagraphSyntax::SimpleParagraph(SimpleParagraphSyntax(sequence)) => {
            push_generated_paragraph_statement_sequence_items(items, sequence, &[])
        }
        ParagraphSyntax::INihoParagraph(paragraph) => {
            push_generated_optional_niho_statement_sequence_items(
                items,
                &paragraph.niho,
                paragraph.statements.as_deref(),
                &paragraph.free_modifiers,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_niho_paragraph_items<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    paragraph: &'syntax NihoParagraphSyntax,
) -> Result<(), SemanticsError> {
    push_generated_optional_niho_statement_sequence_items(
        items,
        &paragraph.niho,
        paragraph.statements.as_deref(),
        &paragraph.free_modifiers,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_optional_niho_statement_sequence_items<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    markers: &'syntax Vec1<Token>,
    sequence: Option<&'syntax ParagraphStatementSequenceSyntax>,
    free_modifiers: &'syntax [FreeModifierSyntax],
) -> Result<(), SemanticsError> {
    if let Some(sequence) = sequence {
        push_generated_paragraph_statement_sequence_items(items, sequence, free_modifiers)
    } else {
        items.push(GeneratedTextPlanItem::StandaloneParagraphBoundary {
            markers,
            free_modifiers: free_modifiers.iter().collect(),
        });
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_paragraph_statement_sequence_items<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    sequence: &'syntax ParagraphStatementSequenceSyntax,
    leading_free_modifiers: &'syntax [FreeModifierSyntax],
) -> Result<(), SemanticsError> {
    items.push(GeneratedTextPlanItem::Root {
        root: semantic_root_from_statement_or_fragment(sequence.initial.0.as_ref())?,
        free_modifiers: leading_free_modifiers.iter().collect(),
        separator_i: None,
    });
    for following in &sequence.following {
        push_generated_following_paragraph_statement_item(items, following)?;
    }
    for trailing in &sequence.trailing {
        items.push(GeneratedTextPlanItem::PendingStatementConnection {
            i: &trailing.i,
            connective: &trailing.connective,
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_generated_following_paragraph_statement_item<'syntax>(
    items: &mut Vec<GeneratedTextPlanItem<'syntax>>,
    following: &'syntax FollowingParagraphStatementSyntax,
) -> Result<(), SemanticsError> {
    if let Some(statement) = following.statement.as_deref() {
        items.push(GeneratedTextPlanItem::Root {
            root: semantic_root_from_statement_or_fragment(statement)?,
            free_modifiers: following.free_modifiers.iter().collect(),
            separator_i: Some(&following.i),
        });
    } else if !following.free_modifiers.is_empty()
        && !indicator_parts_for_token(&following.i).is_empty()
    {
        items.push(GeneratedTextPlanItem::TrailingSeparator {
            i: &following.i,
            free_modifiers: following.free_modifiers.iter().collect(),
        });
    } else if !following.free_modifiers.is_empty() {
        items.push(GeneratedTextPlanItem::StandaloneFreeModifiers(
            following.free_modifiers.iter().collect(),
        ));
    } else if !indicator_parts_for_token(&following.i).is_empty() {
        items.push(GeneratedTextPlanItem::TrailingSeparator {
            i: &following.i,
            free_modifiers: Vec::new(),
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn semantic_root_from_statement_or_fragment(
    statement_or_fragment: &StatementOrFragmentSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
    match statement_or_fragment {
        StatementOrFragmentSyntax::ZantufaStatementTermsStatement(statement) => {
            Ok(GeneratedTextRoot::ZantufaStatementTerms(statement))
        }
        StatementOrFragmentSyntax::StatementOrFragmentStatement(
            StatementOrFragmentStatementSyntax(statement),
        ) => semantic_root_from_statement(statement),
        StatementOrFragmentSyntax::FragmentStatement(fragment) => {
            Ok(GeneratedTextRoot::Fragment(match fragment {
                FragmentStatementSyntax::PrenexFragment(fragment) => {
                    GeneratedFragmentRoot::Prenex(fragment)
                }
                FragmentStatementSyntax::SelbriFragment(fragment) => {
                    GeneratedFragmentRoot::Selbri(fragment)
                }
                FragmentStatementSyntax::EkFragment(fragment) => {
                    GeneratedFragmentRoot::Ek(fragment)
                }
                FragmentStatementSyntax::GihekFragment(fragment) => {
                    GeneratedFragmentRoot::Gihek(fragment)
                }
                FragmentStatementSyntax::MultipleNaFragment(fragment) => {
                    GeneratedFragmentRoot::MultipleNa(fragment)
                }
                FragmentStatementSyntax::SingleNaFragment(fragment) => {
                    GeneratedFragmentRoot::SingleNa(fragment)
                }
                FragmentStatementSyntax::TermsFragment(fragment) => {
                    GeneratedFragmentRoot::Terms(fragment)
                }
                FragmentStatementSyntax::MeksoFragment(fragment) => {
                    GeneratedFragmentRoot::Mekso(fragment)
                }
                FragmentStatementSyntax::RelativeClauseFragment(fragment) => {
                    GeneratedFragmentRoot::RelativeClause(fragment)
                }
                FragmentStatementSyntax::LinkedSumtiContinuationFragment(fragment) => {
                    GeneratedFragmentRoot::LinkedSumtiContinuation(fragment)
                }
                FragmentStatementSyntax::LinkedSumtiFragment(fragment) => {
                    GeneratedFragmentRoot::LinkedSumti(fragment)
                }
                FragmentStatementSyntax::ZantufaMeksoFragment(fragment) => {
                    GeneratedFragmentRoot::ZantufaMekso(fragment)
                }
            }))
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn semantic_root_from_statement(
    statement: &StatementSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
    match statement {
        StatementSyntax::IStatementConnection(connection) => {
            Ok(GeneratedTextRoot::StatementConnection(connection))
        }
        StatementSyntax::PreposedIStatementConnection(connection) => {
            Ok(GeneratedTextRoot::PreposedStatementConnection(connection))
        }
        StatementSyntax::StatementBase(base) => semantic_root_from_statement_base(base),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_text_root_is_utterance(root: &GeneratedTextRoot<'_>) -> bool {
    match root {
        GeneratedTextRoot::Bridi(_) | GeneratedTextRoot::Fragment(_) => true,
        GeneratedTextRoot::PrenexStatement(statement) => {
            generated_statement_is_utterance(&statement.inner_statement)
        }
        GeneratedTextRoot::TextGroupStatement(_) => true,
        GeneratedTextRoot::ZantufaStatementTerms(statement) => {
            let suffix_terms = zantufa_statement_terms_tail_terms(&statement.tail);
            semantic_root_from_statement(&statement.statement).is_ok_and(|root| {
                if suffix_terms.is_empty() {
                    generated_text_root_is_utterance(&root)
                } else {
                    matches!(root, GeneratedTextRoot::Bridi(_))
                }
            })
        }
        GeneratedTextRoot::ForethoughtStatement(_) => false,
        GeneratedTextRoot::StatementConnection(_)
        | GeneratedTextRoot::PreposedStatementConnection(_) => false,
    }
}

#[requires(true)]
#[ensures(ret == match tail {
    ZantufaStatementTermsTailSyntax::ZantufaIauStatementTermsTail(tail) => tail.terms.is_empty(),
    ZantufaStatementTermsTailSyntax::ZantufaBareStatementTermsTail(_) => false,
})]
pub(super) fn zantufa_statement_terms_tail_is_semantically_empty(
    tail: &ZantufaStatementTermsTailSyntax,
) -> bool {
    match tail {
        ZantufaStatementTermsTailSyntax::ZantufaIauStatementTermsTail(tail) => {
            tail.terms.is_empty()
        }
        ZantufaStatementTermsTailSyntax::ZantufaBareStatementTermsTail(_) => false,
    }
}

#[requires(true)]
#[ensures(matches!(tail, ZantufaStatementTermsTailSyntax::ZantufaBareStatementTermsTail(_)) -> !ret.is_empty())]
pub(super) fn zantufa_statement_terms_tail_terms(
    tail: &ZantufaStatementTermsTailSyntax,
) -> Vec<&TermSyntax> {
    match tail {
        ZantufaStatementTermsTailSyntax::ZantufaIauStatementTermsTail(tail) => {
            tail.terms.iter().collect()
        }
        ZantufaStatementTermsTailSyntax::ZantufaBareStatementTermsTail(tail) => {
            tail.0.iter().map(|term| term.as_ref()).collect()
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bridi_force(bridi: &BridiSyntax, truth_question: bool) -> UtteranceForce {
    if truth_question {
        UtteranceForce::Ask
    } else if generated_node_contains_cmavo(bridi, Cmavo::Ko) {
        UtteranceForce::Command
    } else {
        UtteranceForce::Assert
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
pub(super) fn bridi_from_statement_base(
    base: &StatementBaseSyntax,
) -> Result<&BridiSyntax, SemanticsError> {
    match base {
        StatementBaseSyntax::BridiStatement(statement) => bridi_from_bridi_statement(statement),
        StatementBaseSyntax::PrenexStatement(statement) => {
            let root = semantic_root_from_statement(&statement.inner_statement)?;
            let GeneratedTextRoot::Bridi(bridi) = root else {
                return Err(unsupported("prenex non-bridi statement"));
            };
            Ok(bridi)
        }
        StatementBaseSyntax::TextGroupStatement(_) => Err(unsupported("text group statement")),
        StatementBaseSyntax::ForethoughtStatement(_) => {
            Err(unsupported("forethought statement as bridi"))
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn semantic_root_from_statement_base(
    base: &StatementBaseSyntax,
) -> Result<GeneratedTextRoot<'_>, SemanticsError> {
    match base {
        StatementBaseSyntax::PrenexStatement(statement) => {
            Ok(GeneratedTextRoot::PrenexStatement(statement))
        }
        StatementBaseSyntax::BridiStatement(statement) => Ok(GeneratedTextRoot::Bridi(
            bridi_from_bridi_statement(statement)?,
        )),
        StatementBaseSyntax::TextGroupStatement(statement) => {
            Ok(GeneratedTextRoot::TextGroupStatement(statement))
        }
        StatementBaseSyntax::ForethoughtStatement(statement) => {
            Ok(GeneratedTextRoot::ForethoughtStatement(statement))
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_is_utterance(statement: &StatementSyntax) -> bool {
    match statement {
        StatementSyntax::StatementBase(base) => generated_statement_base_is_utterance(base),
        StatementSyntax::IStatementConnection(_)
        | StatementSyntax::PreposedIStatementConnection(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_statement_base_is_utterance(base: &StatementBaseSyntax) -> bool {
    match base {
        StatementBaseSyntax::BridiStatement(statement) => statement.continuations.is_empty(),
        StatementBaseSyntax::PrenexStatement(statement) => {
            generated_statement_is_utterance(&statement.inner_statement)
        }
        StatementBaseSyntax::TextGroupStatement(_) => false,
        StatementBaseSyntax::ForethoughtStatement(_) => false,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
pub(super) fn bridi_from_bridi_statement(
    statement: &BridiStatementSyntax,
) -> Result<&BridiSyntax, SemanticsError> {
    if !statement.continuations.is_empty() {
        return Err(unsupported("bridi statement continuations"));
    }
    Ok(&statement.bridi)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bridi| generated_node_contains_cmavo(*bridi, Cmavo::Ko) || !generated_node_contains_cmavo(*bridi, Cmavo::Ko)) || ret.is_err())]
pub(super) fn bridi_from_statement_after_i_connective(
    statement: &StatementAfterIConnectiveSyntax,
) -> Result<&BridiSyntax, SemanticsError> {
    match statement {
        StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
            bridi_from_bridi_statement(statement)
        }
        StatementAfterIConnectiveSyntax::TextGroupStatement(_) => {
            Err(unsupported("text group statement connection"))
        }
        StatementAfterIConnectiveSyntax::ForethoughtStatement(_) => {
            Err(unsupported("forethought statement as bridi"))
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
pub(super) fn main_generated_selbri_for_subbridi(
    subbridi: &SubbridiSyntax,
) -> Option<&SelbriSyntax> {
    match subbridi {
        SubbridiSyntax::BridiSubbridi(BridiSubbridiSyntax(bridi)) => {
            main_generated_selbri_for_bridi(bridi)
        }
        SubbridiSyntax::PrenexSubbridi(prenex) => {
            main_generated_selbri_for_subbridi(&prenex.inner_subbridi)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
pub(super) fn main_generated_selbri_for_bridi(bridi: &BridiSyntax) -> Option<&SelbriSyntax> {
    match bridi {
        BridiSyntax::BridiWithLeadingTerms(BridiWithLeadingTermsSyntax { bridi_tail, .. })
        | BridiSyntax::BareCuBridi(BareCuBridiSyntax { bridi_tail, .. }) => {
            main_generated_selbri_for_bridi_tail(bridi_tail)
        }
        BridiSyntax::BridiWithPostCuTerms(BridiWithPostCuTermsSyntax { bridi_tail, .. })
        | BridiSyntax::BareCuTermsBridi(BareCuTermsBridiSyntax { bridi_tail, .. }) => {
            main_generated_selbri_for_cu_terms_bridi_tail(bridi_tail)
        }
        BridiSyntax::RelationOnlyBridi(RelationOnlyBridiSyntax(bridi_tail)) => {
            main_generated_selbri_for_bridi_tail(bridi_tail)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
pub(super) fn main_generated_selbri_for_cu_terms_bridi_tail(
    tail: &CuTermsBridiTailSyntax,
) -> Option<&SelbriSyntax> {
    main_generated_selbri_for_bridi_tail(&tail.bridi_tail)
}

#[requires(true)]
#[ensures(ret.is_none_or(|selbri| generated_node_contains_cmavo(selbri, Cmavo::Se) || !generated_node_contains_cmavo(selbri, Cmavo::Se)))]
pub(super) fn main_generated_selbri_for_bridi_tail(
    tail: &BridiTailSyntax,
) -> Option<&SelbriSyntax> {
    simple_tail_from_bridi_tail(tail)
        .ok()
        .map(|simple_tail| simple_tail.selbri.as_ref())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn statement_connection_tail_parts(
    tail: &IStatementConnectionTailSyntax,
) -> Result<
    (
        &[PendingIConnectiveSyntax],
        &Token,
        &IStatementConnectiveSyntax,
        &StatementAfterIConnectiveSyntax,
    ),
    SemanticsError,
> {
    match tail {
        IStatementConnectionTailSyntax::SimpleIConnectiveStatementTail(tail) => Ok((
            &[],
            &tail.i,
            &tail.connective,
            tail.trailing_statement.as_ref(),
        )),
        IStatementConnectionTailSyntax::ChainedIConnectiveStatementTail(tail) => Ok((
            tail.pending.as_slice(),
            &tail.i,
            &tail.connective,
            tail.trailing_statement.as_ref(),
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, spec)| !spec.introduced_by.is_empty() && !spec.relation.is_empty() && spec.visible_place > 0))]
pub(super) fn generated_text_group_statement_connection_spec(
    statement: &StatementAfterIConnectiveSyntax,
) -> Option<(&TenseModalSyntax, GeneratedModalStatementConnectionSpec)> {
    let StatementAfterIConnectiveSyntax::TextGroupStatement(statement) = statement else {
        return None;
    };
    let tense_modal = statement.tense_modal.as_deref()?;
    generated_modal_statement_connection_spec_for_tense_modal(tense_modal)
        .map(|spec| (tense_modal, spec))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn simple_tail_from_bridi_tail(
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
    Ok(simple_tail)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|connection| connection.is_none_or(|connection| generated_node_contains_cmavo(connection, Cmavo::Gi))) || ret.is_err())]
pub(super) fn forethought_connection_from_bridi_tail(
    tail: &BridiTailSyntax,
) -> Result<Option<&ForethoughtBridiConnectionSyntax>, SemanticsError> {
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
    Ok(match first.as_ref() {
        SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(forethought) => Some(&forethought.0),
        SimpleBridiTailSyntax::SelbriSimpleBridiTail(_) => None,
    })
}
