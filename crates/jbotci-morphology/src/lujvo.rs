#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

pub use crate::segment::ConsonantPairClass;

crate::define_string_enum_metadata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LujvoBuildMode {
        Lujvo => ("LUJVO", "lujvo"),
        Cmevla => ("CMEVLA", "cmevla"),
    }
}

#[invariant(!word.is_empty())]
#[invariant(!parts.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoCandidate {
    pub word: String,
    pub parts: Vec<String>,
    pub score: i32,
}

crate::define_string_enum_metadata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum RafsiShape {
        Cvccv => ("CVCCV", "cvccv"),
        Cvcc => ("CVCC", "cvcc"),
        Ccvcv => ("CCVCV", "ccvcv"),
        Ccvc => ("CCVC", "ccvc"),
        Cvc => ("CVC", "cvc"),
        CvhV => ("CVH_V", "cvh-v"),
        Ccv => ("CCV", "ccv"),
        Cvv => ("CVV", "cvv"),
        Other => ("OTHER", "other"),
    }
}

impl RafsiShape {
    #[requires(true)]
    #[ensures(ret >= 0 && ret <= 8)]
    pub const fn score(self) -> i32 {
        match self {
            Self::Cvccv => 1,
            Self::Cvcc => 2,
            Self::Ccvcv => 3,
            Self::Ccvc => 4,
            Self::Cvc => 5,
            Self::CvhV => 6,
            Self::Ccv => 7,
            Self::Cvv => 8,
            Self::Other => 0,
        }
    }
}

/// Every gismu is exactly five letters long (CLL 4.4).
const GISMU_LETTER_COUNT: usize = 5;

crate::define_string_enum_metadata! {
    /// Shape of a full gismu: the only two spellings CLL 4.4 admits.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum GismuShape {
        Ccvcv => ("CCVCV", "ccvcv"),
        Cvccv => ("CVCCV", "cvccv"),
    }
}

impl GismuShape {
    /// Classify `word` as a gismu shape, or `None` when it is not a gismu.
    #[requires(true)]
    #[ensures(ret.is_some() == (word.chars().count() == GISMU_LETTER_COUNT
        && matches!(rafsi_shape(word), RafsiShape::Ccvcv | RafsiShape::Cvccv)))]
    pub fn classify(word: &str) -> Option<Self> {
        // `rafsi_shape` stops at the first character it does not recognize, so
        // it reports `sakli!` as CVCCV. A gismu is exactly five letters, and
        // the length check pins the whole word to the classified shape.
        if word.chars().count() != GISMU_LETTER_COUNT {
            return None;
        }
        match rafsi_shape(word) {
            RafsiShape::Ccvcv => Some(Self::Ccvcv),
            RafsiShape::Cvccv => Some(Self::Cvccv),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        crate::StringEnumMetadata::canonical_name(self)
    }
}

impl std::fmt::Display for GismuShape {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

crate::define_string_enum_metadata! {
    /// Shape of a short rafsi, the three forms a gismu can claim (CLL 4.6).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ShortRafsiShape {
        Cvc => ("CVC", "cvc"),
        Ccv => ("CCV", "ccv"),
        Cvv => ("CVV", "cvv"),
    }
}

