//! Baseline-ownership classification for the extension-first bridi-tail arms.
//!
//! Two rolling-Zantufa arms in the tail family must run before the sourced ones, and each of
//! them can cover an extent a sourced or already-adopted arm covers too. Ordering alone cannot
//! settle those overlaps — the extension arm is the one that has to run first — so the boundary
//! is drawn here, on the completed candidate, in the `#634` pattern epochs 5 and 6c established.
//!
//! **The unbound top continuation.** Zantufa writes `bridi_tail <- bridi_tail_1 (joik_gihek tag?
//! CU_elidible bridi_tail_1)*` (zantufa-1.9999.peg:20). Its operand is the flat level jbotci
//! also starts a tail from, so the arm has to be tried before the flat chain: a flat chain that
//! succeeds on a shorter extent would hide a longer JOIK-led or tag-bearing continuation. Once
//! the candidate is complete, one property decides ownership, and it is a property of the
//! continuations alone: a continuation whose connective is a GIhA and which carries no tag is
//! exactly the sourced flat joint (camxes.peg:78) when its CU is absent, and exactly the adopted
//! camxes-exp reading — the sourced joint over an operand carrying camxes-exp's leading
//! `bridi_tail_2` CU (camxes-exp.peg:107) — when it is present. Both of those have an owner
//! already, so a candidate all of whose continuations are of that shape is returned. A single
//! JOIK/JEK connective or a single tag anywhere in the list keeps the whole candidate here,
//! because nothing outside rolling Zantufa spells either at this joint. The flat joint the
//! shared connective also widens reaches a JOIK-led continuation first whenever it can, being
//! greedy and inside this arm's own operand; the tag is what it cannot reach, so it is the tag
//! that this level ends up carrying.
//!
//! The list is judged as a whole rather than element by element because the arm is one node: a
//! mixed candidate cannot be split between two owners without changing what the tree says the
//! sentence is.
//!
//! **The KE-led tail.** Zantufa writes `bridi_tail_3 <- KE !(selbri_2 KEhE) bridi_tail KEhE?
//! tail_terms / …` (:23). The lookahead is a token predicate over a level the tail itself can
//! derive — `bridi_tail` reaches `selbri_2` through its own selbri arm — so jbotci cannot spell
//! it as a grammar shape without duplicating the ladder, and policy forbids the token lookahead.
//! It is spelled here instead, as the whole-candidate question the lookahead is really asking:
//! could the identical extent be a baseline group rather than a tail? It could exactly when the
//! KE body is a bare selbri, in which case `grouped_tanru_unit` (camxes.peg:184) takes the body
//! and the trailing terms are the selbri tail's own; and when the body is a bare forethought
//! connection with nothing trailing it, in which case `grouped_forethought_bridi_connection`
//! (camxes.peg:190) takes the whole extent. The trailing-terms condition differs between the two
//! because their baseline owners differ in where terms may attach: the selbri arm carries
//! `tail_terms` of its own, the grouped forethought arm does not.
//!
//! Measured, not assumed. `mi ke broda brode ke'e` and `mi ke ge broda gi brode ke'e` are
//! accepted by camxes-standard, camxes-exp and rolling Zantufa alike, so rule R1 of the epoch's
//! decision function keeps them baseline; `mi ke broda co brode ke'e` is Zantufa-only and its
//! owner is epoch 5's `zantufa_ke_co_grouped_tanru_unit`, which this classifier hands it back to
//! for the same reason. `mi ke broda do ke'e` and `mi ke broda gi'e brode ke'e` are Zantufa-only
//! with no grouped-tanru reading of the same extent, and stay here.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{
    AfterthoughtBridiTailSyntax, BoGroupedBridiTailSyntax,
    BoGroupedBridiTailWithoutTailTermsSyntax, BridiTailConnectiveSyntax,
    BridiTailContinuationSyntax, BridiTailContinuationWithoutTailTermsSyntax, BridiTailSyntax,
    SimpleBridiTailSyntax, SimpleBridiTailWithoutTailTermsSyntax, ZantufaContinuedBridiTailSyntax,
    ZantufaContinuedBridiTailWithoutTailTermsSyntax, ZantufaGroupedBridiTailSyntax,
    ZantufaTailContinuationSyntax, ZantufaTailContinuationWithoutTailTermsSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

