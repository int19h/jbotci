//! Typed refusal of a mex that holds a forethought operator expression without PEhO.
//!
//! jbotci's mex keeps CLL's forethought call with an optional PEhO
//! (`[PEhO #] operator mex-2 ... /KUhE#/`), the retained standard MEX design (I12). camxes-exp's
//! mex has no forethought form without PEhO anywhere: its `mex_2` has only
//! `PEhO_clause free* operator mex+ KUhE_elidible free*` (camxes-exp.peg:282). The raw-mex
//! quantifier under the `mex-quantifier` dialect feature exists for camxes-exp fidelity, so it
//! must not read a mex that camxes-exp cannot read: otherwise it makes reading changes that
//! camxes-exp does not make, as in CLL c18e18d2 and camxes corpus 2622 (#982).
//!
//! The classifier walks the completed mex with the generated walker. It looks at every
//! forethought call in the mex's own structure: operands, operators, precedence tails,
//! reverse Polish parts, JOhI arrays, VEI groups, MAhO operator bodies, and the operands that
//! NAhE and LAhE qualify (camxes-exp reads those as `mex` too). It stops at a nested sumti
//! (MOhE), a nested selbri (NIhE, NAhU, a forethought connective's selbri), a tag and a free
//! modifier, because those keep jbotci's own language.

use bityzba::{contract_trait, invariant, requires};

use super::generated_model::{self as model, recovered};
use super::generated_runtime::OutputRejection;

/// Generated-walker pass that records whether a mex holds a forethought call without PEhO.
#[invariant(true)]
struct PehoLessForethoughtFinder {
    found: bool,
}

impl<'tree> model::TreeWalker<'tree> for PehoLessForethoughtFinder {
    #[requires(true)]
    #[ensures(old(self.found) -> self.found)]
    fn walk_forethought_call_mekso(&mut self, node: &'tree model::ForethoughtCallMeksoSyntax) {
        // Destructure exhaustively, so that a new field is a compile error here.
        let model::ForethoughtCallMeksoSyntax {
            peho,
            operator: _,
            operands: _,
            kuhe: _,
        } = node;
        if peho.is_none() {
            self.found = true;
        } else {
            model::walk::forethought_call_mekso(self, node);
        }
    }

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_sumti(&mut self, _node: &'tree model::SumtiSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_selbri(&mut self, _node: &'tree model::SelbriSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_tense_modal(&mut self, _node: &'tree model::TenseModalSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_free_modifier(&mut self, _node: &'tree model::FreeModifierSyntax) {}
}

impl<'tree> recovered::TreeWalker<'tree> for PehoLessForethoughtFinder {
    #[requires(true)]
    #[ensures(old(self.found) -> self.found)]
    fn walk_forethought_call_mekso(&mut self, node: &'tree recovered::ForethoughtCallMeksoSyntax) {
        // Destructure exhaustively, so that a new field is a compile error here.
        let recovered::ForethoughtCallMeksoSyntax {
            peho,
            operator: _,
            operands: _,
            kuhe: _,
        } = node;
        if peho.is_none() {
            self.found = true;
        } else {
            recovered::walk::forethought_call_mekso(self, node);
        }
    }

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_sumti(&mut self, _node: &'tree recovered::SumtiSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_selbri(&mut self, _node: &'tree recovered::SelbriSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_tense_modal(&mut self, _node: &'tree recovered::TenseModalSyntax) {}

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_free_modifier(&mut self, _node: &'tree recovered::FreeModifierSyntax) {}
}

/// Grammar-level refinement that refuses a mex holding a forethought call without PEhO, for
/// the constructs that read camxes-exp's mex language: the raw-mex quantifier, and the
/// `mex_2` of a subscript and of an utterance ordinal.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct PehoLessForethoughtRejection;

const PEHO_LESS_FORETHOUGHT_REJECTION_NAME: &str = "forethought mex without PEhO";

/// Implements the refinement for a strict mex node type and its recovered counterpart. The
/// recovered walk descends through every slot that parsed, so a recovery item elsewhere in
/// the mex does not hide a forethought call without PEhO.
macro_rules! peho_less_forethought_rejection {
    ($($node:ident),+ $(,)?) => {$(
        #[contract_trait]
        impl OutputRejection<model::$node> for PehoLessForethoughtRejection {
            fn rejected_name(&self) -> &'static str {
                PEHO_LESS_FORETHOUGHT_REJECTION_NAME
            }

            fn rejects(&self, value: &model::$node) -> bool {
                let mut finder = PehoLessForethoughtFinder { found: false };
                model::TreeWalkable::walk_with(value, &mut finder);
                finder.found
            }
        }

        #[contract_trait]
        impl OutputRejection<recovered::Recovered<recovered::$node>>
            for PehoLessForethoughtRejection
        {
            fn rejected_name(&self) -> &'static str {
                PEHO_LESS_FORETHOUGHT_REJECTION_NAME
            }

            fn rejects(&self, value: &recovered::Recovered<recovered::$node>) -> bool {
                let mut finder = PehoLessForethoughtFinder { found: false };
                recovered::walk::recovered(&mut finder, value);
                finder.found
            }
        }
    )+};
}

peho_less_forethought_rejection!(MeksoSyntax, ExpMex2Syntax);
