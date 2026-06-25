//! IPA → Lojban phoneme transliteration for gimfihi source words.
//!
//! Implements `docs/source-word-transliteration.md`. A source word given as a
//! broad *phonemic* IPA transcription is reduced to Lojban's gismu-scoring sound
//! inventory so the scorer can match candidates against it. We normalize the IPA
//! (NFD, drop the affricate tie bar, drop suprasegmentals and quality
//! diacritics, decompose palatalization/labialization/nasalization/rhoticity),
//! collapse stop-plus-fricative affricates to the fricative, snap every
//! remaining phoneme to the nearest Lojban letter, and reject a bare schwa so
//! the caller must commit to a real vowel.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{is_consonant, is_vowel};
use unicode_normalization::UnicodeNormalization;

use crate::GimfihiError;

/// What a base IPA segment snaps to in Lojban.
#[invariant(true)]
#[invariant(::Letters(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Snap {
    /// Emit these Lojban letters. `i`/`u` double as on-glides, and a palatal
    /// consonant emits two letters (`ɲ` → `ni`, `ʎ` → `li`).
    Letters(&'static str),
    /// No gismu-scoring target: the glottal stop `ʔ` and ʿayn `ʕ`.
    Drop,
    /// The bare schwa `ə`: reject so the caller resolves it to a full vowel.
    RejectSchwa,
}

/// How a trailing IPA modifier affects the segment it attaches to.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    /// Length, aspiration, emphasis, voicing/quality/tone marks: snap the base.
    Ignore,
    /// Palatalization `ʲ` → an `i` on-glide.
    Palatalize,
    /// Labialization `ʷ` → a `u` on-glide.
    Labialize,
    /// Nasalization `◌̃` → a nasal consonant after the vocalic run.
    Nasalize,
    /// Rhoticity `˞` → a following `r`.
    Rhotic,
}

