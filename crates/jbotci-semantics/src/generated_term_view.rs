//! Shared borrowed views over leaf-listed generated term hierarchy levels.
//!
//! The syntax grammar deliberately repeats leaf variants in the hierarchy enums so Debug and
//! serde output remain stable. `GeneratedSimpleTermRef` gives reference analysis one strongly
//! typed leaf surface over all of them without allocating or cloning the generated nodes. A
//! `None` conversion identifies a connection node whose grouping the caller must handle
//! explicitly rather than flatten as a leaf.
//!
//! Every view here is `Copy` and borrowed, so no path ever converts a term by copying it into
//! another level's enum.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use std::sync::Arc;

use jbotci_syntax::generated_model::{
    BareNaTermSyntax, BoGroupedBridiTailSyntax, BoGroupedBridiTailWithoutTailTermsSyntax,
    BoundTermContinuationSyntax, BoundTermSyntax, BridiTailBoJointSyntax,
    BridiTailBoJointWithoutTailTermsSyntax, BridiTailConnectiveSyntax, CeheTermSyntax,
    ElidedNaheFihoTagTermSyntax, FihoiAdverbialTermSyntax, ForethoughtTermsetSyntax,
    GekTermsetSyntax, JaiTaggedSumtiTermSyntax, KeTermsetSyntax, LeadingTermTagTenseModalSyntax,
    LinkedTermSyntax, LooseTermSyntax, NaKuTermSyntax, NoihaAdverbialTermSyntax,
    NonabsTaggedSumtiTermSyntax, NonabsTermSyntax, NormalTermSyntax, NuhiTermsetSyntax,
    PlaceTaggedLinkedSumtiSyntax, PlaceTaggedSumtiTermSyntax, PlainLinkedSumtiSyntax,
    SimpleTermSyntax, SoiAdverbialTermSyntax, SumtiBoundSyntax, SumtiBoundTailSyntax,
    SumtiConnectiveSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax,
    TaggedSumtiBeforeTagTermSyntax, TaggedSumtiTermSyntax, TenseModalSyntax,
    TenseTaggedLinkedSumtiSyntax, TermSyntax, ZantufaGekTermsetSyntax,
    ZantufaJoikChainedPlaceTagTermSyntax,
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

/// A borrowed BO-bound sumti tail, sourced or Zantufa-connectorless.
///
/// The two arms of `sumti_bound_tail` differ by exactly one field: the sourced tail carries the
/// connective its sources require, and rolling Zantufa's connectorless tail carries none
/// (zantufa-1.9999.peg:35). Everything a traversal needs — the optional tag and the trailing
/// operand — is shared, so structural passes take this view and only callers that read a
/// connective have to ask for it.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBoundSumtiTailRef<'syntax> {
    pub(crate) connective: Option<&'syntax SumtiConnectiveSyntax>,
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) trailing_sumti: &'syntax Arc<SumtiBoundSyntax>,
}

impl<'syntax> GeneratedBoundSumtiTailRef<'syntax> {
    /// Borrow either BO-bound tail shape.
    #[requires(true)]
    #[ensures(ret.connective.is_some() == matches!(tail, SumtiBoundTailSyntax::BoundSumtiTail(_)))]
    pub(crate) fn from_tail(tail: &'syntax SumtiBoundTailSyntax) -> Self {
        match tail {
            SumtiBoundTailSyntax::BoundSumtiTail(tail) => Self {
                connective: Some(tail.connective.as_ref()),
                tense_modal: tail.tense_modal.as_deref(),
                trailing_sumti: &tail.trailing_sumti,
            },
            SumtiBoundTailSyntax::ZantufaBoundSumtiTail(tail) => Self {
                connective: None,
                tense_modal: tail.tense_modal.as_deref(),
                trailing_sumti: &tail.trailing_sumti,
            },
        }
    }
}

/// A borrowed BO-level bridi-tail joint, sourced or Zantufa-connectorless.
///
/// The arms of `bridi_tail_bo_joint` differ by exactly one field, the same way the BO-bound sumti
/// tail's do: the sourced joint carries the connective its sources require, and rolling Zantufa's
/// connectorless `tag BO` opening carries none (zantufa-1.9999.peg:22). Everything a traversal
/// needs — the tag, the operand and the trailing terms — is shared, so structural passes take
/// this view and only the lowerings that read a connective have to ask for it.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBridiTailBoJointRef<'syntax> {
    pub(crate) connective: Option<&'syntax BridiTailConnectiveSyntax>,
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) bridi_tail: &'syntax Arc<BoGroupedBridiTailSyntax>,
    pub(crate) tail_terms: &'syntax [TermSyntax],
}

