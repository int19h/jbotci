//! Shared borrowed views over leaf-listed generated term hierarchy levels.
//!
//! The syntax grammar deliberately repeats leaf variants in the hierarchy enums so Debug and
//! serde output remain stable. These views give downstream algorithms one strongly typed leaf
//! surface without allocating or cloning the generated nodes. A `None` conversion identifies a
//! connection node whose grouping must be handled explicitly rather than flattened as a leaf.
//!
//! `GeneratedSimpleTermRef` is the leaf surface. `GeneratedBridiTermRef` is the whole-term
//! surface: a term at *any* level, which is what a bridi term list actually holds once branches
//! from different levels are spliced into one list. Every view here is `Copy` and borrowed, so no
//! path in lowering ever converts a term by copying it into another level's enum.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use std::sync::Arc;

use jbotci_syntax::generated_model::{
    AtomRef, BalancedTermsetOperandsSyntax, BareNaTermSyntax, BoundNormalTermConnectionSyntax,
    BoundNormalTermSyntax, BoundTermSyntax, CeheTermSyntax, ConnectedNormalTermSyntax,
    ConnectedTermSyntax, ElidedNaheFihoTagTermSyntax, FihoiAdverbialTermSyntax,
    ForethoughtTermsetSyntax, GekTermsetSyntax, GikConnectiveSyntax, JaiTaggedSumtiTermSyntax,
    KeTermsetSyntax, LeadingTermTagTenseModalSyntax, LinkedTermSyntax, LooseTermSyntax,
    ModalForethoughtConnectiveSyntax, NaKuTermSyntax, NodeRef, NoihaAdverbialTermSyntax,
    NonabsTaggedSumtiTermSyntax, NonabsTermSyntax, NormalTermAtomSyntax, NormalTermSyntax,
    NuhiTermsetSyntax, PeheTermsetConnectionSyntax, PlaceTaggedLinkedSumtiSyntax,
    PlaceTaggedSumtiTermSyntax, PlainLinkedSumtiSyntax, SimpleTermSyntax, SoiAdverbialTermSyntax,
    StagBoundTermConnectionSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax,
    TaggedSumtiBeforeTagTermSyntax, TaggedSumtiTermSyntax, TenseTaggedLinkedSumtiSyntax,
    TermSyntax, TermsetGroupSyntax, TreeNode, ZantufaForethoughtTermsetBranchSyntax,
    ZantufaGekTermsetSyntax,
};
use jbotci_tree::TreeVisitor;

/// A borrowed tag-led term leaf.
///
/// The absorption-guarded `TaggedSumtiTermSyntax` and its unguarded `nonabs` twin differ only by
/// the `!selbri` assertion that decides where the term ends. That is a parse-time boundary rule
/// with no semantic content, so lowering sees exactly one shape for both.
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

/// Report whether any operand of a NUhI-less forethought termset satisfies a predicate.
#[requires(true)]
#[ensures(true)]
pub(crate) fn any_gek_termset_operand(
    operands: &BalancedTermsetOperandsSyntax,
    predicate: &mut impl FnMut(&NormalTermSyntax) -> bool,
) -> bool {
    match operands {
        BalancedTermsetOperandsSyntax::GikPairedTermsetOperands(pair) => {
            predicate(pair.leading_operand.as_ref()) || predicate(pair.trailing_operand.as_ref())
        }
        BalancedTermsetOperandsSyntax::NestedPairedTermsetOperands(pair) => {
            predicate(pair.leading_operand.as_ref())
                || any_gek_termset_operand(pair.inner.as_ref(), predicate)
                || predicate(pair.trailing_operand.as_ref())
        }
    }
}

