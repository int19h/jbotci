use std::fmt;
use std::fmt::Write as _;
use std::sync::Arc;

use bityzba::{data, invariant, new, requires};
use jbotci_dialect::{CmavoDialectEntry, CmavoDialectEntryData, DialectDefinition};
use jbotci_source::SourceSpan;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::{Phonemes, Word, WordKey, WordKind, map_word_spans};

/// The fixed maximum output from one source word after all dialect entries.
#[invariant(*words == 10)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DialectExpansionLimit {
    pub words: usize,
}

impl Default for DialectExpansionLimit {
    #[requires(true)]
    #[ensures(ret.words == 10)]
    fn default() -> Self {
        new!(DialectExpansionLimit { words: 10 })
    }
}

#[invariant(!source_word.is_empty() && !formula.is_empty())]
#[invariant(*attempted_words > limit.words)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DialectExpansionViolation {
    pub source_word: String,
    pub formula: Arc<str>,
    pub limit: DialectExpansionLimit,
    pub attempted_words: usize,
}

impl fmt::Display for DialectExpansionViolation {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dialect formula `{}` expands source word `{}` to at least {} words. The limit is {}",
            self.formula, self.source_word, self.attempted_words, self.limit.words
        )
    }
}

impl std::error::Error for DialectExpansionViolation {}

#[invariant(span.char_start < span.char_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DialectExpansionFailure {
    pub violation: DialectExpansionViolation,
    pub span: SourceSpan,
}

impl fmt::Display for DialectExpansionFailure {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.violation.fmt(f)
    }
}

impl std::error::Error for DialectExpansionFailure {
    #[requires(true)]
    #[ensures(ret.is_some())]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.violation)
    }
}

#[invariant(::InvalidWord { word } => !word.is_empty())]
#[invariant(::ExpansionLimit => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialectCompilationError {
    InvalidWord {
        word: String,
    },
    ExpansionLimit {
        violation: DialectExpansionViolation,
    },
}

impl fmt::Display for DialectCompilationError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(DialectCompilationError::ExpansionLimit { violation }) => violation.fmt(f),
            data!(DialectCompilationError::InvalidWord { word }) => {
                write!(f, "dialect word is not morphologically valid: {word}")
            }
        }
    }
}

impl std::error::Error for DialectCompilationError {}

#[invariant(
    entries.iter().all(compiled_dialect_entry_is_valid),
    "compiled dialect entries must have matching word keys"
)]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CompiledDialectDefinition {
    pub entries: Vec<CompiledDialectEntry>,
}

impl CompiledDialectDefinition {
    /// Render the resolved replacement formula for a diagnostic.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn formula(&self) -> String {
        let mut formula = String::from("(");
        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                formula.push(' ');
            }
            match entry.as_data() {
                data!(CompiledDialectEntry::Swap { left, right }) => {
                    write!(
                        formula,
                        "({} <-> {})",
                        left.key.phonemes.as_str(),
                        right.key.phonemes.as_str()
                    )
                    .unwrap();
                }
                data!(CompiledDialectEntry::Expansion {
                    source,
                    replacement
                }) => {
                    write!(formula, "({} ->", source.key.phonemes.as_str()).unwrap();
                    for word in replacement {
                        write!(formula, " {}", word.key.phonemes.as_str()).unwrap();
                    }
                    formula.push(')');
                }
            }
        }
        formula.push(')');
        formula
    }

    /// Plan with borrowed words before any replacement word is materialized.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|plan| plan.as_ref().is_none_or(|words| !words.is_empty() && words.len() <= DialectExpansionLimit::default().words)) || ret.as_ref().err().is_some_and(|count| *count > DialectExpansionLimit::default().words))]
    pub(crate) fn plan_word(
        &self,
        key: &WordKey,
    ) -> Result<Option<Vec<&CompiledDialectWord>>, usize> {
        let limit = DialectExpansionLimit::default();
        let mut plan: Option<Vec<&CompiledDialectWord>> = None;
        for entry in &self.entries {
            if let Some(words) = &plan {
                let mut count = 0;
                let mut changed = false;
                for word in words {
                    let replacement = entry.replacement_for(&word.key);
                    changed |= replacement.is_some();
                    let length = replacement.map_or(1, <[_]>::len);
                    if length > limit.words {
                        return Err(length);
                    }
                    count += length;
                }
                if !changed {
                    continue;
                }
                // Entries never delete words, so later entries cannot reduce this count.
                if count > limit.words {
                    return Err(count);
                }
                let mut next = Vec::with_capacity(count);
                for word in words {
                    if let Some(replacement) = entry.replacement_for(&word.key) {
                        next.extend(replacement);
                    } else {
                        next.push(*word);
                    }
                }
                plan = Some(next);
            } else if let Some(replacement) = entry.replacement_for(key) {
                if replacement.len() > limit.words {
                    return Err(replacement.len());
                }
                plan = Some(replacement.iter().collect());
            }
        }
        Ok(plan)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|compiled| compiled.entries.len() == definition.cmavo_entries.len()) || ret.is_err())]
    pub fn compile(definition: &DialectDefinition) -> Result<Self, DialectCompilationError> {
        let limit = DialectExpansionLimit::default();
        // The first entry is reachable from its own source word. A prefix can
        // make later entries unreachable, so their limits stay runtime checks.
        if let Some(entry) = definition.cmavo_entries.first() {
            if let data!(CmavoDialectEntry::Expansion {
                source,
                replacement
            }) = entry.as_data()
                && replacement.len() > limit.words
            {
                return Err(new!(DialectCompilationError::ExpansionLimit {
                    violation: new!(DialectExpansionViolation {
                        source_word: source.clone(),
                        formula: jbotci_dialect::dialect_definition_to_text(definition).into(),
                        limit,
                        attempted_words: replacement.len(),
                    }),
                }));
            }
        }
        let entries = definition
            .cmavo_entries
            .iter()
            .map(CompiledDialectEntry::compile)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(new!(CompiledDialectDefinition { entries: entries }))
    }
}

