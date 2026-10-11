//! Typed classification of the baseline `quantifier` surfaces a mex can match.
//!
//! camxes-exp broadens `quantifier` to a raw mex (`quantifier <- !selbri !sumti_6 mex`,
//! camxes-exp.peg:273, #888). The mex language also contains the two surfaces that the baseline
//! `quantifier` already owns, so the raw-mex route must refuse them and leave them to the
//! baseline `pa_run_quantifier` and `mekso_quantifier` arms:
//!
//! * a single number operand, whose payload is the very `pa_run_quantifier` rule the baseline
//!   arm uses; and
//! * a single parenthesized operand, whose `VEI`, inner mex and optional `VEhO` fields are the
//!   `mekso_quantifier` production field for field.
//!
//! This is the structural exclusion of #843. Because both surfaces are built from the same
//! component rules the baseline arms use, a refused raw match and its baseline reparse consume
//! the identical extent. The refusal therefore decides ownership and tree shape, but never which
//! inputs the grammar accepts, and the baseline arms own an ordinary quantifier because of its
//! structure, not because of the order of the arms.
//!
//! Every product node on the way down is destructured exhaustively rather than read through
//! field access, and no arm uses `..`. The classification is only extent-preserving while it
//! accounts for every component of every node it descends through, so a field added to any of
//! these productions must be a compile error here rather than a silently ignored extent.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::view::{
    AfterthoughtMeksoOperandSyntaxView, AsView, BoundOrSimpleMeksoOperandSyntaxView, ChainView,
    InfixMeksoSyntaxView, MeksoBaseSyntaxView, MeksoOperandSyntaxView, MeksoPrecedenceSyntaxView,
    MeksoSyntaxRef, MeksoSyntaxView, NumberMeksoSyntaxView, ParenthesizedMeksoOperandSyntaxView,
    SimpleMeksoOperandSyntaxRef, SimpleMeksoOperandSyntaxView,
};
use super::generated_runtime::OutputRejection;

/// Find a single simple operand through a fully valid connection spine.
#[requires(true)]
#[ensures(true)]
fn single_simple_mekso_operand(
    expression: MeksoSyntaxRef<'_>,
) -> Option<SimpleMeksoOperandSyntaxRef<'_>> {
    let infix = match expression.get() {
        MeksoSyntaxView::InfixMekso(infix) => infix,
        MeksoSyntaxView::ReversePolishMekso(_) => return None,
    };
    let InfixMeksoSyntaxView {
        first_expression,
        continuations,
    } = infix.valid()?.get();
    if !continuations.is_empty() {
        return None;
    }
    let MeksoPrecedenceSyntaxView {
        left_expression,
        tail,
    } = first_expression.valid()?.get();
    if tail.is_some() {
        return None;
    }
    let operand = match left_expression.valid()?.get() {
        MeksoBaseSyntaxView::MeksoOperand(operand) => operand,
        MeksoBaseSyntaxView::ForethoughtCallMekso(_) => return None,
    };
    let MeksoOperandSyntaxView {
        connected_expression,
        grouped_continuation,
    } = operand.valid()?.get();
    if grouped_continuation.is_some() {
        return None;
    }
    let AfterthoughtMeksoOperandSyntaxView(ChainView { first, links }) =
        connected_expression.valid()?.get();
    if !links.is_empty() {
        return None;
    }
    match first.valid()?.get() {
        BoundOrSimpleMeksoOperandSyntaxView::SimpleMeksoOperand(operand) => operand.valid(),
        BoundOrSimpleMeksoOperandSyntaxView::BoundMeksoOperand(_) => None,
    }
}

/// Both models use the same baseline fields and valid child selections.
#[requires(true)]
#[ensures(true)]
fn is_baseline_quantifier_surface(expression: MeksoSyntaxRef<'_>) -> bool {
    let Some(operand) = single_simple_mekso_operand(expression) else {
        return false;
    };
    match operand.get() {
        SimpleMeksoOperandSyntaxView::NumberMekso(number) => number.valid().is_some_and(|number| {
            let NumberMeksoSyntaxView(_) = number.get();
            true
        }),
        SimpleMeksoOperandSyntaxView::ParenthesizedMeksoOperand(operand) => {
            operand.valid().is_some_and(|operand| {
                // The baseline and parenthesized productions must retain these exact fields.
                let ParenthesizedMeksoOperandSyntaxView {
                    vei: _,
                    inner_expression: _,
                    veho: _,
                } = operand.get();
                true
            })
        }
        SimpleMeksoOperandSyntaxView::ExpNaheBoMexOperand(_)
        | SimpleMeksoOperandSyntaxView::ExpNaheMexOperand(_)
        | SimpleMeksoOperandSyntaxView::ExpLaheMexOperand(_)
        | SimpleMeksoOperandSyntaxView::ForethoughtMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::QualifiedMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::ScalarNegatedMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::LaheQualifiedMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::SumtiMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::SelbriMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::ArrayMeksoOperand(_)
        | SimpleMeksoOperandSyntaxView::LerfuStringMekso(_) => false,
    }
}

/// Return baseline quantifier surfaces to their baseline alternatives.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineQuantifierRejection;

#[contract_trait]
impl<T> OutputRejection<T> for BaselineQuantifierRejection
where
    for<'tree> T: AsView<'tree, Ref = MeksoSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "baseline quantifier surface"
    }
    fn rejects(&self, value: &T) -> bool {
        // The raw-mex lookahead must not invent or skip input inside any recovered mex.
        value
            .as_view()
            .valid()
            .is_none_or(|node| node.has_errors() || is_baseline_quantifier_surface(node))
    }
}
