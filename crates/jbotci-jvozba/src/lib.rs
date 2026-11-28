//! Lujvo composition and decomposition.

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use jbotci_dictionary::{
    Dictionary, DictionaryLujvoSegmentKind, DictionaryLujvoSource, EntryIndex, LookupKey,
    LookupPatternError, RafsiSource, WordType, fuivla_stem_rafsi, normalize_lookup_query,
};
use jbotci_interrupt::CancellationToken;
use jbotci_morphology::{
    GismuShape, LujvoBuildMode, LujvoBuildPart, LujvoBuildPartData, LujvoPart, MorphologyOptions,
    Phonemes, WordKind, WordLike, canonicalize_text,
    choose_best_lujvo_candidate_from_parts_with_filter, ends_with_consonant, ensure_cmevla_word,
    is_bonding_hyphen, is_fuhivla_rafsi_form, normalize_lojban_input_text,
    parse_cmevla_lujvo_word_part_candidates,
    segment_words_with_modifiers_with_options_and_source_id,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[invariant(::DictionaryWord { .. } => true)]
#[invariant(::FixedRafsi(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum JvozbaInput {
    Word(String),
    /// A word selected from this dictionary's results.
    DictionaryWord {
        entry_index: EntryIndex,
        word: String,
    },
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
#[invariant(::InvalidCharacter { .. } => true)]
#[invariant(::WordEmpty => true)]
#[invariant(::NonFinalUniversalLongRafsi { .. } => true)]
#[invariant(::FinalConsonant { .. } => true)]
#[invariant(::NoRafsiAvailable { .. } => true)]
#[invariant(::NoDictionaryEntry { .. } => true)]
#[invariant(::AmbiguousFuivlaRafsi { .. } => true)]
#[invariant(::LujvoPartWithoutSource { .. } => true)]
#[invariant(::TooMuchWork { .. } => true)]
#[invariant(::CouldNotBuildLujvo => true)]
#[invariant(::CouldNotBuildCompound => true)]
#[invariant(::Interrupted => true)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JvozbaError {
    #[error("jvozba requires at least two rafsi-producing inputs.")]
    RequiresAtLeastTwoInputs,
    #[error("Fixed rafsi cannot be empty.")]
    FixedRafsiEmpty,
    /// A typed word or fixed rafsi has a character that gentufa rejects, such
    /// as `â` in `klâma` or `#` in `klama#`. The error names it, as vlacku
    /// does, rather than reading the text without it.
    #[error("Invalid Lojban word: {word} (unsupported character `{character}`).")]
    InvalidCharacter { word: String, character: char },
    #[error("A word input cannot be empty.")]
    WordEmpty,
    #[error("Fixed rafsi `{offending}` can only appear at the end of a lujvo.")]
    NonFinalUniversalLongRafsi { offending: String },
    #[error("{message}", message = render_final_consonant_message(offending, *is_fixed_rafsi))]
    FinalConsonant {
        offending: String,
        is_fixed_rafsi: bool,
    },
    #[error("No rafsi available for `{offending}`.")]
    NoRafsiAvailable { offending: String },
    #[error("More than one fu'ivla entry has the rafsi `{stem}`. Enter the full source word.")]
    AmbiguousFuivlaRafsi { stem: String },
    /// A lujvo input has a rafsi that names no dictionary word, so the build
    /// cannot expand the input into its source words.
    #[error("The rafsi `{part}` in `{word}` names no dictionary word.")]
    LujvoPartWithoutSource { word: String, part: String },
    #[error("No dictionary entry for `{offending}`.")]
    NoDictionaryEntry { offending: String },
    #[error(
        "Building a word from these inputs needs {amount} {}, above this build's limit of {limit}. Use fewer or shorter pieces.",
        measure.description()
    )]
    TooMuchWork {
        measure: JvozbaWorkMeasure,
        amount: u64,
        limit: u64,
    },
    #[error("Could not build a valid lujvo from the supplied inputs.")]
    CouldNotBuildLujvo,
    #[error("Could not build a valid compound from the supplied inputs.")]
    CouldNotBuildCompound,
    /// The cancellation flag stopped morphology for one input piece.
    #[error("computation interrupted")]
    Interrupted,
}

/// What a refused build measured too much of.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JvozbaWorkMeasure {
    /// Pieces to combine, after dictionary lujvo expand into their own.
    Pieces,
    /// Letters in the longest spelling the search could assemble.
    SpellingLetters,
    /// One placement is one piece of one complete candidate spelling.
    Placements,
}

impl JvozbaWorkMeasure {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Pieces => "pieces to combine",
            Self::SpellingLetters => "letters in one candidate word",
            Self::Placements => "spelling comparisons",
        }
    }
}

/// How much work one build may perform. Every figure is read from the plan
/// the shared candidate expansion produces, so it counts the real search
/// rather than the input's length: dictionary lujvo expand into their own
/// pieces, and a word contributes as many candidates as it has usable rafsi.
///
/// The caller chooses the figures. [`JvozbaBuildLimits::unlimited`] is what
/// [`build_best_jvozba_detailed`] has always done, searching whatever it is
/// given; a service answering within a deadline picks finite figures from its
/// own measurements instead. Even an unlimited build refuses a space whose
/// size overflows a 64-bit count, because such a search cannot finish.
#[invariant(*max_pieces >= 2)]
#[invariant(*max_placements > 0)]
#[invariant(*max_spelling_letters > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JvozbaBuildLimits {
    pub max_pieces: usize,
    pub max_placements: u64,
    pub max_spelling_letters: usize,
}

impl JvozbaBuildLimits {
    /// No limit the caller cares about.
    #[requires(true)]
    #[ensures(ret.max_placements == u64::MAX)]
    pub fn unlimited() -> Self {
        new!(JvozbaBuildLimits {
            max_pieces: usize::MAX,
            max_placements: u64::MAX,
            max_spelling_letters: usize::MAX,
        })
    }
}

/// The letters of the longest spelling `choices` could assemble: the longest
/// candidate of every piece, plus at most one hyphen per junction. `None`
/// when that count overflows.
#[requires(true)]
#[ensures(choices.is_empty() -> ret == Some(0))]
fn longest_spelling_letters(choices: &[Vec<LujvoBuildPart>]) -> Option<usize> {
    let hyphens = choices.len().saturating_sub(1);
    choices.iter().try_fold(hyphens, |letters, choice| {
        let longest = choice
            .iter()
            .map(|part| part.as_text().chars().count())
            .max()
            .unwrap_or(0);
        letters.checked_add(longest)
    })
}

/// Whether searching `choices` exhaustively would exceed `limit` placements.
/// The product is built with early exit, so an astronomically large space is
/// recognized without computing its size.
#[requires(limit > 0)]
#[ensures(choices.is_empty() -> !ret)]
fn placements_exceed_limit(choices: &[Vec<LujvoBuildPart>], limit: u64) -> bool {
    if choices.is_empty() {
        return false;
    }
    let mut placements = choices.len() as u64;
    for choice in choices {
        placements = match placements.checked_mul(choice.len() as u64) {
            Some(placements) if placements <= limit => placements,
            _ => return true,
        };
    }
    false
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|result| !result.word.is_empty()) || ret.is_err())]
pub fn build_best_jvozba_detailed(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    raw_inputs: &[JvozbaInput],
    cancellation: &CancellationToken,
) -> Result<JvozbaBuildResult, JvozbaError> {
    build_best_jvozba_detailed_within(
        mode,
        dictionary,
        raw_inputs,
        JvozbaBuildLimits::unlimited(),
        cancellation,
    )
}

/// Build the best word `raw_inputs` can spell, doing no more work than
/// `limits` allows. The whole request is refused before the search starts,
/// because the search must visit every spelling before it can name the best
/// one: a half-searched space would yield a winner that is not the winner.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|result| !result.word.is_empty()) || ret.is_err())]
pub fn build_best_jvozba_detailed_within(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    raw_inputs: &[JvozbaInput],
    limits: JvozbaBuildLimits,
    cancellation: &CancellationToken,
) -> Result<JvozbaBuildResult, JvozbaError> {
    build_best_jvozba_with_evaluator(
        mode,
        dictionary,
        raw_inputs,
        limits,
        cancellation,
        |mode, choices, inputs, cancellation| {
            choose_best_lujvo_candidate_from_parts_with_filter(
                mode,
                choices,
                cancellation,
                |parsed| sources_match(dictionary, inputs, parsed),
            )
            .map_err(|error| {
                if error.is_interrupted() {
                    JvozbaError::Interrupted
                } else {
                    JvozbaError::CouldNotBuildLujvo
                }
            })
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn build_best_jvozba_with_evaluator(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    raw_inputs: &[JvozbaInput],
    limits: JvozbaBuildLimits,
    cancellation: &CancellationToken,
    evaluate: impl FnOnce(
        LujvoBuildMode,
        &[Vec<LujvoBuildPart>],
        &[ExpandedInput<'_>],
        &CancellationToken,
    ) -> Result<Option<jbotci_morphology::LujvoCandidate>, JvozbaError>,
) -> Result<JvozbaBuildResult, JvozbaError> {
    let mut plans = Vec::with_capacity(raw_inputs.len().min(limits.max_pieces));
    let mut count = Some(0usize);
    for input in raw_inputs {
        // One piece can be long enough that morphology, not the piece cap,
        // is the work. Stop before that segmentation when the flag is already set.
        if cancellation.is_cancelled() {
            return Err(JvozbaError::Interrupted);
        }
        let plan = match plan_expansion(dictionary, input, cancellation) {
            Ok(plan) => plan,
            Err(JvozbaError::Interrupted) => return Err(JvozbaError::Interrupted),
            Err(error) => {
                // When the earlier inputs are over the piece limit, their
                // candidates are not made, and the plan error stands.
                if count.is_none_or(|count| count > limits.max_pieces) {
                    return Err(error);
                }
                return Err(first_error_before_failed_plan(
                    mode,
                    dictionary,
                    plans,
                    error,
                    cancellation,
                ));
            }
        };
        count = count.and_then(|count| count.checked_add(plan.piece_count()));
        // Count every occurrence for the exact refusal reason. Retain a plan
        // only while the eventual expansion can still fit the piece limit.
        if count.is_some_and(|count| count <= limits.max_pieces) {
            plans.push(plan);
        }
    }
    let Some(count) = count.filter(|count| *count <= limits.max_pieces) else {
        return Err(JvozbaError::TooMuchWork {
            measure: JvozbaWorkMeasure::Pieces,
            amount: count.unwrap_or(usize::MAX) as u64,
            limit: limits.max_pieces as u64,
        });
    };
    if count < 2 {
        return Err(JvozbaError::RequiresAtLeastTwoInputs);
    }
    let mut expanded_inputs = Vec::with_capacity(count);
    for plan in plans {
        plan.append_to(&mut expanded_inputs);
    }
    let candidate_lists = build_candidate_lists(mode, dictionary, &expanded_inputs, cancellation)?;
    // A short choice list can still carry long pieces, and every spelling is
    // assembled and scored in full, so the spelling's own size is bounded
    // apart from how many spellings there are.
    let letters = longest_spelling_letters(&candidate_lists);
    if letters.is_none_or(|letters| letters > limits.max_spelling_letters) {
        return Err(JvozbaError::TooMuchWork {
            measure: JvozbaWorkMeasure::SpellingLetters,
            amount: letters.unwrap_or(usize::MAX) as u64,
            limit: limits.max_spelling_letters as u64,
        });
    }
    if placements_exceed_limit(&candidate_lists, limits.max_placements) {
        return Err(JvozbaError::TooMuchWork {
            measure: JvozbaWorkMeasure::Placements,
            // The product is not computed past the limit, so report the limit
            // itself as the figure exceeded rather than invent a total.
            amount: limits.max_placements.saturating_add(1),
            limit: limits.max_placements,
        });
    }
    // The piece loop already loaded the flag. This load covers the search
    // that scores every spelling, which is the work the piece cap does not bound.
    if cancellation.is_cancelled() {
        return Err(JvozbaError::Interrupted);
    }
    let Some(candidate) = evaluate(
        mode.into(),
        &candidate_lists,
        &expanded_inputs,
        cancellation,
    )?
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
#[ensures(true)]
pub fn word_can_enter_jvozba_pane(
    dictionary: &Dictionary<'_>,
    word_text: &str,
    cancellation: &CancellationToken,
) -> Result<bool, JvozbaError> {
    // A word with a character that gentufa rejects cannot enter the pane.
    let Ok(input) = ExpandedInput::for_word(dictionary, word_text) else {
        return Ok(false);
    };
    input_can_enter_jvozba_pane(dictionary, word_text, input, cancellation)
}

/// Test the exact dictionary entry carried by a result card.
#[requires(true)]
#[ensures(true)]
pub fn entry_can_enter_jvozba_pane(
    dictionary: &Dictionary<'_>,
    entry_index: EntryIndex,
    cancellation: &CancellationToken,
) -> Result<bool, JvozbaError> {
    let Some(entry) = dictionary.entry_for_index(entry_index) else {
        return Ok(false);
    };
    input_can_enter_jvozba_pane(
        dictionary,
        entry.word,
        ExpandedInput::DictionaryWord(entry_index),
        cancellation,
    )
}

/// Whether the jvozba pane admits a word.
///
/// A word is admitted when it gives a candidate in some position, as before
/// #1149. A word with no candidate is also admitted when the morphology reads
/// it as a lujvo, even when a build with it can only fail, for example when
/// one of its rafsi names no dictionary word (`lefkroli`). The owner ruled so
/// on 2026-11-26: the user sees why the word cannot build, which is better
/// than a hidden word, and the pane build then shows the error that names the
/// rafsi. Do not make this test refuse such a word. The ruling keeps these
/// words admitted and adds no others: a cmevla that the build expands into
/// source words (`lojban`, `moskov`) has no candidate of its own, so it stays
/// out, as before.
#[requires(true)]
#[ensures(true)]
fn input_can_enter_jvozba_pane(
    dictionary: &Dictionary<'_>,
    word_text: &str,
    input: ExpandedInput<'_>,
    cancellation: &CancellationToken,
) -> Result<bool, JvozbaError> {
    for is_last_input in [false, true] {
        if pane_candidate_available(candidate_list_for_input(
            JvozbaMode::Lujvo,
            dictionary,
            is_last_input,
            input.clone(),
            cancellation,
        ))? {
            return Ok(true);
        }
    }
    is_morphology_lujvo(word_text, cancellation)
}

/// Whether the morphology reads `raw_word` as one lujvo.
///
/// This is the one test for "lujvo form" in jvozba. A cmevla that reads as a
/// compound of known words is not a lujvo here, though
/// `decompose_lujvo_like` also decomposes it.
#[requires(true)]
#[ensures(true)]
fn is_morphology_lujvo(
    raw_word: &str,
    cancellation: &CancellationToken,
) -> Result<bool, JvozbaError> {
    let normalized = normalize_lujvo_like_input(raw_word);
    if normalized.is_empty() {
        return Ok(false);
    }
    Ok(morphology_lujvo_parts(&normalized, cancellation)?.is_some())
}

/// A missing rafsi is "no", and a set flag is an interrupt. The two must not share one answer.
#[requires(true)]
#[ensures(true)]
fn pane_candidate_available(
    result: Result<Vec<LujvoBuildPart>, JvozbaError>,
) -> Result<bool, JvozbaError> {
    match result {
        Ok(_) => Ok(true),
        Err(JvozbaError::Interrupted) => Err(JvozbaError::Interrupted),
        Err(_) => Ok(false),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn decompose_lujvo_like<'a>(
    dictionary: &Dictionary<'a>,
    raw_word: &str,
    cancellation: &CancellationToken,
) -> Result<Option<LujvoDecomposition<'a>>, JvozbaError> {
    let normalized = normalize_lujvo_like_input(raw_word);
    if normalized.is_empty() {
        return Ok(None);
    }

    if let Some(parts) = morphology_lujvo_parts(&normalized, cancellation)? {
        return Ok(decomposition_from_parts(dictionary, parts));
    }

    if !is_cmevla_word(&normalized, cancellation)? {
        return Ok(None);
    }

    for parts in fallback_lujvo_part_candidates(&normalized) {
        if let Some(decomposition) = decomposition_from_parts(dictionary, parts)
            && all_rafsi_segments_have_sources(&decomposition)
        {
            return Ok(Some(decomposition));
        }
    }
    Ok(None)
}

#[invariant(segments
    .iter()
    .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
    .count()
    >= 2)]
// Quadratic and validation-only, so it is not worth paying for on every
// decomposition the dictionary build performs.
#[expensive_invariant(
    source_words
        .iter()
        .enumerate()
        .all(|(index, word)| !source_words[..index].contains(word)),
    "source words name each word once; per-occurrence work reads the segments"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoDecomposition<'a> {
    pub segments: Vec<LujvoSegmentInfo<'a>>,
    pub source_words: Vec<&'a str>,
}

#[invariant(matches!(segment, LujvoPart::Rafsi(_)) == kind.is_rafsi())]
#[invariant(kind.is_rafsi() || sources.is_empty())]
#[invariant(kind == &DictionaryLujvoSegmentKind::FuivlaRafsi || sources.len() <= 1)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoSegmentInfo<'a> {
    pub segment: LujvoPart,
    pub kind: DictionaryLujvoSegmentKind,
    pub sources: Vec<DictionaryLujvoSource<'a>>,
}

/// How the final part of an input lujvo behaves in each position.
///
/// A gismu form (`lefmo`, `bangu`) is the full word finally and its long
/// (4-letter) rafsi non-finally. A dictionary fu'ivla form (`spageti`)
/// is the full word in both positions: finally it ends the lujvo, and
/// non-finally the bonding adds `'y` (CLL 4.16). A self-sourced fu'ivla
/// form (`iismu`) is the full word only finally; non-finally it is
/// refused, in `candidate_list_for_input`. A short rafsi (`bau`, `si'o`)
/// is one rafsi in both positions.
#[invariant(true)]
#[invariant(::Gismu => true)]
#[invariant(::FullBrivla => true)]
#[invariant(::Rafsi => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FinalPartKind {
    Gismu,
    FullBrivla,
    Rafsi,
}