/// The completed-candidate classifier for the unbound top continuation.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineTailContinuationRejection;

/// The tail-terms-free twin of [`BaselineTailContinuationRejection`].
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineTailContinuationWithoutTailTermsRejection;

/// The completed-candidate ownership guard for the Zantufa KE-led tail.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GroupedTanruKeTailRejection;

/// The flat joint's guard against pairing a Zantufa connective with a camxes-exp operand prefix.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpPrefixUnderZantufaConnectiveRejection;

/// The tail-terms-free twin of [`ExpPrefixUnderZantufaConnectiveRejection`].
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpPrefixUnderZantufaConnectiveWithoutTailTermsRejection;

#[requires(true)]
#[ensures(true)]
fn valid<T>(value: &recovered::Recovered<T>) -> Option<&T> {
    match value {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

/// Report whether one continuation is the sourced flat joint, or that joint over an operand
/// carrying camxes-exp's leading CU. The connective sum is matched exhaustively so that a further
/// spelling has to answer this question for itself.
#[requires(true)]
#[ensures(true)]
fn continuation_has_owner(connective: &BridiTailConnectiveSyntax, tagged: bool) -> bool {
    !tagged
        && match connective {
            BridiTailConnectiveSyntax::GihekConnective(_) => true,
            BridiTailConnectiveSyntax::JoikConnective(_)
            | BridiTailConnectiveSyntax::JekConnective(_) => false,
        }
}

#[requires(true)]
#[ensures(true)]
fn continued_tail_has_owner(candidate: &ZantufaContinuedBridiTailSyntax) -> bool {
    let ZantufaContinuedBridiTailSyntax {
        first: _,
        continuations,
    } = candidate;
    continuations.iter().all(|continuation| {
        let ZantufaTailContinuationSyntax {
            connective,
            tense_modal,
            cu: _,
            bridi_tail: _,
        } = continuation;
        continuation_has_owner(connective, tense_modal.is_some())
    })
}

#[requires(true)]
#[ensures(true)]
fn continued_tail_without_tail_terms_has_owner(
    candidate: &ZantufaContinuedBridiTailWithoutTailTermsSyntax,
) -> bool {
    let ZantufaContinuedBridiTailWithoutTailTermsSyntax {
        first: _,
        continuations,
    } = candidate;
    continuations.iter().all(|continuation| {
        let ZantufaTailContinuationWithoutTailTermsSyntax {
            connective,
            tense_modal,
            cu: _,
            bridi_tail: _,
        } = continuation;
        continuation_has_owner(connective, tense_modal.is_some())
    })
}

/// The shape of a KE body that a baseline group could take over the identical extent.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeBodyShape {
    /// A bare selbri: `grouped_tanru_unit` takes the body, trailing terms stay the selbri's.
    BareSelbri,
    /// A bare forethought connection: `grouped_forethought_bridi_connection` takes the whole
    /// extent, and it has no place for terms after its KEhE.
    BareForethought,
    /// Anything else: the body carries a joint, terms or a VAU that no baseline group can hold.
    TailOnly,
}

#[requires(true)]
#[ensures(true)]
fn bo_grouped_shape(operand: &BoGroupedBridiTailSyntax) -> KeBodyShape {
    let BoGroupedBridiTailSyntax {
        cu,
        first,
        bo_continuation,
    } = operand;
    // A leading CU or a BO joint is content no baseline group can hold.
    if cu.is_some() || bo_continuation.is_some() {
        return KeBodyShape::TailOnly;
    }
    match first.as_ref() {
        SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(_) => KeBodyShape::BareForethought,
        SimpleBridiTailSyntax::SelbriSimpleBridiTail(selbri_tail) => {
            if selbri_tail.terms.is_empty() && selbri_tail.vau.is_none() {
                KeBodyShape::BareSelbri
            } else {
                KeBodyShape::TailOnly
            }
        }
        SimpleBridiTailSyntax::ExpPrefixedSimpleBridiTail(_) => KeBodyShape::TailOnly,
    }
}

#[requires(true)]
#[ensures(true)]
fn ke_body_shape(body: &BridiTailSyntax) -> KeBodyShape {
    match body {
        BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
            if tail.ke_continuation.is_some() {
                return KeBodyShape::TailOnly;
            }
            let AfterthoughtBridiTailSyntax(chain) = tail.first.as_ref();
            if !chain.links.is_empty() {
                return KeBodyShape::TailOnly;
            }
            bo_grouped_shape(&chain.first)
        }
        // Every other arm carries something no baseline group can hold: a top continuation, a
        // KE tail of its own, or the tail-terms-free flavour, which is only ever selected when
        // the with-tail-terms one failed over this extent.
        BridiTailSyntax::ZantufaPriorityContinuedBridiTail(_)
        | BridiTailSyntax::ZantufaPriorityContinuedBridiTailWithoutTailTerms(_)
        | BridiTailSyntax::ZantufaPriorityGroupedBridiTail(_)
        | BridiTailSyntax::BridiTailWithoutTailTerms(_) => KeBodyShape::TailOnly,
    }
}

