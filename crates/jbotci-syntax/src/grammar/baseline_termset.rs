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
//! Rolling Zantufa poses the identical question one branch wider, and `ZantufaBaselineGekSumtiRejection`
//! answers it the same way for the `ZantufaConnectives`-gated `gek_term` arm; see that type.
//!
//! Every candidate product is destructured exhaustively and without `..`, so a model change forces
//! this proof to be revisited.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    BalancedTermsetOperandsSyntax, GekTermsetCandidateSyntax, GikPairedTermsetOperandsSyntax,
    NormalTermSyntax, TermSyntax, ZantufaForethoughtTermsetBranchSyntax,
    ZantufaForethoughtTermsetFirstBranchSyntax, ZantufaGekTermsetCandidateSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineGekSumtiRejection;

/// The same ownership question for rolling Zantufa's own NUhI-less termset.
///
/// Zantufa spells the GEK sumti connection n-ary — `sumti_3 <- (… / gek sumti (gik sumti)+
/// GIhI_elidible) relative_clauses?` (zantufa-1.9999.peg:36) — and gives `ge ko'a gi ko'e gi ko'i
/// broda` to it rather than to `gek_term`, exactly as camxes gives the binary case to `sumti_4`.
/// The extent argument is the one above, one branch wider: when every operand position of the
/// candidate holds exactly one bare sumti term, `gek sumti (gik sumti)+` reconstructs the identical
/// extent, and any other shape — a multi-term run in any position, or a non-sumti operand — has no
/// counterpart in the sumti connection's branches. The GIhI slot does not enter the argument
/// because Zantufa's sumti connection carries one too.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ZantufaBaselineGekSumtiRejection;

