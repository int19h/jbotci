//! Shared borrowed views over leaf-listed generated term hierarchy levels.
//!
//! The syntax grammar deliberately repeats leaf variants in the hierarchy enums so Debug and
//! serde output remain stable. These views give downstream algorithms one strongly typed leaf
//! surface without allocating or cloning the generated nodes. A `None` conversion identifies a
//! connection node whose grouping must be handled explicitly rather than flattened as a leaf.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use std::sync::Arc;

use jbotci_syntax::generated_model::{
    BareNaTermSyntax, BoundTermSyntax, CeheTermSyntax, ElidedNaheFihoTagTermSyntax,
    FihoiAdverbialTermSyntax, ForethoughtTermsetSyntax, JaiTaggedSumtiTermSyntax, KeTermsetSyntax,
    LeadingTermTagTenseModalSyntax, LinkedTermSyntax, LooseTermSyntax, NaKuTermSyntax,
    NoihaAdverbialTermSyntax, NonabsTaggedSumtiTermSyntax, NonabsTermSyntax, NuhiTermsetSyntax,
    PlaceTaggedLinkedSumtiSyntax, PlaceTaggedSumtiTermSyntax, PlainLinkedSumtiSyntax,
    SimpleTermSyntax, SoiAdverbialTermSyntax, SumtiTermSyntax, TaggedOrElidedSumtiSyntax,
    TaggedSumtiBeforeTagTermSyntax, TaggedSumtiTermSyntax, TenseTaggedLinkedSumtiSyntax,
    TermSyntax,
};

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
            NonabsTermSyntax::ForethoughtTermset(term) => Some(Self::ForethoughtTermset(term)),
            NonabsTermSyntax::NuhiTermset(term) => Some(Self::NuhiTermset(term)),
            NonabsTermSyntax::KeTermset(term) => Some(Self::KeTermset(term)),
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
