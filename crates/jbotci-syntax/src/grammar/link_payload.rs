//! Eligibility for the complete BE/BEI normal-term payload (#793).
//!
//! Old place/tag/sumti leaves and connections retain their owners. In particular, BAI, FIhO,
//! KI and elided KU are NOT new width (PM ruling 2026-09-05). Tag-family node names are not
//! discriminators: their fields are covered by the old tense-tagged linked leaf. A complete
//! NUhI termset, in contrast, adds width even when every operand is itself an old sumti.
//!
//! The generated walkers recurse only along the payload's connection spine. A sumti's internal
//! bridi, links, or termsets cannot make a PlainLinkedSumti payload new. Recovered leaf evidence
//! is checked separately through the generated in-order traversal: each required source field
//! must actually have parsed, rather than merely occupy a wrapper. An error anywhere in that
//! leaf makes its ownership unproven. Prefix values are inspected, not rejected by enum name.

use bityzba::{contract_trait, invariant, requires};
use jbotci_tree::TreeVisitor;

use super::generated_model::{self as model, recovered};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkPayloadBreadth {
    LegacyLinked,
    AddedFullTerm,
    Unproven,
}

impl LinkPayloadBreadth {
    /// Uncertainty absorbs the entire connection, including when another operand adds width.
    #[requires(true)]
    #[ensures((ret == Self::Unproven) == (self == Self::Unproven || other == Self::Unproven))]
    #[ensures((ret == Self::LegacyLinked) == (self == Self::LegacyLinked && other == Self::LegacyLinked))]
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unproven, Self::LegacyLinked | Self::AddedFullTerm | Self::Unproven)
            | (Self::LegacyLinked | Self::AddedFullTerm, Self::Unproven) => Self::Unproven,
            (Self::AddedFullTerm, Self::LegacyLinked | Self::AddedFullTerm)
            | (Self::LegacyLinked, Self::AddedFullTerm) => Self::AddedFullTerm,
            (Self::LegacyLinked, Self::LegacyLinked) => Self::LegacyLinked,
        }
    }
}

#[invariant(true)]
struct BreadthWalker {
    answer: LinkPayloadBreadth,
}

impl BreadthWalker {
    #[requires(true)]
    #[ensures(self.answer == old(self.answer).combine(answer))]
    fn observe(&mut self, answer: LinkPayloadBreadth) {
        self.answer = self.answer.combine(answer);
    }
}

