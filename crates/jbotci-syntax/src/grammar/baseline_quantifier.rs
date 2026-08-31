//! Typed classification of the baseline `quantifier` surfaces a MEX can match.
//!
//! Zantufa broadens `quantifier` to an arbitrary MEX, and the priority raw-MEX
//! alternative is ordered before both baseline forms so that a genuinely
//! extended expression such as `pa su'i re` wins. The MEX language, however,
//! also contains the two surfaces baseline `quantifier` already owns, so the
//! priority route must decline them and let strict ordered choice reparse them
//! through the baseline `pa_run_quantifier` and `mekso_quantifier`
//! alternatives:
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
//! Only the baseline `infix_mekso` reading is classified. `mekso` already hands
//! baseline surfaces back to that reading through
//! [`BaselineMexRejection`](super::baseline_mex::BaselineMexRejection), so a
//! completed `zantufa_priority_mex` is Zantufa-only by construction, and the
//! `reinterpret_zantufa_mex` and additive `zantufa_mex` readings are the
//! deliberate Zantufa projections of their axes. A quantifier built on any of
//! those is not a baseline quantifier surface and keeps its raw-MEX ownership.
//!
//! Every product node on the way down is destructured exhaustively rather than
//! read through field access, and no arm uses `..`. The classification is only
//! extent-preserving while it accounts for every component of every node it
//! descends through, so a field added to any of these productions — a trailing
//! terminator, a free-modifier slot, anything else that can consume tokens —
//! must be a compile error here rather than a silently ignored extent.

use bityzba::{contract_trait, invariant, requires};
use jbotci_tree::Chain;