/// Base IPA segment → Lojban snap. Matched greedily longest-first, so the
/// affricate digraphs (`tʃ`, `ts`, …) win over their leading stop.
const IPA_SNAPS: &[(&str, Snap)] = &[
    // Affricates (a stop plus its matching fricative) collapse to the fricative.
    ("tʃ", Snap::Letters("c")),
    ("dʒ", Snap::Letters("j")),
    ("tɕ", Snap::Letters("c")),
    ("dʑ", Snap::Letters("j")),
    ("ʈʂ", Snap::Letters("c")),
    ("ɖʐ", Snap::Letters("j")),
    ("ts", Snap::Letters("s")),
    ("dz", Snap::Letters("z")),
    ("pf", Snap::Letters("f")),
    // Plosives (dental/alveolar/retroflex and uvular all neutralize).
    ("p", Snap::Letters("p")),
    ("b", Snap::Letters("b")),
    ("t", Snap::Letters("t")),
    ("ʈ", Snap::Letters("t")),
    ("d", Snap::Letters("d")),
    ("ɖ", Snap::Letters("d")),
    ("k", Snap::Letters("k")),
    ("q", Snap::Letters("k")),
    ("g", Snap::Letters("g")),
    ("ɡ", Snap::Letters("g")),
    // Labial fricatives.
    ("f", Snap::Letters("f")),
    ("ɸ", Snap::Letters("f")),
    ("v", Snap::Letters("v")),
    ("ʋ", Snap::Letters("v")),
    // Coronal fricatives (θ → s, ð → z).
    ("s", Snap::Letters("s")),
    ("θ", Snap::Letters("s")),
    ("z", Snap::Letters("z")),
    ("ð", Snap::Letters("z")),
    // Sibilants — postalveolar/alveolo-palatal/retroflex → c / j.
    ("ʃ", Snap::Letters("c")),
    ("ɕ", Snap::Letters("c")),
    ("ʂ", Snap::Letters("c")),
    ("ʒ", Snap::Letters("j")),
    ("ʑ", Snap::Letters("j")),
    ("ʐ", Snap::Letters("j")),
    // Every fricative at or behind the velum → x.
    ("x", Snap::Letters("x")),
    ("ɣ", Snap::Letters("x")),
    ("χ", Snap::Letters("x")),
    ("c\u{0327}", Snap::Letters("x")), // ç, NFD-decomposed to c + combining cedilla
    ("ħ", Snap::Letters("x")),
    ("h", Snap::Letters("x")),
    ("ɦ", Snap::Letters("x")),
    // Nasals (palatal ɲ → n + i-glide).
    ("m", Snap::Letters("m")),
    ("ɱ", Snap::Letters("m")),
    ("n", Snap::Letters("n")),
    ("ŋ", Snap::Letters("n")),
    ("ɳ", Snap::Letters("n")),
    ("ɴ", Snap::Letters("n")),
    ("ɲ", Snap::Letters("ni")),
    // Liquids (palatal ʎ → l + i-glide; every rhotic → r).
    ("l", Snap::Letters("l")),
    ("ɫ", Snap::Letters("l")),
    ("ɭ", Snap::Letters("l")),
    ("ʎ", Snap::Letters("li")),
    ("r", Snap::Letters("r")),
    ("ɾ", Snap::Letters("r")),
    ("ɹ", Snap::Letters("r")),
    ("ɻ", Snap::Letters("r")),
    ("ʀ", Snap::Letters("r")),
    ("ʁ", Snap::Letters("r")),
    ("ɽ", Snap::Letters("r")),
    // Glides.
    ("j", Snap::Letters("i")),
    ("ʝ", Snap::Letters("i")),
    ("ɥ", Snap::Letters("i")),
    ("w", Snap::Letters("u")),
    ("ʍ", Snap::Letters("u")),
    // No scoring target.
    ("ʔ", Snap::Drop),
    ("ʕ", Snap::Drop),
    // Vowels: front, front-rounded, and central-unrounded → i/e; back and
    // central-rounded → u/o; open → a.
    ("i", Snap::Letters("i")),
    ("ɪ", Snap::Letters("i")),
    ("ɨ", Snap::Letters("i")),
    ("y", Snap::Letters("i")),
    ("ʏ", Snap::Letters("i")),
    ("u", Snap::Letters("u")),
    ("ʊ", Snap::Letters("u")),
    ("ɯ", Snap::Letters("u")),
    ("ʉ", Snap::Letters("u")),
    ("e", Snap::Letters("e")),
    ("ɛ", Snap::Letters("e")),
    ("ø", Snap::Letters("e")),
    ("œ", Snap::Letters("e")),
    ("ɘ", Snap::Letters("e")),
    ("ɜ", Snap::Letters("e")),
    ("o", Snap::Letters("o")),
    ("ɔ", Snap::Letters("o")),
    ("ɤ", Snap::Letters("o")),
    ("ɵ", Snap::Letters("o")),
    ("ɒ", Snap::Letters("o")),
    ("a", Snap::Letters("a")),
    ("æ", Snap::Letters("a")),
    ("ɐ", Snap::Letters("a")),
    ("ɑ", Snap::Letters("a")),
    ("ʌ", Snap::Letters("a")),
    // r-coloured mid-central vowels → e + r.
    ("ɚ", Snap::Letters("er")),
    ("ɝ", Snap::Letters("er")),
    // The bare schwa is rejected.
    ("ə", Snap::RejectSchwa),
];

#[requires(true)]
#[ensures(true)]
fn is_tie_bar(value: char) -> bool {
    matches!(value, '\u{0361}' | '\u{035C}')
}

#[requires(true)]
#[ensures(true)]
fn is_boundary(value: char) -> bool {
    value.is_whitespace()
        || matches!(value, '.' | '/' | '|' | '‖' | 'ˈ' | 'ˌ')
        || ('\u{02E5}'..='\u{02E9}').contains(&value) // tone letters ˥ ˦ ˧ ˨ ˩
}

