//! Unified TOML fixture loader.

use std::fs;
use std::path::{Path, PathBuf};

use jbotci_morphology::WordKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    pub id: String,
    pub lojban: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectations {
    #[serde(default)]
    pub morphology: Option<MorphologyExpectation>,
    #[serde(default)]
    pub syntax: Option<FacetExpectation>,
    #[serde(default)]
    pub semantics: Option<FacetExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphologyExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub tokens: Vec<TokenExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenExpectation {
    pub kind: WordKind,
    pub text: String,
    #[serde(default)]
    pub canonical: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacetExpectation {
    pub status: ExpectationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectationStatus {
    Success,
    Failure,
    Pending,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedTestCase {
    pub path: PathBuf,
    pub test_case: TestCase,
}

#[derive(Debug, Error)]
pub enum FixtureError {
    #[error("failed to read fixture `{path}`: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse fixture `{path}`: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to walk fixture tree `{path}`: {source}")]
    Walk {
        path: PathBuf,
        source: walkdir::Error,
    },
}

pub fn load_fixture_file(path: impl AsRef<Path>) -> Result<TestCase, FixtureError> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| FixtureError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_fixture_tree(root: impl AsRef<Path>) -> Result<Vec<LoadedTestCase>, FixtureError> {
    let root = root.as_ref();
    let mut loaded = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|source| FixtureError::Walk {
            path: root.to_path_buf(),
            source,
        })?;
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "toml")
        {
            continue;
        }
        let path = entry.path().to_path_buf();
        let test_case = load_fixture_file(&path)?;
        loaded.push(LoadedTestCase { path, test_case });
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_smoke_fixture() {
        let fixture_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/adhoc/smoke/coi.toml");
        let test_case = load_fixture_file(fixture_path).expect("fixture should load");
        assert_eq!(test_case.id, "adhoc.smoke.coi");
        assert_eq!(test_case.lojban, "coi");
        assert_eq!(
            test_case
                .expectations
                .morphology
                .expect("morphology expectation")
                .tokens[0]
                .kind,
            WordKind::Cmavo
        );
    }
}
