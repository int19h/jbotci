//! Typed ownership classification for camxes-exp relative-clause continuations.
//!
//! The extension route runs first so it can classify a completed connective-plus-clause
//! candidate. A bare ZIhE connective is rejected here and reparsed by the baseline arm.
//! Both routes begin at the connective and end at the same completed relative-clause atom,
//! so the reparse has identical extent. Token-class lookahead cannot establish this ownership:
//! several words participate in multiple selma'o, and only the completed candidate proves which
//! relative-clause route succeeded. Every generated node used by this proof is destructured
//! exhaustively and without `..`, so model changes force the proof to be revisited.

use bityzba::{contract_trait, invariant, requires};
use jbotci_morphology::Cmavo;

use super::generated_model::{
    BridiSyntax, ExpRelativeClauseConnectiveSyntax, ExpRelativeContinuationSyntax,
    ExpSelbriRelativeClauseConnectiveSyntax, ExpSelbriRelativeClauseContinuationSyntax,
    ExpSoiSubsentenceAdverbialSyntax, SimpleIntervalConnectiveSyntax, SubbridiSyntax, TermSyntax,
    recovered,
};
use super::generated_runtime::OutputRejection;
use crate::tree::WithFreeModifiers;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineRelativeContinuationRejection;

#[requires(true)]
#[ensures(true)]
fn is_baseline_relative_continuation(value: &ExpRelativeContinuationSyntax) -> bool {
    let ExpRelativeContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let ExpRelativeClauseConnectiveSyntax { na, se, head, nai } = connective.as_ref();
    na.is_none() && se.is_none() && head.value.cmavo() == Some(Cmavo::Zihe) && nai.is_none()
}

#[requires(true)]
#[ensures(true)]
fn valid<T>(value: &(impl std::borrow::Borrow<recovered::Recovered<T>> + ?Sized)) -> Option<&T> {
    match value.borrow() {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_relative_continuation(
    value: &recovered::ExpRelativeContinuationSyntax,
) -> bool {
    let recovered::ExpRelativeContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let Some(connective) = valid(connective) else {
        return false;
    };
    let recovered::ExpRelativeClauseConnectiveSyntax { na, se, head, nai } = connective;
    na.is_none()
        && se.is_none()
        && valid(&head.value).is_some_and(|head| head.cmavo() == Some(Cmavo::Zihe))
        && nai.is_none()
}

#[contract_trait]
impl OutputRejection<ExpRelativeContinuationSyntax> for BaselineRelativeContinuationRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline ZIhE relative continuation"
    }

    fn rejects(&self, value: &ExpRelativeContinuationSyntax) -> bool {
        is_baseline_relative_continuation(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpRelativeContinuationSyntax>>
    for BaselineRelativeContinuationRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline ZIhE relative continuation"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpRelativeContinuationSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_baseline_relative_continuation)
    }
}

/// Placement classifier for the free-modifier slot camxes-exp's `joik` does not spell.
///
/// `joik <- NA_clause? SE_clause? (JOI_clause / JA_clause / A_clause) NAI_clause? / interval /
/// GAhO_clause interval GAhO_clause` with `interval <- SE_clause? BIhI_clause NAI_clause?`
/// (camxes-exp.peg:347-349), and the `free*` both relative chains carry is OUTSIDE it -- `(ZIhE_clause
/// / joik) free* relative_clause` (:199, :214). The `_clause` wrappers do not restore the slot:
/// `post_clause <- spaces? si_clause? !ZEI_clause !BU_clause indicators*` carries indicators, not
/// frees. So `je to do brodi toi nai` is not a connective camxes-exp derives, while `je nai to do
/// brodi toi` and `je to do brodi toi` are -- the free modifiers of the latter two being the
/// chain's own `free*`.
///
/// jbotci's connective nodes spell that slot on the head instead, and they are shared: the same
/// `exp_relative_clause_connective` serves the ordinary relative chain at :199, and both interval
/// nodes serve the baseline `joik_connective`, whose consumers supply no outer `free*` at all.
/// Removing the slot from these shared nodes also removes accepted surfaces from other routes,
/// such as `lo broda poi mi brode je to do brodi toi nai poi do brodi ku cu brodi`
/// and `li pa bi'i to do brodi toi nai li re`. Issue #847 covers those placements.
/// This classifier refuses the placement on the selbri relative chain only.
///
/// Only the two head-before-optional-NAI shapes can present it. `zihe_selbri_relative_connective`
/// spells `ZIhE_clause` alone, whose trailing frees are the chain's own; `closed_interval_connective`
/// already carries its slot on the closing GAhO, after the NAI, where the source puts it.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProhibitedRelativeConnectiveFreeModifierRejection;

#[requires(true)]
#[ensures(true)]
fn is_prohibited_connective_free_modifier(
    value: &ExpSelbriRelativeClauseContinuationSyntax,
) -> bool {
    let ExpSelbriRelativeClauseContinuationSyntax {
        connective,
        inner: _,
    } = value;
    match connective.as_ref() {
        ExpSelbriRelativeClauseConnectiveSyntax::ZiheSelbriRelativeConnective(_)
        | ExpSelbriRelativeClauseConnectiveSyntax::ClosedIntervalConnective(_) => false,
        ExpSelbriRelativeClauseConnectiveSyntax::ExpRelativeClauseConnective(connective) => {
            let ExpRelativeClauseConnectiveSyntax {
                na: _,
                se: _,
                head,
                nai,
            } = connective.as_ref();
            connective_free_modifier_placement(head, nai.as_ref())
                == ConnectiveFreeModifierPlacement::Prohibited
        }
        ExpSelbriRelativeClauseConnectiveSyntax::SimpleIntervalConnective(connective) => {
            let SimpleIntervalConnectiveSyntax { se: _, bihi, nai } = connective.as_ref();
            connective_free_modifier_placement(bihi, nai.as_ref())
                == ConnectiveFreeModifierPlacement::Prohibited
        }
    }
}

/// The placement that valid connective nodes prove.
/// The chain rejects prohibited placement. The list returns permitted placement to the chain.
/// An unproven result must trigger neither action. A boolean cannot express all three cases.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectiveFreeModifierPlacement {
    /// A connective camxes-exp's `joik` does spell: no `NAI` for a free modifier to stand in
    /// front of, or no free modifier standing there. Trailing frees with no `NAI` after them are
    /// the chain's own `free*`.
    Permitted,
    /// A `NAI` present and at least one free modifier standing in front of it: the slot
    /// camxes-exp keeps outside the connective.
    Prohibited,
    /// Neither, because a node the answer depends on is a recovery placeholder.
    Unproven,
}

