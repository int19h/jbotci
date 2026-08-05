//! Deterministic offline mint and verifier for the current smusni-v0 candidate bundle.
//!
//! The checked-in JSONL files are the normative registry. This module consumes
//! only pinned repository bytes, projects the one authored completeness ledger,
//! validates every cross-table invariant, and either writes the exact bundle or
//! proves that the checked-in bytes are already current.

#![allow(dead_code)] // build.rs and the rejection-test crate exercise different APIs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use crate::smusni_v0_kernel::datum::{Datum, parse_document, print_document};
use crate::smusni_v0_kernel::syntax::{FallbackReason, parse_v0_expression};
use crate::smusni_v0_kernel::type_system::{
    PlaceLabel, PositiveInteger, Row, SignKind, TypeAtom, TypeExpr,
};

pub const BUNDLE_ROOT: &str = "data/smusni-v0";
pub const MANIFEST_PATH: &str = "registry/manifest.json";
pub const SOURCE_PATH: &str = "sources/registry-source.toml";
pub const SOURCE_PROVENANCE_PATH: &str = "sources/registry-source.provenance.toml";
pub const WITNESS_PATH: &str = "sources/must-compact-witnesses.txt";
pub const OBLIQUE_PATH: &str = "sources/lojban-org/oblique_keywords.txt";
pub const OBLIQUE_METADATA_PATH: &str = "sources/lojban-org/oblique_keywords.metadata.toml";

const INPUT_PREFIX: &str = "sources/generator-inputs";
const SPEC_PATH: &str = "sources/generator-inputs/docs/smusni/spec.md";
const DICTIONARY_PATH: &str =
    "sources/generator-inputs/crates/jbotci-dictionary-data/data/dictionary-en.json";
const DICTIONARY_METADATA_PATH: &str =
    "sources/generator-inputs/crates/jbotci-dictionary-data/data/dictionary-en.metadata.toml";
const COMPLETENESS_INVENTORY_PATH: &str =
    "sources/generator-inputs/crates/jbotci-semantics/src/completeness/inventory.rs.opaque";

const SPEC_SHA256: &str = "c2c0616b0b0d8991251f5145be4985a8191a68704cff3ae10f6f85caa34dbdc1";
const SAMPLES_SHA256: &str = "ee4cfe6c00009f2ca0387efd0dfa6551b4e6db61e6ff9bebf891f0c0346aa50b";
const SMUSNI_SOURCE_REVISION: &str = "86cbd9d1288d2c0232ba86cb214000a431b5db7c";
const OBLIQUE_SHA256: &str = "355786cfd049063c92514fac2d417fc4966df7749dc17d7cfb49bd903fb6a2cb";
const OBLIQUE_BYTE_COUNT: usize = 79_293;
const OBLIQUE_RECORD_COUNT: usize = 3_542;
const DICTIONARY_SHA256: &str = "ba268ad701f8f44656ea4b17a1fd9539cfc1a3c523d0bdf581a44e3e93bb412f";
const FINAL_PLAN_SHA256: &str = "803c3b35f211dacd910efa1d1e793dde3698e0c90b124550a6ab42531d9fe312";
const POLICY_DOSSIER_SHA256: &str =
    "dfc69782de2206d048ad7a519af2a0e3a9b6cbd2fc161f76ad2425c8cb4743d1";
const WITNESS_COUNT: usize = 18;
const SCOPE_POLICY_ROW_COUNT: usize = 8;
const EXTENSIONAL_SCOPE_POLICY_COUNT: usize = 6;
const INTENSIONAL_SCOPE_POLICY_COUNT: usize = 2;
const DISPOSITION_ROW_COUNT: usize = 882;
const FALLBACK_REASON_ROW_COUNT: usize = 60;

const REQUIRED_GRAPH_FAILURE_REASON_IDS: &[&str] = &[
    "smusni.fallback.abstraction-crossing-unlicensed",
    "smusni.fallback.binder-does-not-dominate-use",
    "smusni.fallback.computed-fill-domain-noninjective",
    "smusni.fallback.conflicting-binder-owners",
    "smusni.fallback.de-re-owner-dependency-illegal",
    "smusni.fallback.de-re-owner-missing",
    "smusni.fallback.de-re-owner-opaque",
    "smusni.fallback.de-re-owner-unrelated-or-nondominating",
    "smusni.fallback.de-re-owner-wrong-kind",
    "smusni.fallback.declaration-planning-nonconvergence",
    "smusni.fallback.definition-site-does-not-dominate-use",
    "smusni.fallback.dependent-supplement-unrepresentable",
    "smusni.fallback.dynamic-host-cycle",
    "smusni.fallback.dynamic-host-not-unique",
    "smusni.fallback.effect-handler-missing-or-illegal",
    "smusni.fallback.event-facet-reduction-unregistered",
    "smusni.fallback.event-owner-missing-or-nonunique",
    "smusni.fallback.force-handler-missing-or-illegal",
    "smusni.fallback.force-reduction-unrepresentable",
    "smusni.fallback.generated-eventuality-unbound",
    "smusni.fallback.higher-order-crossing-unlicensed",
    "smusni.fallback.lexical-relation-row-missing",
    "smusni.fallback.lexical-signature-missing-or-stale",
    "smusni.fallback.math-reduction-unregistered",
    "smusni.fallback.modal-tag-reduction-unregistered",
    "smusni.fallback.place-deletion-evidence-missing",
    "smusni.fallback.predicate-closure-unlicensed",
    "smusni.fallback.predicate-fill-type-or-arity-mismatch",
    "smusni.fallback.prelude-reduction-unavailable",
    "smusni.fallback.quantifier-effect-export-illegal",
    "smusni.fallback.quantity-reduction-unregistered",
    "smusni.fallback.question-domain-or-answer-mismatch",
    "smusni.fallback.reference-description-unrepresentable",
    "smusni.fallback.relation-former-reduction-unavailable",
    "smusni.fallback.relation-reduction-unregistered-or-inexact",
    "smusni.fallback.scope-dependency-without-binder",
    "smusni.fallback.sequence-reduction-unregistered",
    "smusni.fallback.sign-identity-missing",
    "smusni.fallback.simultaneous-termset-unlicensed",
    "smusni.fallback.structured-quotation-transcript-entry-missing",
    "smusni.fallback.unguarded-or-unrepresentable-scc",
    "smusni.fallback.unknown-registry-coordinate",
];

const GENERATED_TABLES: &[(&str, SchemaId)] = &[
    (
        "registry/source-artifacts.jsonl",
        SchemaId::SourceArtifactRow,
    ),
    ("registry/evidence.jsonl", SchemaId::EvidenceRow),
    ("registry/lexical.jsonl", SchemaId::LexicalRow),
    ("registry/scope-policies.jsonl", SchemaId::ScopePolicyRow),
    (
        "registry/place-deletions.jsonl",
        SchemaId::PlaceDeletionEvidenceRow,
    ),
    ("registry/tag-reductions.jsonl", SchemaId::TagReductionRow),
    (
        "registry/relation-formers.jsonl",
        SchemaId::RelationFormerReductionRow,
    ),
    (
        "registry/generated-relations.jsonl",
        SchemaId::GeneratedRelationRow,
    ),
    ("registry/scale-literals.jsonl", SchemaId::ScaleLiteralRow),
    (
        "registry/fallback-reasons.jsonl",
        SchemaId::FallbackReasonRow,
    ),
    ("registry/dispositions.jsonl", SchemaId::DispositionRow),
    ("registry/prelude.jsonl", SchemaId::PreludeRow),
    ("registry/runtime.rs", SchemaId::OpaqueBytes),
];

const MIRRORED_GENERATOR_INPUTS: &[(&str, &str)] = &[
    ("sources/generator-inputs/Cargo.lock", "Cargo.lock"),
    ("sources/generator-inputs/Cargo.toml", "Cargo.toml"),
    (SPEC_PATH, "docs/smusni/spec.md"),
    (
        "sources/generator-inputs/crates/bityzba/Cargo.toml",
        "crates/bityzba/Cargo.toml",
    ),
    (
        "sources/generator-inputs/crates/bityzba/src/contract_scanner.rs.opaque",
        "crates/bityzba/src/contract_scanner.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba/src/lib.rs.opaque",
        "crates/bityzba/src/lib.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-contract-syntax/Cargo.toml",
        "crates/bityzba-contract-syntax/Cargo.toml",
    ),
    (
        "sources/generator-inputs/crates/bityzba-contract-syntax/src/lib.rs.opaque",
        "crates/bityzba-contract-syntax/src/lib.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/Cargo.toml",
        "crates/bityzba-macros/Cargo.toml",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/codegen.rs.opaque",
        "crates/bityzba-macros/src/implementation/codegen.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/data.rs.opaque",
        "crates/bityzba-macros/src/implementation/data.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/doc.rs.opaque",
        "crates/bityzba-macros/src/implementation/doc.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/ensures.rs.opaque",
        "crates/bityzba-macros/src/implementation/ensures.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/invariant.rs.opaque",
        "crates/bityzba-macros/src/implementation/invariant.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/mod.rs.opaque",
        "crates/bityzba-macros/src/implementation/mod.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/parse.rs.opaque",
        "crates/bityzba-macros/src/implementation/parse.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/requires.rs.opaque",
        "crates/bityzba-macros/src/implementation/requires.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/traits.rs.opaque",
        "crates/bityzba-macros/src/implementation/traits.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/implementation/type_invariant.rs.opaque",
        "crates/bityzba-macros/src/implementation/type_invariant.rs",
    ),
    (
        "sources/generator-inputs/crates/bityzba-macros/src/lib.rs.opaque",
        "crates/bityzba-macros/src/lib.rs",
    ),
    (
        DICTIONARY_PATH,
        "crates/jbotci-dictionary-data/data/dictionary-en.json",
    ),
    (
        DICTIONARY_METADATA_PATH,
        "crates/jbotci-dictionary-data/data/dictionary-en.metadata.toml",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/Cargo.toml",
        "crates/jbotci-semantics/Cargo.toml",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/build.rs.opaque",
        "crates/jbotci-semantics/build.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/codegen/smusni_v0_bundle.rs.opaque",
        "crates/jbotci-semantics/codegen/smusni_v0_bundle.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/codegen/smusni_v0_completeness.rs.opaque",
        "crates/jbotci-semantics/codegen/smusni_v0_completeness.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/codegen/smusni_v0_dispositions.rs.opaque",
        "crates/jbotci-semantics/codegen/smusni_v0_dispositions.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/codegen/smusni_v0_kernel.rs.opaque",
        "crates/jbotci-semantics/codegen/smusni_v0_kernel.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/codegen/smusni_v0_surface.rs.opaque",
        "crates/jbotci-semantics/codegen/smusni_v0_surface.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/examples/smusni_v0_bundle.rs.opaque",
        "crates/jbotci-semantics/examples/smusni_v0_bundle.rs",
    ),
    (
        COMPLETENESS_INVENTORY_PATH,
        "crates/jbotci-semantics/src/completeness/inventory.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/completeness/model.rs.opaque",
        "crates/jbotci-semantics/src/completeness/model.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/model.rs.opaque",
        "crates/jbotci-semantics/src/model.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/model/semantic_object.rs.opaque",
        "crates/jbotci-semantics/src/model/semantic_object.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/model/event_binding.rs.opaque",
        "crates/jbotci-semantics/src/model/event_binding.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/model/scope_dependence.rs.opaque",
        "crates/jbotci-semantics/src/model/scope_dependence.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/notation/sexpr/datum.rs.opaque",
        "crates/jbotci-semantics/src/notation/sexpr/datum.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/notation/sexpr/syntax.rs.opaque",
        "crates/jbotci-semantics/src/notation/sexpr/syntax.rs",
    ),
    (
        "sources/generator-inputs/crates/jbotci-semantics/src/notation/sexpr/type_system.rs.opaque",
        "crates/jbotci-semantics/src/notation/sexpr/type_system.rs",
    ),
];

const BUNDLE_NATIVE_GENERATOR_INPUTS: &[&str] = &[
    OBLIQUE_PATH,
    OBLIQUE_METADATA_PATH,
    SOURCE_PATH,
    SOURCE_PROVENANCE_PATH,
    WITNESS_PATH,
];

const EXPECTED_WITNESSES: &str = concat!(
    "mi klama\n",
    "mi klama fu lo karce\n",
    "mi dunda zi'o ti\n",
    "mi klama sepi'o lo karce\n",
    "mi klama fi'o pilno lo karce\n",
    "ro mlatu cu jbena\n",
    "ti blanu zdani\n",
    "mi pu klama lo zarci\n",
    "mi djica lo nu mi cilre\n",
    "lo ci gerku cu blabi\n",
    "le gerku poi blabi cu melbi\n",
    "lo cukta noi mi nelci ke'a cu melbi\n",
    "ci gerku ce'e re nanmu cu batci\n",
    "ma klama lo zarci\n",
    "ti mo\n",
    "xu do klama\n",
    "li re su'i ci du li mu\n",
    "ni'o mi klama\n",
);

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleErrorKind {
    Io,
    Parse,
    ByteDomain,
    NonCanonicalOrder,
    DuplicatePrimaryKey,
    ClosedValue,
    ForeignKey,
    Evidence,
    Template,
    Type,
    Summary,
    Digest,
    Manifest,
    Drift,
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleError {
    pub kind: BundleErrorKind,
    pub message: String,
}

impl BundleError {
    #[requires(true)]
    #[ensures(ret.kind == kind && !ret.message.is_empty())]
    fn new(kind: BundleErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        new!(BundleError { kind, message })
    }
}

impl fmt::Display for BundleError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "smusni-v0 bundle {:?}: {}",
            self.kind, self.message
        )
    }
}

impl std::error::Error for BundleError {}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlePaths {
    pub root: PathBuf,
    pub repository_root: PathBuf,
    pub generated_rust: PathBuf,
}

impl BundlePaths {
    #[requires(true)]
    #[ensures(ret.root.ends_with(BUNDLE_ROOT))]
    pub fn for_manifest_dir(manifest_dir: &Path, generated_rust: PathBuf) -> Self {
        let repository_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir)
            .to_path_buf();
        Self {
            root: manifest_dir.join(BUNDLE_ROOT),
            repository_root,
            generated_rust,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleMode {
    Check,
    Generate,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispositionSeed {
    pub owner: String,
    pub model_member: String,
    pub disposition: String,
    pub target_contract: Option<String>,
    pub detail: Option<String>,
    pub fallback_reason_id: Option<String>,
    pub expected_type_schema: Option<String>,
    pub minimum_raw_owner_type: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrySource {
    format_version: u32,
    bundle_schema_version: u32,
    lexical: Vec<LexicalSource>,
    scope_policy: Vec<ScopePolicySource>,
    place_deletion: Vec<PlaceDeletionSource>,
    tag_reduction: Vec<TagReductionSource>,
    relation_former_reduction: Vec<RelationFormerSource>,
    generated_relation: Vec<GeneratedRelationSource>,
    scale_literal: Vec<ScaleLiteralSource>,
    prelude: Vec<PreludeSource>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryProvenance {
    format_version: u32,
    smusni_source_revision: String,
    spec_sha256: String,
    samples_sha256: String,
    approved_plan_sha256: String,
    lexical_policy_dossier_sha256: String,
    approval_record: String,
    supported_lexical_domain: Value,
    scope_policy: Vec<ScopePolicyProvenance>,
    future_rows: Value,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopePolicyProvenance {
    normalized_root: String,
    original_ordinal: u64,
    policy: ScopePolicy,
    evidence: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LexicalSource {
    root: String,
    #[serde(default = "default_word_class")]
    word_class: String,
    #[serde(default)]
    dictionary_entry_id: Option<String>,
    slot_types: Vec<String>,
    slot_close_policies: Vec<ClosePolicy>,
    event_slot: EventSlotPolicy,
    #[serde(default = "default_evidence_source")]
    evidence_source: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum EventSlotPolicy {
    Absent,
    LocalExistential,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopePolicySource {
    normalized_root: String,
    original_ordinal: u64,
    scope_policy: ScopePolicy,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaceDeletionSource {
    expansion_owner: String,
    normalized_root: String,
    original_ordinal: u64,
    surviving_slot_map: Vec<String>,
    semantic_absence_contract: String,
    evidence_id: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TagReductionSource {
    source_family: String,
    source_member: String,
    applicability_guard: String,
    operand_types: Vec<String>,
    source_place_map: Vec<String>,
    host_event_map: HostEventMap,
    required_graph_identities: Vec<String>,
    typed_expansion_template: String,
    resulting_type_schema: String,
    expected_dynamic_summary: DynamicSummary,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationFormerSource {
    former_kind: String,
    source_owner: String,
    applicability_guard: String,
    operand_row_schemas: Vec<String>,
    result_row_schema: String,
    total_provenance_map: Vec<String>,
    typed_link_or_expansion_contract: String,
    expected_dynamic_summary: DynamicSummary,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedRelationSource {
    family: String,
    pascal_case_name: String,
    complete_signature: String,
    irreducibility_reason: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScaleLiteralSource {
    pascal_case_name: String,
    raw_value_type: String,
    source_members: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreludeSource {
    name: String,
    type_parameters: Vec<String>,
    complete_signature_schema: String,
    direct_dependencies: Vec<String>,
    expected_dynamic_summary: DynamicSummary,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostEventMap {
    Shared,
    PossibleOnly,
    Local,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DynamicContextFlow {
    Identity,
    Parameterized,
    Projective,
    Updating,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DynamicEffect {
    Presupposition,
    Supplement,
    ReferenceIntroduction,
    Performance,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DynamicStability {
    Stable,
    SiteStableWithinPerformance,
    Parameterized,
    Unstable,
}

#[invariant(parameter_evaluations.iter().all(|name| is_summary_parameter_name(name)))]
#[invariant(match *context_flow {
    DynamicContextFlow::Identity => parameter_evaluations.is_empty() && ordered_effects.is_empty(),
    DynamicContextFlow::Parameterized => !parameter_evaluations.is_empty() && ordered_effects.is_empty(),
    DynamicContextFlow::Projective => ordered_effects.iter().any(|effect| matches!(effect, DynamicEffect::Presupposition | DynamicEffect::Supplement)),
    DynamicContextFlow::Updating => !ordered_effects.is_empty() && ordered_effects.iter().all(|effect| matches!(effect, DynamicEffect::ReferenceIntroduction | DynamicEffect::Performance)),
})]
#[invariant(match *stability {
    DynamicStability::Stable | DynamicStability::SiteStableWithinPerformance => parameter_evaluations.is_empty() && ordered_effects.is_empty(),
    DynamicStability::Parameterized => !parameter_evaluations.is_empty() && ordered_effects.is_empty(),
    DynamicStability::Unstable => !ordered_effects.is_empty(),
})]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DynamicSummary {
    context_flow: DynamicContextFlow,
    parameter_evaluations: Vec<String>,
    ordered_effects: Vec<DynamicEffect>,
    stability: DynamicStability,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TagSourceIdentity {
    TagSumti,
    HostEvent,
    UtteranceNow,
    HostX1,
    RightwardDisplacement,
    HostReconstruction,
    Speaker,
    RelativeHead,
    Clause,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RequiredGraphIdentity {
    HostEvent,
    HostX1,
    UtteranceGround,
    RelativeHead,
}

#[invariant(!target.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TagSourcePlaceMapping {
    source: TagSourceIdentity,
    target: String,
}

#[invariant(::Hole { name, type_schema } => is_hole_name(name) && !type_schema.is_empty())]
#[invariant(::Constant { spelling } => !spelling.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedTagTarget {
    Hole { name: String, type_schema: String },
    Constant { spelling: String },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosePolicy {
    Required,
    Contextual,
    LocalExistential,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScopePolicy {
    Extensional,
    Intensional,
    Opaque,
}

#[invariant(::Numbered(value) => *value > 0)]
#[invariant(::Eventuality(value) => value == "Eventuality")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SlotLabel {
    Numbered(u64),
    Eventuality(String),
}

#[invariant(!accepted_type_schema.is_empty() && !lexical_provenance.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SlotRow {
    pub label: SlotLabel,
    pub accepted_type_schema: String,
    pub close_policy: ClosePolicy,
    pub lexical_provenance: String,
    pub evidence_id: String,
}

#[invariant(!source_id.is_empty() && !source_kind.is_empty() && !immutable_revision.is_empty() && !canonical_locator.is_empty() && is_digest(&artifact_digest))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceArtifactRow {
    pub source_id: String,
    pub source_kind: String,
    pub immutable_revision: String,
    pub canonical_locator: String,
    pub artifact_digest: String,
}

#[invariant(!evidence_id.is_empty() && !source_id.is_empty() && !exact_locator.is_empty() && is_digest(&cited_content_digest) && !adjudication_note.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EvidenceRow {
    pub evidence_id: String,
    pub source_id: String,
    pub exact_locator: String,
    pub cited_content_digest: String,
    pub adjudication_note: String,
}

#[invariant(!root.is_empty() && root == normalized_root && !word_class.is_empty() && !dictionary_source_id.is_empty() && !dictionary_entry_id.is_empty() && !ordered_numbered_slot_rows.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LexicalRow {
    pub root: String,
    pub normalized_root: String,
    pub word_class: String,
    pub dictionary_source_id: String,
    pub dictionary_entry_id: String,
    pub ordered_numbered_slot_rows: Vec<SlotRow>,
    pub optional_event_slot_row: Option<SlotRow>,
}

#[invariant(!normalized_root.is_empty() && *original_ordinal > 0 && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScopePolicyRow {
    pub normalized_root: String,
    pub original_ordinal: u64,
    pub scope_policy: ScopePolicy,
    pub evidence_id: String,
}

#[invariant(!expansion_owner.is_empty() && !normalized_root.is_empty() && *original_ordinal > 0 && !input_row_schema.is_empty() && !result_row_schema.is_empty() && !surviving_slot_map.is_empty() && !semantic_absence_contract.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlaceDeletionEvidenceRow {
    pub expansion_owner: String,
    pub normalized_root: String,
    pub original_ordinal: u64,
    pub input_row_schema: String,
    pub result_row_schema: String,
    pub surviving_slot_map: Vec<String>,
    pub semantic_absence_contract: String,
    pub evidence_id: String,
}

#[invariant(!source_family.is_empty() && !source_member.is_empty() && !applicability_guard.is_empty() && !operand_types.is_empty() && !source_place_map.is_empty() && !typed_expansion_template.is_empty() && !resulting_type_schema.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TagReductionRow {
    pub source_family: String,
    pub source_member: String,
    pub applicability_guard: String,
    pub operand_types: Vec<String>,
    pub source_place_map: Vec<String>,
    pub host_event_map: HostEventMap,
    pub required_graph_identities: Vec<String>,
    pub typed_expansion_template: String,
    pub resulting_type_schema: String,
    pub evidence_id: String,
}

#[invariant(!former_kind.is_empty() && !source_owner.is_empty() && !applicability_guard.is_empty() && !operand_row_schemas.is_empty() && !result_row_schema.is_empty() && !total_provenance_map.is_empty() && !typed_link_or_expansion_contract.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RelationFormerReductionRow {
    pub former_kind: String,
    pub source_owner: String,
    pub applicability_guard: String,
    pub operand_row_schemas: Vec<String>,
    pub result_row_schema: String,
    pub total_provenance_map: Vec<String>,
    pub typed_link_or_expansion_contract: String,
    pub evidence_id: String,
}

#[invariant(context == "identity" && effects.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextEffectSummary {
    pub context: String,
    pub effects: Vec<String>,
}

#[invariant(!family.is_empty() && is_pascal_case(&pascal_case_name) && !complete_signature.is_empty() && context_effect_summary.context == "identity" && context_effect_summary.effects.is_empty() && stability_summary == "site-stable-within-performance" && !irreducibility_reason.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GeneratedRelationRow {
    pub family: String,
    #[serde(rename = "PascalCase-name")]
    pub pascal_case_name: String,
    pub complete_signature: String,
    pub context_effect_summary: ContextEffectSummary,
    pub stability_summary: String,
    pub irreducibility_reason: String,
    pub evidence_id: String,
}

#[invariant(is_pascal_case(&pascal_case_name) && !raw_value_type.is_empty() && !source_members.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScaleLiteralRow {
    #[serde(rename = "PascalCase-name")]
    pub pascal_case_name: String,
    pub raw_value_type: String,
    pub source_members: Vec<String>,
    pub evidence_id: String,
}

#[invariant(is_reason_id(&reason_id) && !expected_type_schema.is_empty() && !minimum_raw_owner_type.is_empty() && !disposition_owner.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FallbackReasonRow {
    pub reason_id: String,
    pub expected_type_schema: String,
    pub minimum_raw_owner_type: String,
    pub disposition_owner: String,
}

#[invariant(!disposition_owner.is_empty() && !model_constructor_or_field.is_empty() && is_disposition(&disposition) && !target_schema_or_fallback_reason.is_empty() && !evidence_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DispositionRow {
    pub disposition_owner: String,
    pub model_constructor_or_field: String,
    pub disposition: String,
    pub target_schema_or_fallback_reason: String,
    pub evidence_id: String,
}

#[invariant(!name.is_empty() && type_parameters.iter().all(|parameter| is_type_parameter_name(parameter)) && type_parameters.iter().enumerate().all(|(index, parameter)| !type_parameters[..index].contains(parameter)) && !complete_signature_schema.is_empty() && !canonical_definition.is_empty() && is_digest(&definition_digest))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreludeRow {
    pub name: String,
    pub type_parameters: Vec<String>,
    pub complete_signature_schema: String,
    pub canonical_definition: String,
    pub direct_dependencies: Vec<String>,
    pub definition_digest: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaId {
    OpaqueBytes,
    SourceArtifactRow,
    EvidenceRow,
    LexicalRow,
    ScopePolicyRow,
    PlaceDeletionEvidenceRow,
    TagReductionRow,
    RelationFormerReductionRow,
    GeneratedRelationRow,
    ScaleLiteralRow,
    FallbackReasonRow,
    DispositionRow,
    PreludeRow,
}

impl SchemaId {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn as_str(self) -> &'static str {
        match self {
            Self::OpaqueBytes => "OpaqueBytes",
            Self::SourceArtifactRow => "SourceArtifactRow",
            Self::EvidenceRow => "EvidenceRow",
            Self::LexicalRow => "LexicalRow",
            Self::ScopePolicyRow => "ScopePolicyRow",
            Self::PlaceDeletionEvidenceRow => "PlaceDeletionEvidenceRow",
            Self::TagReductionRow => "TagReductionRow",
            Self::RelationFormerReductionRow => "RelationFormerReductionRow",
            Self::GeneratedRelationRow => "GeneratedRelationRow",
            Self::ScaleLiteralRow => "ScaleLiteralRow",
            Self::FallbackReasonRow => "FallbackReasonRow",
            Self::DispositionRow => "DispositionRow",
            Self::PreludeRow => "PreludeRow",
        }
    }
}

#[invariant(!relative_path.is_empty() && !schema_id.is_empty() && *row_count > 0 && is_digest(&digest))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ArtifactRecord {
    relative_path: String,
    schema_id: String,
    row_count: usize,
    digest: String,
}

#[invariant(*format_version == 0 && *bundle_schema_version == 1 && is_digest(&spec_digest) && is_digest(&generator_id) && !generator_inputs.is_empty() && !source_artifacts.is_empty() && !generated_artifacts.is_empty() && is_digest(&bundle_digest))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Manifest {
    format_version: u32,
    bundle_schema_version: u32,
    spec_digest: String,
    generator_id: String,
    generator_inputs: Vec<String>,
    source_artifacts: Vec<ArtifactRecord>,
    generated_artifacts: Vec<ArtifactRecord>,
    bundle_digest: String,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct DictionaryIdentity {
    definition_id: u64,
    word_type: String,
    fingerprint: String,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub struct BundleSnapshot {
    pub artifacts: BTreeMap<String, Vec<u8>>,
    pub manifest: Vec<u8>,
    pub policy_rust: Vec<u8>,
}

#[requires(true)]
#[ensures(ret == "gismu")]
fn default_word_class() -> String {
    "gismu".to_owned()
}

#[requires(true)]
#[ensures(ret == "dictionary")]
fn default_evidence_source() -> String {
    "dictionary".to_owned()
}

#[invariant(true)]
#[derive(Debug)]
struct Tables {
    source_artifacts: Vec<SourceArtifactRow>,
    evidence: Vec<EvidenceRow>,
    lexical: Vec<LexicalRow>,
    scope_policies: Vec<ScopePolicyRow>,
    place_deletions: Vec<PlaceDeletionEvidenceRow>,
    tag_reductions: Vec<TagReductionRow>,
    relation_formers: Vec<RelationFormerReductionRow>,
    generated_relations: Vec<GeneratedRelationRow>,
    scale_literals: Vec<ScaleLiteralRow>,
    fallback_reasons: Vec<FallbackReasonRow>,
    dispositions: Vec<DispositionRow>,
    prelude: Vec<PreludeRow>,
}

/// Generate or check the candidate bundle, then compile its policy table
/// into `OUT_DIR` for the private typed-IR consumer.
#[requires(!dispositions.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn run(
    paths: &BundlePaths,
    dispositions: &[DispositionSeed],
    mode: BundleMode,
) -> Result<(), BundleError> {
    validate_local_dependency_mappings(&paths.repository_root, MIRRORED_GENERATOR_INPUTS)?;
    synchronize_generator_inputs(paths, mode)?;
    let minted = mint_snapshot(&paths.root, dispositions)?;
    verify_snapshot(&paths.root, &minted)?;
    match mode {
        BundleMode::Generate => write_minted(&paths.root, &minted)?,
        BundleMode::Check => check_minted(&paths.root, &minted)?,
    }
    fs::write(&paths.generated_rust, &minted.policy_rust).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("write {}: {error}", paths.generated_rust.display()),
        )
    })?;
    Ok(())
}

