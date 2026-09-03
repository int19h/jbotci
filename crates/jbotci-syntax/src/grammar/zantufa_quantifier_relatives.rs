//! The started-production entry invariant for rolling Zantufa's quantifier relatives (#830 D3c).
//!
//! Rolling Zantufa's quantifier carries a trailing relative list:
//! `quantifier <- (!sumti_5 !selbri mex relative_clauses?)` (zantufa-1.9999.peg:55). jbotci
//! spells the optional list as separate with-relatives sibling variants, so the four-row
//! ownership policy is enforced jointly: a baseline quantifier SPELLING without relatives is
//! returned to the baseline route by `BaselineQuantifierRejection`, and the same spelling WITH
//! relatives is Zantufa-owned by variant selection, because the with-relatives variant is a
//! different rule and carries no baseline rejection at all.
//!
//! That joint enforcement rests on one fact that the strict spine gives for free and the
//! recovered spine does not: that a with-relatives candidate really has relatives. On the
//! recovered spine the runtime can satisfy a mandatory field by synthesizing an error item
//! having consumed no input and having never entered the relative-list production, and it can
//! also fail the field parser, boundary-resynchronize, and hand back a `SkippedTokens` error
//! whose skipped tokens need not begin with any relative opener. Either would let an
//! absent-relative surface enter the with-relatives variant and buy Zantufa ownership with an
//! error item — violating the recovered policy's row 1 without any strict-path symptom.
//!
//! The test is therefore ENTRY EVIDENCE on the completed product, not slot presence and not slot
//! extent:
//!
//! - `Recovered::Valid(list)` is started only through RECURSIVE descent. A slot-level `Valid`
//!   wrapper can still contain a `first` atom that was itself synthesized, exactly the
//!   "a node can be `Valid` with a placeholder under it" condition `baseline_relative.rs`
//!   documents, so the predicate descends through `list.first` and the selected
//!   `relative_clause_atom` product until an actual opening-marker token is proven parsed.
//! - `Recovered::Prefix { value, .. }` is started only through the same recursive descent on its
//!   parsed value.
//! - `Recovered::Error(MissingRequiredField)` — the synthesized missing field, which is the
//!   precise spelling of "zero extent, no attempt", because the runtime always attaches an item
//!   — is NOT started.
//! - `Recovered::Error(SkippedTokens)` is started iff the FIRST skipped token belongs to the
//!   complete source-derived FIRST inventory of `relative_clause_atom`.
//! - any wrapper shape not enumerated above is NOT started, fail-closed.
//!
//! Rejecting the with-relatives candidate lets ordered choice fall to the no-relatives
//! candidate, where the baseline classifier produces exactly the absent-slot answer.

use std::sync::OnceLock;

use bityzba::{contract_trait, data, invariant, requires};
use jbotci_morphology::{Cmavo, Selmaho};

use super::generated_model::{
    ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax,
    ZantufaRawMeksoQuantifierWithRelativesSyntax, recovered,
};
use super::generated_runtime::{OutputRejection, output_rejection_site};
// `SyntaxRecoveryItemData` is named only through the `data!` pattern alias below, which the
// unused-import lint does not see through.
#[allow(unused_imports)]
use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData, Token};