impl ShortRafsiShape {
    /// Whether `form` spells exactly one short rafsi of this shape.
    ///
    /// This validates the whole string rather than classifying a prefix, so
    /// trailing text is rejected: [`rafsi_shape`] stops at the first character
    /// it does not recognize and would accept `sal!` as CVC.
    ///
    /// `Ccv` additionally requires a permissible *initial* cluster, because a
    /// CCV rafsi may begin a lujvo and so must be able to begin a word
    /// (CLL 3.7). `Cvv` covers the `CV'V` form (`sa'i`) and the
    /// apostrophe-free form, which exists only for the four diphthongs `ai`,
    /// `ei`, `oi`, and `au` (CLL 4.6) — a bare `sae` is not a rafsi.
    #[requires(true)]
    #[ensures(ret -> matches!(form.chars().count(), 3 | 4))]
    pub fn matches_form(self, form: &str) -> bool {
        let mut letters = form.chars();
        let (Some(first), Some(second), Some(third)) =
            (letters.next(), letters.next(), letters.next())
        else {
            return false;
        };
        let fourth = letters.next();
        if letters.next().is_some() {
            return false;
        }
        match self {
            // `consonant_pair_class` answers `Some` only for Lojban
            // consonants, so an `Initial` pair also proves both letters are
            // consonants.
            Self::Ccv => {
                fourth.is_none()
                    && consonant_pair_class(first, second) == Some(ConsonantPairClass::Initial)
                    && is_vowel(third)
            }
            Self::Cvc => {
                fourth.is_none() && is_consonant(first) && is_vowel(second) && is_consonant(third)
            }
            Self::Cvv => {
                is_consonant(first)
                    && is_vowel(second)
                    && match fourth {
                        Some(fourth) => third == '\'' && is_vowel(fourth),
                        None => matches!(
                            (second, third),
                            ('a', 'i') | ('e', 'i') | ('o', 'i') | ('a', 'u')
                        ),
                    }
            }
        }
    }
}

/// One short rafsi spelling together with the shape it realizes.
#[invariant(shape.matches_form(form), "the spelling realizes the declared shape")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShortRafsiForm {
    pub form: String,
    pub shape: ShortRafsiShape,
}

#[invariant(true)]
#[invariant(::Rafsi(text) => !text.is_empty())]
#[invariant(::BrivlaCore(text) => !text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LujvoBuildPart {
    Rafsi(String),
    BrivlaCore(String),
}

