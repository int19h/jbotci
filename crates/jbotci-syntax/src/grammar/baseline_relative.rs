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

use super::generated_model::view::{self, AsView, Sequence, Slot, WithFreeModifiersView};
use super::generated_runtime::OutputRejection;

/// Return a bare ZIhE continuation to its baseline owner.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineRelativeContinuationRejection;

#[requires(true)]
#[ensures(true)]
fn is_baseline_relative_continuation(value: view::ExpRelativeContinuationSyntaxRef<'_>) -> bool {
    let view::ExpRelativeContinuationSyntaxView {
        connective,
        inner: _,
    } = value.get();
    let Some(connective) = connective.valid() else {
        return false;
    };
    let view::ExpRelativeClauseConnectiveSyntaxView { na, se, head, nai } = connective.get();
    na.is_none()
        && se.is_none()
        && head
            .value
            .valid()
            .is_some_and(|head| head.cmavo() == Some(Cmavo::Zihe))
        && nai.is_none()
}

#[contract_trait]
impl<T> OutputRejection<T> for BaselineRelativeContinuationRejection
where
    for<'tree> T: AsView<'tree, Ref = view::ExpRelativeContinuationSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "baseline ZIhE relative continuation"
    }
    fn rejects(&self, value: &T) -> bool {
        value
            .as_view()
            .valid()
            .is_some_and(is_baseline_relative_continuation)
    }
}

/// Refuse a proven free modifier before a connective's NAI.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProhibitedRelativeConnectiveFreeModifierRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectiveFreeModifierPlacement {
    Permitted,
    Prohibited,
    Unproven,
}

/// Presence alone is not evidence when a word or modifier slot is invalid.
#[requires(true)]
#[ensures(true)]
fn connective_free_modifier_placement<'tree, V: Copy, S, R>(
    head: WithFreeModifiersView<
        Slot<V>,
        Sequence<'tree, S, R, Slot<view::FreeModifierSyntaxRef<'tree>>>,
    >,
    nai: Option<
        WithFreeModifiersView<
            Slot<V>,
            Sequence<'tree, S, R, Slot<view::FreeModifierSyntaxRef<'tree>>>,
        >,
    >,
) -> ConnectiveFreeModifierPlacement {
    let WithFreeModifiersView {
        value: head,
        free_modifiers,
    } = head;
    if head.valid().is_none() {
        return ConnectiveFreeModifierPlacement::Unproven;
    }
    let Some(WithFreeModifiersView {
        value: nai,
        free_modifiers: _,
    }) = nai
    else {
        return ConnectiveFreeModifierPlacement::Permitted;
    };
    if nai.valid().is_none() {
        return ConnectiveFreeModifierPlacement::Unproven;
    }
    if free_modifiers
        .iter()
        .any(|modifier| modifier.valid().is_some())
    {
        ConnectiveFreeModifierPlacement::Prohibited
    } else if free_modifiers.is_empty() {
        ConnectiveFreeModifierPlacement::Permitted
    } else {
        ConnectiveFreeModifierPlacement::Unproven
    }
}

#[requires(true)]
#[ensures(true)]
fn is_prohibited_connective_free_modifier(
    value: view::ExpSelbriRelativeClauseContinuationSyntaxRef<'_>,
) -> bool {
    let view::ExpSelbriRelativeClauseContinuationSyntaxView {
        connective,
        inner: _,
    } = value.get();
    let Some(connective) = connective.valid() else {
        return false;
    };
    match connective.get() {
        view::ExpSelbriRelativeClauseConnectiveSyntaxView::ZiheSelbriRelativeConnective(_)
        | view::ExpSelbriRelativeClauseConnectiveSyntaxView::ClosedIntervalConnective(_) => false,
        view::ExpSelbriRelativeClauseConnectiveSyntaxView::ExpRelativeClauseConnective(
            connective,
        ) => connective.valid().is_some_and(|connective| {
            let view::ExpRelativeClauseConnectiveSyntaxView {
                na: _,
                se: _,
                head,
                nai,
            } = connective.get();
            connective_free_modifier_placement(head, nai)
                == ConnectiveFreeModifierPlacement::Prohibited
        }),
        view::ExpSelbriRelativeClauseConnectiveSyntaxView::SimpleIntervalConnective(connective) => {
            connective.valid().is_some_and(|connective| {
                let view::SimpleIntervalConnectiveSyntaxView { se: _, bihi, nai } =
                    connective.get();
                connective_free_modifier_placement(bihi, nai)
                    == ConnectiveFreeModifierPlacement::Prohibited
            })
        }
    }
}

#[contract_trait]
impl<T> OutputRejection<T> for ProhibitedRelativeConnectiveFreeModifierRejection
where
    for<'tree> T: AsView<'tree, Ref = view::ExpSelbriRelativeClauseContinuationSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "free modifier before the connective's NAI"
    }
    fn rejects(&self, value: &T) -> bool {
        value
            .as_view()
            .valid()
            .is_some_and(is_prohibited_connective_free_modifier)
    }
}

