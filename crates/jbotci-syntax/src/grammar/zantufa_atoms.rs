//! Source-shaped construction and eligibility for the Zantufa atom family.
//!
//! Parser alternatives may differ to put a diagnostic on a statically known
//! token, but those alternatives do not create public warning-only variants.

use std::sync::{Arc, OnceLock};

use bityzba::{ensures, invariant, requires};
use jbotci_morphology::Selmaho;

use super::generated_model::{
    self as model, FreeModifierSyntax, ZantufaAtomGaOpenerSyntax, recovered,
};
use super::generated_runtime::output_rejection_site;
use crate::Token;
use crate::tree::WithFreeModifiers;

/// A word clause exactly as a `.wf()` parser alternative yields it.
///
/// The generated model stores a *shared* free modifier, so this parser-side
/// shape and the stored shape are different types; `store_clause` is the only
/// conversion between them, mirroring the generator's own containment
/// lowering.
type TokenClause = WithFreeModifiers<Token, FreeModifierSyntax>;

/// A word clause in the shape the generated model stores.
type StoredTokenClause = WithFreeModifiers<Token, Arc<FreeModifierSyntax>>;

/// Lower a parser-produced word clause into the stored shape.
#[requires(true)]
#[ensures(ret.value == old(clause.value.clone()))]
#[ensures(ret.free_modifiers.len() == old(clause.free_modifiers.len()))]
fn store_clause(clause: TokenClause) -> StoredTokenClause {
    let WithFreeModifiers {
        value,
        free_modifiers,
    } = clause;
    WithFreeModifiers::new(value, free_modifiers.into_iter().map(Arc::new).collect())
}

/// Lower an optional parser-produced word clause into the stored shape.
#[requires(true)]
#[ensures(ret.is_some() == old(clause.is_some()))]
fn store_opt_clause(clause: Option<TokenClause>) -> Option<StoredTokenClause> {
    clause.map(store_clause)
}

/// A recovered word clause exactly as a `.wf()` parser alternative yields it.
type RecoveredTokenClause = recovered::WithFreeModifiers<recovered::Recovered<Token>>;

/// A recovered word clause in the shape the generated model stores.
///
/// Recovery has already happened when containment shares a node, so the shared
/// value is the whole `Recovered`, prefix and error information included.
type StoredRecoveredTokenClause = recovered::WithFreeModifiers<
    recovered::Recovered<Token>,
    Arc<recovered::Recovered<recovered::FreeModifierSyntax>>,
>;

/// Lower a parser-produced recovered word clause into the stored shape.
#[requires(true)]
#[ensures(ret.value == old(clause.value.clone()))]
#[ensures(ret.free_modifiers.len() == old(clause.free_modifiers.len()))]
fn store_recovered_clause(clause: RecoveredTokenClause) -> StoredRecoveredTokenClause {
    let recovered::WithFreeModifiers {
        value,
        free_modifiers,
    } = clause;
    recovered::WithFreeModifiers {
        value,
        free_modifiers: free_modifiers.into_iter().map(Arc::new).collect(),
    }
}

/// Lower an optional parser-produced recovered word clause into the stored shape.
#[requires(true)]
#[ensures(ret.is_some() == old(clause.is_some()))]
fn store_opt_recovered_clause(
    clause: Option<RecoveredTokenClause>,
) -> Option<StoredRecoveredTokenClause> {
    clause.map(store_recovered_clause)
}

/// Whether the Zantufa atom-ownership classifier trace is switched on.
///
/// # `JBOTCI_TRACE_ZANTUFA_ATOMS`
///
/// Set the environment variable to any non-empty value to print one line to stderr for every
/// RECOVERED ownership classification the C-e atom family performs:
///
/// ```text
/// zantufa-atom site=tanru_unit_atom_base candidate=fa wrapper=valid bytes=9..23 answer=Present decision=accept
/// ```
///
/// - `site` is the enclosing generated rule that consumed the candidate, read from the parser's
///   active rule stack, which the classifier cannot otherwise see;
/// - `candidate` is which of the six C-e ownership questions ran: `fa`, `standalone-gek`,
///   `enclosed-gek`, `grouped-sumti`, `priority-selbri` or `priority-tail`. `grouped-sumti` is
///   the one whose absence was itself a defect: until it gained a classifier, a recovered `KE`
///   with a synthesized body could claim the construct on no evidence;
/// - `wrapper` is the recovered wrapper the classifier was handed -- `valid` / `prefix` / `error`,
///   or `unwrapped` where the generated route hands over the bare product;
/// - `bytes` is the source extent the classified candidate covers, `empty` when it covers none;
/// - `answer` is the three-valued presence answer and `decision` is what the route does with it.
///
/// Use it when attributing a recovered ownership row: a rejected candidate leaves nothing at all
/// in the rendered tree, so "rejected as Absent", "rejected as Unproven" and "never reached" are
/// indistinguishable without this trace. That attribution is exactly what the epoch's frozen
/// winning-recovery rows record per row.
///
/// The strict spine is deliberately not traced: a strict candidate that is admitted is visible in
/// the resulting tree, and strict classification has no recovery uncertainty to attribute.
#[requires(true)]
#[ensures(true)]
pub(crate) fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("JBOTCI_TRACE_ZANTUFA_ATOMS").is_some_and(|value| !value.is_empty())
    })
}

/// Which of the C-e ownership questions a traced classification answered.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TracedCandidate {
    Fa,
    StandaloneGek,
    EnclosedGek,
    GroupedSumti,
    PrioritySelbri,
    PriorityTail,
}

impl TracedCandidate {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn name(self) -> &'static str {
        match self {
            Self::Fa => "fa",
            Self::StandaloneGek => "standalone-gek",
            Self::EnclosedGek => "enclosed-gek",
            Self::GroupedSumti => "grouped-sumti",
            Self::PrioritySelbri => "priority-selbri",
            Self::PriorityTail => "priority-tail",
        }
    }
}

/// The recovered wrapper a classifier was handed, including the unwrapped generated route.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
enum TracedWrapper {
    Unwrapped,
    Valid,
    Prefix,
    Error,
}

impl TracedWrapper {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn name(self) -> &'static str {
        match self {
            Self::Unwrapped => "unwrapped",
            Self::Valid => "valid",
            Self::Prefix => "prefix",
            Self::Error => "error",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn of<T>(value: &recovered::Recovered<T>) -> Self {
        match value {
            recovered::Recovered::Valid(_) => Self::Valid,
            recovered::Recovered::Prefix(_) => Self::Prefix,
            recovered::Recovered::Error(_) => Self::Error,
        }
    }
}

/// One trace line for a recovered classification, with the consumer that asked for it.
#[requires(true)]
#[ensures(true)]
fn trace_recovered_classification(
    candidate: TracedCandidate,
    wrapper: TracedWrapper,
    node: &impl recovered::TreeNode,
    answer: ZantufaTanruAtomPresence,
) {
    // The alias carrying the refinement is itself a rule and sits on top of the stack; the
    // consumer that asked the ownership question is the innermost frame below all of them.
    let site = output_rejection_site(|frames| {
        frames
            .iter()
            .rev()
            .find(|rule| !rule.ends_with("_candidate"))
            .copied()
            .unwrap_or("<unknown>")
    });
    let bytes = super::generated_runtime::recovered_source_extent(node).map_or_else(
        || "empty".to_owned(),
        |(start, end)| format!("{start}..{end}"),
    );
    let decision = if answer == ZantufaTanruAtomPresence::Present {
        "accept"
    } else {
        "reject"
    };
    let candidate = candidate.name();
    let wrapper = wrapper.name();
    eprintln!(
        "zantufa-atom site={site} candidate={candidate} wrapper={wrapper} bytes={bytes} answer={answer:?} decision={decision}"
    );
}

/// Classify a wrapped recovered candidate, tracing the answer when the trace is on.
///
/// An incomplete wrapper is fail-closed `Unproven` by construction: a candidate that did not
/// complete cannot have proven any discriminator, so the inner classifier is not consulted.
#[requires(true)]
#[ensures(ret == ZantufaTanruAtomPresence::Present -> matches!(value, recovered::Recovered::Valid(_)))]
fn classify_recovered_wrapper<T: recovered::TreeNode>(
    candidate: TracedCandidate,
    value: &recovered::Recovered<T>,
    classify: impl FnOnce(&T) -> ZantufaTanruAtomPresence,
) -> ZantufaTanruAtomPresence {
    let answer = match value {
        recovered::Recovered::Valid(value) => classify(value.as_ref()),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => {
            ZantufaTanruAtomPresence::Unproven
        }
    };
    if trace_enabled() {
        trace_recovered_classification(candidate, TracedWrapper::of(value), value, answer);
    }
    answer
}

/// Classify an unwrapped recovered candidate, tracing the answer when the trace is on.
#[requires(true)]
#[ensures(true)]
fn classify_recovered_product<T: recovered::TreeNode>(
    candidate: TracedCandidate,
    value: &T,
    classify: impl FnOnce(&T) -> ZantufaTanruAtomPresence,
) -> ZantufaTanruAtomPresence {
    let answer = classify(value);
    if trace_enabled() {
        trace_recovered_classification(candidate, TracedWrapper::Unwrapped, value, answer);
    }
    answer
}

/// Recovery may not hand the FA atom a claim it did not parse.
///
/// Recovered-only: the strict FA product proves its own inventory by construction (see
/// [`strict_fa_presence`]), so the grammar applies this through `reject_recovered_output()`.
#[invariant(true)]
#[derive(Clone, Copy)]
pub(crate) struct FaAtomRejection;

#[bityzba::contract_trait]
impl super::generated_runtime::RecoveredOutputRejection<recovered::ZantufaFaTanruUnitSyntax>
    for FaAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven source FA atom"
    }
    fn rejects_uncertain(&self, value: &recovered::ZantufaFaTanruUnitSyntax) -> bool {
        classify_recovered_product(TracedCandidate::Fa, value, recovered_fa_presence)
            != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl
    super::generated_runtime::RecoveredOutputRejection<
        recovered::Recovered<recovered::ZantufaFaTanruUnitSyntax>,
    > for FaAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven source FA atom"
    }
    fn rejects_uncertain(
        &self,
        value: &recovered::Recovered<recovered::ZantufaFaTanruUnitSyntax>,
    ) -> bool {
        classify_recovered_wrapper(TracedCandidate::Fa, value, recovered_fa_presence)
            != ZantufaTanruAtomPresence::Present
    }
}

/// A strict FA product is present by construction.
///
/// `zantufa_fa_tanru_unit` takes its markers from `selmaho(Fa)` and its connectives from
/// `zantufa_atom_joik`, whose slots are exactly the source JOIK inventory (GAhO, NA, SE and a
/// JOI/JA/BIhI head), and the parser matches a class through the same `Token::cmavo()` these
/// predicates read. The inventory is therefore a precondition here, not something to prove.
/// FA uses the source JOIK inventory, not the narrower modifier-free BINARY GEK ownership table;
/// free modifiers do not by themselves disqualify FA.
#[requires(true)]
#[bityzba::expensive_requires(value.fa.value.is_selmaho(Selmaho::Fa) && value.continuations.iter().all(|part| {
    let joik = &part.connective;
    part.fa.value.is_selmaho(Selmaho::Fa)
        && joik.left_gaho.as_ref().is_none_or(|v| v.value.is_selmaho(Selmaho::Gaho))
        && joik.na.as_ref().is_none_or(|v| v.value.is_selmaho(Selmaho::Na))
        && joik.se.as_ref().is_none_or(|v| v.value.is_selmaho(Selmaho::Se))
        && joik.head.value.is_one_of_selmaho(&[Selmaho::Joi, Selmaho::Ja, Selmaho::Bihi])
        && joik.right_gaho.as_ref().is_none_or(|v| v.value.is_selmaho(Selmaho::Gaho))
}))]
#[ensures(ret == ZantufaTanruAtomPresence::Present)]
fn strict_fa_presence(value: &model::ZantufaFaTanruUnitSyntax) -> ZantufaTanruAtomPresence {
    ZantufaTanruAtomPresence::Present
}

#[requires(true)]
#[ensures(true)]
fn recovered_fa_presence(value: &recovered::ZantufaFaTanruUnitSyntax) -> ZantufaTanruAtomPresence {
    use ZantufaTanruAtomPresence::{Absent, Present, Unproven};
    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(value, &mut evidence);
    if evidence.uncertainty || !evidence.parsed_token {
        return Unproven;
    }
    let matches = |value: &StoredRecoveredTokenClause, selmaho| {
        parsed_value(&value.value).is_some_and(|token| token.is_selmaho(selmaho))
    };
    if !matches(&value.fa, Selmaho::Fa) {
        return Absent;
    }
    for part in &value.continuations {
        let Some(part) = parsed_value(part) else {
            return Unproven;
        };
        let Some(joik) = parsed_value(&part.connective) else {
            return Unproven;
        };
        if !matches(&part.fa, Selmaho::Fa)
            || !joik
                .left_gaho
                .as_ref()
                .is_none_or(|v| matches(v, Selmaho::Gaho))
            || !joik.na.as_ref().is_none_or(|v| matches(v, Selmaho::Na))
            || !joik.se.as_ref().is_none_or(|v| matches(v, Selmaho::Se))
            || ![Selmaho::Joi, Selmaho::Ja, Selmaho::Bihi]
                .iter()
                .any(|s| matches(&joik.head, *s))
            || !joik
                .right_gaho
                .as_ref()
                .is_none_or(|v| matches(v, Selmaho::Gaho))
        {
            return Absent;
        }
    }
    Present
}

/// The grouped-sumti owner has to prove its own body, exactly as the atom family does.
///
/// Without this the grouped product is the one C-e owner that could win on no evidence at all: a
/// recovered `KE` with a synthesized body still completes, so `ke ke'e` alone would claim the
/// construct and report its warning while owning nothing.
///
/// Recovered-only: a strict grouped sumti has its `cmavo(Ke)` opener and a complete `arc(sumti)`
/// body by construction, so the grammar applies this through `reject_recovered_output()`.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct GroupedSumtiRejection;

/// Fail closed: a grouped sumti owns its extent only with a proven `KE` and a proven body.
///
/// Recovery can synthesize the mandatory body, so an occupied field is not evidence; the body
/// must carry a parsed value, and any recovery item anywhere under the candidate makes the whole
/// claim unproven, as in every other C-e classifier.
#[requires(true)]
#[ensures(ret == ZantufaTanruAtomPresence::Present -> parsed_value(value.sumti.as_ref()).is_some())]
fn recovered_grouped_sumti_presence(
    value: &recovered::ZantufaGroupedSumtiSyntax,
) -> ZantufaTanruAtomPresence {
    use ZantufaTanruAtomPresence::{Absent, Present, Unproven};

    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(value, &mut evidence);
    if evidence.uncertainty || !evidence.parsed_token {
        return Unproven;
    }
    let Some(ke) = parsed_value(&value.ke.value) else {
        return Unproven;
    };
    if !ke.is_cmavo(jbotci_morphology::Cmavo::Ke) {
        return Absent;
    }
    if parsed_value(value.sumti.as_ref()).is_none() {
        return Unproven;
    }
    Present
}

#[bityzba::contract_trait]
impl super::generated_runtime::RecoveredOutputRejection<recovered::ZantufaGroupedSumtiSyntax>
    for GroupedSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven Zantufa grouped sumti"
    }
    fn rejects_uncertain(&self, value: &recovered::ZantufaGroupedSumtiSyntax) -> bool {
        classify_recovered_product(
            TracedCandidate::GroupedSumti,
            value,
            recovered_grouped_sumti_presence,
        ) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl
    super::generated_runtime::RecoveredOutputRejection<
        recovered::Recovered<recovered::ZantufaGroupedSumtiSyntax>,
    > for GroupedSumtiRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven Zantufa grouped sumti"
    }
    fn rejects_uncertain(
        &self,
        value: &recovered::Recovered<recovered::ZantufaGroupedSumtiSyntax>,
    ) -> bool {
        classify_recovered_wrapper(
            TracedCandidate::GroupedSumti,
            value,
            recovered_grouped_sumti_presence,
        ) != ZantufaTanruAtomPresence::Present
    }
}

/// A complete CoSelbri may take priority only through an admitted atom in its
/// predicate domain. Nested statements, arguments, tags and free modifiers are
/// not evidence that their containing ordinary predicate changes ownership.
#[invariant(true)]
struct PriorityAtomEvidence {
    dialect: super::generated_runtime::SyntaxGrammarDialect,
    answer: ZantufaTanruAtomPresence,
}

impl PriorityAtomEvidence {
    #[requires(true)]
    #[ensures(self.answer == old(self.answer).combine(answer))]
    fn observe(&mut self, answer: ZantufaTanruAtomPresence) {
        self.answer = self.answer.combine(answer);
    }
}

macro_rules! priority_domain_walker {
    ($model:ident, $classify:ident, $classify_fa:ident) => {
        impl<'tree> $model::TreeWalker<'tree> for PriorityAtomEvidence {
            #[requires(true)]
            #[ensures(true)]
            fn walk_zantufa_fa_tanru_unit(
                &mut self,
                node: &'tree $model::ZantufaFaTanruUnitSyntax,
            ) {
                self.observe($classify_fa(node));
                $model::walk::zantufa_fa_tanru_unit(self, node);
            }
            #[requires(true)]
            #[ensures(true)]
            fn walk_zantufa_forethought_tanru_unit(
                &mut self,
                node: &'tree $model::ZantufaForethoughtTanruUnitSyntax,
            ) {
                self.observe($classify(node, &self.dialect));
                $model::walk::zantufa_forethought_tanru_unit(self, node);
            }
            #[requires(true)]
            #[ensures(true)]
            fn walk_free_modifier(&mut self, _node: &'tree $model::FreeModifierSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_subbridi(&mut self, _node: &'tree $model::SubbridiSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_sumti(&mut self, _node: &'tree $model::SumtiSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_term(&mut self, _node: &'tree $model::TermSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_tense_modal(&mut self, _node: &'tree $model::TenseModalSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_mekso(&mut self, _node: &'tree $model::MeksoSyntax) {}
            #[requires(true)]
            #[ensures(true)]
            fn walk_text(&mut self, _node: &'tree $model::TextSyntax) {}
        }
    };
}
priority_domain_walker!(model, strict_standalone_presence, strict_fa_presence);
priority_domain_walker!(
    recovered,
    recovered_standalone_presence,
    recovered_fa_presence
);

#[invariant(true)]
#[derive(Clone, Copy)]
pub(crate) struct PriorityAtomRejection;

/// The priority-atom ownership answer for a complete co-selbri on this parse axis.
///
/// The priority routes exist only on the ZantufaSelbri axis: the grammar gates each of them on
/// `feature(ZantufaSelbri)`, so no candidate reaches this on another axis.
macro_rules! priority_answer {
    ($model:ident, $name:ident, $uncertainty:block) => {
        #[requires(
                            dialect.zantufa_selbri_enabled,
                            "the grammar gates every priority route on feature(ZantufaSelbri)"
                        )]
        #[ensures(true)]
        fn $name(
            value: &$model::CoSelbriSyntax,
            dialect: super::generated_runtime::SyntaxGrammarDialect,
        ) -> ZantufaTanruAtomPresence {
            let uncertain: fn(&$model::CoSelbriSyntax) -> bool = $uncertainty;
            if uncertain(value) {
                return ZantufaTanruAtomPresence::Unproven;
            }
            let mut walker = PriorityAtomEvidence {
                dialect,
                answer: ZantufaTanruAtomPresence::Absent,
            };
            $model::TreeWalkable::walk_with(value, &mut walker);
            walker.answer
        }
    };
}
priority_answer!(model, strict_priority_presence, { |_| false });
priority_answer!(recovered, recovered_priority_presence, {
    |value| {
        let mut evidence = RequiredSubtreeEvidence::default();
        recovered::TreeNode::visit_in_order(value, &mut evidence);
        evidence.uncertainty || !evidence.parsed_token
    }
});

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<model::CoSelbriSyntax> for PriorityAtomRejection {
    fn rejected_name(&self) -> &'static str {
        "unproven Zantufa atom priority"
    }
    #[ensures(ret)]
    fn rejects(&self, _value: &model::CoSelbriSyntax) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &model::CoSelbriSyntax,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        strict_priority_presence(value, dialect) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<recovered::CoSelbriSyntax>
    for PriorityAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven Zantufa atom priority"
    }
    #[ensures(ret)]
    fn rejects(&self, _value: &recovered::CoSelbriSyntax) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &recovered::CoSelbriSyntax,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_product(TracedCandidate::PrioritySelbri, value, |value| {
            recovered_priority_presence(value, dialect)
        }) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<recovered::Recovered<recovered::CoSelbriSyntax>>
    for PriorityAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven Zantufa atom priority"
    }
    #[ensures(ret)]
    fn rejects(&self, _value: &recovered::Recovered<recovered::CoSelbriSyntax>) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &recovered::Recovered<recovered::CoSelbriSyntax>,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_wrapper(TracedCandidate::PrioritySelbri, value, |value| {
            recovered_priority_presence(value, dialect)
        }) != ZantufaTanruAtomPresence::Present
    }
}