impl<'syntax> GeneratedBridiTailBoJointRef<'syntax> {
    /// Borrow either BO-joint shape.
    #[requires(true)]
    #[ensures(ret.connective.is_some() == matches!(joint, BridiTailBoJointSyntax::BridiTailBoContinuation(_)))]
    pub(crate) fn from_joint(joint: &'syntax BridiTailBoJointSyntax) -> Self {
        match joint {
            BridiTailBoJointSyntax::BridiTailBoContinuation(continuation) => Self {
                connective: Some(&continuation.connective),
                tense_modal: continuation.tense_modal.as_deref(),
                bridi_tail: &continuation.bridi_tail,
                tail_terms: &continuation.tail_terms,
            },
            BridiTailBoJointSyntax::ZantufaTagBoBridiTailContinuation(continuation) => Self {
                connective: None,
                tense_modal: Some(&continuation.tense_modal),
                bridi_tail: &continuation.bridi_tail,
                tail_terms: &continuation.tail_terms,
            },
        }
    }
}

/// The tail-terms-free twin of [`GeneratedBridiTailBoJointRef`].
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeneratedBridiTailBoJointWithoutTailTermsRef<'syntax> {
    pub(crate) connective: Option<&'syntax BridiTailConnectiveSyntax>,
    pub(crate) tense_modal: Option<&'syntax TenseModalSyntax>,
    pub(crate) bridi_tail: &'syntax Arc<BoGroupedBridiTailWithoutTailTermsSyntax>,
}

impl<'syntax> GeneratedBridiTailBoJointWithoutTailTermsRef<'syntax> {
    /// Borrow either BO-joint shape.
    #[requires(true)]
    #[ensures(ret.connective.is_some() == matches!(joint, BridiTailBoJointWithoutTailTermsSyntax::BridiTailBoContinuationWithoutTailTerms(_)))]
    pub(crate) fn from_joint(joint: &'syntax BridiTailBoJointWithoutTailTermsSyntax) -> Self {
        match joint {
            BridiTailBoJointWithoutTailTermsSyntax::BridiTailBoContinuationWithoutTailTerms(
                continuation,
            ) => Self {
                connective: Some(&continuation.connective),
                tense_modal: continuation.tense_modal.as_deref(),
                bridi_tail: &continuation.bridi_tail,
            },
            BridiTailBoJointWithoutTailTermsSyntax::ZantufaTagBoBridiTailContinuationWithoutTailTerms(
                continuation,
            ) => Self {
                connective: None,
                tense_modal: Some(&continuation.tense_modal),
                bridi_tail: &continuation.bridi_tail,
            },
        }
    }
}

/// Borrow the operand of an absorption-safe BO term continuation, sourced or connectorless.
#[requires(true)]
#[ensures(true)]
pub(crate) fn bound_term_continuation_operand(
    continuation: &BoundTermContinuationSyntax,
) -> &Arc<SimpleTermSyntax> {
    match continuation {
        BoundTermContinuationSyntax::StagBoundTermContinuation(continuation) => {
            &continuation.trailing_term
        }
        BoundTermContinuationSyntax::ZantufaBoundTermContinuation(continuation) => {
            &continuation.trailing_term
        }
    }
}

