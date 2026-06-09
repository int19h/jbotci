//! Lujvo composition and decomposition.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_dictionary::{Dictionary, RafsiSource, WordType};
use jbotci_morphology::{
    LujvoBuildMode, LujvoBuildPart, LujvoBuildPartData, LujvoPart, Phonemes, WordKind, WordLike,
    bond_rafsis, canonicalize_text, choose_best_lujvo_candidate_from_parts, ends_with_consonant,
    ensure_cmevla_word, is_bonding_hyphen, segment_words_with_modifiers, syllables_pattern,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[invariant(!sources.is_empty())]
#[invariant(!parts.is_empty())]
#[invariant(!output.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LujvoPlan {
    pub sources: Vec<LujvoSource>,
    pub parts: Vec<LujvoPart>,
    pub output: String,
}

#[invariant(!word.is_empty())]
#[invariant(fixed_rafsi.as_ref().is_none_or(|rafsi| !rafsi.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LujvoSource {
    pub word: String,
    pub fixed_rafsi: Option<String>,
}

#[invariant(true)]
#[invariant(::Lujvo => true)]
#[invariant(::Cmevla => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JvozbaMode {
    Lujvo,
    Cmevla,
}

impl From<JvozbaMode> for LujvoBuildMode {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: JvozbaMode) -> Self {
        match value {
            JvozbaMode::Lujvo => Self::Lujvo,
            JvozbaMode::Cmevla => Self::Cmevla,
        }
    }
}

#[invariant(true)]
#[invariant(::Word(_) => true)]
#[invariant(::FixedRafsi(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum JvozbaInput {
    Word(String),
    FixedRafsi(String),
}

#[invariant(!word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JvozbaBuildResult {
    pub word: String,
    pub segments: Vec<JvozbaSegment>,
}

#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JvozbaSegment {
    pub kind: JvozbaSegmentKind,
    pub text: String,
}

#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JvozbaSegmentKind {
    Rafsi,
    Hyphen,
}

#[invariant(true)]
#[invariant(::RequiresAtLeastTwoInputs => true)]
#[invariant(::FixedRafsiEmpty => true)]
#[invariant(::NonFinalUniversalLongRafsi { .. } => true)]
#[invariant(::FinalConsonant { .. } => true)]
#[invariant(::NoRafsiAvailable { .. } => true)]
#[invariant(::NoDictionaryEntry { .. } => true)]
#[invariant(::CouldNotBuildLujvo => true)]
#[invariant(::CouldNotBuildCompound => true)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JvozbaError {
    #[error("jvozba requires at least two rafsi-producing inputs.")]
    RequiresAtLeastTwoInputs,
    #[error("Fixed rafsi cannot be empty.")]
    FixedRafsiEmpty,
    #[error("Fixed rafsi `{offending}` can only appear at the end of a lujvo.")]
    NonFinalUniversalLongRafsi { offending: String },
    #[error("{message}", message = render_final_consonant_message(offending, *is_fixed_rafsi))]
    FinalConsonant {
        offending: String,
        is_fixed_rafsi: bool,
    },
    #[error("No rafsi available for `{offending}`.")]
    NoRafsiAvailable { offending: String },
    #[error("No dictionary entry for `{offending}`.")]
    NoDictionaryEntry { offending: String },
    #[error("Could not build a valid lujvo from the supplied inputs.")]
    CouldNotBuildLujvo,
    #[error("Could not build a valid compound from the supplied inputs.")]
    CouldNotBuildCompound,
}

#[requires(true)]
#[ensures(true)]
pub fn compose_lujvo(
    dictionary: &Dictionary<'_>,
    sources: &[LujvoSource],
) -> Result<LujvoPlan, JvozbaError> {
    let inputs = sources
        .iter()
        .map(|source| match &source.fixed_rafsi {
            Some(fixed_rafsi) => JvozbaInput::FixedRafsi(canonicalize_text(fixed_rafsi)),
            None => JvozbaInput::Word(canonicalize_text(&source.word)),
        })
        .collect::<Vec<_>>();
    let result = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs)?;
    Ok(new!(LujvoPlan {
        sources: sources.to_vec(),
        parts: jvopau_segments(&result.segments),
        output: result.word.clone(),
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|word| !word.is_empty()) || ret.is_err())]
pub fn build_best_jvozba(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    raw_inputs: &[JvozbaInput],
) -> Result<String, String> {
    build_best_jvozba_detailed(mode, dictionary, raw_inputs)
        .map(|result| result.word.clone())
        .map_err(|error| render_jvozba_error(&error))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|result| !result.word.is_empty()) || ret.is_err())]
