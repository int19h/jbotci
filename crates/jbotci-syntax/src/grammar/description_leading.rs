//! R1 ownership for camxes-exp's full-sumti description-leading element (epoch 9, #830).
//!
//! camxes-exp's `sumti_tail` arm 3 (`sumti sumti_tail_1`, camxes-exp.peg:194) admits a FULL
//! sumti — connection level — where the baseline admits a `sumti_6`. jbotci adopts it as a
//! sibling descriptor variant, and the ownership rule for the overlap is R1: the baseline owns
//! every identical extent that reparses baseline, so a candidate whose leading sumti is an
//! extent the BASELINE leading operand already derives is refused here and stays baseline.
//!
//! Ordered choice makes that true on the strict spine by itself — `descriptor_with_gadri_sumti`
//! is tried first — so the strict predicate is very nearly a recorded no-op. It is not one on
//! the RECOVERED spine, where a baseline attempt can fail for a repairable reason and this arm
//! would otherwise take the extent. The classifier is what makes R1 hold there, and it is the
//! invariant the epoch's recovered witnesses pin.
//!
//! The answer is three-valued for the same reason `SumtiOperandTier`'s is: on the recovered
//! spine a candidate can fail to establish anything at all, and "did not parse" is not "known
//! not baseline-derivable". `Unproven` is refused, so an unproven leading sumti never takes an
//! extent away from the baseline route.
//!
//! What the predicate does NOT do is decide the quantifier question. After the operand tier
//! restriction a quantifier-bearing extent is no longer baseline-leading-derivable, so this
//! predicate would PASS it; the rule's own `!quantifier` negative lookahead is what keeps such
//! extents baseline-owned through `sumti_tail_1`, and the two guards are deliberately separate
//! concerns.
//!
//! # Entry evidence, not occupancy
//!
//! Every answer of `ExpOnly` rests on a DISCRIMINATOR: some connection-level slot that the
//! baseline leading operand -- one `sumti_base` of the permitted `sumti_6` tier -- cannot
//! reproduce. On the strict spine an occupied slot is proof, because a strict parse cannot
//! occupy a slot it did not parse. On the recovered spine it is not: the runtime satisfies an
//! optional or repeated slot from a recovery item as readily as from input
//! (`generated_runtime.rs`, `impl RecoveredSyntaxSlot for Option<T>` returns `Some(..)` and the
//! `Vec<T>` impl returns a one-element vector), so a candidate whose leading sumti is a bare
//! baseline `sumti_6` plus a SYNTHESIZED connective, VUhO, KE, BO, GEK or quantifier would read
//! as exp-only and steal the extent from the baseline route -- the same defect class the D3c
//! entry invariant closes for the quantifier's relative list.
//!
//! So each discriminator is established by RECURSIVE ENTRY EVIDENCE: the discriminating token
//! itself -- the continuation's or grouped tail's connective, the VUhO marker, the bound tail's
//! BO, the forethought GEK, the quantified operand's quantifier -- must be proven parsed, down
//! through every recovered wrapper on the way to it. A discriminator that is PRESENT but
//! unproven answers `Unproven`, and `Unproven` is refused, so the baseline route keeps the
//! extent. Occupancy is never evidence here.

use std::sync::OnceLock;

use bityzba::{contract_trait, invariant, requires};
use jbotci_tree::TreeVisitor;

use super::generated_model::{
    ExpFullSumtiDescriptionTailSyntax, SumtiAtomSyntax, SumtiSyntax, recovered,
};
use super::generated_runtime::{OutputRejection, output_rejection_site};
use super::sumti_operand_tier::{SumtiOperandTier, recovered_sumti_base_tier, sumti_base_tier};

/// Where a completed camxes-exp leading sumti's extent belongs.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpLeadingSumtiOrigin {
    /// An extent the baseline leading operand derives: baseline-owned under R1.
    BaselineDerivable,
    /// An extent only camxes-exp's connection-level leading sumti can form.
    ExpOnly,
    /// Nothing was established, so the extent's owner is not decidable from the candidate.
    Unproven,
}

