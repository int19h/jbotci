//! Ownership classification for rolling Zantufa's connectorless BO arms.
//!
//! Zantufa writes both BO tiers with an OPTIONAL connective — `term_1 <- term_2 (joik_ek?
//! BO_clause term_2)*` (zantufa-1.9999.peg:28) and `sumti_2 <- sumti_3 (joik_ek? tag?
//! BO_clause sumti_3)*` (:35) — while camxes-standard and camxes-exp require it at every BO
//! joint they have. jbotci carries only the delta: its arms admit the connector-ABSENT form
//! alone, so the presence of a connective is the marker that keeps a connective-present extent
//! with its sourced owner (reconciliation B3). The ownership rows that marker settles are
//! `ko'a goi ba ko'e .e bo vi ko'i broda`, which camxes-exp and Zantufa both accept and which
//! stays the sourced normal-flavour arm's, against `ko'a goi pu ko'e bo ca ko'i broda`, which
//! camxes-exp rejects outright.
//!
//! B3 asks for the `#634`-pattern whole-candidate classifier on top of that marker, wherever
//! the arm could still capture a connective-present candidate, and this is where that question
//! is answered rather than assumed. The answer is that the sourced owner can never take a
//! candidate of this arm's extent: reproducing it would require a connective at the very joint
//! whose absence defines the arm, and no connective in either inventory matches the empty
//! string (`sumti_connective` and `term_afterthought_connective` are closed sums of JOI, EK,
//! JEhI and VUhU forms). Each classifier below therefore destructures the completed operand
//! exhaustively and without `..`, states that proof in code, and returns the candidate to this
//! arm — so a later widening toward Zantufa's literal `joik_ek?`, or a new operand shape that
//! can carry a same-level joint, cannot land without this proof being revisited.
//!
//! One shape was measured rather than assumed, because it is the only one where the arm's
//! extent contains a connective at all: jbotci spells a BO chain by recursion, as camxes.peg:143
//! writes it, where Zantufa spells it as a flat continuation list, so `ko'a bo ko'e .e bo ko'i
//! broda` nests a sourced tail inside the connectorless tail's own operand. Rejecting that
//! candidate does NOT hand the extent to the sourced owner — the sourced tail needs a
//! connective directly after `ko'a`, and there is none — it merely pushes the whole surface
//! onto the term tier, where the sumti operand swallows `.e bo ko'i` and the reading stops
//! matching Zantufa's own (`sumti_2`, probed at zantufa-1.9999). Silently re-reading a surface
//! neither source reads that way is exactly what this epic's ownership policy forbids, so the
//! nested sourced tail is left where Zantufa puts it and the epoch ledger carries the row.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    NormalTermAtomSyntax, SimpleTermSyntax, SumtiBoundSyntax, SumtiBoundTailSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

/// The sumti-tier classifier for the connectorless BO tail's operand.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConnectivePresentSumtiBoRejection;

/// The absorption-safe term-tier classifier for the connectorless BO continuation's operand.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConnectivePresentTermBoRejection;

/// The normal-flavour term-tier classifier for the connectorless BO continuation's operand.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConnectivePresentNormalTermBoRejection;

/// Report whether the sourced connective-present tail could own the extent this operand closes.
///
/// It never could. The sourced tail is `connective tag? BO trailing` (camxes.peg:143), so
/// covering the same extent needs a connective at the joint the connectorless arm leaves empty;
/// what the operand itself contains cannot supply one, because the operand's own tails are
/// joints of their own subextents rather than of this one. The tail sum is matched exhaustively
/// so that a third tail shape has to answer this question for itself.
#[requires(true)]
#[ensures(!ret)]
fn sourced_owner_takes_sumti_extent(operand: &SumtiBoundSyntax) -> bool {
    let SumtiBoundSyntax {
        leading_sumti: _,
        bound_tail,
    } = operand;
    match bound_tail {
        None
        | Some(SumtiBoundTailSyntax::BoundSumtiTail(_))
        | Some(SumtiBoundTailSyntax::ZantufaBoundSumtiTail(_)) => false,
    }
}

