//! Lojban morphology model.

mod cmavo;
mod grammar;
mod lujvo;
mod segment;
mod syntax_eq;
pub mod tree;

use std::{fmt, sync::Arc};

use bityzba::{data, ensures, invariant, new, requires, try_new};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticNoteMode, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticStyledNote, DiagnosticTextRole, DiagnosticTextSegment, TraceOptions, TraceReport,
    source_span_from_char_offsets,
};
pub use jbotci_dialect::{
    CmavoDialectEntry, CmavoDialectEntryData, DialectDefinition, DialectFeature,
};
use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vec1::Vec1;

pub use cmavo::{Cmavo, Selmaho};
pub use lujvo::{
    LujvoBuildMode, LujvoCandidate, bond_rafsis, can_appear_as_final_lujvo_rafsi,
    choose_best_lujvo_candidate, ends_with_consonant, ends_with_vowel, ensure_cmevla_word,
    is_bonding_hyphen, is_cmevla, is_consonant, is_valid_lujvo_candidate_word, is_vowel,
    permissible_consonant_pair, syllables_pattern,
};
pub use syntax_eq::{strip_diacritics, word_like_syntax_eq, word_syntax_eq};
pub use tree::{
    AtomRef, LujvoPart, NodeRef, TreeNode, Verbatim, VerbatimData, Word, WordData, WordLike,
    WordLikeData,
};

pub const MORPHOLOGY_TRACE_FILTERS: &[&str] = &[
    "morphology",
    "segment",
    "digit sequence",
    "LOhU quote",
    "ZOI quote",
    "single-word quote",
    "ZO quote",
    "FAhO",
    "BU attachment",
    "SI erasure",
    "SA erasure",
    "SU erasure",
    "ZEI lujvo",
    "word",
    "CMAVO",
    "CMAVO prefix",
    "GISMU",
    "LUJVO",
    "FUHIVLA",
    "CMEVLA",
];

#[invariant(self.cmavo_dialect_entries.iter().all(CmavoDialectEntry::is_valid), "cmavo dialect entries must be normalized and internally valid")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MorphologyOptions {
    pub accept_latin: bool,
    pub accept_cyrillic: bool,
    pub accept_zbalermorna: bool,
    pub cmavo_dialect_entries: Vec<CmavoDialectEntry>,
    pub cmevla_as_relation_words: bool,
    pub uppercase_marks_stress: bool,
    #[serde(default)]
    pub trace: TraceOptions,
}

impl Default for MorphologyOptions {
    #[requires(true)]
    #[ensures(true)]
    fn default() -> Self {
        new!(MorphologyOptions {
            accept_latin: true,
            accept_cyrillic: true,
            accept_zbalermorna: true,
            cmavo_dialect_entries: Vec::new(),
            cmevla_as_relation_words: false,
            uppercase_marks_stress: true,
            trace: TraceOptions::disabled(),
        })
    }
}