/// Classify a strict leading sumti.
///
/// The baseline leading operand is one `sumti_base` of the permitted `sumti_6` tier and nothing
/// else, so every connection, grouping, VUhO and forethought slot on the way down must be empty
/// for the extent to be one the baseline route can take. A trailing relative list is included:
/// the baseline description tail carries its own relative list immediately after the leading
/// operand, so that extent is one the baseline route derives as well.
///
/// Occupancy IS evidence here, unlike on the recovered spine: a strict parse cannot synthesize a
/// slot, so an occupied connection, VUhO, grouping, BO, GEK or quantifier slot was parsed from
/// input by construction. The recovered twin carries the entry-evidence discipline instead.
#[requires(true)]
#[ensures(ret != ExpLeadingSumtiOrigin::Unproven, "a strict candidate established its shape")]
fn strict_origin(sumti: &SumtiSyntax) -> ExpLeadingSumtiOrigin {
    let SumtiSyntax {
        base_sumti,
        vuho_attachment,
    } = sumti;
    if vuho_attachment.is_some() {
        return ExpLeadingSumtiOrigin::ExpOnly;
    }
    let super::generated_model::SumtiGroupedSyntax {
        leading_sumti,
        grouped_tail,
    } = base_sumti.as_ref();
    if grouped_tail.is_some() {
        return ExpLeadingSumtiOrigin::ExpOnly;
    }
    let super::generated_model::SumtiAfterthoughtSyntax {
        leading_sumti,
        continuations,
    } = leading_sumti.as_ref();
    if !continuations.is_empty() {
        return ExpLeadingSumtiOrigin::ExpOnly;
    }
    let super::generated_model::SumtiBoundSyntax {
        leading_sumti,
        bound_tail,
    } = leading_sumti.as_ref();
    if bound_tail.is_some() {
        return ExpLeadingSumtiOrigin::ExpOnly;
    }
    let simple = match leading_sumti.as_ref() {
        super::generated_model::SumtiForethoughtSyntax::ForethoughtSumti(_) => {
            return ExpLeadingSumtiOrigin::ExpOnly;
        }
        super::generated_model::SumtiForethoughtSyntax::SimpleSumti(simple) => simple,
    };
    let super::generated_model::SimpleSumtiSyntax {
        base_sumti,
        relative_clauses: _,
    } = simple;
    match base_sumti.as_ref() {
        // A quantified operand is not baseline-leading-derivable: the operand tier restriction
        // is precisely what removed it from the baseline leading operand.  The rule's own
        // `!quantifier` lookahead is what keeps such extents baseline-owned.
        SumtiAtomSyntax::QuantifiedSumti(_) => ExpLeadingSumtiOrigin::ExpOnly,
        SumtiAtomSyntax::SumtiBase(base) => match sumti_base_tier(base) {
            SumtiOperandTier::Sumti6 => ExpLeadingSumtiOrigin::BaselineDerivable,
            SumtiOperandTier::Sumti5 | SumtiOperandTier::Unproven => ExpLeadingSumtiOrigin::ExpOnly,
        },
    }
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

/// Records whether the generated in-order traversal reached a token the parser really consumed.
///
/// `visit_recovered_error` is deliberately left at its trait default. A recovery item is not a
/// parsed token, and neither are the tokens a `SkippedTokens` item carries: they were skipped
/// past a failing production, not consumed into it. Only `AtomRef::Token` -- the atom event the
/// generated model emits for a token that landed in the tree -- sets the flag.
#[invariant(true)]
struct ParsedTokenProbe {
    found: bool,
}

impl<'tree> TreeVisitor<'tree> for ParsedTokenProbe {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.found, "reaching a token atom is exactly the evidence being collected")]
    fn visit_atom(&mut self, _atom: Self::Atom) {
        self.found = true;
    }
}

/// Whether a recovered subtree proves that the parser consumed at least one token into it.
///
/// This is the entry-evidence primitive. It is applied to a DISCRIMINATOR's own subtree -- a
/// connective, a marker word, a whole quantifier -- never to a slot that also contains an
/// operand, so "a token was parsed in here" and "this discriminator was really parsed" are the
/// same statement at every call site. The traversal is the generated `TreeVisitor` walk rather
/// than a hand-rolled descent, so a future field or alternative under one of these productions
/// is covered without this module being edited.
#[requires(true)]
#[ensures(true)]
fn proves_a_parsed_token<T: recovered::TreeNode>(node: &T) -> bool {
    let mut probe = ParsedTokenProbe { found: false };
    recovered::TreeNode::visit_in_order(node, &mut probe);
    probe.found
}

