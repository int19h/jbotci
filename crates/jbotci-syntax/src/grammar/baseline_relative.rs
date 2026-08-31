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
    BridiRelativeClauseSyntax, BridiSyntax, ExpRelativeClauseConnectiveSyntax,
    ExpRelativeContinuationSyntax, ExpSelbriRelativeClauseConnectiveSyntax,
    ExpSelbriRelativeClauseContinuationSyntax, ExpSoiSubsentenceAdverbialSyntax,
    RelativeClauseAtomSyntax, RelativeClauseListSyntax, RelativeClauseTailSyntax,
    SimpleIntervalConnectiveSyntax, SubbridiSyntax, TermSyntax, ZantufaRelativeStatementBaseSyntax,
    ZantufaRelativeStatementSyntax, ZantufaStatementRelativeClauseSyntax,
    ZantufaXoiStatementAdverbialSyntax, recovered,
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
/// owns: the exact marker one of the two baseline arms spells -- `poi`/`voi` restrictive,
/// `noi` incidental -- over a body the baseline `subbridi` can form, which is any run of
/// non-empty prenexes ending in a bridi. Those return here and reparse through that arm, which
/// begins at the same marker and ends at the same terminator, so the reparse has identical
/// extent. A body carrying an I-connection or a TUhE group is not a `subbridi` at all and
/// stays Zantufa's, which is what makes `lo broda poi mi brode ije do brodi ku'o cu brodi` a
/// warned Zantufa surface while `lo broda poi mi brode ku'o cu brodi` stays silent baseline.
/// The extension-only markers `po'oi`, `voi'i` and `no'oi` have no baseline arm at all, so
/// nothing they carry returns from here: they are R3 at these sites in every terminator shape.
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

/// The marker `restrictive_bridi_relative_clause` spells: camxes-standard's own restrictive NOI
/// (camxes.peg:1695), which is what the return has to prove, because that is the exact arm the
/// candidate reparses through. `po'oi` and `voi'i` are rolling-Zantufa extensions with no
/// baseline arm at all.
/// Placement classifier for the free-modifier slot camxes-exp's `joik` does not spell.
///
/// `joik <- NA_clause? SE_clause? (JOI_clause / JA_clause / A_clause) NAI_clause? / interval /
/// GAhO_clause interval GAhO_clause` with `interval <- SE_clause? BIhI_clause NAI_clause?`
/// (camxes-exp.peg:347-349), and the `free*` both relative chains carry is OUTSIDE it -- `(ZIhE_clause
/// / joik) free* relative_clause` (:199, :214). The `_clause` wrappers do not restore the slot:
/// `post_clause <- spaces? si_clause? !ZEI_clause !BU_clause indicators*` carries indicators, not
/// frees. So `je to do brodi toi nai` is not a connective camxes-exp derives, while `je nai to do
/// brodi toi` and `je to do brodi toi` are -- the free modifiers of the latter two being the
/// chain's own `free*`.
///
/// jbotci's connective nodes spell that slot on the head instead, and they are shared: the same
/// `exp_relative_clause_connective` serves the ordinary relative chain at :199, and both interval
/// nodes serve the baseline `joik_connective`, whose consumers supply no outer `free*` at all.
/// Removing the slot from those nodes would therefore withdraw surfaces the epoch base accepts
/// through routes this epoch does not own -- `lo broda poi mi brode je to do brodi toi nai poi do
/// brodi ku cu brodi` and `li pa bi'i to do brodi toi nai li re` are both `A` at `0d791fd35c` --
/// which is the unsourced-placement sweep filed as #847, not this epoch's family. What this epoch
/// does own is the chain it added, so the prohibited placement is refused exactly there.
///
/// Only the two head-before-optional-NAI shapes can present it. `zihe_selbri_relative_connective`
/// spells `ZIhE_clause` alone, whose trailing frees are the chain's own; `closed_interval_connective`
/// already carries its slot on the closing GAhO, after the NAI, where the source puts it.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProhibitedRelativeConnectiveFreeModifierRejection;

