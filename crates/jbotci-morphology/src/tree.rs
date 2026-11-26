//! Morphology parse tree model.

use std::sync::Arc;

use bityzba::{data, invariant, requires};
use jbotci_source::SourceSpan;
use jbotci_tree::tree_model;
use serde::{Deserialize, Serialize};
use vec1::Vec1;

use crate::{Cmavo, Phonemes, Selmaho};

mod slot;

pub use slot::{ClassWords, SlotWalk, TreeSlot};

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
        data!(WordLike::LerfuWord { base, bu }) => bu.is_cmavo(Cmavo::Bu) && base.bare_word().is_some_and(Word::is_hesitation),
        _ => false,
    })]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ReplacementQuoteWord {
        #[tree_child(primary)]
        pub word: TreeSlot<WordLike>,
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
        && runs.first().is_none_or(|run| {
            run.marker.span().byte_end <= lehai.span().byte_start
                && run.marker.span().char_end <= lehai.span().char_start
        }))]
    #[bityzba::expensive_invariant(match self.as_data() {
        data!(WordLike::ReplacementQuote { runs, lehai }) => {
            crate::replacement_quote_spans_in_source_order(runs, lehai)
        },
        _ => true,
    })]
    #[invariant(::DelimitedWordQuote => super::is_single_word_quote_marker(marker))]
    #[invariant(::LerfuWord => bu.is_cmavo(Cmavo::Bu))]
    #[invariant(::ZeiCompound => zei.is_cmavo(Cmavo::Zei))]
    #[derive(Serialize)]
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
            base: TreeSlot<WordLike>,
            bu: Box<Word>,
        },
        ZeiCompound {
            left: TreeSlot<WordLike>,
            zei: Box<Word>,
            #[tree_child(primary)]
            right: Box<Word>,
        },
    }
}

impl<'tree> TreeWalkable<'tree> for TreeSlot<WordLike> {
    #[requires(true)]
    #[ensures(true)]
    fn walk_with<W>(&'tree self, walker: &mut W)
    where
        W: TreeWalker<'tree> + ?Sized,
    {
        let Some(root) = self.as_deref() else {
            return;
        };
        // Recursive on purpose, as for `Serialize`. Each filled child slot
        // calls this function again, so the depth count is the slot depth.
        // A chain of 2,666 `bu` links keeps that many frames. It does not fit
        // in a 160 KiB thread.
        TreeWalkable::walk_with(root, walker);
    }
}

mod word_like_glue;

pub(crate) use jbotci_tree::debug_emit::Emit;
pub(crate) use word_like_glue::{byte_range, map_spans, source_spans_into};

#[cfg(test)]
mod walk_tests {
    use bityzba::{invariant, new, requires};
    use jbotci_source::SourceSpan;

    use crate::{Phonemes, WordKind};

    use super::{
        ReplacementQuoteRun, ReplacementQuoteWord, TreeSlot, TreeWalkable, TreeWalker, Word,
        WordLike,
    };

    #[invariant(true)]
    struct EventWalker<'a> {
        other: Option<&'a TreeSlot<WordLike>>,
        prune_plain_base: bool,
        events: Vec<String>,
    }

