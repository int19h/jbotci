use super::*;

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_operator_label(
    operator: &MeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
            generated_afterthought_mekso_operator_label(operator)
        }
        MeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            generated_bound_mekso_operator_label(operator)
        }
        MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_simple_mekso_operator_label(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_afterthought_mekso_operator_label(
    operator: &AfterthoughtMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    let mut label = generated_bound_or_atom_mekso_operator_label(operator.0.first.as_ref())?;
    for link in &operator.0.links {
        label = format!(
            "{} {}",
            label,
            generated_bound_or_atom_mekso_operator_label(&link.trailing_operator)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_or_atom_mekso_operator_label(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            generated_bound_mekso_operator_label(operator)
        }
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_simple_mekso_operator_label(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_mekso_operator_label(
    operator: &BoundMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    Ok(format!(
        "{} {}",
        generated_simple_mekso_operator_label(&operator.left_operator)?,
        generated_mekso_operator_label(&operator.right_operator)?
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_simple_mekso_operator_label(
    operator: &SimpleMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        SimpleMeksoOperatorSyntax::PrimitiveMeksoOperator(operator) => {
            Ok(token_text(&operator.0.value))
        }
        SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
            generated_mekso_operator_label(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
            generated_mekso_operator_label(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            generated_mekso_operator_label(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ForethoughtMeksoOperator(operator) => Ok(format!(
            "{} {}",
            generated_mekso_operator_label(&operator.left_operator)?,
            generated_mekso_operator_label(&operator.right_operator)?
        )),
        SimpleMeksoOperatorSyntax::SelbriMeksoOperator(operator) => {
            relation_label_from_selbri(&operator.selbri).map(|label| label.display_text())
        }
        SimpleMeksoOperatorSyntax::OperandMeksoOperator(_) => Ok("operand-operator".to_owned()),
        SimpleMeksoOperatorSyntax::ZantufaMahoSelbriMeksoOperator(operator) => {
            relation_label_from_selbri(&operator.selbri).map(|label| label.display_text())
        }
        SimpleMeksoOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_) => {
            Ok("sumti-operator".to_owned())
        }
        SimpleMeksoOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            Ok(generated_operand_connective_source(&operator.0))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_operator_surface_label(
    operator: &MeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
            generated_afterthought_mekso_operator_surface_label(operator)
        }
        MeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            generated_bound_mekso_operator_surface_label(operator)
        }
        MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_simple_mekso_operator_surface_label(operator)
        }
    }
}

#[requires(!operators.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_zantufa_mekso_operator_sequence_label<O: AsRef<MeksoOperatorSyntax>>(
    operators: &[O],
) -> Result<String, SemanticsError> {
    operators
        .iter()
        .map(|operator| generated_mekso_operator_surface_label(operator.as_ref()))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(" "))
}

#[requires(!operators.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|(label, _)| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_zantufa_mekso_operator_sequence_label_with_replacement<
    O: AsRef<MeksoOperatorSyntax>,
>(
    operators: &[O],
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<(String, bool), SemanticsError> {
    let mut replaced = false;
    let mut parts = Vec::with_capacity(operators.len());
    for operator in operators {
        if !replaced && connected_generated_mekso_operator(operator.as_ref())?.is_some() {
            parts.push(generated_mekso_operator_surface_label(
                replacement_operator,
            )?);
            replaced = true;
        } else {
            parts.push(generated_mekso_operator_surface_label(operator.as_ref())?);
        }
    }
    Ok((parts.join(" "), replaced))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_afterthought_mekso_operator_surface_label(
    operator: &AfterthoughtMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    let mut label =
        generated_bound_or_atom_mekso_operator_surface_label(operator.0.first.as_ref())?;
    for link in &operator.0.links {
        label = format!(
            "{} {} {}",
            label,
            generated_statement_connective_core_source(&statement_connective_from_standard(
                &link.connective,
            ))?,
            generated_bound_or_atom_mekso_operator_surface_label(&link.trailing_operator)?
        );
    }
    Ok(label)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_or_atom_mekso_operator_surface_label(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            generated_bound_mekso_operator_surface_label(operator)
        }
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_simple_mekso_operator_surface_label(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_mekso_operator_surface_label(
    operator: &BoundMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    Ok(format!(
        "{} {} bo {}",
        generated_simple_mekso_operator_surface_label(&operator.left_operator)?,
        generated_statement_connective_core_source(&statement_connective_from_standard(
            &operator.connective,
        ))?,
        generated_mekso_operator_surface_label(&operator.right_operator)?
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|label| !label.is_empty()) || ret.is_err())]
pub(super) fn generated_simple_mekso_operator_surface_label(
    operator: &SimpleMeksoOperatorSyntax,
) -> Result<String, SemanticsError> {
    match operator {
        SimpleMeksoOperatorSyntax::PrimitiveMeksoOperator(operator) => {
            Ok(token_text(&operator.0.value))
        }
        SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => Ok(format!(
            "{} {}",
            token_text(&operator.se.value),
            generated_mekso_operator_surface_label(&operator.inner_operator)?
        )),
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => Ok(format!(
            "{} {}",
            token_text(&operator.nahe.value),
            generated_mekso_operator_surface_label(&operator.inner_operator)?
        )),
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            generated_mekso_operator_surface_label(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ForethoughtMeksoOperator(operator) => Ok(format!(
            "{} {} {}",
            generated_guhek_gik_connective_source(&operator.guhek, &operator.gik),
            generated_mekso_operator_surface_label(&operator.left_operator)?,
            generated_mekso_operator_surface_label(&operator.right_operator)?
        )),
        SimpleMeksoOperatorSyntax::SelbriMeksoOperator(operator) => {
            relation_label_from_selbri(&operator.selbri).map(|label| label.display_text())
        }
        SimpleMeksoOperatorSyntax::OperandMeksoOperator(_) => Ok("operand-operator".to_owned()),
        SimpleMeksoOperatorSyntax::ZantufaMahoSelbriMeksoOperator(operator) => Ok(format!(
            "{} {}",
            token_text(&operator.maho.value),
            relation_label_from_selbri(&operator.selbri)?.display_text()
        )),
        SimpleMeksoOperatorSyntax::ZantufaMahoSumtiMeksoOperator(operator) => {
            let mut visitor = GeneratedSpanCollector::default();
            operator.sumti.visit_in_order(&mut visitor);
            Ok(format!(
                "{} {}",
                token_text(&operator.maho.value),
                token_list_text(visitor.tokens.iter())
            ))
        }
        SimpleMeksoOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            Ok(generated_operand_connective_source(&operator.0))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_surface_text(
    expression: &MeksoSyntax,
) -> Result<String, SemanticsError> {
    match expression {
        MeksoSyntax::ZantufaReversePolishMekso(reverse_polish) => {
            generated_zantufa_reverse_polish_surface_text(reverse_polish)
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => generated_zantufa_infix_mekso_surface_text(infix),
        MeksoSyntax::InfixMekso(infix) => generated_infix_mekso_surface_text(infix),
        MeksoSyntax::ReversePolishMekso(reverse_polish) => {
            generated_reverse_polish_surface_text(reverse_polish)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_mekso_surface_text_with_connected_operator_replacement(
    expression: &MeksoSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    match expression {
        MeksoSyntax::ZantufaReversePolishMekso(_) => Ok(None),
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            generated_zantufa_infix_mekso_surface_text_with_connected_operator_replacement(
                infix,
                replacement_operator,
            )
        }
        MeksoSyntax::InfixMekso(infix) => {
            generated_infix_mekso_surface_text_with_connected_operator_replacement(
                infix,
                replacement_operator,
            )
        }
        MeksoSyntax::ReversePolishMekso(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_infix_mekso_surface_text_with_connected_operator_replacement(
    infix: &InfixMeksoSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    if infix.continuations.is_empty() {
        return generated_mekso_precedence_surface_text_with_connected_operator_replacement(
            &infix.first_expression,
            replacement_operator,
        );
    }
    let mut replaced = false;
    let mut text = if let Some(first) =
        generated_mekso_precedence_surface_text_with_connected_operator_replacement(
            &infix.first_expression,
            replacement_operator,
        )? {
        replaced = true;
        first
    } else {
        generated_mekso_precedence_surface_text(&infix.first_expression)?
    };
    for continuation in &infix.continuations {
        if !replaced && connected_generated_mekso_operator(&continuation.operator)?.is_some() {
            replaced = true;
            let right = generated_mekso_precedence_surface_text(&continuation.right_expression)?;
            text = format!(
                "{} {} {}",
                text,
                generated_mekso_operator_surface_label(replacement_operator)?,
                right
            );
        } else {
            let right = if replaced {
                generated_mekso_precedence_surface_text(&continuation.right_expression)?
            } else if let Some(right) =
                generated_mekso_precedence_surface_text_with_connected_operator_replacement(
                    &continuation.right_expression,
                    replacement_operator,
                )?
            {
                replaced = true;
                right
            } else {
                generated_mekso_precedence_surface_text(&continuation.right_expression)?
            };
            text = format!(
                "{} {} {}",
                text,
                generated_mekso_operator_surface_label(&continuation.operator)?,
                right
            );
        }
    }
    Ok(replaced.then_some(text))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_zantufa_infix_mekso_surface_text_with_connected_operator_replacement(
    infix: &ZantufaInfixMeksoSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    if infix.continuations.is_empty() {
        return generated_mekso_precedence_surface_text_with_connected_operator_replacement(
            &infix.first_expression,
            replacement_operator,
        );
    }
    let mut replaced = false;
    let mut text = if let Some(first) =
        generated_mekso_precedence_surface_text_with_connected_operator_replacement(
            &infix.first_expression,
            replacement_operator,
        )? {
        replaced = true;
        first
    } else {
        generated_mekso_precedence_surface_text(&infix.first_expression)?
    };
    for continuation in &infix.continuations {
        let mut operator_texts = Vec::with_capacity(continuation.operators.len());
        for operator in &continuation.operators {
            if !replaced && connected_generated_mekso_operator(operator)?.is_some() {
                replaced = true;
                operator_texts.push(generated_mekso_operator_surface_label(
                    replacement_operator,
                )?);
            } else {
                operator_texts.push(generated_mekso_operator_surface_label(operator)?);
            }
        }
        text = format!("{} {}", text, operator_texts.join(" "));
        if let Some(right_expression) = &continuation.right_expression {
            let right = if replaced {
                generated_mekso_precedence_surface_text(right_expression)?
            } else if let Some(right) =
                generated_mekso_precedence_surface_text_with_connected_operator_replacement(
                    right_expression,
                    replacement_operator,
                )?
            {
                replaced = true;
                right
            } else {
                generated_mekso_precedence_surface_text(right_expression)?
            };
            text = format!("{text} {right}");
        }
    }
    Ok(replaced.then_some(text))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_mekso_precedence_surface_text_with_connected_operator_replacement(
    expression: &MeksoPrecedenceSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    let Some(tail) = &expression.tail else {
        return generated_mekso_base_surface_text_with_connected_operator_replacement(
            &expression.left_expression,
            replacement_operator,
        );
    };
    if connected_generated_mekso_operator(&tail.operator)?.is_some() {
        return Ok(Some(format!(
            "{} {} {}",
            generated_mekso_base_surface_text(&expression.left_expression)?,
            generated_mekso_operator_surface_label(replacement_operator)?,
            generated_mekso_precedence_surface_text(&tail.right_expression)?
        )));
    }
    if let Some(left) = generated_mekso_base_surface_text_with_connected_operator_replacement(
        &expression.left_expression,
        replacement_operator,
    )? {
        return Ok(Some(format!(
            "{} {} {}",
            left,
            generated_mekso_operator_surface_label(&tail.operator)?,
            generated_mekso_precedence_surface_text(&tail.right_expression)?
        )));
    }
    generated_mekso_precedence_surface_text_with_connected_operator_replacement(
        &tail.right_expression,
        replacement_operator,
    )?
    .map(|right| {
        Ok(format!(
            "{} {} {}",
            generated_mekso_base_surface_text(&expression.left_expression)?,
            generated_mekso_operator_surface_label(&tail.operator)?,
            right
        ))
    })
    .transpose()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_mekso_base_surface_text_with_connected_operator_replacement(
    expression: &MeksoBaseSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    match expression {
        MeksoBaseSyntax::MeksoOperand(operand) => {
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )
        }
        MeksoBaseSyntax::ForethoughtCallMekso(call) => {
            if connected_generated_mekso_operator(&call.operator)?.is_some() {
                let mut parts = Vec::with_capacity(call.operands.len() + 1);
                parts.push(generated_mekso_operator_surface_label(
                    replacement_operator,
                )?);
                for operand in &call.operands {
                    parts.push(generated_mekso_base_surface_text(operand)?);
                }
                return Ok(Some(parts.join(" ")));
            }
            let mut parts = Vec::with_capacity(call.operands.len() + 1);
            parts.push(generated_mekso_operator_surface_label(&call.operator)?);
            let mut replaced = false;
            for operand in &call.operands {
                if replaced {
                    parts.push(generated_mekso_base_surface_text(operand)?);
                } else if let Some(text) =
                    generated_mekso_base_surface_text_with_connected_operator_replacement(
                        operand,
                        replacement_operator,
                    )?
                {
                    replaced = true;
                    parts.push(text);
                } else {
                    parts.push(generated_mekso_base_surface_text(operand)?);
                }
            }
            Ok(replaced.then(|| parts.join(" ")))
        }
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
            generated_zantufa_bo_grouped_mekso_base_surface_text_with_connected_operator_replacement(
                group,
                replacement_operator,
            )
        }
        MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => {
            generated_zantufa_grouped_mekso_operand_sequence_surface_text_with_connected_operator_replacement(
                group,
                replacement_operator,
            )
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_zantufa_bo_grouped_mekso_base_surface_text_with_connected_operator_replacement(
    group: &ZantufaBoGroupedMeksoBaseSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    let mut replaced = false;
    let mut parts = Vec::with_capacity(1 + group.continuations.len() * 2);
    if let Some(first) = generated_mekso_operand_surface_text_with_connected_operator_replacement(
        &group.first,
        replacement_operator,
    )? {
        replaced = true;
        parts.push(first);
    } else {
        parts.push(generated_mekso_operand_surface_text(&group.first)?);
    }
    for continuation in &group.continuations {
        parts.push(token_text(&continuation.bo.value));
        if replaced {
            parts.push(generated_mekso_operand_surface_text(
                &continuation.expression,
            )?);
        } else if let Some(expression) =
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                &continuation.expression,
                replacement_operator,
            )?
        {
            replaced = true;
            parts.push(expression);
        } else {
            parts.push(generated_mekso_operand_surface_text(
                &continuation.expression,
            )?);
        }
    }
    Ok(replaced.then(|| parts.join(" ")))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_zantufa_grouped_mekso_operand_sequence_surface_text_with_connected_operator_replacement(
    group: &ZantufaGroupedMeksoOperandSequenceSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    let mut replaced = false;
    let mut parts = Vec::with_capacity(group.operands.len() + 2);
    parts.push(token_text(&group.ke.value));
    for operand in &group.operands {
        if replaced {
            parts.push(generated_mekso_operand_surface_text(operand)?);
        } else if let Some(text) =
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )?
        {
            replaced = true;
            parts.push(text);
        } else {
            parts.push(generated_mekso_operand_surface_text(operand)?);
        }
    }
    if let Some(kehe) = &group.kehe {
        parts.push(token_text(&kehe.value));
    }
    Ok(replaced.then(|| parts.join(" ")))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_mekso_operand_surface_text_with_connected_operator_replacement(
    operand: &MeksoOperandSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    match operand {
        MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
            let chain = &operand.0;
            let mut text = generated_bound_or_simple_mekso_operand_surface_text(&chain.first)?;
            let mut replaced = false;
            if let Some(first) =
                generated_bound_or_simple_mekso_operand_surface_text_with_connected_operator_replacement(
                    &chain.first,
                    replacement_operator,
                )?
            {
                text = first;
                replaced = true;
            }
            for link in &chain.links {
                let trailing = if replaced {
                    generated_bound_or_simple_mekso_operand_surface_text(
                        &link.trailing_expression,
                    )?
                } else if let Some(replaced_trailing) =
                    generated_bound_or_simple_mekso_operand_surface_text_with_connected_operator_replacement(
                        &link.trailing_expression,
                        replacement_operator,
                    )?
                {
                    replaced = true;
                    replaced_trailing
                } else {
                    generated_bound_or_simple_mekso_operand_surface_text(
                        &link.trailing_expression,
                    )?
                };
                text = format!(
                    "{} {} {}",
                    text,
                    generated_operand_connective_source(&link.operand_connective),
                    trailing
                );
            }
            Ok(replaced.then_some(text))
        }
        MeksoOperandSyntax::BoundMeksoOperand(operand) => {
            generated_bound_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )
        }
        MeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_bound_or_simple_mekso_operand_surface_text_with_connected_operator_replacement(
    operand: &BoundOrSimpleMeksoOperandSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
            generated_bound_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )
        }
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_surface_text_with_connected_operator_replacement(
                operand,
                replacement_operator,
            )
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_bound_mekso_operand_surface_text_with_connected_operator_replacement(
    operand: &BoundMeksoOperandSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    if let Some(left) =
        generated_simple_mekso_operand_surface_text_with_connected_operator_replacement(
            &operand.left_expression,
            replacement_operator,
        )?
    {
        return Ok(Some(format!(
            "{} {} {}",
            left,
            generated_operand_connective_source(&operand.operand_connective),
            generated_mekso_operand_surface_text(&operand.right_expression)?
        )));
    }
    generated_mekso_operand_surface_text_with_connected_operator_replacement(
        &operand.right_expression,
        replacement_operator,
    )?
    .map(|right| {
        Ok(format!(
            "{} {} {}",
            generated_simple_mekso_operand_surface_text(&operand.left_expression)?,
            generated_operand_connective_source(&operand.operand_connective),
            right
        ))
    })
    .transpose()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.as_ref().is_none_or(|text| !text.is_empty())) || ret.is_err())]
pub(super) fn generated_simple_mekso_operand_surface_text_with_connected_operator_replacement(
    operand: &SimpleMeksoOperandSyntax,
    replacement_operator: &MeksoOperatorSyntax,
) -> Result<Option<String>, SemanticsError> {
    match operand {
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
            if let Some(left) =
                generated_mekso_operand_surface_text_with_connected_operator_replacement(
                    &operand.left_expression,
                    replacement_operator,
                )?
            {
                return Ok(Some(format!(
                    "{} {} {}",
                    generated_modal_forethought_connective_source(&operand.gek),
                    left,
                    generated_mekso_operand_surface_text(&operand.right_expression)?
                )));
            }
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                &operand.right_expression,
                replacement_operator,
            )?
            .map(|right| {
                Ok(format!(
                    "{} {} {}",
                    generated_modal_forethought_connective_source(&operand.gek),
                    generated_mekso_operand_surface_text(&operand.left_expression)?,
                    right
                ))
            })
            .transpose()
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                &operand.inner_expression,
                replacement_operator,
            )
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_mekso_surface_text_with_connected_operator_replacement(
                &operand.inner_expression,
                replacement_operator,
            )
        }
        SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => {
            let mut parts = Vec::with_capacity(operand.expressions.len());
            let mut replaced = false;
            for expression in &operand.expressions {
                if replaced {
                    parts.push(generated_mekso_surface_text(expression)?);
                } else if let Some(text) =
                    generated_mekso_surface_text_with_connected_operator_replacement(
                        expression,
                        replacement_operator,
                    )?
                {
                    replaced = true;
                    parts.push(text);
                } else {
                    parts.push(generated_mekso_surface_text(expression)?);
                }
            }
            Ok(replaced.then(|| parts.join(" ")))
        }
        SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntax::NumberMekso(_)
        | SimpleMeksoOperandSyntax::LerfuStringMekso(_) => Ok(None),
        SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => {
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                &operand.inner_expression,
                replacement_operator,
            )
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_subscript_mekso_surface_text(
    expression: &MeksoSyntax,
) -> Result<String, SemanticsError> {
    if generated_mekso_contains_operand_connection(expression) {
        return Ok("mekso".to_owned());
    }
    generated_mekso_surface_text(expression)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_mekso_surface_text(
    expression: &MeksoSyntax,
) -> Result<String, SemanticsError> {
    match expression {
        MeksoSyntax::ZantufaReversePolishMekso(reverse_polish) => {
            if generated_zantufa_reverse_polish_contains_operand_connection(reverse_polish) {
                Ok("mekso".to_owned())
            } else {
                generated_zantufa_reverse_polish_surface_text(reverse_polish)
            }
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            generated_number_descriptor_zantufa_infix_mekso_surface_text(infix)
        }
        MeksoSyntax::InfixMekso(infix) => {
            generated_number_descriptor_infix_mekso_surface_text(infix)
        }
        MeksoSyntax::ReversePolishMekso(reverse_polish) => {
            if generated_reverse_polish_parts_contains_operand_connection(&reverse_polish.parts) {
                Ok("mekso".to_owned())
            } else {
                generated_reverse_polish_surface_text(reverse_polish)
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_infix_mekso_surface_text(
    infix: &InfixMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut text =
        generated_number_descriptor_mekso_precedence_surface_text(&infix.first_expression)?;
    for continuation in &infix.continuations {
        text = format!(
            "{} {} {}",
            text,
            generated_mekso_operator_surface_label(&continuation.operator)?,
            generated_number_descriptor_mekso_precedence_surface_text(
                &continuation.right_expression,
            )?
        );
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_zantufa_infix_mekso_surface_text(
    infix: &ZantufaInfixMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut text =
        generated_number_descriptor_mekso_precedence_surface_text(&infix.first_expression)?;
    for continuation in &infix.continuations {
        let mut parts = Vec::with_capacity(continuation.operators.len() + 1);
        for operator in &continuation.operators {
            parts.push(generated_mekso_operator_surface_label(operator)?);
        }
        if let Some(right_expression) = &continuation.right_expression {
            parts.push(generated_number_descriptor_mekso_precedence_surface_text(
                right_expression,
            )?);
        }
        text = format!("{} {}", text, parts.join(" "));
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_mekso_precedence_surface_text(
    expression: &MeksoPrecedenceSyntax,
) -> Result<String, SemanticsError> {
    let mut text =
        generated_number_descriptor_mekso_base_surface_text(&expression.left_expression)?;
    if let Some(tail) = &expression.tail {
        text = format!(
            "{} {} {}",
            text,
            generated_mekso_operator_surface_label(&tail.operator)?,
            generated_number_descriptor_mekso_precedence_surface_text(&tail.right_expression)?
        );
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_mekso_base_surface_text(
    expression: &MeksoBaseSyntax,
) -> Result<String, SemanticsError> {
    match expression {
        MeksoBaseSyntax::MeksoOperand(operand) => {
            generated_number_descriptor_mekso_operand_surface_text(operand)
        }
        MeksoBaseSyntax::ForethoughtCallMekso(call) => {
            let mut parts = vec![generated_mekso_operator_surface_label(&call.operator)?];
            for operand in call.operands.iter() {
                parts.push(generated_number_descriptor_mekso_base_surface_text(
                    operand,
                )?);
            }
            Ok(parts.join(" "))
        }
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
            generated_zantufa_bo_grouped_mekso_base_surface_text(group)
        }
        MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(_) => Ok("mekso".to_owned()),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_number_descriptor_mekso_operand_surface_text(
    operand: &MeksoOperandSyntax,
) -> Result<String, SemanticsError> {
    if generated_mekso_operand_contains_operand_connection(operand) {
        Ok("mekso".to_owned())
    } else {
        generated_mekso_operand_surface_text(operand)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator(
    expression: &MeksoSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match expression {
        MeksoSyntax::ZantufaReversePolishMekso(_) => Ok(None),
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            first_generated_connected_mekso_operator_in_zantufa_infix(infix)
        }
        MeksoSyntax::InfixMekso(infix) => first_generated_connected_mekso_operator_in_infix(infix),
        MeksoSyntax::ReversePolishMekso(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_zantufa_infix(
    infix: &ZantufaInfixMeksoSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    for continuation in &infix.continuations {
        for operator in &continuation.operators {
            if let Some(expansion) = connected_generated_mekso_operator(operator)? {
                return Ok(Some(expansion));
            }
        }
    }
    if let Some(expansion) =
        first_generated_connected_mekso_operator_in_precedence(&infix.first_expression)?
    {
        return Ok(Some(expansion));
    }
    for continuation in &infix.continuations {
        if let Some(right_expression) = &continuation.right_expression
            && let Some(expansion) =
                first_generated_connected_mekso_operator_in_precedence(right_expression)?
        {
            return Ok(Some(expansion));
        }
    }
    Ok(None)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_infix(
    infix: &InfixMeksoSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    for continuation in &infix.continuations {
        if let Some(expansion) = connected_generated_mekso_operator(&continuation.operator)? {
            return Ok(Some(expansion));
        }
    }
    if let Some(expansion) =
        first_generated_connected_mekso_operator_in_precedence(&infix.first_expression)?
    {
        return Ok(Some(expansion));
    }
    for continuation in &infix.continuations {
        if let Some(expansion) =
            first_generated_connected_mekso_operator_in_precedence(&continuation.right_expression)?
        {
            return Ok(Some(expansion));
        }
    }
    Ok(None)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_precedence(
    expression: &MeksoPrecedenceSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    if let Some(tail) = &expression.tail {
        if let Some(expansion) = connected_generated_mekso_operator(&tail.operator)? {
            return Ok(Some(expansion));
        }
        if let Some(expansion) =
            first_generated_connected_mekso_operator_in_base(&expression.left_expression)?
        {
            return Ok(Some(expansion));
        }
        return first_generated_connected_mekso_operator_in_precedence(&tail.right_expression);
    }
    first_generated_connected_mekso_operator_in_base(&expression.left_expression)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_base(
    expression: &MeksoBaseSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match expression {
        MeksoBaseSyntax::MeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(operand)
        }
        MeksoBaseSyntax::ForethoughtCallMekso(call) => {
            if let Some(expansion) = connected_generated_mekso_operator(&call.operator)? {
                return Ok(Some(expansion));
            }
            for operand in &call.operands {
                if let Some(expansion) = first_generated_connected_mekso_operator_in_base(operand)?
                {
                    return Ok(Some(expansion));
                }
            }
            Ok(None)
        }
        MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => {
            for operand in &group.operands {
                if let Some(expansion) =
                    first_generated_connected_mekso_operator_in_operand(operand)?
                {
                    return Ok(Some(expansion));
                }
            }
            Ok(None)
        }
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
            if let Some(expansion) =
                first_generated_connected_mekso_operator_in_operand(&group.first)?
            {
                return Ok(Some(expansion));
            }
            for continuation in &group.continuations {
                if let Some(expansion) =
                    first_generated_connected_mekso_operator_in_operand(&continuation.expression)?
                {
                    return Ok(Some(expansion));
                }
            }
            Ok(None)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_operand(
    operand: &MeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operand {
        MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
            let chain = &operand.0;
            if let Some(expansion) =
                first_generated_connected_mekso_operator_in_bound_or_simple_operand(&chain.first)?
            {
                return Ok(Some(expansion));
            }
            for link in &chain.links {
                if let Some(expansion) =
                    first_generated_connected_mekso_operator_in_bound_or_simple_operand(
                        &link.trailing_expression,
                    )?
                {
                    return Ok(Some(expansion));
                }
            }
            Ok(None)
        }
        MeksoOperandSyntax::BoundMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_bound_operand(operand)
        }
        MeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_simple_operand(operand)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_bound_or_simple_operand(
    operand: &BoundOrSimpleMeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_bound_operand(operand)
        }
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_simple_operand(operand)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_bound_operand(
    operand: &BoundMeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    first_generated_connected_mekso_operator_in_simple_operand(&operand.left_expression)?
        .map_or_else(
            || first_generated_connected_mekso_operator_in_operand(&operand.right_expression),
            |expansion| Ok(Some(expansion)),
        )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_simple_operand(
    operand: &SimpleMeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operand {
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(&operand.left_expression)?
                .map_or_else(
                    || {
                        first_generated_connected_mekso_operator_in_operand(
                            &operand.right_expression,
                        )
                    },
                    |expansion| Ok(Some(expansion)),
                )
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            first_generated_connected_mekso_operator(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => {
            for expression in &operand.expressions {
                if let Some(expansion) = first_generated_connected_mekso_operator(expression)? {
                    return Ok(Some(expansion));
                }
            }
            Ok(None)
        }
        SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntax::NumberMekso(_)
        | SimpleMeksoOperandSyntax::LerfuStringMekso(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_mekso_operator(
    operator: &MeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
            connected_generated_afterthought_mekso_operator(operator)
        }
        MeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            connected_generated_bound_mekso_operator(operator)
        }
        MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            connected_generated_simple_mekso_operator(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_afterthought_mekso_operator(
    operator: &AfterthoughtMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    let chain = &operator.0;
    let Some(link) = chain.links.first() else {
        return connected_generated_bound_or_atom_mekso_operator(&chain.first);
    };
    connected_generated_standard_mekso_operator(
        &link.connective,
        generated_mekso_operator_from_bound_or_atom(&chain.first),
        generated_mekso_operator_from_bound_or_atom(&link.trailing_operator),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_bound_or_atom_mekso_operator(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            connected_generated_bound_mekso_operator(operator)
        }
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            connected_generated_simple_mekso_operator(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_bound_mekso_operator(
    operator: &BoundMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    connected_generated_standard_mekso_operator(
        &operator.connective,
        MeksoOperatorSyntax::SimpleMeksoOperator(operator.left_operator.as_ref().clone()),
        operator.right_operator.as_ref().clone(),
    )?
    .map_or_else(
        || connected_generated_simple_mekso_operator(&operator.left_operator),
        |expansion| Ok(Some(expansion)),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_simple_mekso_operator(
    operator: &SimpleMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operator {
        SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
            connected_generated_mekso_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
            connected_generated_mekso_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            connected_generated_mekso_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ForethoughtMeksoOperator(operator) => {
            connected_generated_forethought_mekso_operator(operator)
        }
        SimpleMeksoOperatorSyntax::PrimitiveMeksoOperator(_)
        | SimpleMeksoOperatorSyntax::ZantufaMahoSelbriMeksoOperator(_)
        | SimpleMeksoOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_)
        | SimpleMeksoOperatorSyntax::ZantufaConnectiveMeksoOperator(_)
        | SimpleMeksoOperatorSyntax::SelbriMeksoOperator(_)
        | SimpleMeksoOperatorSyntax::OperandMeksoOperator(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_standard_mekso_operator(
    connective: &StandardStatementConnectiveSyntax,
    left_operator: MeksoOperatorSyntax,
    right_operator: MeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    let connective = statement_connective_from_standard(connective);
    if !generated_statement_connective_is_logical(&connective)
        || generated_statement_connective_is_interval(&connective)
    {
        return Ok(None);
    }
    let source = generated_statement_connective_core_source(&connective)?;
    Ok(Some(new!(GeneratedConnectedMeksoOperatorExpansion {
        left_operator,
        right_operator,
        operator: generated_statement_connective_formula_operator_for_core(&connective),
        connector: new!(Connector {
            source,
            locus: "mekso-operator".to_owned(),
            truth_table: generated_statement_connective_core_truth_table(&connective),
            parameter: None,
        }),
    })))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == "mekso-operator")) || ret.is_err())]
pub(super) fn connected_generated_forethought_mekso_operator(
    operator: &ForethoughtMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    Ok(Some(new!(GeneratedConnectedMeksoOperatorExpansion {
        left_operator: operator.left_operator.as_ref().clone(),
        right_operator: operator.right_operator.as_ref().clone(),
        operator: generated_guhek_connective_formula_operator(&operator.guhek),
        connector: new!(Connector {
            source: generated_guhek_gik_connective_source(&operator.guhek, &operator.gik),
            locus: "mekso-operator".to_owned(),
            truth_table: generated_guhek_gik_connective_truth_table(&operator.guhek, &operator.gik),
            parameter: None,
        }),
    })))
}
