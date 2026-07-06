//! CLL gismu composition.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_dictionary::{Dictionary, WordType};
use jbotci_morphology::{
    ConsonantPairClass, GlideMark, PhonemeRenderOptions, StressMark, WordKind,
    consonant_pair_class, is_consonant, is_vowel, segment_words_with_modifiers,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod transliterate;

pub use transliterate::transliterate_ipa_to_lojban;

pub const GIMFIHI_DEFAULT_COUNT: usize = 20;
pub const GIMFIHI_MAX_COUNT: usize = 512;
pub const GIMFIHI_MIN_WEIGHT: u16 = 1;
pub const GIMFIHI_MAX_WEIGHT: u16 = 999;

const GIMFIHI_WEIGHT_SCALE: f64 = 1000.0;

const CONSONANTS: &[char] = &[
    'b', 'c', 'd', 'f', 'g', 'j', 'k', 'l', 'm', 'n', 'p', 'r', 's', 't', 'v', 'x', 'z',
];
const VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u'];

type PresetEntrySpec = (&'static str, u16);

static PRESET_1985: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 360),
        ("hin", 160),
        ("eng", 210),
        ("spa", 110),
        ("rus", 90),
        ("ara", 70),
    ])
});
static PRESET_1987: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 360),
        ("hin", 156),
        ("eng", 208),
        ("spa", 116),
        ("rus", 87),
        ("ara", 73),
    ])
});
static PRESET_1994: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 348),
        ("hin", 194),
        ("eng", 163),
        ("spa", 123),
        ("rus", 88),
        ("ara", 84),
    ])
});
static PRESET_1995: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 347),
        ("hin", 196),
        ("eng", 160),
        ("spa", 123),
        ("rus", 89),
        ("ara", 85),
    ])
});
static PRESET_1999: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 334),
        ("hin", 195),
        ("eng", 187),
        ("spa", 116),
        ("rus", 81),
        ("ara", 88),
    ])
});
static PRESET_EVENLY: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 1),
        ("hin", 1),
        ("eng", 1),
        ("spa", 1),
        ("rus", 1),
        ("ara", 1),
    ])
});
static PRESET_ILMEN6: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("eng", 339),
        ("cmn", 255),
        ("hin", 136),
        ("spa", 126),
        ("ara", 74),
        ("fra", 70),
    ])
});
static PRESET_ILMEN8: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 271),
        ("eng", 170),
        ("spa", 130),
        ("hin", 125),
        ("ara", 104),
        ("ben", 76),
        ("rus", 64),
        ("por", 60),
    ])
});
static PRESET_ILMEN12: LazyLock<Box<[PresetEntry]>> = LazyLock::new(|| {
    preset_entries(&[
        ("cmn", 238),
        ("eng", 160),
        ("spa", 113),
        ("hin", 108),
        ("ara", 90),
        ("ben", 66),
        ("rus", 55),
        ("por", 52),
        ("msa", 32),
        ("jpn", 30),
        ("deu", 29),
        ("fra", 26),
    ])
});

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GimfihiPreset {
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

impl GimfihiPreset {
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
    pub fn entries(self) -> &'static [PresetEntry] {
        match self {
            Self::Data1985 => PRESET_1985.as_ref(),
            Self::Data1987 => PRESET_1987.as_ref(),
            Self::Data1994 => PRESET_1994.as_ref(),
            Self::Data1995 => PRESET_1995.as_ref(),
            Self::Data1999 => PRESET_1999.as_ref(),
            Self::Evenly => PRESET_EVENLY.as_ref(),
            Self::Ilmen6 => PRESET_ILMEN6.as_ref(),
            Self::Ilmen8 => PRESET_ILMEN8.as_ref(),
            Self::Ilmen12 => PRESET_ILMEN12.as_ref(),
        }
    }
}

impl fmt::Display for GimfihiPreset {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GimfihiPreset {
    type Err = GimfihiError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|preset| !preset.as_str().is_empty()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_preset(value)
    }
}

#[invariant(!language.is_empty())]
#[invariant(*weight >= GIMFIHI_MIN_WEIGHT && *weight <= GIMFIHI_MAX_WEIGHT)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresetEntry {
    pub language: &'static str,
    pub weight: u16,
}

impl PresetEntry {
    #[requires(!language.is_empty())]
    #[requires(weight >= GIMFIHI_MIN_WEIGHT && weight <= GIMFIHI_MAX_WEIGHT)]
    #[ensures(true)]
    pub fn new(language: &'static str, weight: u16) -> Self {
        new!(PresetEntry {
            language: language,
            weight: weight,
        })
    }
}