impl LujvoBuildPart {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_text(&self) -> &str {
        match self.as_data() {
            data!(LujvoBuildPart::Rafsi(text)) | data!(LujvoBuildPart::BrivlaCore(text)) => text,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_brivla_core(&self) -> bool {
        matches!(self.as_data(), data!(LujvoBuildPart::BrivlaCore(_)))
    }
}

#[requires(true)]
#[ensures(choices.iter().flatten().any(|text| text.is_empty()) -> ret.is_none())]
pub fn choose_best_lujvo_candidate(
    mode: LujvoBuildMode,
    choices: &[Vec<String>],
) -> Option<LujvoCandidate> {
    if choices.iter().flatten().any(|text| text.is_empty()) {
        return None;
    }
    let typed_choices = choices
        .iter()
        .map(|choice| {
            choice
                .iter()
                .cloned()
                .map(|text| new!(LujvoBuildPart::Rafsi(text)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    choose_best_lujvo_candidate_from_parts(mode, &typed_choices)
}

#[requires(true)]
#[ensures(true)]
pub fn choose_best_lujvo_candidate_from_parts(
    mode: LujvoBuildMode,
    choices: &[Vec<LujvoBuildPart>],
) -> Option<LujvoCandidate> {
    choose_best_candidate_from_parts(mode, choices, &mut Vec::new(), None)
}

#[requires(true)]
#[ensures(true)]
fn choose_best_candidate_from_parts<'a>(
    mode: LujvoBuildMode,
    choices: &'a [Vec<LujvoBuildPart>],
    selected: &mut Vec<&'a LujvoBuildPart>,
    best: Option<LujvoCandidate>,
) -> Option<LujvoCandidate> {
    let Some((next_choices, rest)) = choices.split_first() else {
        let bonded = bond_lujvo_build_part_refs(selected)?;
        let word = bonded.concat();
        let candidate = new!(LujvoCandidate {
            score: lujvo_score(&bonded),
            parts: bonded,
            word,
        });
        if mode == LujvoBuildMode::Lujvo && !is_valid_lujvo_candidate(&candidate) {
            return best;
        }
        return Some(select_better_candidate(best, candidate));
    };

    let mut current_best = best;
    for choice in next_choices {
        selected.push(choice);
        current_best = choose_best_candidate_from_parts(mode, rest, selected, current_best);
        selected.pop();
    }
    current_best
}

#[requires(true)]
#[ensures(true)]
fn select_better_candidate(
    current_best: Option<LujvoCandidate>,
    candidate: LujvoCandidate,
) -> LujvoCandidate {
    let Some(current_best) = current_best else {
        return candidate;
    };
    if candidate.score < current_best.score
        || (candidate.score == current_best.score && candidate.word < current_best.word)
    {
        candidate
    } else {
        current_best
    }
}

#[requires(true)]
#[ensures(rafsis.iter().any(|text| text.is_empty()) -> ret.is_none())]
pub fn bond_rafsis(rafsis: &[String]) -> Option<Vec<String>> {
    if rafsis.iter().any(|text| text.is_empty()) {
        return None;
    }
    let parts = rafsis
        .iter()
        .cloned()
        .map(|text| new!(LujvoBuildPart::Rafsi(text)))
        .collect::<Vec<_>>();
    bond_lujvo_build_parts(&parts)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| parts.len() >= 2))]
fn bond_lujvo_build_parts(parts: &[LujvoBuildPart]) -> Option<Vec<String>> {
    let part_refs = parts.iter().collect::<Vec<_>>();
    bond_lujvo_build_part_refs(&part_refs)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| parts.len() >= 2))]
fn bond_lujvo_build_part_refs(parts: &[&LujvoBuildPart]) -> Option<Vec<String>> {
    if parts.len() < 2 {
        return None;
    }
    let first_part = *parts.first()?;
    let first = first_part.as_text();
    let second = parts.get(1)?.as_text();
    let mut bonded = Vec::with_capacity(parts.len() * 2);
    bonded.push(first.to_owned());
    if !first_part.is_brivla_core() && should_insert_cvv_hyphen(first, second, parts.len()) {
        bonded.push(if second.starts_with('r') {
            "n".to_owned()
        } else {
            "r".to_owned()
        });
    }
    for pair in parts.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        if let Some(hyphen) = hyphen_for_build_part_pair(previous, next) {
            bonded.push(hyphen.to_owned());
        }
        bonded.push(next.as_text().to_owned());
    }
    if tosmabru(&bonded) {
        bonded.insert(1, "y".to_owned());
    }
    Some(bonded)
}

#[requires(true)]
#[ensures(ret -> !word_text.is_empty())]
pub fn is_valid_lujvo_candidate_word(word_text: &str) -> bool {
    let Ok(words) = crate::segment_words_with_modifiers(word_text) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    word_like
        .bare_word()
        .is_some_and(|word| word.kind() == crate::WordKind::Lujvo)
}

#[requires(!candidate.word.is_empty())]
#[ensures(ret -> !candidate.parts.is_empty())]
fn is_valid_lujvo_candidate(candidate: &LujvoCandidate) -> bool {
    let Ok(words) = crate::segment_words_with_modifiers(&candidate.word) else {
        return false;
    };
    let [word_like] = words.as_slice() else {
        return false;
    };
    let Some(word) = word_like.bare_word() else {
        return false;
    };
    if word.kind() != crate::WordKind::Lujvo {
        return false;
    }
    word.lujvo_parts()
        .is_some_and(|parts| lujvo_parts_match(parts.as_slice(), &candidate.parts))
}

#[requires(true)]
#[ensures(true)]
fn lujvo_parts_match(actual: &[crate::LujvoPart], expected: &[String]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(part, expected)| crate::canonical_text_eq(part.phonemes().as_str(), expected))
}

#[requires(true)]
#[ensures(true)]
pub fn ensure_cmevla_word(word_text: &str) -> String {
    if is_cmevla(word_text) {
        word_text.to_owned()
    } else {
        format!("{word_text}s")
    }
}