/// Every bundled input or generated artifact whose change must rerun the build
/// verifier.
#[requires(true)]
#[ensures(ret.contains(&MANIFEST_PATH.to_owned()) && ret.contains(&SOURCE_PATH.to_owned()))]
pub fn bundle_rerun_paths() -> Vec<String> {
    let mut paths = generator_input_paths();
    paths.push(MANIFEST_PATH.to_owned());
    paths.extend(GENERATED_TABLES.iter().map(|(path, _)| (*path).to_owned()));
    paths.sort_by(|left, right| scalar_cmp(left, right));
    paths.dedup();
    paths
}

/// Every repository source mirrored into the candidate bundle.
#[requires(true)]
#[ensures(ret.len() == MIRRORED_GENERATOR_INPUTS.len())]
pub fn repository_rerun_paths() -> Vec<String> {
    MIRRORED_GENERATOR_INPUTS
        .iter()
        .map(|(_, source)| (*source).to_owned())
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == MIRRORED_GENERATOR_INPUTS.len() + BUNDLE_NATIVE_GENERATOR_INPUTS.len())]
fn generator_input_paths() -> Vec<String> {
    MIRRORED_GENERATOR_INPUTS
        .iter()
        .map(|(bundled, _)| (*bundled).to_owned())
        .chain(
            BUNDLE_NATIVE_GENERATOR_INPUTS
                .iter()
                .map(|path| (*path).to_owned()),
        )
        .collect()
}

/// Produce exact bytes without touching the filesystem; rejection and
/// reproducibility tests use this boundary.
#[requires(!dispositions.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|bundle| bundle.artifacts.len() == GENERATED_TABLES.len()) || ret.is_err())]
pub fn mint_snapshot(
    root: &Path,
    dispositions: &[DispositionSeed],
) -> Result<BundleSnapshot, BundleError> {
    let source_bytes = read_relative(root, SOURCE_PATH)?;
    mint_snapshot_from_registry_source(root, dispositions, &source_bytes)
}

/// Validate replacement registry-source bytes against all pinned external
/// inputs without writing them. Rejection tests mutate one semantic contract
/// at a time through this boundary.
#[requires(!dispositions.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn validate_registry_source(
    root: &Path,
    dispositions: &[DispositionSeed],
    source_bytes: &[u8],
) -> Result<(), BundleError> {
    mint_snapshot_from_registry_source(root, dispositions, source_bytes).map(|_| ())
}

#[requires(!dispositions.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|bundle| bundle.artifacts.len() == GENERATED_TABLES.len()) || ret.is_err())]
fn mint_snapshot_from_registry_source(
    root: &Path,
    dispositions: &[DispositionSeed],
    source_bytes: &[u8],
) -> Result<BundleSnapshot, BundleError> {
    validate_disposition_coordinate_authority(dispositions)?;
    require_nfc_utf8(SOURCE_PATH, &source_bytes)?;
    let source_text = std::str::from_utf8(source_bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    let source: RegistrySource = toml::from_str(source_text).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Parse,
            format!("parse {SOURCE_PATH}: {error}"),
        )
    })?;
    if source.format_version != 0 || source.bundle_schema_version != 1 {
        return Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            "the candidate bundle must claim format 0 and bundle schema 1",
        ));
    }
    audit_registry_provenance(root, &source.scope_policy)?;

    let spec = read_relative(root, SPEC_PATH)?;
    require_nfc_utf8(SPEC_PATH, &spec)?;
    if sha256_hex(&spec) != SPEC_SHA256 {
        return Err(BundleError::new(
            BundleErrorKind::Digest,
            "docs/smusni/spec.md differs from the frozen v0 digest",
        ));
    }
    let witnesses = read_relative(root, WITNESS_PATH)?;
    audit_witnesses(&witnesses)?;
    let oblique = read_relative(root, OBLIQUE_PATH)?;
    let oblique_roots = audit_oblique(&oblique)?;
    let oblique_metadata = read_relative(root, OBLIQUE_METADATA_PATH)?;
    audit_oblique_metadata(&oblique_metadata)?;
    let dictionary = read_relative(root, DICTIONARY_PATH)?;
    let dictionary_identities = audit_dictionary(&dictionary)?;
    let dictionary_metadata = read_relative(root, DICTIONARY_METADATA_PATH)?;
    audit_dictionary_metadata(&dictionary_metadata)?;

    let source_artifacts = build_source_artifact_rows(root, &dictionary, source_bytes)?;
    let source_digest = source_artifacts
        .iter()
        .find(|row| row.source_id == "smusni-v0-registry-source")
        .expect("source artifact builder supplies the registry source")
        .artifact_digest
        .clone();
    let provenance_digest = source_artifacts
        .iter()
        .find(|row| row.source_id == "smusni-v0-registry-provenance")
        .expect("source artifact builder supplies the provenance sidecar")
        .artifact_digest
        .clone();
    let (lexical, mut evidence) = build_lexical_rows(
        &source.lexical,
        &oblique_roots,
        &dictionary_identities,
        &source_digest,
    )?;
    let scope_policies = build_scope_policy_rows(
        &source.scope_policy,
        &lexical,
        &provenance_digest,
        &mut evidence,
    )?;
    let place_deletions =
        build_place_deletion_rows(&source.place_deletion, &lexical, &mut evidence)?;
    let prelude = build_prelude_rows(&source.prelude, &spec, &place_deletions, &lexical)?;
    let tag_reductions =
        build_tag_reduction_rows(&source.tag_reduction, &lexical, &prelude, &mut evidence)?;
    let relation_formers =
        build_relation_former_rows(&source.relation_former_reduction, &mut evidence)?;
    let generated_relations =
        build_generated_relation_rows(&source.generated_relation, &mut evidence)?;
    let scale_literals = build_scale_literal_rows(&source.scale_literal, &mut evidence)?;
    let (disposition_rows, fallback_reasons) = build_disposition_rows(dispositions)?;
    validate_minted_disposition_coordinates(dispositions, &disposition_rows)?;
    add_common_evidence(&mut evidence, &source_artifacts)?;
    evidence.sort_by(|left, right| scalar_cmp(&left.evidence_id, &right.evidence_id));
    reject_duplicate(
        evidence.iter().map(|row| row.evidence_id.as_str()),
        "evidence-id",
    )?;

    let tables = Tables {
        source_artifacts,
        evidence,
        lexical,
        scope_policies,
        place_deletions,
        tag_reductions,
        relation_formers,
        generated_relations,
        scale_literals,
        fallback_reasons,
        dispositions: disposition_rows,
        prelude,
    };
    validate_tables(&tables, &spec)?;
    validate_source_summary_claims(&source, &tables)?;
    let artifacts = serialize_tables(&tables)?;
    let source_manifest = build_source_manifest(root)?;
    let generated_manifest = build_generated_manifest(&artifacts)?;
    let manifest = build_manifest(source_manifest, generated_manifest)?;
    let manifest_bytes = jcs_line(&manifest)?;
    let policy_rust = generate_policy_rust(&tables)?;
    Ok(BundleSnapshot {
        artifacts,
        manifest: manifest_bytes,
        policy_rust,
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn write_minted(root: &Path, minted: &BundleSnapshot) -> Result<(), BundleError> {
    for (relative, bytes) in &minted.artifacts {
        write_relative(root, relative, bytes)?;
    }
    write_relative(root, MANIFEST_PATH, &minted.manifest)?;
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_minted(root: &Path, minted: &BundleSnapshot) -> Result<(), BundleError> {
    for (relative, expected) in &minted.artifacts {
        let actual = read_relative(root, relative)?;
        if &actual != expected {
            return Err(BundleError::new(
                BundleErrorKind::Drift,
                format!("generated artifact is stale: {relative}"),
            ));
        }
    }
    let actual_manifest = read_relative(root, MANIFEST_PATH)?;
    if actual_manifest != minted.manifest {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "registry/manifest.json is stale",
        ));
    }
    Ok(())
}

/// Reparse and revalidate a complete in-memory bundle. This is the mutation-
/// test boundary: it does not trust generator construction or serde output.
#[requires(snapshot.artifacts.len() == GENERATED_TABLES.len())]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn verify_snapshot(root: &Path, snapshot: &BundleSnapshot) -> Result<(), BundleError> {
    let spec = read_relative(root, SPEC_PATH)?;
    let source_bytes = read_relative(root, SOURCE_PATH)?;
    require_nfc_utf8(SOURCE_PATH, &source_bytes)?;
    let source_text = std::str::from_utf8(&source_bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    let source: RegistrySource = toml::from_str(source_text).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Parse,
            format!("parse {SOURCE_PATH}: {error}"),
        )
    })?;
    let expected_paths = GENERATED_TABLES
        .iter()
        .map(|(path, _)| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    if snapshot.artifacts.keys().cloned().collect::<BTreeSet<_>>() != expected_paths {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            "generated artifact set differs from the closed v0 tables",
        ));
    }
    let tables = Tables {
        source_artifacts: parse_jsonl_artifact(
            &snapshot.artifacts["registry/source-artifacts.jsonl"],
            "registry/source-artifacts.jsonl",
        )?,
        evidence: parse_jsonl_artifact(
            &snapshot.artifacts["registry/evidence.jsonl"],
            "registry/evidence.jsonl",
        )?,
        lexical: parse_jsonl_artifact(
            &snapshot.artifacts["registry/lexical.jsonl"],
            "registry/lexical.jsonl",
        )?,
        scope_policies: parse_jsonl_artifact(
            &snapshot.artifacts["registry/scope-policies.jsonl"],
            "registry/scope-policies.jsonl",
        )?,
        place_deletions: parse_jsonl_artifact(
            &snapshot.artifacts["registry/place-deletions.jsonl"],
            "registry/place-deletions.jsonl",
        )?,
        tag_reductions: parse_jsonl_artifact(
            &snapshot.artifacts["registry/tag-reductions.jsonl"],
            "registry/tag-reductions.jsonl",
        )?,
        relation_formers: parse_jsonl_artifact(
            &snapshot.artifacts["registry/relation-formers.jsonl"],
            "registry/relation-formers.jsonl",
        )?,
        generated_relations: parse_jsonl_artifact(
            &snapshot.artifacts["registry/generated-relations.jsonl"],
            "registry/generated-relations.jsonl",
        )?,
        scale_literals: parse_jsonl_artifact(
            &snapshot.artifacts["registry/scale-literals.jsonl"],
            "registry/scale-literals.jsonl",
        )?,
        fallback_reasons: parse_jsonl_artifact(
            &snapshot.artifacts["registry/fallback-reasons.jsonl"],
            "registry/fallback-reasons.jsonl",
        )?,
        dispositions: parse_jsonl_artifact(
            &snapshot.artifacts["registry/dispositions.jsonl"],
            "registry/dispositions.jsonl",
        )?,
        prelude: parse_jsonl_artifact(
            &snapshot.artifacts["registry/prelude.jsonl"],
            "registry/prelude.jsonl",
        )?,
    };
    validate_tables(&tables, &spec)?;
    validate_source_summary_claims(&source, &tables)?;
    if generate_policy_rust(&tables)? != snapshot.artifacts["registry/runtime.rs"] {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "generated runtime registry differs from the normative JSONL tables",
        ));
    }
    if serialize_tables(&tables)? != snapshot.artifacts {
        return Err(BundleError::new(
            BundleErrorKind::NonCanonicalOrder,
            "parsed tables do not reproduce their exact canonical bytes",
        ));
    }
    if generate_policy_rust(&tables)? != snapshot.policy_rust {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "compiled policy artifact differs from the normative table",
        ));
    }
    let source_manifest = build_source_manifest(root)?;
    let generated_manifest = build_generated_manifest(&snapshot.artifacts)?;
    let expected_manifest = build_manifest(source_manifest, generated_manifest)?;
    if jcs_line(&expected_manifest)? != snapshot.manifest {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            "manifest does not rederive from exact source and generated bytes",
        ));
    }
    let parsed_manifest: Manifest =
        serde_json::from_slice(&snapshot.manifest).map_err(|error| {
            BundleError::new(BundleErrorKind::Parse, format!("parse manifest: {error}"))
        })?;
    if jcs_line(&parsed_manifest)? != snapshot.manifest {
        return Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            "manifest is not one NFC JCS object followed by LF",
        ));
    }
    Ok(())
}

#[requires(!path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|rows| !rows.is_empty()) || ret.is_err())]
fn parse_jsonl_artifact<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    path: &str,
) -> Result<Vec<T>, BundleError> {
    if !bytes.ends_with(b"\n") || bytes.contains(&b'\r') || bytes.starts_with(b"\n") {
        return Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            format!("{path} is not nonempty LF-only JSON Lines"),
        ));
    }
    require_nfc_utf8(path, bytes)?;
    let mut rows = Vec::new();
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: Value = serde_json::from_slice(line).map_err(|error| {
            BundleError::new(BundleErrorKind::Parse, format!("parse {path}: {error}"))
        })?;
        require_nfc_value(&value)?;
        let canonical = canonical_json(&value)?.into_bytes();
        if canonical.as_slice() != line {
            return Err(BundleError::new(
                BundleErrorKind::ByteDomain,
                format!("{path} contains a non-JCS row"),
            ));
        }
        let row = serde_json::from_value(value).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Parse,
                format!("decode typed row in {path}: {error}"),
            )
        })?;
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            format!("{path} has no rows"),
        ));
    }
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn synchronize_generator_inputs(paths: &BundlePaths, mode: BundleMode) -> Result<(), BundleError> {
    for (bundled, repository) in MIRRORED_GENERATOR_INPUTS {
        if !bundled.starts_with(&format!("{INPUT_PREFIX}/")) {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!("mirrored generator input is outside {INPUT_PREFIX}: {bundled}"),
            ));
        }
        let source = read_relative(&paths.repository_root, repository)?;
        match mode {
            BundleMode::Generate => write_relative(&paths.root, bundled, &source)?,
            BundleMode::Check => {
                let checked_in = read_relative(&paths.root, bundled)?;
                if checked_in != source {
                    return Err(BundleError::new(
                        BundleErrorKind::Drift,
                        format!("bundled generator input is stale: {bundled}"),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Prove that a candidate repository-input list contains the complete local
/// path-dependency compilation closure. Mutation tests use this typed boundary
/// to remove one influence without changing generator constants.
#[requires(!repository_inputs.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn validate_local_dependency_closure(
    repository_root: &Path,
    repository_inputs: &[String],
) -> Result<(), BundleError> {
    let expected = discover_local_dependency_closure(repository_root)?;
    let actual = repository_inputs
        .iter()
        .filter(|path| is_local_dependency_path(path))
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
        let extra = actual.difference(&expected).cloned().collect::<Vec<_>>();
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            format!(
                "local path-dependency generator closure differs; missing {missing:?}, extra {extra:?}"
            ),
        ));
    }
    Ok(())
}

#[requires(!mappings.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_local_dependency_mappings(
    repository_root: &Path,
    mappings: &[(&str, &str)],
) -> Result<(), BundleError> {
    let repository_inputs = mappings
        .iter()
        .map(|(_, repository)| (*repository).to_owned())
        .collect::<Vec<_>>();
    validate_local_dependency_closure(repository_root, &repository_inputs)?;
    for (bundled, repository) in mappings
        .iter()
        .filter(|(_, repository)| is_local_dependency_path(repository))
    {
        let expected = bundled_local_dependency_path(repository);
        if *bundled != expected {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!(
                    "local dependency {repository} must be mirrored at {expected}, not {bundled}"
                ),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|paths| !paths.is_empty()) || ret.is_err())]
fn discover_local_dependency_closure(
    repository_root: &Path,
) -> Result<BTreeSet<String>, BundleError> {
    const CRATE_ROOTS: &[&str] = &[
        "crates/bityzba",
        "crates/bityzba-contract-syntax",
        "crates/bityzba-macros",
    ];
    let mut paths = BTreeSet::new();
    for crate_root in CRATE_ROOTS {
        let manifest = format!("{crate_root}/Cargo.toml");
        if !repository_root.join(&manifest).is_file() {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!("local path dependency has no manifest: {manifest}"),
            ));
        }
        paths.insert(manifest);
        let build_script = format!("{crate_root}/build.rs");
        if repository_root.join(&build_script).is_file() {
            paths.insert(build_script);
        }
        collect_dependency_source_files(repository_root, &format!("{crate_root}/src"), &mut paths)?;
    }
    Ok(paths)
}

#[requires(!relative_directory.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_dependency_source_files(
    repository_root: &Path,
    relative_directory: &str,
    paths: &mut BTreeSet<String>,
) -> Result<(), BundleError> {
    let directory = repository_root.join(validate_relative_path(relative_directory)?);
    let mut entries = fs::read_dir(&directory)
        .map_err(|error| {
            BundleError::new(
                BundleErrorKind::Io,
                format!(
                    "read local dependency directory {}: {error}",
                    directory.display()
                ),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            BundleError::new(
                BundleErrorKind::Io,
                format!("read local dependency entry: {error}"),
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry.file_type().map_err(|error| {
            BundleError::new(
                BundleErrorKind::Io,
                format!("inspect local dependency entry: {error}"),
            )
        })?;
        let name = entry.file_name().into_string().map_err(|_| {
            BundleError::new(
                BundleErrorKind::ByteDomain,
                "local dependency source path must be UTF-8",
            )
        })?;
        let relative = format!("{relative_directory}/{name}");
        if file_type.is_symlink() {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!("local dependency source closure forbids symlink {relative}"),
            ));
        }
        if file_type.is_dir() {
            collect_dependency_source_files(repository_root, &relative, paths)?;
        } else if file_type.is_file() {
            paths.insert(relative);
        } else {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!("local dependency source closure has unsupported entry {relative}"),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn is_local_dependency_path(path: &str) -> bool {
    [
        "crates/bityzba/",
        "crates/bityzba-contract-syntax/",
        "crates/bityzba-macros/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

#[requires(is_local_dependency_path(repository_path))]
#[ensures(ret.starts_with(INPUT_PREFIX))]
fn bundled_local_dependency_path(repository_path: &str) -> String {
    if repository_path.ends_with(".rs") {
        format!("{INPUT_PREFIX}/{repository_path}.opaque")
    } else {
        format!("{INPUT_PREFIX}/{repository_path}")
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_relative_path(relative: &str) -> Result<&Path, BundleError> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            format!("artifact path must be a normalized relative path: {relative}"),
        ));
    }
    Ok(path)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn read_relative(root: &Path, relative: &str) -> Result<Vec<u8>, BundleError> {
    let path = validate_relative_path(relative)?;
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("canonicalize {}: {error}", root.display()),
        )
    })?;
    let joined = root.join(path);
    let canonical_path = fs::canonicalize(&joined).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("canonicalize {}: {error}", joined.display()),
        )
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            format!("artifact path resolves outside its root: {relative}"),
        ));
    }
    fs::read(&canonical_path).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("read {}: {error}", canonical_path.display()),
        )
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn write_relative(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), BundleError> {
    let path = validate_relative_path(relative)?;
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("canonicalize {}: {error}", root.display()),
        )
    })?;
    let joined = root.join(path);
    let parent = joined.parent().ok_or_else(|| {
        BundleError::new(BundleErrorKind::Manifest, "artifact path has no parent")
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("create {}: {error}", parent.display()),
        )
    })?;
    let canonical_parent = fs::canonicalize(parent).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("canonicalize {}: {error}", parent.display()),
        )
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            format!("artifact parent resolves outside its root: {relative}"),
        ));
    }
    if joined.exists() {
        let canonical_target = fs::canonicalize(&joined).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Io,
                format!("canonicalize {}: {error}", joined.display()),
            )
        })?;
        if !canonical_target.starts_with(&canonical_root) {
            return Err(BundleError::new(
                BundleErrorKind::Manifest,
                format!("artifact target resolves outside its root: {relative}"),
            ));
        }
    }
    fs::write(&joined, bytes).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Io,
            format!("write {}: {error}", joined.display()),
        )
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn audit_witnesses(bytes: &[u8]) -> Result<(), BundleError> {
    require_nfc_utf8(WITNESS_PATH, bytes)?;
    if bytes != EXPECTED_WITNESSES.as_bytes()
        || bytes.iter().filter(|byte| **byte == b'\n').count() != WITNESS_COUNT
    {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "must-compact witness registry is not the reviewed 18-row set",
        ));
    }
    Ok(())
}

#[requires(scope_rows.len() == SCOPE_POLICY_ROW_COUNT)]
#[ensures(ret.is_ok() || ret.is_err())]
fn audit_registry_provenance(
    root: &Path,
    scope_rows: &[ScopePolicySource],
) -> Result<(), BundleError> {
    let bytes = read_relative(root, SOURCE_PROVENANCE_PATH)?;
    require_nfc_utf8(SOURCE_PROVENANCE_PATH, &bytes)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    let provenance: RegistryProvenance = toml::from_str(text).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Parse,
            format!("parse {SOURCE_PROVENANCE_PATH}: {error}"),
        )
    })?;
    if provenance.format_version != 0
        || provenance.smusni_source_revision != SMUSNI_SOURCE_REVISION
        || provenance.spec_sha256 != SPEC_SHA256
        || provenance.samples_sha256 != SAMPLES_SHA256
        || provenance.approved_plan_sha256 != FINAL_PLAN_SHA256
        || provenance.lexical_policy_dossier_sha256 != POLICY_DOSSIER_SHA256
        || provenance.approval_record.is_empty()
        || !provenance.supported_lexical_domain.is_object()
        || !provenance.future_rows.is_object()
        || provenance.scope_policy.len() != SCOPE_POLICY_ROW_COUNT
        || provenance
            .scope_policy
            .iter()
            .any(|row| row.evidence.is_empty())
    {
        return Err(BundleError::new(
            BundleErrorKind::Evidence,
            "registry provenance sidecar differs from the final reviewed authority",
        ));
    }
    let expected = scope_rows
        .iter()
        .map(|row| {
            (
                row.normalized_root.as_str(),
                row.original_ordinal,
                row.scope_policy,
            )
        })
        .collect::<Vec<_>>();
    let actual = provenance
        .scope_policy
        .iter()
        .map(|row| {
            (
                row.normalized_root.as_str(),
                row.original_ordinal,
                row.policy,
            )
        })
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(BundleError::new(
            BundleErrorKind::Evidence,
            "scope-policy source and provenance sidecar disagree",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|roots| !roots.is_empty()) || ret.is_err())]
fn audit_oblique(bytes: &[u8]) -> Result<BTreeMap<String, BTreeSet<u64>>, BundleError> {
    if bytes.len() != OBLIQUE_BYTE_COUNT
        || sha256_hex(bytes) != OBLIQUE_SHA256
        || bytes.iter().filter(|byte| **byte == b'\n').count() != OBLIQUE_RECORD_COUNT
        || !bytes.ends_with(b"\r\n")
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| *byte == b'\n' && (index == 0 || bytes[index - 1] != b'\r'))
    {
        return Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            "oblique_keywords.txt failed its exact whole-file CRLF audit",
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    let mut roots: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for line in text.split("\r\n").filter(|line| !line.is_empty()) {
        let (key, _) = line.split_once(';').ok_or_else(|| {
            BundleError::new(BundleErrorKind::Parse, "oblique row has no semicolon")
        })?;
        if !seen.insert(key.to_owned()) {
            return Err(BundleError::new(
                BundleErrorKind::DuplicatePrimaryKey,
                format!("duplicate oblique place key {key}"),
            ));
        }
        let digit_start = key
            .find(|character: char| character.is_ascii_digit())
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Parse,
                    format!("place key has no ordinal: {key}"),
                )
            })?;
        let root = &key[..digit_start];
        let ordinal = key[digit_start..].parse::<u64>().map_err(|error| {
            BundleError::new(
                BundleErrorKind::Parse,
                format!("bad ordinal in {key}: {error}"),
            )
        })?;
        if root.is_empty() || ordinal == 0 || !root.bytes().all(|byte| byte.is_ascii_lowercase()) {
            return Err(BundleError::new(
                BundleErrorKind::Parse,
                format!("noncanonical oblique key {key}"),
            ));
        }
        roots.entry(root.to_owned()).or_default().insert(ordinal);
    }
    Ok(roots)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn audit_oblique_metadata(bytes: &[u8]) -> Result<(), BundleError> {
    require_nfc_utf8(OBLIQUE_METADATA_PATH, bytes)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    if !text.contains(&format!("sha256 = \"{OBLIQUE_SHA256}\""))
        || !text.contains(&format!("byte_count = {OBLIQUE_BYTE_COUNT}"))
        || !text.contains(&format!("record_count = {OBLIQUE_RECORD_COUNT}"))
        || !text.contains(&format!("lensisku_sha256 = \"{DICTIONARY_SHA256}\""))
    {
        return Err(BundleError::new(
            BundleErrorKind::Digest,
            "oblique provenance sidecar does not pin the reviewed source envelope",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| !rows.is_empty()) || ret.is_err())]
fn audit_dictionary(bytes: &[u8]) -> Result<BTreeMap<String, DictionaryIdentity>, BundleError> {
    if sha256_hex(bytes) != DICTIONARY_SHA256 {
        return Err(BundleError::new(
            BundleErrorKind::Digest,
            "Lensisku snapshot digest differs",
        ));
    }
    let entries: Vec<Value> = serde_json::from_slice(bytes).map_err(|error| {
        BundleError::new(BundleErrorKind::Parse, format!("parse Lensisku: {error}"))
    })?;
    let mut identities = BTreeMap::new();
    let mut collisions = BTreeSet::new();
    for entry in entries {
        let Some(word) = entry.get("word").and_then(Value::as_str) else {
            return Err(BundleError::new(
                BundleErrorKind::Parse,
                "Lensisku entry has no word",
            ));
        };
        let Some(word_type) = entry.get("word_type").and_then(Value::as_str) else {
            return Err(BundleError::new(
                BundleErrorKind::Parse,
                format!("Lensisku {word} has no word_type"),
            ));
        };
        let Some(definition_id) = entry.get("definition_id").and_then(Value::as_u64) else {
            return Err(BundleError::new(
                BundleErrorKind::Parse,
                format!("Lensisku {word} has no definition_id"),
            ));
        };
        if ["definition", "definition_id", "notes", "word", "word_type"]
            .iter()
            .any(|key| entry.get(key).is_none())
        {
            // Such an entry cannot satisfy the supported-domain identity
            // contract. Keep scanning the full snapshot; an attempted use of
            // this spelling will fail the exact identity lookup below.
            continue;
        }
        let fingerprint = lexical_fingerprint(&entry)?;
        if collisions.contains(word) {
            continue;
        }
        let identity = DictionaryIdentity {
            definition_id,
            word_type: word_type.to_owned(),
            fingerprint,
        };
        if identities.insert(word.to_owned(), identity).is_some() {
            // Normalized dictionary collisions are legitimate globally, but an
            // exact root used by this bundle must resolve uniquely. Preserve a
            // sentinel which the supported-domain lookup rejects below.
            identities.remove(word);
            collisions.insert(word.to_owned());
        }
    }
    Ok(identities)
}

#[requires(entry.is_object())]
#[ensures(ret.as_ref().is_ok_and(|digest| is_digest(digest)) || ret.is_err())]
fn lexical_fingerprint(entry: &Value) -> Result<String, BundleError> {
    let object = entry.as_object().expect("precondition requires object");
    let mut canonical = Map::new();
    for key in ["definition", "definition_id", "notes", "word", "word_type"] {
        canonical.insert(
            key.to_owned(),
            object.get(key).cloned().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Parse,
                    format!("Lensisku fingerprint field absent: {key}"),
                )
            })?,
        );
    }
    let mut bytes = canonical_json(&Value::Object(canonical))?.into_bytes();
    bytes.push(b'\n');
    Ok(sha256_hex(&bytes))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn audit_dictionary_metadata(bytes: &[u8]) -> Result<(), BundleError> {
    require_nfc_utf8(DICTIONARY_METADATA_PATH, bytes)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|error| BundleError::new(BundleErrorKind::ByteDomain, error.to_string()))?;
    if !text.contains(&format!("sha256 = \"{DICTIONARY_SHA256}\"")) {
        return Err(BundleError::new(
            BundleErrorKind::Digest,
            "Lensisku metadata digest differs",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == 6) || ret.is_err())]
