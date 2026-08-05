//! Minimum smusni-v0 typed IR foundation for lexical dynamic edges.
//!
//! This module intentionally stops before host selection. It discovers the
//! graph-owned computation positioned at an original lexical place, resolves
//! the exact curated `(relation, place)` key, and constructs an edge
//! only after successful validation. Raw S-expression datums never enter this
//! layer.

#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires, try_new};

use super::registry::{
    GENERATED_LEXICAL_POLICY_ENTITY_FALLBACK_REASON_ID,
    GENERATED_LEXICAL_POLICY_EVENTUALITY_FALLBACK_REASON_ID, GENERATED_LEXICAL_POLICY_ROWS,
};
#[allow(unused_imports)]
use crate::model::{
    ArgumentValue, DescriptorKind, PlaceIndex, PredicationRelation, PredicationRelationData,
    ReferentCategory, SemanticGraph, SemanticObject, SemanticObjectId, SemanticObjectKind,
    SemanticSort,
};

#[invariant(!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_lowercase()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NormalizedLexicalRelationId {
    text: String,
}

impl NormalizedLexicalRelationId {
    #[requires(true)]
    #[ensures(ret.is_ok() == (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_lowercase())))]
    pub(crate) fn from_canonical(text: &str) -> Result<Self, RelationIdentityError> {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_lowercase()) {
            return Err(RelationIdentityError::Noncanonical);
        }
        Ok(new!(NormalizedLexicalRelationId {
            text: text.to_owned(),
        }))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationIdentityError {
    Noncanonical,
}

#[invariant(*value > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OriginalPlace {
    value: usize,
}

impl OriginalPlace {
    #[requires(value > 0)]
    #[ensures(ret.get() == value)]
    pub(crate) fn new(value: usize) -> Self {
        new!(OriginalPlace { value })
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub(crate) fn get(self) -> usize {
        self.value
    }
}

#[invariant(*value > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AttestedArity {
    value: usize,
}

