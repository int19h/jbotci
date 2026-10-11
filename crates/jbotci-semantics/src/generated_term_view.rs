//! Shared borrowed views over leaf-listed generated term hierarchy levels.
//!
//! Each hierarchy enum carries the leaf variants itself: the grammar splices the level below into
//! every level, so Debug and serde output show no wrapper level. `GeneratedSimpleTermRef` gives
//! reference analysis one strongly typed leaf surface over all of them without allocating or
//! cloning the generated nodes. A `None` conversion identifies a connection node whose grouping
//! the caller must handle explicitly rather than flatten as a leaf.
//!
//! Every view here is `Copy` and borrowed, so no path ever converts a term by copying it into
//! another level's enum.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use std::sync::Arc;

use jbotci_syntax::generated_model::{
    BareNaTermSyntax, BoGroupedBridiTailSyntax, BoGroupedBridiTailWithoutTailTermsSyntax,
    BoundTermContinuationSyntax, BoundTermSyntax, BridiTailBoJointSyntax,
    BridiTailBoJointWithoutTailTermsSyntax, CeheTermSyntax, ElidedNaheFihoTagTermSyntax,
    ExpSoiAdverbialTermSyntax, ExpTailTermsPrefixSyntax, FaChainTaggedSumtiTermSyntax,
    FihoiProposalAdverbialTermSyntax, ForethoughtTermsetSyntax, GekTermsetSyntax,
    LeadingTermTagTenseModalSyntax, LinkedTermSyntax, LooseTermSyntax, NaKuTermSyntax,
    NonabsFaChainTaggedSumtiTermSyntax, NonabsTaggedSumtiTermSyntax, NonabsTermSyntax,
    NormalTermSyntax, NuhiTermsetSyntax, PlaceTaggedLinkedSumtiSyntax, PlaceTaggedSumtiTermSyntax,
    PlainLinkedSumtiSyntax, SelbriSimpleBridiTailSyntax,
    SelbriSimpleBridiTailWithoutTailTermsSyntax, SimpleBridiTailSyntax,
    SimpleBridiTailWithoutTailTermsSyntax, SimpleTermSyntax, SumtiBoundSyntax,
    SumtiBoundTailSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax,
    TaggedSumtiBeforeTagTermSyntax, TaggedSumtiTermSyntax, TenseModalSyntax,
    TenseTaggedLinkedSumtiSyntax, TermSyntax,
};

/// A borrowed tag-led term leaf.
///
/// The absorption-guarded `TaggedSumtiTermSyntax` and its unguarded `nonabs` twin differ only by
/// the `!selbri` assertion that decides where the term ends. That is a parse-time boundary rule
/// with no semantic content, so reference analysis sees exactly one shape for both.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedTaggedTermRef<'syntax> {
    pub(crate) tense_modal: &'syntax Arc<LeadingTermTagTenseModalSyntax>,
    pub(crate) sumti: &'syntax Arc<TaggedOrElidedSumtiSyntax>,
}

impl<'syntax> GeneratedTaggedTermRef<'syntax> {
    /// Borrow the absorption-guarded tag term.
    #[requires(true)]
    #[ensures(true)]
    fn from_guarded(term: &'syntax TaggedSumtiTermSyntax) -> Self {
        Self {
            tense_modal: &term.tense_modal,
            sumti: &term.sumti,
        }
    }

    /// Borrow the unguarded `nonabs` tag term.
    #[requires(true)]
    #[ensures(true)]
    fn from_unguarded(term: &'syntax NonabsTaggedSumtiTermSyntax) -> Self {
        Self {
            tense_modal: &term.tense_modal,
            sumti: &term.sumti,
        }
    }
}

/// A borrowed JOIK-chained FA tag term leaf.
///
/// The guarded and unguarded twins differ only by the `!selbri` assertion that decides where
/// the term ends, so reference analysis sees one shape for both. A chain names several places
/// at once, which no lowering reads yet, so the view exposes only the payload.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedFaChainTermRef<'syntax> {
    pub(crate) sumti: &'syntax Arc<TaggedOrElidedSumtiSyntax>,
}

