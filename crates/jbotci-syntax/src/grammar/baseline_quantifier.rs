//! Typed classification of the baseline `quantifier` surfaces a MEX can match.
//!
//! Zantufa broadens `quantifier` to an arbitrary MEX. Two MEX surfaces are also
//! complete baseline quantifiers, so the Zantufa priority raw-MEX route must
//! decline them and let strict ordered choice reparse them through the baseline
//! `pa_run_quantifier` and `mekso_quantifier` alternatives:
//!
//! * a single number operand, whose payload is the very `pa_run_quantifier`
//!   rule the baseline alternative uses; and
//! * a single parenthesized operand, whose `VEI`, inner MEX, and optional
//!   `VEhO` fields are the `mekso_quantifier` production field for field.
//!
//! Because both surfaces are built from the same component rules the baseline
//! alternatives use, a rejected raw match and its baseline reparse consume the
//! identical extent. Rejection can therefore change ownership, tree shape, and
//! diagnostics, but it cannot change which inputs the grammar accepts.
//!
//! The accessors are borrowed, allocation-free, and shared with the semantic
//! builder so that only one descent over this spine exists per generated model.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    BoundOrSimpleMeksoOperandSyntax, MeksoBaseSyntax, MeksoOperandSyntax, MeksoSyntax,
    PaRunQuantifierSyntax, ParenthesizedMeksoOperandSyntax, SimpleMeksoOperandSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

/// A completed MEX whose whole extent is one baseline `quantifier` surface.
#[invariant(true)]
#[invariant(::PaRun(_) => true, "the borrowed baseline production is already validated")]
#[invariant(::Parenthesized(_) => true, "the borrowed baseline production is already validated")]
#[derive(Debug, Clone, Copy)]
pub enum BaselineQuantifierSurface<'syntax> {
    /// `number` — the baseline `pa_run_quantifier` production, `BOI` optional.
    PaRun(&'syntax PaRunQuantifierSyntax),
    /// `VEI mex [VEhO]` — the baseline `mekso_quantifier` production.
    Parenthesized(&'syntax ParenthesizedMeksoOperandSyntax),
}

/// The single operand a MEX reduces to, when it is exactly one operand.
///
/// Returns `None` for reverse Polish MEX, for infix MEX with operator
/// continuations, and for a precedence expression carrying a `BIhE` tail,
/// because none of those are a lone operand.
#[requires(true)]
#[ensures(true)]
pub fn single_mekso_operand(expression: &MeksoSyntax) -> Option<&MeksoOperandSyntax> {
    let first_expression = match expression {
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        MeksoSyntax::InfixMekso(infix) => {
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        MeksoSyntax::ZantufaReversePolishMekso(_) | MeksoSyntax::ReversePolishMekso(_) => {
            return None;
        }
    };
    if first_expression.tail.is_some() {
        return None;
    }
    match first_expression.left_expression.as_ref() {
        MeksoBaseSyntax::MeksoOperand(operand) => Some(operand),
        MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(_)
        | MeksoBaseSyntax::ForethoughtCallMekso(_)
        | MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(_) => None,
    }
}

/// The single simple operand a MEX reduces to, when it is exactly one.
///
/// `mekso_operand` puts the afterthought-connection chain first, and that chain
/// admits zero links, so a lone operand is currently always reached through
/// `AfterthoughtMeksoOperand`. The direct `SimpleMeksoOperand` alternative is
/// accepted as the same shape rather than silently ignored, so a future
/// reordering of `mekso_operand` cannot quietly change what this classifies.
#[requires(true)]
#[ensures(true)]
pub fn single_simple_mekso_operand(expression: &MeksoSyntax) -> Option<&SimpleMeksoOperandSyntax> {
    match single_mekso_operand(expression)? {
        MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
            if !operand.0.links.is_empty() {
                return None;
            }
            match operand.0.first.as_ref() {
                BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => Some(operand),
                BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => None,
            }
        }
        MeksoOperandSyntax::SimpleMeksoOperand(operand) => Some(operand),
        MeksoOperandSyntax::BoundMeksoOperand(_) => None,
    }
}

/// Classifies a MEX that is exactly one baseline `quantifier` surface.
///
/// Free modifiers, an elided `BOI`, and an elided `VEhO` are all part of the
/// baseline productions, so none of them are required to be absent or present
/// here.
#[requires(true)]
#[ensures(true)]
pub fn baseline_quantifier_surface(
    expression: &MeksoSyntax,
) -> Option<BaselineQuantifierSurface<'_>> {
    match single_simple_mekso_operand(expression)? {
        SimpleMeksoOperandSyntax::NumberMekso(number) => {
            Some(BaselineQuantifierSurface::PaRun(number.0.as_ref()))
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            Some(BaselineQuantifierSurface::Parenthesized(operand))
        }
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(_)
        | SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ArrayMeksoOperand(_)
        | SimpleMeksoOperandSyntax::LerfuStringMekso(_)
        | SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_) => None,
    }
}