/// Whether a recovered VUhO attachment proves its own VUhO marker was parsed.
///
/// The sum is matched exhaustively so a further attachment shape has to answer this question for
/// itself; each arm's marker is checked directly rather than through the subtree probe, because
/// every arm also carries a relative list or a sumti continuation whose tokens are not evidence
/// that the VUhO itself was parsed.
#[requires(true)]
#[ensures(true)]
fn vuho_marker_is_parsed(
    attachment: &recovered::Recovered<recovered::VuhoSumtiAttachmentTailSyntax>,
) -> bool {
    let Some(attachment) = valid(attachment) else {
        return false;
    };
    match attachment {
        recovered::VuhoSumtiAttachmentTailSyntax::ExperimentalVuhoScopedSumtiAttachmentTail(
            tail,
        ) => valid(tail).is_some_and(|tail| valid(&tail.vuho.value).is_some()),
        recovered::VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(tail) => {
            valid(tail).is_some_and(|tail| valid(&tail.vuho.value).is_some())
        }
        // The bare-VUhO extension is a transparent single-field product, so its marker is the
        // whole node.
        recovered::VuhoSumtiAttachmentTailSyntax::ExperimentalBareVuhoSumtiAttachmentTail(tail) => {
            valid(tail).is_some_and(|tail| valid(&tail.0.value).is_some())
        }
    }
}

/// Whether a recovered BO-bound tail proves its own BO marker was parsed.
///
/// BO is the marker both tail shapes share and the one the baseline leading operand can never
/// reproduce; the sourced tail's connective is optional in the Zantufa arm, so BO rather than the
/// connective is what makes the tail a tail.
#[requires(true)]
#[ensures(true)]
fn bound_tail_bo_is_parsed(tail: &recovered::Recovered<recovered::SumtiBoundTailSyntax>) -> bool {
    let Some(tail) = valid(tail) else {
        return false;
    };
    match tail {
        recovered::SumtiBoundTailSyntax::BoundSumtiTail(tail) => {
            valid(tail).is_some_and(|tail| valid(&tail.bo.value).is_some())
        }
        recovered::SumtiBoundTailSyntax::ZantufaBoundSumtiTail(tail) => {
            valid(tail).is_some_and(|tail| valid(&tail.bo.value).is_some())
        }
    }
}

/// Whether the recovered path's deciding exp-only discriminator proves its own entry.
///
/// This is the contract predicate for `recovered_origin`: it follows the same ownership layers,
/// but asks only for the parsed-token evidence at the first occupied discriminator. A synthesized
/// outer discriminator therefore cannot be bypassed by valid evidence deeper in the sumti.
#[requires(true)]
#[ensures(
    ret -> matches!(sumti.base_sumti.as_ref(), recovered::Recovered::Valid(_)),
    "proven exp-only ownership descends through a valid connection level"
)]
fn exp_only_discriminator_is_proven(sumti: &recovered::SumtiSyntax) -> bool {
    let Some(grouped) = valid(&sumti.base_sumti) else {
        return false;
    };
    if let Some(attachment) = &sumti.vuho_attachment {
        return vuho_marker_is_parsed(attachment);
    }
    if let Some(tail) = &grouped.grouped_tail {
        return valid(tail).is_some_and(|tail| proves_a_parsed_token(&tail.connective));
    }
    let Some(afterthought) = valid(&grouped.leading_sumti) else {
        return false;
    };
    if !afterthought.continuations.is_empty() {
        return afterthought.continuations.iter().any(|continuation| {
            valid(continuation)
                .is_some_and(|continuation| proves_a_parsed_token(&continuation.connective))
        });
    }
    let Some(bound) = valid(&afterthought.leading_sumti) else {
        return false;
    };
    if let Some(tail) = &bound.bound_tail {
        return bound_tail_bo_is_parsed(tail);
    }
    let Some(forethought) = valid(&bound.leading_sumti) else {
        return false;
    };
    let simple = match forethought {
        recovered::SumtiForethoughtSyntax::ForethoughtSumti(forethought) => {
            return valid(forethought)
                .is_some_and(|forethought| proves_a_parsed_token(&forethought.gek));
        }
        recovered::SumtiForethoughtSyntax::SimpleSumti(simple) => simple,
    };
    let Some(simple) = valid(simple) else {
        return false;
    };
    let Some(atom) = valid(&simple.base_sumti) else {
        return false;
    };
    match atom {
        recovered::SumtiAtomSyntax::QuantifiedSumti(quantified) => valid(quantified)
            .is_some_and(|quantified| proves_a_parsed_token(&quantified.quantifier)),
        recovered::SumtiAtomSyntax::SumtiBase(base) => valid(base)
            .is_some_and(|base| recovered_sumti_base_tier(base) == SumtiOperandTier::Sumti5),
    }
}

