//! Typed ownership classification for the Zantufa-priority `mex` route.
//!
//! The Zantufa grammar must run first to see extensions that begin with an
//! otherwise baseline-parseable operand. Completed trees that use only the
//! baseline surface inventory are rejected here and reparsed by the baseline
//! alternatives. Every descended product is destructured exhaustively and
//! without `..`: adding a token-consuming field must force this proof to be
//! revisited. A rejected tree is composed exclusively from the same terminal
//! surfaces and greedy infix extent as the baseline rules, so its baseline
//! reparse consumes the identical extent. The one deliberate exception is a
//! wide-qualified head whose entire inner expression is baseline-shaped:
//! owner policy rejects that Zantufa reading so an elided `LUhU` reparses with
//! the narrower baseline qualifier and the inner remainder becomes outer
//! infix syntax at the same extent. With explicit `LUhU`, that rejection can
//! leave an upstream-accepted surface unavailable in the warning union; the
//! reinterpretation feature retains the faithful wide reading. If any part of
//! the inner expression is Zantufa-only, the baseline grammar cannot accept the
//! surface, so retaining the wide reading cannot reinterpret a baseline parse.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    ZantufaBiheMeksoTailSyntax, ZantufaBoGroupedMeksoSyntax, ZantufaForethoughtMeksoSyntax,
    ZantufaMex1Syntax, ZantufaMex2Syntax, ZantufaMexContinuationSyntax, ZantufaMexGroupSyntax,
    ZantufaMexSyntax, ZantufaOperandSyntax, ZantufaOperatorSyntax, ZantufaReversePolishMeksoSyntax,
    recovered,
};
use super::generated_runtime::OutputRejection;

#[requires(true)]
#[ensures(true)]
fn is_baseline_mex(expression: &ZantufaMexSyntax) -> bool {
    let ZantufaMexSyntax {
        first_expression,
        continuations,
    } = expression;
    if baseline_operand_mex(expression)
        || wide_qualified_head_has_baseline_inner(first_expression)
        || continuations.is_empty() && baseline_root_reverse_polish(first_expression)
    {
        return true;
    }
    baseline_precedence(first_expression)
        && continuations.iter().all(|continuation| {
            let ZantufaMexContinuationSyntax {
                operators,
                right_expression,
            } = continuation;
            operators.len() == 1
                && baseline_operator(&operators[0])
                && right_expression.as_deref().is_some_and(baseline_precedence)
        })
}

#[requires(true)]
#[ensures(true)]
fn baseline_precedence(expression: &ZantufaMex1Syntax) -> bool {
    let ZantufaMex1Syntax { first_group, tails } = expression;
    baseline_group_as_base(first_group)
        && tails.iter().all(|tail| {
            let ZantufaBiheMeksoTailSyntax {
                bihe: _,
                operators,
                right_group,
            } = tail;
            operators.len() == 1
                && baseline_operator(&operators[0])
                && right_group.as_deref().is_some_and(baseline_group_as_base)
        })
}