/// The shared half of the two head-before-optional-NAI shapes, over a tree with no recovery
/// placeholders in it. It is one predicate rather than two so the chain's rejection and the S3
/// list classifier that has to anticipate it cannot drift apart.
///
/// The strict twin needs no third state and the contract says so rather than leaving it to be
/// assumed: every slot of a non-recovered tree either holds the node it is for or is absent, so
/// `Unproven` is unconstructible here and the two consumers may read this result in either
/// direction. They read it positively anyway, so that the strict and recovered arms of each
/// consumer ask the same question in the same words.
#[requires(true)]
#[ensures(
    ret != ConnectiveFreeModifierPlacement::Unproven,
    "a tree with no recovery placeholders proves every placement fact"
)]
fn connective_free_modifier_placement<T, F>(
    head: &WithFreeModifiers<T, F>,
    nai: Option<&WithFreeModifiers<T, F>>,
) -> ConnectiveFreeModifierPlacement {
    if nai.is_some() && !head.free_modifiers.is_empty() {
        ConnectiveFreeModifierPlacement::Prohibited
    } else {
        ConnectiveFreeModifierPlacement::Permitted
    }
}

/// The prohibited placement, read off a recovered connective.
///
/// Every fact the rejection stands on has to be a node the recovery tree actually proved: the
/// head/BIhI token, the NAI token, and at least one free modifier standing between them. A
/// `Prefix` or `Error` placeholder occupies the same slot as the thing it stands in for, so
/// reading presence off the slot alone would let a run of unparsed input withdraw a surface
/// this classifier is not entitled to withdraw.
#[requires(true)]
#[ensures(true)]
fn recovered_is_prohibited_connective_free_modifier(
    value: &recovered::ExpSelbriRelativeClauseContinuationSyntax,
) -> bool {
    let recovered::ExpSelbriRelativeClauseContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let Some(connective) = valid(connective) else {
        return false;
    };
    match connective {
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::ZiheSelbriRelativeConnective(_)
        | recovered::ExpSelbriRelativeClauseConnectiveSyntax::ClosedIntervalConnective(_) => false,
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::ExpRelativeClauseConnective(
            connective,
        ) => valid(connective).is_some_and(|connective| {
            let recovered::ExpRelativeClauseConnectiveSyntax {
                na: _,
                se: _,
                head,
                nai,
            } = connective;
            recovered_connective_free_modifier_placement(head, nai.as_ref())
                == ConnectiveFreeModifierPlacement::Prohibited
        }),
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::SimpleIntervalConnective(
            connective,
        ) => valid(connective).is_some_and(|connective| {
            let recovered::SimpleIntervalConnectiveSyntax { se: _, bihi, nai } = connective;
            recovered_connective_free_modifier_placement(bihi, nai.as_ref())
                == ConnectiveFreeModifierPlacement::Prohibited
        }),
    }
}