#[requires(true)]
#[ensures(true)]
fn classify_modifier(value: char) -> Option<Modifier> {
    match value {
        '\u{0303}' => Some(Modifier::Nasalize), // combining tilde ◌̃
        'ʲ' => Some(Modifier::Palatalize),
        'ʷ' => Some(Modifier::Labialize),
        '˞' => Some(Modifier::Rhotic),
        // Other combining marks (quality, voicing, syllabicity, tone, length).
        marker if ('\u{0300}'..='\u{036F}').contains(&marker) => Some(Modifier::Ignore),
        // Spacing modifier letters (aspiration, length, emphasis, …); the
        // stress and tone marks in this block are boundaries, handled elsewhere.
        marker if ('\u{02B0}'..='\u{02FF}').contains(&marker) && !is_boundary(marker) => {
            Some(Modifier::Ignore)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_labial_consonant(value: char) -> bool {
    matches!(value, 'p' | 'b' | 'm' | 'f' | 'v')
}

/// Push Lojban letters, flushing a pending nasal as `m`/`n` before a consonant.
#[requires(true)]
#[ensures(true)]
fn push_letters(letters: &str, out: &mut String, pending_nasal: &mut bool) {
    for letter in letters.chars() {
        if is_consonant(letter) && *pending_nasal {
            out.push(if is_labial_consonant(letter) {
                'm'
            } else {
                'n'
            });
            *pending_nasal = false;
        }
        out.push(letter);
    }
}

#[requires(!remaining.is_empty())]
#[ensures(ret.is_none_or(|(length, _)| length > 0 && length <= remaining.len()))]
fn match_longest(remaining: &str) -> Option<(usize, Snap)> {
    IPA_SNAPS
        .iter()
        .filter(|(symbol, _)| remaining.starts_with(symbol))
        .max_by_key(|(symbol, _)| symbol.len())
        .map(|(symbol, snap)| (symbol.len(), *snap))
}

/// Drop a glide that immediately precedes the identical vowel (`i`-glide + `i`
/// → `i`), which Lojban writes as a single letter; no other doubled vowel
/// sequence arises from the snap.
#[requires(true)]
#[ensures(ret.chars().all(|letter| word.contains(letter)))]
fn collapse_glide_doubles(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut previous = '\0';
    for letter in word.chars() {
        if matches!(letter, 'i' | 'u') && letter == previous {
            continue;
        }
        out.push(letter);
        previous = letter;
    }
    out
}

/// Transliterate a broad phonemic IPA string into a Lojban gismu-scoring word.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|word| !word.is_empty() && word.chars().all(|letter| is_consonant(letter) || is_vowel(letter))) || ret.is_err())]
pub fn transliterate_ipa_to_lojban(ipa: &str) -> Result<String, GimfihiError> {
    // NFD so precomposed letters (ã, ä, é, …) become base + combining mark, then
    // drop the affricate tie bar so t͡ʃ reads as the digraph tʃ.
    let normalized: String = ipa
        .nfd()
        .filter(|character| !is_tie_bar(*character))
        .collect();

    let invalid = |reason: &str| GimfihiError::InvalidIpa {
        ipa: ipa.to_owned(),
        reason: reason.to_owned(),
    };

    let mut out = String::new();
    let mut pending_nasal = false;
    let mut remaining = normalized.as_str();
    while let Some(next) = remaining.chars().next() {
        if is_boundary(next) {
            if pending_nasal {
                out.push('n');
                pending_nasal = false;
            }
            remaining = &remaining[next.len_utf8()..];
            continue;
        }
        // A modifier with no preceding base segment to attach to: drop it.
        if classify_modifier(next).is_some() {
            remaining = &remaining[next.len_utf8()..];
            continue;
        }
        let Some((length, snap)) = match_longest(remaining) else {
            let near: String = remaining.chars().take(8).collect();
            return Err(invalid(&format!(
                "unsupported IPA segment near `{near}` — give a broad phonemic transcription"
            )));
        };
        match snap {
            Snap::RejectSchwa => {
                return Err(invalid(
                    "the neutral schwa `ə` has no Lojban vowel — transcribe the full vowel it is actually pronounced as",
                ));
            }
            Snap::Drop => {}
            Snap::Letters(letters) => push_letters(letters, &mut out, &mut pending_nasal),
        }
        remaining = &remaining[length..];
        // Consume the modifiers trailing this segment.
        while let Some(modifier_char) = remaining.chars().next() {
            let Some(modifier) = classify_modifier(modifier_char) else {
                break;
            };
            match modifier {
                Modifier::Ignore => {}
                Modifier::Palatalize => push_letters("i", &mut out, &mut pending_nasal),
                Modifier::Labialize => push_letters("u", &mut out, &mut pending_nasal),
                Modifier::Rhotic => push_letters("r", &mut out, &mut pending_nasal),
                Modifier::Nasalize => pending_nasal = true,
            }
            remaining = &remaining[modifier_char.len_utf8()..];
        }
    }
    if pending_nasal {
        out.push('n');
    }
    let collapsed = collapse_glide_doubles(&out);
    if collapsed.is_empty() {
        return Err(invalid(
            "no Lojban-scoring sounds remain after transliteration",
        ));
    }
    Ok(collapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn lojban(ipa: &str) -> String {
        transliterate_ipa_to_lojban(ipa).expect("valid IPA")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn worked_examples_from_the_spec() {
        assert_eq!(lojban("jʊŋɕin"), "iuncin");
        assert_eq!(lojban("ɕy"), "ci");
        assert_eq!(lojban("kæt"), "kat");
        assert_eq!(lojban("ɡat"), "gat");
        assert_eq!(lojban("haʊs"), "xaus");
        assert_eq!(lojban("spasʲiba"), "spasiba");
        assert_eq!(lojban("kitaːb"), "kitab");
        assert_eq!(lojban("ħasan"), "xasan");
        assert_eq!(lojban("bɔ̃"), "bon");
        assert_eq!(lojban("ty"), "ti");
        assert_eq!(lojban("ʃøːn"), "cen");
        assert_eq!(lojban("ɡʁyːn"), "grin");
        assert_eq!(lojban("sɯɕi"), "suci");
        assert_eq!(lojban("t͡ʃaːj"), "cai");
        assert_eq!(lojban("pɐ̃w̃"), "paun");
        assert_eq!(lojban("bʱalo"), "balo");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_back_fricative_snaps_to_x() {
        for fricative in ["x", "ɣ", "χ", "ç", "ħ", "h", "ɦ"] {
            assert_eq!(lojban(&format!("{fricative}a")), "xa", "{fricative}a");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn affricates_collapse_to_the_fricative() {
        assert_eq!(lojban("t͡ʃa"), "ca");
        assert_eq!(lojban("tʃa"), "ca");
        assert_eq!(lojban("d͡ʒa"), "ja");
        assert_eq!(lojban("t͡sa"), "sa");
        assert_eq!(lojban("p͡fa"), "fa");
        assert_eq!(lojban("t͡ɕa"), "ca");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn front_rounded_unrounds_and_back_unrounded_stays_back() {
        assert_eq!(lojban("y"), "i");
        assert_eq!(lojban("ø"), "e");
        assert_eq!(lojban("œ"), "e");
        assert_eq!(lojban("ɯ"), "u");
        assert_eq!(lojban("ɤ"), "o");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nasal_vowels_decompose_with_place_assimilation() {
        assert_eq!(lojban("ɑ̃"), "an");
        assert_eq!(lojban("ɛ̃"), "en");
        assert_eq!(lojban("ɔ̃"), "on");
        assert_eq!(lojban("ɑ̃pa"), "ampa"); // m before a labial
        assert_eq!(lojban("ɑ̃ta"), "anta"); // n otherwise
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn palatalization_and_labialization_become_glides() {
        assert_eq!(lojban("nʲe"), "nie");
        assert_eq!(lojban("kʷa"), "kua");
        assert_eq!(lojban("ɲa"), "nia"); // palatal nasal
        assert_eq!(lojban("ʎa"), "lia"); // palatal lateral
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn suprasegmentals_and_diacritics_are_stripped() {
        assert_eq!(lojban("ˈka.ma"), "kama"); // stress + syllable break
        assert_eq!(lojban("pʰa"), "pa"); // aspiration
        assert_eq!(lojban("sˤa"), "sa"); // emphasis
        assert_eq!(lojban("kʰ ʈʰ"), "kt"); // aspiration + retroflex + space
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn precomposed_letters_are_decomposed() {
        assert_eq!(lojban("ã"), "an"); // precomposed nasal a
        assert_eq!(lojban("ä"), "a"); // precomposed centralized a
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_schwa_is_rejected() {
        let error = transliterate_ipa_to_lojban("əbaut").unwrap_err();
        assert!(matches!(error, GimfihiError::InvalidIpa { .. }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_and_empty_inputs_are_rejected() {
        assert!(transliterate_ipa_to_lojban("ka•ma").is_err()); // • is not IPA
        assert!(transliterate_ipa_to_lojban("").is_err());
        assert!(transliterate_ipa_to_lojban("ʔ").is_err()); // only a dropped sound
    }
}
