//! The description/quantifier operand tier boundary (epoch 9, #552 and #837 SUM-02).
//!
//! jbotci's `sumti_base` is camxes `sumti_6` PLUS two `sumti_5`-tier arms:
//! `descriptor_without_gadri_sumti` is camxes `sumti_5` arm 2 (`quantifier selbri KU`), and
//! `descriptor_with_outer_quantifier_sumti` is a specialization of `sumti_5` arm 1
//! (`quantifier? sumti_6`).  Two consumers spell a camxes `sumti_6` and must therefore never
//! reach a quantifier-bearing form, in any dialect, at any warning level:
//!
//! - the LEADING element of a description tail (`description_tail_sumti`), which camxes spells
//!   `sumti_tail <- (sumti_6 relative_clauses?)? sumti_tail_1` (camxes.peg:156) — a VEI
//!   quantifier reaching it is #552;
//! - the operand of an outer quantifier (`quantified_sumti`), which camxes spells
//!   `sumti_5 <- quantifier? sumti_6 relative_clauses?` (camxes.peg:150) — a stacked
//!   quantifier-bearing operand reaching it is #837 SUM-02, plus the third stacking shape
//!   `quantifier` over `descriptor_without_gadri_sumti` that #837's prose does not name.
//!
//! The restriction is expressed once, as the named `description_leading_operand` rule, and
//! consumed by name at both sites.  This module carries its classifier.  The test is a
//! structural exclusion over a WHOLE COMPLETED candidate — not a first-token test, not a
//! lookahead, not a spelling test — which is what #552 requires: the epoch also deletes the one
//! first-token heuristic that used to guard the leading operand, `description_tail_sumti`'s
//! `assert !pa_word()`.
//!
//! The answer is three-valued rather than boolean because the recovered spine has a state the
//! strict spine does not: a candidate at which no arm was selected at all.  "Did not parse" is
//! not "known to be a permitted tier", and both consumers read the answer as permission, so the
//! rejection fires on `Sumti5` AND on `Unproven`.  An unproven candidate never occupies a
//! restricted `sumti_6` slot.
//!
//! Both matches are exhaustive and `..`-free over the generated `SumtiBaseSyntax` sum, so a
//! future arm is a compile error rather than a silent classification.  That is the mechanism
//! working: the epoch's two new descriptor arms had to be classified here before they could
//! compile, and both are `sumti_6`-tier descriptor forms, so both are permitted.

use std::sync::OnceLock;

use bityzba::{contract_trait, invariant, requires};
use jbotci_tree::TreeVisitor;

use super::generated_model::{SumtiBaseSyntax, recovered};
use super::generated_runtime::{OutputRejection, output_rejection_site};

/// The camxes operand tier a completed `sumti_base` candidate belongs to.
///
/// `Unproven` is reachable only on the recovered spine, where the candidate's wrapper carries no
/// selected arm.  It is deliberately a distinct value from both proven tiers: a consumer that
/// reads the answer as permission must be able to tell "known permitted" from "not known".
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SumtiOperandTier {
    /// A camxes `sumti_6` form: permitted at a restricted operand site.
    Sumti6,
    /// A camxes `sumti_5`-tier quantifier-bearing form: never permitted at a restricted site.
    Sumti5,
    /// No arm was selected, so neither tier is established.  Treated as not permitted.
    Unproven,
}