impl<'syntax> GeneratedFaChainTermRef<'syntax> {
    /// Borrow the absorption-guarded FA chain term.
    #[requires(true)]
    #[ensures(true)]
    fn from_guarded(term: &'syntax FaChainTaggedSumtiTermSyntax) -> Self {
        Self { sumti: &term.sumti }
    }

    /// Borrow the unguarded `nonabs` FA chain term.
    #[requires(true)]
    #[ensures(true)]
    fn from_unguarded(term: &'syntax NonabsFaChainTaggedSumtiTermSyntax) -> Self {
        Self { sumti: &term.sumti }
    }
}

/// A borrowed BO-bound sumti tail.
///
/// The view exposes the optional tag and the trailing operand, for the reference passes.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBoundSumtiTailRef<'syntax> {
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) trailing_sumti: &'syntax Arc<SumtiBoundSyntax>,
}

impl<'syntax> GeneratedBoundSumtiTailRef<'syntax> {
    /// Borrow the BO-bound tail.
    #[requires(true)]
    #[ensures(ret.tense_modal.is_some() == tail.tense_modal.is_some())]
    pub(crate) fn from_tail(tail: &'syntax SumtiBoundTailSyntax) -> Self {
        Self {
            tense_modal: tail.tense_modal.as_deref(),
            trailing_sumti: &tail.trailing_sumti,
        }
    }
}

/// A borrowed BO-level bridi-tail joint.
///
/// The view exposes the tag, the operand, and the trailing terms for the reference passes.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBridiTailBoJointRef<'syntax> {
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) bridi_tail: &'syntax Arc<BoGroupedBridiTailSyntax>,
    pub(crate) tail_terms: &'syntax [Arc<TermSyntax>],
}

impl<'syntax> GeneratedBridiTailBoJointRef<'syntax> {
    /// Borrow the BO joint.
    #[requires(true)]
    #[ensures(ret.tense_modal.is_some() == joint.tense_modal.is_some())]
    pub(crate) fn from_joint(joint: &'syntax BridiTailBoJointSyntax) -> Self {
        Self {
            tense_modal: joint.tense_modal.as_deref(),
            bridi_tail: &joint.bridi_tail,
            tail_terms: &joint.tail_terms,
        }
    }
}

/// The tail-terms-free twin of [`GeneratedBridiTailBoJointRef`].
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBridiTailBoJointWithoutTailTermsRef<'syntax> {
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) bridi_tail: &'syntax Arc<BoGroupedBridiTailWithoutTailTermsSyntax>,
}

impl<'syntax> GeneratedBridiTailBoJointWithoutTailTermsRef<'syntax> {
    /// Borrow the BO joint.
    #[requires(true)]
    #[ensures(ret.tense_modal.is_some() == joint.tense_modal.is_some())]
    pub(crate) fn from_joint(joint: &'syntax BridiTailBoJointWithoutTailTermsSyntax) -> Self {
        Self {
            tense_modal: joint.tense_modal.as_deref(),
            bridi_tail: &joint.bridi_tail,
        }
    }
}

/// A borrowed selbri-led `bridi_tail_3`, sourced or camxes-exp-prefixed.
///
/// camxes-exp writes the level as `(terms CU_elidible?)* selbri tail_terms` (camxes-exp.peg:108).
/// The groups before the selbri hold the same bridi terms the sourced level puts after it, and
/// they fill places in the same order, so every pass that walks the level takes this one shape
/// and reads the leading run first.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedSelbriBridiTailRef<'syntax> {
    pub(crate) prefixes: &'syntax [Arc<ExpTailTermsPrefixSyntax>],
    pub(crate) tail: &'syntax SelbriSimpleBridiTailSyntax,
}

impl<'syntax> GeneratedSelbriBridiTailRef<'syntax> {
    /// Borrow either selbri-led shape, or `None` for the forethought alternative.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(tail, SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(_)))]
    pub(crate) fn from_simple(tail: &'syntax SimpleBridiTailSyntax) -> Option<Self> {
        match tail {
            SimpleBridiTailSyntax::SelbriSimpleBridiTail(tail) => Some(Self {
                prefixes: &[],
                tail,
            }),
            SimpleBridiTailSyntax::ExpPrefixedSimpleBridiTail(prefixed) => Some(Self {
                prefixes: &prefixed.prefixes,
                tail: &prefixed.tail,
            }),
            SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(_) => None,
        }
    }