use super::generated_model::{
    AfterthoughtMeksoOperandSyntax, BoundOrSimpleMeksoOperandSyntax, InfixMeksoSyntax,
    MeksoBaseSyntax, MeksoOperandSyntax, MeksoPrecedenceSyntax, MeksoSyntax, NumberMeksoSyntax,
    ParenthesizedMeksoOperandSyntax, SimpleMeksoOperandSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

/// The single simple operand a MEX reduces to, when it is exactly one.
///
/// Returns `None` for reverse Polish MEX, for every Zantufa MEX reading, for
/// infix MEX with operator continuations, for a precedence expression carrying
/// a `BIhE` tail, and for an operand carrying a connective chain or a grouped
/// continuation, because none of those is a lone operand.
///
/// `mekso_operand` puts the afterthought-connection chain first, and that chain
/// admits zero links, so a lone operand is always reached through it.
#[requires(true)]
#[ensures(true)]
fn single_simple_mekso_operand(expression: &MeksoSyntax) -> Option<&SimpleMeksoOperandSyntax> {
    let MeksoSyntax::InfixMekso(infix) = expression else {
        return None;
    };
    let InfixMeksoSyntax {
        first_expression,
        continuations,
    } = infix;
    if !continuations.is_empty() {
        return None;
    }
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
    let MeksoOperandSyntax {
        connected_expression,
        grouped_continuation,
    } = operand;
    if grouped_continuation.is_some() {
        return None;
    }
    let AfterthoughtMeksoOperandSyntax(Chain { first, links }) = &**connected_expression;
    if !links.is_empty() {
        return None;
    }
    match first.as_ref() {
        BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => Some(operand),
        BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => None,
    }
}

/// Reports whether a MEX is exactly one baseline `quantifier` surface.
///
/// Free modifiers, an elided `BOI`, and an elided `VEhO` are all part of the
/// baseline productions, so none of them are required to be absent or present
/// here.
#[requires(true)]
#[ensures(true)]
fn is_baseline_quantifier_surface(expression: &MeksoSyntax) -> bool {
    let Some(operand) = single_simple_mekso_operand(expression) else {
        return false;
    };
    match operand {
        SimpleMeksoOperandSyntax::NumberMekso(number) => {
            // `number_mekso` wraps the very `pa_run_quantifier` rule the
            // baseline alternative parses, so a field added there changes both
            // surfaces identically and cannot make the extents diverge.
            let NumberMeksoSyntax(_) = number;
            true
        }
        SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            // `parenthesized_mekso_operand` and `mekso_quantifier` are separate
            // rule declarations that happen to be the same `VEI mex [VEhO]`
            // surface, so extent preservation depends on them staying field for
            // field identical. Destructure exhaustively: a field added to this
            // one must be a compile error here, not a silent divergence from
            // the baseline alternative that reparses the same text.
            let ParenthesizedMeksoOperandSyntax {
                vei: _,
                inner_expression: _,
                veho: _,
            } = operand;
            true
        }
        SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(_)
        | SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ScalarNegatedMeksoOperand(_)
        | SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntax::ArrayMeksoOperand(_)
        | SimpleMeksoOperandSyntax::LerfuStringMekso(_) => false,
    }
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

/// The single simple operand a recovered MEX reduces to, when it is exactly one.
///
/// This mirrors [`single_simple_mekso_operand`] over the recovered model, which
/// wraps every slot in `Recovered` and therefore cannot share the strict
/// model's descent. Only fully valid spines classify: every recovered slot on
/// the way down must be `Recovered::Valid`, so a MEX that carries a recovery
/// item anywhere on this spine keeps its raw-MEX ownership instead of being
/// reparsed as baseline. The destructuring is exhaustive and `..`-free for the
/// same reason it is in the strict descent.
#[requires(true)]
#[ensures(true)]
fn recovered_single_simple_mekso_operand(
    expression: &recovered::MeksoSyntax,
) -> Option<&recovered::SimpleMeksoOperandSyntax> {
    let recovered::MeksoSyntax::InfixMekso(infix) = expression else {
        return None;
    };
    let recovered::InfixMeksoSyntax {
        first_expression,
        continuations,
    } = valid(infix)?;
    if !continuations.is_empty() {
        return None;
    }
    let recovered::MeksoPrecedenceSyntax {
        left_expression,
        tail,
    } = valid(first_expression)?;
    if tail.is_some() {
        return None;
    }
    let recovered::MeksoBaseSyntax::MeksoOperand(operand) = valid(left_expression)? else {
        return None;
    };
    let recovered::MeksoOperandSyntax {
        connected_expression,
        grouped_continuation,
    } = valid(operand)?;
    if grouped_continuation.is_some() {
        return None;
    }
    let recovered::AfterthoughtMeksoOperandSyntax(Chain { first, links }) =
        valid(connected_expression)?;
    if !links.is_empty() {
        return None;
    }
    let operand = match valid(first)? {
        recovered::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => operand,
        recovered::BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => return None,
    };
    valid(operand)
}

/// Reports whether a recovered MEX is exactly one baseline quantifier surface.
///
/// The descent to the simple operand rejects any non-`Valid` slot on the way
/// down; the operand's own payload is classified as the exhaustive product it
/// is, so both arms stop at the same depth and the ownership decision does not
/// depend on which of the two baseline surfaces was written.
#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_quantifier_surface(expression: &recovered::MeksoSyntax) -> bool {
    let Some(operand) = recovered_single_simple_mekso_operand(expression) else {
        return false;
    };
    match operand {
        // `number_mekso` wraps the very `pa_run_quantifier` rule the baseline
        // alternative parses, so a field added there changes both surfaces
        // identically and cannot make the extents diverge.
        recovered::SimpleMeksoOperandSyntax::NumberMekso(number) => {
            matches!(valid(number), Some(recovered::NumberMeksoSyntax(_)))
        }
        // `parenthesized_mekso_operand` and `mekso_quantifier` are separate rule
        // declarations that happen to be the same `VEI mex [VEhO]` surface, so
        // extent preservation depends on them staying field for field
        // identical. Destructure exhaustively: a field added to this one must be
        // a compile error here, not a silent divergence from the baseline
        // alternative that reparses the same text.
        recovered::SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => matches!(
            valid(operand),
            Some(recovered::ParenthesizedMeksoOperandSyntax {
                vei: _,
                inner_expression: _,
                veho: _,
            })
        ),
        recovered::SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ScalarNegatedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ArrayMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::LerfuStringMekso(_) => false,
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
        is_baseline_quantifier_surface(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::MeksoSyntax>> for BaselineQuantifierRejection {
    fn rejected_name(&self) -> &'static str {
        BASELINE_QUANTIFIER_REJECTION_NAME
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::MeksoSyntax>) -> bool {
        valid(value).is_some_and(recovered_is_baseline_quantifier_surface)
    }
}