/// Classify a strict candidate.  A strict parse always selected an arm, so the answer is proven.
#[requires(true)]
#[ensures(ret != SumtiOperandTier::Unproven, "a strict candidate always selected an arm")]
pub(crate) fn sumti_base_tier(candidate: &SumtiBaseSyntax) -> SumtiOperandTier {
    match candidate {
        SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(_)
        | SumtiBaseSyntax::DescriptorWithoutGadriSumti(_) => SumtiOperandTier::Sumti5,
        SumtiBaseSyntax::ScalarNegatedSumtiWithBo(_)
        | SumtiBaseSyntax::ScalarNegatedSumti(_)
        | SumtiBaseSyntax::LaheSumti(_)
        | SumtiBaseSyntax::LaheTermWrapper(_)
        | SumtiBaseSyntax::ScalarNegatedTermWrapperWithBo(_)
        | SumtiBaseSyntax::ScalarNegatedTermWrapper(_)
        | SumtiBaseSyntax::BridiDescriptionSumti(_)
        | SumtiBaseSyntax::NameSumti(_)
        | SumtiBaseSyntax::DescriptorWithGadriSumti(_)
        | SumtiBaseSyntax::ExpDescriptorWithLeadingSumtiSumti(_)
        | SumtiBaseSyntax::ZantufaDescriptorWithRelativesFirstSumti(_)
        | SumtiBaseSyntax::NumberSumti(_)
        | SumtiBaseSyntax::LerfuStringSumti(_)
        | SumtiBaseSyntax::QuotedSumti(_)
        | SumtiBaseSyntax::ProSumti(_) => SumtiOperandTier::Sumti6,
    }
}

/// Classify the arm of a recovered candidate whose wrapper proved a selection.
#[requires(true)]
#[ensures(ret != SumtiOperandTier::Unproven, "a selected arm always establishes a tier")]
pub(crate) fn recovered_sumti_base_tier(
    candidate: &recovered::SumtiBaseSyntax,
) -> SumtiOperandTier {
    match candidate {
        recovered::SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(_)
        | recovered::SumtiBaseSyntax::DescriptorWithoutGadriSumti(_) => SumtiOperandTier::Sumti5,
        recovered::SumtiBaseSyntax::ScalarNegatedSumtiWithBo(_)
        | recovered::SumtiBaseSyntax::ScalarNegatedSumti(_)
        | recovered::SumtiBaseSyntax::LaheSumti(_)
        | recovered::SumtiBaseSyntax::LaheTermWrapper(_)
        | recovered::SumtiBaseSyntax::ScalarNegatedTermWrapperWithBo(_)
        | recovered::SumtiBaseSyntax::ScalarNegatedTermWrapper(_)
        | recovered::SumtiBaseSyntax::BridiDescriptionSumti(_)
        | recovered::SumtiBaseSyntax::NameSumti(_)
        | recovered::SumtiBaseSyntax::DescriptorWithGadriSumti(_)
        | recovered::SumtiBaseSyntax::ExpDescriptorWithLeadingSumtiSumti(_)
        | recovered::SumtiBaseSyntax::ZantufaDescriptorWithRelativesFirstSumti(_)
        | recovered::SumtiBaseSyntax::NumberSumti(_)
        | recovered::SumtiBaseSyntax::LerfuStringSumti(_)
        | recovered::SumtiBaseSyntax::QuotedSumti(_)
        | recovered::SumtiBaseSyntax::ProSumti(_) => SumtiOperandTier::Sumti6,
    }
}

/// Classify a recovered candidate.
///
/// Only a `Valid` wrapper proves that an arm was selected over the whole candidate.  An `Error`
/// wrapper carries no value at all, and a `Prefix` wrapper's value was reached only after the
/// runtime skipped or synthesized input at the top of the candidate, so neither establishes the
/// tier of the extent the site would consume.  Both are `Unproven`, which is the fail-closed
/// answer at a restricted site.
#[requires(true)]
#[ensures(
    (ret != SumtiOperandTier::Unproven) == matches!(candidate, recovered::Recovered::Valid(_)),
    "a tier is established exactly when the wrapper proved an arm was selected"
)]
fn recovered_tier(
    candidate: &recovered::Recovered<recovered::SumtiBaseSyntax>,
) -> SumtiOperandTier {
    match candidate {
        recovered::Recovered::Valid(candidate) => recovered_sumti_base_tier(candidate),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => {
            SumtiOperandTier::Unproven
        }
    }
}

