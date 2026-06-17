//! CLL gismu composition.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_dictionary::{Dictionary, WordType};
use jbotci_morphology::{
    GlideMark, PhonemeRenderOptions, StressMark, WordKind, is_consonant, is_vowel,
    permissible_consonant_pair, segment_words_with_modifiers,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GIMFIHE_DEFAULT_COUNT: usize = 20;
pub const GIMFIHE_MAX_COUNT: usize = 512;

const CONSONANTS: &[char] = &[
    'b', 'c', 'd', 'f', 'g', 'j', 'k', 'l', 'm', 'n', 'p', 'r', 's', 't', 'v', 'x', 'z',
];
const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u'];

const PRESET_1985: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.36),
    PresetEntry::new("hin", 0.16),
    PresetEntry::new("eng", 0.21),
    PresetEntry::new("spa", 0.11),
    PresetEntry::new("rus", 0.09),
    PresetEntry::new("ara", 0.07),
];
const PRESET_1987: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.36),
    PresetEntry::new("hin", 0.156),
    PresetEntry::new("eng", 0.208),
    PresetEntry::new("spa", 0.116),
    PresetEntry::new("rus", 0.087),
    PresetEntry::new("ara", 0.073),
];
const PRESET_1994: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.348),
    PresetEntry::new("hin", 0.194),
    PresetEntry::new("eng", 0.163),
    PresetEntry::new("spa", 0.123),
    PresetEntry::new("rus", 0.088),
    PresetEntry::new("ara", 0.084),
];
const PRESET_1995: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.347),
    PresetEntry::new("hin", 0.196),
    PresetEntry::new("eng", 0.160),
    PresetEntry::new("spa", 0.123),
    PresetEntry::new("rus", 0.089),
    PresetEntry::new("ara", 0.085),
];
const PRESET_1999: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.334),
    PresetEntry::new("hin", 0.195),
    PresetEntry::new("eng", 0.187),
    PresetEntry::new("spa", 0.116),
    PresetEntry::new("rus", 0.081),
    PresetEntry::new("ara", 0.088),
];
const PRESET_EVENLY: &[PresetEntry] = &[
    PresetEntry::new("cmn", 1.0),
    PresetEntry::new("hin", 1.0),
    PresetEntry::new("eng", 1.0),
    PresetEntry::new("spa", 1.0),
    PresetEntry::new("rus", 1.0),
    PresetEntry::new("ara", 1.0),
];
const PRESET_ILMEN6: &[PresetEntry] = &[
    PresetEntry::new("eng", 0.339),
    PresetEntry::new("cmn", 0.255),
    PresetEntry::new("hin", 0.136),
    PresetEntry::new("spa", 0.126),
    PresetEntry::new("ara", 0.074),
    PresetEntry::new("fra", 0.070),
];
const PRESET_ILMEN8: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.271),
    PresetEntry::new("eng", 0.170),
    PresetEntry::new("spa", 0.130),
    PresetEntry::new("hin", 0.125),
    PresetEntry::new("ara", 0.104),
    PresetEntry::new("ben", 0.076),
    PresetEntry::new("rus", 0.064),
    PresetEntry::new("por", 0.060),
];
const PRESET_ILMEN12: &[PresetEntry] = &[
    PresetEntry::new("cmn", 0.238),
    PresetEntry::new("eng", 0.160),
    PresetEntry::new("spa", 0.113),
    PresetEntry::new("hin", 0.108),
    PresetEntry::new("ara", 0.090),
    PresetEntry::new("ben", 0.066),
    PresetEntry::new("rus", 0.055),
    PresetEntry::new("por", 0.052),
    PresetEntry::new("msa", 0.032),
    PresetEntry::new("jpn", 0.030),
    PresetEntry::new("deu", 0.029),
    PresetEntry::new("fra", 0.026),
];

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GimfihePreset {
    #[serde(rename = "1985")]
    Data1985,
    #[serde(rename = "1987")]
    Data1987,
    #[serde(rename = "1994")]
    Data1994,
    #[serde(rename = "1995")]
    Data1995,
    #[serde(rename = "1999")]
    Data1999,
    Evenly,
    Ilmen6,
    Ilmen8,
    Ilmen12,
}

impl GimfihePreset {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Data1985 => "1985",
            Self::Data1987 => "1987",
            Self::Data1994 => "1994",
            Self::Data1995 => "1995",
            Self::Data1999 => "1999",
            Self::Evenly => "evenly",
            Self::Ilmen6 => "ilmen6",
            Self::Ilmen8 => "ilmen8",
            Self::Ilmen12 => "ilmen12",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn entries(self) -> &'static [PresetEntry] {
        match self {
            Self::Data1985 => PRESET_1985,
            Self::Data1987 => PRESET_1987,
            Self::Data1994 => PRESET_1994,
            Self::Data1995 => PRESET_1995,
            Self::Data1999 => PRESET_1999,
            Self::Evenly => PRESET_EVENLY,
            Self::Ilmen6 => PRESET_ILMEN6,
            Self::Ilmen8 => PRESET_ILMEN8,
            Self::Ilmen12 => PRESET_ILMEN12,
        }
    }
}