impl From<model::CoSelbriSyntax> for model::SelbriSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: model::CoSelbriSyntax) -> Self {
        Self::UntaggedSelbri(model::UntaggedSelbriSyntax::CoSelbri(value.into()).into())
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::SelbriSyntax>>
    for recovered::Recovered<recovered::CoSelbriSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::SelbriSyntax> {
        recovered::Recovered::valid(recovered::SelbriSyntax::UntaggedSelbri(Arc::new(
            recovered::Recovered::valid(recovered::UntaggedSelbriSyntax::CoSelbri(Arc::new(self))),
        )))
    }
}

/// Refine the completed ordinary tail instead of instantiating its memoized
/// product rule with a different selbri argument. Both attempts therefore
/// share exactly the same recognition language and cached product; only this
/// outer, tree-transparent eligibility check differs.
#[invariant(true)]
#[derive(Clone, Copy)]
pub(crate) struct PriorityTailRejection;

#[requires(true)]
#[ensures(true)]
fn strict_priority_tail_selbri(
    value: &model::SelbriSyntax,
    dialect: super::generated_runtime::SyntaxGrammarDialect,
) -> bool {
    use super::generated_runtime::OutputRejection;
    match value {
        model::SelbriSyntax::UntaggedSelbri(value) => {
            #[requires(true)]
            #[ensures(true)]
            fn has_priority(
                value: &model::UntaggedSelbriSyntax,
                dialect: super::generated_runtime::SyntaxGrammarDialect,
            ) -> bool {
                match value {
                    model::UntaggedSelbriSyntax::CoSelbri(value) => {
                        let value: &model::CoSelbriSyntax = value;
                        !PriorityAtomRejection.rejects_in_dialect(value, dialect)
                    }
                    model::UntaggedSelbriSyntax::NegatedSelbri(_) => false,
                }
            }
            has_priority(value, dialect)
        }
        model::SelbriSyntax::ReinterpretZantufaAssignedSelbri(_)
        | model::SelbriSyntax::ZantufaRelativeSelbri(_)
        | model::SelbriSyntax::ZantufaPriorityAssignedSelbri(_)
        | model::SelbriSyntax::TaggedSelbri(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_priority_tail_selbri(
    value: &recovered::SelbriSyntax,
    dialect: super::generated_runtime::SyntaxGrammarDialect,
) -> bool {
    use super::generated_runtime::OutputRejection;
    match value {
        recovered::SelbriSyntax::UntaggedSelbri(value) => {
            let Some(value) = parsed_value(value) else {
                return false;
            };
            match value {
                recovered::UntaggedSelbriSyntax::CoSelbri(value) => {
                    !PriorityAtomRejection.rejects_in_dialect(value.as_ref(), dialect)
                }
                recovered::UntaggedSelbriSyntax::NegatedSelbri(_) => false,
            }
        }
        recovered::SelbriSyntax::ReinterpretZantufaAssignedSelbri(_)
        | recovered::SelbriSyntax::ZantufaRelativeSelbri(_)
        | recovered::SelbriSyntax::ZantufaPriorityAssignedSelbri(_)
        | recovered::SelbriSyntax::TaggedSelbri(_) => false,
    }
}

/// The atom-owned-tail answer for a complete recovered tail payload on this parse axis.
///
/// Uncertainty anywhere in the payload is `Unproven`: the tail is refined as a whole, so an
/// incompletely recovered child leaves the ownership question unanswered rather than answered
/// negatively. A complete payload whose selbri is simply not atom-owned is `Absent`.
#[requires(true)]
#[ensures(ret == ZantufaTanruAtomPresence::Present -> parsed_value(selbri).is_some())]
fn recovered_priority_tail_presence(
    selbri: &recovered::Recovered<recovered::SelbriSyntax>,
    payload: &impl recovered::TreeNode,
    dialect: super::generated_runtime::SyntaxGrammarDialect,
) -> ZantufaTanruAtomPresence {
    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(payload, &mut evidence);
    if evidence.uncertainty {
        return ZantufaTanruAtomPresence::Unproven;
    }
    let Some(selbri) = parsed_value(selbri) else {
        return ZantufaTanruAtomPresence::Unproven;
    };
    if recovered_priority_tail_selbri(selbri, dialect) {
        ZantufaTanruAtomPresence::Present
    } else {
        ZantufaTanruAtomPresence::Absent
    }
}

macro_rules! priority_tail_mapping {
    ($payload:ident, $target:ident, $variant:ident) => {
        #[bityzba::contract_trait]
        impl super::generated_runtime::OutputRejection<model::$payload> for PriorityTailRejection {
            fn rejected_name(&self) -> &'static str {
                "unproven atom-owned tail"
            }
            #[ensures(ret)]
            fn rejects(&self, _value: &model::$payload) -> bool {
                true
            }
            fn rejects_in_dialect(
                &self,
                value: &model::$payload,
                dialect: super::generated_runtime::SyntaxGrammarDialect,
            ) -> bool {
                !strict_priority_tail_selbri(&value.selbri, dialect)
            }
        }
        #[bityzba::contract_trait]
        impl super::generated_runtime::OutputRejection<recovered::$payload>
            for PriorityTailRejection
        {
            fn rejected_name(&self) -> &'static str {
                "unproven atom-owned tail"
            }
            #[ensures(ret)]
            fn rejects(&self, _value: &recovered::$payload) -> bool {
                true
            }
            fn rejects_in_dialect(
                &self,
                value: &recovered::$payload,
                dialect: super::generated_runtime::SyntaxGrammarDialect,
            ) -> bool {
                classify_recovered_product(TracedCandidate::PriorityTail, value, |value| {
                    recovered_priority_tail_presence(&value.selbri, value, dialect)
                }) != ZantufaTanruAtomPresence::Present
            }
        }
        #[bityzba::contract_trait]
        impl super::generated_runtime::OutputRejection<recovered::Recovered<recovered::$payload>>
            for PriorityTailRejection
        {
            fn rejected_name(&self) -> &'static str {
                "unproven atom-owned tail"
            }
            #[ensures(ret)]
            fn rejects(&self, _value: &recovered::Recovered<recovered::$payload>) -> bool {
                true
            }
            fn rejects_in_dialect(
                &self,
                value: &recovered::Recovered<recovered::$payload>,
                dialect: super::generated_runtime::SyntaxGrammarDialect,
            ) -> bool {
                classify_recovered_wrapper(TracedCandidate::PriorityTail, value, |value| {
                    recovered_priority_tail_presence(&value.selbri, value, dialect)
                }) != ZantufaTanruAtomPresence::Present
            }
        }
        impl From<model::$payload> for model::$target {
            #[requires(true)]
            #[ensures(true)]
            fn from(value: model::$payload) -> Self {
                Self::$variant(value.into())
            }
        }
        #[bityzba::contract_trait]
        impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::$target>>
            for recovered::Recovered<recovered::$payload>
        {
            fn grammar_map_to(self) -> recovered::Recovered<recovered::$target> {
                recovered::Recovered::valid(recovered::$target::$variant(Arc::new(self)))
            }
        }
    };
}
priority_tail_mapping!(
    SelbriSimpleBridiTailSyntax,
    SimpleBridiTailSyntax,
    SelbriSimpleBridiTail
);
priority_tail_mapping!(
    SelbriSimpleBridiTailWithoutTailTermsSyntax,
    SimpleBridiTailWithoutTailTermsSyntax,
    SelbriSimpleBridiTailWithoutTailTerms
);

/// Standalone ownership is a property of both the complete product and the
/// active parse axis. A context-free invocation cannot establish ownership.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct StandaloneAtomRejection;

/// Enclosed JAI has a narrower ownership policy than the standalone entry.
/// The parser must still fail closed for every uncertain or tagged product;
/// this marker keeps that policy explicit at the generated alias boundary.
///
/// Kept as an output rejection by design: it is a tree-transparent eligibility refinement over a
/// memoized product shared with another route at the same position. The enclosed and standalone
/// candidates call the same `zantufa_forethought_tanru_unit` instance, and after `jai` the
/// enclosed attempt falls back to the standalone entry at the same start, so restating the GA-only
/// opener as grammar would re-instantiate that product (losing the memo hit) and add recursive
/// handles on the atom descent path.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct EnclosedAtomRejection;

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<model::ZantufaForethoughtTanruUnitSyntax>
    for EnclosedAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unowned enclosed Zantufa atom"
    }
    #[ensures(ret)]
    fn rejects(&self, _value: &model::ZantufaForethoughtTanruUnitSyntax) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &model::ZantufaForethoughtTanruUnitSyntax,
        _dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        strict_enclosed_presence(value) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<recovered::ZantufaForethoughtTanruUnitSyntax>
    for EnclosedAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven enclosed Zantufa atom"
    }
    #[ensures(ret)]
    fn rejects(&self, _value: &recovered::ZantufaForethoughtTanruUnitSyntax) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &recovered::ZantufaForethoughtTanruUnitSyntax,
        _dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_product(TracedCandidate::EnclosedGek, value, |value| {
            enclosed_presence_from_recovered_facts(recovered_gek_facts(value))
        }) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl
    super::generated_runtime::OutputRejection<
        recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
    > for EnclosedAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven enclosed Zantufa atom"
    }
    #[ensures(ret)]
    fn rejects(
        &self,
        _value: &recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
    ) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
        _dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_wrapper(TracedCandidate::EnclosedGek, value, |value| {
            enclosed_presence_from_recovered_facts(recovered_gek_facts(value))
        }) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<model::ZantufaForethoughtTanruUnitSyntax>
    for StandaloneAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unowned standalone Zantufa atom"
    }

    #[ensures(ret)]
    fn rejects(&self, _value: &model::ZantufaForethoughtTanruUnitSyntax) -> bool {
        true
    }

    fn rejects_in_dialect(
        &self,
        value: &model::ZantufaForethoughtTanruUnitSyntax,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        strict_standalone_presence(value, &dialect) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl super::generated_runtime::OutputRejection<recovered::ZantufaForethoughtTanruUnitSyntax>
    for StandaloneAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven or unowned standalone Zantufa atom"
    }

    #[ensures(ret)]
    fn rejects(&self, _value: &recovered::ZantufaForethoughtTanruUnitSyntax) -> bool {
        true
    }

    fn rejects_in_dialect(
        &self,
        value: &recovered::ZantufaForethoughtTanruUnitSyntax,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_product(TracedCandidate::StandaloneGek, value, |value| {
            recovered_standalone_presence(value, &dialect)
        }) != ZantufaTanruAtomPresence::Present
    }
}

#[bityzba::contract_trait]
impl
    super::generated_runtime::OutputRejection<
        recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
    > for StandaloneAtomRejection
{
    fn rejected_name(&self) -> &'static str {
        "unproven or unowned standalone Zantufa atom"
    }
    #[ensures(ret)]
    fn rejects(
        &self,
        _value: &recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
    ) -> bool {
        true
    }
    fn rejects_in_dialect(
        &self,
        value: &recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>,
        dialect: super::generated_runtime::SyntaxGrammarDialect,
    ) -> bool {
        classify_recovered_wrapper(TracedCandidate::StandaloneGek, value, |value| {
            recovered_standalone_presence(value, &dialect)
        }) != ZantufaTanruAtomPresence::Present
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoikHead {
    Joi,
    Ja,
    Bihi,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GiOrder {
    Initial,
    Final,
}

/// Every combination is a source-shaped, modifier-free opener cell. This
/// key is constructed only after separate parsing/completeness checks; it
/// intentionally cannot represent unknown fields or a tag payload.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JoikOwnershipKey {
    head: JoikHead,
    order: GiOrder,
    left_gaho: bool,
    na: bool,
    se: bool,
    right_gaho: bool,
    bo: bool,
    connectives: bool,
}

impl JoikOwnershipKey {
    /// PM's measured 384-cell partition, not a general connective heuristic.
    /// BO is explicitly represented on both paths: equal ownership is a fact
    /// of this population, not permission to discard its evidence elsewhere.
    #[requires(true)]
    #[ensures(self.order == GiOrder::Initial && !self.connectives -> !ret)]
    #[ensures(self.head == JoikHead::Ja && (self.left_gaho || self.right_gaho) -> !ret)]
    fn baseline_overlaps(self) -> bool {
        match self.bo {
            false => self.overlap_for_proven_bo(),
            true => self.overlap_for_proven_bo(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn overlap_for_proven_bo(self) -> bool {
        use GiOrder::{Final, Initial};
        use JoikHead::{Bihi, Ja, Joi};
        // SE has two explicitly admitted states in every measured cell.
        // NA is likewise retained even where a baseline outer negation, not
        // the connective itself, owns those tokens.
        match (self.head, self.order, self.connectives, self.se) {
            (Joi | Bihi, Initial | Final, true, false | true) => true,
            (Joi | Ja | Bihi, Initial, false, false | true) => false,
            (Joi, Final, false, false | true)
            | (Ja, Final, false | true, false | true)
            | (Ja, Initial, true, false | true) => match self.na {
                false | true => !self.left_gaho && !self.right_gaho,
            },
            (Bihi, Final, false, false | true) => {
                match (self.left_gaho, self.na, self.right_gaho) {
                    (false, false | true, false) | (true, false, true) => true,
                    (false, false | true, true)
                    | (true, false | true, false)
                    | (true, true, true) => false,
                }
            }
        }
    }
}

/// Project the adopted lexical inventory into the closed ownership key.
#[requires(true)]
#[ensures(ret.is_some() == (token.is_selmaho(jbotci_morphology::Selmaho::Joi) || token.is_selmaho(jbotci_morphology::Selmaho::Ja) || token.is_selmaho(jbotci_morphology::Selmaho::Bihi)))]
fn joik_head(token: &Token) -> Option<JoikHead> {
    if token.is_selmaho(Selmaho::Joi) {
        Some(JoikHead::Joi)
    } else if token.is_selmaho(Selmaho::Ja) {
        Some(JoikHead::Ja)
    } else if token.is_selmaho(Selmaho::Bihi) {
        Some(JoikHead::Bihi)
    } else {
        None
    }
}

/// Extract only the adjudicated modifier-free JOIK opener key. The caller
/// separately proves BINARY shape, operands and source boundary.
#[requires(true)]
#[ensures(ret.is_some() -> joik.head.free_modifiers.is_empty() && gi.free_modifiers.is_empty())]
fn strict_joik_key(
    joik: &model::ZantufaAtomJoikSyntax,
    gi: &StoredTokenClause,
    bo: Option<&StoredTokenClause>,
    order: GiOrder,
    connectives: bool,
) -> Option<JoikOwnershipKey> {
    use jbotci_morphology::{Cmavo, Selmaho};
    let model::ZantufaAtomJoikSyntax {
        left_gaho,
        na,
        se,
        head,
        right_gaho,
    } = joik;
    for (slot, expected) in [
        (left_gaho.as_ref(), Selmaho::Gaho),
        (na.as_ref(), Selmaho::Na),
        (se.as_ref(), Selmaho::Se),
        (right_gaho.as_ref(), Selmaho::Gaho),
    ] {
        if slot.is_some_and(|clause| {
            !clause.free_modifiers.is_empty() || !clause.value.is_selmaho(expected)
        }) {
            return None;
        }
    }
    if !head.free_modifiers.is_empty()
        || !gi.free_modifiers.is_empty()
        || !gi.value.is_cmavo(Cmavo::Gi)
        || bo.is_some_and(|clause| {
            !clause.free_modifiers.is_empty() || !clause.value.is_cmavo(Cmavo::Bo)
        })
    {
        return None;
    }
    Some(JoikOwnershipKey {
        head: joik_head(&head.value)?,
        order,
        left_gaho: left_gaho.is_some(),
        na: na.is_some(),
        se: se.is_some(),
        right_gaho: right_gaho.is_some(),
        bo: bo.is_some(),
        connectives,
    })
}

/// Each key token must be Valid, not merely a parsed value under Prefix.
/// Empty structural free lists are checked on every opener field, including
/// GI and BO. Optional absence is a distinct proven state, never synthesized
/// from an Error. A caller must also prove wrappers above this JOIK product.
#[requires(true)]
#[ensures(ret.is_some() -> joik.head.free_modifiers.is_empty() && gi.free_modifiers.is_empty())]
fn recovered_joik_key(
    joik: &recovered::ZantufaAtomJoikSyntax,
    gi: &StoredRecoveredTokenClause,
    bo: Option<&StoredRecoveredTokenClause>,
    order: GiOrder,
    connectives: bool,
) -> Option<JoikOwnershipKey> {
    use jbotci_morphology::{Cmavo, Selmaho};
    let recovered::ZantufaAtomJoikSyntax {
        left_gaho,
        na,
        se,
        head,
        right_gaho,
    } = joik;
    for (slot, expected) in [
        (left_gaho.as_ref(), Selmaho::Gaho),
        (na.as_ref(), Selmaho::Na),
        (se.as_ref(), Selmaho::Se),
        (right_gaho.as_ref(), Selmaho::Gaho),
    ] {
        if slot.is_some_and(|clause| !clause.free_modifiers.is_empty()
            || !matches!(&clause.value, recovered::Recovered::Valid(token) if token.is_selmaho(expected))) {
            return None;
        }
    }
    if !head.free_modifiers.is_empty() || !gi.free_modifiers.is_empty()
        || !matches!(&gi.value, recovered::Recovered::Valid(token) if token.is_cmavo(Cmavo::Gi))
        || bo.is_some_and(|clause| !clause.free_modifiers.is_empty()
            || !matches!(&clause.value, recovered::Recovered::Valid(token) if token.is_cmavo(Cmavo::Bo)))
    { return None; }
    let recovered::Recovered::Valid(head) = &head.value else {
        return None;
    };
    Some(JoikOwnershipKey {
        head: joik_head(head)?,
        order,
        left_gaho: left_gaho.is_some(),
        na: na.is_some(),
        se: se.is_some(),
        right_gaho: right_gaho.is_some(),
        bo: bo.is_some(),
        connectives,
    })
}

/// The closed JOIK cell domain excludes GA-family and tag payloads by type.
#[requires(true)]
#[ensures(true)]
fn strict_gek_joik_key(
    gek: &model::ZantufaAtomGekSyntax,
    connectives: bool,
) -> Option<JoikOwnershipKey> {
    match gek.body.as_ref() {
        model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(_) => None,
        model::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(opener) => {
            match opener.payload.as_ref() {
                model::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(joik) => strict_joik_key(
                    joik.as_ref(),
                    &opener.gi,
                    gek.bo.as_ref(),
                    GiOrder::Initial,
                    connectives,
                ),
                model::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(_) => None,
            }
        }
        model::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(opener) => {
            match opener.payload.as_ref() {
                model::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(joik) => strict_joik_key(
                    joik.as_ref(),
                    &opener.gi,
                    gek.bo.as_ref(),
                    GiOrder::Final,
                    connectives,
                ),
                model::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(_) => None,
            }
        }
    }
}

/// A JOIK key wrapper must be Valid. In this specifically adjudicated domain
/// even a Prefix carrying a parsed value cannot establish a cell key.
#[requires(true)]
#[ensures(ret.is_some() == matches!(value.borrow(), recovered::Recovered::Valid(_)))]
fn valid_key_value<T>(
    value: &(impl std::borrow::Borrow<recovered::Recovered<T>> + ?Sized),
) -> Option<&T> {
    match value.borrow() {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(_) | recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_gek_joik_key(
    gek: &recovered::ZantufaAtomGekSyntax,
    connectives: bool,
) -> Option<JoikOwnershipKey> {
    match valid_key_value(&gek.body)? {
        recovered::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(_) => None,
        recovered::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(opener) => {
            let opener = valid_key_value(opener)?;
            match valid_key_value(&opener.payload)? {
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(joik) => {
                    recovered_joik_key(
                        valid_key_value(joik)?,
                        &opener.gi,
                        gek.bo.as_ref(),
                        GiOrder::Initial,
                        connectives,
                    )
                }
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(_) => None,
            }
        }
        recovered::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(opener) => {
            let opener = valid_key_value(opener)?;
            match valid_key_value(&opener.payload)? {
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(joik) => {
                    recovered_joik_key(
                        valid_key_value(joik)?,
                        &opener.gi,
                        gek.bo.as_ref(),
                        GiOrder::Final,
                        connectives,
                    )
                }
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(_) => None,
            }
        }
    }
}

/// Evidence for an extension-owned atom, not merely a nonempty syntax field.
///
/// All three states are valid independent results. Uncertainty in any required
/// child defeats positive evidence from another child; absence is neutral.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZantufaTanruAtomPresence {
    Present,
    Absent,
    Unproven,
}

/// Structural facts established by a complete strict GA-family product.
/// Keeping these facts separate from the standalone policy prevents callers
/// from treating a final verdict as if it were parser evidence.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StrictGekFacts {
    pub head_is_ga: bool,
    pub head_is_guha: bool,
    pub has_nahe: bool,
    pub has_bo: bool,
    pub branch_count: usize,
    pub has_gihi: bool,
    pub opener_se_free_modifiers: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveredGekFacts {
    pub uncertain: bool,
    pub head_is_ga: bool,
    pub head_is_guha: bool,
    pub has_nahe: bool,
    pub has_bo: bool,
    pub branch_count: usize,
    pub has_gihi: bool,
}

#[requires(true)]
#[ensures(true)]
fn recovered_gek_facts(
    candidate: &recovered::ZantufaForethoughtTanruUnitSyntax,
) -> RecoveredGekFacts {
    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(candidate, &mut evidence);
    let (head_is_ga, head_is_guha) = parsed_value(&candidate.gek)
        .and_then(|gek| parsed_value(&gek.body))
        .and_then(|body| match body {
            recovered::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(opener) => {
                parsed_value(opener).map(|opener| {
                    (
                        parsed_value(&opener.head.value)
                            .is_some_and(|token| token.is_selmaho(jbotci_morphology::Selmaho::Ga)),
                        parsed_value(&opener.head.value).is_some_and(|token| {
                            token.is_selmaho(jbotci_morphology::Selmaho::Guha)
                        }),
                    )
                })
            }
            _ => Some((false, false)),
        })
        .unwrap_or((false, false));
    RecoveredGekFacts {
        uncertain: evidence.uncertainty,
        head_is_ga,
        head_is_guha,
        has_nahe: candidate.nahe.is_some(),
        has_bo: parsed_value(&candidate.gek).is_some_and(|gek| gek.bo.is_some()),
        branch_count: candidate.branches.len(),
        has_gihi: candidate.gihi.is_some(),
    }
}

/// A strict enclosed product is owned exactly when its opener is the GA-family variant.
///
/// `zantufa_atom_ga_opener` takes its head from `choice((selmaho(Ga), selmaho(Guha)))`, so the
/// variant already is the GA-or-GUhA fact; the head class is a precondition, not a test.
#[requires(true)]
#[bityzba::expensive_requires(match value.gek.body.as_ref() {
    model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(opener) => {
        opener.head.value.is_one_of_selmaho(&[Selmaho::Ga, Selmaho::Guha])
    }
    model::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(_)
    | model::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(_) => true,
})]
#[ensures((ret == ZantufaTanruAtomPresence::Present) == matches!(value.gek.body.as_ref(), model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(_)))]
fn strict_enclosed_presence(
    value: &model::ZantufaForethoughtTanruUnitSyntax,
) -> ZantufaTanruAtomPresence {
    match value.gek.body.as_ref() {
        model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(_) => {
            ZantufaTanruAtomPresence::Present
        }
        model::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(_)
        | model::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(_) => {
            ZantufaTanruAtomPresence::Unproven
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn enclosed_presence_from_recovered_facts(facts: RecoveredGekFacts) -> ZantufaTanruAtomPresence {
    if facts.uncertain {
        ZantufaTanruAtomPresence::Unproven
    } else if facts.head_is_ga || facts.head_is_guha {
        ZantufaTanruAtomPresence::Present
    } else {
        ZantufaTanruAtomPresence::Unproven
    }
}

#[requires(true)]
#[ensures(true)]
fn strict_gek_facts(candidate: &model::ZantufaForethoughtTanruUnitSyntax) -> StrictGekFacts {
    let model::ZantufaForethoughtTanruUnitSyntax {
        nahe,
        gek,
        branches,
        gihi,
        ..
    } = candidate;
    let model::ZantufaAtomGekSyntax { body, bo } = gek.as_ref();
    let (head_is_ga, head_is_guha, opener_se_free_modifiers) = match body.as_ref() {
        model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(opener) => (
            opener.head.value.is_selmaho(jbotci_morphology::Selmaho::Ga),
            opener
                .head
                .value
                .is_selmaho(jbotci_morphology::Selmaho::Guha),
            opener
                .se
                .as_ref()
                .is_some_and(|se| !se.free_modifiers.is_empty()),
        ),
        model::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(_)
        | model::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(_) => (false, false, false),
    };
    StrictGekFacts {
        head_is_ga,
        head_is_guha,
        has_nahe: nahe.is_some(),
        has_bo: bo.is_some(),
        branch_count: branches.len(),
        has_gihi: gihi.is_some(),
        opener_se_free_modifiers,
    }
}

impl ZantufaTanruAtomPresence {
    #[requires(true)]
    #[ensures((ret == Self::Unproven) == (self == Self::Unproven || other == Self::Unproven))]
    #[ensures((ret == Self::Absent) == (self == Self::Absent && other == Self::Absent))]
    #[ensures((ret == Self::Present) == (self != Self::Unproven && other != Self::Unproven && (self == Self::Present || other == Self::Present)))]
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unproven, Self::Present | Self::Absent | Self::Unproven)
            | (Self::Present | Self::Absent, Self::Unproven) => Self::Unproven,
            (Self::Present, Self::Present | Self::Absent) | (Self::Absent, Self::Present) => {
                Self::Present
            }
            (Self::Absent, Self::Absent) => Self::Absent,
        }
    }
}

/// Standalone ownership of a completed source GEK product.
///
/// The GA-family partition implements design M1 and the adjudicated GUhA+BO
/// class. Modifier-free JOIK BINARY uses its separately adjudicated table;
/// other JOIK shapes and tag payloads remain Unproven. Warning splits and
/// remaining winning-recovery proofs govern fixture acceptance; the guarded parser
/// route is connected and remains fail-closed for unproven evidence.
#[requires(
    dialect.zantufa_selbri_enabled,
    "the product rule asserts feature(ZantufaSelbri), so no candidate exists on another axis"
)]
#[ensures(true)]
fn strict_standalone_presence(
    candidate: &model::ZantufaForethoughtTanruUnitSyntax,
    dialect: &super::generated_runtime::SyntaxGrammarDialect,
) -> ZantufaTanruAtomPresence {
    use ZantufaTanruAtomPresence::{Absent, Present, Unproven};

    let facts = strict_gek_facts(candidate);
    let model::ZantufaForethoughtTanruUnitSyntax {
        nahe,
        gek,
        leading_selbri: _left,
        branches,
        gihi,
    } = candidate;
    let model::ZantufaAtomGekSyntax { body, bo } = gek.as_ref();
    match body.as_ref() {
        model::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(_) => {}
        model::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(_)
        | model::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(_) => {
            if nahe.is_some() || branches.len() != 1 || gihi.is_some() {
                return Unproven;
            }
            let Some(key) = strict_gek_joik_key(gek, dialect.zantufa_connectives_enabled) else {
                return Unproven;
            };
            return if !key.baseline_overlaps()
                || dialect.zantufa_selbri_atom_reinterpretation_enabled
            {
                Present
            } else {
                Absent
            };
        }
    };
    // PM ruling 2026-09-13: the source SE_post slot admits free modifiers,
    // but baseline GA/GUhA opener-SE has no such slot. This exact token
    // sequence has no baseline owner, even if its right operand contains CO.
    // The differing grouping of a nearby empty-list GUhA source follows from
    // its reservation, not from a semantic claim about the parenthetical.
    // Strict parsing proves this field's contents; the recovered twin must
    // separately prove every modifier and must never use Vec occupancy alone.
    if facts.opener_se_free_modifiers {
        return Present;
    }
    // Strict products prove all parsed fields. Opener-SE and left width do
    // not distinguish GUhA ownership; they must not enter M1's projection.
    if facts.head_is_guha {
        if bo.is_some() {
            // Section 9's measured, adjudicated source-only class. This is
            // GUhA+BO specifically, not a universal BO additivity shortcut.
            return Present;
        }
        if branches.len() > 1 || gihi.is_some() {
            return if dialect.zantufa_connectives_enabled {
                Absent
            } else {
                Present
            };
        }
        return if dialect.zantufa_selbri_atom_reinterpretation_enabled
            && strict_right_operand_extends_past_l6(&branches.first().selbri)
        {
            Present
        } else {
            Absent
        };
    }
    if !facts.head_is_ga {
        return Unproven;
    }
    if branches.len() > 1 || nahe.is_some() || gihi.is_some() {
        return Present;
    }
    // Ordinary GA+BO is already owned on the Connectives axis, unlike
    // GUhA+BO. Compare with that same axis with Selbri disabled.
    if dialect.zantufa_selbri_atom_reinterpretation_enabled
        || (bo.is_some() && !dialect.zantufa_connectives_enabled)
    {
        Present
    } else {
        Absent
    }
}