    /// The prefix groups' terms, in source order.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn prefix_terms(&self) -> impl Iterator<Item = &'syntax TermSyntax> + use<'syntax> {
        self.prefixes
            .iter()
            .flat_map(|prefix| prefix.terms.iter().map(Arc::as_ref))
    }
}

/// The tail-terms-free twin of [`GeneratedSelbriBridiTailRef`].
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedSelbriBridiTailWithoutTailTermsRef<'syntax> {
    pub(crate) prefixes: &'syntax [Arc<ExpTailTermsPrefixSyntax>],
    pub(crate) tail: &'syntax SelbriSimpleBridiTailWithoutTailTermsSyntax,
}

impl<'syntax> GeneratedSelbriBridiTailWithoutTailTermsRef<'syntax> {
    /// Borrow either selbri-led shape, or `None` for the forethought alternative.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(tail, SimpleBridiTailWithoutTailTermsSyntax::ForethoughtSimpleBridiTailWithoutTailTerms(_)))]
    pub(crate) fn from_simple(
        tail: &'syntax SimpleBridiTailWithoutTailTermsSyntax,
    ) -> Option<Self> {
        match tail {
            SimpleBridiTailWithoutTailTermsSyntax::SelbriSimpleBridiTailWithoutTailTerms(tail) => {
                Some(Self {
                    prefixes: &[],
                    tail,
                })
            }
            SimpleBridiTailWithoutTailTermsSyntax::ExpPrefixedSimpleBridiTailWithoutTailTerms(
                prefixed,
            ) => Some(Self {
                prefixes: &prefixed.prefixes,
                tail: &prefixed.tail,
            }),
            SimpleBridiTailWithoutTailTermsSyntax::ForethoughtSimpleBridiTailWithoutTailTerms(
                _,
            ) => None,
        }
    }

    /// The prefix groups' terms, in source order.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn prefix_terms(&self) -> impl Iterator<Item = &'syntax TermSyntax> + use<'syntax> {
        self.prefixes
            .iter()
            .flat_map(|prefix| prefix.terms.iter().map(Arc::as_ref))
    }
}

/// Borrow the operand of an absorption-safe BO term continuation, sourced or connectorless.
#[requires(true)]
#[ensures(true)]
pub(crate) fn bound_term_continuation_operand(
    continuation: &BoundTermContinuationSyntax,
) -> &Arc<SimpleTermSyntax> {
    &continuation.trailing_term
}

