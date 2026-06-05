//! Shared Lojban orthography rendering helpers.

use std::fmt;

#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::{invariant, requires};
use jbotci_morphology::WordKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum LojbanScript {
    #[default]
    Latin,
    Cyrillic,
    Zbalermorna,
}

impl fmt::Display for LojbanScript {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Latin => "latin",
            Self::Cyrillic => "cyrillic",
            Self::Zbalermorna => "zbalermorna",
        })
    }
}

#[requires(true)]
#[ensures(true)]
pub fn render_loose_latin_text_for_script(script: LojbanScript, text: &str) -> String {
    match script {
        LojbanScript::Latin => text.to_owned(),
        LojbanScript::Cyrillic => latin_surface_to_cyrillic(text),
        LojbanScript::Zbalermorna => latin_surface_to_zbalermorna(WordKind::Gismu, text),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn render_latin_word_surface_for_script(
    script: LojbanScript,
    kind: WordKind,
    latin: &str,
) -> String {
    match script {
        LojbanScript::Latin => latin.to_owned(),
        LojbanScript::Cyrillic => latin_surface_to_cyrillic(latin),
        LojbanScript::Zbalermorna => latin_surface_to_zbalermorna(kind, latin),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct NormalizedLatinChar {
    base: char,
    stressed: bool,
}

#[requires(true)]
#[ensures(true)]
fn normalized_latin_char(ch: char) -> NormalizedLatinChar {
    match ch {
        'á' | 'Á' => NormalizedLatinChar {
            base: 'a',
            stressed: true,
        },
        'é' | 'É' => NormalizedLatinChar {
            base: 'e',
            stressed: true,
        },
        'í' | 'Í' => NormalizedLatinChar {
            base: 'i',
            stressed: true,
        },
        'ó' | 'Ó' => NormalizedLatinChar {
            base: 'o',
            stressed: true,
        },
        'ú' | 'Ú' => NormalizedLatinChar {
            base: 'u',
            stressed: true,
        },
        'ý' | 'Ý' => NormalizedLatinChar {
            base: 'y',
            stressed: true,
        },
        'A' | 'E' | 'I' | 'O' | 'U' | 'Y' => NormalizedLatinChar {
            base: ch.to_ascii_lowercase(),
            stressed: true,
        },
        'ĭ' | 'Ĭ' => NormalizedLatinChar {
            base: 'ĭ',
            stressed: false,
        },
        'ŭ' | 'Ŭ' => NormalizedLatinChar {
            base: 'ŭ',
            stressed: false,
        },
        other => NormalizedLatinChar {
            base: other.to_ascii_lowercase(),
            stressed: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn latin_surface_to_cyrillic(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        let normalized = normalized_latin_char(ch);
        match normalized.base {
            '.' => output.push('.'),
            ',' => output.push(','),
            '\'' => {}
            'a' => push_cyrillic_vowel(&mut output, 'а', normalized.stressed),
            'e' => push_cyrillic_vowel(&mut output, 'е', normalized.stressed),
            'i' => push_cyrillic_vowel(&mut output, 'и', normalized.stressed),
            'o' => push_cyrillic_vowel(&mut output, 'о', normalized.stressed),
            'u' => push_cyrillic_vowel(&mut output, 'у', normalized.stressed),
            'y' => push_cyrillic_vowel(&mut output, 'ъ', normalized.stressed),
            'ĭ' => output.push('й'),
            'ŭ' => output.push('ў'),
            'b' => output.push('б'),
            'c' => output.push('ш'),
            'd' => output.push('д'),
            'f' => output.push('ф'),
            'g' => output.push('г'),
            'j' => output.push('ж'),
            'k' => output.push('к'),
            'l' => output.push('л'),
            'm' => output.push('м'),
            'n' => output.push('н'),
            'p' => output.push('п'),
            'r' => output.push('р'),
            's' => output.push('с'),
            't' => output.push('т'),
            'v' => output.push('в'),
            'x' => output.push('х'),
            'z' => output.push('з'),
            other => output.push(other),
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn push_cyrillic_vowel(output: &mut String, vowel: char, stressed: bool) {
    output.push(vowel);
    if stressed {
        output.push('\u{0301}');
    }
}

#[requires(true)]
#[ensures(true)]
fn latin_surface_to_zbalermorna(kind: WordKind, text: &str) -> String {
    let full_vowels = matches!(kind, WordKind::Fuhivla | WordKind::Cmevla);
    let chars = text.chars().map(normalized_latin_char).collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        let normalized = chars[index];
        if !full_vowels && let Some(diphthong) = zbalermorna_regular_diphthong(&chars, index) {
            output.push(diphthong);
            if normalized.stressed {
                output.push('\u{ed98}');
            }
            index += 2;
            continue;
        }
        match normalized.base {
            '.' => output.push('\u{ed89}'),
            '\'' => output.push('\u{ed8a}'),
            ',' if full_vowels => output.push('\u{ed9a}'),
            ',' => {}
            'a' => push_zbalermorna_vowel(&mut output, 'a', full_vowels, normalized.stressed),
            'e' => push_zbalermorna_vowel(&mut output, 'e', full_vowels, normalized.stressed),
            'i' => push_zbalermorna_vowel(&mut output, 'i', full_vowels, normalized.stressed),
            'o' => push_zbalermorna_vowel(&mut output, 'o', full_vowels, normalized.stressed),
            'u' => push_zbalermorna_vowel(&mut output, 'u', full_vowels, normalized.stressed),
            'y' => push_zbalermorna_vowel(&mut output, 'y', full_vowels, normalized.stressed),
            'ĭ' => push_zbalermorna_semivowel_or_full_vowel(
                &mut output,
                'ĭ',
                full_vowels,
                zbalermorna_next_is_vowel(&chars, index),
            ),
            'ŭ' => push_zbalermorna_semivowel_or_full_vowel(
                &mut output,
                'ŭ',
                full_vowels,
                zbalermorna_next_is_vowel(&chars, index),
            ),
            'b' => output.push('\u{ed90}'),
            'c' => output.push('\u{ed86}'),
            'd' => output.push('\u{ed91}'),
            'f' => output.push('\u{ed83}'),
            'g' => output.push('\u{ed92}'),
            'j' => output.push('\u{ed96}'),
            'k' => output.push('\u{ed82}'),
            'l' => output.push('\u{ed84}'),
            'm' => output.push('\u{ed87}'),
            'n' => output.push('\u{ed97}'),
            'p' => output.push('\u{ed80}'),
            'r' => output.push('\u{ed94}'),
            's' => output.push('\u{ed85}'),
            't' => output.push('\u{ed81}'),
            'v' => output.push('\u{ed93}'),
            'x' => output.push('\u{ed88}'),
            'z' => output.push('\u{ed95}'),
            other => output.push(other),
        }
        index += 1;
    }
    output
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn zbalermorna_regular_diphthong(chars: &[NormalizedLatinChar], index: usize) -> Option<char> {
    let first = chars.get(index)?;
    let second = chars.get(index + 1)?;
    match (first.base, second.base) {
        ('a', 'ĭ') => Some('\u{eda6}'),
        ('e', 'ĭ') => Some('\u{eda7}'),
        ('o', 'ĭ') => Some('\u{eda8}'),
        ('a', 'ŭ') => Some('\u{eda9}'),
        _ => None,
    }
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn zbalermorna_next_is_vowel(chars: &[NormalizedLatinChar], index: usize) -> bool {
    chars
        .get(index + 1)
        .is_some_and(|next| matches!(next.base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

#[requires(matches!(vowel, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
#[ensures(true)]
fn push_zbalermorna_vowel(output: &mut String, vowel: char, full: bool, stressed: bool) {
    let codepoint = match (full, vowel) {
        (false, 'a') => '\u{eda0}',
        (false, 'e') => '\u{eda1}',
        (false, 'i') => '\u{eda2}',
        (false, 'o') => '\u{eda3}',
        (false, 'u') => '\u{eda4}',
        (false, 'y') => '\u{eda5}',
        (true, 'a') => '\u{edb0}',
        (true, 'e') => '\u{edb1}',
        (true, 'i') => '\u{edb2}',
        (true, 'o') => '\u{edb3}',
        (true, 'u') => '\u{edb4}',
        (true, 'y') => '\u{edb5}',
        _ => unreachable!("requires Lojban vowel"),
    };
    output.push(codepoint);
    if stressed {
        output.push('\u{ed98}');
    }
}

#[requires(matches!(semivowel, 'ĭ' | 'ŭ'))]
#[ensures(true)]
fn push_zbalermorna_semivowel_or_full_vowel(
    output: &mut String,
    semivowel: char,
    full: bool,
    followed_by_vowel: bool,
) {
    if full && !followed_by_vowel {
        push_zbalermorna_vowel(
            output,
            match semivowel {
                'ĭ' => 'i',
                'ŭ' => 'u',
                _ => unreachable!("requires semivowel"),
            },
            true,
            false,
        );
    } else {
        output.push(match semivowel {
            'ĭ' => '\u{edaa}',
            'ŭ' => '\u{edab}',
            _ => unreachable!("requires semivowel"),
        });
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::ensures;
    use bityzba::requires;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_cyrillic_surface() {
        assert_eq!(
            render_latin_word_surface_for_script(LojbanScript::Cyrillic, WordKind::Gismu, "kláma"),
            "кла\u{0301}ма"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_zbalermorna_surface() {
        assert_eq!(
            render_latin_word_surface_for_script(
                LojbanScript::Zbalermorna,
                WordKind::Gismu,
                "kláma"
            ),
            "\u{ed82}\u{ed84}\u{eda0}\u{ed98}\u{ed87}\u{eda0}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_zbalermorna_regular_diphthongs() {
        assert_eq!(
            render_latin_word_surface_for_script(LojbanScript::Zbalermorna, WordKind::Cmavo, "coĭ"),
            "\u{ed86}\u{eda8}"
        );
        assert_eq!(
            render_latin_word_surface_for_script(LojbanScript::Zbalermorna, WordKind::Cmavo, "keĭ"),
            "\u{ed82}\u{eda7}"
        );
        assert_eq!(
            render_latin_word_surface_for_script(
                LojbanScript::Zbalermorna,
                WordKind::Cmavo,
                "co'i"
            ),
            "\u{ed86}\u{eda3}\u{ed8a}\u{eda2}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_zbalermorna_full_vowel_mode_glides_by_position() {
        assert_eq!(
            render_latin_word_surface_for_script(
                LojbanScript::Zbalermorna,
                WordKind::Cmevla,
                "ĭan"
            ),
            "\u{edaa}\u{edb0}\u{ed97}"
        );
        assert_eq!(
            render_latin_word_surface_for_script(
                LojbanScript::Zbalermorna,
                WordKind::Cmevla,
                "coĭs"
            ),
            "\u{ed86}\u{edb3}\u{edb2}\u{ed85}"
        );
    }
}