#[requires(!entries.is_empty())]
#[ensures(!ret.is_empty())]
fn preset_entries(entries: &[PresetEntrySpec]) -> Box<[PresetEntry]> {
    entries
        .iter()
        .map(|(language, weight)| PresetEntry::new(language, *weight))
        .collect::<Vec<_>>()
        .into_boxed_slice()
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
    type Err = GimfihiError;

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
    type Err = GimfihiError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|scope| !scope.as_str().is_empty()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_collision_scope(value)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfihiSourceInput {
    pub language: String,
    pub explicit_weight: Option<u16>,
    pub word: String,
}

impl GimfihiSourceInput {
    /// Build a source from already-structured fields, applying the same
    /// language/word normalization that [`parse_source_spec`] performs on the
    /// `LANG[:WEIGHT]:WORD` CLI spec. This lets callers that already have the
    /// pieces separated (e.g. the MCP tool layer) skip the spec-string round
    /// trip while staying consistent with the parsed form.
    #[requires(explicit_weight.is_none_or(|weight| (GIMFIHI_MIN_WEIGHT..=GIMFIHI_MAX_WEIGHT).contains(&weight)))]
    #[ensures(ret.explicit_weight == explicit_weight)]
    pub fn from_fields(language: &str, word: &str, explicit_weight: Option<u16>) -> Self {
        Self {
            language: normalize_language(language),
            explicit_weight,
            word: normalize_source_word(word),
        }
    }

    /// Build a source from an IPA pronunciation, for callers (the MCP tool) that
    /// take IPA directly rather than via the `[IPA]` bracket convention. The IPA
    /// is transliterated to Lojban letters later, in [`resolve_sources`].
    #[requires(explicit_weight.is_none_or(|weight| (GIMFIHI_MIN_WEIGHT..=GIMFIHI_MAX_WEIGHT).contains(&weight)))]
    #[ensures(ret.explicit_weight == explicit_weight)]
    pub fn from_ipa_fields(language: &str, ipa: &str, explicit_weight: Option<u16>) -> Self {
        Self {
            language: normalize_language(language),
            explicit_weight,
            word: normalize_source_word(&format!("[{ipa}]")),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResolvedSource {
    pub language: String,
    pub weight: u16,
    pub word: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfihiRequest {
    pub preset: Option<GimfihiPreset>,
    pub sources: Vec<GimfihiSourceInput>,
    pub shapes: Vec<GismuShape>,
    pub all_letters: bool,
    pub check_collisions: CollisionScope,
    pub show_collisions: bool,
    pub require_free_short_rafsi: bool,
    pub count: usize,
    pub highlight: Option<String>,
}

impl Default for GimfihiRequest {
    #[requires(true)]
    #[ensures(ret.count == GIMFIHI_DEFAULT_COUNT)]
    fn default() -> Self {
        Self {
            preset: None,
            sources: Vec::new(),
            shapes: default_shapes(),
            all_letters: false,
            check_collisions: CollisionScope::All,
            show_collisions: false,
            require_free_short_rafsi: false,
            count: GIMFIHI_DEFAULT_COUNT,
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
    pub existing_word_type: WordType,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceScore {
    pub language: String,
    pub word: String,
    pub weight: u16,
    pub raw_score: usize,
    pub weighted_score: f64,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfihiCandidate {
    pub word: String,
    pub score: f64,
    pub source_scores: Vec<SourceScore>,
    pub collision: Option<GismuCollision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rafsi: Option<Vec<RafsiCandidate>>,
    pub highlighted: bool,
}

impl GimfihiCandidate {
    #[requires(true)]
    #[ensures(true)]
    pub fn rafsi(&self) -> &[RafsiCandidate] {
        self.rafsi.as_deref().unwrap_or(&[])
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GimfihiOutput {
    pub resolved_sources: Vec<ResolvedSource>,
    pub candidate_count: usize,
    pub filtered_count: usize,
    pub winner: Option<String>,
    pub highlighted_word: Option<String>,
    pub candidates: Vec<GimfihiCandidate>,
}

#[invariant(!word.is_empty())]
#[derive(Debug, Clone, PartialEq)]
struct ScoredGimfihiCandidate {
    word: String,
    score: f64,
    collision: Option<GismuCollision>,
    rafsi: Option<Vec<RafsiCandidate>>,
}

#[invariant(!source.word.is_empty())]
#[invariant(!chars.is_empty())]
#[derive(Debug)]
struct PreparedSource<'source> {
    source: &'source ResolvedSource,
    chars: Vec<char>,
}

#[invariant(true)]
#[derive(Debug, Default)]
struct GismuScoreScratch {
    candidate_chars: Vec<char>,
    lcs: LcsScratch,
}

#[invariant(true)]
#[derive(Debug, Default)]
struct LcsScratch {
    previous: Vec<usize>,
    current: Vec<usize>,
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
#[invariant(::InvalidIpa { .. } => true)]
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GimfihiError {
    #[error("unknown gimfi'i preset `{value}`")]
    UnknownPreset { value: String },
    #[error("unknown gismu shape `{value}`")]
    UnknownShape { value: String },
    #[error("unknown collision checking mode `{value}`")]
    UnknownCollisionScope { value: String },
    #[error("invalid --source `{spec}`: {message}")]
    InvalidSourceSpec { spec: String, message: String },
    #[error("source weight must be an integer from 1 to 999, got `{value}`")]
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
        "source word `{word}` for `{language}` must be non-empty Lojbanized text using only gismu-scoring letters"
    )]
    InvalidSourceWord { language: String, word: String },
    #[error("invalid IPA source `{ipa}`: {reason}")]
    InvalidIpa { ipa: String, reason: String },
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
pub fn all_presets() -> &'static [GimfihiPreset] {
    &[
        GimfihiPreset::Data1985,
        GimfihiPreset::Data1987,
        GimfihiPreset::Data1994,
        GimfihiPreset::Data1995,
        GimfihiPreset::Data1999,
        GimfihiPreset::Evenly,
        GimfihiPreset::Ilmen6,
        GimfihiPreset::Ilmen8,
        GimfihiPreset::Ilmen12,
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
pub fn parse_preset(value: &str) -> Result<GimfihiPreset, GimfihiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1985" => Ok(GimfihiPreset::Data1985),
        "1987" => Ok(GimfihiPreset::Data1987),
        "1994" => Ok(GimfihiPreset::Data1994),
        "1995" => Ok(GimfihiPreset::Data1995),
        "1999" => Ok(GimfihiPreset::Data1999),
        "evenly" => Ok(GimfihiPreset::Evenly),
        "ilmen6" => Ok(GimfihiPreset::Ilmen6),
        "ilmen8" => Ok(GimfihiPreset::Ilmen8),
        "ilmen12" => Ok(GimfihiPreset::Ilmen12),
        _ => Err(GimfihiError::UnknownPreset {
            value: value.to_owned(),
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|shape| !shape.as_str().is_empty()) || ret.is_err())]
pub fn parse_shape(value: &str) -> Result<GismuShape, GimfihiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "ccvcv" => Ok(GismuShape::Ccvcv),
        "cvccv" => Ok(GismuShape::Cvccv),
        _ => Err(GimfihiError::UnknownShape {
            value: value.to_owned(),
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|scope| !scope.as_str().is_empty()) || ret.is_err())]
pub fn parse_collision_scope(value: &str) -> Result<CollisionScope, GimfihiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "all" => Ok(CollisionScope::All),
        "official" => Ok(CollisionScope::Official),
        "none" => Ok(CollisionScope::None),
        _ => Err(GimfihiError::UnknownCollisionScope {
            value: value.to_owned(),
        }),
    }
}

#[requires(!spec.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|source| !source.language.is_empty()) || ret.is_err())]
pub fn parse_source_spec(spec: &str) -> Result<GimfihiSourceInput, GimfihiError> {
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
        _ => Err(GimfihiError::InvalidSourceSpec {
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
    explicit_weight: Option<u16>,
    word: &str,
) -> Result<GimfihiSourceInput, GimfihiError> {
    let language = normalize_language(language);
    let word = normalize_source_word(word);
    if language.is_empty() {
        return Err(GimfihiError::InvalidSourceSpec {
            spec: spec.to_owned(),
            message: "language must be non-empty".to_owned(),
        });
    }
    Ok(GimfihiSourceInput {
        language,
        explicit_weight,
        word,
    })
}

#[requires(!value.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|weight| *weight >= GIMFIHI_MIN_WEIGHT && *weight <= GIMFIHI_MAX_WEIGHT) || ret.is_err())]
pub fn parse_weight(value: &str) -> Result<u16, GimfihiError> {
    let Ok(weight) = value.parse::<u16>() else {
        return Err(GimfihiError::InvalidWeight {
            value: value.to_owned(),
        });
    };
    if (GIMFIHI_MIN_WEIGHT..=GIMFIHI_MAX_WEIGHT).contains(&weight) {
        Ok(weight)
    } else {
        Err(GimfihiError::InvalidWeight {
            value: value.to_owned(),
        })
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn compose_gismu(
    dictionary: &Dictionary<'_>,
    request: &GimfihiRequest,
) -> Result<GimfihiOutput, GimfihiError> {
    let resolved_sources = resolve_sources(request.preset, &request.sources)?;
    if request.shapes.is_empty() {
        return Err(GimfihiError::NoShapes);
    }
    let shapes = unique_shapes(&request.shapes);
    let prepared_sources = prepare_scoring_sources(&resolved_sources);
    let collision_index = CollisionIndex::from_dictionary(dictionary, request.check_collisions);
    let mut scratch = GismuScoreScratch::default();
    let candidates = generate_candidates(&resolved_sources, &shapes, request.all_letters)
        .into_iter()
        .filter(|word| valid_gismu_candidate(word))
        .map(|word| {
            score_candidate_for_ranking_with_scratch(
                dictionary,
                &collision_index,
                &prepared_sources,
                word,
                request.require_free_short_rafsi,
                &mut scratch,
            )
        })
        .collect::<Vec<_>>();
    let candidate_count = candidates.len();
    let mut filtered = candidates
        .into_iter()
        .filter(|candidate| scored_candidate_passes_filters(candidate, request))
        .collect::<Vec<_>>();
    filtered.sort_by(compare_scored_candidates);
    let filtered_count = filtered.len();
    // The winner is the best *usable* candidate: skip any that collide with an
    // existing gismu, even when `show_collisions` keeps them in the list.
    let winner = filtered
        .iter()
        .find(|candidate| candidate.collision.is_none())
        .map(|candidate| candidate.word.clone());
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
        .take(request.count.min(GIMFIHI_MAX_COUNT))
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
    let mut detail_scratch = GismuScoreScratch::default();
    let displayed = displayed
        .into_iter()
        .map(|candidate| {
            materialize_candidate_details(
                dictionary,
                &prepared_sources,
                candidate,
                highlighted_word.as_deref(),
                &mut detail_scratch,
            )
        })
        .collect::<Vec<_>>();
    Ok(GimfihiOutput {
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
    preset: Option<GimfihiPreset>,
    sources: &[GimfihiSourceInput],
) -> Result<Vec<ResolvedSource>, GimfihiError> {
    if sources.is_empty() {
        return Err(GimfihiError::NoSources);
    }
    // Transliterate any `[IPA]` words to Lojban letters before validating, so
    // every step downstream sees plain gismu-scoring letters.
    let prepared = sources
        .iter()
        .map(prepare_source_word)
        .collect::<Result<Vec<_>, _>>()?;
    validate_unique_languages(&prepared)?;
    for source in &prepared {
        validate_source_word(source)?;
    }
    match preset {
        Some(preset) => resolve_preset_sources(preset, &prepared),
        None => resolve_custom_sources(&prepared),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|prepared| prepared.language == source.language && prepared.explicit_weight == source.explicit_weight) || ret.is_err())]
fn prepare_source_word(source: &GimfihiSourceInput) -> Result<GimfihiSourceInput, GimfihiError> {
    match extract_bracketed_ipa(&source.word)? {
        Some(ipa) => Ok(GimfihiSourceInput {
            language: source.language.clone(),
            explicit_weight: source.explicit_weight,
            word: transliterate_ipa_to_lojban(&ipa)?,
        }),
        None => Ok(source.clone()),
    }
}

/// Extract the IPA inside `[ ... ]`, mirroring the sound-search bracket
/// convention; `Ok(None)` means the word is already plain Lojban letters.
#[requires(true)]
#[ensures(true)]
fn extract_bracketed_ipa(word: &str) -> Result<Option<String>, GimfihiError> {
    let trimmed = word.trim();
    let invalid = |reason: &str| GimfihiError::InvalidIpa {
        ipa: word.to_owned(),
        reason: reason.to_owned(),
    };
    match (trimmed.starts_with('['), trimmed.ends_with(']')) {
        (true, true) => {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                Err(invalid("bracketed IPA must not be empty"))
            } else if inner.contains('[') || inner.contains(']') {
                Err(invalid(
                    "use one `[ ... ]` pair around the whole IPA transcription",
                ))
            } else {
                Ok(Some(inner.to_owned()))
            }
        }
        (true, false) => Err(invalid("IPA starts with `[` but does not end with `]`")),
        (false, true) => Err(invalid("IPA ends with `]` but does not start with `[`")),
        (false, false) => Ok(None),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_unique_languages(sources: &[GimfihiSourceInput]) -> Result<(), GimfihiError> {
    let mut seen = BTreeSet::new();
    for source in sources {
        let language = normalize_language(&source.language);
        if !seen.insert(language.clone()) {
            return Err(GimfihiError::DuplicateSourceLanguage { language });
        }
    }
    Ok(())
}

#[requires(!source.language.is_empty() || !source.word.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_source_word(source: &GimfihiSourceInput) -> Result<(), GimfihiError> {
    let word = normalize_source_word(&source.word);
    if word.is_empty() || !word.chars().all(is_valid_source_word_char) {
        return Err(GimfihiError::InvalidSourceWord {
            language: normalize_language(&source.language),
            word,
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sources| !sources.is_empty()) || ret.is_err())]
fn resolve_preset_sources(
    preset: GimfihiPreset,
    sources: &[GimfihiSourceInput],
) -> Result<Vec<ResolvedSource>, GimfihiError> {
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
            return Err(GimfihiError::ExtraPresetLanguage {
                language: language.clone(),
            });
        }
    }
    let mut resolved = Vec::new();
    for entry in preset.entries() {
        let Some(source) = source_by_language.get(entry.language) else {
            return Err(GimfihiError::MissingPresetLanguage {
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
    sources: &[GimfihiSourceInput],
) -> Result<Vec<ResolvedSource>, GimfihiError> {
    let mut resolved = Vec::new();
    for source in sources {
        let language = normalize_language(&source.language);
        let Some(weight) = source.explicit_weight else {
            return Err(GimfihiError::MissingExplicitWeight { language });
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
    is_consonant(value) || is_vowel(value)
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
                    if consonant_pair_class(*c0, *c1) != Some(ConsonantPairClass::Initial) {
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
                            if !consonant_pair_class(*c2, *c3)
                                .is_some_and(ConsonantPairClass::is_permissible)
                            {
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

#[requires(!sources.is_empty())]
#[ensures(ret.len() == sources.len())]
fn prepare_scoring_sources(sources: &[ResolvedSource]) -> Vec<PreparedSource<'_>> {
    sources
        .iter()
        .map(|source| {
            let chars = source.word.chars().collect::<Vec<_>>();
            new!(PreparedSource { source, chars })
        })
        .collect()
}

#[requires(!word.is_empty())]
#[requires(!sources.is_empty())]
#[ensures(!ret.word.is_empty())]
fn score_candidate_for_ranking_with_scratch(
    dictionary: &Dictionary<'_>,
    collision_index: &CollisionIndex,
    sources: &[PreparedSource<'_>],
    word: String,
    include_rafsi: bool,
    scratch: &mut GismuScoreScratch,
) -> ScoredGimfihiCandidate {
    scratch.candidate_chars.clear();
    scratch.candidate_chars.extend(word.chars());
    let score = sources
        .iter()
        .map(|source| score_source_value(&scratch.candidate_chars, source, &mut scratch.lcs))
        .sum::<f64>();
    let collision = collision_index.find(&word);
    let rafsi = include_rafsi.then(|| possible_short_rafsis(dictionary, &word));
    new!(ScoredGimfihiCandidate {
        word,
        score,
        collision,
        rafsi,
    })
}

#[requires(!candidate.word.is_empty())]
#[requires(!sources.is_empty())]
#[ensures(!ret.word.is_empty())]
fn materialize_candidate_details(
    dictionary: &Dictionary<'_>,
    sources: &[PreparedSource<'_>],
    candidate: ScoredGimfihiCandidate,
    highlighted_word: Option<&str>,
    scratch: &mut GismuScoreScratch,
) -> GimfihiCandidate {
    scratch.candidate_chars.clear();
    scratch.candidate_chars.extend(candidate.word.chars());
    let source_scores = sources
        .iter()
        .map(|source| materialize_source_score(&scratch.candidate_chars, source, &mut scratch.lcs))
        .collect::<Vec<_>>();
    let candidate = candidate.into_data();
    let word = candidate.word;
    let rafsi = candidate
        .rafsi
        .unwrap_or_else(|| possible_short_rafsis(dictionary, &word));
    GimfihiCandidate {
        highlighted: highlighted_word.is_some_and(|highlighted| highlighted == word.as_str()),
        word,
        score: candidate.score,
        source_scores,
        collision: candidate.collision,
        rafsi: Some(rafsi),
    }
}

#[requires(!candidate_chars.is_empty())]
#[requires(!source.source.word.is_empty())]
#[ensures(true)]
fn score_source_value(
    candidate_chars: &[char],
    source: &PreparedSource<'_>,
    scratch: &mut LcsScratch,
) -> f64 {
    let raw_score = gismu_match_score_chars(candidate_chars, &source.chars, scratch);
    let scaled_weight = f64::from(source.source.weight) / GIMFIHI_WEIGHT_SCALE;
    raw_score as f64 / source.chars.len() as f64 * scaled_weight
}

#[requires(!candidate_chars.is_empty())]
#[requires(!source.source.word.is_empty())]
#[ensures(ret.weight == source.source.weight)]
fn materialize_source_score(
    candidate_chars: &[char],
    source: &PreparedSource<'_>,
    scratch: &mut LcsScratch,
) -> SourceScore {
    let raw_score = gismu_match_score_chars(candidate_chars, &source.chars, scratch);
    let scaled_weight = f64::from(source.source.weight) / GIMFIHI_WEIGHT_SCALE;
    let weighted_score = raw_score as f64 / source.chars.len() as f64 * scaled_weight;
    SourceScore {
        language: source.source.language.clone(),
        word: source.source.word.clone(),
        weight: source.source.weight,
        raw_score,
        weighted_score,
    }
}

#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures(ret <= candidate.chars().count())]
pub fn gismu_match_score(candidate: &str, source: &str) -> usize {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let source_chars = source.chars().collect::<Vec<_>>();
    let mut scratch = LcsScratch::default();
    gismu_match_score_chars(&candidate_chars, &source_chars, &mut scratch)
}

#[requires(!candidate_chars.is_empty())]
#[requires(!source_chars.is_empty())]
#[ensures(ret <= candidate_chars.len())]
fn gismu_match_score_chars(
    candidate_chars: &[char],
    source_chars: &[char],
    scratch: &mut LcsScratch,
) -> usize {
    let lcs = longest_common_subsequence_len_chars(candidate_chars, source_chars, scratch);
    if lcs >= 3 {
        return lcs;
    }
    if lcs != 2 {
        return 0;
    }
    if has_valid_two_letter_match_chars(candidate_chars, source_chars) {
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
    let mut scratch = LcsScratch::default();
    longest_common_subsequence_len_chars(&left_chars, &right_chars, &mut scratch)
}

#[requires(true)]
#[ensures(ret <= left.len().min(right.len()))]
fn longest_common_subsequence_len_chars(
    left: &[char],
    right: &[char],
    scratch: &mut LcsScratch,
) -> usize {
    scratch.previous.clear();
    scratch.current.clear();
    scratch.previous.resize(right.len() + 1, 0);
    scratch.current.resize(right.len() + 1, 0);
    for left_ch in left {
        for (right_index, right_ch) in right.iter().enumerate() {
            scratch.current[right_index + 1] = if *left_ch == *right_ch {
                scratch.previous[right_index] + 1
            } else {
                scratch.previous[right_index + 1].max(scratch.current[right_index])
            };
        }
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
        scratch.current.fill(0);
    }
    scratch.previous[right.len()]
}

#[requires(true)]
#[ensures(true)]
fn has_valid_two_letter_match(candidate: &str, source: &str) -> bool {
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let source_chars = source.chars().collect::<Vec<_>>();
    has_valid_two_letter_match_chars(&candidate_chars, &source_chars)
}

#[requires(true)]
#[ensures(true)]
fn has_valid_two_letter_match_chars(candidate_chars: &[char], source_chars: &[char]) -> bool {
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

#[invariant(
    collisions
        .iter()
        .all(|(candidate, collision)| !candidate.is_empty() && !collision.existing_word.is_empty())
)]
struct CollisionIndex {
    collisions: BTreeMap<String, GismuCollision>,
}

impl CollisionIndex {
    #[requires(true)]
    #[ensures(scope != CollisionScope::None || ret.collisions.is_empty())]
    fn from_dictionary(dictionary: &Dictionary<'_>, scope: CollisionScope) -> Self {
        let mut collisions = BTreeMap::new();
        if scope != CollisionScope::None {
            for entry in dictionary
                .entries()
                .iter()
                .filter(|entry| collision_scope_includes(scope, entry.word_type))
            {
                insert_collision_entry(&mut collisions, entry.word, entry.word_type);
            }
        }
        new!(CollisionIndex { collisions })
    }

    #[requires(true)]
    #[ensures(true)]
    fn find(&self, candidate: &str) -> Option<GismuCollision> {
        self.collisions.get(candidate).cloned()
    }
}

#[requires(true)]
#[requires(!existing.is_empty())]
#[ensures(true)]
fn insert_collision_entry(
    collisions: &mut BTreeMap<String, GismuCollision>,
    existing: &str,
    word_type: WordType,
) {
    let collision = collision(CollisionKind::Identical, existing, word_type);
    insert_collision(collisions, existing.to_owned(), collision);

    let chars = existing.chars().collect::<Vec<_>>();
    if chars.len() != 5 {
        return;
    }
    insert_final_vowel_collisions(collisions, &chars, existing, word_type);
    insert_similar_consonant_collisions(collisions, &chars, existing, word_type);
}

#[requires(true)]
#[requires(chars.len() == 5)]
#[requires(!existing.is_empty())]
#[ensures(true)]
fn insert_final_vowel_collisions(
    collisions: &mut BTreeMap<String, GismuCollision>,
    chars: &[char],
    existing: &str,
    word_type: WordType,
) {
    if !is_vowel(chars[4]) {
        return;
    }
    for vowel in VOWELS {
        if *vowel == chars[4] {
            continue;
        }
        let mut candidate = chars.to_vec();
        candidate[4] = *vowel;
        let collision = collision(CollisionKind::FinalVowel, existing, word_type);
        insert_collision(collisions, candidate.iter().collect(), collision);
    }
}

#[requires(true)]
#[requires(chars.len() == 5)]
#[requires(!existing.is_empty())]
#[ensures(true)]
fn insert_similar_consonant_collisions(
    collisions: &mut BTreeMap<String, GismuCollision>,
    chars: &[char],
    existing: &str,
    word_type: WordType,
) {
    for (index, existing_consonant) in chars.iter().copied().enumerate() {
        if !is_consonant(existing_consonant) {
            continue;
        }
        for proposed in CONSONANTS
            .iter()
            .copied()
            .filter(|proposed| too_similar_consonant(*proposed, existing_consonant))
        {
            let mut candidate = chars.to_vec();
            candidate[index] = proposed;
            let collision = collision(CollisionKind::SimilarConsonant, existing, word_type);
            insert_collision(collisions, candidate.iter().collect(), collision);
        }
    }
}

#[requires(true)]
#[requires(!candidate.is_empty())]
#[requires(!collision.existing_word.is_empty())]
#[ensures(true)]
fn insert_collision(
    collisions: &mut BTreeMap<String, GismuCollision>,
    candidate: String,
    collision: GismuCollision,
) {
    match collisions.get(&candidate) {
        Some(existing) if !collision_beats(collision.kind, &collision.existing_word, existing) => {}
        _ => {
            collisions.insert(candidate, collision);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collision_beats(kind: CollisionKind, existing_word: &str, current: &GismuCollision) -> bool {
    collision_kind_order(kind)
        .cmp(&collision_kind_order(current.kind))
        .then_with(|| existing_word.cmp(&current.existing_word))
        .is_lt()
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
        existing_word_type: word_type,
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
    if consonant_pair_class(first, second) == Some(ConsonantPairClass::Initial) {
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
fn scored_candidate_passes_filters(
    candidate: &ScoredGimfihiCandidate,
    request: &GimfihiRequest,
) -> bool {
    (request.show_collisions || candidate.collision.is_none())
        && (!request.require_free_short_rafsi
            || candidate
                .rafsi
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|rafsi| rafsi.availability == RafsiAvailability::Free))
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn candidate_passes_filters(candidate: &GimfihiCandidate, request: &GimfihiRequest) -> bool {
    (request.show_collisions || candidate.collision.is_none())
        && (!request.require_free_short_rafsi
            || candidate
                .rafsi()
                .iter()
                .any(|rafsi| rafsi.availability == RafsiAvailability::Free))
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn compare_candidates(left: &GimfihiCandidate, right: &GimfihiCandidate) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| left.word.cmp(&right.word))
}

#[requires(true)]
#[ensures(true)]
fn compare_scored_candidates(
    left: &ScoredGimfihiCandidate,
    right: &ScoredGimfihiCandidate,
) -> std::cmp::Ordering {
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
    let stripped: String = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    // A bracketed `[IPA]` word is transliterated to Lojban later (in
    // `resolve_sources`); leave its contents case-intact here because IPA is
    // case-sensitive. Plain Lojban letters are lowercased as before.
    if stripped.starts_with('[') {
        stripped
    } else {
        stripped.to_ascii_lowercase()
    }
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

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn compose_gismu_eager_for_test(
        dictionary: &Dictionary<'_>,
        request: &GimfihiRequest,
    ) -> Result<GimfihiOutput, GimfihiError> {
        let resolved_sources = resolve_sources(request.preset, &request.sources)?;
        if request.shapes.is_empty() {
            return Err(GimfihiError::NoShapes);
        }
        let shapes = unique_shapes(&request.shapes);
        let collision_index = CollisionIndex::from_dictionary(dictionary, request.check_collisions);
        let candidates = generate_candidates(&resolved_sources, &shapes, request.all_letters)
            .into_iter()
            .filter(|word| valid_gismu_candidate(word))
            .map(|word| {
                score_candidate_eager_for_test(
                    dictionary,
                    &collision_index,
                    &resolved_sources,
                    word,
                    request.require_free_short_rafsi,
                )
            })
            .collect::<Vec<_>>();
        let candidate_count = candidates.len();
        let mut filtered = candidates
            .into_iter()
            .filter(|candidate| candidate_passes_filters(candidate, request))
            .collect::<Vec<_>>();
        filtered.sort_by(compare_candidates);
        let filtered_count = filtered.len();
        let winner = filtered
            .iter()
            .find(|candidate| candidate.collision.is_none())
            .map(|candidate| candidate.word.clone());
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
            .take(request.count.min(GIMFIHI_MAX_COUNT))
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
            if candidate.rafsi.is_none() {
                candidate.rafsi = Some(possible_short_rafsis(dictionary, &candidate.word));
            }
        }
        Ok(GimfihiOutput {
            resolved_sources,
            candidate_count,
            filtered_count,
            winner,
            highlighted_word,
            candidates: displayed,
        })
    }

    #[requires(!word.is_empty())]
    #[requires(!sources.is_empty())]
    #[ensures(!ret.word.is_empty())]
    fn score_candidate_eager_for_test(
        dictionary: &Dictionary<'_>,
        collision_index: &CollisionIndex,
        sources: &[ResolvedSource],
        word: String,
        include_rafsi: bool,
    ) -> GimfihiCandidate {
        let source_scores = sources
            .iter()
            .map(|source| score_source_eager_for_test(word.as_str(), source))
            .collect::<Vec<_>>();
        let score = source_scores
            .iter()
            .map(|source_score| source_score.weighted_score)
            .sum::<f64>();
        let collision = collision_index.find(&word);
        let rafsi = include_rafsi.then(|| possible_short_rafsis(dictionary, &word));
        GimfihiCandidate {
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
    fn score_source_eager_for_test(candidate: &str, source: &ResolvedSource) -> SourceScore {
        let raw_score = gismu_match_score(candidate, &source.word);
        let scaled_weight = f64::from(source.weight) / GIMFIHI_WEIGHT_SCALE;
        let weighted_score = raw_score as f64 / source.word.chars().count() as f64 * scaled_weight;
        SourceScore {
            language: source.language.clone(),
            word: source.word.clone(),
            weight: source.weight,
            raw_score,
            weighted_score,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn find_collision(
        dictionary: &Dictionary<'_>,
        scope: CollisionScope,
        candidate: &str,
    ) -> Option<GismuCollision> {
        CollisionIndex::from_dictionary(dictionary, scope).find(candidate)
    }

    #[requires(true)]
    #[ensures(true)]
    fn find_collision_by_scan(
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

        let source = parse_source_spec("eng:160:ekspekt").expect("source");
        assert_eq!(source.explicit_weight, Some(160));
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
        let resolved = resolve_sources(Some(GimfihiPreset::Data1995), &sources).expect("resolved");
        assert_eq!(
            resolved
                .iter()
                .map(|source| source.language.as_str())
                .collect::<Vec<_>>(),
            ["cmn", "hin", "eng", "spa", "rus", "ara"]
        );
        assert_eq!(resolved[2].weight, 160);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_wrong_preset_language_set() {
        let sources = [
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
        ];
        let error = resolve_sources(Some(GimfihiPreset::Data1995), &sources)
            .expect_err("missing preset language");
        assert!(matches!(error, GimfihiError::MissingPresetLanguage { .. }));

        let sources = [
            parse_source_spec("eng::ekspekt").expect("source"),
            parse_source_spec("hin::rakan").expect("source"),
            parse_source_spec("cmn::uan").expect("source"),
            parse_source_spec("spa::esper").expect("source"),
            parse_source_spec("rus::predpologa").expect("source"),
            parse_source_spec("ara::mulud").expect("source"),
            parse_source_spec("fra::esper").expect("source"),
        ];
        let error = resolve_sources(Some(GimfihiPreset::Data1995), &sources)
            .expect_err("extra preset language");
        assert!(matches!(error, GimfihiError::ExtraPresetLanguage { .. }));
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
    fn indexed_collisions_match_dictionary_scan() {
        let dictionary = jbotci_dictionary_data::english();
        for scope in [CollisionScope::Official, CollisionScope::All] {
            let index = CollisionIndex::from_dictionary(dictionary, scope);
            for candidate in [
                "klama", "klami", "plama", "trado", "ctado", "nanpe", "zzzzz",
            ] {
                assert_eq!(
                    index.find(candidate),
                    find_collision_by_scan(dictionary, scope, candidate),
                    "{scope:?} {candidate}"
                );
            }
        }
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
        let request = GimfihiRequest {
            preset: Some(GimfihiPreset::Data1995),
            sources: sources.to_vec(),
            shapes: default_shapes(),
            all_letters: false,
            check_collisions: CollisionScope::None,
            show_collisions: false,
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compose_high_inventory_request_uses_indexed_collisions() {
        let dictionary = jbotci_dictionary_data::english();
        let sources = [
            parse_source_spec("eng::tradicon").expect("source"),
            parse_source_spec("cmn::tcuanton").expect("source"),
            parse_source_spec("hin::parampara").expect("source"),
            parse_source_spec("spa::tradision").expect("source"),
            parse_source_spec("ara::taklid").expect("source"),
            parse_source_spec("fra::tradision").expect("source"),
        ];
        let request = GimfihiRequest {
            preset: Some(GimfihiPreset::Ilmen6),
            sources: sources.to_vec(),
            shapes: default_shapes(),
            all_letters: false,
            check_collisions: CollisionScope::All,
            show_collisions: false,
            require_free_short_rafsi: false,
            count: 20,
            highlight: None,
        };

        let output = compose_gismu(dictionary, &request).expect("output");
        assert_eq!(output.candidate_count, 16_320);
        assert_eq!(output.filtered_count, 12_222);
        assert_eq!(output.winner.as_deref(), Some("trado"));
        assert_eq!(
            output
                .candidates
                .first()
                .map(|candidate| candidate.word.as_str()),
            Some("trado")
        );

        let request = GimfihiRequest {
            show_collisions: true,
            ..request
        };
        let output = compose_gismu(dictionary, &request).expect("output");
        assert_eq!(output.filtered_count, output.candidate_count);
        assert!(
            output
                .candidates
                .iter()
                .any(|candidate| candidate.collision.is_some())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compose_lazy_details_match_eager_materialization() {
        let dictionary = jbotci_dictionary_data::english();
        let highlighted_sources = [parse_source_spec("eng:100:plini").expect("source")];
        let highlighted_request = GimfihiRequest {
            preset: None,
            sources: highlighted_sources.to_vec(),
            shapes: vec![GismuShape::Ccvcv],
            all_letters: false,
            check_collisions: CollisionScope::None,
            show_collisions: false,
            require_free_short_rafsi: false,
            count: 0,
            highlight: Some("plini".to_owned()),
        };
        let lazy_highlighted =
            compose_gismu(dictionary, &highlighted_request).expect("lazy highlighted output");
        assert_eq!(lazy_highlighted.highlighted_word.as_deref(), Some("plini"));
        assert!(lazy_highlighted.candidates.len() > highlighted_request.count);
        assert_eq!(
            lazy_highlighted,
            compose_gismu_eager_for_test(dictionary, &highlighted_request)
                .expect("eager highlighted output")
        );

        let compact_collision_sources = [parse_source_spec("eng:100:klama").expect("source")];
        let compact_collision_request = GimfihiRequest {
            preset: None,
            sources: compact_collision_sources.to_vec(),
            shapes: vec![GismuShape::Ccvcv],
            all_letters: false,
            check_collisions: CollisionScope::All,
            show_collisions: true,
            require_free_short_rafsi: false,
            count: 3,
            highlight: Some("klama".to_owned()),
        };
        let lazy_compact_collision = compose_gismu(dictionary, &compact_collision_request)
            .expect("lazy compact collision output");
        assert!(
            lazy_compact_collision
                .candidates
                .iter()
                .any(|candidate| candidate.collision.is_some())
        );
        assert!(
            lazy_compact_collision
                .candidates
                .iter()
                .any(|candidate| candidate.rafsi.is_some())
        );
        assert_eq!(
            lazy_compact_collision,
            compose_gismu_eager_for_test(dictionary, &compact_collision_request)
                .expect("eager compact collision output")
        );
    }
}