fn build_source_artifact_rows(
    root: &Path,
    dictionary: &[u8],
    registry_source: &[u8],
) -> Result<Vec<SourceArtifactRow>, BundleError> {
    let inventory = read_relative(root, COMPLETENESS_INVENTORY_PATH)?;
    let oblique = read_relative(root, OBLIQUE_PATH)?;
    let spec = read_relative(root, SPEC_PATH)?;
    let registry_provenance = read_relative(root, SOURCE_PROVENANCE_PATH)?;
    let mut rows = vec![
        new!(SourceArtifactRow {
            source_id: "jbotci-semantic-surface".to_owned(),
            source_kind: "versioned-rust-model-inventory".to_owned(),
            immutable_revision: sha256_hex(&inventory),
            canonical_locator: "jbotci:crates/jbotci-semantics/src/completeness/inventory.rs"
                .to_owned(),
            artifact_digest: sha256_hex(&inventory),
        }),
        new!(SourceArtifactRow {
            source_id: "lensisku-en-2026-07-27".to_owned(),
            source_kind: "versioned-semantic-dictionary".to_owned(),
            immutable_revision: "2026-07-27T07:10:51.776063Z".to_owned(),
            canonical_locator: "jbotci:crates/jbotci-dictionary-data/data/dictionary-en.json"
                .to_owned(),
            artifact_digest: sha256_hex(dictionary),
        }),
        new!(SourceArtifactRow {
            source_id: "lojban-org-oblique-keywords-2005".to_owned(),
            source_kind: "official-numbered-place-key-table".to_owned(),
            immutable_revision: "Tue, 28 Jun 2005 04:50:44 GMT".to_owned(),
            canonical_locator:
                "https://www.lojban.org/static/publications/wordlists/oblique_keywords.txt"
                    .to_owned(),
            artifact_digest: sha256_hex(&oblique),
        }),
        new!(SourceArtifactRow {
            source_id: "smusni-v0-spec".to_owned(),
            source_kind: "frozen-normative-specification".to_owned(),
            immutable_revision: SMUSNI_SOURCE_REVISION.to_owned(),
            canonical_locator: "jbotci:docs/smusni/spec.md".to_owned(),
            artifact_digest: sha256_hex(&spec),
        }),
        new!(SourceArtifactRow {
            source_id: "smusni-v0-registry-provenance".to_owned(),
            source_kind: "reviewed-v0-provenance-sidecar".to_owned(),
            immutable_revision: sha256_hex(&registry_provenance),
            canonical_locator: "jbotci:crates/jbotci-semantics/data/smusni-v0/sources/registry-source.provenance.toml"
                .to_owned(),
            artifact_digest: sha256_hex(&registry_provenance),
        }),
        new!(SourceArtifactRow {
            source_id: "smusni-v0-registry-source".to_owned(),
            source_kind: "curated-v0-registry-source".to_owned(),
            immutable_revision: sha256_hex(&registry_source),
            canonical_locator: "jbotci:crates/jbotci-semantics/data/smusni-v0/sources/registry-source.toml"
                .to_owned(),
            artifact_digest: sha256_hex(&registry_source),
        }),
    ];
    rows.sort_by(|left, right| scalar_cmp(&left.source_id, &right.source_id));
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|(rows, _)| rows.len() == sources.len()) || ret.is_err())]
fn build_lexical_rows(
    sources: &[LexicalSource],
    oblique_roots: &BTreeMap<String, BTreeSet<u64>>,
    dictionary: &BTreeMap<String, DictionaryIdentity>,
    registry_source_digest: &str,
) -> Result<(Vec<LexicalRow>, Vec<EvidenceRow>), BundleError> {
    if sources.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "supported lexical domain cannot be empty",
        ));
    }
    let keys = sources
        .iter()
        .map(|row| row.root.as_str())
        .collect::<Vec<_>>();
    require_sorted_unique(&keys, "lexical source roots")?;
    let mut rows = Vec::with_capacity(sources.len());
    let mut evidence = Vec::with_capacity(sources.len());
    for source in sources {
        if !is_lexical_root(&source.root)
            || source.slot_types.is_empty()
            || source.slot_types.len() != source.slot_close_policies.len()
        {
            return Err(BundleError::new(
                BundleErrorKind::ClosedValue,
                format!("invalid supported lexical row {}", source.root),
            ));
        }
        let evidence_id = format!("smusni.lexical.{}", source.root);
        let (dictionary_source_id, dictionary_entry_id, locator) =
            if source.evidence_source == "dictionary" {
                let ordinals = oblique_roots.get(&source.root).ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::ForeignKey,
                        format!("{} has no official numbered place keys", source.root),
                    )
                })?;
                let expected = (1..=source.slot_types.len() as u64).collect::<BTreeSet<_>>();
                if ordinals != &expected {
                    return Err(BundleError::new(
                        BundleErrorKind::Drift,
                        format!("{} official arity differs from curated row", source.root),
                    ));
                }
                let identity = dictionary.get(&source.root).ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::ForeignKey,
                        format!("{} has no unique Lensisku identity", source.root),
                    )
                })?;
                if identity.word_type != source.word_class {
                    return Err(BundleError::new(
                        BundleErrorKind::Drift,
                        format!("{} word class differs from Lensisku", source.root),
                    ));
                }
                (
                    "lensisku-en-2026-07-27".to_owned(),
                    identity.definition_id.to_string(),
                    format!(
                        "{}1..{}{}; lensisku definition {}; fingerprint sha256:{}",
                        source.root,
                        source.root,
                        source.slot_types.len(),
                        identity.definition_id,
                        identity.fingerprint
                    ),
                )
            } else if source.evidence_source == "spec" {
                let entry = source.dictionary_entry_id.clone().ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Evidence,
                        format!(
                            "{} spec-curated row needs an explicit entry id",
                            source.root
                        ),
                    )
                })?;
                (
                    "smusni-v0-spec".to_owned(),
                    entry,
                    "section 8.4 set/group gadri lexical relation contract".to_owned(),
                )
            } else {
                return Err(BundleError::new(
                    BundleErrorKind::ClosedValue,
                    format!("unknown lexical evidence source {}", source.evidence_source),
                ));
            };

        let mut numbered = Vec::with_capacity(source.slot_types.len());
        for (index, (accepted, close_policy)) in source
            .slot_types
            .iter()
            .zip(&source.slot_close_policies)
            .enumerate()
        {
            let accepted = canonical_type_schema(accepted)?;
            if *close_policy == ClosePolicy::LocalExistential
                || (*close_policy == ClosePolicy::Contextual
                    && !matches!(
                        TypeExpr::parse(&parse_document(&accepted).expect("canonical type parses")),
                        Ok(TypeExpr::Referents(_))
                    ))
            {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    format!(
                        "{} x{} has an incompatible explicit close policy",
                        source.root,
                        index + 1
                    ),
                ));
            }
            numbered.push(new!(SlotRow {
                label: new!(SlotLabel::Numbered((index + 1) as u64)),
                accepted_type_schema: accepted,
                close_policy: *close_policy,
                lexical_provenance: format!("{}:x{}", source.root, index + 1),
                evidence_id: evidence_id.clone(),
            }));
        }
        let event = match source.event_slot {
            EventSlotPolicy::Absent => None,
            EventSlotPolicy::LocalExistential => Some(new!(SlotRow {
                label: new!(SlotLabel::Eventuality("Eventuality".to_owned())),
                accepted_type_schema: "(Referents Eventuality)".to_owned(),
                close_policy: ClosePolicy::LocalExistential,
                lexical_provenance: format!("event-license:{}", source.root),
                evidence_id: evidence_id.clone(),
            })),
        };
        rows.push(new!(LexicalRow {
            root: source.root.clone(),
            normalized_root: source.root.clone(),
            word_class: source.word_class.clone(),
            dictionary_source_id,
            dictionary_entry_id,
            ordered_numbered_slot_rows: numbered,
            optional_event_slot_row: event,
        }));
        evidence.push(new!(EvidenceRow {
            evidence_id,
            source_id: "smusni-v0-registry-source".to_owned(),
            exact_locator: locator,
            cited_content_digest: registry_source_digest.to_owned(),
            adjudication_note: "Numbered identity/arity is source-backed; accepted types, close policies, and event licensing are explicit v0 curation and are never parsed from prose."
                .to_owned(),
        }));
    }
    Ok((rows, evidence))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == SCOPE_POLICY_ROW_COUNT) || ret.is_err())]
fn build_scope_policy_rows(
    sources: &[ScopePolicySource],
    lexical: &[LexicalRow],
    provenance_digest: &str,
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<ScopePolicyRow>, BundleError> {
    if sources.len() != SCOPE_POLICY_ROW_COUNT {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "v0 requires the seven migrated rows plus the frozen kakne x2 row",
        ));
    }
    let keys = sources
        .iter()
        .map(|row| (row.normalized_root.as_str(), row.original_ordinal))
        .collect::<Vec<_>>();
    require_tuple_sorted_unique(&keys, "scope-policy keys")?;
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        let lexical_row = lexical
            .iter()
            .find(|row| row.normalized_root == source.normalized_root)
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!("policy root {} is unsupported", source.normalized_root),
                )
            })?;
        if source.original_ordinal == 0
            || source.original_ordinal as usize > lexical_row.ordered_numbered_slot_rows.len()
        {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "policy place {} x{} is outside its lexical row",
                    source.normalized_root, source.original_ordinal
                ),
            ));
        }
        let evidence_id = format!(
            "smusni.scope.{}.x{}",
            source.normalized_root, source.original_ordinal
        );
        evidence.push(new!(EvidenceRow {
            evidence_id: evidence_id.clone(),
            source_id: "smusni-v0-registry-provenance".to_owned(),
            exact_locator: format!(
                "scope_policy normalized_root={} original_ordinal={}",
                source.normalized_root, source.original_ordinal
            ),
            cited_content_digest: provenance_digest.to_owned(),
            adjudication_note: format!(
                "The reviewed v0 policy is {:?}; runtime value family is graph data, not policy identity.",
                source.scope_policy
            ),
        }));
        rows.push(new!(ScopePolicyRow {
            normalized_root: source.normalized_root.clone(),
            original_ordinal: source.original_ordinal,
            scope_policy: source.scope_policy,
            evidence_id,
        }));
    }
    if rows
        .iter()
        .filter(|row| row.scope_policy == ScopePolicy::Extensional)
        .count()
        != EXTENSIONAL_SCOPE_POLICY_COUNT
        || rows
            .iter()
            .filter(|row| row.scope_policy == ScopePolicy::Intensional)
            .count()
            != INTENSIONAL_SCOPE_POLICY_COUNT
        || rows
            .iter()
            .any(|row| row.scope_policy == ScopePolicy::Opaque)
    {
        return Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            "v0 policy distribution must be six Extensional, two Intensional, zero Opaque",
        ));
    }
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn build_place_deletion_rows(
    sources: &[PlaceDeletionSource],
    lexical: &[LexicalRow],
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<PlaceDeletionEvidenceRow>, BundleError> {
    let mut ordered = sources.to_vec();
    ordered.sort_by(|left, right| {
        scalar_cmp(&left.expansion_owner, &right.expansion_owner)
            .then_with(|| scalar_cmp(&left.normalized_root, &right.normalized_root))
            .then(left.original_ordinal.cmp(&right.original_ordinal))
    });
    reject_duplicate(
        ordered.iter().map(|row| {
            format!(
                "{}\u{0}{}\u{0}{}",
                row.expansion_owner, row.normalized_root, row.original_ordinal
            )
        }),
        "place-deletion primary key",
    )?;
    let mut current_rows: BTreeMap<(String, String), Vec<SlotRow>> = BTreeMap::new();
    let mut rows = Vec::with_capacity(ordered.len());
    for source in ordered {
        let lexical_row = lexical
            .iter()
            .find(|row| row.normalized_root == source.normalized_root)
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!("deletion root {} is unsupported", source.normalized_root),
                )
            })?;
        let key = (
            source.expansion_owner.clone(),
            source.normalized_root.clone(),
        );
        let slots = current_rows.entry(key).or_insert_with(|| {
            let mut slots = lexical_row.ordered_numbered_slot_rows.clone();
            if let Some(event) = &lexical_row.optional_event_slot_row {
                slots.push(event.clone());
            }
            slots
        });
        let input_slots = slots.clone();
        let input_row_schema = row_schema(&input_slots)?;
        let index = slots
            .iter()
            .position(|slot| {
                matches!(slot.label.as_data(), data!(SlotLabel::Numbered(value)) if *value == source.original_ordinal)
            })
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!(
                        "{} cannot delete absent {} x{}",
                        source.expansion_owner, source.normalized_root, source.original_ordinal
                    ),
                )
            })?;
        slots.remove(index);
        let result_row_schema = row_schema(slots)?;
        validate_surviving_map(&source.surviving_slot_map, &input_slots, slots)?;
        evidence.push(spec_evidence(
            &source.evidence_id,
            "sections 4.6, 10.4, and 14.1 deletion contracts",
            &source.semantic_absence_contract,
        ));
        rows.push(new!(PlaceDeletionEvidenceRow {
            expansion_owner: source.expansion_owner,
            normalized_root: source.normalized_root,
            original_ordinal: source.original_ordinal,
            input_row_schema,
            result_row_schema,
            surviving_slot_map: source.surviving_slot_map,
            semantic_absence_contract: source.semantic_absence_contract,
            evidence_id: source.evidence_id,
        }));
    }
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == sources.len()) || ret.is_err())]
fn build_tag_reduction_rows(
    sources: &[TagReductionSource],
    lexical: &[LexicalRow],
    prelude: &[PreludeRow],
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<TagReductionRow>, BundleError> {
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        let applicability_guard =
            canonical_template(&source.applicability_guard, lexical, prelude)?;
        let typed_expansion_template =
            canonical_template(&source.typed_expansion_template, lexical, prelude)?;
        let operand_types = source
            .operand_types
            .iter()
            .map(|schema| canonical_type_schema(schema))
            .collect::<Result<Vec<_>, _>>()?;
        let resulting_type_schema = canonical_type_schema(&source.resulting_type_schema)?;
        validate_typed_template(
            &applicability_guard,
            &derive_template_result(&applicability_guard)?,
            lexical,
            prelude,
        )?;
        let derived = derive_template_result(&typed_expansion_template)?;
        if canonical_type_for_comparison(&derived)?
            != canonical_type_for_comparison(&resulting_type_schema)?
        {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!(
                    "tag {} {} declares {resulting_type_schema} but derives {derived}",
                    source.source_family, source.source_member
                ),
            ));
        }
        validate_typed_template(
            &typed_expansion_template,
            &resulting_type_schema,
            lexical,
            prelude,
        )?;
        validate_tag_metadata(
            &operand_types,
            &source.source_place_map,
            source.host_event_map,
            &source.required_graph_identities,
            &applicability_guard,
            &typed_expansion_template,
            lexical,
            prelude,
        )?;
        let evidence_id = format!(
            "smusni.tag.{}.{}",
            ascii_slug(&source.source_family),
            ascii_slug(&source.source_member)
        );
        evidence.push(spec_evidence(
            &evidence_id,
            "sections 10.1, 10.4, and 14.2 tag reduction contracts",
            &format!(
                "{} {} has a complete typed expansion with an explicit source-place and host-event map.",
                source.source_family, source.source_member
            ),
        ));
        rows.push(new!(TagReductionRow {
            source_family: source.source_family.clone(),
            source_member: source.source_member.clone(),
            applicability_guard,
            operand_types,
            source_place_map: source.source_place_map.clone(),
            host_event_map: source.host_event_map,
            required_graph_identities: source.required_graph_identities.clone(),
            typed_expansion_template,
            resulting_type_schema,
            evidence_id,
        }));
    }
    rows.sort_by(|left, right| {
        scalar_cmp(&left.source_family, &right.source_family)
            .then_with(|| scalar_cmp(&left.source_member, &right.source_member))
            .then_with(|| scalar_cmp(&left.applicability_guard, &right.applicability_guard))
    });
    reject_duplicate(
        rows.iter().map(|row| {
            format!(
                "{}\u{0}{}\u{0}{}",
                row.source_family, row.source_member, row.applicability_guard
            )
        }),
        "tag-reduction primary key",
    )?;
    Ok(rows)
}

#[requires(!operand_types.is_empty() && !source_place_map.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_tag_metadata(
    operand_types: &[String],
    source_place_map: &[String],
    host_event_map: HostEventMap,
    required_graph_identities: &[String],
    applicability_guard: &str,
    typed_expansion_template: &str,
    lexical: &[LexicalRow],
    prelude: &[PreludeRow],
) -> Result<(), BundleError> {
    let expansion = parse_document(typed_expansion_template).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse tag expansion metadata: {error}"),
        )
    })?;
    let guard = parse_document(applicability_guard).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse tag guard metadata: {error}"),
        )
    })?;
    let expansion_holes = collect_template_holes(&expansion)?;
    let guard_holes = collect_template_holes(&guard)?;
    let canonical_operand_types = operand_types
        .iter()
        .map(|operand| canonical_type_schema(operand))
        .collect::<Result<Vec<_>, _>>()?;
    let expansion_types = expansion_holes
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect::<Vec<_>>();
    if canonical_operand_types != expansion_types {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            format!(
                "tag operand types {canonical_operand_types:?} differ from ordered expansion holes {expansion_types:?}"
            ),
        ));
    }
    for (name, value_type) in guard_holes {
        if !expansion_holes
            .iter()
            .any(|(candidate, candidate_type)| candidate == &name && candidate_type == &value_type)
        {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("tag guard Hole {name} has no identical expansion operand"),
            ));
        }
    }

    let mut structural_targets = expansion_holes
        .iter()
        .map(|(name, value_type)| {
            (
                name.clone(),
                new!(ResolvedTagTarget::Hole {
                    name: name.clone(),
                    type_schema: value_type.clone(),
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    collect_lexical_tag_targets(&expansion, lexical, &mut structural_targets)?;
    collect_prelude_tag_targets(&expansion, prelude, &mut structural_targets)?;
    let lexically_placed_holes = structural_targets
        .iter()
        .filter(|(target, _)| target.contains("-x") || target.ends_with("-event"))
        .filter_map(|(_, target)| match target.as_data() {
            data!(ResolvedTagTarget::Hole { name, .. }) => Some(name.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for hole in lexically_placed_holes {
        structural_targets.remove(&hole);
    }

    let mappings = source_place_map
        .iter()
        .map(|mapping| parse_tag_source_place_mapping(mapping))
        .collect::<Result<Vec<_>, _>>()?;
    let source_identities = mappings
        .iter()
        .map(|mapping| mapping.source)
        .collect::<BTreeSet<_>>();
    let target_identities = mappings
        .iter()
        .map(|mapping| mapping.target.as_str())
        .collect::<BTreeSet<_>>();
    if source_identities.len() != mappings.len() || target_identities.len() != mappings.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "tag source-place map must be bijective over declared source and target identities",
        ));
    }
    let mut mapped_holes = BTreeSet::new();
    for mapping in &mappings {
        let target = structural_targets.get(&mapping.target).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "tag source identity {:?} maps to absent structural target {}",
                    mapping.source, mapping.target
                ),
            )
        })?;
        if !tag_source_identity_matches_target(mapping.source, &mapping.target, target) {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "tag source identity {:?} contradicts structural target {target:?}",
                    mapping.source
                ),
            ));
        }
        if let data!(ResolvedTagTarget::Hole { name, .. }) = target.as_data() {
            mapped_holes.insert(name.clone());
        }
    }
    let required_holes = expansion_holes
        .iter()
        .map(|(name, _)| name)
        .filter(|name| name.as_str() != "host")
        .cloned()
        .collect::<BTreeSet<_>>();
    if mapped_holes != required_holes {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            format!(
                "tag source-place map covers Holes {mapped_holes:?}, not every non-host operand {required_holes:?}"
            ),
        ));
    }

    let derived_host_event_map = if source_identities.contains(&TagSourceIdentity::HostEvent) {
        HostEventMap::Shared
    } else if source_identities.contains(&TagSourceIdentity::HostReconstruction) {
        HostEventMap::PossibleOnly
    } else {
        HostEventMap::Local
    };
    if host_event_map != derived_host_event_map {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            format!(
                "tag host-event map {host_event_map:?} differs from structural map {derived_host_event_map:?}"
            ),
        ));
    }

    let declared_required = required_graph_identities
        .iter()
        .map(|identity| parse_required_graph_identity(identity))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if declared_required.len() != required_graph_identities.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "tag required graph identities contain a duplicate",
        ));
    }
    let mut derived_required = BTreeSet::new();
    if source_identities.contains(&TagSourceIdentity::HostEvent) {
        derived_required.insert(RequiredGraphIdentity::HostEvent);
    }
    if source_identities.contains(&TagSourceIdentity::HostX1) {
        derived_required.insert(RequiredGraphIdentity::HostX1);
    }
    if source_identities.contains(&TagSourceIdentity::UtteranceNow) {
        derived_required.insert(RequiredGraphIdentity::UtteranceGround);
    }
    if source_identities.contains(&TagSourceIdentity::RelativeHead) {
        derived_required.insert(RequiredGraphIdentity::RelativeHead);
    }
    if declared_required != derived_required {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            format!(
                "tag required graph identities {declared_required:?} differ from structurally witnessed identities {derived_required:?}"
            ),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_template_holes(datum: &Datum) -> Result<Vec<(String, String)>, BundleError> {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn visit(datum: &Datum, holes: &mut Vec<(String, String)>) -> Result<(), BundleError> {
        if datum.form_head() == Some("Hole") {
            let items = datum.as_list().expect("a form head belongs to a list");
            if items.len() != 3 {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    "tag Hole must contain name and type",
                ));
            }
            let name = items[1].as_string().ok_or_else(|| {
                BundleError::new(BundleErrorKind::Template, "tag Hole name must be a string")
            })?;
            let value_type = canonical_type_schema(&canonical_datum(&items[2]))?;
            if holes.iter().any(|(candidate, _)| candidate == name) {
                return Err(BundleError::new(
                    BundleErrorKind::DuplicatePrimaryKey,
                    format!("tag Hole {name} occurs more than once"),
                ));
            }
            holes.push((name.to_owned(), value_type));
            return Ok(());
        }
        if let Some(items) = datum.as_list() {
            for item in items {
                visit(item, holes)?;
            }
        }
        Ok(())
    }

    let mut holes = Vec::new();
    visit(datum, &mut holes)?;
    Ok(holes)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_lexical_tag_targets(
    datum: &Datum,
    lexical: &[LexicalRow],
    targets: &mut BTreeMap<String, ResolvedTagTarget>,
) -> Result<(), BundleError> {
    if let Some(root) = datum.form_head()
        && let Some(row) = lexical.iter().find(|row| row.normalized_root == root)
    {
        collect_one_lexical_tag_application(datum, row, targets)?;
    }
    if let Some(items) = datum.as_list() {
        for item in items {
            collect_lexical_tag_targets(item, lexical, targets)?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_prelude_tag_targets(
    datum: &Datum,
    prelude: &[PreludeRow],
    targets: &mut BTreeMap<String, ResolvedTagTarget>,
) -> Result<(), BundleError> {
    if let Some(name) = datum.form_head()
        && let Some(row) = prelude.iter().find(|row| row.name == name)
    {
        let items = datum.as_list().expect("a form head belongs to a list");
        let roles = prelude_parameter_roles(row)?;
        if roles.len() != items.len() - 1 {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("tag application of prelude {name} has the wrong role arity"),
            ));
        }
        for (role, argument) in roles.into_iter().zip(&items[1..]) {
            targets.insert(role, resolve_tag_target_value(argument)?);
        }
    }
    if let Some(items) = datum.as_list() {
        for item in items {
            collect_prelude_tag_targets(item, prelude, targets)?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|roles| !roles.is_empty()) || ret.is_err())]
fn prelude_parameter_roles(row: &PreludeRow) -> Result<Vec<String>, BundleError> {
    let definition = parse_document(&row.canonical_definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse prelude {} parameter roles: {error}", row.name),
        )
    })?;
    let items = definition.as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("callable prelude {} has no lambda definition", row.name),
        )
    })?;
    if items.len() != 3 || items[0].as_atom() != Some("λ") {
        return Err(BundleError::new(
            BundleErrorKind::Template,
            format!("callable prelude {} has no lambda definition", row.name),
        ));
    }
    let declarations = items[1].as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("prelude {} lambda declarations are not a list", row.name),
        )
    })?;
    declarations
        .iter()
        .map(|declaration| {
            let declaration = declaration.as_list().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Template,
                    format!("prelude {} parameter declaration is not a pair", row.name),
                )
            })?;
            if declaration.len() != 2 {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    format!("prelude {} parameter declaration is not a pair", row.name),
                ));
            }
            declaration[0]
                .as_atom()
                .and_then(|name| name.strip_prefix('$'))
                .filter(|name| is_summary_parameter_name(name))
                .map(str::to_owned)
                .ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Template,
                        format!("prelude {} parameter role is invalid", row.name),
                    )
                })
        })
        .collect()
}

#[requires(datum.form_head() == Some(row.normalized_root.as_str()))]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_one_lexical_tag_application(
    datum: &Datum,
    row: &LexicalRow,
    targets: &mut BTreeMap<String, ResolvedTagTarget>,
) -> Result<(), BundleError> {
    let items = datum.as_list().expect("the precondition requires a form");
    let mut remaining = row
        .ordered_numbered_slot_rows
        .iter()
        .map(|slot| match slot.label.as_data() {
            data!(SlotLabel::Numbered(value)) => value.to_string(),
            data!(SlotLabel::Eventuality(_)) => unreachable!("numbered row contains no event"),
        })
        .collect::<Vec<_>>();
    if row.optional_event_slot_row.is_some() {
        remaining.push("Eventuality".to_owned());
    }
    let ordered_labels = remaining.clone();
    let mut cursor_after: Option<String> = None;
    let mut index = 1;
    while index < items.len() {
        let (label, value_index, advances_cursor) = if let Some(label) = items[index]
            .as_atom()
            .and_then(|atom| atom.strip_prefix(':'))
        {
            if index + 1 >= items.len() {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    "tag lexical label has no filler",
                ));
            }
            let label = if label == "Eventuality" {
                "Eventuality".to_owned()
            } else {
                PositiveInteger::try_new(label).map_err(|error| {
                    BundleError::new(BundleErrorKind::Template, error.to_string())
                })?;
                label.to_owned()
            };
            (label.clone(), index + 1, label != "Eventuality")
        } else {
            let after_position = cursor_after
                .as_ref()
                .and_then(|cursor| ordered_labels.iter().position(|label| label == cursor));
            let label = remaining
                .iter()
                .find(|label| {
                    after_position.is_none_or(|after| {
                        ordered_labels
                            .iter()
                            .position(|candidate| candidate == *label)
                            .is_some_and(|position| position > after)
                    })
                })
                .cloned()
                .ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Template,
                        format!("tag overfills lexical relation {}", row.normalized_root),
                    )
                })?;
            (label, index, true)
        };
        let slot_index = remaining
            .iter()
            .position(|candidate| candidate == &label)
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!(
                        "tag fills absent lexical place {} {label}",
                        row.normalized_root
                    ),
                )
            })?;
        remaining.remove(slot_index);
        let target_name = if label == "Eventuality" {
            format!("{}-event", row.normalized_root)
        } else {
            format!("{}-x{label}", row.normalized_root)
        };
        let target = resolve_tag_target_value(&items[value_index])?;
        if targets.insert(target_name.clone(), target).is_some() {
            return Err(BundleError::new(
                BundleErrorKind::DuplicatePrimaryKey,
                format!("tag expansion fills structural target {target_name} more than once"),
            ));
        }
        if advances_cursor {
            cursor_after = Some(label);
        }
        index = value_index + 1;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn resolve_tag_target_value(datum: &Datum) -> Result<ResolvedTagTarget, BundleError> {
    if datum.form_head() == Some("Hole") {
        let items = datum.as_list().expect("a form head belongs to a list");
        let name = items[1].as_string().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Template,
                "mapped Hole name must be a string",
            )
        })?;
        return Ok(new!(ResolvedTagTarget::Hole {
            name: name.to_owned(),
            type_schema: canonical_type_schema(&canonical_datum(&items[2]))?,
        }));
    }
    if let Some(spelling) = datum.as_atom() {
        return Ok(new!(ResolvedTagTarget::Constant {
            spelling: spelling.to_owned(),
        }));
    }
    Err(BundleError::new(
        BundleErrorKind::ForeignKey,
        "tag source-place map target must resolve to one typed Hole or closed constant",
    ))
}

#[requires(!text.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_tag_source_place_mapping(text: &str) -> Result<TagSourcePlaceMapping, BundleError> {
    let (source, target) = text.split_once("->").ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Parse,
            format!("bad tag source-place map {text}"),
        )
    })?;
    if target.is_empty() || target.contains("->") {
        return Err(BundleError::new(
            BundleErrorKind::Parse,
            format!("bad tag source-place target {target:?}"),
        ));
    }
    let source = match source {
        "tag-sumti" => TagSourceIdentity::TagSumti,
        "host-event" => TagSourceIdentity::HostEvent,
        "utterance-now" => TagSourceIdentity::UtteranceNow,
        "host-x1" => TagSourceIdentity::HostX1,
        "ri'u" => TagSourceIdentity::RightwardDisplacement,
        "host-reconstruction" => TagSourceIdentity::HostReconstruction,
        "speaker" => TagSourceIdentity::Speaker,
        "relative-head" => TagSourceIdentity::RelativeHead,
        "clause" => TagSourceIdentity::Clause,
        _ => {
            return Err(BundleError::new(
                BundleErrorKind::ClosedValue,
                format!("unknown tag source-place identity {source:?}"),
            ));
        }
    };
    Ok(new!(TagSourcePlaceMapping {
        source,
        target: target.to_owned(),
    }))
}

#[requires(!text.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_required_graph_identity(text: &str) -> Result<RequiredGraphIdentity, BundleError> {
    match text {
        "host-event" => Ok(RequiredGraphIdentity::HostEvent),
        "host-x1" => Ok(RequiredGraphIdentity::HostX1),
        "utterance-ground" => Ok(RequiredGraphIdentity::UtteranceGround),
        "relative-head" => Ok(RequiredGraphIdentity::RelativeHead),
        _ => Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            format!("unknown required graph identity {text:?}"),
        )),
    }
}