/// Classify a recovered leading sumti.
///
/// Any slot on the reduction path that did not prove itself valid leaves the shape unestablished,
/// which is `Unproven` rather than "not baseline-derivable". So does a discriminator that is
/// occupied but whose own token was never parsed: see the module header for why occupancy cannot
/// stand in for entry evidence on this spine.
#[requires(true)]
#[ensures(
    !matches!(sumti.base_sumti.as_ref(), recovered::Recovered::Valid(_))
        -> ret == ExpLeadingSumtiOrigin::Unproven,
    "a connection level that did not prove itself establishes nothing about the extent"
)]
#[ensures(
    ret == ExpLeadingSumtiOrigin::ExpOnly
        -> matches!(sumti.base_sumti.as_ref(), recovered::Recovered::Valid(_)),
    "exp-only ownership requires a valid connection level"
)]
#[ensures(
    ret == ExpLeadingSumtiOrigin::ExpOnly -> exp_only_discriminator_is_proven(sumti),
    "exp-only ownership rests on a proven parsed discriminator"
)]
fn recovered_origin(sumti: &recovered::SumtiSyntax) -> ExpLeadingSumtiOrigin {
    let recovered::SumtiSyntax {
        base_sumti,
        vuho_attachment,
    } = sumti;
    let Some(grouped) = valid(base_sumti) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    if let Some(attachment) = vuho_attachment {
        return if vuho_marker_is_parsed(attachment) {
            ExpLeadingSumtiOrigin::ExpOnly
        } else {
            ExpLeadingSumtiOrigin::Unproven
        };
    }
    let recovered::SumtiGroupedSyntax {
        leading_sumti,
        grouped_tail,
    } = grouped;
    if let Some(tail) = grouped_tail {
        // The grouped tail's own discriminator is its connective, the production's first
        // constituent; the inner sumti it also carries is an operand, not evidence.
        return if valid(tail).is_some_and(|tail| proves_a_parsed_token(&tail.connective)) {
            ExpLeadingSumtiOrigin::ExpOnly
        } else {
            ExpLeadingSumtiOrigin::Unproven
        };
    }
    let Some(afterthought) = valid(leading_sumti) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    let recovered::SumtiAfterthoughtSyntax {
        leading_sumti,
        continuations,
    } = afterthought;
    if !continuations.is_empty() {
        // One continuation with a proven connective is enough to put the extent out of the
        // baseline leading operand's reach; a list of continuations none of which proved a
        // connective proves nothing at all.
        return if continuations.iter().any(|continuation| {
            valid(continuation)
                .is_some_and(|continuation| proves_a_parsed_token(&continuation.connective))
        }) {
            ExpLeadingSumtiOrigin::ExpOnly
        } else {
            ExpLeadingSumtiOrigin::Unproven
        };
    }
    let Some(bound) = valid(leading_sumti) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    let recovered::SumtiBoundSyntax {
        leading_sumti,
        bound_tail,
    } = bound;
    if let Some(tail) = bound_tail {
        return if bound_tail_bo_is_parsed(tail) {
            ExpLeadingSumtiOrigin::ExpOnly
        } else {
            ExpLeadingSumtiOrigin::Unproven
        };
    }
    let Some(forethought) = valid(leading_sumti) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    let simple = match forethought {
        recovered::SumtiForethoughtSyntax::ForethoughtSumti(forethought) => {
            // The GEK subtree is the connective and nothing else, so probing it whole is the
            // same statement as "the forethought opener was parsed".
            return if valid(forethought)
                .is_some_and(|forethought| proves_a_parsed_token(&forethought.gek))
            {
                ExpLeadingSumtiOrigin::ExpOnly
            } else {
                ExpLeadingSumtiOrigin::Unproven
            };
        }
        recovered::SumtiForethoughtSyntax::SimpleSumti(simple) => simple,
    };
    let Some(simple) = valid(simple) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    let recovered::SimpleSumtiSyntax {
        base_sumti,
        relative_clauses: _,
    } = simple;
    let Some(atom) = valid(base_sumti) else {
        return ExpLeadingSumtiOrigin::Unproven;
    };
    match atom {
        recovered::SumtiAtomSyntax::QuantifiedSumti(quantified) => {
            // Without a proven quantifier the extent this atom consumed is its inner operand's,
            // and that operand is a `description_leading_operand` -- exactly what the baseline
            // route derives. A synthesized quantifier may not turn it into an exp-only extent.
            // The quantifier subtree contains only the quantifier's own material, so probing it
            // whole reaches every spelling the production has, raw mex included.
            if valid(quantified)
                .is_some_and(|quantified| proves_a_parsed_token(&quantified.quantifier))
            {
                ExpLeadingSumtiOrigin::ExpOnly
            } else {
                ExpLeadingSumtiOrigin::Unproven
            }
        }
        recovered::SumtiAtomSyntax::SumtiBase(base) => {
            let Some(base) = valid(base) else {
                return ExpLeadingSumtiOrigin::Unproven;
            };
            match recovered_sumti_base_tier(base) {
                SumtiOperandTier::Sumti6 => ExpLeadingSumtiOrigin::BaselineDerivable,
                SumtiOperandTier::Sumti5 => ExpLeadingSumtiOrigin::ExpOnly,
                SumtiOperandTier::Unproven => ExpLeadingSumtiOrigin::Unproven,
            }
        }
    }
}

