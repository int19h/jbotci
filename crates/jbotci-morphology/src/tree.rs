//! Morphology parse tree model.

use bityzba::invariant;
use jbotci_source::SourceSpan;
use jbotci_tree::tree_model;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::{Phonemes, word_data_is_valid};

tree_model! {
    #[invariant(word_data_is_valid(self.as_data()))]
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

    #[invariant(super::word_like_data_is_valid(self.as_data()))]
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