#[requires(true)]
#[ensures(true)]
fn tag_source_identity_matches_target(
    source: TagSourceIdentity,
    target_name: &str,
    target: &ResolvedTagTarget,
) -> bool {
    match (source, target.as_data()) {
        (TagSourceIdentity::TagSumti, data!(ResolvedTagTarget::Hole { name, .. })) => {
            matches!(target_name, "pilno-x1" | "pilno-x2") && name == "filler"
        }
        (TagSourceIdentity::HostEvent, data!(ResolvedTagTarget::Hole { name, type_schema })) => {
            matches!(target_name, "pilno-event" | "purci-x1" | "motion")
                && matches!(name.as_str(), "event" | "motion")
                && matches!(
                    type_schema.as_str(),
                    "Eventuality" | "(Referents Eventuality)"
                )
        }
        (TagSourceIdentity::UtteranceNow, data!(ResolvedTagTarget::Constant { spelling })) => {
            target_name == "purci-x2" && spelling == "Now"
        }
        (TagSourceIdentity::HostX1, data!(ResolvedTagTarget::Hole { name, .. })) => {
            matches!(target_name, "mover" | "bearer") && name == target_name
        }
        (TagSourceIdentity::RightwardDisplacement, data!(ResolvedTagTarget::Hole { name, .. })) => {
            target_name == "displacement" && name == "displacement"
        }
        (TagSourceIdentity::HostReconstruction, data!(ResolvedTagTarget::Hole { name, .. })) => {
            target_name == "property" && name == "property"
        }
        (TagSourceIdentity::Speaker, data!(ResolvedTagTarget::Hole { name, .. })) => {
            target_name == "describer" && name == "describer"
        }
        (TagSourceIdentity::RelativeHead, data!(ResolvedTagTarget::Hole { name, .. })) => {
            target_name == "described" && name == "described"
        }
        (TagSourceIdentity::Clause, data!(ResolvedTagTarget::Hole { name, .. })) => {
            target_name == "property" && name == "property"
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == sources.len()) || ret.is_err())]
fn build_relation_former_rows(
    sources: &[RelationFormerSource],
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<RelationFormerReductionRow>, BundleError> {
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        let applicability_guard = canonical_template_without_registry(&source.applicability_guard)?;
        let operand_row_schemas = source
            .operand_row_schemas
            .iter()
            .map(|schema| canonical_row_schema(schema))
            .collect::<Result<Vec<_>, _>>()?;
        let result_row_schema = canonical_row_schema(&source.result_row_schema)?;
        let typed_link_or_expansion_contract =
            canonical_template_without_registry(&source.typed_link_or_expansion_contract)?;
        validate_total_provenance_map(
            &source.total_provenance_map,
            &operand_row_schemas,
            &result_row_schema,
        )?;
        let operand_type = format!("(PredTerm {})", operand_row_schemas[0]);
        let result_type = format!("(PredTerm {result_row_schema})");
        validate_typed_template(&applicability_guard, &operand_type, &[], &[])?;
        validate_typed_template(&typed_link_or_expansion_contract, &result_type, &[], &[])?;
        let evidence_id = format!(
            "smusni.relation-former.{}.{}",
            ascii_slug(&source.former_kind),
            ascii_slug(&source.source_owner)
        );
        evidence.push(spec_evidence(
            &evidence_id,
            "sections 4.6 and 14.2 relation-former contracts",
            "The place permutation is total over original labels and retains the distinguished event label.",
        ));
        rows.push(new!(RelationFormerReductionRow {
            former_kind: source.former_kind.clone(),
            source_owner: source.source_owner.clone(),
            applicability_guard,
            operand_row_schemas,
            result_row_schema,
            total_provenance_map: source.total_provenance_map.clone(),
            typed_link_or_expansion_contract,
            evidence_id,
        }));
    }
    rows.sort_by(|left, right| {
        scalar_cmp(&left.former_kind, &right.former_kind)
            .then_with(|| scalar_cmp(&left.source_owner, &right.source_owner))
            .then_with(|| scalar_cmp(&left.applicability_guard, &right.applicability_guard))
    });
    reject_duplicate(
        rows.iter().map(|row| {
            format!(
                "{}\u{0}{}\u{0}{}",
                row.former_kind, row.source_owner, row.applicability_guard
            )
        }),
        "relation-former primary key",
    )?;
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == sources.len()) || ret.is_err())]
fn build_generated_relation_rows(
    sources: &[GeneratedRelationSource],
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<GeneratedRelationRow>, BundleError> {
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        if !is_pascal_case(&source.pascal_case_name) || source.irreducibility_reason.is_empty() {
            return Err(BundleError::new(
                BundleErrorKind::ClosedValue,
                "generated relations need PascalCase identity and irreducibility evidence",
            ));
        }
        let complete_signature = canonical_type_schema(&source.complete_signature)?;
        let derived_summary = derive_generated_relation_summary(&complete_signature)?;
        let (context_effect_summary, stability_summary) =
            generated_relation_summary_fields(&derived_summary)?;
        let evidence_id = format!(
            "smusni.generated-relation.{}.{}",
            ascii_slug(&source.family),
            ascii_slug(&source.pascal_case_name)
        );
        evidence.push(spec_evidence(
            &evidence_id,
            "sections 7.3 and 14.2 generated relation contract",
            &source.irreducibility_reason,
        ));
        rows.push(new!(GeneratedRelationRow {
            family: source.family.clone(),
            pascal_case_name: source.pascal_case_name.clone(),
            complete_signature,
            context_effect_summary,
            stability_summary,
            irreducibility_reason: source.irreducibility_reason.clone(),
            evidence_id,
        }));
    }
    rows.sort_by(|left, right| {
        scalar_cmp(&left.family, &right.family)
            .then_with(|| scalar_cmp(&left.pascal_case_name, &right.pascal_case_name))
    });
    reject_duplicate(
        rows.iter()
            .map(|row| format!("{}\u{0}{}", row.family, row.pascal_case_name)),
        "generated-relation primary key",
    )?;
    Ok(rows)
}

#[requires(!signature.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn derive_generated_relation_summary(signature: &str) -> Result<DynamicSummary, BundleError> {
    let datum = parse_document(signature).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Summary,
            format!("parse generated relation signature: {error}"),
        )
    })?;
    let items = datum.as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Summary,
            "generated relation signature must be an Fn type",
        )
    })?;
    if items.len() != 3 || items[0].as_atom() != Some("Fn") || items[2].as_atom() != Some("Content")
    {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            "generated relation must be a complete callable returning Content",
        ));
    }
    let parameters = items[1].as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Summary,
            "generated relation Fn parameters must be a list",
        )
    })?;
    if parameters.is_empty()
        || parameters
            .iter()
            .any(generated_operand_runs_dynamic_content)
    {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            "generated relation operands must be inert first-class values",
        ));
    }
    Ok(new!(DynamicSummary {
        context_flow: DynamicContextFlow::Identity,
        parameter_evaluations: Vec::new(),
        ordered_effects: Vec::new(),
        stability: DynamicStability::SiteStableWithinPerformance,
    }))
}

#[requires(true)]
#[ensures(true)]
fn generated_operand_runs_dynamic_content(parameter: &Datum) -> bool {
    matches!(
        parameter.as_atom(),
        Some("Content" | "Discourse" | "Performable" | "TranscriptEntry")
    ) || matches!(parameter.form_head(), Some("RefComp"))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn generated_relation_summary_fields(
    summary: &DynamicSummary,
) -> Result<(ContextEffectSummary, String), BundleError> {
    if summary.context_flow != DynamicContextFlow::Identity
        || !summary.parameter_evaluations.is_empty()
        || !summary.ordered_effects.is_empty()
        || summary.stability != DynamicStability::SiteStableWithinPerformance
    {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            "v0 generated relation does not derive the registered inert predicate summary",
        ));
    }
    Ok((
        new!(ContextEffectSummary {
            context: "identity".to_owned(),
            effects: Vec::new(),
        }),
        "site-stable-within-performance".to_owned(),
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == sources.len()) || ret.is_err())]
fn build_scale_literal_rows(
    sources: &[ScaleLiteralSource],
    evidence: &mut Vec<EvidenceRow>,
) -> Result<Vec<ScaleLiteralRow>, BundleError> {
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        if source.source_members.is_empty() {
            return Err(BundleError::new(
                BundleErrorKind::Evidence,
                "scale literals need at least one source member",
            ));
        }
        let raw_value_type = canonical_type_schema(&source.raw_value_type)?;
        let evidence_id = format!("smusni.scale.{}", ascii_slug(&source.pascal_case_name));
        evidence.push(spec_evidence(
            &evidence_id,
            "sections 13.2 and 14.2 scale literal contract",
            "DistanceScale is the finite registered raw Scale used by the frozen Measure sample.",
        ));
        rows.push(new!(ScaleLiteralRow {
            pascal_case_name: source.pascal_case_name.clone(),
            raw_value_type,
            source_members: source.source_members.clone(),
            evidence_id,
        }));
    }
    rows.sort_by(|left, right| scalar_cmp(&left.pascal_case_name, &right.pascal_case_name));
    reject_duplicate(
        rows.iter().map(|row| row.pascal_case_name.as_str()),
        "scale literal primary key",
    )?;
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.len() == sources.len()) || ret.is_err())]
fn build_prelude_rows(
    sources: &[PreludeSource],
    spec: &[u8],
    deletions: &[PlaceDeletionEvidenceRow],
    lexical: &[LexicalRow],
) -> Result<Vec<PreludeRow>, BundleError> {
    let names = sources
        .iter()
        .map(|row| row.name.clone())
        .collect::<BTreeSet<_>>();
    if names.len() != sources.len() || names.len() != 20 {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "v0 prelude must contain exactly twenty unique names",
        ));
    }
    let signatures = sources
        .iter()
        .map(|source| {
            Ok((
                source.name.clone(),
                canonical_prelude_type_schema(
                    &source.complete_signature_schema,
                    &source.type_parameters,
                )?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, BundleError>>()?;
    let type_parameters = sources
        .iter()
        .map(|source| (source.name.clone(), source.type_parameters.clone()))
        .collect::<BTreeMap<_, _>>();
    let extracted_definitions = extract_prelude_definitions(spec, &signatures, &type_parameters)?;
    let mut rows = Vec::with_capacity(sources.len());
    for source in sources {
        let complete_signature_schema = canonical_prelude_type_schema(
            &source.complete_signature_schema,
            &source.type_parameters,
        )?;
        let canonical_definition = extracted_definitions
            .get(&source.name)
            .cloned()
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Template,
                    format!(
                        "the frozen spec has no prelude definition for {}",
                        source.name
                    ),
                )
            })?;
        let dependencies = prelude_dependencies(&canonical_definition, &names, &source.name)?;
        let mut declared = source.direct_dependencies.clone();
        declared.sort_by(|left, right| scalar_cmp(left, right));
        declared.dedup();
        if dependencies != declared {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "prelude {} dependencies differ: declared {:?}, derived {:?}",
                    source.name, declared, dependencies
                ),
            ));
        }
        validate_prelude_signature(
            &source.name,
            &complete_signature_schema,
            &canonical_definition,
        )?;
        validate_prelude_type_parameter_usage(
            &source.type_parameters,
            &complete_signature_schema,
            &canonical_definition,
        )?;
        for (root, ordinal) in collect_drop_places(&canonical_definition)? {
            if !deletions.iter().any(|row| {
                row.expansion_owner == source.name
                    && row.normalized_root == root
                    && row.original_ordinal == ordinal
            }) {
                return Err(BundleError::new(
                    BundleErrorKind::Evidence,
                    format!(
                        "prelude {} DropPlace {root} {ordinal} has no deletion evidence",
                        source.name
                    ),
                ));
            }
        }
        rows.push(new!(PreludeRow {
            name: source.name.clone(),
            type_parameters: source.type_parameters.clone(),
            complete_signature_schema,
            definition_digest: sha256_hex(canonical_definition.as_bytes()),
            canonical_definition,
            direct_dependencies: declared,
        }));
    }
    rows.sort_by(|left, right| scalar_cmp(&left.name, &right.name));
    validate_prelude_acyclic(&rows)?;
    let registry = StaticTypeRegistry::from_rows(lexical, &rows)?;
    for row in &rows {
        let definition = parse_document(&row.canonical_definition)
            .map_err(|error| BundleError::new(BundleErrorKind::Template, error.to_string()))?;
        let signature = StaticType::parse(
            &parse_document(&row.complete_signature_schema)
                .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?,
            true,
        )?;
        check_expression(&definition, &signature, &BTreeMap::new(), &registry).map_err(
            |error| {
                BundleError::new(
                    error.kind,
                    format!("prelude {}: {}", row.name, error.message),
                )
            },
        )?;
    }
    Ok(rows)
}

#[invariant(parameter_evaluations.iter().all(|name| is_summary_parameter_name(name)))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DynamicSummaryFacts {
    parameter_evaluations: Vec<String>,
    ordered_effects: Vec<DynamicEffect>,
    site_stable: bool,
}

impl DynamicSummaryFacts {
    #[requires(true)]
    #[ensures(ret.parameter_evaluations.is_empty() && ret.ordered_effects.is_empty() && !ret.site_stable)]
    fn inert() -> Self {
        new!(DynamicSummaryFacts {
            parameter_evaluations: Vec::new(),
            ordered_effects: Vec::new(),
            site_stable: false,
        })
    }

    #[requires(true)]
    #[ensures(ret.parameter_evaluations.len() == old(self.parameter_evaluations.len()) + old(other.parameter_evaluations.len()))]
    fn append(self, other: Self) -> Self {
        let data!(DynamicSummaryFacts {
            mut parameter_evaluations,
            mut ordered_effects,
            site_stable,
        }) = self.into_data();
        let data!(DynamicSummaryFacts {
            parameter_evaluations: other_parameters,
            ordered_effects: other_effects,
            site_stable: other_site_stable,
        }) = other.into_data();
        parameter_evaluations.extend(other_parameters);
        ordered_effects.extend(other_effects);
        new!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            site_stable: site_stable || other_site_stable,
        })
    }

    #[requires(is_summary_parameter_name(name))]
    #[ensures(ret.parameter_evaluations.len() == old(self.parameter_evaluations.len()) + 1)]
    fn with_parameter(self, name: &str) -> Self {
        let data!(DynamicSummaryFacts {
            mut parameter_evaluations,
            ordered_effects,
            site_stable,
        }) = self.into_data();
        parameter_evaluations.push(name.to_owned());
        new!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            site_stable,
        })
    }

    #[requires(true)]
    #[ensures(ret.ordered_effects.len() == old(self.ordered_effects.len()) + 1)]
    fn with_effect(self, effect: DynamicEffect) -> Self {
        let data!(DynamicSummaryFacts {
            parameter_evaluations,
            mut ordered_effects,
            site_stable,
        }) = self.into_data();
        ordered_effects.push(effect);
        new!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            site_stable,
        })
    }

    #[requires(true)]
    #[ensures(ret.site_stable)]
    fn with_site_stability(self) -> Self {
        let data!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            ..
        }) = self.into_data();
        new!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            site_stable: true,
        })
    }

    #[requires(true)]
    #[ensures(ret.parameter_evaluations == old(self.parameter_evaluations.clone()) && ret.ordered_effects == old(self.ordered_effects.clone()))]
    fn into_summary(self) -> DynamicSummary {
        let data!(DynamicSummaryFacts {
            parameter_evaluations,
            ordered_effects,
            site_stable,
        }) = self.into_data();
        let context_flow = if ordered_effects.iter().any(|effect| {
            matches!(
                effect,
                DynamicEffect::Presupposition | DynamicEffect::Supplement
            )
        }) {
            DynamicContextFlow::Projective
        } else if ordered_effects.iter().any(|effect| {
            matches!(
                effect,
                DynamicEffect::ReferenceIntroduction | DynamicEffect::Performance
            )
        }) {
            DynamicContextFlow::Updating
        } else if parameter_evaluations.is_empty() {
            DynamicContextFlow::Identity
        } else {
            DynamicContextFlow::Parameterized
        };
        let stability = if !ordered_effects.is_empty() {
            DynamicStability::Unstable
        } else if !parameter_evaluations.is_empty() {
            DynamicStability::Parameterized
        } else if site_stable {
            DynamicStability::SiteStableWithinPerformance
        } else {
            DynamicStability::Stable
        };
        new!(DynamicSummary {
            context_flow,
            parameter_evaluations,
            ordered_effects,
            stability,
        })
    }
}

#[requires(!rows.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|summaries| summaries.len() == rows.len()) || ret.is_err())]
fn derive_prelude_summaries(
    rows: &[PreludeRow],
) -> Result<BTreeMap<String, DynamicSummary>, BundleError> {
    let mut summaries = BTreeMap::new();
    let mut active = BTreeSet::new();
    for row in rows {
        derive_one_prelude_summary(&row.name, rows, &mut summaries, &mut active)?;
    }
    Ok(summaries)
}

#[requires(rows.iter().any(|row| row.name == name))]
#[ensures(ret.is_ok() || ret.is_err())]
fn derive_one_prelude_summary(
    name: &str,
    rows: &[PreludeRow],
    summaries: &mut BTreeMap<String, DynamicSummary>,
    active: &mut BTreeSet<String>,
) -> Result<(), BundleError> {
    if summaries.contains_key(name) {
        return Ok(());
    }
    if !active.insert(name.to_owned()) {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            format!("recursive prelude dynamic summary at {name}"),
        ));
    }
    let row = rows
        .iter()
        .find(|row| row.name == name)
        .expect("the precondition requires the named prelude row");
    for dependency in &row.direct_dependencies {
        derive_one_prelude_summary(dependency, rows, summaries, active)?;
    }
    let datum = parse_document(&row.canonical_definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Summary,
            format!("parse prelude {name} for dynamic derivation: {error}"),
        )
    })?;
    let summary = summarize_callable(&datum, summaries)?.into_summary();
    active.remove(name);
    summaries.insert(name.to_owned(), summary);
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn summarize_callable(
    datum: &Datum,
    prelude: &BTreeMap<String, DynamicSummary>,
) -> Result<DynamicSummaryFacts, BundleError> {
    let mut body = datum;
    while body.form_head() == Some("λ") {
        let items = body.as_list().expect("a form head belongs to a list");
        if items.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Summary,
                "lambda dynamic summary requires declarations and body",
            ));
        }
        body = &items[2];
    }
    summarize_runtime_expression(body, prelude)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn summarize_runtime_expression(
    datum: &Datum,
    prelude: &BTreeMap<String, DynamicSummary>,
) -> Result<DynamicSummaryFacts, BundleError> {
    let Some(items) = datum.as_list() else {
        return Ok(DynamicSummaryFacts::inert());
    };
    if items.is_empty() {
        return Ok(DynamicSummaryFacts::inert());
    }
    if datum.form_head() == Some("Hole") {
        if items.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Summary,
                "Hole dynamic summary requires name and type",
            ));
        }
        let name = items[1].as_string().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Summary,
                "Hole summary name must be a string",
            )
        })?;
        let value_type = canonical_type_schema(&canonical_datum(&items[2]))?;
        return if matches!(value_type.as_str(), "Content" | "Discourse" | "Performable") {
            Ok(DynamicSummaryFacts::inert().with_parameter(name))
        } else {
            Ok(DynamicSummaryFacts::inert())
        };
    }
    let head = items.first().and_then(Datum::as_atom);
    if head == Some("λ") {
        return Ok(DynamicSummaryFacts::inert());
    }
    if let Some(name) = head.and_then(|head| head.strip_prefix('$')) {
        let mut facts = summarize_runtime_sequence(&items[1..], prelude)?;
        facts = facts.with_parameter(name);
        return Ok(facts);
    }
    if let Some(summary) = head.and_then(|head| prelude.get(head)) {
        let argument_facts = summarize_runtime_sequence(&items[1..], prelude)?;
        return Ok(argument_facts.append(summary_facts(summary)));
    }
    match head {
        Some("Presuppose") => {
            if items.len() != 3 {
                return Err(BundleError::new(
                    BundleErrorKind::Summary,
                    "Presuppose dynamic summary requires trigger and body",
                ));
            }
            let trigger = summarize_runtime_expression(&items[1], prelude)?;
            let body = summarize_runtime_expression(&items[2], prelude)?;
            Ok(trigger
                .with_effect(DynamicEffect::Presupposition)
                .append(body))
        }
        Some("Supplement") => Ok(summarize_runtime_sequence(&items[1..], prelude)?
            .with_effect(DynamicEffect::Supplement)),
        Some("Refer") => Ok(summarize_runtime_sequence(&items[1..], prelude)?
            .with_effect(DynamicEffect::ReferenceIntroduction)),
        Some("Perform" | "PerformUtterance" | "Do") => {
            Ok(summarize_runtime_sequence(&items[1..], prelude)?
                .with_effect(DynamicEffect::Performance))
        }
        Some("Context" | "Deictic") => {
            Ok(summarize_runtime_sequence(&items[1..], prelude)?.with_site_stability())
        }
        Some("∀" | "∃" | "SetOf") => {
            let mut facts = DynamicSummaryFacts::inert();
            for argument in &items[1..] {
                facts = facts.append(if argument.form_head() == Some("λ") {
                    summarize_callable(argument, prelude)?
                } else {
                    summarize_runtime_expression(argument, prelude)?
                });
            }
            Ok(facts)
        }
        Some("Bind") => summarize_bind(items, prelude),
        Some(_) | None => summarize_runtime_sequence(&items[1..], prelude),
    }
}

#[requires(items.first().and_then(Datum::as_atom) == Some("Bind"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn summarize_bind(
    items: &[Datum],
    prelude: &BTreeMap<String, DynamicSummary>,
) -> Result<DynamicSummaryFacts, BundleError> {
    if items.len() != 3 {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            "Bind dynamic summary requires bindings and body",
        ));
    }
    let bindings = items[1].as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Summary,
            "Bind dynamic summary bindings must be a list",
        )
    })?;
    let mut facts = DynamicSummaryFacts::inert();
    for binding in bindings {
        let binding = binding.as_list().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Summary,
                "Bind dynamic summary binding must be a list",
            )
        })?;
        if binding.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Summary,
                "Bind dynamic summary binding must contain name, type, and computation",
            ));
        }
        facts = facts.append(summarize_runtime_expression(&binding[2], prelude)?);
    }
    Ok(facts.append(summarize_runtime_expression(&items[2], prelude)?))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn summarize_runtime_sequence(
    data: &[Datum],
    prelude: &BTreeMap<String, DynamicSummary>,
) -> Result<DynamicSummaryFacts, BundleError> {
    data.iter()
        .try_fold(DynamicSummaryFacts::inert(), |facts, datum| {
            Ok(facts.append(summarize_runtime_expression(datum, prelude)?))
        })
}

#[requires(true)]
#[ensures(ret.parameter_evaluations == summary.parameter_evaluations && ret.ordered_effects == summary.ordered_effects)]
fn summary_facts(summary: &DynamicSummary) -> DynamicSummaryFacts {
    new!(DynamicSummaryFacts {
        parameter_evaluations: summary.parameter_evaluations.clone(),
        ordered_effects: summary.ordered_effects.clone(),
        site_stable: summary.stability == DynamicStability::SiteStableWithinPerformance,
    })
}

#[requires(true)]
#[ensures(true)]
fn is_summary_parameter_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_source_summary_claims(
    source: &RegistrySource,
    tables: &Tables,
) -> Result<(), BundleError> {
    let prelude_summaries = derive_prelude_summaries(&tables.prelude)?;
    for declaration in &source.prelude {
        let derived = prelude_summaries.get(&declaration.name).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Summary,
                format!(
                    "prelude summary has no generated row for {}",
                    declaration.name
                ),
            )
        })?;
        require_dynamic_summary(
            &format!("prelude {}", declaration.name),
            derived,
            &declaration.expected_dynamic_summary,
        )?;
    }
    for declaration in &source.tag_reduction {
        let row = tables
            .tag_reductions
            .iter()
            .find(|row| {
                row.source_family == declaration.source_family
                    && row.source_member == declaration.source_member
            })
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Summary,
                    format!(
                        "tag summary has no generated row for {}",
                        declaration.source_member
                    ),
                )
            })?;
        let datum = parse_document(&row.typed_expansion_template).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Summary,
                format!(
                    "parse tag {} for dynamic derivation: {error}",
                    row.source_member
                ),
            )
        })?;
        let derived = summarize_runtime_expression(&datum, &prelude_summaries)?.into_summary();
        require_dynamic_summary(
            &format!("tag {}", row.source_member),
            &derived,
            &declaration.expected_dynamic_summary,
        )?;
    }
    for declaration in &source.relation_former_reduction {
        let row = tables
            .relation_formers
            .iter()
            .find(|row| {
                row.former_kind == declaration.former_kind
                    && row.source_owner == declaration.source_owner
            })
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Summary,
                    format!(
                        "relation-former summary has no generated row for {}",
                        declaration.source_owner
                    ),
                )
            })?;
        let datum = parse_document(&row.typed_link_or_expansion_contract).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Summary,
                format!(
                    "parse relation former {} for dynamic derivation: {error}",
                    row.source_owner
                ),
            )
        })?;
        let derived = summarize_runtime_expression(&datum, &prelude_summaries)?.into_summary();
        require_dynamic_summary(
            &format!("relation former {}", row.source_owner),
            &derived,
            &declaration.expected_dynamic_summary,
        )?;
    }
    Ok(())
}

#[requires(!owner.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_dynamic_summary(
    owner: &str,
    derived: &DynamicSummary,
    expected: &DynamicSummary,
) -> Result<(), BundleError> {
    if derived != expected {
        return Err(BundleError::new(
            BundleErrorKind::Summary,
            format!("{owner} dynamic summary differs: expected {expected:?}, derived {derived:?}"),
        ));
    }
    Ok(())
}

