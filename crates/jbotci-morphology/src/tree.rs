//! Morphology parse tree model.

use bityzba::invariant;
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
    #[invariant(::ZoQuote => zo.is_cmavo(Cmavo::Zo))]
    #[invariant(::ZoiQuote => zoi.is_selmaho(Selmaho::Zoi)
        && opening_delimiter.span().byte_end <= quoted_text.span.byte_start
        && quoted_text.span.byte_end <= closing_delimiter.span().byte_start)]
    #[invariant(::LohuQuote => lohu.is_cmavo(Cmavo::Lohu) && lehu.is_cmavo(Cmavo::Lehu))]
    #[invariant(::SingleWordQuote => super::is_single_word_quote_marker(marker))]
    #[invariant(::Letter => bu.is_cmavo(Cmavo::Bu))]
    #[invariant(::ZeiLujvo => zei.is_cmavo(Cmavo::Zei))]
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
