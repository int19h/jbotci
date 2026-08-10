//! Shared borrowed views over leaf-listed generated term hierarchy levels.
//!
//! The syntax grammar deliberately repeats leaf variants in the hierarchy enums so Debug and
//! serde output remain stable. These views give downstream algorithms one strongly typed leaf
//! surface without allocating or cloning the generated nodes. A `None` conversion identifies a
//! connection node whose grouping must be handled explicitly rather than flattened as a leaf.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_syntax::generated_model::{
    BareNaTermSyntax, BoundTermSyntax, FihoiAdverbialTermSyntax, ForethoughtTermsetSyntax,
    JaiTaggedSumtiTermSyntax, KeTermsetSyntax, LinkedTermSyntax, NaKuTermSyntax,
    NoihaAdverbialTermSyntax, NuhiTermsetSyntax, PlaceTaggedLinkedSumtiSyntax,
    PlaceTaggedSumtiTermSyntax, PlainLinkedSumtiSyntax, SimpleTermSyntax, SoiAdverbialTermSyntax,
    SumtiTermSyntax, TaggedSumtiBeforeTagTermSyntax, TaggedSumtiTermSyntax,
    TenseTaggedLinkedSumtiSyntax,
};

/// A borrowed simple-term leaf shared by `SimpleTermSyntax` and `BoundTermSyntax`.
#[invariant(::PlaceTaggedSumtiTerm(_) => true)]
#[invariant(::JaiTaggedSumtiTerm(_) => true)]
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
    TaggedSumtiBeforeTagTerm(&'syntax TaggedSumtiBeforeTagTermSyntax),
    TaggedSumtiTerm(&'syntax TaggedSumtiTermSyntax),
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
            SimpleTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Self::TaggedSumtiBeforeTagTerm(term)
            }
            SimpleTermSyntax::TaggedSumtiTerm(term) => Self::TaggedSumtiTerm(term),
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
            BoundTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                Some(Self::TaggedSumtiBeforeTagTerm(term))
            }
            BoundTermSyntax::TaggedSumtiTerm(term) => Some(Self::TaggedSumtiTerm(term)),
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