#[requires(!spec.is_empty() && !signatures.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|definitions| definitions.len() == signatures.len()) || ret.is_err())]
fn extract_prelude_definitions(
    spec: &[u8],
    signatures: &BTreeMap<String, String>,
    type_parameters: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, String>, BundleError> {
    if signatures.keys().ne(type_parameters.keys()) {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "prelude signatures and type-parameter declarations differ",
        ));
    }
    require_nfc_utf8(SPEC_PATH, spec)?;
    let spec = std::str::from_utf8(spec).map_err(|error| {
        BundleError::new(
            BundleErrorKind::ByteDomain,
            format!("the frozen spec is not UTF-8: {error}"),
        )
    })?;
    let mut definitions = BTreeMap::new();
    for (index, fenced) in spec.split("```").enumerate() {
        if index % 2 == 0 {
            continue;
        }
        let Some((language, body)) = fenced.split_once('\n') else {
            continue;
        };
        match language.trim() {
            "lisp" if body.contains("(Let") && body.contains("⟦body⟧") => {
                let mentions_prelude = signatures.keys().any(|name| body.contains(name));
                if !mentions_prelude {
                    continue;
                }
                let parseable = body.replace("⟦body⟧", "$prelude_body");
                let datum = parse_document(parseable.trim()).map_err(|error| {
                    BundleError::new(
                        BundleErrorKind::Template,
                        format!("parse frozen prelude Let: {error}"),
                    )
                })?;
                collect_spec_let_definitions(
                    &datum,
                    signatures,
                    type_parameters,
                    &mut definitions,
                )?;
            }
            "text" => {
                collect_spec_equation_definitions(
                    body,
                    signatures,
                    type_parameters,
                    &mut definitions,
                )?;
            }
            _ => {}
        }
    }
    if definitions.len() != signatures.len() || definitions.keys().ne(signatures.keys()) {
        let missing = signatures
            .keys()
            .filter(|name| !definitions.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();
        return Err(BundleError::new(
            BundleErrorKind::Template,
            format!(
                "frozen spec prelude extraction produced {} definitions; missing {missing:?}",
                definitions.len()
            ),
        ));
    }
    Ok(definitions)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_spec_let_definitions(
    datum: &Datum,
    signatures: &BTreeMap<String, String>,
    type_parameters: &BTreeMap<String, Vec<String>>,
    definitions: &mut BTreeMap<String, String>,
) -> Result<(), BundleError> {
    if datum.form_head() == Some("Let") {
        let items = datum.as_list().expect("a form head belongs to a list");
        if items.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                "frozen prelude Let must have bindings and a body",
            ));
        }
        let bindings = items[1].as_list().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Template,
                "frozen prelude Let bindings must be a list",
            )
        })?;
        for binding in bindings {
            let binding = binding.as_list().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Template,
                    "frozen prelude Let binding must be a list",
                )
            })?;
            if binding.len() != 3 {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    "frozen prelude Let binding must contain name, signature, and initializer",
                ));
            }
            let Some(name) = binding[0].as_atom() else {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    "frozen prelude Let name must be an atom",
                ));
            };
            let Some(expected_signature) = signatures.get(name) else {
                continue;
            };
            let declared = type_parameters.get(name).ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!("prelude {name} has no type-parameter declaration"),
                )
            })?;
            let actual_signature =
                canonical_prelude_type_schema(&canonical_datum(&binding[1]), declared)?;
            if &actual_signature != expected_signature {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    format!("frozen prelude Let signature differs for {name}"),
                ));
            }
            let definition = canonical_prelude_definition(&canonical_datum(&binding[2]), declared)?;
            if definitions.insert(name.to_owned(), definition).is_some() {
                return Err(BundleError::new(
                    BundleErrorKind::DuplicatePrimaryKey,
                    format!("frozen spec defines prelude {name} more than once"),
                ));
            }
        }
        collect_spec_let_definitions(&items[2], signatures, type_parameters, definitions)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_spec_equation_definitions(
    block: &str,
    signatures: &BTreeMap<String, String>,
    type_parameters: &BTreeMap<String, Vec<String>>,
    definitions: &mut BTreeMap<String, String>,
) -> Result<(), BundleError> {
    let lines = block.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some((lhs, first_rhs)) = line.split_once(" = ") else {
            index += 1;
            continue;
        };
        let mut lhs_items = lhs.split_whitespace();
        let Some(name) = lhs_items.next() else {
            index += 1;
            continue;
        };
        let Some(signature) = signatures.get(name) else {
            index += 1;
            continue;
        };
        let declared_type_parameters = type_parameters.get(name).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("prelude {name} has no type-parameter declaration"),
            )
        })?;
        let raw_parameters = lhs_items.collect::<Vec<_>>();
        // General prelude equations use at least one `$`-prefixed value
        // parameter. The five closed mathematical equations are the explicit
        // exception and use conventional bare variables. Other specification
        // tables deliberately reuse names such as `Some` for named reduction
        // cases (`Some Di = ...`); those are not callable-name definitions.
        let is_closed_mathematical_equation = matches!(name, "≠" | ">" | "≥" | "∪" | "∩");
        if !is_closed_mathematical_equation
            && !raw_parameters
                .iter()
                .any(|parameter| parameter.starts_with('$'))
        {
            index += 1;
            continue;
        }
        let parameters = raw_parameters
            .into_iter()
            .map(|parameter| parameter.trim_start_matches('$').to_owned())
            .collect::<Vec<_>>();
        if parameters.is_empty()
            || parameters
                .iter()
                .any(|parameter| !is_spec_equation_parameter(parameter))
        {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("invalid frozen prelude equation lhs {lhs:?}"),
            ));
        }
        let mut rhs = first_rhs.trim().to_owned();
        let mut balance = parenthesis_balance(&rhs)?;
        while balance > 0 {
            index += 1;
            let Some(next) = lines.get(index) else {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    format!("unterminated frozen prelude equation for {name}"),
                ));
            };
            rhs.push(' ');
            rhs.push_str(next.trim());
            balance += parenthesis_balance(next)?;
            if balance < 0 {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    format!("unbalanced frozen prelude equation for {name}"),
                ));
            }
        }
        if balance != 0 {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("unbalanced frozen prelude equation for {name}"),
            ));
        }
        let rhs = parse_document(&rhs).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Template,
                format!("parse frozen prelude equation {name}: {error}"),
            )
        })?;
        let rhs = normalize_equation_parameters(&rhs, &parameters);
        let rhs = rewrite_schematic_type_parameters(&rhs, declared_type_parameters)?;
        let parameter_types = function_parameter_datums(signature)?;
        if parameter_types.len() != parameters.len() {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("frozen prelude equation arity differs for {name}"),
            ));
        }
        let declarations = parameters
            .iter()
            .zip(parameter_types)
            .map(|(parameter, parameter_type)| {
                Datum::list([Datum::atom(format!("${parameter}")), parameter_type])
            })
            .collect::<Vec<_>>();
        let definition = canonical_prelude_definition(
            &canonical_datum(&Datum::form("λ", [Datum::list(declarations), rhs])),
            declared_type_parameters,
        )?;
        if definitions.insert(name.to_owned(), definition).is_some() {
            return Err(BundleError::new(
                BundleErrorKind::DuplicatePrimaryKey,
                format!("frozen spec defines prelude {name} more than once"),
            ));
        }
        index += 1;
    }
    Ok(())
}

#[requires(!signature.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|parameters| !parameters.is_empty()) || ret.is_err())]
fn function_parameter_datums(signature: &str) -> Result<Vec<Datum>, BundleError> {
    let datum = parse_document(signature).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse prelude signature for equation extraction: {error}"),
        )
    })?;
    let items = datum.as_list().ok_or_else(|| {
        BundleError::new(BundleErrorKind::Type, "prelude signature must be a list")
    })?;
    if items.len() != 3 || items[0].as_atom() != Some("Fn") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "equation-defined prelude must have an Fn signature",
        ));
    }
    let parameters = items[1].as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Type,
            "equation-defined prelude Fn parameters must be a list",
        )
    })?;
    if parameters.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "equation-defined prelude needs at least one parameter",
        ));
    }
    Ok(parameters.to_vec())
}

#[requires(true)]
#[ensures(true)]
fn normalize_equation_parameters(datum: &Datum, parameters: &[String]) -> Datum {
    match datum {
        Datum::Atom(atom) => {
            let bare = atom.as_str().trim_start_matches('$');
            if parameters.iter().any(|parameter| parameter == bare) {
                Datum::atom(format!("${bare}"))
            } else {
                datum.clone()
            }
        }
        Datum::List(items) => Datum::list(
            items
                .iter()
                .map(|item| normalize_equation_parameters(item, parameters))
                .collect::<Vec<_>>(),
        ),
        _ => datum.clone(),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn rewrite_schematic_type_parameters(
    datum: &Datum,
    declared: &[String],
) -> Result<Datum, BundleError> {
    if matches!(datum.form_head(), Some("λ" | "Let" | "Bind" | "LetRec")) {
        let items = datum.as_list().expect("a form head belongs to a list");
        if items.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                "typed binding form in a prelude equation must have bindings and a body",
            ));
        }
        let bindings = items[1].as_list().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Template,
                "typed prelude equation bindings must be a list",
            )
        })?;
        let mut rewritten = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let fields = binding.as_list().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Template,
                    "typed prelude equation binding must be a list",
                )
            })?;
            let expected_fields = if datum.form_head() == Some("λ") {
                2
            } else {
                3
            };
            if fields.len() != expected_fields {
                return Err(BundleError::new(
                    BundleErrorKind::Template,
                    "typed prelude equation binding has the wrong arity",
                ));
            }
            let mut fields = fields.to_vec();
            fields[1] = rewrite_schematic_type_datum(&fields[1], declared)?;
            if fields.len() == 3 {
                fields[2] = rewrite_schematic_type_parameters(&fields[2], declared)?;
            }
            rewritten.push(Datum::list(fields));
        }
        return Ok(Datum::list([
            items[0].clone(),
            Datum::list(rewritten),
            rewrite_schematic_type_parameters(&items[2], declared)?,
        ]));
    }
    match datum {
        Datum::List(items) => Ok(Datum::list(
            items
                .iter()
                .map(|item| rewrite_schematic_type_parameters(item, declared))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        _ => Ok(datum.clone()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn rewrite_schematic_type_datum(datum: &Datum, declared: &[String]) -> Result<Datum, BundleError> {
    if datum.form_head() == Some("TypeParam") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "frozen specification definition must use schematic names, not registry TypeParam",
        ));
    }
    match datum {
        Datum::Atom(atom) if declared.iter().any(|name| name == atom.as_str()) => Ok(Datum::form(
            "TypeParam",
            [Datum::string(atom.as_str().to_owned())],
        )),
        Datum::List(items) => Ok(Datum::list(
            items
                .iter()
                .map(|item| rewrite_schematic_type_datum(item, declared))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        _ => Ok(datum.clone()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parenthesis_balance(text: &str) -> Result<isize, BundleError> {
    let mut balance = 0_isize;
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
    }
    if in_string || escaped {
        return Err(BundleError::new(
            BundleErrorKind::Template,
            "frozen prelude equation has an unterminated string",
        ));
    }
    Ok(balance)
}

#[requires(true)]
#[ensures(true)]
fn is_spec_equation_parameter(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

/// Prove exact equality between the AST scan and the independently authored
/// inventory projection before any registry rows are minted.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn validate_disposition_coordinate_authority(
    sources: &[DispositionSeed],
) -> Result<(), BundleError> {
    let scan = crate::smusni_v0_surface::scan_semantic_surface().map_err(|error| {
        BundleError::new(
            BundleErrorKind::Drift,
            format!("scan semantic surface: {error}"),
        )
    })?;
    let scanned = scan
        .coordinates
        .iter()
        .map(crate::smusni_v0_surface::SemanticCoordinate::owner)
        .collect::<BTreeSet<_>>();
    let authored = sources
        .iter()
        .map(|source| source.owner.clone())
        .collect::<BTreeSet<_>>();
    if authored.len() != sources.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "authored inventory contains a duplicate semantic coordinate",
        ));
    }
    if scanned != authored {
        let missing = scanned.difference(&authored).cloned().collect::<Vec<_>>();
        let orphan = authored.difference(&scanned).cloned().collect::<Vec<_>>();
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            format!(
                "semantic scan and authored inventory differ: missing={missing:?}, orphan={orphan:?}"
            ),
        ));
    }
    Ok(())
}

#[requires(sources.len() == rows.len())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_minted_disposition_coordinates(
    sources: &[DispositionSeed],
    rows: &[DispositionRow],
) -> Result<(), BundleError> {
    let authored = sources
        .iter()
        .map(|source| source.owner.as_str())
        .collect::<BTreeSet<_>>();
    let minted = rows
        .iter()
        .map(|row| row.disposition_owner.as_str())
        .collect::<BTreeSet<_>>();
    if authored != minted || minted.len() != rows.len() {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "authored inventory and minted disposition coordinates differ",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|(rows, reasons)| rows.len() == sources.len() && reasons.len() <= rows.len()) || ret.is_err())]
fn build_disposition_rows(
    sources: &[DispositionSeed],
) -> Result<(Vec<DispositionRow>, Vec<FallbackReasonRow>), BundleError> {
    let mut ordered = sources.to_vec();
    ordered.sort_by(|left, right| scalar_cmp(&left.owner, &right.owner));
    reject_duplicate(
        ordered.iter().map(|row| row.owner.as_str()),
        "disposition-owner",
    )?;
    let mut rows = Vec::with_capacity(ordered.len());
    let mut reasons = Vec::new();
    for source in ordered {
        if !is_disposition(&source.disposition) {
            return Err(BundleError::new(
                BundleErrorKind::ClosedValue,
                format!("unknown disposition {}", source.disposition),
            ));
        }
        let requires_detail = matches!(
            source.disposition.as_str(),
            "NotationDefault" | "ProvenanceSuppression" | "TypedFallback"
        );
        if source.detail.as_deref().is_some_and(str::is_empty)
            || source.detail.is_some() != requires_detail
        {
            return Err(BundleError::new(
                BundleErrorKind::Evidence,
                format!(
                    "disposition {} has an invalid explanatory detail",
                    source.owner
                ),
            ));
        }
        if source.disposition != "TypedFallback"
            && (source.fallback_reason_id.is_some()
                || source.expected_type_schema.is_some()
                || source.minimum_raw_owner_type.is_some())
        {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("non-fallback {} carries a fallback boundary", source.owner),
            ));
        }
        if source.disposition == "TypedFallback" {
            if source.target_contract.is_some() {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    format!("fallback {} carries a lowering target", source.owner),
                ));
            }
        } else {
            let target = source
                .target_contract
                .as_deref()
                .filter(|target| !target.is_empty())
                .ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Type,
                        format!("non-fallback {} has no target contract", source.owner),
                    )
                })?;
            if matches!(
                target,
                "smusni-v0-normal-form" | "verified-transparent-desugaring"
            ) || target.contains("LATER_LOWERING")
            {
                return Err(BundleError::new(
                    BundleErrorKind::Evidence,
                    format!("disposition {} retains a placeholder target", source.owner),
                ));
            }
        }
        let target = if source.disposition == "TypedFallback" {
            let expected_type_schema = source.expected_type_schema.as_deref().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Type,
                    format!("fallback {} has no expected type", source.owner),
                )
            })?;
            let minimum_raw_owner_type = source
                .minimum_raw_owner_type
                .as_deref()
                .filter(|owner| !owner.is_empty())
                .ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Type,
                        format!("fallback {} has no minimum raw owner", source.owner),
                    )
                })?;
            let reason_id = source.fallback_reason_id.clone().ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!("fallback {} has no reviewed reason id", source.owner),
                )
            })?;
            FallbackReason::try_new(&reason_id).map_err(|error| {
                BundleError::new(BundleErrorKind::ClosedValue, error.to_string())
            })?;
            let expected_type_schema = canonical_type_schema(expected_type_schema)?;
            reasons.push(new!(FallbackReasonRow {
                reason_id: reason_id.clone(),
                expected_type_schema,
                minimum_raw_owner_type: minimum_raw_owner_type.to_owned(),
                disposition_owner: source.owner.clone(),
            }));
            reason_id
        } else {
            source
                .target_contract
                .clone()
                .expect("non-fallback target was checked")
        };
        rows.push(new!(DispositionRow {
            disposition_owner: source.owner,
            model_constructor_or_field: source.model_member,
            disposition: source.disposition.clone(),
            target_schema_or_fallback_reason: target,
            evidence_id: "smusni.semantic-surface.inventory".to_owned(),
        }));
    }
    reasons.sort_by(|left, right| scalar_cmp(&left.reason_id, &right.reason_id));
    reject_duplicate(
        reasons.iter().map(|row| row.reason_id.as_str()),
        "fallback reason-id",
    )?;
    Ok((rows, reasons))
}

#[requires(source_artifacts.len() == 6)]
#[ensures(ret.is_ok() || ret.is_err())]
fn add_common_evidence(
    evidence: &mut Vec<EvidenceRow>,
    source_artifacts: &[SourceArtifactRow],
) -> Result<(), BundleError> {
    let model = source_artifacts
        .iter()
        .find(|row| row.source_id == "jbotci-semantic-surface")
        .ok_or_else(|| BundleError::new(BundleErrorKind::ForeignKey, "model source absent"))?;
    evidence.push(new!(EvidenceRow {
        evidence_id: "smusni.semantic-surface.inventory".to_owned(),
        source_id: model.source_id.clone(),
        exact_locator: "render_field_inventory() complete generated-model projection".to_owned(),
        cited_content_digest: model.artifact_digest.clone(),
        adjudication_note: "Disposition rows are generated from the existing exact inventory candidate; no second authored ledger exists."
            .to_owned(),
    }));
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_tables(tables: &Tables, spec: &[u8]) -> Result<(), BundleError> {
    validate_table_order(tables)?;
    validate_registry_contracts(tables, spec)?;
    let source_ids = tables
        .source_artifacts
        .iter()
        .map(|row| row.source_id.as_str())
        .collect::<BTreeSet<_>>();
    if source_ids.len() != tables.source_artifacts.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "duplicate SourceArtifactRow source-id",
        ));
    }
    for row in &tables.evidence {
        let Some(source) = tables
            .source_artifacts
            .iter()
            .find(|source| source.source_id == row.source_id)
        else {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "evidence {} has unknown source {}",
                    row.evidence_id, row.source_id
                ),
            ));
        };
        if row.cited_content_digest != source.artifact_digest {
            return Err(BundleError::new(
                BundleErrorKind::Evidence,
                format!(
                    "evidence {} digest differs from source {}",
                    row.evidence_id, row.source_id
                ),
            ));
        }
    }
    let evidence_ids = tables
        .evidence
        .iter()
        .map(|row| row.evidence_id.as_str())
        .collect::<BTreeSet<_>>();
    let require_evidence = |id: &str, owner: &str| {
        if evidence_ids.contains(id) {
            Ok(())
        } else {
            Err(BundleError::new(
                BundleErrorKind::Evidence,
                format!("{owner} has unknown evidence-id {id}"),
            ))
        }
    };
    for row in &tables.lexical {
        if !source_ids.contains(row.dictionary_source_id.as_str()) {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "lexical {} has unknown dictionary source",
                    row.normalized_root
                ),
            ));
        }
        for (index, slot) in row.ordered_numbered_slot_rows.iter().enumerate() {
            if !matches!(slot.label.as_data(), data!(SlotLabel::Numbered(value)) if *value == (index + 1) as u64)
            {
                return Err(BundleError::new(
                    BundleErrorKind::NonCanonicalOrder,
                    format!(
                        "lexical {} numbered slots are not contiguous",
                        row.normalized_root
                    ),
                ));
            }
            require_evidence(&slot.evidence_id, &row.normalized_root)?;
        }
        match &row.optional_event_slot_row {
            Some(slot)
                if matches!(slot.label.as_data(), data!(SlotLabel::Eventuality(value)) if value == "Eventuality")
                    && slot.close_policy == ClosePolicy::LocalExistential =>
            {
                require_evidence(&slot.evidence_id, &row.normalized_root)?;
            }
            Some(_) => {
                return Err(BundleError::new(
                    BundleErrorKind::ClosedValue,
                    format!("lexical {} has malformed event slot", row.normalized_root),
                ));
            }
            None => {}
        }
    }
    for row in &tables.scope_policies {
        require_evidence(&row.evidence_id, &row.normalized_root)?;
    }
    for row in &tables.place_deletions {
        require_evidence(&row.evidence_id, &row.expansion_owner)?;
    }
    for row in &tables.tag_reductions {
        require_evidence(&row.evidence_id, &row.source_member)?;
    }
    for row in &tables.relation_formers {
        require_evidence(&row.evidence_id, &row.source_owner)?;
    }
    for row in &tables.generated_relations {
        require_evidence(&row.evidence_id, &row.pascal_case_name)?;
        let derived = derive_generated_relation_summary(&row.complete_signature)?;
        let (derived_context_effect, derived_stability) =
            generated_relation_summary_fields(&derived)?;
        if row.context_effect_summary != derived_context_effect
            || row.stability_summary != derived_stability
        {
            return Err(BundleError::new(
                BundleErrorKind::Summary,
                format!(
                    "generated relation {} summary differs from its typed inert predicate contract",
                    row.pascal_case_name
                ),
            ));
        }
    }
    for row in &tables.scale_literals {
        require_evidence(&row.evidence_id, &row.pascal_case_name)?;
    }
    let disposition_owners = tables
        .dispositions
        .iter()
        .map(|row| row.disposition_owner.as_str())
        .collect::<BTreeSet<_>>();
    if disposition_owners.len() != tables.dispositions.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "duplicate disposition owner",
        ));
    }
    for row in &tables.dispositions {
        require_evidence(&row.evidence_id, &row.disposition_owner)?;
        if row.disposition == "TypedFallback"
            && !tables.fallback_reasons.iter().any(|reason| {
                reason.reason_id == row.target_schema_or_fallback_reason
                    && reason.disposition_owner == row.disposition_owner
            })
        {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("{} has no exact fallback-reason row", row.disposition_owner),
            ));
        }
    }
    for row in &tables.fallback_reasons {
        if !tables.dispositions.iter().any(|disposition| {
            disposition.disposition_owner == row.disposition_owner
                && disposition.disposition == "TypedFallback"
                && disposition.target_schema_or_fallback_reason == row.reason_id
        }) {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!(
                    "fallback {} does not join its exact TypedFallback owner",
                    row.reason_id
                ),
            ));
        }
        canonical_type_schema(&row.expected_type_schema)?;
    }
    let mut referenced_evidence = BTreeSet::new();
    for row in &tables.lexical {
        referenced_evidence.extend(
            row.ordered_numbered_slot_rows
                .iter()
                .map(|slot| slot.evidence_id.as_str()),
        );
        if let Some(event) = &row.optional_event_slot_row {
            referenced_evidence.insert(event.evidence_id.as_str());
        }
    }
    referenced_evidence.extend(
        tables
            .scope_policies
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .place_deletions
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .tag_reductions
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .relation_formers
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .generated_relations
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .scale_literals
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    referenced_evidence.extend(
        tables
            .dispositions
            .iter()
            .map(|row| row.evidence_id.as_str()),
    );
    if let Some(orphan) = tables
        .evidence
        .iter()
        .find(|row| !referenced_evidence.contains(row.evidence_id.as_str()))
    {
        return Err(BundleError::new(
            BundleErrorKind::Evidence,
            format!("orphan evidence row {}", orphan.evidence_id),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_table_order(tables: &Tables) -> Result<(), BundleError> {
    require_sorted_unique(
        &tables
            .source_artifacts
            .iter()
            .map(|row| row.source_id.as_str())
            .collect::<Vec<_>>(),
        "source artifact primary keys",
    )?;
    require_sorted_unique(
        &tables
            .evidence
            .iter()
            .map(|row| row.evidence_id.as_str())
            .collect::<Vec<_>>(),
        "evidence primary keys",
    )?;
    require_sorted_unique(
        &tables
            .lexical
            .iter()
            .map(|row| row.normalized_root.as_str())
            .collect::<Vec<_>>(),
        "lexical primary keys",
    )?;
    require_tuple_sorted_unique(
        &tables
            .scope_policies
            .iter()
            .map(|row| (row.normalized_root.as_str(), row.original_ordinal))
            .collect::<Vec<_>>(),
        "scope-policy primary keys",
    )?;
    require_sorted_unique(
        &tables
            .place_deletions
            .iter()
            .map(|row| {
                format!(
                    "{}\u{0}{}\u{0}{:020}",
                    row.expansion_owner, row.normalized_root, row.original_ordinal
                )
            })
            .collect::<Vec<_>>(),
        "place-deletion primary keys",
    )?;
    require_sorted_unique(
        &tables
            .tag_reductions
            .iter()
            .map(|row| {
                format!(
                    "{}\u{0}{}\u{0}{}",
                    row.source_family, row.source_member, row.applicability_guard
                )
            })
            .collect::<Vec<_>>(),
        "tag-reduction primary keys",
    )?;
    require_sorted_unique(
        &tables
            .relation_formers
            .iter()
            .map(|row| {
                format!(
                    "{}\u{0}{}\u{0}{}",
                    row.former_kind, row.source_owner, row.applicability_guard
                )
            })
            .collect::<Vec<_>>(),
        "relation-former primary keys",
    )?;
    require_sorted_unique(
        &tables
            .generated_relations
            .iter()
            .map(|row| format!("{}\u{0}{}", row.family, row.pascal_case_name))
            .collect::<Vec<_>>(),
        "generated-relation primary keys",
    )?;
    for (values, context) in [
        (
            tables
                .scale_literals
                .iter()
                .map(|row| row.pascal_case_name.as_str())
                .collect::<Vec<_>>(),
            "scale-literal primary keys",
        ),
        (
            tables
                .fallback_reasons
                .iter()
                .map(|row| row.reason_id.as_str())
                .collect::<Vec<_>>(),
            "fallback-reason primary keys",
        ),
        (
            tables
                .dispositions
                .iter()
                .map(|row| row.disposition_owner.as_str())
                .collect::<Vec<_>>(),
            "disposition primary keys",
        ),
        (
            tables
                .prelude
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            "prelude primary keys",
        ),
    ] {
        require_sorted_unique(&values, context)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_registry_contracts(tables: &Tables, spec: &[u8]) -> Result<(), BundleError> {
    if tables.source_artifacts.len() != 6
        || tables.lexical.len() != 44
        || tables.scope_policies.len() != SCOPE_POLICY_ROW_COUNT
        || tables.place_deletions.len() != 7
        || tables.tag_reductions.len() != 6
        || tables.relation_formers.len() != 1
        || tables.generated_relations.len() != 1
        || tables.scale_literals.len() != 1
        || tables.dispositions.len() != DISPOSITION_ROW_COUNT
        || tables.fallback_reasons.len() != FALLBACK_REASON_ROW_COUNT
        || tables.prelude.len() != 20
    {
        return Err(BundleError::new(
            BundleErrorKind::Drift,
            "candidate v0 registry table cardinalities differ",
        ));
    }
    for row in &tables.lexical {
        for (index, slot) in row.ordered_numbered_slot_rows.iter().enumerate() {
            if canonical_type_schema(&slot.accepted_type_schema)? != slot.accepted_type_schema
                || slot.close_policy == ClosePolicy::LocalExistential
                || slot.lexical_provenance != format!("{}:x{}", row.normalized_root, index + 1)
            {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    format!(
                        "lexical {} x{} contract differs",
                        row.normalized_root,
                        index + 1
                    ),
                ));
            }
        }
        if let Some(event) = &row.optional_event_slot_row
            && (event.accepted_type_schema != "(Referents Eventuality)"
                || event.close_policy != ClosePolicy::LocalExistential
                || event.lexical_provenance != format!("event-license:{}", row.normalized_root))
        {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("lexical {} event contract differs", row.normalized_root),
            ));
        }
    }
    if tables
        .scope_policies
        .iter()
        .filter(|row| row.scope_policy == ScopePolicy::Extensional)
        .count()
        != EXTENSIONAL_SCOPE_POLICY_COUNT
        || tables
            .scope_policies
            .iter()
            .filter(|row| row.scope_policy == ScopePolicy::Intensional)
            .count()
            != INTENSIONAL_SCOPE_POLICY_COUNT
        || tables
            .scope_policies
            .iter()
            .any(|row| row.scope_policy == ScopePolicy::Opaque)
    {
        return Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            "v0 lexical policy distribution differs",
        ));
    }
    for policy in &tables.scope_policies {
        let lexical = tables
            .lexical
            .iter()
            .find(|row| row.normalized_root == policy.normalized_root)
            .ok_or_else(|| {
                BundleError::new(BundleErrorKind::ForeignKey, "scope-policy root is absent")
            })?;
        if policy.original_ordinal as usize > lexical.ordered_numbered_slot_rows.len() {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                "scope-policy ordinal is outside the lexical row",
            ));
        }
        let slot = &lexical.ordered_numbered_slot_rows[policy.original_ordinal as usize - 1];
        compiled_dynamic_family(&slot.accepted_type_schema)?;
    }
    let mut deletion_rows: BTreeMap<(String, String), Vec<SlotRow>> = BTreeMap::new();
    for deletion in &tables.place_deletions {
        let lexical = tables
            .lexical
            .iter()
            .find(|row| row.normalized_root == deletion.normalized_root)
            .ok_or_else(|| {
                BundleError::new(BundleErrorKind::ForeignKey, "deletion root is absent")
            })?;
        let slots = deletion_rows
            .entry((
                deletion.expansion_owner.clone(),
                deletion.normalized_root.clone(),
            ))
            .or_insert_with(|| {
                let mut slots = lexical.ordered_numbered_slot_rows.clone();
                if let Some(event) = &lexical.optional_event_slot_row {
                    slots.push(event.clone());
                }
                slots
            });
        if row_schema(slots)? != deletion.input_row_schema {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "deletion input row does not derive from the current lexical row",
            ));
        }
        let input_slots = slots.clone();
        let index = slots
            .iter()
            .position(|slot| matches!(slot.label.as_data(), data!(SlotLabel::Numbered(value)) if *value == deletion.original_ordinal))
            .ok_or_else(|| {
                BundleError::new(BundleErrorKind::ForeignKey, "deletion target is absent")
            })?;
        slots.remove(index);
        if row_schema(slots)? != deletion.result_row_schema {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "deletion result row does not preserve surviving labels",
            ));
        }
        validate_surviving_map(&deletion.surviving_slot_map, &input_slots, slots)?;
        if deletion.surviving_slot_map.iter().any(|mapping| {
            mapping
                .split_once("->")
                .is_none_or(|(source, result)| source != result)
        }) {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                "DropPlace surviving labels must retain original identity",
            ));
        }
    }
    for row in &tables.tag_reductions {
        if canonical_template_without_registry(&row.applicability_guard)? != row.applicability_guard
            || canonical_template(
                &row.typed_expansion_template,
                &tables.lexical,
                &tables.prelude,
            )? != row.typed_expansion_template
            || canonical_type_schema(&row.resulting_type_schema)? != row.resulting_type_schema
            || derive_template_result(&row.typed_expansion_template)? != row.resulting_type_schema
        {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("tag {} expansion does not rederive", row.source_member),
            ));
        }
        for operand in &row.operand_types {
            if canonical_type_schema(operand)? != *operand {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    "tag operand type is noncanonical",
                ));
            }
        }
        validate_typed_template(
            &row.applicability_guard,
            &derive_template_result(&row.applicability_guard)?,
            &tables.lexical,
            &tables.prelude,
        )?;
        validate_tag_metadata(
            &row.operand_types,
            &row.source_place_map,
            row.host_event_map,
            &row.required_graph_identities,
            &row.applicability_guard,
            &row.typed_expansion_template,
            &tables.lexical,
            &tables.prelude,
        )?;
        validate_typed_template(
            &row.typed_expansion_template,
            &row.resulting_type_schema,
            &tables.lexical,
            &tables.prelude,
        )?;
    }
    for row in &tables.relation_formers {
        if canonical_template_without_registry(&row.applicability_guard)? != row.applicability_guard
            || canonical_template_without_registry(&row.typed_link_or_expansion_contract)?
                != row.typed_link_or_expansion_contract
        {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                "relation-former template is noncanonical",
            ));
        }
        for operand in &row.operand_row_schemas {
            if canonical_row_schema(operand)? != *operand {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    "relation-former operand row is noncanonical",
                ));
            }
        }
        if canonical_row_schema(&row.result_row_schema)? != row.result_row_schema {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "relation-former result row is noncanonical",
            ));
        }
        validate_total_provenance_map(
            &row.total_provenance_map,
            &row.operand_row_schemas,
            &row.result_row_schema,
        )?;
        validate_typed_template(
            &row.applicability_guard,
            &format!("(PredTerm {})", row.operand_row_schemas[0]),
            &[],
            &[],
        )?;
        validate_typed_template(
            &row.typed_link_or_expansion_contract,
            &format!("(PredTerm {})", row.result_row_schema),
            &[],
            &[],
        )?;
    }
    for row in &tables.generated_relations {
        if canonical_type_schema(&row.complete_signature)? != row.complete_signature {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "generated relation signature is noncanonical",
            ));
        }
    }
    for row in &tables.scale_literals {
        if canonical_type_schema(&row.raw_value_type)? != row.raw_value_type {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "scale literal type is noncanonical",
            ));
        }
        require_unique_nonempty(&row.source_members, "scale source members")?;
    }
    let prelude_names = tables
        .prelude
        .iter()
        .map(|row| row.name.clone())
        .collect::<BTreeSet<_>>();
    let prelude_signatures = tables
        .prelude
        .iter()
        .map(|row| (row.name.clone(), row.complete_signature_schema.clone()))
        .collect::<BTreeMap<_, _>>();
    let prelude_type_parameters = tables
        .prelude
        .iter()
        .map(|row| (row.name.clone(), row.type_parameters.clone()))
        .collect::<BTreeMap<_, _>>();
    let spec_definitions =
        extract_prelude_definitions(spec, &prelude_signatures, &prelude_type_parameters)?;
    for row in &tables.prelude {
        if spec_definitions.get(&row.name) != Some(&row.canonical_definition)
            || canonical_prelude_type_schema(&row.complete_signature_schema, &row.type_parameters)?
                != row.complete_signature_schema
            || canonical_prelude_definition(&row.canonical_definition, &row.type_parameters)?
                != row.canonical_definition
            || sha256_hex(row.canonical_definition.as_bytes()) != row.definition_digest
            || prelude_dependencies(&row.canonical_definition, &prelude_names, &row.name)?
                != row.direct_dependencies
        {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("prelude {} does not rederive", row.name),
            ));
        }
        validate_prelude_signature(
            &row.name,
            &row.complete_signature_schema,
            &row.canonical_definition,
        )?;
        validate_prelude_type_parameter_usage(
            &row.type_parameters,
            &row.complete_signature_schema,
            &row.canonical_definition,
        )?;
        for (root, ordinal) in collect_drop_places(&row.canonical_definition)? {
            if !tables.place_deletions.iter().any(|deletion| {
                deletion.expansion_owner == row.name
                    && deletion.normalized_root == root
                    && deletion.original_ordinal == ordinal
            }) {
                return Err(BundleError::new(
                    BundleErrorKind::Evidence,
                    format!("prelude {} has an unevidenced DropPlace", row.name),
                ));
            }
        }
    }
    validate_prelude_acyclic(&tables.prelude)?;
    let registry = StaticTypeRegistry::from_rows(&tables.lexical, &tables.prelude)?;
    for row in &tables.prelude {
        let definition = parse_document(&row.canonical_definition)
            .map_err(|error| BundleError::new(BundleErrorKind::Template, error.to_string()))?;
        let signature = StaticType::parse(
            &parse_document(&row.complete_signature_schema)
                .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?,
            true,
        )?;
        check_expression(&definition, &signature, &BTreeMap::new(), &registry).map_err(
            |error| {
                BundleError::new(
                    error.kind,
                    format!("prelude {}: {}", row.name, error.message),
                )
            },
        )?;
    }
    for row in &tables.dispositions {
        if !is_disposition(&row.disposition) {
            return Err(BundleError::new(
                BundleErrorKind::ClosedValue,
                "disposition uses an unregistered class",
            ));
        }
    }
    let graph_failures = tables
        .fallback_reasons
        .iter()
        .filter(|row| row.minimum_raw_owner_type == "SemanticGraph")
        .map(|row| row.reason_id.as_str())
        .collect::<BTreeSet<_>>();
    let required_graph_failures = REQUIRED_GRAPH_FAILURE_REASON_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if !required_graph_failures.is_subset(&graph_failures) {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "one or more registered planning/elaboration failure ids disappeared",
        ));
    }
    for row in &tables.fallback_reasons {
        canonical_type_schema(&row.expected_type_schema)?;
        let compatible = match row.minimum_raw_owner_type.as_str() {
            "SemanticGraph" => row.expected_type_schema == "Performable",
            "Referent" => matches!(
                row.expected_type_schema.as_str(),
                "(Referents Entity)" | "(Referents Eventuality)"
            ),
            "Eventuality" => row.expected_type_schema == "Content",
            "Quantity" => row.expected_type_schema == "Quantity",
            "MathExpression" => row.expected_type_schema == "MathExpression",
            _ => false,
        };
        if !compatible {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!(
                    "fallback {} has an unknown or incompatible minimum raw owner boundary",
                    row.reason_id
                ),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|artifacts| artifacts.len() == GENERATED_TABLES.len()) || ret.is_err())]
