use bityzba::requires;
#[allow(unused_imports)]
use bityzba::{ensures, expensive_ensures};

#[requires(true)]
#[ensures(true)]
pub fn strip_lojban_diacritic(value: char) -> Option<char> {
    Some(match value {
        'á' | 'à' | 'Á' | 'À' => 'a',
        'é' | 'è' | 'É' | 'È' => 'e',
        'í' | 'ì' | 'Í' | 'Ì' => 'i',
        'ó' | 'ò' | 'Ó' | 'Ò' => 'o',
        'ú' | 'ù' | 'Ú' | 'Ù' => 'u',
        'ý' | 'ỳ' | 'Ý' | 'Ỳ' => 'y',
        'ĭ' | 'Ĭ' => 'ĭ',
        'ŭ' | 'Ŭ' => 'ŭ',
        '\u{0301}' | '\u{0300}' | '\u{0306}' => return None,
        other => other,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn fold_lojban_diacritic(value: char) -> Option<char> {
    strip_lojban_diacritic(value).map(|stripped| match stripped {
        'ĭ' => 'i',
        'ŭ' => 'u',
        other => other,
    })
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
pub fn push_stripped_lojban_diacritics_to(text: &str, output: &mut String) {
    output.extend(text.chars().filter_map(strip_lojban_diacritic));
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
pub fn push_folded_lojban_diacritics_to(text: &str, output: &mut String) {
    output.extend(text.chars().filter_map(fold_lojban_diacritic));
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.chars().all(|value| strip_lojban_diacritic(value) == Some(value)))]
pub fn strip_lojban_diacritics(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    push_stripped_lojban_diacritics_to(text, &mut stripped);
    stripped
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.chars().all(|value| fold_lojban_diacritic(value) == Some(value)))]
pub fn fold_lojban_diacritics(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    push_folded_lojban_diacritics_to(text, &mut folded);
    folded
}

#[requires(true)]
#[ensures(true)]
pub fn stripped_lojban_diacritics_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter_map(strip_lojban_diacritic)
        .eq(right.chars().filter_map(strip_lojban_diacritic))
}

#[requires(true)]
#[ensures(true)]
pub fn folded_lojban_diacritics_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter_map(fold_lojban_diacritic)
        .eq(right.chars().filter_map(fold_lojban_diacritic))
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
pub fn push_stripped_diacritics_to(text: &str, output: &mut String) {
    push_folded_lojban_diacritics_to(text, output);
}

#[requires(true)]
#[ensures(true)]
pub fn strip_diacritics(text: &str) -> String {
    fold_lojban_diacritics(text)
}

#[requires(true)]
#[ensures(true)]
pub fn strip_diacritics_eq(left: &str, right: &str) -> bool {
    folded_lojban_diacritics_eq(left, right)
}