/// Shared GA-family projection entry point for context-specific ownership
/// policies. The standalone policy remains the compatibility implementation;
/// enclosed JAI will apply its narrower policy to this same projection.
#[requires(true)]
#[ensures(true)]
pub(crate) fn strict_gek_projection(
    candidate: &model::ZantufaForethoughtTanruUnitSyntax,
    dialect: &super::generated_runtime::SyntaxGrammarDialect,
) -> ZantufaTanruAtomPresence {
    let facts = strict_gek_facts(candidate);
    if !dialect.zantufa_selbri_enabled {
        return ZantufaTanruAtomPresence::Absent;
    }
    if facts.opener_se_free_modifiers {
        return ZantufaTanruAtomPresence::Present;
    }
    if !facts.head_is_ga && !facts.head_is_guha {
        return ZantufaTanruAtomPresence::Unproven;
    }
    if facts.head_is_guha {
        if facts.has_bo {
            return ZantufaTanruAtomPresence::Present;
        }
        return if facts.branch_count > 1 || facts.has_gihi {
            if dialect.zantufa_connectives_enabled {
                ZantufaTanruAtomPresence::Absent
            } else {
                ZantufaTanruAtomPresence::Present
            }
        } else if dialect.zantufa_selbri_atom_reinterpretation_enabled {
            ZantufaTanruAtomPresence::Present
        } else {
            ZantufaTanruAtomPresence::Absent
        };
    }
    if facts.branch_count > 1 || facts.has_nahe || facts.has_gihi {
        return ZantufaTanruAtomPresence::Present;
    }
    if dialect.zantufa_selbri_atom_reinterpretation_enabled
        || (facts.has_bo && !dialect.zantufa_connectives_enabled)
    {
        ZantufaTanruAtomPresence::Present
    } else {
        ZantufaTanruAtomPresence::Absent
    }
}

/// Recovered GA/JOIK partitions on the parsed product, before public routing.
/// The caller retains any outer Prefix wrapper and its ordered errors. Every
/// required child inside this product must establish its own evidence; a
/// positive opener discriminator cannot mask an unproven branch or marker.
#[requires(
    dialect.zantufa_selbri_enabled,
    "the product rule asserts feature(ZantufaSelbri), so no candidate exists on another axis"
)]
#[ensures(true)]
fn recovered_standalone_presence(
    candidate: &recovered::ZantufaForethoughtTanruUnitSyntax,
    dialect: &super::generated_runtime::SyntaxGrammarDialect,
) -> ZantufaTanruAtomPresence {
    use ZantufaTanruAtomPresence::{Absent, Present, Unproven};
    use jbotci_morphology::Selmaho;

    let facts = recovered_gek_facts(candidate);
    let recovered::ZantufaForethoughtTanruUnitSyntax {
        nahe,
        gek,
        leading_selbri,
        branches,
        gihi,
    } = candidate;
    let Some(gek) = parsed_value(gek) else {
        return Unproven;
    };
    let recovered::ZantufaAtomGekSyntax { body, bo } = gek;
    let Some(body) = parsed_value(body) else {
        return Unproven;
    };
    // Visit the selected construction to detect Prefix/error wrappers above
    // the fields projected below. This detects uncertainty only, never gives
    // positive ownership from a token elsewhere in the candidate.
    let mut uncertainty = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(candidate, &mut uncertainty);
    if facts.uncertain || uncertainty.uncertainty {
        return Unproven;
    }
    if nahe.as_ref().is_some_and(|clause| {
        !recovered_clause_is_complete(clause, |token| token.is_selmaho(Selmaho::Nahe))
    }) || bo.as_ref().is_some_and(|clause| {
        !recovered_clause_is_complete(clause, |token| token.is_cmavo(jbotci_morphology::Cmavo::Bo))
    }) || gihi.as_ref().is_some_and(|clause| {
        !recovered_clause_is_complete(clause, |token| {
            token.is_cmavo(jbotci_morphology::Cmavo::Gihi)
        })
    }) {
        return Unproven;
    }
    let Some(left) = parsed_value(leading_selbri) else {
        return Unproven;
    };
    if recovered_right_operand_extends_past_l6(left).is_none() || branches.is_empty() {
        return Unproven;
    }
    let mut sole_right_extends = false;
    for branch in branches {
        let Some(branch) = parsed_value(branch) else {
            return Unproven;
        };
        let recovered::ZantufaAtomGekBranchSyntax { gi, selbri } = branch;
        if !recovered_clause_is_complete(gi, |token| token.is_cmavo(jbotci_morphology::Cmavo::Gi)) {
            return Unproven;
        }
        let Some(right) = parsed_value(selbri) else {
            return Unproven;
        };
        let Some(extends) = recovered_right_operand_extends_past_l6(right) else {
            return Unproven;
        };
        sole_right_extends = extends;
    }
    let opener = match body {
        recovered::ZantufaAtomGekBodySyntax::ZantufaAtomGaOpener(opener) => {
            let Some(opener) = parsed_value(opener) else {
                return Unproven;
            };
            opener
        }
        recovered::ZantufaAtomGekBodySyntax::ZantufaAtomInitialGiOpener(_)
        | recovered::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(_) => {
            if nahe.is_some() || branches.len() != 1 || gihi.is_some() {
                return Unproven;
            }
            let Some(key) = recovered_gek_joik_key(gek, dialect.zantufa_connectives_enabled) else {
                return Unproven;
            };
            return if !key.baseline_overlaps()
                || dialect.zantufa_selbri_atom_reinterpretation_enabled
            {
                Present
            } else {
                Absent
            };
        }
    };
    let opener_free = recovered_opener_se_free_presence(opener);
    if opener_free == Unproven {
        return Unproven;
    }
    if opener_free == Present {
        return Present;
    }
    let head = parsed_value(&opener.head.value).expect("opener evidence proved the head");
    if head.is_selmaho(Selmaho::Guha) {
        if bo.is_some() {
            return Present;
        }
        if branches.len() > 1 || gihi.is_some() {
            return if dialect.zantufa_connectives_enabled {
                Absent
            } else {
                Present
            };
        }
        return if dialect.zantufa_selbri_atom_reinterpretation_enabled && sole_right_extends {
            Present
        } else {
            Absent
        };
    }
    if branches.len() > 1
        || nahe.is_some()
        || gihi.is_some()
        || dialect.zantufa_selbri_atom_reinterpretation_enabled
        || (bo.is_some() && !dialect.zantufa_connectives_enabled)
    {
        Present
    } else {
        Absent
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_gek_projection(
    candidate: &recovered::ZantufaForethoughtTanruUnitSyntax,
    dialect: &super::generated_runtime::SyntaxGrammarDialect,
) -> ZantufaTanruAtomPresence {
    recovered_standalone_presence(candidate, dialect)
}

/// A selected marker and its own modifiers must be parsed, not synthesized.
#[requires(true)]
#[ensures(ret -> parsed_value(&clause.value).is_some_and(&expected))]
fn recovered_clause_is_complete(
    clause: &StoredRecoveredTokenClause,
    expected: impl Fn(&Token) -> bool,
) -> bool {
    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(clause, &mut evidence);
    !evidence.uncertainty && parsed_value(&clause.value).is_some_and(&expected)
}

/// M1's fixed-depth projection of the sole *right* source operand.
///
/// This answers only the attachment question, not source inventory or recovery
/// completeness. The standard GUhA right operand ends at PlainBoSelbri/L6;
/// the new product's right operand extends to CoSelbri. Width inside that L6
/// operand (including KE groups and plain BO) therefore does not distinguish
/// their outer grouping. The left operand is deliberately not consulted.
#[requires(true)]
#[ensures(ret == (right.co_tail.is_some() || !right.leading_selbri.additional_selbri.is_empty() || !right.leading_selbri.first_selbri.continuations.is_empty() || right.leading_selbri.first_selbri.leading_selbri.bo_tail.is_some()))]
fn strict_right_operand_extends_past_l6(right: &model::CoSelbriSyntax) -> bool {
    // Destructure every spine product exhaustively: adding a field must force
    // this source-to-baseline boundary projection to be reconsidered.
    let model::CoSelbriSyntax {
        leading_selbri,
        co_tail,
    } = right;
    let model::TanruSelbriSyntax {
        first_selbri,
        additional_selbri,
    } = leading_selbri.as_ref();
    let model::ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = first_selbri.as_ref();
    let model::BoundSelbriSyntax {
        leading_selbri: _l6_operand,
        bo_tail,
    } = leading_selbri.as_ref();
    co_tail.is_some()
        || !additional_selbri.is_empty()
        || !continuations.is_empty()
        || bo_tail.is_some()
}

/// Local completeness evidence for a selected required subtree.
///
/// Every combination is meaningful: a missing required slot can have no
/// parsed token, or can coexist with other parsed tokens. This is not the
/// priority owner's whole-tree presence classifier.
#[invariant(true)]
#[derive(Default)]
struct RequiredSubtreeEvidence {
    parsed_token: bool,
    uncertainty: bool,
}

impl<'tree> jbotci_tree::TreeVisitor<'tree> for RequiredSubtreeEvidence {
    type Node = recovered::NodeRef<'tree>;
    type Atom = recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.parsed_token)]
    fn visit_atom(&mut self, _atom: Self::Atom) {
        self.parsed_token = true;
    }

    #[requires(true)]
    #[ensures(self.uncertainty)]
    fn visit_recovered_error<E: jbotci_tree::RecoveryItemState + serde::Serialize>(
        &mut self,
        _item: &'tree E,
    ) {
        self.uncertainty = true;
    }
}

/// Evidence for the adjudicated opener-SE slot, not whole-GEK eligibility.
///
/// The caller must additionally prove the branches, discriminators and source
/// boundary. Only this opener's SE field supplies the positive discriminator.
/// Its head and all attached modifiers must be complete; nested recovery on
/// that path cannot be ignored even when another modifier is proven parsed.
#[requires(true)]
#[ensures(ret == ZantufaTanruAtomPresence::Present -> opener.se.as_ref().is_some_and(|se| !se.free_modifiers.is_empty()))]
fn recovered_opener_se_free_presence(
    opener: &recovered::ZantufaAtomGaOpenerSyntax,
) -> ZantufaTanruAtomPresence {
    use ZantufaTanruAtomPresence::{Absent, Present, Unproven};
    use jbotci_morphology::Selmaho;

    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(opener, &mut evidence);
    if evidence.uncertainty {
        return Unproven;
    }
    let Some(head) = parsed_value(&opener.head.value) else {
        return Unproven;
    };
    if !head.is_selmaho(Selmaho::Ga) && !head.is_selmaho(Selmaho::Guha) {
        return Unproven;
    }
    let Some(se) = &opener.se else {
        return Absent;
    };
    if !parsed_value(&se.value).is_some_and(|token| token.is_selmaho(Selmaho::Se)) {
        return Unproven;
    }
    if se.free_modifiers.is_empty() {
        return Absent;
    }
    for modifier in &se.free_modifiers {
        let mut evidence = RequiredSubtreeEvidence::default();
        recovered::TreeNode::visit_in_order(modifier, &mut evidence);
        if !evidence.parsed_token || evidence.uncertainty {
            return Unproven;
        }
    }
    Present
}

/// A parsed value does not borrow its skipped prefix tokens as entry proof.
#[requires(true)]
#[ensures(ret.is_none() == matches!(value.borrow(), recovered::Recovered::Error(_)))]
fn parsed_value<T>(
    value: &(impl std::borrow::Borrow<recovered::Recovered<T>> + ?Sized),
) -> Option<&T> {
    match value.borrow() {
        recovered::Recovered::Valid(value) => Some(value),
        recovered::Recovered::Prefix(prefix) => Some(&prefix.value),
        recovered::Recovered::Error(_) => None,
    }
}