/// The recovered twin, read off nodes the recovery tree actually proved.
///
/// `Prohibited` needs all three of a proven head token, a proven NAI token after it, and a proven
/// free modifier in between. `Permitted` needs the
/// head proven too, and then either an absent NAI slot or an empty free-modifier slot beside a
/// proven NAI. An absent optional slot is a shape fact of a node that parsed, but a placeholder
/// is not: a placeholder head stands for a run of unparsed input that could hold the frees, and a
/// placeholder NAI stands for one that could have consumed the frees that would have stood in
/// front of it, so an empty free-modifier slot beside it proves nothing. What is left -- a NAI
/// slot holding a placeholder, or a free-modifier slot holding nothing but placeholders -- proves
/// neither answer and is `Unproven`.
#[requires(true)]
#[ensures(true)]
fn recovered_connective_free_modifier_placement<T, F>(
    head: &recovered::WithFreeModifiers<recovered::Recovered<T>, F>,
    nai: Option<&recovered::WithFreeModifiers<recovered::Recovered<T>, F>>,
) -> ConnectiveFreeModifierPlacement
where
    F: std::borrow::Borrow<recovered::Recovered<recovered::FreeModifierSyntax>>,
{
    if valid(&head.value).is_none() {
        return ConnectiveFreeModifierPlacement::Unproven;
    }
    let Some(nai) = nai else {
        return ConnectiveFreeModifierPlacement::Permitted;
    };
    if valid(&nai.value).is_none() {
        return ConnectiveFreeModifierPlacement::Unproven;
    }
    if head
        .free_modifiers
        .iter()
        .any(|free_modifier| valid(free_modifier).is_some())
    {
        ConnectiveFreeModifierPlacement::Prohibited
    } else if head.free_modifiers.is_empty() {
        ConnectiveFreeModifierPlacement::Permitted
    } else {
        ConnectiveFreeModifierPlacement::Unproven
    }
}

#[contract_trait]
impl OutputRejection<ExpSelbriRelativeClauseContinuationSyntax>
    for ProhibitedRelativeConnectiveFreeModifierRejection
{
    fn rejected_name(&self) -> &'static str {
        "free modifier before the connective's NAI"
    }

    fn rejects(&self, value: &ExpSelbriRelativeClauseContinuationSyntax) -> bool {
        is_prohibited_connective_free_modifier(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpSelbriRelativeClauseContinuationSyntax>>
    for ProhibitedRelativeConnectiveFreeModifierRejection
{
    fn rejected_name(&self) -> &'static str {
        "free modifier before the connective's NAI"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpSelbriRelativeClauseContinuationSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_prohibited_connective_free_modifier)
    }
}

/// R1 no-steal for the camxes-exp SOI adverbial.
///
/// `mi broda soi mi brode` is accepted by all three reference parsers and camxes-standard
/// reads it as the reciprocal `soi mi` with `brode` continuing the tanru outside. The
/// adverbial arm therefore returns any completed candidate that reparses that way: the marker
/// is `soi` -- `xoi` and `fi'oi` are in no reciprocal -- the SEhU is elided, so the extent has
/// no terminator of its own to keep it whole, and the subsentence opens with the exact `sumti`
/// the reciprocal would take as its `leading_sumti`. An explicit SEhU, a body whose first term
/// is anything other than a bare sumti, or either of the other two markers is not a reparse the
/// baseline can produce, and stays here.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineReciprocalSoiRejection;