/// A borrowed simple-term leaf shared by every level of the composed term hierarchy.
#[invariant(::PlaceTaggedSumtiTerm(_) => true)]
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
    /// The unguarded tag leaf lowers exactly like its absorption-guarded twin.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_nonabs(term: &'syntax NonabsTermSyntax) -> Option<Self> {
        match term {
            NonabsTermSyntax::ConnectedTerm(_) | NonabsTermSyntax::StagBoundTermConnection(_) => {
                None
            }
            NonabsTermSyntax::PlaceTaggedSumtiTerm(term) => Some(Self::PlaceTaggedSumtiTerm(term)),
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

    /// Borrow a leaf from the normal-flavour BO-bound level, or report a grouped connection.
    #[requires(true)]
    #[ensures(ret.is_none() == matches!(term, BoundNormalTermSyntax::BoundNormalTermConnection(_)))]
    pub(crate) fn from_bound_normal(term: &'syntax BoundNormalTermSyntax) -> Option<Self> {
        match term {
            BoundNormalTermSyntax::BoundNormalTermConnection(_) => None,
            BoundNormalTermSyntax::PlaceTaggedSumtiTerm(term) => {
                Some(Self::PlaceTaggedSumtiTerm(term))
            }
            BoundNormalTermSyntax::JaiTaggedSumtiTerm(term) => Some(Self::JaiTaggedSumtiTerm(term)),
            BoundNormalTermSyntax::ElidedNaheFihoTagTerm(term) => {
                Some(Self::ElidedNaheFihoTagTerm(term))
            }
            BoundNormalTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            BoundNormalTermSyntax::NonabsTaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(
                GeneratedTaggedTermRef::from_unguarded(term),
            )),
            BoundNormalTermSyntax::NoihaAdverbialTerm(term) => Some(Self::NoihaAdverbialTerm(term)),
            BoundNormalTermSyntax::FihoiAdverbialTerm(term) => Some(Self::FihoiAdverbialTerm(term)),
            BoundNormalTermSyntax::SoiAdverbialTerm(term) => Some(Self::SoiAdverbialTerm(term)),
            BoundNormalTermSyntax::NaKuTerm(term) => Some(Self::NaKuTerm(term)),
            BoundNormalTermSyntax::SumtiTerm(term) => Some(Self::SumtiTerm(term)),
            BoundNormalTermSyntax::BareNaTerm(term) => Some(Self::BareNaTerm(term)),
            BoundNormalTermSyntax::GekTermset(term) => Some(Self::GekTermset(term)),
            BoundNormalTermSyntax::ZantufaGekTermset(term) => Some(Self::ZantufaGekTermset(term)),
            BoundNormalTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            BoundNormalTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            BoundNormalTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
        }
    }

    /// Borrow the leaf of the normal-flavour atom level, which admits no connective tier.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_normal_atom(term: &'syntax NormalTermAtomSyntax) -> Self {
        match term {
            NormalTermAtomSyntax::PlaceTaggedSumtiTerm(term) => Self::PlaceTaggedSumtiTerm(term),
            NormalTermAtomSyntax::JaiTaggedSumtiTerm(term) => Self::JaiTaggedSumtiTerm(term),
            NormalTermAtomSyntax::ElidedNaheFihoTagTerm(term) => Self::ElidedNaheFihoTagTerm(term),
            NormalTermAtomSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Self::TaggedSumtiBeforeTagTerm(term)
            }
            NormalTermAtomSyntax::NonabsTaggedSumtiTerm(term) => {
                Self::TaggedSumtiTerm(GeneratedTaggedTermRef::from_unguarded(term))
            }
            NormalTermAtomSyntax::NoihaAdverbialTerm(term) => Self::NoihaAdverbialTerm(term),
            NormalTermAtomSyntax::FihoiAdverbialTerm(term) => Self::FihoiAdverbialTerm(term),
            NormalTermAtomSyntax::SoiAdverbialTerm(term) => Self::SoiAdverbialTerm(term),
            NormalTermAtomSyntax::NaKuTerm(term) => Self::NaKuTerm(term),
            NormalTermAtomSyntax::SumtiTerm(term) => Self::SumtiTerm(term),
            NormalTermAtomSyntax::BareNaTerm(term) => Self::BareNaTerm(term),
            NormalTermAtomSyntax::GekTermset(term) => Self::GekTermset(term),
            NormalTermAtomSyntax::ZantufaGekTermset(term) => Self::ZantufaGekTermset(term),
            NormalTermAtomSyntax::ForethoughtTermset(term) => Self::ForethoughtTermset(term),
            NormalTermAtomSyntax::NuhiTermset(term) => Self::NuhiTermset(term),
            NormalTermAtomSyntax::KeTermset(term) => Self::KeTermset(term),
        }
    }

    /// Describe experimental leaf forms that semantic lowering does not yet model.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn undefined_experimental_description(self) -> Option<&'static str> {
        match self {
            Self::NoihaAdverbialTerm(_) => Some("an experimental NOIhA adverbial term"),
            Self::FihoiAdverbialTerm(_) => Some("an experimental FIhOI adverbial term"),
            Self::SoiAdverbialTerm(_) => Some("an experimental SOI/XOI adverbial term"),
            Self::JaiTaggedSumtiTerm(_) => Some("an experimental Zantufa JAI tag term"),
            _ => None,
        }
    }
}