/// Recovered M1 projection: occupancy alone cannot prove a changed attachment.
/// The caller handles its outer Valid/Prefix/Error wrapper separately. All
/// required evidence inside this selected right operand must be complete.
#[requires(true)]
#[ensures(true)]
fn recovered_right_operand_extends_past_l6(right: &recovered::CoSelbriSyntax) -> Option<bool> {
    let mut evidence = RequiredSubtreeEvidence::default();
    recovered::TreeNode::visit_in_order(right, &mut evidence);
    if !evidence.parsed_token || evidence.uncertainty {
        return None;
    }
    let recovered::CoSelbriSyntax {
        leading_selbri,
        co_tail,
    } = right;
    let recovered::TanruSelbriSyntax {
        first_selbri,
        additional_selbri,
    } = parsed_value(leading_selbri)?;
    let recovered::ConnectedSelbriSyntax {
        leading_selbri,
        continuations,
    } = parsed_value(first_selbri)?;
    let recovered::BoundSelbriSyntax {
        leading_selbri: _l6_operand,
        bo_tail,
    } = parsed_value(leading_selbri)?;
    if let Some(tail) = co_tail {
        if !parsed_value(&parsed_value(tail)?.co.value)?.is_cmavo(jbotci_morphology::Cmavo::Co) {
            return None;
        }
    }
    if let Some(tail) = bo_tail {
        if !parsed_value(&parsed_value(tail)?.bo.value)?.is_cmavo(jbotci_morphology::Cmavo::Bo) {
            return None;
        }
    }
    Some(
        co_tail.is_some()
            || !additional_selbri.is_empty()
            || !continuations.is_empty()
            || bo_tail.is_some(),
    )
}

impl From<(TokenClause, TokenClause)> for ZantufaAtomGaOpenerSyntax {
    #[requires(true)]
    #[ensures(ret.se.is_some())]
    #[expensive_ensures(ret.se == Some(store_clause(old(value.0.clone()))) && ret.head == store_clause(old(value.1.clone())))]
    fn from(value: (TokenClause, TokenClause)) -> Self {
        let (se, head) = value;
        Self {
            se: Some(store_clause(se)),
            head: store_clause(head),
        }
    }
}

impl From<TokenClause> for ZantufaAtomGaOpenerSyntax {
    #[requires(true)]
    #[ensures(ret.se.is_none())]
    #[expensive_ensures(ret.head == store_clause(old(head.clone())))]
    fn from(head: TokenClause) -> Self {
        Self {
            se: None,
            head: store_clause(head),
        }
    }
}

impl From<(RecoveredTokenClause, RecoveredTokenClause)> for recovered::ZantufaAtomGaOpenerSyntax {
    #[requires(true)]
    #[ensures(ret.se.is_some())]
    #[expensive_ensures(ret.se == Some(store_recovered_clause(old(value.0.clone()))) && ret.head == store_recovered_clause(old(value.1.clone())))]
    fn from(value: (RecoveredTokenClause, RecoveredTokenClause)) -> Self {
        let (se, head) = value;
        // Each token's own Valid/Prefix/Error wrapper and every free modifier
        // remain untouched in the completed product's fields.
        // The eligibility walker still has to prove that nested evidence.
        recovered::ZantufaAtomGaOpenerSyntax {
            se: Some(store_recovered_clause(se)),
            head: store_recovered_clause(head),
        }
    }
}

impl From<RecoveredTokenClause> for recovered::ZantufaAtomGaOpenerSyntax {
    #[requires(true)]
    #[ensures(ret.se.is_none())]
    #[expensive_ensures(ret.head == store_recovered_clause(old(head.clone())))]
    fn from(head: RecoveredTokenClause) -> Self {
        recovered::ZantufaAtomGaOpenerSyntax {
            se: None,
            head: store_recovered_clause(head),
        }
    }
}

impl
    From<(
        (TokenClause, model::TenseModalSyntax),
        model::TanruUnitAtomSyntax,
    )> for model::JaiModalTanruUnitSyntax
{
    #[requires(true)]
    #[ensures(true)]
    fn from(
        value: (
            (TokenClause, model::TenseModalSyntax),
            model::TanruUnitAtomSyntax,
        ),
    ) -> Self {
        let ((jai, tense_modal), inner_unit) = value;
        Self {
            jai: store_clause(jai),
            tense_modal: Some(std::sync::Arc::new(tense_modal)),
            inner_unit: std::sync::Arc::new(inner_unit),
        }
    }
}

impl From<(TokenClause, model::TanruUnitAtomSyntax)> for model::JaiModalTanruUnitSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: (TokenClause, model::TanruUnitAtomSyntax)) -> Self {
        let (jai, inner_unit) = value;
        Self {
            jai: store_clause(jai),
            tense_modal: None,
            inner_unit: std::sync::Arc::new(inner_unit),
        }
    }
}

impl
    From<(
        (
            RecoveredTokenClause,
            recovered::Recovered<recovered::TenseModalSyntax>,
        ),
        recovered::Recovered<recovered::TanruUnitAtomSyntax>,
    )> for recovered::JaiModalTanruUnitSyntax
{
    fn from(
        value: (
            (
                RecoveredTokenClause,
                recovered::Recovered<recovered::TenseModalSyntax>,
            ),
            recovered::Recovered<recovered::TanruUnitAtomSyntax>,
        ),
    ) -> Self {
        let ((jai, tense_modal), inner_unit) = value;
        recovered::JaiModalTanruUnitSyntax {
            jai: store_recovered_clause(jai),
            tense_modal: Some(std::sync::Arc::new(tense_modal)),
            inner_unit: std::sync::Arc::new(inner_unit),
        }
    }
}

impl
    From<(
        RecoveredTokenClause,
        recovered::Recovered<recovered::TanruUnitAtomSyntax>,
    )> for recovered::JaiModalTanruUnitSyntax
{
    fn from(
        value: (
            RecoveredTokenClause,
            recovered::Recovered<recovered::TanruUnitAtomSyntax>,
        ),
    ) -> Self {
        let (jai, inner_unit) = value;
        recovered::JaiModalTanruUnitSyntax {
            jai: store_recovered_clause(jai),
            tense_modal: None,
            inner_unit: std::sync::Arc::new(inner_unit),
        }
    }
}

// Parser sequences are left-associated pairs, not flat tuples. Each split
// moves every source field unchanged into the shared product.

impl
    From<(
        (
            ((TokenClause, Option<TokenClause>), Option<TokenClause>),
            TokenClause,
        ),
        Option<TokenClause>,
    )> for model::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[expensive_ensures(ret.left_gaho == Some(store_clause(old(value.0.0.0.0.clone()))))]
    #[expensive_ensures(ret.na == store_opt_clause(old(value.0.0.0.1.clone())))]
    #[expensive_ensures(ret.se == store_opt_clause(old(value.0.0.1.clone())))]
    #[expensive_ensures(ret.head == store_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_clause(old(value.1.clone())))]
    fn from(
        value: (
            (
                ((TokenClause, Option<TokenClause>), Option<TokenClause>),
                TokenClause,
            ),
            Option<TokenClause>,
        ),
    ) -> Self {
        let ((((left_gaho, na), se), head), right_gaho) = value;
        Self {
            left_gaho: Some(store_clause(left_gaho)),
            na: store_opt_clause(na),
            se: store_opt_clause(se),
            head: store_clause(head),
            right_gaho: store_opt_clause(right_gaho),
        }
    }
}

impl
    From<(
        ((TokenClause, Option<TokenClause>), TokenClause),
        Option<TokenClause>,
    )> for model::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[expensive_ensures(ret.na == Some(store_clause(old(value.0.0.0.clone()))))]
    #[expensive_ensures(ret.se == store_opt_clause(old(value.0.0.1.clone())))]
    #[expensive_ensures(ret.head == store_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_clause(old(value.1.clone())))]
    fn from(
        value: (
            ((TokenClause, Option<TokenClause>), TokenClause),
            Option<TokenClause>,
        ),
    ) -> Self {
        let (((na, se), head), right_gaho) = value;
        Self {
            left_gaho: None,
            na: Some(store_clause(na)),
            se: store_opt_clause(se),
            head: store_clause(head),
            right_gaho: store_opt_clause(right_gaho),
        }
    }
}

impl From<((TokenClause, TokenClause), Option<TokenClause>)> for model::ZantufaAtomJoikSyntax {
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[ensures(ret.na.is_none())]
    #[expensive_ensures(ret.se == Some(store_clause(old(value.0.0.clone()))))]
    #[expensive_ensures(ret.head == store_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_clause(old(value.1.clone())))]
    fn from(value: ((TokenClause, TokenClause), Option<TokenClause>)) -> Self {
        let ((se, head), right_gaho) = value;
        Self {
            left_gaho: None,
            na: None,
            se: Some(store_clause(se)),
            head: store_clause(head),
            right_gaho: store_opt_clause(right_gaho),
        }
    }
}

impl From<(TokenClause, Option<TokenClause>)> for model::ZantufaAtomJoikSyntax {
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[ensures(ret.na.is_none())]
    #[ensures(ret.se.is_none())]
    #[expensive_ensures(ret.head == store_clause(old(value.0.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_clause(old(value.1.clone())))]
    fn from(value: (TokenClause, Option<TokenClause>)) -> Self {
        let (head, right_gaho) = value;
        Self {
            left_gaho: None,
            na: None,
            se: None,
            head: store_clause(head),
            right_gaho: store_opt_clause(right_gaho),
        }
    }
}

impl
    From<(
        (
            (
                (RecoveredTokenClause, Option<RecoveredTokenClause>),
                Option<RecoveredTokenClause>,
            ),
            RecoveredTokenClause,
        ),
        Option<RecoveredTokenClause>,
    )> for recovered::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[expensive_ensures(ret.left_gaho == Some(store_recovered_clause(old(value.0.0.0.0.clone()))))]
    #[expensive_ensures(ret.na == store_opt_recovered_clause(old(value.0.0.0.1.clone())))]
    #[expensive_ensures(ret.se == store_opt_recovered_clause(old(value.0.0.1.clone())))]
    #[expensive_ensures(ret.head == store_recovered_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_recovered_clause(old(value.1.clone())))]
    fn from(
        value: (
            (
                (
                    (RecoveredTokenClause, Option<RecoveredTokenClause>),
                    Option<RecoveredTokenClause>,
                ),
                RecoveredTokenClause,
            ),
            Option<RecoveredTokenClause>,
        ),
    ) -> Self {
        let ((((left_gaho, na), se), head), right_gaho) = value;
        Self {
            left_gaho: Some(store_recovered_clause(left_gaho)),
            na: store_opt_recovered_clause(na),
            se: store_opt_recovered_clause(se),
            head: store_recovered_clause(head),
            right_gaho: store_opt_recovered_clause(right_gaho),
        }
    }
}

impl
    From<(
        (
            (RecoveredTokenClause, Option<RecoveredTokenClause>),
            RecoveredTokenClause,
        ),
        Option<RecoveredTokenClause>,
    )> for recovered::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[expensive_ensures(ret.na == Some(store_recovered_clause(old(value.0.0.0.clone()))))]
    #[expensive_ensures(ret.se == store_opt_recovered_clause(old(value.0.0.1.clone())))]
    #[expensive_ensures(ret.head == store_recovered_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_recovered_clause(old(value.1.clone())))]
    fn from(
        value: (
            (
                (RecoveredTokenClause, Option<RecoveredTokenClause>),
                RecoveredTokenClause,
            ),
            Option<RecoveredTokenClause>,
        ),
    ) -> Self {
        let (((na, se), head), right_gaho) = value;
        Self {
            left_gaho: None,
            na: Some(store_recovered_clause(na)),
            se: store_opt_recovered_clause(se),
            head: store_recovered_clause(head),
            right_gaho: store_opt_recovered_clause(right_gaho),
        }
    }
}

impl
    From<(
        (RecoveredTokenClause, RecoveredTokenClause),
        Option<RecoveredTokenClause>,
    )> for recovered::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[ensures(ret.na.is_none())]
    #[expensive_ensures(ret.se == Some(store_recovered_clause(old(value.0.0.clone()))))]
    #[expensive_ensures(ret.head == store_recovered_clause(old(value.0.1.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_recovered_clause(old(value.1.clone())))]
    fn from(
        value: (
            (RecoveredTokenClause, RecoveredTokenClause),
            Option<RecoveredTokenClause>,
        ),
    ) -> Self {
        let ((se, head), right_gaho) = value;
        Self {
            left_gaho: None,
            na: None,
            se: Some(store_recovered_clause(se)),
            head: store_recovered_clause(head),
            right_gaho: store_opt_recovered_clause(right_gaho),
        }
    }
}

impl From<(RecoveredTokenClause, Option<RecoveredTokenClause>)>
    for recovered::ZantufaAtomJoikSyntax
{
    #[requires(true)]
    #[ensures(ret.left_gaho.is_none())]
    #[ensures(ret.na.is_none())]
    #[ensures(ret.se.is_none())]
    #[expensive_ensures(ret.head == store_recovered_clause(old(value.0.clone())))]
    #[expensive_ensures(ret.right_gaho == store_opt_recovered_clause(old(value.1.clone())))]
    fn from(value: (RecoveredTokenClause, Option<RecoveredTokenClause>)) -> Self {
        let (head, right_gaho) = value;
        Self {
            left_gaho: None,
            na: None,
            se: None,
            head: store_recovered_clause(head),
            right_gaho: store_opt_recovered_clause(right_gaho),
        }
    }
}

impl From<(model::ZantufaAtomJoikSyntax, TokenClause)> for model::ZantufaAtomFinalGiOpenerSyntax {
    #[requires(true)]
    #[expensive_ensures(*ret.payload == model::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(old(value.0.clone()).into()))]
    #[expensive_ensures(ret.gi == store_clause(old(value.1.clone())))]
    fn from(value: (model::ZantufaAtomJoikSyntax, TokenClause)) -> Self {
        let (payload, gi) = value;
        Self {
            payload: Arc::new(model::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(
                payload.into(),
            )),
            gi: store_clause(gi),
        }
    }
}

impl From<(model::ZantufaAtomTagSyntax, TokenClause)> for model::ZantufaAtomFinalGiOpenerSyntax {
    #[requires(true)]
    #[expensive_ensures(*ret.payload == model::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(old(value.0.clone()).into()))]
    #[expensive_ensures(ret.gi == store_clause(old(value.1.clone())))]
    fn from(value: (model::ZantufaAtomTagSyntax, TokenClause)) -> Self {
        let (payload, gi) = value;
        Self {
            payload: Arc::new(model::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(
                payload.into(),
            )),
            gi: store_clause(gi),
        }
    }
}

impl
    From<(
        recovered::Recovered<recovered::ZantufaAtomJoikSyntax>,
        RecoveredTokenClause,
    )> for recovered::ZantufaAtomFinalGiOpenerSyntax
{
    #[requires(true)]
    #[expensive_ensures(*ret.payload == recovered::Recovered::valid(recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(Arc::new(old(value.0.clone())))))]
    #[expensive_ensures(ret.gi == store_recovered_clause(old(value.1.clone())))]
    fn from(
        value: (
            recovered::Recovered<recovered::ZantufaAtomJoikSyntax>,
            RecoveredTokenClause,
        ),
    ) -> Self {
        let (payload, gi) = value;
        Self {
            payload: Arc::new(recovered::Recovered::valid(
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(Arc::new(payload)),
            )),
            gi: store_recovered_clause(gi),
        }
    }
}

impl
    From<(
        recovered::Recovered<recovered::ZantufaAtomTagSyntax>,
        RecoveredTokenClause,
    )> for recovered::ZantufaAtomFinalGiOpenerSyntax
{
    #[requires(true)]
    #[expensive_ensures(*ret.payload == recovered::Recovered::valid(recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(Arc::new(old(value.0.clone())))))]
    #[expensive_ensures(ret.gi == store_recovered_clause(old(value.1.clone())))]
    fn from(
        value: (
            recovered::Recovered<recovered::ZantufaAtomTagSyntax>,
            RecoveredTokenClause,
        ),
    ) -> Self {
        let (payload, gi) = value;
        Self {
            payload: Arc::new(recovered::Recovered::valid(
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(Arc::new(payload)),
            )),
            gi: store_recovered_clause(gi),
        }
    }
}