/// True when the candidate's body opens with the exact constituent the reciprocal would take as
/// its `leading_sumti`.
///
/// `soi_free_modifier` spells `SOI free* sumti sumti? SEhU_elidible`, so the reparse needs a
/// bare `sumti` in first position, not merely a first `term`: `term` also covers tagged sumti,
/// termsets, `na ku` and the adverbials themselves, none of which the reciprocal can consume.
/// `sumti_term` is `term`'s one arm that is exactly `sumti`, so the reparse is proved by the
/// arm rather than inferred from the run being non-empty.
#[requires(true)]
#[ensures(true)]
fn subsentence_opens_with_leading_sumti(value: &SubbridiSyntax) -> bool {
    match value {
        SubbridiSyntax::PrenexSubbridi(_) => false,
        SubbridiSyntax::BridiSubbridi(bridi) => match bridi.0.as_ref() {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                matches!(
                    bridi.leading_terms.first().as_ref(),
                    TermSyntax::SumtiTerm(_)
                )
            }
            BridiSyntax::BareCuBridi(_) | BridiSyntax::RelationOnlyBridi(_) => false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_subsentence_opens_with_leading_sumti(value: &recovered::SubbridiSyntax) -> bool {
    match value {
        recovered::SubbridiSyntax::PrenexSubbridi(_) => false,
        recovered::SubbridiSyntax::BridiSubbridi(bridi) => valid(bridi).is_some_and(|bridi| {
            valid(&bridi.0).is_some_and(|bridi| match bridi {
                recovered::BridiSyntax::BridiWithLeadingTerms(bridi) => {
                    valid(bridi).is_some_and(|bridi| {
                        valid(bridi.leading_terms.first())
                            .is_some_and(|term| matches!(term, recovered::TermSyntax::SumtiTerm(_)))
                    })
                }
                recovered::BridiSyntax::BareCuBridi(_)
                | recovered::BridiSyntax::RelationOnlyBridi(_) => false,
            })
        }),
    }
}

#[contract_trait]
impl OutputRejection<ExpSoiSubsentenceAdverbialSyntax> for BaselineReciprocalSoiRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline SOI reciprocal"
    }

    fn rejects(&self, value: &ExpSoiSubsentenceAdverbialSyntax) -> bool {
        value.soi.value.cmavo() == Some(Cmavo::Soi)
            && value.sehu.is_none()
            && subsentence_opens_with_leading_sumti(&value.subsentence)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpSoiSubsentenceAdverbialSyntax>>
    for BaselineReciprocalSoiRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline SOI reciprocal"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpSoiSubsentenceAdverbialSyntax>,
    ) -> bool {
        valid(value).is_some_and(|value| {
            valid(&value.soi.value).is_some_and(|soi| soi.cmavo() == Some(Cmavo::Soi))
                && value.sehu.is_none()
                && valid(&value.subsentence)
                    .is_some_and(recovered_subsentence_opens_with_leading_sumti)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[allow(unused_imports)]
    use bityzba::{ensures, new, requires};
    use jbotci_morphology::segment_words_with_modifiers;

    use crate::grammar::{SyntaxRecoveryItemData, syntax_tokens};
    use crate::tree::SyntaxRecoveryItem;

    use super::*;

    /// A recovery placeholder standing in for a child that did not parse.
    #[requires(true)]
    #[ensures(true)]
    fn recovery_placeholder() -> SyntaxRecoveryItem {
        let span = jbotci_diagnostics::source_span_from_byte_offsets(None, "", 0, 0)
            .expect("valid zero-width source span");
        new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 0,
            span: Arc::new(span),
            expected: "statement".to_owned(),
        })
    }

    /// The single syntax token the given text -- which must segment to exactly one word --
    /// morphologises to.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn one_token(text: &str) -> crate::tree::Token {
        let words = segment_words_with_modifiers(text).expect("valid morphology");
        let tokens = syntax_tokens(&words);
        let [token] = tokens.as_slice() else {
            panic!("text must be exactly one word");
        };
        token.clone()
    }

    /// A free modifier whose own payload did not parse. It is still a free-modifier node the
    /// tree proved to be there; a `Recovered::error` in its place is not.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_free_modifier() -> recovered::Recovered<recovered::FreeModifierSyntax> {
        recovered::Recovered::valid(recovered::FreeModifierSyntax::ParentheticalText(Arc::new(
            recovered::Recovered::error(recovery_placeholder()),
        )))
    }

    /// A free-modifier slot entry standing in for a free modifier that did not parse.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_unparsed_free_modifier() -> recovered::Recovered<recovered::FreeModifierSyntax> {
        recovered::Recovered::error(recovery_placeholder())
    }

    /// A proven token, over text that must segment to exactly one word.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn recovered_token(text: &str) -> recovered::Recovered<crate::tree::Token> {
        recovered::Recovered::valid(one_token(text))
    }

    /// A token slot standing in for a token that did not parse.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_unparsed_token() -> recovered::Recovered<crate::tree::Token> {
        recovered::Recovered::error(recovery_placeholder())
    }

    /// A `WithFreeModifiers` head slot over the given token state and free-modifier states.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_head_slot(
        value: recovered::Recovered<crate::tree::Token>,
        free_modifiers: Vec<recovered::Recovered<recovered::FreeModifierSyntax>>,
    ) -> recovered::WithFreeModifiers<
        recovered::Recovered<crate::tree::Token>,
        Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
    > {
        recovered::WithFreeModifiers {
            value,
            free_modifiers: free_modifiers.into_iter().map(Arc::new).collect(),
        }
    }

    /// A continuation whose clause did not parse: the placement classifier ignores `inner`, so
    /// the connective is the whole of what these cases vary.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_selbri_continuation(
        connective: recovered::Recovered<recovered::ExpSelbriRelativeClauseConnectiveSyntax>,
    ) -> recovered::Recovered<recovered::ExpSelbriRelativeClauseContinuationSyntax> {
        recovered::Recovered::valid(recovered::ExpSelbriRelativeClauseContinuationSyntax {
            connective: Arc::new(connective),
            inner: Arc::new(recovered::Recovered::error(recovery_placeholder())),
        })
    }

    /// A merged-head `NA? SE? (JOI / JA / A) NAI?` connective over the given head and NAI slots.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_merged_head_connective(
        head: recovered::WithFreeModifiers<
            recovered::Recovered<crate::tree::Token>,
            Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
        >,
        nai: Option<
            recovered::WithFreeModifiers<
                recovered::Recovered<crate::tree::Token>,
                Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
            >,
        >,
    ) -> recovered::Recovered<recovered::ExpSelbriRelativeClauseConnectiveSyntax> {
        recovered::Recovered::valid(
            recovered::ExpSelbriRelativeClauseConnectiveSyntax::ExpRelativeClauseConnective(
                Arc::new(recovered::Recovered::valid(
                    recovered::ExpRelativeClauseConnectiveSyntax {
                        na: None,
                        se: None,
                        head,
                        nai,
                    },
                )),
            ),
        )
    }

    /// A `SE? BIhI NAI?` interval connective over the given BIhI and NAI slots.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_simple_interval_connective(
        bihi: recovered::WithFreeModifiers<
            recovered::Recovered<crate::tree::Token>,
            Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
        >,
        nai: Option<
            recovered::WithFreeModifiers<
                recovered::Recovered<crate::tree::Token>,
                Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
            >,
        >,
    ) -> recovered::Recovered<recovered::ExpSelbriRelativeClauseConnectiveSyntax> {
        recovered::Recovered::valid(
            recovered::ExpSelbriRelativeClauseConnectiveSyntax::SimpleIntervalConnective(Arc::new(
                recovered::Recovered::valid(recovered::SimpleIntervalConnectiveSyntax {
                    se: None,
                    bihi,
                    nai,
                }),
            )),
        )
    }

    /// The prohibited placement is a rejection, so every fact it stands on has to be a node the
    /// recovery tree proved. A placeholder head, a placeholder NAI, or a free-modifier slot
    /// holding nothing but placeholders is not that proof, and withdrawing a surface on one
    /// would be exactly the fail-open direction this classifier must not have.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_prohibited_placement_needs_every_node_proven() {
        let rejection = ProhibitedRelativeConnectiveFreeModifierRejection;
        let placeholder = recovery_placeholder();
        let free_modifier = recovered_free_modifier;
        let unparsed_free_modifier = recovered_unparsed_free_modifier;
        let je = || recovered_token("je");
        let nai = || recovered_token("nai");
        let bihi = || recovered_token("bi'i");
        let unparsed_token = recovered_unparsed_token;

        assert!(
            rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(je(), vec![free_modifier()]),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "a proven head, a proven NAI and a proven free modifier between them is the placement"
        );
        assert!(
            rejection.rejects(&recovered_selbri_continuation(
                recovered_simple_interval_connective(
                    recovered_head_slot(bihi(), vec![free_modifier()]),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "the interval shape carries the same slot on its BIhI"
        );

        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(unparsed_token(), vec![free_modifier()]),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "a head that did not parse does not prove there is a connective head to place before"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_simple_interval_connective(
                    recovered_head_slot(unparsed_token(), vec![free_modifier()]),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "nor does an unparsed BIhI"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(je(), vec![free_modifier()]),
                    Some(recovered_head_slot(unparsed_token(), Vec::new())),
                )
            )),
            "a NAI slot that did not parse does not prove the free modifier stands before a NAI"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(je(), vec![unparsed_free_modifier()]),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "a free-modifier slot holding only placeholders proves no free modifier at all"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(je(), Vec::new()),
                    Some(recovered_head_slot(nai(), Vec::new())),
                )
            )),
            "an empty free-modifier slot is the connective camxes-exp does spell"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(
                recovered_merged_head_connective(
                    recovered_head_slot(je(), vec![free_modifier()]),
                    None,
                )
            )),
            "with no NAI the trailing frees are the chain's own free* and the placement is legal"
        );
        assert!(
            !rejection.rejects(&recovered_selbri_continuation(recovered::Recovered::error(
                recovery_placeholder()
            ))),
            "a connective that did not parse is not a completed candidate"
        );
        assert!(
            !rejection.rejects(&recovered::Recovered::<
                recovered::ExpSelbriRelativeClauseContinuationSyntax,
            >::error(placeholder)),
            "an unparsed continuation is not a completed candidate either"
        );
    }

    /// The placement result is three-way because the recovered tree has a third state, and every
    /// answer other than `Unproven` comes from proven nodes.
    /// The probe for stranded relative lists requires `Permitted` before it hands a continuation to the chain.
    /// That result requires positive proof.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_placement_is_permitted_only_when_the_tree_proved_it() {
        let je = || recovered_token("je");
        let nai = || recovered_head_slot(recovered_token("nai"), Vec::new());
        let unparsed_nai = || recovered_head_slot(recovered_unparsed_token(), Vec::new());

        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), vec![recovered_free_modifier()]),
                Some(&nai()),
            ),
            ConnectiveFreeModifierPlacement::Prohibited,
            "a proven head, a proven NAI and a proven free modifier between them is the placement"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), vec![recovered_free_modifier()]),
                None,
            ),
            ConnectiveFreeModifierPlacement::Permitted,
            "with no NAI slot at all the trailing frees are the chain's own free*"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), Vec::new()),
                Some(&nai()),
            ),
            ConnectiveFreeModifierPlacement::Permitted,
            "a proven NAI over an empty free-modifier slot is the connective camxes-exp spells"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), vec![recovered_unparsed_free_modifier()]),
                Some(&nai()),
            ),
            ConnectiveFreeModifierPlacement::Unproven,
            "a free-modifier slot holding only placeholders proves neither answer"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), vec![recovered_free_modifier()]),
                Some(&unparsed_nai()),
            ),
            ConnectiveFreeModifierPlacement::Unproven,
            "a NAI slot that did not parse proves neither a NAI present nor a NAI absent"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(je(), Vec::new()),
                Some(&unparsed_nai()),
            ),
            ConnectiveFreeModifierPlacement::Unproven,
            "an unparsed NAI may have consumed the frees that would stand in front of it"
        );
        assert_eq!(
            recovered_connective_free_modifier_placement(
                &recovered_head_slot(recovered_unparsed_token(), Vec::new()),
                None,
            ),
            ConnectiveFreeModifierPlacement::Unproven,
            "a head that did not parse stands for input that could hold either shape"
        );
    }
}
