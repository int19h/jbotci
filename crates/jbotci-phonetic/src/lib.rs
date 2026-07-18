//! IPA rendering and ALINE-style sound similarity.

use std::cmp::Ordering;
use std::sync::LazyLock;

#[allow(unused_imports)]
use bityzba::expensive_invariant;
use bityzba::{data, invariant, new, requires};
use jbotci_morphology::{
    LeadingPauseContext, LeadingPauseVowelMode, Phonemes, Word, WordKind, WordLike, WordLikeData,
    pronunciation_syllables, segment_words_with_modifiers, word_needs_leading_pause_in_context,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Morphology { .. } => true)]
#[invariant(::NoPronounceableWords { .. } => true)]
#[invariant(::UnsupportedSegment { .. } => true)]
#[invariant(::EmptyQuery => true)]
#[invariant(::EmptyBracketedIpa => true)]
#[invariant(::NestedBrackets => true)]
#[invariant(::MissingClosingBracket => true)]
#[invariant(::MissingOpeningBracket => true)]
#[invariant(::PartialBracketedQuery => true)]
#[invariant(::Syllabification { .. } => true)]
pub enum PhoneticError {
    #[error("{message}")]
    Morphology { message: String },
    #[error("no pronounceable words in `{input}`")]
    NoPronounceableWords { input: String },
    #[error("Unsupported IPA segment near `{near}` for ALINE sound search.")]
    UnsupportedSegment { near: String },
    #[error("Sound search requires at least one IPA segment.")]
    EmptyQuery,
    #[error("Bracketed IPA input must not be empty.")]
    EmptyBracketedIpa,
    #[error("IPA input must use one pair of brackets around the whole query.")]
    NestedBrackets,
    #[error("IPA input starts with `[` but does not end with `]`.")]
    MissingClosingBracket,
    #[error("IPA input ends with `]` but does not start with `[`.")]
    MissingOpeningBracket,
    #[error("Use `[ ... ]` around the whole IPA query.")]
    PartialBracketedQuery,
    #[error("{message}")]
    Syllabification { message: String },
}

#[invariant(::SourceSide => true)]
#[invariant(::CandidateSide => true)]
#[invariant(::Symmetric => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlineNormalizer {
    SourceSide,
    CandidateSide,
    Symmetric,
}

impl Default for AlineNormalizer {
    #[requires(true)]
    #[ensures(ret == AlineNormalizer::SourceSide)]
    fn default() -> Self {
        Self::SourceSide
    }
}

#[invariant(::Syllabic => true)]
#[invariant(::Place => true)]
#[invariant(::Manner => true)]
#[invariant(::Voice => true)]
#[invariant(::Nasal => true)]
#[invariant(::Retroflex => true)]
#[invariant(::Lateral => true)]
#[invariant(::Aspirated => true)]
#[invariant(::High => true)]
#[invariant(::Back => true)]
#[invariant(::Round => true)]
#[invariant(::Long => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlineFeature {
    Syllabic,
    Place,
    Manner,
    Voice,
    Nasal,
    Retroflex,
    Lateral,
    Aspirated,
    High,
    Back,
    Round,
    Long,
}

impl AlineFeature {
    #[requires(true)]
    #[ensures(ret.len() == 12)]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Syllabic,
            Self::Place,
            Self::Manner,
            Self::Voice,
            Self::Nasal,
            Self::Retroflex,
            Self::Lateral,
            Self::Aspirated,
            Self::High,
            Self::Back,
            Self::Round,
            Self::Long,
        ]
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Syllabic => "syllabic",
            Self::Place => "place",
            Self::Manner => "manner",
            Self::Voice => "voice",
            Self::Nasal => "nasal",
            Self::Retroflex => "retroflex",
            Self::Lateral => "lateral",
            Self::Aspirated => "aspirated",
            Self::High => "high",
            Self::Back => "back",
            Self::Round => "round",
            Self::Long => "long",
        }
    }
}

#[invariant(
    syllabic.is_finite() && *syllabic >= 0.0
        && place.is_finite() && *place >= 0.0
        && manner.is_finite() && *manner >= 0.0
        && voice.is_finite() && *voice >= 0.0
        && nasal.is_finite() && *nasal >= 0.0
        && retroflex.is_finite() && *retroflex >= 0.0
        && lateral.is_finite() && *lateral >= 0.0
        && aspirated.is_finite() && *aspirated >= 0.0
        && high.is_finite() && *high >= 0.0
        && back.is_finite() && *back >= 0.0
        && round.is_finite() && *round >= 0.0
        && long.is_finite() && *long >= 0.0
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AlineSaliences {
    pub syllabic: f64,
    pub place: f64,
    pub manner: f64,
    pub voice: f64,
    pub nasal: f64,
    pub retroflex: f64,
    pub lateral: f64,
    pub aspirated: f64,
    pub high: f64,
    pub back: f64,
    pub round: f64,
    pub long: f64,
}

impl Default for AlineSaliences {
    #[requires(true)]
    #[ensures(ret.value(AlineFeature::Manner) == 50.0)]
    fn default() -> Self {
        new!(AlineSaliences {
            syllabic: 5.0,
            place: 40.0,
            manner: 50.0,
            voice: 10.0,
            nasal: 10.0,
            retroflex: 10.0,
            lateral: 10.0,
            aspirated: 5.0,
            high: 5.0,
            back: 5.0,
            round: 5.0,
            long: 1.0,
        })
    }
}

impl AlineSaliences {
    #[requires(true)]
    #[ensures(ret.is_finite() && ret >= 0.0)]
    pub fn value(&self, feature: AlineFeature) -> f64 {
        match feature {
            AlineFeature::Syllabic => self.syllabic,
            AlineFeature::Place => self.place,
            AlineFeature::Manner => self.manner,
            AlineFeature::Voice => self.voice,
            AlineFeature::Nasal => self.nasal,
            AlineFeature::Retroflex => self.retroflex,
            AlineFeature::Lateral => self.lateral,
            AlineFeature::Aspirated => self.aspirated,
            AlineFeature::High => self.high,
            AlineFeature::Back => self.back,
            AlineFeature::Round => self.round,
            AlineFeature::Long => self.long,
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|saliences| saliences.value(feature) == value) || ret.is_err())]
    pub fn with_feature(
        self,
        feature: AlineFeature,
        value: f64,
    ) -> Result<Self, AlineParameterError> {
        validate_nonnegative_finite(feature.as_str(), value)?;
        Ok(match feature {
            AlineFeature::Syllabic => self.with_data(data! { syllabic: value }),
            AlineFeature::Place => self.with_data(data! { place: value }),
            AlineFeature::Manner => self.with_data(data! { manner: value }),
            AlineFeature::Voice => self.with_data(data! { voice: value }),
            AlineFeature::Nasal => self.with_data(data! { nasal: value }),
            AlineFeature::Retroflex => self.with_data(data! { retroflex: value }),
            AlineFeature::Lateral => self.with_data(data! { lateral: value }),
            AlineFeature::Aspirated => self.with_data(data! { aspirated: value }),
            AlineFeature::High => self.with_data(data! { high: value }),
            AlineFeature::Back => self.with_data(data! { back: value }),
            AlineFeature::Round => self.with_data(data! { round: value }),
            AlineFeature::Long => self.with_data(data! { long: value }),
        })
    }
}

#[invariant(c_sub.is_finite() && *c_sub > 2.0 * *c_vwl)]
#[invariant(c_exp.is_finite())]
#[invariant(c_skip.is_finite() && *c_skip <= 0.0)]
#[invariant(c_vwl.is_finite() && *c_vwl >= 0.0)]
#[invariant(c_flank.is_finite() && *c_flank >= *c_skip && *c_flank <= 0.0)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AlineParameters {
    pub saliences: AlineSaliences,
    pub c_sub: f64,
    pub c_exp: f64,
    pub c_skip: f64,
    pub c_vwl: f64,
    pub c_flank: f64,
    pub normalizer: AlineNormalizer,
}

impl Default for AlineParameters {
    #[requires(true)]
    #[ensures(ret.normalizer == AlineNormalizer::SourceSide)]
    fn default() -> Self {
        new!(AlineParameters {
            saliences: AlineSaliences::default(),
            c_sub: ALINE_SUBSTITUTION_CEILING,
            c_exp: ALINE_EXPANSION_CEILING,
            c_skip: ALINE_SKIP_SCORE,
            c_vwl: ALINE_VOWEL_PENALTY,
            c_flank: 0.0,
            normalizer: AlineNormalizer::SourceSide,
        })
    }
}

impl AlineParameters {
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|parameters| parameters.c_sub == c_sub) || ret.is_err())]
    pub fn try_new(
        saliences: AlineSaliences,
        c_sub: f64,
        c_exp: f64,
        c_skip: f64,
        c_vwl: f64,
        c_flank: f64,
        normalizer: AlineNormalizer,
    ) -> Result<Self, AlineParameterError> {
        validate_finite("c-sub", c_sub)?;
        validate_finite("c-exp", c_exp)?;
        validate_finite("c-skip", c_skip)?;
        validate_finite("c-vwl", c_vwl)?;
        validate_finite("c-flank", c_flank)?;
        if c_vwl < 0.0 {
            return Err(invalid_parameter("c-vwl", "must be nonnegative"));
        }
        if c_sub <= 2.0 * c_vwl {
            return Err(invalid_parameter(
                "c-sub",
                "must be greater than twice c-vwl so identity normalization is positive",
            ));
        }
        if c_skip > 0.0 {
            return Err(invalid_parameter("c-skip", "must be nonpositive"));
        }
        if c_flank < c_skip || c_flank > 0.0 {
            return Err(invalid_parameter(
                "c-flank",
                "must lie between c-skip and 0",
            ));
        }
        Ok(new!(AlineParameters {
            saliences,
            c_sub,
            c_exp,
            c_skip,
            c_vwl,
            c_flank,
            normalizer,
        }))
    }
}

