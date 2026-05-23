//! Morphology parse tree model.

use bityzba::invariant;
use jbotci_source::SourceSpan;
use jbotci_tree::tree_model;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::Phonemes;

tree_model! {
    #[invariant(::Cmavo => !phonemes.as_str().is_empty())]
    #[invariant(::Gismu => !phonemes.as_str().is_empty())]
    #[invariant(::Lujvo => !parts.is_empty())]
    #[invariant(::Fuhivla => !phonemes.as_str().is_empty())]
    #[invariant(::Cmevla => !phonemes.as_str().is_empty())]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Word {
        Cmavo {
            phonemes: Phonemes,
            span: SourceSpan,
        },
        Gismu {
            phonemes: Phonemes,
            span: SourceSpan,
        },
        Lujvo {
            #[tree_child(primary)]
            parts: Vec1<Jvopau>,
            span: SourceSpan,
        },
        Fuhivla {
            phonemes: Phonemes,
            span: SourceSpan,
        },
        Cmevla {
            phonemes: Phonemes,
            span: SourceSpan,
        },
    }

    #[invariant(true)]
    #[invariant(::Rafsi(_) => true)]
    #[invariant(::Hyphen(_) => true)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Jvopau {
        Rafsi(Phonemes),
        Hyphen(Phonemes),
    }

    #[invariant(self.span.char_len() == self.text.chars().count(), "verbatim text must match span length")]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Verbatim {
        pub span: SourceSpan,
        pub text: String,
    }

    #[invariant(::Bare(_) => true)]
    #[invariant(::ZoQuote => zo.is_cmavo_text("zo"))]
    #[invariant(::ZoiQuote => zoi.selmaho() == Some("ZOI")
        && opening_delimiter.span().byte_end <= quoted_text.span.byte_start
        && quoted_text.span.byte_end <= closing_delimiter.span().byte_start)]
    #[invariant(::LohuQuote => lohu.is_cmavo_text("lo'u") && lehu.is_cmavo_text("le'u"))]
    #[invariant(::SingleWordQuote => super::is_single_word_quote_marker(marker))]
    #[invariant(::Letter => bu.is_cmavo_text("bu"))]
    #[invariant(::ZeiLujvo => zei.is_cmavo_text("zei"))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub enum WordLike {
        Bare(#[tree_child(primary)] Word),
        ZoQuote {
            zo: Word,
            #[tree_child(primary)]
            word: Word,
        },
        ZoiQuote {
            zoi: Word,
            opening_delimiter: Word,
            quoted_text: Verbatim,
            closing_delimiter: Word,
        },
        LohuQuote {
            lohu: Word,
            #[tree_child(primary)]
            quoted_words: Vec<Word>,
            lehu: Word,
        },
        SingleWordQuote {
            marker: Word,
            #[tree_child(primary)]
            quoted_text: Verbatim,
        },
        Letter {
            #[tree_child(primary)]
            base: Box<WordLike>,
            bu: Word,
        },
        ZeiLujvo {
            left: Box<WordLike>,
            zei: Word,
            #[tree_child(primary)]
            right: Word,
        },
    }
}
