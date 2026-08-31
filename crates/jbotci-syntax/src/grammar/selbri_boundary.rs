//! Lossless normalization of description-boundary selbri parser results.
//!
//! The restricted grammar entry exists only to keep the terminal Zantufa
//! relative slot out of description consumers. Its successful trees contain
//! the same ordinary selbri constructs, so consumers should not need a second
//! syntax domain. These conversions erase only the internal boundary nodes.

use std::sync::Arc;

use bityzba::{contract_trait, requires};

use super::generated_model::{
    LinkedTanruUnitSyntax, NegatedSelbriSyntax, NegatedSelbriWithoutTerminalRelativeSyntax,
    SelbriSyntax, SelbriWithoutTerminalRelativeSyntax, TaggedSelbriSyntax,
    TaggedSelbriWithoutTerminalRelativeSyntax, TanruUnitSyntax, UntaggedSelbriSyntax,
    UntaggedSelbriWithoutTerminalRelativeSyntax, ZantufaAssignedSelbriSyntax,
    ZantufaAssignedSelbriWithoutTerminalRelativeSyntax, ZantufaPriorityAssignedSelbriSyntax,
    ZantufaSelbriAssignmentSyntax, ZantufaSelbriAssignmentWithoutTerminalRelativeSyntax, recovered,
};
use super::generated_runtime::{GrammarMapTo, RecoveredSyntaxSlot};

impl From<LinkedTanruUnitSyntax> for TanruUnitSyntax {
    #[requires(true)]
    #[ensures(ret.assignments.is_empty())]
    fn from(base: LinkedTanruUnitSyntax) -> Self {
        Self {
            base: Arc::new(base),
            assignments: Vec::new(),
        }
    }
}