/// Whether the restricted-operand classifier trace is switched on.
///
/// # `JBOTCI_TRACE_SUMTI_OPERAND_TIER`
///
/// Set the environment variable to any non-empty value to print one line to stderr for every
/// RECOVERED classification the `description_leading_operand` route performs:
///
/// ```text
/// sumti-operand-tier site=description_tail_sumti bytes=3..9 wrapper=valid tier=Sumti6 decision=permit
/// ```
///
/// - `site` is the enclosing generated rule that consumed the restricted operand -- one of the
///   two restricted field sites, `description_tail_sumti` or `quantified_sumti` -- read from the
///   parser's active rule stack, which the classifier cannot otherwise see;
/// - `bytes` is the source extent the classified candidate covers, `empty` when it covers none;
/// - `wrapper` is the recovered wrapper shape the site was handed, `valid` / `prefix` / `error`;
/// - `tier` is this module's three-valued answer and `decision` is what the site does with it.
///
/// Use it when a fixture's recovered parse moves owner, diagnostics or recovery items and the
/// question is whether either restricted site was reached at all and with what answer. That
/// attribution is exactly what the epoch's C-a recovered-delta enumeration records per row, and
/// it cannot be read off the rendered tree: a PERMITTED classification leaves no mark there, so
/// "permitted" and "never reached" are indistinguishable without this trace.
///
/// The strict spine is deliberately not traced. It has no `Unproven` state and no ownership
/// question to attribute: every strict candidate selected an arm.
#[requires(true)]
#[ensures(true)]
pub(crate) fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("JBOTCI_TRACE_SUMTI_OPERAND_TIER").is_some_and(|value| !value.is_empty())
    })
}

/// Collects the source extent a recovered candidate covers, recovery items included.
///
/// The endpoints are one field rather than two so that "seen nothing yet" cannot be spelled
/// half-way; every combination of the single field is a valid state.
#[invariant(true)]
struct CandidateExtentProbe {
    extent: Option<(usize, usize)>,
}

impl CandidateExtentProbe {
    #[requires(byte_start <= byte_end)]
    #[ensures(self.extent.is_some())]
    fn observe(&mut self, byte_start: usize, byte_end: usize) {
        self.extent = Some(self.extent.map_or((byte_start, byte_end), |(start, end)| {
            (start.min(byte_start), end.max(byte_end))
        }));
    }
}

impl<'tree> TreeVisitor<'tree> for CandidateExtentProbe {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let recovered::AtomRef::Token(token) = atom;
        for span in token.source_spans() {
            self.observe(span.byte_start, span.byte_end);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: jbotci_tree::RecoveryItemState + serde::Serialize>(
        &mut self,
        item: &'tree E,
    ) {
        let mut observed = Vec::new();
        item.visit_source_spans(&mut |span| observed.push((span.byte_start, span.byte_end)));
        for (byte_start, byte_end) in observed {
            self.observe(byte_start, byte_end);
        }
    }
}

/// One trace line for a recovered classification, with the consumer that asked for it.
#[requires(true)]
#[ensures(true)]
fn trace_recovered_classification(
    candidate: &recovered::Recovered<recovered::SumtiBaseSyntax>,
    tier: SumtiOperandTier,
    rejected: bool,
) {
    // The alias carrying the refinement is itself a rule, so it sits on top of the stack; the
    // consumer is the innermost frame below it.
    let site = output_rejection_site(|frames| {
        frames
            .iter()
            .rev()
            .find(|rule| **rule != "description_leading_operand")
            .copied()
            .unwrap_or("<unknown>")
    });
    let mut extent = CandidateExtentProbe { extent: None };
    recovered::TreeNode::visit_in_order(candidate, &mut extent);
    let bytes = extent.extent.map_or_else(
        || "empty".to_owned(),
        |(start, end)| format!("{start}..{end}"),
    );
    let wrapper = match candidate {
        recovered::Recovered::Valid(_) => "valid",
        recovered::Recovered::Prefix(_) => "prefix",
        recovered::Recovered::Error(_) => "error",
    };
    let decision = if rejected { "reject" } else { "permit" };
    eprintln!(
        "sumti-operand-tier site={site} bytes={bytes} wrapper={wrapper} tier={tier:?} decision={decision}"
    );
}

/// Whether a classified tier is refused at a restricted `sumti_6` operand site.
///
/// Written as an exhaustive match rather than an inequality so that a future tier value has to
/// answer this question for itself.
#[requires(true)]
#[ensures(ret == (tier != SumtiOperandTier::Sumti6))]
fn tier_is_rejected(tier: SumtiOperandTier) -> bool {
    match tier {
        SumtiOperandTier::Sumti6 => false,
        SumtiOperandTier::Sumti5 | SumtiOperandTier::Unproven => true,
    }
}

/// Refuses the `sumti_5`-tier arms of `sumti_base` at the two restricted operand sites.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuantifierBearingSumtiRejection;

#[contract_trait]
impl OutputRejection<SumtiBaseSyntax> for QuantifierBearingSumtiRejection {
    fn rejected_name(&self) -> &'static str {
        "quantifier-bearing sumti operand"
    }