/// A borrowed simple-term leaf shared by every level of the composed term hierarchy.
#[invariant(::PlaceTaggedSumtiTerm(_) => true)]
#[invariant(::FaChainTaggedSumtiTerm(_) => true)]
#[invariant(::ElidedNaheFihoTagTerm(_) => true)]
#[invariant(::TaggedSumtiBeforeTagTerm(_) => true)]
#[invariant(::TaggedSumtiTerm(_) => true)]
#[invariant(::FihoiProposalAdverbialTerm(_) => true)]
#[invariant(::ExpSoiAdverbialTerm(_) => true)]
#[invariant(::NaKuTerm(_) => true)]
#[invariant(::SumtiTerm(_) => true)]
#[invariant(::BareNaTerm(_) => true)]
#[invariant(::GekTermset(_) => true)]
#[invariant(::ForethoughtTermset(_) => true)]
#[invariant(::NuhiTermset(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedSimpleTermRef<'syntax> {
    PlaceTaggedSumtiTerm(&'syntax PlaceTaggedSumtiTermSyntax),
    FaChainTaggedSumtiTerm(GeneratedFaChainTermRef<'syntax>),
    ElidedNaheFihoTagTerm(&'syntax ElidedNaheFihoTagTermSyntax),
    TaggedSumtiBeforeTagTerm(&'syntax TaggedSumtiBeforeTagTermSyntax),
    TaggedSumtiTerm(GeneratedTaggedTermRef<'syntax>),
    FihoiProposalAdverbialTerm(&'syntax FihoiProposalAdverbialTermSyntax),
    ExpSoiAdverbialTerm(&'syntax ExpSoiAdverbialTermSyntax),
    NaKuTerm(&'syntax NaKuTermSyntax),
    SumtiTerm(&'syntax SumtiTermSyntax),
    BareNaTerm(&'syntax BareNaTermSyntax),
    GekTermset(&'syntax GekTermsetSyntax),
    ForethoughtTermset(&'syntax ForethoughtTermsetSyntax),
    NuhiTermset(&'syntax NuhiTermsetSyntax),
}

impl<'syntax> GeneratedSimpleTermRef<'syntax> {
    /// Borrow a leaf from the original flat simple-term sum.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_simple(term: &'syntax SimpleTermSyntax) -> Self {
        match term {
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => Self::PlaceTaggedSumtiTerm(term),
            SimpleTermSyntax::FaChainTaggedSumtiTerm(term) => {
                Self::FaChainTaggedSumtiTerm(GeneratedFaChainTermRef::from_guarded(term))
            }
            SimpleTermSyntax::ElidedNaheFihoTagTerm(term) => Self::ElidedNaheFihoTagTerm(term),
            SimpleTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Self::TaggedSumtiBeforeTagTerm(term)
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                Self::TaggedSumtiTerm(GeneratedTaggedTermRef::from_guarded(term))
            }
            SimpleTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Self::FihoiProposalAdverbialTerm(term)
            }
            SimpleTermSyntax::ExpSoiAdverbialTerm(term) => Self::ExpSoiAdverbialTerm(term),
            SimpleTermSyntax::NaKuTerm(term) => Self::NaKuTerm(term),
            SimpleTermSyntax::SumtiTerm(term) => Self::SumtiTerm(term),
            SimpleTermSyntax::BareNaTerm(term) => Self::BareNaTerm(term),
            SimpleTermSyntax::GekTermset(term) => Self::GekTermset(term),
            SimpleTermSyntax::ForethoughtTermset(term) => Self::ForethoughtTermset(term),
            SimpleTermSyntax::NuhiTermset(term) => Self::NuhiTermset(term),
        }
    }

    /// Borrow a leaf from the BO-bound level, or report that the node is a grouped connection.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(term, BoundTermSyntax::StagBoundTermConnection(_)))]
    pub(crate) fn from_bound(term: &'syntax BoundTermSyntax) -> Option<Self> {
        match term {
            BoundTermSyntax::StagBoundTermConnection(_) => None,
            BoundTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            BoundTermSyntax::FaChainTaggedSumtiTerm(term) => Some(Self::FaChainTaggedSumtiTerm(
                GeneratedFaChainTermRef::from_guarded(term),
            )),
            BoundTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            BoundTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            BoundTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            BoundTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            BoundTermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            BoundTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            BoundTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            BoundTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            BoundTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            BoundTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            BoundTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }

    /// Borrow a leaf from the PEhE level, or report that the node is a grouped connection.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_term(term: &'syntax TermSyntax) -> Option<Self> {
        match term {
            TermSyntax::PeheTermsetConnection(_)
            | TermSyntax::TermsetGroup(_)
            | TermSyntax::ConnectedTerm(_)
            | TermSyntax::StagBoundTermConnection(_) => None,
            TermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            TermSyntax::FaChainTaggedSumtiTerm(term) => Some(Self::FaChainTaggedSumtiTerm(
                GeneratedFaChainTermRef::from_guarded(term),
            )),
            TermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            TermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            TermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            TermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            TermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            TermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            TermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            TermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            TermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            TermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            TermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }

    /// Borrow a leaf from the CEhE level, or report that the node is a grouped connection.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_cehe(term: &'syntax CeheTermSyntax) -> Option<Self> {
        match term {
            CeheTermSyntax::TermsetGroup(_)
            | CeheTermSyntax::ConnectedTerm(_)
            | CeheTermSyntax::StagBoundTermConnection(_) => None,
            CeheTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            CeheTermSyntax::FaChainTaggedSumtiTerm(term) => Some(Self::FaChainTaggedSumtiTerm(
                GeneratedFaChainTermRef::from_guarded(term),
            )),
            CeheTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            CeheTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            CeheTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            CeheTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            CeheTermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            CeheTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            CeheTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            CeheTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            CeheTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            CeheTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            CeheTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }

    /// Borrow a leaf from the loose connective level, or report a grouped connection.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_loose(term: &'syntax LooseTermSyntax) -> Option<Self> {
        match term {
            LooseTermSyntax::ConnectedTerm(_) | LooseTermSyntax::StagBoundTermConnection(_) => None,
            LooseTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            LooseTermSyntax::FaChainTaggedSumtiTerm(term) => Some(Self::FaChainTaggedSumtiTerm(
                GeneratedFaChainTermRef::from_guarded(term),
            )),
            LooseTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            LooseTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            LooseTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            LooseTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            LooseTermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            LooseTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            LooseTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            LooseTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            LooseTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            LooseTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            LooseTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }

    /// Borrow a leaf from the unguarded `nonabs` level, or report a grouped connection.
    ///
    /// The unguarded tag leaf is analyzed exactly like its absorption-guarded twin.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_nonabs(term: &'syntax NonabsTermSyntax) -> Option<Self> {
        match term {
            NonabsTermSyntax::ConnectedTerm(_) | NonabsTermSyntax::StagBoundTermConnection(_) => {
                None
            }
            NonabsTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            NonabsTermSyntax::NonabsFaChainTaggedSumtiTerm(term) => Some(
                Self::FaChainTaggedSumtiTerm(GeneratedFaChainTermRef::from_unguarded(term)),
            ),
            NonabsTermSyntax::ElidedNaheFihoTagTerm(term) => {
                Some(Self::ElidedNaheFihoTagTerm(term))
            }
            NonabsTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            NonabsTermSyntax::NonabsTaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_unguarded(term),
            )),
            NonabsTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            NonabsTermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            NonabsTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            NonabsTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            NonabsTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            NonabsTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            NonabsTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            NonabsTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }

    /// Borrow a leaf from the normal-flavour loose level, or report a grouped connection.
    ///
    /// The normal flavour is a second ladder over the same leaf inventory, so the leaves it
    /// yields are exactly the ones the `nonabs` ladder yields; only the connective tiers above
    /// them differ.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(term, NormalTermSyntax::ConnectedNormalTerm(_) | NormalTermSyntax::BoundNormalTermConnection(_)))]
    pub(crate) fn from_normal(term: &'syntax NormalTermSyntax) -> Option<Self> {
        match term {
            NormalTermSyntax::ConnectedNormalTerm(_)
            | NormalTermSyntax::BoundNormalTermConnection(_) => None,
            NormalTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            NormalTermSyntax::NonabsFaChainTaggedSumtiTerm(term) => Some(
                Self::FaChainTaggedSumtiTerm(GeneratedFaChainTermRef::from_unguarded(term)),
            ),
            NormalTermSyntax::ElidedNaheFihoTagTerm(term) => {
                Some(Self::ElidedNaheFihoTagTerm(term))
            }
            NormalTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            NormalTermSyntax::NonabsTaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_unguarded(term),
            )),
            NormalTermSyntax::FihoiProposalAdverbialTerm(term) => {
                Some(Self::FihoiProposalAdverbialTerm(term))
            }
            NormalTermSyntax::ExpSoiAdverbialTerm(term) => Some(Self::ExpSoiAdverbialTerm(term)),
            NormalTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            NormalTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            NormalTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            NormalTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            NormalTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            NormalTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
        }
    }
}

/// A borrowed sumti-association payload: the shapes a GOI-family relative phrase can carry.
///
/// The payload constituent is the shared normal-flavour term, because that is what both
/// sources spell at `relative_clause_1` (camxes.peg:168, camxes-exp.peg:207). A
/// sumti-association phrase relates its head to a SUMTI, so only the
/// leaves that carry one have a reading, plus `NA KU`, which deliberately carries none and
/// negates the phrase instead. Every other leaf of the shared inventory — a termset, an
/// adverbial, a bare NA, or a term connection — reaches this projection as `None` and is
/// reported rather than guessed at.
#[invariant(::Plain(_) => true)]
#[invariant(::Tagged(_) => true)]
#[invariant(::PlaceTagged(_) => true)]
#[invariant(::NaKu => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedAssociationPayloadRef<'syntax> {
    Plain(&'syntax SumtiTermSyntax),
    Tagged(GeneratedTaggedTermRef<'syntax>),
    PlaceTagged(&'syntax PlaceTaggedSumtiTermSyntax),
    NaKu,
}

impl<'syntax> GeneratedAssociationPayloadRef<'syntax> {
    /// Project a payload term onto the association shapes, or report that it has no reading.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_payload(term: &'syntax NormalTermSyntax) -> Option<Self> {
        match GeneratedSimpleTermRef::from_normal(term)? {
            GeneratedSimpleTermRef::SumtiTerm(term) => Some(Self::Plain(term)),
            GeneratedSimpleTermRef::TaggedSumtiTerm(term) => Some(Self::Tagged(term)),
            GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTagged(term)),
            GeneratedSimpleTermRef::NaKuTerm(_) => Some(Self::NaKu),
            GeneratedSimpleTermRef::FaChainTaggedSumtiTerm(_)
            | GeneratedSimpleTermRef::ElidedNaheFihoTagTerm(_)
            | GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(_)
            | GeneratedSimpleTermRef::FihoiProposalAdverbialTerm(_)
            | GeneratedSimpleTermRef::ExpSoiAdverbialTerm(_)
            | GeneratedSimpleTermRef::BareNaTerm(_)
            | GeneratedSimpleTermRef::GekTermset(_)
            | GeneratedSimpleTermRef::ForethoughtTermset(_)
            | GeneratedSimpleTermRef::NuhiTermset(_) => None,
        }
    }

    /// The payload sumti of a tag-led association, which may be an elided KU.
    ///
    /// Both tag-led shapes hold the same `tagged_or_elided_sumti` node: camxes-standard gives FA
    /// its own `term` alternative (camxes.peg:128) while camxes-exp folds FA into `tense_modal`
    /// and reaches the same surface through `tag_term` (camxes-exp.peg:149), so the two differ
    /// only in which token introduces the payload.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Tagged(_) | Self::PlaceTagged(_)))]
    pub(crate) fn tagged_sumti(self) -> Option<&'syntax Arc<TaggedOrElidedSumtiSyntax>> {
        match self {
            Self::Tagged(term) => Some(term.sumti),
            Self::PlaceTagged(term) => Some(&term.sumti),
            Self::Plain(_) | Self::NaKu => None,
        }
    }
}

/// A borrowed linked-sumti leaf shared by the flat and hierarchical link enums.
#[invariant(::PlaceTagged(_) => true)]
#[invariant(::TenseTagged(_) => true)]
#[invariant(::Plain(_) => true)]
#[invariant(::FullTerm(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedLinkedSumtiRef<'syntax> {
    PlaceTagged(&'syntax PlaceTaggedLinkedSumtiSyntax),
    TenseTagged(&'syntax TenseTaggedLinkedSumtiSyntax),
    Plain(&'syntax PlainLinkedSumtiSyntax),
    FullTerm(&'syntax NormalTermSyntax),
}

impl<'syntax> GeneratedLinkedSumtiRef<'syntax> {
    /// Borrow a leaf from the loose link level, or report a grouped link connection.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(link, LinkedTermSyntax::ConnectedLinkedTerm(_) | LinkedTermSyntax::BoundLinkedTermConnection(_)))]
    pub(crate) fn from_linked_term(link: &'syntax LinkedTermSyntax) -> Option<Self> {
        match link {
            LinkedTermSyntax::ConnectedLinkedTerm(_)
            | LinkedTermSyntax::BoundLinkedTermConnection(_) => None,
            LinkedTermSyntax::PlaceTaggedLinkedSumti(link) => Some(Self::PlaceTagged(link)),
            LinkedTermSyntax::TenseTaggedLinkedSumti(link) => Some(Self::TenseTagged(link)),
            LinkedTermSyntax::PlainLinkedSumti(link) => Some(Self::Plain(link)),
            LinkedTermSyntax::FullLinkedTerm(link) => Some(Self::FullTerm(&link.0)),
        }
    }
}