/// A rafsi of an input lujvo, kept in the form the input used.
///
/// The form is the surface the morphology decomposition produced, so the
/// builder offers it as the only candidate and the scorer cannot re-choose
/// a different rafsi of the source word. `source` is the dictionary entry
/// the rafsi names; a fu'ivla stem is kept apart because its candidate is
/// a `FuhivlaStem`, whose bonding restores the hyphen.
#[invariant(!form.is_empty())]
#[invariant(hyphen_after.as_deref().is_none_or(is_bonding_hyphen), "a kept joint is a bonding hyphen")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeptRafsi {
    form: String,
    source: EntryIndex,
    /// The hyphen the input wrote after this rafsi, when it wrote one.
    /// The joint between two kept parts of one input is copied as the
    /// input wrote it; the builder chooses hyphens only between inputs.
    hyphen_after: Option<String>,
}

/// A fu'ivla rafsi of an input lujvo, kept as its stem and its hyphen.
///
/// A fu'ivla rafsi segment is always followed by its hyphen segment, so
/// the hyphen is part of the piece, not an option.
#[invariant(!form.is_empty())]
#[invariant(
    is_bonding_hyphen(hyphen),
    "a kept fu'ivla stem carries its bonding hyphen"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeptFuhivlaStem {
    form: String,
    source: EntryIndex,
    hyphen: String,
}

/// The final part of an input lujvo that names a dictionary word, kept in
/// the form the input used (see [`FinalPartKind`]).
#[invariant(!form.is_empty())]
#[invariant(*kind != FinalPartKind::Gismu || GismuShape::classify(form).is_some(), "a gismu final part is a gismu form")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeptFinalPart {
    form: String,
    kind: FinalPartKind,
    source: EntryIndex,
}

/// The final part of an input lujvo that is its own source.
///
/// It needs no dictionary entry: the form is a valid gismu or fu'ivla, so
/// it can end a lujvo as itself. `kind` is the morphology's classification
/// of the form alone.
#[invariant(!form.is_empty())]
#[invariant(kind != &FinalPartKind::Rafsi)]
#[invariant(*kind != FinalPartKind::Gismu || GismuShape::classify(form).is_some(), "a gismu final part is a gismu form")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SelfSourcedFinalPart {
    form: String,
    kind: FinalPartKind,
}

/// A long gismu rafsi pinned by its `y` hyphen (`golf` in `golfyjvi`).
///
/// Without a dictionary entry the gismu's final vowel is unknown, but the
/// 4-letter form is one valid rafsi whatever the gismu is, so it is pinned
/// like a `--rafsi` form. It is built only in a non-final position, so it
/// never has to end a lujvo.
#[invariant(GismuShape::classify(&format!("{stem}a")).is_some(), "the stem with a final vowel is a gismu form")]
#[invariant(matches!(hyphen.as_str(), "y" | "y'"), "a pinned long rafsi carries its y hyphen")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PinnedLongRafsi {
    stem: String,
    /// The `y` hyphen the input wrote after this rafsi.
    hyphen: String,
}

/// The source chosen before the builder generates any candidate spellings.
#[invariant(true)]
#[invariant(::FixedRafsi(_) => true)]
#[invariant(::UnlistedWord(_) => true)]
#[invariant(::DictionaryWord(_) => true)]
#[invariant(::KeptRafsi(_) => true)]
#[invariant(::KeptFuhivlaStem(_) => true)]
#[invariant(::KeptFinalPart(_) => true)]
#[invariant(::SelfSourcedFinalPart(_) => true)]
#[invariant(::PinnedLongRafsi(_) => true)]
#[derive(Debug, Clone)]
enum ExpandedInput<'a> {
    FixedRafsi(&'a str),
    UnlistedWord(&'a str),
    DictionaryWord(EntryIndex),
    KeptRafsi(KeptRafsi),
    KeptFuhivlaStem(KeptFuhivlaStem),
    KeptFinalPart(KeptFinalPart),
    SelfSourcedFinalPart(SelfSourcedFinalPart),
    PinnedLongRafsi(PinnedLongRafsi),
}

impl<'a> ExpandedInput<'a> {
    /// The input for a typed word. The word is keyed with the typed-query
    /// rule, so a character that gentufa rejects is an error that names it.
    /// A word with no key character, such as `,`, is an unlisted word, and
    /// its error names it as typed.
    #[requires(true)]
    #[ensures(ret.as_ref().is_err_and(|error| matches!(error, JvozbaError::InvalidCharacter { .. })) || ret.is_ok())]
    fn for_word(dictionary: &Dictionary<'_>, word: &'a str) -> Result<Self, JvozbaError> {
        match LookupKey::from_typed(word) {
            Ok(key) => Ok(dictionary
                .lookup_word_index(&key)
                .map_or(Self::UnlistedWord(word), Self::DictionaryWord)),
            Err(error) => typed_input_error(word, &error).map_or(Ok(Self::UnlistedWord(word)), Err),
        }
    }
}

/// The error for typed text that the typed-query key rejects, or `None` when
/// the text only has no key character, which the caller reports itself.
#[requires(true)]
#[ensures(ret.is_some() == error.invalid_character().is_some())]
fn typed_input_error(text: &str, error: &LookupPatternError) -> Option<JvozbaError> {
    error
        .invalid_character()
        .map(|character| JvozbaError::InvalidCharacter {
            word: text.trim().to_owned(),
            character,
        })
}

/// A borrowed input, or the pieces of its decomposition, before expanded
/// strings are allocated.
#[invariant(true)]
#[invariant(::Plain { .. } => true)]
#[invariant(::Decomposed { pieces } => pieces.len() >= 2, "a decomposed input has a piece for each of its two or more rafsi")]
#[derive(Debug)]
enum ExpansionPlan<'input> {
    Plain {
        input: ExpandedInput<'input>,
    },
    /// One piece for each rafsi of the input, in order, with repeats. Each
    /// piece keeps the rafsi form the input used, so no part is dropped and
    /// none is re-chosen.
    Decomposed {
        pieces: Vec<ExpandedInput<'input>>,
    },
}
impl<'input> ExpansionPlan<'input> {
    #[requires(true)]
    #[ensures(ret > 0)]
    fn piece_count(&self) -> usize {
        match self.as_data() {
            data!(ExpansionPlan::Plain { .. }) => 1,
            data!(ExpansionPlan::Decomposed { pieces }) => pieces.len(),
        }
    }
    #[requires(true)]
    #[ensures(destination.len() == old(destination.len()) + old(self.piece_count()))]
    fn append_to(self, destination: &mut Vec<ExpandedInput<'input>>) {
        match self.into_data() {
            data!(ExpansionPlan::Plain { input }) => destination.push(input),
            data!(ExpansionPlan::Decomposed { pieces }) => destination.extend(pieces),
        }
    }
}
/// The error to report when the plan of one input fails.
///
/// jvozba reports the error of the first failing input, in the order that the
/// user wrote, as before #1149. A plan error of a later input must not hide a
/// candidate error of an earlier one (`djan lefkroli` names `djan`). So the
/// earlier inputs get their candidates first. Each of them is followed by the
/// failing input, so none of them is the last input. The caller calls this
/// only when the earlier inputs fit the piece limit. A caller that finds them
/// over the limit keeps the plan error and does not call.
///
/// An interruption of the plan stops the build before this call, so the plan
/// error is never `Interrupted`.
#[requires(!matches!(plan_error, JvozbaError::Interrupted))]
#[ensures(true)]
fn first_error_before_failed_plan(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    earlier: Vec<ExpansionPlan<'_>>,
    plan_error: JvozbaError,
    cancellation: &CancellationToken,
) -> JvozbaError {
    let mut expanded = Vec::new();
    for plan in earlier {
        plan.append_to(&mut expanded);
    }
    for input in expanded {
        if let Err(error) = candidate_list_for_input(mode, dictionary, false, input, cancellation) {
            return error;
        }
    }
    plan_error
}

#[requires(true)]
#[ensures(true)]
fn plan_expansion<'input>(
    dictionary: &Dictionary<'_>,
    input: &'input JvozbaInput,
    cancellation: &CancellationToken,
) -> Result<ExpansionPlan<'input>, JvozbaError> {
    let (input, word_text) = match input {
        // A word of spaces only names nothing. A word of other characters that
        // normalize to nothing keeps its typed spelling in the error.
        JvozbaInput::Word(text) if text.trim().is_empty() => return Err(JvozbaError::WordEmpty),
        JvozbaInput::Word(text) => (ExpandedInput::for_word(dictionary, text)?, Some(text)),
        JvozbaInput::DictionaryWord { entry_index, word } => {
            if !dictionary
                .entry_for_index(*entry_index)
                .is_some_and(|entry| entry.word == word)
            {
                return Err(JvozbaError::NoDictionaryEntry {
                    offending: word.clone(),
                });
            }
            (ExpandedInput::DictionaryWord(*entry_index), Some(word))
        }
        JvozbaInput::FixedRafsi(text) => (ExpandedInput::FixedRafsi(text), None),
    };
    if let Some(text) = word_text
        && let Some(decomposition) = decompose_lujvo_like(dictionary, text, cancellation)?
    {
        Ok(new!(ExpansionPlan::Decomposed {
            pieces: decomposition_pieces(dictionary, text, &decomposition, cancellation)?,
        }))
    } else {
        Ok(new!(ExpansionPlan::Plain { input }))
    }
}

/// One piece for each rafsi of a decomposed input, in order.
///
/// A lujvo-form input always expands, and every rafsi becomes exactly one
/// piece, so the build never drops a part. Each piece keeps the form the
/// input used. A rafsi that names a dictionary word keeps that form: its
/// source is recorded, but the scorer gets the one form, not every rafsi
/// of the source word. The final part needs no dictionary word when it is
/// a valid gismu or fu'ivla, because it is its own source. A non-final
/// 4-letter rafsi before a `y` hyphen needs none either, because that long
/// form is valid whatever the gismu is. Any other rafsi with no source is
/// an error that names it (`bolvii'ybau` has no source for `bolvii`, and
/// `lefkroli` names `lef`). A fu'ivla rafsi whose sources have more than
/// one lookup key is ambiguous. Sources that share a lookup key are one
/// word.
#[requires(true)]
#[ensures(match &ret {
    Ok(pieces) => pieces.len() == decomposition.segments.iter().filter(|segment| segment.kind.is_rafsi()).count(),
    Err(error) => matches!(error, JvozbaError::LujvoPartWithoutSource { .. } | JvozbaError::AmbiguousFuivlaRafsi { .. } | JvozbaError::Interrupted),
})]
fn decomposition_pieces(
    dictionary: &Dictionary<'_>,
    word: &str,
    decomposition: &LujvoDecomposition<'_>,
    cancellation: &CancellationToken,
) -> Result<Vec<ExpandedInput<'static>>, JvozbaError> {
    let segments = &decomposition.segments;
    let last_rafsi = segments.iter().rposition(|segment| segment.kind.is_rafsi());
    let mut pieces = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        if !segment.kind.is_rafsi() {
            continue;
        }
        let form = canonicalize_text(segment.segment.phonemes().as_str());
        let missing = || JvozbaError::LujvoPartWithoutSource {
            word: offending_spelling(word, &normalized_spelling(word)),
            part: form.clone(),
        };
        // The hyphen the input wrote after this rafsi, for a rafsi that is
        // not the last one. The last rafsi has no internal joint: the joint
        // after it joins a different input and the builder chooses it.
        let hyphen_after = if Some(index) == last_rafsi {
            None
        } else {
            match segments.get(index + 1) {
                Some(next) => match &next.segment {
                    LujvoPart::Hyphen(phonemes) => Some(canonicalize_text(phonemes.as_str())),
                    LujvoPart::Rafsi(_) => None,
                },
                None => None,
            }
        };
        if segment.kind == DictionaryLujvoSegmentKind::FuivlaRafsi {
            // A fu'ivla rafsi is never the final segment: that kind is
            // defined by the hyphen segment that follows it, so that
            // hyphen is always written here.
            let hyphen = hyphen_after
                .clone()
                .expect("a fu'ivla rafsi segment is followed by its hyphen segment");
            match segment.sources.first() {
                Some(first) => {
                    if single_word_key(&segment.sources).is_none() {
                        return Err(JvozbaError::AmbiguousFuivlaRafsi { stem: form });
                    }
                    pieces.push(ExpandedInput::KeptFuhivlaStem(new!(KeptFuhivlaStem {
                        form,
                        source: first.entry_index,
                        hyphen,
                    })));
                }
                None => {
                    // An unlisted fu'ivla is accepted only as the final
                    // part of a lujvo, as the word itself. Inside a lujvo
                    // it is in a non-final position, so it still names no
                    // dictionary word (`bolvii`, `gamseti`).
                    return Err(missing());
                }
            }
            continue;
        }
        if Some(index) == last_rafsi {
            match segment.sources.first() {
                Some(first) => {
                    let kind = dictionary
                        .entry_for_index(first.entry_index)
                        .map_or(FinalPartKind::Rafsi, |entry| final_part_kind(entry, &form));
                    // A name built from rafsi (`lojban`, `latlat`) ends in
                    // a CVC rafsi, which cannot end a lujvo. That part is
                    // not a final lujvo part, so it stays a dictionary
                    // piece whose final form the scorer chooses.
                    if kind == FinalPartKind::Rafsi && ends_with_consonant(&form) {
                        pieces.push(ExpandedInput::DictionaryWord(first.entry_index));
                        continue;
                    }
                    pieces.push(ExpandedInput::KeptFinalPart(new!(KeptFinalPart {
                        form,
                        kind,
                        source: first.entry_index,
                    })));
                }
                None => {
                    let Some(kind) = self_sourced_final_kind(&form, cancellation)? else {
                        return Err(missing());
                    };
                    pieces.push(ExpandedInput::SelfSourcedFinalPart(new!(
                        SelfSourcedFinalPart { form, kind }
                    )));
                }
            }
            continue;
        }
        match segment.sources.first() {
            Some(first) => pieces.push(ExpandedInput::KeptRafsi(new!(KeptRafsi {
                form,
                source: first.entry_index,
                hyphen_after,
            }))),
            None => {
                let y_hyphen = match segments.get(index + 1) {
                    Some(next) => match &next.segment {
                        LujvoPart::Hyphen(phonemes) if phonemes.as_str().starts_with('y') => {
                            Some(canonicalize_text(phonemes.as_str()))
                        }
                        _ => None,
                    },
                    None => None,
                };
                if let Some(hyphen) = y_hyphen
                    && is_long_gismu_stem(&form)
                {
                    pieces.push(ExpandedInput::PinnedLongRafsi(new!(PinnedLongRafsi {
                        stem: form,
                        hyphen,
                    })));
                } else {
                    return Err(missing());
                }
            }
        }
    }
    Ok(pieces)
}

/// How a sourced final part behaves: the full source word when the form is
/// that word, and one rafsi otherwise.
#[requires(true)]
#[ensures(true)]
fn final_part_kind(entry: &jbotci_dictionary::DictionaryEntry<'_>, form: &str) -> FinalPartKind {
    let is_full_word = normalize_lookup_query(entry.word) == normalize_lookup_query(form);
    if !is_full_word {
        return FinalPartKind::Rafsi;
    }
    if entry.word_type.is_gismu_like() {
        FinalPartKind::Gismu
    } else if word_type_is_direct_brivla(entry.word_type) {
        FinalPartKind::FullBrivla
    } else {
        FinalPartKind::Rafsi
    }
}

/// The kind of an unsourced final part, when it can end a lujvo as itself.
///
/// The morphology classifies the form alone: a gismu or a fu'ivla can end a
/// lujvo, and each is then its own source. Any other form names no word.
#[requires(true)]
#[ensures(match &ret { Ok(kind) => kind.is_none_or(|kind| kind != FinalPartKind::Rafsi), Err(_) => true })]
fn self_sourced_final_kind(
    form: &str,
    cancellation: &CancellationToken,
) -> Result<Option<FinalPartKind>, JvozbaError> {
    Ok(match word_kind_of_form(form, cancellation)? {
        Some(WordKind::Gismu) => Some(FinalPartKind::Gismu),
        Some(WordKind::Fuhivla) => Some(FinalPartKind::FullBrivla),
        _ => None,
    })
}

