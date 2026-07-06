//! Unified TOML fixture loader, selectors, and runner support.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use bityzba::{invariant, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticSeverity, source_text_for_span};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_orthography::LojbanScript;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct TestCase {
    pub id: String,
    pub lojban: String,
    #[serde(
        default,
        rename = "lojban-filename",
        alias = "lojban_filename",
        skip_serializing_if = "Option::is_none"
    )]
    pub lojban_filename: Option<PathBuf>,
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

impl<'de> Deserialize<'de> for TestCase {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[invariant(true)]
        struct TestCaseWire {
            id: String,
            #[serde(default)]
            lojban: Option<String>,
            #[serde(default, rename = "lojban-filename", alias = "lojban_filename")]
            lojban_filename: Option<PathBuf>,
            #[serde(default)]
            dialect: Option<String>,
            #[serde(default, rename = "translation-en")]
            translation_en: Option<String>,
            #[serde(default, rename = "gloss-en")]
            gloss_en: Option<String>,
            #[serde(default)]
            tags: Vec<String>,
            #[serde(default)]
            provenance: Vec<Provenance>,
            #[serde(default)]
            expectations: Expectations,
        }

        let wire = TestCaseWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            lojban: wire.lojban.unwrap_or_default(),
            lojban_filename: wire.lojban_filename,
            dialect: wire.dialect,
            translation_en: wire.translation_en,
            gloss_en: wire.gloss_en,
            tags: wire.tags,
            provenance: wire.provenance,
            expectations: wire.expectations,
        })
    }
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

    #[requires(true)]
    #[ensures(ret.iter().all(|facet| self.expectation_status(*facet).is_some()))]
    pub fn available_facets(&self) -> BTreeSet<Facet> {
        let mut facets = BTreeSet::new();
        if self.expectations.morphology.is_some() {
            facets.insert(Facet::Morphology);
        }
        if self.expectations.jvozba.is_some() {
            facets.insert(Facet::Jvozba);
        }
        if self.expectations.syntax.is_some() {
            facets.insert(Facet::Syntax);
        }
        if self
            .expectations
            .semantics
            .as_ref()
            .is_some_and(|semantics| semantics.refs.is_some())
        {
            facets.insert(Facet::SemanticsRefs);
        }
        if let Some(output) = &self.expectations.output {
            if output
                .vlasei
                .as_ref()
                .and_then(|vlasei| vlasei.brackets.as_ref())
                .is_some_and(|brackets| brackets.has_script(LojbanScript::Latin))
            {
                facets.insert(Facet::VlaseiBrackets);
            }
            if output
                .vlasei
                .as_ref()
                .and_then(|vlasei| vlasei.brackets.as_ref())
                .is_some_and(|brackets| brackets.has_script(LojbanScript::Cyrillic))
            {
                facets.insert(Facet::VlaseiBracketsCyrillic);
            }
            if output
                .vlasei
                .as_ref()
                .and_then(|vlasei| vlasei.brackets.as_ref())
                .is_some_and(|brackets| brackets.has_script(LojbanScript::Zbalermorna))
            {
                facets.insert(Facet::VlaseiBracketsZbalermorna);
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
            if output
                .gentufa
                .as_ref()
                .and_then(|gentufa| gentufa.show_elided.as_ref())
                .is_some_and(|show_elided| show_elided.brackets.is_some())
            {
                facets.insert(Facet::GentufaBracketsShowElided);
            }
            if output
                .gentufa
                .as_ref()
                .and_then(|gentufa| gentufa.show_elided.as_ref())
                .is_some_and(|show_elided| show_elided.tree.is_some())
            {
                facets.insert(Facet::GentufaTreeShowElided);
            }
            if output
                .gentufa
                .as_ref()
                .and_then(|gentufa| gentufa.show_elided.as_ref())
                .is_some_and(|show_elided| show_elided.json.is_some())
            {
                facets.insert(Facet::GentufaJsonShowElided);
            }
            if output
                .tersmu
                .as_ref()
                .is_some_and(|tersmu| tersmu.json.is_some() || tersmu.error.is_some())
            {
                facets.insert(Facet::TersmuJson);
            }
        }
        facets
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || self.expectations.syntax.as_ref().and_then(|syntax| syntax.xfail.as_ref()).is_some())]
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
            Facet::Jvozba => self.expectations.jvozba.as_ref().map(|value| value.status),
            Facet::Syntax => self.expectations.syntax.as_ref().map(|value| value.status),
            Facet::SemanticsRefs => self
                .expectations
                .semantics
                .as_ref()
                .and_then(|semantics| semantics.refs.as_ref())
                .map(|value| value.status),
            Facet::VlaseiBrackets => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Latin))
                .map(|_| ExpectationStatus::Success),
            Facet::VlaseiBracketsCyrillic => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Cyrillic))
                .map(|_| ExpectationStatus::Success),
            Facet::VlaseiBracketsZbalermorna => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.vlasei.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Zbalermorna))
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
            Facet::GentufaBracketsShowElided => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.show_elided.as_ref())
                .and_then(|output| output.brackets.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::GentufaTreeShowElided => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.show_elided.as_ref())
                .and_then(|output| output.tree.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::GentufaJsonShowElided => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.gentufa.as_ref())
                .and_then(|output| output.show_elided.as_ref())
                .and_then(|output| output.json.as_ref())
                .map(|_| ExpectationStatus::Success),
            Facet::TersmuJson => self
                .expectations
                .output
                .as_ref()
                .and_then(|output| output.tersmu.as_ref())
                .filter(|output| output.json.is_some() || output.error.is_some())
                .map(|output| output.status),
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
    pub jvozba: Option<JvozbaExpectation>,
    #[serde(default)]
    pub syntax: Option<SyntaxExpectation>,
    #[serde(default)]
    pub semantics: Option<SemanticsExpectations>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct OutputExpectations {
    #[serde(default)]
    pub vlasei: Option<VlaseiOutputExpectation>,
    #[serde(default)]
    pub gentufa: Option<GentufaOutputExpectation>,
    #[serde(default)]
    pub tersmu: Option<TersmuOutputExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct VlaseiOutputExpectation {
    #[serde(default)]
    pub brackets: Option<BracketExpectations>,
    #[serde(default)]
    pub tree: Option<TextExpectation>,
    #[serde(default)]
    pub json: Option<TextExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct GentufaOutputExpectation {
    #[serde(default)]
    pub brackets: Option<TextExpectation>,
    #[serde(default)]
    pub tree: Option<TextExpectation>,
    #[serde(default)]
    pub json: Option<TextExpectation>,
    #[serde(default, rename = "show-elided")]
    pub show_elided: Option<CommandOutputExpectation>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct TersmuOutputExpectation {
    #[serde(default = "success_expectation_status")]
    pub status: ExpectationStatus,
    #[serde(default, rename = "story-time")]
    pub story_time: bool,
    #[serde(default)]
    pub json: Option<TextExpectation>,
    #[serde(default)]
    pub error: Option<TextExpectation>,
}

impl Default for TersmuOutputExpectation {
    #[requires(true)]
    #[ensures(ret.status == ExpectationStatus::Success)]
    fn default() -> Self {
        Self {
            status: ExpectationStatus::Success,
            story_time: false,
            json: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[invariant(::Legacy(_) => true)]
#[invariant(::Scripts(_) => true)]
pub enum BracketExpectations {
    Legacy(TextExpectation),
    Scripts(ScriptBracketExpectations),
}

impl BracketExpectations {
    #[requires(true)]
    #[ensures(ret == self.expectation_for_script(script).is_some())]
    pub fn has_script(&self, script: LojbanScript) -> bool {
        self.expectation_for_script(script).is_some()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn expectation_for_script(&self, script: LojbanScript) -> Option<&TextExpectation> {
        match self {
            Self::Legacy(expectation) if script == LojbanScript::Latin => Some(expectation),
            Self::Legacy(_) => None,
            Self::Scripts(expectations) => expectations.expectation_for_script(script),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn expectation_for_script_mut(
        &mut self,
        script: LojbanScript,
    ) -> Option<&mut TextExpectation> {
        match self {
            Self::Legacy(expectation) if script == LojbanScript::Latin => Some(expectation),
            Self::Legacy(_) => None,
            Self::Scripts(expectations) => expectations.expectation_for_script_mut(script),
        }
    }

    #[requires(true)]
    #[ensures(matches!(ret, Self::Legacy(_)))]
    pub fn latin(expectation: TextExpectation) -> Self {
        Self::Legacy(expectation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ScriptBracketExpectations {
    #[serde(default)]
    pub latin: Option<TextExpectation>,
    #[serde(default)]
    pub cyrillic: Option<TextExpectation>,
    #[serde(default)]
    pub zbalermorna: Option<TextExpectation>,
}

impl ScriptBracketExpectations {
    #[requires(true)]
    #[ensures(true)]
    pub fn expectation_for_script(&self, script: LojbanScript) -> Option<&TextExpectation> {
        match script {
            LojbanScript::Latin => self.latin.as_ref(),
            LojbanScript::Cyrillic => self.cyrillic.as_ref(),
            LojbanScript::Zbalermorna => self.zbalermorna.as_ref(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn expectation_for_script_mut(
        &mut self,
        script: LojbanScript,
    ) -> Option<&mut TextExpectation> {
        match script {
            LojbanScript::Latin => self.latin.as_mut(),
            LojbanScript::Cyrillic => self.cyrillic.as_mut(),
            LojbanScript::Zbalermorna => self.zbalermorna.as_mut(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct SemanticsExpectations {
    #[serde(default)]
    pub refs: Option<ReferenceExpectation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct MorphologyExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub raw: Option<TextExpectation>,
    #[serde(default)]
    pub diagnostics: Vec<DiagnosticExpectation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct JvozbaExpectation {
    pub status: ExpectationStatus,
    pub mode: JvozbaFixtureMode,
    pub inputs: Vec<JvozbaFixtureInput>,
    #[serde(default)]
    pub output: Option<JvozbaOutputExpectation>,
    #[serde(default)]
    pub error: Option<TextExpectation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Lujvo => true)]
#[invariant(::Cmevla => true)]
pub enum JvozbaFixtureMode {
    Lujvo,
    Cmevla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[invariant(true)]
#[invariant(::Word { .. } => true)]
#[invariant(::FixedRafsi { .. } => true)]
pub enum JvozbaFixtureInput {
    Word { text: String },
    FixedRafsi { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct JvozbaOutputExpectation {
    pub word: String,
    pub segments: Vec<JvozbaSegmentExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct JvozbaSegmentExpectation {
    pub kind: JvozbaSegmentKindExpectation,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
pub enum JvozbaSegmentKindExpectation {
    Rafsi,
    Hyphen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct SyntaxExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub raw: Option<TextExpectation>,
    #[serde(default)]
    pub diagnostics: Vec<DiagnosticExpectation>,
    #[serde(default)]
    pub xfail: Option<XfailExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ReferenceExpectation {
    pub status: ExpectationStatus,
    #[serde(default)]
    pub raw: Option<TextExpectation>,
    #[serde(default)]
    pub error: Option<TextExpectation>,
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
pub struct DiagnosticExpectation {
    pub severity: DiagnosticSeverity,
    pub code: String,
    #[serde(rename = "byte-span")]
    pub byte_span: [usize; 2],
    #[serde(rename = "source-text")]
    pub source_text: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, rename = "word-index")]
    pub word_index: Option<usize>,
}

impl DiagnosticExpectation {
    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn from_diagnostic(source: &str, diagnostic: &Diagnostic) -> Self {
        let label = diagnostic.primary_label();
        let source_text = source_text_for_span(source, &label.span)
            .expect("diagnostic spans are derived from the fixture source text");
        DiagnosticExpectation {
            severity: diagnostic.severity,
            code: diagnostic.code.clone(),
            byte_span: [label.span.byte_start, label.span.byte_end],
            source_text,
            message: Some(diagnostic.message.clone()),
            word_index: diagnostic.word_index,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextExpectation {
    pub text: String,
    pub sha256: Option<String>,
}

impl Serialize for TextExpectation {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let Some(sha256) = &self.sha256 else {
            return serializer.serialize_str(&self.text);
        };
        let mut map = serializer.serialize_map(Some(if self.text.is_empty() { 1 } else { 2 }))?;
        if !self.text.is_empty() {
            map.serialize_entry("text", &self.text)?;
        }
        map.serialize_entry("sha256", sha256)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for TextExpectation {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[invariant(true)]
        struct TextExpectationTable {
            #[serde(default)]
            text: String,
            #[serde(default)]
            sha256: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        #[invariant(true)]
        #[invariant(::Text(_) => true)]
        #[invariant(::Table(_) => true)]
        enum TextExpectationWire {
            Text(String),
            Table(TextExpectationTable),
        }

        let expectation = match TextExpectationWire::deserialize(deserializer)? {
            TextExpectationWire::Text(text) => Self { text, sha256: None },
            TextExpectationWire::Table(table) => Self {
                text: table.text,
                sha256: table.sha256,
            },
        };
        if let Some(sha256) = &expectation.sha256
            && (sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(serde::de::Error::custom(
                "`sha256` text expectations must be 64 hex digits",
            ));
        }
        Ok(expectation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectationStatus {
    Success,
    Failure,
    Pending,
    NotApplicable,
}

#[requires(true)]
#[ensures(ret == ExpectationStatus::Success)]
fn success_expectation_status() -> ExpectationStatus {
    ExpectationStatus::Success
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Facet {
    Morphology,
    Jvozba,
    Syntax,
    SemanticsRefs,
    VlaseiBrackets,
    VlaseiBracketsCyrillic,
    VlaseiBracketsZbalermorna,
    VlaseiTree,
    VlaseiJson,
    GentufaBrackets,
    GentufaTree,
    GentufaJson,
    GentufaBracketsShowElided,
    GentufaTreeShowElided,
    GentufaJsonShowElided,
    TersmuJson,
}

impl Facet {
    #[requires(true)]
    #[ensures(true)]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Morphology,
            Self::Jvozba,
            Self::Syntax,
            Self::SemanticsRefs,
            Self::VlaseiBrackets,
            Self::VlaseiBracketsCyrillic,
            Self::VlaseiBracketsZbalermorna,
            Self::VlaseiTree,
            Self::VlaseiJson,
            Self::GentufaBrackets,
            Self::GentufaTree,
            Self::GentufaJson,
            Self::GentufaBracketsShowElided,
            Self::GentufaTreeShowElided,
            Self::GentufaJsonShowElided,
            Self::TersmuJson,
        ]
    }
}

impl fmt::Display for Facet {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Morphology => "morphology",
            Self::Jvozba => "jvozba",
            Self::Syntax => "syntax",
            Self::SemanticsRefs => "semantics-refs",
            Self::VlaseiBrackets => "vlasei-brackets",
            Self::VlaseiBracketsCyrillic => "vlasei-brackets-cyrillic",
            Self::VlaseiBracketsZbalermorna => "vlasei-brackets-zbalermorna",
            Self::VlaseiTree => "vlasei-tree",
            Self::VlaseiJson => "vlasei-json",
            Self::GentufaBrackets => "gentufa-brackets",
            Self::GentufaTree => "gentufa-tree",
            Self::GentufaJson => "gentufa-json",
            Self::GentufaBracketsShowElided => "gentufa-brackets-show-elided",
            Self::GentufaTreeShowElided => "gentufa-tree-show-elided",
            Self::GentufaJsonShowElided => "gentufa-json-show-elided",
            Self::TersmuJson => "tersmu-json",
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
            "jvozba" => Ok(Self::Jvozba),
            "syntax" => Ok(Self::Syntax),
            "semantics-refs" => Ok(Self::SemanticsRefs),
            "vlasei-brackets" => Ok(Self::VlaseiBrackets),
            "vlasei-brackets-cyrillic" => Ok(Self::VlaseiBracketsCyrillic),
            "vlasei-brackets-zbalermorna" => Ok(Self::VlaseiBracketsZbalermorna),
            "vlasei-tree" => Ok(Self::VlaseiTree),
            "vlasei-json" => Ok(Self::VlaseiJson),
            "gentufa-brackets" => Ok(Self::GentufaBrackets),
            "gentufa-tree" => Ok(Self::GentufaTree),
            "gentufa-json" => Ok(Self::GentufaJson),
            "gentufa-brackets-show-elided" => Ok(Self::GentufaBracketsShowElided),
            "gentufa-tree-show-elided" => Ok(Self::GentufaTreeShowElided),
            "gentufa-json-show-elided" => Ok(Self::GentufaJsonShowElided),
            "tersmu-json" => Ok(Self::TersmuJson),
            other => Err(format!("unknown fixture facet `{other}`")),
        }
    }
}

#[invariant(selector.provenance.iter().all(|value| !value.is_empty()))]
#[invariant(selector.tags.iter().all(|value| !value.is_empty()))]
#[invariant(selector.ids.iter().all(|value| !value.is_empty()))]
#[invariant(selector.path_prefixes.iter().all(|value| !value.is_empty()))]
#[invariant(selector.paths.iter().all(|value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureProfile {
    #[serde(default)]
    pub facets: Vec<Facet>,
    #[serde(default)]
    pub selector: FixtureSelector,
}

#[invariant(provenance.iter().all(|value| !value.is_empty()))]
#[invariant(tags.iter().all(|value| !value.is_empty()))]
#[invariant(ids.iter().all(|value| !value.is_empty()))]
#[invariant(path_prefixes.iter().all(|value| !value.is_empty()))]
#[invariant(paths.iter().all(|value| !value.is_empty()))]
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
    pub paths: Vec<String>,
    #[serde(default)]
    pub cll: Option<CllSelector>,
    #[serde(default)]
    pub muplis: Option<MuplisSelector>,
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
#[invariant(::InvalidLojbanSource => true)]
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
    #[error("fixture `{path}` has invalid Lojban source declaration: {message}")]
    InvalidLojbanSource { path: PathBuf, message: String },
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
    let mut test_case: TestCase =
        toml::from_str(&text).map_err(|source| FixtureError::ParseToml {
            path: path.to_path_buf(),
            source,
        })?;
    resolve_fixture_lojban_source(path, &text, &mut test_case)?;
    Ok(test_case)
}

#[requires(true)]
#[ensures(ret.is_ok() -> !test_case.lojban.is_empty() || test_case.lojban_filename.is_none())]
fn resolve_fixture_lojban_source(
    fixture_path: &Path,
    fixture_text: &str,
    test_case: &mut TestCase,
) -> Result<(), FixtureError> {
    let shape = fixture_lojban_source_shape(fixture_path, fixture_text)?;
    match (shape.inline, shape.filename) {
        (true, false) => {
            test_case.lojban_filename = None;
            Ok(())
        }
        (false, true) => {
            let Some(relative) = test_case.lojban_filename.as_ref() else {
                return Err(FixtureError::InvalidLojbanSource {
                    path: fixture_path.to_path_buf(),
                    message: "`lojban-filename` must be a string path".to_owned(),
                });
            };
            if !is_safe_fixture_source_path(relative) {
                return Err(FixtureError::InvalidLojbanSource {
                    path: fixture_path.to_path_buf(),
                    message: format!(
                        "`lojban-filename` must be a relative child path, got `{}`",
                        relative.display()
                    ),
                });
            }
            let parent = fixture_path.parent().unwrap_or_else(|| Path::new("."));
            test_case.lojban = read_text(&parent.join(relative))?;
            Ok(())
        }
        (true, true) => Err(FixtureError::InvalidLojbanSource {
            path: fixture_path.to_path_buf(),
            message: "`lojban` and `lojban-filename` are mutually exclusive".to_owned(),
        }),
        (false, false) => Err(FixtureError::InvalidLojbanSource {
            path: fixture_path.to_path_buf(),
            message: "fixture must declare either `lojban` or `lojban-filename`".to_owned(),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct FixtureLojbanSourceShape {
    inline: bool,
    filename: bool,
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn fixture_lojban_source_shape(
    fixture_path: &Path,
    fixture_text: &str,
) -> Result<FixtureLojbanSourceShape, FixtureError> {
    let value =
        toml::from_str::<toml::Value>(fixture_text).map_err(|source| FixtureError::ParseToml {
            path: fixture_path.to_path_buf(),
            source,
        })?;
    let Some(table) = value.as_table() else {
        return Ok(FixtureLojbanSourceShape {
            inline: false,
            filename: false,
        });
    };
    Ok(FixtureLojbanSourceShape {
        inline: table.contains_key("lojban"),
        filename: table.contains_key("lojban-filename") || table.contains_key("lojban_filename"),
    })
}

#[requires(true)]
#[ensures(ret -> !path.is_absolute())]
fn is_safe_fixture_source_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn reject_legacy_expectation_format(path: &Path, text: &str) -> Result<(), FixtureError> {
    let Ok(value) = toml::from_str::<toml::Value>(text) else {
        return Ok(());
    };
    if let Some(message) = legacy_expectation_marker(&value) {
        return Err(FixtureError::LegacyExpectationFormat {
            path: path.to_path_buf(),
            message,
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn legacy_expectation_marker(value: &toml::Value) -> Option<String> {
    legacy_expectation_marker_in_value(value)
}

#[requires(true)]
#[ensures(true)]
fn legacy_expectation_marker_in_value(value: &toml::Value) -> Option<String> {
    let toml::Value::Table(table) = value else {
        if let toml::Value::Array(items) = value {
            for item in items {
                if let Some(message) = legacy_expectation_marker_in_value(item) {
                    return Some(message);
                }
            }
        }
        return None;
    };

    for (key, item) in table {
        if key == "parse-tree" {
            return Some("found legacy `parse-tree` key".to_owned());
        }
        if matches!(
            key.as_str(),
            "BaseWord" | "StandaloneIndicator" | "NotEof" | "LojbanText" | "constructor" | "words"
        ) {
            return Some(format!("found legacy `{key}` key"));
        }
        if key == "kind" && item.as_str().is_some_and(is_legacy_expectation_kind_value) {
            return Some(format!(
                "found legacy `kind = \"{}\"` value",
                item.as_str().unwrap_or_default()
            ));
        }
        if let Some(message) = legacy_expectation_marker_in_value(item) {
            return Some(message);
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn is_legacy_expectation_kind_value(value: &str) -> bool {
    matches!(
        value,
        "node"
            | "base-word"
            | "standalone-indicator"
            | "emphasized"
            | "with-indicator"
            | "not-eof"
            | "bare"
            | "zo-quote"
            | "zoi-quote"
            | "lohu-quote"
            | "single-word-quote"
            | "letter"
            | "zei-lujvo"
    )
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

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|paths| paths.iter().all(|path| path.extension().is_some_and(|ext| ext == "toml"))))]
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

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|summary| summary.fixture_count > 0))]
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
#[ensures(true)]
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

#[requires(true)]
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

#[requires(true)]
#[ensures(true)]
pub fn fixture_matches_selector(
    root: &Path,
    fixture: &LoadedTestCase,
    selector: &FixtureSelector,
) -> bool {
    matches_selector(root, fixture, selector)
}

#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|summary| summary.written > 0))]
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
    if !selector.paths.is_empty() {
        let relative = fixture.path.strip_prefix(root).unwrap_or(&fixture.path);
        let relative_text = relative.to_string_lossy();
        if !selector.paths.iter().any(|path| path == &relative_text) {
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
    let text = fs::read_to_string(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(normalize_fixture_storage_newlines(text))
}

#[requires(true)]
#[ensures(!ret.contains('\r'))]
fn normalize_fixture_storage_newlines(text: String) -> String {
    if !text.contains('\r') {
        return text;
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}