#[requires(true)]
#[ensures(true)]
pub fn ends_with_consonant(word_text: &str) -> bool {
    word_text.chars().last().is_some_and(is_consonant)
}

#[requires(true)]
#[ensures(true)]
pub fn ends_with_vowel(word_text: &str) -> bool {
    word_text.chars().last().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
pub fn is_bonding_hyphen(part: &str) -> bool {
    matches!(part, "y" | "y'" | "'y" | "'y'" | "r" | "n")
}

#[requires(true)]
#[ensures(true)]
pub fn syllables_pattern(text: &str) -> Option<String> {
    text.chars().map(classify_syllable_char).collect()
}

#[requires(true)]
#[ensures(true)]
pub fn rafsi_shape(text: &str) -> RafsiShape {
    let mut pattern = text.chars().map(classify_syllable_char);
    let first = pattern.next().flatten();
    let second = pattern.next().flatten();
    let third = pattern.next().flatten();
    let fourth = pattern.next().flatten();
    let fifth = pattern.next().flatten();
    let sixth = pattern.next().flatten();
    match (first, second, third, fourth, fifth, sixth) {
        (Some('C'), Some('V'), Some('C'), Some('C'), Some('V'), None) => RafsiShape::Cvccv,
        (Some('C'), Some('V'), Some('C'), Some('C'), None, None) => RafsiShape::Cvcc,
        (Some('C'), Some('C'), Some('V'), Some('C'), Some('V'), None) => RafsiShape::Ccvcv,
        (Some('C'), Some('C'), Some('V'), Some('C'), None, None) => RafsiShape::Ccvc,
        (Some('C'), Some('V'), Some('C'), None, None, None) => RafsiShape::Cvc,
        (Some('C'), Some('V'), Some('\''), Some('V'), None, None) => RafsiShape::CvhV,
        (Some('C'), Some('C'), Some('V'), None, None, None) => RafsiShape::Ccv,
        (Some('C'), Some('V'), Some('V'), None, None, None) => RafsiShape::Cvv,
        _ => RafsiShape::Other,
    }
}

/// Return every short rafsi a gismu of this shape may claim under CLL 4.6.
///
/// Purely phonotactic: whether a form is already assigned to another word is a
/// dictionary question, answered by `jbotci_dictionary::Dictionary`.
/// Input that is not a well-formed CVCCV or CCVCV gismu yields no forms.
#[requires(true)]
#[ensures(GismuShape::classify(gismu).is_none() -> ret.is_empty())]
#[ensures(
    ret.windows(2).all(|pair| pair[0].form < pair[1].form),
    "forms are returned sorted by spelling with duplicates removed"
)]
pub fn possible_short_rafsi_forms(gismu: &str) -> Vec<ShortRafsiForm> {
    let Some(shape) = GismuShape::classify(gismu) else {
        return Vec::new();
    };
    let letters = gismu.chars().collect::<Vec<_>>();
    let mut forms = Vec::new();
    match shape {
        GismuShape::Cvccv => {
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvc,
                String::from_iter([letters[0], letters[1], letters[2]]),
            );
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvc,
                String::from_iter([letters[0], letters[1], letters[3]]),
            );
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvv,
                String::from_iter([letters[0], letters[1], '\'', letters[4]]),
            );
            push_cvv_without_apostrophe(&mut forms, letters[0], letters[1], letters[4]);
            push_ccv_if_initial(&mut forms, letters[2], letters[3], letters[4]);
            push_ccv_if_initial(&mut forms, letters[0], letters[2], letters[1]);
        }
        GismuShape::Ccvcv => {
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvc,
                String::from_iter([letters[0], letters[2], letters[3]]),
            );
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvc,
                String::from_iter([letters[1], letters[2], letters[3]]),
            );
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvv,
                String::from_iter([letters[0], letters[2], '\'', letters[4]]),
            );
            push_cvv_without_apostrophe(&mut forms, letters[0], letters[2], letters[4]);
            push_short_rafsi(
                &mut forms,
                ShortRafsiShape::Cvv,
                String::from_iter([letters[1], letters[2], '\'', letters[4]]),
            );
            push_cvv_without_apostrophe(&mut forms, letters[1], letters[2], letters[4]);
            push_ccv_if_initial(&mut forms, letters[0], letters[1], letters[2]);
        }
    }
    forms.sort_by(|left, right| left.form.cmp(&right.form));
    forms.dedup_by(|left, right| left.form == right.form);
    forms
}