fn serialize_tables(tables: &Tables) -> Result<BTreeMap<String, Vec<u8>>, BundleError> {
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        "registry/source-artifacts.jsonl".to_owned(),
        jsonl(&tables.source_artifacts)?,
    );
    artifacts.insert(
        "registry/evidence.jsonl".to_owned(),
        jsonl(&tables.evidence)?,
    );
    artifacts.insert("registry/lexical.jsonl".to_owned(), jsonl(&tables.lexical)?);
    artifacts.insert(
        "registry/scope-policies.jsonl".to_owned(),
        jsonl(&tables.scope_policies)?,
    );
    artifacts.insert(
        "registry/place-deletions.jsonl".to_owned(),
        jsonl(&tables.place_deletions)?,
    );
    artifacts.insert(
        "registry/tag-reductions.jsonl".to_owned(),
        jsonl(&tables.tag_reductions)?,
    );
    artifacts.insert(
        "registry/relation-formers.jsonl".to_owned(),
        jsonl(&tables.relation_formers)?,
    );
    artifacts.insert(
        "registry/generated-relations.jsonl".to_owned(),
        jsonl(&tables.generated_relations)?,
    );
    artifacts.insert(
        "registry/scale-literals.jsonl".to_owned(),
        jsonl(&tables.scale_literals)?,
    );
    artifacts.insert(
        "registry/fallback-reasons.jsonl".to_owned(),
        jsonl(&tables.fallback_reasons)?,
    );
    artifacts.insert(
        "registry/dispositions.jsonl".to_owned(),
        jsonl(&tables.dispositions)?,
    );
    artifacts.insert("registry/prelude.jsonl".to_owned(), jsonl(&tables.prelude)?);
    artifacts.insert(
        "registry/runtime.rs".to_owned(),
        generate_policy_rust(tables)?,
    );
    Ok(artifacts)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.ends_with(b"\n")) || ret.is_err())]
fn jsonl<T: Serialize>(rows: &[T]) -> Result<Vec<u8>, BundleError> {
    let mut output = Vec::new();
    for row in rows {
        let value = serde_json::to_value(row).map_err(|error| {
            BundleError::new(BundleErrorKind::Parse, format!("serialize row: {error}"))
        })?;
        require_nfc_value(&value)?;
        output.extend_from_slice(canonical_json(&value)?.as_bytes());
        output.push(b'\n');
    }
    if rows.is_empty() {
        // Empty registered families still have a canonical empty JSONL byte
        // sequence. The initial bundle presently has no empty family.
        return Ok(output);
    }
    Ok(output)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|records| records.len() == MIRRORED_GENERATOR_INPUTS.len() + BUNDLE_NATIVE_GENERATOR_INPUTS.len()) || ret.is_err())]
fn build_source_manifest(root: &Path) -> Result<Vec<ArtifactRecord>, BundleError> {
    let expected_count = MIRRORED_GENERATOR_INPUTS.len() + BUNDLE_NATIVE_GENERATOR_INPUTS.len();
    let mut paths = generator_input_paths();
    paths.sort_by(|left, right| scalar_cmp(left, right));
    paths.dedup();
    if paths.len() != expected_count {
        return Err(BundleError::new(
            BundleErrorKind::Manifest,
            "generator-input closure contains duplicate paths",
        ));
    }
    paths
        .into_iter()
        .map(|relative| {
            let bytes = read_relative(root, &relative)?;
            Ok(new!(ArtifactRecord {
                relative_path: relative,
                schema_id: SchemaId::OpaqueBytes.as_str().to_owned(),
                row_count: 1,
                digest: sha256_hex(&bytes),
            }))
        })
        .collect()
}

#[requires(artifacts.len() == GENERATED_TABLES.len())]
#[ensures(ret.as_ref().is_ok_and(|records| records.len() == GENERATED_TABLES.len()) || ret.is_err())]
fn build_generated_manifest(
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<ArtifactRecord>, BundleError> {
    let schemas = GENERATED_TABLES
        .iter()
        .map(|(path, schema)| ((*path).to_owned(), *schema))
        .collect::<BTreeMap<_, _>>();
    artifacts
        .iter()
        .map(|(relative, bytes)| {
            let schema = schemas.get(relative).ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Manifest,
                    format!("generated artifact has no closed schema id: {relative}"),
                )
            })?;
            let row_count = if *schema == SchemaId::OpaqueBytes {
                if bytes.is_empty() {
                    return Err(BundleError::new(
                        BundleErrorKind::ByteDomain,
                        format!("generated opaque artifact is empty: {relative}"),
                    ));
                }
                1
            } else {
                let count = bytes.iter().filter(|byte| **byte == b'\n').count();
                if count == 0 || !bytes.ends_with(b"\n") {
                    return Err(BundleError::new(
                        BundleErrorKind::ByteDomain,
                        format!("generated JSONL is not nonempty LF records: {relative}"),
                    ));
                }
                count
            };
            Ok(new!(ArtifactRecord {
                relative_path: relative.clone(),
                schema_id: schema.as_str().to_owned(),
                row_count,
                digest: sha256_hex(bytes),
            }))
        })
        .collect()
}

#[requires(!source.is_empty() && !generated.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|manifest| manifest.format_version == 0) || ret.is_err())]
fn build_manifest(
    source: Vec<ArtifactRecord>,
    generated: Vec<ArtifactRecord>,
) -> Result<Manifest, BundleError> {
    require_artifact_order(&source, "source-artifacts")?;
    require_artifact_order(&generated, "generated-artifacts")?;
    let generator_inputs = source
        .iter()
        .map(|record| record.relative_path.clone())
        .collect::<Vec<_>>();
    let generator_pairs = source
        .iter()
        .map(|record| (record.relative_path.clone(), record.digest.clone()))
        .collect::<Vec<_>>();
    let generator_id = digest_pairs(&generator_pairs)?;
    let mut bundle_pairs = generator_pairs;
    bundle_pairs.extend(
        generated
            .iter()
            .map(|record| (record.relative_path.clone(), record.digest.clone())),
    );
    bundle_pairs.sort_by(|left, right| scalar_cmp(&left.0, &right.0));
    let bundle_digest = digest_pairs(&bundle_pairs)?;
    Ok(new!(Manifest {
        format_version: 0,
        bundle_schema_version: 1,
        spec_digest: SPEC_SHA256.to_owned(),
        generator_id,
        generator_inputs,
        source_artifacts: source,
        generated_artifacts: generated,
        bundle_digest,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.ends_with(b"\n")) || ret.is_err())]
fn jcs_line<T: Serialize>(value: &T) -> Result<Vec<u8>, BundleError> {
    let value = serde_json::to_value(value).map_err(|error| {
        BundleError::new(BundleErrorKind::Parse, format!("serialize JCS: {error}"))
    })?;
    require_nfc_value(&value)?;
    let mut bytes = canonical_json(&value)?.into_bytes();
    bytes.push(b'\n');
    Ok(bytes)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rust| rust.ends_with(b"\n")) || ret.is_err())]
fn generate_policy_rust(tables: &Tables) -> Result<Vec<u8>, BundleError> {
    let lexical = tables
        .lexical
        .iter()
        .map(|row| (row.normalized_root.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut output =
        String::from("// @generated from the smusni-v0 candidate registry; do not edit.\n\n");
    output.push_str(
        "pub(super) const GENERATED_LEXICAL_POLICY_ROWS: &[GeneratedLexicalPolicyRow] = &[\n",
    );
    for row in &tables.scope_policies {
        let lexical_row = lexical.get(row.normalized_root.as_str()).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::ForeignKey,
                "policy lexical row disappeared",
            )
        })?;
        let slot = lexical_row
            .ordered_numbered_slot_rows
            .get(row.original_ordinal as usize - 1)
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    "policy lexical place disappeared",
                )
            })?;
        let accepted_family = compiled_dynamic_family(&slot.accepted_type_schema)?;
        output.push_str(&format!(
            "    GeneratedLexicalPolicyRow {{ relation: {:?}, original_place: {}, attested_arity: {}, accepted_family: DynamicValueFamily::{}, policy: ScopePolicy::{:?} }},\n",
            row.normalized_root,
            row.original_ordinal,
            lexical_row.ordered_numbered_slot_rows.len(),
            accepted_family,
            row.scope_policy
        ));
    }
    output.push_str("];\n");
    for (constant, reason_id) in [
        (
            "GENERATED_LEXICAL_POLICY_ENTITY_FALLBACK_REASON_ID",
            "smusni.fallback.lexical-policy.entity",
        ),
        (
            "GENERATED_LEXICAL_POLICY_EVENTUALITY_FALLBACK_REASON_ID",
            "smusni.fallback.lexical-policy.eventuality",
        ),
    ] {
        let reason = tables
            .fallback_reasons
            .iter()
            .find(|row| row.reason_id == reason_id)
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::ForeignKey,
                    format!("compiled fallback reason {reason_id} disappeared"),
                )
            })?;
        output.push_str(&format!(
            "pub(super) const {constant}: &str = {:?};\n",
            reason.reason_id
        ));
    }
    output.push_str("\nconst GENERATED_DISPOSITION_ROWS: &[GeneratedDispositionRow] = &[\n");
    for row in &tables.dispositions {
        let coordinate = parse_compiled_disposition_coordinate(&row.disposition_owner)?;
        let target = (row.disposition != "TypedFallback")
            .then_some(row.target_schema_or_fallback_reason.as_str());
        let fallback = (row.disposition == "TypedFallback")
            .then_some(row.target_schema_or_fallback_reason.as_str());
        output.push_str(&format!(
            "    GeneratedDispositionRow {{ category: CoordinateCategory::{}, surface: {:?}, kind: CoordinateKind::{}, member: {:?}, qualifier: {:?}, disposition: DispositionKind::{}, target_contract: {:?}, fallback_reason_id: {:?} }},\n",
            coordinate.category,
            coordinate.surface,
            coordinate.kind,
            coordinate.member,
            coordinate.qualifier,
            row.disposition,
            target,
            fallback,
        ));
    }
    output.push_str(
        "];\n\nconst GENERATED_FALLBACK_REASON_ROWS: &[GeneratedFallbackReasonRow] = &[\n",
    );
    for row in &tables.fallback_reasons {
        output.push_str(&format!(
            "    GeneratedFallbackReasonRow {{ reason_id: {:?}, expected_type_schema: {:?}, minimum_raw_owner_type: {:?}, disposition_owner: {:?} }},\n",
            row.reason_id,
            row.expected_type_schema,
            row.minimum_raw_owner_type,
            row.disposition_owner,
        ));
    }
    output.push_str("];\n");
    Ok(output.into_bytes())
}

#[invariant(!category.is_empty() && !surface.is_empty() && !kind.is_empty() && !member.is_empty())]
#[invariant(qualifier.as_ref().is_none_or(|value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledDispositionCoordinate {
    category: String,
    surface: String,
    kind: String,
    member: String,
    qualifier: Option<String>,
}

#[requires(!owner.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_compiled_disposition_coordinate(
    owner: &str,
) -> Result<CompiledDispositionCoordinate, BundleError> {
    let mut parts = owner.splitn(4, ':');
    let category = parts.next().unwrap_or_default();
    let surface = parts.next().unwrap_or_default();
    let kind = parts.next().unwrap_or_default();
    let member = parts.next().unwrap_or_default();
    if !matches!(category, "Object" | "ValueStruct" | "Enum" | "Document")
        || !matches!(
            kind,
            "Constructor"
                | "Discriminator"
                | "Field"
                | "EnumVariant"
                | "VariantField"
                | "DerivedFact"
        )
        || surface.is_empty()
        || member.is_empty()
    {
        return Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            format!("invalid disposition coordinate {owner}"),
        ));
    }
    let (member, qualifier) = member.split_once('@').map_or_else(
        || (member, None),
        |(member, qualifier)| (member, Some(qualifier)),
    );
    if member.is_empty() || qualifier.is_some_and(str::is_empty) {
        return Err(BundleError::new(
            BundleErrorKind::ClosedValue,
            format!("invalid qualified disposition coordinate {owner}"),
        ));
    }
    Ok(new!(CompiledDispositionCoordinate {
        category: category.to_owned(),
        surface: surface.to_owned(),
        kind: kind.to_owned(),
        member: member.to_owned(),
        qualifier: qualifier.map(str::to_owned),
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|name| !name.is_empty()) || ret.is_err())]
fn compiled_dynamic_family(accepted_type_schema: &str) -> Result<&'static str, BundleError> {
    match accepted_type_schema {
        "(Referents Entity)" => Ok("RefCompReferentsEntity"),
        "(Referents Eventuality)" => Ok("RefCompReferentsEventuality"),
        _ => Err(BundleError::new(
            BundleErrorKind::Type,
            format!("scope-policy place has non-reference accepted type {accepted_type_schema}"),
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn canonical_type_schema(source: &str) -> Result<String, BundleError> {
    canonical_type_schema_impl(source, false)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn canonical_prelude_type_schema(
    source: &str,
    declared_type_parameters: &[String],
) -> Result<String, BundleError> {
    validate_type_parameter_declarations(declared_type_parameters)?;
    let canonical = canonical_type_schema_impl(source, true)?;
    let datum = parse_document(&canonical).expect("canonical type was just parsed");
    let used = collect_type_parameter_names(&datum)?;
    let declared = declared_type_parameters
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if used.iter().any(|name| !declared.contains(name.as_str())) {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "PreludeRow schema uses an undeclared TypeParam",
        ));
    }
    Ok(canonical)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn canonical_type_schema_impl(
    source: &str,
    allow_type_parameters: bool,
) -> Result<String, BundleError> {
    let datum = parse_document(source).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse type {source:?}: {error}"),
        )
    })?;
    StaticType::parse(&datum, allow_type_parameters)?;
    Ok(canonical_datum(&datum))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn canonical_row_schema(source: &str) -> Result<String, BundleError> {
    let row = parse_document(source).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse row {source:?}: {error}"),
        )
    })?;
    if row.form_head() != Some("Row") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "row schema must be a Row form",
        ));
    }
    let wrapped = Datum::form("PredTerm", [row.clone()]);
    TypeExpr::parse(&wrapped).map_err(|error| {
        BundleError::new(BundleErrorKind::Type, format!("validate row: {error}"))
    })?;
    Ok(canonical_datum(&row))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn canonical_type_for_comparison(source: &str) -> Result<String, BundleError> {
    let datum = parse_document(source).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse type comparison: {error}"),
        )
    })?;
    let parsed = StaticType::parse(&datum, true)?;
    Ok(canonical_datum(&parsed.to_datum()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|datum| !contains_form(datum, "TypeParam")) || ret.is_err())]
fn replace_type_parameters_for_syntax_validation(datum: &Datum) -> Result<Datum, BundleError> {
    match datum {
        Datum::List(values) if datum.form_head() == Some("TypeParam") => {
            StaticType::parse(datum, true)?;
            debug_assert_eq!(values.len(), 2);
            // Registry type parameters are rigid during semantic checking. A
            // concrete stand-in is used only to exercise the closed surface
            // syntax parser, which intentionally has no TypeParam production.
            Ok(Datum::atom("Entity"))
        }
        Datum::List(values) => Ok(Datum::list(
            values
                .iter()
                .map(replace_type_parameters_for_syntax_validation)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        _ => Ok(datum.clone()),
    }
}

#[invariant(is_type_parameter_name(name))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TypeParameterName {
    name: String,
}

#[invariant(::Concrete(_) => true)]
#[invariant(::TypeParameter(_) => true)]
#[invariant(::Referents(_) => true)]
#[invariant(::Set(_) => true)]
#[invariant(::Group(_) => true)]
#[invariant(::List(_) => true)]
#[invariant(::Interval(_) => true)]
#[invariant(::Tuple(_) => true)]
#[invariant(::Function { .. } => true)]
#[invariant(::Predicate(_) => true)]
#[invariant(::ReferenceComputation(_) => true)]
#[invariant(::Query(_) => true)]
#[invariant(::AnswerSelection(_) => true)]
#[invariant(::GeneralizedQuantifier(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticType {
    Concrete(TypeExpr),
    TypeParameter(TypeParameterName),
    Referents(Box<StaticType>),
    Set(Box<StaticType>),
    Group(Box<StaticType>),
    List(Box<StaticType>),
    Interval(Box<StaticType>),
    Tuple(Vec<StaticType>),
    Function {
        parameters: Vec<StaticType>,
        result: Box<StaticType>,
    },
    Predicate(StaticPredicate),
    ReferenceComputation(Box<StaticType>),
    Query(Vec<StaticType>),
    AnswerSelection(Vec<StaticType>),
    GeneralizedQuantifier(Box<StaticType>),
}

