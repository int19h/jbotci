use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use bityzba::{ensures, invariant, requires};
use serde::Deserialize;

use super::model::{
    ConsumerLanguage, LifecycleDisposition, MigrationDisposition, PythonCoverageClass,
};
use super::source::sha256_hex;

pub(crate) const CLASSIFICATION_CATALOG_PATH: &str =
    "xtask-full/semantic-source-inventory-rules-v1.toml";

#[invariant(*version == 1)]
#[invariant(!python_files.is_empty() && !executable_consumers.is_empty() && !migration_rules.is_empty())]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ClassificationCatalog {
    pub(crate) version: u32,
    pub(crate) python_files: Vec<PythonFileRule>,
    pub(crate) executable_consumers: Vec<ExecutableConsumerRule>,
    pub(crate) migration_rules: Vec<MigrationRule>,
}

#[invariant(!path.is_empty() && !reason.is_empty())]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct PythonFileRule {
    pub(crate) path: String,
    pub(crate) class: PythonCoverageClass,
    pub(crate) reason: String,
}

#[invariant(!path.is_empty() && !consumer_kind.is_empty() && !exact_invocation.is_empty())]
#[invariant(!downstream_migration_owners.is_empty())]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ExecutableConsumerRule {
    pub(crate) path: String,
    pub(crate) language: ConsumerLanguage,
    pub(crate) consumer_kind: String,
    #[serde(default)]
    pub(crate) cli_formats: Vec<String>,
    pub(crate) cli_default: Option<String>,
    #[serde(default)]
    pub(crate) error_surfaces: Vec<String>,
    #[serde(default)]
    pub(crate) fixture_reads: Vec<String>,
    #[serde(default)]
    pub(crate) fixture_writes: Vec<String>,
    #[serde(default)]
    pub(crate) hard_coded_tokens: Vec<String>,
    #[serde(default)]
    pub(crate) base_revision_assumptions: Vec<String>,
    #[serde(default)]
    pub(crate) current_revision_assumptions: Vec<String>,
    #[serde(default)]
    pub(crate) binary_assumptions: Vec<String>,
    pub(crate) exact_invocation: String,
    #[serde(default)]
    pub(crate) allowlist_count_hash_pins: Vec<String>,
    pub(crate) downstream_migration_owners: Vec<u16>,
    pub(crate) lifecycle_disposition: LifecycleDisposition,
    pub(crate) replacement_gate: Option<String>,
    #[serde(default)]
    pub(crate) preserved_witnesses: BTreeMap<String, String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RecordFamily {
    File,
    Module,
    Type,
    Variant,
    Field,
    Function,
    Contract,
    Serialization,
    TraversalAndReferenceEdge,
    LoweringSite,
    RendererParserConsumer,
    TestAndFixture,
    PythonSource,
    ExecutableConsumer,
}

impl RecordFamily {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Module => "module",
            Self::Type => "type",
            Self::Variant => "variant",
            Self::Field => "field",
            Self::Function => "function",
            Self::Contract => "contract",
            Self::Serialization => "serialization",
            Self::TraversalAndReferenceEdge => "traversal-and-reference-edge",
            Self::LoweringSite => "lowering-site",
            Self::RendererParserConsumer => "renderer-parser-consumer",
            Self::TestAndFixture => "test-and-fixture",
            Self::PythonSource => "python-source",
            Self::ExecutableConsumer => "executable-consumer",
        }
    }
}

#[invariant(!id.is_empty() && !paths.is_empty() && !record_families.is_empty() && !rationale.is_empty())]
#[invariant((569..=582).contains(primary_owner) && *primary_owner != 580)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct MigrationRule {
    pub(crate) id: String,
    pub(crate) paths: Vec<String>,
    pub(crate) record_families: Vec<RecordFamily>,
    #[serde(default)]
    pub(crate) record_ids: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_record_ids: Vec<String>,
    #[serde(default)]
    pub(crate) owner_types: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_owner_types: Vec<String>,
    pub(crate) disposition: MigrationDisposition,
    pub(crate) primary_owner: u16,
    #[serde(default)]
    pub(crate) additional_owners: Vec<u16>,
    #[serde(default)]
    pub(crate) decision_ids: Vec<String>,
    pub(crate) rationale: String,
}

