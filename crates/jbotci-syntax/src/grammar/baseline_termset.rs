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

use super::generated_model::view::{
    AsView, BalancedTermsetOperandsSyntaxView, GekTermsetCandidateSyntaxRef,
    GekTermsetCandidateSyntaxView, GikPairedTermsetOperandsSyntaxView, NormalTermSyntaxRef,
    NormalTermSyntaxView,
};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineGekSumtiRejection;

#[requires(true)]
#[ensures(true)]
fn is_bare_sumti_operand(operand: NormalTermSyntaxRef<'_>) -> bool {
    match operand.get() {
        NormalTermSyntaxView::SumtiTerm(_) => true,
        NormalTermSyntaxView::ConnectedNormalTerm(_)
        | NormalTermSyntaxView::BoundNormalTermConnection(_)
        | NormalTermSyntaxView::PlaceTaggedSumtiTerm(_)
        | NormalTermSyntaxView::ElidedNaheFihoTagTerm(_)
        | NormalTermSyntaxView::TaggedSumtiBeforeTagTerm(_)
        | NormalTermSyntaxView::NonabsTaggedSumtiTerm(_)
        | NormalTermSyntaxView::NonabsFaChainTaggedSumtiTerm(_)
        | NormalTermSyntaxView::FihoiProposalAdverbialTerm(_)
        | NormalTermSyntaxView::ExpSoiAdverbialTerm(_)
        | NormalTermSyntaxView::NaKuTerm(_)
        | NormalTermSyntaxView::BareNaTerm(_)
        | NormalTermSyntaxView::GekTermset(_)
        | NormalTermSyntaxView::ForethoughtTermset(_)
        | NormalTermSyntaxView::NuhiTermset(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_gek_sumti(candidate: GekTermsetCandidateSyntaxRef<'_>) -> bool {
    let GekTermsetCandidateSyntaxView { gek: _, operands } = candidate.get();
    let Some(operands) = operands.valid() else {
        return false;
    };
    match operands.get() {
        BalancedTermsetOperandsSyntaxView::GikPairedTermsetOperands(pair) => {
            pair.valid().is_some_and(|pair| {
                let GikPairedTermsetOperandsSyntaxView {
                    leading_operand,
                    gik: _,
                    trailing_operand,
                } = pair.get();
                leading_operand.valid().is_some_and(is_bare_sumti_operand)
                    && trailing_operand.valid().is_some_and(is_bare_sumti_operand)
            })
        }
        BalancedTermsetOperandsSyntaxView::NestedPairedTermsetOperands(_) => false,
    }
}

#[contract_trait]
impl<T> OutputRejection<T> for BaselineGekSumtiRejection
where
    for<'tree> T: AsView<'tree, Ref = GekTermsetCandidateSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "baseline GEK sumti connection"
    }
    fn rejects(&self, value: &T) -> bool {
        value.as_view().valid().is_some_and(is_baseline_gek_sumti)
    }
}
