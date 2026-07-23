use std::collections::BTreeMap;

use bityzba::{ensures, invariant, requires};
use serde::{Deserialize, Serialize};

pub(crate) const EXTRACTOR_VERSION: &str = "semantic-source-inventory-v1";
pub(crate) const ARTIFACT_DIRECTORY: &str =
    "docs/semantic-model-migration/source-inventory-v1";

pub(crate) const JSONL_ARTIFACTS: &[&str] = &[
    "files.jsonl",
    "modules.jsonl",
    "types.jsonl",
    "variants.jsonl",
    "fields.jsonl",
    "functions.jsonl",
    "contracts.jsonl",
    "serialization.jsonl",
    "traversal-and-reference-edges.jsonl",
    "lowering-sites.jsonl",
    "renderer-parser-consumers.jsonl",
    "tests-and-fixtures.jsonl",
    "python-source-census.jsonl",
    "executable-consumers.jsonl",
    "migration-ledger.jsonl",
];

#[invariant(!path.is_empty(), "a source identity always names a pinned path")]
#[invariant(byte_start <= byte_end, "source byte ranges are ordered")]
#[invariant(line_start > 0 && line_end > 0, "source lines are one-based")]
#[invariant(line_start < line_end || (line_start == line_end && column_start <= column_end), "source line/column ranges are ordered")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct SourceIdentity {
    pub(crate) path: String,
    pub(crate) byte_start: usize,
    pub(crate) byte_end: usize,
    pub(crate) line_start: usize,
    pub(crate) column_start: usize,
    pub(crate) line_end: usize,
    pub(crate) column_end: usize,
}

impl SourceIdentity {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn stable_id(&self) -> String {
        format!("{}:{}-{}", self.path, self.byte_start, self.byte_end)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FileClass {
    RustSemanticModel,
    RustLowering,
    RustRenderer,
    RustConsumer,
    RustValidationTool,
    Python,
    SemanticDocumentation,
    SemanticFixture,
    SemanticFixtureConfiguration,
}

#[invariant(!id.is_empty() && git_blob.len() == 40 && sha256.len() == 64 && *byte_length > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FileRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) class: FileClass,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
}

#[invariant(!id.is_empty() && !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ModuleRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
    pub(crate) declared_path: Option<String>,
    pub(crate) inline: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TypeKind {
    Struct,
    Enum,
    Union,
    Alias,
    Trait,
    TraitAlias,
    AssociatedType,
    ForeignType,
}

#[invariant(!id.is_empty() && !name.is_empty() && !module_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TypeRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) module_id: String,
    pub(crate) name: String,
    pub(crate) kind: TypeKind,
    pub(crate) visibility: String,
    pub(crate) generic_parameters: String,
}

#[invariant(!id.is_empty() && !type_id.is_empty() && !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct VariantRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) type_id: String,
    pub(crate) name: String,
    pub(crate) discriminant: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FieldStyle {
    Named,
    Tuple,
    Unit,
}

#[invariant(!id.is_empty() && !owner_id.is_empty() && !name.is_empty())]
#[invariant((*style == FieldStyle::Unit) == (*name == "$unit" && *rust_type == "()"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FieldRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) owner_id: String,
    pub(crate) name: String,
    pub(crate) style: FieldStyle,
    pub(crate) visibility: String,
    pub(crate) rust_type: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FunctionKind {
    Free,
    InherentMethod,
    TraitMethod,
    TraitDeclaration,
    Foreign,
}

#[invariant(!id.is_empty() && !module_id.is_empty() && !name.is_empty() && !signature.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FunctionRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) module_id: String,
    pub(crate) owner_type: Option<String>,
    pub(crate) name: String,
    pub(crate) kind: FunctionKind,
    pub(crate) visibility: String,
    pub(crate) signature: String,
    pub(crate) is_async: bool,
    pub(crate) is_unsafe: bool,
    pub(crate) is_test: bool,
}

