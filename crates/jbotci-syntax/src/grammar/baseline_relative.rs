//! Typed ownership classification for camxes-exp relative-clause continuations.
//!
//! The extension route runs first so it can classify a completed connective-plus-clause
//! candidate. A bare ZIhE connective is rejected here and reparsed by the baseline arm.
//! Both routes begin at the connective and end at the same completed relative-clause atom,
//! so the reparse has identical extent. Token-class lookahead cannot establish this ownership:
//! several words participate in multiple selma'o, and only the completed candidate proves which
//! relative-clause route succeeded. Every generated node used by this proof is destructured
//! exhaustively and without `..`, so model changes force the proof to be revisited.

use bityzba::{contract_trait, invariant, requires};
use jbotci_morphology::Cmavo;

use super::generated_model::{
    BridiRelativeClauseSyntax, ExpRelativeClauseConnectiveSyntax, ExpRelativeContinuationSyntax,
    RelativeClauseAtomSyntax, RelativeClauseListSyntax, RelativeClauseTailSyntax,
    ZantufaRelativeStatementBaseSyntax, ZantufaRelativeStatementSyntax,
    ZantufaStatementRelativeClauseSyntax, recovered,
};
use super::generated_runtime::OutputRejection;

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineRelativeContinuationRejection;

#[requires(true)]
#[ensures(true)]
fn is_baseline_relative_continuation(value: &ExpRelativeContinuationSyntax) -> bool {
    let ExpRelativeContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let ExpRelativeClauseConnectiveSyntax { na, se, head, nai } = connective;
    na.is_none() && se.is_none() && head.value.cmavo() == Some(Cmavo::Zihe) && nai.is_none()
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
fn recovered_is_baseline_relative_continuation(
    value: &recovered::ExpRelativeContinuationSyntax,
) -> bool {
    let recovered::ExpRelativeContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let Some(connective) = valid(connective) else {
        return false;
    };
    let recovered::ExpRelativeClauseConnectiveSyntax { na, se, head, nai } = connective;
    na.is_none()
        && se.is_none()
        && valid(&head.value).is_some_and(|head| head.cmavo() == Some(Cmavo::Zihe))
        && nai.is_none()
}

#[contract_trait]
impl OutputRejection<ExpRelativeContinuationSyntax> for BaselineRelativeContinuationRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline ZIhE relative continuation"
    }

    fn rejects(&self, value: &ExpRelativeContinuationSyntax) -> bool {
        is_baseline_relative_continuation(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpRelativeContinuationSyntax>>
    for BaselineRelativeContinuationRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline ZIhE relative continuation"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpRelativeContinuationSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_baseline_relative_continuation)
    }
}

/// Ownership classifier for the rolling-Zantufa statement relative clause at the two sumti
/// site classes, S1 and S2.
///
/// The Zantufa arm keeps the full source NOI inventory (`voi'i / voi / poi / po'oi / noi /
/// no'oi`, zantufa-1.9999.peg:590) because the Zantufa-only statement bodies attach to the
/// shared markers too. What it must not keep is the identical extent the baseline already
/// owns: a `poi`, `noi` or `voi` marker over a body the baseline `subbridi` can form, which
/// is any run of non-empty prenexes ending in a bridi. Those return here and reparse through
/// the baseline arm, which begins at the same marker and ends at the same terminator, so the
/// reparse has identical extent. A body carrying an I-connection or a TUhE group is not a
/// `subbridi` at all and stays Zantufa's, which is what makes
/// `lo broda poi mi brode ije do brodi ku'o cu brodi` a warned Zantufa surface while
/// `lo broda poi mi brode ku'o cu brodi` stays silent baseline.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineStatementRelativeRejection;

/// Ownership classifier for the S3 selbri-level parent.
///
/// The S3 arm is ordered ahead of the selbri ladder that carries camxes-exp's tanru-unit
/// relative, so a completed list every one of whose clauses that route could form is returned
/// here and reaches it by falling through. "Could form" is the exp production read literally:
/// `selbri_relative_clause_1 <- NOhOI_clause free* subsentence KUhOI_elidible` chained by
/// `(ZIhE_clause / joik)` (camxes-exp.peg:214-218), so the marker must be `no'oi` or `po'oi`,
/// the body must be a subsentence shape, the KUhO must be absent -- an explicit `ku'o` is a
/// terminator exp does not have -- and every continuation must be one exp spells. The list is
/// judged whole because the exp chain is one node: a ZIhE-joined list with a `poi` clause in
/// it is not an extent exp can form, and splitting it would change what the tree says.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpSelbriRelativeListRejection;

