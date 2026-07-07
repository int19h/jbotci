#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::{invariant, requires};

use crate::{Word, WordKind, fold_lojban_diacritic};

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingPauseVowelMode {
    FoldedVowels,
    LatinSurfaceVowels,
}

#[requires(true)]
#[ensures(true)]
pub fn word_needs_leading_pause(word: &Word, mode: LeadingPauseVowelMode) -> bool {
    word.kind() == WordKind::Cmevla
        || match mode {
            LeadingPauseVowelMode::FoldedVowels => starts_with_folded_vowel(word),
            LeadingPauseVowelMode::LatinSurfaceVowels => starts_with_latin_surface_vowel(word),
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