pub fn build_best_jvozba_detailed(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    raw_inputs: &[JvozbaInput],
) -> Result<JvozbaBuildResult, JvozbaError> {
    let expanded_inputs = raw_inputs
        .iter()
        .flat_map(|input| expand_input(dictionary, input))
        .collect::<Vec<_>>();
    if expanded_inputs.len() < 2 {
        return Err(JvozbaError::RequiresAtLeastTwoInputs);
    }
    let candidate_lists = build_candidate_lists(mode, dictionary, &expanded_inputs)?;
    let Some(candidate) = choose_best_lujvo_candidate_from_parts(mode.into(), &candidate_lists)
    else {
        return Err(match mode {
            JvozbaMode::Lujvo => JvozbaError::CouldNotBuildLujvo,
            JvozbaMode::Cmevla => JvozbaError::CouldNotBuildCompound,
        });
    };
    Ok(build_result_for_mode(
        mode,
        candidate.word.clone(),
        candidate.parts.clone(),
    ))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_jvozba_error(error: &JvozbaError) -> String {
    error.to_string()
}

#[requires(true)]
#[ensures(true)]
pub fn word_can_enter_jvozba_pane(dictionary: &Dictionary<'_>, word_text: &str) -> bool {
    let canonical_word = canonicalize_text(word_text);
    candidate_list_for_word(JvozbaMode::Lujvo, dictionary, false, &canonical_word).is_ok()
        || candidate_list_for_word(JvozbaMode::Lujvo, dictionary, true, &canonical_word).is_ok()
}

#[requires(true)]
#[ensures(true)]
pub fn decompose_lujvo_like<'a>(
    dictionary: &Dictionary<'a>,
    raw_word: &str,
) -> Option<LujvoDecomposition<'a>> {
    let normalized = normalize_lujvo_like_input(raw_word);
    if normalized.is_empty() {
        return None;
    }

    if let Some(parts) = morphology_lujvo_parts(&normalized) {
        return decomposition_from_parts(dictionary, parts);
    }

    if !is_cmevla_word(&normalized) {
        return None;
    }

    let decomposition = decomposition_from_parts(dictionary, fallback_lujvo_parts(&normalized)?)?;
    all_rafsi_segments_have_sources(&decomposition).then_some(decomposition)
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoDecomposition<'a> {
    pub segments: Vec<LujvoSegmentInfo<'a>>,
    pub source_words: Vec<&'a str>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoSegmentInfo<'a> {
    pub segment: LujvoPart,
    pub source: Option<&'a str>,
}

#[requires(true)]
#[ensures(true)]
fn expand_input(dictionary: &Dictionary<'_>, input: &JvozbaInput) -> Vec<JvozbaInput> {
    match input {
        JvozbaInput::FixedRafsi(rafsi_text) => {
            vec![JvozbaInput::FixedRafsi(canonicalize_text(rafsi_text))]
        }
        JvozbaInput::Word(word_text) => match decompose_lujvo_like(dictionary, word_text) {
            Some(decomposition) => decomposition
                .source_words
                .into_iter()
                .map(|source_word| JvozbaInput::Word(source_word.to_owned()))
                .collect(),
            None => vec![JvozbaInput::Word(canonicalize_text(word_text))],
        },
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|lists| lists.len() == inputs.len()) || ret.is_err())]
fn build_candidate_lists(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    inputs: &[JvozbaInput],
) -> Result<Vec<Vec<LujvoBuildPart>>, JvozbaError> {
    let total_count = inputs.len();
    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            candidate_list_for_input(mode, dictionary, index + 1 == total_count, input)
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|candidates| !candidates.is_empty()) || ret.is_err())]
fn candidate_list_for_input(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    is_last_input: bool,
    input: &JvozbaInput,
) -> Result<Vec<LujvoBuildPart>, JvozbaError> {
    match input {
        JvozbaInput::FixedRafsi(rafsi_text) => {
            if rafsi_text.is_empty() {
                return Err(JvozbaError::FixedRafsiEmpty);
            }
            if !is_last_input && is_fixed_universal_long_gismu_rafsi(dictionary, rafsi_text) {
                return Err(JvozbaError::NonFinalUniversalLongRafsi {
                    offending: rafsi_text.clone(),
                });
            }
            Ok(vec![new!(LujvoBuildPart::Rafsi(rafsi_text.clone()))])
        }
        JvozbaInput::Word(word_text) => {
            candidate_list_for_word(mode, dictionary, is_last_input, word_text)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|candidates| !candidates.is_empty()) || ret.is_err())]