    impl<'tree> TreeWalker<'tree> for EventWalker<'tree> {
        #[requires(true)]
        #[ensures(true)]
        fn walk_word_like_lerfu_word(
            &mut self,
            base: &'tree TreeSlot<WordLike>,
            bu: &'tree Box<Word>,
        ) {
            if let Some(other) = self.other {
                other.walk_with(self);
            }
            let prune = self.prune_plain_base
                && base
                    .as_deref()
                    .is_some_and(|node| node.bare_word().is_some());
            self.events.push(format!("before:{}", bu.span().char_start));
            if !prune {
                super::walk::word_like_lerfu_word(self, base, bu);
            }
            self.events.push(format!("after:{}", bu.span().char_start));
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_word_like_zei_compound(
            &mut self,
            left: &'tree TreeSlot<WordLike>,
            zei: &'tree Box<Word>,
            right: &'tree Box<Word>,
        ) {
            self.events
                .push(format!("before:{}", zei.span().char_start));
            super::walk::word_like_zei_compound(self, left, zei, right);
            self.events.push(format!("after:{}", zei.span().char_start));
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_word(&mut self, word: &'tree Word) {
            self.events.push(format!(
                "word:{}:{}",
                word.span().char_start,
                word.phonemes().as_str()
            ));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn cmavo(text: &str, start: usize) -> Word {
        let end = start + text.chars().count();
        let phonemes = Phonemes::from_canonical(text.to_owned()).expect("phonemes");
        let span = SourceSpan::new(None, start, end, start, end).expect("span");
        Word::from_kind(WordKind::Cmavo, phonemes, span)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r23_nested_walk_sees_a_separate_word() {
        let other = TreeSlot::new(WordLike::bare(cmavo("do", 10)));
        let tree = TreeSlot::new(WordLike::letter(
            WordLike::bare(cmavo("a", 0)),
            cmavo("bu", 2),
        ));
        let mut walker = EventWalker {
            other: Some(&other),
            prune_plain_base: false,
            events: Vec::new(),
        };
        tree.walk_with(&mut walker);
        assert_eq!(
            walker.events,
            ["word:10:do", "before:2", "word:0:a", "word:2:bu", "after:2",]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r23_pruned_inner_lerfu_omits_its_children() {
        let inner = WordLike::letter(WordLike::bare(cmavo("a", 0)), cmavo("bu", 2));
        let tree = TreeSlot::new(WordLike::letter(inner, cmavo("bu", 6)));
        let mut walker = EventWalker {
            other: None,
            prune_plain_base: true,
            events: Vec::new(),
        };
        tree.walk_with(&mut walker);
        assert!(walker.events.contains(&"word:6:bu".to_owned()));
        assert!(!walker.events.iter().any(|event| event == "word:0:a"));
        assert!(!walker.events.iter().any(|event| event == "word:2:bu"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r23_inner_lerfu_post_order_follows_its_children() {
        let inner = WordLike::letter(WordLike::bare(cmavo("a", 0)), cmavo("bu", 2));
        let tree = TreeSlot::new(WordLike::letter(inner, cmavo("bu", 6)));
        let mut walker = EventWalker {
            other: None,
            prune_plain_base: false,
            events: Vec::new(),
        };
        tree.walk_with(&mut walker);
        assert_eq!(
            walker.events,
            [
                "before:6",
                "before:2",
                "word:0:a",
                "word:2:bu",
                "after:2",
                "word:6:bu",
                "after:6",
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r23_zei_post_order_follows_its_children() {
        let tree = TreeSlot::new(WordLike::zei_lujvo(
            WordLike::bare(cmavo("a", 0)),
            cmavo("zei", 2),
            cmavo("b", 6),
        ));
        let mut walker = EventWalker {
            other: None,
            prune_plain_base: false,
            events: Vec::new(),
        };
        tree.walk_with(&mut walker);
        assert_eq!(
            walker.events,
            ["before:2", "word:0:a", "word:2:zei", "word:6:b", "after:2",]
        );
    }

    #[invariant(true)]
    struct QuoteWalker {
        events: Vec<String>,
    }

    impl<'tree> TreeWalker<'tree> for QuoteWalker {
        #[requires(true)]
        #[ensures(true)]
        fn walk_replacement_quote_run(&mut self, node: &'tree ReplacementQuoteRun) {
            self.events.push("run".to_owned());
            super::walk::replacement_quote_run(self, node);
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_replacement_quote_word(&mut self, node: &'tree ReplacementQuoteWord) {
            self.events.push("qword".to_owned());
            super::walk::replacement_quote_word(self, node);
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_word(&mut self, word: &'tree Word) {
            self.events
                .push(format!("word:{}", word.phonemes().as_str()));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn replacement_quote() -> WordLike {
        let marker = cmavo("lo'ai", 0);
        let inner = cmavo("sa", 6);
        let lehai = cmavo("le'ai", 9);
        let run = new!(ReplacementQuoteRun {
            marker,
            words: vec![new!(ReplacementQuoteWord {
                word: TreeSlot::new(WordLike::bare(inner)),
            })],
        });
        WordLike::replacement_quote(vec![run], lehai)
    }

    #[requires(true)]
    #[ensures(true)]
    fn quote_events(word: &WordLike) -> Vec<String> {
        let mut walker = QuoteWalker { events: Vec::new() };
        word.walk_with(&mut walker);
        walker.events
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r24_replacement_quote_methods_run_at_depth_1() {
        let quote = replacement_quote();
        let direct = quote_events(&quote);
        let mut slotted = QuoteWalker { events: Vec::new() };
        TreeSlot::new(quote.clone()).walk_with(&mut slotted);
        assert_eq!(slotted.events, direct);
        assert!(direct.iter().any(|event| event == "run"), "{direct:?}");
        assert!(direct.iter().any(|event| event == "qword"), "{direct:?}");
        let wrapped = WordLike::letter(quote, cmavo("bu", 20));
        let nested = quote_events(&wrapped);
        assert!(
            nested.windows(direct.len()).any(|window| window == direct),
            "{nested:?} missing {direct:?}"
        );
    }

    #[invariant(true)]
    struct PanicWalker {
        events: Vec<String>,
        arm_nested: bool,
    }

    impl<'tree> TreeWalker<'tree> for PanicWalker {
        #[requires(true)]
        #[ensures(true)]
        fn walk_word(&mut self, word: &'tree Word) {
            let text = word.phonemes().as_str().to_owned();
            self.events.push(text.clone());
            if text == "do" {
                panic!("nested walk");
            }
            if text == "a" && self.arm_nested {
                self.arm_nested = false;
                let local = WordLike::letter(WordLike::bare(cmavo("do", 20)), cmavo("bu", 23));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    local.walk_with(self);
                }));
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r24_caught_panic_does_not_visit_the_local_bu() {
        let tree = TreeSlot::new(WordLike::zei_lujvo(
            WordLike::bare(cmavo("a", 0)),
            cmavo("zei", 2),
            cmavo("b", 6),
        ));
        let mut walker = PanicWalker {
            events: Vec::new(),
            arm_nested: true,
        };
        tree.walk_with(&mut walker);
        assert_eq!(walker.events, ["a", "do", "zei", "b"]);
    }
}