/// The absorption-safe term-tier twin, with the same answer for a stronger reason: `simple_term`
/// is the leaf inventory BELOW the BO tier, so an operand here cannot carry a same-level joint
/// at all, connective-bearing or not.
#[requires(true)]
#[ensures(!ret)]
fn sourced_owner_takes_simple_term_extent(operand: &SimpleTermSyntax) -> bool {
    match operand {
        SimpleTermSyntax::PlaceTaggedSumtiTerm(_)
        | SimpleTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | SimpleTermSyntax::JaiTaggedSumtiTerm(_)
        | SimpleTermSyntax::ElidedNaheFihoTagTerm(_)
        | SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | SimpleTermSyntax::TaggedSumtiTerm(_)
        | SimpleTermSyntax::NoihaAdverbialTerm(_)
        | SimpleTermSyntax::FihoiAdverbialTerm(_)
        | SimpleTermSyntax::SoiAdverbialTerm(_)
        | SimpleTermSyntax::NaKuTerm(_)
        | SimpleTermSyntax::SumtiTerm(_)
        | SimpleTermSyntax::BareNaTerm(_)
        | SimpleTermSyntax::GekTermset(_)
        | SimpleTermSyntax::ZantufaGekTermset(_)
        | SimpleTermSyntax::ForethoughtTermset(_)
        | SimpleTermSyntax::NuhiTermset(_)
        | SimpleTermSyntax::KeTermset(_) => false,
    }
}

