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
                token_list_text(visitor.tokens.iter().copied())
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
    let chain = &operand.connected_expression.0;
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
            generated_bound_or_simple_mekso_operand_surface_text(&link.trailing_expression)?
        } else if let Some(replaced_trailing) =
            generated_bound_or_simple_mekso_operand_surface_text_with_connected_operator_replacement(
                &link.trailing_expression,
                replacement_operator,
            )?
        {
            replaced = true;
            replaced_trailing
        } else {
            generated_bound_or_simple_mekso_operand_surface_text(&link.trailing_expression)?
        };
        text = format!(
            "{} {} {}",
            text,
            generated_operand_connective_source(&link.operand_connective),
            trailing
        );
    }
    if let Some(group) = &operand.grouped_continuation {
        let inner = if replaced {
            generated_mekso_operand_surface_text(&group.inner_expression)?
        } else if let Some(inner) =
            generated_mekso_operand_surface_text_with_connected_operator_replacement(
                &group.inner_expression,
                replacement_operator,
            )?
        {
            replaced = true;
            inner
        } else {
            generated_mekso_operand_surface_text(&group.inner_expression)?
        };
        text = format!(
            "{} {} {} {}",
            text,
            generated_operand_connective_source(&group.operand_connective),
            token_text(&group.ke.value),
            inner
        );
    }
    Ok(replaced.then_some(text))
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
            generated_bound_or_simple_mekso_operand_surface_text(&operand.right_expression)?
        )));
    }
    generated_bound_or_simple_mekso_operand_surface_text_with_connected_operator_replacement(
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
                    generated_simple_mekso_operand_surface_text(&operand.right_expression)?
                )));
            }
            generated_simple_mekso_operand_surface_text_with_connected_operator_replacement(
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
        SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_operand(
    operand: &MeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    let chain = &operand.connected_expression.0;
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
    operand
        .grouped_continuation
        .as_ref()
        .map_or(Ok(None), |group| {
            first_generated_connected_mekso_operator_in_operand(&group.inner_expression)
        })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_bound_operand(
    operand: &BoundMeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    first_generated_connected_mekso_operator_in_simple_operand(&operand.left_expression)?
        .map_or_else(
            || {
                first_generated_connected_mekso_operator_in_bound_or_simple_operand(
                    &operand.right_expression,
                )
            },
            |expansion| Ok(Some(expansion)),
        )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
pub(super) fn first_generated_connected_mekso_operator_in_simple_operand(
    operand: &SimpleMeksoOperandSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    match operand {
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(&operand.left_expression)?
                .map_or_else(
                    || {
                        first_generated_connected_mekso_operator_in_simple_operand(
                            &operand.right_expression,
                        )
                    },
                    |expansion| Ok(Some(expansion)),
                )
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            first_generated_connected_mekso_operator_in_operand(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
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
            source: ConnectorSource::surface_word(source),
            locus: ConnectorLocus::MathOperator,
            truth_table: generated_statement_connective_core_truth_table(&connective),
            parameter: None,
        }),
    })))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|expansion| expansion.as_ref().is_none_or(|expansion| expansion.connector.locus == ConnectorLocus::MathOperator)) || ret.is_err())]
