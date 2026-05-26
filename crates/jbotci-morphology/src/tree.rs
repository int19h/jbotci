//! Morphology parse tree model.

use std::sync::Arc;

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
            span: Arc<SourceSpan>,
        },
        Gismu {
            phonemes: Phonemes,
            span: Arc<SourceSpan>,
        },
        Lujvo {
            #[tree_child(primary)]
            parts: Vec1<Jvopau>,
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
    pub enum Jvopau {
        Rafsi(Phonemes),
        Hyphen(Phonemes),
    }

    #[invariant(span.char_len() == text.chars().count(), "verbatim text must match span length")]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Verbatim {
        pub span: Arc<SourceSpan>,
        pub text: String,
    }

    #[invariant(::Bare(_) => true)]
    #[invariant(::ZoQuote => zo.is_cmavo(Cmavo::Zo))]
    #[invariant(::ZoiQuote => zoi.is_selmaho(Selmaho::Zoi)
        && crate::canonical_text_eq(
            opening_delimiter.phonemes().as_str(),
            closing_delimiter.phonemes().as_str(),
        )
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
            zo: Box<Word>,
            #[tree_child(primary)]
            word: Box<Word>,
        },
        ZoiQuote {
            zoi: Box<Word>,
            opening_delimiter: Box<Word>,
            quoted_text: Box<Verbatim>,
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
            quoted_text: Box<Verbatim>,
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