fn candidate_list_for_word(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    is_last_input: bool,
    word_text: &str,
) -> Result<Vec<LujvoBuildPart>, JvozbaError> {
    let canonical_word = canonicalize_text(word_text);
    let Some(entry) = dictionary.lookup_word(&canonical_word) else {
        return non_dictionary_word_candidates(mode, is_last_input, &canonical_word).ok_or(
            JvozbaError::NoDictionaryEntry {
                offending: canonical_word,
            },
        );
    };
    let listed_rafsi = entry
        .rafsi
        .iter()
        .map(|rafsi| canonicalize_text(rafsi.0))
        .collect::<Vec<_>>();
    let gismu_extras = if entry.word_type.is_gismu_like() {
        jbotci_dictionary::universal_gismu_rafsi_forms(&canonical_word)
            .into_iter()
            .map(|(rafsi, _)| rafsi)
            .filter(|rafsi| is_last_input || rafsi != &canonical_word)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut all_candidates = listed_rafsi
        .into_iter()
        .chain(gismu_extras)
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| new!(LujvoBuildPart::Rafsi(candidate)))
        .collect::<Vec<_>>();
    if mode == JvozbaMode::Lujvo
        && is_last_input
        && word_type_allows_final_brivla_core(entry.word_type)
    {
        all_candidates.push(new!(LujvoBuildPart::BrivlaCore(canonical_word.clone())));
    }
    if mode == JvozbaMode::Lujvo
        && !is_last_input
        && all_candidates.is_empty()
        && word_type_allows_nonfinal_brivla_core(entry.word_type)
    {
        all_candidates.push(new!(LujvoBuildPart::BrivlaCore(canonical_word.clone())));
    }
    let all_candidates = unique_candidate_parts(all_candidates);
    let candidates = match (mode, is_last_input) {
        (JvozbaMode::Lujvo, true) => all_candidates,
        (JvozbaMode::Cmevla, true) => {
            let consonant_final_candidates = all_candidates
                .iter()
                .filter(|candidate| ends_with_consonant(candidate.as_text()))
                .cloned()
                .collect::<Vec<_>>();
            if consonant_final_candidates.is_empty() {
                all_candidates
            } else {
                consonant_final_candidates
            }
        }
        _ => all_candidates,
    };
    if candidates.is_empty() {
        if mode == JvozbaMode::Lujvo && is_last_input {
            Err(JvozbaError::FinalConsonant {
                offending: canonical_word,
                is_fixed_rafsi: false,
            })
        } else {
            Err(JvozbaError::NoRafsiAvailable {
                offending: canonical_word,
            })
        }
    } else {
        Ok(candidates)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|candidates| !candidates.is_empty()))]
fn non_dictionary_word_candidates(
    mode: JvozbaMode,
    is_last_input: bool,
    canonical_word: &str,
) -> Option<Vec<LujvoBuildPart>> {
    if mode != JvozbaMode::Lujvo || !is_last_input {
        return None;
    }
    let words = segment_words_with_modifiers(canonical_word).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    let word = word_like.bare_word()?;
    if !matches!(
        word.kind(),
        WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
    ) {
        return None;
    }
    Some(vec![new!(LujvoBuildPart::BrivlaCore(canonicalize_text(
        word.phonemes().as_str(),
    )))])
}

#[requires(true)]
#[ensures(true)]
fn unique_candidate_parts(values: Vec<LujvoBuildPart>) -> Vec<LujvoBuildPart> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

#[requires(true)]
#[ensures(true)]
fn unique_source_words<'a>(values: Vec<&'a str>) -> Vec<&'a str> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

