//! Baseline-ownership classification for the NUhI-less forethought termset.
//!
//! `gek_termset <- gek terms_gik_terms` (camxes.peg:136) and the baseline GEK sumti connection
//! `sumti_4 <- sumti_5 / gek sumti gik sumti_4` (camxes.peg:141) both begin `GEK … GIK …`, and on
//! `ge ko'a gi ko'e broda` they cover the identical extent. Both upstream parsers resolve that
//! collision in favour of the sumti connection, so the termset arm is extension-first here and the
//! completed candidate is returned to the baseline owner when the baseline owns its extent.
//!
//! The proof is an extent proof rather than an arm-order argument. Arm order alone would leave the
//! sumti owner only *usually* in front: the sumti term is listed earlier at every level, but a
//! locally failing outer parse backtracks into the termset arm, which would then reclaim an extent
//! the baseline had already covered. The classifier removes that path.
//!
//! A candidate is baseline-owned exactly when its operand tree is one GIK-paired level whose two
//! operands are both bare sumti terms. Given that shape, `gek sumti gik sumti_4` reconstructs the
//! same extent: the leading operand's sumti is admissible at the baseline's full-`sumti` first
//! branch, and the trailing operand's sumti is admissible either at the baseline's `sumti_4` second
//! branch or — when it is wider than that level, as in `ge ko'a gi ko'e .e ko'i broda` — through
//! the sumti ladder that encloses the whole connection, which yields the same extent with the
//! baseline's grouping. Any other shape has no baseline counterpart: a nested pair carries more
//! than one GIK-joined operand pair, and a non-sumti operand is precisely what the baseline's sumti
//! branches cannot accept.
//!
//! Every candidate product is destructured exhaustively and without `..`, so a model change forces
//! this proof to be revisited.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    BalancedTermsetOperandsSyntax, GekTermsetCandidateSyntax, GikPairedTermsetOperandsSyntax,
    NonabsTermSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineGekSumtiRejection;

#[requires(true)]
#[ensures(true)]
fn is_bare_sumti_operand(operand: &NonabsTermSyntax) -> bool {
    match operand {
        NonabsTermSyntax::SumtiTerm(_) => true,
        NonabsTermSyntax::ConnectedTerm(_)
        | NonabsTermSyntax::StagBoundTermConnection(_)
        | NonabsTermSyntax::PlaceTaggedSumtiTerm(_)
        | NonabsTermSyntax::JaiTaggedSumtiTerm(_)
        | NonabsTermSyntax::ElidedNaheFihoTagTerm(_)
        | NonabsTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | NonabsTermSyntax::NonabsTaggedSumtiTerm(_)
        | NonabsTermSyntax::NoihaAdverbialTerm(_)
        | NonabsTermSyntax::FihoiAdverbialTerm(_)
        | NonabsTermSyntax::SoiAdverbialTerm(_)
        | NonabsTermSyntax::NaKuTerm(_)
        | NonabsTermSyntax::BareNaTerm(_)
        | NonabsTermSyntax::GekTermset(_)
        | NonabsTermSyntax::ForethoughtTermset(_)
        | NonabsTermSyntax::NuhiTermset(_)
        | NonabsTermSyntax::KeTermset(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_gek_sumti(candidate: &GekTermsetCandidateSyntax) -> bool {
    let GekTermsetCandidateSyntax { gek: _, operands } = candidate;
    match operands.as_ref() {
        BalancedTermsetOperandsSyntax::GikPairedTermsetOperands(pair) => {
            let GikPairedTermsetOperandsSyntax {
                leading_operand,
                gik: _,
                trailing_operand,
            } = pair;
            is_bare_sumti_operand(leading_operand.as_ref())
                && is_bare_sumti_operand(trailing_operand.as_ref())
        }
        BalancedTermsetOperandsSyntax::NestedPairedTermsetOperands(_) => false,
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
fn recovered_is_bare_sumti_operand(operand: &recovered::NonabsTermSyntax) -> bool {
    match operand {
        recovered::NonabsTermSyntax::SumtiTerm(_) => true,
        recovered::NonabsTermSyntax::ConnectedTerm(_)
        | recovered::NonabsTermSyntax::StagBoundTermConnection(_)
        | recovered::NonabsTermSyntax::PlaceTaggedSumtiTerm(_)
        | recovered::NonabsTermSyntax::JaiTaggedSumtiTerm(_)
        | recovered::NonabsTermSyntax::ElidedNaheFihoTagTerm(_)
        | recovered::NonabsTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | recovered::NonabsTermSyntax::NonabsTaggedSumtiTerm(_)
        | recovered::NonabsTermSyntax::NoihaAdverbialTerm(_)
        | recovered::NonabsTermSyntax::FihoiAdverbialTerm(_)
        | recovered::NonabsTermSyntax::SoiAdverbialTerm(_)
        | recovered::NonabsTermSyntax::NaKuTerm(_)
        | recovered::NonabsTermSyntax::BareNaTerm(_)
        | recovered::NonabsTermSyntax::GekTermset(_)
        | recovered::NonabsTermSyntax::ForethoughtTermset(_)
        | recovered::NonabsTermSyntax::NuhiTermset(_)
        | recovered::NonabsTermSyntax::KeTermset(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_gek_sumti(candidate: &recovered::GekTermsetCandidateSyntax) -> bool {
    let recovered::GekTermsetCandidateSyntax { gek: _, operands } = candidate;
    valid(operands).is_some_and(|operands| match operands {
        recovered::BalancedTermsetOperandsSyntax::GikPairedTermsetOperands(pair) => valid(pair)
            .is_some_and(|pair| {
                let recovered::GikPairedTermsetOperandsSyntax {
                    leading_operand,
                    gik: _,
                    trailing_operand,
                } = pair;
                valid(leading_operand).is_some_and(recovered_is_bare_sumti_operand)
                    && valid(trailing_operand).is_some_and(recovered_is_bare_sumti_operand)
            }),
        recovered::BalancedTermsetOperandsSyntax::NestedPairedTermsetOperands(_) => false,
    })
}

#[contract_trait]
impl OutputRejection<GekTermsetCandidateSyntax> for BaselineGekSumtiRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline GEK sumti connection"
    }

    fn rejects(&self, value: &GekTermsetCandidateSyntax) -> bool {
        is_baseline_gek_sumti(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::GekTermsetCandidateSyntax>>
    for BaselineGekSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline GEK sumti connection"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::GekTermsetCandidateSyntax>) -> bool {
        valid(value).is_some_and(recovered_is_baseline_gek_sumti)
    }
}
