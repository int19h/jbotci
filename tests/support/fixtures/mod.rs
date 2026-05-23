//! Unified TOML fixture loader, selectors, and runner support.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use bityzba::{ensures, invariant, requires};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

mod format;
mod runner;

use format::format_test_case_toml;
#[allow(unused_imports)]
pub use runner::{
    FacetResult, FacetStatus, FixtureBackend, RunSummary, run_fixture_facets,
    run_fixture_facets_parallel,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct TestCase {
    pub id: String,
    pub lojban: String,
    #[serde(default)]
    pub dialect: Option<String>,
    #[serde(default, rename = "translation-en")]
    pub translation_en: Option<String>,
    #[serde(default, rename = "gloss-en")]
    pub gloss_en: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub provenance: Vec<Provenance>,
    #[serde(default)]
    pub expectations: Expectations,
}

impl TestCase {
    #[requires(true)]
    #[ensures(ret -> !self.id.is_empty())]
    #[ensures(ret -> self.validate_xfail_metadata().is_ok())]
    #[ensures(ret -> self.dialect_definition().is_ok())]
    pub fn is_valid_fixture_metadata(&self) -> bool {
        !self.id.is_empty()
            && self
                .dialect
                .as_deref()
                .is_none_or(|formula| parse_dialect_definition(formula).is_ok())
            && self.validate_xfail_metadata().is_ok()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn dialect_definition(&self) -> Result<DialectDefinition, FixtureError> {
        match &self.dialect {
            Some(formula) => {
                parse_dialect_definition(formula).map_err(|source| FixtureError::InvalidDialect {
                    id: self.id.clone(),
                    formula: formula.clone(),
                    message: source.message().to_owned(),
                })
            }
            None => Ok(DialectDefinition::baseline()),
        }
    }

    #[ensures(ret.iter().all(|facet| self.expectation_status(*facet).is_some()))]
    #[requires(true)]
    pub fn available_facets(&self) -> BTreeSet<Facet> {
        let mut facets = BTreeSet::new();
        if self.expectations.morphology.is_some() {
            facets.insert(Facet::Morphology);
        }
        if self.expectations.syntax.is_some() {
            facets.insert(Facet::Syntax);
        }
        if self.expectations.warnings.is_some() {
            facets.insert(Facet::Warnings);
        }
        if let Some(output) = &self.expectations.output {
            if output
                .vlasei
                .as_ref()
                .is_some_and(|vlasei| vlasei.brackets.is_some())
            {
                facets.insert(Facet::VlaseiBrackets);
            }
            if output
                .vlasei
                .as_ref()
                .is_some_and(|vlasei| vlasei.tree.is_some())
            {
                facets.insert(Facet::VlaseiTree);
            }
            if output
                .vlasei
                .as_ref()
                .is_some_and(|vlasei| vlasei.json.is_some())
            {
                facets.insert(Facet::VlaseiJson);
            }
            if output
                .gentufa
                .as_ref()
                .is_some_and(|gentufa| gentufa.brackets.is_some())
            {
                facets.insert(Facet::GentufaBrackets);
            }
            if output
                .gentufa
                .as_ref()
                .is_some_and(|gentufa| gentufa.tree.is_some())
            {
                facets.insert(Facet::GentufaTree);
            }
            if output
                .gentufa
                .as_ref()
                .is_some_and(|gentufa| gentufa.json.is_some())
            {
                facets.insert(Facet::GentufaJson);
            }
        }
        facets
    }

    #[ensures(ret.is_ok() || self.expectations.syntax.as_ref().and_then(|syntax| syntax.xfail.as_ref()).is_some())]
    #[requires(true)]
    pub fn validate_xfail_metadata(&self) -> Result<(), FixtureError> {
        let Some(syntax) = &self.expectations.syntax else {
            return Ok(());
        };
        let Some(xfail) = &syntax.xfail else {
            return Ok(());
        };
        if !xfail.is_valid_for_status(syntax.status) {
            return Err(FixtureError::InvalidXfail {
                id: self.id.clone(),
                message: xfail.invalid_reason_for_status(syntax.status),
            });
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn expectation_status(&self, facet: Facet) -> Option<ExpectationStatus> {
        match facet {
            Facet::Morphology => self
                .expectations
                .morphology
                .as_ref()
                .map(|value| value.status),
            Facet::Syntax => self.expectations.syntax.as_ref().map(|value| value.status),
            Facet::Warnings => self
                .expectations
                .warnings
                .as_ref()
                .map(|_| ExpectationStatus::Success),
            Facet::VlaseiBrackets => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::VlaseiTree => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.tree.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::VlaseiJson => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.json.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::GentufaBrackets => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::GentufaTree => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.tree.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::GentufaJson => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.json.as_ref())
                .map(|_| ExpectationStatus::Success),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[invariant(true)]
#[invariant(::Cll => true)]
#[invariant(::Muplis => true)]
#[invariant(::Corpus => true)]
#[invariant(::Adhoc => true)]
#[invariant(::Other => true)]
pub enum Provenance {
    Cll {
        chapter: u16,
        #[serde(rename = "section-number")]
        section_number: String,
        #[serde(rename = "section-id")]
        section_id: String,
        #[serde(default, rename = "example-number")]
        example_number: Option<String>,
        #[serde(default, rename = "example-id")]
        example_id: Option<String>,
        #[serde(default, rename = "source-path")]
        source_path: Option<String>,
    },
    Muplis {
        #[serde(rename = "collection-id")]
        collection_id: String,
        #[serde(default, rename = "item-id")]
        item_id: Option<String>,
        #[serde(default)]
        form: Option<MuplisForm>,
        #[serde(default)]
        url: Option<String>,
    },
    Corpus {
        corpus: String,
        #[serde(default, rename = "entry-id")]
        entry_id: Option<String>,
        #[serde(default)]
        md5: Option<String>,
    },
    Adhoc {
        #[serde(default)]
        description: Option<String>,
    },
    Other {
        name: String,
        #[serde(default)]
        url: Option<String>,
        #[serde(default)]
        description: Option<String>,
    },
}

impl Provenance {
    #[requires(true)]
    #[ensures(true)]
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Cll { .. } => "cll",
            Self::Muplis { .. } => "muplis",
            Self::Corpus { .. } => "corpus",
            Self::Adhoc { .. } => "adhoc",
            Self::Other { .. } => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MuplisForm {
    Front,
    Canonical,
}

impl fmt::Display for MuplisForm {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Front => f.write_str("front"),
            Self::Canonical => f.write_str("canonical"),
        }
    }
}

impl std::str::FromStr for MuplisForm {
    type Err = String;

    #[requires(true)]
    #[ensures(true)]
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "front" => Ok(Self::Front),
            "canonical" => Ok(Self::Canonical),
            other => Err(format!("unknown Muplis form `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct Expectations {
    #[serde(default)]
    pub output: Option<OutputExpectations>,
    #[serde(default)]
    pub morphology: Option<MorphologyExpectation>,
    #[serde(default)]
    pub syntax: Option<SyntaxExpectation>,
    #[serde(default)]
    pub warnings: Option<StructuredExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct OutputExpectations {
    #[serde(default)]
    pub vlasei: Option<CommandOutputExpectation>,
    #[serde(default)]
    pub gentufa: Option<CommandOutputExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct CommandOutputExpectation {
    #[serde(default)]
    pub brackets: Option<TextExpectation>,
    #[serde(default)]
    pub tree: Option<TextExpectation>,
    #[serde(default)]
    pub json: Option<TextExpectation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct MorphologyExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub raw: Option<TextExpectation>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct SyntaxExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub raw: Option<TextExpectation>,
    #[serde(default)]
    pub error: Option<ParseErrorExpectation>,
    #[serde(default)]
    pub xfail: Option<XfailExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct XfailExpectation {
    pub source: String,
    pub reason: String,
    #[serde(rename = "accepted-status")]
    pub accepted_status: ExpectationStatus,
}

impl XfailExpectation {
    #[requires(true)]
    #[ensures(true)]
    pub fn is_valid_for_status(&self, expected_status: ExpectationStatus) -> bool {
        self.invalid_reason_for_status(expected_status).is_empty()
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_reason_for_status(&self, expected_status: ExpectationStatus) -> String {
        if self.source.is_empty() {
            return "xfail source must not be empty".to_owned();
        }
        if self.reason.is_empty() {
            return "xfail reason must not be empty".to_owned();
        }
        if !matches!(
            expected_status,
            ExpectationStatus::Success | ExpectationStatus::Failure
        ) {
            return format!("xfail cannot be attached to {expected_status:?} expectation");
        }
        if !matches!(
            self.accepted_status,
            ExpectationStatus::Success | ExpectationStatus::Failure
        ) {
            return format!(
                "xfail accepted-status must be success or failure, got {:?}",
                self.accepted_status
            );
        }
        if self.accepted_status == expected_status {
            return "xfail accepted-status must differ from the normative status".to_owned();
        }
        String::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ParseErrorExpectation {
    pub position: usize,
    #[serde(default, rename = "allowed-next")]
    pub allowed_next: Vec<AllowedNextExpectation>,
    #[serde(default)]
    pub message: Option<String>,
}

#[invariant(true)]
#[invariant(::Cmavo => !text.is_empty())]
#[invariant(::CmavoOf => !selmaho.is_empty() && !values.is_empty() && values.iter().all(|value| !value.is_empty()))]
#[invariant(::SingleWordQuote => !markers.is_empty() && markers.iter().all(|marker| !marker.is_empty()))]
#[invariant(::Negative => true)]
#[invariant(::Other => !name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AllowedNextExpectation {
    Cmavo {
        text: String,
    },
    CmavoOf {
        selmaho: String,
        values: Vec<String>,
    },
    Brivla,
    Cmevla,
    Letter,
    By,
    Pa,
    ZoQuote,
    SingleWordQuote {
        markers: Vec<String>,
    },
    ZoiQuote,
    LohuQuote,
    QuotedWord,
    AnyWordLike,
    Eof,
    Negative {
        expectation: Box<AllowedNextExpectation>,
    },
    Other {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct StructuredExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[invariant(true)]
pub struct TextExpectation {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ExpectationStatus {
    Success,
    Failure,
    Pending,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum Facet {
    Morphology,
    Syntax,
    Warnings,
    VlaseiBrackets,
    VlaseiTree,
    VlaseiJson,
    GentufaBrackets,
    GentufaTree,
    GentufaJson,
}

impl Facet {
    #[requires(true)]
    #[ensures(true)]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Morphology,
            Self::Syntax,
            Self::Warnings,
            Self::VlaseiBrackets,
            Self::VlaseiTree,
            Self::VlaseiJson,
            Self::GentufaBrackets,
            Self::GentufaTree,
            Self::GentufaJson,
        ]
    }
}

impl fmt::Display for Facet {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Morphology => "morphology",
            Self::Syntax => "syntax",
            Self::Warnings => "warnings",
            Self::VlaseiBrackets => "vlasei-brackets",
            Self::VlaseiTree => "vlasei-tree",
            Self::VlaseiJson => "vlasei-json",
            Self::GentufaBrackets => "gentufa-brackets",
            Self::GentufaTree => "gentufa-tree",
            Self::GentufaJson => "gentufa-json",
        };
        f.write_str(text)
    }
}

impl std::str::FromStr for Facet {
    type Err = String;

    #[requires(true)]
    #[ensures(true)]
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "morphology" => Ok(Self::Morphology),
            "syntax" => Ok(Self::Syntax),
            "warnings" => Ok(Self::Warnings),
            "vlasei-brackets" => Ok(Self::VlaseiBrackets),
            "vlasei-tree" => Ok(Self::VlaseiTree),
            "vlasei-json" => Ok(Self::VlaseiJson),
            "gentufa-brackets" => Ok(Self::GentufaBrackets),
            "gentufa-tree" => Ok(Self::GentufaTree),
            "gentufa-json" => Ok(Self::GentufaJson),
            other => Err(format!("unknown fixture facet `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct FixtureProfile {
    #[serde(default)]
    pub facets: Vec<Facet>,
    #[serde(default)]
    pub selector: FixtureSelector,
}

impl FixtureProfile {
    #[ensures(ret -> self.selector.is_valid())]
    #[requires(true)]
    pub fn is_valid(&self) -> bool {
        self.selector.is_valid()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct FixtureSelector {
    #[serde(default)]
    pub provenance: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default, rename = "path-prefixes")]
    pub path_prefixes: Vec<String>,
    #[serde(default)]
    pub cll: Option<CllSelector>,
    #[serde(default)]
    pub muplis: Option<MuplisSelector>,
}

impl FixtureSelector {
    #[ensures(ret -> self.provenance.iter().all(|value| !value.is_empty()))]
    #[ensures(ret -> self.tags.iter().all(|value| !value.is_empty()))]
    #[ensures(ret -> self.ids.iter().all(|value| !value.is_empty()))]
    #[ensures(ret -> self.path_prefixes.iter().all(|value| !value.is_empty()))]
    #[requires(true)]
    pub fn is_valid(&self) -> bool {
        self.provenance.iter().all(|value| !value.is_empty())
            && self.tags.iter().all(|value| !value.is_empty())
            && self.ids.iter().all(|value| !value.is_empty())
            && self.path_prefixes.iter().all(|value| !value.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct CllSelector {
    #[serde(default)]
    pub chapter: Option<u16>,
    #[serde(default, rename = "section-number")]
    pub section_number: Option<String>,
    #[serde(default, rename = "section-id")]
    pub section_id: Option<String>,
    #[serde(default, rename = "example-number")]
    pub example_number: Option<String>,
    #[serde(default, rename = "example-id")]
    pub example_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct MuplisSelector {
    #[serde(default, rename = "collection-id")]
    pub collection_id: Option<String>,
    #[serde(default, rename = "item-id")]
    pub item_id: Option<String>,
    #[serde(default)]
    pub form: Option<MuplisForm>,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct LoadedTestCase {
    pub path: PathBuf,
    pub test_case: TestCase,
}

#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Read => true)]
#[invariant(::Write => true)]
#[invariant(::ParseToml => true)]
#[invariant(::EncodeToml => true)]
#[invariant(::ParseJson => true)]
#[invariant(::Walk => true)]
#[invariant(::DuplicateId => true)]
#[invariant(::UnknownFacet => true)]
#[invariant(::InvalidDialect => true)]
#[invariant(::InvalidXfail => true)]
#[invariant(::LegacyExpectationFormat => true)]
pub enum FixtureError {
    #[error("failed to read `{path}`: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write `{path}`: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse TOML `{path}`: {source}")]
    ParseToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to encode TOML `{path}`: {source}")]
    EncodeToml {
        path: PathBuf,
        source: toml::ser::Error,
    },
    #[error("failed to parse JSON `{path}`: {source}")]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to walk fixture tree `{path}`: {source}")]
    Walk {
        path: PathBuf,
        source: walkdir::Error,
    },
    #[error("duplicate fixture id `{id}` in `{first}` and `{second}`")]
    DuplicateId {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
    #[error("profile `{profile}` references unknown facet `{facet}`")]
    UnknownFacet { profile: PathBuf, facet: String },
    #[error("fixture `{id}` has invalid dialect formula `{formula}`: {message}")]
    InvalidDialect {
        id: String,
        formula: String,
        message: String,
    },
    #[error("fixture `{id}` has invalid syntax xfail metadata: {message}")]
    InvalidXfail { id: String, message: String },
    #[error("fixture `{path}` uses legacy expectation format: {message}")]
    LegacyExpectationFormat { path: PathBuf, message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct FixtureExport {
    #[serde(default = "default_schema_version", rename = "schema-version")]
    pub schema_version: u16,
    pub cases: Vec<TestCase>,
}

#[requires(true)]
#[ensures(true)]
fn default_schema_version() -> u16 {
    1
}

#[requires(true)]
#[ensures(true)]
pub fn load_fixture_file(path: impl AsRef<Path>) -> Result<TestCase, FixtureError> {
    let path = path.as_ref();
    let text = read_text(path)?;
    reject_legacy_expectation_format(path, &text)?;
    toml::from_str(&text).map_err(|source| FixtureError::ParseToml {
        path: path.to_path_buf(),
        source,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn reject_legacy_expectation_format(path: &Path, text: &str) -> Result<(), FixtureError> {
    let legacy_patterns = [
        "[expectations.syntax.parse-tree]",
        "parse-tree",
        "BaseWord =",
        "StandaloneIndicator =",
        "NotEof =",
        "LojbanText =",
        "constructor =",
        "words = [",
        "kind = \"node\"",
        "kind = \"base-word\"",
        "kind = \"standalone-indicator\"",
        "kind = \"emphasized\"",
        "kind = \"with-indicator\"",
        "kind = \"not-eof\"",
        "kind = \"bare\"",
        "kind = \"zo-quote\"",
        "kind = \"zoi-quote\"",
        "kind = \"lohu-quote\"",
        "kind = \"single-word-quote\"",
        "kind = \"letter\"",
        "kind = \"zei-lujvo\"",
    ];
    for pattern in legacy_patterns {
        if text.contains(pattern) {
            return Err(FixtureError::LegacyExpectationFormat {
                path: path.to_path_buf(),
                message: format!("found `{pattern}`"),
            });
        }
    }
    Ok(())
}

#[requires(test_case.is_valid_fixture_metadata())]
#[ensures(true)]
pub fn write_fixture_file(
    path: impl AsRef<Path>,
    test_case: &TestCase,
) -> Result<(), FixtureError> {
    let path = path.as_ref();
    let mut text = format_test_case_toml(test_case).map_err(|source| FixtureError::EncodeToml {
        path: path.to_path_buf(),
        source,
    })?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| FixtureError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, text).map_err(|source| FixtureError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn load_fixture_tree(root: impl AsRef<Path>) -> Result<Vec<LoadedTestCase>, FixtureError> {
    let root = root.as_ref();
    let mut loaded = Vec::new();
    for path in fixture_paths(root)? {
        loaded.push(load_fixture_path(path)?);
    }
    Ok(loaded)
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|paths| paths.iter().all(|path| path.extension().is_some_and(|ext| ext == "toml"))))]
#[requires(true)]
pub fn fixture_paths(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, FixtureError> {
    let root = root.as_ref();
    let mut paths = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|source| FixtureError::Walk {
            path: root.to_path_buf(),
            source,
        })?;
        if !entry.file_type().is_file()
            || entry.path().extension().is_none_or(|ext| ext != "toml")
            || entry
                .path()
                .components()
                .any(|component| component.as_os_str() == "profiles")
        {
            continue;
        }
        paths.push(entry.path().to_path_buf());
    }
    Ok(paths)
}

#[requires(true)]
#[ensures(true)]
pub fn load_fixture_path(path: impl AsRef<Path>) -> Result<LoadedTestCase, FixtureError> {
    let path = path.as_ref();
    let test_case = load_fixture_file(path)?;
    Ok(LoadedTestCase {
        path: path.to_path_buf(),
        test_case,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn visit_fixture_tree<F>(root: impl AsRef<Path>, mut visitor: F) -> Result<usize, FixtureError>
where
    F: FnMut(LoadedTestCase) -> Result<(), FixtureError>,
{
    let paths = fixture_paths(root)?;
    let count = paths.len();
    for path in paths {
        visitor(load_fixture_path(path)?)?;
    }
    Ok(count)
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|summary| summary.fixture_count > 0))]
#[requires(true)]
pub fn validate_fixture_tree(root: impl AsRef<Path>) -> Result<FixtureSummary, FixtureError> {
    let root = root.as_ref();
    let mut seen = BTreeMap::new();
    let mut fixture_count = 0;
    for path in fixture_paths(root)? {
        let test_case = load_fixture_file(&path)?;
        test_case.dialect_definition()?;
        test_case.validate_xfail_metadata()?;
        if let Some(first) = seen.insert(test_case.id.clone(), path.clone()) {
            return Err(FixtureError::DuplicateId {
                id: test_case.id,
                first,
                second: path,
            });
        }
        fixture_count += 1;
    }
    let profiles = load_profiles(root.join("profiles"))?;
    Ok(FixtureSummary {
        fixture_count,
        profile_count: profiles.len(),
    })
}

#[requires(true)]
#[ensures(true)]
pub fn load_profiles(
    root: impl AsRef<Path>,
) -> Result<BTreeMap<String, FixtureProfile>, FixtureError> {
    let root = root.as_ref();
    let mut profiles = BTreeMap::new();
    if !root.exists() {
        return Ok(profiles);
    }
    for entry in WalkDir::new(root).max_depth(1).sort_by_file_name() {
        let entry = entry.map_err(|source| FixtureError::Walk {
            path: root.to_path_buf(),
            source,
        })?;
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "toml")
        {
            continue;
        }
        let path = entry.path();
        let text = read_text(path)?;
        let profile: FixtureProfile =
            toml::from_str(&text).map_err(|source| FixtureError::ParseToml {
                path: path.to_path_buf(),
                source,
            })?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        profiles.insert(name, profile);
    }
    Ok(profiles)
}

#[requires(!name.is_empty(), "fixture profile names must not be empty")]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(FixtureProfile::is_valid))]
pub fn load_profile(
    fixtures_root: impl AsRef<Path>,
    name: &str,
) -> Result<FixtureProfile, FixtureError> {
    let path = fixtures_root
        .as_ref()
        .join("profiles")
        .join(format!("{name}.toml"));
    let text = read_text(&path)?;
    toml::from_str(&text).map_err(|source| FixtureError::ParseToml { path, source })
}

#[requires(selector.is_valid())]
#[expensive_ensures(ret.iter().all(|fixture| fixture.test_case.is_valid_fixture_metadata()))]
pub fn filter_fixtures<'a>(
    root: &Path,
    fixtures: &'a [LoadedTestCase],
    selector: &FixtureSelector,
) -> Vec<&'a LoadedTestCase> {
    fixtures
        .iter()
        .filter(|fixture| matches_selector(root, fixture, selector))
        .collect()
}

#[requires(selector.is_valid())]
#[ensures(true)]
pub fn fixture_matches_selector(
    root: &Path,
    fixture: &LoadedTestCase,
    selector: &FixtureSelector,
) -> bool {
    matches_selector(root, fixture, selector)
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|summary| summary.written > 0))]
#[requires(true)]
pub fn import_export_file(
    input_path: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<ImportSummary, FixtureError> {
    let input_path = input_path.as_ref();
    let text = read_text(input_path)?;
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    deserializer.disable_recursion_limit();
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
    let export =
        FixtureExport::deserialize(deserializer).map_err(|source| FixtureError::ParseJson {
            path: input_path.to_path_buf(),
            source,
        })?;
    let output_root = output_root.as_ref();
    let mut written = 0;
    for case in &export.cases {
        let path = output_root.join(path_for_case(case));
        write_fixture_file(path, case)?;
        written += 1;
    }
    Ok(ImportSummary { written })
}

#[requires(case.is_valid_fixture_metadata())]
#[ensures(ret.extension().is_some_and(|ext| ext == "toml"))]
pub fn path_for_case(case: &TestCase) -> PathBuf {
    match case.provenance.first() {
        Some(Provenance::Cll {
            chapter,
            section_number,
            example_id,
            ..
        }) => {
            let file = example_id
                .as_deref()
                .unwrap_or(case.id.as_str())
                .replace(['/', '\\'], "_");
            PathBuf::from("cll")
                .join(format!("chapter-{chapter:02}"))
                .join(format!("section-{section_number}"))
                .join(format!("{file}.toml"))
        }
        Some(Provenance::Muplis {
            collection_id,
            item_id,
            form,
            ..
        }) => {
            let item = item_id.as_deref().unwrap_or(case.id.as_str());
            let suffix = form.map_or("unknown", |form| match form {
                MuplisForm::Front => "front",
                MuplisForm::Canonical => "canonical",
            });
            PathBuf::from("muplis")
                .join(format!("collection-{collection_id}"))
                .join(format!("{item}-{suffix}.toml"))
        }
        Some(Provenance::Corpus {
            corpus, entry_id, ..
        }) => {
            let item = entry_id.as_deref().unwrap_or(case.id.as_str());
            PathBuf::from("corpus")
                .join(corpus)
                .join(format!("{}.toml", item.replace(['/', '\\'], "_")))
        }
        Some(Provenance::Adhoc { .. }) | None => {
            PathBuf::from("adhoc").join(format!("{}.toml", case.id.replace('.', "/")))
        }
        Some(Provenance::Other { name, .. }) => PathBuf::from("other")
            .join(name)
            .join(format!("{}.toml", case.id.replace(['/', '\\'], "_"))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct FixtureSummary {
    pub fixture_count: usize,
    pub profile_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct ImportSummary {
    pub written: usize,
}

#[requires(true)]
#[ensures(true)]
fn matches_selector(root: &Path, fixture: &LoadedTestCase, selector: &FixtureSelector) -> bool {
    if !selector.ids.is_empty() && !selector.ids.iter().any(|id| id == &fixture.test_case.id) {
        return false;
    }
    if !selector.tags.is_empty()
        && !selector.tags.iter().all(|tag| {
            fixture
                .test_case
                .tags
                .iter()
                .any(|fixture_tag| fixture_tag == tag)
        })
    {
        return false;
    }
    if !selector.provenance.is_empty()
        && !fixture.test_case.provenance.iter().any(|provenance| {
            selector
                .provenance
                .iter()
                .any(|kind| kind == provenance.kind_name())
        })
    {
        return false;
    }
    if !selector.path_prefixes.is_empty() {
        let relative = fixture.path.strip_prefix(root).unwrap_or(&fixture.path);
        let relative_text = relative.to_string_lossy();
        if !selector
            .path_prefixes
            .iter()
            .any(|prefix| relative_text.starts_with(prefix))
        {
            return false;
        }
    }
    if let Some(cll) = &selector.cll
        && !fixture
            .test_case
            .provenance
            .iter()
            .any(|provenance| matches_cll_selector(provenance, cll))
    {
        return false;
    }
    if let Some(muplis) = &selector.muplis
        && !fixture
            .test_case
            .provenance
            .iter()
            .any(|provenance| matches_muplis_selector(provenance, muplis))
    {
        return false;
    }
    true
}

#[requires(true)]
#[ensures(true)]
fn matches_cll_selector(provenance: &Provenance, selector: &CllSelector) -> bool {
    let Provenance::Cll {
        chapter,
        section_number,
        section_id,
        example_number,
        example_id,
        ..
    } = provenance
    else {
        return false;
    };
    selector.chapter.is_none_or(|value| value == *chapter)
        && selector
            .section_number
            .as_ref()
            .is_none_or(|value| value == section_number)
        && selector
            .section_id
            .as_ref()
            .is_none_or(|value| value == section_id)
        && selector
            .example_number
            .as_ref()
            .is_none_or(|value| example_number.as_ref() == Some(value))
        && selector
            .example_id
            .as_ref()
            .is_none_or(|value| example_id.as_ref() == Some(value))
}

#[requires(true)]
#[ensures(true)]
fn matches_muplis_selector(provenance: &Provenance, selector: &MuplisSelector) -> bool {
    let Provenance::Muplis {
        collection_id,
        item_id,
        form,
        ..
    } = provenance
    else {
        return false;
    };
    selector
        .collection_id
        .as_ref()
        .is_none_or(|value| value == collection_id)
        && selector
            .item_id
            .as_ref()
            .is_none_or(|value| item_id.as_ref() == Some(value))
        && selector.form.is_none_or(|value| form == &Some(value))
}

#[requires(true)]
#[ensures(true)]
fn read_text(path: &Path) -> Result<String, FixtureError> {
    fs::read_to_string(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })
}