impl fmt::Display for GimfihePreset {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GimfihePreset {
    type Err = GimfiheError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|preset| !preset.as_str().is_empty()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_preset(value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetEntry {
    pub language: &'static str,
    pub weight: f64,
}

impl PresetEntry {
    #[requires(!language.is_empty())]
    #[requires(weight > 0.0 && weight.is_finite())]
    #[ensures(true)]
    pub const fn new(language: &'static str, weight: f64) -> Self {
        Self { language, weight }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GismuShape {
    Ccvcv,
    Cvccv,
}

impl GismuShape {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ccvcv => "ccvcv",
            Self::Cvccv => "cvccv",
        }
    }
}

impl fmt::Display for GismuShape {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GismuShape {
    type Err = GimfiheError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|shape| !shape.as_str().is_empty()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_shape(value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CollisionScope {
    All,
    Official,
    None,
}

impl CollisionScope {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Official => "official",
            Self::None => "none",
        }
    }
}

impl Default for CollisionScope {
    #[requires(true)]
    #[ensures(ret == CollisionScope::All)]
    fn default() -> Self {
        Self::All
    }
}

impl fmt::Display for CollisionScope {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CollisionScope {
    type Err = GimfiheError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| !scope.as_str().is_empty()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_collision_scope(value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfiheSourceInput {
    pub language: String,
    pub explicit_weight: Option<f64>,
    pub word: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolvedSource {
    pub language: String,
    pub weight: f64,
    pub word: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfiheRequest {
    pub preset: Option<GimfihePreset>,
    pub sources: Vec<GimfiheSourceInput>,
    pub shapes: Vec<GismuShape>,
    pub all_letters: bool,
    pub check_collisions: CollisionScope,
    pub require_free_short_rafsi: bool,
    pub count: usize,
    pub highlight: Option<String>,
}

impl Default for GimfiheRequest {
    #[requires(true)]
    #[ensures(ret.count == GIMFIHE_DEFAULT_COUNT)]
    fn default() -> Self {
        Self {
            preset: None,
            sources: Vec::new(),
            shapes: default_shapes(),
            all_letters: false,
            check_collisions: CollisionScope::All,
            require_free_short_rafsi: false,
            count: GIMFIHE_DEFAULT_COUNT,
            highlight: None,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RafsiKind {
    Cvc,
    Ccv,
    Cvv,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RafsiAvailability {
    Free,
    OfficialTaken,
    ExperimentalTaken,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RafsiCandidate {
    pub form: String,
    pub kind: RafsiKind,
    pub availability: RafsiAvailability,
    pub taken_by: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CollisionKind {
    Identical,
    FinalVowel,
    SimilarConsonant,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GismuCollision {
    pub kind: CollisionKind,
    pub existing_word: String,
    pub existing_word_type: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceScore {
    pub language: String,
    pub word: String,
    pub weight: f64,
    pub raw_score: usize,
    pub weighted_score: f64,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfiheCandidate {
    pub word: String,
    pub score: f64,
    pub source_scores: Vec<SourceScore>,
    pub collision: Option<GismuCollision>,
    pub rafsi: Vec<RafsiCandidate>,
    pub highlighted: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfiheOutput {
    pub resolved_sources: Vec<ResolvedSource>,
    pub candidate_count: usize,
    pub filtered_count: usize,
    pub winner: Option<String>,
    pub highlighted_word: Option<String>,
    pub candidates: Vec<GimfiheCandidate>,
}

#[invariant(true)]
#[invariant(::UnknownPreset { .. } => true)]
#[invariant(::UnknownShape { .. } => true)]
#[invariant(::UnknownCollisionScope { .. } => true)]
#[invariant(::InvalidSourceSpec { .. } => true)]
#[invariant(::InvalidWeight { .. } => true)]
#[invariant(::MissingPresetLanguage { .. } => true)]
#[invariant(::ExtraPresetLanguage { .. } => true)]
#[invariant(::DuplicateSourceLanguage { .. } => true)]
#[invariant(::MissingExplicitWeight { .. } => true)]
#[invariant(::InvalidSourceWord { .. } => true)]
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GimfiheError {
    #[error("unknown gimfi'e preset `{value}`")]
    UnknownPreset { value: String },
    #[error("unknown gismu shape `{value}`")]
    UnknownShape { value: String },
    #[error("unknown collision checking mode `{value}`")]
    UnknownCollisionScope { value: String },
    #[error("invalid --source `{spec}`: {message}")]
    InvalidSourceSpec { spec: String, message: String },
    #[error("source weight must be a positive finite number, got `{value}`")]
    InvalidWeight { value: String },
    #[error("preset source language `{language}` is missing")]
    MissingPresetLanguage { language: String },
    #[error("source language `{language}` is not in the selected preset")]
    ExtraPresetLanguage { language: String },
    #[error("source language `{language}` appears more than once")]
    DuplicateSourceLanguage { language: String },
    #[error("source language `{language}` needs an explicit weight without --preset")]
    MissingExplicitWeight { language: String },
    #[error(
        "source word `{word}` for `{language}` must be non-empty Lojbanized text using only gismu-scoring letters and apostrophe"
    )]
    InvalidSourceWord { language: String, word: String },
    #[error("at least one source is required")]
    NoSources,
    #[error("at least one shape is required")]
    NoShapes,
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn default_shapes() -> Vec<GismuShape> {
    vec![GismuShape::Ccvcv, GismuShape::Cvccv]
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn all_presets() -> &'static [GimfihePreset] {
    &[
        GimfihePreset::Data1985,
        GimfihePreset::Data1987,
        GimfihePreset::Data1994,
        GimfihePreset::Data1995,
        GimfihePreset::Data1999,
        GimfihePreset::Evenly,
        GimfihePreset::Ilmen6,
        GimfihePreset::Ilmen8,
        GimfihePreset::Ilmen12,
    ]
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn preset_language_suggestions() -> Vec<String> {
    let mut languages = BTreeSet::new();
    for preset in all_presets() {
        for entry in preset.entries() {
            languages.insert(entry.language.to_owned());
        }
    }
    languages.into_iter().collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|preset| !preset.as_str().is_empty()) || ret.is_err())]
pub fn parse_preset(value: &str) -> Result<GimfihePreset, GimfiheError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1985" => Ok(GimfihePreset::Data1985),
        "1987" => Ok(GimfihePreset::Data1987),
        "1994" => Ok(GimfihePreset::Data1994),
        "1995" => Ok(GimfihePreset::Data1995),
        "1999" => Ok(GimfihePreset::Data1999),
        "evenly" => Ok(GimfihePreset::Evenly),
        "ilmen6" => Ok(GimfihePreset::Ilmen6),
        "ilmen8" => Ok(GimfihePreset::Ilmen8),
        "ilmen12" => Ok(GimfihePreset::Ilmen12),
        _ => Err(GimfiheError::UnknownPreset {
            value: value.to_owned(),
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|shape| !shape.as_str().is_empty()) || ret.is_err())]
pub fn parse_shape(value: &str) -> Result<GismuShape, GimfiheError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "ccvcv" => Ok(GismuShape::Ccvcv),
        "cvccv" => Ok(GismuShape::Cvccv),
        _ => Err(GimfiheError::UnknownShape {
            value: value.to_owned(),
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|scope| !scope.as_str().is_empty()) || ret.is_err())]
pub fn parse_collision_scope(value: &str) -> Result<CollisionScope, GimfiheError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "all" => Ok(CollisionScope::All),
        "official" => Ok(CollisionScope::Official),
        "none" => Ok(CollisionScope::None),
        _ => Err(GimfiheError::UnknownCollisionScope {
            value: value.to_owned(),
        }),
    }
}

#[requires(!spec.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|source| !source.language.is_empty()) || ret.is_err())]
pub fn parse_source_spec(spec: &str) -> Result<GimfiheSourceInput, GimfiheError> {
    let parts = spec.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        [language, word] => parsed_source_spec(spec, language, None, word),
        [language, weight, word] => {
            let explicit_weight = if weight.is_empty() {
                None
            } else {
                Some(parse_weight(weight)?)
            };
            parsed_source_spec(spec, language, explicit_weight, word)
        }
        _ => Err(GimfiheError::InvalidSourceSpec {
            spec: spec.to_owned(),
            message: "expected LANG:WORD or LANG:WEIGHT:WORD".to_owned(),
        }),
    }
}

#[requires(!spec.is_empty())]
#[requires(!language.is_empty() || !word.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|source| !source.language.is_empty()) || ret.is_err())]
fn parsed_source_spec(
    spec: &str,
    language: &str,
    explicit_weight: Option<f64>,
    word: &str,
) -> Result<GimfiheSourceInput, GimfiheError> {
    let language = normalize_language(language);
    let word = normalize_source_word(word);
    if language.is_empty() {
        return Err(GimfiheError::InvalidSourceSpec {
            spec: spec.to_owned(),
            message: "language must be non-empty".to_owned(),
        });
    }
    Ok(GimfiheSourceInput {
        language,
        explicit_weight,
        word,
    })
}

#[requires(!value.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|weight| weight.is_finite() && *weight > 0.0) || ret.is_err())]
pub fn parse_weight(value: &str) -> Result<f64, GimfiheError> {
    let Ok(weight) = value.parse::<f64>() else {
        return Err(GimfiheError::InvalidWeight {
            value: value.to_owned(),
        });
    };
    if weight.is_finite() && weight > 0.0 {
        Ok(weight)
    } else {
        Err(GimfiheError::InvalidWeight {
            value: value.to_owned(),
        })
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn compose_gismu(
    dictionary: &Dictionary<'_>,
    request: &GimfiheRequest,
) -> Result<GimfiheOutput, GimfiheError> {
    let resolved_sources = resolve_sources(request.preset, &request.sources)?;
    if request.shapes.is_empty() {
        return Err(GimfiheError::NoShapes);
    }
    let shapes = unique_shapes(&request.shapes);
    let mut candidates = generate_candidates(&resolved_sources, &shapes, request.all_letters)
        .into_iter()
        .filter(|word| valid_gismu_candidate(word))
        .map(|word| {
            score_candidate(
                dictionary,
                request.check_collisions,
                &resolved_sources,
                word,
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);
    let candidate_count = candidates.len();
    let mut filtered = candidates
        .into_iter()
        .filter(|candidate| candidate_passes_filters(candidate, request))
        .collect::<Vec<_>>();
    filtered.sort_by(compare_candidates);
    let filtered_count = filtered.len();
    let winner = filtered.first().map(|candidate| candidate.word.clone());
    let requested_highlight = request
        .highlight
        .as_ref()
        .map(|value| normalize_gismu(value))
        .filter(|value| !value.is_empty());
    let highlighted_word = requested_highlight
        .as_ref()
        .filter(|word| filtered.iter().any(|candidate| &candidate.word == *word))
        .cloned()
        .or_else(|| winner.clone());
    let mut displayed = filtered
        .iter()
        .take(request.count.min(GIMFIHE_MAX_COUNT))
        .cloned()
        .collect::<Vec<_>>();
    if let Some(highlighted_word) = &highlighted_word
        && !displayed
            .iter()
            .any(|candidate| &candidate.word == highlighted_word)
        && let Some(candidate) = filtered
            .iter()
            .find(|candidate| &candidate.word == highlighted_word)
    {
        displayed.push(candidate.clone());
    }
    for candidate in &mut displayed {
        candidate.highlighted = highlighted_word
            .as_ref()
            .is_some_and(|highlighted| highlighted == &candidate.word);
    }
    Ok(GimfiheOutput {
        resolved_sources,
        candidate_count,
        filtered_count,
        winner,
        highlighted_word,
        candidates: displayed,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sources| !sources.is_empty()) || ret.is_err())]
pub fn resolve_sources(
    preset: Option<GimfihePreset>,
    sources: &[GimfiheSourceInput],
) -> Result<Vec<ResolvedSource>, GimfiheError> {
    if sources.is_empty() {
        return Err(GimfiheError::NoSources);
    }
    validate_unique_languages(sources)?;
    for source in sources {
        validate_source_word(source)?;
    }
    match preset {
        Some(preset) => resolve_preset_sources(preset, sources),
        None => resolve_custom_sources(sources),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_unique_languages(sources: &[GimfiheSourceInput]) -> Result<(), GimfiheError> {
    let mut seen = BTreeSet::new();
    for source in sources {
        let language = normalize_language(&source.language);
        if !seen.insert(language.clone()) {
            return Err(GimfiheError::DuplicateSourceLanguage { language });
        }
    }
    Ok(())
}

#[requires(!source.language.is_empty() || !source.word.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_source_word(source: &GimfiheSourceInput) -> Result<(), GimfiheError> {
    let word = normalize_source_word(&source.word);
    if word.is_empty() || !word.chars().all(is_valid_source_word_char) {
        return Err(GimfiheError::InvalidSourceWord {
            language: normalize_language(&source.language),
            word,
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sources| !sources.is_empty()) || ret.is_err())]
fn resolve_preset_sources(
    preset: GimfihePreset,
    sources: &[GimfiheSourceInput],
) -> Result<Vec<ResolvedSource>, GimfiheError> {
    let source_by_language = sources
        .iter()
        .map(|source| (normalize_language(&source.language), source))
        .collect::<BTreeMap<_, _>>();
    for language in source_by_language.keys() {
        if !preset
            .entries()
            .iter()
            .any(|entry| entry.language == language)
        {
            return Err(GimfiheError::ExtraPresetLanguage {
                language: language.clone(),
            });
        }
    }
    let mut resolved = Vec::new();
    for entry in preset.entries() {
        let Some(source) = source_by_language.get(entry.language) else {
            return Err(GimfiheError::MissingPresetLanguage {
                language: entry.language.to_owned(),
            });
        };
        resolved.push(ResolvedSource {
            language: entry.language.to_owned(),
            weight: source.explicit_weight.unwrap_or(entry.weight),
            word: normalize_source_word(&source.word),
        });
    }
    Ok(resolved)
}

#[requires(!sources.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|resolved| resolved.len() == sources.len()) || ret.is_err())]
fn resolve_custom_sources(
    sources: &[GimfiheSourceInput],
) -> Result<Vec<ResolvedSource>, GimfiheError> {
    let mut resolved = Vec::new();
    for source in sources {
        let language = normalize_language(&source.language);
        let Some(weight) = source.explicit_weight else {
            return Err(GimfiheError::MissingExplicitWeight { language });
        };
        resolved.push(ResolvedSource {
            language,
            weight,
            word: normalize_source_word(&source.word),
        });
    }
    Ok(resolved)
}

#[requires(true)]
#[ensures(true)]
fn is_valid_source_word_char(value: char) -> bool {
    is_consonant(value) || is_vowel(value) || value == '\''
}

#[requires(true)]
#[ensures(true)]
fn unique_shapes(shapes: &[GismuShape]) -> Vec<GismuShape> {
    let mut output = Vec::new();
    for shape in shapes {
        if !output.contains(shape) {
            output.push(*shape);
        }
    }
    output
}

#[requires(!sources.is_empty())]
#[requires(!shapes.is_empty())]
#[ensures(true)]
fn generate_candidates(
    sources: &[ResolvedSource],
    shapes: &[GismuShape],
    all_letters: bool,
) -> Vec<String> {
    let (consonants, vowels) = candidate_inventory(sources, all_letters);
    if consonants.is_empty() || vowels.is_empty() {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for shape in shapes {
        append_shape_candidates(*shape, &consonants, &vowels, &mut candidates);
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

#[requires(!sources.is_empty())]
#[ensures(ret.0.iter().all(|value| is_consonant(*value)))]
#[ensures(ret.1.iter().all(|value| is_vowel(*value)))]
fn candidate_inventory(sources: &[ResolvedSource], all_letters: bool) -> (Vec<char>, Vec<char>) {
    if all_letters {
        return (CONSONANTS.to_vec(), VOWELS.to_vec());
    }
    let mut consonants = BTreeSet::new();
    let mut vowels = BTreeSet::new();
    for source in sources {
        for value in source.word.chars() {
            if is_consonant(value) {
                consonants.insert(value);
            } else if is_vowel(value) {
                vowels.insert(value);
            }
        }
    }
    (
        consonants.into_iter().collect(),
        vowels.into_iter().collect(),
    )
}

#[requires(!consonants.is_empty())]
#[requires(!vowels.is_empty())]
#[ensures(true)]
fn append_shape_candidates(
    shape: GismuShape,
    consonants: &[char],
    vowels: &[char],
    candidates: &mut Vec<String>,
) {
    match shape {
        GismuShape::Ccvcv => {
            for c0 in consonants {
                for c1 in consonants {
                    if permissible_consonant_pair(*c0, *c1) != Some(2) {
                        continue;
                    }
                    for v2 in vowels {
                        for c3 in consonants {
                            for v4 in vowels {
                                candidates.push(format!("{c0}{c1}{v2}{c3}{v4}"));
                            }
                        }
                    }
                }
            }
        }
        GismuShape::Cvccv => {
            for c0 in consonants {
                for v1 in vowels {
                    for c2 in consonants {
                        for c3 in consonants {
                            if permissible_consonant_pair(*c2, *c3).is_none_or(|rank| rank == 0) {
                                continue;
                            }
                            for v4 in vowels {
                                candidates.push(format!("{c0}{v1}{c2}{c3}{v4}"));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(!word.is_empty())]
#[ensures(ret -> word.len() == 5)]
fn valid_gismu_candidate(word: &str) -> bool {
    if word.len() != 5 {
        return false;
    }
    let Ok(words) = segment_words_with_modifiers(word) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    let Some(parsed) = word_like.bare_word() else {
        return false;
    };
    parsed.kind() == WordKind::Gismu && rendered_phonemes_without_marks(parsed) == word
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn rendered_phonemes_without_marks(word: &jbotci_morphology::Word) -> String {
    word.phonemes().render(PhonemeRenderOptions {
        mark_stress: StressMark::None,
        mark_glides: GlideMark::None,
    })
}

#[requires(!word.is_empty())]
#[requires(!sources.is_empty())]
#[ensures(!ret.word.is_empty())]
fn score_candidate(
    dictionary: &Dictionary<'_>,
    scope: CollisionScope,
    sources: &[ResolvedSource],
    word: String,
) -> GimfiheCandidate {
    let source_scores = sources
        .iter()
        .map(|source| score_source(word.as_str(), source))
        .collect::<Vec<_>>();
    let score = source_scores
        .iter()
        .map(|source_score| source_score.weighted_score)
        .sum::<f64>();
    let collision = find_collision(dictionary, scope, &word);
    let rafsi = possible_short_rafsis(dictionary, &word);
    GimfiheCandidate {
        word,
        score,
        source_scores,
        collision,
        rafsi,
        highlighted: false,
    }
}

#[requires(!candidate.is_empty())]
#[requires(!source.word.is_empty())]
#[ensures(ret.weight == source.weight)]
fn score_source(candidate: &str, source: &ResolvedSource) -> SourceScore {
    let raw_score = gismu_match_score(candidate, &source.word);
    let weighted_score = raw_score as f64 / source.word.chars().count() as f64 * source.weight;
    SourceScore {
        language: source.language.clone(),
        word: source.word.clone(),
        weight: source.weight,
        raw_score,
        weighted_score,
    }
}

#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures(ret <= candidate.chars().count())]
pub fn gismu_match_score(candidate: &str, source: &str) -> usize {
    let lcs = longest_common_subsequence_len(candidate, source);
    if lcs >= 3 {
        return lcs;
    }
    if lcs != 2 {
        return 0;
    }
    if has_valid_two_letter_match(candidate, source) {
        2
    } else {
        0
    }
}

#[requires(true)]
#[ensures(ret <= left.chars().count().min(right.chars().count()))]
fn longest_common_subsequence_len(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = vec![0usize; right_chars.len() + 1];
    let mut current = vec![0usize; right_chars.len() + 1];
    for left_ch in left_chars {
        for (right_index, right_ch) in right_chars.iter().enumerate() {
            current[right_index + 1] = if left_ch == *right_ch {
                previous[right_index] + 1
            } else {
                previous[right_index + 1].max(current[right_index])
            };
        }
        std::mem::swap(&mut previous, &mut current);
        current.fill(0);
    }
    previous[right_chars.len()]
}

#[requires(true)]
#[ensures(true)]
fn has_valid_two_letter_match(candidate: &str, source: &str) -> bool {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let source_chars = source.chars().collect::<Vec<_>>();
    for left_candidate in 0..candidate_chars.len() {
        for right_candidate in left_candidate + 1..candidate_chars.len() {
            let candidate_gap = right_candidate - left_candidate;
            if !matches!(candidate_gap, 1 | 2) {
                continue;
            }
            for left_source in 0..source_chars.len() {
                for right_source in left_source + 1..source_chars.len() {
                    let source_gap = right_source - left_source;
                    if source_gap == candidate_gap
                        && candidate_chars[left_candidate] == source_chars[left_source]
                        && candidate_chars[right_candidate] == source_chars[right_source]
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn find_collision(
    dictionary: &Dictionary<'_>,
    scope: CollisionScope,
    candidate: &str,
) -> Option<GismuCollision> {
    if scope == CollisionScope::None {
        return None;
    }
    dictionary
        .entries()
        .iter()
        .filter(|entry| collision_scope_includes(scope, entry.word_type))
        .filter_map(|entry| collision_with_entry(candidate, entry.word, entry.word_type))
        .min_by(|left, right| {
            collision_kind_order(left.kind)
                .cmp(&collision_kind_order(right.kind))
                .then_with(|| left.existing_word.cmp(&right.existing_word))
        })
}

#[requires(true)]
#[ensures(true)]
fn collision_scope_includes(scope: CollisionScope, word_type: WordType) -> bool {
    match scope {
        CollisionScope::All => matches!(word_type, WordType::Gismu | WordType::ExperimentalGismu),
        CollisionScope::Official => word_type == WordType::Gismu,
        CollisionScope::None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn collision_with_entry(
    candidate: &str,
    existing: &str,
    word_type: WordType,
) -> Option<GismuCollision> {
    if candidate == existing {
        return Some(collision(CollisionKind::Identical, existing, word_type));
    }
    if same_except_final_vowel(candidate, existing) {
        return Some(collision(CollisionKind::FinalVowel, existing, word_type));
    }
    if differs_by_too_similar_consonant(candidate, existing) {
        return Some(collision(
            CollisionKind::SimilarConsonant,
            existing,
            word_type,
        ));
    }
    None
}

#[requires(!existing.is_empty())]
#[ensures(!ret.existing_word.is_empty())]
fn collision(kind: CollisionKind, existing: &str, word_type: WordType) -> GismuCollision {
    GismuCollision {
        kind,
        existing_word: existing.to_owned(),
        existing_word_type: word_type.as_str().to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
fn collision_kind_order(kind: CollisionKind) -> usize {
    match kind {
        CollisionKind::Identical => 0,
        CollisionKind::FinalVowel => 1,
        CollisionKind::SimilarConsonant => 2,
    }
}

#[requires(true)]
#[ensures(true)]
fn same_except_final_vowel(candidate: &str, existing: &str) -> bool {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let existing_chars = existing.chars().collect::<Vec<_>>();
    candidate_chars.len() == 5
        && existing_chars.len() == 5
        && candidate_chars[..4] == existing_chars[..4]
        && candidate_chars[4] != existing_chars[4]
        && is_vowel(candidate_chars[4])
        && is_vowel(existing_chars[4])
}

#[requires(true)]
#[ensures(true)]
fn differs_by_too_similar_consonant(candidate: &str, existing: &str) -> bool {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let existing_chars = existing.chars().collect::<Vec<_>>();
    if candidate_chars.len() != existing_chars.len() {
        return false;
    }
    let mut differing = candidate_chars
        .iter()
        .zip(existing_chars.iter())
        .filter(|(candidate, existing)| candidate != existing);
    let Some((candidate, existing)) = differing.next() else {
        return false;
    };
    differing.next().is_none()
        && is_consonant(*candidate)
        && is_consonant(*existing)
        && too_similar_consonant(*candidate, *existing)
}

#[requires(is_consonant(proposed))]
#[requires(is_consonant(existing))]
#[ensures(true)]
fn too_similar_consonant(proposed: char, existing: char) -> bool {
    match proposed {
        'b' => matches!(existing, 'p' | 'v'),
        'c' => matches!(existing, 'j' | 's'),
        'd' => existing == 't',
        'f' => matches!(existing, 'p' | 'v'),
        'g' => matches!(existing, 'k' | 'x'),
        'j' => matches!(existing, 'c' | 'z'),
        'k' => matches!(existing, 'g' | 'x'),
        'l' => existing == 'r',
        'm' => existing == 'n',
        'n' => existing == 'm',
        'p' => matches!(existing, 'b' | 'f'),
        'r' => existing == 'l',
        's' => matches!(existing, 'c' | 'z'),
        't' => existing == 'd',
        'v' => matches!(existing, 'b' | 'f'),
        'x' => matches!(existing, 'g' | 'k'),
        'z' => matches!(existing, 'j' | 's'),
        _ => false,
    }
}

#[requires(word.chars().count() == 5)]
#[ensures(true)]
pub fn possible_short_rafsis(dictionary: &Dictionary<'_>, word: &str) -> Vec<RafsiCandidate> {
    let chars = word.chars().collect::<Vec<_>>();
    let shape = gismu_shape_for_word(&chars);
    let mut raw = Vec::new();
    match shape {
        Some(GismuShape::Cvccv) => {
            push_rafsi(&mut raw, RafsiKind::Cvc, &[chars[0], chars[1], chars[2]]);
            push_rafsi(&mut raw, RafsiKind::Cvc, &[chars[0], chars[1], chars[3]]);
            push_rafsi(
                &mut raw,
                RafsiKind::Cvv,
                &[chars[0], chars[1], '\'', chars[4]],
            );
            push_cvv_without_apostrophe(&mut raw, chars[0], chars[1], chars[4]);
            push_ccv_if_initial(&mut raw, chars[2], chars[3], chars[4]);
            push_ccv_if_initial(&mut raw, chars[0], chars[2], chars[1]);
        }
        Some(GismuShape::Ccvcv) => {
            push_rafsi(&mut raw, RafsiKind::Cvc, &[chars[0], chars[2], chars[3]]);
            push_rafsi(&mut raw, RafsiKind::Cvc, &[chars[1], chars[2], chars[3]]);
            push_rafsi(
                &mut raw,
                RafsiKind::Cvv,
                &[chars[0], chars[2], '\'', chars[4]],
            );
            push_cvv_without_apostrophe(&mut raw, chars[0], chars[2], chars[4]);
            push_rafsi(
                &mut raw,
                RafsiKind::Cvv,
                &[chars[1], chars[2], '\'', chars[4]],
            );
            push_cvv_without_apostrophe(&mut raw, chars[1], chars[2], chars[4]);
            push_ccv_if_initial(&mut raw, chars[0], chars[1], chars[2]);
        }
        None => {}
    }
    raw.sort_by(|left, right| left.form.cmp(&right.form));
    raw.dedup_by(|left, right| left.form == right.form);
    raw.into_iter()
        .map(|mut rafsi| {
            let (availability, taken_by) = rafsi_availability(dictionary, &rafsi.form);
            rafsi.availability = availability;
            rafsi.taken_by = taken_by;
            rafsi
        })
        .collect()
}

#[requires(chars.len() == 5)]
#[ensures(true)]
fn gismu_shape_for_word(chars: &[char]) -> Option<GismuShape> {
    if is_consonant(chars[0])
        && is_consonant(chars[1])
        && is_vowel(chars[2])
        && is_consonant(chars[3])
        && is_vowel(chars[4])
    {
        Some(GismuShape::Ccvcv)
    } else if is_consonant(chars[0])
        && is_vowel(chars[1])
        && is_consonant(chars[2])
        && is_consonant(chars[3])
        && is_vowel(chars[4])
    {
        Some(GismuShape::Cvccv)
    } else {
        None
    }
}

#[requires(!letters.is_empty())]
#[ensures(output.last().is_some_and(|rafsi| !rafsi.form.is_empty()))]
fn push_rafsi(output: &mut Vec<RafsiCandidate>, kind: RafsiKind, letters: &[char]) {
    output.push(RafsiCandidate {
        form: letters.iter().collect(),
        kind,
        availability: RafsiAvailability::Free,
        taken_by: Vec::new(),
    });
}

#[requires(is_consonant(consonant))]
#[requires(is_vowel(first_vowel))]
#[requires(is_vowel(second_vowel))]
#[ensures(true)]
fn push_cvv_without_apostrophe(
    output: &mut Vec<RafsiCandidate>,
    consonant: char,
    first_vowel: char,
    second_vowel: char,
) {
    if matches!(
        (first_vowel, second_vowel),
        ('a', 'i') | ('e', 'i') | ('o', 'i') | ('a', 'u')
    ) {
        push_rafsi(
            output,
            RafsiKind::Cvv,
            &[consonant, first_vowel, second_vowel],
        );
    }
}

#[requires(is_consonant(first))]
#[requires(is_consonant(second))]
#[requires(is_vowel(vowel))]
#[ensures(true)]
fn push_ccv_if_initial(output: &mut Vec<RafsiCandidate>, first: char, second: char, vowel: char) {
    if permissible_consonant_pair(first, second) == Some(2) {
        push_rafsi(output, RafsiKind::Ccv, &[first, second, vowel]);
    }
}

#[requires(!rafsi.is_empty())]
#[ensures(true)]
fn rafsi_availability(
    dictionary: &Dictionary<'_>,
    rafsi: &str,
) -> (RafsiAvailability, Vec<String>) {
    let matches = dictionary.lookup_rafsi(rafsi).collect::<Vec<_>>();
    let official = matches
        .iter()
        .filter(|matched| matched.entry.word_type == WordType::Gismu)
        .map(|matched| matched.entry.word.to_owned())
        .collect::<Vec<_>>();
    if !official.is_empty() {
        return (RafsiAvailability::OfficialTaken, official);
    }
    let experimental = matches
        .iter()
        .filter(|matched| matched.entry.word_type == WordType::ExperimentalGismu)
        .map(|matched| matched.entry.word.to_owned())
        .collect::<Vec<_>>();
    if !experimental.is_empty() {
        (RafsiAvailability::ExperimentalTaken, experimental)
    } else {
        (RafsiAvailability::Free, Vec::new())
    }
}

#[requires(true)]
#[ensures(true)]
fn candidate_passes_filters(candidate: &GimfiheCandidate, request: &GimfiheRequest) -> bool {
    candidate.collision.is_none()
        && (!request.require_free_short_rafsi
            || candidate
                .rafsi
                .iter()
                .any(|rafsi| rafsi.availability == RafsiAvailability::Free))
}

#[requires(true)]
#[ensures(true)]
fn compare_candidates(left: &GimfiheCandidate, right: &GimfiheCandidate) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| left.word.cmp(&right.word))
}

#[requires(true)]
#[ensures(!ret.contains(char::is_whitespace))]
fn normalize_language(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[requires(true)]
#[ensures(!ret.contains(char::is_whitespace))]
fn normalize_source_word(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[requires(true)]
#[ensures(ret.chars().all(|value| is_consonant(value) || is_vowel(value)))]
fn normalize_gismu(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|value| is_consonant(*value) || is_vowel(*value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scoring_uses_cll_lcs_rules() {
        assert_eq!(gismu_match_score("kanpe", "ekspekt"), 3);
        assert_eq!(gismu_match_score("kanpe", "uan"), 2);
        assert_eq!(gismu_match_score("kanpe", "na"), 0);
        assert_eq!(gismu_match_score("kanpe", "ap"), 0);
        assert_eq!(gismu_match_score("kanpe", "akp"), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_sources_with_empty_preset_weight() {
        let source = parse_source_spec("eng::ekspekt").expect("source");
        assert_eq!(source.language, "eng");
        assert_eq!(source.explicit_weight, None);
        assert_eq!(source.word, "ekspekt");

        let source = parse_source_spec("eng:0.160:ekspekt").expect("source");
        assert_eq!(source.explicit_weight, Some(0.160));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn resolves_preset_languages_in_preset_order() {
        let sources = [
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
            parse_source_spec("cmn::uan").expect("source"),
            parse_source_spec("spa::esper").expect("source"),
            parse_source_spec("rus::predpologa").expect("source"),
            parse_source_spec("ara::mulud").expect("source"),
        ];
        let resolved = resolve_sources(Some(GimfihePreset::Data1995), &sources).expect("resolved");
        assert_eq!(
            resolved
                .iter()
                .map(|source| source.language.as_str())
                .collect::<Vec<_>>(),
            ["cmn", "hin", "eng", "spa", "rus", "ara"]
        );
        assert_eq!(resolved[2].weight, 0.160);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_wrong_preset_language_set() {
        let sources = [
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
        ];
        let error = resolve_sources(Some(GimfihePreset::Data1995), &sources)
            .expect_err("missing preset language");
        assert!(matches!(error, GimfiheError::MissingPresetLanguage { .. }));

        let sources = [
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
            parse_source_spec("cmn::uan").expect("source"),
            parse_source_spec("spa::esper").expect("source"),
            parse_source_spec("rus::predpologa").expect("source"),
            parse_source_spec("ara::mulud").expect("source"),
            parse_source_spec("fra::esper").expect("source"),
        ];
        let error = resolve_sources(Some(GimfihePreset::Data1995), &sources)
            .expect_err("extra preset language");
        assert!(matches!(error, GimfiheError::ExtraPresetLanguage { .. }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_shapes_and_collision_scopes() {
        assert_eq!(parse_shape("ccvcv").expect("shape"), GismuShape::Ccvcv);
        assert!(parse_shape("cvcvv").is_err());
        assert_eq!(
            parse_collision_scope("official").expect("scope"),
            CollisionScope::Official
        );
        assert!(parse_collision_scope("some").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn detects_cll_collision_kinds() {
        let dictionary = jbotci_dictionary_data::english();
        assert_eq!(
            find_collision(dictionary, CollisionScope::Official, "klama")
                .expect("identical")
                .kind,
            CollisionKind::Identical
        );
        assert_eq!(
            find_collision(dictionary, CollisionScope::Official, "klami")
                .expect("final vowel")
                .kind,
            CollisionKind::FinalVowel
        );
        assert_eq!(
            find_collision(dictionary, CollisionScope::Official, "plama")
                .expect("similar consonant")
                .kind,
            CollisionKind::SimilarConsonant
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rafsi_availability_uses_dictionary_index() {
        let dictionary = jbotci_dictionary_data::english();
        let rafsi = possible_short_rafsis(dictionary, "sakli");
        let sal = rafsi
            .iter()
            .find(|candidate| candidate.form == "sal")
            .expect("sal");
        assert_eq!(sal.availability, RafsiAvailability::OfficialTaken);
        let nanpe = possible_short_rafsis(dictionary, "nanpe");
        let free = nanpe
            .iter()
            .any(|candidate| candidate.availability == RafsiAvailability::Free);
        assert!(free);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compose_finds_highlighted_candidate_outside_count() {
        let dictionary = jbotci_dictionary_data::english();
        let sources = [
            parse_source_spec("cmn::uan").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("spa::esper").expect("source"),
            parse_source_spec("rus::predpologa").expect("source"),
            parse_source_spec("ara::mulud").expect("source"),
        ];
        let request = GimfiheRequest {
            preset: Some(GimfihePreset::Data1995),
            sources: sources.to_vec(),
            shapes: default_shapes(),
            all_letters: false,
            check_collisions: CollisionScope::None,
            require_free_short_rafsi: false,
            count: 1,
            highlight: Some("nanpe".to_owned()),
        };
        let output = compose_gismu(dictionary, &request).expect("output");
        assert_eq!(
            output
                .candidates
                .first()
                .map(|candidate| candidate.word.as_str()),
            output.winner.as_deref()
        );
        assert!(
            output
                .candidates
                .iter()
                .any(|candidate| candidate.word == "nanpe" && candidate.highlighted)
        );
    }
}