/// Whether a token is an opener of `relative_clause_atom`.
///
/// The inventory is the COMPLETE source-derived FIRST set of the production, read off
/// `generated.rs` by descending it rather than probed:
///
/// ```text
/// relative_clause_atom := sumti_association_relative_clause | bridi_relative_clause
/// sumti_association_relative_clause      := selmaho(Goi) ...
/// bridi_relative_clause                  := statement_relative_clause
///                                         | restrictive_bridi_relative_clause
///                                         | incidental_bridi_relative_clause
/// statement_relative_clause              := (alias) zantufa_statement_relative_clause
///                                         := zantufa_restrictive_statement_relative_clause
///                                          | zantufa_incidental_statement_relative_clause
/// zantufa_restrictive_statement_relative_clause := choice(Poi, Pohoi, Voi, Voihi) ...
/// zantufa_incidental_statement_relative_clause  := choice(Noi, Nohoi) ...
/// restrictive_bridi_relative_clause      := choice(Poi, Voi) ...
/// incidental_bridi_relative_clause       := cmavo(Noi) ...
/// ```
///
/// Every alternative's first constituent is a single marker word, so the union is selma'o GOI in
/// full (`goi`, `ne`, `no'u`, `pe`, `po`, `po'e`, `po'u`, `voi'e`) plus the six relative markers.
/// ZIhE is deliberately absent: it is the continuation connective of `relative_clause_list`,
/// reached only after the list's `first` atom, so it can never open the list.
///
/// The rolling-Zantufa markers are in the inventory unconditionally. The predicate asks whether
/// the relative-list PRODUCTION was entered; a gated alternative the profile disables simply
/// never produces a candidate, so including its opener can only make the answer more
/// conservative about calling a slot unstarted, never less.
#[requires(true)]
#[ensures(
    ret -> !token.is_cmavo(Cmavo::Zihe),
    "ZIhE is the list's continuation connective and can never open it"
)]
// This exhaustive body is the specification; restating it as an equivalence is tautological.
fn opens_relative_clause_atom(token: &Token) -> bool {
    token.is_selmaho(Selmaho::Goi)
        || token.is_cmavo(Cmavo::Poi)
        || token.is_cmavo(Cmavo::Pohoi)
        || token.is_cmavo(Cmavo::Voi)
        || token.is_cmavo(Cmavo::Voihi)
        || token.is_cmavo(Cmavo::Noi)
        || token.is_cmavo(Cmavo::Nohoi)
}

/// The value of a recovered slot that proved itself valid, or `None`.
#[requires(true)]
#[ensures(
    ret.is_some() == matches!(value, recovered::Recovered::Valid(_)),
    "only a `Valid` wrapper carries a value that proved itself"
)]
fn valid<T>(value: &recovered::Recovered<T>) -> Option<&T> {
    match value {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

/// Whether a recovered relative-clause atom proves its own opening marker was parsed.
///
/// The match is exhaustive over both sums, so a further relative alternative has to answer this
/// question for itself rather than defaulting either way.
#[requires(true)]
#[ensures(
    ret -> matches!(
        atom,
        recovered::RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(
            recovered::Recovered::Valid(_)
        ) | recovered::RelativeClauseAtomSyntax::BridiRelativeClause(
            recovered::Recovered::Valid(_)
        )
    ),
    "an opener can only be proven under a clause wrapper that proved itself"
)]
// This exhaustive body is the specification; restating it as an equivalence is tautological.
fn atom_opener_is_parsed(atom: &recovered::RelativeClauseAtomSyntax) -> bool {
    match atom {
        recovered::RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) => {
            valid(clause).is_some_and(|clause| valid(&clause.association_marker.value).is_some())
        }
        recovered::RelativeClauseAtomSyntax::BridiRelativeClause(clause) => {
            valid(clause).is_some_and(|clause| match clause {
                recovered::BridiRelativeClauseSyntax::ZantufaStatementRelativeClause(clause) => {
                    valid(clause).is_some_and(|clause| match clause {
                        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(clause) => {
                            valid(clause).is_some_and(|clause| valid(&clause.poi.value).is_some())
                        }
                        recovered::ZantufaStatementRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(clause) => {
                            valid(clause).is_some_and(|clause| valid(&clause.noi.value).is_some())
                        }
                    })
                }
                recovered::BridiRelativeClauseSyntax::RestrictiveBridiRelativeClause(clause) => {
                    valid(clause).is_some_and(|clause| valid(&clause.poi.value).is_some())
                }
                recovered::BridiRelativeClauseSyntax::IncidentalBridiRelativeClause(clause) => {
                    valid(clause).is_some_and(|clause| valid(&clause.noi.value).is_some())
                }
            })
        }
    }
}

/// Whether a recovered relative list proves that the production was actually entered.
#[requires(true)]
#[ensures(
    ret -> matches!(list.first, recovered::Recovered::Valid(_)),
    "a started list proved its own first atom"
)]
// This body is the specification; restating its single predicate as an equivalence is tautological.
fn list_is_started(list: &recovered::RelativeClauseListSyntax) -> bool {
    let recovered::RelativeClauseListSyntax {
        first,
        additional: _,
    } = list;
    valid(first).is_some_and(atom_opener_is_parsed)
}