impl StaticType {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn parse(datum: &Datum, allow_type_parameters: bool) -> Result<Self, BundleError> {
        if datum.as_atom() == Some("T") {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "bare T is specification metanotation; registry schemas require (TypeParam \"T\")",
            ));
        }
        if datum.form_head() == Some("TypeParam") {
            if !allow_type_parameters {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    "TypeParam is allowed only in a PreludeRow schema",
                ));
            }
            let items = datum.as_list().expect("a form head belongs to a list");
            let name = items
                .get(1)
                .filter(|_| items.len() == 2)
                .and_then(Datum::as_string)
                .filter(|name| is_type_parameter_name(name))
                .ok_or_else(|| {
                    BundleError::new(
                        BundleErrorKind::Type,
                        "TypeParam requires exactly one canonical ASCII name string",
                    )
                })?;
            return Ok(Self::TypeParameter(new!(TypeParameterName {
                name: name.to_owned(),
            })));
        }
        if !contains_form(datum, "TypeParam") {
            return TypeExpr::parse(datum)
                .map(Self::from_concrete)
                .map_err(|error| {
                    BundleError::new(BundleErrorKind::Type, format!("validate type: {error}"))
                });
        }
        if !allow_type_parameters {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "TypeParam is allowed only in a PreludeRow schema",
            ));
        }
        let items = datum.as_list().ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Type,
                "type-parameterized constructor must be a list",
            )
        })?;
        let head = items.first().and_then(Datum::as_atom).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Type,
                "type-parameterized constructor must be named",
            )
        })?;
        match head {
            "Referents" | "Set" | "Group" | "List" | "Interval" | "RefComp" | "GQ"
                if items.len() == 2 =>
            {
                let inner = Box::new(Self::parse(&items[1], true)?);
                Ok(match head {
                    "Referents" => Self::Referents(inner),
                    "Set" => Self::Set(inner),
                    "Group" => Self::Group(inner),
                    "List" => Self::List(inner),
                    "Interval" => Self::Interval(inner),
                    "RefComp" => Self::ReferenceComputation(inner),
                    "GQ" => Self::GeneralizedQuantifier(inner),
                    _ => unreachable!("closed generic unary constructor"),
                })
            }
            "Tuple" | "Query" | "AnswerSelection" if items.len() == 2 => {
                let elements = items[1].as_list().ok_or_else(|| {
                    BundleError::new(BundleErrorKind::Type, "generic type tuple must be a list")
                })?;
                let elements = elements
                    .iter()
                    .map(|element| Self::parse(element, true))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(match head {
                    "Tuple" => Self::Tuple(elements),
                    "Query" => Self::Query(elements),
                    "AnswerSelection" => Self::AnswerSelection(elements),
                    _ => unreachable!("closed generic tuple constructor"),
                })
            }
            "Fn" if items.len() == 3 => {
                let parameters = items[1].as_list().ok_or_else(|| {
                    BundleError::new(BundleErrorKind::Type, "generic Fn parameters need a list")
                })?;
                Ok(Self::Function {
                    parameters: parameters
                        .iter()
                        .map(|parameter| Self::parse(parameter, true))
                        .collect::<Result<Vec<_>, _>>()?,
                    result: Box::new(Self::parse(&items[2], true)?),
                })
            }
            _ => Err(BundleError::new(
                BundleErrorKind::Type,
                format!("TypeParam occurs in unsupported constructor {head}"),
            )),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_concrete(value: TypeExpr) -> Self {
        match value {
            TypeExpr::Referents(inner) => Self::Referents(Box::new(Self::from_concrete(*inner))),
            TypeExpr::Set(inner) => Self::Set(Box::new(Self::from_concrete(*inner))),
            TypeExpr::Group(inner) => Self::Group(Box::new(Self::from_concrete(*inner))),
            TypeExpr::List(inner) => Self::List(Box::new(Self::from_concrete(*inner))),
            TypeExpr::Interval(inner) => Self::Interval(Box::new(Self::from_concrete(*inner))),
            TypeExpr::Tuple(elements) => {
                Self::Tuple(elements.into_iter().map(Self::from_concrete).collect())
            }
            TypeExpr::Function { parameters, result } => Self::Function {
                parameters: parameters.into_iter().map(Self::from_concrete).collect(),
                result: Box::new(Self::from_concrete(*result)),
            },
            TypeExpr::Predicate(row) => {
                Self::Predicate(StaticPredicate::from_row(row, ClosePolicy::Required))
            }
            TypeExpr::ReferenceComputation(inner) => {
                Self::ReferenceComputation(Box::new(Self::from_concrete(*inner)))
            }
            TypeExpr::Query(elements) => {
                Self::Query(elements.into_iter().map(Self::from_concrete).collect())
            }
            TypeExpr::AnswerSelection(elements) => {
                Self::AnswerSelection(elements.into_iter().map(Self::from_concrete).collect())
            }
            TypeExpr::GeneralizedQuantifier(inner) => {
                Self::GeneralizedQuantifier(Box::new(Self::from_concrete(*inner)))
            }
            concrete => Self::Concrete(concrete),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn to_datum(&self) -> Datum {
        match self {
            Self::Concrete(value) => value.to_datum(),
            Self::TypeParameter(name) => {
                Datum::form("TypeParam", [Datum::string(name.name.clone())])
            }
            Self::Referents(inner) => Datum::form("Referents", [inner.to_datum()]),
            Self::Set(inner) => Datum::form("Set", [inner.to_datum()]),
            Self::Group(inner) => Datum::form("Group", [inner.to_datum()]),
            Self::List(inner) => Datum::form("List", [inner.to_datum()]),
            Self::Interval(inner) => Datum::form("Interval", [inner.to_datum()]),
            Self::Tuple(elements) => {
                Datum::form("Tuple", [Datum::list(elements.iter().map(Self::to_datum))])
            }
            Self::Function { parameters, result } => Datum::form(
                "Fn",
                [
                    Datum::list(parameters.iter().map(Self::to_datum)),
                    result.to_datum(),
                ],
            ),
            Self::Predicate(predicate) => TypeExpr::Predicate(predicate.row.clone()).to_datum(),
            Self::ReferenceComputation(inner) => Datum::form("RefComp", [inner.to_datum()]),
            Self::Query(elements) => {
                Datum::form("Query", [Datum::list(elements.iter().map(Self::to_datum))])
            }
            Self::AnswerSelection(elements) => Datum::form(
                "AnswerSelection",
                [Datum::list(elements.iter().map(Self::to_datum))],
            ),
            Self::GeneralizedQuantifier(inner) => Datum::form("GQ", [inner.to_datum()]),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn to_concrete(&self) -> Option<TypeExpr> {
        Some(match self {
            Self::Concrete(value) => value.clone(),
            Self::TypeParameter(_) => return None,
            Self::Referents(inner) => TypeExpr::Referents(Box::new(inner.to_concrete()?)),
            Self::Set(inner) => TypeExpr::Set(Box::new(inner.to_concrete()?)),
            Self::Group(inner) => TypeExpr::Group(Box::new(inner.to_concrete()?)),
            Self::List(inner) => TypeExpr::List(Box::new(inner.to_concrete()?)),
            Self::Interval(inner) => TypeExpr::Interval(Box::new(inner.to_concrete()?)),
            Self::Tuple(elements) => TypeExpr::Tuple(
                elements
                    .iter()
                    .map(Self::to_concrete)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Self::Function { parameters, result } => TypeExpr::Function {
                parameters: parameters
                    .iter()
                    .map(Self::to_concrete)
                    .collect::<Option<Vec<_>>>()?,
                result: Box::new(result.to_concrete()?),
            },
            Self::Predicate(predicate) => TypeExpr::Predicate(predicate.row.clone()),
            Self::ReferenceComputation(inner) => {
                TypeExpr::ReferenceComputation(Box::new(inner.to_concrete()?))
            }
            Self::Query(elements) => TypeExpr::Query(
                elements
                    .iter()
                    .map(Self::to_concrete)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Self::AnswerSelection(elements) => TypeExpr::AnswerSelection(
                elements
                    .iter()
                    .map(Self::to_concrete)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Self::GeneralizedQuantifier(inner) => {
                TypeExpr::GeneralizedQuantifier(Box::new(inner.to_concrete()?))
            }
        })
    }
}

#[invariant(close_policies.len() == row.slots().len())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StaticPredicate {
    row: Row,
    close_policies: Vec<ClosePolicy>,
}

impl StaticPredicate {
    #[requires(true)]
    #[ensures(ret.close_policies.len() == ret.row.slots().len())]
    fn from_row(row: Row, policy: ClosePolicy) -> Self {
        let close_policies = vec![policy; row.slots().len()];
        new!(StaticPredicate {
            row,
            close_policies,
        })
    }

    #[requires(row.slots().len() == close_policies.len())]
    #[ensures(ret.close_policies.len() == ret.row.slots().len())]
    fn new(row: Row, close_policies: Vec<ClosePolicy>) -> Self {
        new!(StaticPredicate {
            row,
            close_policies,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_closeable(&self) -> bool {
        !self.row.has_open_numbered_tail()
            && self.row.slots().iter().zip(&self.close_policies).all(
                |(slot, policy)| match policy {
                    ClosePolicy::Required => false,
                    ClosePolicy::Contextual => {
                        matches!(slot.accepted_type(), TypeExpr::Referents(_))
                    }
                    ClosePolicy::LocalExistential => {
                        slot.label() == PlaceLabel::Eventuality
                            && matches!(
                                slot.accepted_type(),
                                TypeExpr::Referents(inner)
                                    if inner.as_ref() == &TypeExpr::Atom(TypeAtom::Eventuality)
                            )
                    }
                },
            )
    }
}

#[requires(true)]
#[ensures(true)]
fn static_type_contains_parameter(value: &StaticType) -> bool {
    match value {
        StaticType::TypeParameter(_) => true,
        StaticType::Referents(inner)
        | StaticType::Set(inner)
        | StaticType::Group(inner)
        | StaticType::List(inner)
        | StaticType::Interval(inner)
        | StaticType::ReferenceComputation(inner)
        | StaticType::GeneralizedQuantifier(inner) => static_type_contains_parameter(inner),
        StaticType::Tuple(elements)
        | StaticType::Query(elements)
        | StaticType::AnswerSelection(elements) => {
            elements.iter().any(static_type_contains_parameter)
        }
        StaticType::Function { parameters, result } => {
            parameters.iter().any(static_type_contains_parameter)
                || static_type_contains_parameter(result)
        }
        StaticType::Concrete(_) | StaticType::Predicate(_) => false,
    }
}

#[requires(true)]
#[ensures(ret || actual != expected)]
fn unify_static_types(
    actual: &StaticType,
    expected: &StaticType,
    substitutions: &mut BTreeMap<String, StaticType>,
) -> bool {
    if actual == expected {
        return true;
    }
    if let StaticType::TypeParameter(name) = expected {
        if let Some(bound) = substitutions.get(&name.name).cloned() {
            return unify_static_types(actual, &bound, substitutions);
        }
        return bind_static_parameter(&name.name, actual, substitutions);
    }
    if let StaticType::TypeParameter(name) = actual {
        if let Some(bound) = substitutions.get(&name.name).cloned() {
            return unify_static_types(&bound, expected, substitutions);
        }
        return bind_static_parameter(&name.name, expected, substitutions);
    }
    let structural = match (actual, expected) {
        (StaticType::Referents(left), StaticType::Referents(right))
        | (StaticType::Set(left), StaticType::Set(right))
        | (StaticType::Group(left), StaticType::Group(right))
        | (StaticType::List(left), StaticType::List(right))
        | (StaticType::Interval(left), StaticType::Interval(right))
        | (StaticType::ReferenceComputation(left), StaticType::ReferenceComputation(right))
        | (StaticType::GeneralizedQuantifier(left), StaticType::GeneralizedQuantifier(right)) => {
            Some(unify_static_types(left, right, substitutions))
        }
        (StaticType::Tuple(left), StaticType::Tuple(right))
        | (StaticType::Query(left), StaticType::Query(right))
        | (StaticType::AnswerSelection(left), StaticType::AnswerSelection(right)) => {
            Some(unify_static_type_lists(left, right, substitutions))
        }
        (
            StaticType::Function {
                parameters: left_parameters,
                result: left_result,
            },
            StaticType::Function {
                parameters: right_parameters,
                result: right_result,
            },
        ) => Some(
            unify_static_type_lists(left_parameters, right_parameters, substitutions)
                && unify_static_types(left_result, right_result, substitutions),
        ),
        (StaticType::Predicate(left), StaticType::Predicate(right)) => Some(left.row == right.row),
        _ => None,
    };
    if let Some(result) = structural {
        return result;
    }
    actual
        .to_concrete()
        .zip(expected.to_concrete())
        .is_some_and(|(actual, expected)| actual.implicit_conversion_to(&expected).is_some())
}

#[requires(true)]
#[ensures(ret || left.len() != right.len() || left != right)]
fn unify_static_type_lists(
    left: &[StaticType],
    right: &[StaticType],
    substitutions: &mut BTreeMap<String, StaticType>,
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| unify_static_types(left, right, substitutions))
}

#[requires(true)]
#[ensures(ret || static_type_contains_parameter(value) || substitutions.contains_key(name))]
fn bind_static_parameter(
    name: &str,
    value: &StaticType,
    substitutions: &mut BTreeMap<String, StaticType>,
) -> bool {
    debug_assert!(!substitutions.contains_key(name));
    if static_type_contains_parameter(value) {
        return false;
    }
    substitutions.insert(name.to_owned(), value.clone());
    true
}

#[requires(true)]
#[ensures(true)]
fn substitute_static_type(
    value: &StaticType,
    substitutions: &BTreeMap<String, StaticType>,
) -> StaticType {
    match value {
        StaticType::TypeParameter(name) => substitutions.get(&name.name).map_or_else(
            || value.clone(),
            |bound| substitute_static_type(bound, substitutions),
        ),
        StaticType::Referents(inner) => {
            StaticType::Referents(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::Set(inner) => {
            StaticType::Set(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::Group(inner) => {
            StaticType::Group(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::List(inner) => {
            StaticType::List(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::Interval(inner) => {
            StaticType::Interval(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::ReferenceComputation(inner) => {
            StaticType::ReferenceComputation(Box::new(substitute_static_type(inner, substitutions)))
        }
        StaticType::GeneralizedQuantifier(inner) => StaticType::GeneralizedQuantifier(Box::new(
            substitute_static_type(inner, substitutions),
        )),
        StaticType::Tuple(elements) => StaticType::Tuple(
            elements
                .iter()
                .map(|element| substitute_static_type(element, substitutions))
                .collect(),
        ),
        StaticType::Query(elements) => StaticType::Query(
            elements
                .iter()
                .map(|element| substitute_static_type(element, substitutions))
                .collect(),
        ),
        StaticType::AnswerSelection(elements) => StaticType::AnswerSelection(
            elements
                .iter()
                .map(|element| substitute_static_type(element, substitutions))
                .collect(),
        ),
        StaticType::Function { parameters, result } => StaticType::Function {
            parameters: parameters
                .iter()
                .map(|parameter| substitute_static_type(parameter, substitutions))
                .collect(),
            result: Box::new(substitute_static_type(result, substitutions)),
        },
        StaticType::Concrete(_) | StaticType::Predicate(_) => value.clone(),
    }
}

#[invariant(true)]
struct StaticTypeRegistry {
    lexical: BTreeMap<String, StaticType>,
    prelude: BTreeMap<String, StaticType>,
}

impl StaticTypeRegistry {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|registry| registry.lexical.len() == lexical.len() && registry.prelude.len() == prelude.len()) || ret.is_err())]
    fn from_rows(lexical: &[LexicalRow], prelude: &[PreludeRow]) -> Result<Self, BundleError> {
        let mut lexical_types = BTreeMap::new();
        for row in lexical {
            let mut slots = row.ordered_numbered_slot_rows.clone();
            if let Some(event) = &row.optional_event_slot_row {
                slots.push(event.clone());
            }
            let row_type = TypeExpr::parse(&Datum::form(
                "PredTerm",
                [parse_document(&row_schema(&slots)?)
                    .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?],
            ))
            .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?;
            let TypeExpr::Predicate(predicate_row) = row_type else {
                unreachable!("PredTerm schema parses as a predicate")
            };
            lexical_types.insert(
                row.normalized_root.clone(),
                StaticType::Predicate(StaticPredicate::new(
                    predicate_row,
                    slots.iter().map(|slot| slot.close_policy).collect(),
                )),
            );
        }
        let mut prelude_types = BTreeMap::new();
        for row in prelude {
            let signature = canonical_prelude_type_schema(
                &row.complete_signature_schema,
                &row.type_parameters,
            )?;
            let datum = parse_document(&signature)
                .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?;
            prelude_types.insert(row.name.clone(), StaticType::parse(&datum, true)?);
        }
        Ok(Self {
            lexical: lexical_types,
            prelude: prelude_types,
        })
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_expression(
    datum: &Datum,
    expected: &StaticType,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<(), BundleError> {
    let mut substitutions = BTreeMap::new();
    check_expression_with_substitutions(datum, expected, environment, registry, &mut substitutions)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_expression_with_substitutions(
    datum: &Datum,
    expected: &StaticType,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
    substitutions: &mut BTreeMap<String, StaticType>,
) -> Result<(), BundleError> {
    let context_dependencies = if datum.as_atom() == Some("Context") {
        Some(&[][..])
    } else {
        datum
            .as_list()
            .filter(|items| items.first().and_then(Datum::as_atom) == Some("Context"))
            .map(|items| &items[1..])
    };
    if let Some(dependencies) = context_dependencies {
        if dependencies.is_empty() && datum.form_head() == Some("Context") {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "zero-dependency Context must use the canonical bare atom",
            ));
        }
        if !matches!(
            expected,
            StaticType::ReferenceComputation(inner)
                if matches!(inner.as_ref(), StaticType::Referents(_))
        ) {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "Context requires an expected RefComp<Referents<T>> type",
            ));
        }
        for dependency in dependencies {
            infer_expression(dependency, environment, registry)?;
        }
        return Ok(());
    }
    if let StaticType::Function { parameters, result } = expected
        && datum.form_head() == Some("λ")
    {
        return check_lambda(
            datum,
            parameters,
            result,
            environment,
            registry,
            substitutions,
        );
    }
    if let StaticType::GeneralizedQuantifier(inner) = expected
        && datum.form_head() == Some("λ")
    {
        let property = StaticType::Function {
            parameters: vec![inner.as_ref().clone()],
            result: Box::new(StaticType::from_concrete(TypeExpr::Atom(TypeAtom::Content))),
        };
        return check_lambda(
            datum,
            &[property],
            &StaticType::from_concrete(TypeExpr::Atom(TypeAtom::Content)),
            environment,
            registry,
            substitutions,
        );
    }
    let actual = infer_expression(datum, environment, registry)?;
    if matches!(actual, StaticType::Predicate(ref predicate) if predicate.is_closeable())
        && expected == &StaticType::from_concrete(TypeExpr::Atom(TypeAtom::Content))
    {
        return Ok(());
    }
    if unify_static_types(&actual, expected, substitutions) {
        Ok(())
    } else {
        Err(BundleError::new(
            BundleErrorKind::Type,
            format!(
                "expression {} has type {}, expected {}",
                canonical_datum(datum),
                canonical_datum(&actual.to_datum()),
                canonical_datum(&expected.to_datum())
            ),
        ))
    }
}

#[requires(datum.form_head() == Some("λ"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_lambda(
    datum: &Datum,
    expected_parameters: &[StaticType],
    expected_result: &StaticType,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
    substitutions: &mut BTreeMap<String, StaticType>,
) -> Result<(), BundleError> {
    let items = datum
        .as_list()
        .expect("lambda precondition requires a list");
    if items.len() != 3 {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "lambda requires declarations and body",
        ));
    }
    let declarations = typed_declarations(&items[1])?;
    if declarations.len() != expected_parameters.len() {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "lambda parameter list differs from the expected function type",
        ));
    }
    for ((_, actual), expected) in declarations.iter().zip(expected_parameters) {
        if !unify_static_types(actual, expected, substitutions) {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "lambda parameter type differs from the expected function type",
            ));
        }
    }
    let mut body_environment = environment.clone();
    body_environment.extend(declarations);
    check_expression_with_substitutions(
        &items[2],
        expected_result,
        &body_environment,
        registry,
        substitutions,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|declarations| declarations.iter().all(|(name, _)| name.starts_with('$'))) || ret.is_err())]
fn typed_declarations(datum: &Datum) -> Result<Vec<(String, StaticType)>, BundleError> {
    let declarations = datum.as_list().ok_or_else(|| {
        BundleError::new(BundleErrorKind::Type, "lambda declarations must be a list")
    })?;
    let mut result = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let declaration = declaration.as_list().ok_or_else(|| {
            BundleError::new(BundleErrorKind::Type, "lambda declaration must be a pair")
        })?;
        if declaration.len() != 2 {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "lambda declaration must contain one variable and type",
            ));
        }
        let variable = declaration[0].as_atom().ok_or_else(|| {
            BundleError::new(BundleErrorKind::Type, "lambda binder must be a variable")
        })?;
        if !variable.starts_with('$') {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                "lambda binder must use the variable namespace",
            ));
        }
        let ty = StaticType::parse(&declaration[1], true)?;
        result.push((variable.to_owned(), ty));
    }
    reject_duplicate(
        result.iter().map(|(name, _)| name.as_str()),
        "lambda binder",
    )?;
    Ok(result)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_expression(
    datum: &Datum,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if let Some(text) = datum.as_string() {
        let _ = text;
        return Ok(static_atom(TypeAtom::Text));
    }
    if let Some(integer) = datum.as_integer() {
        return Ok(static_atom(if integer.starts_with('-') {
            TypeAtom::Number
        } else {
            TypeAtom::Natural
        }));
    }
    if let Some(atom) = datum.as_atom() {
        if let Some(ty) = environment.get(atom) {
            return Ok(ty.clone());
        }
        if let Some(ty) = registry.prelude.get(atom) {
            return Ok(ty.clone());
        }
        if let Some(ty) = registry.lexical.get(atom) {
            return Ok(ty.clone());
        }
        return infer_constant(atom);
    }
    let items = datum.as_list().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Type,
            "empty list is not a typed expression",
        )
    })?;
    if items.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "empty list is not a typed expression",
        ));
    }
    match datum.form_head() {
        Some("λ") => infer_lambda(datum, environment, registry),
        Some("Bind") => infer_bind(items, environment, registry),
        Some("Refer") => infer_refer(items, environment, registry),
        Some("SetOf") => infer_set_of(items, environment, registry),
        Some("Card") => infer_card(items, environment, registry),
        Some("Deictic") => {
            if items.len() != 3 {
                return Err(BundleError::new(BundleErrorKind::Type, "Deictic arity"));
            }
            check_expression(
                &items[1],
                &static_atom(TypeAtom::Proximity),
                environment,
                registry,
            )?;
            check_expression(
                &items[2],
                &static_atom(TypeAtom::DeicticGround),
                environment,
                registry,
            )?;
            Ok(StaticType::Referents(Box::new(static_atom(
                TypeAtom::Entity,
            ))))
        }
        Some("NameSign") => {
            if items.len() != 2 {
                return Err(BundleError::new(BundleErrorKind::Type, "NameSign arity"));
            }
            check_expression(
                &items[1],
                &static_atom(TypeAtom::Text),
                environment,
                registry,
            )?;
            Ok(StaticType::from_concrete(TypeExpr::Sign(SignKind::Name)))
        }
        Some("¬") => {
            check_content_operands(&items[1..], 1, environment, registry)?;
            Ok(static_atom(TypeAtom::Content))
        }
        Some("∧" | "∨" | "→" | "↔" | "⊕" | "Joi") => {
            check_content_operands(&items[1..], 2, environment, registry)?;
            Ok(static_atom(TypeAtom::Content))
        }
        Some("Presuppose" | "Supplement") => {
            check_content_operands(&items[1..], 2, environment, registry)?;
            if items.len() != 3 {
                return Err(BundleError::new(
                    BundleErrorKind::Type,
                    "binary effect arity",
                ));
            }
            Ok(static_atom(TypeAtom::Content))
        }
        Some("∀" | "∃") => infer_quantifier(items, environment, registry),
        Some("=" | "<" | "≤" | "∈") => infer_binary_relation(items, environment, registry),
        Some("DropPlace") => infer_drop_place(items, environment, registry),
        _ => infer_application(items, environment, registry),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_constant(atom: &str) -> Result<StaticType, BundleError> {
    let ty = match atom {
        "Speaker" | "Audience" => StaticType::Referents(Box::new(static_atom(TypeAtom::Entity))),
        "Now" => StaticType::Referents(Box::new(static_atom(TypeAtom::Eventuality))),
        "Here" => StaticType::Referents(Box::new(static_atom(TypeAtom::Location))),
        "CurrentGround" => static_atom(TypeAtom::DeicticGround),
        "Proximal" | "Medial" | "Distal" => static_atom(TypeAtom::Proximity),
        "DistanceScale" => static_atom(TypeAtom::Scale),
        _ => {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("unregistered value atom {atom}"),
            ));
        }
    };
    Ok(ty)
}

#[requires(true)]
#[ensures(matches!(ret, StaticType::Concrete(TypeExpr::Atom(value)) if value == atom))]
fn static_atom(atom: TypeAtom) -> StaticType {
    StaticType::Concrete(TypeExpr::Atom(atom))
}

#[requires(is_type_parameter_name(name))]
#[ensures(matches!(ret, StaticType::TypeParameter(_)))]
fn static_type_parameter(name: &str) -> StaticType {
    StaticType::TypeParameter(new!(TypeParameterName {
        name: name.to_owned(),
    }))
}

#[requires(datum.form_head() == Some("λ"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_lambda(
    datum: &Datum,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    let items = datum.as_list().expect("lambda is a list");
    if items.len() != 3 {
        return Err(BundleError::new(BundleErrorKind::Type, "lambda arity"));
    }
    let declarations = typed_declarations(&items[1])?;
    let mut body_environment = environment.clone();
    body_environment.extend(declarations.iter().cloned());
    let result = infer_expression(&items[2], &body_environment, registry)?;
    Ok(StaticType::Function {
        parameters: declarations.into_iter().map(|(_, ty)| ty).collect(),
        result: Box::new(result),
    })
}

#[requires(items.first().and_then(Datum::as_atom) == Some("Bind"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_bind(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 3 {
        return Err(BundleError::new(BundleErrorKind::Type, "Bind arity"));
    }
    let bindings = items[1]
        .as_list()
        .filter(|bindings| bindings.len() == 1)
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "Bind requires one binding"))?;
    let binding = bindings[0]
        .as_list()
        .filter(|binding| binding.len() == 3)
        .ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::Type,
                "Bind binding must have variable/type/value",
            )
        })?;
    let variable = binding[0]
        .as_atom()
        .filter(|atom| atom.starts_with('$'))
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "Bind binder must be a variable"))?;
    let value_type = StaticType::parse(&binding[1], true)?;
    check_expression(
        &binding[2],
        &StaticType::ReferenceComputation(Box::new(value_type.clone())),
        environment,
        registry,
    )?;
    let mut body_environment = environment.clone();
    body_environment.insert(variable.to_owned(), value_type);
    infer_expression(&items[2], &body_environment, registry)
}

#[requires(items.first().and_then(Datum::as_atom) == Some("Refer"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_refer(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 2 || items[1].form_head() != Some("λ") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "Refer requires one property",
        ));
    }
    let lambda = items[1].as_list().expect("lambda is a list");
    let declarations = typed_declarations(&lambda[1])?;
    if declarations.len() != 1 {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "Refer property arity",
        ));
    }
    check_lambda(
        &items[1],
        &[declarations[0].1.clone()],
        &static_atom(TypeAtom::Content),
        environment,
        registry,
        &mut BTreeMap::new(),
    )?;
    Ok(StaticType::ReferenceComputation(Box::new(
        declarations[0].1.clone(),
    )))
}

#[requires(items.first().and_then(Datum::as_atom) == Some("SetOf"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_set_of(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 2 || items[1].form_head() != Some("λ") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "SetOf requires one property",
        ));
    }
    let lambda = items[1].as_list().expect("lambda is a list");
    let declarations = typed_declarations(&lambda[1])?;
    if declarations.len() != 1 {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "SetOf property arity",
        ));
    }
    check_lambda(
        &items[1],
        &[declarations[0].1.clone()],
        &static_atom(TypeAtom::Content),
        environment,
        registry,
        &mut BTreeMap::new(),
    )?;
    Ok(StaticType::Set(Box::new(declarations[0].1.clone())))
}

#[requires(items.first().and_then(Datum::as_atom) == Some("Card"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_card(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 2
        || !matches!(
            infer_expression(&items[1], environment, registry)?,
            StaticType::Set(_) | StaticType::List(_)
        )
    {
        return Err(BundleError::new(BundleErrorKind::Type, "Card operand type"));
    }
    Ok(static_atom(TypeAtom::Cardinal))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn check_content_operands(
    operands: &[Datum],
    minimum: usize,
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<(), BundleError> {
    if operands.len() < minimum {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "content connective arity",
        ));
    }
    for operand in operands {
        check_expression(
            operand,
            &static_atom(TypeAtom::Content),
            environment,
            registry,
        )?;
    }
    Ok(())
}

#[requires(matches!(items.first().and_then(Datum::as_atom), Some("∀" | "∃")))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_quantifier(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 2 || items[1].form_head() != Some("λ") {
        return Err(BundleError::new(BundleErrorKind::Type, "quantifier arity"));
    }
    let lambda = items[1].as_list().expect("lambda is a list");
    let declarations = typed_declarations(&lambda[1])?;
    if declarations.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "quantifier binder arity",
        ));
    }
    check_lambda(
        &items[1],
        &declarations
            .iter()
            .map(|(_, ty)| ty.clone())
            .collect::<Vec<_>>(),
        &static_atom(TypeAtom::Content),
        environment,
        registry,
        &mut BTreeMap::new(),
    )?;
    Ok(static_atom(TypeAtom::Content))
}

#[requires(matches!(items.first().and_then(Datum::as_atom), Some("=" | "<" | "≤" | "∈")))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_binary_relation(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 3 {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "binary relation arity",
        ));
    }
    let left = infer_expression(&items[1], environment, registry)?;
    if items[0].as_atom() == Some("∈") {
        check_expression(
            &items[2],
            &StaticType::Set(Box::new(left)),
            environment,
            registry,
        )?;
    } else {
        check_expression(&items[2], &left, environment, registry)?;
    }
    Ok(static_atom(TypeAtom::Content))
}

#[requires(items.first().and_then(Datum::as_atom) == Some("DropPlace"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_drop_place(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    if items.len() != 3 {
        return Err(BundleError::new(BundleErrorKind::Type, "DropPlace arity"));
    }
    let StaticType::Predicate(predicate) = infer_expression(&items[1], environment, registry)?
    else {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "DropPlace relation type",
        ));
    };
    let ordinal = items[2]
        .as_integer()
        .and_then(|text| PositiveInteger::try_new(text).ok())
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "DropPlace ordinal"))?;
    let label = PlaceLabel::Numbered(ordinal);
    let data!(StaticPredicate {
        row,
        mut close_policies,
    }) = predicate.into_data();
    let mut slots = row.slots().to_vec();
    let index = slots
        .iter()
        .position(|slot| slot.label() == label)
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "DropPlace target absent"))?;
    slots.remove(index);
    close_policies.remove(index);
    Ok(StaticType::Predicate(StaticPredicate::new(
        Row::new(slots, row.has_open_numbered_tail()),
        close_policies,
    )))
}

