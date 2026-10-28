//! Morphology parse tree model.

use std::sync::Arc;

use bityzba::{data, invariant};
use jbotci_source::SourceSpan;
use jbotci_tree::tree_model;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::{Cmavo, Phonemes, Selmaho};

tree_model! {
    #[invariant(::Cmavo => span.char_len() > 0)]
    #[invariant(::Gismu => span.char_len() > 0)]
    #[invariant(::Lujvo => span.char_len() > 0)]
    #[invariant(::Fuhivla => span.char_len() > 0)]
    #[invariant(::Cmevla => span.char_len() > 0)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Word {
        Cmavo {
            phonemes: Phonemes,
            span: Arc<SourceSpan>,
        },
        Gismu {
            phonemes: Phonemes,
            span: Arc<SourceSpan>,
        },
        Lujvo {
            #[tree_child(primary)]
            parts: Vec1<LujvoPart>,
            span: Arc<SourceSpan>,
        },
        Fuhivla {
            phonemes: Phonemes,
            span: Arc<SourceSpan>,
        },
        Cmevla {
            phonemes: Phonemes,
            span: Arc<SourceSpan>,
        },
    }

    #[invariant(::Rafsi(_) => true)]
    #[invariant(::Hyphen(_) => true)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum LujvoPart {
        Rafsi(Phonemes),
        Hyphen(Phonemes),
    }

    #[invariant(span.char_len() == text.chars().count(), "verbatim text must match span length")]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Verbatim {
        pub span: Arc<SourceSpan>,
        pub text: String,
    }

    /// One raw word, with the shared Y+BU reader exception.
    #[invariant(match word.as_data() {
        data!(WordLike::PlainWord(word)) => !word.is_selmaho(Selmaho::Lohai) && !word.is_selmaho(Selmaho::Lehai),
        data!(WordLike::LerfuWord { base, bu }) => bu.is_cmavo(Cmavo::Bu) && base.bare_word().is_some_and(|word| word.kind() == crate::WordKind::Cmavo && super::canonical_text_is_all(word.phonemes().as_str(), 'y')),
        _ => false,
    })]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ReplacementQuoteWord {
        #[tree_child(primary)]
        pub word: WordLike,
    }

    /// One run of raw words in a replacement quote.
    #[invariant(marker.is_selmaho(Selmaho::Lohai))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ReplacementQuoteRun {
        pub marker: Word,
        #[tree_child(primary)]
        pub words: Vec<ReplacementQuoteWord>,
    }

    #[invariant(::PlainWord(_) => true)]
    #[invariant(::QuotedWord => zo.is_cmavo(Cmavo::Zo))]
    #[invariant(::SelmahoQuotedWord => mahoi.is_cmavo(Cmavo::Mahoi))]
    #[invariant(::DelimitedNonLojbanQuote => zoi.is_selmaho(Selmaho::Zoi)
        && crate::canonical_text_eq(
            opening_delimiter.phonemes().as_str(),
            closing_delimiter.phonemes().as_str(),
        )
        && opening_delimiter.span().byte_end <= quoted_text.span.byte_start
        && quoted_text.span.byte_end <= closing_delimiter.span().byte_start)]
    #[invariant(::QuotedWords => lohu.is_cmavo(Cmavo::Lohu) && lehu.is_cmavo(Cmavo::Lehu))]
    #[invariant(::ReplacementQuote => runs.len() <= 2 && lehai.is_cmavo(Cmavo::Lehai)
        && runs.first().is_none_or(|run| run.marker.span().byte_end <= lehai.span().byte_start))]
    #[bityzba::expensive_invariant(::ReplacementQuote => {
        let mut end = 0;
        runs.iter().all(|run| {
            let ordered = end <= run.marker.span().byte_start;
            end = run.marker.span().byte_end;
            ordered && run.words.iter().all(|word| {
                let mut ordered = true;
                for span in word.word.source_spans() {
                    ordered &= end <= span.byte_start;
                    end = span.byte_end;
                }
                ordered
            })
        }) && end <= lehai.span().byte_start
    })]
    #[invariant(::DelimitedWordQuote => super::is_single_word_quote_marker(marker))]
    #[invariant(::LerfuWord => bu.is_cmavo(Cmavo::Bu))]
    #[invariant(::ZeiCompound => zei.is_cmavo(Cmavo::Zei))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub enum WordLike {
        PlainWord(#[tree_child(primary)] Word),
        QuotedWord {
            zo: Box<Word>,
            #[tree_child(primary)]
            word: Box<Word>,
        },
        SelmahoQuotedWord {
            mahoi: Box<Word>,
            #[tree_child(primary)]
            word: Box<Word>,
        },
        DelimitedNonLojbanQuote {
            zoi: Box<Word>,
            opening_delimiter: Box<Word>,
            quoted_text: Box<Verbatim>,
            closing_delimiter: Box<Word>,
        },
        QuotedWords {
            lohu: Box<Word>,
            #[tree_child(primary)]
            quoted_words: Vec<Word>,
            lehu: Box<Word>,
        },
        ReplacementQuote {
            #[tree_child(primary)]
            runs: Vec<ReplacementQuoteRun>,
            lehai: Box<Word>,
        },
        DelimitedWordQuote {
            marker: Box<Word>,
            #[tree_child(primary)]
            quoted_text: Box<Verbatim>,
        },
        LerfuWord {
            #[tree_child(primary)]
            base: Box<WordLike>,
            bu: Box<Word>,
        },
        ZeiCompound {
            left: Box<WordLike>,
            zei: Box<Word>,
            #[tree_child(primary)]
            right: Box<Word>,
        },
    }
}