/// Whether the recovered `relative_clauses` slot proves the relative-list production started.
#[requires(true)]
#[ensures(
    matches!(slot, recovered::Recovered::Error(item)
        if matches!(item.as_data(), data!(SyntaxRecoveryItem::MissingRequiredField { .. })))
        -> !ret,
    "the synthesized missing field never entered the production (R5)"
)]
// This exhaustive body is the specification; restating it as an equivalence is tautological.
fn slot_is_started(slot: &recovered::Recovered<recovered::RelativeClauseListSyntax>) -> bool {
    match slot {
        recovered::Recovered::Valid(list) => list_is_started(list),
        recovered::Recovered::Prefix(prefix) => list_is_started(prefix.value.as_ref()),
        recovered::Recovered::Error(item) => match item.as_data() {
            // The synthesized missing field: zero extent, no attempt, never entered.
            data!(SyntaxRecoveryItem::MissingRequiredField { .. }) => false,
            // Boundary resynchronization skipped input here.  It is an ATTEMPT only if the input
            // it skipped begins where a relative clause would have begun.
            data!(SyntaxRecoveryItem::SkippedTokens { tokens, .. }) => {
                opens_relative_clause_atom(tokens.first())
            }
        },
    }
}

/// Whether the quantifier-relatives entry-evidence trace is switched on.
///
/// # `JBOTCI_TRACE_QUANTIFIER_RELATIVES`
///
/// Set the environment variable to any non-empty value to print one line to stderr for every
/// RECOVERED with-relatives quantifier candidate this classifier judges:
///
/// ```text
/// quantifier-relatives site=quantified_sumti slot=valid entry=parsed-opener started=yes decision=accept
/// ```
///
/// - `site` is the enclosing generated rule that consumed the quantifier, read from the parser's
///   active rule stack, which the classifier cannot otherwise see;
/// - `slot` is the recovered wrapper the mandatory `relative_clauses` field was handed
///   (`valid` / `prefix` / `missing` / `skipped` / `unproven` for a candidate that is not even
///   `Valid` itself);
/// - `entry` is what the recursive descent found: `parsed-opener`, `synthesized-opener`,
///   `skipped-<word>` naming the first skipped token, or `none`;
/// - `started` is the predicate's answer and `decision` is what the route does with it.
///
/// Use it when hunting for, or arguing the impossibility of, a particular recovered slot shape at
/// this position: the four ownership rows and the two negative controls are distinguished by
/// exactly these fields, and none of them is visible in the rendered tree, because a rejected
/// candidate leaves nothing behind at all.
#[requires(true)]
#[ensures(true)]
pub(crate) fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("JBOTCI_TRACE_QUANTIFIER_RELATIVES").is_some_and(|value| !value.is_empty())
    })
}

/// One trace line for a recovered with-relatives candidate, with the consumer that asked for it.
#[requires(true)]
#[ensures(true)]
fn trace_recovered_candidate(
    slot: Option<&recovered::Recovered<recovered::RelativeClauseListSyntax>>,
    started: bool,
    rejected: bool,
) {
    let site = output_rejection_site(|frames| {
        frames
            .iter()
            .rev()
            .find(|rule| !rule.ends_with("_with_relatives_candidate"))
            .copied()
            .unwrap_or("<unknown>")
    });
    let (slot_kind, entry) = match slot {
        None => ("unproven", "none".to_owned()),
        Some(recovered::Recovered::Valid(list)) => (
            "valid",
            if list_is_started(list) {
                "parsed-opener".to_owned()
            } else {
                "synthesized-opener".to_owned()
            },
        ),
        Some(recovered::Recovered::Prefix(prefix)) => (
            "prefix",
            if list_is_started(prefix.value.as_ref()) {
                "parsed-opener".to_owned()
            } else {
                "synthesized-opener".to_owned()
            },
        ),
        Some(recovered::Recovered::Error(item)) => match item.as_data() {
            data!(SyntaxRecoveryItem::MissingRequiredField { .. }) => {
                ("missing", "none".to_owned())
            }
            data!(SyntaxRecoveryItem::SkippedTokens { tokens, .. }) => {
                ("skipped", format!("skipped-{}", tokens.first().core_word()))
            }
        },
    };
    let decision = if rejected { "reject" } else { "accept" };
    let started = if started { "yes" } else { "no" };
    eprintln!(
        "quantifier-relatives site={site} slot={slot_kind} entry={entry} started={started} decision={decision}"
    );
}

