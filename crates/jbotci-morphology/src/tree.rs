//! Morphology parse tree model.

use bityzba::invariant;
use jbotci_source::SourceSpan;
use jbotci_tree::tree_model;
use serde::{Deserialize, Serialize};

use crate::WordKind;

tree_model! {
    #[invariant(!self.phonemes.is_empty(), "word phoneme text must not be empty")]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Word {
        pub kind: WordKind,
        pub phonemes: String,
        pub span: SourceSpan,
    }

    #[invariant(super::word_like_data_is_valid(self.as_data()))]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub enum WordLike {
        Bare(#[tree_child(primary)] Box<Word>),
        ZoQuote {
            zo: Box<Word>,
            #[tree_child(primary)]
            word: Box<Word>,
        },
        ZoiQuote {
            zoi: Box<Word>,
            opening_delimiter: Box<Word>,
            quoted_text: SourceSpan,
            closing_delimiter: Box<Word>,
        },
        LohuQuote {
            lohu: Box<Word>,
            #[tree_child(primary)]
            quoted_words: Vec<Word>,
            lehu: Box<Word>,
        },
        SingleWordQuote {
            marker: Box<Word>,
            #[tree_child(primary)]
            quoted_text: SourceSpan,
        },
        Letter {
            #[tree_child(primary)]
            base: Box<WordLike>,
            bu: Box<Word>,
        },
        ZeiLujvo {
            left: Box<WordLike>,
            zei: Box<Word>,
            #[tree_child(primary)]
            right: Box<Word>,
        },
    }
}