#[requires(true)]
#[ensures(true)]
fn baseline_group_as_base(group: &ZantufaMexGroupSyntax) -> bool {
    match group {
        ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(_) => false,
        ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) => {
            let ZantufaBoGroupedMeksoSyntax {
                first_expression,
                continuations,
            } = group;
            continuations.is_empty() && baseline_atom_as_base(first_expression.as_ref())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_atom_as_base(expression: &ZantufaMex2Syntax) -> bool {
    match expression {
        ZantufaMex2Syntax::ZantufaOperand(operand) => baseline_operand(operand),
        ZantufaMex2Syntax::ZantufaForethoughtMekso(expression) => baseline_forethought(expression),
        ZantufaMex2Syntax::ZantufaReversePolishMekso(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand(expression: &ZantufaOperandSyntax) -> bool {
    match expression {
        ZantufaOperandSyntax::NumberMekso(number) => {
            let super::generated_model::NumberMeksoSyntax(quantifier) = number;
            let _ = quantifier;
            true
        }
        ZantufaOperandSyntax::LerfuStringMekso(letters) => {
            let super::generated_model::LerfuStringMeksoSyntax {
                letters: _,
                boi: _,
                free_modifiers: _,
            } = letters;
            true
        }
        ZantufaOperandSyntax::ZantufaParenthesizedMeksoOperand(operand) => {
            let super::generated_model::ZantufaParenthesizedMeksoOperandSyntax {
                vei: _,
                inner_expression: _,
                veho: _,
            } = operand;
            true
        }
        ZantufaOperandSyntax::ZantufaSumtiMoheMeksoOperand(operand) => {
            let super::generated_model::ZantufaSumtiMoheMeksoOperandSyntax {
                mohe: _,
                sumti: _,
                tehu: _,
            } = operand;
            true
        }
        ZantufaOperandSyntax::ZantufaLaheQualifiedMeksoOperand(operand) => {
            let super::generated_model::ZantufaLaheQualifiedMeksoOperandSyntax {
                lahe: _,
                inner_expression,
                luhu: _,
            } = operand;
            baseline_operand_mex(inner_expression.as_ref())
        }
        ZantufaOperandSyntax::ZantufaNaheBoQualifiedMeksoOperand(operand) => {
            let super::generated_model::ZantufaNaheBoQualifiedMeksoOperandSyntax {
                nahe: _,
                bo: _,
                inner_expression,
                luhu: _,
            } = operand;
            baseline_operand_mex(inner_expression.as_ref())
        }
        ZantufaOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | ZantufaOperandSyntax::ZantufaScalarNegatedMeksoOperand(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand_mex(expression: &ZantufaMexSyntax) -> bool {
    let ZantufaMexSyntax {
        first_expression,
        continuations,
    } = expression;
    baseline_operand_precedence(first_expression)
        && continuations
            .iter()
            .enumerate()
            .all(|(index, continuation)| {
                let ZantufaMexContinuationSyntax {
                    operators,
                    right_expression,
                } = continuation;
                operators.len() == 1
                    && baseline_operand_connective(&operators[0])
                    && right_expression.as_deref().is_some_and(|right| {
                        baseline_operand_precedence(right)
                            || index + 1 == continuations.len()
                                && baseline_grouped_operand_continuation_right(right)
                    })
            })
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand_precedence(expression: &ZantufaMex1Syntax) -> bool {
    let ZantufaMex1Syntax { first_group, tails } = expression;
    tails.is_empty() && baseline_operand_group(first_group)
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand_group(group: &ZantufaMexGroupSyntax) -> bool {
    match group {
        ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(_) => false,
        ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) => {
            let ZantufaBoGroupedMeksoSyntax {
                first_expression,
                continuations,
            } = group;
            continuations.is_empty() && baseline_operand_atom(first_expression)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_grouped_operand_continuation_right(expression: &ZantufaMex1Syntax) -> bool {
    let ZantufaMex1Syntax { first_group, tails } = expression;
    if !tails.is_empty() {
        return false;
    }
    match first_group.as_ref() {
        ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(_) => false,
        ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(group) => {
            let super::generated_model::ZantufaKeGroupedMeksoSyntax {
                ke: _,
                expressions,
                kehe: _,
            } = group;
            expressions.len() == 1 && baseline_operand_atom(&expressions[0])
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand_atom(expression: &ZantufaMex2Syntax) -> bool {
    match expression {
        ZantufaMex2Syntax::ZantufaOperand(operand) => baseline_operand(operand),
        ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
        | ZantufaMex2Syntax::ZantufaForethoughtMekso(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_operand_connective(operator: &ZantufaOperatorSyntax) -> bool {
    match operator {
        ZantufaOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            let super::generated_model::ZantufaConnectiveMeksoOperatorSyntax(_) = operator;
            true
        }
        ZantufaOperatorSyntax::ZantufaConvertedMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaScalarNegatedMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaMahoMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaPrimitiveMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaMahoSelbriMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn wide_qualified_head_has_baseline_inner(expression: &ZantufaMex1Syntax) -> bool {
    let ZantufaMex1Syntax {
        first_group,
        tails: _,
    } = expression;
    let ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) = first_group.as_ref() else {
        return false;
    };
    let ZantufaBoGroupedMeksoSyntax {
        first_expression,
        continuations: _,
    } = group;
    let ZantufaMex2Syntax::ZantufaOperand(operand) = first_expression.as_ref() else {
        return false;
    };
    let inner = match operand {
        ZantufaOperandSyntax::ZantufaLaheQualifiedMeksoOperand(operand) => {
            let super::generated_model::ZantufaLaheQualifiedMeksoOperandSyntax {
                lahe: _,
                inner_expression,
                luhu: _,
            } = operand;
            inner_expression.as_ref()
        }
        ZantufaOperandSyntax::ZantufaNaheBoQualifiedMeksoOperand(operand) => {
            let super::generated_model::ZantufaNaheBoQualifiedMeksoOperandSyntax {
                nahe: _,
                bo: _,
                inner_expression,
                luhu: _,
            } = operand;
            inner_expression.as_ref()
        }
        ZantufaOperandSyntax::NumberMekso(_)
        | ZantufaOperandSyntax::LerfuStringMekso(_)
        | ZantufaOperandSyntax::ZantufaParenthesizedMeksoOperand(_)
        | ZantufaOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | ZantufaOperandSyntax::ZantufaSumtiMoheMeksoOperand(_)
        | ZantufaOperandSyntax::ZantufaScalarNegatedMeksoOperand(_) => return false,
    };
    // A fully baseline-shaped inner MEX has a same-extent narrow reading:
    // qualifier(first operand), followed by its remainder as outer infix syntax.
    // If the inner contains any Zantufa-only element, that baseline reparse cannot
    // succeed, so keeping the wide tree cannot steal or reinterpret a baseline parse.
    is_baseline_mex(inner)
}

#[requires(true)]
#[ensures(true)]
fn baseline_forethought(expression: &ZantufaForethoughtMeksoSyntax) -> bool {
    let ZantufaForethoughtMeksoSyntax {
        peho: _,
        operator,
        operands,
        continuation,
        kuhe: _,
    } = expression;
    continuation.is_none()
        && baseline_operator(operator)
        && operands
            .iter()
            .all(|operand| baseline_atom_as_base(operand))
}

#[requires(true)]
#[ensures(true)]
fn baseline_root_reverse_polish(expression: &ZantufaMex1Syntax) -> bool {
    let ZantufaMex1Syntax { first_group, tails } = expression;
    if !tails.is_empty() {
        return false;
    }
    let ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) = first_group.as_ref() else {
        return false;
    };
    let ZantufaBoGroupedMeksoSyntax {
        first_expression,
        continuations,
    } = group;
    if !continuations.is_empty() {
        return false;
    }
    match first_expression.as_ref() {
        ZantufaMex2Syntax::ZantufaReversePolishMekso(expression) => {
            baseline_reverse_polish(expression)
        }
        ZantufaMex2Syntax::ZantufaOperand(_) | ZantufaMex2Syntax::ZantufaForethoughtMekso(_) => {
            false
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_reverse_polish(expression: &ZantufaReversePolishMeksoSyntax) -> bool {
    let ZantufaReversePolishMeksoSyntax {
        fuha: _,
        operands,
        operator,
        tails,
        kuhe: _,
    } = expression;
    if !operands.iter().all(|operand| match operand.as_ref() {
        ZantufaMex2Syntax::ZantufaOperand(operand) => baseline_operand(operand),
        ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
        | ZantufaMex2Syntax::ZantufaForethoughtMekso(_) => false,
    }) || !baseline_operator(operator)
    {
        return false;
    }
    let Some(mut depth) = operands.len().checked_sub(1) else {
        return false;
    };
    for tail in tails {
        let super::generated_model::ZantufaReversePolishTailSyntax { operands, operator } = tail;
        if !operands.iter().all(|operand| match operand.as_ref() {
            ZantufaMex2Syntax::ZantufaOperand(operand) => baseline_operand(operand),
            ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
            | ZantufaMex2Syntax::ZantufaForethoughtMekso(_) => false,
        }) || !baseline_operator(operator)
        {
            return false;
        }
        depth = depth.saturating_add(operands.len()).saturating_sub(1);
    }
    depth == 1
}

#[requires(true)]
#[ensures(true)]
fn baseline_operator(operator: &ZantufaOperatorSyntax) -> bool {
    match operator {
        ZantufaOperatorSyntax::ZantufaConvertedMeksoOperator(operator) => {
            let super::generated_model::ZantufaConvertedMeksoOperatorSyntax {
                se: _,
                inner_operator,
            } = operator;
            baseline_operator(inner_operator.as_ref())
        }
        ZantufaOperatorSyntax::ZantufaScalarNegatedMeksoOperator(operator) => {
            let super::generated_model::ZantufaScalarNegatedMeksoOperatorSyntax {
                nahe: _,
                inner_operator,
            } = operator;
            baseline_operator(inner_operator.as_ref())
        }
        ZantufaOperatorSyntax::ZantufaMahoMeksoOperator(operator) => {
            let super::generated_model::ZantufaMahoMeksoOperatorSyntax {
                maho: _,
                mekso: _,
                tehu: _,
            } = operator;
            true
        }
        ZantufaOperatorSyntax::ZantufaPrimitiveMeksoOperator(operator) => {
            let super::generated_model::ZantufaPrimitiveMeksoOperatorSyntax(_) = operator;
            true
        }
        ZantufaOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            let super::generated_model::ZantufaConnectiveMeksoOperatorSyntax(_) = operator;
            true
        }
        ZantufaOperatorSyntax::ZantufaMahoSelbriMeksoOperator(_)
        | ZantufaOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_) => false,
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineMexRejection;

#[contract_trait]
impl OutputRejection<ZantufaMexSyntax> for BaselineMexRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline mex surface"
    }

    fn rejects(&self, value: &ZantufaMexSyntax) -> bool {
        is_baseline_mex(value)
    }
}

#[requires(true)]
#[ensures(true)]
fn valid<T>(value: &recovered::Recovered<T>) -> Option<&T> {
    match value {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn valid_wf<T>(value: &recovered::WithFreeModifiers<recovered::Recovered<T>>) -> bool {
    valid(&value.value).is_some()
        && value
            .free_modifiers
            .iter()
            .all(|modifier| valid(modifier).is_some())
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_mex(expression: &recovered::ZantufaMexSyntax) -> bool {
    let recovered::ZantufaMexSyntax {
        first_expression,
        continuations,
    } = expression;
    let Some(first_expression) = valid(first_expression) else {
        return false;
    };
    if recovered_baseline_operand_mex(expression)
        || recovered_wide_qualified_head_has_baseline_inner(first_expression)
        || continuations.is_empty() && recovered_baseline_root_reverse_polish(first_expression)
    {
        return true;
    }
    recovered_baseline_precedence(first_expression)
        && continuations.iter().all(|continuation| {
            let Some(recovered::ZantufaMexContinuationSyntax {
                operators,
                right_expression,
            }) = valid(continuation)
            else {
                return false;
            };
            operators.len() == 1
                && valid(&operators[0]).is_some_and(recovered_baseline_operator)
                && right_expression
                    .as_ref()
                    .and_then(|value| valid(value.as_ref()))
                    .is_some_and(recovered_baseline_precedence)
        })
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_precedence(expression: &recovered::ZantufaMex1Syntax) -> bool {
    let recovered::ZantufaMex1Syntax { first_group, tails } = expression;
    valid(first_group).is_some_and(recovered_baseline_group_as_base)
        && tails.iter().all(|tail| {
            let Some(recovered::ZantufaBiheMeksoTailSyntax {
                bihe: _,
                operators,
                right_group,
            }) = valid(tail)
            else {
                return false;
            };
            operators.len() == 1
                && valid(&operators[0]).is_some_and(recovered_baseline_operator)
                && right_group
                    .as_ref()
                    .and_then(|value| valid(value.as_ref()))
                    .is_some_and(recovered_baseline_group_as_base)
        })
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_group_as_base(group: &recovered::ZantufaMexGroupSyntax) -> bool {
    match group {
        recovered::ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(_) => false,
        recovered::ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) => {
            let Some(recovered::ZantufaBoGroupedMeksoSyntax {
                first_expression,
                continuations,
            }) = valid(group)
            else {
                return false;
            };
            continuations.is_empty()
                && valid(first_expression).is_some_and(recovered_baseline_atom_as_base)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_atom_as_base(expression: &recovered::ZantufaMex2Syntax) -> bool {
    match expression {
        recovered::ZantufaMex2Syntax::ZantufaOperand(operand) => {
            valid(operand).is_some_and(recovered_baseline_operand)
        }
        recovered::ZantufaMex2Syntax::ZantufaForethoughtMekso(expression) => {
            valid(expression).is_some_and(recovered_baseline_forethought)
        }
        recovered::ZantufaMex2Syntax::ZantufaReversePolishMekso(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand(expression: &recovered::ZantufaOperandSyntax) -> bool {
    match expression {
        recovered::ZantufaOperandSyntax::NumberMekso(number) => valid(number)
            .is_some_and(|recovered::NumberMeksoSyntax(quantifier)| valid(quantifier).is_some()),
        recovered::ZantufaOperandSyntax::LerfuStringMekso(letters) => valid(letters).is_some_and(
            |recovered::LerfuStringMeksoSyntax {
                 letters,
                 boi,
                 free_modifiers,
             }| {
                valid(letters).is_some()
                    && boi.as_ref().is_none_or(|boi| valid(boi).is_some())
                    && free_modifiers
                        .iter()
                        .all(|modifier| valid(modifier).is_some())
            },
        ),
        recovered::ZantufaOperandSyntax::ZantufaParenthesizedMeksoOperand(operand) => {
            valid(operand).is_some_and(
                |recovered::ZantufaParenthesizedMeksoOperandSyntax {
                     vei,
                     inner_expression,
                     veho,
                 }| {
                    valid_wf(vei)
                        && valid(inner_expression).is_some()
                        && veho.as_ref().is_none_or(valid_wf)
                },
            )
        }
        recovered::ZantufaOperandSyntax::ZantufaSumtiMoheMeksoOperand(operand) => valid(operand)
            .is_some_and(
                |recovered::ZantufaSumtiMoheMeksoOperandSyntax { mohe, sumti, tehu }| {
                    valid_wf(mohe) && valid(sumti).is_some() && tehu.as_ref().is_none_or(valid_wf)
                },
            ),
        recovered::ZantufaOperandSyntax::ZantufaLaheQualifiedMeksoOperand(operand) => {
            valid(operand).is_some_and(
                |recovered::ZantufaLaheQualifiedMeksoOperandSyntax {
                     lahe,
                     inner_expression,
                     luhu,
                 }| {
                    valid_wf(lahe)
                        && valid(inner_expression).is_some_and(recovered_baseline_operand_mex)
                        && luhu.as_ref().is_none_or(valid_wf)
                },
            )
        }
        recovered::ZantufaOperandSyntax::ZantufaNaheBoQualifiedMeksoOperand(operand) => {
            valid(operand).is_some_and(
                |recovered::ZantufaNaheBoQualifiedMeksoOperandSyntax {
                     nahe,
                     bo,
                     inner_expression,
                     luhu,
                 }| {
                    valid_wf(nahe)
                        && valid_wf(bo)
                        && valid(inner_expression).is_some_and(recovered_baseline_operand_mex)
                        && luhu.as_ref().is_none_or(valid_wf)
                },
            )
        }
        recovered::ZantufaOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | recovered::ZantufaOperandSyntax::ZantufaScalarNegatedMeksoOperand(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand_mex(expression: &recovered::ZantufaMexSyntax) -> bool {
    let recovered::ZantufaMexSyntax {
        first_expression,
        continuations,
    } = expression;
    valid(first_expression).is_some_and(recovered_baseline_operand_precedence)
        && continuations
            .iter()
            .enumerate()
            .all(|(index, continuation)| {
                let Some(recovered::ZantufaMexContinuationSyntax {
                    operators,
                    right_expression,
                }) = valid(continuation)
                else {
                    return false;
                };
                operators.len() == 1
                    && valid(&operators[0]).is_some_and(recovered_baseline_operand_connective)
                    && right_expression
                        .as_ref()
                        .and_then(|value| valid(value.as_ref()))
                        .is_some_and(|right| {
                            recovered_baseline_operand_precedence(right)
                                || index + 1 == continuations.len()
                                    && recovered_baseline_grouped_operand_continuation_right(right)
                        })
            })
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand_precedence(expression: &recovered::ZantufaMex1Syntax) -> bool {
    let recovered::ZantufaMex1Syntax { first_group, tails } = expression;
    tails.is_empty() && valid(first_group).is_some_and(recovered_baseline_operand_group)
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand_group(group: &recovered::ZantufaMexGroupSyntax) -> bool {
    match group {
        recovered::ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(_) => false,
        recovered::ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group) => valid(group).is_some_and(
            |recovered::ZantufaBoGroupedMeksoSyntax {
                 first_expression,
                 continuations,
             }| {
                continuations.is_empty()
                    && valid(first_expression).is_some_and(recovered_baseline_operand_atom)
            },
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_grouped_operand_continuation_right(
    expression: &recovered::ZantufaMex1Syntax,
) -> bool {
    let recovered::ZantufaMex1Syntax { first_group, tails } = expression;
    if !tails.is_empty() {
        return false;
    }
    match valid(first_group) {
        Some(recovered::ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(_)) => false,
        Some(recovered::ZantufaMexGroupSyntax::ZantufaKeGroupedMekso(group)) => valid(group)
            .is_some_and(
                |recovered::ZantufaKeGroupedMeksoSyntax {
                     ke,
                     expressions,
                     kehe,
                 }| {
                    valid_wf(ke)
                        && expressions.len() == 1
                        && valid(&expressions[0]).is_some_and(recovered_baseline_operand_atom)
                        && kehe.as_ref().is_none_or(valid_wf)
                },
            ),
        None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand_atom(expression: &recovered::ZantufaMex2Syntax) -> bool {
    match expression {
        recovered::ZantufaMex2Syntax::ZantufaOperand(operand) => {
            valid(operand).is_some_and(recovered_baseline_operand)
        }
        recovered::ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
        | recovered::ZantufaMex2Syntax::ZantufaForethoughtMekso(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operand_connective(operator: &recovered::ZantufaOperatorSyntax) -> bool {
    match operator {
        recovered::ZantufaOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            valid(operator).is_some_and(
                |recovered::ZantufaConnectiveMeksoOperatorSyntax(connective)| {
                    valid(connective).is_some()
                },
            )
        }
        recovered::ZantufaOperatorSyntax::ZantufaConvertedMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaScalarNegatedMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaMahoMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaPrimitiveMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaMahoSelbriMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_wide_qualified_head_has_baseline_inner(
    expression: &recovered::ZantufaMex1Syntax,
) -> bool {
    let recovered::ZantufaMex1Syntax {
        first_group,
        tails: _,
    } = expression;
    let Some(recovered::ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group)) = valid(first_group)
    else {
        return false;
    };
    let Some(recovered::ZantufaBoGroupedMeksoSyntax {
        first_expression,
        continuations: _,
    }) = valid(group)
    else {
        return false;
    };
    let Some(recovered::ZantufaMex2Syntax::ZantufaOperand(operand)) = valid(first_expression)
    else {
        return false;
    };
    let Some(operand) = valid(operand) else {
        return false;
    };
    let inner = match operand {
        recovered::ZantufaOperandSyntax::ZantufaLaheQualifiedMeksoOperand(operand) => {
            let Some(recovered::ZantufaLaheQualifiedMeksoOperandSyntax {
                lahe: _,
                inner_expression,
                luhu: _,
            }) = valid(operand)
            else {
                return false;
            };
            valid(inner_expression)
        }
        recovered::ZantufaOperandSyntax::ZantufaNaheBoQualifiedMeksoOperand(operand) => {
            let Some(recovered::ZantufaNaheBoQualifiedMeksoOperandSyntax {
                nahe: _,
                bo: _,
                inner_expression,
                luhu: _,
            }) = valid(operand)
            else {
                return false;
            };
            valid(inner_expression)
        }
        recovered::ZantufaOperandSyntax::NumberMekso(_)
        | recovered::ZantufaOperandSyntax::LerfuStringMekso(_)
        | recovered::ZantufaOperandSyntax::ZantufaParenthesizedMeksoOperand(_)
        | recovered::ZantufaOperandSyntax::ZantufaSelbriMoheMeksoOperand(_)
        | recovered::ZantufaOperandSyntax::ZantufaSumtiMoheMeksoOperand(_)
        | recovered::ZantufaOperandSyntax::ZantufaScalarNegatedMeksoOperand(_) => return false,
    };
    // Mirror the strict classifier's complete-inner proof. A recovered tree with
    // any invalid component cannot establish baseline ownership and stays kept.
    inner.is_some_and(recovered_is_baseline_mex)
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_forethought(expression: &recovered::ZantufaForethoughtMeksoSyntax) -> bool {
    let recovered::ZantufaForethoughtMeksoSyntax {
        peho,
        operator,
        operands,
        continuation,
        kuhe,
    } = expression;
    peho.as_ref().is_none_or(valid_wf)
        && valid(operator).is_some_and(recovered_baseline_operator)
        && operands
            .iter()
            .all(|operand| valid(operand).is_some_and(recovered_baseline_atom_as_base))
        && continuation.is_none()
        && kuhe.as_ref().is_none_or(valid_wf)
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_root_reverse_polish(expression: &recovered::ZantufaMex1Syntax) -> bool {
    let recovered::ZantufaMex1Syntax { first_group, tails } = expression;
    if !tails.is_empty() {
        return false;
    }
    let Some(recovered::ZantufaMexGroupSyntax::ZantufaBoGroupedMekso(group)) = valid(first_group)
    else {
        return false;
    };
    let Some(recovered::ZantufaBoGroupedMeksoSyntax {
        first_expression,
        continuations,
    }) = valid(group)
    else {
        return false;
    };
    if !continuations.is_empty() {
        return false;
    }
    match valid(first_expression) {
        Some(recovered::ZantufaMex2Syntax::ZantufaReversePolishMekso(expression)) => {
            valid(expression).is_some_and(recovered_baseline_reverse_polish)
        }
        Some(
            recovered::ZantufaMex2Syntax::ZantufaOperand(_)
            | recovered::ZantufaMex2Syntax::ZantufaForethoughtMekso(_),
        )
        | None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_reverse_polish(
    expression: &recovered::ZantufaReversePolishMeksoSyntax,
) -> bool {
    let recovered::ZantufaReversePolishMeksoSyntax {
        fuha,
        operands,
        operator,
        tails,
        kuhe,
    } = expression;
    if !valid_wf(fuha)
        || !operands.iter().all(|operand| match valid(operand) {
            Some(recovered::ZantufaMex2Syntax::ZantufaOperand(operand)) => {
                valid(operand).is_some_and(recovered_baseline_operand)
            }
            Some(
                recovered::ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
                | recovered::ZantufaMex2Syntax::ZantufaForethoughtMekso(_),
            )
            | None => false,
        })
        || !valid(operator).is_some_and(recovered_baseline_operator)
        || !kuhe.as_ref().is_none_or(valid_wf)
    {
        return false;
    }
    let Some(mut depth) = operands.len().checked_sub(1) else {
        return false;
    };
    for tail in tails {
        let Some(recovered::ZantufaReversePolishTailSyntax { operands, operator }) = valid(tail)
        else {
            return false;
        };
        if !operands.iter().all(|operand| match valid(operand) {
            Some(recovered::ZantufaMex2Syntax::ZantufaOperand(operand)) => {
                valid(operand).is_some_and(recovered_baseline_operand)
            }
            Some(
                recovered::ZantufaMex2Syntax::ZantufaReversePolishMekso(_)
                | recovered::ZantufaMex2Syntax::ZantufaForethoughtMekso(_),
            )
            | None => false,
        }) || !valid(operator).is_some_and(recovered_baseline_operator)
        {
            return false;
        }
        depth = depth.saturating_add(operands.len()).saturating_sub(1);
    }
    depth == 1
}

#[requires(true)]
#[ensures(true)]
fn recovered_baseline_operator(operator: &recovered::ZantufaOperatorSyntax) -> bool {
    match operator {
        recovered::ZantufaOperatorSyntax::ZantufaConvertedMeksoOperator(operator) => {
            valid(operator).is_some_and(
                |recovered::ZantufaConvertedMeksoOperatorSyntax { se, inner_operator }| {
                    valid_wf(se) && valid(inner_operator).is_some_and(recovered_baseline_operator)
                },
            )
        }
        recovered::ZantufaOperatorSyntax::ZantufaScalarNegatedMeksoOperator(operator) => {
            valid(operator).is_some_and(
                |recovered::ZantufaScalarNegatedMeksoOperatorSyntax {
                     nahe,
                     inner_operator,
                 }| {
                    valid_wf(nahe) && valid(inner_operator).is_some_and(recovered_baseline_operator)
                },
            )
        }
        recovered::ZantufaOperatorSyntax::ZantufaMahoMeksoOperator(operator) => valid(operator)
            .is_some_and(
                |recovered::ZantufaMahoMeksoOperatorSyntax { maho, mekso, tehu }| {
                    valid_wf(maho) && valid(mekso).is_some() && tehu.as_ref().is_none_or(valid_wf)
                },
            ),
        recovered::ZantufaOperatorSyntax::ZantufaPrimitiveMeksoOperator(operator) => {
            valid(operator)
                .is_some_and(|recovered::ZantufaPrimitiveMeksoOperatorSyntax(vuhu)| valid_wf(vuhu))
        }
        recovered::ZantufaOperatorSyntax::ZantufaConnectiveMeksoOperator(operator) => {
            valid(operator).is_some_and(
                |recovered::ZantufaConnectiveMeksoOperatorSyntax(connective)| {
                    valid(connective).is_some()
                },
            )
        }
        recovered::ZantufaOperatorSyntax::ZantufaMahoSelbriMeksoOperator(_)
        | recovered::ZantufaOperatorSyntax::ZantufaMahoSumtiMeksoOperator(_) => false,
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaMexSyntax>> for BaselineMexRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline mex surface"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::ZantufaMexSyntax>) -> bool {
        valid(value).is_some_and(recovered_is_baseline_mex)
    }
}