/// A borrowed simple-term leaf shared by every level of the composed term hierarchy.
#[invariant(::PlaceTaggedSumtiTerm(_) => true)]
#[invariant(::ZantufaJoikChainedPlaceTagTerm(_) => true)]
#[invariant(::JaiTaggedSumtiTerm(_) => true)]
#[invariant(::ElidedNaheFihoTagTerm(_) => true)]
#[invariant(::TaggedSumtiBeforeTagTerm(_) => true)]
#[invariant(::TaggedSumtiTerm(_) => true)]
#[invariant(::NoihaAdverbialTerm(_) => true)]
#[invariant(::FihoiAdverbialTerm(_) => true)]
#[invariant(::SoiAdverbialTerm(_) => true)]
#[invariant(::NaKuTerm(_) => true)]
#[invariant(::SumtiTerm(_) => true)]
#[invariant(::BareNaTerm(_) => true)]
#[invariant(::GekTermset(_) => true)]
#[invariant(::ZantufaGekTermset(_) => true)]
#[invariant(::ForethoughtTermset(_) => true)]
#[invariant(::NuhiTermset(_) => true)]
#[invariant(::KeTermset(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedSimpleTermRef<'syntax> {
    PlaceTaggedSumtiTerm(&'syntax PlaceTaggedSumtiTermSyntax),
    ZantufaJoikChainedPlaceTagTerm(&'syntax ZantufaJoikChainedPlaceTagTermSyntax),
    JaiTaggedSumtiTerm(&'syntax JaiTaggedSumtiTermSyntax),
    ElidedNaheFihoTagTerm(&'syntax ElidedNaheFihoTagTermSyntax),
    TaggedSumtiBeforeTagTerm(&'syntax TaggedSumtiBeforeTagTermSyntax),
    TaggedSumtiTerm(GeneratedTaggedTermRef<'syntax>),
    NoihaAdverbialTerm(&'syntax NoihaAdverbialTermSyntax),
    FihoiAdverbialTerm(&'syntax FihoiAdverbialTermSyntax),
    SoiAdverbialTerm(&'syntax SoiAdverbialTermSyntax),
    NaKuTerm(&'syntax NaKuTermSyntax),
    SumtiTerm(&'syntax SumtiTermSyntax),
    BareNaTerm(&'syntax BareNaTermSyntax),
    GekTermset(&'syntax GekTermsetSyntax),
    ZantufaGekTermset(&'syntax ZantufaGekTermsetSyntax),
    ForethoughtTermset(&'syntax ForethoughtTermsetSyntax),
    NuhiTermset(&'syntax NuhiTermsetSyntax),
    KeTermset(&'syntax KeTermsetSyntax),
}

impl<'syntax> GeneratedSimpleTermRef<'syntax> {
    /// Borrow a leaf from the original flat simple-term sum.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_simple(term: &'syntax SimpleTermSyntax) -> Self {
        match term {
            SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => Self::PlaceTaggedSumtiTerm(term),
            SimpleTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Self::ZantufaJoikChainedPlaceTagTerm(term)
            }
            SimpleTermSyntax::JaiTaggedSumtiTerm(term) => Self::JaiTaggedSumtiTerm(term),
            SimpleTermSyntax::ElidedNaheFihoTagTerm(term) => Self::ElidedNaheFihoTagTerm(term),
            SimpleTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Self::TaggedSumtiBeforeTagTerm(term)
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => {
                Self::TaggedSumtiTerm(GeneratedTaggedTermRef::from_guarded(term))
            }
            SimpleTermSyntax::NoihaAdverbialTerm(term) => Self::NoihaAdverbialTerm(term),
            SimpleTermSyntax::FihoiAdverbialTerm(term) => Self::FihoiAdverbialTerm(term),
            SimpleTermSyntax::SoiAdverbialTerm(term) => Self::SoiAdverbialTerm(term),
            SimpleTermSyntax::NaKuTerm(term) => Self::NaKuTerm(term),
            SimpleTermSyntax::SumtiTerm(term) => Self::SumtiTerm(term),
            SimpleTermSyntax::BareNaTerm(term) => Self::BareNaTerm(term),
            SimpleTermSyntax::GekTermset(term) => Self::GekTermset(term),
            SimpleTermSyntax::ZantufaGekTermset(term) => Self::ZantufaGekTermset(term),
            SimpleTermSyntax::ForethoughtTermset(term) => Self::ForethoughtTermset(term),
            SimpleTermSyntax::NuhiTermset(term) => Self::NuhiTermset(term),
            SimpleTermSyntax::KeTermset(term) => Self::KeTermset(term),
        }
    }

    /// Borrow a leaf from the BO-bound level, or report that the node is a grouped connection.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(term, BoundTermSyntax::StagBoundTermConnection(_)))]
    pub(crate) fn from_bound(term: &'syntax BoundTermSyntax) -> Option<Self> {
        match term {
            BoundTermSyntax::StagBoundTermConnection(_) => None,
            BoundTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            BoundTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            BoundTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            BoundTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            BoundTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            BoundTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            BoundTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            BoundTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            BoundTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            BoundTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            BoundTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            BoundTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            BoundTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            BoundTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            BoundTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            BoundTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            BoundTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
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
            TermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            TermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            TermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            TermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            TermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            TermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            TermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            TermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            TermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            TermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            TermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            TermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            TermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            TermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            TermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            TermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
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
            CeheTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            CeheTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            CeheTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            CeheTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            CeheTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            CeheTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            CeheTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            CeheTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            CeheTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            CeheTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            CeheTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            CeheTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            CeheTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            CeheTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            CeheTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            CeheTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
        }
    }

    /// Borrow a leaf from the loose connective level, or report a grouped connection.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_loose(term: &'syntax LooseTermSyntax) -> Option<Self> {
        match term {
            LooseTermSyntax::ConnectedTerm(_) | LooseTermSyntax::StagBoundTermConnection(_) => None,
            LooseTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
            LooseTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            LooseTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            LooseTermSyntax::ElidedNaheFihoTagTerm(term) => Some(Self::ElidedNaheFihoTagTerm(term)),
            LooseTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            LooseTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_guarded(term),
            )),
            LooseTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            LooseTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            LooseTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            LooseTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            LooseTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            LooseTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            LooseTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            LooseTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            LooseTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            LooseTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            LooseTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
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
            NonabsTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            NonabsTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            NonabsTermSyntax::ElidedNaheFihoTagTerm(term) => {
                Some(Self::ElidedNaheFihoTagTerm(term))
            }
            NonabsTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            NonabsTermSyntax::NonabsTaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_unguarded(term),
            )),
            NonabsTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            NonabsTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            NonabsTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            NonabsTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            NonabsTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            NonabsTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            NonabsTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            NonabsTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            NonabsTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            NonabsTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            NonabsTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
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
            NormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(term) => {
                Some(Self::ZantufaJoikChainedPlaceTagTerm(term))
            }
            NormalTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            NormalTermSyntax::ElidedNaheFihoTagTerm(term) => {
                Some(Self::ElidedNaheFihoTagTerm(term))
            }
            NormalTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            NormalTermSyntax::NonabsTaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_unguarded(term),
            )),
            NormalTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            NormalTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            NormalTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            NormalTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            NormalTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            NormalTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            NormalTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            NormalTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            NormalTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            NormalTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            NormalTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
        }
    }
}

/// A borrowed sumti-association payload: the shapes a GOI-family relative phrase can carry.
///
/// The payload constituent is the shared normal-flavour term, because that is what all three
/// sources spell at `relative_clause_1` (camxes.peg:168, camxes-exp.peg:207,
/// zantufa-1.9999.peg:43). A sumti-association phrase relates its head to a SUMTI, so only the
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
            GeneratedSimpleTermRef::JaiTaggedSumtiTerm(_)
            | GeneratedSimpleTermRef::ZantufaJoikChainedPlaceTagTerm(_)
            | GeneratedSimpleTermRef::ElidedNaheFihoTagTerm(_)
            | GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(_)
            | GeneratedSimpleTermRef::NoihaAdverbialTerm(_)
            | GeneratedSimpleTermRef::FihoiAdverbialTerm(_)
            | GeneratedSimpleTermRef::SoiAdverbialTerm(_)
            | GeneratedSimpleTermRef::BareNaTerm(_)
            | GeneratedSimpleTermRef::GekTermset(_)
            | GeneratedSimpleTermRef::ZantufaGekTermset(_)
            | GeneratedSimpleTermRef::ForethoughtTermset(_)
            | GeneratedSimpleTermRef::NuhiTermset(_)
            | GeneratedSimpleTermRef::KeTermset(_) => None,
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
#[invariant(::Empty => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedLinkedSumtiRef<'syntax> {
    PlaceTagged(&'syntax PlaceTaggedLinkedSumtiSyntax),
    TenseTagged(&'syntax TenseTaggedLinkedSumtiSyntax),
    Plain(&'syntax PlainLinkedSumtiSyntax),
    Empty,
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
            LinkedTermSyntax::EmptyLinkedSumti(_) => Some(Self::Empty),
        }
    }
}
