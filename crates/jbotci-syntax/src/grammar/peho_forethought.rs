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

use super::generated_model::view::{self, AsView, Walkable};
use super::generated_runtime::OutputRejection;

/// Only these three grammar roots use the reference mex restrictions.
#[contract_trait]
trait ReferenceMexRoot<'tree>: Walkable<'tree> {}
#[contract_trait]
impl<'tree> ReferenceMexRoot<'tree> for view::MeksoSyntaxRef<'tree> {}
#[contract_trait]
impl<'tree> ReferenceMexRoot<'tree> for view::ExpMexSyntaxRef<'tree> {}
#[contract_trait]
impl<'tree> ReferenceMexRoot<'tree> for view::ExpMex2SyntaxRef<'tree> {}

/// Find missing PEhO and arrays through the generated view walker.
#[invariant(true)]
struct ReferenceMexFinder {
    found: bool,
    array_found: bool,
}

#[contract_trait]
impl<'tree> view::Walker<'tree> for ReferenceMexFinder {
    #[requires(true)]
    #[ensures(self.array_found)]
    #[ensures(old(self.found) -> self.found)]
    fn walk_array_mekso_operand(&mut self, node: view::ArrayMeksoOperandSyntaxRef<'tree>) {
        self.array_found = true;
        view::walk::array_mekso_operand(self, node);
    }

    #[requires(true)]
    #[ensures(old(self.found) -> self.found)]
    fn walk_forethought_call_mekso(&mut self, node: view::ForethoughtCallMeksoSyntaxRef<'tree>) {
        let view::ForethoughtCallMeksoSyntaxView {
            peho,
            operator: _,
            operands: _,
            kuhe: _,
        } = node.get();
        if peho.is_none() {
            self.found = true;
        }
        view::walk::forethought_call_mekso(self, node);
    }

    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_sumti(&mut self, _node: view::SumtiSyntaxRef<'tree>) {}
    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_selbri(&mut self, _node: view::SelbriSyntaxRef<'tree>) {}
    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_tense_modal(&mut self, _node: view::TenseModalSyntaxRef<'tree>) {}
    #[requires(true)]
    #[ensures(self.found == old(self.found))]
    fn walk_free_modifier(&mut self, _node: view::FreeModifierSyntaxRef<'tree>) {}
}

#[requires(true)]
#[ensures(true)]
fn reference_mex_findings<'tree>(value: impl Walkable<'tree>) -> ReferenceMexFinder {
    let mut finder = ReferenceMexFinder {
        found: false,
        array_found: false,
    };
    value.walk_with(&mut finder);
    finder
}

/// Refuse a forethought call without PEhO in the reference mex structure.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct PehoLessForethoughtRejection;

#[contract_trait]
impl<T> OutputRejection<T> for PehoLessForethoughtRejection
where
    for<'tree> T: AsView<'tree>,
    for<'tree> <T as AsView<'tree>>::Ref: ReferenceMexRoot<'tree>,
{
    fn rejected_name(&self) -> &'static str {
        "forethought mex without PEhO"
    }
    fn rejects(&self, value: &T) -> bool {
        reference_mex_findings(value.as_view()).found
    }
}

/// Refuse an array in the reference mex structure.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct CamxesArrayRejection;

#[contract_trait]
impl<T> OutputRejection<T> for CamxesArrayRejection
where
    for<'tree> T: AsView<'tree>,
    for<'tree> <T as AsView<'tree>>::Ref: ReferenceMexRoot<'tree>,
{
    fn rejected_name(&self) -> &'static str {
        "JOhI array outside the reference operand tier"
    }
    fn rejects(&self, value: &T) -> bool {
        reference_mex_findings(value.as_view()).array_found
    }
}