impl AttestedArity {
    #[requires(value > 0)]
    #[ensures(ret.get() == value)]
    pub(crate) fn new(value: usize) -> Self {
        new!(AttestedArity { value })
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub(crate) fn get(self) -> usize {
        self.value
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DynamicValueFamily {
    RefCompReferentsEntity,
    RefCompReferentsEventuality,
}

impl DynamicValueFamily {
    #[requires(true)]
    #[ensures(ret.starts_with("smusni.fallback.lexical-policy."))]
    fn fallback_reason_id(self) -> &'static str {
        match self {
            Self::RefCompReferentsEntity => GENERATED_LEXICAL_POLICY_ENTITY_FALLBACK_REASON_ID,
            Self::RefCompReferentsEventuality => {
                GENERATED_LEXICAL_POLICY_EVENTUALITY_FALLBACK_REASON_ID
            }
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopePolicy {
    Extensional,
    Intensional,
    Opaque,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LexicalPolicyKey {
    relation: NormalizedLexicalRelationId,
    original_place: OriginalPlace,
}

impl LexicalPolicyKey {
    #[requires(true)]
    #[ensures(ret.relation == old(relation.clone()) && ret.original_place == original_place)]
    pub(crate) fn new(
        relation: NormalizedLexicalRelationId,
        original_place: OriginalPlace,
    ) -> Self {
        // Every combination of these validated components is a valid lookup key.
        LexicalPolicyKey {
            relation,
            original_place,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn relation(&self) -> &str {
        self.relation.as_str()
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub(crate) fn original_place(&self) -> usize {
        self.original_place.get()
    }
}

#[invariant(key.original_place.get() <= attested_arity.get())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedLexicalPolicy {
    key: LexicalPolicyKey,
    attested_arity: AttestedArity,
    accepted_family: DynamicValueFamily,
    policy: ScopePolicy,
}

impl VerifiedLexicalPolicy {
    #[requires(key.original_place.get() <= attested_arity.get())]
    #[ensures(ret.key == old(key.clone()) && ret.attested_arity == attested_arity && ret.accepted_family == accepted_family && ret.policy == policy)]
    fn new(
        key: LexicalPolicyKey,
        attested_arity: AttestedArity,
        accepted_family: DynamicValueFamily,
        policy: ScopePolicy,
    ) -> Self {
        new!(VerifiedLexicalPolicy {
            key,
            attested_arity,
            accepted_family,
            policy,
        })
    }

    #[requires(true)]
    #[ensures(ret == self.policy)]
    pub(crate) fn policy(&self) -> ScopePolicy {
        self.policy
    }
}

#[invariant(matches!(object.object_kind(), SemanticObjectKind::Utterance | SemanticObjectKind::Sequence | SemanticObjectKind::Formula | SemanticObjectKind::Question))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphRefHostId {
    object: SemanticObjectId,
}

impl GraphRefHostId {
    #[requires(true)]
    #[ensures(ret.is_ok() == matches!(object.object_kind(), SemanticObjectKind::Utterance | SemanticObjectKind::Sequence | SemanticObjectKind::Formula | SemanticObjectKind::Question))]
    pub(crate) fn try_new(object: SemanticObjectId) -> Result<Self, GraphRefHostError> {
        if !matches!(
            object.object_kind(),
            SemanticObjectKind::Utterance
                | SemanticObjectKind::Sequence
                | SemanticObjectKind::Formula
                | SemanticObjectKind::Question
        ) {
            return Err(GraphRefHostError::UnsupportedObjectKind);
        }
        Ok(new!(GraphRefHostId { object }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphRefHostError {
    UnsupportedObjectKind,
}

#[invariant(*policy != ScopePolicy::Opaque || de_re_owner.is_none())]
#[invariant(key.original_place.get() <= attested_arity.get())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexicalDynamicEdge {
    key: LexicalPolicyKey,
    family: DynamicValueFamily,
    attested_arity: AttestedArity,
    policy: ScopePolicy,
    de_re_owner: Option<GraphRefHostId>,
}

impl LexicalDynamicEdge {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|edge| edge.key == old(verified.key.clone()) && edge.family == old(verified.accepted_family) && edge.policy == old(verified.policy)) || ret.is_err())]
    fn from_verified(
        verified: VerifiedLexicalPolicy,
        de_re_owner: Option<GraphRefHostId>,
    ) -> Result<Self, LexicalEdgeOwnerFailure> {
        if verified.policy == ScopePolicy::Opaque && de_re_owner.is_some() {
            return Err(LexicalEdgeOwnerFailure::OpaqueCannotEscape);
        }
        let verified = verified.into_data();
        Ok(new!(LexicalDynamicEdge {
            key: verified.key,
            family: verified.accepted_family,
            attested_arity: verified.attested_arity,
            policy: verified.policy,
            de_re_owner,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexicalEdgeOwnerFailure {
    OpaqueCannotEscape,
}

#[invariant(::UnknownRelation { .. } => true)]
#[invariant(::UnsupportedPlace { .. } => true)]
#[invariant(::DynamicFamilyMismatch { observed, expected, .. } => observed != expected)]
#[invariant(::StaleArity { observed, expected, .. } => observed != expected)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LexicalPolicyLookupFailure {
    UnknownRelation {
        key: LexicalPolicyKey,
    },
    UnsupportedPlace {
        key: LexicalPolicyKey,
    },
    DynamicFamilyMismatch {
        key: LexicalPolicyKey,
        observed: DynamicValueFamily,
        expected: DynamicValueFamily,
    },
    StaleArity {
        key: LexicalPolicyKey,
        observed: AttestedArity,
        expected: AttestedArity,
    },
}

#[invariant(::Lookup(_) => true)]
#[invariant(::IncompatibleOwner { failure, .. } => *failure == LexicalEdgeOwnerFailure::OpaqueCannotEscape)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LexicalEdgeFallbackReason {
    Lookup(LexicalPolicyLookupFailure),
    IncompatibleOwner {
        key: LexicalPolicyKey,
        failure: LexicalEdgeOwnerFailure,
    },
}

#[invariant(::Predication { predication } => predication.object_kind() == SemanticObjectKind::Predication)]
#[invariant(::Adjunct { predication, .. } => predication.object_kind() == SemanticObjectKind::Predication)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LexicalEdgeSite {
    Predication {
        predication: SemanticObjectId,
    },
    Adjunct {
        predication: SemanticObjectId,
        adjunct_index: usize,
    },
}

#[invariant(value.object_kind() == SemanticObjectKind::Referent)]
#[invariant(key.original_place.get() <= observed_arity.get())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexicalEdgeCandidate {
    site: LexicalEdgeSite,
    value: SemanticObjectId,
    key: LexicalPolicyKey,
    family: DynamicValueFamily,
    observed_arity: AttestedArity,
}

impl LexicalEdgeCandidate {
    #[requires(value.object_kind() == SemanticObjectKind::Referent)]
    #[requires(key.original_place.get() <= observed_arity.get())]
    #[ensures(ret.site == site && ret.value == value && ret.key == old(key.clone()) && ret.family == family)]
    fn new(
        site: LexicalEdgeSite,
        value: SemanticObjectId,
        key: LexicalPolicyKey,
        family: DynamicValueFamily,
        observed_arity: AttestedArity,
    ) -> Self {
        new!(LexicalEdgeCandidate {
            site,
            value,
            key,
            family,
            observed_arity,
        })
    }
}

#[invariant(fallback_reason_key(reason) == key)]
#[invariant(!fallback_reason_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexicalPolicyDiagnostic {
    site: LexicalEdgeSite,
    key: LexicalPolicyKey,
    reason: LexicalEdgeFallbackReason,
    fallback_reason_id: &'static str,
}

#[invariant(::Constructed { .. } => true)]
#[invariant(::Fallback { reason, diagnostic } => reason == &diagnostic.reason)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LexicalEdgeAttempt {
    Constructed {
        edge: LexicalDynamicEdge,
    },
    Fallback {
        reason: LexicalEdgeFallbackReason,
        diagnostic: LexicalPolicyDiagnostic,
    },
}

#[invariant(rows_have_unique_keys(rows))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LexicalPolicyRegistry {
    rows: Vec<VerifiedLexicalPolicy>,
}

impl LexicalPolicyRegistry {
    #[requires(rows_have_unique_keys(&rows))]
    #[ensures(ret.rows.len() == old(rows.len()))]
    fn new(rows: Vec<VerifiedLexicalPolicy>) -> Self {
        new!(LexicalPolicyRegistry { rows })
    }

    #[requires(true)]
    #[ensures(ret.rows.len() == GENERATED_LEXICAL_POLICY_ROWS.len())]
    fn generated() -> Self {
        let rows = GENERATED_LEXICAL_POLICY_ROWS
            .iter()
            .map(|row| {
                let relation = NormalizedLexicalRelationId::from_canonical(row.relation)
                    .expect("generated relations were audited as canonical");
                let key = LexicalPolicyKey::new(relation, OriginalPlace::new(row.original_place));
                VerifiedLexicalPolicy::new(
                    key,
                    AttestedArity::new(row.attested_arity),
                    row.accepted_family,
                    row.policy,
                )
            })
            .collect();
        Self::new(rows)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|policy| policy.key == *key) || ret.is_err())]
    fn lookup(
        &self,
        key: &LexicalPolicyKey,
        observed_arity: AttestedArity,
        observed_family: DynamicValueFamily,
    ) -> Result<VerifiedLexicalPolicy, LexicalPolicyLookupFailure> {
        let Some(row) = self.rows.iter().find(|row| row.key == *key) else {
            let failure = if !self.rows.iter().any(|row| row.key.relation == key.relation) {
                new!(LexicalPolicyLookupFailure::UnknownRelation { key: key.clone() })
            } else {
                new!(LexicalPolicyLookupFailure::UnsupportedPlace { key: key.clone() })
            };
            return Err(failure);
        };
        if row.attested_arity != observed_arity {
            return Err(new!(LexicalPolicyLookupFailure::StaleArity {
                key: key.clone(),
                observed: observed_arity,
                expected: row.attested_arity,
            }));
        }
        if row.accepted_family != observed_family {
            return Err(new!(LexicalPolicyLookupFailure::DynamicFamilyMismatch {
                key: key.clone(),
                observed: observed_family,
                expected: row.accepted_family,
            }));
        }
        Ok(row.clone())
    }
}

#[requires(true)]
#[ensures(true)]
fn rows_have_unique_keys(rows: &[VerifiedLexicalPolicy]) -> bool {
    let mut seen = BTreeSet::new();
    rows.iter().all(|row| seen.insert(row.key.clone()))
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(LexicalEdgeAttempt::Constructed { .. }))
    || matches!(ret.as_data(), data!(LexicalEdgeAttempt::Fallback { .. })))]
fn attempt_lexical_dynamic_edge(
    registry: &LexicalPolicyRegistry,
    candidate: &LexicalEdgeCandidate,
    de_re_owner: Option<GraphRefHostId>,
) -> LexicalEdgeAttempt {
    let verified = match registry.lookup(&candidate.key, candidate.observed_arity, candidate.family)
    {
        Ok(verified) => verified,
        Err(failure) => {
            let reason = new!(LexicalEdgeFallbackReason::Lookup(failure));
            return fallback_attempt(candidate, reason);
        }
    };
    match LexicalDynamicEdge::from_verified(verified, de_re_owner) {
        Ok(edge) => new!(LexicalEdgeAttempt::Constructed { edge }),
        Err(failure) => {
            let reason = new!(LexicalEdgeFallbackReason::IncompatibleOwner {
                key: candidate.key.clone(),
                failure,
            });
            fallback_attempt(candidate, reason)
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(LexicalEdgeAttempt::Fallback { .. })))]
fn fallback_attempt(
    candidate: &LexicalEdgeCandidate,
    reason: LexicalEdgeFallbackReason,
) -> LexicalEdgeAttempt {
    let diagnostic = new!(LexicalPolicyDiagnostic {
        site: candidate.site,
        key: candidate.key.clone(),
        reason: reason.clone(),
        fallback_reason_id: candidate.family.fallback_reason_id(),
    });
    new!(LexicalEdgeAttempt::Fallback { reason, diagnostic })
}

#[requires(true)]
#[ensures(true)]
fn fallback_reason_key(reason: &LexicalEdgeFallbackReason) -> &LexicalPolicyKey {
    match reason.as_data() {
        data!(LexicalEdgeFallbackReason::Lookup(failure)) => lookup_failure_key(failure),
        data!(LexicalEdgeFallbackReason::IncompatibleOwner { key, .. }) => key,
    }
}

#[requires(true)]
#[ensures(true)]
fn lookup_failure_key(failure: &LexicalPolicyLookupFailure) -> &LexicalPolicyKey {
    match failure.as_data() {
        data!(LexicalPolicyLookupFailure::UnknownRelation { key })
        | data!(LexicalPolicyLookupFailure::UnsupportedPlace { key })
        | data!(LexicalPolicyLookupFailure::DynamicFamilyMismatch { key, .. })
        | data!(LexicalPolicyLookupFailure::StaleArity { key, .. }) => key,
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreHostLexicalIr {
    owning_edges: Vec<LexicalEdgeCandidate>,
}

impl PreHostLexicalIr {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(true)]
    pub(crate) fn collect(graph: &SemanticGraph) -> Self {
        let dynamic_values: BTreeMap<SemanticObjectId, DynamicValueFamily> = graph
            .objects
            .iter()
            .filter_map(|(id, object)| dynamic_value_family(object).map(|family| (*id, family)))
            .collect();
        let definition_subgraphs: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> =
            dynamic_values
                .keys()
                .map(|id| (*id, definition_subgraph(graph, *id)))
                .collect();
        let mut owning_edges = Vec::new();
        for (predication_id, object) in &graph.objects {
            let Some(predication) = object.as_predication() else {
                continue;
            };
            let data!(PredicationRelation::Named { relation }) = predication.relation.as_data()
            else {
                continue;
            };
            collect_argument_edges(
                *predication_id,
                new!(LexicalEdgeSite::Predication {
                    predication: *predication_id,
                }),
                relation,
                &predication.arguments,
                &dynamic_values,
                &definition_subgraphs,
                &mut owning_edges,
            );
            for (adjunct_index, adjunct) in predication.adjuncts.iter().enumerate() {
                let Some(relation) = adjunct.relation.as_deref() else {
                    continue;
                };
                collect_argument_edges(
                    *predication_id,
                    new!(LexicalEdgeSite::Adjunct {
                        predication: *predication_id,
                        adjunct_index,
                    }),
                    relation,
                    &adjunct.arguments,
                    &dynamic_values,
                    &definition_subgraphs,
                    &mut owning_edges,
                );
            }
        }
        PreHostLexicalIr { owning_edges }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn owning_edges(&self) -> &[LexicalEdgeCandidate] {
        &self.owning_edges
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(predication_id.object_kind() == SemanticObjectKind::Predication)]
#[ensures(out.len() >= old(out.len()))]
fn collect_argument_edges(
    predication_id: SemanticObjectId,
    site: LexicalEdgeSite,
    relation: &str,
    arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
    dynamic_values: &BTreeMap<SemanticObjectId, DynamicValueFamily>,
    definition_subgraphs: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    out: &mut Vec<LexicalEdgeCandidate>,
) {
    let Ok(relation) = NormalizedLexicalRelationId::from_canonical(relation) else {
        return;
    };
    let Some(observed_arity) = contiguous_argument_arity(arguments) else {
        return;
    };
    for (place, argument) in arguments {
        let Some(value) = argument.value else {
            continue;
        };
        let Some(family) = dynamic_values.get(&value).copied() else {
            continue;
        };
        if definition_subgraphs
            .get(&value)
            .is_some_and(|subgraph| subgraph.contains(&predication_id))
        {
            continue;
        }
        let key = LexicalPolicyKey::new(relation.clone(), OriginalPlace::new(place.get()));
        out.push(LexicalEdgeCandidate::new(
            site,
            value,
            key,
            family,
            observed_arity,
        ));
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|arity| arity.get() == arguments.len()))]
fn contiguous_argument_arity(
    arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
) -> Option<AttestedArity> {
    let maximum = arguments.keys().next_back()?.get();
    ((arguments.len() == maximum)
        && arguments
            .keys()
            .enumerate()
            .all(|(index, place)| place.get() == index + 1))
    .then(|| AttestedArity::new(maximum))
}

#[requires(true)]
#[ensures(true)]
fn dynamic_value_family(object: &SemanticObject) -> Option<DynamicValueFamily> {
    if let Some(referent) = object.as_referent() {
        let descriptor = referent.descriptor.as_ref()?;
        return (referent.category == ReferentCategory::Constant
            && referent.sort == SemanticSort::Entity
            && descriptor.kind != DescriptorKind::Elided)
            .then_some(DynamicValueFamily::RefCompReferentsEntity);
    }
    let eventuality = object.as_eventuality()?;
    (eventuality.denotation.category() == Some(ReferentCategory::Constant)
        && (eventuality.content.is_some()
            || eventuality.body.is_some()
            || eventuality
                .descriptor
                .as_ref()
                .is_some_and(|descriptor| descriptor.kind != DescriptorKind::Elided)))
    .then_some(DynamicValueFamily::RefCompReferentsEventuality)
}

#[requires(dynamic_value.object_kind() == SemanticObjectKind::Referent)]
#[ensures(true)]
fn definition_subgraph(
    graph: &SemanticGraph,
    dynamic_value: SemanticObjectId,
) -> BTreeSet<SemanticObjectId> {
    let Some(object) = graph.objects.get(&dynamic_value) else {
        return BTreeSet::new();
    };
    let mut pending = definition_roots(object);
    let mut visited = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        if let Some(object) = graph.objects.get(&id) {
            object.references_into(&mut pending);
        }
    }
    visited
}

#[requires(true)]
#[ensures(true)]
fn definition_roots(object: &SemanticObject) -> Vec<SemanticObjectId> {
    // A description's body and relative clauses are property definitions. Any
    // occurrence of the described object reached from these roots is the
    // definition-bound value (`ke'a` in witness 12, and the already-bound
    // speaker-description base in witness 11), never a fresh `RefComp` edge.
    let mut roots = Vec::new();
    if let Some(descriptor) = object.descriptor() {
        roots.extend(descriptor.body);
        roots.extend(descriptor.relative_clauses.iter().map(|clause| clause.body));
    }
    if let Some(referent) = object.as_referent() {
        roots.extend(referent.relative_clauses.iter().map(|clause| clause.body));
        roots.extend(referent.body);
    }
    if let Some(eventuality) = object.as_eventuality() {
        roots.extend(
            eventuality
                .relative_clauses
                .iter()
                .map(|clause| clause.body),
        );
        roots.extend(eventuality.content);
        roots.extend(eventuality.body);
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    use jbotci_dialect::DialectDefinition;
    use jbotci_morphology::{
        MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
    };
    use jbotci_source::SourceId;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

    use crate::{SemanticBuildOptions, build_generated_semantic_graph_with_dictionary_and_options};

    const WITNESSES: &str = include_str!("../../data/smusni-v0/sources/must-compact-witnesses.txt");

    #[requires(!text.is_empty())]
    #[ensures(ret.objects.contains_key(&ret.root))]
    fn graph_for(text: &str) -> SemanticGraph {
        let dialect = DialectDefinition::default();
        let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
        let source_id = Some(SourceId("<lexical-policy-test>".to_owned()));
        let words = segment_words_with_modifiers_with_options_and_source_id(
            text,
            &morphology_options,
            source_id,
        )
        .expect("mandatory witness morphology");
        let syntax = parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            text,
            &syntax_options,
        )
        .expect("mandatory witness syntax");
        build_generated_semantic_graph_with_dictionary_and_options(
            &syntax,
            SemanticBuildOptions {
                source_text: Some(text),
                story_time: false,
            },
            jbotci_dictionary_data::english(),
        )
        .expect("mandatory witness semantics")
    }

    #[requires(true)]
    #[ensures(ret.relation() == relation && ret.original_place() == place)]
    fn key(relation: &str, place: usize) -> LexicalPolicyKey {
        LexicalPolicyKey::new(
            NormalizedLexicalRelationId::from_canonical(relation).expect("test relation"),
            OriginalPlace::new(place),
        )
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_registry_has_exact_closed_policy_counts() {
        let registry = LexicalPolicyRegistry::generated();
        assert_eq!(registry.rows.len(), 8);
        assert_eq!(
            registry
                .rows
                .iter()
                .filter(|row| row.policy == ScopePolicy::Extensional)
                .count(),
            6
        );
        assert_eq!(
            registry
                .rows
                .iter()
                .filter(|row| row.policy == ScopePolicy::Intensional)
                .count(),
            2
        );
        assert!(
            registry
                .rows
                .iter()
                .all(|row| row.policy != ScopePolicy::Opaque)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lookup_is_exact_and_never_borrows_or_defaults() {
        let registry = LexicalPolicyRegistry::generated();
        let success = registry
            .lookup(
                &key("klama", 2),
                AttestedArity::new(5),
                DynamicValueFamily::RefCompReferentsEntity,
            )
            .expect("admitted final key");
        assert_eq!(success.policy(), ScopePolicy::Extensional);
        let capability = registry
            .lookup(
                &key("kakne", 2),
                AttestedArity::new(3),
                DynamicValueFamily::RefCompReferentsEventuality,
            )
            .expect("frozen capability-content key");
        assert_eq!(capability.policy(), ScopePolicy::Intensional);
        let unknown = registry
            .lookup(
                &key("nonce", 2),
                AttestedArity::new(5),
                DynamicValueFamily::RefCompReferentsEntity,
            )
            .expect_err("unknown relation");
        assert!(matches!(
            unknown.as_data(),
            data!(LexicalPolicyLookupFailure::UnknownRelation { .. })
        ));
        let unsupported_place = registry
            .lookup(
                &key("klama", 4),
                AttestedArity::new(5),
                DynamicValueFamily::RefCompReferentsEntity,
            )
            .expect_err("unsupported place");
        assert!(matches!(
            unsupported_place.as_data(),
            data!(LexicalPolicyLookupFailure::UnsupportedPlace { .. })
        ));
        let mismatched_family = registry
            .lookup(
                &key("klama", 2),
                AttestedArity::new(5),
                DynamicValueFamily::RefCompReferentsEventuality,
            )
            .expect_err("actual graph family must match the lexical slot type");
        assert!(matches!(
            mismatched_family.as_data(),
            data!(LexicalPolicyLookupFailure::DynamicFamilyMismatch { .. })
        ));
        let stale = registry
            .lookup(
                &key("klama", 2),
                AttestedArity::new(4),
                DynamicValueFamily::RefCompReferentsEntity,
            )
            .expect_err("stale arity");
        assert!(matches!(
            stale.as_data(),
            data!(LexicalPolicyLookupFailure::StaleArity { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn failure_constructs_no_edge_and_exactly_one_typed_diagnostic() {
        let candidate = LexicalEdgeCandidate::new(
            new!(LexicalEdgeSite::Predication {
                predication: SemanticObjectId::predication(1),
            }),
            SemanticObjectId::referent(2),
            key("skicu", 2),
            DynamicValueFamily::RefCompReferentsEntity,
            AttestedArity::new(4),
        );
        let attempt =
            attempt_lexical_dynamic_edge(&LexicalPolicyRegistry::generated(), &candidate, None);
        let data!(LexicalEdgeAttempt::Fallback { reason, diagnostic }) = attempt.as_data() else {
            panic!("unsupported lexical edge must not construct an edge");
        };
        assert_eq!(reason, &diagnostic.reason);
        assert_eq!(diagnostic.site, candidate.site);
        assert_eq!(diagnostic.key, candidate.key);
        assert_eq!(
            diagnostic.fallback_reason_id,
            "smusni.fallback.lexical-policy.entity"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn graph_owned_de_re_owner_is_checked_at_edge_construction() {
        let registry = LexicalPolicyRegistry::new(vec![VerifiedLexicalPolicy::new(
            key("djica", 2),
            AttestedArity::new(3),
            DynamicValueFamily::RefCompReferentsEventuality,
            ScopePolicy::Opaque,
        )]);
        let candidate = LexicalEdgeCandidate::new(
            new!(LexicalEdgeSite::Predication {
                predication: SemanticObjectId::predication(1),
            }),
            SemanticObjectId::referent(2),
            key("djica", 2),
            DynamicValueFamily::RefCompReferentsEventuality,
            AttestedArity::new(3),
        );
        let owner = GraphRefHostId::try_new(SemanticObjectId::utterance(3)).expect("act host");
        let attempt = attempt_lexical_dynamic_edge(&registry, &candidate, Some(owner));
        let data!(LexicalEdgeAttempt::Fallback { reason, .. }) = attempt.as_data() else {
            panic!("opaque edge with escaping owner must fail");
        };
        assert!(matches!(
            reason.as_data(),
            data!(LexicalEdgeFallbackReason::IncompatibleOwner {
                failure: LexicalEdgeOwnerFailure::OpaqueCannotEscape,
                ..
            })
        ));
        assert!(matches!(
            attempt_lexical_dynamic_edge(&registry, &candidate, None).as_data(),
            data!(LexicalEdgeAttempt::Constructed { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_row_accepts_each_other_closed_policy_without_identity_drift() {
        let generated = LexicalPolicyRegistry::generated();
        let all = [
            ScopePolicy::Extensional,
            ScopePolicy::Intensional,
            ScopePolicy::Opaque,
        ];
        for selected in &generated.rows {
            for alternative in all
                .iter()
                .copied()
                .filter(|policy| *policy != selected.policy)
            {
                let rows = generated
                    .rows
                    .iter()
                    .map(|row| {
                        VerifiedLexicalPolicy::new(
                            row.key.clone(),
                            row.attested_arity,
                            row.accepted_family,
                            if row.key == selected.key {
                                alternative
                            } else {
                                row.policy
                            },
                        )
                    })
                    .collect();
                let mutated = LexicalPolicyRegistry::new(rows);
                let looked_up = mutated
                    .lookup(
                        &selected.key,
                        selected.attested_arity,
                        selected.accepted_family,
                    )
                    .expect("mutating policy must preserve lookup identity");
                assert_eq!(looked_up.policy(), alternative);
                let candidate = LexicalEdgeCandidate::new(
                    new!(LexicalEdgeSite::Predication {
                        predication: SemanticObjectId::predication(80),
                    }),
                    SemanticObjectId::referent(81),
                    selected.key.clone(),
                    selected.accepted_family,
                    selected.attested_arity,
                );
                assert!(matches!(
                    attempt_lexical_dynamic_edge(&mutated, &candidate, None).as_data(),
                    data!(LexicalEdgeAttempt::Constructed { .. })
                ));
                let owner = GraphRefHostId::try_new(SemanticObjectId::utterance(82))
                    .expect("synthetic act host");
                let owned = attempt_lexical_dynamic_edge(&mutated, &candidate, Some(owner));
                assert_eq!(
                    matches!(owned.as_data(), data!(LexicalEdgeAttempt::Fallback { .. })),
                    alternative == ScopePolicy::Opaque,
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mandatory_pre_host_collection_is_seven_keys_and_nine_occurrences() {
        let expected: BTreeMap<(String, usize, DynamicValueFamily), usize> = [
            (
                (
                    "blabi".to_owned(),
                    1,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                1,
            ),
            (
                (
                    "djica".to_owned(),
                    2,
                    DynamicValueFamily::RefCompReferentsEventuality,
                ),
                1,
            ),
            (
                (
                    "klama".to_owned(),
                    2,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                2,
            ),
            (
                (
                    "klama".to_owned(),
                    5,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                1,
            ),
            (
                (
                    "melbi".to_owned(),
                    1,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                2,
            ),
            (
                (
                    "pilno".to_owned(),
                    1,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                1,
            ),
            (
                (
                    "pilno".to_owned(),
                    2,
                    DynamicValueFamily::RefCompReferentsEntity,
                ),
                1,
            ),
        ]
        .into_iter()
        .collect();
        let mut actual = BTreeMap::new();
        let mut occurrence_count = 0usize;
        for witness in WITNESSES.lines() {
            let ir = PreHostLexicalIr::collect(&graph_for(witness));
            for edge in ir.owning_edges() {
                *actual
                    .entry((
                        edge.key.relation().to_owned(),
                        edge.key.original_place(),
                        edge.family,
                    ))
                    .or_default() += 1;
                occurrence_count += 1;
            }
        }
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 7);
        assert_eq!(occurrence_count, 9);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn owning_edge_collection_is_independent_of_an_always_failing_policy_table() {
        let rejecting_registry = LexicalPolicyRegistry::new(Vec::new());
        let candidates: Vec<LexicalEdgeCandidate> = WITNESSES
            .lines()
            .flat_map(|witness| {
                PreHostLexicalIr::collect(&graph_for(witness))
                    .owning_edges
                    .into_iter()
            })
            .collect();
        assert_eq!(candidates.len(), 9);
        let attempts: Vec<LexicalEdgeAttempt> = candidates
            .iter()
            .map(|candidate| attempt_lexical_dynamic_edge(&rejecting_registry, candidate, None))
            .collect();
        assert_eq!(attempts.len(), candidates.len());
        assert!(attempts.iter().all(|attempt| matches!(
            attempt.as_data(),
            data!(LexicalEdgeAttempt::Fallback { .. })
        )));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn description_internal_skicu_and_bound_relative_nelci_are_not_owning_edges() {
        for witness_index in [11usize, 12] {
            let text = WITNESSES
                .lines()
                .nth(witness_index - 1)
                .expect("frozen witness index");
            let graph = graph_for(text);
            let ir = PreHostLexicalIr::collect(&graph);
            let forbidden = if witness_index == 11 {
                "skicu"
            } else {
                "nelci"
            };
            assert!(
                ir.owning_edges()
                    .iter()
                    .all(|edge| edge.key.relation() != forbidden)
            );
            let dynamic = ir
                .owning_edges()
                .iter()
                .find(|edge| edge.key.relation() == "melbi")
                .expect("outer melbi owns the description")
                .value;
            let definitions = definition_subgraph(&graph, dynamic);
            let descriptor = graph.objects[&dynamic]
                .descriptor()
                .expect("dynamic description");
            assert_eq!(descriptor.relative_clauses.len(), 1);
            assert!(definitions.contains(&descriptor.relative_clauses[0].body));
            let internal = graph.objects.iter().find_map(|(id, object)| {
                let predication = object.as_predication()?;
                matches!(
                    predication.relation.as_data(),
                    data!(PredicationRelation::Named { relation }) if relation == forbidden
                )
                .then_some((*id, predication))
            });
            let (internal_id, internal) = internal.expect("defining predication");
            assert!(definitions.contains(&internal_id));
            assert!(
                internal
                    .arguments
                    .values()
                    .any(|argument| argument.value == Some(dynamic))
            );
        }
    }
}
