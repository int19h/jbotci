//! Baseline-ownership classification for extension-first selbri candidates.
//!
//! The extension parser must run first because a locally successful standard
//! CEI prefix can otherwise hide a later Zantufa-only assignment. Once the
//! whole candidate is available, this module recognizes whether the exact
//! token extent also has the pre-C4 tree shape. The proof is deliberately
//! candidate-wide rather than an operand-width heuristic: `broda cei brode
//! brodi` has a wide Zantufa operand, but its whole extent is the standard CEI
//! unit followed by ordinary adjacency.
//!
//! Every candidate product is destructured without `..`. Descendant coverage
//! uses the generated in-order traversal, so newly generated fields cannot be
//! omitted from the old-shape check. A candidate is old-shaped precisely when
//! its leading level-2 tree and every full operand contain neither of C4's new
//! additive nodes, and every operand begins with an untagged level-2 selbri.
//! Splitting the first linked unit from such an operand and leaving its existing
//! standard continuations reconstructs the baseline parse over the identical
//! extent. Tagged and NA-led operands cannot supply the linked unit immediately
//! after CEI and therefore remain extension-owned.

use bityzba::{contract_trait, invariant, requires};
use jbotci_tree::TreeVisitor;

use super::generated_model::{
    NodeRef, SelbriSyntax, UntaggedSelbriSyntax, ZantufaAssignedSelbriSyntax,
    ZantufaSelbriAssignmentSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Default)]
struct C4NodeVisitor {
    found: bool,
}

#[invariant(true)]
#[derive(Debug, Default)]
struct RecoveredC4NodeVisitor {
    found: bool,
}

impl<'tree> TreeVisitor<'tree> for RecoveredC4NodeVisitor {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if matches!(
            node,
            recovered::NodeRef::ZantufaAssignedSelbriSyntax(_)
                | recovered::NodeRef::ZantufaKeCoGroupedTanruUnitSyntax(_)
        ) {
            self.found = true;
        }
    }
}

impl<'tree> TreeVisitor<'tree> for C4NodeVisitor {
    type Node = NodeRef<'tree>;
    type Atom = super::generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if matches!(
            node,
            NodeRef::ZantufaAssignedSelbriSyntax(_) | NodeRef::ZantufaKeCoGroupedTanruUnitSyntax(_)
        ) {
            self.found = true;
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn has_c4_node(tree: &impl super::generated_model::TreeNode) -> bool {
    let mut visitor = C4NodeVisitor::default();
    super::generated_model::TreeNode::visit_in_order(tree, &mut visitor);
    visitor.found
}

#[requires(true)]
#[ensures(true)]
fn recovered_has_c4_node(tree: &impl recovered::TreeNode) -> bool {
    let mut visitor = RecoveredC4NodeVisitor::default();
    recovered::TreeNode::visit_in_order(tree, &mut visitor);
    visitor.found
}

#[requires(true)]
#[ensures(true)]
fn operand_starts_with_old_unit(selbri: &SelbriSyntax) -> bool {
    match selbri {
        SelbriSyntax::ZantufaPriorityAssignedSelbri(_) | SelbriSyntax::TaggedSelbri(_) => false,
        SelbriSyntax::UntaggedSelbri(selbri) => match selbri {
            UntaggedSelbriSyntax::NegatedSelbri(_) => false,
            UntaggedSelbriSyntax::CoSelbri(selbri) => !has_c4_node(selbri),
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_assignment(candidate: &ZantufaAssignedSelbriSyntax) -> bool {
    let ZantufaAssignedSelbriSyntax {
        leading_selbri,
        assignments,
    } = candidate;
    !has_c4_node(leading_selbri.as_ref())
        && assignments.iter().all(|assignment| {
            let ZantufaSelbriAssignmentSyntax { cei: _, selbri } = assignment;
            operand_starts_with_old_unit(selbri.as_ref())
        })
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineSelbriAssignmentRejection;

#[contract_trait]
impl OutputRejection<ZantufaAssignedSelbriSyntax> for BaselineSelbriAssignmentRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline selbri assignment surface"
    }

    fn rejects(&self, value: &ZantufaAssignedSelbriSyntax) -> bool {
        is_baseline_assignment(value)
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
#[ensures(true)]
fn recovered_operand_starts_with_old_unit(selbri: &recovered::SelbriSyntax) -> bool {
    match selbri {
        recovered::SelbriSyntax::ZantufaPriorityAssignedSelbri(_)
        | recovered::SelbriSyntax::TaggedSelbri(_) => false,
        recovered::SelbriSyntax::UntaggedSelbri(selbri) => {
            valid(selbri).is_some_and(|selbri| match selbri {
                recovered::UntaggedSelbriSyntax::NegatedSelbri(_) => false,
                recovered::UntaggedSelbriSyntax::CoSelbri(selbri) => {
                    valid(selbri).is_some_and(|selbri| !recovered_has_c4_node(selbri))
                }
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_assignment(candidate: &recovered::ZantufaAssignedSelbriSyntax) -> bool {
    let recovered::ZantufaAssignedSelbriSyntax {
        leading_selbri,
        assignments,
    } = candidate;
    valid(leading_selbri).is_some_and(|selbri| !recovered_has_c4_node(selbri))
        && assignments.iter().all(|assignment| {
            valid(assignment).is_some_and(|assignment| {
                let recovered::ZantufaSelbriAssignmentSyntax { cei: _, selbri } = assignment;
                valid(selbri).is_some_and(recovered_operand_starts_with_old_unit)
            })
        })
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaAssignedSelbriSyntax>>
    for BaselineSelbriAssignmentRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline selbri assignment surface"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaAssignedSelbriSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_baseline_assignment)
    }
}