#[requires(true)]
#[ensures(true)]
fn is_prohibited_connective_free_modifier(
    value: &ExpSelbriRelativeClauseContinuationSyntax,
) -> bool {
    let ExpSelbriRelativeClauseContinuationSyntax {
        connective,
        inner: _,
    } = value;
    match connective {
        ExpSelbriRelativeClauseConnectiveSyntax::ZiheSelbriRelativeConnective(_)
        | ExpSelbriRelativeClauseConnectiveSyntax::ClosedIntervalConnective(_) => false,
        ExpSelbriRelativeClauseConnectiveSyntax::ExpRelativeClauseConnective(connective) => {
            let ExpRelativeClauseConnectiveSyntax {
                na: _,
                se: _,
                head,
                nai,
            } = connective;
            nai.is_some() && !head.free_modifiers.is_empty()
        }
        ExpSelbriRelativeClauseConnectiveSyntax::SimpleIntervalConnective(connective) => {
            let SimpleIntervalConnectiveSyntax { se: _, bihi, nai } = connective;
            nai.is_some() && !bihi.free_modifiers.is_empty()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_prohibited_connective_free_modifier(
    value: &recovered::ExpSelbriRelativeClauseContinuationSyntax,
) -> bool {
    let recovered::ExpSelbriRelativeClauseContinuationSyntax {
        connective,
        inner: _,
    } = value;
    let Some(connective) = valid(connective) else {
        return false;
    };
    match connective {
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::ZiheSelbriRelativeConnective(_)
        | recovered::ExpSelbriRelativeClauseConnectiveSyntax::ClosedIntervalConnective(_) => false,
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::ExpRelativeClauseConnective(
            connective,
        ) => valid(connective).is_some_and(|connective| {
            let recovered::ExpRelativeClauseConnectiveSyntax {
                na: _,
                se: _,
                head,
                nai,
            } = connective;
            nai.is_some() && !head.free_modifiers.is_empty()
        }),
        recovered::ExpSelbriRelativeClauseConnectiveSyntax::SimpleIntervalConnective(
            connective,
        ) => valid(connective).is_some_and(|connective| {
            let recovered::SimpleIntervalConnectiveSyntax { se: _, bihi, nai } = connective;
            nai.is_some() && !bihi.free_modifiers.is_empty()
        }),
    }
}

#[contract_trait]
impl OutputRejection<ExpSelbriRelativeClauseContinuationSyntax>
    for ProhibitedRelativeConnectiveFreeModifierRejection
{
    fn rejected_name(&self) -> &'static str {
        "free modifier before the connective's NAI"
    }

    fn rejects(&self, value: &ExpSelbriRelativeClauseContinuationSyntax) -> bool {
        is_prohibited_connective_free_modifier(value)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpSelbriRelativeClauseContinuationSyntax>>
    for ProhibitedRelativeConnectiveFreeModifierRejection
{
    fn rejected_name(&self) -> &'static str {
        "free modifier before the connective's NAI"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpSelbriRelativeClauseContinuationSyntax>,
    ) -> bool {
        valid(value).is_some_and(recovered_is_prohibited_connective_free_modifier)
    }
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_restrictive_marker(cmavo: Option<Cmavo>) -> bool {
    matches!(cmavo, Some(Cmavo::Poi | Cmavo::Voi))
}

/// The marker `incidental_bridi_relative_clause` spells. `no'oi` is the extension.
#[requires(true)]
#[ensures(true)]
fn is_baseline_incidental_marker(cmavo: Option<Cmavo>) -> bool {
    matches!(cmavo, Some(Cmavo::Noi))
}

#[requires(true)]
#[ensures(true)]
fn is_nohoi_marker(cmavo: Option<Cmavo>) -> bool {
    matches!(cmavo, Some(Cmavo::Nohoi | Cmavo::Pohoi))
}

/// What the classifier has PROVED about a Zantufa relative body, as opposed to what it merely
/// failed to prove.
///
/// A boolean cannot carry this: on the recovered side a body that did not parse is not a
/// statement-width body, and folding the two together sends an unparseable candidate down the
/// longer-extent branch and hands it to a baseline arm that cannot reparse it. Every ownership
/// fact this classifier decides on is proved from a valid node or it is `Unproven`.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelativeBodyShape {
    /// A run of prenexes ending in a bridi: a shape the baseline `subbridi` and camxes-exp's
    /// `subsentence` can both form. Zantufa's own prenex requires terms, so every prenex
    /// reached here is one both of the other shapes admit.
    Subbridi,
    /// Wider than any `subbridi`: an I-connection or a TUhE group.
    StatementWidth,
    /// Neither, because a node the answer depends on is a recovery placeholder.
    Unproven,
}

#[requires(true)]
#[ensures(true)]
fn body_shape(value: &ZantufaRelativeStatementSyntax) -> RelativeBodyShape {
    match value {
        ZantufaRelativeStatementSyntax::ZantufaRelativePrenexStatement(prenex) => {
            body_shape(&prenex.inner_statement)
        }
        ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(_) => {
            RelativeBodyShape::StatementWidth
        }
        ZantufaRelativeStatementSyntax::ZantufaRelativeStatementBase(base) => match base {
            ZantufaRelativeStatementBaseSyntax::TextGroupStatement(_) => {
                RelativeBodyShape::StatementWidth
            }
            ZantufaRelativeStatementBaseSyntax::ZantufaRelativeBridiStatement(_) => {
                RelativeBodyShape::Subbridi
            }
        },
    }
}

/// True when the Zantufa relative body is a shape the baseline `subbridi` and camxes-exp's
/// `subsentence` can both form.
#[requires(true)]
#[ensures(true)]
fn is_subbridi_shaped_body(value: &ZantufaRelativeStatementSyntax) -> bool {
    matches!(body_shape(value), RelativeBodyShape::Subbridi)
}

#[requires(true)]
#[ensures(true)]
fn is_baseline_statement_relative(value: &ZantufaStatementRelativeClauseSyntax) -> bool {
    match value {
        ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(clause) => {
            returns_to_baseline(
                is_baseline_restrictive_marker(clause.poi.value.cmavo()),
                clause.kuho.is_none(),
                body_shape(&clause.statement),
            )
        }
        ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(clause) => {
            returns_to_baseline(
                is_baseline_incidental_marker(clause.noi.value.cmavo()),
                clause.kuho.is_none(),
                body_shape(&clause.statement),
            )
        }
    }
}

/// The two halves of the return, over the three facts that decide it.
///
/// BOTH halves require the baseline marker, because a return with no baseline owner is not a
/// return at all: `po'oi`, `voi'i` and `no'oi` have no baseline arm, so declining one of those
/// candidates leaves the extent to nothing. The frozen S1/S2 rule is a baseline marker AND a
/// body the baseline can form, and `baseline_marker` here is the EXACT arm the candidate would
/// reparse through -- `poi`/`voi` for the restrictive clause, `noi` for the incidental one --
/// rather than the union of the two.
///
/// The identical-extent half is R1 as the plan states it: a standard marker over a body the
/// baseline `subbridi` can form is the baseline's own clause, and the baseline arm begins at
/// the same marker and ends at the same terminator, so the reparse has identical extent.
///
/// The longer-extent half is the mirror of the reservation D2's clause carries, and it is what
/// keeps this arm from taking a baseline parse apart rather than adding to it.  A statement-
/// width body -- an I-connection or a TUhE group -- is wider than any `subbridi`, so with the
/// KUhO elided the arm does not merely re-own the baseline's clause, it swallows whatever
/// follows it: on `ko erve tu'a pa litce poi ladru .ije ganai zvati fa su'o caksova gi ...` the
/// baseline closes the clause at `ladru` and reads the `.ije` as the paragraph's own join, and
/// thirteen corpus fixtures read that way.  Rolling Zantufa's own terminator is what tells the
/// two apart, so the Zantufa-only body keeps the extent only when the Zantufa terminator is
/// there to close it; that is the same rule, and the same word, that decides the D2 boundary.
/// With an extension-only marker there is no such competing baseline reading to preserve, so
/// the extent stays Zantufa's under R3 whatever its terminator does.
///
/// `Unproven` never returns: an ownership fact that is not proved cannot decide a route.
#[requires(true)]
#[ensures(!ret || baseline_marker, "a candidate is never returned without a proven baseline owner")]
fn returns_to_baseline(baseline_marker: bool, kuho_elided: bool, body: RelativeBodyShape) -> bool {
    match body {
        RelativeBodyShape::Subbridi => baseline_marker,
        RelativeBodyShape::StatementWidth => baseline_marker && kuho_elided,
        RelativeBodyShape::Unproven => false,
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

/// True when the adopted camxes-exp chain can spell this relative-list continuation.
///
/// The source is `selbri_relative_clauses <- selbri_relative_clause ((ZIhE_clause / joik) free*
/// selbri_relative_clause)*` (camxes-exp.peg:214), whose `joik` is the very nonterminal the
/// ordinary `relative_clauses` chain uses one level up (:199) -- `NA_clause? SE_clause?
/// (JOI_clause / JA_clause / A_clause) NAI_clause?` under an explicit A-JA-JOI merge (:346).
/// D2's chain and this list therefore hold the SAME two connective nodes,
/// `zihe_selbri_relative_connective`/`joined_relative_clause_tail` for ZIhE and
/// `exp_relative_clause_connective` for the joik, so every connective a continuation here can
/// present is one the exp chain consumes and the test is the clause alone. Rolling Zantufa's
/// bare adjacency is the one tail with no camxes-exp counterpart, and it is refused by shape.
#[requires(true)]
#[ensures(true)]
fn is_exp_selbri_relative_continuation(value: &RelativeClauseTailSyntax) -> bool {
    match value {
        RelativeClauseTailSyntax::RelativeClauseExpContinuation(continuation) => {
            let ExpRelativeContinuationSyntax {
                connective: _,
                inner,
            } = continuation.0.as_ref();
            is_exp_selbri_relative_clause(inner)
        }
        RelativeClauseTailSyntax::JoinedRelativeClauseTail(joined) => {
            is_exp_selbri_relative_clause(&joined.inner)
        }
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
fn recovered_body_shape(value: &recovered::ZantufaRelativeStatementSyntax) -> RelativeBodyShape {
    match value {
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativePrenexStatement(prenex) => {
            valid(prenex)
                .and_then(|prenex| valid(&prenex.inner_statement))
                .map_or(RelativeBodyShape::Unproven, recovered_body_shape)
        }
        // The selected arm is itself the proof of width: an I-connection is what this variant
        // is, whether or not its payload survived recovery.
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(_) => {
            RelativeBodyShape::StatementWidth
        }
        recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeStatementBase(base) => {
            valid(base).map_or(RelativeBodyShape::Unproven, |base| match base {
                recovered::ZantufaRelativeStatementBaseSyntax::TextGroupStatement(_) => {
                    RelativeBodyShape::StatementWidth
                }
                recovered::ZantufaRelativeStatementBaseSyntax::ZantufaRelativeBridiStatement(_) => {
                    RelativeBodyShape::Subbridi
                }
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_is_subbridi_shaped_body(value: &recovered::ZantufaRelativeStatementSyntax) -> bool {
    matches!(recovered_body_shape(value), RelativeBodyShape::Subbridi)
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
            returns_to_baseline(
                valid(&clause.poi.value)
                    .is_some_and(|poi| is_baseline_restrictive_marker(poi.cmavo())),
                clause.kuho.is_none(),
                valid(&clause.statement)
                    .map_or(RelativeBodyShape::Unproven, recovered_body_shape),
            )
        }),
        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(
            clause,
        ) => valid(clause).is_some_and(|clause| {
            returns_to_baseline(
                valid(&clause.noi.value)
                    .is_some_and(|noi| is_baseline_incidental_marker(noi.cmavo())),
                clause.kuho.is_none(),
                valid(&clause.statement)
                    .map_or(RelativeBodyShape::Unproven, recovered_body_shape),
            )
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

/// Ownership classifier for the rolling-Zantufa XOI adverbial.
///
/// Zantufa's `term_2 <- XOI_clause statement SEhU_elidible` (zantufa-1.9999.peg:29) and
/// camxes-exp's `SOI_clause free* subsentence SEhU_elidible` (camxes-exp.peg:149, :160) share
/// two of their three words and disagree on the body. The Zantufa arm runs first because its
/// body is the wider one -- the shorter camxes-exp reading would otherwise succeed and leave
/// the rest of an I-connected body behind -- and hands back every extent camxes-exp can form,
/// so it keeps only the statement-width ones. R2: adopted camxes-exp owns the shared extents.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpSubsentenceAdverbialRejection;

/// R1 no-steal for the camxes-exp SOI adverbial.
///
/// `mi broda soi mi brode` is accepted by all three reference parsers and camxes-standard
/// reads it as the reciprocal `soi mi` with `brode` continuing the tanru outside. The
/// adverbial arm therefore returns any completed candidate that reparses that way: the marker
/// is `soi` -- `xoi` and `fi'oi` are in no reciprocal -- the SEhU is elided, so the extent has
/// no terminator of its own to keep it whole, and the subsentence opens with the exact `sumti`
/// the reciprocal would take as its `leading_sumti`. An explicit SEhU, a body whose first term
/// is anything other than a bare sumti, or either of the other two markers is not a reparse the
/// baseline can produce, and stays here.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct BaselineReciprocalSoiRejection;

/// True when the candidate's body opens with the exact constituent the reciprocal would take as
/// its `leading_sumti`.
///
/// `soi_free_modifier` spells `SOI free* sumti sumti? SEhU_elidible`, so the reparse needs a
/// bare `sumti` in first position, not merely a first `term`: `term` also covers tagged sumti,
/// termsets, `na ku` and the adverbials themselves, none of which the reciprocal can consume.
/// `sumti_term` is `term`'s one arm that is exactly `sumti`, so the reparse is proved by the
/// arm rather than inferred from the run being non-empty.
#[requires(true)]
#[ensures(true)]
fn subsentence_opens_with_leading_sumti(value: &SubbridiSyntax) -> bool {
    match value {
        SubbridiSyntax::PrenexSubbridi(_) => false,
        SubbridiSyntax::BridiSubbridi(bridi) => match bridi.0.as_ref() {
            BridiSyntax::BridiWithLeadingTerms(bridi) => {
                matches!(bridi.leading_terms.first(), TermSyntax::SumtiTerm(_))
            }
            BridiSyntax::BareCuBridi(_) | BridiSyntax::RelationOnlyBridi(_) => false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_subsentence_opens_with_leading_sumti(value: &recovered::SubbridiSyntax) -> bool {
    match value {
        recovered::SubbridiSyntax::PrenexSubbridi(_) => false,
        recovered::SubbridiSyntax::BridiSubbridi(bridi) => valid(bridi).is_some_and(|bridi| {
            valid(&bridi.0).is_some_and(|bridi| match bridi {
                recovered::BridiSyntax::BridiWithLeadingTerms(bridi) => {
                    valid(bridi).is_some_and(|bridi| {
                        valid(bridi.leading_terms.first())
                            .is_some_and(|term| matches!(term, recovered::TermSyntax::SumtiTerm(_)))
                    })
                }
                recovered::BridiSyntax::BareCuBridi(_)
                | recovered::BridiSyntax::RelationOnlyBridi(_) => false,
            })
        }),
    }
}

#[contract_trait]
impl OutputRejection<ZantufaXoiStatementAdverbialSyntax> for ExpSubsentenceAdverbialRejection {
    fn rejected_name(&self) -> &'static str {
        "camxes-exp SOI subsentence adverbial"
    }

    fn rejects(&self, value: &ZantufaXoiStatementAdverbialSyntax) -> bool {
        is_subbridi_shaped_body(&value.statement)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaXoiStatementAdverbialSyntax>>
    for ExpSubsentenceAdverbialRejection
{
    fn rejected_name(&self) -> &'static str {
        "camxes-exp SOI subsentence adverbial"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaXoiStatementAdverbialSyntax>,
    ) -> bool {
        valid(value).is_some_and(|value| {
            valid(&value.statement).is_some_and(recovered_is_subbridi_shaped_body)
        })
    }
}

#[contract_trait]
impl OutputRejection<ExpSoiSubsentenceAdverbialSyntax> for BaselineReciprocalSoiRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline SOI reciprocal"
    }

    fn rejects(&self, value: &ExpSoiSubsentenceAdverbialSyntax) -> bool {
        value.soi.value.cmavo() == Some(Cmavo::Soi)
            && value.sehu.is_none()
            && subsentence_opens_with_leading_sumti(&value.subsentence)
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpSoiSubsentenceAdverbialSyntax>>
    for BaselineReciprocalSoiRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline SOI reciprocal"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpSoiSubsentenceAdverbialSyntax>,
    ) -> bool {
        valid(value).is_some_and(|value| {
            valid(&value.soi.value).is_some_and(|soi| soi.cmavo() == Some(Cmavo::Soi))
                && value.sehu.is_none()
                && valid(&value.subsentence)
                    .is_some_and(recovered_subsentence_opens_with_leading_sumti)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[allow(unused_imports)]
    use bityzba::{ensures, new, requires};
    use jbotci_morphology::segment_words_with_modifiers;

    use crate::ParseOptions;
    use crate::grammar::{SyntaxRecoveryItemData, syntax_tokens};
    use crate::tree::SyntaxRecoveryItem;

    use super::*;

    /// A recovery placeholder standing in for a child that did not parse.
    #[requires(true)]
    #[ensures(true)]
    fn recovery_placeholder() -> SyntaxRecoveryItem {
        let span = jbotci_diagnostics::source_span_from_byte_offsets(None, "", 0, 0)
            .expect("valid zero-width source span");
        new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 0,
            span: Arc::new(span),
            expected: "statement".to_owned(),
        })
    }

    /// A restrictive Zantufa statement relative clause with an elided KUhO, over the given
    /// marker text -- which must segment to exactly one word -- and the given body.
    #[requires(!marker.is_empty())]
    #[ensures(true)]
    fn recovered_restrictive_clause(
        marker: &str,
        statement: recovered::Recovered<recovered::ZantufaRelativeStatementSyntax>,
    ) -> recovered::Recovered<recovered::ZantufaStatementRelativeClauseSyntax> {
        let words = segment_words_with_modifiers(marker).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let [token] = tokens.as_slice() else {
            panic!("marker text must be exactly one word");
        };
        recovered::Recovered::valid(
            recovered::ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(
                recovered::Recovered::valid(
                    recovered::ZantufaRestrictiveStatementRelativeClauseSyntax {
                        poi: recovered::WithFreeModifiers {
                            value: recovered::Recovered::valid(token.clone()),
                            free_modifiers: Vec::new(),
                        },
                        statement: Arc::new(statement),
                        kuho: None,
                    },
                ),
            ),
        )
    }

    /// The three ownership facts compose exactly one way: nothing is returned to a baseline arm
    /// that does not exist, and nothing is decided from a fact the tree did not prove.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn returns_to_baseline_needs_every_fact_proven() {
        for kuho_elided in [false, true] {
            for body in [
                RelativeBodyShape::Subbridi,
                RelativeBodyShape::StatementWidth,
                RelativeBodyShape::Unproven,
            ] {
                assert!(
                    !returns_to_baseline(false, kuho_elided, body),
                    "an extension-only marker never returns ({kuho_elided}, {body:?})"
                );
            }
            assert!(
                !returns_to_baseline(true, kuho_elided, RelativeBodyShape::Unproven),
                "an unproven body never returns ({kuho_elided})"
            );
            assert!(
                returns_to_baseline(true, kuho_elided, RelativeBodyShape::Subbridi),
                "R1: a baseline marker over a subbridi body is the baseline's ({kuho_elided})"
            );
        }
        assert!(
            returns_to_baseline(true, true, RelativeBodyShape::StatementWidth),
            "the longer-extent half fires for a baseline marker with no Zantufa terminator"
        );
        assert!(
            !returns_to_baseline(true, false, RelativeBodyShape::StatementWidth),
            "an explicit KUhO closes the statement-width extent for rolling Zantufa"
        );
    }

    /// A body that did not parse is `Unproven`, not "not a subbridi": the shape is read off a
    /// valid node or it is not read at all. The connected arm is the one exception, and it is
    /// the variant tag rather than the payload that proves it.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_body_shape_is_unproven_when_the_body_did_not_parse() {
        let placeholder = recovery_placeholder();
        assert_eq!(
            recovered_body_shape(
                &recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeStatementBase(
                    recovered::Recovered::error(placeholder.clone()),
                )
            ),
            RelativeBodyShape::Unproven,
        );
        assert_eq!(
            recovered_body_shape(
                &recovered::ZantufaRelativeStatementSyntax::ZantufaRelativePrenexStatement(
                    recovered::Recovered::error(placeholder.clone()),
                )
            ),
            RelativeBodyShape::Unproven,
        );
        assert_eq!(
            recovered_body_shape(
                &recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(
                    recovered::Recovered::error(placeholder),
                )
            ),
            RelativeBodyShape::StatementWidth,
        );
    }

    /// The recovered twin of the S1/S2 return, over the three facts directly.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_baseline_statement_relative_returns_only_proven_baseline_extents() {
        let rejection = BaselineStatementRelativeRejection;
        let placeholder = recovery_placeholder();
        let connected = || {
            recovered::Recovered::valid(
                recovered::ZantufaRelativeStatementSyntax::ZantufaRelativeConnectedStatement(
                    recovered::Recovered::error(placeholder.clone()),
                ),
            )
        };

        assert!(
            rejection.rejects(&recovered_restrictive_clause("poi", connected())),
            "a baseline marker over a statement-width body with no KUhO is the longer-extent half"
        );
        assert!(
            !rejection.rejects(&recovered_restrictive_clause("po'oi", connected())),
            "po'oi has no baseline arm, so nothing it carries returns"
        );
        assert!(
            !rejection.rejects(&recovered_restrictive_clause("voi'i", connected())),
            "voi'i has no baseline arm either"
        );
        assert!(
            !rejection.rejects(&recovered_restrictive_clause(
                "poi",
                recovered::Recovered::error(placeholder.clone()),
            )),
            "a body that did not parse cannot be handed to an arm that must reparse it"
        );
        assert!(
            !rejection.rejects(&recovered::Recovered::error(placeholder)),
            "an unparsed clause is not a completed candidate at all"
        );
    }
}
