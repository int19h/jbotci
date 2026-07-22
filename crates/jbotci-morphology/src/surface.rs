#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::requires;

use crate::{Cmavo, Selmaho, Word, WordKind, fold_lojban_diacritic};

crate::define_string_enum_metadata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LeadingPauseVowelMode {
        FoldedVowels => ("FOLDED_VOWELS", "folded-vowels"),
        LatinSurfaceVowels => ("LATIN_SURFACE_VOWELS", "latin-surface-vowels"),
    }
}

crate::define_string_enum_metadata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LeadingPauseContext {
        IndependentWord => ("INDEPENDENT_WORD", "independent-word"),
        BuLetterBase => ("BU_LETTER_BASE", "bu-letter-base"),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn word_needs_leading_pause(word: &Word, mode: LeadingPauseVowelMode) -> bool {
    word_needs_leading_pause_in_context(word, mode, LeadingPauseContext::IndependentWord)
}

#[requires(true)]
#[ensures(true)]
pub fn word_needs_leading_pause_in_context(
    word: &Word,
    mode: LeadingPauseVowelMode,
    context: LeadingPauseContext,
) -> bool {
    word.kind() == WordKind::Cmevla
        || y_initial_by_word_needs_leading_pause(word, context)
        || match mode {
            LeadingPauseVowelMode::FoldedVowels => starts_with_folded_vowel(word),
            LeadingPauseVowelMode::LatinSurfaceVowels => starts_with_latin_surface_vowel(word),
        }
}

#[requires(true)]
#[ensures(true)]
fn y_initial_by_word_needs_leading_pause(word: &Word, context: LeadingPauseContext) -> bool {
    // BPFK morphology treats bare `y` as hesitation noise (selma'o Y), so it
    // does not get a written leading pause. When the same surface is the base
    // of BU, the full word is BY (`ybu`); `y'y` is also BY. Those y-initial BY
    // words follow the normal written-leading-pause convention. See #254:
    // https://github.com/int19h/jbotci/issues/254#issuecomment-4898786631
    match word.cmavo() {
        Some(Cmavo::Y) => context == LeadingPauseContext::BuLetterBase,
        Some(cmavo) => cmavo.is_selmaho(Selmaho::By) && starts_with_folded_y(word),
        None => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_folded_vowel(word: &Word) -> bool {
    word.phonemes()
        .as_str()
        .chars()
        .filter_map(fold_lojban_diacritic)
        .next()
        .is_some_and(|value| matches!(value, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn starts_with_folded_y(word: &Word) -> bool {
    word.phonemes()
        .as_str()
        .chars()
        .filter_map(fold_lojban_diacritic)
        .next()
        .is_some_and(|value| value == 'y')
}

#[requires(true)]
#[ensures(true)]
fn starts_with_latin_surface_vowel(word: &Word) -> bool {
    word.phonemes()
        .as_str()
        .chars()
        .next()
        .is_some_and(is_latin_vowel_surface_char)
}

#[requires(true)]
#[ensures(true)]
fn is_latin_vowel_surface_char(value: char) -> bool {
    match value {
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' => true,
        other => matches!(
            other,
            'A' | 'E' | 'I' | 'O' | 'U' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú'
        ),
    }
}