#[invariant(pair_feature_differences.len() == IPA_SEGMENT_SYMBOLS.len() * IPA_SEGMENT_SYMBOLS.len())]
#[invariant(vowel_penalties.len() == IPA_SEGMENT_SYMBOLS.len())]
#[derive(Debug, Clone, PartialEq)]
pub struct AlineScorer {
    parameters: AlineParameters,
    pair_feature_differences: Vec<f64>,
    vowel_penalties: Vec<f64>,
}

impl AlineScorer {
    #[requires(true)]
    #[ensures(ret.parameters == parameters)]
    pub fn new(parameters: AlineParameters) -> Self {
        let segment_count = IPA_SEGMENT_SYMBOLS.len();
        let mut pair_feature_differences = Vec::with_capacity(segment_count * segment_count);
        for left in 0..segment_count {
            for right in 0..segment_count {
                pair_feature_differences.push(parameterized_feature_difference(
                    IpaSegmentId::from_static_index(left as u16),
                    IpaSegmentId::from_static_index(right as u16),
                    &parameters.saliences,
                ));
            }
        }
        let vowel_penalties = IPA_SEGMENT_FEATURES
            .iter()
            .map(|features| {
                if features.is_consonant {
                    0.0
                } else {
                    parameters.c_vwl
                }
            })
            .collect();
        new!(AlineScorer {
            parameters: parameters.clone(),
            pair_feature_differences,
            vowel_penalties,
        })
    }

    #[requires(true)]
    #[ensures(ret == &self.parameters)]
    pub fn parameters(&self) -> &AlineParameters {
        &self.parameters
    }

    #[requires(!candidate.is_empty())]
    #[requires(!source.is_empty())]
    #[ensures(ret.is_finite())]
    pub fn raw_similarity_with_scratch(
        &self,
        candidate: &[IpaSegmentId],
        source: &[IpaSegmentId],
        scratch: &mut AlineSimilarityScratch,
    ) -> f64 {
        semiglobal_raw_similarity_with_scratch(candidate, source, self, scratch)
    }

    #[requires(!sequence.is_empty())]
    #[ensures(ret.is_finite() && ret > 0.0)]
    pub fn self_similarity_with_scratch(
        &self,
        sequence: &[IpaSegmentId],
        scratch: &mut AlineSimilarityScratch,
    ) -> f64 {
        self.raw_similarity_with_scratch(sequence, sequence, scratch)
    }

    #[requires(raw.is_finite())]
    #[requires(candidate_self.is_finite() && candidate_self > 0.0)]
    #[requires(source_self.is_finite() && source_self > 0.0)]
    #[ensures((0.0..=1.0).contains(&ret))]
    pub fn normalize(&self, raw: f64, candidate_self: f64, source_self: f64) -> f64 {
        let normalizer = match self.parameters.normalizer {
            AlineNormalizer::SourceSide => source_self,
            AlineNormalizer::CandidateSide => candidate_self,
            AlineNormalizer::Symmetric => (candidate_self + source_self) / 2.0,
        };
        (raw / normalizer).clamp(0.0, 1.0)
    }

    #[requires(true)]
    #[ensures(ret.is_finite() && ret >= 0.0)]
    fn feature_difference(&self, left: IpaSegmentId, right: IpaSegmentId) -> f64 {
        let segment_count = IPA_SEGMENT_SYMBOLS.len();
        self.pair_feature_differences[(left.get() as usize) * segment_count + right.get() as usize]
    }

    #[requires(true)]
    #[ensures(ret == 0.0 || ret == self.parameters.c_vwl)]
    fn vowel_penalty(&self, segment: IpaSegmentId) -> f64 {
        self.vowel_penalties[segment.get() as usize]
    }

    #[requires(true)]
    #[ensures(ret.is_finite())]
    fn substitution_score(&self, left: IpaSegmentId, right: IpaSegmentId) -> f64 {
        self.parameters.c_sub
            - self.feature_difference(left, right)
            - self.vowel_penalty(left)
            - self.vowel_penalty(right)
    }

    #[requires(true)]
    #[ensures(ret.is_finite())]
    fn expansion_score(
        &self,
        single: IpaSegmentId,
        first_second: IpaSegmentId,
        second_second: IpaSegmentId,
    ) -> f64 {
        self.parameters.c_exp
            - self.feature_difference(single, first_second)
            - self.feature_difference(single, second_second)
            - self.vowel_penalty(single)
            - self
                .vowel_penalty(first_second)
                .max(self.vowel_penalty(second_second))
    }
}

#[invariant(::InvalidValue { parameter, reason } => !parameter.is_empty() && !reason.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlineParameterError {
    InvalidValue { parameter: String, reason: String },
}

impl std::fmt::Display for AlineParameterError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data!(AlineParameterError::InvalidValue { parameter, reason }) = self.as_data();
        write!(formatter, "invalid ALINE parameter `{parameter}`: {reason}")
    }
}

impl std::error::Error for AlineParameterError {}

#[requires(!parameter.is_empty())]
#[requires(!reason.is_empty())]
#[ensures(matches!(ret.as_data(), data!(AlineParameterError::InvalidValue { .. })))]
fn invalid_parameter(parameter: &str, reason: &str) -> AlineParameterError {
    new!(AlineParameterError::InvalidValue {
        parameter: parameter.to_owned(),
        reason: reason.to_owned(),
    })
}

#[requires(!parameter.is_empty())]
#[ensures(ret.is_ok() -> value.is_finite())]
fn validate_finite(parameter: &str, value: f64) -> Result<(), AlineParameterError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid_parameter(parameter, "must be finite"))
    }
}

#[requires(!parameter.is_empty())]
#[ensures(ret.is_ok() -> (value.is_finite() && value >= 0.0))]
fn validate_nonnegative_finite(parameter: &str, value: f64) -> Result<(), AlineParameterError> {
    validate_finite(parameter, value)?;
    if value < 0.0 {
        Err(invalid_parameter(parameter, "must be nonnegative"))
    } else {
        Ok(())
    }
}

#[invariant((self.as_data().0 as usize) < IPA_SEGMENT_SYMBOLS.len())]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IpaSegmentId(u16);

impl IpaSegmentId {
    #[requires(true)]
    #[ensures(true)]
    pub const fn from_static_index(index: u16) -> Self {
        assert!(
            (index as usize) < IPA_SEGMENT_SYMBOLS.len(),
            "static IPA segment id must index IPA_SEGMENT_SYMBOLS"
        );
        Self(data!(IpaSegmentId(index)))
    }

    #[requires(true)]
    #[ensures((ret as usize) < IPA_SEGMENT_SYMBOLS.len())]
    pub fn get(self) -> u16 {
        self.as_data().0
    }
}

#[invariant(!segments.is_empty())]
#[invariant(self_similarity.is_finite())]
#[invariant(*self_similarity > 0.0)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IpaTokenSequenceView<'a> {
    pub segments: &'a [IpaSegmentId],
    pub self_similarity: f64,
}