/// Whether `form` is the 4-letter long rafsi of some gismu.
///
/// The final vowel is unknown, so the test classifies the stem with a
/// vowel after it through the morphology shape classifier: only a gismu
/// shape passes.
#[requires(true)]
#[ensures(true)]
fn is_long_gismu_stem(form: &str) -> bool {
    form.chars().count() == 4 && GismuShape::classify(&format!("{form}a")).is_some()
}

/// Whether `form` is an unknown fu'ivla rafsi: a fu'ivla rafsi form
/// (a full fu'ivla word, or a stem that the morphology reads as a
/// fu'ivla rafsi before its hyphen) that names no dictionary fu'ivla
/// at all: not a dictionary word, not a short stem of one, and not any
/// dictionary rafsi. Such a form cannot round-trip without a
/// dictionary entry.
#[requires(true)]
#[ensures(true)]
fn is_unknown_fuhivla_form(
    dictionary: &Dictionary<'_>,
    form: &str,
    cancellation: &CancellationToken,
) -> Result<bool, JvozbaError> {
    let is_fuhivla_form = is_fuhivla_rafsi_form(form)
        || word_kind_of_form(form, cancellation)? == Some(WordKind::Fuhivla);
    Ok(is_fuhivla_form
        && fuivla_rafsi_source_set(dictionary, form).is_empty()
        && dictionary
            .lookup_rafsi(&LookupKey::from_stored(form))
            .next()
            .is_none())
}

/// The morphology kind of one word form, or `None` when it is not one word.
#[requires(true)]
#[ensures(true)]
fn word_kind_of_form(
    form: &str,
    cancellation: &CancellationToken,
) -> Result<Option<WordKind>, JvozbaError> {
    let Some(words) = segment_for_jvozba(form, cancellation)? else {
        return Ok(None);
    };
    let [word_like] = words.as_slice() else {
        return Ok(None);
    };
    Ok(word_like.bare_word().map(|word| word.kind()))
}

/// The long (4-letter) rafsi of a gismu form: the form without its final
/// vowel. The form is a gismu, so it has five letters and the first four
/// are the rafsi.
#[requires(GismuShape::classify(form).is_some())]
#[ensures(ret.chars().count() == 4)]
fn long_gismu_stem(form: &str) -> String {
    form.chars().take(4).collect()
}

/// The candidates a final part offers in each position.
///
/// A gismu form is the full word finally and its long rafsi non-finally;
/// in cmevla mode the long rafsi is the form in both positions, because a
/// cmevla ends in a consonant. A short rafsi is itself in both positions.
/// A fu'ivla form is the full word finally; non-finally a sourced part
/// offers both of its positional forms, the stem and the full word
/// (CLL 4.16), when `stem` is one of them. A self-sourced part is
/// refused non-finally in `candidate_list_for_input`, so the only
/// self-sourced candidate made here is the final full word. Both forms
/// of a sourced part are the same kept part, so the scorer still cannot
/// re-choose a different rafsi.
#[requires(!form.is_empty())]
#[ensures(!ret.is_empty())]
fn final_part_candidates(
    form: &str,
    kind: FinalPartKind,
    is_last_input: bool,
    stem: Option<String>,
    mode: JvozbaMode,
) -> Vec<LujvoBuildPart> {
    match (kind, is_last_input, mode) {
        (FinalPartKind::Gismu, _, JvozbaMode::Cmevla)
        | (FinalPartKind::Gismu, false, JvozbaMode::Lujvo) => {
            vec![new!(LujvoBuildPart::Rafsi(long_gismu_stem(form)))]
        }
        (FinalPartKind::Gismu, true, JvozbaMode::Lujvo) => {
            vec![new!(LujvoBuildPart::BrivlaCore(form.to_owned()))]
        }
        (FinalPartKind::FullBrivla, false, _) => stem
            .map(|stem| new!(LujvoBuildPart::FuhivlaStem(stem)))
            .into_iter()
            .chain(std::iter::once(new!(LujvoBuildPart::BrivlaCore(
                form.to_owned(),
            ))))
            .collect(),
        (FinalPartKind::FullBrivla, true, _) => {
            vec![new!(LujvoBuildPart::BrivlaCore(form.to_owned()))]
        }
        (FinalPartKind::Rafsi, _, _) => vec![new!(LujvoBuildPart::Rafsi(form.to_owned()))],
    }
}

/// The stem a sourced kept fu'ivla final part may use non-finally, when
/// it still names the same word: the dictionary stem that names its
/// entry. A self-sourced part has no stem candidate at all.
#[requires(true)]
#[ensures(true)]
fn kept_fuivla_stem(dictionary: &Dictionary<'_>, form: &str, source: EntryIndex) -> Option<String> {
    let stem = fuivla_stem_rafsi(form)?;
    fuivla_rafsi_names_entry(dictionary, &stem, source).then_some(stem)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|lists| lists.len() == inputs.len()) || ret.is_err())]
fn build_candidate_lists(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    inputs: &[ExpandedInput<'_>],
    cancellation: &CancellationToken,
) -> Result<Vec<Vec<LujvoBuildPart>>, JvozbaError> {
    let total_count = inputs.len();
    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            candidate_list_for_input(
                mode,
                dictionary,
                index + 1 == total_count,
                input.clone(),
                cancellation,
            )
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|candidates| !candidates.is_empty()) || ret.is_err())]
fn candidate_list_for_input(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    is_last_input: bool,
    input: ExpandedInput<'_>,
    cancellation: &CancellationToken,
) -> Result<Vec<LujvoBuildPart>, JvozbaError> {
    match input {
        ExpandedInput::FixedRafsi(typed) => {
            // A typed rafsi goes through the same typed-query rule as a typed
            // word, so `--rafsi кла` reads as `kla`, and `--rafsi klâ` is an
            // error that names `â`.
            let rafsi_text = match LookupKey::from_typed(typed) {
                Ok(key) => key.as_str().to_owned(),
                Err(error) => {
                    return Err(
                        typed_input_error(typed, &error).unwrap_or(JvozbaError::FixedRafsiEmpty)
                    );
                }
            };
            if rafsi_text.is_empty() {
                return Err(JvozbaError::FixedRafsiEmpty);
            }
            if !is_last_input && is_fixed_universal_long_gismu_rafsi(dictionary, &rafsi_text) {
                return Err(JvozbaError::NonFinalUniversalLongRafsi {
                    offending: rafsi_text,
                });
            }
            // A pinned rafsi whose form is an unknown fu'ivla is refused
            // non-finally, like any other unlisted fu'ivla: without a
            // dictionary entry it cannot round-trip reliably. A form
            // that names a dictionary fu'ivla (`actce` names `actce'o`
            // by its stem) is known, and it still works.
            if !is_last_input && is_unknown_fuhivla_form(dictionary, &rafsi_text, cancellation)? {
                return Err(JvozbaError::NoDictionaryEntry {
                    offending: rafsi_text,
                });
            }
            // A literal fu'ivla rafsi takes its `y` or `'y` hyphen by rule, not by the
            // consonant pair rule, so `--rafsi tci'il` builds like `tci'ile`.
            // The literal form stays a candidate too, and the score and the
            // parse check choose between them.
            let stem = (!is_last_input && is_fuhivla_rafsi_form(&rafsi_text))
                .then(|| new!(LujvoBuildPart::FuhivlaStem(rafsi_text.clone())));
            Ok(std::iter::once(new!(LujvoBuildPart::Rafsi(rafsi_text)))
                .chain(stem)
                .collect())
        }
        ExpandedInput::DictionaryWord(index) => {
            let entry = dictionary
                .entry_for_index(index)
                .expect("resolved entry index");
            candidate_list_for_entry(mode, dictionary, is_last_input, index, entry)
        }
        ExpandedInput::KeptRafsi(kept) => {
            let data!(KeptRafsi {
                form,
                hyphen_after,
                ..
            }) = kept.into_data();
            // A hyphen the input wrote is prescribed, so this joint is
            // copied as the input wrote it. Without one, the builder
            // chooses the hyphen for the joint.
            Ok(vec![match hyphen_after {
                Some(hyphen) => new!(LujvoBuildPart::RafsiWithHyphen { text: form, hyphen }),
                None => new!(LujvoBuildPart::Rafsi(form)),
            }])
        }
        ExpandedInput::KeptFuhivlaStem(kept) => {
            let data!(KeptFuhivlaStem { form, hyphen, .. }) = kept.into_data();
            Ok(vec![new!(LujvoBuildPart::RafsiWithHyphen {
                text: form,
                hyphen,
            })])
        }
        ExpandedInput::KeptFinalPart(kept) => {
            let data!(KeptFinalPart { form, kind, source }) = kept.into_data();
            let stem = if kind == FinalPartKind::FullBrivla && !is_last_input {
                kept_fuivla_stem(dictionary, &form, source)
            } else {
                None
            };
            Ok(final_part_candidates(
                &form,
                kind,
                is_last_input,
                stem,
                mode,
            ))
        }
        ExpandedInput::SelfSourcedFinalPart(part) => {
            let data!(SelfSourcedFinalPart { form, kind }) = part.into_data();
            // An unlisted fu'ivla is accepted only as the final part, as
            // the word itself. In a non-final position it is refused as a
            // word with no dictionary entry.
            if kind == FinalPartKind::FullBrivla && !is_last_input {
                return Err(JvozbaError::NoDictionaryEntry { offending: form });
            }
            Ok(final_part_candidates(
                &form,
                kind,
                is_last_input,
                None,
                mode,
            ))
        }
        ExpandedInput::PinnedLongRafsi(pinned) => {
            let data!(PinnedLongRafsi { stem, hyphen }) = pinned.into_data();
            Ok(vec![new!(LujvoBuildPart::RafsiWithHyphen {
                text: stem,
                hyphen,
            })])
        }
        ExpandedInput::UnlistedWord(word_text) => {
            let canonical_word = normalized_spelling(word_text);
            let offending = offending_spelling(word_text, &canonical_word);
            non_dictionary_word_candidates(mode, is_last_input, &canonical_word, cancellation)?
                .ok_or(JvozbaError::NoDictionaryEntry { offending })
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|candidates| !candidates.is_empty()) || ret.is_err())]
fn candidate_list_for_entry(
    mode: JvozbaMode,
    dictionary: &Dictionary<'_>,
    is_last_input: bool,
    entry_index: EntryIndex,
    entry: &jbotci_dictionary::DictionaryEntry<'_>,
) -> Result<Vec<LujvoBuildPart>, JvozbaError> {
    let canonical_word = normalized_spelling(entry.word);
    // A fu'ivla keeps the same forms in both modes: the full word, and
    // non-finally its stem when the stem still names this word.
    let stem = if !is_last_input && entry.word_type.is_fuivla_like() {
        fuivla_stem_rafsi(entry.word)
    } else {
        None
    };
    let listed_rafsi = entry
        .rafsi
        .iter()
        .map(|rafsi| normalized_spelling(rafsi.form))
        .filter(|rafsi| stem.as_deref() != Some(rafsi))
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
    // A stored headword can normalize to nothing, for example a lone `.`.
    // Such a word has no full form, so it adds no full-word candidate. Its
    // listed rafsi stay usable.
    let has_full_word = !canonical_word.is_empty();
    if is_last_input
        && has_full_word
        && word_type_allows_final_brivla_core(entry.word_type)
        && (mode == JvozbaMode::Lujvo || entry.word_type.is_fuivla_like())
    {
        all_candidates.push(new!(LujvoBuildPart::BrivlaCore(canonical_word.clone())));
    }
    if !is_last_input && entry.word_type.is_fuivla_like() {
        // The full form stays in the candidate list when it names only this
        // word. A shorter valid rafsi has the better score and wins.
        if has_full_word && fuivla_rafsi_names_entry(dictionary, &canonical_word, entry_index) {
            all_candidates.push(new!(LujvoBuildPart::BrivlaCore(canonical_word.clone())));
        }
        if let Some(stem) = stem
            && fuivla_rafsi_names_entry(dictionary, &stem, entry_index)
        {
            all_candidates.push(new!(LujvoBuildPart::FuhivlaStem(stem)));
        }
    }
    let all_candidates = unique_preserving_order(all_candidates);
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
        // The error names the stored spelling when normalization leaves nothing.
        let offending = offending_spelling(entry.word, &canonical_word);
        if mode == JvozbaMode::Lujvo && is_last_input {
            Err(JvozbaError::FinalConsonant {
                offending,
                is_fixed_rafsi: false,
            })
        } else {
            Err(JvozbaError::NoRafsiAvailable { offending })
        }
    } else {
        Ok(candidates)
    }
}

#[requires(true)]
#[ensures(match &ret {
    Ok(Some(candidates)) => !candidates.is_empty(),
    Ok(None) | Err(_) => true,
})]
fn non_dictionary_word_candidates(
    mode: JvozbaMode,
    is_last_input: bool,
    canonical_word: &str,
    cancellation: &CancellationToken,
) -> Result<Option<Vec<LujvoBuildPart>>, JvozbaError> {
    let Some(words) = segment_for_jvozba(canonical_word, cancellation)? else {
        return Ok(None);
    };
    let [word_like] = words.as_slice() else {
        return Ok(None);
    };
    let Some(word) = word_like.bare_word() else {
        return Ok(None);
    };
    // In a build, a lujvo never comes here, because it always expands into
    // its pieces (see `decomposition_pieces`). The pane admission test does
    // send one here: it gets no candidate, and `is_morphology_lujvo` then
    // decides. Only a gismu or a fu'ivla has a form that is predictable
    // without the dictionary: the full word finally, the long rafsi of a
    // gismu non-finally and in cmevla mode, and the full fu'ivla in the
    // final position (an unlisted fu'ivla is accepted only there).
    let full = canonicalize_text(word.phonemes().as_str());
    let candidate = match word.kind() {
        WordKind::Gismu if mode == JvozbaMode::Cmevla || !is_last_input => {
            new!(LujvoBuildPart::Rafsi(long_gismu_stem(&full)))
        }
        WordKind::Gismu => new!(LujvoBuildPart::BrivlaCore(full)),
        // An unlisted fu'ivla is accepted only as the final part, as the
        // word itself; a non-final one has no candidate here and is
        // refused as a word with no dictionary entry by the caller.
        WordKind::Fuhivla if is_last_input => new!(LujvoBuildPart::BrivlaCore(full)),
        _ => return Ok(None),
    };
    Ok(Some(vec![candidate]))
}

/// Segment one user piece. An interrupt is not "this is not a word".
#[requires(true)]
#[ensures(true)]
fn segment_for_jvozba(
    text: &str,
    cancellation: &CancellationToken,
) -> Result<Option<Vec<WordLike>>, JvozbaError> {
    let options = MorphologyOptions::default().with_cancellation(cancellation);
    match segment_words_with_modifiers_with_options_and_source_id(text, &options, None) {
        Ok(words) => Ok(Some(words)),
        Err(error) if error.is_interrupted() => Err(JvozbaError::Interrupted),
        Err(_) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
fn unique_preserving_order<T: PartialEq>(values: Vec<T>) -> Vec<T> {
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
        && dictionary
            .lookup_word(&LookupKey::from_stored(rafsi_text))
            .is_some_and(|entry| {
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
    normalized_spelling(raw_word)
}

#[requires(true)]
#[ensures(true)]
fn normalized_spelling(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('.');
    normalize_lojban_input_text(trimmed).unwrap_or_else(|| canonicalize_text(trimmed))
}

#[requires(true)]
#[ensures(!raw.trim().is_empty() -> !ret.is_empty())]
#[ensures(!normalized.is_empty() -> ret == normalized)]
fn offending_spelling(raw: &str, normalized: &str) -> String {
    if !normalized.is_empty() {
        return normalized.to_owned();
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        normalized.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_lujvo_parts(
    normalized: &str,
    cancellation: &CancellationToken,
) -> Result<Option<Vec<LujvoPart>>, JvozbaError> {
    // `parse_lujvo_word_parts` classifies through a cache that cannot see this
    // flag. The shared segmenter is the morphology this piece is allowed to run.
    let Some(words) = segment_for_jvozba(normalized, cancellation)? else {
        return Ok(None);
    };
    let [word_like] = words.as_slice() else {
        return Ok(None);
    };
    let Some(word) = word_like.bare_word() else {
        return Ok(None);
    };
    Ok(word
        .lujvo_parts()
        .map(|parts| parts.iter().cloned().collect()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|decomposition| decomposition.segments.iter().filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_))).count() >= 2))]
fn decomposition_from_parts<'a>(
    dictionary: &Dictionary<'a>,
    parts: Vec<LujvoPart>,
) -> Option<LujvoDecomposition<'a>> {
    let mut remaining = parts.into_iter().peekable();
    let segments = std::iter::from_fn(|| {
        let part = remaining.next()?;
        Some(segment_with_sources(dictionary, part, remaining.peek()))
    })
    .collect::<Vec<_>>();
    let rafsi_count = segments
        .iter()
        .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
        .count();
    if rafsi_count < 2 {
        return None;
    }

    let source_words = unique_preserving_order(
        segments
            .iter()
            .flat_map(|segment| segment.sources.iter().map(|source| source.word))
            .collect::<Vec<_>>(),
    );
    Some(new!(LujvoDecomposition {
        segments: segments,
        source_words: source_words,
    }))
}

#[requires(true)]
#[ensures(true)]
fn all_rafsi_segments_have_sources(decomposition: &LujvoDecomposition<'_>) -> bool {
    decomposition
        .segments
        .iter()
        .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
        .all(|segment| !segment.sources.is_empty())
}

#[requires(true)]
#[ensures(true)]
fn is_cmevla_word(normalized: &str, cancellation: &CancellationToken) -> Result<bool, JvozbaError> {
    let Some(words) = segment_for_jvozba(normalized, cancellation)? else {
        return Ok(false);
    };
    let [word_like] = words.as_slice() else {
        return Ok(false);
    };
    let Some(word) = word_like.bare_word() else {
        return Ok(false);
    };
    Ok(word.kind() == WordKind::Cmevla)
}

#[requires(true)]
#[ensures(true)]
fn segment_with_sources<'a>(
    dictionary: &Dictionary<'a>,
    segment: LujvoPart,
    next: Option<&LujvoPart>,
) -> LujvoSegmentInfo<'a> {
    let (kind, sources) = match &segment {
        LujvoPart::Hyphen(_) => (DictionaryLujvoSegmentKind::Hyphen, Vec::new()),
        LujvoPart::Rafsi(phonemes) if segment.is_fuhivla_rafsi_before(next) => (
            DictionaryLujvoSegmentKind::FuivlaRafsi,
            fuivla_rafsi_source_set(dictionary, phonemes.as_str()),
        ),
        LujvoPart::Rafsi(phonemes) => (
            DictionaryLujvoSegmentKind::Rafsi,
            primary_rafsi_source(dictionary, phonemes)
                .into_iter()
                .collect(),
        ),
    };
    new!(LujvoSegmentInfo {
        segment,
        kind,
        sources
    })
}