#[invariant(!id.is_empty() && !owner_id.is_empty() && !attribute.is_empty() && !contract_kind.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ContractRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) owner_id: String,
    pub(crate) owner_kind: String,
    pub(crate) contract_kind: String,
    pub(crate) attribute: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SerializationKind {
    SerdeAttribute,
    SerdeNaming,
    SerdeCustomCodec,
    SerdeOmission,
    SerdeFlattening,
    CustomSerializeImplementation,
    CustomDeserializeImplementation,
    FormatConstant,
    SerializedKey,
}

#[invariant(!id.is_empty() && !owner_id.is_empty() && !detail.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct SerializationRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) owner_id: String,
    pub(crate) kind: SerializationKind,
    pub(crate) detail: String,
    pub(crate) key: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EdgeKind {
    FieldType,
    ReferenceCollection,
    TreeVisit,
    WalkerDescent,
}

#[invariant(!id.is_empty() && !owner_id.is_empty() && !target.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct EdgeRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) owner_id: String,
    pub(crate) kind: EdgeKind,
    pub(crate) target: String,
}

#[invariant(!id.is_empty() && !function_id.is_empty() && !operation.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct LoweringSiteRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) function_id: String,
    pub(crate) operation: String,
    pub(crate) constructed_type: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RendererParserConsumerKind {
    Renderer,
    Parser,
    CliSurface,
    McpSurface,
    FixtureHarness,
}

#[invariant(!id.is_empty() && !owner_id.is_empty() && !symbol.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RendererParserConsumerRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) owner_id: String,
    pub(crate) kind: RendererParserConsumerKind,
    pub(crate) symbol: String,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TestFixtureKind {
    RustTest,
    SemanticReferenceFixture,
    TersmuOutputFixture,
    CombinedSemanticFixture,
    FixtureProfile,
    CoverageAllowlist,
}

#[invariant(!id.is_empty() && !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TestFixtureRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) name: String,
    pub(crate) kind: TestFixtureKind,
    pub(crate) owner_id: Option<String>,
    pub(crate) semantic_reference_expectation: bool,
    pub(crate) tersmu_output_expectation: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PythonCoverageClass {
    BindingRuntime,
    BindingStub,
    BindingTest,
    BindingMaintenance,
    SemanticFixtureVerifier,
    RecoveryFixtureMaintenance,
    EmbeddingArtifactTooling,
    XarsnuServiceTooling,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PythonOccurrenceKind {
    Identifier,
    StringLiteral,
}

#[invariant(!token.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PythonOccurrence {
    pub(crate) source: SourceIdentity,
    pub(crate) kind: PythonOccurrenceKind,
    pub(crate) token: String,
}