#[invariant(!sha256.is_empty() && *byte_length > 0)]
#[derive(Debug)]
pub(crate) struct LoadedClassificationCatalog {
    pub(crate) catalog: ClassificationCatalog,
    pub(crate) sha256: String,
    pub(crate) byte_length: usize,
}

#[requires(repository_root.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|loaded| !loaded.sha256.is_empty()))]
pub(crate) fn load_catalog(repository_root: &Path) -> Result<LoadedClassificationCatalog> {
    let path = repository_root.join(CLASSIFICATION_CATALOG_PATH);
    let bytes = fs::read(&path)
        .with_context(|| format!("reading classification catalog `{}`", path.display()))?;
    let text = std::str::from_utf8(&bytes).context("classification catalog must be UTF-8")?;
    let catalog: ClassificationCatalog = toml::from_str(text)
        .with_context(|| format!("parsing classification catalog `{}`", path.display()))?;
    validate_catalog(&catalog)?;
    Ok(bityzba::new!(LoadedClassificationCatalog {
        catalog,
        sha256: sha256_hex(&bytes),
        byte_length: bytes.len(),
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_catalog(catalog: &ClassificationCatalog) -> Result<()> {
    let mut python_paths = BTreeSet::new();
    for rule in &catalog.python_files {
        validate_repository_path(&rule.path, "Python classification")?;
        if !python_paths.insert(rule.path.as_str()) {
            bail!("classification catalog repeats Python path `{}`", rule.path);
        }
    }
    let mut executable_paths = BTreeSet::new();
    for rule in &catalog.executable_consumers {
        validate_repository_path(&rule.path, "executable consumer")?;
        if !executable_paths.insert(rule.path.as_str()) {
            bail!(
                "classification catalog repeats executable consumer path `{}`",
                rule.path
            );
        }
        validate_issue_ids(
            &rule.downstream_migration_owners,
            &format!("executable consumer `{}`", rule.path),
        )?;
        for (name, values) in [
            ("CLI formats", &rule.cli_formats),
            ("error surfaces", &rule.error_surfaces),
            ("fixture reads", &rule.fixture_reads),
            ("fixture writes", &rule.fixture_writes),
            ("hard-coded tokens", &rule.hard_coded_tokens),
            ("base revision assumptions", &rule.base_revision_assumptions),
            (
                "current revision assumptions",
                &rule.current_revision_assumptions,
            ),
            ("binary assumptions", &rule.binary_assumptions),
            ("allowlist/count/hash pins", &rule.allowlist_count_hash_pins),
        ] {
            if values.iter().any(String::is_empty) {
                bail!(
                    "executable consumer `{}` {name} contains an empty value",
                    rule.path
                );
            }
            require_unique(
                values,
                &format!("executable consumer `{}` {name}", rule.path),
            )?;
        }
        if rule
            .cli_default
            .as_ref()
            .is_some_and(|default| !rule.cli_formats.contains(default))
        {
            bail!(
                "executable consumer `{}` names a default outside its CLI formats",
                rule.path
            );
        }
        match rule.lifecycle_disposition {
            LifecycleDisposition::SupersedeWithReplacement
            | LifecycleDisposition::RetireOneShotWithPreservedWitness => {
                if rule.replacement_gate.as_deref().is_none_or(str::is_empty)
                    || rule.preserved_witnesses.is_empty()
                {
                    bail!(
                        "executable consumer `{}` must name a replacement gate and preserved witnesses for its lifecycle disposition",
                        rule.path
                    );
                }
                let replacement_gate = rule
                    .replacement_gate
                    .as_deref()
                    .expect("replacement lifecycle requires a gate");
                if rule
                    .preserved_witnesses
                    .iter()
                    .any(|(witness, gate)| witness.is_empty() || gate != replacement_gate)
                {
                    bail!(
                        "executable consumer `{}` must map every named witness to its exact replacement gate",
                        rule.path
                    );
                }
            }
            LifecycleDisposition::RetainAndRun | LifecycleDisposition::AdaptAndRun => {
                if rule.replacement_gate.is_some() || !rule.preserved_witnesses.is_empty() {
                    bail!(
                        "retained/adapted executable consumer `{}` must not name replacement-only fields",
                        rule.path
                    );
                }
            }
        }
    }
    let mut migration_ids = BTreeSet::new();
    for rule in &catalog.migration_rules {
        if !migration_ids.insert(rule.id.as_str()) {
            bail!("classification catalog repeats migration rule id `{}`", rule.id);
        }
        if rule.paths.iter().any(String::is_empty)
            || rule.record_ids.iter().any(String::is_empty)
            || rule.exclude_record_ids.iter().any(String::is_empty)
            || rule.owner_types.iter().any(String::is_empty)
            || rule.exclude_owner_types.iter().any(String::is_empty)
        {
            bail!("migration rule `{}` contains an empty selector", rule.id);
        }
        for path in &rule.paths {
            validate_repository_path(path, &format!("migration rule `{}`", rule.id))?;
        }
        require_unique(
            &rule.paths,
            &format!("migration rule `{}` paths", rule.id),
        )?;
        require_unique(
            &rule.record_families,
            &format!("migration rule `{}` record families", rule.id),
        )?;
        require_unique(
            &rule.record_ids,
            &format!("migration rule `{}` record ids", rule.id),
        )?;
        require_unique(
            &rule.exclude_record_ids,
            &format!("migration rule `{}` excluded record ids", rule.id),
        )?;
        require_unique(
            &rule.owner_types,
            &format!("migration rule `{}` owner types", rule.id),
        )?;
        require_unique(
            &rule.exclude_owner_types,
            &format!("migration rule `{}` excluded owner types", rule.id),
        )?;
        if rule
            .record_ids
            .iter()
            .any(|record_id| rule.exclude_record_ids.contains(record_id))
        {
            bail!(
                "migration rule `{}` includes and excludes the same record id",
                rule.id
            );
        }
        if rule
            .owner_types
            .iter()
            .any(|owner| rule.exclude_owner_types.contains(owner))
        {
            bail!(
                "migration rule `{}` includes and excludes the same owner type",
                rule.id
            );
        }
        let mut issues = vec![rule.primary_owner];
        issues.extend(rule.additional_owners.iter().copied());
        validate_issue_ids(&issues, &format!("migration rule `{}`", rule.id))?;
        validate_decision_ids(&rule.decision_ids, &rule.id)?;
    }
    Ok(())
}

#[requires(!context.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_repository_path(path: &str, context: &str) -> Result<()> {
    if path.is_empty()
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("{context} contains invalid repository-relative path `{path}`");
    }
    Ok(())
}

#[requires(!context.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn require_unique<T>(values: &[T], context: &str) -> Result<()>
where
    T: Ord,
{
    let mut unique = BTreeSet::new();
    for value in values {
        if !unique.insert(value) {
            bail!("{context} contains a duplicate value");
        }
    }
    Ok(())
}

#[requires(!context.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_issue_ids(issues: &[u16], context: &str) -> Result<()> {
    if issues.is_empty() {
        bail!("{context} has no owning issue");
    }
    let mut unique = BTreeSet::new();
    for issue in issues {
        if !(569..=582).contains(issue) || *issue == 580 {
            bail!(
                "{context} names invalid current-source owner issue #{issue}; #569..#582 except #580 are valid"
            );
        }
        if !unique.insert(*issue) {
            bail!("{context} repeats owning issue #{issue}");
        }
    }
    Ok(())
}

#[requires(!rule_id.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_decision_ids(decisions: &[String], rule_id: &str) -> Result<()> {
    let mut unique = BTreeSet::new();
    for decision in decisions {
        let Some(number) = decision.strip_prefix('D') else {
            bail!("migration rule `{rule_id}` has invalid decision id `{decision}`");
        };
        let number = number
            .parse::<u8>()
            .with_context(|| format!("migration rule `{rule_id}` has invalid decision id"))?;
        if !(1..=12).contains(&number) || !unique.insert(number) {
            bail!(
                "migration rule `{rule_id}` has invalid or duplicate decision id `{decision}`"
            );
        }
    }
    Ok(())
}