/// Whether a classified origin is refused at the camxes-exp leading-sumti route.
///
/// Written as an exhaustive match so a future origin has to answer this question for itself.
#[requires(true)]
#[ensures(ret == (origin != ExpLeadingSumtiOrigin::ExpOnly))]
fn origin_is_rejected(origin: ExpLeadingSumtiOrigin) -> bool {
    match origin {
        ExpLeadingSumtiOrigin::ExpOnly => false,
        ExpLeadingSumtiOrigin::BaselineDerivable | ExpLeadingSumtiOrigin::Unproven => true,
    }
}

/// Whether the camxes-exp leading-sumti classifier trace is switched on.
///
/// # `JBOTCI_TRACE_DESCRIPTION_LEADING`
///
/// Set the environment variable to any non-empty value to print one line to stderr for every
/// RECOVERED candidate this classifier judges:
///
/// ```text
/// description-leading site=descriptor_with_gadri_sumti slot=valid origin=BaselineDerivable decision=reject
/// ```
///
/// - `site` is the enclosing generated rule that consumed the camxes-exp descriptor, read from
///   the parser's active rule stack, which the classifier cannot otherwise see;
/// - `slot` is the recovered wrapper the rule's own `leading_sumti` field was handed
///   (`valid` / `prefix` / `error`, or `unproven-candidate` when the whole candidate is not
///   `Valid`);
/// - `origin` is the three-valued answer and `decision` is whether the route keeps the extent.
///
/// Use it to establish which of R1's three recovered rows a surface actually exercises. None of
/// them is visible in the rendered tree: a refused candidate leaves the extent to the baseline
/// route and nothing behind, so "refused as baseline-derivable" and "refused as unproven" look
/// identical from outside.
#[requires(true)]
#[ensures(true)]
pub(crate) fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("JBOTCI_TRACE_DESCRIPTION_LEADING").is_some_and(|value| !value.is_empty())
    })
}