#[invariant(!id.is_empty() && !sha256.is_empty() && !coverage_reason.is_empty())]
#[invariant(git_blob.len() == 40 && sha256.len() == 64)]
#[invariant(*semantic_consumer == occurrences.iter().any(|occurrence| occurrence.token == "tersmu"))]
#[invariant(occurrences.iter().all(|occurrence| occurrence.source.path.as_str() == source.path.as_str()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PythonCensusRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
    pub(crate) coverage_class: PythonCoverageClass,
    pub(crate) coverage_reason: String,
    pub(crate) semantic_consumer: bool,
    pub(crate) occurrences: Vec<PythonOccurrence>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConsumerLanguage {
    Rust,
    Python,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LifecycleDisposition {
    RetainAndRun,
    AdaptAndRun,
    SupersedeWithReplacement,
    RetireOneShotWithPreservedWitness,
}

#[invariant(!id.is_empty() && !consumer_kind.is_empty() && !exact_invocation.is_empty() && !downstream_migration_owners.is_empty())]
#[invariant(replacement_gate.is_some() == matches!(*lifecycle_disposition, LifecycleDisposition::SupersedeWithReplacement | LifecycleDisposition::RetireOneShotWithPreservedWitness))]
#[invariant(preserved_witnesses.is_empty() == matches!(*lifecycle_disposition, LifecycleDisposition::RetainAndRun | LifecycleDisposition::AdaptAndRun))]
#[invariant(replacement_gate.as_ref().is_none_or(|gate| preserved_witnesses.iter().all(|(witness, mapped_gate)| !witness.is_empty() && mapped_gate == gate)))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ExecutableConsumerRecord {
    pub(crate) id: String,
    pub(crate) source: SourceIdentity,
    pub(crate) language: ConsumerLanguage,
    pub(crate) consumer_kind: String,
    pub(crate) cli_formats: Vec<String>,
    pub(crate) cli_default: Option<String>,
    pub(crate) error_surfaces: Vec<String>,
    pub(crate) fixture_reads: Vec<String>,
    pub(crate) fixture_writes: Vec<String>,
    pub(crate) hard_coded_tokens: Vec<String>,
    pub(crate) base_revision_assumptions: Vec<String>,
    pub(crate) current_revision_assumptions: Vec<String>,
    pub(crate) binary_assumptions: Vec<String>,
    pub(crate) exact_invocation: String,
    pub(crate) allowlist_count_hash_pins: Vec<String>,
    pub(crate) downstream_migration_owners: Vec<u16>,
    pub(crate) lifecycle_disposition: LifecycleDisposition,
    pub(crate) replacement_gate: Option<String>,
    pub(crate) preserved_witnesses: BTreeMap<String, String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MigrationDisposition {
    Retain,
    Rename,
    Split,
    Merge,
    Replace,
    Delete,
    ProvenanceOnly,
    External,
    PendingDesignOwner,
}

#[invariant(!id.is_empty() && !record_id.is_empty() && !record_kind.is_empty() && !rationale.is_empty())]
#[invariant((569..=582).contains(primary_owner) && *primary_owner != 580)]
#[invariant(additional_owners.iter().all(|owner| (569..=582).contains(owner) && *owner != 580 && owner != primary_owner))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct MigrationLedgerRecord {
    pub(crate) id: String,
    pub(crate) record_id: String,
    pub(crate) record_kind: String,
    pub(crate) disposition: MigrationDisposition,
    pub(crate) primary_owner: u16,
    pub(crate) additional_owners: Vec<u16>,
    pub(crate) decision_ids: Vec<String>,
    pub(crate) rationale: String,
}

#[invariant(!path.is_empty() && git_blob.len() == 40 && sha256.len() == 64 && *byte_length > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ManifestSourceFile {
    pub(crate) path: String,
    pub(crate) git_blob: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
}

#[invariant(!name.is_empty() && sha256.len() == 64 && *byte_length > 0 && *record_count > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ManifestArtifact {
    pub(crate) name: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
    pub(crate) record_count: usize,
}

#[invariant(!path.is_empty() && sha256.len() == 64 && *byte_length > 0 && !rule_hits.is_empty() && *issue_580_current_source_rows == 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ManifestClassificationCatalog {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
    pub(crate) rule_hits: BTreeMap<String, usize>,
    pub(crate) issue_580_current_source_rows: usize,
}

#[invariant(!commit.is_empty() && !tree.is_empty() && !extractor_version.is_empty() && !generation_command.is_empty())]
#[invariant(commit.len() == 40 && tree.len() == 40 && !source_files.is_empty() && !artifacts.is_empty() && !record_counts.is_empty())]
#[invariant(artifacts.len() == 15 && record_counts.len() == 15)]
#[invariant(legacy_haskell_file_hashes.len() == 2 && legacy_provenance_facts.len() == 4)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Manifest {
    pub(crate) commit: String,
    pub(crate) tree: String,
    pub(crate) extractor_version: String,
    pub(crate) generation_command: String,
    pub(crate) classification_catalog: ManifestClassificationCatalog,
    pub(crate) source_files: Vec<ManifestSourceFile>,
    pub(crate) artifacts: Vec<ManifestArtifact>,
    pub(crate) record_counts: BTreeMap<String, usize>,
    pub(crate) legacy_haskell_file_hashes: BTreeMap<String, String>,
    pub(crate) legacy_provenance_facts: Vec<String>,
}
