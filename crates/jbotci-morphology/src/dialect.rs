use std::fmt;

use bityzba::{data, invariant, new, requires};
use jbotci_dialect::{CmavoDialectEntry, CmavoDialectEntryData, DialectDefinition};
use jbotci_source::SourceSpan;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::{Phonemes, Word, WordKey, WordKind, map_word_spans};

#[invariant(::InvalidWord { word } => !word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialectCompilationError {
    InvalidWord { word: String },
}

impl fmt::Display for DialectCompilationError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
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
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|compiled| compiled.entries.len() == definition.cmavo_entries.len()) || ret.is_err())]
    pub fn compile(definition: &DialectDefinition) -> Result<Self, DialectCompilationError> {
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
    pub(crate) fn replacement_for(&self, key: &WordKey) -> Option<Vec<CompiledDialectWord>> {
        match self.as_data() {
            data!(CompiledDialectEntry::Swap { left, right }) if &left.key == key => {
                Some(vec![right.clone()])
            }
            data!(CompiledDialectEntry::Swap { left, right }) if &right.key == key => {
                Some(vec![left.clone()])
            }
            data!(CompiledDialectEntry::Expansion {
                source,
                replacement,
            }) if &source.key == key => Some(replacement.clone()),
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