#[requires(shape.matches_form(&form))]
#[ensures(output.len() == old(output.len()) + 1)]
fn push_short_rafsi(output: &mut Vec<ShortRafsiForm>, shape: ShortRafsiShape, form: String) {
    output.push(new!(ShortRafsiForm {
        form: form,
        shape: shape,
    }));
}

/// Push the apostrophe-free CVV rafsi, which exists only for the four
/// diphthongs `ai`, `ei`, `oi`, and `au` (CLL 4.6).
#[requires(is_consonant(consonant))]
#[requires(is_vowel(first_vowel) && is_vowel(second_vowel))]
#[ensures(output.len() <= old(output.len()) + 1)]
fn push_cvv_without_apostrophe(
    output: &mut Vec<ShortRafsiForm>,
    consonant: char,
    first_vowel: char,
    second_vowel: char,
) {
    if matches!(
        (first_vowel, second_vowel),
        ('a', 'i') | ('e', 'i') | ('o', 'i') | ('a', 'u')
    ) {
        push_short_rafsi(
            output,
            ShortRafsiShape::Cvv,
            String::from_iter([consonant, first_vowel, second_vowel]),
        );
    }
}

/// Push a CCV rafsi only when its consonant pair is a permissible initial
/// cluster, since a CCV rafsi may begin a lujvo.
#[requires(is_consonant(first) && is_consonant(second))]
#[requires(is_vowel(vowel))]
#[ensures(output.len() <= old(output.len()) + 1)]
fn push_ccv_if_initial(output: &mut Vec<ShortRafsiForm>, first: char, second: char, vowel: char) {
    if consonant_pair_class(first, second) == Some(ConsonantPairClass::Initial) {
        push_short_rafsi(
            output,
            ShortRafsiShape::Ccv,
            String::from_iter([first, second, vowel]),
        );
    }
}

#[requires(true)]
#[ensures(true)]
pub fn is_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u')
}

#[requires(true)]
#[ensures(true)]
pub fn is_consonant(value: char) -> bool {
    matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmevla(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|value| !matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | '\''))
}

#[requires(true)]
#[ensures(true)]
pub fn consonant_pair_class(first: char, second: char) -> Option<ConsonantPairClass> {
    crate::segment::consonant_pair_class(first, second)
}

#[requires(true)]
#[ensures(ret == consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_permissible))]
pub fn permissible_consonant_pair(first: char, second: char) -> bool {
    consonant_pair_class(first, second).is_some_and(ConsonantPairClass::is_permissible)
}