#[requires(true)]
#[ensures(true)]
fn is_baseline_marker(cmavo: Option<Cmavo>) -> bool {
    matches!(cmavo, Some(Cmavo::Poi | Cmavo::Noi | Cmavo::Voi))
}

#[requires(true)]
#[ensures(true)]
fn is_nohoi_marker(cmavo: Option<Cmavo>) -> bool {
    matches!(cmavo, Some(Cmavo::Nohoi | Cmavo::Pohoi))
}

/// True when the Zantufa relative body is a shape the baseline `subbridi` and camxes-exp's
/// `subsentence` can both form: a run of prenexes ending in a bridi. Zantufa's own prenex
/// requires terms, so every prenex reached here is one both of the other shapes admit.
#[requires(true)]
#[ensures(true)]
fn is_subbridi_shaped_body(value: &ZantufaRelativeStatementSyntax) -> bool {
    match value {
        ZantufaRelativeStatementSyntax::ZantufaRelativePrenexStatement(prenex) => {
            is_subbridi_shaped_body(&prenex.inner_statement)
        }
        ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(_) => false,
        ZantufaRelativeStatementSyntax::ZantufaRelativeStatementBase(base) => match base {
            ZantufaRelativeStatementBaseSyntax::TextGroupStatement(_) => false,
            ZantufaRelativeStatementBaseSyntax::ZantufaRelativeBridiStatement(_) => true,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_statement_relative(value: &ZantufaStatementRelativeClauseSyntax) -> bool {
    match value {
        ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(clause) => {
            is_baseline_marker(clause.poi.value.cmavo())
                && is_subbridi_shaped_body(&clause.statement)
        }
        ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(clause) => {
            is_baseline_marker(clause.noi.value.cmavo())
                && is_subbridi_shaped_body(&clause.statement)
        }
    }
}

/// True when camxes-exp's tanru-unit relative could form this one clause.
#[requires(true)]
#[ensures(true)]
fn is_exp_selbri_relative_clause(value: &RelativeClauseAtomSyntax) -> bool {
    let RelativeClauseAtomSyntax::BridiRelativeClause(bridi) = value else {
        return false;
    };
    let BridiRelativeClauseSyntax::ZantufaStatementRelativeClause(clause) = bridi else {
        return false;
    };
    match clause {
        ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(clause) => {
            is_nohoi_marker(clause.poi.value.cmavo())
                && clause.kuho.is_none()
                && is_subbridi_shaped_body(&clause.statement)
        }
        ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(clause) => {
            is_nohoi_marker(clause.noi.value.cmavo())
                && clause.kuho.is_none()
                && is_subbridi_shaped_body(&clause.statement)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_exp_selbri_relative_continuation(value: &RelativeClauseTailSyntax) -> bool {
    match value {
        RelativeClauseTailSyntax::RelativeClauseExpContinuation(continuation) => {
            is_exp_selbri_relative_clause(&continuation.0.inner)
        }
        RelativeClauseTailSyntax::JoinedRelativeClauseTail(joined) => {
            is_exp_selbri_relative_clause(&joined.inner)
        }
        // Rolling Zantufa's bare adjacency has no camxes-exp counterpart at all.
        RelativeClauseTailSyntax::ZantufaBareRelativeClauseTail(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_exp_selbri_relative_list(value: &RelativeClauseListSyntax) -> bool {
    let RelativeClauseListSyntax { first, additional } = value;
    is_exp_selbri_relative_clause(first)
        && additional.iter().all(is_exp_selbri_relative_continuation)
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_subbridi_shaped_body(value: &recovered::ZantufaRelativeStatementSyntax) -> bool {
    match value {
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativePrenexStatement(prenex) => {
            valid(prenex).is_some_and(|prenex| {
                valid(&prenex.inner_statement)
                    .is_some_and(|inner| recovered_is_subbridi_shaped_body(inner))
            })
        }
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(_) => false,
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeStatementBase(base) => {
            valid(base).is_some_and(|base| {
                matches!(
                    base,
                    recovered::ZantufaRelativeStatementBaseSyntax::ZantufaRelativeBridiStatement(_)
                )
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_baseline_statement_relative(
    value: &recovered::ZantufaStatementRelativeClauseSyntax,
) -> bool {
    match value {
        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(
            clause,
        ) => valid(clause).is_some_and(|clause| {
            valid(&clause.poi.value).is_some_and(|poi| is_baseline_marker(poi.cmavo()))
                && valid(&clause.statement)
                    .is_some_and(|statement| recovered_is_subbridi_shaped_body(statement))
        }),
        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(
            clause,
        ) => valid(clause).is_some_and(|clause| {
            valid(&clause.noi.value).is_some_and(|noi| is_baseline_marker(noi.cmavo()))
                && valid(&clause.statement)
                    .is_some_and(|statement| recovered_is_subbridi_shaped_body(statement))
        }),
    }
}

#[contract_trait]
impl OutputRejection<ZantufaStatementRelativeClauseSyntax> for BaselineStatementRelativeRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline NOI relative clause"
    }

    fn rejects(&self, value: &ZantufaStatementRelativeClauseSyntax) -> bool {
        is_baseline_statement_relative(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaStatementRelativeClauseSyntax>>
    for BaselineStatementRelativeRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline NOI relative clause"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaStatementRelativeClauseSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_baseline_statement_relative)
    }
}

#[contract_trait]
impl OutputRejection<RelativeClauseListSyntax> for ExpSelbriRelativeListRejection {
    fn rejected_name(&self) -> &'static str {
        "camxes-exp tanru-unit relative clause list"
    }

    fn rejects(&self, value: &RelativeClauseListSyntax) -> bool {
        is_exp_selbri_relative_list(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::RelativeClauseListSyntax>>
    for ExpSelbriRelativeListRejection
{
    fn rejected_name(&self) -> &'static str {
        "camxes-exp tanru-unit relative clause list"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::RelativeClauseListSyntax>) -> bool {
        valid(value).is_some_and(recovered_is_exp_selbri_relative_list)
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_exp_selbri_relative_clause(value: &recovered::RelativeClauseAtomSyntax) -> bool {
    let recovered::RelativeClauseAtomSyntax::BridiRelativeClause(bridi) = value else {
        return false;
    };
    let Some(bridi) = valid(bridi) else {
        return false;
    };
    let recovered::BridiRelativeClauseSyntax::ZantufaStatementRelativeClause(clause) = bridi else {
        return false;
    };
    let Some(clause) = valid(clause) else {
        return false;
    };
    match clause {
        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(
            clause,
        ) => valid(clause).is_some_and(|clause| {
            valid(&clause.poi.value).is_some_and(|poi| is_nohoi_marker(poi.cmavo()))
                && clause.kuho.is_none()
                && valid(&clause.statement)
                    .is_some_and(|statement| recovered_is_subbridi_shaped_body(statement))
        }),
        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(
            clause,
        ) => valid(clause).is_some_and(|clause| {
            valid(&clause.noi.value).is_some_and(|noi| is_nohoi_marker(noi.cmavo()))
                && clause.kuho.is_none()
                && valid(&clause.statement)
                    .is_some_and(|statement| recovered_is_subbridi_shaped_body(statement))
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_exp_selbri_relative_continuation(
    value: &recovered::RelativeClauseTailSyntax,
) -> bool {
    match value {
        recovered::RelativeClauseTailSyntax::RelativeClauseExpContinuation(continuation) => {
            valid(continuation).is_some_and(|continuation| {
                valid(&continuation.0).is_some_and(|continuation| {
                    valid(&continuation.inner).is_some_and(recovered_is_exp_selbri_relative_clause)
                })
            })
        }
        recovered::RelativeClauseTailSyntax::JoinedRelativeClauseTail(joined) => valid(joined)
            .is_some_and(|joined| {
                valid(&joined.inner)
                    .is_some_and(|inner| recovered_is_exp_selbri_relative_clause(inner))
            }),
        recovered::RelativeClauseTailSyntax::ZantufaBareRelativeClauseTail(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_exp_selbri_relative_list(value: &recovered::RelativeClauseListSyntax) -> bool {
    let recovered::RelativeClauseListSyntax { first, additional } = value;
    valid(first).is_some_and(recovered_is_exp_selbri_relative_clause)
        && additional
            .iter()
            .all(|tail| valid(tail).is_some_and(recovered_is_exp_selbri_relative_continuation))
}
