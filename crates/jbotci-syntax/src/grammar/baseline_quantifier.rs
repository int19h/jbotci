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
use jbotci_tree::Chain;

use super::generated_model::{
    AfterthoughtMeksoOperandSyntax, BoundOrSimpleMeksoOperandSyntax, InfixMeksoSyntax,
    MeksoBaseSyntax, MeksoOperandSyntax, MeksoPrecedenceSyntax, MeksoSyntax, NumberMeksoSyntax,
    PaRunQuantifierSyntax, ParenthesizedMeksoOperandSyntax, SimpleMeksoOperandSyntax,
    ZantufaInfixMeksoSyntax, recovered,
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
///
/// Every product node on the way down is destructured exhaustively rather than
/// read through field access, and no arm uses `..`. The classification is only
/// extent-preserving while it accounts for every component of every node it
/// descends through, so a field added to any of these productions — a trailing
/// terminator, a free-modifier slot, anything else that can consume tokens —
/// must be a compile error here rather than a silently ignored extent.
#[requires(true)]
#[ensures(true)]
pub fn single_mekso_operand(expression: &MeksoSyntax) -> Option<&MeksoOperandSyntax> {
    let first_expression = match expression {
        MeksoSyntax::ZantufaInfixMekso(infix) => {
            let ZantufaInfixMeksoSyntax {
                first_expression,
                continuations,
            } = infix;
            if !continuations.is_empty() {
                return None;
            }
            first_expression
        }
        MeksoSyntax::InfixMekso(infix) => {
            let InfixMeksoSyntax {
                first_expression,
                continuations,
            } = infix;
            if !continuations.is_empty() {
                return None;
            }
            first_expression
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
    match left_expression.as_ref() {
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
            let AfterthoughtMeksoOperandSyntax(Chain { first, links }) = operand;
            if !links.is_empty() {
                return None;
            }
            match first.as_ref() {
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
            // `number_mekso` wraps the very `pa_run_quantifier` rule the
            // baseline alternative parses, so a field added there changes both
            // surfaces identically and cannot make the extents diverge.
            let NumberMeksoSyntax(quantifier) = number;
            Some(BaselineQuantifierSurface::PaRun(quantifier.as_ref()))
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
/// This mirrors [`single_mekso_operand`] and [`single_simple_mekso_operand`]
/// over the recovered model, which wraps every slot in `Recovered` and therefore
/// cannot share the strict model's descent. Only fully valid spines classify:
/// every recovered slot *on the descent itself* must be `Recovered::Valid`, so a
/// MEX that carries a recovery item on the way down to the simple operand keeps
/// its raw-MEX ownership instead of being reparsed as baseline. The descent ends
/// at the simple operand; what the operand's own fields carry is the caller's
/// question, not this function's. The destructuring is exhaustive and `..`-free
/// for the same reason it is in the strict descent: a field added to any
/// recovered product node on this spine must force the same ownership decision
/// at compile time.
#[requires(true)]
#[ensures(true)]
fn recovered_single_simple_mekso_operand(
    expression: &recovered::MeksoSyntax,
) -> Option<&recovered::SimpleMeksoOperandSyntax> {
    let first_expression = match expression {
        recovered::MeksoSyntax::ZantufaInfixMekso(infix) => {
            let recovered::ZantufaInfixMeksoSyntax {
                first_expression,
                continuations,
            } = valid(infix)?;
            if !continuations.is_empty() {
                return None;
            }
            first_expression
        }
        recovered::MeksoSyntax::InfixMekso(infix) => {
            let recovered::InfixMeksoSyntax {
                first_expression,
                continuations,
            } = valid(infix)?;
            if !continuations.is_empty() {
                return None;
            }
            first_expression
        }
        recovered::MeksoSyntax::ZantufaReversePolishMekso(_)
        | recovered::MeksoSyntax::ReversePolishMekso(_) => return None,
    };
    let recovered::MeksoPrecedenceSyntax {
        left_expression,
        tail,
    } = valid(first_expression)?;
    if tail.is_some() {
        return None;
    }
    let operand = match valid(left_expression)? {
        recovered::MeksoBaseSyntax::MeksoOperand(operand) => operand,
        recovered::MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(_)
        | recovered::MeksoBaseSyntax::ForethoughtCallMekso(_)
        | recovered::MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(_) => return None,
    };
    let operand = match valid(operand)? {
        recovered::MeksoOperandSyntax::AfterthoughtMeksoOperand(operand) => {
            let recovered::AfterthoughtMeksoOperandSyntax(Chain { first, links }) = valid(operand)?;
            if !links.is_empty() {
                return None;
            }
            match valid(first)? {
                recovered::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => operand,
                recovered::BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => return None,
            }
        }
        recovered::MeksoOperandSyntax::SimpleMeksoOperand(operand) => operand,
        recovered::MeksoOperandSyntax::BoundMeksoOperand(_) => return None,
    };
    valid(operand)
}

/// Reports whether a recovered MEX is exactly one baseline quantifier surface.
///
/// This mirrors [`baseline_quantifier_surface`], but answers only yes or no.
/// The strict classifier returns [`BaselineQuantifierSurface`] because the
/// semantic builder reads which surface matched; the recovered spine is
/// classified solely to answer the grammar's own rejection question below, so
/// naming the surface would be a borrowed value nothing reads.
///
/// Both arms stop at the same depth: the operand variant must have completed
/// (`Recovered::Valid`) and its product is destructured exhaustively, but what
/// its own fields carry is deliberately not inspected. A completed operand is
/// the baseline surface whatever recovery happened inside it, and the arms must
/// agree about that or an ownership decision would depend on which of the two
/// baseline surfaces was written — a `VEI` operand whose inner MEX recovered
/// would be handed back to baseline while a PA-run operand whose number
/// recovered would keep the raw route and its false experimental warning.
#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_quantifier_surface(expression: &recovered::MeksoSyntax) -> bool {
    let Some(operand) = recovered_single_simple_mekso_operand(expression) else {
        return false;
    };
    match operand {
        recovered::SimpleMeksoOperandSyntax::NumberMekso(number) => {
            // `number_mekso` wraps the very `pa_run_quantifier` rule the
            // baseline alternative parses, so a field added there changes both
            // surfaces identically and cannot make the extents diverge.
            // Destructure exhaustively for the same reason the parenthesized arm
            // below does, and for the same reason do not require the wrapped
            // production itself to have parsed without recovery.
            matches!(
                valid(number),
                Some(recovered::NumberMeksoSyntax(_quantifier))
            )
        }
        recovered::SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            // `parenthesized_mekso_operand` and `mekso_quantifier` are separate
            // rule declarations that happen to be the same `VEI mex [VEhO]`
            // surface, so extent preservation depends on them staying field for
            // field identical. Destructure exhaustively: a field added to this
            // one must be a compile error here, not a silent divergence from
            // the baseline alternative that reparses the same text.
            matches!(
                valid(operand),
                Some(recovered::ParenthesizedMeksoOperandSyntax {
                    vei: _,
                    inner_expression: _,
                    veho: _,
                })
            )
        }
        recovered::SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::QualifiedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SumtiMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::SelbriMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ArrayMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::LerfuStringMekso(_)
        | recovered::SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(_)
        | recovered::SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(_) => false,
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
        valid(value).is_some_and(recovered_is_baseline_quantifier_surface)
    }
}

/// Recovered-model ownership decisions no known input stages end to end.
///
/// The `zantufa_quantifier_*` grammar tests cover the recovered arms, but only
/// over spines the recovery search actually builds. A recovery directive names
/// one rule instance and one field, and the two operands sit at different
/// depths under a repair: `parenthesized_mekso_operand` owns the `VEI` position
/// itself, while `number_mekso`'s only field wraps `pa_run_quantifier`, whose
/// own `number` field starts at the same token one level deeper. A 3,983-input
/// mutation sweep over quantifier-bearing sources produced 29 recovered
/// `parenthesized_mekso_operand` fields and no recovered `number_mekso` slot at
/// all; every PA-run repair landed on `pa_run_quantifier.number` instead.
///
/// That is a fact about which candidate the recovery search prefers, not a
/// structural impossibility — a whole-rule abandon directive resumes past every
/// field, including a one-field rule's — and an ownership rule must not depend
/// on it either way. So the number arm is pinned here against a directly
/// constructed recovered model. This is the layer the rule lives at: the
/// refinement reads a recovered value and answers one question about it, with
/// no parser state involved.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bityzba::new;

    use super::*;
    use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData};

    /// A repair standing in for whatever recovery put in a slot.
    #[requires(true)]
    #[ensures(true)]
    fn repair() -> SyntaxRecoveryItem {
        let span = jbotci_diagnostics::source_span_from_byte_offsets(None, "re", 0, 0)
            .expect("valid zero-width source span");
        new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 0,
            span: Arc::new(span),
            expected: "quantifier".to_owned(),
        })
    }

    /// The recovered MEX a quantifier position holds when it is exactly one
    /// simple operand: infix with no continuations, no `BIhE` tail, and an
    /// afterthought chain with no links.
    #[requires(true)]
    #[ensures(true)]
    fn lone_operand_mekso(
        operand: recovered::SimpleMeksoOperandSyntax,
    ) -> recovered::Recovered<recovered::MeksoSyntax> {
        let operand = recovered::MeksoOperandSyntax::AfterthoughtMeksoOperand(
            recovered::Recovered::valid(recovered::AfterthoughtMeksoOperandSyntax(Chain {
                first: Arc::new(recovered::Recovered::valid(
                    recovered::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(
                        recovered::Recovered::valid(operand),
                    ),
                )),
                links: Vec::new(),
            })),
        );
        let precedence = recovered::MeksoPrecedenceSyntax {
            left_expression: Arc::new(recovered::Recovered::valid(
                recovered::MeksoBaseSyntax::MeksoOperand(recovered::Recovered::valid(operand)),
            )),
            tail: None,
        };
        recovered::Recovered::valid(recovered::MeksoSyntax::InfixMekso(
            recovered::Recovered::valid(recovered::InfixMeksoSyntax {
                first_expression: Arc::new(recovered::Recovered::valid(precedence)),
                continuations: Vec::new(),
            }),
        ))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_quantifier_recovered_number_arm_owns_a_repaired_pa_run() {
        // The PA-run surface belongs to baseline `pa_run_quantifier` whether or
        // not the production underneath it needed repair. Before the arms were
        // aligned this returned false, leaving the surface on the priority raw
        // route with a false `experimental-zantufa-mex` warning.
        let number = recovered::SimpleMeksoOperandSyntax::NumberMekso(recovered::Recovered::valid(
            recovered::NumberMeksoSyntax(Arc::new(recovered::Recovered::error(repair()))),
        ));
        assert!(BaselineQuantifierRejection.rejects(&lone_operand_mekso(number)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_quantifier_recovered_descent_still_requires_valid_slots() {
        // The relaxation is confined to the operand's own fields. A repair on
        // the descent itself still leaves the MEX on the raw route, because a
        // spine that did not complete is not known to be one baseline surface.
        let mekso = recovered::Recovered::valid(recovered::MeksoSyntax::InfixMekso(
            recovered::Recovered::error(repair()),
        ));
        assert!(!BaselineQuantifierRejection.rejects(&mekso));
        assert!(
            !BaselineQuantifierRejection.rejects(
                &recovered::Recovered::<recovered::MeksoSyntax>::error(repair())
            )
        );
    }
}