impl<'tree> model::TreeWalker<'tree> for BreadthWalker {
    #[requires(self.answer != LinkPayloadBreadth::Unproven)]
    #[ensures(self.answer != LinkPayloadBreadth::Unproven)]
    fn walk_normal_term(&mut self, node: &'tree model::NormalTermSyntax) {
        match node {
            model::NormalTermSyntax::ConnectedNormalTerm(_)
            | model::NormalTermSyntax::BoundNormalTermConnection(_) => {
                model::walk::normal_term(self, node)
            }
            model::NormalTermSyntax::PlaceTaggedSumtiTerm(_)
            | model::NormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
            | model::NormalTermSyntax::ElidedNaheFihoTagTerm(_)
            | model::NormalTermSyntax::TaggedSumtiBeforeTagTerm(_)
            | model::NormalTermSyntax::NonabsTaggedSumtiTerm(_)
            | model::NormalTermSyntax::SumtiTerm(_) => {
                self.observe(LinkPayloadBreadth::LegacyLinked)
            }
            model::NormalTermSyntax::JaiTaggedSumtiTerm(_)
            | model::NormalTermSyntax::NoihaAdverbialTerm(_)
            | model::NormalTermSyntax::FihoiProposalAdverbialTerm(_)
            | model::NormalTermSyntax::ZantufaXoiAdverbialTerm(_)
            | model::NormalTermSyntax::ExpSoiAdverbialTerm(_)
            | model::NormalTermSyntax::NaKuTerm(_)
            | model::NormalTermSyntax::BareNaTerm(_)
            | model::NormalTermSyntax::GekTermset(_)
            | model::NormalTermSyntax::ZantufaGekTermset(_)
            | model::NormalTermSyntax::ForethoughtTermset(_)
            | model::NormalTermSyntax::NuhiTermset(_)
            | model::NormalTermSyntax::KeTermset(_) => {
                self.observe(LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(self.answer != LinkPayloadBreadth::Unproven)]
    #[ensures(self.answer != LinkPayloadBreadth::Unproven)]
    fn walk_bound_normal_term(&mut self, node: &'tree model::BoundNormalTermSyntax) {
        match node {
            model::BoundNormalTermSyntax::BoundNormalTermConnection(_) => {
                model::walk::bound_normal_term(self, node)
            }
            model::BoundNormalTermSyntax::PlaceTaggedSumtiTerm(_)
            | model::BoundNormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(_)
            | model::BoundNormalTermSyntax::ElidedNaheFihoTagTerm(_)
            | model::BoundNormalTermSyntax::TaggedSumtiBeforeTagTerm(_)
            | model::BoundNormalTermSyntax::NonabsTaggedSumtiTerm(_)
            | model::BoundNormalTermSyntax::SumtiTerm(_) => {
                self.observe(LinkPayloadBreadth::LegacyLinked)
            }
            model::BoundNormalTermSyntax::JaiTaggedSumtiTerm(_)
            | model::BoundNormalTermSyntax::NoihaAdverbialTerm(_)
            | model::BoundNormalTermSyntax::FihoiProposalAdverbialTerm(_)
            | model::BoundNormalTermSyntax::ZantufaXoiAdverbialTerm(_)
            | model::BoundNormalTermSyntax::ExpSoiAdverbialTerm(_)
            | model::BoundNormalTermSyntax::NaKuTerm(_)
            | model::BoundNormalTermSyntax::BareNaTerm(_)
            | model::BoundNormalTermSyntax::GekTermset(_)
            | model::BoundNormalTermSyntax::ZantufaGekTermset(_)
            | model::BoundNormalTermSyntax::ForethoughtTermset(_)
            | model::BoundNormalTermSyntax::NuhiTermset(_)
            | model::BoundNormalTermSyntax::KeTermset(_) => {
                self.observe(LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(self.answer != LinkPayloadBreadth::Unproven)]
    #[ensures(self.answer != LinkPayloadBreadth::Unproven)]
    fn walk_normal_term_atom(&mut self, node: &'tree model::NormalTermAtomSyntax) {
        match node {
            model::NormalTermAtomSyntax::PlaceTaggedSumtiTerm(_)
            | model::NormalTermAtomSyntax::ZantufaJoikChainedPlaceTagTerm(_)
            | model::NormalTermAtomSyntax::ElidedNaheFihoTagTerm(_)
            | model::NormalTermAtomSyntax::TaggedSumtiBeforeTagTerm(_)
            | model::NormalTermAtomSyntax::NonabsTaggedSumtiTerm(_)
            | model::NormalTermAtomSyntax::SumtiTerm(_) => {
                self.observe(LinkPayloadBreadth::LegacyLinked)
            }
            model::NormalTermAtomSyntax::JaiTaggedSumtiTerm(_)
            | model::NormalTermAtomSyntax::NoihaAdverbialTerm(_)
            | model::NormalTermAtomSyntax::FihoiProposalAdverbialTerm(_)
            | model::NormalTermAtomSyntax::ZantufaXoiAdverbialTerm(_)
            | model::NormalTermAtomSyntax::ExpSoiAdverbialTerm(_)
            | model::NormalTermAtomSyntax::NaKuTerm(_)
            | model::NormalTermAtomSyntax::BareNaTerm(_)
            | model::NormalTermAtomSyntax::GekTermset(_)
            | model::NormalTermAtomSyntax::ZantufaGekTermset(_)
            | model::NormalTermAtomSyntax::ForethoughtTermset(_)
            | model::NormalTermAtomSyntax::NuhiTermset(_)
            | model::NormalTermAtomSyntax::KeTermset(_) => {
                self.observe(LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(self.answer != LinkPayloadBreadth::Unproven)]
    #[ensures(self.answer != LinkPayloadBreadth::Unproven)]
    fn walk_normal_term_bo_continuation(
        &mut self,
        node: &'tree model::NormalTermBoContinuationSyntax,
    ) {
        match node {
            model::NormalTermBoContinuationSyntax::BoundNormalTermContinuation(_) => {}
            // The old link ladder requires a connective. BO alone is new width even if both
            // operands are legacy, so the continuation's own sourced marker contributes too.
            model::NormalTermBoContinuationSyntax::ZantufaBoundNormalTermContinuation(_) => {
                self.observe(LinkPayloadBreadth::AddedFullTerm);
            }
        }
        model::walk::normal_term_bo_continuation(self, node);
    }

    #[requires(true)]
    #[ensures(self.answer == old(self.answer))]
    fn walk_term_afterthought_connective(
        &mut self,
        _node: &'tree model::TermAfterthoughtConnectiveSyntax,
    ) {
        // The old connection owns this whole field, including any nested modifier syntax.
    }

    #[requires(true)]
    #[ensures(self.answer == old(self.answer))]
    fn walk_tense_modal(&mut self, _node: &'tree model::TenseModalSyntax) {
        // Optional connection tags are also old fields; an internal FIhO bridi is not an operand.
    }

    #[requires(true)]
    #[ensures(self.answer == old(self.answer))]
    fn walk_free_modifier(&mut self, _node: &'tree model::FreeModifierSyntax) {
        // In particular, BO's free modifiers cannot add width to the surrounding connection.
    }
}

/// The source value supplied by a wrapper; skipped prefix input is not itself entry evidence.
#[requires(true)]
#[ensures(ret.is_none() == matches!(node, recovered::Recovered::Error(_)))]
fn parsed_value<T>(node: &recovered::Recovered<T>) -> Option<&T> {
    match node {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(prefix) => Some(&prefix.value),
        recovered::Recovered::Error(_) => None,
    }
}

#[invariant(true)]
struct LeafEvidence {
    parsed_token: bool,
    uncertainty: bool,
}

impl<'tree> TreeVisitor<'tree> for LeafEvidence {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.parsed_token)]
    fn visit_atom(&mut self, _atom: Self::Atom) {
        self.parsed_token = true;
    }

    #[requires(true)]
    #[ensures(self.uncertainty)]
    fn visit_recovered_error<E: jbotci_tree::RecoveryItemState + serde::Serialize>(
        &mut self,
        _item: &'tree E,
    ) {
        self.uncertainty = true;
    }
}

impl BreadthWalker {
    /// A selected leaf alone is not proof: its required marker and child slots must all parse.
    /// A top-level Prefix may prove its value, but cannot borrow its skipped tokens as proof.
    #[requires(answer != LinkPayloadBreadth::Unproven)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    #[ensures(matches!(node, recovered::Recovered::Error(_)) -> self.answer == LinkPayloadBreadth::Unproven)]
    fn observe_leaf<T: recovered::TreeNode>(
        &mut self,
        node: &recovered::Recovered<T>,
        answer: LinkPayloadBreadth,
    ) {
        let Some(value) = parsed_value(node) else {
            self.observe(LinkPayloadBreadth::Unproven);
            return;
        };
        let mut evidence = LeafEvidence {
            parsed_token: false,
            uncertainty: false,
        };
        recovered::TreeNode::visit_in_order(value, &mut evidence);
        self.observe(if evidence.parsed_token && !evidence.uncertainty {
            answer
        } else {
            LinkPayloadBreadth::Unproven
        });
    }

    /// Dispatch a parsed connection child back into the generated recursive walker.
    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    #[ensures(matches!(node, recovered::Recovered::Error(_)) -> self.answer == LinkPayloadBreadth::Unproven)]
    fn descend<'tree, T: recovered::TreeWalkable<'tree>>(
        &mut self,
        node: &'tree recovered::Recovered<T>,
    ) {
        match parsed_value(node) {
            Some(value) => recovered::TreeWalkable::walk_with(value, self),
            None => self.observe(LinkPayloadBreadth::Unproven),
        }
    }
}

impl<'tree> recovered::TreeWalker<'tree> for BreadthWalker {
    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_normal_term(&mut self, node: &'tree recovered::NormalTermSyntax) {
        match node {
            recovered::NormalTermSyntax::ConnectedNormalTerm(value) => self.descend(value),
            recovered::NormalTermSyntax::BoundNormalTermConnection(value) => self.descend(value),
            recovered::NormalTermSyntax::PlaceTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::ElidedNaheFihoTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::TaggedSumtiBeforeTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::NonabsTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::SumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermSyntax::JaiTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::NoihaAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::FihoiProposalAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::ZantufaXoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::ExpSoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::NaKuTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::BareNaTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::GekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::ZantufaGekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::ForethoughtTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::NuhiTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermSyntax::KeTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_bound_normal_term(&mut self, node: &'tree recovered::BoundNormalTermSyntax) {
        match node {
            recovered::BoundNormalTermSyntax::BoundNormalTermConnection(value) => {
                self.descend(value)
            }
            recovered::BoundNormalTermSyntax::PlaceTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::ZantufaJoikChainedPlaceTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::ElidedNaheFihoTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::TaggedSumtiBeforeTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::NonabsTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::SumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::BoundNormalTermSyntax::JaiTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::NoihaAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::FihoiProposalAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::ZantufaXoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::ExpSoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::NaKuTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::BareNaTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::GekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::ZantufaGekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::ForethoughtTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::NuhiTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::BoundNormalTermSyntax::KeTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_normal_term_atom(&mut self, node: &'tree recovered::NormalTermAtomSyntax) {
        match node {
            recovered::NormalTermAtomSyntax::PlaceTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::ZantufaJoikChainedPlaceTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::ElidedNaheFihoTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::TaggedSumtiBeforeTagTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::NonabsTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::SumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::LegacyLinked)
            }
            recovered::NormalTermAtomSyntax::JaiTaggedSumtiTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::NoihaAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::FihoiProposalAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::ZantufaXoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::ExpSoiAdverbialTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::NaKuTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::BareNaTerm(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::GekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::ZantufaGekTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::ForethoughtTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::NuhiTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
            recovered::NormalTermAtomSyntax::KeTermset(value) => {
                self.observe_leaf(value, LinkPayloadBreadth::AddedFullTerm)
            }
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_connected_normal_term(&mut self, node: &'tree recovered::ConnectedNormalTermSyntax) {
        let recovered::ConnectedNormalTermSyntax {
            leading_term,
            continuations,
        } = node;
        self.descend(leading_term);
        if continuations.is_empty() {
            self.observe(LinkPayloadBreadth::Unproven);
        }
        for continuation in continuations {
            self.descend(continuation);
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_connected_normal_term_continuation(
        &mut self,
        node: &'tree recovered::ConnectedNormalTermContinuationSyntax,
    ) {
        let recovered::ConnectedNormalTermContinuationSyntax {
            connective,
            trailing_term,
        } = node;
        self.observe_leaf(connective, LinkPayloadBreadth::LegacyLinked);
        self.descend(trailing_term);
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_bound_normal_term_connection(
        &mut self,
        node: &'tree recovered::BoundNormalTermConnectionSyntax,
    ) {
        let recovered::BoundNormalTermConnectionSyntax {
            leading_term,
            continuations,
        } = node;
        self.descend(leading_term);
        if continuations.is_empty() {
            self.observe(LinkPayloadBreadth::Unproven);
        }
        for continuation in continuations {
            self.descend(continuation);
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_normal_term_bo_continuation(
        &mut self,
        node: &'tree recovered::NormalTermBoContinuationSyntax,
    ) {
        match node {
            recovered::NormalTermBoContinuationSyntax::BoundNormalTermContinuation(value) => {
                self.descend(value)
            }
            recovered::NormalTermBoContinuationSyntax::ZantufaBoundNormalTermContinuation(
                value,
            ) => self.descend(value),
        }
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_bound_normal_term_continuation(
        &mut self,
        node: &'tree recovered::BoundNormalTermContinuationSyntax,
    ) {
        let recovered::BoundNormalTermContinuationSyntax {
            connective,
            tense_modal,
            bo,
            trailing_term,
        } = node;
        self.observe_leaf(connective, LinkPayloadBreadth::LegacyLinked);
        if let Some(tag) = tense_modal {
            self.observe_leaf(tag, LinkPayloadBreadth::LegacyLinked);
        }
        self.observe_leaf(&bo.value, LinkPayloadBreadth::LegacyLinked);
        self.descend(trailing_term);
    }

    #[requires(true)]
    #[ensures(old(self.answer) == LinkPayloadBreadth::Unproven -> self.answer == LinkPayloadBreadth::Unproven)]
    fn walk_zantufa_bound_normal_term_continuation(
        &mut self,
        node: &'tree recovered::ZantufaBoundNormalTermContinuationSyntax,
    ) {
        let recovered::ZantufaBoundNormalTermContinuationSyntax { bo, trailing_term } = node;
        self.observe_leaf(&bo.value, LinkPayloadBreadth::AddedFullTerm);
        self.descend(trailing_term);
    }
}

#[requires(true)]
#[ensures(ret != LinkPayloadBreadth::Unproven)]
fn breadth(candidate: &model::FullLinkedTermSyntax) -> LinkPayloadBreadth {
    let mut walker = BreadthWalker {
        answer: LinkPayloadBreadth::LegacyLinked,
    };
    model::TreeWalkable::walk_with(candidate.0.as_ref(), &mut walker);
    walker.answer
}

#[requires(true)]
#[ensures(matches!(candidate, recovered::Recovered::Error(_)) -> ret == LinkPayloadBreadth::Unproven)]
fn recovered_breadth(
    candidate: &recovered::Recovered<recovered::FullLinkedTermSyntax>,
) -> LinkPayloadBreadth {
    let Some(candidate) = parsed_value(candidate) else {
        return LinkPayloadBreadth::Unproven;
    };
    let mut walker = BreadthWalker {
        answer: LinkPayloadBreadth::LegacyLinked,
    };
    walker.descend(&candidate.0);
    walker.answer
}

/// A rule-level refinement: only proven new width can occupy the Full arm.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct LegacyLinkPayloadRejection;

#[contract_trait]
impl OutputRejection<model::FullLinkedTermSyntax> for LegacyLinkPayloadRejection {
    fn rejected_name(&self) -> &'static str {
        "legacy or unproven linked payload"
    }

    fn rejects(&self, candidate: &model::FullLinkedTermSyntax) -> bool {
        breadth(candidate) != LinkPayloadBreadth::AddedFullTerm
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::FullLinkedTermSyntax>>
    for LegacyLinkPayloadRejection
{
    fn rejected_name(&self) -> &'static str {
        "legacy or unproven linked payload"
    }

    fn rejects(&self, candidate: &recovered::Recovered<recovered::FullLinkedTermSyntax>) -> bool {
        let answer = recovered_breadth(candidate);
        #[cfg(test)]
        tests::record_recovered_attempt(candidate, answer);
        answer != LinkPayloadBreadth::AddedFullTerm
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::Arc;

    use bityzba::new;
    use jbotci_dialect::parse_dialect_definition;
    use jbotci_morphology::segment_words_with_modifiers;
    use vec1::vec1;

    use super::*;
    use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData};
    use crate::{ParseOptions, SyntaxParse, parse_syntax_tree_with_source_and_options};

    #[invariant(matches!(candidate, recovered::Recovered::Valid(_)))]
    #[bityzba::expensive_invariant(*answer == recovered_breadth(candidate))]
    #[derive(Debug)]
    struct RecoveredAttempt {
        candidate: recovered::Recovered<recovered::FullLinkedTermSyntax>,
        answer: LinkPayloadBreadth,
    }

    thread_local! {
        static RECOVERED_ATTEMPTS: RefCell<Option<Vec<RecoveredAttempt>>> = const { RefCell::new(None) };
    }

    // This recorder sees only actual guard calls. Final containing-field wrappers must be
    // inspected independently in the parse tree, never inferred from the attempt list.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_recovered_attempt(
        candidate: &recovered::Recovered<recovered::FullLinkedTermSyntax>,
        answer: LinkPayloadBreadth,
    ) {
        RECOVERED_ATTEMPTS.with_borrow_mut(|attempts| {
            if let Some(attempts) = attempts {
                attempts.push(new!(RecoveredAttempt {
                    candidate: candidate.clone(),
                    answer,
                }));
            }
        });
    }

    // Test helpers deliberately assert the parser result in their body: no independent
    // pre/postcondition can specify the returned grammar without repeating that same parser.
    #[requires(true)]
    #[ensures(true)]
    fn parse(source: &str, dialect: &str) -> SyntaxParse {
        let dialect = parse_dialect_definition(dialect).expect("test dialect");
        let words = segment_words_with_modifiers(source).expect("test morphology");
        parse_syntax_tree_with_source_and_options(
            &words,
            source,
            &ParseOptions::default().with_dialect_definition(&dialect),
        )
        .unwrap_or_else(|error| panic!("{source} ({dialect:?}): {error:?}"))
    }

    #[invariant(true)]
    #[derive(Default)]
    struct FullPayloads<'tree> {
        values: Vec<&'tree model::FullLinkedTermSyntax>,
    }

    impl<'tree> model::TreeWalker<'tree> for FullPayloads<'tree> {
        #[requires(true)]
        #[ensures(self.values.len() == old(self.values.len()) + 1)]
        fn walk_full_linked_term(&mut self, node: &'tree model::FullLinkedTermSyntax) {
            self.values.push(node);
        }
    }

    #[invariant(true)]
    #[derive(Default)]
    struct NormalPayloads<'tree> {
        values: Vec<&'tree model::NormalTermSyntax>,
    }

    impl<'tree> model::TreeWalker<'tree> for NormalPayloads<'tree> {
        #[requires(true)]
        #[ensures(self.values.len() == old(self.values.len()) + 1)]
        fn walk_normal_term(&mut self, node: &'tree model::NormalTermSyntax) {
            self.values.push(node);
        }
    }

    #[invariant(true)]
    #[derive(Default)]
    struct LinkParents<'tree> {
        values: Vec<&'tree model::LinkargsSyntax>,
    }

    impl<'tree> model::TreeWalker<'tree> for LinkParents<'tree> {
        #[requires(true)]
        #[ensures(self.values.len() > old(self.values.len()))]
        fn walk_linkargs(&mut self, node: &'tree model::LinkargsSyntax) {
            self.values.push(node);
            model::walk::linkargs(self, node);
        }
    }

    #[requires(true)]
    #[ensures(ret.start <= ret.end)]
    fn extent(node: &impl model::TreeNode) -> std::ops::Range<usize> {
        let mut spans = Vec::new();
        let mut collect = |span: &jbotci_source::SourceSpan| {
            spans.push(span.byte_start..span.byte_end);
        };
        let mut visitor = crate::GeneratedModelSourceSpanVisitor::<_, false> {
            visitor: &mut collect,
        };
        model::TreeNode::visit_in_order(node, &mut visitor);
        assert!(spans.windows(2).all(|pair| pair[0].end <= pair[1].start));
        spans.first().expect("nonempty test node").start
            ..spans.last().expect("nonempty test node").end
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn closed_na_ku_and_nuhi_payloads_reach_full_through_be_and_bei() {
        for dialect in ["()", "(zantufa)"] {
            for source in [
                "mi broda be na ku be'o",
                "mi broda be ko'a bei na ku be'o",
                "mi broda be nu'i ko'a ce'e ko'e nu'u be'o",
                "mi broda be ko'a bei nu'i ko'e ce'e ko'i nu'u be'o",
            ] {
                let parsed = parse(source, dialect);
                assert!(
                    parsed.warnings.is_empty(),
                    "{source}: {:?}",
                    parsed.warnings
                );
                let mut payloads = FullPayloads::default();
                model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut payloads);
                assert_eq!(payloads.values.len(), 1, "{source}");
                assert_eq!(
                    breadth(payloads.values[0]),
                    LinkPayloadBreadth::AddedFullTerm
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn old_tagged_and_plain_payloads_are_legacy_not_full() {
        for dialect in ["()", "(zantufa)"] {
            for payload in [
                "ko'a",
                "fa ko'a",
                "bau do",
                "ga'a mi",
                "pu ku",
                "ki ku",
                "fi'o broda fe'u ko'a",
            ] {
                let source = format!("mi broda be {payload} be'o");
                let parsed = parse(&source, dialect);
                let mut links = FullPayloads::default();
                model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut links);
                assert!(links.values.is_empty(), "old owner was lost: {source}");
                assert!(
                    parsed.warnings.is_empty(),
                    "{source}: {:?}",
                    parsed.warnings
                );

                // GOI supplies a normal-term slot without enabling the new link route.
                let source = format!("mi broda ko'e goi {payload} ge'u");
                let parsed = parse(&source, dialect);
                let mut terms = NormalPayloads::default();
                model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut terms);
                assert_eq!(terms.values.len(), 1, "{source}");
                let candidate = model::FullLinkedTermSyntax(Arc::new(terms.values[0].clone()));
                assert_eq!(
                    breadth(&candidate),
                    LinkPayloadBreadth::LegacyLinked,
                    "{source}"
                );
                assert!(LegacyLinkPayloadRejection.rejects(&candidate));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_new_operand_makes_a_legacy_shaped_connection_full() {
        let parsed = parse("mi broda ko'a goi bau do .e bo na ku ge'u", "()");
        let mut terms = NormalPayloads::default();
        model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut terms);
        assert_eq!(terms.values.len(), 1);
        let candidate = model::FullLinkedTermSyntax(Arc::new(terms.values[0].clone()));
        assert_eq!(breadth(&candidate), LinkPayloadBreadth::AddedFullTerm);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mixed_connections_reach_full_through_be_and_bei() {
        for dialect in ["()", "(zantufa)", "(+zantufa-terms)"] {
            for payload in [
                "bau do .e bo na ku",
                "na ku .e bo bau do",
                "bau do .e na ku",
                "na ku .e bau do",
                "bau do .e bo pu ku .e bo na ku",
                "bau do .e pu ku .e na ku",
            ] {
                // Equal token counts and padding give the payload identical token indices and
                // source coordinates. Compare the entire typed normal term and warnings, not a
                // string-stripped tree or a warning count that could hide ownership changes.
                let control_source = format!("{:24}{payload} ge'u", "mi broda ko'a goi ");
                let control = parse(&control_source, dialect);
                let mut terms = NormalPayloads::default();
                model::TreeWalkable::walk_with(control.parse_tree.as_ref(), &mut terms);
                assert_eq!(terms.values.len(), 1, "{control_source}");
                assert!(
                    !control.warnings.is_empty(),
                    "connection control must be diagnosed"
                );

                for is_bei in [false, true] {
                    let prefix = if is_bei {
                        "broda be ko'a bei "
                    } else {
                        "mi cu broda be "
                    };
                    let source = format!("{prefix:24}{payload} be'o");
                    let parsed = parse(&source, dialect);
                    let mut links = FullPayloads::default();
                    model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut links);
                    assert_eq!(links.values.len(), 1, "{source}");
                    let full = links.values[0];
                    assert_eq!(breadth(full), LinkPayloadBreadth::AddedFullTerm);
                    assert_eq!(full.0.as_ref(), terms.values[0], "{source}");
                    assert_eq!(extent(full), 24..24 + payload.len(), "{source}");
                    assert_eq!(parsed.warnings, control.warnings, "{source}");

                    let mut parents = LinkParents::default();
                    model::TreeWalkable::walk_with(parsed.parse_tree.as_ref(), &mut parents);
                    assert_eq!(parents.values.len(), 1, "{source}");
                    let parent = parents.values[0];
                    assert_eq!(parent.bei_links.len(), usize::from(is_bei), "{source}");
                    let target = if is_bei {
                        assert_eq!(extent(&parent.bei_links[0].bei), 14..17);
                        &parent.bei_links[0].link
                    } else {
                        &parent.first_link
                    };
                    let model::LinkedTermSyntax::FullLinkedTerm(target) = target.as_ref() else {
                        panic!("wrong parent link owner: {source}");
                    };
                    assert!(std::ptr::eq(target.as_ref(), full), "{source}");
                    let marker = if is_bei { 6..8 } else { 12..14 };
                    assert_eq!(extent(&parent.be), marker);
                    assert_eq!(extent(parent), marker.start..source.len(), "{source}");
                    assert_eq!(
                        extent(parent.beho.as_ref().expect("closed BEhO")),
                        source.len() - 4..source.len()
                    );
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn uncertainty_dominates_every_connection_order() {
        use LinkPayloadBreadth::{AddedFullTerm, LegacyLinked, Unproven};
        for answer in [LegacyLinked, AddedFullTerm, Unproven] {
            assert_eq!(answer.combine(Unproven), Unproven);
            assert_eq!(Unproven.combine(answer), Unproven);
        }
        assert_eq!(LegacyLinked.combine(AddedFullTerm), AddedFullTerm);
        assert_eq!(AddedFullTerm.combine(LegacyLinked), AddedFullTerm);
        assert_eq!(LegacyLinked.combine(LegacyLinked), LegacyLinked);
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing() -> crate::SyntaxRecoveryItem {
        new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 0,
            span: Arc::new(
                jbotci_diagnostics::source_span_from_byte_offsets(None, "", 0, 0).unwrap()
            ),
            expected: "ku".to_owned(),
        })
    }

    /// Synthetic classifier data, not a claimed winning recovery fixture.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_na_ku(missing_ku: bool) -> recovered::FullLinkedTermSyntax {
        let words = segment_words_with_modifiers("na ku").unwrap();
        let tokens = crate::grammar::syntax_tokens(&words, &ParseOptions::default());
        let [na, ku] = tokens.as_slice() else {
            panic!("two syntax tokens")
        };
        recovered::FullLinkedTermSyntax(Arc::new(recovered::Recovered::valid(
            recovered::NormalTermSyntax::NaKuTerm(Arc::new(recovered::Recovered::valid(
                recovered::NaKuTermSyntax {
                    na: recovered::Recovered::valid(na.clone()),
                    na_ku: recovered::WithFreeModifiers {
                        value: if missing_ku {
                            recovered::Recovered::Error(missing())
                        } else {
                            recovered::Recovered::valid(ku.clone())
                        },
                        free_modifiers: Vec::new(),
                    },
                },
            ))),
        )))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_and_prefix_values_need_their_own_complete_markers() {
        for missing_ku in [false, true] {
            let answer = if missing_ku {
                LinkPayloadBreadth::Unproven
            } else {
                LinkPayloadBreadth::AddedFullTerm
            };
            let valid = recovered::Recovered::valid(recovered_na_ku(missing_ku));
            let prefix = recovered::Recovered::Prefix(jbotci_tree::RecoveredPrefix {
                errors: vec1![missing()],
                value: Box::new(recovered_na_ku(missing_ku)),
            });
            assert_eq!(recovered_breadth(&valid), answer);
            assert_eq!(recovered_breadth(&prefix), answer);
            assert_eq!(LegacyLinkPayloadRejection.rejects(&valid), missing_ku);
            assert_eq!(LegacyLinkPayloadRejection.rejects(&prefix), missing_ku);
        }
        let error = recovered::Recovered::Error(missing());
        assert_eq!(recovered_breadth(&error), LinkPayloadBreadth::Unproven);
        assert!(LegacyLinkPayloadRejection.rejects(&error));
    }

    #[invariant(true)]
    #[derive(Default)]
    struct ParsedTokens<'tree> {
        tokens: Vec<&'tree crate::tree::Token>,
    }

    impl<'tree> TreeVisitor<'tree> for ParsedTokens<'tree> {
        type Node = recovered::NodeRef<'tree>;
        type Atom = recovered::AtomRef<'tree>;

        #[requires(true)]
        #[ensures(self.tokens.len() == old(self.tokens.len()) + 1)]
        fn visit_atom(&mut self, atom: Self::Atom) {
            let recovered::AtomRef::Token(token) = atom;
            self.tokens.push(token);
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|extent| extent.start < extent.end))]
    fn recovered_extent(node: &impl recovered::TreeNode) -> Option<std::ops::Range<usize>> {
        let mut tokens = ParsedTokens::default();
        recovered::TreeNode::visit_in_order(node, &mut tokens);
        tokens
            .tokens
            .iter()
            .flat_map(|token| token.source_spans())
            .fold(None, |extent: Option<std::ops::Range<usize>>, span| {
                Some(extent.map_or(span.byte_start..span.byte_end, |extent| {
                    extent.start.min(span.byte_start)..extent.end.max(span.byte_end)
                }))
            })
    }

    #[invariant(true)]
    struct RecoveredTarget<'tree> {
        marker: &'tree recovered::Recovered<crate::tree::Token>,
        field: &'tree recovered::Recovered<recovered::LinkedTermSyntax>,
    }

    #[invariant(true)]
    #[derive(Default)]
    struct RecoveredTargets<'tree> {
        values: Vec<RecoveredTarget<'tree>>,
    }

    impl<'tree> recovered::TreeWalker<'tree> for RecoveredTargets<'tree> {
        #[requires(true)]
        #[ensures(self.values.len() > old(self.values.len()))]
        fn walk_linkargs(&mut self, node: &'tree recovered::LinkargsSyntax) {
            self.values.push(RecoveredTarget {
                marker: &node.be.value,
                field: &node.first_link,
            });
            recovered::walk::linkargs(self, node);
        }

        #[requires(true)]
        #[ensures(self.values.len() > old(self.values.len()))]
        fn walk_bei_link(&mut self, node: &'tree recovered::BeiLinkSyntax) {
            self.values.push(RecoveredTarget {
                marker: &node.bei.value,
                field: &node.link,
            });
            recovered::walk::bei_link(self, node);
        }
    }

    #[invariant(true)]
    struct AttemptedParse {
        parsed: crate::RecoveredSyntaxParse,
        attempts: Vec<RecoveredAttempt>,
    }

    #[requires(true)]
    #[ensures(true)]
    fn parse_with_attempts(source: &str, dialect: &str) -> AttemptedParse {
        let definition = parse_dialect_definition(dialect).unwrap();
        let options = ParseOptions::default().with_dialect_definition(&definition);
        let words = segment_words_with_modifiers(source).unwrap();
        RECOVERED_ATTEMPTS.set(Some(Vec::new()));
        let parsed =
            crate::parse_syntax_tree_recovered_with_source_and_options(&words, source, &options);
        let attempts = RECOVERED_ATTEMPTS.take().unwrap();
        assert!(
            attempts
                .iter()
                .all(|attempt| matches!(attempt.candidate, recovered::Recovered::Valid(_)))
        );
        AttemptedParse { parsed, attempts }
    }

    #[requires(true)]
    #[ensures(true)]
    fn child_breadth<'tree, T: recovered::TreeWalkable<'tree>>(
        node: &'tree recovered::Recovered<T>,
    ) -> LinkPayloadBreadth {
        let mut walker = BreadthWalker {
            answer: LinkPayloadBreadth::LegacyLinked,
        };
        walker.descend(node);
        walker.answer
    }

    /// Real parses of the LR fixtures, including independent damage after the mixed payload.
    /// The full fixture oracle additionally pins raw tree, diagnostics, tokens and items.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complete_uncertain_connections_reject_in_both_orders_and_link_positions() {
        for dialect in ["()", "(+zantufa-terms)", "(zantufa)"] {
            for is_bei in [false, true] {
                for reverse in [false, true] {
                    for field_damage in [false, true] {
                        let prefix = if is_bei {
                            "mi broda be ko'a bei "
                        } else {
                            "mi broda be "
                        };
                        let payload = if reverse {
                            "lu mi ku li'u .e bo na ku"
                        } else {
                            "na ku .e bo lu mi ku li'u"
                        };
                        let damage = if field_damage { " ku" } else { "" };
                        let source = format!("{prefix}{payload}{damage} be'o .i mi ku .i do broda");
                        let result = parse_with_attempts(&source, dialect);
                        let start = prefix.len();
                        let complete = start..start + payload.len();
                        let mut complete_attempts = 0;
                        for attempt in &result.attempts {
                            if recovered_extent(&attempt.candidate) != Some(complete.clone()) {
                                continue;
                            }
                            let candidate = parsed_value(&attempt.candidate).unwrap();
                            let recovered::NormalTermSyntax::BoundNormalTermConnection(connection) =
                                parsed_value(&candidate.0).unwrap()
                            else {
                                continue;
                            };
                            let connection = parsed_value(connection).unwrap();
                            assert_eq!(connection.continuations.len(), 1, "{source}");
                            let recovered::NormalTermBoContinuationSyntax::BoundNormalTermContinuation(continuation) = parsed_value(&connection.continuations[0]).unwrap() else { panic!("explicit connective BO: {source}"); };
                            let continuation = parsed_value(continuation).unwrap();
                            let answers = [
                                child_breadth(&connection.leading_term),
                                child_breadth(&continuation.trailing_term),
                            ];
                            let expected = if reverse {
                                [
                                    LinkPayloadBreadth::Unproven,
                                    LinkPayloadBreadth::AddedFullTerm,
                                ]
                            } else {
                                [
                                    LinkPayloadBreadth::AddedFullTerm,
                                    LinkPayloadBreadth::Unproven,
                                ]
                            };
                            assert_eq!(answers, expected, "{source}");
                            assert_eq!(attempt.answer, LinkPayloadBreadth::Unproven, "{source}");
                            // Calling the predicate with recording disabled proves the actual
                            // observed answer is a rejection, not a classifier trace label.
                            assert!(
                                LegacyLinkPayloadRejection.rejects(&attempt.candidate),
                                "{source}"
                            );
                            complete_attempts += 1;
                        }
                        assert!(
                            complete_attempts > 0,
                            "no complete mixed candidate: {source} {dialect}"
                        );
                        let mut targets = RecoveredTargets::default();
                        recovered::TreeWalkable::walk_with(
                            result.parsed.parse_tree.as_ref(),
                            &mut targets,
                        );
                        let marker = if is_bei { 17..20 } else { 9..11 };
                        let targets: Vec<_> = targets
                            .values
                            .iter()
                            .filter(|target| {
                                recovered_extent(target.marker) == Some(marker.clone())
                            })
                            .collect();
                        assert_eq!(targets.len(), 1, "{source}");
                        let field = targets[0].field;
                        assert_eq!(
                            matches!(field, recovered::Recovered::Prefix(_)),
                            !reverse && !is_bei,
                            "{source}"
                        );
                        let value = parsed_value(field).expect("actual surviving field");
                        if reverse {
                            assert!(
                                matches!(value, recovered::LinkedTermSyntax::PlainLinkedSumti(_)),
                                "{source}"
                            );
                            assert_eq!(
                                recovered_extent(value),
                                Some(start..start + 13),
                                "{source}"
                            );
                            assert!(result.parsed.warnings.is_empty(), "{source}");
                        } else {
                            let recovered::LinkedTermSyntax::FullLinkedTerm(full) = value else {
                                panic!("shorter Full: {source}");
                            };
                            assert_eq!(
                                recovered_breadth(full),
                                LinkPayloadBreadth::AddedFullTerm,
                                "{source}"
                            );
                            assert_eq!(
                                recovered_extent(value),
                                Some(start..start + 17),
                                "{source}"
                            );
                            assert_ne!(
                                recovered_extent(value),
                                Some(complete),
                                "shorter Full is not the rejected complete candidate"
                            );
                            assert_eq!(result.parsed.warnings.len(), 1, "{source}");
                            assert_eq!(
                                result.parsed.warnings[0].kind,
                                crate::ExperimentalConstruct::ExperimentalTermBoConnection,
                                "{source}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn actual_full_legacy_and_abandoned_fields_have_separate_attempt_evidence() {
        #[invariant(marker.start < marker.end && marker.end <= payload.start && payload.start < payload.end)]
        struct Case {
            source: &'static str,
            marker: std::ops::Range<usize>,
            payload: std::ops::Range<usize>,
            full: bool,
            prefix: bool,
        }
        let cases = [
            new!(Case {
                source: "mi broda be na ku be'o .i mi ku .i do broda",
                marker: 9..11,
                payload: 12..17,
                full: true,
                prefix: false
            }),
            new!(Case {
                source: "mi broda be na ku ku be'o",
                marker: 9..11,
                payload: 12..17,
                full: true,
                prefix: true
            }),
            new!(Case {
                source: "mi broda be ko'a ku be'o .i mi ku .i do broda",
                marker: 9..11,
                payload: 12..16,
                full: false,
                prefix: true
            }),
            new!(Case {
                source: "mi broda be ko'a bei na ku ku be'o",
                marker: 17..20,
                payload: 21..26,
                full: true,
                prefix: false
            }),
            new!(Case {
                source: "mi broda be ko'a bei ko'e ku be'o",
                marker: 17..20,
                payload: 21..25,
                full: false,
                prefix: false
            }),
        ];
        for dialect in ["()", "(+zantufa-terms)", "(zantufa)"] {
            for case in &cases {
                let result = parse_with_attempts(case.source, dialect);
                assert!(!result.parsed.errors.is_empty());
                assert!(result.parsed.warnings.is_empty());
                let expected = if case.full {
                    LinkPayloadBreadth::AddedFullTerm
                } else {
                    LinkPayloadBreadth::LegacyLinked
                };
                let matching: Vec<_> = result
                    .attempts
                    .iter()
                    .filter(|attempt| {
                        recovered_extent(&attempt.candidate) == Some(case.payload.clone())
                    })
                    .collect();
                assert!(
                    !matching.is_empty(),
                    "no complete candidate: {} {dialect}",
                    case.source
                );
                for attempt in matching {
                    assert_eq!(attempt.answer, expected);
                    assert_eq!(
                        LegacyLinkPayloadRejection.rejects(&attempt.candidate),
                        !case.full
                    );
                }
                let mut targets = RecoveredTargets::default();
                recovered::TreeWalkable::walk_with(result.parsed.parse_tree.as_ref(), &mut targets);
                let matching: Vec<_> = targets
                    .values
                    .iter()
                    .filter(|target| recovered_extent(target.marker) == Some(case.marker.clone()))
                    .collect();
                assert_eq!(matching.len(), 1);
                let field = matching[0].field;
                assert_eq!(
                    matches!(field, recovered::Recovered::Prefix(_)),
                    case.prefix
                );
                let value = parsed_value(field).expect("real final field value");
                assert_eq!(recovered_extent(value), Some(case.payload.clone()));
                if case.full {
                    assert!(matches!(
                        value,
                        recovered::LinkedTermSyntax::FullLinkedTerm(_)
                    ));
                } else {
                    assert!(matches!(
                        value,
                        recovered::LinkedTermSyntax::PlainLinkedSumti(_)
                    ));
                }
            }
        }
        // Built-in Zantufa gives the KU a real old owner instead of this Error field, so it
        // is not labeled as an Error witness. On these two axes the guard is never invoked.
        for dialect in ["()", "(+zantufa-terms)"] {
            let result =
                parse_with_attempts("mi broda be ku ko'a be'o .i mi ku .i do broda", dialect);
            assert!(
                result.attempts.is_empty(),
                "no invocation fabricated for an abandoned field"
            );
            assert!(!result.parsed.errors.is_empty());
            assert!(result.parsed.warnings.is_empty());
            let mut targets = RecoveredTargets::default();
            recovered::TreeWalkable::walk_with(result.parsed.parse_tree.as_ref(), &mut targets);
            assert_eq!(targets.values.len(), 1);
            assert_eq!(recovered_extent(targets.values[0].marker), Some(9..11));
            assert!(matches!(
                targets.values[0].field,
                recovered::Recovered::Error(_)
            ));
            assert_eq!(recovered_extent(targets.values[0].field), None);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complete_legacy_connections_reject_full_before_field_recovery() {
        for dialect in ["()", "(+zantufa-terms)", "(zantufa)"] {
            for is_bei in [false, true] {
                for bound in [false, true] {
                    let prefix = if is_bei {
                        "mi broda be ko'i bei "
                    } else {
                        "mi broda be "
                    };
                    let payload = if bound {
                        "bau ko'a .e bo pu ko'e"
                    } else {
                        "bau ko'a .e pu ko'e"
                    };
                    let source = format!("{prefix}{payload} ku be'o .i mi ku .i do broda");
                    let result = parse_with_attempts(&source, dialect);
                    let extent = prefix.len()..prefix.len() + payload.len();
                    let attempts: Vec<_> = result
                        .attempts
                        .iter()
                        .filter(|attempt| {
                            recovered_extent(&attempt.candidate) == Some(extent.clone())
                        })
                        .collect();
                    assert!(
                        !attempts.is_empty(),
                        "no complete legacy candidate: {source} {dialect}"
                    );
                    for attempt in attempts {
                        assert_eq!(attempt.answer, LinkPayloadBreadth::LegacyLinked, "{source}");
                        assert!(LegacyLinkPayloadRejection.rejects(&attempt.candidate));
                    }
                    let mut targets = RecoveredTargets::default();
                    recovered::TreeWalkable::walk_with(
                        result.parsed.parse_tree.as_ref(),
                        &mut targets,
                    );
                    let marker = if is_bei { 17..20 } else { 9..11 };
                    let matching: Vec<_> = targets
                        .values
                        .iter()
                        .filter(|target| recovered_extent(target.marker) == Some(marker.clone()))
                        .collect();
                    assert_eq!(matching.len(), 1);
                    assert_eq!(
                        matches!(matching[0].field, recovered::Recovered::Prefix(_)),
                        !is_bei
                    );
                    let value = parsed_value(matching[0].field).expect("old connection survives");
                    assert_eq!(recovered_extent(value), Some(extent));
                    if bound {
                        assert!(matches!(
                            value,
                            recovered::LinkedTermSyntax::BoundLinkedTermConnection(_)
                        ));
                        assert_eq!(result.parsed.warnings.len(), 1);
                        assert_eq!(
                            result.parsed.warnings[0].kind,
                            crate::ExperimentalConstruct::ExperimentalTermBoConnection
                        );
                    } else {
                        assert!(matches!(
                            value,
                            recovered::LinkedTermSyntax::ConnectedLinkedTerm(_)
                        ));
                        assert!(result.parsed.warnings.is_empty());
                    }
                }
            }
        }
    }
}
