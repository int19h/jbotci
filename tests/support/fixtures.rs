//! Unified TOML fixture loader, selectors, and runner support.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use jbotci_morphology::WordWithModifiers;
use jbotci_syntax::SyntaxValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl TestCase {
    pub fn available_facets(&self) -> BTreeSet<Facet> {
        let mut facets = BTreeSet::new();
        if self
            .expectations
            .output
            .as_ref()
            .is_some_and(|output| output.brackets.is_some())
        {
            facets.insert(Facet::Brackets);
        }
        if self.expectations.morphology.is_some() {
            facets.insert(Facet::Morphology);
        }
        if self.expectations.syntax.is_some() {
            facets.insert(Facet::Syntax);
        }
        if self.expectations.syntax_refs.is_some() {
            facets.insert(Facet::SyntaxRefs);
        }
        if self.expectations.warnings.is_some() {
            facets.insert(Facet::Warnings);
        }
        facets
    }
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
pub enum MuplisForm {
    Front,
    Canonical,
}

impl fmt::Display for MuplisForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Front => f.write_str("front"),
            Self::Canonical => f.write_str("canonical"),
        }
    }
}

impl std::str::FromStr for MuplisForm {
    type Err = String;

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
pub struct Expectations {
    #[serde(default)]
    pub output: Option<OutputExpectation>,
    #[serde(default)]
    pub morphology: Option<MorphologyExpectation>,
    #[serde(default)]
    pub syntax: Option<SyntaxExpectation>,
    #[serde(default, rename = "syntax-refs")]
    pub syntax_refs: Option<StructuredExpectation>,
    #[serde(default)]
    pub warnings: Option<StructuredExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputExpectation {
    #[serde(default)]
    pub brackets: Option<TextExpectation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphologyExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub words: Vec<WordWithModifiers>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntaxExpectation {
    pub status: ExpectationStatus,
    #[serde(default, rename = "parse-tree")]
    pub parse_tree: Option<SyntaxValue>,
    #[serde(default)]
    pub error: Option<ParseErrorExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParseErrorExpectation {
    pub position: usize,
    #[serde(default, rename = "allowed-next")]
    pub allowed_next: Vec<AllowedNextExpectation>,
    #[serde(default)]
    pub message: Option<String>,
}

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
    AnyWordWithModifiers,
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
pub struct StructuredExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextExpectation {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectationStatus {
    Success,
    Failure,
    Pending,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Facet {
    Morphology,
    Syntax,
    SyntaxRefs,
    Warnings,
    Brackets,
}

impl Facet {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Morphology,
            Self::Syntax,
            Self::SyntaxRefs,
            Self::Warnings,
            Self::Brackets,
        ]
    }
}

impl fmt::Display for Facet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Morphology => "morphology",
            Self::Syntax => "syntax",
            Self::SyntaxRefs => "syntax-refs",
            Self::Warnings => "warnings",
            Self::Brackets => "brackets",
        };
        f.write_str(text)
    }
}

impl std::str::FromStr for Facet {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "morphology" => Ok(Self::Morphology),
            "syntax" => Ok(Self::Syntax),
            "syntax-refs" | "syntaxrefs" => Ok(Self::SyntaxRefs),
            "warnings" => Ok(Self::Warnings),
            "brackets" => Ok(Self::Brackets),
            other => Err(format!("unknown fixture facet `{other}`")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureProfile {
    #[serde(default)]
    pub facets: Vec<Facet>,
    #[serde(default)]
    pub selector: FixtureSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
pub struct MuplisSelector {
    #[serde(default, rename = "collection-id")]
    pub collection_id: Option<String>,
    #[serde(default, rename = "item-id")]
    pub item_id: Option<String>,
    #[serde(default)]
    pub form: Option<MuplisForm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedTestCase {
    pub path: PathBuf,
    pub test_case: TestCase,
}

#[derive(Debug, Error)]
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureExport {
    #[serde(default = "default_schema_version", rename = "schema-version")]
    pub schema_version: u16,
    pub cases: Vec<TestCase>,
}

fn default_schema_version() -> u16 {
    1
}

pub fn load_fixture_file(path: impl AsRef<Path>) -> Result<TestCase, FixtureError> {
    let path = path.as_ref();
    let text = read_text(path)?;
    toml::from_str(&text).map_err(|source| FixtureError::ParseToml {
        path: path.to_path_buf(),
        source,
    })
}

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

fn format_test_case_toml(test_case: &TestCase) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    push_field(&mut output, "id", &test_case.id)?;
    push_field(&mut output, "lojban", &test_case.lojban)?;
    push_optional_field(&mut output, "translation-en", &test_case.translation_en)?;
    push_optional_field(&mut output, "gloss-en", &test_case.gloss_en)?;
    if !test_case.tags.is_empty() {
        push_field(&mut output, "tags", &test_case.tags)?;
    }
    for provenance in &test_case.provenance {
        push_provenance_toml(&mut output, provenance)?;
    }
    push_expectations_toml(&mut output, &test_case.expectations)?;
    Ok(output)
}

fn push_provenance_toml(
    output: &mut String,
    provenance: &Provenance,
) -> Result<(), toml::ser::Error> {
    output.push_str("\n[[provenance]]\n");
    push_field(output, "kind", provenance.kind_name())?;
    match provenance {
        Provenance::Cll {
            chapter,
            section_number,
            section_id,
            example_number,
            example_id,
            source_path,
        } => {
            push_field(output, "chapter", chapter)?;
            push_field(output, "section-number", section_number)?;
            push_field(output, "section-id", section_id)?;
            push_optional_field(output, "example-number", example_number)?;
            push_optional_field(output, "example-id", example_id)?;
            push_optional_field(output, "source-path", source_path)?;
        }
        Provenance::Muplis {
            collection_id,
            item_id,
            form,
            url,
        } => {
            push_field(output, "collection-id", collection_id)?;
            push_optional_field(output, "item-id", item_id)?;
            push_optional_field(output, "form", form)?;
            push_optional_field(output, "url", url)?;
        }
        Provenance::Corpus {
            corpus,
            entry_id,
            md5,
        } => {
            push_field(output, "corpus", corpus)?;
            push_optional_field(output, "entry-id", entry_id)?;
            push_optional_field(output, "md5", md5)?;
        }
        Provenance::Adhoc { description } => {
            push_optional_field(output, "description", description)?;
        }
        Provenance::Other {
            name,
            url,
            description,
        } => {
            push_field(output, "name", name)?;
            push_optional_field(output, "url", url)?;
            push_optional_field(output, "description", description)?;
        }
    }
    Ok(())
}

fn push_expectations_toml(
    output: &mut String,
    expectations: &Expectations,
) -> Result<(), toml::ser::Error> {
    if let Some(output_expectation) = &expectations.output
        && output_expectation.brackets.is_some()
    {
        output.push_str("\n[expectations.output]\n");
        if let Some(brackets) = &output_expectation.brackets {
            push_field(output, "brackets", brackets)?;
        }
    }
    if let Some(morphology) = &expectations.morphology {
        output.push_str("\n[expectations.morphology]\n");
        push_field(output, "status", &morphology.status)?;
        if !morphology.words.is_empty() {
            output.push_str("words = [\n");
            for word in &morphology.words {
                output.push_str("    ");
                output.push_str(&format_toml_value(word)?);
                output.push_str(",\n");
            }
            output.push_str("]\n");
        }
        push_optional_field(output, "error", &morphology.error)?;
    }
    if let Some(syntax) = &expectations.syntax {
        output.push_str("\n[expectations.syntax]\n");
        push_field(output, "status", &syntax.status)?;
        if let Some(parse_tree) = &syntax.parse_tree {
            output.push_str("parse-tree = ");
            output.push_str(&format_syntax_value_toml(parse_tree, 0)?);
            output.push('\n');
        }
        push_optional_field(output, "error", &syntax.error)?;
    }
    if let Some(syntax_refs) = &expectations.syntax_refs {
        output.push_str("\n[expectations.syntax-refs]\n");
        push_field(output, "status", &syntax_refs.status)?;
        push_optional_field(output, "value", &syntax_refs.value)?;
    }
    if let Some(warnings) = &expectations.warnings {
        output.push_str("\n[expectations.warnings]\n");
        push_field(output, "status", &warnings.status)?;
        push_optional_field(output, "value", &warnings.value)?;
    }
    Ok(())
}

fn push_field<T: Serialize + ?Sized>(
    output: &mut String,
    key: &str,
    value: &T,
) -> Result<(), toml::ser::Error> {
    output.push_str(key);
    output.push_str(" = ");
    output.push_str(&format_toml_value(value)?);
    output.push('\n');
    Ok(())
}

fn push_optional_field<T: Serialize>(
    output: &mut String,
    key: &str,
    value: &Option<T>,
) -> Result<(), toml::ser::Error> {
    if let Some(value) = value {
        push_field(output, key, value)?;
    }
    Ok(())
}

fn format_syntax_value_toml(
    value: &SyntaxValue,
    indent: usize,
) -> Result<String, toml::ser::Error> {
    match value {
        SyntaxValue::Null => Ok(r#"{ kind = "null" }"#.to_owned()),
        SyntaxValue::Bool { value } => Ok(format!(r#"{{ kind = "bool", value = {value} }}"#)),
        SyntaxValue::Integer { value } => Ok(format!(r#"{{ kind = "integer", value = {value} }}"#)),
        SyntaxValue::Text { value } => Ok(format!(
            r#"{{ kind = "text", value = {} }}"#,
            format_toml_value(value)?
        )),
        SyntaxValue::Word { word } => Ok(format!(
            r#"{{ kind = "word", word = {} }}"#,
            format_toml_value(word.as_ref())?
        )),
        SyntaxValue::Json { value } => Ok(format!(
            r#"{{ kind = "json", value = {} }}"#,
            format_toml_value(value)?
        )),
        SyntaxValue::List { items } => format_syntax_list_toml(items, indent),
        SyntaxValue::Node { node } => {
            let child = indent + 4;
            let field_indent = indent + 8;
            let mut output = String::new();
            output.push_str("{\n");
            output.push_str(&spaces(child));
            output.push_str(r#"kind = "node","#);
            output.push('\n');
            output.push_str(&spaces(child));
            output.push_str("node = {\n");
            output.push_str(&spaces(field_indent));
            output.push_str("constructor = ");
            output.push_str(&format_toml_value(&node.constructor)?);
            output.push_str(",\n");
            output.push_str(&spaces(field_indent));
            output.push_str("fields = ");
            output.push_str(&format_syntax_fields_toml(&node.fields, field_indent)?);
            output.push('\n');
            output.push_str(&spaces(child));
            output.push_str("}\n");
            output.push_str(&spaces(indent));
            output.push('}');
            Ok(output)
        }
    }
}

fn format_syntax_list_toml(
    items: &[SyntaxValue],
    indent: usize,
) -> Result<String, toml::ser::Error> {
    if items.is_empty() {
        return Ok(r#"{ kind = "list", items = [] }"#.to_owned());
    }
    let child = indent + 4;
    let item_indent = indent + 8;
    let mut output = String::new();
    output.push_str("{\n");
    output.push_str(&spaces(child));
    output.push_str(r#"kind = "list","#);
    output.push('\n');
    output.push_str(&spaces(child));
    output.push_str("items = [\n");
    for (index, item) in items.iter().enumerate() {
        output.push_str(&spaces(item_indent));
        output.push_str(&format_syntax_value_toml(item, item_indent)?);
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&spaces(child));
    output.push_str("]\n");
    output.push_str(&spaces(indent));
    output.push('}');
    Ok(output)
}

fn format_syntax_fields_toml(
    fields: &[jbotci_syntax::SyntaxField],
    indent: usize,
) -> Result<String, toml::ser::Error> {
    if fields.is_empty() {
        return Ok("[]".to_owned());
    }
    let item_indent = indent + 4;
    let mut output = String::new();
    output.push_str("[\n");
    for (index, field) in fields.iter().enumerate() {
        output.push_str(&spaces(item_indent));
        output.push_str("{\n");
        if let Some(name) = &field.name {
            output.push_str(&spaces(item_indent + 4));
            output.push_str("name = ");
            output.push_str(&format_toml_value(name)?);
            output.push_str(",\n");
        }
        output.push_str(&spaces(item_indent + 4));
        output.push_str("value = ");
        output.push_str(&format_syntax_value_toml(&field.value, item_indent + 4)?);
        output.push('\n');
        output.push_str(&spaces(item_indent));
        output.push('}');
        if index + 1 != fields.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&spaces(indent));
    output.push(']');
    Ok(output)
}

fn format_toml_value<T: Serialize + ?Sized>(value: &T) -> Result<String, toml::ser::Error> {
    let mut output = String::new();
    value.serialize(toml::ser::ValueSerializer::new(&mut output))?;
    Ok(output)
}

fn spaces(count: usize) -> String {
    " ".repeat(count)
}

pub fn load_fixture_tree(root: impl AsRef<Path>) -> Result<Vec<LoadedTestCase>, FixtureError> {
    let root = root.as_ref();
    let mut loaded = Vec::new();
    visit_fixture_tree(root, |test_case| {
        loaded.push(test_case);
        Ok(())
    })?;
    Ok(loaded)
}

pub fn visit_fixture_tree<F>(root: impl AsRef<Path>, mut visitor: F) -> Result<usize, FixtureError>
where
    F: FnMut(LoadedTestCase) -> Result<(), FixtureError>,
{
    let root = root.as_ref();
    let mut count = 0;
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
        let path = entry.path().to_path_buf();
        let test_case = load_fixture_file(&path)?;
        visitor(LoadedTestCase { path, test_case })?;
        count += 1;
    }
    Ok(count)
}

pub fn validate_fixture_tree(root: impl AsRef<Path>) -> Result<FixtureSummary, FixtureError> {
    let root = root.as_ref();
    let mut seen = BTreeMap::new();
    let mut fixture_count = 0;
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
        let path = entry.path().to_path_buf();
        let test_case = load_fixture_file(&path)?;
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

pub fn fixture_matches_selector(
    root: &Path,
    fixture: &LoadedTestCase,
    selector: &FixtureSelector,
) -> bool {
    matches_selector(root, fixture, selector)
}

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

pub trait FixtureBackend {
    fn run(&self, fixture: &LoadedTestCase, facet: Facet) -> FacetResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetResult {
    pub status: FacetStatus,
    pub message: Option<String>,
}

impl FacetResult {
    pub fn passed() -> Self {
        Self {
            status: FacetStatus::Passed,
            message: None,
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            status: FacetStatus::Failed,
            message: Some(message.into()),
        }
    }

    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            status: FacetStatus::Skipped,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunSummary {
    pub selected_fixtures: usize,
    pub selected_facets: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

pub fn run_fixture_facets<B: FixtureBackend>(
    backend: &B,
    fixtures: &[&LoadedTestCase],
    facets: &[Facet],
) -> RunSummary {
    let mut summary = RunSummary {
        selected_fixtures: fixtures.len(),
        selected_facets: facets.len(),
        ..RunSummary::default()
    };
    for fixture in fixtures {
        for facet in facets {
            match backend.run(fixture, *facet).status {
                FacetStatus::Passed => summary.passed += 1,
                FacetStatus::Failed => summary.failed += 1,
                FacetStatus::Skipped => summary.skipped += 1,
            }
        }
    }
    summary
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSummary {
    pub fixture_count: usize,
    pub profile_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub written: usize,
}

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

fn read_text(path: &Path) -> Result<String, FixtureError> {
    fs::read_to_string(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })
}