pub(super) fn connected_generated_forethought_mekso_operator(
    operator: &ForethoughtMeksoOperatorSyntax,
) -> Result<Option<GeneratedConnectedMeksoOperatorExpansion>, SemanticsError> {
    Ok(Some(new!(GeneratedConnectedMeksoOperatorExpansion {
        left_operator: operator.left_operator.as_ref().clone(),
        right_operator: operator.right_operator.as_ref().clone(),
        operator: generated_guhek_connective_formula_operator(&operator.guhek),
        connector: new!(Connector {
            source: ConnectorSource::surface_word(generated_guhek_gik_connective_source(
                &operator.guhek,
                &operator.gik,
            )),
            locus: ConnectorLocus::MathOperator,
            truth_table: generated_guhek_gik_connective_truth_table(&operator.guhek, &operator.gik),
            parameter: None,
        }),
    })))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_mekso_operator_from_bound_or_atom(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> MeksoOperatorSyntax {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(operator) => {
            MeksoOperatorSyntax::BoundMeksoOperator(operator.clone())
        }
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            MeksoOperatorSyntax::SimpleMeksoOperator(operator.clone())
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_mekso_contains_operand_connection(expression: &MeksoSyntax) -> bool {
    match expression {
        MeksoSyntax::ZantufaReversePolishMekso(reverse_polish) => {
            generated_zantufa_reverse_polish_contains_operand_connection(reverse_polish)
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            generated_mekso_precedence_contains_operand_connection(&infix.first_expression)
                || infix.continuations.iter().any(|continuation| {
                    continuation.right_expression.as_ref().is_some_and(|right| {
                        generated_mekso_precedence_contains_operand_connection(right)
                    })
                })
        }
        MeksoSyntax::InfixMekso(infix) => {
            generated_mekso_precedence_contains_operand_connection(&infix.first_expression)
                || infix.continuations.iter().any(|continuation| {
                    generated_mekso_precedence_contains_operand_connection(
                        &continuation.right_expression,
                    )
                })
        }
        MeksoSyntax::ReversePolishMekso(reverse_polish) => {
            generated_reverse_polish_parts_contains_operand_connection(&reverse_polish.parts)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_zantufa_reverse_polish_contains_operand_connection(
    reverse_polish: &ZantufaReversePolishMeksoSyntax,
) -> bool {
    reverse_polish
        .operands
        .iter()
        .any(generated_mekso_base_contains_operand_connection)
        || reverse_polish.tails.iter().any(|tail| {
            tail.operands
                .iter()
                .any(generated_mekso_base_contains_operand_connection)
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_reverse_polish_parts_contains_operand_connection(
    parts: &ReversePolishPartsSyntax,
) -> bool {
    generated_mekso_operand_contains_operand_connection(&parts.first_operand)
        || parts.tails.iter().any(|tail| {
            generated_reverse_polish_parts_contains_operand_connection(&tail.right_parts)
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_mekso_precedence_contains_operand_connection(
    expression: &MeksoPrecedenceSyntax,
) -> bool {
    generated_mekso_base_contains_operand_connection(&expression.left_expression)
        || expression.tail.as_ref().is_some_and(|tail| {
            generated_mekso_precedence_contains_operand_connection(&tail.right_expression)
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_mekso_base_contains_operand_connection(
    expression: &MeksoBaseSyntax,
) -> bool {
    match expression {
        MeksoBaseSyntax::MeksoOperand(operand) => {
            generated_mekso_operand_contains_operand_connection(operand)
        }
        MeksoBaseSyntax::ForethoughtCallMekso(call) => call
            .operands
            .iter()
            .any(generated_mekso_base_contains_operand_connection),
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
            generated_mekso_operand_contains_operand_connection(&group.first)
                || group.continuations.iter().any(|continuation| {
                    generated_mekso_operand_contains_operand_connection(&continuation.expression)
                })
        }
        MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => group
            .operands
            .iter()
            .any(|operand| generated_mekso_operand_contains_operand_connection(operand)),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_mekso_operand_contains_operand_connection(
    operand: &MeksoOperandSyntax,
) -> bool {
    let connected = &operand.connected_expression.0;
    operand.grouped_continuation.is_some()
        || !connected.links.is_empty()
        || generated_bound_or_simple_mekso_operand_contains_operand_connection(
            connected.first.as_ref(),
        )
        || connected.links.iter().any(|link| {
            generated_bound_or_simple_mekso_operand_contains_operand_connection(
                &link.trailing_expression,
            )
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bound_or_simple_mekso_operand_contains_operand_connection(
    operand: &BoundOrSimpleMeksoOperandSyntax,
) -> bool {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
            generated_bound_mekso_operand_contains_operand_connection(operand)
        }
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_contains_operand_connection(operand)
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_bound_mekso_operand_contains_operand_connection(
    operand: &BoundMeksoOperandSyntax,
) -> bool {
    generated_simple_mekso_operand_contains_operand_connection(&operand.left_expression)
        || generated_bound_or_simple_mekso_operand_contains_operand_connection(
            &operand.right_expression,
        )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_mekso_operand_contains_operand_connection(
    operand: &SimpleMeksoOperandSyntax,
) -> bool {
    match operand {
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
            generated_mekso_operand_contains_operand_connection(&operand.left_expression)
                || generated_simple_mekso_operand_contains_operand_connection(
                    &operand.right_expression,
                )
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            generated_mekso_operand_contains_operand_connection(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
            generated_mekso_operand_contains_operand_connection(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => {
            generated_mekso_operand_contains_operand_connection(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_mekso_contains_operand_connection(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => operand
            .expressions
            .iter()
            .any(generated_mekso_contains_operand_connection),
        SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntax::NumberMekso(_)
        | SimpleMeksoOperandSyntax::LerfuStringMekso(_) => false,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_infix_mekso_surface_text(
    infix: &InfixMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut text = generated_mekso_precedence_surface_text(&infix.first_expression)?;
    for continuation in &infix.continuations {
        text = format!(
            "{} {} {}",
            text,
            generated_mekso_operator_surface_label(&continuation.operator)?,
            generated_mekso_precedence_surface_text(&continuation.right_expression)?
        );
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_zantufa_infix_mekso_surface_text(
    infix: &ZantufaInfixMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut text = generated_mekso_precedence_surface_text(&infix.first_expression)?;
    for continuation in &infix.continuations {
        let mut parts = Vec::with_capacity(continuation.operators.len() + 1);
        for operator in &continuation.operators {
            parts.push(generated_mekso_operator_surface_label(operator)?);
        }
        if let Some(right_expression) = &continuation.right_expression {
            parts.push(generated_mekso_precedence_surface_text(right_expression)?);
        }
        text = format!("{} {}", text, parts.join(" "));
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_precedence_surface_text(
    expression: &MeksoPrecedenceSyntax,
) -> Result<String, SemanticsError> {
    let mut text = generated_mekso_base_surface_text(&expression.left_expression)?;
    if let Some(tail) = &expression.tail {
        text = format!(
            "{} {} {}",
            text,
            generated_mekso_operator_surface_label(&tail.operator)?,
            generated_mekso_precedence_surface_text(&tail.right_expression)?
        );
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_base_surface_text(
    expression: &MeksoBaseSyntax,
) -> Result<String, SemanticsError> {
    match expression {
        MeksoBaseSyntax::MeksoOperand(operand) => generated_mekso_operand_surface_text(operand),
        MeksoBaseSyntax::ForethoughtCallMekso(call) => {
            let mut parts = vec![generated_mekso_operator_surface_label(&call.operator)?];
            for operand in call.operands.iter() {
                parts.push(generated_mekso_base_surface_text(operand)?);
            }
            Ok(parts.join(" "))
        }
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
            generated_zantufa_bo_grouped_mekso_base_surface_text(group)
        }
        MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => {
            let mut parts = Vec::with_capacity(group.operands.len() + 2);
            parts.push(token_text(&group.ke.value));
            for operand in &group.operands {
                parts.push(generated_mekso_operand_surface_text(operand)?);
            }
            if let Some(kehe) = &group.kehe {
                parts.push(token_text(&kehe.value));
            }
            Ok(parts.join(" "))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_zantufa_bo_grouped_mekso_base_surface_text(
    group: &ZantufaBoGroupedMeksoBaseSyntax,
) -> Result<String, SemanticsError> {
    let mut parts = Vec::with_capacity(1 + group.continuations.len() * 2);
    parts.push(generated_mekso_operand_surface_text(&group.first)?);
    for continuation in &group.continuations {
        parts.push(token_text(&continuation.bo.value));
        parts.push(generated_mekso_operand_surface_text(
            &continuation.expression,
        )?);
    }
    Ok(parts.join(" "))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_mekso_operand_surface_text(
    operand: &MeksoOperandSyntax,
) -> Result<String, SemanticsError> {
    let chain = &operand.connected_expression.0;
    let mut text = generated_bound_or_simple_mekso_operand_surface_text(&chain.first)?;
    for link in &chain.links {
        text = format!(
            "{} {} {}",
            text,
            generated_operand_connective_source(&link.operand_connective),
            generated_bound_or_simple_mekso_operand_surface_text(&link.trailing_expression)?
        );
    }
    if let Some(group) = &operand.grouped_continuation {
        text = format!(
            "{} {} {} {}",
            text,
            generated_operand_connective_source(&group.operand_connective),
            token_text(&group.ke.value),
            generated_mekso_operand_surface_text(&group.inner_expression)?
        );
    }
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_or_simple_mekso_operand_surface_text(
    operand: &BoundOrSimpleMeksoOperandSyntax,
) -> Result<String, SemanticsError> {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
            generated_bound_mekso_operand_surface_text(operand)
        }
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_surface_text(operand)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_bound_mekso_operand_surface_text(
    operand: &BoundMeksoOperandSyntax,
) -> Result<String, SemanticsError> {
    Ok(format!(
        "{} {} {}",
        generated_simple_mekso_operand_surface_text(&operand.left_expression)?,
        generated_operand_connective_source(&operand.operand_connective),
        generated_bound_or_simple_mekso_operand_surface_text(&operand.right_expression)?
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_simple_mekso_operand_surface_text(
    operand: &SimpleMeksoOperandSyntax,
) -> Result<String, SemanticsError> {
    match operand {
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => Ok(format!(
            "{} {} {}",
            generated_modal_forethought_connective_source(&operand.gek),
            generated_mekso_operand_surface_text(&operand.left_expression)?,
            generated_simple_mekso_operand_surface_text(&operand.right_expression)?
        )),
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            generated_mekso_operand_surface_text(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
            generated_mekso_operand_surface_text(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => Ok(format!(
            "{} {}",
            token_text(&operand.nahe.value),
            generated_mekso_operand_surface_text(&operand.inner_expression)?
        )),
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_mekso_surface_text(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::SumtiMeksoOperand(operand) => {
            let mut visitor = GeneratedSpanCollector::default();
            operand.sumti.visit_in_order(&mut visitor);
            Ok(format!(
                "{} {}",
                token_text(&operand.mohe.value),
                token_list_text(visitor.tokens.iter().copied())
            ))
        }
        SimpleMeksoOperandSyntax::SelbriMeksoOperand(operand) => Ok(format!(
            "{} {}",
            token_text(&operand.nihe.value),
            relation_label_from_selbri(&operand.selbri)?.display_text()
        )),
        SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(operand) => Ok(format!(
            "{} {}",
            token_text(&operand.mohe.value),
            relation_label_from_selbri(&operand.selbri)?.display_text()
        )),
        SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => operand
            .expressions
            .iter()
            .map(generated_mekso_surface_text)
            .collect::<Result<Vec<_>, _>>()
            .map(|parts| parts.join(" ")),
        SimpleMeksoOperandSyntax::NumberMekso(number) => {
            Ok(generated_number_words_text(&number.0.number.value))
        }
        SimpleMeksoOperandSyntax::LerfuStringMekso(letter) => {
            Ok(generated_letter_string_text(&letter.letters))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_reverse_polish_surface_text(
    reverse_polish: &ReversePolishMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    reverse_polish.visit_in_order(&mut visitor);
    non_empty_token_list_text(visitor.tokens.iter().copied())
        .ok_or_else(|| invalid_graph("generated reverse Polish mekso has no tokens".to_owned()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(super) fn generated_zantufa_reverse_polish_surface_text(
    reverse_polish: &ZantufaReversePolishMeksoSyntax,
) -> Result<String, SemanticsError> {
    let mut visitor = GeneratedSpanCollector::default();
    reverse_polish.visit_in_order(&mut visitor);
    non_empty_token_list_text(visitor.tokens.iter().copied()).ok_or_else(|| {
        invalid_graph("generated Zantufa reverse Polish mex has no tokens".to_owned())
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_number_words_text(words: &NumberWordsSyntax) -> String {
    let mut tokens = vec![words.first_number.clone()];
    for continuation in &words.continuations {
        match continuation {
            NumberWordContinuationSyntax::NumberWordPaContinuation(continuation) => {
                tokens.push(continuation.0.clone());
            }
            NumberWordContinuationSyntax::NumberWordLerfuContinuation(continuation) => {
                tokens.extend(generated_letter_tokens(&continuation.0));
            }
        }
    }
    token_list_text(tokens.iter())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_letter_string_text(letters: &LetterStringSyntax) -> String {
    let tokens = generated_letter_string_tokens(letters);
    generated_math_letteral_text(&tokens)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_math_letteral_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(generated_math_letteral_token_text)
        .collect::<Vec<_>>()
        .join("")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_math_letteral_token_text(token: &Token) -> String {
    match token.core_word().as_data() {
        data!(WordLike::PlainWord(word)) => generated_math_letteral_word_text(word),
        data!(WordLike::LerfuWord { base, .. }) => generated_math_letteral_word_like_text(base),
        _ => token_text(token),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_math_letteral_word_like_text(word_like: &WordLike) -> String {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => generated_math_letteral_word_text(word),
        data!(WordLike::LerfuWord { base, .. }) => generated_math_letteral_word_like_text(base),
        _ => word_like.to_string(),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_math_letteral_word_text(word: &Word) -> String {
    match word.cmavo() {
        Some(Cmavo::A) => "a".to_owned(),
        Some(Cmavo::By) => "b".to_owned(),
        Some(Cmavo::Cy) => "c".to_owned(),
        Some(Cmavo::Xy) => "x".to_owned(),
        _ => word_text(word),
    }
}

#[requires(true)]
#[ensures(!ret.iter().any(|key| key.is_empty()))]
pub(super) fn generated_argument_letter_keys(sumti: &SumtiSyntax) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(base_letter) = generated_argument_letter_base(sumti) {
        keys.push(base_letter);
    }
    if let Some(initials) = generated_argument_name_initials(sumti)
        && !keys.iter().any(|key| key == &initials)
    {
        keys.push(initials);
    }
    keys
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_argument_name_initials(sumti: &SumtiSyntax) -> Option<String> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    let SumtiAtomSyntax::SumtiBase(base) = simple.base_sumti.as_ref() else {
        return None;
    };
    let SumtiBaseSyntax::NameSumti(name) = base else {
        return None;
    };
    generated_word_run_initial_key(&name.names.value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_argument_letter_base(sumti: &SumtiSyntax) -> Option<String> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    match simple.base_sumti.as_ref() {
        SumtiAtomSyntax::SumtiBase(base) => generated_argument_letter_base_from_sumti_base(base),
        SumtiAtomSyntax::QuantifiedSumti(quantified) => {
            generated_argument_letter_base_from_sumti_base(&quantified.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_argument_letter_base_from_sumti_base(
    sumti: &SumtiBaseSyntax,
) -> Option<String> {
    match sumti {
        SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        SumtiBaseSyntax::DescriptionConnectionSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        SumtiBaseSyntax::DescriptorWithoutGadriSumti(description) => {
            generated_selbri_base_letter(&description.selbri)
        }
        SumtiBaseSyntax::NameSumti(name) => generated_token_base_letter(name.names.value.first()),
        SumtiBaseSyntax::LaheSumti(sumti) => generated_argument_letter_base(&sumti.inner_sumti),
        SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
            generated_argument_letter_base(&sumti.inner_sumti)
        }
        SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
            generated_argument_letter_base(&sumti.inner_sumti)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_description_tail_base_letter(
    tail: &DescriptionTailSyntax,
) -> Option<String> {
    if let Some(tail_sumti) = &tail.leading_tail_elements.tail_sumti
        && let Some(letter) = generated_argument_letter_base_from_sumti_base(tail_sumti.0.as_ref())
    {
        return Some(letter);
    }
    match tail.tail.as_ref() {
        DescriptionTailBodySyntax::RelationDescriptionTail(tail) => {
            generated_selbri_base_letter(&tail.selbri)
        }
        DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(tail) => {
            generated_selbri_base_letter(&tail.selbri)
        }
        DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(tail) => {
            generated_argument_letter_base(&tail.sumti)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_selbri_base_letter(selbri: &SelbriSyntax) -> Option<String> {
    let label = generated_pro_bridi_target_relation_label(selbri)
        .ok()
        .flatten()?;
    generated_text_base_letter(&label.display_text())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_letter_string_initial_key(letters: &LetterStringSyntax) -> Option<String> {
    let tokens = generated_letter_string_tokens(letters);
    generated_token_run_initial_key(&tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_word_run_initial_key(tokens: &[Token]) -> Option<String> {
    if tokens.len() <= 1 {
        return None;
    }
    generated_token_run_initial_key(tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_token_run_initial_key(tokens: &[Token]) -> Option<String> {
    let initials = tokens
        .iter()
        .map(generated_token_base_letter)
        .collect::<Option<Vec<_>>>()?
        .join("");
    (!initials.is_empty()).then_some(initials)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_token_base_letter(token: &Token) -> Option<String> {
    generated_word_like_base_letter(token.core_word())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_word_like_base_letter(word_like: &WordLike) -> Option<String> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => generated_word_base_letter(word),
        data!(WordLike::LerfuWord { base, .. }) => generated_word_like_base_letter(base),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_word_base_letter(word: &Word) -> Option<String> {
    generated_text_base_letter(&word_text(word))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
pub(super) fn generated_text_base_letter(text: &str) -> Option<String> {
    text.chars()
        .find(|character| character.is_alphabetic())
        .map(|character| character.to_string())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_letter_string_tokens(letters: &LetterStringSyntax) -> Vec<Token> {
    let mut tokens = generated_letter_tokens(&letters.first_letter);
    for continuation in &letters.continuations {
        match continuation {
            LetterStringContinuationSyntax::LetterStringPaContinuation(continuation) => {
                tokens.push(continuation.0.clone());
            }
            LetterStringContinuationSyntax::LetterStringLerfuContinuation(continuation) => {
                tokens.extend(generated_letter_tokens(&continuation.0));
            }
        }
    }
    tokens
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn generated_letter_tokens(letter: &LetterTokensSyntax) -> Vec<Token> {
    match letter {
        LetterTokensSyntax::SimpleLerfuWord(word) => vec![word.0.clone()],
        LetterTokensSyntax::LauLerfuWord(word) => {
            let mut tokens = vec![word.lau.clone()];
            tokens.extend(generated_letter_tokens(&word.letter));
            tokens
        }
        LetterTokensSyntax::TeiLerfuWord(word) => {
            let mut tokens = vec![word.tei.clone()];
            tokens.extend(generated_letter_string_tokens(&word.letters));
            tokens.push(word.foi.clone());
            tokens
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_pa_quantity_value_for_mekso(
    expression: &MeksoSyntax,
) -> Option<QuantityValue> {
    let text = generated_mekso_number_words_text(expression)?;
    parse_generated_relational_pa_integer(&text)
        .map(QuantityValue::integer)
        .or_else(|| parse_generated_simple_pa_integer(&text).map(QuantityValue::integer))
        .or_else(|| parse_generated_simple_pa_decimal(&text).map(QuantityValue::text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_pa_quantity_value_for_mekso_operand(
    expression: &MeksoOperandSyntax,
) -> Option<QuantityValue> {
    let text = generated_mekso_operand_number_words_text(expression)?;
    parse_generated_relational_pa_integer(&text)
        .map(QuantityValue::integer)
        .or_else(|| parse_generated_simple_pa_integer(&text).map(QuantityValue::integer))
        .or_else(|| parse_generated_simple_pa_decimal(&text).map(QuantityValue::text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_pa_quantity_value_for_bound_or_simple_mekso_operand(
    expression: &BoundOrSimpleMeksoOperandSyntax,
) -> Option<QuantityValue> {
    let text = generated_bound_or_simple_mekso_operand_number_words_text(expression)?;
    parse_generated_relational_pa_integer(&text)
        .map(QuantityValue::integer)
        .or_else(|| parse_generated_simple_pa_integer(&text).map(QuantityValue::integer))
        .or_else(|| parse_generated_simple_pa_decimal(&text).map(QuantityValue::text))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_simple_pa_quantity_value_for_simple_mekso_operand(
    expression: &SimpleMeksoOperandSyntax,
) -> Option<QuantityValue> {
    let text = generated_simple_mekso_operand_number_words_text(expression)?;
    parse_generated_relational_pa_integer(&text)
        .map(QuantityValue::integer)
        .or_else(|| parse_generated_simple_pa_integer(&text).map(QuantityValue::integer))
        .or_else(|| parse_generated_simple_pa_decimal(&text).map(QuantityValue::text))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn generated_mekso_number_words_text(expression: &MeksoSyntax) -> Option<String> {
    match expression {
        MeksoSyntax::InfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            generated_mekso_precedence_number_words_text(&infix.first_expression)
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            generated_mekso_precedence_number_words_text(&infix.first_expression)
        }
        MeksoSyntax::ZantufaReversePolishMekso(_) | MeksoSyntax::ReversePolishMekso(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn generated_mekso_precedence_number_words_text(
    expression: &MeksoPrecedenceSyntax,
) -> Option<String> {
    if expression.tail.is_some() {
        return None;
    }
    let MeksoBaseSyntax::MeksoOperand(operand) = expression.left_expression.as_ref() else {
        return None;
    };
    generated_mekso_operand_number_words_text(operand)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn generated_mekso_operand_number_words_text(
    operand: &MeksoOperandSyntax,
) -> Option<String> {
    let chain = &operand.connected_expression.0;
    if operand.grouped_continuation.is_some() || !chain.links.is_empty() {
        return None;
    }
    generated_bound_or_simple_mekso_operand_number_words_text(&chain.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn generated_bound_or_simple_mekso_operand_number_words_text(
    operand: &BoundOrSimpleMeksoOperandSyntax,
) -> Option<String> {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => None,
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_number_words_text(operand)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub(super) fn generated_simple_mekso_operand_number_words_text(
    operand: &SimpleMeksoOperandSyntax,
) -> Option<String> {
    match operand {
        SimpleMeksoOperandSyntax::NumberMekso(number) => {
            Some(generated_number_words_text(&number.0.number.value))
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_mekso_number_words_text(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_) => None,
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_forethought_mekso_operand_from_mekso(
    expression: &MeksoSyntax,
) -> Option<&ForethoughtMeksoOperandSyntax> {
    let operand = generated_single_mekso_operand_from_mekso(expression)?;
    if operand.grouped_continuation.is_some() {
        return None;
    }
    let connected = &operand.connected_expression.0;
    if !connected.links.is_empty() {
        return None;
    }
    let BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand),
    ) = &*connected.first
    else {
        return None;
    };
    Some(operand)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_parenthesized_mekso_operand_from_mekso(
    expression: &MeksoSyntax,
) -> Option<&ParenthesizedMeksoOperandSyntax> {
    let operand = generated_single_mekso_operand_from_mekso(expression)?;
    if operand.grouped_continuation.is_some() {
        return None;
    }
    let connected = &operand.connected_expression.0;
    if !connected.links.is_empty() {
        return None;
    }
    let BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand),
    ) = &*connected.first
    else {
        return None;
    };
    Some(operand)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn generated_single_mekso_operand_from_mekso(
    expression: &MeksoSyntax,
) -> Option<&MeksoOperandSyntax> {
    let first_expression = match expression {
        MeksoSyntax::InfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        MeksoSyntax::ZantufaReversePolishMekso(_) | MeksoSyntax::ReversePolishMekso(_) => {
            return None;
        }
    };
    let MeksoPrecedenceSyntax {
        left_expression,
        tail,
    } = &**first_expression;
    if tail.is_some() {
        return None;
    }
    let MeksoBaseSyntax::MeksoOperand(operand) = left_expression.as_ref() else {
        return None;
    };
    Some(operand)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.0.is_empty()))]
pub(super) fn generated_mekso_letteral_tokens<'syntax>(
    expression: &'syntax MeksoSyntax,
) -> Option<(Vec<Token>, Option<&'syntax [FreeModifierSyntax]>)> {
    match expression {
        MeksoSyntax::InfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            generated_mekso_precedence_letteral_tokens(&infix.first_expression)
        }
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            generated_mekso_precedence_letteral_tokens(&infix.first_expression)
        }
        MeksoSyntax::ZantufaReversePolishMekso(_) | MeksoSyntax::ReversePolishMekso(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|name| !name.is_empty()))]
pub(super) fn generated_math_variable_name(expression: &MeksoSyntax) -> Option<String> {
    generated_mekso_letteral_tokens(expression)
        .map(|(tokens, _free_modifiers)| generated_math_letteral_text(&tokens))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.0.is_empty()))]
pub(super) fn generated_mekso_precedence_letteral_tokens<'syntax>(
    expression: &'syntax MeksoPrecedenceSyntax,
) -> Option<(Vec<Token>, Option<&'syntax [FreeModifierSyntax]>)> {
    if expression.tail.is_some() {
        return None;
    }
    let MeksoBaseSyntax::MeksoOperand(operand) = expression.left_expression.as_ref() else {
        return None;
    };
    generated_mekso_operand_letteral_tokens(operand)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.0.is_empty()))]
pub(super) fn generated_mekso_operand_letteral_tokens<'syntax>(
    operand: &'syntax MeksoOperandSyntax,
) -> Option<(Vec<Token>, Option<&'syntax [FreeModifierSyntax]>)> {
    let chain = &operand.connected_expression.0;
    if operand.grouped_continuation.is_some() || !chain.links.is_empty() {
        return None;
    }
    generated_bound_or_simple_mekso_operand_letteral_tokens(&chain.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.0.is_empty()))]
pub(super) fn generated_bound_or_simple_mekso_operand_letteral_tokens<'syntax>(
    operand: &'syntax BoundOrSimpleMeksoOperandSyntax,
) -> Option<(Vec<Token>, Option<&'syntax [FreeModifierSyntax]>)> {
    match operand {
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => None,
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_letteral_tokens(operand)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|tokens| !tokens.0.is_empty()))]
pub(super) fn generated_simple_mekso_operand_letteral_tokens<'syntax>(
    operand: &'syntax SimpleMeksoOperandSyntax,
) -> Option<(Vec<Token>, Option<&'syntax [FreeModifierSyntax]>)> {
    match operand {
        SimpleMeksoOperandSyntax::LerfuStringMekso(letter) => Some((
            generated_letter_string_tokens(&letter.letters),
            Some(&letter.free_modifiers),
        )),
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_mekso_letteral_tokens(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
            generated_mekso_operand_letteral_tokens(&operand.inner_expression)
        }
        SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
            generated_mekso_operand_letteral_tokens(&operand.inner_expression)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn generated_math_operator_label(
    operator: &MeksoOperatorSyntax,
) -> Result<MathOperator, SemanticsError> {
    let source = generated_mekso_operator_label(operator)?;
    Ok(match source.as_str() {
        "su'i" => new!(MathOperator::Add),
        "pi'i" => new!(MathOperator::Multiply),
        "te'a" => new!(MathOperator::Power),
        "vu'u" => new!(MathOperator::Subtract),
        "fe'i" => new!(MathOperator::Divide),
        "ju'u" => new!(MathOperator::Base),
        _ => MathOperator::from_label(source),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_operator(
    operator: &MeksoOperatorSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
            generated_math_operator_question_token_for_afterthought_operator(operator)
        }
        MeksoOperatorSyntax::BoundMeksoOperator(_) => Ok(None),
        MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_math_operator_question_token_for_simple_operator(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_afterthought_operator(
    operator: &AfterthoughtMeksoOperatorSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    if !operator.0.links.is_empty() {
        return Ok(None);
    }
    generated_math_operator_question_token_for_bound_or_atom(operator.0.first.as_ref())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_bound_or_atom(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(_) => Ok(None),
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_math_operator_question_token_for_simple_operator(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_simple_operator(
    operator: &SimpleMeksoOperatorSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    match operator {
        SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
            generated_math_operator_question_token_for_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
            generated_math_operator_question_token_for_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            generated_math_operator_question_token_for_operator(&operator.inner_operator)
        }
        SimpleMeksoOperatorSyntax::SelbriMeksoOperator(operator) => {
            generated_math_operator_question_token_for_selbri(&operator.selbri)
        }
        _ => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_selbri(
    selbri: &SelbriSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    match selbri {
        SelbriSyntax::TaggedSelbri(tagged) => {
            generated_math_operator_question_token_for_untagged_selbri(&tagged.inner_selbri)
        }
        SelbriSyntax::UntaggedSelbri(untagged) => {
            generated_math_operator_question_token_for_untagged_selbri(untagged)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|token| token.is_none_or(|token| token.cmavo() == Some(Cmavo::Mo))) || ret.is_err())]
pub(super) fn generated_math_operator_question_token_for_untagged_selbri(
    selbri: &UntaggedSelbriSyntax,
) -> Result<Option<&Token>, SemanticsError> {
    match selbri {
        UntaggedSelbriSyntax::CoSelbri(co_selbri) => {
            relation_question_syntax_from_co_selbri(co_selbri)
                .map(|question| question.map(generated_relation_question_token))
        }
        _ => Ok(None),
    }
}

#[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
#[ensures(ret.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
pub(super) fn generated_math_operands_for_operator(
    operator: &MeksoOperatorSyntax,
    operands: Vec<SemanticObjectId>,
) -> Vec<SemanticObjectId> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) if operator.0.links.is_empty() => {
            generated_math_operands_for_bound_or_atom_operator(operator.0.first.as_ref(), operands)
        }
        MeksoOperatorSyntax::BoundMeksoOperator(_) => operands,
        MeksoOperatorSyntax::SimpleMeksoOperator(
            SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator),
        ) => converted_math_operands_for_generated(operator.se.value.cmavo(), operands),
        MeksoOperatorSyntax::SimpleMeksoOperator(
            SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator),
        ) => generated_math_operands_for_operator(&operator.inner_operator, operands),
        MeksoOperatorSyntax::SimpleMeksoOperator(
            SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator),
        ) => generated_math_operands_for_operator(&operator.inner_operator, operands),
        _ => operands,
    }
}

#[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
#[ensures(ret.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
pub(super) fn generated_math_operands_for_bound_or_atom_operator(
    operator: &BoundOrAtomMeksoOperatorSyntax,
    operands: Vec<SemanticObjectId>,
) -> Vec<SemanticObjectId> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(_) => operands,
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            generated_math_operands_for_simple_operator(operator, operands)
        }
    }
}

#[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
#[ensures(ret.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
pub(super) fn generated_math_operands_for_simple_operator(
    operator: &SimpleMeksoOperatorSyntax,
    operands: Vec<SemanticObjectId>,
) -> Vec<SemanticObjectId> {
    match operator {
        SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
            converted_math_operands_for_generated(operator.se.value.cmavo(), operands)
        }
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
            generated_math_operands_for_operator(&operator.inner_operator, operands)
        }
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            generated_math_operands_for_operator(&operator.inner_operator, operands)
        }
        _ => operands,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
pub(super) fn scalar_negation_for_generated_mekso_operator(
    operator: &MeksoOperatorSyntax,
) -> Option<ScalarNegation> {
    match operator {
        MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) if operator.0.links.is_empty() => {
            scalar_negation_for_generated_bound_or_atom_mekso_operator(operator.0.first.as_ref())
        }
        MeksoOperatorSyntax::BoundMeksoOperator(_) => None,
        MeksoOperatorSyntax::SimpleMeksoOperator(
            SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator),
        ) => Some(scalar_negation_for_token(&operator.nahe.value)),
        MeksoOperatorSyntax::SimpleMeksoOperator(
            SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator),
        ) => scalar_negation_for_generated_mekso_operator(&operator.inner_operator),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
pub(super) fn scalar_negation_for_generated_bound_or_atom_mekso_operator(
    operator: &BoundOrAtomMeksoOperatorSyntax,
) -> Option<ScalarNegation> {
    match operator {
        BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(_) => None,
        BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
            scalar_negation_for_generated_simple_mekso_operator(operator)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|negation| !negation.introduced_by.is_empty()))]
pub(super) fn scalar_negation_for_generated_simple_mekso_operator(
    operator: &SimpleMeksoOperatorSyntax,
) -> Option<ScalarNegation> {
    match operator {
        SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
            Some(scalar_negation_for_token(&operator.nahe.value))
        }
        SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
            scalar_negation_for_generated_mekso_operator(&operator.inner_operator)
        }
        _ => None,
    }
}

#[requires(operands.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
#[ensures(ret.iter().all(|operand| operand.object_kind() == crate::model::SemanticObjectKind::MathExpression))]
pub(super) fn converted_math_operands_for_generated(
    cmavo: Option<Cmavo>,
    mut operands: Vec<SemanticObjectId>,
) -> Vec<SemanticObjectId> {
    let Some(target_index) = cmavo.and_then(se_conversion_target_index) else {
        return operands;
    };
    if target_index < operands.len() {
        operands.swap(0, target_index);
    }
    operands
}

#[requires(true)]
#[ensures(ret.is_none_or(|index| index > 0))]
pub(super) fn se_conversion_target_index(cmavo: Cmavo) -> Option<usize> {
    match cmavo {
        Cmavo::Se => Some(1),
        Cmavo::Te => Some(2),
        Cmavo::Ve => Some(3),
        Cmavo::Xe => Some(4),
        _ => None,
    }
}
