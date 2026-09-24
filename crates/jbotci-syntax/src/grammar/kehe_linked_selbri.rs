//! Recovery eligibility for the Zantufa KEhE-linked selbri owner (#834).
//!
//! The strict owner is ordinary grammar: `!KE`, a level-2 selbri, a required `ke'e` and linked
//! arguments decide it, with no classifier. Recovery can synthesize any of those, so a recovered
//! candidate must not claim the construct -- or report its warning -- on a KEhE, BE or link payload
//! it did not parse. `broda be ko'a` with a synthesized `ke'e` is baseline tanru-unit linkargs, not
//! this owner. That is exactly the recovered-only policy `reject_recovered_output` exists for.

use bityzba::{invariant, requires};

use super::generated_model::recovered;
use super::generated_runtime::{RecoveredOutputRejection, carries_recovery_uncertainty};

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeheLinkedRecoveredRejection;

/// A recovered wrapper that is not `Valid` did not complete, so it cannot prove its markers; one
/// that is `Valid` is judged by `inner`.
#[requires(true)]
#[bityzba::ensures(true)]
fn wrapper_uncertain<T>(value: &recovered::Recovered<T>, inner: impl FnOnce(&T) -> bool) -> bool {
    match value {
        recovered::Recovered::Valid(value) => inner(value.as_ref()),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => true,
    }
}

macro_rules! kehe_linked_rejection {
    ($product:ident) => {
        #[bityzba::contract_trait]
        impl RecoveredOutputRejection<recovered::$product> for KeheLinkedRecoveredRejection {
            fn rejected_name(&self) -> &'static str {
                "unproven Zantufa KEhE-linked selbri"
            }
            fn rejects_uncertain(&self, value: &recovered::$product) -> bool {
                carries_recovery_uncertainty(&value.kehe)
                    || carries_recovery_uncertainty(value.linkargs.as_ref())
            }
        }

        #[bityzba::contract_trait]
        impl RecoveredOutputRejection<recovered::Recovered<recovered::$product>>
            for KeheLinkedRecoveredRejection
        {
            fn rejected_name(&self) -> &'static str {
                "unproven Zantufa KEhE-linked selbri"
            }
            fn rejects_uncertain(&self, value: &recovered::Recovered<recovered::$product>) -> bool {
                wrapper_uncertain(value, |value| {
                    RecoveredOutputRejection::<recovered::$product>::rejects_uncertain(self, value)
                })
            }
        }
    };
}
kehe_linked_rejection!(ZantufaKeheLinkedSelbriSyntax);
kehe_linked_rejection!(ZantufaKeheLinkedSelbriWithoutTerminalRelativeSyntax);

// The owner's first reach is from `zantufa_selbri_entry`, ahead of the atom priority arm, where it
// must yield a whole selbri. Its model position is `untagged_selbri`'s variant, so it maps there.
impl From<super::generated_model::ZantufaKeheLinkedSelbriSyntax>
    for super::generated_model::SelbriSyntax
{
    #[requires(true)]
    #[bityzba::ensures(true)]
    fn from(value: super::generated_model::ZantufaKeheLinkedSelbriSyntax) -> Self {
        Self::UntaggedSelbri(std::sync::Arc::new(
            super::generated_model::UntaggedSelbriSyntax::ZantufaKeheLinkedSelbri(
                std::sync::Arc::new(value),
            ),
        ))
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::SelbriSyntax>>
    for recovered::Recovered<recovered::ZantufaKeheLinkedSelbriSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::SelbriSyntax> {
        recovered::Recovered::valid(recovered::SelbriSyntax::UntaggedSelbri(
            std::sync::Arc::new(recovered::Recovered::valid(
                recovered::UntaggedSelbriSyntax::ZantufaKeheLinkedSelbri(std::sync::Arc::new(self)),
            )),
        ))
    }
}
