//! IPA → Lojban phoneme transliteration for gimfihi source words.
//!
//! Implements `docs/source-word-transliteration.md`. A source word given as a
//! phonetic IPA transcription within the supported inventory is reduced to
//! Lojban scoring letters. The scorer matches candidates against these letters.
//! We normalize the IPA
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

/// Transliterate supported phonetic IPA into a Lojban gismu-scoring word.
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
                "unsupported IPA segment `{next}` near `{near}`. Use narrow phonetic IPA. Only the inventory listed in the gimfihi word instructions is supported."
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

    #[requires(true)]
    #[ensures(true)]
    fn code_spans(text: &str) -> Vec<&str> {
        let parts: Vec<_> = text.split('`').collect();
        assert_eq!(parts.len() % 2, 1, "unclosed code span");
        let spans: Vec<_> = parts.into_iter().skip(1).step_by(2).collect();
        assert!(
            spans.iter().all(|span| !span.trim().is_empty()),
            "empty code span"
        );
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
        assert_eq!(
            text.matches(start).count(),
            1,
            "missing or repeated section start"
        );
        assert_eq!(
            text.matches(end).count(),
            1,
            "missing or repeated section end"
        );
        text.split_once(start)
            .unwrap()
            .1
            .split_once(end)
            .expect("section order")
            .0
    }

    #[requires(true)]
    #[ensures(true)]
    fn tool_ipa_documentation() -> String {
        let source = include_str!("../../../apps/jbotci/src/tool.rs");
        let source = section(
            source,
            "pub struct ToolGimfihiSource {",
            "pub word: String,",
        );
        let word = source.split_once("pub language: String,").unwrap().1;
        word.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim()
                    .strip_prefix("///")
                    .expect("unrecognized word docstring structure")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Code spans contain IPA. Source spelling uses italics.
    #[requires(true)]
    #[ensures(true)]
    fn assert_documented_ipa(text: &str) {
        let tool = tool_ipa_documentation();
        let documentation = section(
            &tool,
            "Use standard IPA from this inventory;",
            "Reason carefully",
        );
        let supported: std::collections::BTreeSet<_> = code_spans(documentation)
            .into_iter()
            .flat_map(|span| span.nfd())
            .collect();
        let spans = code_spans(text);
        assert!(!spans.is_empty(), "the document must contain IPA examples");
        for span in spans {
            for symbol in span.split_whitespace() {
                let symbol = symbol.trim_matches(['[', ']', '(', ')']).replace('◌', "a");
                assert!(!symbol.is_empty(), "empty IPA symbol");
                assert!(
                    symbol.nfd().all(|part| supported.contains(&part)),
                    "documented IPA `{symbol}` is outside the tool inventory and examples"
                );
                // A vowel supplies a base for standalone modifiers and tie bars.
                transliterate_ipa_to_lojban(&format!("a{symbol}")).unwrap_or_else(|error| {
                    panic!("documented IPA `{symbol}` was rejected: {error}")
                });
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_ipa_examples(text: &str) {
        assert_documented_ipa(text);
        assert!(text.contains('→'), "missing IPA example");
        for target in text.split('→').skip(1) {
            let target = target
                .trim_start()
                .strip_prefix('`')
                .expect("IPA example must be a code span");
            let value = target.split_once('`').expect("unclosed IPA example").0;
            transliterate_ipa_to_lojban(value.trim_matches(['[', ']'])).unwrap();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_skill_table(text: &str) {
        let mut rows = text.lines().filter(|line| !line.trim().is_empty());
        assert_eq!(rows.next().unwrap().trim(), "| Lang | Word | IPA | note |");
        assert_eq!(rows.next().unwrap().trim(), "|------|------|-----|------|");
        let mut count = 0;
        for row in rows {
            let row = row.trim();
            let cells: Vec<_> = row
                .strip_prefix('|')
                .and_then(|row| row.strip_suffix('|'))
                .expect("unrecognized table row")
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(cells.len(), 4, "unrecognized table columns");
            assert!(
                !cells[0].is_empty() && !cells[1].is_empty(),
                "empty source cell"
            );
            let ipa = cells[2]
                .strip_prefix('`')
                .and_then(|value| value.strip_suffix('`'))
                .expect("IPA cell must be a code span");
            assert!(!ipa.contains('`'), "unrecognized IPA cell");
            assert_documented_ipa(cells[2]);
            transliterate_ipa_to_lojban(ipa).unwrap();
            let notes = code_spans(cells[3]);
            if !notes.is_empty() {
                assert_documented_ipa(cells[3]);
            }
            count += 1;
        }
        assert!(count > 0, "missing table data rows");
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_skill_json(text: &str) {
        let parts: Vec<_> = text.split("```").collect();
        assert!(parts.len() > 1, "missing JSON examples");
        assert_eq!(parts.len() % 2, 1, "unclosed JSON example");
        for block in parts.into_iter().skip(1).step_by(2) {
            let json = block
                .strip_prefix("json\n")
                .expect("unrecognized example fence");
            // The introductory example contains a full-line placeholder comment.
            let json = json
                .lines()
                .filter(|line| {
                    if line.trim_start().starts_with("//") {
                        assert_eq!(
                            line.trim(),
                            "// ... one per preset language",
                            "unrecognized JSON comment"
                        );
                        false
                    } else {
                        true
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let value: serde_json::Value =
                serde_json::from_str(&json).expect("unrecognized JSON example");
            let sources = value
                .get("sources")
                .and_then(serde_json::Value::as_array)
                .expect("missing JSON sources");
            assert!(!sources.is_empty(), "empty JSON sources");
            for source in sources {
                let word = source
                    .get("word")
                    .and_then(serde_json::Value::as_str)
                    .expect("unrecognized JSON source");
                assert_documented_ipa(&format!("`{word}`"));
                transliterate_ipa_to_lojban(word).unwrap();
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_reference_layout(reference: &str) {
        let sections: Vec<_> = reference.split("## ").skip(1).collect();
        // Pin paragraph structure without copying IPA symbols. A markup change
        // must update the reader, so it cannot silently remove a transcription.
        let layouts: &[&[usize]] = &[
            &[0, 9, 2, 2, 0, 0, 0],
            &[7, 0, 8, 0, 5, 0],
            &[0, 11, 8, 6, 6, 8, 5, 4, 4, 10, 4],
            &[10],
        ];
        assert_eq!(
            sections.len(),
            layouts.len(),
            "unrecognized reference sections"
        );
        for (section, layout) in sections.into_iter().zip(layouts) {
            let body = section.split_once('\n').unwrap().1.trim();
            let paragraphs: Vec<_> = body.split("\n\n").collect();
            assert_eq!(
                paragraphs.len(),
                layout.len(),
                "unrecognized reference paragraphs"
            );
            for (paragraph, count) in paragraphs.into_iter().zip(*layout) {
                assert_eq!(
                    code_spans(paragraph).len(),
                    *count,
                    "unrecognized reference paragraph"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tool_word_inventory_and_examples_are_accepted() {
        let word = tool_ipa_documentation();
        let inventory = section(
            &word,
            "Use standard IPA from this inventory;",
            "Examples (word → IPA):",
        );
        // Pin the layout, not the symbols. Missing quotes and extra rows must fail.
        let lines: Vec<_> = inventory
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let counts = [2, 4, 1, 2, 3, 3, 3, 2];
        assert_eq!(lines.len(), counts.len(), "unrecognized inventory rows");
        for (line, count) in lines.into_iter().zip(counts) {
            assert_eq!(code_spans(line).len(), count, "unrecognized inventory row");
        }
        assert_documented_ipa(inventory);
        let examples = section(&word, "Examples (word → IPA):", "Reason carefully");
        assert_eq!(code_spans(examples).len(), 9, "unrecognized tool examples");
        assert_ipa_examples(examples);
        let prefix = word
            .split_once("Use standard IPA from this inventory;")
            .unwrap()
            .0;
        let spans = code_spans(prefix);
        assert_eq!(spans.len(), 2, "unrecognized word instructions");
        for span in spans {
            if span == "ə" {
                assert!(transliterate_ipa_to_lojban(span).is_err());
            } else {
                assert_documented_ipa(&format!("`{span}`"));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skill_transcriptions_are_accepted() {
        let reference = include_str!(
            "../../../apps/jbotci-server/resources/skills/gismu-making/references/ipa-transcription.md"
        );
        let headings: Vec<_> = reference
            .lines()
            .filter(|line| line.starts_with("## "))
            .collect();
        assert_eq!(
            headings,
            [
                "## The rules",
                "## Supported IPA inventory",
                "## Per-language gotchas",
                "## Examples (word → IPA)"
            ]
        );
        assert_reference_layout(reference);
        let (prefix, body) = reference.split_once("## The rules").unwrap();
        assert_eq!(
            code_spans(prefix).len(),
            3,
            "unrecognized reference introduction"
        );
        assert!(
            code_spans(prefix).iter().all(|span| *span == "gimfihi"),
            "unrecognized reference introduction code span"
        );
        assert_documented_ipa(body);
        let examples = body.split_once("## Examples (word → IPA)").unwrap().1;
        let mut count = 0;
        for line in examples.lines().filter(|line| !line.trim().is_empty()) {
            if line.starts_with("- ") {
                assert_ipa_examples(line);
                count += 1;
            } else {
                assert!(line.starts_with("  "), "unrecognized example row");
            }
        }
        assert!(count > 0, "missing example rows");
        assert_ipa_examples(examples);
        let skill =
            include_str!("../../../apps/jbotci-server/resources/skills/gismu-making/SKILL.md");
        let worked = section(skill, "## Worked example — a gismu for \"cat\"", "\nCall:");
        let start = worked
            .find("| Lang | Word | IPA | note |")
            .expect("missing IPA table");
        assert!(
            !worked[..start].contains('|'),
            "unrecognized data before table header"
        );
        assert_skill_table(&worked[start..]);
        assert_skill_json(skill);
    }

    #[test]
    #[should_panic(expected = "documented IPA `aβa` is outside")]
    #[requires(true)]
    #[ensures(true)]
    fn skill_table_rejects_ipa_with_an_unquoted_language_cell() {
        assert_skill_table(
            "| Lang | Word | IPA | note |\n|------|------|-----|------|\n| spa | gato | `aβa` | noun |",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn drift_readers_refuse_unrecognized_structure() {
        assert!(
            std::panic::catch_unwind(|| assert_skill_table(
                "| Lang | Word | IPA | note |\n|------|------|-----|------|\nspa gato `gat`"
            ))
            .is_err()
        );
        assert!(std::panic::catch_unwind(|| assert_skill_table("| Lang | Word | IPA | note |\n|------|------|-----|------|\n| spa | gato | gat | noun |")).is_err());
        assert!(std::panic::catch_unwind(|| assert_documented_ipa("`a` `")).is_err());
        assert!(std::panic::catch_unwind(|| assert_documented_ipa("`a` ``")).is_err());
        assert!(
            std::panic::catch_unwind(|| assert_ipa_examples("cat → `kat`; dog → aβa")).is_err()
        );
        assert!(std::panic::catch_unwind(|| section("start end start", "start", "end")).is_err());
        assert!(
            std::panic::catch_unwind(|| assert_skill_json(
                "```json\n{\"sources\":[{\"language\":\"spa\"}]}\n```"
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(|| assert_skill_json(
                "```json\n{\"sources\":[{\"word\":\"aβa\"}]}\n```"
            ))
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_layout_refuses_missing_quotes_and_extra_content() {
        let reference = include_str!(
            "../../../apps/jbotci-server/resources/skills/gismu-making/references/ipa-transcription.md"
        );
        let inventory = section(
            reference,
            "## Supported IPA inventory",
            "## Per-language gotchas",
        );
        let span = code_spans(inventory)[0];
        let changed = reference.replacen(&format!("`{span}`"), span, 1);
        assert!(std::panic::catch_unwind(|| assert_reference_layout(&changed)).is_err());
        let changed = format!("{reference}\n\nNew data row without a code span.");
        assert!(std::panic::catch_unwind(|| assert_reference_layout(&changed)).is_err());
        assert!(std::panic::catch_unwind(|| assert_skill_json("``` json\n{}\n```")).is_err());
        assert!(
            std::panic::catch_unwind(|| assert_skill_json(
                "```json\n// hidden source row\n{}\n```"
            ))
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unsupported_segment_error_states_the_inventory_contract() {
        let error = transliterate_ipa_to_lojban("aβa").unwrap_err().to_string();
        assert!(error.contains("unsupported IPA segment `β`"));
        assert!(error.contains("Use narrow phonetic IPA."));
        assert!(
            error.contains(
                "Only the inventory listed in the gimfihi word instructions is supported."
            )
        );
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
    fn phonetic_tokenizer_accepts_every_classic_transliterator_base_symbol() {
        for (symbol, _) in IPA_SNAPS {
            jbotci_phonetic::tokenize_ipa_text(symbol).unwrap_or_else(|error| {
                panic!("phonetic scorer rejected classic IPA base `{symbol}`: {error}")
            });
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn phonetic_tokenizer_accepts_twelve_language_inventory_spot_checks() {
        for (language, transcriptions) in [
            ("cmn", &["ɕi", "tɕʰi", "ʈʂʰa"][..]),
            ("hin", &["ɖʱɑ̃l"][..]),
            ("eng", &["bɪt", "ʃʊd", "bɝd"][..]),
            ("spa", &["kaθa", "aɲo", "kaʎe"][..]),
            ("rus", &["nʲet"][..]),
            ("ara", &["sˤaːd", "ħaːl"][..]),
            ("fra", &["ɥi", "bɔ̃"][..]),
            ("ben", &["bʱɑ̃lo"][..]),
            ("por", &["pɐ̃w̃"][..]),
            ("msa", &["ʔəmas"][..]),
            ("jpn", &["sɯ̟ɕiː"][..]),
            ("deu", &["pfaɪ̯f", "ʃøːn"][..]),
        ] {
            for transcription in transcriptions {
                jbotci_phonetic::tokenize_ipa_text(transcription).unwrap_or_else(|error| {
                    panic!(
                        "phonetic scorer rejected {language} transcription `{transcription}`: {error}"
                    )
                });
            }
        }
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