#[requires(true)]
#[ensures(true)]
fn is_bare_sumti_operand(operand: &NormalTermSyntax) -> bool {
    match operand {
        NormalTermSyntax::SumtiTerm(_) => true,
        NormalTermSyntax::ConnectedNormalTerm(_)
        | NormalTermSyntax::BoundNormalTermConnection(_)
        | NormalTermSyntax::PlaceTaggedSumtiTerm(_)
        | NormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | NormalTermSyntax::JaiTaggedSumtiTerm(_)
        | NormalTermSyntax::ElidedNaheFihoTagTerm(_)
        | NormalTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | NormalTermSyntax::NonabsTaggedSumtiTerm(_)
        | NormalTermSyntax::NoihaAdverbialTerm(_)
        | NormalTermSyntax::FihoiAdverbialTerm(_)
        | NormalTermSyntax::SoiAdverbialTerm(_)
        | NormalTermSyntax::NaKuTerm(_)
        | NormalTermSyntax::BareNaTerm(_)
        | NormalTermSyntax::GekTermset(_)
        | NormalTermSyntax::ZantufaGekTermset(_)
        | NormalTermSyntax::ForethoughtTermset(_)
        | NormalTermSyntax::NuhiTermset(_)
        | NormalTermSyntax::KeTermset(_) => false,
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
fn recovered_is_bare_sumti_operand(operand: &recovered::NormalTermSyntax) -> bool {
    match operand {
        recovered::NormalTermSyntax::SumtiTerm(_) => true,
        recovered::NormalTermSyntax::ConnectedNormalTerm(_)
        | recovered::NormalTermSyntax::BoundNormalTermConnection(_)
        | recovered::NormalTermSyntax::PlaceTaggedSumtiTerm(_)
        | recovered::NormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | recovered::NormalTermSyntax::JaiTaggedSumtiTerm(_)
        | recovered::NormalTermSyntax::ElidedNaheFihoTagTerm(_)
        | recovered::NormalTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | recovered::NormalTermSyntax::NonabsTaggedSumtiTerm(_)
        | recovered::NormalTermSyntax::NoihaAdverbialTerm(_)
        | recovered::NormalTermSyntax::FihoiAdverbialTerm(_)
        | recovered::NormalTermSyntax::SoiAdverbialTerm(_)
        | recovered::NormalTermSyntax::NaKuTerm(_)
        | recovered::NormalTermSyntax::BareNaTerm(_)
        | recovered::NormalTermSyntax::GekTermset(_)
        | recovered::NormalTermSyntax::ZantufaGekTermset(_)
        | recovered::NormalTermSyntax::ForethoughtTermset(_)
        | recovered::NormalTermSyntax::NuhiTermset(_)
        | recovered::NormalTermSyntax::KeTermset(_) => false,
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

#[requires(true)]
#[ensures(true)]
fn is_bare_sumti_term(term: &TermSyntax) -> bool {
    match term {
        TermSyntax::SumtiTerm(_) => true,
        TermSyntax::PeheTermsetConnection(_)
        | TermSyntax::TermsetGroup(_)
        | TermSyntax::ConnectedTerm(_)
        | TermSyntax::StagBoundTermConnection(_)
        | TermSyntax::PlaceTaggedSumtiTerm(_)
        | TermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | TermSyntax::JaiTaggedSumtiTerm(_)
        | TermSyntax::ElidedNaheFihoTagTerm(_)
        | TermSyntax::TaggedSumtiBeforeTagTerm(_)
        | TermSyntax::TaggedSumtiTerm(_)
        | TermSyntax::NoihaAdverbialTerm(_)
        | TermSyntax::FihoiAdverbialTerm(_)
        | TermSyntax::SoiAdverbialTerm(_)
        | TermSyntax::NaKuTerm(_)
        | TermSyntax::BareNaTerm(_)
        | TermSyntax::GekTermset(_)
        | TermSyntax::ZantufaGekTermset(_)
        | TermSyntax::ForethoughtTermset(_)
        | TermSyntax::NuhiTermset(_)
        | TermSyntax::KeTermset(_) => false,
    }
}

/// Report whether a `term+` operand run is a single bare sumti term.
#[requires(true)]
#[ensures(true)]
fn is_single_bare_sumti_run(terms: &vec1::Vec1<std::sync::Arc<TermSyntax>>) -> bool {
    terms.len() == 1 && is_bare_sumti_term(terms.first().as_ref())
}

#[requires(true)]
#[ensures(true)]
fn is_zantufa_baseline_gek_sumti(candidate: &ZantufaGekTermsetCandidateSyntax) -> bool {
    let ZantufaGekTermsetCandidateSyntax {
        gek: _,
        terms,
        first_branch,
        additional_branches,
        gihi: _,
    } = candidate;
    let ZantufaForethoughtTermsetFirstBranchSyntax {
        gik: _,
        terms: first_branch_terms,
    } = first_branch;
    is_single_bare_sumti_run(terms)
        && is_single_bare_sumti_run(first_branch_terms)
        && additional_branches.iter().all(|branch| {
            let ZantufaForethoughtTermsetBranchSyntax {
                gik: _,
                terms: branch_terms,
            } = branch;
            is_single_bare_sumti_run(branch_terms)
        })
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_bare_sumti_term(term: &recovered::TermSyntax) -> bool {
    match term {
        recovered::TermSyntax::SumtiTerm(_) => true,
        recovered::TermSyntax::PeheTermsetConnection(_)
        | recovered::TermSyntax::TermsetGroup(_)
        | recovered::TermSyntax::ConnectedTerm(_)
        | recovered::TermSyntax::StagBoundTermConnection(_)
        | recovered::TermSyntax::PlaceTaggedSumtiTerm(_)
        | recovered::TermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | recovered::TermSyntax::JaiTaggedSumtiTerm(_)
        | recovered::TermSyntax::ElidedNaheFihoTagTerm(_)
        | recovered::TermSyntax::TaggedSumtiBeforeTagTerm(_)
        | recovered::TermSyntax::TaggedSumtiTerm(_)
        | recovered::TermSyntax::NoihaAdverbialTerm(_)
        | recovered::TermSyntax::FihoiAdverbialTerm(_)
        | recovered::TermSyntax::SoiAdverbialTerm(_)
        | recovered::TermSyntax::NaKuTerm(_)
        | recovered::TermSyntax::BareNaTerm(_)
        | recovered::TermSyntax::GekTermset(_)
        | recovered::TermSyntax::ZantufaGekTermset(_)
        | recovered::TermSyntax::ForethoughtTermset(_)
        | recovered::TermSyntax::NuhiTermset(_)
        | recovered::TermSyntax::KeTermset(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_single_bare_sumti_run(
    terms: &vec1::Vec1<std::sync::Arc<recovered::Recovered<recovered::TermSyntax>>>,
) -> bool {
    terms.len() == 1 && valid(terms.first()).is_some_and(recovered_is_bare_sumti_term)
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_zantufa_baseline_gek_sumti(
    candidate: &recovered::ZantufaGekTermsetCandidateSyntax,
) -> bool {
    let recovered::ZantufaGekTermsetCandidateSyntax {
        gek: _,
        terms,
        first_branch,
        additional_branches,
        gihi: _,
    } = candidate;
    recovered_is_single_bare_sumti_run(terms)
        && valid(first_branch).is_some_and(|first_branch| {
            let recovered::ZantufaForethoughtTermsetFirstBranchSyntax {
                gik: _,
                terms: first_branch_terms,
            } = first_branch;
            recovered_is_single_bare_sumti_run(first_branch_terms)
        })
        && additional_branches.iter().all(|branch| {
            valid(branch).is_some_and(|branch| {
                let recovered::ZantufaForethoughtTermsetBranchSyntax {
                    gik: _,
                    terms: branch_terms,
                } = branch;
                recovered_is_single_bare_sumti_run(branch_terms)
            })
        })
}

#[contract_trait]
impl OutputRejection<ZantufaGekTermsetCandidateSyntax> for ZantufaBaselineGekSumtiRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa GEK sumti connection"
    }

    fn rejects(&self, value: &ZantufaGekTermsetCandidateSyntax) -> bool {
        is_zantufa_baseline_gek_sumti(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaGekTermsetCandidateSyntax>>
    for ZantufaBaselineGekSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa GEK sumti connection"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaGekTermsetCandidateSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_zantufa_baseline_gek_sumti)
    }
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