#[requires(true)]
#[ensures(true)]
fn primary_rafsi_source<'a>(
    dictionary: &Dictionary<'a>,
    phonemes: &Phonemes,
) -> Option<DictionaryLujvoSource<'a>> {
    let surface = phonemes.as_str();
    if let Some(matched) = dictionary
        .lookup_rafsi(&LookupKey::from_stored(surface))
        .find(|matched| matched.source != RafsiSource::FuivlaStem)
    {
        return Some(DictionaryLujvoSource {
            entry_index: matched.entry_index,
            word: matched.entry.word,
        });
    }
    let entry_index = dictionary.lookup_word_index(&LookupKey::from_stored(surface))?;
    let entry = dictionary.entry_for_index(entry_index)?;
    word_type_is_direct_brivla(entry.word_type).then_some(DictionaryLujvoSource {
        entry_index,
        word: entry.word,
    })
}

/// The fu'ivla that a fu'ivla rafsi part comes from, ranked by score from
/// high to low. Entries with the same score stay in source order, because
/// `lookup_words` and `lookup_rafsi` rank their results that way.
///
/// CLL 4.16 gives two forms. The full form is a fu'ivla itself before `'y`,
/// and it names that fu'ivla alone (owner rule, #1136): when fu'ivla entries
/// have the part's lookup key, they are the sources. Otherwise the sources
/// are the short forms, the derived stems in the rafsi index (`ierx` from
/// `ierxi`, `actce` from `actce'o`). Two short forms can still share a part,
/// and then the part names no single word.
#[requires(true)]
#[ensures(ret.iter().all(|source| source.entry_index.0 < dictionary.entries().len()))]
#[ensures(ret.windows(2).all(|pair| {
    let order = dictionary.entries()[pair[0].entry_index.0]
        .score
        .get()
        .total_cmp(&dictionary.entries()[pair[1].entry_index.0].score.get());
    order.is_gt() || (order.is_eq() && pair[0].entry_index < pair[1].entry_index)
}), "the sources are ranked by score, then by source order")]
fn fuivla_rafsi_source_set<'a>(
    dictionary: &Dictionary<'a>,
    part: &str,
) -> Vec<DictionaryLujvoSource<'a>> {
    let full_forms = dictionary
        .lookup_words(&LookupKey::from_stored(part))
        .filter(|entry| entry.word_type.is_fuivla_like())
        .filter_map(|entry| {
            dictionary
                .entry_index_for_entry(entry)
                .map(|entry_index| DictionaryLujvoSource {
                    entry_index,
                    word: entry.word,
                })
        })
        .collect::<Vec<_>>();
    if !full_forms.is_empty() {
        return full_forms;
    }
    dictionary
        .lookup_rafsi(&LookupKey::from_stored(part))
        .filter(|matched| matched.source == RafsiSource::FuivlaStem)
        .map(|matched| DictionaryLujvoSource {
            entry_index: matched.entry_index,
            word: matched.entry.word,
        })
        .collect()
}

/// The lookup key of the one word that a set of fu'ivla rafsi sources names.
///
/// Entries that share a lookup key are one word, for example `onji` and
/// `.onji`. The part is ambiguous when its sources have more than one key:
/// two fu'ivla whose short forms are equal, such as `eska` and `esku`, which
/// both shorten to `esk`. Then the part names no word, and jvozba does not
/// offer it. A full form never has this problem, because it names only the
/// entries with its own key.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn single_word_key(sources: &[DictionaryLujvoSource<'_>]) -> Option<String> {
    let mut keys = sources
        .iter()
        .map(|source| normalize_lookup_query(source.word));
    let first = keys.next().filter(|key| !key.is_empty())?;
    keys.all(|key| key == first).then_some(first)
}

/// True when the fu'ivla rafsi part names exactly the word of `expected`.
///
/// One source set answers both questions: whether the part names one word,
/// and whether that word includes `expected`.
#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret -> fuivla_rafsi_source_set(dictionary, part).iter().any(|source| source.entry_index == expected))]
fn fuivla_rafsi_names_entry(dictionary: &Dictionary<'_>, part: &str, expected: EntryIndex) -> bool {
    let sources = fuivla_rafsi_source_set(dictionary, part);
    single_word_key(&sources).is_some()
        && sources.iter().any(|source| source.entry_index == expected)
}

/// True when the sources of an ordinary rafsi part include the entry at
/// `expected`.
///
/// The sources are the entries that list or derive the form as a rafsi. When
/// no entry does, the part is a full word, and its sources are the direct
/// brivla entries with that lookup key. The part does not have to rank the
/// entry first: `lengu` and `lenga` share the form `leng`, so `lengykla` is a
/// valid build from `lengu`.
#[requires(true)]
#[ensures(true)]
fn rafsi_part_names_entry(
    dictionary: &Dictionary<'_>,
    phonemes: &Phonemes,
    expected: EntryIndex,
) -> bool {
    let surface = phonemes.as_str();
    let mut rafsi_sources = dictionary
        .lookup_rafsi(&LookupKey::from_stored(surface))
        .filter(|matched| matched.source != RafsiSource::FuivlaStem)
        .peekable();
    if rafsi_sources.peek().is_some() {
        return rafsi_sources.any(|matched| matched.entry_index == expected);
    }
    dictionary.entry_for_index(expected).is_some_and(|entry| {
        word_type_is_direct_brivla(entry.word_type)
            && normalize_lookup_query(entry.word) == normalize_lookup_query(surface)
    })
}