/// A recovered-model MEX whose whole extent is one baseline quantifier surface.
///
/// Only fully valid spines classify: every recovered slot on the way down must
/// be `Recovered::Valid`, so a MEX that carries a recovery item anywhere on this
/// spine keeps its raw-MEX ownership instead of being reparsed as baseline.
#[invariant(true)]
#[invariant(::PaRun(_) => true, "the borrowed baseline production is already validated")]
#[invariant(::Parenthesized(_) => true, "the borrowed baseline production is already validated")]
#[derive(Debug, Clone, Copy)]
pub enum RecoveredBaselineQuantifierSurface<'syntax> {
    /// `number` — the baseline `pa_run_quantifier` production, `BOI` optional.
    PaRun(&'syntax recovered::PaRunQuantifierSyntax),
    /// `VEI mex [VEhO]` — the baseline `mekso_quantifier` production.
    Parenthesized(&'syntax recovered::ParenthesizedMeksoOperandSyntax),
}

/// The valid payload of a recovered slot, if the slot holds one without errors.
#[requires(true)]
#[ensures(true)]
fn valid<T>(value: &recovered::Recovered<T>) -> Option<&T> {
    match value {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

/// Classifies a recovered MEX that is exactly one baseline quantifier surface.
///
/// This mirrors [`baseline_quantifier_surface`] over the recovered model, which
/// wraps every slot in `Recovered` and therefore cannot share the strict
/// model's descent.
#[requires(true)]
#[ensures(true)]
pub fn recovered_baseline_quantifier_surface(
    expression: &recovered::MeksoSyntax,
) -> Option<RecoveredBaselineQuantifierSurface<'_>> {
    let first_expression = match expression {
        recovered::MeksoSyntax::ZantufaInfixMekso(infix) => {
            let infix = valid(infix)?;
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        recovered::MeksoSyntax::InfixMekso(infix) => {
            let infix = valid(infix)?;
            if !infix.continuations.is_empty() {
                return None;
            }
            &infix.first_expression
        }
        recovered::MeksoSyntax::ZantufaReversePolishMekso(_)
        | recovered::MeksoSyntax::ReversePolishMekso(_) => return None,
    };
    let first_expression = valid(first_expression)?;
    if first_expression.tail.is_some() {
        return None;
    }
    let operand = match valid(&first_expression.left_expression)? {
        recovered::MeksoBaseSyntax::MeksoOperand(operand) => operand,
        recovered::MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(_)
        | recovered::MeksoBaseSyntax::ForethoughtCallMekso(_)
        | recovered::MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(_) => return None,
    };
    let operand = match valid(operand)? {
        recovered::MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
            let operand = valid(operand)?;
            if !operand.0.links.is_empty() {
                return None;
            }
            match valid(&operand.0.first)? {
                recovered::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => operand,
                recovered::BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => return None,
            }
        }
        recovered::MeksoOperandSyntax::SimpleMeksoOperand(operand) => operand,
        recovered::MeksoOperandSyntax::BoundMeksoOperand(_) => return None,
    };
    match valid(operand)? {
        recovered::SimpleMeksoOperandSyntax::NumberMekso(number) => {
            let number = valid(number)?;
            Some(RecoveredBaselineQuantifierSurface::PaRun(valid(&number.0)?))
        }
        recovered::SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => Some(
            RecoveredBaselineQuantifierSurface::Parenthesized(valid(operand)?),
        ),
        recovered::SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ArrayMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::LerfuStringMekso(_)
        | recovered::SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_) => None,
    }
}

/// Grammar-level refinement that hands baseline quantifier surfaces back to the
/// baseline `quantifier` alternatives.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineQuantifierRejection;

const BASELINE_QUANTIFIER_REJECTION_NAME: &str = "baseline quantifier surface";

#[contract_trait]
impl OutputRejection<MeksoSyntax> for BaselineQuantifierRejection {
    fn rejected_name(&self) -> &'static str {
        BASELINE_QUANTIFIER_REJECTION_NAME
    }

    fn rejects(&self, value: &MeksoSyntax) -> bool {
        baseline_quantifier_surface(value).is_some()
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::MeksoSyntax>> for BaselineQuantifierRejection {
    fn rejected_name(&self) -> &'static str {
        BASELINE_QUANTIFIER_REJECTION_NAME
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::MeksoSyntax>) -> bool {
        valid(value).is_some_and(|value| recovered_baseline_quantifier_surface(value).is_some())
    }
}