#[requires(true)]
#[ensures(true)]
fn word_type_is_direct_brivla(word_type: WordType) -> bool {
    matches!(
        word_type,
        WordType::Gismu
            | WordType::ExperimentalGismu
            | WordType::Lujvo
            | WordType::Fuivla
            | WordType::ObsoleteFuivla
    )
}

#[requires(true)]
#[ensures(ret -> word_type_is_direct_brivla(word_type))]
fn word_type_allows_final_brivla_core(word_type: WordType) -> bool {
    word_type_is_direct_brivla(word_type)
}

#[requires(true)]
#[ensures(ret -> word_type_is_direct_brivla(word_type))]
fn word_type_allows_nonfinal_brivla_core(word_type: WordType) -> bool {
    matches!(word_type, WordType::Fuivla | WordType::ObsoleteFuivla)
}

#[requires(true)]
#[ensures(true)]
fn build_result_for_mode(
    mode: JvozbaMode,
    base_word: String,
    parts: Vec<String>,
) -> JvozbaBuildResult {
    let base_segments = jvozba_segments_from_parts(&parts);
    match mode {
        JvozbaMode::Lujvo => new!(JvozbaBuildResult {
            word: base_word,
            segments: base_segments,
        }),
        JvozbaMode::Cmevla => {
            let cmevla_word = ensure_cmevla_word(&base_word);
            let suffix = cmevla_word
                .strip_prefix(&base_word)
                .unwrap_or_default()
                .to_owned();
            let mut segments = base_segments;
            if !suffix.is_empty() {
                segments.push(new!(JvozbaSegment {
                    kind: JvozbaSegmentKind::Hyphen,
                    text: suffix,
                }));
            }
            new!(JvozbaBuildResult {
                word: cmevla_word,
                segments,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn jvozba_segments_from_parts(parts: &[String]) -> Vec<JvozbaSegment> {
    parts
        .iter()
        .map(|part| {
            new!(JvozbaSegment {
                kind: if is_bonding_hyphen(part) {
                    JvozbaSegmentKind::Hyphen
                } else {
                    JvozbaSegmentKind::Rafsi
                },
                text: part.clone(),
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn jvopau_segments(segments: &[JvozbaSegment]) -> Vec<LujvoPart> {
    segments
        .iter()
        .filter_map(|segment| {
            let phonemes = Phonemes::from_canonical(segment.text.clone()).ok()?;
            Some(match segment.kind {
                JvozbaSegmentKind::Rafsi => LujvoPart::rafsi(phonemes),
                JvozbaSegmentKind::Hyphen => LujvoPart::hyphen(phonemes),
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn render_final_consonant_message(offending: &str, is_fixed_rafsi: bool) -> String {
    if is_fixed_rafsi {
        format!(
            "Fixed rafsi `{offending}` cannot appear as the final rafsi of a lujvo. Use --cmevla to allow consonant-final output."
        )
    } else {
        format!(
            "No final rafsi for `{offending}` can end a lujvo. Use --cmevla to allow consonant-final output."
        )
    }
}

#[requires(true)]
#[ensures(true)]
fn is_fixed_universal_long_gismu_rafsi(dictionary: &Dictionary<'_>, rafsi_text: &str) -> bool {
    is_universal_gismu_long_rafsi(rafsi_text)
        && dictionary.lookup_word(rafsi_text).is_some_and(|entry| {
            matches!(
                entry.word_type,
                WordType::Gismu | WordType::ExperimentalGismu
            )
        })
}

#[requires(true)]
#[ensures(true)]
fn is_universal_gismu_long_rafsi(rafsi_text: &str) -> bool {
    jbotci_dictionary::universal_gismu_rafsi_forms(rafsi_text)
        .iter()
        .any(|(rafsi, source)| rafsi == rafsi_text && *source == RafsiSource::UniversalLong)
}

#[requires(true)]
#[ensures(true)]
fn normalize_lujvo_like_input(raw_word: &str) -> String {
    let apostrophe_normalized = raw_word
        .trim()
        .trim_matches('.')
        .chars()
        .map(normalize_apostrophe)
        .collect::<String>();
    canonicalize_text(&apostrophe_normalized)
}

#[requires(true)]
#[ensures(true)]
fn normalize_apostrophe(value: char) -> char {
    match value {
        '\'' | 'h' | 'H' | '’' | '\u{a78b}' | '\u{a78c}' | '\u{2bb}' | '\u{2bf}' | '\u{2b0}'
        | '\u{2d2}' => '\'',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_lujvo_parts(normalized: &str) -> Option<Vec<LujvoPart>> {
    let words = segment_words_with_modifiers(normalized).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    let word = word_like.bare_word()?;
    let parts = word.lujvo_parts()?;
    Some(parts.iter().cloned().collect())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|decomposition| decomposition.segments.iter().filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_))).count() >= 2))]
fn decomposition_from_parts<'a>(
    dictionary: &Dictionary<'a>,
    parts: Vec<LujvoPart>,
) -> Option<LujvoDecomposition<'a>> {
    let segments = parts
        .into_iter()
        .map(|segment| segment_with_source(dictionary, segment))
        .collect::<Vec<_>>();
    let rafsi_count = segments
        .iter()
        .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
        .count();
    if rafsi_count < 2 {
        return None;
    }

    let source_words = unique_source_words(
        segments
            .iter()
            .filter_map(|segment| match &segment.segment {
                LujvoPart::Rafsi(_) => segment.source,
                LujvoPart::Hyphen(_) => None,
            })
            .collect::<Vec<_>>(),
    );
    Some(LujvoDecomposition {
        segments,
        source_words,
    })
}

#[requires(true)]
#[ensures(true)]
fn all_rafsi_segments_have_sources(decomposition: &LujvoDecomposition<'_>) -> bool {
    decomposition
        .segments
        .iter()
        .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
        .all(|segment| segment.source.is_some())
}

#[requires(true)]
#[ensures(true)]
fn is_cmevla_word(normalized: &str) -> bool {
    let Ok(words) = segment_words_with_modifiers(normalized) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    let Some(word) = word_like.bare_word() else {
        return false;
    };
    word.kind() == WordKind::Cmevla
}

#[requires(true)]
#[ensures(true)]
fn segment_with_source<'a>(
    dictionary: &Dictionary<'a>,
    segment: LujvoPart,
) -> LujvoSegmentInfo<'a> {
    let source = match &segment {
        LujvoPart::Rafsi(phonemes) => source_word_for_rafsi_segment(dictionary, phonemes),
        LujvoPart::Hyphen(_) => None,
    };
    LujvoSegmentInfo { segment, source }
}

#[requires(true)]
#[ensures(true)]
fn source_word_for_rafsi_segment<'a>(
    dictionary: &Dictionary<'a>,
    phonemes: &Phonemes,
) -> Option<&'a str> {
    dictionary
        .lookup_rafsi(phonemes.as_str())
        .next()
        .map(|matched| matched.entry.word)
        .or_else(|| exact_brivla_source_word(dictionary, phonemes.as_str()))
}

#[requires(true)]
#[ensures(ret.is_none_or(|word| !word.is_empty()))]
fn exact_brivla_source_word<'a>(dictionary: &Dictionary<'a>, surface: &str) -> Option<&'a str> {
    let canonical_surface = canonicalize_text(surface);
    dictionary
        .lookup_word(&canonical_surface)
        .filter(|entry| word_type_is_direct_brivla(entry.word_type))
        .map(|entry| entry.word)
}

#[requires(true)]
#[ensures(true)]
fn fallback_lujvo_parts(normalized: &str) -> Option<Vec<LujvoPart>> {
    let parts = sloppy_decompose(normalized)?;
    let rafsi_parts = parts
        .iter()
        .filter_map(|part| match part {
            RawLujvoSegment::Rafsi(text) => Some(text.clone()),
            RawLujvoSegment::Hyphen(_) => None,
        })
        .collect::<Vec<_>>();
    let bonded = bond_rafsis(&rafsi_parts)?;
    if bonded.concat() == normalized {
        Some(
            parts
                .into_iter()
                .filter_map(|part| match part {
                    RawLujvoSegment::Rafsi(text) => Some(LujvoPart::rafsi(phonemes(text)?)),
                    RawLujvoSegment::Hyphen(text) => Some(LujvoPart::hyphen(phonemes(text)?)),
                })
                .collect(),
        )
    } else {
        None
    }
}

#[invariant(true)]
#[invariant(::Rafsi(_) => true)]
#[invariant(::Hyphen(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawLujvoSegment {
    Rafsi(String),
    Hyphen(String),
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn phonemes(text: String) -> Option<Phonemes> {
    Phonemes::from_canonical(text).ok()
}

#[requires(true)]
#[ensures(true)]
fn sloppy_decompose(normalized: &str) -> Option<Vec<RawLujvoSegment>> {
    sloppy_decompose_from(Vec::new(), normalized)
}

#[requires(true)]
#[ensures(true)]
fn sloppy_decompose_from(
    mut acc: Vec<RawLujvoSegment>,
    remaining: &str,
) -> Option<Vec<RawLujvoSegment>> {
    if remaining.is_empty() {
        acc.reverse();
        return Some(acc);
    }

    if should_drop_hyphen(&acc, remaining) {
        let (hyphen, rest) = split_char_at(remaining, 1)?;
        acc.push(RawLujvoSegment::Hyphen(hyphen.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if has_head_syllable(remaining, "CVV") && has_vowel_pair_after_initial(remaining) {
        let (rafsi, rest) = split_char_at(remaining, 3)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if split_char_at(remaining, 4)
        .and_then(|(prefix, _)| syllables_pattern(prefix))
        .as_deref()
        == Some("CV'V")
    {
        let (rafsi, rest) = split_char_at(remaining, 4)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if has_head_syllable(remaining, "CVCCY") || has_head_syllable(remaining, "CCVCY") {
        let (rafsi, rest_with_hyphen) = split_char_at(remaining, 4)?;
        let (_, rest) = split_char_at(rest_with_hyphen, 1)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        acc.push(RawLujvoSegment::Hyphen("y".to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if matches!(
        syllables_pattern(remaining).as_deref(),
        Some("CVCCV" | "CCVCV")
    ) {
        acc.push(RawLujvoSegment::Rafsi(remaining.to_owned()));
        acc.reverse();
        return Some(acc);
    }

    if has_head_syllable(remaining, "CVC") || has_head_syllable(remaining, "CCV") {
        let (rafsi, rest) = split_char_at(remaining, 3)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    None
}

#[requires(true)]
#[ensures(true)]
fn split_char_at(text: &str, count: usize) -> Option<(&str, &str)> {
    let byte_index = text
        .char_indices()
        .nth(count)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    if text.chars().count() < count {
        None
    } else {
        Some(text.split_at(byte_index))
    }
}

#[requires(true)]
#[ensures(true)]
fn should_drop_hyphen(acc: &[RawLujvoSegment], remaining: &str) -> bool {
    previous_is_rafsi(acc)
        && (remaining.starts_with('y')
            || remaining.starts_with("nr")
            || (remaining.starts_with('r') && has_head_syllable(remaining, "C")))
}

#[requires(true)]
#[ensures(true)]
fn previous_is_rafsi(acc: &[RawLujvoSegment]) -> bool {
    matches!(acc.last(), Some(RawLujvoSegment::Rafsi(_)))
}

#[requires(true)]
#[ensures(true)]
fn has_head_syllable(text: &str, pattern: &str) -> bool {
    split_char_at(text, pattern.chars().count())
        .and_then(|(prefix, _)| syllables_pattern(prefix))
        .is_some_and(|actual| actual == pattern)
}

#[requires(true)]
#[ensures(true)]
fn has_vowel_pair_after_initial(text: &str) -> bool {
    split_char_at(text, 3)
        .map(|(prefix, _)| prefix.chars().skip(1).collect::<String>())
        .is_some_and(|pair| matches!(pair.as_str(), "ai" | "ei" | "oi" | "au"))
}

#[requires(true)]
#[ensures(true)]
pub fn word_like_type_key(word_like: &WordLike) -> Option<&'static str> {
    let word = word_like.bare_word()?;
    Some(match word.kind() {
        jbotci_morphology::WordKind::Cmavo => "cmavo",
        jbotci_morphology::WordKind::Gismu => "gismu",
        jbotci_morphology::WordKind::Lujvo => "lujvo",
        jbotci_morphology::WordKind::Fuhivla => "fu'ivla",
        jbotci_morphology::WordKind::Cmevla => "cmevla",
    })
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn builds_simple_lujvo_from_dictionary_words() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("bangu".to_owned()),
            ],
        )
        .expect("jvozba result");
        assert_eq!(result.word, "jbobau");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmevla_mode_allows_consonant_final_output() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Cmevla,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::FixedRafsi("bau".to_owned()),
            ],
        )
        .expect("jvozba result");
        assert!(result.word.ends_with('s'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_rejects_candidates_rejected_by_morphology() {
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::FixedRafsi("kerl".to_owned()),
                JvozbaInput::FixedRafsi("u'u".to_owned()),
                JvozbaInput::Word("kerlo".to_owned()),
            ],
        )
        .expect_err("candidate must be rejected by full morphology validation");

        assert_eq!(error, JvozbaError::CouldNotBuildLujvo);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_accepts_final_vowel_initial_word_core() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("fulta".to_owned()),
                JvozbaInput::Word("ismu".to_owned()),
            ],
        )
        .expect("jvozba result");

        assert_eq!(result.word, "fuly'ismu");
        assert_segment_texts(&result, &["ful", "y'", "ismu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_accepts_final_vowel_initial_fuhivla_core() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::FixedRafsi("akt".to_owned()),
                JvozbaInput::Word("iismu".to_owned()),
            ],
        )
        .expect("jvozba result");

        assert_eq!(result.word, "aktyiismu");
        assert_segment_texts(&result, &["akt", "y", "iismu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_accepts_dictionary_rafsi_before_vowel_initial_fuhivla_core() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("gismu".to_owned()),
                JvozbaInput::Word("iismu".to_owned()),
            ],
        )
        .expect("jvozba result");

        assert_eq!(result.word, "gimyiismu");
        assert_segment_texts(&result, &["gim", "y", "iismu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_accepts_compound_before_vowel_initial_fuhivla_core() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("mutce".to_owned()),
                JvozbaInput::Word("nelci".to_owned()),
                JvozbaInput::Word("iismu".to_owned()),
            ],
        )
        .expect("jvozba result");

        assert_eq!(result.word, "tcenelyiismu");
        assert_segment_texts(&result, &["tce", "nel", "y", "iismu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_accepts_nonfinal_fuhivla_self_core() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("jenjigu".to_owned()),
                JvozbaInput::Word("dirce".to_owned()),
            ],
        )
        .expect("jvozba result");

        assert_eq!(result.word, "jenjigu'ydi'e");
        assert_segment_texts(&result, &["jenjigu", "'y", "di'e"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_mode_expands_dictionary_lujvo_with_fuhivla_self_core_source() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[JvozbaInput::Word("jenjigu'ydi'e".to_owned())],
        )
        .expect("expanded lujvo should provide at least two source inputs");

        assert_eq!(result.word, "jenjigu'ydi'e");
        assert_segment_texts(&result, &["jenjigu", "'y", "di'e"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_missing_dictionary_entries() {
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("notlojban".to_owned()),
            ],
        )
        .expect_err("missing entry");
        assert_eq!(
            render_jvozba_error(&error),
            "No dictionary entry for `notlojban`."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_exact_dictionary_final_component_source() {
        let decomposition = decompose_lujvo_like(jbotci_dictionary_data::english(), "jetcybolxada")
            .expect("morphology-backed lujvo decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["jetc", "y", "bolxáda"]);
        assert_eq!(decomposition.source_words, ["jetce", "bolxada"]);
        assert!(
            decomposition
                .segments
                .iter()
                .any(|segment| segment.segment.phonemes().as_str() == "bolxáda"
                    && segment.source == Some("bolxada"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_dictionary_backed_fuhivla_self_core_source() {
        let decomposition =
            decompose_lujvo_like(jbotci_dictionary_data::english(), "jenjigu'ydi'e")
                .expect("morphology-backed lujvo decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.source)
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["jenjigu", "'y", "dí'e"]);
        assert_eq!(sources, [Some("jenjigu"), None, Some("dirce")]);
        assert_eq!(decomposition.source_words, ["jenjigu", "dirce"]);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_segment_texts(result: &JvozbaBuildResult, expected: &[&str]) {
        let actual = result
            .segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