#[invariant(::Swap { left, right } => left.word.key() == left.key.clone() && right.word.key() == right.key.clone())]
#[invariant(::Expansion { replacement, .. } => !replacement.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CompiledDialectEntry {
    Swap {
        left: CompiledDialectWord,
        right: CompiledDialectWord,
    },
    Expansion {
        source: CompiledDialectWord,
        replacement: Vec<CompiledDialectWord>,
    },
}

impl CompiledDialectEntry {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn compile(entry: &CmavoDialectEntry) -> Result<Self, DialectCompilationError> {
        match entry.as_data() {
            data!(CmavoDialectEntry::Swap { left, right }) => {
                Ok(new!(CompiledDialectEntry::Swap {
                    left: CompiledDialectWord::parse(left)?,
                    right: CompiledDialectWord::parse(right)?,
                }))
            }
            data!(CmavoDialectEntry::Expansion {
                source,
                replacement,
            }) => Ok(new!(CompiledDialectEntry::Expansion {
                source: CompiledDialectWord::parse(source)?,
                replacement: replacement
                    .iter()
                    .map(|word| CompiledDialectWord::parse(word))
                    .collect::<Result<Vec<_>, _>>()?,
            })),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|words| !words.is_empty()))]
    pub(crate) fn replacement_for(&self, key: &WordKey) -> Option<&[CompiledDialectWord]> {
        match self.as_data() {
            data!(CompiledDialectEntry::Swap { left, right }) if &left.key == key => {
                Some(std::slice::from_ref(right))
            }
            data!(CompiledDialectEntry::Swap { left, right }) if &right.key == key => {
                Some(std::slice::from_ref(left))
            }
            data!(CompiledDialectEntry::Expansion {
                source,
                replacement,
            }) if &source.key == key => Some(replacement.as_slice()),
            _ => None,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn compiled_dialect_entry_is_valid(entry: &CompiledDialectEntry) -> bool {
    match entry.as_data() {
        data!(CompiledDialectEntry::Swap { left, right }) => {
            left.word.key() == left.key.clone() && right.word.key() == right.key.clone()
        }
        data!(CompiledDialectEntry::Expansion {
            source,
            replacement,
        }) => {
            source.word.key() == source.key.clone()
                && !replacement.is_empty()
                && replacement
                    .iter()
                    .all(|word| word.word.key() == word.key.clone())
        }
    }
}

#[invariant(word.key() == key.clone(), "compiled dialect word key must match the stored word")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CompiledDialectWord {
    pub word: Word,
    pub key: WordKey,
}

impl CompiledDialectWord {
    #[requires(!text.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|word| word.key.phonemes.as_str().len() > 0) || ret.is_err())]
    fn parse(text: &str) -> Result<Self, DialectCompilationError> {
        let normalized = crate::segment::normalize_word_checked_with_options(
            text,
            &crate::MorphologyOptions::default(),
        )
        .ok_or_else(|| {
            new!(DialectCompilationError::InvalidWord {
                word: text.to_owned()
            })
        })?;
        let (kind, phoneme_text) =
            parse_cmavo_or_classified_word(&normalized).ok_or_else(|| {
                new!(DialectCompilationError::InvalidWord {
                    word: text.to_owned()
                })
            })?;
        let phonemes = Phonemes::from_canonical(phoneme_text).map_err(|_| {
            new!(DialectCompilationError::InvalidWord {
                word: text.to_owned()
            })
        })?;
        let span = SourceSpan::new(None, 0, text.len(), 0, text.chars().count()).map_err(|_| {
            new!(DialectCompilationError::InvalidWord {
                word: text.to_owned()
            })
        })?;
        let word = compiled_word(normalized.as_str(), kind, phonemes, span, text)?;
        Ok(new!(CompiledDialectWord {
            key: word.key(),
            word,
        }))
    }

    #[requires(true)]
    #[ensures(matches!(ret.as_data(), data!(crate::WordLike::PlainWord(_))))]
    pub(crate) fn to_word_like_with_span(&self, span: &SourceSpan) -> crate::WordLike {
        crate::WordLike::bare(
            map_word_spans(self.word.clone(), &|_| Ok(span.clone()))
                .expect("replacement span is valid"),
        )
    }
}

#[requires(!normalized.is_empty())]
#[ensures(ret.as_ref().is_none_or(|(_, phonemes)| !phonemes.is_empty()))]
fn parse_cmavo_or_classified_word(normalized: &str) -> Option<(WordKind, String)> {
    crate::segment::parse_cmavo_form(normalized)
        .map(|phonemes| (WordKind::Cmavo, phonemes))
        .or_else(|| crate::segment::classify_word(normalized))
}

#[requires(!normalized.is_empty())]
#[requires(!source_text.is_empty())]
#[requires(!phonemes.as_str().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|word| word.kind() == kind) || ret.is_err())]
fn compiled_word(
    normalized: &str,
    kind: WordKind,
    phonemes: Phonemes,
    span: SourceSpan,
    source_text: &str,
) -> Result<Word, DialectCompilationError> {
    if kind == WordKind::Lujvo {
        let parts = crate::parse_lujvo_word_parts(normalized).ok_or_else(|| {
            new!(DialectCompilationError::InvalidWord {
                word: source_text.to_owned()
            })
        })?;
        let parts = Vec1::try_from_vec(parts).map_err(|_| {
            new!(DialectCompilationError::InvalidWord {
                word: source_text.to_owned()
            })
        })?;
        Ok(Word::lujvo(parts, span))
    } else {
        Ok(Word::from_kind(kind, phonemes, span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MorphologyError, MorphologyOptions, segment_words_for_display_with_options_and_source_id,
        segment_words_with_modifiers_recovered_with_options_and_source_id,
        segment_words_with_modifiers_with_options_and_source_id,
    };

    // Keep regression inputs small when the output guard is absent.
    #[requires((1..=12).contains(&rounds))]
    #[ensures(ret.matches("(mi -> mi mi)").count() == rounds)]
    fn doubling_formula(rounds: usize) -> String {
        format!("({})", "(mi -> mi mi) ".repeat(rounds).trim_end())
    }

    #[requires(true)]
    #[ensures(true)]
    fn options(formula: &str) -> MorphologyOptions {
        MorphologyOptions::default()
            .try_with_dialect_definition(
                &jbotci_dialect::parse_dialect_definition(formula).unwrap(),
            )
            .unwrap()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unreachable_oversized_entries_do_not_reject_safe_formulas() {
        let formula = format!("((mi -> do) (mi -> {}))", vec!["mi"; 11].join(" "));
        let safe = options(&formula);
        let words =
            segment_words_with_modifiers_with_options_and_source_id("mi", &safe, None).unwrap();
        assert_eq!(words.len(), 1);
        let data!(crate::WordLike::PlainWord(word)) = words[0].as_data() else {
            panic!("plain word");
        };
        assert_eq!(word.key().phonemes.as_str(), "do");
        let formula = format!("((do -> mi) (mi -> {}))", vec!["mi"; 11].join(" "));
        let options = options(&formula);
        assert!(matches!(
            segment_words_with_modifiers_with_options_and_source_id("do", &options, None),
            Err(MorphologyError::DialectExpansion(_))
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compile_rejects_oversized_entry_before_parsing_replacement_words() {
        let definition = jbotci_dialect::parse_dialect_definition(&format!(
            "((mi -> {}))",
            vec!["aaa"; 11].join(" ")
        ))
        .unwrap();
        let error = CompiledDialectDefinition::compile(&definition).unwrap_err();
        let data!(DialectCompilationError::ExpansionLimit { violation }) = error.as_data() else {
            panic!("the size guard must run before invalid replacement words are compiled");
        };
        assert_eq!(violation.source_word, "mi");
        assert_eq!(violation.attempted_words, 11);
        assert_eq!(violation.limit.words, 10);
        assert_eq!(
            violation.formula.as_ref(),
            jbotci_dialect::dialect_definition_to_text(&definition)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn doubling_chains_are_bounded_in_strict_and_display_morphology() {
        for rounds in [4, 5, 10, 12] {
            let formula = doubling_formula(rounds);
            let options = options(&formula);
            for error in [
                segment_words_with_modifiers_with_options_and_source_id(" do mi ", &options, None)
                    .unwrap_err(),
                segment_words_for_display_with_options_and_source_id(" do mi ", &options, None)
                    .unwrap_err(),
            ] {
                let MorphologyError::DialectExpansion(failure) = &error else {
                    panic!("expected structured expansion failure: {error}");
                };
                assert_eq!(failure.violation.source_word, "mi");
                assert_eq!(failure.violation.formula.as_ref(), formula);
                assert_eq!(failure.violation.attempted_words, 16);
                assert_eq!(failure.violation.limit.words, 10);
                assert_eq!((failure.span.char_start, failure.span.char_end), (4, 6));
                let diagnostic = error.to_diagnostic(None, " do mi ").unwrap();
                assert_eq!(diagnostic.code, "morphology.dialect-expansion-limit");
                assert!(diagnostic.message.contains(&formula));
                assert!(error.to_string().contains("source word `mi`"));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn limit_is_per_source_word_and_preserves_order_and_spans() {
        let chained = options("((mi -> do do do do do) (do -> mi mi))");
        let words =
            segment_words_with_modifiers_with_options_and_source_id("mi do", &chained, None)
                .unwrap();
        assert_eq!(words.len(), 12);
        for (index, word) in words.iter().enumerate() {
            let data!(crate::WordLike::PlainWord(word)) = word.as_data() else {
                panic!("plain replacement");
            };
            assert_eq!(word.key().phonemes.as_str(), "mi");
            assert_eq!(
                (word.span().char_start, word.span().char_end),
                if index < 10 { (0, 2) } else { (3, 5) }
            );
        }
        let exact = options_for_ten_words();
        assert_eq!(
            segment_words_with_modifiers_with_options_and_source_id("mi", &exact, None)
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            segment_words_with_modifiers_with_options_and_source_id(
                "mi",
                &options(&doubling_formula(3)),
                None
            )
            .unwrap()
            .len(),
            8
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn options_for_ten_words() -> MorphologyOptions {
        options(&format!(
            "((mi -> {}) (mi <-> do))",
            vec!["mi"; 10].join(" ")
        ))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_reports_expansion_without_replacement_output() {
        let options = options(&doubling_formula(12));
        let result = segment_words_with_modifiers_recovered_with_options_and_source_id(
            "mi do mi", &options, None,
        );
        assert_eq!(result.errors.len(), 2);
        let MorphologyError::DialectExpansion(first) = &result.errors[0] else {
            panic!("expansion failure");
        };
        assert!(result.errors.iter().all(|error| matches!(error,
            MorphologyError::DialectExpansion(failure) if Arc::ptr_eq(&first.violation.formula, &failure.violation.formula)
        )));
        let serialized = serde_json::to_value(first).unwrap();
        assert_eq!(serialized["violation"]["source-word"], "mi");
        assert_eq!(serialized["violation"]["limit"]["words"], 10);

        assert!(
            result
                .errors
                .iter()
                .all(|error| matches!(error, MorphologyError::DialectExpansion(_)))
        );
        assert_eq!(result.words.len(), 1);
        assert_eq!(result.words[0].byte_range(), Some(3..5));
        assert_eq!(
            result
                .error_regions
                .iter()
                .map(|span| (span.char_start, span.char_end))
                .collect::<Vec<_>>(),
            [(0, 2), (6, 8)]
        );
        let capped = MorphologyOptions {
            max_recovery_errors: std::num::NonZeroUsize::new(1).unwrap(),
            ..options
        };
        let capped = segment_words_with_modifiers_recovered_with_options_and_source_id(
            "mi do mi", &capped, None,
        );
        assert_eq!(capped.errors.len(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quoted_and_erased_words_keep_their_existing_dialect_behavior() {
        let options = options(&doubling_formula(12));
        assert_eq!(
            segment_words_with_modifiers_with_options_and_source_id("zo mi", &options, None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            segment_words_with_modifiers_with_options_and_source_id("mi si do", &options, None)
                .unwrap()
                .len(),
            1
        );
    }
}