/// Refuses a with-relatives raw-mex quantifier whose relative list was never entered.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct UnstartedRelativeListRejection;

const UNSTARTED_RELATIVE_LIST_REJECTION_NAME: &str = "unstarted quantifier relative list";

// The strict impls are recorded no-ops in the audited `#[invariant(true)]` spirit: a strict parse
// cannot synthesize a field, so a strict with-relatives candidate really parsed its mandatory
// relative list and there is nothing left to establish.
#[contract_trait]
impl OutputRejection<ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax>
    for UnstartedRelativeListRejection
{
    fn rejected_name(&self) -> &'static str {
        UNSTARTED_RELATIVE_LIST_REJECTION_NAME
    }

    fn rejects(&self, _value: &ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax) -> bool {
        false
    }
}

#[contract_trait]
impl OutputRejection<ZantufaRawMeksoQuantifierWithRelativesSyntax>
    for UnstartedRelativeListRejection
{
    fn rejected_name(&self) -> &'static str {
        UNSTARTED_RELATIVE_LIST_REJECTION_NAME
    }

    fn rejects(&self, _value: &ZantufaRawMeksoQuantifierWithRelativesSyntax) -> bool {
        false
    }
}

#[contract_trait]
impl
    OutputRejection<
        recovered::Recovered<recovered::ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax>,
    > for UnstartedRelativeListRejection
{
    fn rejected_name(&self) -> &'static str {
        UNSTARTED_RELATIVE_LIST_REJECTION_NAME
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<
            recovered::ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax,
        >,
    ) -> bool {
        // A candidate that did not prove itself proves nothing about its slot either.
        let slot = valid(value).map(|candidate| &candidate.relative_clauses);
        let rejected = !slot.is_some_and(slot_is_started);
        if trace_enabled() {
            trace_recovered_candidate(slot, !rejected, rejected);
        }
        rejected
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ZantufaRawMeksoQuantifierWithRelativesSyntax>>
    for UnstartedRelativeListRejection
{
    fn rejected_name(&self) -> &'static str {
        UNSTARTED_RELATIVE_LIST_REJECTION_NAME
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ZantufaRawMeksoQuantifierWithRelativesSyntax>,
    ) -> bool {
        let slot = valid(value).map(|candidate| &candidate.relative_clauses);
        let rejected = !slot.is_some_and(slot_is_started);
        if trace_enabled() {
            trace_recovered_candidate(slot, !rejected, rejected);
        }
        rejected
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[allow(unused_imports)]
    use bityzba::{ensures, new, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use vec1::vec1;

    use crate::ParseOptions;
    use crate::grammar::syntax_tokens;

    use super::*;

    /// A recovery placeholder standing in for a child that did not parse: zero extent, no
    /// attempt, which is exactly the synthesized-missing shape row R5 turns on.
    #[requires(true)]
    #[ensures(true)]
    fn missing_field() -> SyntaxRecoveryItem {
        let span = jbotci_diagnostics::source_span_from_byte_offsets(None, "", 0, 0)
            .expect("valid zero-width source span");
        new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 0,
            span: Arc::new(span),
            expected: "relative clauses".to_owned(),
        })
    }

    /// The one syntax token the given text -- which must segment to exactly one word --
    /// morphologises to.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn one_token(text: &str) -> Token {
        let words = segment_words_with_modifiers(text).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let [token] = tokens.as_slice() else {
            panic!("text must be exactly one word");
        };
        token.clone()
    }

    /// A boundary-resynchronization error over the words of one source text.
    ///
    /// The text is segmented in ONE pass so that the skipped tokens carry ordered source
    /// attribution, which is the item type's own expensive invariant.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn skipped(text: &str) -> SyntaxRecoveryItem {
        let words = segment_words_with_modifiers(text).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let mut tokens = tokens.iter().cloned();
        let first = tokens.next().expect("at least one word");
        let mut skipped = vec1![first];
        skipped.extend(tokens);
        new!(SyntaxRecoveryItem::SkippedTokens {
            error_index: 0,
            tokens: skipped,
        })
    }

    /// A relative-clause atom whose opening marker is either proven parsed or synthesized.
    #[requires(true)]
    #[ensures(true)]
    fn atom(opener_parsed: bool) -> recovered::Recovered<recovered::RelativeClauseAtomSyntax> {
        let poi = if opener_parsed {
            recovered::Recovered::valid(one_token("poi"))
        } else {
            recovered::Recovered::Error(missing_field())
        };
        recovered::Recovered::valid(recovered::RelativeClauseAtomSyntax::BridiRelativeClause(
            recovered::Recovered::valid(
                recovered::BridiRelativeClauseSyntax::RestrictiveBridiRelativeClause(
                    recovered::Recovered::valid(recovered::RestrictiveBridiRelativeClauseSyntax {
                        poi: recovered::WithFreeModifiers {
                            value: poi,
                            free_modifiers: Vec::new(),
                        },
                        subbridi: Arc::new(recovered::Recovered::Error(missing_field())),
                        kuho: None,
                    }),
                ),
            ),
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn list(opener_parsed: bool) -> recovered::RelativeClauseListSyntax {
        recovered::RelativeClauseListSyntax {
            first: atom(opener_parsed),
            additional: Vec::new(),
        }
    }

    /// R2: a `Valid` slot whose opener is proven parsed is started.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_slot_with_a_parsed_opener_is_started() {
        assert!(slot_is_started(&recovered::Recovered::valid(list(true))));
    }

    /// The SEVENTH recovered control (I1): a slot-level `Valid` whose NESTED opener was
    /// synthesized proves nothing, because the recovered model nests and a wrapper can be
    /// `Valid` over a placeholder. Without the recursive descent this row would pass.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_slot_over_a_synthesized_opener_is_not_started() {
        assert!(!slot_is_started(&recovered::Recovered::valid(list(false))));
    }

    /// R4: a `Prefix` slot is started only through the same recursive descent on its value.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prefix_slot_is_started_only_through_a_parsed_opener() {
        let started = recovered::Recovered::Prefix(jbotci_tree::RecoveredPrefix {
            errors: vec1![missing_field()],
            value: Box::new(list(true)),
        });
        let unstarted = recovered::Recovered::Prefix(jbotci_tree::RecoveredPrefix {
            errors: vec1![missing_field()],
            value: Box::new(list(false)),
        });
        assert!(slot_is_started(&started));
        assert!(!slot_is_started(&unstarted));
    }

    /// R5: the synthesized missing field never entered the production, so it may not buy
    /// Zantufa ownership. This is the row that has no strict-path symptom at all.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn synthesized_missing_slot_is_not_started() {
        let slot: recovered::Recovered<recovered::RelativeClauseListSyntax> =
            recovered::Recovered::Error(missing_field());
        assert!(!slot_is_started(&slot));
    }

    /// R3: boundary resynchronization that skipped a relative OPENER is a real attempt.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skipped_relative_opener_is_started() {
        let slot: recovered::Recovered<recovered::RelativeClauseListSyntax> =
            recovered::Recovered::Error(skipped("poi mi"));
        assert!(slot_is_started(&slot));
    }

    /// The SIXTH recovered control (H1): resynchronization that skipped NON-relative input has a
    /// non-empty extent and is still not an attempt. Extent alone would let this row through.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skipped_non_relative_input_is_not_started() {
        let slot: recovered::Recovered<recovered::RelativeClauseListSyntax> =
            recovered::Recovered::Error(skipped("mi klama"));
        assert!(!slot_is_started(&slot));
    }

    /// The whole GOI selma'o opens the list, not only the six relative markers.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_full_goi_selmaho_opens_the_list() {
        for word in ["goi", "ne", "no'u", "pe", "po", "po'e", "po'u", "voi'e"] {
            assert!(opens_relative_clause_atom(&one_token(word)), "{word}");
        }
        for word in ["poi", "po'oi", "voi", "voi'i", "noi", "no'oi"] {
            assert!(opens_relative_clause_atom(&one_token(word)), "{word}");
        }
        // ZIhE is the list's continuation connective, reached only after its `first` atom.
        assert!(!opens_relative_clause_atom(&one_token("zi'e")));
    }

    /// A candidate that did not prove itself proves nothing about its slot either.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unproven_candidate_is_refused() {
        let candidate: recovered::Recovered<
            recovered::ZantufaPriorityRawMeksoQuantifierWithRelativesSyntax,
        > = recovered::Recovered::Error(missing_field());
        assert!(UnstartedRelativeListRejection.rejects(&candidate));
    }
}