    fn rejects(&self, value: &SumtiBaseSyntax) -> bool {
        tier_is_rejected(sumti_base_tier(value))
    }
}

#[contract_trait]
impl OutputRejection<recovered::Recovered<recovered::SumtiBaseSyntax>>
    for QuantifierBearingSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "quantifier-bearing sumti operand"
    }

    fn rejects(&self, value: &recovered::Recovered<recovered::SumtiBaseSyntax>) -> bool {
        let tier = recovered_tier(value);
        let rejected = tier_is_rejected(tier);
        if trace_enabled() {
            trace_recovered_classification(value, tier, rejected);
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

    /// The recovered pro-sumti `mi`: the simplest permitted camxes `sumti_6` arm.
    #[requires(true)]
    #[ensures(true)]
    fn recovered_pro_sumti() -> recovered::SumtiBaseSyntax {
        let words = segment_words_with_modifiers("mi").expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let [token] = tokens.as_slice() else {
            panic!("`mi` must be exactly one word");
        };
        recovered::SumtiBaseSyntax::ProSumti(recovered::Recovered::valid(
            recovered::ProSumtiSyntax(recovered::WithFreeModifiers {
                value: recovered::Recovered::valid(token.clone()),
                free_modifiers: Vec::new(),
            }),
        ))
    }

    /// A permitted arm under a `Valid` wrapper is the only shape the restricted sites accept.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_sumti_six_arm_is_permitted() {
        let candidate = recovered::Recovered::valid(recovered_pro_sumti());
        assert_eq!(recovered_tier(&candidate), SumtiOperandTier::Sumti6);
        assert!(!QuantifierBearingSumtiRejection.rejects(&candidate));
    }

    /// A wrapper that selected no arm at all is `Unproven`, and `Unproven` is refused: "did not
    /// parse" is not "known to be a permitted tier", so an unproven candidate never occupies a
    /// restricted `sumti_6` slot.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn error_wrapper_is_unproven_and_refused() {
        let candidate: recovered::Recovered<recovered::SumtiBaseSyntax> =
            recovered::Recovered::Error(recovery_placeholder());
        assert_eq!(recovered_tier(&candidate), SumtiOperandTier::Unproven);
        assert!(QuantifierBearingSumtiRejection.rejects(&candidate));
    }

    /// A `Prefix` wrapper reached its value only after the runtime repaired input at the top of
    /// the candidate, so it does not establish the tier of the extent the site would consume --
    /// even when the value it carries is itself a permitted arm.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prefix_wrapper_is_unproven_and_refused_even_over_a_permitted_arm() {
        let candidate = recovered::Recovered::Prefix(jbotci_tree::RecoveredPrefix {
            errors: vec1![recovery_placeholder()],
            value: Box::new(recovered_pro_sumti()),
        });
        assert_eq!(recovered_tier(&candidate), SumtiOperandTier::Unproven);
        assert!(QuantifierBearingSumtiRejection.rejects(&candidate));
    }

    /// The two `sumti_5`-tier arms are the only arms the classifier refuses, and it refuses them
    /// on the strict spine as well as the recovered one.  The parser-level consequence is pinned
    /// by the epoch's acceptance witnesses; this pins the classification itself.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn only_the_two_sumti_five_arms_are_refused() {
        assert!(tier_is_rejected(SumtiOperandTier::Sumti5));
        assert!(tier_is_rejected(SumtiOperandTier::Unproven));
        assert!(!tier_is_rejected(SumtiOperandTier::Sumti6));
    }
}