impl MorphologyOptions {
    #[ensures(ret.cmavo_dialect_entries == definition.cmavo_entries)]
    #[ensures(definition.features.contains(&DialectFeature::Cbm) -> ret.cmevla_as_relation_words)]
    #[ensures(definition.features.contains(&DialectFeature::CaseInsensitive) -> !ret.uppercase_marks_stress)]
    #[requires(true)]
    pub fn with_dialect_definition(self, definition: &DialectDefinition) -> Self {
        let cmevla_as_relation_words = self.cmevla_as_relation_words;
        let uppercase_marks_stress = self.uppercase_marks_stress;
        self.with_data(data! {
            cmavo_dialect_entries: definition.cmavo_entries.clone(),
            cmevla_as_relation_words: cmevla_as_relation_words
                || definition.features.contains(&DialectFeature::Cbm),
            uppercase_marks_stress: uppercase_marks_stress
                && !definition.features.contains(&DialectFeature::CaseInsensitive),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_trace_options(self, trace: TraceOptions) -> Self {
        self.with_data(data! { trace: trace })
    }
}

#[invariant(warnings.iter().all(|warning| warning.char_start < warning.char_end))]
#[derive(Debug, Clone)]
pub struct MorphologySegmentAttempt {
    pub result: Result<Vec<WordLike>, MorphologyError>,
    pub warnings: Vec<MorphologyWarning>,
    pub trace: Option<TraceReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WordKind {
    #[serde(rename = "cmavo")]
    Cmavo,
    #[serde(rename = "gismu")]
    Gismu,
    #[serde(rename = "lujvo")]
    Lujvo,
    #[serde(rename = "fu'ivla")]
    Fuhivla,
    #[serde(rename = "cmevla")]
    Cmevla,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StressMark {
    None,
    Acute,
    Caps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlideMark {
    None,
    Breve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct PhonemeRenderOptions {
    pub mark_stress: StressMark,
    pub mark_glides: GlideMark,
}

impl Default for PhonemeRenderOptions {
    #[requires(true)]
    #[ensures(ret.mark_stress == StressMark::Acute)]
    #[ensures(ret.mark_glides == GlideMark::Breve)]
    fn default() -> Self {
        Self {
            mark_stress: StressMark::Acute,
            mark_glides: GlideMark::Breve,
        }
    }
}

#[invariant(!text.is_empty(), "phoneme text must not be empty")]
#[invariant(text.chars().all(is_valid_phoneme), "phonemes must use canonical Lojban phoneme characters")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Phonemes {
    text: String,
}

impl Phonemes {
    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn from_canonical(text: String) -> Result<Self, String> {
        try_new!(Phonemes { text: text }).map_err(|error| error.to_string())
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn into_string(self) -> String {
        self.into_data().text
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn render(&self, options: PhonemeRenderOptions) -> String {
        self.text
            .chars()
            .map(|ch| render_phoneme_char(ch, options))
            .collect()
    }
}

#[requires(true)]
#[ensures(true)]
fn render_phoneme_char(ch: char, options: PhonemeRenderOptions) -> char {
    match ch {
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' => render_stressed_vowel(ch, options.mark_stress),
        'ĭ' | 'ŭ' => render_glide(ch, options.mark_glides),
        other => other,
    }
}

#[requires(matches!(ch, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý'))]
#[ensures(true)]
fn render_stressed_vowel(ch: char, mark: StressMark) -> char {
    match mark {
        StressMark::Acute => ch,
        StressMark::None => unstressed_vowel(ch),
        StressMark::Caps => unstressed_vowel(ch).to_ascii_uppercase(),
    }
}

#[requires(matches!(ch, 'ĭ' | 'ŭ'))]
#[ensures(true)]
fn render_glide(ch: char, mark: GlideMark) -> char {
    match (ch, mark) {
        ('ĭ', GlideMark::Breve) => 'ĭ',
        ('ŭ', GlideMark::Breve) => 'ŭ',
        ('ĭ', GlideMark::None) => 'i',
        ('ŭ', GlideMark::None) => 'u',
        _ => ch,
    }
}

#[requires(matches!(ch, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý'))]
#[ensures(matches!(ret, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
fn unstressed_vowel(ch: char) -> char {
    match ch {
        'á' => 'a',
        'é' => 'e',
        'í' => 'i',
        'ó' => 'o',
        'ú' => 'u',
        'ý' => 'y',
        _ => ch,
    }
}

impl fmt::Display for WordKind {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Cmavo => "cmavo",
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuhivla => "fu'ivla",
            Self::Cmevla => "cmevla",
        };
        f.write_str(text)
    }
}

impl fmt::Display for Word {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind(), self.phonemes().as_str())
    }
}

impl Word {
    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(ret.kind() == kind)]
    pub fn from_kind(kind: WordKind, phonemes: Phonemes, span: SourceSpan) -> Self {
        match kind {
            WordKind::Cmavo => new!(Word::Cmavo {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Gismu => new!(Word::Gismu {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Lujvo => new!(Word::Lujvo {
                parts: Vec1::new(LujvoPart::rafsi(phonemes)),
                span: Arc::new(span),
            }),
            WordKind::Fuhivla => new!(Word::Fuhivla {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Cmevla => new!(Word::Cmevla {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
        }
    }

    #[requires(!parts.is_empty())]
    #[ensures(ret.kind() == WordKind::Lujvo)]
    pub fn lujvo(parts: Vec1<LujvoPart>, span: SourceSpan) -> Self {
        new!(Word::Lujvo {
            parts: parts,
            span: Arc::new(span),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> WordKind {
        match self.as_data() {
            data!(Word::Cmavo { .. }) => WordKind::Cmavo,
            data!(Word::Gismu { .. }) => WordKind::Gismu,
            data!(Word::Lujvo { .. }) => WordKind::Lujvo,
            data!(Word::Fuhivla { .. }) => WordKind::Fuhivla,
            data!(Word::Cmevla { .. }) => WordKind::Cmevla,
        }
    }

    #[requires(true)]
    #[ensures(!ret.as_str().is_empty())]
    pub fn phonemes(&self) -> Phonemes {
        match self.as_data() {
            data!(Word::Cmavo { phonemes, .. })
            | data!(Word::Gismu { phonemes, .. })
            | data!(Word::Fuhivla { phonemes, .. })
            | data!(Word::Cmevla { phonemes, .. }) => phonemes.clone(),
            data!(Word::Lujvo { parts, .. }) => Phonemes::from_canonical(
                parts
                    .iter()
                    .map(LujvoPart::phonemes)
                    .map(Phonemes::as_str)
                    .collect::<String>(),
            )
            .expect("lujvo parts are valid phoneme text"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn phonemes_ref(&self) -> Option<&Phonemes> {
        match self.as_data() {
            data!(Word::Cmavo { phonemes, .. })
            | data!(Word::Gismu { phonemes, .. })
            | data!(Word::Fuhivla { phonemes, .. })
            | data!(Word::Cmevla { phonemes, .. }) => Some(phonemes),
            data!(Word::Lujvo { .. }) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn lujvo_parts(&self) -> Option<&Vec1<LujvoPart>> {
        match self.as_data() {
            data!(Word::Lujvo { parts, .. }) => Some(parts),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.char_start <= ret.char_end)]
    pub fn span(&self) -> &SourceSpan {
        match self.as_data() {
            data!(Word::Cmavo { span, .. })
            | data!(Word::Gismu { span, .. })
            | data!(Word::Lujvo { span, .. })
            | data!(Word::Fuhivla { span, .. })
            | data!(Word::Cmevla { span, .. }) => span.as_ref(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn canonical_phonemes(&self) -> String {
        canonicalize_text(self.phonemes().as_str())
    }

    #[requires(true)]
    #[ensures(ret == (self.kind() == WordKind::Cmavo))]
    pub fn is_cmavo_word(&self) -> bool {
        self.kind() == WordKind::Cmavo
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.kind(), WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla))]
    pub fn is_brivla(&self) -> bool {
        matches!(
            self.kind(),
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
        )
    }

    #[requires(true)]
    #[ensures(ret == (self.kind() == WordKind::Cmevla))]
    pub fn is_cmevla(&self) -> bool {
        self.kind() == WordKind::Cmevla
    }

    #[requires(true)]
    #[ensures(ret.is_some() -> self.kind() == WordKind::Cmavo)]
    pub fn cmavo(&self) -> Option<Cmavo> {
        if self.is_cmavo_word() {
            Cmavo::from_text(self.phonemes().as_str())
        } else {
            None
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn is_cmavo_text(&self, text: &str) -> bool {
        self.is_cmavo_word() && canonical_text_eq(self.phonemes().as_str(), text)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn selmaho(&self) -> Option<&'static str> {
        if self.is_cmavo_word() {
            selmaho(self.phonemes().as_str())
        } else {
            None
        }
    }
}

impl LujvoPart {
    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(true)]
    pub fn rafsi(phonemes: Phonemes) -> Self {
        LujvoPart::Rafsi(phonemes)
    }

    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(true)]
    pub fn hyphen(phonemes: Phonemes) -> Self {
        LujvoPart::Hyphen(phonemes)
    }

    #[requires(true)]
    #[ensures(!ret.as_str().is_empty())]
    pub fn phonemes(&self) -> &Phonemes {
        match self {
            LujvoPart::Rafsi(phonemes) | LujvoPart::Hyphen(phonemes) => phonemes,
        }
    }
}

impl Verbatim {
    #[requires(span.char_len() == text.chars().count())]
    #[ensures(true)]
    pub fn new(span: SourceSpan, text: String) -> Self {
        new!(Verbatim {
            span: Arc::new(span),
            text: text,
        })
    }
}

impl WordLike {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word: Word) -> Self {
        new!(WordLike::PlainWord(word))
    }

    #[requires(zo.is_cmavo(Cmavo::Zo))]
    #[ensures(true)]
    pub fn zo_quote(zo: Word, word: Word) -> Self {
        new!(WordLike::QuotedWord {
            zo: Box::new(zo),
            word: Box::new(word),
        })
    }

    #[requires(zoi.is_selmaho(Selmaho::Zoi))]
    #[requires(canonical_text_eq(
        opening_delimiter.phonemes().as_str(),
        closing_delimiter.phonemes().as_str(),
    ))]
    #[requires(opening_delimiter.span().byte_end <= quoted_text.span.byte_start)]
    #[requires(quoted_text.span.byte_end <= closing_delimiter.span().byte_start)]
    #[ensures(true)]
    pub fn zoi_quote(
        zoi: Word,
        opening_delimiter: Word,
        quoted_text: Verbatim,
        closing_delimiter: Word,
    ) -> Self {
        new!(WordLike::DelimitedNonLojbanQuote {
            zoi: Box::new(zoi),
            opening_delimiter: Box::new(opening_delimiter),
            quoted_text: Box::new(quoted_text),
            closing_delimiter: Box::new(closing_delimiter),
        })
    }

    #[requires(lohu.is_cmavo(Cmavo::Lohu))]
    #[requires(lehu.is_cmavo(Cmavo::Lehu))]
    #[ensures(true)]
    pub fn lohu_quote(lohu: Word, quoted_words: Vec<Word>, lehu: Word) -> Self {
        new!(WordLike::QuotedWords {
            lohu: Box::new(lohu),
            quoted_words: quoted_words,
            lehu: Box::new(lehu),
        })
    }

    #[requires(is_single_word_quote_marker(&marker))]
    #[ensures(true)]
    pub fn single_word_quote(marker: Word, quoted_text: Verbatim) -> Self {
        new!(WordLike::DelimitedWordQuote {
            marker: Box::new(marker),
            quoted_text: Box::new(quoted_text),
        })
    }

    #[requires(bu.is_cmavo(Cmavo::Bu))]
    #[ensures(true)]
    pub fn letter(base: WordLike, bu: Word) -> Self {
        new!(WordLike::LerfuWord {
            base: Box::new(base),
            bu: Box::new(bu),
        })
    }

    #[requires(zei.is_cmavo(Cmavo::Zei))]
    #[ensures(true)]
    pub fn zei_lujvo(left: WordLike, zei: Word, right: Word) -> Self {
        new!(WordLike::ZeiCompound {
            left: Box::new(left),
            zei: Box::new(zei),
            right: Box::new(right),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(WordLike::PlainWord(_))))]
    pub fn bare_word(&self) -> Option<&Word> {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(WordLike::QuotedWord { .. }) | data!(WordLike::DelimitedNonLojbanQuote { .. }) | data!(WordLike::QuotedWords { .. }) | data!(WordLike::DelimitedWordQuote { .. })))]
    pub fn quote_marker_cmavo(&self) -> Option<Cmavo> {
        match self.as_data() {
            data!(WordLike::QuotedWord { zo, .. }) => zo.cmavo(),
            data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) => zoi.cmavo(),
            data!(WordLike::QuotedWords { lohu, .. }) => lohu.cmavo(),
            data!(WordLike::DelimitedWordQuote { marker, .. }) => marker.cmavo(),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.quote_marker_cmavo() == Some(cmavo)))]
    pub fn is_quote_marker_cmavo(&self, cmavo: Cmavo) -> bool {
        self.quote_marker_cmavo() == Some(cmavo)
    }

    #[requires(true)]
    #[ensures(ret.is_some() == self.bare_word().is_some_and(|word| word.cmavo().is_some()))]
    pub fn cmavo(&self) -> Option<Cmavo> {
        self.bare_word().and_then(Word::cmavo)
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_brivla()))]
    pub fn is_brivla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_brivla())
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_cmevla()))]
    pub fn is_cmevla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_cmevla())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
    pub fn byte_range(&self) -> Option<std::ops::Range<usize>> {
        word_like_byte_range(self)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&SourceSpan> {
        let mut spans = Vec::new();
        self.source_spans_into(&mut spans);
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a SourceSpan>) {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => out.push(word.span()),
            data!(WordLike::QuotedWord { zo, word }) => {
                out.push(zo.span());
                out.push(word.span());
            }
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => {
                out.push(zoi.span());
                out.push(opening_delimiter.span());
                out.push(quoted_text.span.as_ref());
                out.push(closing_delimiter.span());
            }
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => {
                out.push(lohu.span());
                for word in quoted_words {
                    out.push(word.span());
                }
                out.push(lehu.span());
            }
            data!(WordLike::DelimitedWordQuote {
                marker,
                quoted_text,
            }) => {
                out.push(marker.span());
                out.push(quoted_text.span.as_ref());
            }
            data!(WordLike::LerfuWord { base, bu }) => {
                base.source_spans_into(out);
                out.push(bu.span());
            }
            data!(WordLike::ZeiCompound { left, zei, right }) => {
                left.source_spans_into(out);
                out.push(zei.span());
                out.push(right.span());
            }
        }
    }
}

impl<'de> Deserialize<'de> for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        word_like_from_json(serde_json::Value::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => write!(f, "{word}"),
            data!(WordLike::QuotedWord { zo, word }) => write!(f, "{zo}-<<{word}>>"),
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => write!(
                f,
                "{zoi}-{opening_delimiter}-{:?}-{closing_delimiter}",
                quoted_text.text
            ),
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => {
                write!(f, "{lohu}-<<")?;
                for (index, word) in quoted_words.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{word}")?;
                }
                write!(f, ">>-{lehu}")
            }
            data!(WordLike::DelimitedWordQuote {
                marker,
                quoted_text,
            }) => write!(f, "{marker}-{text:?}", text = quoted_text.text),
            data!(WordLike::LerfuWord { base, bu }) => write!(f, "{base}-{bu}"),
            data!(WordLike::ZeiCompound { left, zei, right }) => {
                write!(f, "{left}-{zei}-{right}")
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_like_spans<F>(word_like: WordLike, map_span: &F) -> Result<WordLike, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    Ok(match word_like.into_data() {
        data!(WordLike::PlainWord(word)) => WordLike::bare(map_word_spans(word, map_span)?),
        data!(WordLike::QuotedWord { zo, word }) => WordLike::zo_quote(
            map_word_spans(*zo, map_span)?,
            map_word_spans(*word, map_span)?,
        ),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => WordLike::zoi_quote(
            map_word_spans(*zoi, map_span)?,
            map_word_spans(*opening_delimiter, map_span)?,
            map_verbatim_span(*quoted_text, map_span)?,
            map_word_spans(*closing_delimiter, map_span)?,
        ),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => WordLike::lohu_quote(
            map_word_spans(*lohu, map_span)?,
            quoted_words
                .into_iter()
                .map(|word| map_word_spans(word, map_span))
                .collect::<Result<Vec<_>, _>>()?,
            map_word_spans(*lehu, map_span)?,
        ),
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => WordLike::single_word_quote(
            map_word_spans(*marker, map_span)?,
            map_verbatim_span(*quoted_text, map_span)?,
        ),
        data!(WordLike::LerfuWord { base, bu }) => WordLike::letter(
            map_word_like_spans(*base, map_span)?,
            map_word_spans(*bu, map_span)?,
        ),
        data!(WordLike::ZeiCompound { left, zei, right }) => WordLike::zei_lujvo(
            map_word_like_spans(*left, map_span)?,
            map_word_spans(*zei, map_span)?,
            map_word_spans(*right, map_span)?,
        ),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_spans<F>(word: Word, map_span: &F) -> Result<Word, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    Ok(match word.into_data() {
        data!(Word::Cmavo { phonemes, span }) => new!(Word::Cmavo {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Gismu { phonemes, span }) => new!(Word::Gismu {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Lujvo { parts, span }) => new!(Word::Lujvo {
            parts: parts,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Fuhivla { phonemes, span }) => new!(Word::Fuhivla {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Cmevla { phonemes, span }) => new!(Word::Cmevla {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_verbatim_span<F>(verbatim: Verbatim, map_span: &F) -> Result<Verbatim, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    let data = verbatim.into_data();
    Ok(Verbatim::new(map_span((*data.span).clone())?, data.text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyErrorKind {
    InvalidCharacter,
    ExpectedWord,
    UnrecognizedWord,
    InvalidApostrophe,
    GeminatedConsonant,
    VoicingMismatch,
    ForbiddenConsonantPair,
    ForbiddenConsonantTriple,
    VowelHiatus,
    YHiatus,
    BreveNotGlide,
    DigitApostrophe,
    DigitVowel,
    Slinkuhi,
    InvalidLujvo,
    InvalidQuoteMarker,
    InvalidZoiDelimiter,
}

impl MorphologyErrorKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "morphology.invalid-character",
            Self::ExpectedWord => "morphology.expected-word",
            Self::UnrecognizedWord => "morphology.unrecognized-word",
            Self::InvalidApostrophe => "morphology.invalid-apostrophe",
            Self::GeminatedConsonant => "morphology.geminated-consonant",
            Self::VoicingMismatch => "morphology.voicing-mismatch",
            Self::ForbiddenConsonantPair => "morphology.forbidden-consonant-pair",
            Self::ForbiddenConsonantTriple => "morphology.forbidden-consonant-triple",
            Self::VowelHiatus => "morphology.vowel-hiatus",
            Self::YHiatus => "morphology.y-hiatus",
            Self::BreveNotGlide => "morphology.breve-not-glide",
            Self::DigitApostrophe => "morphology.digit-apostrophe",
            Self::DigitVowel => "morphology.digit-vowel",
            Self::Slinkuhi => "morphology.slinkuhi",
            Self::InvalidLujvo => "morphology.invalid-lujvo",
            Self::InvalidQuoteMarker => "morphology.invalid-quote-marker",
            Self::InvalidZoiDelimiter => "morphology.invalid-zoi-delimiter",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "invalid character in Lojban word",
            Self::ExpectedWord => "expected Lojban word",
            Self::UnrecognizedWord => "word is not a valid Lojban word",
            Self::InvalidApostrophe => "apostrophe is only allowed between vowels",
            Self::GeminatedConsonant => "geminated consonants are not allowed",
            Self::VoicingMismatch => "adjacent consonants must agree in voicing",
            Self::ForbiddenConsonantPair => "forbidden consonant pair",
            Self::ForbiddenConsonantTriple => "forbidden consonant triple",
            Self::VowelHiatus => "vowels in hiatus are not allowed",
            Self::YHiatus => "y cannot be followed by a non-y vowel nucleus",
            Self::BreveNotGlide => "breve-marked vowel is not in a glide position",
            Self::DigitApostrophe => "digit cannot be followed by apostrophe",
            Self::DigitVowel => "digit cannot be followed by a vowel",
            Self::Slinkuhi => "slinku'i form is not a valid word",
            Self::InvalidLujvo => "invalid lujvo decomposition",
            Self::InvalidQuoteMarker => "quote marker must be a single word",
            Self::InvalidZoiDelimiter => "ZOI delimiter must be a single non-y word",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyWarningKind {
    ExperimentalCgv,
    ExperimentalMz,
}

impl MorphologyWarningKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn code(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => "morphology.warning.experimental-cgv",
            Self::ExperimentalMz => "morphology.warning.experimental-mz",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => "experimental morphology: consonant-glide-vowel sequence",
            Self::ExperimentalMz => "experimental morphology: MZ consonant pair",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => {
                "consonant-glide-vowel sequence accepted as experimental morphology"
            }
            Self::ExperimentalMz => "MZ consonant pair accepted as experimental morphology",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn detail_reason(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => {
                "accepted by the experimental consonant-glide-vowel relaxation"
            }
            Self::ExperimentalMz => "accepted by the experimental MZ consonant-pair relaxation",
        }
    }
}

#[invariant(self.char_start < self.char_end, "morphology warnings must cover a non-empty span")]
#[invariant(!self.text.is_empty(), "morphology warnings must preserve offending source text")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologyWarning {
    pub kind: MorphologyWarningKind,
    pub char_start: usize,
    pub char_end: usize,
    pub text: String,
    pub context: Option<MorphologyContext>,
}

impl MorphologyWarning {
    #[requires(char_start < char_end)]
    #[requires(!text.is_empty())]
    #[ensures(ret.kind == kind)]
    #[ensures(ret.char_start == char_start)]
    #[ensures(ret.char_end == char_end)]
    pub fn new(
        kind: MorphologyWarningKind,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<MorphologyContext>,
    ) -> Self {
        new!(MorphologyWarning {
            kind: kind,
            char_start: char_start,
            char_end: char_end,
            text: text,
            context: context,
        })
    }

    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        morphology_diagnostic(
            source_id,
            source,
            new!(MorphologyDiagnosticDetails {
                severity: DiagnosticSeverity::Warning,
                code: self.kind.code(),
                message: self.kind.message(),
            }),
            self.char_start,
            self.char_end,
            self.kind.label(),
            self.context.as_ref(),
        )
        .with_styled_notes(vec![morphology_detail_note(
            self.kind.message(),
            &self.text,
            self.kind.detail_reason(),
        )])
    }
}

impl fmt::Display for MorphologyErrorKind {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyContextKind {
    Cmavo,
    Gismu,
    Lujvo,
    Fuhivla,
    Cmevla,
    QuotedWord,
    DelimitedNonLojbanQuote,
    QuotedWords,
    DelimitedWordQuote,
    Bu,
    Zei,
}

impl MorphologyContextKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cmavo => "while parsing cmavo",
            Self::Gismu => "while parsing gismu",
            Self::Lujvo => "while parsing lujvo",
            Self::Fuhivla => "while parsing fu'ivla",
            Self::Cmevla => "while parsing cmevla",
            Self::QuotedWord => "while parsing ZO quote",
            Self::DelimitedNonLojbanQuote => "while parsing ZOI quote",
            Self::QuotedWords => "while parsing LOhU quote",
            Self::DelimitedWordQuote => "while parsing single-word quote",
            Self::Bu => "while applying BU",
            Self::Zei => "while applying ZEI",
        }
    }
}

#[invariant(self.char_start < self.char_end, "morphology context labels must cover a non-empty span")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologyContext {
    pub kind: MorphologyContextKind,
    pub char_start: usize,
    pub char_end: usize,
}

impl MorphologyContext {
    #[requires(char_start < char_end)]
    #[ensures(ret.char_start == char_start)]
    #[ensures(ret.char_end == char_end)]
    pub fn new(kind: MorphologyContextKind, char_start: usize, char_end: usize) -> Self {
        new!(MorphologyContext {
            kind: kind,
            char_start: char_start,
            char_end: char_end,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(&self) -> &'static str {
        self.kind.label()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum LujvoParseExpectation {
    InitialOrStandaloneFinalRafsi,
    FinalOrInitialRafsi,
}

impl LujvoParseExpectation {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::InitialOrStandaloneFinalRafsi => "an initial rafsi or a standalone final rafsi",
            Self::FinalOrInitialRafsi => "a final rafsi or another initial rafsi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum ExpectedWordDetailKind {
    PlainWord,
    QuoteTarget,
    BuOperand,
    ZeiOperand,
    ZoiDelimiter,
}

impl ExpectedWordDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::PlainWord => "the parser reached a point where a Lojban word is required",
            Self::QuoteTarget => "ZO requires one following non-y word to quote",
            Self::BuOperand => "BU must attach to a preceding word",
            Self::ZeiOperand => "ZEI must have a word on both sides",
            Self::ZoiDelimiter => "ZOI requires an opening delimiter word after the quote marker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum ZoiDelimiterDetailKind {
    Missing,
    YWord,
    NotSingleWord,
}

impl ZoiDelimiterDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::Missing => "ZOI requires an opening delimiter word after the quote marker",
            Self::YWord => "y is grammar noise, so it cannot delimit a ZOI quote",
            Self::NotSingleWord => "a ZOI delimiter must be exactly one bare word",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum PhonotacticDetailKind {
    InvalidCharacter,
    InvalidApostrophe,
    GeminatedConsonant,
    VoicingMismatch,
    ForbiddenConsonantPair,
    ForbiddenConsonantTriple,
    VowelHiatus,
    YHiatus,
    BreveNotGlide,
    DigitApostrophe,
    DigitVowel,
}

impl PhonotacticDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "this character is not part of Lojban morphology",
            Self::InvalidApostrophe => "apostrophe can only separate two vowel nuclei",
            Self::GeminatedConsonant => "the same consonant appears twice in a row",
            Self::VoicingMismatch => "this consonant pair mixes voiced and unvoiced consonants",
            Self::ForbiddenConsonantPair => "this consonant pair is not a permissible Lojban pair",
            Self::ForbiddenConsonantTriple => {
                "this consonant triple does not contain a permissible adjacent pair"
            }
            Self::VowelHiatus => "these adjacent vowel nuclei need a separating apostrophe",
            Self::YHiatus => "y cannot be immediately followed by another vowel nucleus",
            Self::BreveNotGlide => "a breve-marked vowel must be part of a glide",
            Self::DigitApostrophe => "digit lerfu cannot be followed directly by apostrophe",
            Self::DigitVowel => "digit lerfu cannot be followed directly by a vowel nucleus",
        }
    }
}

#[invariant(true)]
#[invariant(::InvalidLujvo => parsed_prefix.as_ref().is_none_or(|prefix| !prefix.is_empty()))]
#[invariant(::ExpectedWord => matches!(expected,
    ExpectedWordDetailKind::PlainWord
        | ExpectedWordDetailKind::QuoteTarget
        | ExpectedWordDetailKind::BuOperand
        | ExpectedWordDetailKind::ZeiOperand
        | ExpectedWordDetailKind::ZoiDelimiter))]
#[invariant(::InvalidZoiDelimiter => matches!(reason,
    ZoiDelimiterDetailKind::Missing
        | ZoiDelimiterDetailKind::YWord
        | ZoiDelimiterDetailKind::NotSingleWord))]
#[invariant(::Phonotactic => matches!(reason,
    PhonotacticDetailKind::InvalidCharacter
        | PhonotacticDetailKind::InvalidApostrophe
        | PhonotacticDetailKind::GeminatedConsonant
        | PhonotacticDetailKind::VoicingMismatch
        | PhonotacticDetailKind::ForbiddenConsonantPair
        | PhonotacticDetailKind::ForbiddenConsonantTriple
        | PhonotacticDetailKind::VowelHiatus
        | PhonotacticDetailKind::YHiatus
        | PhonotacticDetailKind::BreveNotGlide
        | PhonotacticDetailKind::DigitApostrophe
        | PhonotacticDetailKind::DigitVowel))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphologyErrorDetail {
    InvalidLujvo {
        parsed_prefix: Option<String>,
        expected: LujvoParseExpectation,
    },
    FuhivlaContainsY,
    Slinkuhi,
    ExpectedWord {
        expected: ExpectedWordDetailKind,
    },
    InvalidZoiDelimiter {
        reason: ZoiDelimiterDetailKind,
    },
    Phonotactic {
        reason: PhonotacticDetailKind,
    },
}

impl MorphologyErrorDetail {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn construct(&self) -> &'static str {
        match self.as_data() {
            data!(MorphologyErrorDetail::InvalidLujvo { .. }) => "invalid lujvo",
            data!(MorphologyErrorDetail::FuhivlaContainsY) => "fu'ivla",
            data!(MorphologyErrorDetail::Slinkuhi) => "slinku'i",
            data!(MorphologyErrorDetail::ExpectedWord { .. }) => "expected word",
            data!(MorphologyErrorDetail::InvalidZoiDelimiter { .. }) => "ZOI delimiter",
            data!(MorphologyErrorDetail::Phonotactic { .. }) => "phonotactics",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn reason(&self) -> String {
        match self.as_data() {
            data!(MorphologyErrorDetail::InvalidLujvo {
                parsed_prefix,
                expected,
            }) => parsed_prefix.as_ref().map_or_else(
                || {
                    format!(
                        "the lujvo parser expected {} at the start",
                        expected.description()
                    )
                },
                |prefix| {
                    format!(
                        "after parsing `{prefix}`, the lujvo parser expected {} at the next source position",
                        expected.description()
                    )
                },
            ),
            data!(MorphologyErrorDetail::FuhivlaContainsY) => {
                "fu'ivla syllables cannot use y as a vowel nucleus".to_owned()
            }
            data!(MorphologyErrorDetail::Slinkuhi) => {
                "adding a leading consonant before a lujvo-shaped form would break word resolution"
                    .to_owned()
            }
            data!(MorphologyErrorDetail::ExpectedWord { expected }) => {
                expected.description().to_owned()
            }
            data!(MorphologyErrorDetail::InvalidZoiDelimiter { reason }) => {
                reason.description().to_owned()
            }
            data!(MorphologyErrorDetail::Phonotactic { reason }) => {
                reason.description().to_owned()
            }
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Invalid => true)]
#[invariant(::UnterminatedZoiQuote => true)]
#[invariant(::SourceSpan(_) => true)]
pub enum MorphologyError {
    #[error("{kind} at character {char_start}: `{text}`")]
    Invalid {
        kind: MorphologyErrorKind,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<MorphologyContext>,
        detail: Option<MorphologyErrorDetail>,
    },
    #[error("unterminated ZOI quote, expected closing delimiter `{delimiter}`")]
    UnterminatedZoiQuote {
        char_offset: usize,
        delimiter: String,
        context: Option<MorphologyContext>,
    },
    #[error("invalid source span: {0}")]
    SourceSpan(#[from] SourceLocationError),
}

impl MorphologyError {
    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        match self {
            Self::Invalid {
                kind,
                char_start,
                char_end,
                text,
                context,
                detail,
            } => {
                let diagnostic = morphology_diagnostic(
                    source_id.clone(),
                    source,
                    new!(MorphologyDiagnosticDetails {
                        severity: DiagnosticSeverity::Error,
                        code: kind.code(),
                        message: kind.message(),
                    }),
                    *char_start,
                    *char_end,
                    kind.message(),
                    context.as_ref(),
                );
                diagnostic_with_optional_detail(diagnostic, text, detail.as_ref())
            }
            Self::UnterminatedZoiQuote {
                char_offset,
                delimiter,
                context,
            } => {
                let source_end = source.chars().count();
                morphology_diagnostic(
                    source_id.clone(),
                    source,
                    new!(MorphologyDiagnosticDetails {
                        severity: DiagnosticSeverity::Error,
                        code: "morphology.unterminated-zoi-quote",
                        message: "unterminated ZOI quote",
                    }),
                    *char_offset,
                    source_end,
                    &format!("expected closing delimiter `{delimiter}`"),
                    context.as_ref(),
                )
                .with_styled_notes(vec![morphology_detail_note(
                    "unterminated ZOI quote",
                    delimiter,
                    "expected closing delimiter",
                )])
            }
            Self::SourceSpan(error) => {
                let span = source_span_from_char_offsets(source_id, source, 0, 0)
                    .expect("the start of a source string is always a valid source span");
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticPhase::Morphology,
                    "morphology.source-span".to_owned(),
                    "invalid source span".to_owned(),
                    vec![DiagnosticLabel::new(span, error.to_string(), true)],
                    Vec::new(),
                    None,
                )
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.code.is_empty())]
fn diagnostic_with_optional_detail(
    diagnostic: Diagnostic,
    text: &str,
    detail: Option<&MorphologyErrorDetail>,
) -> Diagnostic {
    let Some(detail) = detail else {
        return diagnostic;
    };
    let reason = detail.reason();
    diagnostic.with_styled_notes(vec![morphology_detail_note(
        detail.construct(),
        text,
        &reason,
    )])
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn phonotactic_error_detail(kind: MorphologyErrorKind) -> Option<MorphologyErrorDetail> {
    let reason = match kind {
        MorphologyErrorKind::InvalidCharacter => PhonotacticDetailKind::InvalidCharacter,
        MorphologyErrorKind::InvalidApostrophe => PhonotacticDetailKind::InvalidApostrophe,
        MorphologyErrorKind::GeminatedConsonant => PhonotacticDetailKind::GeminatedConsonant,
        MorphologyErrorKind::VoicingMismatch => PhonotacticDetailKind::VoicingMismatch,
        MorphologyErrorKind::ForbiddenConsonantPair => {
            PhonotacticDetailKind::ForbiddenConsonantPair
        }
        MorphologyErrorKind::ForbiddenConsonantTriple => {
            PhonotacticDetailKind::ForbiddenConsonantTriple
        }
        MorphologyErrorKind::VowelHiatus => PhonotacticDetailKind::VowelHiatus,
        MorphologyErrorKind::YHiatus => PhonotacticDetailKind::YHiatus,
        MorphologyErrorKind::BreveNotGlide => PhonotacticDetailKind::BreveNotGlide,
        MorphologyErrorKind::DigitApostrophe => PhonotacticDetailKind::DigitApostrophe,
        MorphologyErrorKind::DigitVowel => PhonotacticDetailKind::DigitVowel,
        _ => return None,
    };
    Some(new!(MorphologyErrorDetail::Phonotactic { reason }))
}

#[requires(!message.is_empty())]
#[requires(!reason.is_empty())]
#[ensures(!ret.segments.is_empty())]
fn morphology_detail_note(message: &str, text: &str, reason: &str) -> DiagnosticStyledNote {
    let display_text = if text.is_empty() { "input" } else { text };
    DiagnosticStyledNote::new(
        DiagnosticNoteMode::Detailed,
        vec![
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, "morphology detail: ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, message.to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, display_text.to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "reason".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ": ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, reason.to_owned()),
        ],
    )
}

#[invariant(!self.code.is_empty())]
#[invariant(!self.message.is_empty())]
struct MorphologyDiagnosticDetails {
    severity: DiagnosticSeverity,
    code: &'static str,
    message: &'static str,
}

#[requires(!label.is_empty())]
#[requires(char_start <= char_end)]
#[ensures(!ret.code.is_empty())]
fn morphology_diagnostic(
    source_id: Option<SourceId>,
    source: &str,
    details: MorphologyDiagnosticDetails,
    char_start: usize,
    char_end: usize,
    label: &str,
    context: Option<&MorphologyContext>,
) -> Diagnostic {
    let span = source_span_from_char_offsets(source_id.clone(), source, char_start, char_end)
        .expect("morphology errors store offsets derived from the same source text");
    let mut labels = vec![DiagnosticLabel::new(span, label.to_owned(), true)];
    if let Some(context_label) = context.and_then(|context| {
        source_span_from_char_offsets(
            source_id.clone(),
            source,
            context.char_start,
            context.char_end,
        )
        .ok()
        .map(|span| DiagnosticLabel::new(span, context.label().to_owned(), false))
    }) {
        labels.push(context_label);
    }
    Diagnostic::new(
        details.severity,
        DiagnosticPhase::Morphology,
        details.code.to_owned(),
        details.message.to_owned(),
        labels,
        Vec::new(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id_attempt(input, options, source_id)
        .into_data()
        .result
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> MorphologySegmentAttempt {
    let attempt = grammar::segment_words_with_modifiers_attempt(input, options, source_id);
    let data = attempt.into_data();
    let result = data
        .result
        .map(|words| apply_cmavo_dialect_entries(words, &options.cmavo_dialect_entries));
    new!(MorphologySegmentAttempt {
        result,
        warnings: data.warnings,
        trace: data.trace,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    grammar::segment_words_with_modifiers_raw_attempt(input, options, source_id)
        .into_data()
        .result
        .map(|words| apply_cmavo_dialect_entries(words, &options.cmavo_dialect_entries))
}

#[requires(!phonemes.as_str().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|syllables| !syllables.is_empty() && syllables.iter().all(|syllable| !syllable.is_empty())) || ret.as_ref().err().is_some_and(|message| !message.is_empty()))]
pub fn pronunciation_syllables(phonemes: &Phonemes) -> Result<Vec<String>, String> {
    segment::pronunciation_syllable_texts(phonemes.as_str())
        .ok_or_else(|| format!("could not syllabify `{}`", phonemes.as_str()))
}

#[requires(entries.iter().all(CmavoDialectEntry::is_valid))]
#[ensures(true)]
fn apply_cmavo_dialect_entries(
    mut words: Vec<WordLike>,
    entries: &[CmavoDialectEntry],
) -> Vec<WordLike> {
    for entry in entries {
        words = apply_cmavo_dialect_entry(words, entry);
    }
    words
}

#[requires(entry.is_valid())]
#[ensures(true)]
fn apply_cmavo_dialect_entry(words: Vec<WordLike>, entry: &CmavoDialectEntry) -> Vec<WordLike> {
    words
        .into_iter()
        .flat_map(|word_like| apply_cmavo_dialect_entry_to_word_like(word_like, entry))
        .collect()
}

#[requires(entry.is_valid())]
#[ensures(!ret.is_empty())]
fn apply_cmavo_dialect_entry_to_word_like(
    word_like: WordLike,
    entry: &CmavoDialectEntry,
) -> Vec<WordLike> {
    let data!(WordLike::PlainWord(word)) = word_like.as_data() else {
        return vec![word_like];
    };
    let Some(replacement) = cmavo_dialect_replacement(word, entry) else {
        return vec![word_like];
    };
    replacement
}

#[requires(entry.is_valid())]
#[ensures(ret.as_ref().is_none_or(|words| !words.is_empty()))]
fn cmavo_dialect_replacement(word: &Word, entry: &CmavoDialectEntry) -> Option<Vec<WordLike>> {
    if word.kind() != WordKind::Cmavo {
        return None;
    }
    let replacement = match entry.as_data() {
        data!(CmavoDialectEntry::Swap { left, right })
            if cmavo_dialect_entry_matches(word, left) =>
        {
            vec![right]
        }
        data!(CmavoDialectEntry::Swap { left, right })
            if cmavo_dialect_entry_matches(word, right) =>
        {
            vec![left]
        }
        data!(CmavoDialectEntry::Expansion {
            source,
            replacement,
        }) if cmavo_dialect_entry_matches(word, source) => replacement.iter().collect(),
        _ => return None,
    };
    Some(
        replacement
            .into_iter()
            .map(|phonemes| replacement_cmavo(phonemes, word.span()))
            .collect(),
    )
}

#[requires(!candidate.is_empty())]
#[ensures(true)]
fn cmavo_dialect_entry_matches(word: &Word, candidate: &str) -> bool {
    canonical_text_eq(word.phonemes().as_str(), candidate)
}

#[requires(!phonemes.is_empty())]
#[ensures(matches!(ret.as_data(), data!(WordLike::PlainWord(word)) if word.kind() == WordKind::Cmavo))]
fn replacement_cmavo(phonemes: &str, span: &SourceSpan) -> WordLike {
    let normalized =
        segment::parse_cmavo_form(phonemes).unwrap_or_else(|| canonicalize_text(phonemes));
    WordLike::bare(Word::from_kind(
        WordKind::Cmavo,
        Phonemes::from_canonical(normalized).expect("dialect cmavo entry is normalized"),
        span.clone(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn is_single_word_quote_marker(word: &Word) -> bool {
    word.is_one_of_cmavo(&[
        Cmavo::Zohoi,
        Cmavo::Lahoi,
        Cmavo::Rahoi,
        Cmavo::Mehoi,
        Cmavo::Gohoi,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_from_json(value: serde_json::Value) -> Result<WordLike, String> {
    let mut object = json_object(value)?;
    if let Some(kind) = object.remove("kind") {
        let kind = json_string(kind)?;
        return match kind.as_str() {
            "bare" => Ok(WordLike::bare(word_field(&mut object, "word")?)),
            "zo-quote" => Ok(WordLike::zo_quote(
                word_field(&mut object, "zo")?,
                word_field(&mut object, "word")?,
            )),
            "zoi-quote" => Ok(WordLike::zoi_quote(
                word_field(&mut object, "zoi")?,
                word_field(&mut object, "opening_delimiter")?,
                verbatim_field(&mut object, "quoted_text")?,
                word_field(&mut object, "closing_delimiter")?,
            )),
            "lohu-quote" => Ok(WordLike::lohu_quote(
                word_field(&mut object, "lohu")?,
                words_field(&mut object, "quoted_words")?,
                word_field(&mut object, "lehu")?,
            )),
            "single-word-quote" => Ok(WordLike::single_word_quote(
                word_field(&mut object, "marker")?,
                verbatim_field(&mut object, "quoted_text")?,
            )),
            "letter" => Ok(WordLike::letter(
                word_like_field(&mut object, "base")?,
                word_field(&mut object, "bu")?,
            )),
            "zei-lujvo" => Ok(WordLike::zei_lujvo(
                word_like_field(&mut object, "left")?,
                word_field(&mut object, "zei")?,
                word_field(&mut object, "right")?,
            )),
            other => Err(format!("unknown word-like kind `{other}`")),
        };
    }
    let (constructor, payload) = single_constructor(object)?;
    let mut payload = json_object(payload)?;
    match constructor.as_str() {
        "Bare" | "PlainWord" => Ok(WordLike::bare(word_payload(payload)?)),
        "QuotedWord" => Ok(WordLike::zo_quote(
            word_field(&mut payload, "zo")?,
            word_field(&mut payload, "word")?,
        )),
        "DelimitedNonLojbanQuote" => Ok(WordLike::zoi_quote(
            word_field(&mut payload, "zoi")?,
            word_field(&mut payload, "opening_delimiter")?,
            verbatim_field(&mut payload, "quoted_text")?,
            word_field(&mut payload, "closing_delimiter")?,
        )),
        "QuotedWords" => Ok(WordLike::lohu_quote(
            word_field(&mut payload, "lohu")?,
            words_field(&mut payload, "quoted_words")?,
            word_field(&mut payload, "lehu")?,
        )),
        "DelimitedWordQuote" => Ok(WordLike::single_word_quote(
            word_field(&mut payload, "marker")?,
            verbatim_field(&mut payload, "quoted_text")?,
        )),
        "Letter" => Ok(WordLike::letter(
            word_like_field(&mut payload, "base")?,
            word_field(&mut payload, "bu")?,
        )),
        "ZeiCompound" => Ok(WordLike::zei_lujvo(
            word_like_field(&mut payload, "left")?,
            word_field(&mut payload, "zei")?,
            word_field(&mut payload, "right")?,
        )),
        other => Err(format!("unknown word-like constructor `{other}`")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Word, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid word field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_payload(mut object: serde_json::Map<String, serde_json::Value>) -> Result<Word, String> {
    if object.contains_key("word") {
        return word_field(&mut object, "word");
    }
    serde_json::from_value(serde_json::Value::Object(object))
        .map_err(|error| format!("invalid word payload: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn words_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Vec<Word>, String> {
    let Some(value) = object.remove(name) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value)
        .map_err(|error| format!("invalid word list field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn source_span_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<SourceSpan, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid source span field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn verbatim_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Verbatim, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid verbatim field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<WordLike, String> {
    word_like_from_json(required_field(object, name)?)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn required_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<serde_json::Value, String> {
    object
        .remove(name)
        .ok_or_else(|| format!("missing field `{name}`"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_object(
    value: serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    match value {
        serde_json::Value::Object(object) => Ok(object),
        other => Err(format!("expected object, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_string(value: serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::String(text) => Ok(text),
        other => Err(format!("expected string, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn single_constructor(
    object: serde_json::Map<String, serde_json::Value>,
) -> Result<(String, serde_json::Value), String> {
    if object.len() != 1 {
        return Err(format!(
            "expected single constructor key, got {}",
            object.len()
        ));
    }
    Ok(object.into_iter().next().expect("object has one item"))
}

#[requires(true)]
#[ensures(true)]
pub fn is_valid_phoneme(value: char) -> bool {
    matches!(
        value,
        'a' | 'á'
            | 'e'
            | 'é'
            | 'i'
            | 'í'
            | 'ĭ'
            | 'o'
            | 'ó'
            | 'u'
            | 'ú'
            | 'ŭ'
            | 'y'
            | 'ý'
            | '\''
            | ','
            | '0'..='9'
    ) || matches!(
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

#[ensures(!ret.is_empty() || text.is_empty())]
#[requires(true)]
pub fn canonicalize_text(text: &str) -> String {
    text.chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn canonical_text_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .eq(right
            .chars()
            .filter(|value| *value != ',')
            .flat_map(strip_diacritic)
            .flat_map(char::to_lowercase))
}

#[requires(true)]
#[ensures(ret -> !text.is_empty())]
pub fn canonical_text_is_all(text: &str, expected: char) -> bool {
    let mut saw_char = false;
    for value in text
        .chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
    {
        if value != expected {
            return false;
        }
        saw_char = true;
    }
    saw_char
}

#[requires(true)]
#[ensures(true)]
pub fn strip_diacritics_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter_map(strip_diacritic)
        .eq(right.chars().filter_map(strip_diacritic))
}

#[requires(true)]
#[ensures(true)]
fn strip_diacritic(value: char) -> Option<char> {
    Some(match value {
        'á' | 'à' | 'Á' | 'À' => 'a',
        'é' | 'è' | 'É' | 'È' => 'e',
        'í' | 'ì' | 'ĭ' | 'Ĭ' | 'Í' | 'Ì' => 'i',
        'ó' | 'ò' | 'Ó' | 'Ò' => 'o',
        'ú' | 'ù' | 'ŭ' | 'Ŭ' | 'Ú' | 'Ù' => 'u',
        'ý' | 'ỳ' | 'Ý' | 'Ỳ' => 'y',
        '\u{0301}' | '\u{0300}' | '\u{0306}' => return None,
        other => other,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn selmaho(cmavo: &str) -> Option<&'static str> {
    match canonicalize_text(cmavo).as_str() {
        "a" | "e" | "ji" | "o" | "u" => Some("A"),
        "ba" | "ca" | "pu" => Some("PU"),
        "ba'e" | "za'e" => Some("BAhE"),
        "be" => Some("BE"),
        "bei" => Some("BEI"),
        "be'o" => Some("BEhO"),
        "by" | "cy" | "dy" | "fy" | "gy" | "jy" | "ky" | "ly" | "my" | "ny" | "py" | "ry"
        | "sy" | "ty" | "vy" | "xy" | "y'y" | "zy" => Some("BY"),
        "cai" | "cu'i" | "pei" | "ru'e" | "sai" => Some("CAI"),
        "ce'e" => Some("CEhE"),
        "co" => Some("CO"),
        "coi" | "co'o" | "je'e" | "ju'i" | "mi'e" | "mu'o" | "ta'a" => Some("COI"),
        "cu" => Some("CU"),
        "da" | "de" | "di" | "do" | "fo'a" | "fo'e" | "fo'i" | "fo'o" | "fo'u" | "ko" | "ko'a"
        | "ko'e" | "ko'i" | "ko'o" | "ko'u" | "ma" | "mi" | "ra" | "ri" | "ru" | "ta" | "ti"
        | "tu" | "vo'a" | "vo'e" | "vo'i" | "vo'o" | "vo'u" | "zo'e" | "zu'i" => Some("KOhA"),
        "fa" | "fai" | "fe" | "fi" | "fi'a" | "fo" | "fu" => Some("FA"),
        "fa'o" => Some("FAhO"),
        "fu'a" => Some("FUhA"),
        "ge'a" | "pa'i" | "re'a" | "sa'i" | "sa'o" | "su'i" | "te'a" | "va'a" | "vu'u" | "cu'a"
        | "de'o" | "fe'a" | "fe'i" | "fu'u" | "ju'u" | "ne'o" | "pi'a" | "pi'i" | "ri'o" => {
            Some("VUhU")
        }
        "goi" | "ne" | "no'u" | "pe" | "po" | "po'e" | "po'u" => Some("GOI"),
        "i" => Some("I"),
        "ja" | "je" | "jo" | "ju" => Some("JA"),
        "jei" | "ka" | "li'i" | "mu'e" | "ni" | "nu" | "pu'u" | "si'o" | "su'u" | "za'i"
        | "zu'o" => Some("NU"),
        "jo'i" => Some("JOhI"),
        "joi" | "ce" | "ce'o" | "fa'u" | "jo'e" | "jo'u" | "ju'e" | "ku'a" | "pi'u" => Some("JOI"),
        "la" | "lai" | "la'i" => Some("LA"),
        "le" | "lei" | "le'e" | "le'i" | "lo" | "loi" | "lo'e" | "lo'i" => Some("LE"),
        "li" | "me'o" => Some("LI"),
        "la'e" | "lu'a" | "lu'e" | "lu'i" | "lu'o" | "tu'a" | "vu'i" => Some("LAhE"),
        "li'u" => Some("LIhU"),
        "lo'o" => Some("LOhO"),
        "ni'o" => Some("NIhO"),
        "lu" => Some("LU"),
        "tu'e" => Some("TUhE"),
        "to" => Some("TO"),
        "toi" => Some("TOI"),
        "zo" | "ma'oi" => Some("ZO"),
        "zoi" | "la'o" | "mu'oi" => Some("ZOI"),
        "lo'u" => Some("LOhU"),
        "le'u" => Some("LEhU"),
        "bu" => Some("BU"),
        "zei" => Some("ZEI"),
        "na" | "ja'a" => Some("NA"),
        "nai" => Some("NAI"),
        "na'e" | "je'a" | "no'e" | "to'e" => Some("NAhE"),
        "noi" | "poi" | "voi" => Some("NOI"),
        "pa" | "re" | "ci" | "vo" | "mu" | "xa" | "ze" | "bi" | "so" | "no" | "pi" => Some("PA"),
        "pe'e" => Some("PEhE"),
        "sa" => Some("SA"),
        "se" | "te" | "ve" | "xe" => Some("SE"),
        "si" => Some("SI"),
        "su" => Some("SU"),
        "va" | "vi" | "vu" => Some("VA"),
        "vau" => Some("VAU"),
        "vei" => Some("VEI"),
        "ve'o" => Some("VEhO"),
        "xi" => Some("XI"),
        "y" => Some("Y"),
        "za" | "zi" | "zu" => Some("ZI"),
        "zo'u" => Some("ZOhU"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn erasure_selmaho(word_like: &WordLike) -> Option<&'static str> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word.selmaho(),
        data!(WordLike::QuotedWord { .. }) => Some("ZO"),
        data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) => zoi.selmaho(),
        data!(WordLike::QuotedWords { .. }) => Some("LOhU"),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => marker.selmaho(),
        data!(WordLike::LerfuWord { .. }) => Some("BU"),
        data!(WordLike::ZeiCompound { .. }) => Some("ZEI"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<std::ops::Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => Some(word.span().byte_start..word.span().byte_end),
        data!(WordLike::QuotedWord { zo, word }) => {
            Some(zo.span().byte_start..word.span().byte_end)
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span().byte_start..closing_delimiter.span().byte_end),
        data!(WordLike::QuotedWords { lohu, lehu, .. }) => {
            Some(lohu.span().byte_start..lehu.span().byte_end)
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text
        }) => Some(marker.span().byte_start..quoted_text.span.byte_end),
        data!(WordLike::LerfuWord { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span().byte_end.max(range.end))
        }
        data!(WordLike::ZeiCompound { left, right, .. }) => word_like_byte_range(left)
            .map(|range| range.start..right.span().byte_end.max(range.end)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_is_enabled_by_default_with_warning() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "la siatl.",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("CgV relaxation should permit cmevla");

        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("sĭatl"));
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(
            data.warnings[0].kind,
            MorphologyWarningKind::ExperimentalCgv
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mz_relaxation_is_enabled_by_default_with_warning() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "la djeimz.",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("MZ relaxation should permit cmevla");

        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("djeĭmz"));
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(data.warnings[0].kind, MorphologyWarningKind::ExperimentalMz);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_error_kind_codes_are_stable() {
        let cases = [
            (
                MorphologyErrorKind::InvalidCharacter,
                "morphology.invalid-character",
                "invalid character in Lojban word",
            ),
            (
                MorphologyErrorKind::ExpectedWord,
                "morphology.expected-word",
                "expected Lojban word",
            ),
            (
                MorphologyErrorKind::UnrecognizedWord,
                "morphology.unrecognized-word",
                "word is not a valid Lojban word",
            ),
            (
                MorphologyErrorKind::InvalidApostrophe,
                "morphology.invalid-apostrophe",
                "apostrophe is only allowed between vowels",
            ),
            (
                MorphologyErrorKind::GeminatedConsonant,
                "morphology.geminated-consonant",
                "geminated consonants are not allowed",
            ),
            (
                MorphologyErrorKind::VoicingMismatch,
                "morphology.voicing-mismatch",
                "adjacent consonants must agree in voicing",
            ),
            (
                MorphologyErrorKind::ForbiddenConsonantPair,
                "morphology.forbidden-consonant-pair",
                "forbidden consonant pair",
            ),
            (
                MorphologyErrorKind::ForbiddenConsonantTriple,
                "morphology.forbidden-consonant-triple",
                "forbidden consonant triple",
            ),
            (
                MorphologyErrorKind::VowelHiatus,
                "morphology.vowel-hiatus",
                "vowels in hiatus are not allowed",
            ),
            (
                MorphologyErrorKind::YHiatus,
                "morphology.y-hiatus",
                "y cannot be followed by a non-y vowel nucleus",
            ),
            (
                MorphologyErrorKind::BreveNotGlide,
                "morphology.breve-not-glide",
                "breve-marked vowel is not in a glide position",
            ),
            (
                MorphologyErrorKind::DigitApostrophe,
                "morphology.digit-apostrophe",
                "digit cannot be followed by apostrophe",
            ),
            (
                MorphologyErrorKind::DigitVowel,
                "morphology.digit-vowel",
                "digit cannot be followed by a vowel",
            ),
            (
                MorphologyErrorKind::Slinkuhi,
                "morphology.slinkuhi",
                "slinku'i form is not a valid word",
            ),
            (
                MorphologyErrorKind::InvalidLujvo,
                "morphology.invalid-lujvo",
                "invalid lujvo decomposition",
            ),
            (
                MorphologyErrorKind::InvalidQuoteMarker,
                "morphology.invalid-quote-marker",
                "quote marker must be a single word",
            ),
            (
                MorphologyErrorKind::InvalidZoiDelimiter,
                "morphology.invalid-zoi-delimiter",
                "ZOI delimiter must be a single non-y word",
            ),
        ];

        for (kind, code, message) in cases {
            assert_eq!(kind.code(), code);
            assert_eq!(kind.message(), message);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_kind_codes_are_stable() {
        let cases = [
            (
                MorphologyWarningKind::ExperimentalCgv,
                "morphology.warning.experimental-cgv",
                "experimental morphology: consonant-glide-vowel sequence",
            ),
            (
                MorphologyWarningKind::ExperimentalMz,
                "morphology.warning.experimental-mz",
                "experimental morphology: MZ consonant pair",
            ),
        ];

        for (kind, code, message) in cases {
            assert_eq!(kind.code(), code);
            assert_eq!(kind.message(), message);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_diagnostic_uses_precise_vowel_hiatus_span() {
        let error = segment_words_with_modifiers("aa").expect_err("vowel hiatus must fail");
        let diagnostic = error.to_diagnostic(None, "aa");

        assert_eq!(diagnostic.code, "morphology.vowel-hiatus");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.byte_start, 0);
        assert_eq!(label.span.byte_end, 2);
        assert_eq!(label.span.char_start, 0);
        assert_eq!(label.span.char_end, 2);
        assert_eq!(label.message, "vowels in hiatus are not allowed");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_diagnostic_maps_non_ascii_source_span() {
        let source = "éa";
        let error = segment_words_with_modifiers(source).expect_err("vowel hiatus must fail");
        let diagnostic = error.to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.vowel-hiatus");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.byte_start, 0);
        assert_eq!(label.span.byte_end, 3);
        assert_eq!(label.span.char_start, 0);
        assert_eq!(label.span.char_end, 2);
        assert!(diagnostic.styled_notes.iter().any(|note| {
            note.segments
                .iter()
                .any(|segment| segment.text.contains("adjacent vowel nuclei"))
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_diagnostic_maps_comma_crossing_cgv_span() {
        let source = "melxi,or.";
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        data.result.expect("CgV relaxation should parse");
        assert_eq!(data.warnings.len(), 1);
        let diagnostic = data.warnings[0].to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.warning.experimental-cgv");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.char_start, 3);
        assert_eq!(label.span.char_end, 7);
        assert_eq!(&source[label.span.byte_start..label.span.byte_end], "xi,o");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_diagnostic_maps_comma_crossing_mz_span() {
        let source = "nam,zi";
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        data.result.expect("MZ relaxation should parse");
        assert_eq!(data.warnings.len(), 1);
        let diagnostic = data.warnings[0].to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.warning.experimental-mz");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.char_start, 2);
        assert_eq!(label.span.char_end, 5);
        assert_eq!(&source[label.span.byte_start..label.span.byte_end], "m,z");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_simple_cmavo_and_gismu() {
        let words = segment_words_with_modifiers("mi klama do").expect("valid morphology");
        assert_eq!(words.len(), 3);
        assert_eq!(base_word(&words[0]).map(Word::kind), Some(WordKind::Cmavo));
        assert_eq!(base_phonemes(&words[0]).as_deref(), Some("mi"));
        assert_eq!(base_word(&words[1]).map(Word::kind), Some(WordKind::Gismu));
        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("kláma"));
        assert_eq!(base_word(&words[2]).map(Word::kind), Some(WordKind::Cmavo));
        assert_eq!(
            base_word(&words[2]).map(|word| word.span().char_start),
            Some(9)
        );
        assert_eq!(
            base_word(&words[2]).map(|word| word.span().char_end),
            Some(11)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn splits_adjacent_cmavo() {
        let words = segment_words_with_modifiers("mimi").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["mi".to_owned(), "mi".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn marks_cmavo_glides() {
        let words = segment_words_with_modifiers_raw("coi .ui").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["coĭ".to_owned(), "ŭi".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn phonemes_render_stress_and_glides() {
        let phonemes = Phonemes::from_canonical("bródacoĭ".to_owned()).expect("valid phonemes");
        assert_eq!(phonemes.render(PhonemeRenderOptions::default()), "bródacoĭ");
        assert_eq!(
            phonemes.render(PhonemeRenderOptions {
                mark_stress: StressMark::None,
                mark_glides: GlideMark::None,
            }),
            "brodacoi"
        );
        assert_eq!(
            phonemes.render(PhonemeRenderOptions {
                mark_stress: StressMark::Caps,
                mark_glides: GlideMark::Breve,
            }),
            "brOdacoĭ"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cbm_dialect_to_morphology_options() {
        let dialect = jbotci_dialect::parse_dialect_definition("(cbm)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("mi .alis. do sa broda", &options)
            .expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["mi".to_owned(), "bróda".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_case_insensitive_dialect_to_morphology_options() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(case-insensitive)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("NALSELTRO", &options)
            .expect("valid morphology");
        assert_eq!(base_phonemes(&words[0]).as_deref(), Some("nalséltro"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_combined_dialect_formula_to_morphology_options() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(case-insensitive)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("la ITALIAS.", &options)
            .expect("valid morphology");
        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("italĭas"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cmavo_dialect_swaps_in_order() {
        let dialect = jbotci_dialect::parse_dialect_definition("((ce'u <-> ce) (ce'u <-> ki))")
            .expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words =
            segment_words_with_modifiers_with_options("ce", &options).expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["ki"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cmavo_dialect_expansions() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("((la'u -> la'e di'u))").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words =
            segment_words_with_modifiers_with_options("la'u", &options).expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["la'e", "di'u"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_builtin_cmavo_dialects() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(jboponei ce-ki-tau)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words = segment_words_with_modifiers_with_options("po nei ce ki tau su'o", &options)
            .expect("valid morphology");

        assert_eq!(
            base_phoneme_texts(&words),
            vec!["lo", "su'u", "keĭ", "ce'u", "ke'a", "tu'a", "su"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_v0_lujvo_examples() {
        let cases: &[(&str, &[&str])] = &[
            ("gerzda", &["rafsi:ger", "rafsi:zda"]),
            ("sutkla", &["rafsi:sut", "rafsi:kla"]),
            ("ge'urzdani", &["rafsi:ge'ur", "rafsi:zdani"]),
            ("ba'irgau", &["rafsi:ba'ir", "rafsi:gaŭ"]),
            ("so'irdja", &["rafsi:so'ir", "rafsi:dja"]),
            ("bajyzda", &["rafsi:baj", "hyphen:y", "rafsi:zda"]),
            ("kamykla", &["rafsi:kam", "hyphen:y", "rafsi:kla"]),
            ("papykla", &["rafsi:pap", "hyphen:y", "rafsi:kla"]),
            ("selpa'i", &["rafsi:sel", "rafsi:pa'i"]),
            ("tolsi'arai", &["rafsi:tol", "rafsi:si'a", "rafsi:raĭ"]),
            (
                "jboplijvogau",
                &["rafsi:jbo", "rafsi:pli", "rafsi:jvo", "rafsi:gaŭ"],
            ),
            ("baibra", &["rafsi:baĭ", "rafsi:bra"]),
            ("xlagau", &["rafsi:xla", "rafsi:gaŭ"]),
        ];

        for (source, expected) in cases {
            assert_eq!(lujvo_part_labels(source), *expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmavo_do_not_have_lujvo_parts() {
        for source in ["mi", "do", "lo"] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let word = base_word(&words[0]).expect("base word");
            assert!(word.lujvo_parts().is_none(), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_equivalence_ignores_spans_and_diacritics_on_words() {
        let mut left = segment_words_with_modifiers("coi").expect("valid morphology");
        let mut right = segment_words_with_modifiers("coi").expect("valid morphology");
        let word = match right[0].as_data() {
            data!(WordLike::PlainWord(word)) => word.clone(),
            _ => panic!("expected bare word"),
        };
        right[0] = WordLike::bare(Word::from_kind(
            word.kind(),
            Phonemes::from_canonical("coĭ".to_owned()).expect("valid phonemes"),
            SourceSpan::new(None, 99, 102, 99, 102).expect("valid span"),
        ));

        assert!(word_like_syntax_eq(&left.remove(0), &right.remove(0)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_morphology_options_are_rejected() {
        let panic = std::panic::catch_unwind(|| {
            let _ = MorphologyOptions::default().with_data(data! {
                cmavo_dialect_entries: vec![new!(CmavoDialectEntry::Expansion {
                    source: "mi".to_owned(),
                    replacement: Vec::new(),
                })],
            });
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_deserialization_rejects_invalid_words() {
        let error = serde_json::from_str::<Word>(
            r#"{
                "Cmavo": {
                    "phonemes": "",
                    "span": {
                        "source_id": null,
                        "byte_start": 0,
                        "byte_end": 0,
                        "char_start": 0,
                        "char_end": 0,
                        "start": null,
                        "end": null
                    }
                }
            }"#,
        )
        .expect_err("empty phoneme text must be rejected");

        assert!(error.to_string().contains("phoneme text must not be empty"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_deserializes_compact_constructor_json() {
        let word_like = serde_json::from_str::<WordLike>(
            r#"{
                "QuotedWord": {
                    "zo": {"Cmavo": {"phonemes": "zo", "span": {"source_id": null, "byte_start": 0, "byte_end": 2, "char_start": 0, "char_end": 2, "start": null, "end": null}}},
                    "word": {"Cmavo": {"phonemes": "coi", "span": {"source_id": null, "byte_start": 3, "byte_end": 6, "char_start": 3, "char_end": 6, "start": null, "end": null}}}
                }
            }"#,
        )
        .expect("compact constructor JSON should deserialize");

        let data!(WordLike::QuotedWord { zo, word }) = word_like.as_data() else {
            panic!("expected zo quote");
        };
        assert!(zo.is_cmavo(Cmavo::Zo));
        assert_eq!(word.phonemes().as_str(), "coi");
        assert_eq!(word.span().char_start, 3);
        assert_eq!(word.span().char_end, 6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_wrong_zo_marker() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zo_quote(
                test_word(WordKind::Cmavo, "mi", 0),
                test_word(WordKind::Cmavo, "do", 3),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_wrong_bu_marker() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::letter(
                WordLike::bare(test_word(WordKind::Cmavo, "a", 0)),
                test_word(WordKind::Cmavo, "cu", 2),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_unordered_zoi_quote_spans() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zoi_quote(
                test_word(WordKind::Cmavo, "zoi", 0),
                test_word(WordKind::Cmavo, "gy", 4),
                Verbatim::new(
                    SourceSpan::new(None, 10, 12, 10, 12).expect("valid test span"),
                    "xx".to_owned(),
                ),
                test_word(WordKind::Cmavo, "gy", 8),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_mismatched_zoi_quote_delimiters() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zoi_quote(
                test_word(WordKind::Cmavo, "zoi", 0),
                test_word(WordKind::Cmavo, "gy", 4),
                Verbatim::new(
                    SourceSpan::new(None, 7, 11, 7, 11).expect("valid test span"),
                    "test".to_owned(),
                ),
                test_word(WordKind::Cmavo, "ly", 12),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_cmavo_parsing_is_canonical_and_multi_class() {
        assert_eq!(Cmavo::from_text("NÁ'E"), Some(Cmavo::Nahe));
        assert_eq!(Cmavo::Nahe.canonical_text(), "na'e");
        assert!(Selmaho::Nahe.contains(Cmavo::Nahe));

        assert!(Selmaho::Bai.contains(Cmavo::Lahei));
        assert!(Selmaho::Le.contains(Cmavo::Lahei));
        assert!(Selmaho::Ui.contains(Cmavo::Lahei));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_cmavo_checks_are_only_for_bare_cmavo_words() {
        let cmavo = test_word(WordKind::Cmavo, "zo", 0);
        let quoted = test_word(WordKind::Cmavo, "coi", 3);
        let word_like = WordLike::zo_quote(cmavo, quoted);

        assert_eq!(word_like.bare_word(), None);
        assert_eq!(word_like.cmavo(), None);
        assert!(!word_like.is_cmavo(Cmavo::Zo));
        assert!(!word_like.is_selmaho(Selmaho::Zo));

        let bare = WordLike::bare(test_word(WordKind::Cmavo, "zo", 0));
        assert_eq!(bare.bare_word().and_then(Word::cmavo), Some(Cmavo::Zo));
        assert!(bare.is_cmavo(Cmavo::Zo));
        assert!(bare.is_selmaho(Selmaho::Zo));
        assert!(bare.is_one_of_selmaho(&[Selmaho::A, Selmaho::Zo]));

        let letter = WordLike::letter(
            WordLike::bare(test_word(WordKind::Cmavo, "zo", 0)),
            test_word(WordKind::Cmavo, "bu", 3),
        );
        assert_eq!(letter.cmavo(), None);
        assert!(!letter.is_cmavo(Cmavo::Zo));

        let zei_lujvo = WordLike::zei_lujvo(
            WordLike::bare(test_word(WordKind::Cmavo, "zo", 0)),
            test_word(WordKind::Cmavo, "zei", 3),
            test_word(WordKind::Cmavo, "coi", 7),
        );
        assert_eq!(zei_lujvo.cmavo(), None);
        assert!(!zei_lujvo.is_cmavo(Cmavo::Zo));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_marker_cmavo_checks_quote_markers_only() {
        let zo_quote = WordLike::zo_quote(
            test_word(WordKind::Cmavo, "zo", 0),
            test_word(WordKind::Cmavo, "coi", 3),
        );
        assert_eq!(zo_quote.quote_marker_cmavo(), Some(Cmavo::Zo));
        assert!(zo_quote.is_quote_marker_cmavo(Cmavo::Zo));
        assert!(!zo_quote.is_cmavo(Cmavo::Zo));

        let zoi_quote = WordLike::zoi_quote(
            test_word(WordKind::Cmavo, "zoi", 0),
            test_word(WordKind::Cmavo, "gy", 4),
            Verbatim::new(
                SourceSpan::new(None, 7, 11, 7, 11).expect("valid test span"),
                "test".to_owned(),
            ),
            test_word(WordKind::Cmavo, "gy", 12),
        );
        assert_eq!(zoi_quote.quote_marker_cmavo(), Some(Cmavo::Zoi));

        let lohu_quote = WordLike::lohu_quote(
            test_word(WordKind::Cmavo, "lo'u", 0),
            vec![test_word(WordKind::Cmavo, "coi", 5)],
            test_word(WordKind::Cmavo, "le'u", 9),
        );
        assert_eq!(lohu_quote.quote_marker_cmavo(), Some(Cmavo::Lohu));

        let single_word_quote = WordLike::single_word_quote(
            test_word(WordKind::Cmavo, "zo'oi", 0),
            Verbatim::new(
                SourceSpan::new(None, 6, 11, 6, 11).expect("valid test span"),
                "hello".to_owned(),
            ),
        );
        assert_eq!(single_word_quote.quote_marker_cmavo(), Some(Cmavo::Zohoi));

        let letter = WordLike::letter(
            WordLike::bare(test_word(WordKind::Cmavo, "a", 0)),
            test_word(WordKind::Cmavo, "bu", 2),
        );
        assert_eq!(letter.quote_marker_cmavo(), None);
    }

    #[requires(!phonemes.is_empty())]
    #[ensures(ret.kind() == kind)]
    fn test_word(kind: WordKind, phonemes: &str, byte_start: usize) -> Word {
        let byte_end = byte_start + phonemes.len();
        let char_end = byte_start + phonemes.chars().count();
        Word::from_kind(
            kind,
            Phonemes::from_canonical(phonemes.to_owned()).expect("valid test phonemes"),
            SourceSpan::new(None, byte_start, byte_end, byte_start, char_end)
                .expect("valid test span"),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn base_word(word: &WordLike) -> Option<&Word> {
        match word.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn base_phonemes(word: &WordLike) -> Option<String> {
        base_word(word).map(|word| word.phonemes().into_string())
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|text| !text.is_empty()))]
    fn base_phoneme_texts(words: &[WordLike]) -> Vec<String> {
        words
            .iter()
            .map(|word| base_phonemes(word).expect("base word"))
            .collect()
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.iter().all(|label| !label.is_empty()))]
    fn lujvo_part_labels(source: &str) -> Vec<String> {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let word = base_word(&words[0]).expect("base word");
        word.lujvo_parts()
            .expect("lujvo parts")
            .iter()
            .map(jvopau_label)
            .collect()
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn jvopau_label(part: &LujvoPart) -> String {
        match part {
            LujvoPart::Rafsi(phonemes) => format!("rafsi:{}", render_unstressed(phonemes)),
            LujvoPart::Hyphen(phonemes) => format!("hyphen:{}", render_unstressed(phonemes)),
        }
    }

    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_unstressed(phonemes: &Phonemes) -> String {
        phonemes.render(PhonemeRenderOptions {
            mark_stress: StressMark::None,
            mark_glides: GlideMark::Breve,
        })
    }
}
