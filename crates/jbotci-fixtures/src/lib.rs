//! Unified TOML fixture loader, selectors, and runner support.

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
        if self.expectations.brackets.is_some() {
            facets.insert(Facet::Brackets);
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
    pub morphology: Option<MorphologyExpectation>,
    #[serde(default)]
    pub syntax: Option<SyntaxExpectation>,
    #[serde(default, rename = "syntax-refs")]
    pub syntax_refs: Option<StructuredExpectation>,
    #[serde(default)]
    pub warnings: Option<StructuredExpectation>,
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
#[serde(deny_unknown_fields)]
pub struct TextExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub text: Option<String>,
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
    let parse_tree = test_case
        .expectations
        .syntax
        .as_ref()
        .and_then(|syntax| syntax.parse_tree.as_ref());
    let mut serialized = test_case.clone();
    if let Some(syntax) = &mut serialized.expectations.syntax {
        syntax.parse_tree = None;
    }
    let mut text =
        toml::to_string_pretty(&serialized).map_err(|source| FixtureError::EncodeToml {
            path: path.to_path_buf(),
            source,
        })?;
    if let Some(parse_tree) = parse_tree {
        let assignment = format!(
            "parse-tree = {}\n",
            format_syntax_value_toml(parse_tree, 0).map_err(|source| {
                FixtureError::EncodeToml {
                    path: path.to_path_buf(),
                    source,
                }
            })?
        );
        text = insert_table_assignment(text, "[expectations.syntax]", &assignment);
    }
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

fn insert_table_assignment(mut document: String, table_header: &str, assignment: &str) -> String {
    let mut output = String::with_capacity(document.len() + assignment.len() + 1);
    let mut in_table = false;
    let mut inserted = false;
    if !document.ends_with('\n') {
        document.push('\n');
    }
    for line in document.lines() {
        if in_table && line.starts_with('[') {
            output.push_str(assignment);
            inserted = true;
            in_table = false;
        }
        output.push_str(line);
        output.push('\n');
        if line == table_header {
            in_table = true;
        }
    }
    if in_table && !inserted {
        output.push_str(assignment);
        inserted = true;
    }
    if !inserted {
        output.push('\n');
        output.push_str(table_header);
        output.push('\n');
        output.push_str(assignment);
    }
    output
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use jbotci_morphology::{WordKind, WordLike};
    use jbotci_syntax::{SyntaxField, SyntaxValue};

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
                .words[0]
                .base_word_kind(),
            Some(WordKind::Cmavo)
        );
    }

    #[test]
    fn profile_filters_cll_chapter_and_muplis_form() {
        let root = Path::new("fixtures");
        let cll = loaded_case(
            "fixtures/cll/chapter-18/section-18.3/c18e3d1.toml",
            TestCase {
                id: "cll.18.3.c18e3d1".into(),
                lojban: "coi".into(),
                translation_en: None,
                gloss_en: None,
                tags: vec!["cll".into()],
                provenance: vec![Provenance::Cll {
                    chapter: 18,
                    section_number: "18.3".into(),
                    section_id: "c18s3".into(),
                    example_number: Some("18.12".into()),
                    example_id: Some("c18e3d1".into()),
                    source_path: Some("vendor/cll/chapters/18.xml".into()),
                }],
                expectations: Expectations::default(),
            },
        );
        let muplis = loaded_case(
            "fixtures/muplis/collection-18/1-front.toml",
            TestCase {
                id: "muplis.18.1.front".into(),
                lojban: "coi".into(),
                translation_en: None,
                gloss_en: None,
                tags: vec!["muplis".into()],
                provenance: vec![Provenance::Muplis {
                    collection_id: "18".into(),
                    item_id: Some("1".into()),
                    form: Some(MuplisForm::Front),
                    url: None,
                }],
                expectations: Expectations::default(),
            },
        );
        let fixtures = vec![cll, muplis];
        let cll_selector = FixtureSelector {
            cll: Some(CllSelector {
                chapter: Some(18),
                example_id: Some("c18e3d1".into()),
                ..CllSelector::default()
            }),
            ..FixtureSelector::default()
        };
        assert_eq!(filter_fixtures(root, &fixtures, &cll_selector).len(), 1);

        let muplis_selector = FixtureSelector {
            muplis: Some(MuplisSelector {
                collection_id: Some("18".into()),
                form: Some(MuplisForm::Front),
                ..MuplisSelector::default()
            }),
            ..FixtureSelector::default()
        };
        assert_eq!(filter_fixtures(root, &fixtures, &muplis_selector).len(), 1);
    }

    #[test]
    fn fake_runner_counts_failures() {
        struct FakeBackend;
        impl FixtureBackend for FakeBackend {
            fn run(&self, _fixture: &LoadedTestCase, facet: Facet) -> FacetResult {
                match facet {
                    Facet::Morphology => FacetResult::passed(),
                    Facet::Syntax => FacetResult::failed("syntax failed"),
                    _ => FacetResult::skipped("not selected"),
                }
            }
        }

        let case = loaded_case(
            "fixtures/adhoc/smoke/coi.toml",
            TestCase {
                id: "adhoc.smoke.coi".into(),
                lojban: "coi".into(),
                translation_en: None,
                gloss_en: None,
                tags: vec!["adhoc".into()],
                provenance: vec![Provenance::Adhoc { description: None }],
                expectations: Expectations::default(),
            },
        );
        let fixtures = vec![&case];
        let summary =
            run_fixture_facets(&FakeBackend, &fixtures, &[Facet::Morphology, Facet::Syntax]);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn import_writes_toml_fixture() {
        let temp_root = std::env::temp_dir().join(format!(
            "jbotci-fixtures-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp root");
        let input = temp_root.join("export.json");
        let output = temp_root.join("fixtures");
        let export = FixtureExport {
            schema_version: 1,
            cases: vec![TestCase {
                id: "adhoc.import".into(),
                lojban: "coi".into(),
                translation_en: None,
                gloss_en: None,
                tags: vec!["adhoc".into()],
                provenance: vec![Provenance::Adhoc {
                    description: Some("test".into()),
                }],
                expectations: Expectations::default(),
            }],
        };
        fs::write(&input, serde_json::to_string(&export).expect("json")).expect("write export");
        let summary = import_export_file(&input, &output).expect("import");
        assert_eq!(summary.written, 1);
        assert_eq!(load_fixture_tree(&output).expect("fixtures").len(), 1);
        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn writer_keeps_parse_tree_as_inline_value() {
        let temp_root = std::env::temp_dir().join(format!(
            "jbotci-fixtures-writer-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&temp_root).expect("temp root");
        let fixture_path = temp_root.join("fixture.toml");
        let test_case = TestCase {
            id: "adhoc.syntax".into(),
            lojban: "coi".into(),
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![],
            expectations: Expectations {
                syntax: Some(SyntaxExpectation {
                    status: ExpectationStatus::Success,
                    parse_tree: Some(SyntaxValue::node(
                        "LojbanText",
                        vec![SyntaxField {
                            name: Some("paragraphs".into()),
                            value: SyntaxValue::List { items: vec![] },
                        }],
                    )),
                    error: None,
                }),
                ..Expectations::default()
            },
        };
        write_fixture_file(&fixture_path, &test_case).expect("write fixture");
        let text = fs::read_to_string(&fixture_path).expect("read fixture");
        assert!(text.contains("[expectations.syntax]\nstatus = \"success\"\nparse-tree = {"));
        assert!(!text.contains("[expectations.syntax.parse-tree"));
        assert_eq!(
            load_fixture_file(&fixture_path).expect("load fixture"),
            test_case
        );
        let _ = fs::remove_dir_all(temp_root);
    }

    fn loaded_case(path: &str, test_case: TestCase) -> LoadedTestCase {
        LoadedTestCase {
            path: PathBuf::from(path),
            test_case,
        }
    }

    trait WordWithModifiersExpectationExt {
        fn base_word_kind(&self) -> Option<WordKind>;
    }

    impl WordWithModifiersExpectationExt for WordWithModifiers {
        fn base_word_kind(&self) -> Option<WordKind> {
            match self {
                WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                    WordLike::Bare { word } => Some(word.kind),
                    _ => None,
                },
                _ => None,
            }
        }
    }
}