/// One trace line for a recovered candidate, with the consumer that asked for it.
#[requires(true)]
#[ensures(true)]
fn trace_recovered_candidate(
    leading_sumti: Option<&recovered::Recovered<recovered::SumtiSyntax>>,
    origin: Option<ExpLeadingSumtiOrigin>,
    rejected: bool,
) {
    let site = output_rejection_site(|frames| {
        frames
            .iter()
            .rev()
            .find(|rule| **rule != "exp_full_sumti_description_tail")
            .copied()
            .unwrap_or("<unknown>")
    });
    let slot = match leading_sumti {
        None => "unproven-candidate",
        Some(recovered::Recovered::Valid(_)) => "valid",
        Some(recovered::Recovered::Prefix(_)) => "prefix",
        Some(recovered::Recovered::Error(_)) => "error",
    };
    let origin = origin.map_or_else(|| "unreached".to_owned(), |origin| format!("{origin:?}"));
    let decision = if rejected { "reject" } else { "keep" };
    eprintln!("description-leading site={site} slot={slot} origin={origin} decision={decision}");
}

/// Returns to the baseline route every extent the baseline leading operand derives (R1).
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExpDescriptionLeadingSumtiRejection;

#[contract_trait]
impl OutputRejection<ExpFullSumtiDescriptionTailSyntax> for ExpDescriptionLeadingSumtiRejection {
    fn rejected_name(&self) -> &'static str {
        "baseline-derivable description leading sumti"
    }

    fn rejects(&self, value: &ExpFullSumtiDescriptionTailSyntax) -> bool {
        origin_is_rejected(strict_origin(value.leading_sumti.as_ref()))
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::ExpFullSumtiDescriptionTailSyntax>>
    for ExpDescriptionLeadingSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "baseline-derivable description leading sumti"
    }

    fn rejects(
        &self,
        value: &recovered::Recovered<recovered::ExpFullSumtiDescriptionTailSyntax>,
    ) -> bool {
        // A candidate that did not prove itself is `Unproven`, and `Unproven` is refused, so the
        // whole wrapper is fail-closed in the same direction as the operand tier classifier.
        let slot = valid(value).map(|tail| tail.leading_sumti.as_ref());
        let origin = slot.and_then(valid).map(recovered_origin);
        let rejected = origin.is_none_or(origin_is_rejected);
        if trace_enabled() {
            trace_recovered_candidate(slot, origin, rejected);
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
            expected: "sumti".to_owned(),
        })
    }

    /// The recovered pro-sumti `mi` as a whole recovered `sumti`: the simplest extent the
    /// BASELINE leading operand derives, wrapped in every connection level with its slot empty.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_bare_pro_sumti() -> recovered::SumtiSyntax {
        let words = segment_words_with_modifiers("mi").expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let [token] = tokens.as_slice() else {
            panic!("`mi` must be exactly one word");
        };
        let base = recovered::SumtiBaseSyntax::ProSumti(recovered::Recovered::valid(
            recovered::ProSumtiSyntax(recovered::WithFreeModifiers {
                value: recovered::Recovered::valid(token.clone()),
                free_modifiers: Vec::new(),
            }),
        ));
        let atom = recovered::SumtiAtomSyntax::SumtiBase(recovered::Recovered::valid(base));
        let simple = recovered::SumtiForethoughtSyntax::SimpleSumti(recovered::Recovered::valid(
            recovered::SimpleSumtiSyntax {
                base_sumti: Arc::new(recovered::Recovered::valid(atom)),
                relative_clauses: None,
            },
        ));
        recovered::SumtiSyntax {
            base_sumti: Arc::new(recovered::Recovered::valid(recovered::SumtiGroupedSyntax {
                leading_sumti: Arc::new(recovered::Recovered::valid(
                    recovered::SumtiAfterthoughtSyntax {
                        leading_sumti: Arc::new(recovered::Recovered::valid(
                            recovered::SumtiBoundSyntax {
                                leading_sumti: Arc::new(recovered::Recovered::valid(simple)),
                                bound_tail: None,
                            },
                        )),
                        continuations: Vec::new(),
                    },
                )),
                grouped_tail: None,
            })),
            vuho_attachment: None,
        }
    }

    /// R1's own row: an extent the baseline leading operand derives stays BASELINE-owned, and
    /// the camxes-exp route refuses it.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn baseline_derivable_leading_sumti_is_refused() {
        let sumti = recovered_bare_pro_sumti();
        assert_eq!(
            recovered_origin(&sumti),
            ExpLeadingSumtiOrigin::BaselineDerivable
        );
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::BaselineDerivable));
    }

    /// The one syntax token the given text -- which must segment to exactly one word --
    /// morphologises to.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn one_token(text: &str) -> crate::tree::Token {
        let words = segment_words_with_modifiers(text).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let [token] = tokens.as_slice() else {
            panic!("text must be exactly one word");
        };
        token.clone()
    }

    /// An afterthought continuation whose connective is either proven parsed or synthesized.
    #[requires(true)]
    #[ensures(true)]
    fn continuation(
        connective_parsed: bool,
    ) -> recovered::Recovered<recovered::SumtiAfterthoughtTailSyntax> {
        let connective = if connective_parsed {
            recovered::Recovered::valid(recovered::SumtiConnectiveSyntax::EkConnective(
                recovered::Recovered::valid(recovered::EkConnectiveSyntax {
                    na: None,
                    se: None,
                    a: recovered::WithFreeModifiers {
                        value: recovered::Recovered::valid(one_token(".e")),
                        free_modifiers: Vec::new(),
                    },
                    nai: None,
                }),
            ))
        } else {
            recovered::Recovered::Error(recovery_placeholder())
        };
        recovered::Recovered::valid(recovered::SumtiAfterthoughtTailSyntax {
            connective,
            sumti: Arc::new(recovered::Recovered::Error(recovery_placeholder())),
        })
    }

    /// The bare baseline leading sumti with one afterthought continuation spliced in.
    #[requires(true)]
    #[ensures(true)]
    fn connected(
        continuation: recovered::Recovered<recovered::SumtiAfterthoughtTailSyntax>,
    ) -> recovered::SumtiSyntax {
        let mut sumti = recovered_bare_pro_sumti();
        let grouped = match sumti.base_sumti.as_ref() {
            recovered::Recovered::Valid(grouped) => grouped.as_ref().clone(),
            _ => panic!("constructed value is valid"),
        };
        let afterthought = match grouped.leading_sumti.as_ref() {
            recovered::Recovered::Valid(afterthought) => afterthought.as_ref().clone(),
            _ => panic!("constructed value is valid"),
        };
        let connected = recovered::SumtiAfterthoughtSyntax {
            leading_sumti: afterthought.leading_sumti.clone(),
            continuations: vec![continuation],
        };
        sumti.base_sumti = Arc::new(recovered::Recovered::valid(recovered::SumtiGroupedSyntax {
            leading_sumti: Arc::new(recovered::Recovered::valid(connected)),
            grouped_tail: grouped.grouped_tail.clone(),
        }));
        sumti
    }

    /// An exp-only extent -- here the connection level, which is what makes it exp-only -- is
    /// the one shape the route accepts, and it is the PARSED connective that establishes it:
    /// the baseline leading operand takes a single `sumti_base`, never a connection.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_connective_continuation_is_exp_only_and_accepted() {
        let sumti = connected(continuation(true));
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::ExpOnly);
        assert!(!origin_is_rejected(ExpLeadingSumtiOrigin::ExpOnly));
    }

    /// The rejection control for the same shape: the recovery runtime satisfies a repeated slot
    /// from a recovery item as readily as from input, so an OCCUPIED continuation list whose
    /// connective was never parsed is not evidence of anything. Reading occupancy as proof here
    /// would let a bare baseline `sumti_6` plus a synthesized connective steal the extent from
    /// the baseline route and warn about it.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn synthesized_continuation_is_unproven_and_refused() {
        let sumti = connected(continuation(false));
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::Unproven));
    }

    /// The same control one level down: a continuation that IS `Valid` but whose connective sum
    /// carries a placeholder under it. The recovered model nests, so a wrapper can be `Valid`
    /// over a synthesized child; only the recursive descent to the connective word answers this.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_continuation_over_a_synthesized_connective_is_unproven() {
        let continuation = recovered::Recovered::valid(recovered::SumtiAfterthoughtTailSyntax {
            connective: recovered::Recovered::valid(
                recovered::SumtiConnectiveSyntax::EkConnective(recovered::Recovered::Error(
                    recovery_placeholder(),
                )),
            ),
            sumti: Arc::new(recovered::Recovered::Error(recovery_placeholder())),
        });
        let sumti = connected(continuation);
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
    }

    /// The quantified-operand discriminator: with the quantifier synthesized the extent the atom
    /// consumed is its inner operand's, which the baseline route derives, so the arm may not
    /// answer `ExpOnly` on the strength of the wrapper alone.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn synthesized_quantifier_over_an_operand_is_unproven() {
        let mut sumti = recovered_bare_pro_sumti();
        let quantified = recovered::SumtiAtomSyntax::QuantifiedSumti(recovered::Recovered::valid(
            recovered::QuantifiedSumtiSyntax {
                quantifier: recovered::Recovered::Error(recovery_placeholder()),
                inner_sumti: Arc::new(recovered::Recovered::Error(recovery_placeholder())),
            },
        ));
        let simple = recovered::SumtiForethoughtSyntax::SimpleSumti(recovered::Recovered::valid(
            recovered::SimpleSumtiSyntax {
                base_sumti: Arc::new(recovered::Recovered::valid(quantified)),
                relative_clauses: None,
            },
        ));
        sumti.base_sumti = Arc::new(recovered::Recovered::valid(recovered::SumtiGroupedSyntax {
            leading_sumti: Arc::new(recovered::Recovered::valid(
                recovered::SumtiAfterthoughtSyntax {
                    leading_sumti: Arc::new(recovered::Recovered::valid(
                        recovered::SumtiBoundSyntax {
                            leading_sumti: Arc::new(recovered::Recovered::valid(simple)),
                            bound_tail: None,
                        },
                    )),
                    continuations: Vec::new(),
                },
            )),
            grouped_tail: None,
        }));
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::Unproven));
    }

    /// A slot that did not prove itself leaves the shape unestablished, and `Unproven` is
    /// refused: an unproven leading sumti never takes an extent away from the baseline route.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unproven_leading_sumti_is_refused() {
        let sumti = recovered::SumtiSyntax {
            base_sumti: Arc::new(recovered::Recovered::Error(recovery_placeholder())),
            vuho_attachment: None,
        };
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::Unproven));
    }

    /// A parsed VUhO is an exp-only discriminator only after the extent's core proved valid.
    /// Recovery-model occupancy outside an unproven core must not bypass the fail-closed rule.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_vuho_does_not_prove_an_unproven_core() {
        let attachment =
            recovered::VuhoSumtiAttachmentTailSyntax::ExperimentalBareVuhoSumtiAttachmentTail(
                recovered::Recovered::valid(
                    recovered::ExperimentalBareVuhoSumtiAttachmentTailSyntax(
                        recovered::WithFreeModifiers {
                            value: recovered::Recovered::valid(one_token("vu'o")),
                            free_modifiers: Vec::new(),
                        },
                    ),
                ),
            );
        let sumti = recovered::SumtiSyntax {
            base_sumti: Arc::new(recovered::Recovered::Error(recovery_placeholder())),
            vuho_attachment: Some(recovered::Recovered::valid(attachment)),
        };

        assert!(!exp_only_discriminator_is_proven(&sumti));
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::Unproven));
    }

    /// A `Prefix` wrapper carries a value but only after repaired input, so it does not
    /// establish the extent's shape either.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prefix_wrapped_leading_sumti_is_unproven() {
        let grouped = match recovered_bare_pro_sumti().base_sumti.as_ref() {
            recovered::Recovered::Valid(grouped) => grouped.as_ref().clone(),
            _ => panic!("constructed value is valid"),
        };
        let sumti = recovered::SumtiSyntax {
            base_sumti: Arc::new(recovered::Recovered::Prefix(jbotci_tree::RecoveredPrefix {
                errors: vec1![recovery_placeholder()],
                value: Box::new(grouped),
            })),
            vuho_attachment: None,
        };
        assert_eq!(recovered_origin(&sumti), ExpLeadingSumtiOrigin::Unproven);
        assert!(origin_is_rejected(ExpLeadingSumtiOrigin::Unproven));
    }
}