/// Return an elided SOI adverbial to a reciprocal with the same first sumti.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineReciprocalSoiRejection;

#[requires(true)]
#[ensures(true)]
fn subsentence_opens_with_leading_sumti(value: view::SubbridiSyntaxRef<'_>) -> bool {
    match value.get() {
        view::SubbridiSyntaxView::PrenexSubbridi(_) => false,
        view::SubbridiSyntaxView::BridiSubbridi(bridi) => bridi.valid().is_some_and(|bridi| {
            let view::BridiSubbridiSyntaxView(bridi) = bridi.get();
            bridi.valid().is_some_and(|bridi| match bridi.get() {
                view::BridiSyntaxView::BridiWithLeadingTerms(bridi) => {
                    bridi.valid().is_some_and(|bridi| {
                        let view::BridiWithLeadingTermsSyntaxView {
                            leading_terms,
                            cu: _,
                            bridi_tail: _,
                        } = bridi.get();
                        leading_terms
                            .get(0)
                            .and_then(Slot::valid)
                            .is_some_and(|term| match term.get() {
                                view::TermSyntaxView::SumtiTerm(_) => true,
                                view::TermSyntaxView::PeheTermsetConnection(_)
                                | view::TermSyntaxView::TermsetGroup(_)
                                | view::TermSyntaxView::ConnectedTerm(_)
                                | view::TermSyntaxView::StagBoundTermConnection(_)
                                | view::TermSyntaxView::FaChainTaggedSumtiTerm(_)
                                | view::TermSyntaxView::PlaceTaggedSumtiTerm(_)
                                | view::TermSyntaxView::ElidedNaheFihoTagTerm(_)
                                | view::TermSyntaxView::TaggedSumtiBeforeTagTerm(_)
                                | view::TermSyntaxView::TaggedSumtiTerm(_)
                                | view::TermSyntaxView::FihoiProposalAdverbialTerm(_)
                                | view::TermSyntaxView::ExpSoiAdverbialTerm(_)
                                | view::TermSyntaxView::NaKuTerm(_)
                                | view::TermSyntaxView::BareNaTerm(_)
                                | view::TermSyntaxView::GekTermset(_)
                                | view::TermSyntaxView::ForethoughtTermset(_)
                                | view::TermSyntaxView::NuhiTermset(_) => false,
                            })
                    })
                }
                view::BridiSyntaxView::BareCuBridi(_)
                | view::BridiSyntaxView::RelationOnlyBridi(_) => false,
            })
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_soi(value: view::ExpSoiSubsentenceAdverbialSyntaxRef<'_>) -> bool {
    let view::ExpSoiSubsentenceAdverbialSyntaxView {
        soi,
        subsentence,
        sehu,
    } = value.get();
    soi.value
        .valid()
        .is_some_and(|soi| soi.cmavo() == Some(Cmavo::Soi))
        && sehu.is_none()
        && subsentence
            .valid()
            .is_some_and(subsentence_opens_with_leading_sumti)
}

#[contract_trait]
impl<T> OutputRejection<T> for BaselineReciprocalSoiRejection
where
    for<'tree> T: AsView<'tree, Ref = view::ExpSoiSubsentenceAdverbialSyntaxRef<'tree>>,
{
    fn rejected_name(&self) -> &'static str {
        "baseline SOI reciprocal"
    }
    fn rejects(&self, value: &T) -> bool {
        value.as_view().valid().is_some_and(is_baseline_soi)
    }
}

#[cfg(test)]
use super::generated_model::recovered;

/// Adapt the existing low-level test inputs to the generated view's slot rules.
#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn recovered_connective_free_modifier_placement<T, F>(
    head: &recovered::WithFreeModifiers<recovered::Recovered<T>, F>,
    nai: Option<&recovered::WithFreeModifiers<recovered::Recovered<T>, F>>,
) -> ConnectiveFreeModifierPlacement
where
    F: std::borrow::Borrow<recovered::Recovered<recovered::FreeModifierSyntax>>,
{
    #[requires(true)]
    #[ensures(true)]
    fn project<'tree, T, F>(
        word: &'tree recovered::WithFreeModifiers<recovered::Recovered<T>, F>,
    ) -> WithFreeModifiersView<
        Slot<&'tree T>,
        Sequence<'tree, (), F, Slot<view::FreeModifierSyntaxRef<'tree>>>,
    >
    where
        F: std::borrow::Borrow<recovered::Recovered<recovered::FreeModifierSyntax>>,
    {
        WithFreeModifiersView {
            value: view::slot(&word.value, |value| value),
            free_modifiers: Sequence::Recovered {
                values: &word.free_modifiers,
                project: |value| view::slot(value.borrow(), view::FreeModifierSyntaxRef::Recovered),
            },
        }
    }
    connective_free_modifier_placement(project(head), nai.map(project))
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