#[requires(true)]
#[ensures(true)]
fn grouped_ke_tail_has_baseline_reading(candidate: &ZantufaGroupedBridiTailSyntax) -> bool {
    let ZantufaGroupedBridiTailSyntax {
        ke: _,
        bridi_tail,
        kehe: _,
        tail_terms,
        vau,
    } = candidate;
    match ke_body_shape(bridi_tail) {
        KeBodyShape::BareSelbri => true,
        KeBodyShape::BareForethought => tail_terms.is_empty() && vau.is_none(),
        KeBodyShape::TailOnly => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_continued_tail_has_owner(
    candidate: &recovered::ZantufaContinuedBridiTailSyntax,
) -> bool {
    let recovered::ZantufaContinuedBridiTailSyntax {
        first: _,
        continuations,
    } = candidate;
    continuations.iter().all(|continuation| {
        // Fail closed: a continuation the recovery pass could not complete is not evidence that
        // a baseline or adopted-exp owner exists over this extent, so it keeps the whole
        // candidate here exactly as a JOIK/JEK or tagged one would.
        valid(continuation).is_some_and(|continuation| {
            let recovered::ZantufaTailContinuationSyntax {
                connective,
                tense_modal,
                cu: _,
                bridi_tail: _,
            } = continuation;
            valid(connective).is_some_and(|connective| {
                recovered_connective_has_owner(connective, tense_modal.is_some())
            })
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_connective_has_owner(
    connective: &recovered::BridiTailConnectiveSyntax,
    tagged: bool,
) -> bool {
    !tagged
        && match connective {
            recovered::BridiTailConnectiveSyntax::GihekConnective(_) => true,
            recovered::BridiTailConnectiveSyntax::JoikConnective(_)
            | recovered::BridiTailConnectiveSyntax::JekConnective(_) => false,
        }
}

#[requires(true)]
#[ensures(true)]
fn recovered_continued_tail_without_tail_terms_has_owner(
    candidate: &recovered::ZantufaContinuedBridiTailWithoutTailTermsSyntax,
) -> bool {
    let recovered::ZantufaContinuedBridiTailWithoutTailTermsSyntax {
        first: _,
        continuations,
    } = candidate;
    continuations.iter().all(|continuation| {
        // Fail closed for the same reason the with-tail-terms twin does.
        valid(continuation).is_some_and(|continuation| {
            let recovered::ZantufaTailContinuationWithoutTailTermsSyntax {
                connective,
                tense_modal,
                cu: _,
                bridi_tail: _,
            } = continuation;
            valid(connective).is_some_and(|connective| {
                recovered_connective_has_owner(connective, tense_modal.is_some())
            })
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_bo_grouped_shape(operand: &recovered::BoGroupedBridiTailSyntax) -> KeBodyShape {
    let recovered::BoGroupedBridiTailSyntax {
        cu,
        first,
        bo_continuation,
    } = operand;
    if cu.is_some() || bo_continuation.is_some() {
        return KeBodyShape::TailOnly;
    }
    match valid(first.as_ref()) {
        None => KeBodyShape::TailOnly,
        Some(recovered::SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(_)) => {
            KeBodyShape::BareForethought
        }
        Some(recovered::SimpleBridiTailSyntax::SelbriSimpleBridiTail(selbri_tail)) => {
            match valid(selbri_tail) {
                Some(selbri_tail) if selbri_tail.terms.is_empty() && selbri_tail.vau.is_none() => {
                    KeBodyShape::BareSelbri
                }
                _ => KeBodyShape::TailOnly,
            }
        }
        Some(recovered::SimpleBridiTailSyntax::ExpPrefixedSimpleBridiTail(_)) => {
            KeBodyShape::TailOnly
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_ke_body_shape(body: &recovered::BridiTailSyntax) -> KeBodyShape {
    match body {
        recovered::BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
            let Some(tail) = valid(tail) else {
                return KeBodyShape::TailOnly;
            };
            if tail.ke_continuation.is_some() {
                return KeBodyShape::TailOnly;
            }
            let Some(recovered::AfterthoughtBridiTailSyntax(chain)) = valid(tail.first.as_ref())
            else {
                return KeBodyShape::TailOnly;
            };
            if !chain.links.is_empty() {
                return KeBodyShape::TailOnly;
            }
            match valid(&chain.first) {
                None => KeBodyShape::TailOnly,
                Some(operand) => recovered_bo_grouped_shape(operand),
            }
        }
        recovered::BridiTailSyntax::ZantufaPriorityContinuedBridiTail(_)
        | recovered::BridiTailSyntax::ZantufaPriorityContinuedBridiTailWithoutTailTerms(_)
        | recovered::BridiTailSyntax::ZantufaPriorityGroupedBridiTail(_)
        | recovered::BridiTailSyntax::BridiTailWithoutTailTerms(_) => KeBodyShape::TailOnly,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_grouped_ke_tail_has_baseline_reading(
    candidate: &recovered::ZantufaGroupedBridiTailSyntax,
) -> bool {
    let recovered::ZantufaGroupedBridiTailSyntax {
        ke: _,
        bridi_tail,
        kehe: _,
        tail_terms,
        vau,
    } = candidate;
    match valid(bridi_tail.as_ref()).map(recovered_ke_body_shape) {
        Some(KeBodyShape::BareSelbri) => true,
        Some(KeBodyShape::BareForethought) => tail_terms.is_empty() && vau.is_none(),
        Some(KeBodyShape::TailOnly) | None => false,
    }
}

#[contract_trait]
impl OutputRejection<ZantufaContinuedBridiTailSyntax> for BaselineTailContinuationRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa unbound bridi-tail continuation"
    }

    fn rejects(&self, value: &ZantufaContinuedBridiTailSyntax) -> bool {
        continued_tail_has_owner(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaContinuedBridiTailSyntax>>
    for BaselineTailContinuationRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa unbound bridi-tail continuation"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaContinuedBridiTailSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_continued_tail_has_owner)
    }
}

#[contract_trait]
impl OutputRejection<ZantufaContinuedBridiTailWithoutTailTermsSyntax>
    for BaselineTailContinuationWithoutTailTermsRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa unbound bridi-tail continuation"
    }

    fn rejects(&self, value: &ZantufaContinuedBridiTailWithoutTailTermsSyntax) -> bool {
        continued_tail_without_tail_terms_has_owner(value)
    }
}

#[contract_trait]
impl
    OutputRejection<
        recovered::Recovered<recovered::ZantufaContinuedBridiTailWithoutTailTermsSyntax>,
    > for BaselineTailContinuationWithoutTailTermsRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa unbound bridi-tail continuation"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaContinuedBridiTailWithoutTailTermsSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_continued_tail_without_tail_terms_has_owner)
    }
}

#[contract_trait]
impl OutputRejection<ZantufaGroupedBridiTailSyntax> for GroupedTanruKeTailRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa KE bridi-tail grouping"
    }

    fn rejects(&self, value: &ZantufaGroupedBridiTailSyntax) -> bool {
        grouped_ke_tail_has_baseline_reading(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaGroupedBridiTailSyntax>>
    for GroupedTanruKeTailRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa KE bridi-tail grouping"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaGroupedBridiTailSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_grouped_ke_tail_has_baseline_reading)
    }
}

/// Report whether a bridi-tail joint's connective is one of the arms rolling Zantufa adds.
#[requires(true)]
#[ensures(true)]
fn connective_is_zantufa(connective: &BridiTailConnectiveSyntax) -> bool {
    !continuation_has_owner(connective, false)
}

/// Report whether a flat-joint operand opens with camxes-exp's own prefix.
#[requires(true)]
#[ensures(true)]
fn operand_carries_exp_prefix(operand: &BoGroupedBridiTailSyntax) -> bool {
    operand.cu.is_some()
        || matches!(
            operand.first.as_ref(),
            SimpleBridiTailSyntax::ExpPrefixedSimpleBridiTail(_)
        )
}

/// The tail-terms-free twin of [`operand_carries_exp_prefix`].
#[requires(true)]
#[ensures(true)]
fn operand_without_tail_terms_carries_exp_prefix(
    operand: &BoGroupedBridiTailWithoutTailTermsSyntax,
) -> bool {
    operand.cu.is_some()
        || matches!(
            operand.first.as_ref(),
            SimpleBridiTailWithoutTailTermsSyntax::ExpPrefixedSimpleBridiTailWithoutTailTerms(_)
        )
}

#[contract_trait]
impl OutputRejection<BridiTailContinuationSyntax> for ExpPrefixUnderZantufaConnectiveRejection {
    fn rejected_name(&self) -> &'static str {
        "Zantufa flat bridi-tail joint over a camxes-exp prefix"
    }

    fn rejects(&self, value: &BridiTailContinuationSyntax) -> bool {
        connective_is_zantufa(&value.connective) && operand_carries_exp_prefix(&value.bridi_tail)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::BridiTailContinuationSyntax>>
    for ExpPrefixUnderZantufaConnectiveRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa flat bridi-tail joint over a camxes-exp prefix"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::BridiTailContinuationSyntax>,
    ) -> bool {
        valid(value).is_some_and(|value| {
            valid(&value.connective)
                .is_some_and(|connective| !recovered_connective_has_owner(connective, false))
                && valid(value.bridi_tail.as_ref()).is_some_and(|operand| {
                    operand.cu.is_some()
                        || matches!(
                            valid(operand.first.as_ref()),
                            Some(recovered::SimpleBridiTailSyntax::ExpPrefixedSimpleBridiTail(_))
                        )
                })
        })
    }
}

#[contract_trait]
impl OutputRejection<BridiTailContinuationWithoutTailTermsSyntax>
    for ExpPrefixUnderZantufaConnectiveWithoutTailTermsRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa flat bridi-tail joint over a camxes-exp prefix"
    }

    fn rejects(&self, value: &BridiTailContinuationWithoutTailTermsSyntax) -> bool {
        connective_is_zantufa(&value.connective)
            && operand_without_tail_terms_carries_exp_prefix(&value.bridi_tail)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::BridiTailContinuationWithoutTailTermsSyntax>>
    for ExpPrefixUnderZantufaConnectiveWithoutTailTermsRejection
{
    fn rejected_name(&self) -> &'static str {
        "Zantufa flat bridi-tail joint over a camxes-exp prefix"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::BridiTailContinuationWithoutTailTermsSyntax>,
    ) -> bool {
        valid(value).is_some_and(|value| {
            valid(&value.connective).is_some_and(|connective| {
                !recovered_connective_has_owner(connective, false)
            }) && valid(value.bridi_tail.as_ref()).is_some_and(|operand| {
                operand.cu.is_some()
                    || matches!(
                        valid(operand.first.as_ref()),
                        Some(
                            recovered::SimpleBridiTailWithoutTailTermsSyntax::ExpPrefixedSimpleBridiTailWithoutTailTerms(_)
                        )
                    )
            })
        })
    }
}