/// The unguarded twin of [`sourced_owner_takes_simple_term_extent`].
#[requires(true)]
#[ensures(!ret)]
fn sourced_owner_takes_normal_term_extent(operand: &NormalTermAtomSyntax) -> bool {
    match operand {
        NormalTermAtomSyntax::PlaceTaggedSumtiTerm(_)
        | NormalTermAtomSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | NormalTermAtomSyntax::JaiTaggedSumtiTerm(_)
        | NormalTermAtomSyntax::ElidedNaheFihoTagTerm(_)
        | NormalTermAtomSyntax::TaggedSumtiBeforeTagTerm(_)
        | NormalTermAtomSyntax::NonabsTaggedSumtiTerm(_)
        | NormalTermAtomSyntax::NoihaAdverbialTerm(_)
        | NormalTermAtomSyntax::FihoiAdverbialTerm(_)
        | NormalTermAtomSyntax::SoiAdverbialTerm(_)
        | NormalTermAtomSyntax::NaKuTerm(_)
        | NormalTermAtomSyntax::SumtiTerm(_)
        | NormalTermAtomSyntax::BareNaTerm(_)
        | NormalTermAtomSyntax::GekTermset(_)
        | NormalTermAtomSyntax::ZantufaGekTermset(_)
        | NormalTermAtomSyntax::ForethoughtTermset(_)
        | NormalTermAtomSyntax::NuhiTermset(_)
        | NormalTermAtomSyntax::KeTermset(_) => false,
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
#[ensures(!ret)]
fn recovered_sourced_owner_takes_sumti_extent(operand: &recovered::SumtiBoundSyntax) -> bool {
    let recovered::SumtiBoundSyntax {
        leading_sumti: _,
        bound_tail,
    } = operand;
    match bound_tail.as_ref().and_then(valid) {
        None
        | Some(recovered::SumtiBoundTailSyntax::BoundSumtiTail(_))
        | Some(recovered::SumtiBoundTailSyntax::ZantufaBoundSumtiTail(_)) => false,
    }
}

#[requires(true)]
#[ensures(!ret)]
fn recovered_sourced_owner_takes_simple_term_extent(operand: &recovered::SimpleTermSyntax) -> bool {
    match operand {
        recovered::SimpleTermSyntax::PlaceTaggedSumtiTerm(_)
        | recovered::SimpleTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | recovered::SimpleTermSyntax::JaiTaggedSumtiTerm(_)
        | recovered::SimpleTermSyntax::ElidedNaheFihoTagTerm(_)
        | recovered::SimpleTermSyntax::TaggedSumtiBeforeTagTerm(_)
        | recovered::SimpleTermSyntax::TaggedSumtiTerm(_)
        | recovered::SimpleTermSyntax::NoihaAdverbialTerm(_)
        | recovered::SimpleTermSyntax::FihoiAdverbialTerm(_)
        | recovered::SimpleTermSyntax::SoiAdverbialTerm(_)
        | recovered::SimpleTermSyntax::NaKuTerm(_)
        | recovered::SimpleTermSyntax::SumtiTerm(_)
        | recovered::SimpleTermSyntax::BareNaTerm(_)
        | recovered::SimpleTermSyntax::GekTermset(_)
        | recovered::SimpleTermSyntax::ZantufaGekTermset(_)
        | recovered::SimpleTermSyntax::ForethoughtTermset(_)
        | recovered::SimpleTermSyntax::NuhiTermset(_)
        | recovered::SimpleTermSyntax::KeTermset(_) => false,
    }
}

#[requires(true)]
#[ensures(!ret)]
fn recovered_sourced_owner_takes_normal_term_extent(
    operand: &recovered::NormalTermAtomSyntax,
) -> bool {
    match operand {
        recovered::NormalTermAtomSyntax::PlaceTaggedSumtiTerm(_)
        | recovered::NormalTermAtomSyntax::ZantufaJoikChainedPlaceTagTerm(_)
        | recovered::NormalTermAtomSyntax::JaiTaggedSumtiTerm(_)
        | recovered::NormalTermAtomSyntax::ElidedNaheFihoTagTerm(_)
        | recovered::NormalTermAtomSyntax::TaggedSumtiBeforeTagTerm(_)
        | recovered::NormalTermAtomSyntax::NonabsTaggedSumtiTerm(_)
        | recovered::NormalTermAtomSyntax::NoihaAdverbialTerm(_)
        | recovered::NormalTermAtomSyntax::FihoiAdverbialTerm(_)
        | recovered::NormalTermAtomSyntax::SoiAdverbialTerm(_)
        | recovered::NormalTermAtomSyntax::NaKuTerm(_)
        | recovered::NormalTermAtomSyntax::SumtiTerm(_)
        | recovered::NormalTermAtomSyntax::BareNaTerm(_)
        | recovered::NormalTermAtomSyntax::GekTermset(_)
        | recovered::NormalTermAtomSyntax::ZantufaGekTermset(_)
        | recovered::NormalTermAtomSyntax::ForethoughtTermset(_)
        | recovered::NormalTermAtomSyntax::NuhiTermset(_)
        | recovered::NormalTermAtomSyntax::KeTermset(_) => false,
    }
}

#[contract_trait]
impl OutputRejection<SumtiBoundSyntax> for ConnectivePresentSumtiBoRejection {
    fn rejected_name(&self) -> &'static str {
        "connective-present BO sumti connection"
    }

    fn rejects(&self, value: &SumtiBoundSyntax) -> bool {
        sourced_owner_takes_sumti_extent(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::SumtiBoundSyntax>>
    for ConnectivePresentSumtiBoRejection
{
    fn rejected_name(&self) -> &'static str {
        "connective-present BO sumti connection"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::SumtiBoundSyntax>) -> bool {
        valid(value).is_some_and(recovered_sourced_owner_takes_sumti_extent)
    }
}

#[contract_trait]
impl OutputRejection<SimpleTermSyntax> for ConnectivePresentTermBoRejection {
    fn rejected_name(&self) -> &'static str {
        "connective-present BO term connection"
    }

    fn rejects(&self, value: &SimpleTermSyntax) -> bool {
        sourced_owner_takes_simple_term_extent(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::SimpleTermSyntax>>
    for ConnectivePresentTermBoRejection
{
    fn rejected_name(&self) -> &'static str {
        "connective-present BO term connection"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::SimpleTermSyntax>) -> bool {
        valid(value).is_some_and(recovered_sourced_owner_takes_simple_term_extent)
    }
}

#[contract_trait]
impl OutputRejection<NormalTermAtomSyntax> for ConnectivePresentNormalTermBoRejection {
    fn rejected_name(&self) -> &'static str {
        "connective-present BO term connection"
    }

    fn rejects(&self, value: &NormalTermAtomSyntax) -> bool {
        sourced_owner_takes_normal_term_extent(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::NormalTermAtomSyntax>>
    for ConnectivePresentNormalTermBoRejection
{
    fn rejected_name(&self) -> &'static str {
        "connective-present BO term connection"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::NormalTermAtomSyntax>) -> bool {
        valid(value).is_some_and(recovered_sourced_owner_takes_normal_term_extent)
    }
}