/// A borrowed sumti-association payload: the shapes a GOI-family relative phrase can lower.
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

    /// Whether the payload carries a sumti for an association to read.
    ///
    /// `NA KU` is a projected association shape but holds no sumti, so it joins the leaves that
    /// project to nothing at all: both are payloads a `goi` assignment cannot act on. They are
    /// the payloads that have to be reported rather than silently associating nothing.
    #[requires(true)]
    #[ensures(ret == !matches!(self, Self::NaKu))]
    pub(crate) fn associates_a_sumti(self) -> bool {
        match self {
            Self::Plain(_) | Self::Tagged(_) | Self::PlaceTagged(_) => true,
            Self::NaKu => false,
        }
    }
}

/// A borrowed grouping node: a term that is a connection of other terms rather than a leaf.
///
/// Every level of the composed hierarchy admits a suffix of these tiers — the leaf level admits
/// none, the BO-bound level admits only the BO connection, and the PEhE level admits all four —
/// but the product node a given tier builds is the same type at every level that offers it, so
/// grouping is level-independent even though the enums that carry it are not.
#[invariant(::PeheTermsetConnection(_) => true)]
#[invariant(::TermsetGroup(_) => true)]
#[invariant(::ConnectedTerm(_) => true)]
#[invariant(::StagBoundTermConnection(_) => true)]
#[invariant(::ConnectedNormalTerm(_) => true)]
#[invariant(::BoundNormalTermConnection(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedTermGroupingRef<'syntax> {
    PeheTermsetConnection(&'syntax PeheTermsetConnectionSyntax),
    TermsetGroup(&'syntax TermsetGroupSyntax),
    ConnectedTerm(&'syntax ConnectedTermSyntax),
    StagBoundTermConnection(&'syntax StagBoundTermConnectionSyntax),
    ConnectedNormalTerm(&'syntax ConnectedNormalTermSyntax),
    BoundNormalTermConnection(&'syntax BoundNormalTermConnectionSyntax),
}

/// A borrowed term of the composed hierarchy, at whichever level the term list drew it from.
///
/// Bridi lowering does not work on one syntactic level. A sentence contributes its own
/// `TermSyntax` terms, a tail contributes more of them, a NUhI-present termset branch contributes
/// `TermSyntax` again, and a NUhI-less GEK termset branch contributes the unguarded
/// `NonabsTermSyntax` operands that are what make `ge ko'a gi pu broda` parse at all. The levels
/// differ only in which connective tiers they admit above the shared leaf inventory — mechanism E
/// re-lists the same leaves at every level — and lowering asks a term only two things: which leaf
/// it is, or, failing that, which grouping node it is. Both answers are level-independent.
///
/// So one `Copy` view spans every level, and it holds the level's own node rather than a projection
/// of it: `simple` and `grouping` are the two projections, and `visit_in_order` still walks the
/// real node, so nothing that traverses a term list observes a different event stream than it did
/// when the list was a slice of one level's references.
///
/// This type exists precisely so that splicing a branch into the surrounding term list stays a
/// borrow. There is deliberately no conversion that copies a node out of one level's enum into
/// another's.
#[invariant(::Term(_) => true)]
#[invariant(::Cehe(_) => true)]
#[invariant(::Loose(_) => true)]
#[invariant(::Nonabs(_) => true)]
#[invariant(::Bound(_) => true)]
#[invariant(::Simple(_) => true)]
#[invariant(::Normal(_) => true)]
#[invariant(::BoundNormal(_) => true)]
#[invariant(::NormalAtom(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedBridiTermRef<'syntax> {
    Term(&'syntax TermSyntax),
    Cehe(&'syntax CeheTermSyntax),
    Loose(&'syntax LooseTermSyntax),
    Nonabs(&'syntax NonabsTermSyntax),
    Bound(&'syntax BoundTermSyntax),
    Simple(&'syntax SimpleTermSyntax),
    Normal(&'syntax NormalTermSyntax),
    BoundNormal(&'syntax BoundNormalTermSyntax),
    NormalAtom(&'syntax NormalTermAtomSyntax),
}

impl<'syntax> GeneratedBridiTermRef<'syntax> {
    /// Borrow the leaf, or report that this term is a grouping node instead.
    #[requires(true)]
    #[ensures(ret.is_none() == self.grouping().is_some())]
    pub(crate) fn simple(self) -> Option<GeneratedSimpleTermRef<'syntax>> {
        match self {
            Self::Term(term) => GeneratedSimpleTermRef::from_term(term),
            Self::Cehe(term) => GeneratedSimpleTermRef::from_cehe(term),
            Self::Loose(term) => GeneratedSimpleTermRef::from_loose(term),
            Self::Nonabs(term) => GeneratedSimpleTermRef::from_nonabs(term),
            Self::Bound(term) => GeneratedSimpleTermRef::from_bound(term),
            Self::Simple(term) => Some(GeneratedSimpleTermRef::from_simple(term)),
            Self::Normal(term) => GeneratedSimpleTermRef::from_normal(term),
            Self::BoundNormal(term) => GeneratedSimpleTermRef::from_bound_normal(term),
            Self::NormalAtom(term) => Some(GeneratedSimpleTermRef::from_normal_atom(term)),
        }
    }

    /// Borrow the grouping node, or report that this term is a leaf instead.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn grouping(self) -> Option<GeneratedTermGroupingRef<'syntax>> {
        match self {
            Self::Term(TermSyntax::PeheTermsetConnection(connection)) => {
                Some(GeneratedTermGroupingRef::PeheTermsetConnection(connection))
            }
            Self::Term(TermSyntax::TermsetGroup(group))
            | Self::Cehe(CeheTermSyntax::TermsetGroup(group)) => {
                Some(GeneratedTermGroupingRef::TermsetGroup(group))
            }
            Self::Term(TermSyntax::ConnectedTerm(connection))
            | Self::Cehe(CeheTermSyntax::ConnectedTerm(connection))
            | Self::Loose(LooseTermSyntax::ConnectedTerm(connection))
            | Self::Nonabs(NonabsTermSyntax::ConnectedTerm(connection)) => {
                Some(GeneratedTermGroupingRef::ConnectedTerm(connection))
            }
            Self::Term(TermSyntax::StagBoundTermConnection(connection))
            | Self::Cehe(CeheTermSyntax::StagBoundTermConnection(connection))
            | Self::Loose(LooseTermSyntax::StagBoundTermConnection(connection))
            | Self::Nonabs(NonabsTermSyntax::StagBoundTermConnection(connection))
            | Self::Bound(BoundTermSyntax::StagBoundTermConnection(connection)) => Some(
                GeneratedTermGroupingRef::StagBoundTermConnection(connection),
            ),
            Self::Normal(NormalTermSyntax::ConnectedNormalTerm(connection)) => {
                Some(GeneratedTermGroupingRef::ConnectedNormalTerm(connection))
            }
            Self::Normal(NormalTermSyntax::BoundNormalTermConnection(connection))
            | Self::BoundNormal(BoundNormalTermSyntax::BoundNormalTermConnection(connection)) => {
                Some(GeneratedTermGroupingRef::BoundNormalTermConnection(
                    connection,
                ))
            }
            _ => None,
        }
    }

    /// Walk the underlying node in source order.
    ///
    /// The view is not a `TreeNode` — it has no node identity of its own — but every arm borrows
    /// one, so in-order scans dispatch straight through to it and observe the node they would have
    /// observed before the term list became level-agnostic.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn visit_in_order<V>(self, visitor: &mut V)
    where
        V: TreeVisitor<'syntax, Node = NodeRef<'syntax>, Atom = AtomRef<'syntax>>,
    {
        match self {
            Self::Term(term) => term.visit_in_order(visitor),
            Self::Cehe(term) => term.visit_in_order(visitor),
            Self::Loose(term) => term.visit_in_order(visitor),
            Self::Nonabs(term) => term.visit_in_order(visitor),
            Self::Bound(term) => term.visit_in_order(visitor),
            Self::Simple(term) => term.visit_in_order(visitor),
            Self::Normal(term) => term.visit_in_order(visitor),
            Self::BoundNormal(term) => term.visit_in_order(visitor),
            Self::NormalAtom(term) => term.visit_in_order(visitor),
        }
    }
}

/// A forethought termset in a bridi term list, in whichever sourced shape it parsed as.
///
/// Three arms join their branches with a GEK/GIK pair and therefore lower as a logical connection:
/// the NUhI-mandatory arm, whose two branches are `terms` sequences written out in source order;
/// the NUhI-less `gek_termset`, whose two branches have to be recovered from a balanced operand
/// tree; and rolling Zantufa's `gek_term`, whose branches are `term+` runs in source order and may
/// number more than two. Everything downstream of "which terms are in which branch" is identical,
/// so the shapes are distinguished only here.
#[invariant(::Nuhi(_) => true)]
#[invariant(::Gek(_) => true)]
#[invariant(::Zantufa(_) => true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum GeneratedForethoughtTermsetRef<'syntax> {
    Nuhi(&'syntax ForethoughtTermsetSyntax),
    Gek(&'syntax GekTermsetSyntax),
    Zantufa(&'syntax ZantufaGekTermsetSyntax),
}

impl<'syntax> GeneratedForethoughtTermsetRef<'syntax> {
    /// The opening forethought connective.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn gek(self) -> &'syntax ModalForethoughtConnectiveSyntax {
        match self {
            Self::Nuhi(termset) => &termset.gek,
            Self::Gek(termset) => &termset.0.gek,
            Self::Zantufa(termset) => &termset.0.gek,
        }
    }

    /// The GIK that separates the first two branches.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn gik(self) -> &'syntax GikConnectiveSyntax {
        match self {
            Self::Nuhi(termset) => &termset.first_branch.gik,
            // The GIK alternative is listed before the recursive one, exactly as upstream orders
            // it, so the innermost pair is the one that carries the GIK.
            Self::Gek(termset) => innermost_gek_termset_gik(termset.0.operands.as_ref()),
            Self::Zantufa(termset) => &termset.0.first_branch.gik,
        }
    }

    /// The experimental Zantufa branches beyond the first two, which only its own arm can carry.
    #[requires(true)]
    #[ensures(ret.is_empty() || matches!(self, Self::Zantufa(_)))]
    pub(crate) fn additional_branches(self) -> &'syntax [ZantufaForethoughtTermsetBranchSyntax] {
        match self {
            Self::Nuhi(_) | Self::Gek(_) => &[],
            Self::Zantufa(termset) => &termset.0.additional_branches,
        }
    }

    /// The two branches, in the order the connective combines them.
    ///
    /// For the NUhI arm both branches are written out in source order. For the NUhI-less arm
    /// `terms_gik_terms <- nonabs_term (gik / terms_gik_terms) nonabs_term` pairs operands by
    /// NESTING rather than by concatenation: each level contributes exactly one operand before its
    /// centre and one after it, so an n-operand termset nests n/2 deep and the outermost operands
    /// are the outermost pair. The branch before the GIK is therefore the leading operands read
    /// OUTERMOST-FIRST, and the branch behind it is the trailing operands read INNERMOST-FIRST.
    /// On a symmetric termset such as `ge pu ko'a pu ko'e gi pu ko'i pu ko'u broda` that is
    /// exactly the membership the old flat reading produced, which is why re-shaping the tree
    /// leaves the corpus graphs alone.
    #[requires(true)]
    #[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
    pub(crate) fn branches(
        self,
    ) -> (
        Vec<GeneratedBridiTermRef<'syntax>>,
        Vec<GeneratedBridiTermRef<'syntax>>,
    ) {
        match self {
            Self::Nuhi(termset) => (
                termset
                    .terms
                    .iter()
                    .map(|term| GeneratedBridiTermRef::Term(term.as_ref()))
                    .collect(),
                termset
                    .first_branch
                    .terms
                    .iter()
                    .map(|term| GeneratedBridiTermRef::Term(term.as_ref()))
                    .collect(),
            ),
            Self::Gek(termset) => {
                let mut leading = Vec::new();
                let mut trailing = Vec::new();
                collect_gek_termset_branches(
                    termset.0.operands.as_ref(),
                    &mut leading,
                    &mut trailing,
                );
                (leading, trailing)
            }
            // Zantufa's `gek_term <- gek term+ (gik term+)+ GIhI?` writes each branch out as a
            // whole run in source order, so the first two branches need no reconstruction; any
            // further ones are `additional_branches`.
            Self::Zantufa(termset) => (
                termset
                    .0
                    .terms
                    .iter()
                    .map(|term| GeneratedBridiTermRef::Term(term.as_ref()))
                    .collect(),
                termset
                    .0
                    .first_branch
                    .terms
                    .iter()
                    .map(|term| GeneratedBridiTermRef::Term(term.as_ref()))
                    .collect(),
            ),
        }
    }
}

/// Append a NUhI-less termset's operands to its two branches; see `branches` for the rule.
#[requires(true)]
#[ensures(
    leading.len() - old(leading.len()) == trailing.len() - old(trailing.len()),
    "every nesting level contributes exactly one operand to each branch"
)]
fn collect_gek_termset_branches<'syntax>(
    operands: &'syntax BalancedTermsetOperandsSyntax,
    leading: &mut Vec<GeneratedBridiTermRef<'syntax>>,
    trailing: &mut Vec<GeneratedBridiTermRef<'syntax>>,
) {
    match operands {
        BalancedTermsetOperandsSyntax::GikPairedTermsetOperands(pair) => {
            leading.push(GeneratedBridiTermRef::Normal(pair.leading_operand.as_ref()));
            trailing.push(GeneratedBridiTermRef::Normal(
                pair.trailing_operand.as_ref(),
            ));
        }
        BalancedTermsetOperandsSyntax::NestedPairedTermsetOperands(pair) => {
            leading.push(GeneratedBridiTermRef::Normal(pair.leading_operand.as_ref()));
            collect_gek_termset_branches(pair.inner.as_ref(), leading, trailing);
            trailing.push(GeneratedBridiTermRef::Normal(
                pair.trailing_operand.as_ref(),
            ));
        }
    }
}

/// The GIK of a NUhI-less termset, which the innermost operand pair carries.
#[requires(true)]
#[ensures(true)]
fn innermost_gek_termset_gik(operands: &BalancedTermsetOperandsSyntax) -> &GikConnectiveSyntax {
    match operands {
        BalancedTermsetOperandsSyntax::GikPairedTermsetOperands(pair) => &pair.gik,
        BalancedTermsetOperandsSyntax::NestedPairedTermsetOperands(pair) => {
            innermost_gek_termset_gik(pair.inner.as_ref())
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