#[requires(true)]
#[ensures(true)]
fn classify_syllable_char(value: char) -> Option<char> {
    if is_vowel(value) {
        Some('V')
    } else if is_consonant(value) {
        Some('C')
    } else if value == '\'' {
        Some('\'')
    } else if value == 'y' {
        Some('Y')
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn needs_y_hyphen(previous: &str, next: &str) -> bool {
    let previous_shape = rafsi_shape(previous);
    let previous_tail = previous.chars().last();
    let next_head = next.chars().next();
    matches!(previous_shape, RafsiShape::Cvcc | RafsiShape::Ccvc)
        || matches!(
            (previous_tail, next_head),
            (Some(left), Some(right))
                if is_consonant(left)
                    && is_consonant(right)
                    && consonant_pair_class(left, right) == Some(ConsonantPairClass::Forbidden)
        )
        || (previous_tail == Some('n')
            && (next.starts_with("ts")
                || next.starts_with("tc")
                || next.starts_with("dz")
                || next.starts_with("dj")))
}

#[requires(true)]
#[ensures(ret.is_none_or(|hyphen| is_bonding_hyphen(hyphen)))]
fn y_hyphen_for_pair(previous: &str, next: &str) -> Option<&'static str> {
    let previous_tail = previous.chars().last();
    let needs_hyphen = needs_y_hyphen(previous, next)
        || previous_tail.is_some_and(is_consonant) && starts_with_vowel_or_glide(next);
    needs_hyphen.then(|| {
        if starts_with_vowel_nucleus_after_y(next) {
            "y'"
        } else {
            "y"
        }
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|hyphen| is_bonding_hyphen(hyphen)))]
fn hyphen_for_build_part_pair(
    previous: &LujvoBuildPart,
    next: &LujvoBuildPart,
) -> Option<&'static str> {
    if previous.is_brivla_core() && previous.as_text().chars().last().is_some_and(is_vowel) {
        Some("'y")
    } else {
        y_hyphen_for_pair(previous.as_text(), next.as_text())
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_vowel_or_glide(text: &str) -> bool {
    text.chars().next().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
fn starts_with_vowel_nucleus_after_y(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_vowel(first) {
        return false;
    }
    !matches!(first, 'i' | 'u') || !chars.next().is_some_and(is_vowel)
}

#[requires(true)]
#[ensures(true)]
fn should_insert_cvv_hyphen(first_rafsi: &str, second: &str, rafsi_count: usize) -> bool {
    matches!(rafsi_shape(first_rafsi), RafsiShape::Cvv | RafsiShape::CvhV)
        && (rafsi_count > 2 || rafsi_shape(second) != RafsiShape::Ccv)
}

#[requires(true)]
#[ensures(true)]
fn tosmabru(parts: &[String]) -> bool {
    let Some(last_part) = parts.last() else {
        return false;
    };
    if is_cmevla(last_part) {
        return false;
    }
    if let Some(y_index) = parts.iter().position(|part| part == "y") {
        let heads = &parts[..y_index];
        return heads.len() > 1
            && heads
                .iter()
                .all(|part| rafsi_shape(part) == RafsiShape::Cvc)
            && heads
                .windows(2)
                .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
    }
    if rafsi_shape(last_part) == RafsiShape::Cvccv {
        let chars = last_part.chars().collect::<Vec<_>>();
        if chars.len() >= 4
            && is_consonant(chars[2])
            && is_consonant(chars[3])
            && consonant_pair_class(chars[2], chars[3]) == Some(ConsonantPairClass::Initial)
        {
            let heads = &parts[..parts.len().saturating_sub(1)];
            return !heads.is_empty()
                && heads
                    .iter()
                    .all(|part| rafsi_shape(part) == RafsiShape::Cvc)
                && parts
                    .windows(2)
                    .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn consonant_pair_is_rank_two(left: &str, right: &str) -> bool {
    matches!(
        (left.chars().last(), right.chars().next()),
        (Some(left_tail), Some(right_head))
            if is_consonant(left_tail)
                && is_consonant(right_head)
                && consonant_pair_class(left_tail, right_head) == Some(ConsonantPairClass::Initial)
    )
}

#[requires(true)]
#[ensures(true)]
fn lujvo_score(rafsi_sequence: &[String]) -> i32 {
    let lujvo_text = rafsi_sequence.concat();
    let total_length = lujvo_text.chars().count() as i32;
    let apostrophe_count = lujvo_text.chars().filter(|value| *value == '\'').count() as i32;
    let hyphen_count = rafsi_sequence
        .iter()
        .filter(|part| is_bonding_hyphen(part))
        .count() as i32;
    let rafsi_shape_score = rafsi_sequence
        .iter()
        .map(|part| rafsi_shape(part).score())
        .sum::<i32>();
    let vowel_count = lujvo_text.chars().filter(|value| is_vowel(*value)).count() as i32;
    1000 * total_length - 500 * apostrophe_count + 100 * hyphen_count
        - 10 * rafsi_shape_score
        - vowel_count
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires, try_new};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bonds_cvv_initial_rafsi_with_r_or_n() {
        assert_eq!(
            bond_rafsis(&["bau".to_owned(), "gri".to_owned(), "kla".to_owned()]),
            Some(vec![
                "bau".to_owned(),
                "r".to_owned(),
                "gri".to_owned(),
                "kla".to_owned()
            ])
        );
        assert_eq!(
            bond_rafsis(&["bau".to_owned(), "rok".to_owned()]),
            Some(vec!["bau".to_owned(), "n".to_owned(), "rok".to_owned()])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inserts_y_for_impermissible_consonant_pair() {
        assert_eq!(
            bond_rafsis(&["jbon".to_owned(), "bau".to_owned()]),
            Some(vec!["jbon".to_owned(), "y".to_owned(), "bau".to_owned()])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn textual_lujvo_helpers_reject_empty_parts() {
        assert_eq!(bond_rafsis(&[String::new(), "jbo".to_owned()]), None);
        assert_eq!(
            choose_best_lujvo_candidate(
                LujvoBuildMode::Lujvo,
                &[vec![String::new(), "jbo".to_owned()]],
            ),
            None
        );
        assert!(try_new!(LujvoBuildPart::Rafsi(String::new())).is_err());
        assert!(try_new!(LujvoBuildPart::BrivlaCore(String::new())).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strict_lujvo_candidates_need_valid_lujvo_shape() {
        assert!(is_valid_lujvo_candidate_word("jbogri"));
        assert!(is_valid_lujvo_candidate_word("soirsai"));
        assert!(is_valid_lujvo_candidate_word("ro'inre'o"));
        assert!(is_valid_lujvo_candidate_word("jetcybolxada"));
        assert!(!is_valid_lujvo_candidate_word("babau"));
        assert!(!is_valid_lujvo_candidate_word("soisai"));
        assert!(!is_valid_lujvo_candidate_word("xlamkai"));
        assert!(!is_valid_lujvo_candidate_word("xlaglymlu"));
        assert!(!is_valid_lujvo_candidate_word("kerlyu'ukerlo"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_spellings(gismu: &str) -> Vec<String> {
        possible_short_rafsi_forms(gismu)
            .into_iter()
            .map(|form| form.into_data().form)
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derives_every_cvccv_short_rafsi() {
        // CLL 4.6: `sakli` yields both CVC forms, the CV'V form, the
        // apostrophe-free `ai` diphthong form, and both permissible CCV forms.
        assert_eq!(
            short_rafsi_spellings("sakli"),
            vec!["kli", "sa'i", "sai", "sak", "sal", "ska"]
        );
        assert_eq!(
            possible_short_rafsi_forms("sakli")
                .into_iter()
                .map(|form| form.shape)
                .collect::<Vec<_>>(),
            vec![
                ShortRafsiShape::Ccv,
                ShortRafsiShape::Cvv,
                ShortRafsiShape::Cvv,
                ShortRafsiShape::Cvc,
                ShortRafsiShape::Cvc,
                ShortRafsiShape::Ccv,
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derives_every_ccvcv_short_rafsi() {
        // Either initial consonant heads a CVC and a CV'V form, and the
        // initial cluster itself heads one CCV form.
        assert_eq!(
            short_rafsi_spellings("bridi"),
            vec!["bi'i", "bid", "bri", "ri'i", "rid"]
        );
        // `blaci` additionally yields apostrophe-free forms, since `ai` is one
        // of the four diphthongs that need no apostrophe.
        assert_eq!(
            short_rafsi_spellings("blaci"),
            vec!["ba'i", "bac", "bai", "bla", "la'i", "lac", "lai"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skips_ccv_forms_with_impermissible_initial_clusters() {
        // `banli` yields neither `nli` nor `bna`: `nl` and `bn` may not begin a
        // Lojban word, so no lujvo could ever use those rafsi.
        assert_eq!(
            short_rafsi_spellings("banli"),
            vec!["ba'i", "bai", "bal", "ban"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_gismu_shapes_derive_no_short_rafsi() {
        for word in [
            "",
            "sakl",
            "sakliu",
            "coi",
            "jetcybolxada",
            "sákli",
            "sa,kli",
            // `rafsi_shape` alone would read this as CVCCV and stop at the
            // space; a gismu is exactly five letters.
            "toldu ",
            "toldu!",
            "not lojban at all",
        ] {
            assert!(
                possible_short_rafsi_forms(word).is_empty(),
                "{word} is not a gismu"
            );
            assert_eq!(GismuShape::classify(word), None, "{word} is not a gismu");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_shapes_validate_the_whole_spelling() {
        for (shape, form) in [
            (ShortRafsiShape::Cvc, "sal"),
            (ShortRafsiShape::Ccv, "ska"),
            (ShortRafsiShape::Cvv, "sa'i"),
            (ShortRafsiShape::Cvv, "sai"),
            (ShortRafsiShape::Cvv, "sei"),
            (ShortRafsiShape::Cvv, "soi"),
            (ShortRafsiShape::Cvv, "sau"),
        ] {
            assert!(shape.matches_form(form), "{shape:?} should accept {form}");
        }

        for (shape, form) in [
            // Trailing text must be rejected: `rafsi_shape` alone stops at the
            // first character it does not recognize.
            (ShortRafsiShape::Cvc, "sal!"),
            (ShortRafsiShape::Cvc, "sal!!!!"),
            (ShortRafsiShape::Cvv, "sa'i "),
            // Only ai, ei, oi, and au need no apostrophe.
            (ShortRafsiShape::Cvv, "sae"),
            (ShortRafsiShape::Cvv, "sia"),
            // Wrong length.
            (ShortRafsiShape::Cvc, "sa"),
            (ShortRafsiShape::Cvc, "salk"),
            (ShortRafsiShape::Ccv, ""),
            // A CCV rafsi may begin a lujvo, so its cluster must be initial.
            (ShortRafsiShape::Ccv, "nra"),
            (ShortRafsiShape::Ccv, "bna"),
            // Wrong letter classes for the declared shape.
            (ShortRafsiShape::Cvc, "ska"),
            (ShortRafsiShape::Ccv, "sal"),
            (ShortRafsiShape::Cvv, "sal"),
            (ShortRafsiShape::Cvc, "sái"),
        ] {
            assert!(!shape.matches_form(form), "{shape:?} should reject {form}");
        }

        assert!(
            try_new!(ShortRafsiForm {
                form: "sal!".to_owned(),
                shape: ShortRafsiShape::Cvc,
            })
            .is_err()
        );
        assert!(
            try_new!(ShortRafsiForm {
                form: "sae".to_owned(),
                shape: ShortRafsiShape::Cvv,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classifies_both_gismu_shapes() {
        assert_eq!(GismuShape::classify("sakli"), Some(GismuShape::Cvccv));
        assert_eq!(GismuShape::classify("blaci"), Some(GismuShape::Ccvcv));
        // Derivation is pure phonotactics: a well-shaped word that no
        // dictionary lists still yields its short rafsi.
        assert_eq!(GismuShape::classify("toldu"), Some(GismuShape::Cvccv));
        assert_eq!(short_rafsi_spellings("toldu"), vec!["to'u", "tod", "tol"]);
    }
}