#[requires(!items.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_application(
    items: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    let head = infer_expression(&items[0], environment, registry)?;
    match head {
        StaticType::Function { parameters, result } => {
            if parameters.len() != items.len() - 1 {
                return Err(BundleError::new(BundleErrorKind::Type, "function arity"));
            }
            let mut substitutions = BTreeMap::new();
            for (argument, expected) in items[1..].iter().zip(&parameters) {
                check_expression_with_substitutions(
                    argument,
                    expected,
                    environment,
                    registry,
                    &mut substitutions,
                )?;
            }
            Ok(substitute_static_type(&result, &substitutions))
        }
        StaticType::Predicate(predicate) => {
            infer_predicate_application(predicate, &items[1..], environment, registry)
        }
        _ => Err(BundleError::new(
            BundleErrorKind::Type,
            "application head is neither Fn nor PredTerm",
        )),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn infer_predicate_application(
    predicate: StaticPredicate,
    arguments: &[Datum],
    environment: &BTreeMap<String, StaticType>,
    registry: &StaticTypeRegistry,
) -> Result<StaticType, BundleError> {
    let data!(StaticPredicate {
        row,
        mut close_policies,
    }) = predicate.into_data();
    let mut remaining = row.slots().to_vec();
    let mut cursor_after = None::<PositiveInteger>;
    let mut index = 0;
    while index < arguments.len() {
        let labelled = arguments[index]
            .as_atom()
            .and_then(|atom| atom.strip_prefix(':'));
        let (slot_index, labelled_number) = if let Some(label) = labelled {
            index += 1;
            if index == arguments.len() {
                return Err(BundleError::new(BundleErrorKind::Type, "label lacks value"));
            }
            let requested = if label == "Eventuality" {
                PlaceLabel::Eventuality
            } else {
                PlaceLabel::Numbered(PositiveInteger::try_new(label).map_err(|_| {
                    BundleError::new(BundleErrorKind::Type, "invalid labelled place")
                })?)
            };
            let slot_index = remaining
                .iter()
                .position(|slot| slot.label() == requested)
                .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "labelled place absent"))?;
            let numbered = match remaining[slot_index].label() {
                PlaceLabel::Numbered(number) => Some(number),
                PlaceLabel::Eventuality => None,
            };
            (slot_index, numbered)
        } else {
            let slot_index = remaining
                .iter()
                .position(|slot| {
                    matches!(slot.label(), PlaceLabel::Numbered(ref place) if cursor_after.as_ref().is_none_or(|cursor| place > cursor))
                })
                .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "ordinary place absent"))?;
            let PlaceLabel::Numbered(number) = remaining[slot_index].label() else {
                unreachable!("plain cursor selects only numbered slots")
            };
            (slot_index, Some(number))
        };
        let slot = remaining.remove(slot_index);
        close_policies.remove(slot_index);
        check_expression(
            &arguments[index],
            &StaticType::from_concrete(slot.accepted_type().clone()),
            environment,
            registry,
        )?;
        if let Some(number) = labelled_number {
            cursor_after = Some(number);
        }
        index += 1;
    }
    Ok(StaticType::Predicate(StaticPredicate::new(
        Row::new(remaining, row.has_open_numbered_tail()),
        close_policies,
    )))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_typed_template(
    template: &str,
    expected: &str,
    lexical: &[LexicalRow],
    prelude: &[PreludeRow],
) -> Result<(), BundleError> {
    let datum = parse_document(template)
        .map_err(|error| BundleError::new(BundleErrorKind::Template, error.to_string()))?;
    let mut holes = BTreeMap::new();
    let expression = substitute_holes(&datum, &mut holes)?;
    let mut environment = BTreeMap::new();
    for (name, schema) in holes {
        let ty = StaticType::parse(
            &parse_document(&schema)
                .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?,
            false,
        )?;
        environment.insert(format!("$registry_{name}"), ty);
    }
    let expected = StaticType::parse(
        &parse_document(expected)
            .map_err(|error| BundleError::new(BundleErrorKind::Type, error.to_string()))?,
        false,
    )?;
    let registry = StaticTypeRegistry::from_rows(lexical, prelude)?;
    check_expression(&expression, &expected, &environment, &registry)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn canonical_template(
    source: &str,
    lexical: &[LexicalRow],
    prelude: &[PreludeRow],
) -> Result<String, BundleError> {
    let canonical = canonical_template_without_registry(source)?;
    let datum = parse_document(&canonical).expect("canonical template was just parsed");
    let lexical = lexical
        .iter()
        .map(|row| row.normalized_root.as_str())
        .collect::<BTreeSet<_>>();
    let prelude = prelude
        .iter()
        .map(|row| row.name.as_str())
        .collect::<BTreeSet<_>>();
    validate_template_heads(&datum, &lexical, &prelude)?;
    Ok(canonical)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn canonical_template_without_registry(source: &str) -> Result<String, BundleError> {
    let datum = parse_document(source).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse template: {error}"),
        )
    })?;
    let mut holes = BTreeMap::new();
    let substituted = substitute_holes(&datum, &mut holes)?;
    if holes.is_empty() {
        return Err(BundleError::new(
            BundleErrorKind::Template,
            "registry template must declare at least one typed Hole",
        ));
    }
    parse_v0_expression(&canonical_datum(&substituted)).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("expanded template is not ordinary v0 syntax: {error}"),
        )
    })?;
    Ok(canonical_datum(&datum))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn substitute_holes(
    datum: &Datum,
    holes: &mut BTreeMap<String, String>,
) -> Result<Datum, BundleError> {
    if datum.form_head() == Some("Hole") {
        let items = datum.as_list().expect("form head requires a list");
        if items.len() != 3 {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                "Hole requires exactly name and type",
            ));
        }
        let name = items[1].as_string().ok_or_else(|| {
            BundleError::new(BundleErrorKind::Template, "Hole name must be a string")
        })?;
        if !is_hole_name(name) {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("invalid Hole name {name:?}"),
            ));
        }
        let schema = canonical_datum(&items[2]);
        canonical_type_schema(&schema)?;
        if holes.insert(name.to_owned(), schema).is_some() {
            return Err(BundleError::new(
                BundleErrorKind::Template,
                format!("duplicate Hole name {name}"),
            ));
        }
        return Ok(Datum::atom(format!("$registry_{name}")));
    }
    match datum {
        Datum::List(values) => Ok(Datum::list(
            values
                .iter()
                .map(|value| substitute_holes(value, holes))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        _ => Ok(datum.clone()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_template_heads(
    datum: &Datum,
    lexical: &BTreeSet<&str>,
    prelude: &BTreeSet<&str>,
) -> Result<(), BundleError> {
    if let Some(items) = datum.as_list() {
        if datum.form_head() == Some("Hole") {
            return Ok(());
        }
        if let Some(head) = items.first().and_then(Datum::as_atom)
            && is_lexical_root(head)
            && !lexical.contains(head)
        {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("template uses unsupported lexical root {head}"),
            ));
        }
        if let Some(head) = items.first().and_then(Datum::as_atom)
            && head.chars().next().is_some_and(char::is_uppercase)
            && matches!(
                head,
                "DescribedAs" | "InnatelyCapable" | "MotionVector" | "Named"
            )
            && !prelude.contains(head)
        {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("template uses unavailable prelude {head}"),
            ));
        }
        for item in items {
            validate_template_heads(item, lexical, prelude)?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn derive_template_result(template: &str) -> Result<String, BundleError> {
    let datum = parse_document(template).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse derived template: {error}"),
        )
    })?;
    if datum.form_head() == Some("Hole") {
        let items = datum.as_list().expect("Hole is a list");
        return canonical_type_schema(&canonical_datum(&items[2]));
    }
    let head = datum.form_head().ok_or_else(|| {
        BundleError::new(
            BundleErrorKind::Type,
            "template result cannot be derived from a scalar",
        )
    })?;
    if matches!(
        head,
        "Joi"
            | "DescribedAs"
            | "InnatelyCapable"
            | "MotionVector"
            | "Presuppose"
            | "Supplement"
            | "∧"
            | "∨"
            | "¬"
    ) || is_lexical_root(head)
    {
        return Ok("Content".to_owned());
    }
    Err(BundleError::new(
        BundleErrorKind::Type,
        format!("no closed result derivation for template head {head}"),
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn canonical_prelude_definition(
    source: &str,
    declared_type_parameters: &[String],
) -> Result<String, BundleError> {
    validate_type_parameter_declarations(declared_type_parameters)?;
    let datum = parse_document(source).map_err(|error| {
        BundleError::new(BundleErrorKind::Template, format!("parse prelude: {error}"))
    })?;
    if contains_atom(&datum, "Hole") {
        return Err(BundleError::new(
            BundleErrorKind::Template,
            "Hole is forbidden in a PreludeRow definition",
        ));
    }
    let declared = declared_type_parameters
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if collect_type_parameter_names(&datum)?
        .iter()
        .any(|name| !declared.contains(name.as_str()))
    {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "PreludeRow definition uses an undeclared TypeParam",
        ));
    }
    let instantiated = replace_type_parameters_for_syntax_validation(&datum)?;
    parse_v0_expression(&canonical_datum(&instantiated)).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("prelude is not ordinary v0 syntax after monomorphization: {error}"),
        )
    })?;
    Ok(canonical_datum(&datum))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_prelude_type_parameter_usage(
    declared: &[String],
    signature: &str,
    definition: &str,
) -> Result<(), BundleError> {
    validate_type_parameter_declarations(declared)?;
    let signature = parse_document(signature).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse PreludeRow signature type parameters: {error}"),
        )
    })?;
    let definition = parse_document(definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse PreludeRow definition type parameters: {error}"),
        )
    })?;
    let mut used = collect_type_parameter_names(&signature)?;
    used.extend(collect_type_parameter_names(&definition)?);
    let declared = declared.iter().cloned().collect::<BTreeSet<_>>();
    if used != declared {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            format!(
                "PreludeRow type-parameter usage differs: declared {declared:?}, used {used:?}"
            ),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_prelude_signature(
    name: &str,
    signature: &str,
    definition: &str,
) -> Result<(), BundleError> {
    let signature_datum = parse_document(signature).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Type,
            format!("parse {name} signature: {error}"),
        )
    })?;
    let definition_datum = parse_document(definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse {name} definition: {error}"),
        )
    })?;
    if signature_datum.form_head() != Some("Fn") {
        if !matches!(name, "This" | "That" | "Yonder")
            || definition_datum.form_head() != Some("Deictic")
            || canonical_type_for_comparison(signature)?
                != canonical_type_for_comparison("(Referents Entity)")?
        {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("nonfunction prelude {name} does not derive its declared type"),
            ));
        }
        return Ok(());
    }
    if definition_datum.form_head() != Some("λ") {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            format!("function prelude {name} must have an inert lambda initializer"),
        ));
    }
    let signature_items = signature_datum.as_list().expect("Fn is a list");
    let parameters = signature_items[1]
        .as_list()
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "Fn parameters need a list"))?;
    let lambda_items = definition_datum.as_list().expect("lambda is a list");
    let declarations = lambda_items[1].as_list().ok_or_else(|| {
        BundleError::new(BundleErrorKind::Type, "lambda declarations need a list")
    })?;
    if parameters.len() != declarations.len() {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            format!("prelude {name} lambda arity differs from its signature"),
        ));
    }
    for (parameter, declaration) in parameters.iter().zip(declarations) {
        let declaration = declaration.as_list().ok_or_else(|| {
            BundleError::new(BundleErrorKind::Type, "lambda declaration is not a pair")
        })?;
        if declaration.len() != 2
            || canonical_type_for_comparison(&canonical_datum(parameter))?
                != canonical_type_for_comparison(&canonical_datum(&declaration[1]))?
        {
            return Err(BundleError::new(
                BundleErrorKind::Type,
                format!("prelude {name} parameter type differs from its signature"),
            ));
        }
    }
    let result = canonical_datum(&signature_items[2]);
    validate_prelude_body_result(name, &result, &lambda_items[2])
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_prelude_body_result(
    name: &str,
    declared_result: &str,
    body: &Datum,
) -> Result<(), BundleError> {
    let result = parse_document(declared_result).map_err(|error| {
        BundleError::new(BundleErrorKind::Type, format!("parse result type: {error}"))
    })?;
    let valid = if result.as_atom() == Some("Content") {
        expression_is_content(body)
    } else if result.form_head() == Some("GQ") {
        body.form_head() == Some("λ")
            && body
                .as_list()
                .is_some_and(|items| items.len() == 3 && expression_is_content(&items[2]))
    } else if result.form_head() == Some("Set") {
        body.form_head() == Some("SetOf")
    } else {
        false
    };
    if valid {
        Ok(())
    } else {
        Err(BundleError::new(
            BundleErrorKind::Type,
            format!("prelude {name} body does not derive {declared_result}"),
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn expression_is_content(datum: &Datum) -> bool {
    if datum
        .as_list()
        .and_then(|items| items.first())
        .is_some_and(|head| head.form_head() == Some("DropPlace"))
    {
        return true;
    }
    datum.form_head().is_some_and(|head| {
        is_lexical_root(head)
            || matches!(
                head,
                "¬" | "∧"
                    | "∨"
                    | "→"
                    | "↔"
                    | "⊕"
                    | "∀"
                    | "∃"
                    | "="
                    | "<"
                    | "≤"
                    | ">"
                    | "≥"
                    | "Presuppose"
            )
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_prelude_acyclic(rows: &[PreludeRow]) -> Result<(), BundleError> {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn visit<'a>(
        name: &'a str,
        rows: &'a [PreludeRow],
        active: &mut BTreeSet<&'a str>,
        finished: &mut BTreeSet<&'a str>,
    ) -> Result<(), BundleError> {
        if finished.contains(name) {
            return Ok(());
        }
        if !active.insert(name) {
            return Err(BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("recursive prelude dependency at {name}"),
            ));
        }
        let row = rows.iter().find(|row| row.name == name).ok_or_else(|| {
            BundleError::new(
                BundleErrorKind::ForeignKey,
                format!("unknown prelude {name}"),
            )
        })?;
        for dependency in &row.direct_dependencies {
            visit(dependency, rows, active, finished)?;
        }
        active.remove(name);
        finished.insert(name);
        Ok(())
    }
    let mut finished = BTreeSet::new();
    for row in rows {
        visit(&row.name, rows, &mut BTreeSet::new(), &mut finished)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|dependencies| dependencies.windows(2).all(|pair| scalar_cmp(&pair[0], &pair[1]).is_lt())) || ret.is_err())]
fn prelude_dependencies(
    definition: &str,
    names: &BTreeSet<String>,
    own_name: &str,
) -> Result<Vec<String>, BundleError> {
    let datum = parse_document(definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse dependencies: {error}"),
        )
    })?;
    let mut dependencies = BTreeSet::new();
    collect_atoms(&datum, &mut |atom| {
        if atom != own_name && names.contains(atom) {
            dependencies.insert(atom.to_owned());
        }
    });
    Ok(dependencies.into_iter().collect())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_drop_places(definition: &str) -> Result<Vec<(String, u64)>, BundleError> {
    let datum = parse_document(definition).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Template,
            format!("parse DropPlace uses: {error}"),
        )
    })?;
    let mut uses = Vec::new();
    #[requires(true)]
    #[ensures(true)]
    fn visit(datum: &Datum, uses: &mut Vec<(String, u64)>) {
        if datum.form_head() == Some("DropPlace") {
            let items = datum.as_list().expect("DropPlace is a list");
            if items.len() == 3
                && let Some(root) = items[1].as_atom()
                && is_lexical_root(root)
                && let Some(ordinal) = items[2]
                    .as_integer()
                    .and_then(|text| text.parse::<u64>().ok())
            {
                uses.push((root.to_owned(), ordinal));
            }
        }
        if let Some(items) = datum.as_list() {
            for item in items {
                visit(item, uses);
            }
        }
    }
    visit(&datum, &mut uses);
    Ok(uses)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|schema| !schema.is_empty()) || ret.is_err())]
fn row_schema(slots: &[SlotRow]) -> Result<String, BundleError> {
    let mut fields = Vec::with_capacity(slots.len());
    for slot in slots {
        let label = match slot.label.as_data() {
            data!(SlotLabel::Numbered(value)) => Datum::unsigned(u128::from(*value)),
            data!(SlotLabel::Eventuality(_)) => Datum::atom("Eventuality"),
        };
        let accepted = parse_document(&slot.accepted_type_schema).map_err(|error| {
            BundleError::new(BundleErrorKind::Type, format!("parse slot type: {error}"))
        })?;
        fields.push(Datum::list([label, accepted]));
    }
    canonical_row_schema(&canonical_datum(&Datum::form("Row", fields)))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_surviving_map(
    mapping: &[String],
    input_slots: &[SlotRow],
    result_slots: &[SlotRow],
) -> Result<(), BundleError> {
    if mapping.len() != result_slots.len() {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "surviving-slot-map does not cover every result slot",
        ));
    }
    let input_labels = input_slots
        .iter()
        .map(|slot| match slot.label.as_data() {
            data!(SlotLabel::Numbered(value)) => value.to_string(),
            data!(SlotLabel::Eventuality(_)) => "Eventuality".to_owned(),
        })
        .collect::<BTreeSet<_>>();
    let expected = result_slots
        .iter()
        .map(|slot| match slot.label.as_data() {
            data!(SlotLabel::Numbered(value)) => value.to_string(),
            data!(SlotLabel::Eventuality(_)) => "Eventuality".to_owned(),
        })
        .collect::<BTreeSet<_>>();
    let mut left = BTreeSet::new();
    let mut right = BTreeSet::new();
    for item in mapping {
        let (source, result) = item.split_once("->").ok_or_else(|| {
            BundleError::new(BundleErrorKind::Parse, format!("bad slot map {item}"))
        })?;
        if !left.insert(source.to_owned()) || !right.insert(result.to_owned()) {
            return Err(BundleError::new(
                BundleErrorKind::DuplicatePrimaryKey,
                format!("noninjective slot map {item}"),
            ));
        }
    }
    if left != expected || !left.is_subset(&input_labels) {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            format!(
                "surviving map sources {left:?} do not identify the surviving members of input row {input_labels:?}"
            ),
        ));
    }
    if right != expected {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            format!("surviving map results {right:?} differ from row labels {expected:?}"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_total_provenance_map(
    mapping: &[String],
    operand_rows: &[String],
    result_row: &str,
) -> Result<(), BundleError> {
    if operand_rows.len() != 1 {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "initial relation former supports exactly one source row",
        ));
    }
    let source_labels = row_labels(&operand_rows[0])?;
    let result_labels = row_labels(result_row)?;
    if mapping.len() != source_labels.len() || mapping.len() != result_labels.len() {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "total provenance map does not cover both rows",
        ));
    }
    let mut left = BTreeSet::new();
    let mut right = BTreeSet::new();
    for item in mapping {
        let (source, result) = item.split_once("->").ok_or_else(|| {
            BundleError::new(BundleErrorKind::Parse, format!("bad provenance map {item}"))
        })?;
        left.insert(source.to_owned());
        right.insert(result.to_owned());
    }
    if left != source_labels || right != result_labels {
        return Err(BundleError::new(
            BundleErrorKind::ForeignKey,
            "provenance map is not total over source and result labels",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|labels| !labels.is_empty()) || ret.is_err())]
fn row_labels(schema: &str) -> Result<BTreeSet<String>, BundleError> {
    let datum = parse_document(schema).map_err(|error| {
        BundleError::new(BundleErrorKind::Type, format!("parse row labels: {error}"))
    })?;
    let items = datum
        .as_list()
        .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "row schema is not a list"))?;
    items[1..]
        .iter()
        .map(|slot| {
            let slot = slot
                .as_list()
                .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "row slot is not a pair"))?;
            slot.first()
                .and_then(|label| label.as_atom().or_else(|| label.as_integer()))
                .map(str::to_owned)
                .ok_or_else(|| BundleError::new(BundleErrorKind::Type, "row label is invalid"))
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.evidence_id.is_empty())]
fn spec_evidence(evidence_id: &str, locator: &str, note: &str) -> EvidenceRow {
    new!(EvidenceRow {
        evidence_id: evidence_id.to_owned(),
        source_id: "smusni-v0-spec".to_owned(),
        exact_locator: locator.to_owned(),
        cited_content_digest: SPEC_SHA256.to_owned(),
        adjudication_note: note.to_owned(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn canonical_json(value: &Value) -> Result<String, BundleError> {
    match value {
        Value::Null => Ok("null".to_owned()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(number.to_string()),
        Value::Number(_) => Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            "registry JCS forbids non-integer JSON numbers",
        )),
        Value::String(text) => serde_json::to_string(text).map_err(|error| {
            BundleError::new(
                BundleErrorKind::Parse,
                format!("serialize JCS string: {error}"),
            )
        }),
        Value::Array(values) => {
            let values = values
                .iter()
                .map(canonical_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", values.join(",")))
        }
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort_by(|left, right| scalar_cmp(left, right));
            let mut members = Vec::with_capacity(keys.len());
            for original in keys {
                let rendered = serde_json::to_string(original)
                    .map_err(|error| BundleError::new(BundleErrorKind::Parse, error.to_string()))?;
                members.push(format!("{rendered}:{}", canonical_json(&object[original])?));
            }
            Ok(format!("{{{}}}", members.join(",")))
        }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_nfc_value(value: &Value) -> Result<(), BundleError> {
    match value {
        Value::String(text) if !text.nfc().eq(text.chars()) => Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            "registry JSON string is not NFC",
        )),
        Value::Array(values) => {
            for value in values {
                require_nfc_value(value)?;
            }
            Ok(())
        }
        Value::Object(object) => {
            for (key, value) in object {
                if !key.is_ascii() || !key.nfc().eq(key.chars()) {
                    return Err(BundleError::new(
                        BundleErrorKind::ByteDomain,
                        "registry JSON object key is not canonical NFC ASCII",
                    ));
                }
                require_nfc_value(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_nfc_utf8(path: &str, bytes: &[u8]) -> Result<(), BundleError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        BundleError::new(
            BundleErrorKind::ByteDomain,
            format!("{path} is not UTF-8: {error}"),
        )
    })?;
    if !text.nfc().eq(text.chars()) {
        return Err(BundleError::new(
            BundleErrorKind::ByteDomain,
            format!("{path} is not NFC"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|digest| is_digest(digest)) || ret.is_err())]
fn digest_pairs(pairs: &[(String, String)]) -> Result<String, BundleError> {
    let value = serde_json::to_value(pairs).map_err(|error| {
        BundleError::new(
            BundleErrorKind::Parse,
            format!("serialize digest pairs: {error}"),
        )
    })?;
    Ok(sha256_hex(canonical_json(&value)?.as_bytes()))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_artifact_order(records: &[ArtifactRecord], context: &str) -> Result<(), BundleError> {
    if records
        .windows(2)
        .any(|pair| !scalar_cmp(&pair[0].relative_path, &pair[1].relative_path).is_lt())
    {
        return Err(BundleError::new(
            BundleErrorKind::NonCanonicalOrder,
            format!("{context} paths are not unique scalar-value order"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_sorted_unique<T: AsRef<str>>(values: &[T], context: &str) -> Result<(), BundleError> {
    if values
        .windows(2)
        .any(|pair| !scalar_cmp(pair[0].as_ref(), pair[1].as_ref()).is_lt())
    {
        return Err(BundleError::new(
            BundleErrorKind::NonCanonicalOrder,
            format!("{context} are not unique scalar-value order"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_tuple_sorted_unique(values: &[(&str, u64)], context: &str) -> Result<(), BundleError> {
    if values.windows(2).any(|pair| {
        let order = scalar_cmp(pair[0].0, pair[1].0).then(pair[0].1.cmp(&pair[1].1));
        !order.is_lt()
    }) {
        return Err(BundleError::new(
            BundleErrorKind::NonCanonicalOrder,
            format!("{context} are not unique tuple order"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn require_unique_nonempty(values: &[String], context: &str) -> Result<(), BundleError> {
    if values.iter().any(String::is_empty)
        || values.iter().collect::<BTreeSet<_>>().len() != values.len()
    {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            format!("{context} must be nonempty and unique"),
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn reject_duplicate<T: Ord + fmt::Display>(
    values: impl IntoIterator<Item = T>,
    context: &str,
) -> Result<(), BundleError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(BundleError::new(
                BundleErrorKind::DuplicatePrimaryKey,
                format!("duplicate {context}"),
            ));
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret == left.chars().cmp(right.chars()))]
fn scalar_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    left.chars().cmp(right.chars())
}

#[requires(true)]
#[ensures(ret.len() == 64 && ret.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))]
fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[requires(true)]
#[ensures(true)]
fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[requires(true)]
#[ensures(true)]
fn is_lexical_root(text: &str) -> bool {
    !text.is_empty()
        && text.nfc().eq(text.chars())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'\'')
}

#[requires(true)]
#[ensures(true)]
fn is_pascal_case(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase) && text.chars().all(char::is_alphanumeric)
}

#[requires(true)]
#[ensures(true)]
fn is_reason_id(text: &str) -> bool {
    text.starts_with("smusni.")
        && text.len() > "smusni.".len()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

#[requires(true)]
#[ensures(true)]
fn is_disposition(text: &str) -> bool {
    matches!(
        text,
        "DirectLowering"
            | "ProvenDesugaring"
            | "NotationDefault"
            | "ProvenanceSuppression"
            | "DiagnosticCollection"
            | "TypedFallback"
    )
}

#[requires(true)]
#[ensures(true)]
fn is_hole_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && text.as_bytes()[0].is_ascii_lowercase()
}

#[requires(true)]
#[ensures(true)]
fn is_type_parameter_name(text: &str) -> bool {
    text.bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn ascii_slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        sha256_hex(text.as_bytes())[..16].to_owned()
    } else {
        slug
    }
}

#[requires(true)]
#[ensures(!ret.ends_with('\n'))]
fn canonical_datum(datum: &Datum) -> String {
    print_document(datum)
        .strip_suffix('\n')
        .expect("datum printer always appends LF")
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn contains_atom(datum: &Datum, expected: &str) -> bool {
    match datum {
        Datum::Atom(atom) => atom.as_str() == expected,
        Datum::List(values) => values.iter().any(|value| contains_atom(value, expected)),
        Datum::String(_) | Datum::Integer(_) => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn contains_form(datum: &Datum, expected: &str) -> bool {
    datum.form_head() == Some(expected)
        || datum
            .as_list()
            .is_some_and(|values| values.iter().any(|value| contains_form(value, expected)))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_type_parameter_declarations(declared: &[String]) -> Result<(), BundleError> {
    if declared.iter().any(|name| !is_type_parameter_name(name)) {
        return Err(BundleError::new(
            BundleErrorKind::Type,
            "PreludeRow type-parameter names must be canonical ASCII identifiers",
        ));
    }
    if declared.iter().collect::<BTreeSet<_>>().len() != declared.len() {
        return Err(BundleError::new(
            BundleErrorKind::DuplicatePrimaryKey,
            "PreludeRow type-parameter declarations contain a duplicate",
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn collect_type_parameter_names(datum: &Datum) -> Result<BTreeSet<String>, BundleError> {
    if datum.form_head() == Some("TypeParam") {
        let items = datum.as_list().expect("a form head belongs to a list");
        let name = items
            .get(1)
            .filter(|_| items.len() == 2)
            .and_then(Datum::as_string)
            .filter(|name| is_type_parameter_name(name))
            .ok_or_else(|| {
                BundleError::new(
                    BundleErrorKind::Type,
                    "TypeParam requires exactly one canonical ASCII name string",
                )
            })?;
        return Ok(BTreeSet::from([name.to_owned()]));
    }
    let mut names = BTreeSet::new();
    if let Some(items) = datum.as_list() {
        for item in items {
            names.extend(collect_type_parameter_names(item)?);
        }
    }
    Ok(names)
}

#[requires(true)]
#[ensures(true)]
fn collect_atoms(datum: &Datum, visitor: &mut impl FnMut(&str)) {
    match datum {
        Datum::Atom(atom) => visitor(atom.as_str()),
        Datum::List(values) => {
            for value in values {
                collect_atoms(value, visitor);
            }
        }
        Datum::String(_) | Datum::Integer(_) => {}
    }
}

#[cfg(test)]
mod static_checker_tests {
    use super::*;
    use crate::smusni_v0_kernel::type_system::RowSlot;

    #[requires(true)]
    #[ensures(true)]
    fn empty_registry() -> StaticTypeRegistry {
        StaticTypeRegistry {
            lexical: BTreeMap::new(),
            prelude: BTreeMap::new(),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rigid_type_parameter_is_unified_once_and_substituted_across_arguments() {
        let mut environment = BTreeMap::new();
        environment.insert(
            "$same".to_owned(),
            StaticType::Function {
                parameters: vec![static_type_parameter("T"), static_type_parameter("T")],
                result: Box::new(static_atom(TypeAtom::Content)),
            },
        );
        let registry = empty_registry();
        assert!(
            infer_expression(
                &parse_document("($same 1 2)").unwrap(),
                &environment,
                &registry,
            )
            .is_ok()
        );
        assert!(
            infer_expression(
                &parse_document("($same 1 DistanceScale)").unwrap(),
                &environment,
                &registry,
            )
            .is_err()
        );

        let mut substitutions = BTreeMap::new();
        assert!(!unify_static_types(
            &static_type_parameter("T"),
            &static_type_parameter("U"),
            &mut substitutions,
        ));
        assert!(substitutions.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn deictics_and_context_require_their_closed_types() {
        let registry = empty_registry();
        let environment = BTreeMap::new();
        assert_eq!(
            infer_constant("Speaker").unwrap(),
            StaticType::Referents(Box::new(static_atom(TypeAtom::Entity)))
        );
        assert_eq!(
            infer_constant("Now").unwrap(),
            StaticType::Referents(Box::new(static_atom(TypeAtom::Eventuality)))
        );
        assert!(
            infer_expression(
                &parse_document("(Deictic Proximal CurrentGround)").unwrap(),
                &environment,
                &registry,
            )
            .is_ok()
        );
        assert!(
            infer_expression(
                &parse_document("(Deictic Speaker CurrentGround)").unwrap(),
                &environment,
                &registry,
            )
            .is_err()
        );
        assert!(
            infer_expression(
                &parse_document("(Deictic Proximal Speaker)").unwrap(),
                &environment,
                &registry,
            )
            .is_err()
        );
        let expected = StaticType::ReferenceComputation(Box::new(StaticType::Referents(Box::new(
            static_atom(TypeAtom::Entity),
        ))));
        assert!(
            check_expression(
                &parse_document("Context").unwrap(),
                &expected,
                &environment,
                &registry,
            )
            .is_ok()
        );
        assert!(
            check_expression(
                &parse_document("(Context Speaker)").unwrap(),
                &expected,
                &environment,
                &registry,
            )
            .is_ok()
        );
        assert!(
            check_expression(
                &parse_document("(Context)").unwrap(),
                &expected,
                &environment,
                &registry,
            )
            .is_err()
        );
        assert!(
            check_expression(
                &parse_document("Context").unwrap(),
                &static_atom(TypeAtom::Entity),
                &environment,
                &registry,
            )
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn predicate_cursor_survives_event_fill_and_huge_drop_place() {
        let relation = "cursor";
        let huge = PositiveInteger::try_new("429496729600000000000000000000").unwrap();
        let row = Row::new(
            vec![
                RowSlot::new(PlaceLabel::numbered(1), TypeExpr::Atom(TypeAtom::Text)),
                RowSlot::new(
                    PlaceLabel::numbered(2),
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity))),
                ),
                RowSlot::new(PlaceLabel::numbered(3), TypeExpr::Atom(TypeAtom::Natural)),
                RowSlot::new(
                    PlaceLabel::Numbered(huge.clone()),
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity))),
                ),
                RowSlot::new(
                    PlaceLabel::Eventuality,
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                ),
            ],
            false,
        );
        let policies = vec![
            ClosePolicy::Required,
            ClosePolicy::Contextual,
            ClosePolicy::Required,
            ClosePolicy::Contextual,
            ClosePolicy::LocalExistential,
        ];
        let mut registry = empty_registry();
        registry.lexical.insert(
            relation.to_owned(),
            StaticType::Predicate(StaticPredicate::new(row, policies)),
        );
        let environment = BTreeMap::new();
        let inferred = infer_expression(
            &parse_document("(cursor :2 Speaker :Eventuality Now 7)").unwrap(),
            &environment,
            &registry,
        )
        .unwrap();
        let StaticType::Predicate(inferred) = inferred else {
            panic!("predicate application retains a predicate row");
        };
        assert_eq!(
            inferred
                .row
                .slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1), PlaceLabel::Numbered(huge.clone())]
        );
        let dropped = infer_expression(
            &parse_document(&format!("(DropPlace cursor {huge})")).unwrap(),
            &environment,
            &registry,
        )
        .unwrap();
        let StaticType::Predicate(dropped) = dropped else {
            panic!("DropPlace retains a predicate row");
        };
        assert!(
            !dropped
                .row
                .slots()
                .iter()
                .any(|slot| slot.label() == PlaceLabel::Numbered(huge.clone()))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn only_verified_finite_remainders_close_to_content() {
        let referents = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)));
        let event = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality)));
        let mut registry = empty_registry();
        registry.lexical.insert(
            "closeable".to_owned(),
            StaticType::Predicate(StaticPredicate::new(
                Row::new(
                    vec![
                        RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                        RowSlot::new(PlaceLabel::Eventuality, event.clone()),
                    ],
                    false,
                ),
                vec![ClosePolicy::Contextual, ClosePolicy::LocalExistential],
            )),
        );
        registry.lexical.insert(
            "required".to_owned(),
            StaticType::Predicate(StaticPredicate::new(
                Row::new(
                    vec![
                        RowSlot::new(PlaceLabel::numbered(1), referents),
                        RowSlot::new(PlaceLabel::Eventuality, event),
                    ],
                    false,
                ),
                vec![ClosePolicy::Required, ClosePolicy::LocalExistential],
            )),
        );
        let environment = BTreeMap::new();
        let content = static_atom(TypeAtom::Content);
        assert!(
            check_expression(
                &parse_document("closeable").unwrap(),
                &content,
                &environment,
                &registry,
            )
            .is_ok()
        );
        assert!(
            check_expression(
                &parse_document("required").unwrap(),
                &content,
                &environment,
                &registry,
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn artifact_paths_reject_parent_and_symlink_escapes() {
        use std::os::unix::fs::symlink;

        let scratch = PathBuf::from(format!(
            "/build/jbotci/scratch/issue-741/smusni-v0-path-test-{}",
            std::process::id()
        ));
        let root = scratch.join("root");
        let outside = scratch.join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("payload"), b"outside").unwrap();
        assert_eq!(
            read_relative(&root, "../outside/payload").unwrap_err().kind,
            BundleErrorKind::Manifest
        );
        assert_eq!(
            read_relative(&root, "./inside").unwrap_err().kind,
            BundleErrorKind::Manifest
        );
        symlink(outside.join("payload"), root.join("escape")).unwrap();
        assert_eq!(
            read_relative(&root, "escape").unwrap_err().kind,
            BundleErrorKind::Manifest
        );
        assert_eq!(
            write_relative(&root, "escape", b"replacement")
                .unwrap_err()
                .kind,
            BundleErrorKind::Manifest
        );
    }
}