impl<'a> IpaTokenSequenceView<'a> {
    #[requires(!segments.is_empty())]
    #[requires(self_similarity.is_finite())]
    #[requires(self_similarity > 0.0)]
    #[ensures(ret.segment_count() == segments.len())]
    pub fn new(segments: &'a [IpaSegmentId], self_similarity: f64) -> Self {
        new!(IpaTokenSequenceView {
            segments,
            self_similarity,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn from_static_parts(segments: &'a [IpaSegmentId], self_similarity: f64) -> Self {
        assert!(
            !segments.is_empty(),
            "static IPA token sequence must contain at least one segment"
        );
        assert!(
            self_similarity.is_finite(),
            "static IPA token sequence self-similarity must be finite"
        );
        assert!(
            self_similarity > 0.0,
            "static IPA token sequence self-similarity must be positive"
        );
        Self(data!(IpaTokenSequenceView {
            segments,
            self_similarity,
        }))
    }

    #[requires(true)]
    #[ensures(ret == self.segments.len())]
    pub fn segment_count(self) -> usize {
        self.segments.len()
    }
}

#[invariant(!segments.is_empty())]
#[invariant(self_similarity.is_finite())]
#[invariant(*self_similarity > 0.0)]
#[expensive_invariant(*self_similarity == aline_raw_similarity(segments, segments))]
#[derive(Debug, Clone, PartialEq)]
pub struct IpaTokenSequence {
    segments: Vec<IpaSegmentId>,
    self_similarity: f64,
}

impl IpaTokenSequence {
    #[requires(true)]
    #[ensures(ret == self.segments.len())]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    #[requires(true)]
    #[ensures(ret.len() == self.segments.len())]
    pub fn segments(&self) -> &[IpaSegmentId] {
        &self.segments
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn self_similarity(&self) -> f64 {
        self.self_similarity
    }

    #[requires(true)]
    #[ensures(ret.segment_count() == self.segment_count())]
    pub fn view(&self) -> IpaTokenSequenceView<'_> {
        IpaTokenSequenceView::new(&self.segments, self.self_similarity)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct IpaTokenizedText {
    pub ipa: String,
    pub token_sequence: IpaTokenSequence,
}

#[derive(Debug, Default, Clone, PartialEq)]
#[invariant(true)]
pub struct AlineSimilarityScratch {
    previous_previous: Vec<f64>,
    previous: Vec<f64>,
    current: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct AlineFeatures {
    is_consonant: bool,
    syllabic_value: f64,
    place_value: f64,
    manner_value: f64,
    voice_value: f64,
    nasal_value: f64,
    retroflex_value: f64,
    lateral_value: f64,
    aspirated_value: f64,
    high_value: f64,
    back_value: f64,
    round_value: f64,
    long_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Word { .. } => true)]
#[invariant(::Text(_) => true)]
enum IpaSurfaceChunk<'word> {
    Word {
        word: &'word Word,
        leading_pause_context: LeadingPauseContext,
    },
    Text(&'word str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct IpaRenderedWord {
    body: String,
    leading_pause_required: bool,
    trailing_pause_required: bool,
}

const ALINE_SKIP_SCORE: f64 = -10.0;
const ALINE_SUBSTITUTION_CEILING: f64 = 35.0;
const ALINE_EXPANSION_CEILING: f64 = 45.0;
const ALINE_VOWEL_PENALTY: f64 = 10.0;

const BILABIAL_SYMBOLS: &[&str] = &["p", "b", "m", "ʙ", "β", "ɸ"];
// The mixed bilabial-labiodental affricate pf follows its labiodental release on
// ALINE's single-valued place scale.
const LABIODENTAL_SYMBOLS: &[&str] = &["f", "v", "ʋ", "ɱ", "pf"];
const DENTAL_SYMBOLS: &[&str] = &["θ", "ð"];
// Kondrak's feature inventory has no velarization feature, so dark ɫ is
// intentionally identical to alveolar l rather than receiving an ad hoc value.
const ALVEOLAR_SYMBOLS: &[&str] = &["t", "d", "n", "s", "z", "r", "l", "ɫ", "ɹ", "ɾ", "ɬ", "ɮ"];
// Retroflex affricates use ALINE place 0.8 and the retroflex flag. The same flag
// is the closest ALINE analogue for both retroflex lateral ɭ and r-coloured
// vowels ɚ/ɝ, since the model has no separate vowel-rhoticity feature.
const RETROFLEX_SYMBOLS: &[&str] = &["ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɻ", "ɽ", "ʈʂ", "ɖʐ", "ɭ", "ɚ", "ɝ"];
const PALATO_ALVEOLAR_SYMBOLS: &[&str] = &["ʃ", "ʒ"];
// Alveolo-palatals receive place 0.725, the midpoint between ALINE's adjacent
// palato-alveolar (0.75) and palatal (0.70) positions.
const ALVEOLO_PALATAL_SYMBOLS: &[&str] = &["ɕ", "ʑ", "tɕ", "dʑ"];
const PALATAL_SYMBOLS: &[&str] = &["j", "c", "ɟ", "ɲ", "ç", "ʝ", "ɥ", "ʎ"];
const VELAR_SYMBOLS: &[&str] = &["k", "g", "x", "ɣ", "ŋ", "w", "ʍ", "ɰ"];
const UVULAR_SYMBOLS: &[&str] = &["q", "ɢ", "χ", "ʁ", "ʀ", "ɴ"];
const PHARYNGEAL_SYMBOLS: &[&str] = &["ħ", "ʕ"];
const GLOTTAL_SYMBOLS: &[&str] = &["h", "ɦ", "ʔ"];
const AFFRICATE_SYMBOLS: &[&str] = &[
    "t͡ʃ", "d͡ʒ", "tʃ", "dʒ", "ts", "dz", "tɕ", "dʑ", "ʈʂ", "ɖʐ", "pf",
];
const TRILL_SYMBOLS: &[&str] = &["r", "ʀ", "ʙ"];
const TAP_SYMBOLS: &[&str] = &["ɾ", "ɽ"];
// ɥ shares j's palatal approximant features plus rounding; ʍ shares w's velar
// rounded approximant features but is deliberately absent from the voiced list.
const APPROXIMANT_SYMBOLS: &[&str] = &["j", "w", "ɥ", "ʍ", "ʋ", "ɹ", "ɻ", "ɰ"];
const FRICATIVE_SYMBOLS: &[&str] = &[
    "ɸ", "β", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "ɕ", "ʑ", "ʂ", "ʐ", "ç", "ʝ", "x", "ɣ", "χ",
    "ʁ", "ħ", "ʕ", "h", "ɦ", "ɬ", "ɮ",
];
const VOICED_CONSONANT_SYMBOLS: &[&str] = &[
    "b", "d", "ɖ", "ɟ", "g", "ɢ", "m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ", "ʙ", "r", "ʀ", "ɾ", "ɽ", "β",
    "v", "ð", "z", "ʒ", "ʑ", "ʐ", "ʝ", "ɣ", "ʁ", "ʕ", "ɦ", "ɮ", "ʋ", "ɹ", "ɻ", "ɰ", "j", "w", "ɥ",
    "l", "ɫ", "ɭ", "ʎ", "d͡ʒ", "dʒ", "dz", "dʑ", "ɖʐ",
];
const NASAL_SYMBOLS: &[&str] = &["m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ"];
// ɭ is both retroflex and lateral; palatal ʎ and velarized ɫ remain laterals.
const LATERAL_SYMBOLS: &[&str] = &["l", "ɫ", "ɭ", "ʎ", "ɬ", "ɮ"];
// ALINE has no tenseness or near-high feature, so lax ɪ/ʊ/ʏ intentionally use
// the same high/front/back/round values as their close counterparts i/u/y.
const HIGH_VOWEL_SYMBOLS: &[&str] = &["i", "y", "ɨ", "ʉ", "ɯ", "u", "ɪ", "ʊ", "ʏ"];
const MID_VOWEL_SYMBOLS: &[&str] = &[
    "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ", "ɜ", "ɞ", "ʌ", "ɔ", "ɚ", "ɝ",
];
const FRONT_VOWEL_SYMBOLS: &[&str] = &["i", "y", "ɪ", "ʏ", "e", "ø", "ɛ", "œ", "æ", "a", "ɶ"];
// Mid-central ɚ/ɝ share ə's height and backness; their r-colouring is carried by
// RETROFLEX_SYMBOLS because ALINE has no vowel-rhoticity feature.
const CENTRAL_VOWEL_SYMBOLS: &[&str] = &["ɨ", "ʉ", "ɘ", "ɵ", "ə", "ɜ", "ɞ", "ɐ", "ä", "ɚ", "ɝ"];
// Round is compared only for vowels by ALINE, but keeping it accurate on ɥ, w,
// and ʍ makes the derived feature vectors faithful to the rounded glides.
const ROUNDED_SYMBOLS: &[&str] = &[
    "y", "ʏ", "ʉ", "u", "ʊ", "ø", "ɵ", "o", "œ", "ɞ", "ɔ", "ɶ", "ɒ", "ɥ", "w", "ʍ",
];

// This is a real-IPA inventory. The historical Kondrak ASCII stand-ins
// I/U/E/O/N/R/B had no producer outside this table and duplicated IPA entries.
const IPA_SEGMENT_SYMBOLS: &[&str] = &[
    "p", "b", "m", "ʙ", "β", "ɸ", "f", "v", "ʋ", "ɱ", "θ", "ð", "t", "d", "n", "s", "z", "r", "l",
    "ɹ", "ɾ", "ɬ", "ɮ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɻ", "ɽ", "ʃ", "ʒ", "j", "c", "ɟ", "ɲ", "ç", "ʝ",
    "k", "g", "x", "ɣ", "ŋ", "w", "ɰ", "q", "ɢ", "χ", "ʁ", "ʀ", "ɴ", "ħ", "ʕ", "h", "ɦ", "ʔ", "t͡ʃ",
    "d͡ʒ", "tʃ", "dʒ", "ts", "dz", "i", "y", "ɨ", "ʉ", "ɯ", "u", "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə",
    "ɛ", "œ", "ɜ", "ɞ", "ʌ", "ɔ", "æ", "ɐ", "a", "ɶ", "ä", "ɑ", "ɒ", "iː", "yː", "ɨː", "ʉː", "ɯː",
    "uː", "eː", "øː", "ɘː", "ɵː", "ɤː", "oː", "əː", "ɛː", "œː", "ɜː", "ɞː", "ʌː", "ɔː", "æː", "ɐː",
    "aː", "ɶː", "äː", "ɑː", "ɒː",
    // Aspiration and breathy voice are contrastively notated on plosives and affricates.
    "pʰ", "bʰ", "tʰ", "dʰ", "ʈʰ", "ɖʰ", "cʰ", "ɟʰ", "kʰ", "gʰ", "qʰ", "ɢʰ", "tʃʰ", "dʒʰ", "tsʰ",
    "dzʰ",
    // Source-language IPA accepted by gimfihi. New entries stay append-only so
    // generated dictionary segment IDs are rebuilt rather than silently reused.
    "ɕ", "ʑ", "tɕ", "dʑ", "ʈʂ", "ɖʐ", "pf", "tɕʰ", "dʑʰ", "ʈʂʰ", "ɖʐʰ", "ɥ", "ʍ", "ɫ", "ɭ", "ʎ",
    "ɪ", "ʊ", "ʏ", "ɚ", "ɝ",
];

const CONSONANT_RELEVANT_FEATURES: &[AlineFeature] = &[
    AlineFeature::Syllabic,
    AlineFeature::Manner,
    AlineFeature::Voice,
    AlineFeature::Nasal,
    AlineFeature::Retroflex,
    AlineFeature::Lateral,
    AlineFeature::Aspirated,
    AlineFeature::Place,
];

const VOWEL_RELEVANT_FEATURES: &[AlineFeature] = &[
    AlineFeature::Syllabic,
    AlineFeature::Nasal,
    AlineFeature::Retroflex,
    AlineFeature::High,
    AlineFeature::Back,
    AlineFeature::Round,
    AlineFeature::Long,
];

static IPA_SEGMENT_FEATURES: LazyLock<Vec<AlineFeatures>> = LazyLock::new(|| {
    IPA_SEGMENT_SYMBOLS
        .iter()
        .map(|symbol| derive_aline_features(symbol))
        .collect()
});

static IPA_SEGMENT_NORMALIZED_SYMBOLS: LazyLock<Vec<String>> = LazyLock::new(|| {
    IPA_SEGMENT_SYMBOLS
        .iter()
        .map(|symbol| normalize_ipa_query(symbol))
        .collect()
});

static ALINE_PAIR_FEATURE_DIFFERENCES: LazyLock<Vec<f64>> = LazyLock::new(|| {
    let segment_count = IPA_SEGMENT_SYMBOLS.len();
    let mut differences = Vec::with_capacity(segment_count * segment_count);
    for left in IPA_SEGMENT_FEATURES.iter().copied() {
        for right in IPA_SEGMENT_FEATURES.iter().copied() {
            differences.push(feature_difference_from_features(left, right));
        }
    }
    differences
});

static ALINE_VOWEL_PENALTIES: LazyLock<Vec<f64>> = LazyLock::new(|| {
    IPA_SEGMENT_FEATURES
        .iter()
        .map(|features| {
            if features.is_consonant {
                0.0
            } else {
                ALINE_VOWEL_PENALTY
            }
        })
        .collect()
});

static LOJBAN_GISMU_LETTER_SEGMENTS: LazyLock<[Option<IpaSegmentId>; 128]> = LazyLock::new(|| {
    let mut segments = [None; 128];
    for letter in "bcdfgjklmnprstvxzaeiou".chars() {
        let index = IPA_SEGMENT_SYMBOLS
            .iter()
            .position(|symbol| match letter {
                'c' => *symbol == "ʃ",
                'j' => *symbol == "ʒ",
                _ => symbol.chars().eq([letter]),
            })
            .expect("every Lojban gismu letter has an IPA segment");
        segments[letter as usize] = Some(IpaSegmentId::from_static_index(index as u16));
    }
    segments
});

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn sound_query_to_token_sequence(raw_query: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let ipa = sound_query_to_ipa(raw_query)?;
    tokenize_ipa_text(&ipa)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.trim().is_empty()) || ret.is_err())]
pub fn sound_query_to_ipa(raw_query: &str) -> Result<String, PhoneticError> {
    match bracketed_ipa_query(raw_query)? {
        Some(ipa) => Ok(ipa),
        None => lojban_text_to_ipa(raw_query),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.trim().is_empty()) || ret.is_err())]
pub fn lojban_text_to_ipa(raw_text: &str) -> Result<String, PhoneticError> {
    let words =
        segment_words_with_modifiers(raw_text).map_err(|error| PhoneticError::Morphology {
            message: error.to_string(),
        })?;
    ipa_morphology_text(&words, raw_text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.ipa.trim().is_empty()) || ret.is_err())]
pub fn lojban_text_to_tokenized_ipa(raw_text: &str) -> Result<IpaTokenizedText, PhoneticError> {
    let ipa = lojban_text_to_ipa(raw_text)?;
    let token_sequence = tokenize_ipa_text(&ipa)?;
    Ok(IpaTokenizedText {
        ipa,
        token_sequence,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn ipa_morphology_text(words: &[WordLike], source: &str) -> Result<String, PhoneticError> {
    let chunks = words
        .iter()
        .flat_map(flatten_word_like_ipa)
        .collect::<Vec<_>>();
    if chunks.is_empty() {
        return Err(PhoneticError::NoPronounceableWords {
            input: source.to_owned(),
        });
    }
    render_ipa_surface_chunks(&chunks, source)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn tokenize_ipa_text(text: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let mut segments = Vec::new();
    let normalized = normalize_ipa_query(text);
    let mut remaining = normalized.trim();
    while !remaining.is_empty() {
        let Some(first) = remaining.chars().next() else {
            break;
        };
        if is_ipa_boundary(first) {
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        if is_ipa_ignored_modifier(first) {
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        let Some((segment_id, segment_length)) = match_longest_segment(remaining) else {
            return Err(PhoneticError::UnsupportedSegment {
                near: remaining.chars().take(12).collect::<String>(),
            });
        };
        segments.push(segment_id);
        remaining = &remaining[segment_length..];
        while let Some(modifier) = remaining.chars().next() {
            if is_ipa_ignored_modifier(modifier) {
                remaining = &remaining[modifier.len_utf8()..];
            } else {
                break;
            }
        }
    }
    if segments.is_empty() {
        Err(PhoneticError::EmptyQuery)
    } else {
        Ok(make_token_sequence(segments))
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_ipa_query(text: &str) -> String {
    text.nfd()
        .filter_map(|value| match value {
            '\u{0261}' => Some('g'),
            value if is_ipa_tie_bar(value) => None,
            value => Some(value),
        })
        .collect()
}

#[requires(true)]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_phonetic_similarity(
    source: IpaTokenSequenceView<'_>,
    target: IpaTokenSequenceView<'_>,
) -> f64 {
    let mut scratch = AlineSimilarityScratch::default();
    aline_phonetic_similarity_with_scratch(source, target, &mut scratch)
}

#[requires(true)]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_phonetic_similarity_with_scratch(
    source: IpaTokenSequenceView<'_>,
    target: IpaTokenSequenceView<'_>,
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let raw_similarity =
        aline_raw_similarity_with_scratch(source.segments, target.segments, scratch);
    let normalizer = source.self_similarity + target.self_similarity;
    (2.0 * raw_similarity / normalizer).clamp(0.0, 1.0)
}

/// Score an alignment in which the candidate is consumed in full while source
/// prefixes and suffixes use the configured flank rate.
#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures(ret.is_finite())]
pub fn aline_semiglobal_raw_similarity(
    candidate: &[IpaSegmentId],
    source: &[IpaSegmentId],
    parameters: &AlineParameters,
) -> f64 {
    let scorer = AlineScorer::new(parameters.clone());
    let mut scratch = AlineSimilarityScratch::default();
    scorer.raw_similarity_with_scratch(candidate, source, &mut scratch)
}

/// Scratch-reusing form of [`aline_semiglobal_raw_similarity`].
#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures(ret.is_finite())]
pub fn aline_semiglobal_raw_similarity_with_scratch(
    candidate: &[IpaSegmentId],
    source: &[IpaSegmentId],
    parameters: &AlineParameters,
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    AlineScorer::new(parameters.clone()).raw_similarity_with_scratch(candidate, source, scratch)
}

/// Return the semi-global score normalized according to `parameters`.
#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_semiglobal_similarity(
    candidate: &[IpaSegmentId],
    source: &[IpaSegmentId],
    parameters: &AlineParameters,
) -> f64 {
    let scorer = AlineScorer::new(parameters.clone());
    let mut scratch = AlineSimilarityScratch::default();
    let raw = scorer.raw_similarity_with_scratch(candidate, source, &mut scratch);
    let candidate_self = scorer.self_similarity_with_scratch(candidate, &mut scratch);
    let source_self = scorer.self_similarity_with_scratch(source, &mut scratch);
    scorer.normalize(raw, candidate_self, source_self)
}

/// Scratch-reusing form of [`aline_semiglobal_similarity`].
#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_semiglobal_similarity_with_scratch(
    candidate: &[IpaSegmentId],
    source: &[IpaSegmentId],
    parameters: &AlineParameters,
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let scorer = AlineScorer::new(parameters.clone());
    let raw = scorer.raw_similarity_with_scratch(candidate, source, scratch);
    let candidate_self = scorer.self_similarity_with_scratch(candidate, scratch);
    let source_self = scorer.self_similarity_with_scratch(source, scratch);
    scorer.normalize(raw, candidate_self, source_self)
}

#[requires(true)]
#[ensures(matches!(ret, Ordering::Less | Ordering::Equal | Ordering::Greater))]
pub fn compare_similarity_then_index(left: (usize, f64), right: (usize, f64)) -> Ordering {
    right
        .1
        .partial_cmp(&left.1)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.0.cmp(&right.0))
}

#[requires(true)]
#[ensures(ret.is_some_and(|symbol| !symbol.is_empty()))]
pub fn ipa_segment_symbol(id: IpaSegmentId) -> Option<&'static str> {
    Some(IPA_SEGMENT_SYMBOLS[id.get() as usize])
}

/// Map one letter from the gismu consonant/vowel inventory to the segment
/// emitted by the deterministic Lojban IPA renderer.
#[requires(true)]
#[ensures(ret.is_some() == matches!(letter, 'b' | 'c' | 'd' | 'f' | 'g' | 'j' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 's' | 't' | 'v' | 'x' | 'z' | 'a' | 'e' | 'i' | 'o' | 'u'))]
pub fn lojban_gismu_letter_to_ipa_segment(letter: char) -> Option<IpaSegmentId> {
    usize::try_from(u32::from(letter))
        .ok()
        .and_then(|index| LOJBAN_GISMU_LETTER_SEGMENTS.get(index))
        .copied()
        .flatten()
}

#[requires(true)]
#[ensures(true)]
fn bracketed_ipa_query(raw_query: &str) -> Result<Option<String>, PhoneticError> {
    let trimmed = raw_query.trim();
    let starts = trimmed.starts_with('[');
    let ends = trimmed.ends_with(']');
    match (starts, ends) {
        (true, true) => {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                Err(PhoneticError::EmptyBracketedIpa)
            } else if inner.contains('[') || inner.contains(']') {
                Err(PhoneticError::NestedBrackets)
            } else {
                Ok(Some(inner.to_owned()))
            }
        }
        (true, false) => Err(PhoneticError::MissingClosingBracket),
        (false, true) => Err(PhoneticError::MissingOpeningBracket),
        (false, false) if trimmed.contains('[') || trimmed.contains(']') => {
            Err(PhoneticError::PartialBracketedQuery)
        }
        (false, false) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_boundary(value: char) -> bool {
    value.is_whitespace()
        || matches!(value, '.' | '/' | '|' | '‖' | 'ˈ' | 'ˌ')
        || ('\u{02E5}'..='\u{02E9}').contains(&value)
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_tie_bar(value: char) -> bool {
    matches!(value, '\u{0361}' | '\u{035C}')
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_ignored_modifier(value: char) -> bool {
    matches!(value, '\u{0300}'..='\u{036F}' | '\u{02B0}'..='\u{02FF}')
        && !is_ipa_boundary(value)
        && !is_ipa_tie_bar(value)
}

#[requires(!remaining.is_empty())]
#[ensures(ret.as_ref().is_some_and(|(_id, length)| *length > 0) || ret.is_none())]
fn match_longest_segment(remaining: &str) -> Option<(IpaSegmentId, usize)> {
    IPA_SEGMENT_NORMALIZED_SYMBOLS
        .iter()
        .enumerate()
        .filter(|(_, symbol)| remaining.starts_with(symbol.as_str()))
        .max_by(|(_, left), (_, right)| left.len().cmp(&right.len()).then_with(|| right.cmp(left)))
        .and_then(|(index, _)| u16::try_from(index).ok())
        .map(|index| {
            let id = new!(IpaSegmentId(index));
            let normalized_symbol = &IPA_SEGMENT_NORMALIZED_SYMBOLS[id.get() as usize];
            (id, normalized_symbol.len())
        })
}

#[requires(!segments.is_empty())]
#[ensures(ret.segment_count() > 0)]
fn make_token_sequence(segments: Vec<IpaSegmentId>) -> IpaTokenSequence {
    let self_similarity = aline_raw_similarity(&segments, &segments);
    new!(IpaTokenSequence {
        segments,
        self_similarity,
    })
}

#[requires(!source.is_empty())]
#[requires(!target.is_empty())]
#[ensures(true)]
fn aline_raw_similarity(source: &[IpaSegmentId], target: &[IpaSegmentId]) -> f64 {
    let mut scratch = AlineSimilarityScratch::default();
    aline_raw_similarity_with_scratch(source, target, &mut scratch)
}

#[requires(!source.is_empty())]
#[requires(!target.is_empty())]
#[ensures(true)]
fn aline_raw_similarity_with_scratch(
    source: &[IpaSegmentId],
    target: &[IpaSegmentId],
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let row_width = target.len() + 1;
    scratch.previous_previous.resize(row_width, 0.0);
    scratch.previous.resize(row_width, 0.0);
    scratch.current.resize(row_width, 0.0);
    scratch.previous.fill(0.0);

    let mut has_previous_previous = false;
    let mut best: f64 = 0.0;
    for source_index in 0..source.len() {
        scratch.current[0] = 0.0;
        for target_index in 1..=target.len() {
            let delete_source = scratch.previous[target_index] + ALINE_SKIP_SCORE;
            let insert_target = scratch.current[target_index - 1] + ALINE_SKIP_SCORE;
            let substitute = scratch.previous[target_index - 1]
                + substitution_score(source[source_index], target[target_index - 1]);
            let compress_source = if has_previous_previous && source_index > 0 {
                scratch.previous_previous[target_index - 1]
                    + expansion_score(
                        target[target_index - 1],
                        source[source_index - 1],
                        source[source_index],
                    )
            } else {
                0.0
            };
            let expand_target = if target_index > 1 {
                scratch.previous[target_index - 2]
                    + expansion_score(
                        source[source_index],
                        target[target_index - 2],
                        target[target_index - 1],
                    )
            } else {
                0.0
            };
            let cell = delete_source
                .max(insert_target)
                .max(substitute)
                .max(compress_source)
                .max(expand_target)
                .max(0.0);
            scratch.current[target_index] = cell;
            best = best.max(cell);
        }
        std::mem::swap(&mut scratch.previous_previous, &mut scratch.previous);
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
        has_previous_previous = true;
    }
    best
}

#[requires(!candidate.is_empty())]
#[requires(!source.is_empty())]
#[ensures(ret.is_finite())]
fn semiglobal_raw_similarity_with_scratch(
    candidate: &[IpaSegmentId],
    source: &[IpaSegmentId],
    scorer: &AlineScorer,
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let parameters = scorer.parameters();
    let row_width = source.len() + 1;
    scratch.previous_previous.resize(row_width, 0.0);
    scratch.previous.resize(row_width, 0.0);
    scratch.current.resize(row_width, 0.0);
    for (source_index, cell) in scratch.previous.iter_mut().enumerate() {
        *cell = source_index as f64 * parameters.c_flank;
    }

    for candidate_index in 1..=candidate.len() {
        scratch.current[0] = candidate_index as f64 * parameters.c_skip;
        for source_index in 1..=source.len() {
            let substitute = scratch.previous[source_index - 1]
                + scorer
                    .substitution_score(candidate[candidate_index - 1], source[source_index - 1]);
            let skip_candidate = scratch.previous[source_index] + parameters.c_skip;
            let skip_source = scratch.current[source_index - 1] + parameters.c_skip;
            let expand_source = if source_index >= 2 {
                scratch.previous[source_index - 2]
                    + scorer.expansion_score(
                        candidate[candidate_index - 1],
                        source[source_index - 2],
                        source[source_index - 1],
                    )
            } else {
                f64::NEG_INFINITY
            };
            let expand_candidate = if candidate_index >= 2 {
                scratch.previous_previous[source_index - 1]
                    + scorer.expansion_score(
                        source[source_index - 1],
                        candidate[candidate_index - 2],
                        candidate[candidate_index - 1],
                    )
            } else {
                f64::NEG_INFINITY
            };
            scratch.current[source_index] = substitute
                .max(skip_candidate)
                .max(skip_source)
                .max(expand_source)
                .max(expand_candidate);
        }
        std::mem::swap(&mut scratch.previous_previous, &mut scratch.previous);
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
    }

    scratch
        .previous
        .iter()
        .enumerate()
        .map(|(source_index, score)| {
            score + (source.len() - source_index) as f64 * parameters.c_flank
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

#[requires(true)]
#[ensures(ret.is_finite())]
fn parameterized_substitution_score(
    left: IpaSegmentId,
    right: IpaSegmentId,
    parameters: &AlineParameters,
) -> f64 {
    parameters.c_sub
        - parameterized_feature_difference(left, right, &parameters.saliences)
        - parameterized_vowel_penalty(left, parameters.c_vwl)
        - parameterized_vowel_penalty(right, parameters.c_vwl)
}

#[requires(true)]
#[ensures(ret.is_finite())]
fn parameterized_expansion_score(
    single: IpaSegmentId,
    first_second: IpaSegmentId,
    second_second: IpaSegmentId,
    parameters: &AlineParameters,
) -> f64 {
    parameters.c_exp
        - parameterized_feature_difference(single, first_second, &parameters.saliences)
        - parameterized_feature_difference(single, second_second, &parameters.saliences)
        - parameterized_vowel_penalty(single, parameters.c_vwl)
        - parameterized_vowel_penalty(first_second, parameters.c_vwl)
            .max(parameterized_vowel_penalty(second_second, parameters.c_vwl))
}

#[requires(true)]
#[ensures(ret.is_finite() && ret >= 0.0)]
fn parameterized_feature_difference(
    left: IpaSegmentId,
    right: IpaSegmentId,
    saliences: &AlineSaliences,
) -> f64 {
    let left_features = segment_features(left);
    let right_features = segment_features(right);
    relevant_features(left_features, right_features)
        .iter()
        .map(|feature| {
            (feature_value(*feature, left_features) - feature_value(*feature, right_features)).abs()
                * saliences.value(*feature)
        })
        .sum()
}

#[requires(c_vwl.is_finite() && c_vwl >= 0.0)]
#[ensures(ret == 0.0 || ret == c_vwl)]
fn parameterized_vowel_penalty(segment: IpaSegmentId, c_vwl: f64) -> f64 {
    if segment_features(segment).is_consonant {
        0.0
    } else {
        c_vwl
    }
}

#[requires(true)]
#[ensures(true)]
fn substitution_score(left: IpaSegmentId, right: IpaSegmentId) -> f64 {
    ALINE_SUBSTITUTION_CEILING
        - feature_difference(left, right)
        - vowel_penalty(left)
        - vowel_penalty(right)
}

#[requires(true)]
#[ensures(true)]
fn expansion_score(
    single: IpaSegmentId,
    first_second: IpaSegmentId,
    second_second: IpaSegmentId,
) -> f64 {
    ALINE_EXPANSION_CEILING
        - feature_difference(single, first_second)
        - feature_difference(single, second_second)
        - vowel_penalty(single)
        - vowel_penalty(first_second).max(vowel_penalty(second_second))
}

#[requires(true)]
#[ensures(true)]
fn feature_difference(left: IpaSegmentId, right: IpaSegmentId) -> f64 {
    let segment_count = IPA_SEGMENT_SYMBOLS.len();
    let index = (left.get() as usize) * segment_count + (right.get() as usize);
    // The ALINE tables are built from IPA_SEGMENT_SYMBOLS, and IpaSegmentId
    // guarantees every id indexes that same symbol table.
    ALINE_PAIR_FEATURE_DIFFERENCES[index]
}

#[requires(true)]
#[ensures(true)]
fn feature_difference_from_features(
    left_features: AlineFeatures,
    right_features: AlineFeatures,
) -> f64 {
    relevant_features(left_features, right_features)
        .iter()
        .map(|feature| {
            (feature_value(*feature, left_features) - feature_value(*feature, right_features)).abs()
                * feature_salience(*feature)
        })
        .sum()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relevant_features(left: AlineFeatures, right: AlineFeatures) -> &'static [AlineFeature] {
    if left.is_consonant || right.is_consonant {
        CONSONANT_RELEVANT_FEATURES
    } else {
        VOWEL_RELEVANT_FEATURES
    }
}

#[requires(true)]
#[ensures(true)]
fn feature_value(feature: AlineFeature, values: AlineFeatures) -> f64 {
    match feature {
        AlineFeature::Syllabic => values.syllabic_value,
        AlineFeature::Place => values.place_value,
        AlineFeature::Manner => values.manner_value,
        AlineFeature::Voice => values.voice_value,
        AlineFeature::Nasal => values.nasal_value,
        AlineFeature::Retroflex => values.retroflex_value,
        AlineFeature::Lateral => values.lateral_value,
        AlineFeature::Aspirated => values.aspirated_value,
        AlineFeature::High => values.high_value,
        AlineFeature::Back => values.back_value,
        AlineFeature::Round => values.round_value,
        AlineFeature::Long => values.long_value,
    }
}

#[requires(true)]
#[ensures(ret > 0.0)]
fn feature_salience(feature: AlineFeature) -> f64 {
    match feature {
        AlineFeature::Syllabic => 5.0,
        AlineFeature::Voice => 10.0,
        AlineFeature::Lateral => 10.0,
        AlineFeature::High => 5.0,
        AlineFeature::Manner => 50.0,
        AlineFeature::Long => 1.0,
        AlineFeature::Place => 40.0,
        AlineFeature::Nasal => 10.0,
        AlineFeature::Aspirated => 5.0,
        AlineFeature::Retroflex => 10.0,
        AlineFeature::Round => 5.0,
        AlineFeature::Back => 5.0,
    }
}

#[requires(true)]
#[ensures(true)]
fn vowel_penalty(segment: IpaSegmentId) -> f64 {
    ALINE_VOWEL_PENALTIES[segment.get() as usize]
}

#[requires(true)]
#[ensures(true)]
fn segment_features(segment: IpaSegmentId) -> AlineFeatures {
    IPA_SEGMENT_FEATURES[segment.get() as usize]
}

#[requires(!symbol.is_empty())]
#[ensures(true)]
fn derive_aline_features(symbol: &str) -> AlineFeatures {
    let base_symbol = strip_length_mark(strip_aspiration_mark(symbol));
    let is_consonant = !all_short_vowel_symbols().contains(&base_symbol);
    AlineFeatures {
        is_consonant,
        syllabic_value: if is_consonant { 0.0 } else { 1.0 },
        place_value: derive_place_value(base_symbol, is_consonant),
        manner_value: derive_manner_value(base_symbol, is_consonant),
        voice_value: derive_voice_value(base_symbol, is_consonant),
        nasal_value: flag(NASAL_SYMBOLS.contains(&base_symbol)),
        retroflex_value: flag(RETROFLEX_SYMBOLS.contains(&base_symbol)),
        lateral_value: flag(LATERAL_SYMBOLS.contains(&base_symbol)),
        aspirated_value: flag(symbol.ends_with('ʰ')),
        high_value: derive_high_value(base_symbol),
        back_value: derive_back_value(base_symbol),
        round_value: flag(ROUNDED_SYMBOLS.contains(&base_symbol)),
        long_value: flag(symbol.ends_with('ː')),
    }
}

#[requires(true)]
#[ensures(matches!(ret, 0.0 | 1.0))]
fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

#[requires(!symbol.is_empty())]
#[ensures(!ret.is_empty())]
fn strip_length_mark(symbol: &str) -> &str {
    symbol.strip_suffix('ː').unwrap_or(symbol)
}

#[requires(true)]
#[ensures(ret.len() <= symbol.len())]
fn strip_aspiration_mark(symbol: &str) -> &str {
    symbol.strip_suffix('ʰ').unwrap_or(symbol)
}

#[requires(true)]
#[ensures(true)]
fn derive_place_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant {
        -1.0
    } else if BILABIAL_SYMBOLS.contains(&symbol) {
        1.0
    } else if LABIODENTAL_SYMBOLS.contains(&symbol) {
        0.95
    } else if DENTAL_SYMBOLS.contains(&symbol) {
        0.9
    } else if ALVEOLAR_SYMBOLS.contains(&symbol) {
        0.85
    } else if RETROFLEX_SYMBOLS.contains(&symbol) {
        0.8
    } else if PALATO_ALVEOLAR_SYMBOLS.contains(&symbol) {
        0.75
    } else if ALVEOLO_PALATAL_SYMBOLS.contains(&symbol) {
        0.725
    } else if PALATAL_SYMBOLS.contains(&symbol) {
        0.7
    } else if VELAR_SYMBOLS.contains(&symbol) {
        0.6
    } else if UVULAR_SYMBOLS.contains(&symbol) {
        0.5
    } else if PHARYNGEAL_SYMBOLS.contains(&symbol) {
        0.3
    } else if GLOTTAL_SYMBOLS.contains(&symbol) {
        0.1
    } else {
        0.5
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_manner_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant {
        vowel_manner_value(symbol)
    } else if TRILL_SYMBOLS.contains(&symbol) {
        0.7
    } else if TAP_SYMBOLS.contains(&symbol) {
        0.65
    } else if APPROXIMANT_SYMBOLS.contains(&symbol) {
        0.6
    } else if AFFRICATE_SYMBOLS.contains(&symbol) {
        0.9
    } else if FRICATIVE_SYMBOLS.contains(&symbol) {
        0.8
    } else {
        1.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_voice_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant || VOICED_CONSONANT_SYMBOLS.contains(&symbol) {
        1.0
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn vowel_manner_value(symbol: &str) -> f64 {
    if HIGH_VOWEL_SYMBOLS.contains(&symbol) {
        0.4
    } else if MID_VOWEL_SYMBOLS.contains(&symbol) {
        0.2
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_high_value(symbol: &str) -> f64 {
    if HIGH_VOWEL_SYMBOLS.contains(&symbol) {
        1.0
    } else if MID_VOWEL_SYMBOLS.contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_back_value(symbol: &str) -> f64 {
    if FRONT_VOWEL_SYMBOLS.contains(&symbol) {
        1.0
    } else if CENTRAL_VOWEL_SYMBOLS.contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn all_short_vowel_symbols() -> &'static [&'static str] {
    &[
        "i", "y", "ɨ", "ʉ", "ɯ", "u", "ɪ", "ʊ", "ʏ", "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ",
        "ɜ", "ɞ", "ʌ", "ɔ", "ɚ", "ɝ", "æ", "ɐ", "a", "ɶ", "ä", "ɑ", "ɒ",
    ]
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_ipa(word_like: &WordLike) -> Vec<IpaSurfaceChunk<'_>> {
    flatten_word_like_ipa_in_context(word_like, LeadingPauseContext::IndependentWord)
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_ipa_in_context(
    word_like: &WordLike,
    leading_pause_context: LeadingPauseContext,
) -> Vec<IpaSurfaceChunk<'_>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => vec![IpaSurfaceChunk::Word {
            word,
            leading_pause_context,
        }],
        data!(WordLike::QuotedWord { zo, word }) => {
            vec![word_ipa_chunk(zo), word_ipa_chunk(word)]
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            vec![word_ipa_chunk(mahoi), word_ipa_chunk(word)]
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            word_ipa_chunk(zoi),
            word_ipa_chunk(opening_delimiter),
            IpaSurfaceChunk::Text(drop_leading_zoi_separator_ref(&quoted_text.text)),
            word_ipa_chunk(closing_delimiter),
        ],
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut chunks = vec![word_ipa_chunk(lohu)];
            chunks.extend(quoted_words.iter().map(word_ipa_chunk));
            chunks.push(word_ipa_chunk(lehu));
            chunks
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => vec![
            word_ipa_chunk(marker),
            IpaSurfaceChunk::Text(&quoted_text.text),
        ],
        data!(WordLike::LerfuWord { base, bu }) => {
            let mut chunks =
                flatten_word_like_ipa_in_context(base, LeadingPauseContext::BuLetterBase);
            chunks.push(word_ipa_chunk(bu));
            chunks
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            let mut chunks = flatten_word_like_ipa(left);
            chunks.push(word_ipa_chunk(zei));
            chunks.push(word_ipa_chunk(right));
            chunks
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn word_ipa_chunk(word: &Word) -> IpaSurfaceChunk<'_> {
    IpaSurfaceChunk::Word {
        word,
        leading_pause_context: LeadingPauseContext::IndependentWord,
    }
}

#[requires(true)]
#[ensures(true)]
fn drop_leading_zoi_separator_ref(text: &str) -> &str {
    text.strip_prefix(' ').unwrap_or(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_ipa_surface_chunks(
    chunks: &[IpaSurfaceChunk<'_>],
    source: &str,
) -> Result<String, PhoneticError> {
    let mut rendered = Vec::new();
    let mut previous_word: Option<IpaRenderedWord> = None;
    for chunk in chunks {
        match chunk {
            IpaSurfaceChunk::Word {
                word,
                leading_pause_context,
            } => {
                let word = render_word_ipa(word, source, *leading_pause_context)?;
                let pause_before = previous_word
                    .as_ref()
                    .is_some_and(|previous| previous.trailing_pause_required)
                    || (previous_word.is_some() && word.leading_pause_required);
                let body = if pause_before {
                    ipa_body_with_leading_pause(&word.body)
                } else {
                    word.body.clone()
                };
                rendered.push(body);
                previous_word = Some(word);
            }
            IpaSurfaceChunk::Text(text) => {
                if !text.is_empty() {
                    rendered.push((*text).to_owned());
                }
                previous_word = None;
            }
        }
    }
    Ok(rendered.join(" "))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rendered| !rendered.body.is_empty()) || ret.is_err())]
fn render_word_ipa(
    word: &Word,
    source: &str,
    leading_pause_context: LeadingPauseContext,
) -> Result<IpaRenderedWord, PhoneticError> {
    let phonemes = word.phonemes();
    let body = if word.kind() == WordKind::Cmevla {
        render_cmevla_ipa_body(&phonemes)
    } else {
        render_syllabified_ipa_body(&pronunciation_syllables(&phonemes).map_err(|error| {
            PhoneticError::Syllabification {
                message: error.to_string(),
            }
        })?)
    };
    Ok(IpaRenderedWord {
        body,
        leading_pause_required: explicit_leading_pause_count(source, word) > 0
            || required_leading_pause_count(word, leading_pause_context) > 0,
        trailing_pause_required: explicit_trailing_pause_count(source, word) > 0
            || word.kind() == WordKind::Cmevla,
    })
}

#[requires(!body.is_empty())]
#[ensures(!ret.is_empty())]
fn ipa_body_with_leading_pause(body: &str) -> String {
    body.strip_prefix('ˈ')
        .map(|rest| format!("ˈʔ{rest}"))
        .unwrap_or_else(|| format!("ʔ{body}"))
}

#[requires(!phonemes.as_str().is_empty())]
#[ensures(!ret.is_empty())]
fn render_cmevla_ipa_body(phonemes: &Phonemes) -> String {
    let text = phonemes.as_str();
    if text.contains(',') {
        let syllables = text
            .split(',')
            .filter(|syllable| !syllable.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if !syllables.is_empty() {
            return render_syllabified_ipa_body(&syllables);
        }
    }
    if text.chars().any(is_explicit_stress_char) {
        return render_unsyllabified_cmevla_ipa(text);
    }
    match pronunciation_syllables(phonemes) {
        Ok(syllables) => render_syllabified_ipa_body(&syllables),
        Err(_) => render_unsyllabified_cmevla_ipa(text),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || syllables.is_empty())]
fn render_syllabified_ipa_body(syllables: &[String]) -> String {
    let stress_index = explicit_stress_syllable_index(syllables)
        .or_else(|| conventional_stress_syllable_index(syllables));

    let mut rendered = String::new();
    for (index, syllable) in syllables.iter().enumerate() {
        if index > 0 {
            rendered.push('.');
        }
        if stress_index == Some(index) {
            rendered.push('ˈ');
        }
        rendered.push_str(&render_ipa_syllable(syllable));
    }
    rendered
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn render_unsyllabified_cmevla_ipa(text: &str) -> String {
    let mut rendered = String::new();
    for value in text.chars() {
        if is_explicit_stress_char(value) {
            rendered.push('ˈ');
        }
        push_ipa_phoneme(&mut rendered, value);
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn explicit_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    syllables
        .iter()
        .position(|syllable| syllable.chars().any(is_explicit_stress_char))
}

#[requires(true)]
#[ensures(true)]
fn conventional_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    let stressable = syllables
        .iter()
        .enumerate()
        .filter_map(|(index, syllable)| syllable_has_full_vowel(syllable).then_some(index))
        .collect::<Vec<_>>();
    stressable.iter().rev().nth(1).copied()
}

#[requires(true)]
#[ensures(true)]
fn syllable_has_full_vowel(syllable: &str) -> bool {
    syllable
        .chars()
        .any(|value| matches!(strip_vowel_diacritic(value), 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn is_explicit_stress_char(value: char) -> bool {
    matches!(
        value,
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ỳ'
    )
}

#[requires(true)]
#[ensures(true)]
fn render_ipa_syllable(syllable: &str) -> String {
    let mut rendered = String::new();
    for value in syllable.chars() {
        push_ipa_phoneme(&mut rendered, value);
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn push_ipa_phoneme(output: &mut String, value: char) {
    match value {
        'a' | 'á' | 'à' => output.push('a'),
        'e' | 'é' | 'è' => output.push('e'),
        'i' | 'í' | 'ì' => output.push('i'),
        'o' | 'ó' | 'ò' => output.push('o'),
        'u' | 'ú' | 'ù' => output.push('u'),
        'y' | 'ý' | 'ỳ' => output.push('ə'),
        'ĭ' => output.push('j'),
        'ŭ' => output.push('w'),
        '\'' => output.push('h'),
        '.' => output.push('ʔ'),
        'c' => output.push('ʃ'),
        'j' => output.push('ʒ'),
        other => output.push(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_vowel_diacritic(value: char) -> char {
    match value {
        'á' | 'à' => 'a',
        'é' | 'è' => 'e',
        'í' | 'ì' | 'ĭ' => 'i',
        'ó' | 'ò' => 'o',
        'ú' | 'ù' | 'ŭ' => 'u',
        'ý' | 'ỳ' => 'y',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn explicit_leading_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(..word.span().byte_start.min(source.len()))
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(true)]
fn explicit_trailing_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(word.span().byte_end.min(source.len())..)
        .unwrap_or_default()
        .iter()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(ret <= 1)]
fn required_leading_pause_count(word: &Word, context: LeadingPauseContext) -> usize {
    usize::from(word_needs_leading_pause_in_context(
        word,
        LeadingPauseVowelMode::FoldedVowels,
        context,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[requires(!candidate.is_empty())]
    #[requires(!source.is_empty())]
    #[ensures(ret.is_finite())]
    fn reference_global_raw_similarity(
        candidate: &[IpaSegmentId],
        source: &[IpaSegmentId],
        parameters: &AlineParameters,
    ) -> f64 {
        let mut table = vec![vec![0.0; source.len() + 1]; candidate.len() + 1];
        for candidate_index in 1..=candidate.len() {
            table[candidate_index][0] = candidate_index as f64 * parameters.c_skip;
        }
        for source_index in 1..=source.len() {
            table[0][source_index] = source_index as f64 * parameters.c_skip;
        }
        for candidate_index in 1..=candidate.len() {
            for source_index in 1..=source.len() {
                let mut best = table[candidate_index - 1][source_index - 1]
                    + parameterized_substitution_score(
                        candidate[candidate_index - 1],
                        source[source_index - 1],
                        parameters,
                    );
                best = best.max(table[candidate_index - 1][source_index] + parameters.c_skip);
                best = best.max(table[candidate_index][source_index - 1] + parameters.c_skip);
                if source_index >= 2 {
                    best = best.max(
                        table[candidate_index - 1][source_index - 2]
                            + parameterized_expansion_score(
                                candidate[candidate_index - 1],
                                source[source_index - 2],
                                source[source_index - 1],
                                parameters,
                            ),
                    );
                }
                if candidate_index >= 2 {
                    best = best.max(
                        table[candidate_index - 2][source_index - 1]
                            + parameterized_expansion_score(
                                source[source_index - 1],
                                candidate[candidate_index - 2],
                                candidate[candidate_index - 1],
                                parameters,
                            ),
                    );
                }
                table[candidate_index][source_index] = best;
            }
        }
        table[candidate.len()][source.len()]
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|sequence| sequence.segment_count() > 0))]
    fn property_corpus() -> Vec<IpaTokenSequence> {
        [
            "klama",
            "blanu",
            "ʃoj",
            "qɑt",
            "d͡ʒa",
            "ɡʌvərnmənt",
            "tradisjon",
        ]
        .iter()
        .map(|text| tokenize_ipa_text(text).expect("property corpus tokenizes"))
        .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_parameter_defaults_guard_current_kondrak_constants() {
        let parameters = AlineParameters::default();
        assert_eq!(parameters.c_sub, ALINE_SUBSTITUTION_CEILING);
        assert_eq!(parameters.c_exp, ALINE_EXPANSION_CEILING);
        assert_eq!(parameters.c_skip, ALINE_SKIP_SCORE);
        assert_eq!(parameters.c_vwl, ALINE_VOWEL_PENALTY);
        assert_eq!(parameters.c_flank, 0.0);
        assert_eq!(parameters.normalizer, AlineNormalizer::SourceSide);
        for feature in AlineFeature::all() {
            assert_eq!(
                parameters.saliences.value(*feature),
                feature_salience(*feature)
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semiglobal_with_skip_rate_flanks_matches_independent_global_reference() {
        let defaults = AlineParameters::default();
        let global = AlineParameters::try_new(
            defaults.saliences.clone(),
            defaults.c_sub,
            defaults.c_exp,
            defaults.c_skip,
            defaults.c_vwl,
            defaults.c_skip,
            defaults.normalizer,
        )
        .expect("global parameters");
        let corpus = property_corpus();
        for candidate in &corpus {
            for source in &corpus {
                let actual = aline_semiglobal_raw_similarity(
                    candidate.segments(),
                    source.segments(),
                    &global,
                );
                let expected = reference_global_raw_similarity(
                    candidate.segments(),
                    source.segments(),
                    &global,
                );
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_normalized_semiglobal_identity_is_one_over_tokenized_corpus() {
        let parameters = AlineParameters::default();
        for sequence in property_corpus() {
            assert_eq!(
                aline_semiglobal_similarity(sequence.segments(), sequence.segments(), &parameters,),
                1.0
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn free_source_flanks_preserve_contiguous_identity_window_score() {
        let parameters = AlineParameters::default();
        let candidate = tokenize_ipa_text("klama").expect("candidate");
        let source = tokenize_ipa_text("xklamah").expect("containing source");
        let raw =
            aline_semiglobal_raw_similarity(candidate.segments(), source.segments(), &parameters);
        let window_self = aline_semiglobal_raw_similarity(
            candidate.segments(),
            candidate.segments(),
            &parameters,
        );
        assert_eq!(raw, window_self);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_alignment_regimes_are_nested_over_pair_corpus() {
        let semiglobal = AlineParameters::default();
        let global = AlineParameters::try_new(
            semiglobal.saliences.clone(),
            semiglobal.c_sub,
            semiglobal.c_exp,
            semiglobal.c_skip,
            semiglobal.c_vwl,
            semiglobal.c_skip,
            semiglobal.normalizer,
        )
        .expect("global parameters");
        let corpus = property_corpus();
        for candidate in &corpus {
            for source in &corpus {
                let local = aline_raw_similarity(candidate.segments(), source.segments());
                let semi = aline_semiglobal_raw_similarity(
                    candidate.segments(),
                    source.segments(),
                    &semiglobal,
                );
                let full = aline_semiglobal_raw_similarity(
                    candidate.segments(),
                    source.segments(),
                    &global,
                );
                assert!(local >= semi, "local {local} < semi-global {semi}");
                assert!(semi >= full, "semi-global {semi} < global {full}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracketed_ipa_and_lojban_sound_queries_tokenize_to_the_same_word() {
        let bracketed = sound_query_to_token_sequence("[ˈkla.ma]").expect("bracketed IPA");
        let lojban = sound_query_to_token_sequence("klama").expect("Lojban query");

        assert_eq!(bracketed.segment_count(), 5);
        assert_eq!(lojban.segment_count(), 5);
        assert_eq!(
            aline_phonetic_similarity(bracketed.view(), lojban.view()),
            1.0
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_tokenizer_prefers_long_affricate_segments() {
        let tied_affricate = tokenize_ipa_text("t͡ʃa").expect("tie-bar affricate");
        let plain_affricate = tokenize_ipa_text("tʃa").expect("plain affricate");
        let separated = tokenize_ipa_text("t.ʃ.a").expect("separated segments");

        assert_eq!(tied_affricate.segment_count(), 2);
        assert_eq!(plain_affricate.segment_count(), 2);
        assert_eq!(separated.segment_count(), 3);
        assert!(
            aline_phonetic_similarity(tied_affricate.view(), plain_affricate.view())
                > aline_phonetic_similarity(tied_affricate.view(), separated.view())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aspirated_segment_variants_preserve_plain_consonant_features() {
        for (aspirated_symbol, plain_symbol) in [
            ("pʰ", "p"),
            ("bʰ", "b"),
            ("tʰ", "t"),
            ("dʰ", "d"),
            ("ʈʰ", "ʈ"),
            ("ɖʰ", "ɖ"),
            ("cʰ", "c"),
            ("ɟʰ", "ɟ"),
            ("kʰ", "k"),
            ("gʰ", "g"),
            ("qʰ", "q"),
            ("ɢʰ", "ɢ"),
            ("tʃʰ", "tʃ"),
            ("dʒʰ", "dʒ"),
            ("tsʰ", "ts"),
            ("dzʰ", "dz"),
            ("tɕʰ", "tɕ"),
            ("dʑʰ", "dʑ"),
            ("ʈʂʰ", "ʈʂ"),
            ("ɖʐʰ", "ɖʐ"),
        ] {
            let sequence = tokenize_ipa_text(aspirated_symbol).expect("aspirated IPA segment");
            let [segment] = sequence.segments() else {
                panic!("{aspirated_symbol} should tokenize as exactly one IPA segment");
            };
            assert_eq!(ipa_segment_symbol(*segment), Some(aspirated_symbol));

            let aspirated_features = segment_features(*segment);
            let plain_features = derive_aline_features(plain_symbol);
            assert_eq!(aspirated_features.aspirated_value, 1.0);
            assert_eq!(aspirated_features.long_value, 0.0);
            for feature in AlineFeature::all()
                .iter()
                .copied()
                .filter(|feature| *feature != AlineFeature::Aspirated)
            {
                assert_eq!(
                    feature_value(feature, aspirated_features),
                    feature_value(feature, plain_features),
                    "{aspirated_symbol} should preserve the {feature:?} feature of {plain_symbol}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gimfihi_inventory_additions_have_expected_aline_features() {
        let features = |symbol| {
            let sequence = tokenize_ipa_text(symbol).expect("supported IPA segment");
            let [segment] = sequence.segments() else {
                panic!("{symbol} should tokenize as exactly one IPA segment");
            };
            segment_features(*segment)
        };

        let voiceless_alveolo_palatal = features("ɕ");
        assert_eq!(voiceless_alveolo_palatal.place_value, 0.725);
        assert_eq!(voiceless_alveolo_palatal.manner_value, 0.8);
        assert_eq!(voiceless_alveolo_palatal.voice_value, 0.0);

        let voiced_alveolo_palatal = features("ʑ");
        assert_eq!(voiced_alveolo_palatal.place_value, 0.725);
        assert_eq!(voiced_alveolo_palatal.manner_value, 0.8);
        assert_eq!(voiced_alveolo_palatal.voice_value, 1.0);

        let alveolo_palatal_affricate = features("tɕ");
        assert_eq!(alveolo_palatal_affricate.place_value, 0.725);
        assert_eq!(alveolo_palatal_affricate.manner_value, 0.9);
        assert_eq!(alveolo_palatal_affricate.retroflex_value, 0.0);

        let retroflex_affricate = features("ʈʂ");
        assert_eq!(retroflex_affricate.place_value, 0.8);
        assert_eq!(retroflex_affricate.manner_value, 0.9);
        assert_eq!(retroflex_affricate.retroflex_value, 1.0);

        let labial_affricate = features("pf");
        assert_eq!(labial_affricate.place_value, 0.95);
        assert_eq!(labial_affricate.manner_value, 0.9);
        assert_eq!(labial_affricate.voice_value, 0.0);

        let palatal_glide = features("j");
        assert_eq!(
            features("ɥ"),
            AlineFeatures {
                round_value: 1.0,
                ..palatal_glide
            }
        );
        let labiovelar_glide = features("w");
        assert_eq!(
            features("ʍ"),
            AlineFeatures {
                voice_value: 0.0,
                ..labiovelar_glide
            }
        );

        assert_eq!(features("ɫ"), features("l"));
        let retroflex_lateral = features("ɭ");
        assert_eq!(retroflex_lateral.place_value, 0.8);
        assert_eq!(retroflex_lateral.retroflex_value, 1.0);
        assert_eq!(retroflex_lateral.lateral_value, 1.0);
        let palatal_lateral = features("ʎ");
        assert_eq!(palatal_lateral.place_value, 0.7);
        assert_eq!(palatal_lateral.lateral_value, 1.0);

        assert_eq!(features("ɪ"), features("i"));
        assert_eq!(features("ʊ"), features("u"));
        assert_eq!(features("ʏ"), features("y"));
        for rhotic_vowel in ["ɚ", "ɝ"] {
            let rhotic = features(rhotic_vowel);
            assert!(!rhotic.is_consonant);
            assert_eq!(rhotic.manner_value, 0.2);
            assert_eq!(rhotic.high_value, 0.5);
            assert_eq!(rhotic.back_value, 0.5);
            assert_eq!(rhotic.retroflex_value, 1.0);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_similarity_distinguishes_aspiration_from_identity_and_voicing() {
        let aspirated = tokenize_ipa_text("pʰa").expect("aspirated consonant");
        let plain = tokenize_ipa_text("pa").expect("plain consonant");
        let voiced = tokenize_ipa_text("ba").expect("voiced consonant");

        assert_eq!(aspirated.segment_count(), 2);
        assert_eq!(ipa_segment_symbol(aspirated.segments()[0]), Some("pʰ"));
        assert_ne!(aspirated.segments(), plain.segments());

        let aspiration_similarity = aline_phonetic_similarity(aspirated.view(), plain.view());
        let voicing_similarity = aline_phonetic_similarity(aspirated.view(), voiced.view());
        let identity_similarity = aline_phonetic_similarity(plain.view(), plain.view());
        assert!(aspiration_similarity > voicing_similarity);
        assert!(identity_similarity > aspiration_similarity);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_tokenizer_ignores_aspiration_outside_supported_consonants() {
        assert_eq!(
            tokenize_ipa_text("sʰa").expect("aspirated fricative fallback"),
            tokenize_ipa_text("sa").expect("plain fricative")
        );
        assert_eq!(
            tokenize_ipa_text("aʰ").expect("aspirated vowel fallback"),
            tokenize_ipa_text("a").expect("plain vowel")
        );
        assert_eq!(
            tokenize_ipa_text("ʰpa").expect("preaspiration fallback"),
            tokenize_ipa_text("pa").expect("plain consonant")
        );

        let tied = tokenize_ipa_text("t͡ʃʰa").expect("tie-bar aspirated affricate");
        let untied = tokenize_ipa_text("tʃʰa").expect("untied aspirated affricate");
        assert_eq!(tied, untied);
        assert_eq!(ipa_segment_symbol(tied.segments()[0]), Some("tʃʰ"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_tokenizer_accepts_gimfihi_ipa_normalization_inputs() {
        let normalized = tokenize_ipa_text("ɡátʰ").expect("normalized IPA input");
        let plain = tokenize_ipa_text("gat").expect("plain IPA input");
        let tied = tokenize_ipa_text("d͡ʒa").expect("tie-bar affricate");
        let untied = tokenize_ipa_text("dʒa").expect("untied affricate");

        assert_eq!(normalized.segment_count(), 3);
        assert_eq!(ipa_segment_symbol(normalized.segments()[2]), Some("tʰ"));
        let normalized_similarity = aline_phonetic_similarity(normalized.view(), plain.view());
        assert!(normalized_similarity > 0.9);
        assert!(normalized_similarity < 1.0);
        assert_eq!(aline_phonetic_similarity(tied.view(), untied.view()), 1.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nfd_unstable_ipa_segments_match_their_table_entries() {
        for symbol in ["ç", "ä", "äː"] {
            let sequence = tokenize_ipa_text(symbol).expect("IPA segment tokenizes");
            let [segment] = sequence.segments() else {
                panic!("{symbol} should tokenize as exactly one IPA segment");
            };

            assert_eq!(ipa_segment_symbol(*segment), Some(symbol));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_similarity_reuses_scratch_without_changing_scores() {
        let long = sound_query_to_token_sequence("klama").expect("longer query");
        let short = sound_query_to_token_sequence("ka").expect("shorter query");
        let different = sound_query_to_token_sequence("coi").expect("different query");
        let mut scratch = AlineSimilarityScratch::default();

        assert_eq!(
            aline_phonetic_similarity_with_scratch(long.view(), short.view(), &mut scratch),
            aline_phonetic_similarity(long.view(), short.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(short.view(), long.view(), &mut scratch),
            aline_phonetic_similarity(short.view(), long.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(different.view(), long.view(), &mut scratch),
            aline_phonetic_similarity(different.view(), long.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(long.view(), long.view(), &mut scratch),
            1.0
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_similarity_sorts_descending_with_dictionary_order_tie_breaks() {
        let mut scored = vec![(2, 0.5), (1, 0.8), (0, 0.8), (3, 0.2)];
        scored.sort_by(|left, right| compare_similarity_then_index(*left, *right));

        assert_eq!(scored, vec![(0, 0.8), (1, 0.8), (2, 0.5), (3, 0.2)]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_standard_lojban_ipa() {
        assert_eq!(lojban_text_to_ipa("klama").expect("IPA"), "ˈkla.ma");
        assert_eq!(lojban_text_to_ipa("coi").expect("IPA"), "ʃoj");
    }
}