/// True when every rafsi part of `parsed` names its input.
///
/// A parsed part names a dictionary input when its sources include that
/// entry (see `rafsi_part_names_entry` and `fuivla_rafsi_names_entry`).
#[requires(true)]
#[ensures(true)]
fn sources_match(
    dictionary: &Dictionary<'_>,
    inputs: &[ExpandedInput<'_>],
    parsed: &[LujvoPart],
) -> bool {
    let mut inputs = inputs.iter();
    for (index, part) in parsed.iter().enumerate() {
        let LujvoPart::Rafsi(phonemes) = part else {
            continue;
        };
        let names_entry = |expected: EntryIndex| {
            if part.is_fuhivla_rafsi_before(parsed.get(index + 1)) {
                fuivla_rafsi_names_entry(dictionary, phonemes.as_str(), expected)
            } else {
                rafsi_part_names_entry(dictionary, phonemes, expected)
            }
        };
        // A parsed part can carry stress marks (`ĭísmu`), while a pinned
        // form is canonical text, so the pinned comparisons canonicalize.
        let surface = canonicalize_text(phonemes.as_str());
        match inputs.next() {
            Some(ExpandedInput::DictionaryWord(expected)) => {
                if !names_entry(*expected) {
                    return false;
                }
            }
            Some(ExpandedInput::KeptRafsi(kept)) => {
                let pinned = surface == kept.form.as_str() || names_entry(kept.source);
                if !pinned {
                    return false;
                }
            }
            Some(ExpandedInput::KeptFuhivlaStem(kept)) => {
                let pinned = surface == kept.form.as_str() || names_entry(kept.source);
                if !pinned {
                    return false;
                }
            }
            Some(ExpandedInput::KeptFinalPart(kept)) => {
                let pinned = surface == kept.form.as_str()
                    || (kept.kind == FinalPartKind::Gismu
                        && surface == long_gismu_stem(&kept.form))
                    || names_entry(kept.source);
                if !pinned {
                    return false;
                }
            }
            Some(ExpandedInput::SelfSourcedFinalPart(part_form)) => {
                let pinned = surface == part_form.form.as_str()
                    || (part_form.kind == FinalPartKind::Gismu
                        && surface == long_gismu_stem(&part_form.form));
                if !pinned {
                    return false;
                }
            }
            Some(ExpandedInput::PinnedLongRafsi(pinned)) => {
                if surface != pinned.stem.as_str() {
                    return false;
                }
            }
            Some(ExpandedInput::FixedRafsi(_) | ExpandedInput::UnlistedWord(_)) => {}
            None => return false,
        }
    }
    inputs.next().is_none()
}

#[requires(true)]
#[ensures(ret.iter().all(|parts| !parts.is_empty()))]
fn fallback_lujvo_part_candidates(normalized: &str) -> Vec<Vec<LujvoPart>> {
    parse_cmevla_lujvo_word_part_candidates(normalized)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;
    use jbotci_interrupt::assert_exit_check;
    use jbotci_morphology::normalize_latin_apostrophe;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_fixed_rafsi_that_canonicalizes_to_nothing_is_empty() {
        let dictionary = jbotci_dictionary_data::english();
        for rafsi in [",", ",,", "\u{0301}"] {
            let error = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::FixedRafsi(rafsi.to_owned()),
                    JvozbaInput::Word("klama".to_owned()),
                ],
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect_err("a rafsi with no letters");
            assert_eq!(error, JvozbaError::FixedRafsiEmpty, "{rafsi:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_typed_character_that_gentufa_rejects_is_named() {
        // A typed word or fixed rafsi is keyed with the typed-query rule, so a
        // non-stress diacritic or a symbol is an error that names it, as
        // vlacku reports it. The index rule, which would read `klâma` as
        // `klama`, is only for stored headwords.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        for (inputs, typed, character) in [
            (vec![word("klâma"), word("gasnu")], "klâma", 'â'),
            (vec![word("çmene"), word("gasnu")], "çmene", 'ç'),
            (vec![word("klämä"), word("gasnu")], "klämä", 'ä'),
            (vec![word("kľama"), word("gasnu")], "kľama", 'ľ'),
            (vec![word("gâsnu"), word("klâma")], "gâsnu", 'â'),
            (vec![word("klama#"), word("gasnu")], "klama#", '#'),
            (vec![word("klaqma"), word("gasnu")], "klaqma", 'q'),
            (vec![rafsi("klâ"), word("gasnu")], "klâ", 'â'),
        ] {
            let error = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token)
                .expect_err("a rejected character");
            assert_eq!(
                error,
                JvozbaError::InvalidCharacter {
                    word: typed.to_owned(),
                    character,
                },
                "{inputs:?}"
            );
        }
        assert_eq!(
            build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[word("klâma"), word("gasnu")],
                &token
            )
            .unwrap_err()
            .to_string(),
            "Invalid Lojban word: klâma (unsupported character `â`)."
        );
        // The decomposed spelling is rejected too.
        let decomposed = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[word("kla\u{0302}ma"), word("gasnu")],
            &token,
        )
        .expect_err("a non-stress mark");
        assert!(
            matches!(decomposed, JvozbaError::InvalidCharacter { .. }),
            "{decomposed:?}"
        );
        // A fixed rafsi in another script converts as a word does.
        for inputs in [
            vec![word("клама"), word("gasnu")],
            vec![rafsi("кла"), word("gasnu")],
        ] {
            let result = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token)
                .unwrap_or_else(|error| panic!("{inputs:?}: {error}"));
            assert_eq!(result.word, "klagau", "{inputs:?}");
        }
        // The pane does not admit such a word.
        assert_eq!(
            word_can_enter_jvozba_pane(dictionary, "klâma", &token),
            Ok(false)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_word_that_canonicalizes_to_nothing_has_no_dictionary_entry() {
        let dictionary = jbotci_dictionary_data::english();
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word(",".to_owned()),
                JvozbaInput::Word("klama".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect_err("a comma is not a dictionary word");
        assert_eq!(
            error,
            JvozbaError::NoDictionaryEntry {
                offending: ",".to_owned()
            }
        );
        assert_eq!(error.to_string(), "No dictionary entry for `,`.");
        // Spaces around removed marks do not make a name, and a word of
        // spaces only is empty.
        for (word, expected) in [
            (
                " , ",
                JvozbaError::NoDictionaryEntry {
                    offending: ",".to_owned(),
                },
            ),
            (
                " \u{0301} ",
                JvozbaError::NoDictionaryEntry {
                    offending: "\u{0301}".to_owned(),
                },
            ),
            ("   ", JvozbaError::WordEmpty),
            ("", JvozbaError::WordEmpty),
        ] {
            let error = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::Word(word.to_owned()),
                    JvozbaInput::Word("klama".to_owned()),
                ],
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect_err("no letters");
            assert_eq!(error, expected, "{word:?}");
        }
    }

    /// The figures the Discord service uses, so the boundaries these tests
    /// pin are the ones a caller with a deadline actually applies.
    #[requires(true)]
    #[ensures(true)]
    fn bounded() -> JvozbaBuildLimits {
        new!(JvozbaBuildLimits {
            max_pieces: 24,
            max_placements: 8192,
            max_spelling_letters: 256,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_bounded_build_refuses_an_oversized_search_before_it_starts() {
        let dictionary = jbotci_dictionary_data::english();
        // Eight plain words are inside the limit and still build; a `klama`
        // chain crosses it at the ninth (4096 placements against 9216).
        let ordinary = (0..8)
            .map(|_| JvozbaInput::Word("klama".to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            build_best_jvozba_detailed_within(
                JvozbaMode::Lujvo,
                dictionary,
                &ordinary,
                bounded(),
                &jbotci_interrupt::CancellationToken::never()
            )
            .expect("inside the limits")
            .word,
            "klaklaklaklaklaklaklakla"
        );
        assert!(matches!(
            build_best_jvozba_detailed_within(
                JvozbaMode::Lujvo,
                dictionary,
                &(0..9)
                    .map(|_| JvozbaInput::Word("klama".to_owned()))
                    .collect::<Vec<_>>(),
                bounded(),
                &jbotci_interrupt::CancellationToken::never()
            ),
            Err(JvozbaError::TooMuchWork {
                measure: JvozbaWorkMeasure::Placements,
                ..
            })
        ));

        // Sixty-four are refused before any spelling is compared.
        let explosive = (0..64)
            .map(|_| JvozbaInput::Word("klama".to_owned()))
            .collect::<Vec<_>>();
        let started = std::time::Instant::now();
        let error = build_best_jvozba_detailed_within(
            JvozbaMode::Lujvo,
            dictionary,
            &explosive,
            bounded(),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect_err("above the limits");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the refusal does not depend on searching"
        );
        let message = error.to_string();
        assert!(message.contains("pieces to combine"), "{message}");
        assert!(message.contains("Use fewer or shorter pieces"), "{message}");

        // Inputs that only grow once dictionary lujvo expand are refused on
        // the expanded count, not the count the user typed.
        let expanding = (0..20)
            .map(|_| JvozbaInput::Word("jbobau".to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            build_best_jvozba_detailed_within(
                JvozbaMode::Cmevla,
                dictionary,
                &expanding,
                bounded(),
                &jbotci_interrupt::CancellationToken::never()
            ),
            Err(JvozbaError::TooMuchWork {
                measure: JvozbaWorkMeasure::Pieces,
                amount: 40,
                limit: 24,
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn each_limit_measures_its_own_part_of_the_work() {
        let piece = |text: &str| new!(LujvoBuildPart::Rafsi(text.to_owned()));
        let single = vec![vec![piece("kla")], vec![piece("bajra")]];
        assert!(!placements_exceed_limit(&single, 2));
        assert_eq!(longest_spelling_letters(&single), Some(9));

        // Three pieces of two candidates each: 8 spellings, 24 placements.
        let branching = vec![vec![piece("kla"), piece("klam")]; 3];
        assert!(!placements_exceed_limit(&branching, 24));
        assert!(placements_exceed_limit(&branching, 23));
        // The longest spelling counts the longest candidate of every piece
        // plus one hyphen per junction.
        assert_eq!(longest_spelling_letters(&branching), Some(14));

        // A space too large to count answers without overflowing.
        let explosive = vec![vec![piece("kla"), piece("klam")]; 80];
        assert!(placements_exceed_limit(&explosive, u64::MAX - 1));

        // Few choices can still mean a long spelling, which is refused on
        // its own measure.
        let dictionary = jbotci_dictionary_data::english();
        let long = build_best_jvozba_detailed_within(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("klama".to_owned()),
                JvozbaInput::Word("bajra".to_owned()),
            ],
            new!(JvozbaBuildLimits {
                max_pieces: 24,
                max_placements: 8192,
                max_spelling_letters: 4,
            }),
            &jbotci_interrupt::CancellationToken::never(),
        );
        assert!(
            matches!(
                long,
                Err(JvozbaError::TooMuchWork {
                    measure: JvozbaWorkMeasure::SpellingLetters,
                    ..
                })
            ),
            "{long:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_unbounded_api_still_builds_what_a_bounded_caller_refuses() {
        // Twenty-five fixed rafsi are one spelling, so the search is cheap
        // however many pieces there are: the bounded caller still refuses it
        // on its piece count, and the long-standing entry point still builds
        // it, which is the behaviour that must not change.
        let dictionary = jbotci_dictionary_data::english();
        let pieces = (0..25)
            .map(|_| JvozbaInput::FixedRafsi("kla".to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            build_best_jvozba_detailed_within(
                JvozbaMode::Lujvo,
                dictionary,
                &pieces,
                bounded(),
                &jbotci_interrupt::CancellationToken::never()
            ),
            Err(JvozbaError::TooMuchWork {
                measure: JvozbaWorkMeasure::Pieces,
                amount: 25,
                limit: 24,
            })
        );
        assert_eq!(
            build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &pieces,
                &jbotci_interrupt::CancellationToken::never()
            )
            .expect("no caller limit")
            .word,
            "kla".repeat(25)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unbounded_build_refuses_a_search_it_cannot_even_count() {
        // No caller limit still leaves the arithmetic one: eighty ordinary
        // words offer more spellings than a 64-bit count can hold, and a
        // search whose size cannot be counted cannot be finished either. The
        // refusal is therefore part of the unbounded API, not only of the
        // bounded one, which is why every caller of it must handle it.
        let dictionary = jbotci_dictionary_data::english();
        let words = (0..80)
            .map(|_| JvozbaInput::Word("klama".to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &words,
                &jbotci_interrupt::CancellationToken::never()
            ),
            Err(JvozbaError::TooMuchWork {
                measure: JvozbaWorkMeasure::Placements,
                amount: u64::MAX,
                limit: u64::MAX,
            })
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_source_dictionary(
        entries: &[jbotci_dictionary::DictionaryEntry<'_>],
        test: impl FnOnce(&Dictionary<'_>),
    ) {
        use jbotci_dictionary::{RafsiIndexEntry, WordIndexEntry, build_owned_indexes};

        let indexes = build_owned_indexes(entries);
        let words = indexes
            .word_index
            .iter()
            .map(|entry| WordIndexEntry {
                key: &entry.key,
                targets: &entry.targets,
            })
            .collect::<Vec<_>>();
        let rafsi = indexes
            .rafsi_index
            .iter()
            .map(|entry| RafsiIndexEntry {
                key: &entry.key,
                targets: &entry.targets,
            })
            .collect::<Vec<_>>();
        let dictionary =
            Dictionary::from_static_slices(entries, &words, &rafsi, &[], &[], &[], &[], 0);
        test(&dictionary);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposition_and_expansion_keep_the_original_rafsi_entry() {
        use jbotci_dictionary::{DictionaryEntry, Rafsi, RafsiClaimKind, Score};

        let english = jbotci_dictionary_data::english();
        let listed = [Rafsi {
            form: "dat",
            standing: RafsiClaimKind::Official,
        }];
        let datni = DictionaryEntry {
            rafsi: &listed,
            ..*english
                .lookup_word(&LookupKey::from_stored("datni"))
                .unwrap()
        };
        let dat_ni = DictionaryEntry {
            word: "dat,ni",
            rafsi: &[],
            score: Score(datni.score.get() + 1.0),
            ..datni
        };
        let sidbo = *english
            .lookup_word(&LookupKey::from_stored("sidbo"))
            .unwrap();
        let token = CancellationToken::never();
        with_source_dictionary(&[datni, dat_ni, sidbo], |dictionary| {
            assert_eq!(
                dictionary
                    .lookup_word(&LookupKey::from_stored("datni"))
                    .unwrap()
                    .word,
                "dat,ni"
            );
            let decomposition = decompose_lujvo_like(dictionary, "datsi'o", &token)
                .unwrap()
                .unwrap();
            let source = &decomposition.segments[0].sources[0];
            assert_eq!(source.entry_index, EntryIndex(0));
            assert_eq!(source.word, "datni");
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[JvozbaInput::Word("datsi'o".to_owned())],
                &token,
            )
            .unwrap();
            assert_eq!(result.word, "datsi'o");
            let selected = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::DictionaryWord {
                        entry_index: EntryIndex(0),
                        word: "datni".to_owned(),
                    },
                    JvozbaInput::Word("sidbo".to_owned()),
                ],
                &token,
            )
            .unwrap();
            assert_eq!(selected.word, "datsi'o");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_listed_rafsi_must_still_name_its_source_after_parsing() {
        use jbotci_dictionary::{DictionaryEntry, Rafsi, RafsiClaimKind};

        let english = jbotci_dictionary_data::english();
        let listed = [Rafsi {
            form: "ierx",
            standing: RafsiClaimKind::Experimental,
        }];
        let bahe = DictionaryEntry {
            rafsi: &listed,
            ..*english
                .lookup_word(&LookupKey::from_stored("ba'e"))
                .unwrap()
        };
        let ierxi = *english
            .lookup_word(&LookupKey::from_stored("ierxi"))
            .unwrap();
        let ierxa = DictionaryEntry {
            word: "ierxa",
            rafsi: &[],
            ..ierxi
        };
        let klama = *english
            .lookup_word(&LookupKey::from_stored("klama"))
            .unwrap();
        let token = CancellationToken::never();
        with_source_dictionary(&[bahe, ierxi, ierxa, klama], |dictionary| {
            let final_piece = JvozbaInput::FixedRafsi("kla".to_owned());
            assert_eq!(
                build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[JvozbaInput::Word("ba'e".to_owned()), final_piece.clone()],
                    &token
                ),
                Err(JvozbaError::CouldNotBuildLujvo),
            );
            let literal = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[JvozbaInput::FixedRafsi("ierx".to_owned()), final_piece],
                &token,
            )
            .unwrap();
            assert_eq!(literal.word, "ierxykla");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_shared_universal_rafsi_names_every_word_that_shares_it() {
        use jbotci_dictionary::{DictionaryEntry, Score};

        // `klam` is a universal form of both words. Its sources include the
        // lower-ranked `klami`, so `klamybajra` is a valid build from it.
        let english = jbotci_dictionary_data::english();
        let klama = DictionaryEntry {
            rafsi: &[],
            score: Score(2.0),
            ..*english
                .lookup_word(&LookupKey::from_stored("klama"))
                .unwrap()
        };
        let klami = DictionaryEntry {
            word: "klami",
            score: Score(1.0),
            ..klama
        };
        let token = CancellationToken::never();
        with_source_dictionary(&[klami, klama], |dictionary| {
            for word in ["klami", "klama"] {
                let result = build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[
                        JvozbaInput::Word(word.to_owned()),
                        JvozbaInput::FixedRafsi("bajra".to_owned()),
                    ],
                    &token,
                )
                .unwrap();
                assert_eq!(result.word, "klamybajra", "{word}");
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_lower_ranked_gismu_builds_through_a_shared_four_letter_form() {
        // `lenga` and `lengu` share `leng`, and `lenga` ranks higher. Main
        // built `lengykla` from `lengu`, and so does this rule.
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("lengu".to_owned()),
                JvozbaInput::Word("klama".to_owned()),
            ],
            &CancellationToken::never(),
        )
        .unwrap();
        assert_eq!(result.word, "lengykla");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuivla_rafsi_parts_follow_cll_4_16() {
        // Each row is a lujvo, its first part, and the fu'ivla that part comes
        // from in the shipped dictionary. The full form is the fu'ivla before
        // `'y`. The short form drops one final vowel, and an apostrophe right
        // before it. A diphthong-final fu'ivla has no short form, so `adz`
        // and `adza` are not stems of `adzau`.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for (lujvo, part, sources) in [
            ("spageti'ykukte", "spageti", &["spageti"][..]),
            // The full form names `enri` alone. `enrio` would shorten to
            // `enri`, which is that full form, so it has no short form.
            ("enri'ykukte", "enri", &["enri"][..]),
            ("enrykukte", "enr", &["enri"][..]),
            ("actce'ybau", "actce", &["actce'o"][..]),
            ("adzau'ybau", "adzau", &["adzau"][..]),
            ("adza'ybau", "adza", &[][..]),
            ("adzybau", "adz", &[][..]),
        ] {
            let decomposition = decompose_lujvo_like(dictionary, lujvo, &token)
                .unwrap()
                .unwrap_or_else(|| panic!("{lujvo} decomposes"));
            let first = &decomposition.segments[0];
            assert_eq!(
                canonicalize_text(first.segment.phonemes().as_str()),
                part,
                "{lujvo}"
            );
            assert_eq!(
                first.kind,
                DictionaryLujvoSegmentKind::FuivlaRafsi,
                "{lujvo}"
            );
            assert_eq!(
                first
                    .sources
                    .iter()
                    .map(|source| source.word)
                    .collect::<Vec<_>>(),
                sources,
                "{lujvo}"
            );
        }
        for (first, second, expected) in [
            ("spageti", "kukte", "spageti'ykukte"),
            ("enri", "kukte", "enrykukte"),
            ("actce'o", "bangu", "actce'ybau"),
            ("adzau", "bangu", "adzau'ybau"),
        ] {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::Word(first.to_owned()),
                    JvozbaInput::Word(second.to_owned()),
                ],
                &token,
            )
            .unwrap_or_else(|error| panic!("{first} {second}: {error}"));
            assert_eq!(result.word, expected, "{first} {second}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_string_of_ordinary_rafsi_before_y_is_not_a_fuivla_rafsi() {
        // These parts are ordinary rafsi strings or known lujvo, as on main
        // (#1139 review). They keep their ordinary sources.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for (lujvo, part, sources) in [
            ("velski'y'esku", "velski", &["velski"][..]),
            ("jbobau'ybra", "jbobau", &["jbobau"][..]),
            ("jbokla'ybangu", "jbokla", &[][..]),
        ] {
            let decomposition = decompose_lujvo_like(dictionary, lujvo, &token)
                .unwrap()
                .unwrap_or_else(|| panic!("{lujvo} decomposes"));
            let first = &decomposition.segments[0];
            assert_eq!(
                canonicalize_text(first.segment.phonemes().as_str()),
                part,
                "{lujvo}"
            );
            assert_eq!(first.kind, DictionaryLujvoSegmentKind::Rafsi, "{lujvo}");
            assert_eq!(
                first
                    .sources
                    .iter()
                    .map(|source| source.word)
                    .collect::<Vec<_>>(),
                sources,
                "{lujvo}"
            );
        }
        let shipped = decompose_lujvo_like(
            dictionary,
            "facyborselkaizilkanpyborselkaimau'yraunzu",
            &token,
        )
        .unwrap()
        .unwrap();
        assert!(
            shipped
                .segments
                .iter()
                .all(|segment| segment.kind != DictionaryLujvoSegmentKind::FuivlaRafsi),
            "{shipped:?}"
        );
        // `bancu'e` is a string of ordinary rafsi, so `bancu'e'a` keeps its
        // full form, and a literal `jbobau` stays refused before `bratu`.
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .map(|result| result.word.clone())
        };
        assert_eq!(
            build(&[
                JvozbaInput::Word("bancu'e'a".to_owned()),
                JvozbaInput::Word("bangu".to_owned()),
            ])
            .as_deref(),
            Ok("bancu'e'a'ybau")
        );
        assert!(
            build(&[
                JvozbaInput::FixedRafsi("jbobau".to_owned()),
                JvozbaInput::Word("bratu".to_owned()),
            ])
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_full_form_names_its_fuivla_alone() {
        // Owner rules for round 8: F + `'y` names F alone, a short form that
        // equals a full form does not exist, and a short form that adds the
        // experimental CgV warning is refused.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for (lujvo, sources) in [
            ("esku'ykukte", &["esku"][..]),
            ("enri'ykukte", &["enri"][..]),
            ("bolviia'ybau", &["bolviia"][..]),
        ] {
            let decomposition = decompose_lujvo_like(dictionary, lujvo, &token)
                .unwrap()
                .unwrap();
            assert_eq!(
                decomposition.segments[0]
                    .sources
                    .iter()
                    .map(|source| source.word)
                    .collect::<Vec<_>>(),
                sources,
                "{lujvo}"
            );
        }
        // Two short forms that collide still list both, and neither is used.
        let esky = decompose_lujvo_like(dictionary, "eskykukte", &token)
            .unwrap()
            .unwrap();
        let mut esky_sources = esky.segments[0]
            .sources
            .iter()
            .map(|source| source.word)
            .collect::<Vec<_>>();
        esky_sources.sort_unstable();
        assert_eq!(esky_sources, ["eska", "esku"]);
        for (first, second, expected) in [
            ("esku", "kukte", "esku'ykukte"),
            ("eskua", "kukte", "eskua'ykukte"),
            ("eska", "kukte", "eska'ykukte"),
            ("enri", "kukte", "enrykukte"),
            ("enrio", "kukte", "enrio'ykukte"),
            ("bolviia", "bangu", "bolviia'ybau"),
            // `abniena` has the warning itself, so its short form adds
            // nothing and stays.
            ("abniena", "kukte", "abnienykukte"),
            ("burjuia", "bangu", "burjuia'ybau"),
            ("cpikiui", "bangu", "cpikiui'ybau"),
            ("bantuuo", "bangu", "bantuuo'ybau"),
        ] {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::Word(first.to_owned()),
                    JvozbaInput::Word(second.to_owned()),
                ],
                &token,
            )
            .unwrap_or_else(|error| panic!("{first} {second}: {error}"));
            assert_eq!(result.word, expected, "{first} {second}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_glide_before_the_final_vowel_keeps_the_short_form() {
        // The morphology reads `barbekeiu` as `barbekéĭu`: the final nucleus is
        // `u`, after the glide `ĭ`, so the short form is `barbekei'y`.
        // `adzau` ends in the diphthong `aŭ` and has none.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let decomposition = decompose_lujvo_like(dictionary, "barbekei'ybau", &token)
            .unwrap()
            .unwrap();
        assert_eq!(
            decomposition.segments[0]
                .sources
                .iter()
                .map(|source| source.word)
                .collect::<Vec<_>>(),
            ["barbekeiu"]
        );
        for (first, expected) in [("barbekeiu", "barbekei'ybau"), ("adzau", "adzau'ybau")] {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::Word(first.to_owned()),
                    JvozbaInput::Word("bangu".to_owned()),
                ],
                &token,
            )
            .unwrap();
            assert_eq!(result.word, expected, "{first}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_vowel_nucleus_after_y_takes_an_apostrophe() {
        // One rule follows the left part's hyphen: before a vowel nucleus the
        // `y` takes an apostrophe. A glide is not a nucleus, so `iismu` takes
        // none. This covers full forms, short forms, consonant stems and
        // fixed rafsi.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for (first, second, expected) in [
            ("spageti", "enri", "spageti'y'enri"),
            ("adzau", "enri", "adzau'y'enri"),
            ("actce'o", "enri", "actce'y'enri"),
            ("ierxi", "enri", "ierxy'enri"),
            ("spageti", "iismu", "spageti'yiismu"),
            ("actce'o", "iismu", "actce'yiismu"),
            ("ierxi", "iismu", "ierxyiismu"),
        ] {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::Word(first.to_owned()),
                    JvozbaInput::Word(second.to_owned()),
                ],
                &token,
            )
            .unwrap_or_else(|error| panic!("{first} {second}: {error}"));
            assert_eq!(result.word, expected, "{first} {second}");
        }
        let fixed = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::FixedRafsi("actce".to_owned()),
                JvozbaInput::Word("enri".to_owned()),
            ],
            &token,
        )
        .unwrap();
        assert_eq!(fixed.word, "actce'y'enri");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_full_form_shadows_an_equal_short_form() {
        use jbotci_dictionary::DictionaryEntry;

        // `actce` is the full form of a fu'ivla `actce`, and `actce'o` would
        // shorten to it. The full form names `actce` alone, so `actce'o` has
        // no short form (owner rules for round 8). Decomposition lists only
        // `actce`. `actce` builds with its own shorter form `actc`, and
        // `actce'o` keeps its full form.
        let english = jbotci_dictionary_data::english();
        let actceho = *english
            .lookup_word(&LookupKey::from_stored("actce'o"))
            .unwrap();
        let actce = DictionaryEntry {
            word: "actce",
            ..actceho
        };
        let bangu = *english
            .lookup_word(&LookupKey::from_stored("bangu"))
            .unwrap();
        let token = CancellationToken::never();
        with_source_dictionary(&[actceho, actce, bangu], |dictionary| {
            let decomposition = decompose_lujvo_like(dictionary, "actce'ybau", &token)
                .unwrap()
                .unwrap();
            assert_eq!(
                decomposition.segments[0]
                    .sources
                    .iter()
                    .map(|source| source.word)
                    .collect::<Vec<_>>(),
                ["actce"]
            );
            for (index, word, expected) in
                [(0, "actce'o", "actce'o'ybau"), (1, "actce", "actcybau")]
            {
                let result = build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[
                        JvozbaInput::DictionaryWord {
                            entry_index: EntryIndex(index),
                            word: word.to_owned(),
                        },
                        JvozbaInput::Word("bangu".to_owned()),
                    ],
                    &token,
                )
                .unwrap_or_else(|error| panic!("{word}: {error}"));
                assert_eq!(result.word, expected, "{word}");
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn entries_that_share_a_lookup_key_are_one_fuivla() {
        use jbotci_dictionary::DictionaryEntry;

        // `onji` and `.onji` are two entries for one word. Their shared stem
        // `onj` names that word, so it is not ambiguous, and either entry can
        // enter a build. The word input and both entry indexes give the same
        // result.
        let english = jbotci_dictionary_data::english();
        let onji = *english
            .lookup_word(&LookupKey::from_stored("onji"))
            .unwrap();
        let dotted = DictionaryEntry {
            word: ".onji",
            ..onji
        };
        let klama = *english
            .lookup_word(&LookupKey::from_stored("klama"))
            .unwrap();
        let token = CancellationToken::never();
        with_source_dictionary(&[onji, dotted, klama], |dictionary| {
            let decomposition = decompose_lujvo_like(dictionary, "onjykla", &token)
                .unwrap()
                .unwrap();
            assert_eq!(decomposition.segments[0].sources.len(), 2);
            let build = |first: JvozbaInput| {
                build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[first, JvozbaInput::Word("klama".to_owned())],
                    &token,
                )
                .map(|result| result.word.clone())
            };
            let from_word = build(JvozbaInput::Word("onji".to_owned()));
            assert_eq!(from_word.as_deref(), Ok("onjykla"));
            // The compound itself expands to `onji` and `klama` again.
            let expanded = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[JvozbaInput::Word("onjykla".to_owned())],
                &token,
            )
            .map(|result| result.word.clone());
            assert_eq!(expanded, from_word);
            for (index, word) in [(0, "onji"), (1, ".onji")] {
                assert_eq!(
                    build(JvozbaInput::DictionaryWord {
                        entry_index: EntryIndex(index),
                        word: word.to_owned(),
                    }),
                    from_word,
                    "{word}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_literal_fuivla_stem_takes_its_hyphen_by_rule() {
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for (stem, expected) in [("tci'il", "tci'ilykla"), ("ierx", "ierxykla")] {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::FixedRafsi(stem.to_owned()),
                    JvozbaInput::Word("klama".to_owned()),
                ],
                &token,
            )
            .unwrap_or_else(|error| panic!("{stem}: {error}"));
            assert_eq!(result.word, expected);
        }
    }

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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
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
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("expanded lujvo should provide at least two source inputs");

        assert_eq!(result.word, "jenjigu'ydi'e");
        assert_segment_texts(&result, &["jenjigu", "'y", "di'e"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_lujvo_part_with_no_source_is_an_error_that_names_it() {
        // #1149: a lujvo input expands into the source word of each rafsi. A
        // rafsi with no source used to vanish, so `bolvii'ybau klama` built
        // `baukla`. Now the build stops and names the rafsi.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
        };
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        for (lujvo, part, other) in [
            // A fu'ivla rafsi: `bolviia` has no short form, because
            // `bolvii'y` adds the experimental CgV warning.
            ("bolvii'ybau", "bolvii", "klama"),
            // An ordinary rafsi: `lef` is nobody's rafsi.
            ("lefkla", "lef", "bajra"),
        ] {
            let expected = JvozbaError::LujvoPartWithoutSource {
                word: lujvo.to_owned(),
                part: part.to_owned(),
            };
            for inputs in [
                vec![word(lujvo), word(other)],
                vec![word(other), word(lujvo)],
            ] {
                assert_eq!(build(&inputs).unwrap_err(), expected, "{inputs:?}");
            }
        }
        assert_eq!(
            build(&[word("bolvii'ybau"), word("klama")])
                .unwrap_err()
                .to_string(),
            "The rafsi `bolvii` in `bolvii'ybau` names no dictionary word."
        );
        // A dictionary lujvo with a short unsourced part reports it too,
        // whether the input names the word or the exact entry, as the
        // jvozba pane does. `golfyjvi` is not such a word under #1161: its
        // first part `golf` is a long rafsi before its `y` hyphen, so it
        // is pinned and the word builds (see the #1161 tests below).
        let entry_index = dictionary
            .lookup_word_index(&LookupKey::from_stored("golfyjvi"))
            .expect("golfyjvi entry");
        for input in [
            word("golfyjvi"),
            JvozbaInput::DictionaryWord {
                entry_index,
                word: "golfyjvi".to_owned(),
            },
        ] {
            assert_eq!(
                build(&[input, word("klama")])
                    .expect("golf is a pinned long rafsi")
                    .word,
                "golfyjvikla"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_final_part_of_a_lujvo_input_is_its_own_source() {
        // #1161 rule 1: the final part of a lujvo-form input needs no
        // dictionary entry. `iismu` and `lefmo` name no dictionary word,
        // but each is a valid word that can end a lujvo, so each is a
        // rafsi whose source is itself.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the final part is its own source")
                .word
                .clone()
        };
        // Every row of the issue table that this rule changes, in both
        // orders where the part stays final.
        assert_eq!(build(&[word("bajra"), word("aktyiismu")]), "bajy'aktyiismu");
        assert_eq!(build(&[word("bajra"), word("klalefmo")]), "bajyklalefmo");
        // The same final part in a non-final position takes its long form.
        assert_eq!(
            build(&[word("bajra"), word("klalefmo"), word("klama")]),
            "bajyklalefmykla"
        );
        assert_eq!(build(&[word("klalefmo"), word("klama")]), "klalefmykla");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_input_lujvo_keeps_its_rafsi() {
        // #1161 rule 3: an input lujvo keeps its rafsi. The final
        // full-gismu part stays pinned in its long form when it moves to
        // a non-final position (`bangu` -> `bangy`, not `bau`), and stays
        // the full word when it stays final, not the scored short rafsi.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the input lujvo keeps its rafsi")
                .word
                .clone()
        };
        assert_eq!(build(&[word("lojbangu"), word("klama")]), "lojbangykla");
        assert_eq!(build(&[word("klama"), word("lojbangu")]), "klalojbangu");
        assert_eq!(build(&[word("lojbangu")]), "lojbangu");
        // A short final rafsi is kept as it is in both positions.
        assert_eq!(build(&[word("klama"), word("jbobau")]), "klajbobau");
        assert_eq!(build(&[word("jbobau"), word("klama")]), "jbobaukla");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_input_lujvo_keeps_its_joints_too() {
        // The kept parts of an input lujvo keep their own joints: the
        // kept sequence is copied as the input wrote it, with its
        // hyphens (`y`, `r`, `n`, and the apostrophe forms), and the
        // builder chooses hyphens only between different inputs. Without
        // the written `y`, `dutyku'arkykla` is the word that morphology
        // builds, and it reads as one fu'ivla rafsi plus `kla`, not as
        // the input's parts.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the input lujvo keeps its joints")
                .word
                .clone()
        };
        for (lujvo, first, last) in [
            ("dutyku'arka", "dutyku'arkykla", "kladutyku'arka"),
            ("gosymantodi", "gosymantodi'ykla", "klagosymantodi"),
            ("kutysetmima", "kutysetmima'ykla", "klakutysetmima"),
            (
                "prejumydu'a'y'utka",
                "prejumydu'a'y'utkykla",
                "klaprejumydu'a'y'utka",
            ),
            (
                "selprejumydu'a'y'utka",
                "selprejumydu'a'y'utkykla",
                "klaselprejumydu'a'y'utka",
            ),
            ("xunysinjabu", "xunysinjabu'ykla", "klaxunysinjabu"),
            ("zasyku'arka", "zasyku'arkykla", "klazasyku'arka"),
            ("velski'y'esku", "velski'y'esku'ykla", "klamyvelski'y'esku"),
        ] {
            assert_eq!(build(&[word(lujvo), word("klama")]), first, "{lujvo} first");
            assert_eq!(build(&[word("klama"), word(lujvo)]), last, "{lujvo} last");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_rafsi_before_y_is_pinned_without_a_dictionary_entry() {
        // #1161 rule 2: a long gismu rafsi ending in `y` is a pinned
        // rafsi. `golf` in the dictionary lujvo `golfyjvi` and `lefm` in
        // `lefmybajra` name no dictionary word, but the 4-letter form is
        // valid whatever the gismu is.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the long rafsi is pinned")
                .word
                .clone()
        };
        assert_eq!(build(&[word("golfyjvi"), word("klama")]), "golfyjvikla");
        assert_eq!(build(&[word("lefmybajra"), word("klama")]), "lefmybajrykla");
        // A short rafsi before `y` is not predictable, so it still names
        // no word: `lef` in `lefykla` is an error, not a pinned rafsi.
        assert_eq!(
            build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[word("lefykla"), word("bajra")],
                &token,
            ),
            Err(JvozbaError::LujvoPartWithoutSource {
                word: "lefykla".to_owned(),
                part: "lef".to_owned(),
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unlisted_gismu_form_input_works_in_any_position() {
        // #1161 rule 4: an unlisted gismu-form input works through its
        // long rafsi in a non-final position and as itself finally.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the unlisted word has a predictable form")
                .word
                .clone()
        };
        assert_eq!(build(&[word("lefmo"), word("bajra")]), "lefmybajra");
        assert_eq!(build(&[word("bajra"), word("lefmo")]), "bajlefmo");
        assert_eq!(
            build(&[word("lefmo"), word("bajra"), word("klama")]),
            "lefmybajykla"
        );
        // An unlisted fu'ivla stands as itself in the final position.
        assert_eq!(build(&[word("kukte"), word("tciile")]), "kuktytciile");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unlisted_fuhivla_is_accepted_only_as_the_final_part() {
        // An unlisted fu'ivla has no dictionary entry, so for fu'ivla
        // the dictionary is needed outside the final position: the word
        // is accepted only as the final part, as itself. Everywhere
        // else it is refused with the error that names it (gismu are
        // different, because a gismu has one long rafsi form). A rafsi
        // pinned with --rafsi whose form is an unknown fu'ivla is
        // refused too, because it cannot round-trip reliably without a
        // dictionary entry.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        let build = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
        };
        let built = |inputs: &[JvozbaInput]| {
            build(inputs)
                .expect("the unlisted fu'ivla is the final part")
                .word
                .clone()
        };
        // The final position stays, plain and inside an input lujvo.
        assert_eq!(built(&[word("bajra"), word("aktyiismu")]), "bajy'aktyiismu");
        assert_eq!(built(&[word("klama"), word("iismu")]), "klamyiismu");
        assert_eq!(built(&[word("kukte"), word("tciile")]), "kuktytciile");
        // Plain and final-part pieces in a non-final position name the
        // word. `actce` and `barbekei` are also dictionary stems (of
        // `actce'o` and `barbekeiu`), and they get the same named error,
        // not the generic build failure.
        for (inputs, offending) in [
            (vec![word("iismu"), word("klama")], "iismu"),
            (vec![word("aktyiismu"), word("klama")], "iismu"),
            (vec![word("tciile"), word("kukte")], "tciile"),
            (vec![word("actce"), word("klama")], "actce"),
            (vec![word("barbekei"), word("klama")], "barbekei"),
            (vec![word("bajybarbekei"), word("klama")], "barbekei"),
            (vec![word("aktytci'ilo"), word("klama")], "tci'ilo"),
            (vec![word("akty'esko"), word("klama")], "esko"),
        ] {
            assert_eq!(
                build(&inputs),
                Err(JvozbaError::NoDictionaryEntry {
                    offending: offending.to_owned(),
                }),
                "{inputs:?}"
            );
        }
        // An unsourced fu'ivla part inside an input lujvo names the
        // part as a rafsi with no dictionary word, in any position.
        for (lujvo, part) in [
            ("iismu'ykla", "iismu"),
            ("tciile'ykukte", "tciile"),
            ("gamseti'yra'a", "gamseti"),
        ] {
            for inputs in [
                vec![word(lujvo), word("bajra")],
                vec![word("bajra"), word(lujvo)],
            ] {
                assert_eq!(
                    build(&inputs),
                    Err(JvozbaError::LujvoPartWithoutSource {
                        word: lujvo.to_owned(),
                        part: part.to_owned(),
                    }),
                    "{inputs:?}"
                );
            }
        }
        // A pinned rafsi whose form is an unknown fu'ivla is refused
        // the same way: without a dictionary entry it cannot round-trip
        // reliably. A pinned form that names a dictionary fu'ivla
        // (`actce`, the stem of `actce'o`) still works; the apostrophe
        // test in this module builds `actce'y'enri` from it.
        for (inputs, offending) in [
            (vec![rafsi("iismu"), word("klama")], "iismu"),
            (vec![rafsi("tciile"), word("kukte")], "tciile"),
        ] {
            assert_eq!(
                build(&inputs),
                Err(JvozbaError::NoDictionaryEntry {
                    offending: offending.to_owned(),
                }),
                "{inputs:?}"
            );
        }
        // Unknown fu'ivla stems are refused too, in both modes: `iism`,
        // `adz` and `bolvii` are fu'ivla rafsi forms that name no
        // dictionary fu'ivla. A pinned form that names one (`tci'il`)
        // and ordinary rafsi (`lefm`, `lef`) still build.
        for mode in [JvozbaMode::Lujvo, JvozbaMode::Cmevla] {
            for (inputs, offending) in [
                (vec![rafsi("iism"), word("klama")], "iism"),
                (vec![rafsi("adz"), word("bangu")], "adz"),
                (vec![rafsi("bolvii"), word("bangu")], "bolvii"),
            ] {
                assert_eq!(
                    build_best_jvozba_detailed(mode, dictionary, &inputs, &token),
                    Err(JvozbaError::NoDictionaryEntry {
                        offending: offending.to_owned(),
                    }),
                    "{mode:?} {inputs:?}"
                );
            }
        }
        assert_eq!(built(&[rafsi("tci'il"), word("klama")]), "tci'ilykla");
        assert_eq!(built(&[rafsi("lefm"), word("bajra")]), "lefmybajra");
        assert_eq!(built(&[rafsi("lef"), word("bajra")]), "lefybajra");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tosmabru_sees_the_apostrophe_y_hyphen() {
        // The tosmabru test scans the leading CVC joints up to the first
        // y-hyphen, which is spelled `y'` before a vowel nucleus. With
        // the joint `res` + `y'` + `enri` kept, a leading `cak` still
        // takes its tosmabru `y`.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        let built = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, &token)
                .expect("the tosmabru hyphen is inserted")
                .word
                .clone()
        };
        assert_eq!(built(&[rafsi("cak"), word("resy'enri")]), "cakyresy'enri");
        assert_eq!(built(&[word("calku"), word("resy'enri")]), "cakyresy'enri");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmevla_mode_follows_the_same_piece_rules() {
        // In cmevla mode every piece path follows the mode: an unlisted
        // gismu works through its long rafsi, a kept or self-sourced
        // final gismu takes its consonant-final long rafsi, and a
        // fu'ivla behaves the same plain or decomposed.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let built = |inputs: &[JvozbaInput]| {
            build_best_jvozba_detailed(JvozbaMode::Cmevla, dictionary, inputs, &token)
                .expect("cmevla mode follows the same rules")
                .word
                .clone()
        };
        assert_eq!(built(&[word("lefmo"), word("bajra")]), "lefmybaj");
        assert_eq!(built(&[word("klama"), word("lojbangu")]), "klalojbang");
        assert_eq!(built(&[word("bajra"), word("klalefmo")]), "bajyklalefm");
        // A plain fu'ivla and the same fu'ivla inside an input lujvo
        // take the same form; the decomposed build only adds `cidjy`.
        assert_eq!(built(&[word("spageti"), word("kukte")]), "spageti'ykuk");
        assert_eq!(
            built(&[word("cidjyspageti"), word("kukte")]),
            "cidjyspageti'ykuk"
        );
        // An unlisted fu'ivla is refused non-finally in cmevla mode too,
        // with the same named error, and accepted as the final part.
        assert_eq!(
            build_best_jvozba_detailed(
                JvozbaMode::Cmevla,
                dictionary,
                &[word("tciile"), word("kukte")],
                &token,
            ),
            Err(JvozbaError::NoDictionaryEntry {
                offending: "tciile".to_owned(),
            })
        );
        assert!(built(&[word("kukte"), word("tciile")]).ends_with("tciiles"));
        // A vowel-final rafsi before a vowel-initial fu'ivla takes the
        // `y'` joint, in every piece path, because the built word must
        // be one cmevla as the segmenter decides: the hiatus spelling
        // is not a word at all, so it is never the winner.
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        assert_eq!(built(&[word("klama"), word("onji")]), "klamy'onjis");
        assert_eq!(built(&[word("klama"), word("actce")]), "klamy'actces");
        assert_eq!(
            built(&[word("klama"), word("aktyiismu")]),
            "klamy'aktyiismus"
        );
        // The spelling Carbon's probe printed on the old head,
        // `kerlyu'ukerl`, is not a word (a `y` cannot be followed by a
        // vowel nucleus); the apostrophe spelling is the valid one.
        assert_eq!(
            built(&[rafsi("kerl"), rafsi("u'u"), word("kerlo")]),
            "kerly'u'uker"
        );
        // With no valid cmevla candidate, the error is the cmevla one.
        for inputs in [
            vec![rafsi("kla"), word("onji")],
            vec![rafsi("kla.bai"), word("gasnu")],
            vec![rafsi("kla bai"), word("gasnu")],
        ] {
            assert_eq!(
                build_best_jvozba_detailed(JvozbaMode::Cmevla, dictionary, &inputs, &token),
                Err(JvozbaError::CouldNotBuildCompound),
                "{inputs:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_cmevla_sweep_result_is_one_cmevla_word() {
        // The cmevla counterpart of the round-trip sweep: every word
        // the cmevla mode builds from a dictionary lujvo or fu'ivla, in
        // both positions, is exactly one cmevla as the segmenter
        // decides.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let words = dictionary
            .entries()
            .iter()
            .filter(|entry| matches!(entry.word_type, WordType::Lujvo | WordType::Fuivla))
            .map(|entry| entry.word)
            .collect::<std::collections::BTreeSet<_>>();
        let mut built_count = 0usize;
        for entry_word in words {
            for inputs in [
                vec![word(entry_word), word("klama")],
                vec![word("klama"), word(entry_word)],
            ] {
                let Ok(result) =
                    build_best_jvozba_detailed(JvozbaMode::Cmevla, dictionary, &inputs, &token)
                else {
                    continue;
                };
                built_count += 1;
                assert!(
                    is_cmevla_word(&result.word, &token)
                        .expect("a built word segments without interruption"),
                    "{} is not one cmevla word",
                    result.word
                );
            }
        }
        assert!(built_count > 0, "the sweep must build some words");
    }

    /// The source entry a built part must name on decomposition, or
    /// `None` when the part is its own source.
    #[requires(true)]
    #[ensures(true)]
    fn expected_source_entry(input: &ExpandedInput<'_>) -> Option<EntryIndex> {
        match input {
            ExpandedInput::DictionaryWord(entry) => Some(*entry),
            ExpandedInput::KeptRafsi(kept) => Some(kept.source),
            ExpandedInput::KeptFuhivlaStem(kept) => Some(kept.source),
            ExpandedInput::KeptFinalPart(kept) => Some(kept.source),
            _ => None,
        }
    }

    /// The form a self-sourced part must decompose back to, if any.
    #[requires(true)]
    #[ensures(true)]
    fn expected_self_source_form(input: &ExpandedInput<'_>) -> Option<String> {
        match input {
            ExpandedInput::SelfSourcedFinalPart(part) => Some(part.form.clone()),
            ExpandedInput::PinnedLongRafsi(pinned) => Some(pinned.stem.clone()),
            ExpandedInput::FixedRafsi(text) => Some(canonicalize_text(text)),
            ExpandedInput::UnlistedWord(text) => Some(normalized_spelling(text)),
            _ => None,
        }
    }

    /// Build `inputs` and check the round-trip against the inputs
    /// themselves: the built word decomposes into its own segments,
    /// every decomposed part names the source that built it, and the
    /// word rebuilds from itself.
    #[requires(true)]
    #[ensures(true)]
    fn assert_round_trip(
        dictionary: &Dictionary<'_>,
        inputs: &[JvozbaInput],
        token: &CancellationToken,
    ) -> JvozbaBuildResult {
        let result = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, inputs, token)
            .unwrap_or_else(|error| panic!("{inputs:?} does not build: {error}"));
        let mut expanded = Vec::new();
        for input in inputs {
            plan_expansion(dictionary, input, token)
                .unwrap_or_else(|error| panic!("{input:?} has no plan: {error}"))
                .append_to(&mut expanded);
        }
        let decomposition = decompose_lujvo_like(dictionary, &result.word, token)
            .expect("a built word decomposes without interruption")
            .unwrap_or_else(|| panic!("built word {} does not decompose", result.word));
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| canonicalize_text(segment.segment.phonemes().as_str()))
            .collect::<Vec<_>>();
        let expected = result
            .segments
            .iter()
            .map(|segment| segment.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(surfaces, expected, "decomposition of {}", result.word);
        let parsed = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.clone())
            .collect::<Vec<_>>();
        assert!(
            sources_match(dictionary, &expanded, &parsed),
            "the parts of {} name the sources that built it",
            result.word
        );
        let rafsi_segments = decomposition
            .segments
            .iter()
            .filter(|segment| segment.kind.is_rafsi());
        for (segment, input) in rafsi_segments.zip(expanded.iter()) {
            match expected_source_entry(input) {
                // A fu'ivla rafsi part must name its exact source in the
                // decomposition too: that is where a full form was once
                // hijacked to another fu'ivla (`tci'ilo` to `tci'ile`).
                // An ordinary rafsi can be a shared universal form
                // (`tend` for both `tendo` and `tende`), whose primary
                // decomposition source is the higher-ranked claimant;
                // `sources_match` above is its source check.
                Some(entry) if segment.kind == DictionaryLujvoSegmentKind::FuivlaRafsi => {
                    assert!(
                        segment
                            .sources
                            .iter()
                            .any(|source| source.entry_index == entry),
                        "a part of {} does not name its source entry",
                        result.word
                    )
                }
                Some(_) => {}
                None => {
                    // A self-sourced part names itself. Its stem can also
                    // be the shared universal form of a dictionary word
                    // (`rimn` is the long rafsi of both `rimna` and
                    // `rimni`), which is the standing shared-form rule,
                    // not a different source.
                    let form = expected_self_source_form(input).unwrap_or_default();
                    let surface = canonicalize_text(segment.segment.phonemes().as_str());
                    for source in &segment.sources {
                        let entry = dictionary
                            .entry_for_index(source.entry_index)
                            .expect("decomposition source entry");
                        let entry_word = normalized_spelling(entry.word);
                        let names_itself =
                            normalize_lookup_query(&entry_word) == normalize_lookup_query(&form);
                        let shares_form = if entry.word_type.is_gismu_like() {
                            GismuShape::classify(&entry_word).is_some()
                                && long_gismu_stem(&entry_word) == surface
                        } else if entry.word_type.is_fuivla_like() {
                            fuivla_stem_rafsi(&entry_word).as_deref() == Some(surface.as_str())
                        } else {
                            false
                        };
                        assert!(
                            names_itself || shares_form,
                            "a part of {} names a source other than itself",
                            result.word
                        );
                    }
                }
            }
        }
        let rebuilt = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[JvozbaInput::Word(result.word.clone())],
            token,
        )
        .unwrap_or_else(|error| panic!("built word {} does not rebuild: {error}", result.word));
        assert_eq!(rebuilt.word, result.word, "round-trip of {}", result.word);
        result
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn built_words_decompose_back_into_the_parts_that_built_them() {
        // Round-trip over every dictionary lujvo and fu'ivla, in both
        // positions: a built word must decompose (the decomposition
        // vlacku shows) into exactly its own segments, and must rebuild
        // from itself as the only input, which only succeeds when every
        // parsed part names the source that built it.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        // A dictionary fu'ivla built through its full form decomposes
        // back into that fu'ivla itself: `esku'ykukte` names `esku`.
        let esku = dictionary
            .lookup_word_index(&LookupKey::from_stored("esku"))
            .expect("esku entry");
        let built = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("esku".to_owned()),
                JvozbaInput::Word("kukte".to_owned()),
            ],
            &token,
        )
        .expect("esku builds through its full form");
        assert_eq!(built.word, "esku'ykukte");
        let decomposition = decompose_lujvo_like(dictionary, &built.word, &token)
            .expect("the built word decomposes")
            .expect("the built word has a decomposition");
        assert_eq!(
            decomposition.segments[0].sources,
            vec![jbotci_dictionary::DictionaryLujvoSource {
                entry_index: esku,
                word: "esku",
            }],
            "the full form names the fu'ivla itself"
        );
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let words = dictionary
            .entries()
            .iter()
            .filter(|entry| matches!(entry.word_type, WordType::Lujvo | WordType::Fuivla))
            .map(|entry| entry.word)
            .collect::<std::collections::BTreeSet<_>>();
        let mut built_count = 0usize;
        for entry_word in words {
            for inputs in [
                vec![word(entry_word), word("klama")],
                vec![word("klama"), word(entry_word)],
            ] {
                if build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token)
                    .is_err()
                {
                    continue;
                }
                built_count += 1;
                assert_round_trip(dictionary, &inputs, &token);
            }
        }
        assert!(built_count > 0, "the sweep must build some words");
        // Valid unlisted builds round-trip by the same assertion.
        for (inputs, expected_word) in [
            (vec![word("bajra"), word("aktyiismu")], "bajy'aktyiismu"),
            (vec![word("bajra"), word("klalefmo")], "bajyklalefmo"),
            (vec![word("lefmo"), word("bajra")], "lefmybajra"),
            (vec![word("golfyjvi"), word("klama")], "golfyjvikla"),
            (
                vec![word("klama"), JvozbaInput::FixedRafsi("iismu".to_owned())],
                "klamyiismu",
            ),
        ] {
            let result = assert_round_trip(dictionary, &inputs, &token);
            assert_eq!(result.word, expected_word, "{inputs:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_lujvo_with_no_sourced_rafsi_names_its_first_part() {
        // Round 2 of #1149: `lefkroli` splits into `lef` and `kroli`, and
        // neither names a dictionary word. The input still expands, so every
        // position, mode and partner gets the error that names `lef`. As a
        // final brivla core it could never build, because its two rafsi
        // cannot match one input.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        let expected = JvozbaError::LujvoPartWithoutSource {
            word: "lefkroli".to_owned(),
            part: "lef".to_owned(),
        };
        for mode in [JvozbaMode::Lujvo, JvozbaMode::Cmevla] {
            for inputs in [
                vec![word("lefkroli"), word("bajra")],
                vec![word("bajra"), word("lefkroli")],
                vec![word("bajra"), word("lefkroli"), word("klama")],
                vec![word("bajra"), word("klama"), word("lefkroli")],
                vec![rafsi("baj"), word("lefkroli")],
            ] {
                let error = build_best_jvozba_detailed(mode, dictionary, &inputs, &token)
                    .expect_err("lefkroli has no sourced rafsi");
                assert_eq!(error, expected, "{mode:?} {inputs:?}");
            }
        }
        // The error names the word in its normalized spelling, as the
        // other errors do.
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[word(" .LEFKLA. "), word("bajra")],
            &token,
        )
        .expect_err("lef has no source");
        assert_eq!(
            error.to_string(),
            "The rafsi `lef` in `lefkla` names no dictionary word."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_pane_admits_a_lujvo_that_can_only_report_its_missing_part() {
        // Owner ruling for #1149 round 2: keep the "add to jvozba" control for
        // such a word, and show the error when the pane builds.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        for text in ["lefkroli", "lefkla", "golfyjvi", "bolvii'ybau"] {
            assert_eq!(
                word_can_enter_jvozba_pane(dictionary, text, &token),
                Ok(true),
                "{text}"
            );
        }
        let entry_index = dictionary
            .lookup_word_index(&LookupKey::from_stored("golfyjvi"))
            .expect("golfyjvi entry");
        assert_eq!(
            entry_can_enter_jvozba_pane(dictionary, entry_index, &token),
            Ok(true)
        );
        // A word with no candidate that the morphology does not read as a
        // lujvo stays out.
        assert_eq!(
            word_can_enter_jvozba_pane(dictionary, "lefmeq", &token),
            Ok(false)
        );
        // The ruling adds no words: a dictionary cmevla that the build
        // expands into source words has no candidate of its own, and it stays
        // out, as before #1149.
        for text in ["lojban", "moskov"] {
            assert_eq!(
                word_can_enter_jvozba_pane(dictionary, text, &token),
                Ok(false),
                "{text}"
            );
            let entry_index = dictionary
                .lookup_word_index(&LookupKey::from_stored(text))
                .expect("cmevla entry");
            assert_eq!(
                entry_can_enter_jvozba_pane(dictionary, entry_index, &token),
                Ok(false),
                "{text}"
            );
            assert!(
                decompose_lujvo_like(dictionary, text, &token)
                    .unwrap()
                    .is_some(),
                "{text} still expands in a build"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_first_failing_input_names_the_error() {
        // jvozba reports the error of the first failing input, in the order
        // that the user wrote. A part error of a later input does not hide a
        // candidate error of an earlier one.
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());
        let lef = JvozbaError::LujvoPartWithoutSource {
            word: "lefkroli".to_owned(),
            part: "lef".to_owned(),
        };
        let djan = JvozbaError::NoDictionaryEntry {
            offending: "djan".to_owned(),
        };
        for (inputs, expected) in [
            (vec![word("djan"), word("lefkroli")], djan.clone()),
            (vec![word("lefkroli"), word("djan")], lef.clone()),
            (
                vec![word("bajra"), word("djan"), word("lefkroli")],
                djan.clone(),
            ),
            (
                vec![rafsi(","), word("lefkroli")],
                JvozbaError::FixedRafsiEmpty,
            ),
            (vec![word("djan"), word("eskykukte")], djan),
            (
                vec![word("eskykukte"), word("djan")],
                JvozbaError::AmbiguousFuivlaRafsi {
                    stem: "esk".to_owned(),
                },
            ),
        ] {
            let error = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token)
                .expect_err("one input fails");
            assert_eq!(error, expected, "{inputs:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_bounded_build_keeps_the_plan_error_when_earlier_inputs_exceed_the_piece_limit() {
        // The bounded-build policy: when the earlier inputs are already over
        // the piece limit (three `djan` against `max_pieces: 2`), the build
        // does not make their candidates, so the error of the failing plan
        // stands. An unbounded build, or a limit that the earlier inputs fit,
        // makes them and names the first failing input (`djan`).
        let dictionary = jbotci_dictionary_data::english();
        let token = CancellationToken::never();
        let inputs = [
            JvozbaInput::Word("djan".to_owned()),
            JvozbaInput::Word("djan".to_owned()),
            JvozbaInput::Word("djan".to_owned()),
            JvozbaInput::Word("lefkroli".to_owned()),
        ];
        let djan = JvozbaError::NoDictionaryEntry {
            offending: "djan".to_owned(),
        };
        let lef = JvozbaError::LujvoPartWithoutSource {
            word: "lefkroli".to_owned(),
            part: "lef".to_owned(),
        };
        assert_eq!(
            build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token)
                .expect_err("djan has no entry"),
            djan
        );
        for (max_pieces, expected) in [(2, &lef), (3, &djan), (4, &djan)] {
            let limits = bounded().with_data(bityzba::data! { max_pieces: max_pieces });
            assert_eq!(
                build_best_jvozba_detailed_within(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &inputs,
                    limits,
                    &token,
                )
                .expect_err("one input fails"),
                *expected,
                "max_pieces {max_pieces}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_missing_dictionary_entries() {
        // The name has to be one the dictionary does not hold *and* cannot
        // decompose: a name built from rafsi is expanded into the words it
        // is built from before it is ever looked up. `feklef` splits as
        // cleanly as `feklat`, but `lef` is nobody's rafsi, so it reaches the
        // lookup as itself.
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("feklef".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect_err("missing entry");
        assert_eq!(error.to_string(), "No dictionary entry for `feklef`.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_empty_normalized_word_keeps_the_typed_spelling_in_the_error() {
        let error = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word(".".to_owned()),
                JvozbaInput::Word("klama".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect_err("a period is not a dictionary word");
        assert_eq!(error.to_string(), "No dictionary entry for `.`.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_shorter_valid_rafsi_still_wins_over_the_full_fuivla() {
        let result = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            jbotci_dictionary_data::english(),
            &[
                JvozbaInput::Word("ierxi".to_owned()),
                JvozbaInput::Word("klama".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("ierxi has a usable rafsi");
        assert_eq!(result.word, "ierxykla");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_nonfinal_fuivla_keeps_the_full_word_after_a_mismatched_listed_rafsi() {
        use jbotci_dictionary::{DictionaryEntry, Rafsi, RafsiClaimKind};

        // `spageti` lists `ierx`, but before `y` that form parses as the
        // derived stem of `ierxi`, so its sources do not include `spageti`.
        // The build must keep `spageti` through its full form.
        let english = jbotci_dictionary_data::english();
        let listed = [Rafsi {
            form: "ierx",
            standing: RafsiClaimKind::Experimental,
        }];
        let ierxi = *english
            .lookup_word(&LookupKey::from_stored("ierxi"))
            .unwrap();
        let spageti = DictionaryEntry {
            rafsi: &listed,
            ..*english
                .lookup_word(&LookupKey::from_stored("spageti"))
                .unwrap()
        };
        let kukte = *english
            .lookup_word(&LookupKey::from_stored("kukte"))
            .unwrap();
        let token = CancellationToken::never();
        with_source_dictionary(&[ierxi, spageti, kukte], |dictionary| {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[
                    JvozbaInput::DictionaryWord {
                        entry_index: EntryIndex(1),
                        word: "spageti".to_owned(),
                    },
                    JvozbaInput::Word("kukte".to_owned()),
                ],
                &token,
            )
            .expect("the full fu'ivla remains a candidate");
            assert!(
                result.word.starts_with("spageti"),
                "the listed rafsi must not replace the source word: {}",
                result.word
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_fuivla_whose_headword_normalizes_to_nothing_keeps_its_listed_rafsi() {
        use jbotci_dictionary::{DictionaryEntry, Rafsi, RafsiClaimKind};

        // A stored headword of `.` normalizes to an empty spelling. It has no
        // full form, so jvozba must not build an empty full-word candidate.
        // Its listed rafsi stays usable, and an error names the stored `.`.
        let english = jbotci_dictionary_data::english();
        let listed = [Rafsi {
            form: "spa",
            standing: RafsiClaimKind::Experimental,
        }];
        let period = DictionaryEntry {
            word: ".",
            rafsi: &listed,
            ..*english
                .lookup_word(&LookupKey::from_stored("spageti"))
                .unwrap()
        };
        let unlisted = DictionaryEntry {
            word: ".",
            rafsi: &[],
            ..period
        };
        let token = CancellationToken::never();
        let input = JvozbaInput::DictionaryWord {
            entry_index: EntryIndex(0),
            word: ".".to_owned(),
        };
        with_source_dictionary(&[period], |dictionary| {
            let result = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[input.clone(), JvozbaInput::FixedRafsi("klama".to_owned())],
                &token,
            )
            .expect("the listed rafsi builds the lujvo");
            assert_eq!(result.word, "spaklama");
            // In the final slot, the listed CCV rafsi is the only candidate.
            let final_slot = build_best_jvozba_detailed(
                JvozbaMode::Lujvo,
                dictionary,
                &[JvozbaInput::FixedRafsi("spa".to_owned()), input.clone()],
                &token,
            )
            .expect("the listed rafsi ends the lujvo");
            assert_eq!(final_slot.word, "spaspa");
        });
        with_source_dictionary(&[unlisted], |dictionary| {
            assert_eq!(
                build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[input.clone(), JvozbaInput::FixedRafsi("klama".to_owned())],
                    &token,
                ),
                Err(JvozbaError::NoRafsiAvailable {
                    offending: ".".to_owned(),
                }),
            );
            assert_eq!(
                build_best_jvozba_detailed(
                    JvozbaMode::Lujvo,
                    dictionary,
                    &[JvozbaInput::FixedRafsi("spa".to_owned()), input.clone()],
                    &token,
                ),
                Err(JvozbaError::FinalConsonant {
                    offending: ".".to_owned(),
                    is_fixed_rafsi: false,
                }),
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_name_built_from_rafsi_enters_a_build_as_the_words_it_is_built_from() {
        // Naming a compound is naming its parts, which is what a lujvo
        // input has always done; #914 restores it for a name built from
        // rafsi. `lojban` is `logji` + `bangu`, so building from it must
        // reach the same word as building from those two.
        let dictionary = jbotci_dictionary_data::english();
        let from_name = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("lojban".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("a name built from rafsi expands");
        let from_parts = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("logji".to_owned()),
                JvozbaInput::Word("bangu".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("the same build from the words themselves");

        assert_eq!(from_name, from_parts);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_exact_dictionary_final_component_source() {
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "jetcybolxada",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
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
                    && segment.sources.first().map(|source| source.word) == Some("bolxada"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_dictionary_backed_fuhivla_self_core_source() {
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "jenjigu'ydi'e",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("morphology-backed lujvo decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["jenjigu", "'y", "dí'e"]);
        assert_eq!(sources, [Some("jenjigu"), None, Some("dirce")]);
        assert_eq!(decomposition.source_words, ["jenjigu", "dirce"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_r_hyphen_after_initial_cvv_rafsi() {
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "ci'artai",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("morphology-backed lujvo decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["ci'á", "r", "taĭ"]);
        assert_eq!(sources, [Some("ciska"), None, Some("tarmi")]);
        assert_eq!(decomposition.source_words, ["ciska", "tarmi"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_lujvo_with_r_initial_rafsi_without_r_hyphen() {
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "jetrok",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("r-initial rafsi decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["jet", "rok"]);
        assert_eq!(sources.len(), 2);
        assert!(sources[0].is_some());
        assert_eq!(sources[1], Some("rokci"));
        assert_eq!(decomposition.source_words.len(), 2);
        assert_eq!(decomposition.source_words[1], "rokci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_cmevla_lujvo_with_cvv_prefix_before_r_initial_cvc_rafsi() {
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "baunrok",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("cmevla fallback decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["bau", "n", "rok"]);
        assert_eq!(sources, [Some("bangu"), None, Some("rokci")]);
        assert_eq!(decomposition.source_words, ["bangu", "rokci"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_cmevla_lujvo_from_a_plain_rafsi_chain() {
        // The #914 case: a name built from rafsi that happen to need no
        // hyphen between them still shows the words it was built from.
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "feklat",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("cmevla fallback decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["fek", "lat"]);
        assert_eq!(sources, [Some("fenki"), Some("mlatu")]);
        assert_eq!(decomposition.source_words, ["fenki", "mlatu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_name_that_repeats_a_word_keeps_both_occurrences_in_its_segments() {
        // `latlat` is `lat` + `lat`, and both are `mlatu`.
        let decomposition = decompose_lujvo_like(
            jbotci_dictionary_data::english(),
            "latlat",
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("an uninterrupted decomposition")
        .expect("cmevla fallback decomposition");
        let surfaces = decomposition
            .segments
            .iter()
            .map(|segment| segment.segment.phonemes().as_str())
            .collect::<Vec<_>>();
        let sources = decomposition
            .segments
            .iter()
            .map(|segment| segment.sources.first().map(|source| source.word))
            .collect::<Vec<_>>();

        assert_eq!(surfaces, ["lat", "lat"]);
        assert_eq!(sources, [Some("mlatu"), Some("mlatu")]);
        // The source words name each word once on purpose: a lookup or an
        // index wants the words, not the pieces. Anything that works piece
        // by piece has to read the segments instead.
        assert_eq!(decomposition.source_words, ["mlatu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_name_that_repeats_a_word_enters_a_build_once_per_occurrence() {
        let dictionary = jbotci_dictionary_data::english();

        let from_name = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("latlat".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("a repeated rafsi enters once per occurrence");
        let from_parts = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("lojbo".to_owned()),
                JvozbaInput::Word("mlatu".to_owned()),
                JvozbaInput::Word("mlatu".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("the same build from the words themselves");

        assert_eq!(from_name, from_parts);

        // On its own the name is already two words, so it builds rather than
        // failing for want of a second input.
        let alone = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[JvozbaInput::Word("latlat".to_owned())],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("a name of two rafsi is two inputs");
        let pair = build_best_jvozba_detailed(
            JvozbaMode::Lujvo,
            dictionary,
            &[
                JvozbaInput::Word("mlatu".to_owned()),
                JvozbaInput::Word("mlatu".to_owned()),
            ],
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("the same build from the words themselves");

        assert_eq!(alone, pair);
    }

    /// Issue #1119. U+02BC is an apostrophe here, the same as U+2019.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn modifier_letter_apostrophe_matches_the_quotation_apostrophe() {
        assert_eq!(
            normalize_lujvo_like_input("co\u{02bc}a\u{02bc}a"),
            normalize_lujvo_like_input("co'a'a")
        );
        assert_eq!(
            normalize_lujvo_like_input("co\u{02bc}a\u{02bc}a"),
            normalize_lujvo_like_input("co\u{2019}a\u{2019}a")
        );
        assert_eq!(
            normalize_lujvo_like_input("co\u{2018}a"),
            canonicalize_text("co\u{2018}a")
        );
    }

    /// Issue #1120. Every Latin apostrophe spelling builds the same word.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_apostrophe_spellings_build_the_same_jvozba() {
        let dictionary = jbotci_dictionary_data::english();
        let cancellation = &jbotci_interrupt::CancellationToken::never();
        let rafsi_spellings = ["ta'a", "ta\u{2019}a", "ta\u{02bc}a", "ta\u{02d2}a"];
        let ascii_rafsi = &rafsi_spellings[0];
        for mode in [JvozbaMode::Lujvo, JvozbaMode::Cmevla] {
            for klama_first in [false, true] {
                let ascii = build_apostrophe_jvozba(
                    mode,
                    dictionary,
                    klama_first,
                    &JvozbaInput::FixedRafsi((*ascii_rafsi).to_owned()),
                    cancellation,
                );
                for spelling in &rafsi_spellings[1..] {
                    let actual = build_apostrophe_jvozba(
                        mode,
                        dictionary,
                        klama_first,
                        &JvozbaInput::FixedRafsi((*spelling).to_owned()),
                        cancellation,
                    );
                    assert_eq!(
                        actual, ascii,
                        "{mode:?} klama_first={klama_first} {spelling}"
                    );
                }
                assert!(
                    ascii.as_ref().is_ok_and(|result| {
                        !result
                            .word
                            .chars()
                            .any(|value| value != '\'' && normalize_latin_apostrophe(value) == '\'')
                            && result.segments.iter().all(|segment| {
                                !segment.text.chars().any(|value| {
                                    value != '\'' && normalize_latin_apostrophe(value) == '\''
                                })
                            })
                    }),
                    "{mode:?} klama_first={klama_first} {ascii:?}"
                );
            }
        }
        assert_eq!(
            build_apostrophe_jvozba(
                JvozbaMode::Lujvo,
                dictionary,
                false,
                &JvozbaInput::FixedRafsi("ta'a".to_owned()),
                cancellation,
            )
            .expect("lujvo")
            .word,
            "ta'akla"
        );
        assert_eq!(
            build_apostrophe_jvozba(
                JvozbaMode::Lujvo,
                dictionary,
                true,
                &JvozbaInput::FixedRafsi("ta'a".to_owned()),
                cancellation,
            )
            .expect("lujvo")
            .word,
            "klata'a"
        );
        let cmevla_first = build_apostrophe_jvozba(
            JvozbaMode::Cmevla,
            dictionary,
            false,
            &JvozbaInput::FixedRafsi("ta'a".to_owned()),
            cancellation,
        )
        .expect("cmevla");
        assert_eq!(cmevla_first.word, "ta'arklam");
        assert_segment_texts(&cmevla_first, &["ta'a", "r", "klam"]);
        let cmevla_second = build_apostrophe_jvozba(
            JvozbaMode::Cmevla,
            dictionary,
            true,
            &JvozbaInput::FixedRafsi("ta'a".to_owned()),
            cancellation,
        )
        .expect("cmevla");
        assert_eq!(cmevla_second.word, "klata'as");
        assert_segment_texts(&cmevla_second, &["kla", "ta'a", "s"]);

        let fuivla_spellings = [
            "ba'alto",
            "ba\u{2019}alto",
            "ba\u{02bc}alto",
            "bahalto",
            "ba\u{a78b}alto",
        ];
        for spelling in fuivla_spellings {
            assert_eq!(
                word_can_enter_jvozba_pane(dictionary, spelling, cancellation),
                Ok(true),
                "{spelling}"
            );
            for klama_first in [false, true] {
                let actual = build_apostrophe_jvozba(
                    JvozbaMode::Lujvo,
                    dictionary,
                    klama_first,
                    &JvozbaInput::Word(spelling.to_owned()),
                    cancellation,
                );
                let ascii = build_apostrophe_jvozba(
                    JvozbaMode::Lujvo,
                    dictionary,
                    klama_first,
                    &JvozbaInput::Word("ba'alto".to_owned()),
                    cancellation,
                );
                assert_eq!(actual, ascii, "klama_first={klama_first} {spelling}");
            }
        }
        let forward = build_apostrophe_jvozba(
            JvozbaMode::Lujvo,
            dictionary,
            true,
            &JvozbaInput::Word("ba'alto".to_owned()),
            cancellation,
        )
        .expect("fu'ivla lujvo");
        assert_eq!(forward.word, "klamyba'alto");
        assert_segment_texts(&forward, &["klam", "y", "ba'alto"]);
        let reverse = build_apostrophe_jvozba(
            JvozbaMode::Lujvo,
            dictionary,
            false,
            &JvozbaInput::Word("ba'alto".to_owned()),
            cancellation,
        )
        .expect("fu'ivla lujvo");
        assert_eq!(reverse.word, "ba'altykla");
        assert_segment_texts(&reverse, &["ba'alt", "y", "kla"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn does_not_decompose_a_cmevla_whose_pieces_are_not_all_rafsi() {
        // A hyphen-free chain is accepted on the strength of its rafsi, so
        // the dictionary still has the last word. `feklef` splits as cleanly
        // as `feklat` does, but `lef` is nobody's rafsi and the name is
        // therefore not a compound.
        assert!(
            decompose_lujvo_like(
                jbotci_dictionary_data::english(),
                "feklef",
                &jbotci_interrupt::CancellationToken::never()
            )
            .expect("an uninterrupted decomposition")
            .is_none(),
            "a segment with no source word must not yield a decomposition"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn build_apostrophe_jvozba(
        mode: JvozbaMode,
        dictionary: &jbotci_dictionary::Dictionary<'_>,
        klama_first: bool,
        other: &JvozbaInput,
        cancellation: &jbotci_interrupt::CancellationToken,
    ) -> Result<JvozbaBuildResult, JvozbaError> {
        let klama = JvozbaInput::Word("klama".to_owned());
        let inputs = if klama_first {
            vec![klama, other.clone()]
        } else {
            vec![other.clone(), klama]
        };
        build_best_jvozba_detailed(mode, dictionary, &inputs, cancellation)
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
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_operation_limit_refuses_work_before_candidate_evaluation() {
        use std::cell::Cell;
        let dictionary = jbotci_dictionary_data::english();
        let inputs = [
            JvozbaInput::Word("klama".to_owned()),
            JvozbaInput::Word("bajra".to_owned()),
            JvozbaInput::Word("gerku".to_owned()),
        ];
        for mode in [JvozbaMode::Lujvo, JvozbaMode::Cmevla] {
            for (limits, measure) in [
                (
                    bounded().with_data(bityzba::data! { max_pieces: 2 }),
                    JvozbaWorkMeasure::Pieces,
                ),
                (
                    bounded().with_data(bityzba::data! { max_placements: 1 }),
                    JvozbaWorkMeasure::Placements,
                ),
                (
                    bounded().with_data(bityzba::data! { max_spelling_letters: 1 }),
                    JvozbaWorkMeasure::SpellingLetters,
                ),
            ] {
                let evaluations = Cell::new(0);
                let result = build_best_jvozba_with_evaluator(
                    mode,
                    dictionary,
                    &inputs,
                    limits,
                    &jbotci_interrupt::CancellationToken::never(),
                    |_, _, _, _| {
                        evaluations.set(evaluations.get() + 1);
                        Ok(None)
                    },
                );
                assert!(
                    matches!(result, Err(JvozbaError::TooMuchWork { measure: actual, .. }) if actual == measure),
                    "{mode:?}: {result:?}"
                );
                assert_eq!(
                    evaluations.get(),
                    0,
                    "rejected work reached the candidate evaluator"
                );
            }
        }
    }

    /// Copies of `zbasu` before the closing `y` in the slow-loop test.
    ///
    /// Release time of `build_best_jvozba_detailed` on this one word with a
    /// clear token is 74.39 s. The build decomposes the word before it can
    /// ask for a second piece. That decomposition is the long morphology.
    /// The test trips at check 1,000 and does not repeat that run.
    const SLOW_ZBASU_COPIES: usize = 500;
    const SLOW_LOOP_TRIP: usize = 1_000;
    const SLOW_LOOP_BOUND: std::time::Duration = std::time::Duration::from_secs(10);

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_late_trip_stops_inside_jvozba_and_the_next_index_builds_jbobau() {
        let dictionary = jbotci_dictionary_data::english();
        let inputs = [
            JvozbaInput::Word("lojbo".to_owned()),
            JvozbaInput::Word("bangu".to_owned()),
        ];
        let result = assert_exit_check(
            |token| build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, token),
            |error| matches!(error, JvozbaError::Interrupted),
        );
        assert_eq!(result.word, "jbobau");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_mid_loop_trip_stops_a_long_zbasu_query_quickly() {
        let dictionary = jbotci_dictionary_data::english();
        let inputs = [JvozbaInput::Word(format!(
            "{}y",
            "zbasu".repeat(SLOW_ZBASU_COPIES)
        ))];
        let started = std::time::Instant::now();
        let token = jbotci_interrupt::CancellationToken::trip_after_checks(SLOW_LOOP_TRIP);
        let result = build_best_jvozba_detailed(JvozbaMode::Lujvo, dictionary, &inputs, &token);
        let elapsed = started.elapsed();
        assert!(
            elapsed < SLOW_LOOP_BOUND,
            "jvozba kept running for {elapsed:?}; a missing loop check decomposes the whole word"
        );
        assert!(
            matches!(result, Err(JvozbaError::Interrupted)),
            "the early trip must be an interrupt, not a finished refusal"
        );
    }
}