impl From<model::ZantufaForethoughtTanruUnitSyntax> for model::TanruUnitAtomSyntax {
    #[requires(true)]
    #[ensures(ret.conversions.is_empty())]
    #[expensive_ensures(ret.base.as_ref() == &model::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(old(value.clone()).into()))]
    fn from(value: model::ZantufaForethoughtTanruUnitSyntax) -> Self {
        Self {
            conversions: Vec::new(),
            base: std::sync::Arc::new(model::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(
                value.into(),
            )),
        }
    }
}
impl From<model::TanruUnitAtomBaseSyntax> for model::TanruUnitAtomSyntax {
    #[requires(true)]
    #[ensures(ret.conversions.is_empty())]
    #[expensive_ensures(ret.base.as_ref() == &old(value.clone()))]
    fn from(value: model::TanruUnitAtomBaseSyntax) -> Self {
        Self {
            conversions: Vec::new(),
            base: std::sync::Arc::new(value),
        }
    }
}
impl From<(TokenClause, model::TanruUnitAtomSyntax)> for model::TanruUnitAtomSyntax {
    #[requires(true)]
    #[ensures(ret.conversions.len() == old(value.1.conversions.len()) + 1)]
    #[expensive_ensures(ret.base == old(value.1.base.clone()))]
    fn from(value: (TokenClause, model::TanruUnitAtomSyntax)) -> Self {
        let (se, mut inner) = value;
        inner.conversions.insert(0, store_clause(se));
        inner
    }
}
#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::TanruUnitAtomSyntax>>
    for recovered::Recovered<recovered::ZantufaForethoughtTanruUnitSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::TanruUnitAtomSyntax> {
        recovered::Recovered::valid(recovered::TanruUnitAtomSyntax {
            conversions: Vec::new(),
            base: std::sync::Arc::new(recovered::Recovered::valid(
                recovered::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(Arc::new(self)),
            )),
        })
    }
}
#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::TanruUnitAtomSyntax>>
    for recovered::Recovered<recovered::TanruUnitAtomBaseSyntax>
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::TanruUnitAtomSyntax> {
        recovered::Recovered::valid(recovered::TanruUnitAtomSyntax {
            conversions: Vec::new(),
            base: std::sync::Arc::new(self),
        })
    }
}
#[bityzba::contract_trait]
impl super::generated_runtime::GrammarMapTo<recovered::Recovered<recovered::TanruUnitAtomSyntax>>
    for (
        RecoveredTokenClause,
        recovered::Recovered<recovered::TanruUnitAtomSyntax>,
    )
{
    fn grammar_map_to(self) -> recovered::Recovered<recovered::TanruUnitAtomSyntax> {
        let (se, inner) = self;
        let se = store_recovered_clause(se);
        match inner {
            recovered::Recovered::Valid(mut inner) => {
                inner.conversions.insert(0, se);
                recovered::Recovered::Valid(inner)
            }
            recovered::Recovered::Prefix(mut prefix) => {
                prefix.value.conversions.insert(0, se);
                recovered::Recovered::Prefix(prefix)
            }
            recovered::Recovered::Error(error) => recovered::Recovered::Error(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use bityzba::{new, requires};
    use jbotci_dialect::parse_dialect_definition;
    use jbotci_morphology::{Cmavo, segment_words_with_modifiers};

    use super::*;
    use crate::grammar::parser_core::{Input, MappedInput, Parser, SimpleSpan, custom};
    use crate::grammar::{ParserState, generated_model, generated_runtime, syntax_tokens, tokens};
    use crate::{ExperimentalConstruct, ParseOptions};

    macro_rules! recovered_fa_tanru_unit_parser {
        () => {
            generated_model::recovered_zantufa_fa_tanru_unit_parser(
                generated_model::recovered_generated_zantufa_tanru_unit_atom_entry_parser(),
                generated_model::strict_generated_zantufa_tanru_unit_atom_entry_parser(),
                generated_model::recovered_generated_free_modifier_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
        };
    }

    macro_rules! recovered_forethought_tanru_unit_parser {
        () => {
            generated_model::recovered_zantufa_forethought_tanru_unit_parser(
                generated_model::recovered_generated_co_selbri_parser(),
                generated_model::recovered_generated_zantufa_tcita_selci_parser(),
                generated_model::recovered_generated_zantufa_boundary_term_parser(),
                generated_model::strict_generated_co_selbri_parser(),
                generated_model::strict_generated_zantufa_tcita_selci_parser(),
                generated_model::strict_generated_zantufa_boundary_term_parser(),
                generated_model::recovered_generated_free_modifier_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
        };
    }

    macro_rules! recovered_atom_gek_parser {
        () => {
            generated_model::recovered_zantufa_atom_gek_parser(
                generated_model::recovered_generated_zantufa_tcita_selci_parser(),
                generated_model::strict_generated_zantufa_tcita_selci_parser(),
                generated_model::recovered_generated_free_modifier_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
        };
    }

    macro_rules! recovered_selbri_simple_bridi_tail_parser {
        () => {
            generated_model::recovered_selbri_simple_bridi_tail_parser(
                generated_model::recovered_generated_zantufa_selbri_entry_parser(),
                generated_model::recovered_generated_term_parser(),
                generated_model::strict_generated_zantufa_selbri_entry_parser(),
                generated_model::strict_generated_term_parser(),
                generated_model::recovered_generated_free_modifier_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
        };
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fa_classifier_proves_source_inventory_and_recursive_operand() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").unwrap();
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        for source in [
            "fa broda",
            "fa joi fe broda",
            "fai broda",
            "fi'a broda",
            "fa se broda",
            "fa me'oi walk",
        ] {
            let words = segment_words_with_modifiers(source).unwrap();
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().unwrap().span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            let mut state = ParserState::new(&words, &options);
            let strict = generated_model::strict_zantufa_fa_tanru_unit_parser(
                generated_model::strict_generated_zantufa_tanru_unit_atom_entry_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
            .parse_with_state(input, &mut state)
            .into_result()
            .expect("complete strict FA")
            .into_owned();
            assert_eq!(
                strict_fa_presence(&strict),
                ZantufaTanruAtomPresence::Present,
                "{source}"
            );
            let mut state = ParserState::new(&words, &options);
            let parsed = recovered_fa_tanru_unit_parser!()
                .parse_with_state(input, &mut state)
                .into_result()
                .expect("complete recovered FA")
                .into_owned();
            assert_eq!(
                recovered_fa_presence(&parsed),
                ZantufaTanruAtomPresence::Present,
                "{source}"
            );
            let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                error_index: 92,
                tokens: vec1::Vec1::new(words[0].clone()),
            });
            let mut uncertain = parsed.clone();
            uncertain.inner_unit = std::sync::Arc::new(recovered::Recovered::error(error));
            assert_eq!(
                recovered_fa_presence(&uncertain),
                ZantufaTanruAtomPresence::Unproven
            );
        }
    }

    /// The strict FA rule states the source JOIK inventory itself, so the strict parser needs no
    /// output rejection: every in-inventory connective slot parses, and a connective outside it
    /// never yields a strict FA product.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strict_fa_rule_alone_enforces_the_source_joik_inventory() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let strict_fa = |source: &str| {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty input").span.end;
            let mut state = ParserState::new(&words, &options);
            generated_model::strict_zantufa_fa_tanru_unit_parser(
                generated_model::strict_generated_zantufa_tanru_unit_atom_entry_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .map(|product| product.into_owned())
            .map_err(|_| ())
        };
        for source in [
            "fa joi fe broda",
            "fa ja fe broda",
            "fa bi'i fe broda",
            "fa ga'o joi ga'o fe broda",
            "fa na ja fe broda",
            "fa se joi fe broda",
            "fa na se bi'i fe broda",
            "fa joi fe ja fi broda",
        ] {
            let product = strict_fa(source).unwrap_or_else(|_| panic!("strict FA for {source}"));
            assert_eq!(
                strict_fa_presence(&product),
                ZantufaTanruAtomPresence::Present,
                "{source}"
            );
        }
        // An afterthought A, a GIhA and a NAI-bearing JOI are connectives, but not source JOIK.
        for source in ["fa .e fe broda", "fa gi'e fe broda", "fa joi nai fe broda"] {
            assert!(
                strict_fa(source).is_err(),
                "{source} must not parse as a strict FA atom"
            );
        }
    }

    /// The FA classifier's inventory rejection has no parser route; pin it directly.
    ///
    /// `recovered_fa_presence` answers `Absent` when the marker occupying the `fa` slot is not
    /// FA, or when a continuation's JOIK head is outside the JOI/JA/BIhI inventory. No parse can
    /// produce either state: the generated rule spells those slots `selmaho(Fa)` and
    /// `choice((selmaho(Joi), selmaho(Ja), selmaho(Bihi)))`, and the classifier re-tests the same
    /// adopted identity through `is_selmaho`, so a completed product cannot contradict it. A
    /// dialect cmavo swap cannot separate the two either, because it rewrites the adopted identity
    /// that the rule and the classifier both read. A bounded search of 15,596 damaged inputs over
    /// every axis (96,238 traced classifications) produced `Present` and `Unproven` for this
    /// classifier and never `Absent`.
    ///
    /// The branch is deliberate defense in depth: the classifier states its own inventory rather
    /// than trusting the route that produced the value. This test is therefore the honest home for
    /// it -- a direct substitution into a really-parsed product, with its winning-tree
    /// counterparts pinned end to end by the `ce-fr1-*` (valid inventory admitted) and `ce-fr5-*`
    /// (incomplete evidence fails closed) fixtures.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fa_inventory_rejection_has_no_parser_route_and_is_pinned_directly() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers("fa joi fa broda").expect("valid morphology");
        let words = syntax_tokens(&words, &options);
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().expect("nonempty FA chain").span.end;
        let mut state = ParserState::new(&words, &options);
        let original = recovered_fa_tanru_unit_parser!()
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .expect("complete recovered FA chain")
            .into_owned();
        assert_eq!(
            recovered_fa_presence(&original),
            ZantufaTanruAtomPresence::Present
        );

        // `broda` is the one token in this input that is neither FA nor a source JOIK head.
        let outsider = words.last().expect("trailing brivla").clone();
        assert!(!outsider.is_selmaho(Selmaho::Fa));

        let mut wrong_marker = original.clone();
        wrong_marker.fa.value = recovered::Recovered::valid(outsider.clone());
        assert_eq!(
            recovered_fa_presence(&wrong_marker),
            ZantufaTanruAtomPresence::Absent,
            "a non-FA marker in the FA slot is a known shape, not missing evidence"
        );

        let mut wrong_continuation_marker = original.clone();
        let recovered::Recovered::Valid(part) =
            Arc::make_mut(&mut wrong_continuation_marker.continuations[0])
        else {
            panic!("complete continuation");
        };
        part.fa.value = recovered::Recovered::valid(outsider.clone());
        assert_eq!(
            recovered_fa_presence(&wrong_continuation_marker),
            ZantufaTanruAtomPresence::Absent
        );

        let mut wrong_joik_head = original.clone();
        let recovered::Recovered::Valid(part) =
            Arc::make_mut(&mut wrong_joik_head.continuations[0])
        else {
            panic!("complete continuation");
        };
        let recovered::Recovered::Valid(joik) = Arc::make_mut(&mut part.connective) else {
            panic!("complete JOIK");
        };
        joik.head.value = recovered::Recovered::valid(outsider);
        assert_eq!(
            recovered_fa_presence(&wrong_joik_head),
            ZantufaTanruAtomPresence::Absent
        );
    }

    // All nonnegative counts are valid intermediate traversal states.
    #[invariant(true)]
    #[derive(Default)]
    struct NestedGekCount {
        count: usize,
    }

    macro_rules! count_nested_gek {
        ($model:ident) => {
            impl<'tree> $model::TreeWalker<'tree> for NestedGekCount {
                #[requires(true)]
                #[ensures(self.count > old(self.count))]
                fn walk_zantufa_forethought_tanru_unit(
                    &mut self,
                    node: &'tree $model::ZantufaForethoughtTanruUnitSyntax,
                ) {
                    self.count += 1;
                    $model::walk::zantufa_forethought_tanru_unit(self, node);
                }
            }
        };
    }
    count_nested_gek!(model);
    count_nested_gek!(recovered);

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn priority_evidence_does_not_escape_nested_nonpredicate_domains() {
        use generated_runtime::OutputRejection;
        for definition in [
            "(+ZANTUFA-SELBRI)",
            "(+ZANTUFA-SELBRI +ZANTUFA-CONNECTIVES)",
        ] {
            let dialect = parse_dialect_definition(definition).unwrap();
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
            for source in [
                "broda sei ga brode gi brodi gi brodo se'u",
                "nu ga broda gi brode gi brodi kei",
                "me lo ga broda gi brode gi brodi ku me'u",
                "broda be lo ga brode gi brodi gi brodo ku be'o",
            ] {
                let words = segment_words_with_modifiers(source).unwrap();
                let words = syntax_tokens(&words, &options);
                let spanned = tokens::spanned_tokens(&words);
                let eoi = spanned.last().unwrap().span.end;
                let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
                let mut state = ParserState::new(&words, &options);
                let parsed = generated_model::strict_generated_co_selbri_parser()
                    .parse_with_state(input, &mut state)
                    .into_result()
                    .unwrap_or_else(|error| panic!("strict {definition}: {source}: {error:?}"));
                let mut counter = NestedGekCount::default();
                model::TreeWalkable::walk_with(&parsed, &mut counter);
                assert_eq!(counter.count, 1, "real nested atom: {source}");
                assert!(
                    PriorityAtomRejection.rejects_in_dialect(&parsed, flags),
                    "nested atom cannot reown outer predicate: {source}"
                );
                let mut state = ParserState::new(&words, &options);
                let parsed = generated_model::recovered_generated_co_selbri_parser()
                    .parse_with_state(input, &mut state)
                    .into_result()
                    .unwrap_or_else(|error| panic!("recovered {definition}: {source}: {error:?}"));
                let mut counter = NestedGekCount::default();
                recovered::TreeWalkable::walk_with(&parsed, &mut counter);
                assert_eq!(counter.count, 1, "real recovered nested atom: {source}");
                assert!(
                    PriorityAtomRejection.rejects_in_dialect(&parsed, flags),
                    "recovered nested atom cannot reown outer predicate: {source}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_atom_entry_maps_preserve_recovered_payloads_and_outer_states() {
        use generated_runtime::GrammarMapTo;
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").unwrap();
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers("ga broda gi brode gi brodi").unwrap();
        let words = syntax_tokens(&words, &options);
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().unwrap().span.end;
        let mut state = ParserState::new(&words, &options);
        let product = recovered_forethought_tanru_unit_parser!()
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .expect("real complete GEK product")
            .into_owned();
        let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
            error_index: 91,
            tokens: vec1::Vec1::new(words[0].clone()),
        });
        // These synthetic wrapper states test lossless mappings, not winning
        // malformed input or classifier eligibility.
        for payload in [
            recovered::Recovered::valid(product.clone()),
            recovered::Recovered::prefix(vec![error.clone()], product.clone()),
            recovered::Recovered::error(error.clone()),
        ] {
            let mapped: recovered::Recovered<recovered::TanruUnitAtomSyntax> =
                payload.clone().grammar_map_to();
            let recovered::Recovered::Valid(atom) = mapped else {
                panic!("carrier is valid")
            };
            assert!(atom.conversions.is_empty());
            let recovered::Recovered::Valid(base) = atom.base.as_ref() else {
                panic!("carrier base is valid")
            };
            let recovered::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(actual) =
                base.as_ref()
            else {
                panic!("same GEK carrier")
            };
            assert_eq!(actual.as_ref(), &payload);
        }
        let base = recovered::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(Arc::new(
            recovered::Recovered::valid(product),
        ));
        let se_words = segment_words_with_modifiers("se").unwrap();
        let se_words = syntax_tokens(&se_words, &options);
        for base in [
            recovered::Recovered::valid(base.clone()),
            recovered::Recovered::prefix(vec![error.clone()], base),
            recovered::Recovered::error(error.clone()),
        ] {
            let mapped: recovered::Recovered<recovered::TanruUnitAtomSyntax> =
                base.clone().grammar_map_to();
            let recovered::Recovered::Valid(atom) = mapped else {
                panic!("atom preserves base slot")
            };
            assert_eq!(atom.base.as_ref(), &base);
            for inner in [
                recovered::Recovered::Valid(atom.clone()),
                recovered::Recovered::prefix_boxed(vec![error.clone()], atom.clone()),
                recovered::Recovered::error(error.clone()),
            ] {
                for token in [
                    recovered::Recovered::valid(se_words[0].clone()),
                    recovered::Recovered::prefix(vec![error.clone()], se_words[0].clone()),
                    recovered::Recovered::error(error.clone()),
                ] {
                    let se = RecoveredTokenClause {
                        value: token,
                        free_modifiers: Vec::new(),
                    };
                    let mapped: recovered::Recovered<recovered::TanruUnitAtomSyntax> =
                        (se.clone(), inner.clone()).grammar_map_to();
                    let stored_se = store_recovered_clause(se);
                    let mut expected = inner.clone();
                    match &mut expected {
                        recovered::Recovered::Valid(value) => {
                            value.conversions.insert(0, stored_se)
                        }
                        recovered::Recovered::Prefix(prefix) => {
                            prefix.value.conversions.insert(0, stored_se)
                        }
                        recovered::Recovered::Error(_) => {}
                    }
                    assert_eq!(mapped, expected);
                }
            }
        }
    }

    /// The same memo-replay equivalence, across a truncation that actually runs.
    ///
    /// `strict_tail_memo_replay_preserves_rolled_back_diagnostics` below takes its
    /// checkpoint before any frame is open, so `checkpoint.frame` is `None` and the
    /// `if let Some(mark)` branch of `restore_diagnostics` - the one that truncates the
    /// frame's `diagnostic_observations` - never executes in it. That branch was added
    /// with the frame mark and has had no test that can observe it. This one enters a
    /// frame first, so the checkpoint carries a mark and the truncation is exercised.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn frame_scoped_truncation_preserves_memo_replay_equivalence() {
        let source = "klama le";
        let options = ParseOptions::default();
        let words = segment_words_with_modifiers(source).unwrap();
        let words = syntax_tokens(&words, &options);
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().unwrap().span.end;
        let mut state = ParserState::new(&words, &options);

        // The difference from the vacuous test: open a frame BEFORE checkpointing, so
        // the checkpoint records a frame mark and the truncation branch is live.
        state.begin_syntax_memo_rule_frame();
        let checkpoint = state.diagnostic_checkpoint();
        let marked_len = checkpoint
            .frame
            .as_ref()
            .expect("checkpoint must carry a frame mark or this test cannot observe the truncation")
            .observation_len;

        let parser = generated_model::strict_selbri_simple_bridi_tail_parser(
            generated_model::strict_generated_zantufa_selbri_entry_parser(),
            generated_model::strict_generated_term_parser(),
            generated_model::strict_generated_free_modifier_parser(),
        );
        assert!(
            parser
                .clone()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .is_err()
        );
        let fresh = state.diagnostic_candidates_snapshot();
        assert!(!fresh.is_empty());

        // The parse must have recorded observations past the mark, or the truncation below
        // removes nothing and the equivalence assertion is trivially true.
        let recorded_len = state
            .syntax_memo_rule_frames
            .last()
            .expect("the frame opened above is still open")
            .diagnostic_observations
            .len();
        assert!(
            recorded_len > marked_len,
            "the parse must record observations past the mark ({recorded_len} <= {marked_len})"
        );

        // Roll back through the frame-aware path, which truncates the frame's
        // observation suffix as well as restoring the candidates.
        state.restore_diagnostics(checkpoint);
        assert_eq!(
            state
                .syntax_memo_rule_frames
                .last()
                .expect("the frame opened above is still open")
                .diagnostic_observations
                .len(),
            marked_len,
            "restore must truncate the frame's observations back to the mark"
        );
        assert!(
            parser
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .is_err()
        );
        let replayed = state.diagnostic_candidates_snapshot();
        assert_eq!(
            replayed.len(),
            fresh.len(),
            "truncating the frame's observations must not change what a re-parse reports"
        );
        state.finish_syntax_memo_rule_frame();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strict_tail_memo_replay_preserves_rolled_back_diagnostics() {
        let source = "klama le";
        let options = ParseOptions::default();
        let words = segment_words_with_modifiers(source).unwrap();
        let words = syntax_tokens(&words, &options);
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().unwrap().span.end;
        let mut state = ParserState::new(&words, &options);
        let checkpoint = state.diagnostic_checkpoint();
        let parser = generated_model::strict_selbri_simple_bridi_tail_parser(
            generated_model::strict_generated_zantufa_selbri_entry_parser(),
            generated_model::strict_generated_term_parser(),
            generated_model::strict_generated_free_modifier_parser(),
        );
        // The tail completes before LE; the complete-input check fails. The
        // failed optional description must still supply its deeper diagnostic.
        assert!(
            parser
                .clone()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .is_err()
        );
        let fresh = state.diagnostic_candidates_snapshot();
        assert!(!fresh.is_empty());
        assert!(fresh.iter().any(|error| error.span().start == source.len()));
        // Model reject_output's rollback, then the ordinary same-product
        // fallback. Memo replay must be observationally equivalent to parsing.
        state.restore_diagnostics(checkpoint);
        assert!(
            parser
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .is_err()
        );
        let replayed = state.diagnostic_candidates_snapshot();
        assert_eq!(replayed.len(), fresh.len());
        assert!(
            replayed
                .iter()
                .zip(&fresh)
                .all(|(a, b)| a.same_report_content(b))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn priority_tail_rejects_outer_and_nested_recovery_uncertainty() {
        use generated_runtime::OutputRejection;
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").unwrap();
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
        for (source, admitted) in [("broda", false), ("ga'o je ke'i gi broda gi brode", true)] {
            let words = segment_words_with_modifiers(source).unwrap();
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().unwrap().span.end;
            let mut state = ParserState::new(&words, &options);
            let parsed = recovered_selbri_simple_bridi_tail_parser!()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .expect("complete recovered tail")
                .into_shared();
            let tail = parsed.as_ref();
            assert_eq!(
                PriorityTailRejection.rejects_in_dialect(parsed.as_ref(), flags),
                !admitted
            );
            let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                error_index: 1,
                tokens: vec1::Vec1::new(words[0].clone()),
            });
            // These are mapping/classifier uncertainty controls, not claims
            // that the parser selects any malformed-but-winning recovery row.
            let valid = recovered::Recovered::valid(tail.clone());
            assert_eq!(
                PriorityTailRejection.rejects_in_dialect(&valid, flags),
                !admitted
            );
            let prefix = recovered::Recovered::prefix(vec![error.clone()], tail.clone());
            assert!(PriorityTailRejection.rejects_in_dialect(&prefix, flags));
            let missing: recovered::Recovered<recovered::SelbriSimpleBridiTailSyntax> =
                recovered::Recovered::error(error.clone());
            assert!(PriorityTailRejection.rejects_in_dialect(&missing, flags));
            let mut nested = tail.clone();
            nested.selbri = std::sync::Arc::new(recovered::Recovered::error(error));
            assert!(PriorityTailRejection.rejects_in_dialect(&nested, flags));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn enclosed_rejection_preserves_recovered_uncertainty_boundary() {
        use generated_runtime::OutputRejection;
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").unwrap();
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let words = syntax_tokens(
            &segment_words_with_modifiers("ga broda gi brode").unwrap(),
            &options,
        );
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().unwrap().span.end;
        let mut state = ParserState::new(&words, &options);
        let parsed = recovered_forethought_tanru_unit_parser!()
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .expect("complete recovered candidate")
            .into_shared();
        let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
        let value = parsed.as_ref();
        // A completed GA-family product is eligible in the enclosed route;
        // uncertainty remains fail-closed through the recovered adapter.
        assert!(!EnclosedAtomRejection.rejects_in_dialect(value, flags));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn routed_nary_ga_reaches_complete_atom_before_bridi_prefix() {
        let source = "ga broda gi brode gi brodi";
        for definition in [
            "(+ZANTUFA-SELBRI)",
            "(+ZANTUFA-SELBRI +ZANTUFA-CONNECTIVES)",
        ] {
            let dialect = parse_dialect_definition(definition).unwrap();
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let words = segment_words_with_modifiers(source).unwrap();
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().unwrap().span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            let candidate = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                generated_model::strict_generated_co_selbri_parser(),
                generated_model::strict_generated_zantufa_tcita_selci_parser(),
                generated_model::strict_generated_zantufa_boundary_term_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
            .map(|value| value.into_owned());
            let candidate = generated_runtime::reject_output(candidate, StandaloneAtomRejection);
            let mut state = ParserState::new(&words, &options);
            let candidate_extent = with_consumed_extent(candidate.map(|_| ()).boxed())
                .parse_with_state(input, &mut state)
                .into_result();
            assert_eq!(
                candidate_extent.expect("candidate parses"),
                6,
                "complete admitted atom: {definition}"
            );
            for (route, parser) in [
                ("strict bridi_tail", generated_model::strict_generated_bridi_tail_parser().map(|_| ()).boxed()),
                ("strict bo_grouped_bridi_tail_without_tail_terms", generated_model::strict_generated_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
                ("strict description_relative_bridi_tail", generated_model::strict_generated_description_relative_bridi_tail_parser().map(|_| ()).boxed()),
                ("strict description_relative_bo_grouped_bridi_tail_without_tail_terms", generated_model::strict_generated_description_relative_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
                ("recovery_checkpoint_strict bridi_tail", generated_model::recovery_checkpoint_strict_generated_bridi_tail_parser().map(|_| ()).boxed()),
                ("recovery_checkpoint_strict bo_grouped_bridi_tail_without_tail_terms", generated_model::recovery_checkpoint_strict_generated_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
                ("recovery_checkpoint_strict description_relative_bridi_tail", generated_model::recovery_checkpoint_strict_generated_description_relative_bridi_tail_parser().map(|_| ()).boxed()),
                ("recovery_checkpoint_strict description_relative_bo_grouped_bridi_tail_without_tail_terms", generated_model::recovery_checkpoint_strict_generated_description_relative_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
                ("recovered bridi_tail", generated_model::recovered_generated_bridi_tail_parser().map(|_| ()).boxed()),
                ("recovered bo_grouped_bridi_tail_without_tail_terms", generated_model::recovered_generated_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
                ("recovered description_relative_bridi_tail", generated_model::recovered_generated_description_relative_bridi_tail_parser().map(|_| ()).boxed()),
                ("recovered description_relative_bo_grouped_bridi_tail_without_tail_terms", generated_model::recovered_generated_description_relative_bo_grouped_bridi_tail_without_tail_terms_parser().map(|_| ()).boxed()),
            ] {
                let mut state = ParserState::new(&words, &options);
                let tail_extent = with_consumed_extent(parser)
                    .parse_with_state(input, &mut state).into_result();
                assert_eq!(tail_extent.expect("complete tail"), 6,
                    "no shorter bridi prefix: {definition}, {route}");
                assert_eq!(state.warnings.len(), 1, "one atom warning: {definition}, {route}");
                assert_eq!(state.warnings[0].kind, ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit);
                assert_eq!(state.warnings[0].anchor_index, 0);
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joik_partition_matches_all_384_measured_modifier_free_cells() {
        use GiOrder::{Final, Initial};
        use JoikHead::{Bihi, Ja, Joi};
        // Independent expected rows transcribed from the frozen baseline
        // audit. Order is left-GAhO, NA, SE, right-GAhO, false before true.
        // Numeric masks remain absent from product classification.
        let no_endpoints = [
            true, false, true, false, true, false, true, false, false, false, false, false, false,
            false, false, false,
        ];
        let default_bihi = [
            true, false, true, false, true, false, true, false, false, true, false, true, false,
            false, false, false,
        ];
        let mut cells = 0;
        let mut overlaps = 0;
        for (head, order, connectives, expected) in [
            (Joi, Final, false, no_endpoints),
            (Bihi, Final, false, default_bihi),
            (Ja, Final, false, no_endpoints),
            (Joi, Initial, false, [false; 16]),
            (Bihi, Initial, false, [false; 16]),
            (Ja, Initial, false, [false; 16]),
            (Joi, Final, true, [true; 16]),
            (Bihi, Final, true, [true; 16]),
            (Ja, Final, true, no_endpoints),
            (Joi, Initial, true, [true; 16]),
            (Bihi, Initial, true, [true; 16]),
            (Ja, Initial, true, no_endpoints),
        ] {
            let mut row = 0;
            for left_gaho in [false, true] {
                for na in [false, true] {
                    for se in [false, true] {
                        for right_gaho in [false, true] {
                            for bo in [false, true] {
                                let key = JoikOwnershipKey {
                                    head,
                                    order,
                                    left_gaho,
                                    na,
                                    se,
                                    right_gaho,
                                    bo,
                                    connectives,
                                };
                                let actual = key.baseline_overlaps();
                                assert_eq!(actual, expected[row], "{key:?}");
                                // Parse both real generated opener products;
                                // the table's hand-constructed key must match
                                // their independently extracted typed fields.
                                let mut payload = Vec::new();
                                if left_gaho {
                                    payload.push("ga'o");
                                }
                                if na {
                                    payload.push("na");
                                }
                                if se {
                                    payload.push("se");
                                }
                                payload.push(match head {
                                    Joi => "joi",
                                    Ja => "je",
                                    Bihi => "bi'i",
                                });
                                if right_gaho {
                                    payload.push("ke'i");
                                }
                                let payload = payload.join(" ");
                                let mut source = match order {
                                    Initial => format!("gi {payload}"),
                                    Final => format!("{payload} gi"),
                                };
                                if bo {
                                    source.push_str(" bo");
                                }
                                let definition = if connectives {
                                    "(+ZANTUFA-SELBRI +ZANTUFA-CONNECTIVES)"
                                } else {
                                    "(+ZANTUFA-SELBRI)"
                                };
                                let dialect =
                                    parse_dialect_definition(definition).expect("measured axis");
                                let options =
                                    ParseOptions::default().with_dialect_definition(&dialect);
                                let words = segment_words_with_modifiers(&source)
                                    .expect("valid morphology");
                                let words = syntax_tokens(&words, &options);
                                let spanned = tokens::spanned_tokens(&words);
                                let eoi = spanned.last().expect("nonempty opener").span.end;
                                let input =
                                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
                                let mut state = ParserState::new(&words, &options);
                                let strict = generated_model::strict_zantufa_atom_gek_parser(
                                    generated_model::strict_generated_zantufa_tcita_selci_parser(),
                                    generated_model::strict_generated_free_modifier_parser(),
                                )
                                .parse_with_state(input, &mut state)
                                .into_result()
                                .unwrap_or_else(|errors| panic!("{source}: {errors:?}"))
                                .into_shared();
                                assert_eq!(
                                    strict_gek_joik_key(&strict, connectives),
                                    Some(key),
                                    "strict {source}"
                                );
                                let mut state = ParserState::new(&words, &options);
                                let recovered = recovered_atom_gek_parser!()
                                    .parse_with_state(input, &mut state)
                                    .into_result()
                                    .unwrap_or_else(|errors| {
                                        panic!("recovered {source}: {errors:?}")
                                    })
                                    .into_shared();
                                assert_eq!(
                                    recovered_gek_joik_key(&recovered, connectives),
                                    Some(key),
                                    "recovered {source}"
                                );
                                let full_source = format!("{source} broda gi brode");
                                let full_words = segment_words_with_modifiers(&full_source)
                                    .expect("valid full product");
                                let full_words = syntax_tokens(&full_words, &options);
                                let full_spanned = tokens::spanned_tokens(&full_words);
                                let full_eoi =
                                    full_spanned.last().expect("nonempty product").span.end;
                                let full_input = full_spanned
                                    .as_slice()
                                    .split_spanned(SimpleSpan::from(full_eoi..full_eoi));
                                let mut state = ParserState::new(&full_words, &options);
                                let strict_product = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                                    generated_model::strict_generated_co_selbri_parser(),
                                    generated_model::strict_generated_zantufa_tcita_selci_parser(),
                                    generated_model::strict_generated_zantufa_boundary_term_parser(),
                                    generated_model::strict_generated_free_modifier_parser(),
                                ).parse_with_state(full_input, &mut state).into_result()
                                    .unwrap_or_else(|errors| panic!("{full_source}: {errors:?}")).into_shared();
                                let mut state = ParserState::new(&full_words, &options);
                                let recovered_product = recovered_forethought_tanru_unit_parser!()
                                    .parse_with_state(full_input, &mut state)
                                    .into_result()
                                    .unwrap_or_else(|errors| {
                                        panic!("recovered {full_source}: {errors:?}")
                                    })
                                    .into_shared();
                                for pair in [false, true] {
                                    // This is the pure ownership gate, not a
                                    // second parse under a different dialect.
                                    let mut flags =
                                        generated_runtime::SyntaxGrammarDialect::from_options(
                                            &options,
                                        );
                                    flags.zantufa_selbri_atom_reinterpretation_enabled = pair;
                                    let presence = if pair || !expected[row] {
                                        ZantufaTanruAtomPresence::Present
                                    } else {
                                        ZantufaTanruAtomPresence::Absent
                                    };
                                    assert_eq!(
                                        strict_standalone_presence(&strict_product, &flags),
                                        presence,
                                        "{full_source}, pair={pair}"
                                    );
                                    assert_eq!(
                                        recovered_standalone_presence(&recovered_product, &flags),
                                        presence,
                                        "recovered {full_source}, pair={pair}"
                                    );
                                }
                                cells += 1;
                                overlaps += usize::from(actual);
                            }
                            row += 1;
                        }
                    }
                }
            }
            assert_eq!(row, 16);
        }
        assert_eq!(cells, 384);
        assert_eq!(overlaps, 172);
    }

    // Test-only adapters exercise the real output-rejection combinator with
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joik_key_rejects_prefix_and_error_on_every_marker() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let words =
            segment_words_with_modifiers("ga'o na se joi ke'i gi bo").expect("valid morphology");
        let words = syntax_tokens(&words, &options);
        let spanned = tokens::spanned_tokens(&words);
        let eoi = spanned.last().expect("nonempty opener").span.end;
        let mut state = ParserState::new(&words, &options);
        let original = recovered_atom_gek_parser!()
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .expect("complete full-field opener")
            .into_owned();
        assert!(recovered_gek_joik_key(&original, false).is_some());
        for field in ["left_gaho", "na", "se", "head", "right_gaho", "gi", "bo"] {
            for prefix in [false, true] {
                let mut candidate = original.clone();
                let recovered::Recovered::Valid(body) = Arc::make_mut(&mut candidate.body) else {
                    panic!("complete body");
                };
                let recovered::ZantufaAtomGekBodySyntax::ZantufaAtomFinalGiOpener(opener) =
                    body.as_mut()
                else {
                    panic!("final-GI opener");
                };
                let recovered::Recovered::Valid(opener) = Arc::make_mut(opener) else {
                    panic!("final-GI opener");
                };
                let recovered::Recovered::Valid(payload) = Arc::make_mut(&mut opener.payload)
                else {
                    panic!("complete payload");
                };
                let recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(joik) =
                    payload.as_mut()
                else {
                    panic!("complete JOIK");
                };
                let recovered::Recovered::Valid(joik) = Arc::make_mut(joik) else {
                    panic!("complete JOIK");
                };
                let clause = match field {
                    "left_gaho" => joik.left_gaho.as_mut().unwrap(),
                    "na" => joik.na.as_mut().unwrap(),
                    "se" => joik.se.as_mut().unwrap(),
                    "head" => &mut joik.head,
                    "right_gaho" => joik.right_gaho.as_mut().unwrap(),
                    "gi" => &mut opener.gi,
                    "bo" => candidate.bo.as_mut().unwrap(),
                    _ => unreachable!("enumerated marker field"),
                };
                let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                    error_index: 59,
                    tokens: vec1::Vec1::new(words[0].clone()),
                });
                clause.value = if prefix {
                    recovered::Recovered::prefix(
                        vec![error],
                        parsed_value(&clause.value).expect("parsed marker").clone(),
                    )
                } else {
                    recovered::Recovered::error(error)
                };
                // Synthetic key-boundary test, not a winning recovery row.
                assert_eq!(
                    recovered_gek_joik_key(&candidate, false),
                    None,
                    "{field}, prefix={prefix}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joik_products_outside_adjudicated_domain_are_unproven() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
        for source in [
            "ga'o to mi klama toi joi gi broda gi brode",
            "na to mi klama toi joi gi broda gi brode",
            "se to mi klama toi joi gi broda gi brode",
            "joi to mi klama toi gi broda gi brode",
            "joi ke'i to mi klama toi gi broda gi brode",
            "joi gi to mi klama toi broda gi brode",
            "gi to mi klama toi joi broda gi brode",
            "joi gi bo to mi klama toi broda gi brode",
            "pu gi broda gi brode",
            "gi pu broda gi brode",
            "joi gi broda gi brode gi brodi",
            "na'e joi gi broda gi brode",
            "joi gi broda gi brode gi'i",
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty product").span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            let mut state = ParserState::new(&words, &options);
            let strict = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                generated_model::strict_generated_co_selbri_parser(),
                generated_model::strict_generated_zantufa_tcita_selci_parser(),
                generated_model::strict_generated_zantufa_boundary_term_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
            .parse_with_state(input, &mut state)
            .into_result()
            .unwrap_or_else(|errors| panic!("{source}: {errors:?}"))
            .into_shared();
            assert_eq!(
                strict_standalone_presence(&strict, &flags),
                ZantufaTanruAtomPresence::Unproven,
                "{source}"
            );
            let mut state = ParserState::new(&words, &options);
            let recovered = recovered_forethought_tanru_unit_parser!()
                .parse_with_state(input, &mut state)
                .into_result()
                .unwrap_or_else(|errors| panic!("recovered {source}: {errors:?}"))
                .into_shared();
            assert_eq!(
                recovered_standalone_presence(&recovered, &flags),
                ZantufaTanruAtomPresence::Unproven,
                "recovered {source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_rejection_reads_the_actual_parser_dialect() {
        for (definition, admitted) in [
            ("(+ZANTUFA-SELBRI -ZANTUFA-CONNECTIVES)", true),
            ("(+ZANTUFA-SELBRI +ZANTUFA-CONNECTIVES)", false),
            (
                "(+ZANTUFA-SELBRI +ZANTUFA-SELBRI-REINTERPRETATION -ZANTUFA-CONNECTIVES)",
                true,
            ),
            (
                "(+ZANTUFA-SELBRI +ZANTUFA-SELBRI-REINTERPRETATION +ZANTUFA-CONNECTIVES)",
                true,
            ),
        ] {
            let dialect = parse_dialect_definition(definition).expect("explicit axis");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let words =
                segment_words_with_modifiers("ga'o joi gi broda gi brode").expect("morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty source").span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            {
                let candidate = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                    generated_model::strict_generated_co_selbri_parser(),
                    generated_model::strict_generated_zantufa_tcita_selci_parser(),
                    generated_model::strict_generated_zantufa_boundary_term_parser(),
                    generated_model::strict_generated_free_modifier_parser(),
                )
                .map(|value| value.into_owned());
                let mut state = ParserState::new(&words, &options);
                let result = generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                    .parse_with_state(input, &mut state)
                    .into_result();
                assert_eq!(result.is_ok(), admitted, "strict {definition}");
                assert_eq!(state.warnings.len(), usize::from(admitted));
            }
            {
                let candidate =
                    recovered_forethought_tanru_unit_parser!().map(|value| value.into_owned());
                let mut state = ParserState::new(&words, &options);
                let result = generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                    .parse_with_state(input, &mut state)
                    .into_result();
                assert_eq!(result.is_ok(), admitted, "recovered {definition}");
                assert_eq!(state.warnings.len(), usize::from(admitted));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completed_ga_rejection_rolls_back_warning_before_real_baseline_fallback() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        for (source, atom_wins) in [
            ("gu'a broda gi brode", false),
            ("se gu'a broda gi brode", false),
            ("se gu'a broda gi brode co brodi", false),
            ("se to mi klama toi gu'a broda gi brode", true),
            ("se to mi klama toi gu'a broda gi brode co brodi", true),
            ("gu'a bo broda gi brode", true),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty source").span.end;
            for recovered_mode in [false, true] {
                let mut state = ParserState::new(&words, &options);
                let parser = if recovered_mode {
                    let candidate =
                        recovered_forethought_tanru_unit_parser!().map(|value| value.into_owned());
                    generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                        .map(|_| true)
                        .or(generated_model::recovered_generated_co_selbri_parser().map(|_| false))
                        .boxed()
                } else {
                    let candidate = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                        generated_model::strict_generated_co_selbri_parser(),
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_zantufa_boundary_term_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|value| value.into_owned());
                    generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                        .map(|_| true)
                        .or(generated_model::strict_generated_co_selbri_parser().map(|_| false))
                        .boxed()
                };
                let result = parser
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result()
                    .unwrap_or_else(|errors| {
                        panic!("{source}, recovered={recovered_mode}: {errors:?}")
                    });
                assert_eq!(result, atom_wins, "{source}, recovered={recovered_mode}");
                assert_eq!(
                    state.warnings.len(),
                    usize::from(atom_wins),
                    "{source}, recovered={recovered_mode}"
                );
                if atom_wins {
                    assert_eq!(
                        state.warnings[0].kind,
                        ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                    );
                    assert_eq!(state.warnings[0].anchor_index, 0);
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joik_rejection_preserves_real_bridi_tail_survivors_and_their_warnings() {
        for (source, connectives, legacy_anchor) in [
            ("joi gi broda gi brode", false, None),
            ("se joi gi broda gi brode", false, None),
            ("ga'o joi gi broda gi brode", true, Some(0)),
            ("joi ke'i gi broda gi brode", true, Some(1)),
        ] {
            let definition = if connectives {
                "(+ZANTUFA-SELBRI +ZANTUFA-CONNECTIVES)"
            } else {
                "(+ZANTUFA-SELBRI -ZANTUFA-CONNECTIVES)"
            };
            let dialect = parse_dialect_definition(definition).expect("explicit axis");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty source").span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            {
                let baseline = generated_model::strict_generated_bridi_tail_parser();
                let mut baseline_state = ParserState::new(&words, &options);
                let expected = baseline
                    .clone()
                    .parse_with_state(input, &mut baseline_state)
                    .into_result()
                    .unwrap_or_else(|errors| panic!("strict baseline {source}: {errors:?}"));
                let candidate = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                    generated_model::strict_generated_co_selbri_parser(),
                    generated_model::strict_generated_zantufa_tcita_selci_parser(),
                    generated_model::strict_generated_zantufa_boundary_term_parser(),
                    generated_model::strict_generated_free_modifier_parser(),
                )
                .map(|value| value.into_owned());
                // Prove this is a completed candidate rejected by the actual
                // classifier, rather than a failing parser bypassing rejection.
                let mut candidate_state = ParserState::new(&words, &options);
                let parsed = candidate
                    .clone()
                    .parse_with_state(input, &mut candidate_state)
                    .into_result()
                    .expect("complete JOIK candidate");
                let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
                assert_eq!(
                    strict_standalone_presence(&parsed, &flags),
                    ZantufaTanruAtomPresence::Absent
                );
                assert_eq!(candidate_state.warnings.len(), 1);
                assert_eq!(
                    candidate_state.warnings[0].kind,
                    ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                );
                let parser = generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                    .map(|_| None)
                    .or(baseline.map(Some));
                let mut state = ParserState::new(&words, &options);
                let survivor = parser
                    .parse_with_state(input, &mut state)
                    .into_result()
                    .expect("real baseline fallback")
                    .expect("baseline wins");
                assert_eq!(survivor, expected, "strict survivor {source}");
                // No finish/dedup: legacy warning must survive exactly once,
                // and the rejected atom warning must be completely absent.
                assert_eq!(state.warnings, baseline_state.warnings, "strict {source}");
                assert_eq!(state.warnings.len(), usize::from(legacy_anchor.is_some()));
                if let Some(anchor) = legacy_anchor {
                    assert_eq!(
                        state.warnings[0].kind,
                        ExperimentalConstruct::ExperimentalZantufaGek
                    );
                    assert_eq!(state.warnings[0].anchor_index, anchor);
                    assert_eq!(
                        state.warnings[0].anchor,
                        Token::bare(words[anchor].core_word().clone())
                    );
                }
            }
            {
                let baseline = generated_model::recovered_generated_bridi_tail_parser();
                let mut baseline_state = ParserState::new(&words, &options);
                let expected = baseline
                    .clone()
                    .parse_with_state(input, &mut baseline_state)
                    .into_result()
                    .unwrap_or_else(|errors| panic!("recovered baseline {source}: {errors:?}"));
                let candidate =
                    recovered_forethought_tanru_unit_parser!().map(|value| value.into_owned());
                // Prove this is a completed candidate rejected by the actual
                // classifier, rather than a failing parser bypassing rejection.
                let mut candidate_state = ParserState::new(&words, &options);
                let parsed = candidate
                    .clone()
                    .parse_with_state(input, &mut candidate_state)
                    .into_result()
                    .expect("complete JOIK candidate");
                let flags = generated_runtime::SyntaxGrammarDialect::from_options(&options);
                assert_eq!(
                    recovered_standalone_presence(&parsed, &flags),
                    ZantufaTanruAtomPresence::Absent
                );
                // Exercise the same real parser/rejection/fallback sequence
                // with a synthetic uncertain required opener. This proves
                // Unproven rollback, not a malformed-but-winning recovery row.
                for prefix in [false, true] {
                    let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                        error_index: 71,
                        tokens: vec1::Vec1::new(words[0].clone()),
                    });
                    let uncertain = candidate.clone().map(move |mut value| {
                        let recovered::Recovered::Valid(opener) = Arc::unwrap_or_clone(value.gek)
                        else {
                            panic!("complete parsed opener before mutation");
                        };
                        value.gek = Arc::new(if prefix {
                            recovered::Recovered::prefix_boxed(vec![error.clone()], opener)
                        } else {
                            recovered::Recovered::error(error.clone())
                        });
                        assert_eq!(
                            recovered_standalone_presence(&value, &flags),
                            ZantufaTanruAtomPresence::Unproven
                        );
                        value
                    });
                    let parser =
                        generated_runtime::reject_output(uncertain, StandaloneAtomRejection)
                            .map(|_| None)
                            .or(baseline.clone().map(Some));
                    let mut state = ParserState::new(&words, &options);
                    let survivor = parser
                        .parse_with_state(input, &mut state)
                        .into_result()
                        .expect("uncertain atom rejection permits real fallback")
                        .expect("baseline wins after Unproven");
                    assert_eq!(survivor, expected, "Unproven prefix={prefix}: {source}");
                    assert_eq!(
                        state.warnings, baseline_state.warnings,
                        "Unproven prefix={prefix}: {source}"
                    );
                }
                assert_eq!(candidate_state.warnings.len(), 1);
                assert_eq!(
                    candidate_state.warnings[0].kind,
                    ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                );
                let parser = generated_runtime::reject_output(candidate, StandaloneAtomRejection)
                    .map(|_| None)
                    .or(baseline.map(Some));
                let mut state = ParserState::new(&words, &options);
                let survivor = parser
                    .parse_with_state(input, &mut state)
                    .into_result()
                    .expect("real baseline fallback")
                    .expect("baseline wins");
                assert_eq!(survivor, expected, "recovered survivor {source}");
                // No finish/dedup: legacy warning must survive exactly once,
                // and the rejected atom warning must be completely absent.
                assert_eq!(
                    state.warnings, baseline_state.warnings,
                    "recovered {source}"
                );
                assert_eq!(state.warnings.len(), usize::from(legacy_anchor.is_some()));
                if let Some(anchor) = legacy_anchor {
                    assert_eq!(
                        state.warnings[0].kind,
                        ExperimentalConstruct::ExperimentalZantufaGek
                    );
                    assert_eq!(state.warnings[0].anchor_index, anchor);
                    assert_eq!(
                        state.warnings[0].anchor,
                        Token::bare(words[anchor].core_word().clone())
                    );
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn guha_right_attachment_never_uses_occupied_recovery_slots_as_proof() {
        for source in ["broda", "broda co brode"] {
            let options = ParseOptions::default();
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty operand").span.end;
            let mut state = ParserState::new(&words, &options);
            let parsed = generated_model::recovered_generated_co_selbri_parser()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .expect("complete right operand");
            let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                error_index: 12,
                tokens: vec1::Vec1::new(words[0].clone()),
            });
            // The second source still has a complete CO tail. That positive
            // attachment signal cannot compensate for an unproven leading L6.
            for leading_selbri in [
                recovered::Recovered::error(error.clone()),
                recovered::Recovered::prefix(
                    vec![error],
                    parsed_value(&parsed.leading_selbri)
                        .expect("parsed leading subtree")
                        .clone(),
                ),
            ] {
                let incomplete = recovered::CoSelbriSyntax {
                    leading_selbri: std::sync::Arc::new(leading_selbri),
                    co_tail: parsed.co_tail.clone(),
                };
                assert_eq!(recovered_right_operand_extends_past_l6(&incomplete), None);
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn guha_right_attachment_projection_stops_at_the_l6_operand() {
        for (source, connectives, expected) in [
            ("broda", false, Some(false)),
            ("broda co brode", false, Some(true)),
            ("broda brode", false, Some(true)),
            ("broda je brode", false, Some(true)),
            ("broda je bo brode", false, Some(true)),
            ("broda bo brode", false, Some(false)),
            ("ke broda brode ke'e", false, Some(false)),
            // The ordinary KE atom contains TanruSelbri, not CoSelbri.
            // Internal KE/CO is an already adopted Connectives-only atom.
            ("ke broda co brode ke'e", false, None),
            ("ke broda co brode ke'e", true, Some(false)),
            ("ke broda je brode ke'e", false, Some(false)),
        ] {
            let options = if connectives {
                let dialect =
                    parse_dialect_definition("(+ZANTUFA-CONNECTIVES)").expect("feature axis");
                ParseOptions::default().with_dialect_definition(&dialect)
            } else {
                ParseOptions::default()
            };
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty operand").span.end;
            let mut state = ParserState::new(&words, &options);
            let parsed = generated_model::strict_generated_co_selbri_parser()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result();
            assert_eq!(
                parsed
                    .as_ref()
                    .ok()
                    .map(strict_right_operand_extends_past_l6),
                expected,
                "{source}, connectives={connectives}: {parsed:?}"
            );
            let mut recovered_state = ParserState::new(&words, &options);
            let recovered = generated_model::recovered_generated_co_selbri_parser()
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut recovered_state,
                )
                .into_result();
            assert_eq!(
                recovered
                    .as_ref()
                    .ok()
                    .map(recovered_right_operand_extends_past_l6),
                expected.map(Some),
                "{source}, connectives={connectives}: {recovered:?}"
            );
        }
    }

    // The production recognizer is a prefix test. Consume the remainder only
    // in this harness so the driver's EOF check does not turn it into a full
    // input recognizer or hide where the source guard actually stopped.
    #[requires(true)]
    #[ensures(true)]
    fn with_consumed_extent<'tokens>(
        parser: crate::grammar::BoxedParser<'tokens, ()>,
    ) -> crate::grammar::BoxedParser<'tokens, usize> {
        custom(move |input| {
            input.parse(&parser)?;
            let consumed = MappedInput::cursor_location(input.cursor().inner());
            while input.next().is_some() {}
            Ok(consumed)
        })
        .boxed()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_boundary_term_guards_preserve_exact_prefix_extents() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        // Original source :27-32: positive terms, continuation exclusion,
        // mandatory KEhE lookahead, NA bridi/connective exclusions and BO.
        for (source, expected) in [
            ("mi", Some(1)),
            ("mi e do", Some(3)),
            ("mi je pu bo cu broda", Some(1)),
            ("ke mi", Some(2)),
            ("ke mi ke'e", None),
            ("na ku", Some(2)),
            ("na broda", None),
            ("na gi'a", None),
            ("pu bo mi", None),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty case").span.end;
            for checkpoint in [false, true] {
                let mut state = ParserState::new(&words, &options);
                let parser = if checkpoint {
                    generated_model::recovery_checkpoint_strict_generated_zantufa_boundary_term_parser()
                } else {
                    generated_model::strict_generated_zantufa_boundary_term_parser()
                };
                let result = with_consumed_extent(parser)
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result();
                assert_eq!(result.ok(), expected, "{source}, checkpoint={checkpoint}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_boundary_is_not_the_shared_normal_term_language() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        for (source, source_extent, shared_extent) in
            [("mi", Some(1), Some(1)), ("ke mi ke'e", None, Some(3))]
        {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty case").span.end;
            for (parser, expected) in [
                (
                    generated_model::strict_generated_zantufa_boundary_term_parser(),
                    source_extent,
                ),
                (
                    generated_model::strict_generated_normal_term_parser()
                        .map(|_| ())
                        .boxed(),
                    shared_extent,
                ),
            ] {
                let mut state = ParserState::new(&words, &options);
                let result = with_consumed_extent(parser)
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result();
                assert_eq!(result.ok(), expected, "{source}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_opener_se_modifier_evidence_is_local_and_fail_closed() {
        use ZantufaTanruAtomPresence::{Absent as A, Present as P, Unproven};
        for (source, expected) in [
            ("ga", A),
            ("se ga", A),
            ("se ui gu'a", A),
            ("se gu'a to mi klama toi", A),
            ("se to mi klama toi ga", P),
            ("se to mi klama toi gu'a", P),
        ] {
            let options = ParseOptions::default();
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty opener").span.end;
            let mut state = ParserState::new(&words, &options);
            let product = generated_model::recovered_zantufa_atom_ga_opener_parser(
                generated_model::recovered_generated_free_modifier_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            )
            .parse_with_state(
                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                &mut state,
            )
            .into_result()
            .expect("complete recovered opener")
            .into_owned();
            assert_eq!(
                recovered_opener_se_free_presence(&product),
                expected,
                "{source}"
            );
            if expected != P {
                continue;
            }
            let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                error_index: 31,
                tokens: vec1::Vec1::new(words[0].clone()),
            });
            // Synthetic uncertainty mutations are NOT malformed-but-winning
            // parser fixtures. They test the evidence boundary independently.
            let mut uncertain_se = product.clone();
            let se = uncertain_se.se.as_mut().expect("positive has SE");
            se.value = recovered::Recovered::prefix(
                vec![error.clone()],
                parsed_value(&se.value).expect("parsed SE").clone(),
            );
            assert_eq!(recovered_opener_se_free_presence(&uncertain_se), Unproven);
            let mut uncertain_head = product.clone();
            uncertain_head.head.value = recovered::Recovered::error(error.clone());
            assert_eq!(recovered_opener_se_free_presence(&uncertain_head), Unproven);
            let mut uncertain_modifier = product;
            let modifier = &mut uncertain_modifier
                .se
                .as_mut()
                .expect("positive has SE")
                .free_modifiers[0];
            *modifier = Arc::new(recovered::Recovered::error(error));
            assert_eq!(
                recovered_opener_se_free_presence(&uncertain_modifier),
                Unproven
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standalone_ga_partition_uses_complete_products_and_same_axis_owners() {
        use ZantufaTanruAtomPresence::{Absent as A, Present as P};
        // Columns: Selbri only, raw pair, Selbri+Connectives, pair+Connectives.
        // Each row is parsed as a real, complete product before classification.
        for (source, expected) in [
            ("ga broda gi brode", [A, P, A, P]),
            ("se ga broda gi brode", [A, P, A, P]),
            ("ga bo broda gi brode", [P, P, A, P]),
            ("gu'a bo broda gi brode", [P, P, P, P]),
            ("se gu'a bo broda gi brode", [P, P, P, P]),
            ("gu'a broda gi brode", [A, A, A, A]),
            ("na'e se gu'a broda gi brode", [A, A, A, A]),
            ("gu'a broda gi brode co brodi", [A, P, A, P]),
            ("gu'a broda gi brode brodi", [A, P, A, P]),
            ("gu'a broda co brodi gi brode", [A, A, A, A]),
            ("gu'a broda gi brode bo brodi", [A, A, A, A]),
            ("gu'a broda gi ke brode brodi ke'e", [A, A, A, A]),
            ("gu'a broda gi brode gi brodi", [P, P, A, A]),
            ("gu'a broda gi brode gi'i", [P, P, A, A]),
            ("na'e ga broda gi brode", [P, P, P, P]),
            ("se to mi klama toi ga broda gi brode", [P, P, P, P]),
            ("se to mi klama toi gu'a broda gi brode", [P, P, P, P]),
            (
                "se to mi klama toi gu'a broda gi brode co brodi",
                [P, P, P, P],
            ),
            ("se gu'a broda gi brode co brodi", [A, P, A, P]),
            // Modifiers on the head or branch do not qualify as opener-SE
            // evidence; indicators on SE do not populate its free list.
            ("se gu'a to mi klama toi broda gi brode", [A, A, A, A]),
            ("se gu'a broda gi to mi klama toi brode", [A, A, A, A]),
            ("se ui gu'a broda gi brode", [A, A, A, A]),
        ] {
            for (column, (pair, connectives)) in
                [(false, false), (true, false), (false, true), (true, true)]
                    .into_iter()
                    .enumerate()
            {
                let definition = format!(
                    "(+ZANTUFA-SELBRI {}ZANTUFA-SELBRI-REINTERPRETATION {}ZANTUFA-CONNECTIVES)",
                    if pair { "+" } else { "-" },
                    if connectives { "+" } else { "-" },
                );
                let dialect = parse_dialect_definition(&definition).expect("explicit axis");
                let options = ParseOptions::default().with_dialect_definition(&dialect);
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                let words = syntax_tokens(&words, &options);
                let spanned = tokens::spanned_tokens(&words);
                let eoi = spanned.last().expect("nonempty product").span.end;
                let mut state = ParserState::new(&words, &options);
                let product = generated_model::strict_zantufa_forethought_tanru_unit_parser(
                    generated_model::strict_generated_co_selbri_parser(),
                    generated_model::strict_generated_zantufa_tcita_selci_parser(),
                    generated_model::strict_generated_zantufa_boundary_term_parser(),
                    generated_model::strict_generated_free_modifier_parser(),
                )
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result()
                .unwrap_or_else(|errors| panic!("{source}, {definition}: {errors:?}"));
                let product = product.into_shared();
                assert_eq!(
                    strict_standalone_presence(
                        &product,
                        &generated_runtime::SyntaxGrammarDialect::from_options(&options)
                    ),
                    expected[column],
                    "{source}, {definition}",
                );
                let mut state = ParserState::new(&words, &options);
                let recovered_product = recovered_forethought_tanru_unit_parser!()
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result()
                    .unwrap_or_else(|errors| panic!("recovered {source}, {definition}: {errors:?}"))
                    .into_shared();
                assert_eq!(
                    recovered_standalone_presence(
                        &recovered_product,
                        &generated_runtime::SyntaxGrammarDialect::from_options(&options)
                    ),
                    expected[column],
                    "recovered {source}, {definition}",
                );
                if expected[column] == P {
                    let error = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                        error_index: 47,
                        tokens: vec1::Vec1::new(words[0].clone()),
                    });
                    // Local synthetic counterexamples, not winning recovery
                    // fixtures: proven additive opener evidence cannot hide
                    // a missing or uncertain required first operand.
                    for leading in [
                        recovered::Recovered::error(error.clone()),
                        recovered::Recovered::prefix(
                            vec![error],
                            parsed_value(&recovered_product.leading_selbri)
                                .expect("complete parsed first operand")
                                .clone(),
                        ),
                    ] {
                        let mut uncertain = (*recovered_product).clone();
                        uncertain.leading_selbri = std::sync::Arc::new(leading);
                        assert_eq!(
                            recovered_standalone_presence(
                                &uncertain,
                                &generated_runtime::SyntaxGrammarDialect::from_options(&options)
                            ),
                            ZantufaTanruAtomPresence::Unproven,
                            "uncertain first operand: {source}, {definition}",
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complete_gek_product_checks_source_boundary_before_optional_gihi() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        // These are construction/boundary tests, not standalone eligibility:
        // the plain GA binary product is deliberately still disconnected.
        for (source, expected) in [
            ("ga broda gi brode", Some(4)),
            ("ga broda gi brode gi'i", Some(5)),
            ("ga broda gi brode mi", None),
            ("ga broda gi brode cu", None),
            ("ga broda gi brode gi mi", None),
            ("ga broda gi brode gi cu", None),
            ("ga broda gi brode gi'i mi", Some(5)),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty product").span.end;
            for mode in ["strict", "checkpoint", "recovered"] {
                let mut state = ParserState::new(&words, &options);
                let parser = match mode {
                    "strict" => generated_model::strict_zantufa_forethought_tanru_unit_parser(
                        generated_model::strict_generated_co_selbri_parser(),
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_zantufa_boundary_term_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    ).map(|_| ()).boxed(),
                    "checkpoint" => generated_model::recovery_checkpoint_strict_zantufa_forethought_tanru_unit_parser(
                        generated_model::recovery_checkpoint_strict_generated_co_selbri_parser(),
                        generated_model::recovery_checkpoint_strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::recovery_checkpoint_strict_generated_zantufa_boundary_term_parser(),
                        generated_model::strict_generated_co_selbri_parser(),
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_zantufa_boundary_term_parser(),
                        generated_model::recovery_checkpoint_strict_generated_free_modifier_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    ).map(|_| ()).boxed(),
                    "recovered" => recovered_forethought_tanru_unit_parser!()
                        .map(|_| ())
                        .boxed(),
                    _ => unreachable!("the test enumerates all three parser flavors"),
                };
                let result = with_consumed_extent(parser)
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result();
                assert_eq!(result.ok(), expected, "{source}, {mode}");
                assert_eq!(
                    state.warnings.len(),
                    usize::from(expected.is_some()),
                    "{source}, {mode}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_atom_end_guard_checks_optional_gi_then_term_or_cu() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        // Source :52 has optional GI here; source :24's bridi guard does not.
        // Success is zero-width, including when a trial GI was consumed.
        for (source, admitted) in [
            ("", true),
            ("broda", true),
            ("gi broda", true),
            ("mi", false),
            ("cu", false),
            ("gi mi", false),
            ("gi cu", false),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().map_or(0, |token| token.span.end);
            let mut state = ParserState::new(&words, &options);
            let parser = generated_model::strict_zantufa_boundary_atom_end_parser(
                generated_model::strict_generated_zantufa_boundary_term_parser(),
                generated_model::strict_generated_free_modifier_parser(),
            );
            let result = with_consumed_extent(parser.map(|_| ()).boxed())
                .parse_with_state(
                    spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                    &mut state,
                )
                .into_result();
            assert_eq!(result.ok(), admitted.then_some(0), "{source}");
            assert!(
                state.warnings.is_empty(),
                "guard cannot emit a warning: {source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn atom_reinterpretation_uses_the_raw_pair_without_legacy_dependencies() {
        for selbri in [false, true] {
            for reinterpretation in [false, true] {
                for terms in [false, true] {
                    for tags in [false, true] {
                        for connectives in [false, true] {
                            let definition = format!(
                                "({}ZANTUFA-SELBRI {}ZANTUFA-SELBRI-REINTERPRETATION {}ZANTUFA-TERMS {}ZANTUFA-TAGS {}ZANTUFA-CONNECTIVES)",
                                if selbri { "+" } else { "-" },
                                if reinterpretation { "+" } else { "-" },
                                if terms { "+" } else { "-" },
                                if tags { "+" } else { "-" },
                                if connectives { "+" } else { "-" },
                            );
                            let dialect = parse_dialect_definition(&definition)
                                .expect("explicit feature axis");
                            let options = ParseOptions::default().with_dialect_definition(&dialect);
                            let flags =
                                generated_runtime::SyntaxGrammarDialect::from_options(&options);
                            assert_eq!(flags.zantufa_selbri_enabled, selbri);
                            assert_eq!(
                                flags.zantufa_selbri_atom_reinterpretation_enabled,
                                selbri && reinterpretation
                            );
                            assert_eq!(
                                flags.zantufa_selbri_reinterpretation_enabled,
                                terms && reinterpretation
                            );
                            assert_eq!(flags.zantufa_tags_enabled, tags);
                            assert_eq!(flags.zantufa_connectives_enabled, connectives);
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn atom_presence_combination_is_an_uncertainty_absorbing_join() {
        use ZantufaTanruAtomPresence::{Absent, Present, Unproven};
        let states = [Absent, Present, Unproven];
        for first in states {
            assert_eq!(first.combine(first), first);
            assert_eq!(first.combine(Absent), first);
            assert_eq!(first.combine(Unproven), Unproven);
            for second in states {
                assert_eq!(first.combine(second), second.combine(first));
                for third in states {
                    assert_eq!(
                        first.combine(second).combine(third),
                        first.combine(second.combine(third))
                    );
                }
            }
        }
        // A discovered extension cannot turn an incomplete sibling into proof.
        assert_eq!(Present.combine(Absent.combine(Unproven)), Unproven);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ga_opener_mapping_preserves_all_token_recovery_states() {
        // This tests the conversion boundary, not the existence of any winning
        // parser recovery row. In particular a Prefix is not converted to Valid.
        let words = segment_words_with_modifiers("mi se ga").expect("valid morphology");
        let words = syntax_tokens(&words, &ParseOptions::default());
        let skipped = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
            error_index: 3,
            tokens: vec1::Vec1::new(words[0].clone()),
        });
        let missing = new!(crate::tree::SyntaxRecoveryItem::MissingRequiredField {
            error_index: 7,
            span: std::sync::Arc::new(
                jbotci_diagnostics::source_span_from_byte_offsets(None, "mi se ga", 8, 8,)
                    .expect("empty source span")
            ),
            expected: "GA".to_owned(),
        });
        let se_states = [
            recovered::Recovered::valid(words[1].clone()),
            recovered::Recovered::prefix(vec![skipped.clone()], words[1].clone()),
            recovered::Recovered::error(skipped.clone()),
            recovered::Recovered::error(missing.clone()),
        ];
        let head_states = [
            recovered::Recovered::valid(words[2].clone()),
            recovered::Recovered::prefix(vec![skipped.clone(), missing.clone()], words[2].clone()),
            recovered::Recovered::error(skipped),
            recovered::Recovered::error(missing),
        ];
        for head in head_states {
            let head = recovered::WithFreeModifiers {
                value: head,
                free_modifiers: Vec::new(),
            };
            let absent = recovered::ZantufaAtomGaOpenerSyntax::from(head.clone());
            assert!(absent.se.is_none());
            assert_eq!(absent.head, store_recovered_clause(head.clone()));
            for se in &se_states {
                let se = recovered::WithFreeModifiers {
                    value: se.clone(),
                    free_modifiers: Vec::new(),
                };
                let present =
                    recovered::ZantufaAtomGaOpenerSyntax::from((se.clone(), head.clone()));
                assert_eq!(present.se, Some(store_recovered_clause(se)));
                assert_eq!(present.head, store_recovered_clause(head.clone()));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warned_final_gi_preserves_whole_products_and_recovered_payload_states() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let mut synthetic_cases = 0;
        for source in [
            "ga'o to mi klama toi na to do klama toi se to mi klama toi joi to do klama toi ke'i to mi klama toi gi to do klama toi",
            "na to mi klama toi se to do klama toi joi to mi klama toi ke'i to do klama toi gi to mi klama toi",
            "se to mi klama toi joi to do klama toi ke'i to mi klama toi gi to do klama toi",
            "joi to mi klama toi ke'i to do klama toi gi to mi klama toi",
            "fi'o brodi fe'u gi to mi klama toi",
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty opener").span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            let mut strict_outputs = Vec::new();
            let mut recovered_outputs = Vec::new();
            for warned in [false, true] {
                let mut state = ParserState::new(&words, &options);
                let parser = if warned {
                    generated_model::strict_warned_zantufa_atom_final_gi_opener_parser(
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                } else {
                    generated_model::strict_zantufa_atom_final_gi_opener_parser(
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                };
                strict_outputs.push(
                    parser
                        .parse_with_state(input, &mut state)
                        .into_result()
                        .expect("strict final GI")
                        .into_owned(),
                );
                assert_eq!(state.warnings.len(), usize::from(warned), "{source}");
                let mut state = ParserState::new(&words, &options);
                let parser = if warned {
                    generated_model::recovered_warned_zantufa_atom_final_gi_opener_parser(
                        generated_model::recovered_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::recovered_generated_free_modifier_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|value| value.into_owned())
                    .boxed()
                } else {
                    generated_model::recovered_zantufa_atom_final_gi_opener_parser(
                        generated_model::recovered_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::recovered_generated_free_modifier_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|value| value.into_owned())
                    .boxed()
                };
                recovered_outputs.push(
                    parser
                        .parse_with_state(input, &mut state)
                        .into_result()
                        .expect("recovered final GI"),
                );
                assert_eq!(state.warnings.len(), usize::from(warned), "{source}");
            }
            assert_eq!(strict_outputs[0], strict_outputs[1], "{source}");
            assert_eq!(recovered_outputs[0], recovered_outputs[1], "{source}");
            assert_eq!(strict_outputs[0].gi.free_modifiers.len(), 1);
            assert_eq!(recovered_outputs[0].gi.free_modifiers.len(), 1);
            let original = &recovered_outputs[0];
            let skipped = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
                error_index: 3,
                tokens: vec1::Vec1::new(words[0].clone()),
            });
            let missing = new!(crate::tree::SyntaxRecoveryItem::MissingRequiredField {
                error_index: 7,
                span: std::sync::Arc::new(
                    jbotci_diagnostics::source_span_from_byte_offsets(
                        None,
                        source,
                        source.len(),
                        source.len()
                    )
                    .expect("empty end span")
                ),
                expected: "payload".to_owned(),
            });
            // Synthetic conversion-boundary coverage, not winning recovery
            // fixtures. Compare complete error records, their order, and frees.
            macro_rules! check_payload {
                ($payload:expr, $variant:path) => {{
                    let payload = parsed_value($payload).expect("parsed payload");
                    for state in [
                        recovered::Recovered::valid(payload.clone()),
                        recovered::Recovered::prefix(
                            vec![skipped.clone(), missing.clone()],
                            payload.clone(),
                        ),
                        recovered::Recovered::error(skipped.clone()),
                        recovered::Recovered::error(missing.clone()),
                    ] {
                        for gi_state in [
                            original.gi.value.clone(),
                            recovered::Recovered::prefix(
                                vec![missing.clone(), skipped.clone()],
                                parsed_value(&original.gi.value).expect("parsed GI").clone(),
                            ),
                            recovered::Recovered::error(skipped.clone()),
                            recovered::Recovered::error(missing.clone()),
                        ] {
                            // `original` is a stored product, so its shared free
                            // modifiers unwrap back to the parser-side shape the
                            // conversion under test actually consumes.
                            let gi = RecoveredTokenClause {
                                value: gi_state,
                                free_modifiers: original
                                    .gi
                                    .free_modifiers
                                    .iter()
                                    .map(|modifier| (**modifier).clone())
                                    .collect(),
                            };
                            let mapped = recovered::ZantufaAtomFinalGiOpenerSyntax::from((
                                state.clone(),
                                gi.clone(),
                            ));
                            assert_eq!(mapped.gi, store_recovered_clause(gi));
                            assert_eq!(
                                *mapped.payload,
                                recovered::Recovered::valid($variant(Arc::new(state.clone())))
                            );
                            synthetic_cases += 1;
                        }
                    }
                }};
            }
            match parsed_value(&original.payload).expect("parsed enum") {
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik(payload) => {
                    check_payload!(
                        payload,
                        recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomJoik
                    );
                }
                recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag(payload) => {
                    check_payload!(
                        payload,
                        recovered::ZantufaAtomGekPayloadSyntax::ZantufaAtomTag
                    );
                }
            }
        }
        assert_eq!(synthetic_cases, 80);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn final_gi_contextual_anchors_survive_speculation_without_duplicate_warnings() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let mut attempts = 0;
        for (source, anchor) in [
            ("ga'o na se joi ke'i gi", 0),
            ("na se joi ke'i gi", 0),
            ("se joi ke'i gi", 0),
            ("joi ke'i gi", 0),
            ("fi'o brodi fe'u gi", 3),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty opener").span.end;
            for mode in 0..3 {
                for speculation in 0..3 {
                    let mut state = ParserState::new(&words, &options);
                    let parser = match mode {
                        0 => generated_model::strict_warned_zantufa_atom_final_gi_opener_parser(
                            generated_model::strict_generated_zantufa_tcita_selci_parser(),
                            generated_model::strict_generated_free_modifier_parser(),
                        ).map(|_| ()).boxed(),
                        1 => generated_model::recovery_checkpoint_strict_warned_zantufa_atom_final_gi_opener_parser(
                            generated_model::recovery_checkpoint_strict_generated_zantufa_tcita_selci_parser(),
                            generated_model::strict_generated_zantufa_tcita_selci_parser(),
                            generated_model::recovery_checkpoint_strict_generated_free_modifier_parser(),
                            generated_model::strict_generated_free_modifier_parser(),
                        ).map(|_| ()).boxed(),
                        _ => generated_model::recovered_warned_zantufa_atom_final_gi_opener_parser(
                            generated_model::recovered_generated_zantufa_tcita_selci_parser(),
                            generated_model::strict_generated_zantufa_tcita_selci_parser(),
                            generated_model::recovered_generated_free_modifier_parser(),
                            generated_model::strict_generated_free_modifier_parser(),
                        ).map(|_| ()).boxed(),
                    };
                    let parser = match speculation {
                        0 => parser,
                        1 => generated_runtime::lookahead(parser.clone())
                            .ignore_then(parser)
                            .boxed(),
                        _ => parser
                            .clone()
                            .then(tokens::cmavo(Cmavo::Gi))
                            .map(|_| ())
                            .or(parser)
                            .boxed(),
                    };
                    parser
                        .parse_with_state(
                            spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                            &mut state,
                        )
                        .into_result()
                        .expect("opener survives failed speculative branch");
                    // Raw ParserState warnings: finish/dedup must not mask a leak.
                    assert_eq!(state.warnings.len(), 1, "{source}, {mode}, {speculation}");
                    assert_eq!(
                        state.warnings[0].kind,
                        ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                    );
                    assert_eq!(state.warnings[0].anchor_index, anchor);
                    assert_eq!(
                        state.warnings[0].anchor,
                        Token::bare(words[anchor].core_word().clone())
                    );
                    attempts += 1;
                }
            }
        }
        assert_eq!(attempts, 45);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joik_split_mapping_preserves_each_internal_recovery_state() {
        let source = "ga'o na se joi ke'i";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let words = syntax_tokens(&words, &ParseOptions::default());
        let skipped = new!(crate::tree::SyntaxRecoveryItem::SkippedTokens {
            error_index: 11,
            tokens: vec1::Vec1::new(words[0].clone()),
        });
        let missing = new!(crate::tree::SyntaxRecoveryItem::MissingRequiredField {
            error_index: 23,
            span: std::sync::Arc::new(
                jbotci_diagnostics::source_span_from_byte_offsets(
                    None,
                    source,
                    source.len(),
                    source.len()
                )
                .expect("empty end span")
            ),
            expected: "JOIK field".to_owned(),
        });
        let clauses: Vec<_> = words
            .iter()
            .map(|word| RecoveredTokenClause {
                value: recovered::Recovered::valid(word.clone()),
                free_modifiers: Vec::new(),
            })
            .collect();
        let mut attempts = 0;
        // Four arms, with five/four/three/two remaining fields respectively.
        // This is synthetic mapping coverage, not a winning recovery claim.
        for first in 0..4 {
            for slot in first..5 {
                for value in [
                    recovered::Recovered::valid(words[slot].clone()),
                    recovered::Recovered::prefix(
                        vec![skipped.clone(), missing.clone()],
                        words[slot].clone(),
                    ),
                    recovered::Recovered::error(skipped.clone()),
                    recovered::Recovered::error(missing.clone()),
                ] {
                    let mut fields = clauses.clone();
                    fields[slot].value = value;
                    let expected = recovered::ZantufaAtomJoikSyntax {
                        left_gaho: (first == 0).then(|| store_recovered_clause(fields[0].clone())),
                        na: (first <= 1).then(|| store_recovered_clause(fields[1].clone())),
                        se: (first <= 2).then(|| store_recovered_clause(fields[2].clone())),
                        head: store_recovered_clause(fields[3].clone()),
                        right_gaho: Some(store_recovered_clause(fields[4].clone())),
                    };
                    let mapped = match first {
                        0 => recovered::ZantufaAtomJoikSyntax::from((
                            (
                                (
                                    (fields[0].clone(), Some(fields[1].clone())),
                                    Some(fields[2].clone()),
                                ),
                                fields[3].clone(),
                            ),
                            Some(fields[4].clone()),
                        )),
                        1 => recovered::ZantufaAtomJoikSyntax::from((
                            (
                                (fields[1].clone(), Some(fields[2].clone())),
                                fields[3].clone(),
                            ),
                            Some(fields[4].clone()),
                        )),
                        2 => recovered::ZantufaAtomJoikSyntax::from((
                            (fields[2].clone(), fields[3].clone()),
                            Some(fields[4].clone()),
                        )),
                        3 => recovered::ZantufaAtomJoikSyntax::from((
                            fields[3].clone(),
                            Some(fields[4].clone()),
                        )),
                        _ => unreachable!("four mapping arms"),
                    };
                    assert_eq!(mapped, expected, "arm {first}, field {slot}");
                    attempts += 1;
                }
            }
        }
        assert_eq!(attempts, 56);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warned_ga_opener_keeps_the_unwarned_product_with_real_free_modifiers() {
        // Compare whole typed products against the original optional-SE rule,
        // including free modifiers on both fields, rather than projecting them
        // away or using a public warning-only wrapper.
        for source in [
            "ga to mi klama toi",
            "se to mi klama toi gu'a to do klama toi",
        ] {
            let options = ParseOptions::default();
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty opener").span.end;
            let input = spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi));
            let mut strict_outputs = Vec::new();
            let mut recovered_outputs = Vec::new();
            for warned in [false, true] {
                let mut state = ParserState::new(&words, &options);
                let free = generated_model::strict_generated_free_modifier_parser();
                let parser = if warned {
                    generated_model::strict_warned_zantufa_atom_ga_opener_parser(free)
                } else {
                    generated_model::strict_zantufa_atom_ga_opener_parser(free)
                };
                strict_outputs.push(
                    parser
                        .parse_with_state(input, &mut state)
                        .into_result()
                        .expect("strict opener")
                        .into_owned(),
                );
                assert_eq!(state.warnings.len(), usize::from(warned));

                let mut state = ParserState::new(&words, &options);
                let free = generated_model::recovered_generated_free_modifier_parser();
                let parser = if warned {
                    generated_model::recovered_warned_zantufa_atom_ga_opener_parser(
                        free,
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|value| value.into_owned())
                    .boxed()
                } else {
                    generated_model::recovered_zantufa_atom_ga_opener_parser(
                        free,
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|value| value.into_owned())
                    .boxed()
                };
                recovered_outputs.push(
                    parser
                        .parse_with_state(input, &mut state)
                        .into_result()
                        .expect("recovered opener"),
                );
                assert_eq!(state.warnings.len(), usize::from(warned));
            }
            assert_eq!(strict_outputs[0], strict_outputs[1]);
            assert_eq!(recovered_outputs[0], recovered_outputs[1]);
            assert_eq!(strict_outputs[0].head.free_modifiers.len(), 1);
            assert_eq!(recovered_outputs[0].head.free_modifiers.len(), 1);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gi_opener_preserves_nested_component_warning_order_before_dedup() {
        // CLL 9.5 supplies FIhO selbri FEhU; the already accepted C-d
        // MEhOI atom supplies the independently warned selbri components.
        // No global warning sorting/deduplication is involved in this check.
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let outer = ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit;
        let inner = ExperimentalConstruct::ExperimentalMehOiSelbriUnit;
        for (source, expected) in [
            (
                "gi fi'o me'oi broda me'oi brode fe'u",
                [(outer, 0), (inner, 2), (inner, 3)],
            ),
            (
                "fi'o me'oi broda me'oi brode fe'u gi",
                [(inner, 1), (inner, 2), (outer, 4)],
            ),
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let words = syntax_tokens(&words, &options);
            let spanned = tokens::spanned_tokens(&words);
            let eoi = spanned.last().expect("nonempty opener").span.end;
            for recovered_mode in [false, true] {
                let mut state = ParserState::new(&words, &options);
                let parser = if recovered_mode {
                    recovered_atom_gek_parser!().map(|_| ()).boxed()
                } else {
                    generated_model::strict_zantufa_atom_gek_parser(
                        generated_model::strict_generated_zantufa_tcita_selci_parser(),
                        generated_model::strict_generated_free_modifier_parser(),
                    )
                    .map(|_| ())
                    .boxed()
                };
                parser
                    .parse_with_state(
                        spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                        &mut state,
                    )
                    .into_result()
                    .unwrap_or_else(|errors| {
                        panic!("{source}, recovered={recovered_mode}: {errors:?}")
                    });
                assert_eq!(
                    state.warnings.len(),
                    expected.len(),
                    "{source}, recovered={recovered_mode}"
                );
                for (warning, (kind, anchor_index)) in state.warnings.iter().zip(expected) {
                    assert_eq!(warning.kind, kind, "{source}, recovered={recovered_mode}");
                    assert_eq!(
                        warning.anchor_index, anchor_index,
                        "{source}, recovered={recovered_mode}"
                    );
                    assert_eq!(
                        warning.anchor,
                        Token::bare(words[anchor_index].core_word().clone())
                    );
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gi_opener_anchors_use_the_real_recursive_payload_parsers() {
        let dialect = parse_dialect_definition("(+ZANTUFA-SELBRI)").expect("minimal feature");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        // Each payload belongs to the reviewed opener audit. JOIK anchors its
        // first field; only a tag payload retains the final-GI fallback.
        for payload in [
            "joi",
            "je",
            "bi'i",
            "na joi",
            "ga'o joi",
            "joi ke'i",
            "pu",
            "fi'o brodi fe'u",
        ] {
            for gi_first in [true, false] {
                for bo in [false, true] {
                    let mut source = if gi_first {
                        format!("gi {payload}")
                    } else {
                        format!("{payload} gi")
                    };
                    if bo {
                        source.push_str(" bo");
                    }
                    let words = segment_words_with_modifiers(&source).expect("valid morphology");
                    let words = syntax_tokens(&words, &options);
                    let gi_index = words
                        .iter()
                        .position(|token| token.is_cmavo(Cmavo::Gi))
                        .expect("GI exists");
                    let spanned = tokens::spanned_tokens(&words);
                    let eoi = spanned.last().expect("nonempty opener").span.end;
                    for recovered_mode in [false, true] {
                        let mut state = ParserState::new(&words, &options);
                        let parser = if recovered_mode {
                            recovered_atom_gek_parser!().map(|_| ()).boxed()
                        } else {
                            generated_model::strict_zantufa_atom_gek_parser(
                                generated_model::strict_generated_zantufa_tcita_selci_parser(),
                                generated_model::strict_generated_free_modifier_parser(),
                            )
                            .map(|_| ())
                            .boxed()
                        };
                        parser
                            .parse_with_state(
                                spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                                &mut state,
                            )
                            .into_result()
                            .unwrap_or_else(|errors| {
                                panic!("{source}, recovered={recovered_mode}: {errors:?}")
                            });
                        assert_eq!(
                            state.warnings.len(),
                            1,
                            "{source}, recovered={recovered_mode}"
                        );
                        assert_eq!(
                            state.warnings[0].kind,
                            ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                        );
                        let expected_anchor =
                            if gi_first || matches!(payload, "pu" | "fi'o brodi fe'u") {
                                gi_index
                            } else {
                                0
                            };
                        assert_eq!(
                            state.warnings[0].anchor_index, expected_anchor,
                            "{source}, recovered={recovered_mode}"
                        );
                        assert_eq!(
                            state.warnings[0].anchor,
                            Token::bare(words[expected_anchor].core_word().clone())
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ga_opener_anchors_and_speculative_warning_rollback() {
        // These deliberately isolate the token-only arms. Nested free-modifier
        // and complete-owner rejection tests are separate obligations: an empty
        // free parser here is not evidence about either of those paths.
        for (source, has_se, head) in [
            ("ga", false, Cmavo::Ga),
            ("gu'a", false, Cmavo::Guha),
            ("se ga", true, Cmavo::Ga),
            ("se gu'a", true, Cmavo::Guha),
        ] {
            for recovery_mode in 0..3 {
                for speculative_mode in 0..3 {
                    let options = ParseOptions::default();
                    let words = segment_words_with_modifiers(source).expect("valid morphology");
                    let words = syntax_tokens(&words, &options);
                    let spanned = tokens::spanned_tokens(&words);
                    let eoi = spanned.last().expect("nonempty opener").span.end;
                    let mut state = ParserState::new(&words, &options);
                    let parser = match recovery_mode {
                        0 | 1 => {
                            let free = generated_runtime::strict_empty_free_modifier_parser();
                            let parser = if recovery_mode == 0 {
                                generated_model::strict_warned_zantufa_atom_ga_opener_parser(free)
                            } else {
                                generated_model::recovery_checkpoint_strict_warned_zantufa_atom_ga_opener_parser(
                                    free,
                                    generated_runtime::strict_empty_free_modifier_parser(),
                                )
                            };
                            parser.map(move |value| {
                                let value = value.into_owned();
                                assert_eq!(value.se.is_some(), has_se);
                                if let Some(se) = value.se {
                                    assert!(se.value.is_cmavo(Cmavo::Se));
                                    assert!(se.free_modifiers.is_empty());
                                }
                                assert!(value.head.value.is_cmavo(head));
                                assert!(value.head.free_modifiers.is_empty());
                            }).boxed()
                        }
                        _ => generated_model::recovered_warned_zantufa_atom_ga_opener_parser(
                            generated_runtime::recovered_empty_free_modifier_parser(),
                            generated_runtime::strict_empty_free_modifier_parser(),
                        ).map(move |value| {
                            let value = value.into_owned();
                            assert_eq!(value.se.is_some(), has_se);
                            if let Some(se) = value.se {
                                assert!(matches!(se.value, recovered::Recovered::Valid(token) if token.is_cmavo(Cmavo::Se)));
                                assert!(se.free_modifiers.is_empty());
                            }
                            assert!(matches!(value.head.value, recovered::Recovered::Valid(token) if token.is_cmavo(head)));
                            assert!(value.head.free_modifiers.is_empty());
                        }).boxed(),
                    };
                    let parser = match speculative_mode {
                        0 => parser,
                        1 => generated_runtime::lookahead(parser.clone())
                            .ignore_then(parser)
                            .boxed(),
                        _ => parser
                            .clone()
                            .then(tokens::cmavo(Cmavo::Gi))
                            .map(|_| ())
                            .or(parser)
                            .boxed(),
                    };
                    parser
                        .parse_with_state(
                            spanned.as_slice().split_spanned(SimpleSpan::from(eoi..eoi)),
                            &mut state,
                        )
                        .into_result()
                        .expect("complete opener survives speculation");
                    // Do not call finish(): it deduplicates warnings and could
                    // hide a leak from lookahead or the later-failing first arm.
                    assert_eq!(
                        state.warnings.len(),
                        1,
                        "{source}: recovery={recovery_mode}, speculation={speculative_mode}"
                    );
                    let warning = &state.warnings[0];
                    assert_eq!(
                        warning.kind,
                        ExperimentalConstruct::ExperimentalZantufaForethoughtTanruUnit
                    );
                    assert_eq!(warning.anchor_index, 0);
                    assert!(
                        warning
                            .anchor
                            .is_cmavo(if has_se { Cmavo::Se } else { head })
                    );
                }
            }
        }
    }
}