impl From<recovered::Recovered<recovered::LinkedTanruUnitSyntax>> for recovered::TanruUnitSyntax {
    #[requires(true)]
    #[ensures(ret.assignments.is_empty())]
    fn from(base: recovered::Recovered<recovered::LinkedTanruUnitSyntax>) -> Self {
        Self {
            base: Arc::new(base),
            assignments: Vec::new(),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn assigned_into_full(
    value: ZantufaAssignedSelbriWithoutTerminalRelativeSyntax,
) -> ZantufaAssignedSelbriSyntax {
    let ZantufaAssignedSelbriWithoutTerminalRelativeSyntax {
        leading_selbri,
        preceding_assignments,
        final_assignment,
    } = value;
    let ZantufaSelbriAssignmentWithoutTerminalRelativeSyntax { cei, selbri } = final_assignment;
    let mut assignments = preceding_assignments;
    assignments.push(ZantufaSelbriAssignmentSyntax {
        cei,
        selbri: Arc::new(Arc::unwrap_or_clone(selbri).into()),
    });
    ZantufaAssignedSelbriSyntax {
        leading_selbri,
        assignments: vec1::Vec1::try_from_vec(assignments)
            .expect("the final assignment makes the normalized sequence non-empty"),
    }
}

impl From<SelbriWithoutTerminalRelativeSyntax> for SelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: SelbriWithoutTerminalRelativeSyntax) -> Self {
        match value {
            SelbriWithoutTerminalRelativeSyntax::ZantufaPriorityAssignedSelbriWithoutTerminalRelative(
                value,
            ) => {
                let value = Arc::unwrap_or_clone(value.0);
                Self::ZantufaPriorityAssignedSelbri(ZantufaPriorityAssignedSelbriSyntax(Arc::new(
                    assigned_into_full(value),
                )))
            }
            SelbriWithoutTerminalRelativeSyntax::TaggedSelbriWithoutTerminalRelative(value) => {
                let TaggedSelbriWithoutTerminalRelativeSyntax {
                    tense_modal,
                    inner_selbri,
                } = value;
                Self::TaggedSelbri(TaggedSelbriSyntax {
                    tense_modal,
                    inner_selbri: Arc::new(Arc::unwrap_or_clone(inner_selbri).into()),
                })
            }
            SelbriWithoutTerminalRelativeSyntax::UntaggedSelbriWithoutTerminalRelative(value) => {
                Self::UntaggedSelbri(value.into())
            }
        }
    }
}

impl From<UntaggedSelbriWithoutTerminalRelativeSyntax> for UntaggedSelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: UntaggedSelbriWithoutTerminalRelativeSyntax) -> Self {
        match value {
            UntaggedSelbriWithoutTerminalRelativeSyntax::NegatedSelbriWithoutTerminalRelative(
                value,
            ) => {
                let NegatedSelbriWithoutTerminalRelativeSyntax { na, inner_selbri } = value;
                Self::NegatedSelbri(NegatedSelbriSyntax {
                    na,
                    inner_selbri: Arc::new(Arc::unwrap_or_clone(inner_selbri).into()),
                })
            }
            UntaggedSelbriWithoutTerminalRelativeSyntax::CoSelbri(value) => Self::CoSelbri(value),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn map_recovered<T, U>(
    value: recovered::Recovered<T>,
    convert: impl FnOnce(T) -> U,
) -> recovered::Recovered<U> {
    match value {
        recovered::Recovered::Valid(value) => recovered::Recovered::valid(convert(*value)),
        recovered::Recovered::Error(error) => recovered::Recovered::error(error),
        recovered::Recovered::Prefix(prefix) => recovered::Recovered::prefix_boxed(
            prefix.errors.into_vec(),
            Box::new(convert(*prefix.value)),
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_restricted_assignment_into_assignment(
    value: recovered::ZantufaSelbriAssignmentWithoutTerminalRelativeSyntax,
) -> recovered::ZantufaSelbriAssignmentSyntax {
    let recovered::ZantufaSelbriAssignmentWithoutTerminalRelativeSyntax { cei, selbri } = value;
    recovered::ZantufaSelbriAssignmentSyntax {
        cei,
        selbri: Arc::new(map_recovered(
            Arc::unwrap_or_clone(selbri),
            recovered_selbri_into_selbri,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_assigned_into_full(
    value: recovered::ZantufaAssignedSelbriWithoutTerminalRelativeSyntax,
) -> recovered::ZantufaAssignedSelbriSyntax {
    let recovered::ZantufaAssignedSelbriWithoutTerminalRelativeSyntax {
        leading_selbri,
        preceding_assignments,
        final_assignment,
    } = value;
    let mut assignments = preceding_assignments;
    assignments.push(map_recovered(
        final_assignment,
        recovered_restricted_assignment_into_assignment,
    ));
    recovered::ZantufaAssignedSelbriSyntax {
        leading_selbri,
        assignments: vec1::Vec1::try_from_vec(assignments)
            .expect("the final recovered assignment makes the normalized sequence non-empty"),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_priority_into_priority(
    value: recovered::ZantufaPriorityAssignedSelbriWithoutTerminalRelativeSyntax,
) -> recovered::ZantufaPriorityAssignedSelbriSyntax {
    recovered::ZantufaPriorityAssignedSelbriSyntax(Arc::new(map_recovered(
        Arc::unwrap_or_clone(value.0),
        recovered_assigned_into_full,
    )))
}

#[requires(true)]
#[ensures(true)]
fn recovered_untagged_into_untagged(
    value: recovered::UntaggedSelbriWithoutTerminalRelativeSyntax,
) -> recovered::UntaggedSelbriSyntax {
    match value {
        recovered::UntaggedSelbriWithoutTerminalRelativeSyntax::NegatedSelbriWithoutTerminalRelative(
            value,
        ) => recovered::UntaggedSelbriSyntax::NegatedSelbri(map_recovered(value, |value| {
            let recovered::NegatedSelbriWithoutTerminalRelativeSyntax { na, inner_selbri } =
                value;
            recovered::NegatedSelbriSyntax {
                na,
                inner_selbri: Arc::new(map_recovered(
                    Arc::unwrap_or_clone(inner_selbri),
                    recovered_selbri_into_selbri,
                )),
            }
        })),
        recovered::UntaggedSelbriWithoutTerminalRelativeSyntax::CoSelbri(value) => {
            recovered::UntaggedSelbriSyntax::CoSelbri(value)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_selbri_into_selbri(
    value: recovered::SelbriWithoutTerminalRelativeSyntax,
) -> recovered::SelbriSyntax {
    match value {
        recovered::SelbriWithoutTerminalRelativeSyntax::ZantufaPriorityAssignedSelbriWithoutTerminalRelative(
            value,
        ) => recovered::SelbriSyntax::ZantufaPriorityAssignedSelbri(map_recovered(
            value,
            recovered_priority_into_priority,
        )),
        recovered::SelbriWithoutTerminalRelativeSyntax::TaggedSelbriWithoutTerminalRelative(
            value,
        ) => recovered::SelbriSyntax::TaggedSelbri(map_recovered(value, |value| {
            let recovered::TaggedSelbriWithoutTerminalRelativeSyntax {
                tense_modal,
                inner_selbri,
            } = value;
            recovered::TaggedSelbriSyntax {
                tense_modal,
                inner_selbri: Arc::new(map_recovered(
                    Arc::unwrap_or_clone(inner_selbri),
                    recovered_untagged_into_untagged,
                )),
            }
        })),
        recovered::SelbriWithoutTerminalRelativeSyntax::UntaggedSelbriWithoutTerminalRelative(
            value,
        ) => recovered::SelbriSyntax::UntaggedSelbri(map_recovered(
            value,
            recovered_untagged_into_untagged,
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn prepend_selbri_recovery_item(
    value: recovered::SelbriSyntax,
    item: super::SyntaxRecoveryItem,
) -> recovered::SelbriSyntax {
    match value {
        recovered::SelbriSyntax::ZantufaPriorityAssignedSelbri(value) => {
            recovered::SelbriSyntax::ZantufaPriorityAssignedSelbri(
                value.prepend_recovery_item(item),
            )
        }
        recovered::SelbriSyntax::ReinterpretZantufaAssignedSelbri(value) => {
            recovered::SelbriSyntax::ReinterpretZantufaAssignedSelbri(
                value.prepend_recovery_item(item),
            )
        }
        recovered::SelbriSyntax::ZantufaRelativeSelbri(value) => {
            recovered::SelbriSyntax::ZantufaRelativeSelbri(value.prepend_recovery_item(item))
        }
        recovered::SelbriSyntax::TaggedSelbri(value) => {
            recovered::SelbriSyntax::TaggedSelbri(value.prepend_recovery_item(item))
        }
        recovered::SelbriSyntax::UntaggedSelbri(value) => {
            recovered::SelbriSyntax::UntaggedSelbri(value.prepend_recovery_item(item))
        }
    }
}

#[contract_trait]
impl GrammarMapTo<recovered::SelbriSyntax>
    for recovered::Recovered<recovered::SelbriWithoutTerminalRelativeSyntax>
{
    fn grammar_map_to(self) -> recovered::SelbriSyntax {
        match self {
            recovered::Recovered::Valid(value) => recovered_selbri_into_selbri(*value),
            recovered::Recovered::Error(error) => {
                recovered::SelbriSyntax::UntaggedSelbri(recovered::Recovered::error(error))
            }
            recovered::Recovered::Prefix(prefix) => {
                let mut value = recovered_selbri_into_selbri(*prefix.value);
                for error in prefix.errors.into_vec().into_iter().rev() {
                    value = prepend_selbri_recovery_item(value, error);
                }
                value
            }
        }
    }
}

impl From<recovered::Recovered<recovered::SelbriWithoutTerminalRelativeSyntax>>
    for recovered::SelbriSyntax
{
    #[requires(true)]
    #[ensures(true)]
    fn from(value: recovered::Recovered<recovered::SelbriWithoutTerminalRelativeSyntax>) -> Self {
        GrammarMapTo::grammar_map_to(value)
    }
}

#[contract_trait]
impl GrammarMapTo<recovered::Recovered<recovered::SelbriSyntax>>
    for recovered::Recovered<recovered::SelbriWithoutTerminalRelativeSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::SelbriSyntax> {
        map_recovered(self, recovered_selbri_into_selbri)
    }
}
