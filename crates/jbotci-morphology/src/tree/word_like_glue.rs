//! Iterative `Clone`, `PartialEq`, and `Debug` for `WordLike`.
//!
//! `bu` and `zei` chains are one node per link. Derived glue walked that chain on the
//! call stack and overflowed a 160 KiB frame. These impls keep one frame and a heap stack.
//! `Debug` writes one buffer. Compact mode writes each byte once. Pretty mode writes
//! each output byte once. The bytes match a derived `Debug`. The walk stays on the heap,
//! so a long `bu` chain still fits.
//! `Serialize` stays the derived nested form. `Drop` lives on `TreeSlot<WordLike>`,
//! not on the validated wrapper.

use std::fmt;
use std::ops::Range;

use bityzba::{contract_trait, data, invariant, new, requires, try_new};
use jbotci_source::SourceSpan;

use super::Emit;
use super::{
    ReplacementQuoteRun, ReplacementQuoteWord, SlotWalk, TreeSlot, Word, WordLike, WordLikeData,
};

#[invariant(true)]
#[invariant(::Node(_) => true)]
#[invariant(::Span(_) => true)]
enum SpanJob<'a> {
    Node(&'a WordLike),
    Span(&'a SourceSpan),
}

/// A finish frame owns the children it has received.
/// `base` and `left` are absent while that child is still on the stack.
/// `got` stays at most the sum of `counts`.
#[invariant(::Enter(_) => true)]
#[invariant(::FinishLerfu { bu, .. } => bu.is_cmavo(crate::Cmavo::Bu))]
#[invariant(::FinishZei { zei, .. } => zei.is_cmavo(crate::Cmavo::Zei))]
#[invariant(::FinishQuote { lehai, markers, counts, got } =>
    lehai.is_cmavo(crate::Cmavo::Lehai)
        && markers.len() == counts.len()
        && counts.len() <= 2
        && markers.iter().all(|marker| marker.is_selmaho(crate::Selmaho::Lohai))
        && got.len() <= counts.iter().sum::<usize>())]
enum CloneTask<'a> {
    Enter(&'a WordLike),
    FinishLerfu {
        bu: Word,
        base: Option<WordLike>,
    },
    FinishZei {
        zei: Word,
        right: Word,
        left: Option<WordLike>,
    },
    FinishQuote {
        lehai: Word,
        markers: Vec<Word>,
        counts: Vec<usize>,
        got: Vec<WordLike>,
    },
    /// Placeholder while a collector frame is updated.
    Slot,
}

/// A finish frame keeps the fields that are mapped after the child.
/// A quote frame maps one marker, then that run's children, then the next run.
/// `lehai` stays unmapped until every run is done.
#[invariant(::Enter(_) => true)]
#[invariant(::FinishLerfu { bu } => bu.is_cmavo(crate::Cmavo::Bu))]
#[invariant(::FinishZei { zei, .. } => zei.is_cmavo(crate::Cmavo::Zei))]
#[invariant(::Quote { lehai, pending, rest, marker, got, built } =>
    lehai.is_cmavo(crate::Cmavo::Lehai)
        && pending.len() + built.len() + usize::from(marker.is_some()) <= 2
        && match marker {
            Some(marker) => marker.is_selmaho(crate::Selmaho::Lohai),
            None => rest.is_empty() && got.is_empty(),
        })]
enum MapTask {
    Enter(WordLike),
    FinishLerfu {
        bu: Word,
    },
    FinishZei {
        zei: Word,
        right: Word,
    },
    Quote {
        lehai: Word,
        /// Runs not started. The next run is at the back.
        pending: Vec<ReplacementQuoteRun>,
        /// Children of the active run that are not entered yet. The next child is at the back.
        rest: Vec<WordLike>,
        /// Mapped marker of the active run.
        marker: Option<Word>,
        /// Mapped children of the active run.
        got: Vec<WordLike>,
        built: Vec<ReplacementQuoteRun>,
    },
}

#[invariant(::BeginStruct(name) => !name.is_empty())]
#[invariant(::BeginTuple(name) => !name.is_empty())]
#[invariant(::Field(name) => !name.is_empty())]
#[invariant(::Node(_) => true)]
#[invariant(::Leaf(_) => true)]
enum Step<'a> {
    Node(&'a WordLike),
    Leaf(&'a dyn fmt::Debug),
    BeginStruct(&'static str),
    BeginTuple(&'static str),
    BeginList,
    Field(&'static str),
    EndField,
    TupleField,
    EndTupleField,
    Item,
    EndItem,
    EndStruct,
    EndTuple,
    EndList,
}

#[requires(true)]
#[ensures(true)]
fn apply_step<'a>(emit: &mut Emit, stack: &mut Vec<Step<'a>>, step: Step<'a>) {
    match step.as_data() {
        data!(Step::Node(word)) => expand_word(stack, word),
        data!(Step::Leaf(value)) => emit.debug_value(value),
        data!(Step::BeginStruct(name)) => emit.begin_struct(name),
        data!(Step::BeginTuple(name)) => emit.begin_tuple(name),
        data!(Step::BeginList) => emit.begin_list(),
        data!(Step::Field(name)) => emit.begin_field(name),
        data!(Step::EndField) => emit.end_field(),
        data!(Step::TupleField) => emit.begin_tuple_field(),
        data!(Step::EndTupleField) => emit.end_tuple_field(),
        data!(Step::Item) => emit.begin_item(),
        data!(Step::EndItem) => emit.end_item(),
        data!(Step::EndStruct) => emit.end_struct(),
        data!(Step::EndTuple) => emit.end_tuple(),
        data!(Step::EndList) => emit.end_list(),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_forward<'a>(stack: &mut Vec<Step<'a>>, steps: Vec<Step<'a>>) {
    stack.extend(steps.into_iter().rev());
}

#[requires(true)]
#[ensures(true)]
fn expand_word<'a>(stack: &mut Vec<Step<'a>>, word: &'a WordLike) {
    match word.as_data() {
        data!(WordLike::PlainWord(word)) => push_forward(
            stack,
            vec![
                new!(Step::BeginTuple("PlainWord")),
                new!(Step::TupleField),
                new!(Step::Leaf(word)),
                new!(Step::EndTupleField),
                new!(Step::EndTuple),
            ],
        ),
        data!(WordLike::QuotedWord { zo, word }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("QuotedWord")),
                new!(Step::Field("zo")),
                new!(Step::Leaf(zo)),
                new!(Step::EndField),
                new!(Step::Field("word")),
                new!(Step::Leaf(word)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("SelmahoQuotedWord")),
                new!(Step::Field("mahoi")),
                new!(Step::Leaf(mahoi)),
                new!(Step::EndField),
                new!(Step::Field("word")),
                new!(Step::Leaf(word)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("DelimitedNonLojbanQuote")),
                new!(Step::Field("zoi")),
                new!(Step::Leaf(zoi)),
                new!(Step::EndField),
                new!(Step::Field("opening_delimiter")),
                new!(Step::Leaf(opening_delimiter)),
                new!(Step::EndField),
                new!(Step::Field("quoted_text")),
                new!(Step::Leaf(quoted_text)),
                new!(Step::EndField),
                new!(Step::Field("closing_delimiter")),
                new!(Step::Leaf(closing_delimiter)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("QuotedWords")),
                new!(Step::Field("lohu")),
                new!(Step::Leaf(lohu)),
                new!(Step::EndField),
                new!(Step::Field("quoted_words")),
                new!(Step::Leaf(quoted_words)),
                new!(Step::EndField),
                new!(Step::Field("lehu")),
                new!(Step::Leaf(lehu)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("DelimitedWordQuote")),
                new!(Step::Field("marker")),
                new!(Step::Leaf(marker)),
                new!(Step::EndField),
                new!(Step::Field("quoted_text")),
                new!(Step::Leaf(quoted_text)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::LerfuWord { base, bu }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("LerfuWord")),
                new!(Step::Field("base")),
                new!(Step::Node(base.as_ref())),
                new!(Step::EndField),
                new!(Step::Field("bu")),
                new!(Step::Leaf(bu)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::ZeiCompound { left, zei, right }) => push_forward(
            stack,
            vec![
                new!(Step::BeginStruct("ZeiCompound")),
                new!(Step::Field("left")),
                new!(Step::Node(left.as_ref())),
                new!(Step::EndField),
                new!(Step::Field("zei")),
                new!(Step::Leaf(zei)),
                new!(Step::EndField),
                new!(Step::Field("right")),
                new!(Step::Leaf(right)),
                new!(Step::EndField),
                new!(Step::EndStruct),
            ],
        ),
        data!(WordLike::ReplacementQuote { runs, lehai }) => {
            let mut steps = vec![
                new!(Step::BeginStruct("ReplacementQuote")),
                new!(Step::Field("runs")),
                new!(Step::BeginList),
            ];
            for run in runs {
                steps.push(new!(Step::Item));
                steps.push(new!(Step::BeginStruct("ReplacementQuoteRun")));
                steps.push(new!(Step::Field("marker")));
                steps.push(new!(Step::Leaf(&run.marker)));
                steps.push(new!(Step::EndField));
                steps.push(new!(Step::Field("words")));
                steps.push(new!(Step::BeginList));
                for word in &run.words {
                    steps.push(new!(Step::Item));
                    steps.push(new!(Step::BeginStruct("ReplacementQuoteWord")));
                    steps.push(new!(Step::Field("word")));
                    steps.push(new!(Step::Node(word.word.as_ref())));
                    steps.push(new!(Step::EndField));
                    steps.push(new!(Step::EndStruct));
                    steps.push(new!(Step::EndItem));
                }
                steps.push(new!(Step::EndList));
                steps.push(new!(Step::EndField));
                steps.push(new!(Step::EndStruct));
                steps.push(new!(Step::EndItem));
            }
            steps.push(new!(Step::EndList));
            steps.push(new!(Step::EndField));
            steps.push(new!(Step::Field("lehai")));
            steps.push(new!(Step::Leaf(lehai)));
            steps.push(new!(Step::EndField));
            steps.push(new!(Step::EndStruct));
            push_forward(stack, steps);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_clone_lerfu<'a>(stack: &mut Vec<CloneTask<'a>>, open: &mut Vec<usize>, bu: Word) {
    stack.push(new!(CloneTask::FinishLerfu { bu, base: None }));
    open.push(stack.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn push_clone_zei<'a>(
    stack: &mut Vec<CloneTask<'a>>,
    open: &mut Vec<usize>,
    zei: Word,
    right: Word,
) {
    stack.push(new!(CloneTask::FinishZei {
        zei,
        right,
        left: None,
    }));
    open.push(stack.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn push_clone_quote<'a>(
    stack: &mut Vec<CloneTask<'a>>,
    open: &mut Vec<usize>,
    lehai: Word,
    markers: Vec<Word>,
    counts: Vec<usize>,
) {
    let need = counts.iter().sum::<usize>();
    stack.push(new!(CloneTask::FinishQuote {
        lehai,
        markers,
        counts,
        got: Vec::new(),
    }));
    if need > 0 {
        open.push(stack.len() - 1);
    }
}

#[requires(true)]
#[ensures(true)]
fn deliver_clone<'a>(
    stack: &mut Vec<CloneTask<'a>>,
    open: &mut Vec<usize>,
    built: &mut Vec<WordLike>,
    child: WordLike,
) {
    let Some(&index) = open.last() else {
        built.push(child);
        return;
    };
    let task = std::mem::replace(&mut stack[index], new!(CloneTask::Slot));
    let (task, full) = match task.into_data() {
        CloneTaskData::FinishLerfu { bu, .. } => (
            new!(CloneTask::FinishLerfu {
                bu,
                base: Some(child),
            }),
            true,
        ),
        CloneTaskData::FinishZei { zei, right, .. } => (
            new!(CloneTask::FinishZei {
                zei,
                right,
                left: Some(child),
            }),
            true,
        ),
        CloneTaskData::FinishQuote {
            lehai,
            markers,
            counts,
            mut got,
        } => {
            got.push(child);
            let full = got.len() == counts.iter().sum::<usize>();
            (
                new!(CloneTask::FinishQuote {
                    lehai,
                    markers,
                    counts,
                    got,
                }),
                full,
            )
        }
        other => (CloneTask::from_data(other), false),
    };
    stack[index] = task;
    if full {
        open.pop();
    }
}

impl Clone for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn clone(&self) -> Self {
        let mut stack = vec![new!(CloneTask::Enter(self))];
        let mut open = Vec::new();
        let mut built = Vec::new();
        while let Some(task) = stack.pop() {
            match task.into_data() {
                CloneTaskData::Slot => {}
                CloneTaskData::FinishLerfu { bu, base } => {
                    let base = base.expect("lerfu base");
                    deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::letter(base, bu),
                    );
                }
                CloneTaskData::FinishZei { zei, right, left } => {
                    let left = left.expect("zei left");
                    deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::zei_lujvo(left, zei, right),
                    );
                }
                CloneTaskData::FinishQuote {
                    lehai,
                    markers,
                    counts,
                    got,
                } => {
                    let mut kids = got.into_iter();
                    let mut runs = Vec::new();
                    for (marker, count) in markers.into_iter().zip(counts) {
                        let mut words = Vec::with_capacity(count);
                        for _ in 0..count {
                            words.push(new!(ReplacementQuoteWord {
                                word: TreeSlot::new(kids.next().expect("quote child"))
                            }));
                        }
                        runs.push(new!(ReplacementQuoteRun { marker, words }));
                    }
                    deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::replacement_quote(runs, lehai),
                    );
                }
                CloneTaskData::Enter(word) => match word.as_data() {
                    data!(WordLike::PlainWord(word)) => deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::bare(word.clone()),
                    ),
                    data!(WordLike::QuotedWord { zo, word }) => {
                        deliver_clone(
                            &mut stack,
                            &mut open,
                            &mut built,
                            WordLike::zo_quote(zo.as_ref().clone(), word.as_ref().clone()),
                        );
                    }
                    data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
                        deliver_clone(
                            &mut stack,
                            &mut open,
                            &mut built,
                            WordLike::mahoi_quote(mahoi.as_ref().clone(), word.as_ref().clone()),
                        );
                    }
                    data!(WordLike::DelimitedNonLojbanQuote {
                        zoi,
                        opening_delimiter,
                        quoted_text,
                        closing_delimiter,
                    }) => deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::zoi_quote(
                            zoi.as_ref().clone(),
                            opening_delimiter.as_ref().clone(),
                            quoted_text.as_ref().clone(),
                            closing_delimiter.as_ref().clone(),
                        ),
                    ),
                    data!(WordLike::QuotedWords {
                        lohu,
                        quoted_words,
                        lehu,
                    }) => deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::lohu_quote(
                            lohu.as_ref().clone(),
                            quoted_words.clone(),
                            lehu.as_ref().clone(),
                        ),
                    ),
                    data!(WordLike::DelimitedWordQuote {
                        marker,
                        quoted_text,
                    }) => deliver_clone(
                        &mut stack,
                        &mut open,
                        &mut built,
                        WordLike::single_word_quote(
                            marker.as_ref().clone(),
                            quoted_text.as_ref().clone(),
                        ),
                    ),
                    data!(WordLike::LerfuWord { base, bu }) => {
                        push_clone_lerfu(&mut stack, &mut open, bu.as_ref().clone());
                        stack.push(new!(CloneTask::Enter(base.as_ref())));
                    }
                    data!(WordLike::ZeiCompound { left, zei, right }) => {
                        push_clone_zei(
                            &mut stack,
                            &mut open,
                            zei.as_ref().clone(),
                            right.as_ref().clone(),
                        );
                        stack.push(new!(CloneTask::Enter(left.as_ref())));
                    }
                    data!(WordLike::ReplacementQuote { runs, lehai }) => {
                        let mut markers = Vec::new();
                        let mut counts = Vec::new();
                        let mut children = Vec::new();
                        for run in runs {
                            markers.push(run.marker.clone());
                            counts.push(run.words.len());
                            children.extend(run.words.iter().map(|word| word.word.as_ref()));
                        }
                        push_clone_quote(
                            &mut stack,
                            &mut open,
                            lehai.as_ref().clone(),
                            markers,
                            counts,
                        );
                        for child in children.into_iter().rev() {
                            stack.push(new!(CloneTask::Enter(child)));
                        }
                    }
                },
            }
        }
        built.pop().expect("clone")
    }
}

impl PartialEq for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn eq(&self, other: &Self) -> bool {
        let mut stack = vec![(self, other)];
        while let Some((left, right)) = stack.pop() {
            if !eq_step(&mut stack, left, right) {
                return false;
            }
        }
        true
    }
}

impl Eq for WordLike {}

#[requires(true)]
#[ensures(true)]
fn eq_step<'a>(
    stack: &mut Vec<(&'a WordLike, &'a WordLike)>,
    left: &'a WordLike,
    right: &'a WordLike,
) -> bool {
    match (left.as_data(), right.as_data()) {
        (data!(WordLike::PlainWord(left)), data!(WordLike::PlainWord(right))) => left == right,
        (
            data!(WordLike::QuotedWord {
                zo: left_zo,
                word: left_word
            }),
            data!(WordLike::QuotedWord {
                zo: right_zo,
                word: right_word
            }),
        ) => left_zo.as_ref() == right_zo.as_ref() && left_word.as_ref() == right_word.as_ref(),
        (
            data!(WordLike::SelmahoQuotedWord {
                mahoi: left_mahoi,
                word: left_word
            }),
            data!(WordLike::SelmahoQuotedWord {
                mahoi: right_mahoi,
                word: right_word
            }),
        ) => {
            left_mahoi.as_ref() == right_mahoi.as_ref() && left_word.as_ref() == right_word.as_ref()
        }
        (
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi: left_zoi,
                opening_delimiter: left_open,
                quoted_text: left_text,
                closing_delimiter: left_close,
            }),
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi: right_zoi,
                opening_delimiter: right_open,
                quoted_text: right_text,
                closing_delimiter: right_close,
            }),
        ) => {
            left_zoi.as_ref() == right_zoi.as_ref()
                && left_open.as_ref() == right_open.as_ref()
                && left_text.as_ref() == right_text.as_ref()
                && left_close.as_ref() == right_close.as_ref()
        }
        (
            data!(WordLike::QuotedWords {
                lohu: left_lohu,
                quoted_words: left_words,
                lehu: left_lehu,
            }),
            data!(WordLike::QuotedWords {
                lohu: right_lohu,
                quoted_words: right_words,
                lehu: right_lehu,
            }),
        ) => {
            left_lohu.as_ref() == right_lohu.as_ref()
                && left_words == right_words
                && left_lehu.as_ref() == right_lehu.as_ref()
        }
        (
            data!(WordLike::DelimitedWordQuote {
                marker: left_marker,
                quoted_text: left_text,
            }),
            data!(WordLike::DelimitedWordQuote {
                marker: right_marker,
                quoted_text: right_text,
            }),
        ) => {
            left_marker.as_ref() == right_marker.as_ref()
                && left_text.as_ref() == right_text.as_ref()
        }
        (
            data!(WordLike::LerfuWord {
                base: left_base,
                bu: left_bu
            }),
            data!(WordLike::LerfuWord {
                base: right_base,
                bu: right_bu
            }),
        ) => {
            if left_bu.as_ref() != right_bu.as_ref() {
                return false;
            }
            stack.push((left_base.as_ref(), right_base.as_ref()));
            true
        }
        (
            data!(WordLike::ZeiCompound {
                left: left_left,
                zei: left_zei,
                right: left_right,
            }),
            data!(WordLike::ZeiCompound {
                left: right_left,
                zei: right_zei,
                right: right_right,
            }),
        ) => {
            if left_zei.as_ref() != right_zei.as_ref()
                || left_right.as_ref() != right_right.as_ref()
            {
                return false;
            }
            stack.push((left_left.as_ref(), right_left.as_ref()));
            true
        }
        (
            data!(WordLike::ReplacementQuote {
                runs: left_runs,
                lehai: left_lehai
            }),
            data!(WordLike::ReplacementQuote {
                runs: right_runs,
                lehai: right_lehai
            }),
        ) => {
            if left_lehai.as_ref() != right_lehai.as_ref() || left_runs.len() != right_runs.len() {
                return false;
            }
            for (left_run, right_run) in left_runs.iter().zip(right_runs.iter()) {
                if left_run.marker != right_run.marker
                    || left_run.words.len() != right_run.words.len()
                {
                    return false;
                }
                for (left_word, right_word) in left_run.words.iter().zip(right_run.words.iter()) {
                    stack.push((left_word.word.as_ref(), right_word.word.as_ref()));
                }
            }
            true
        }
        _ => false,
    }
}

#[invariant(true)]
#[invariant(::Node(_) => true)]
#[invariant(::Marker(_) => true)]
#[invariant(::Closing(_) => true)]
#[derive(Debug)]
pub enum PendingWord {
    Node(WordLike),
    Marker(Word),
    Closing(Box<Word>),
}

#[requires(true)]
#[ensures(true)]
fn detach_word(node: WordLike, pending: &mut Vec<PendingWord>) {
    match node.into_data() {
        WordLikeData::LerfuWord { mut base, bu } => {
            drop(bu);
            if let Some(child) = base.take() {
                pending.push(PendingWord::Node(*child));
            }
        }
        WordLikeData::ZeiCompound {
            mut left,
            zei,
            right,
        } => {
            drop(right);
            drop(zei);
            if let Some(child) = left.take() {
                pending.push(PendingWord::Node(*child));
            }
        }
        WordLikeData::ReplacementQuote { runs, lehai } => {
            // Push the closing word first, then each run from first to last.
            // The stack pops the last run first: its marker, then its words from last to first.
            // A word's own children sit above the earlier words, so that word finishes first.
            pending.push(PendingWord::Closing(lehai));
            for run in runs {
                let run = run.into_data();
                for word in run.words {
                    let mut slot = word.into_data().word;
                    if let Some(child) = slot.take() {
                        pending.push(PendingWord::Node(*child));
                    }
                }
                pending.push(PendingWord::Marker(run.marker));
            }
        }
        other => drop(other),
    }
}

#[contract_trait]
impl SlotWalk for WordLike {
    type Piece = PendingWord;

    fn into_piece(self) -> PendingWord {
        PendingWord::Node(self)
    }

    fn detach_piece(piece: PendingWord, pending: &mut Vec<PendingWord>) {
        match piece {
            PendingWord::Node(node) => detach_word(node, pending),
            PendingWord::Marker(marker) => drop(marker),
            PendingWord::Closing(lehai) => drop(lehai),
        }
    }
}

impl fmt::Debug for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut emit = Emit::new(formatter.alternate());
        let mut stack = vec![new!(Step::Node(self))];
        while let Some(step) = stack.pop() {
            apply_step(&mut emit, &mut stack, step);
        }
        formatter.write_str(&emit.finish())
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn source_spans_into<'a>(word: &'a WordLike, out: &mut Vec<&'a SourceSpan>) {
    let mut stack = vec![SpanJob::Node(word)];
    while let Some(job) = stack.pop() {
        match job {
            SpanJob::Span(span) => out.push(span),
            SpanJob::Node(word) => push_span_jobs(&mut stack, word),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_span_jobs<'a>(stack: &mut Vec<SpanJob<'a>>, word: &'a WordLike) {
    match word.as_data() {
        data!(WordLike::PlainWord(word)) => stack.push(SpanJob::Span(word.span())),
        data!(WordLike::QuotedWord { zo, word }) => {
            stack.push(SpanJob::Span(word.span()));
            stack.push(SpanJob::Span(zo.span()));
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            stack.push(SpanJob::Span(word.span()));
            stack.push(SpanJob::Span(mahoi.span()));
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => {
            stack.push(SpanJob::Span(closing_delimiter.span()));
            stack.push(SpanJob::Span(quoted_text.span.as_ref()));
            stack.push(SpanJob::Span(opening_delimiter.span()));
            stack.push(SpanJob::Span(zoi.span()));
        }
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu
        }) => {
            stack.push(SpanJob::Span(lehu.span()));
            for word in quoted_words.iter().rev() {
                stack.push(SpanJob::Span(word.span()));
            }
            stack.push(SpanJob::Span(lohu.span()));
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text
        }) => {
            stack.push(SpanJob::Span(quoted_text.span.as_ref()));
            stack.push(SpanJob::Span(marker.span()));
        }
        data!(WordLike::ReplacementQuote { runs, lehai }) => {
            stack.push(SpanJob::Span(lehai.span()));
            for run in runs.iter().rev() {
                for word in run.words.iter().rev() {
                    stack.push(SpanJob::Node(word.word.as_ref()));
                }
                stack.push(SpanJob::Span(run.marker.span()));
            }
        }
        data!(WordLike::LerfuWord { base, bu }) => {
            stack.push(SpanJob::Span(bu.span()));
            stack.push(SpanJob::Node(base.as_ref()));
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            stack.push(SpanJob::Span(right.span()));
            stack.push(SpanJob::Span(zei.span()));
            stack.push(SpanJob::Node(left.as_ref()));
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
pub(crate) fn byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    let mut node = word_like;
    let mut tail_end = None;
    loop {
        match node.as_data() {
            data!(WordLike::LerfuWord { base, bu }) => {
                let end = bu.span().byte_end;
                tail_end = Some(tail_end.map_or(end, |tail: usize| tail.max(end)));
                node = base.as_ref();
            }
            data!(WordLike::ZeiCompound { left, right, .. }) => {
                let end = right.span().byte_end;
                tail_end = Some(tail_end.map_or(end, |tail: usize| tail.max(end)));
                node = left.as_ref();
            }
            other => {
                let range = shallow_byte_range(other)?;
                let end = tail_end.map_or(range.end, |tail| tail.max(range.end));
                return Some(range.start..end);
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn shallow_byte_range(word_like: &WordLikeData) -> Option<Range<usize>> {
    match word_like {
        WordLikeData::PlainWord(word) => Some(word.span().byte_start..word.span().byte_end),
        WordLikeData::QuotedWord { zo, word } => Some(zo.span().byte_start..word.span().byte_end),
        WordLikeData::SelmahoQuotedWord { mahoi, word } => {
            Some(mahoi.span().byte_start..word.span().byte_end)
        }
        WordLikeData::DelimitedNonLojbanQuote {
            zoi,
            closing_delimiter,
            ..
        } => Some(zoi.span().byte_start..closing_delimiter.span().byte_end),
        WordLikeData::QuotedWords { lohu, lehu, .. } => {
            Some(lohu.span().byte_start..lehu.span().byte_end)
        }
        WordLikeData::DelimitedWordQuote {
            marker,
            quoted_text,
        } => Some(marker.span().byte_start..quoted_text.span.byte_end),
        WordLikeData::ReplacementQuote { runs, lehai } => Some(
            runs.first()
                .map_or(lehai.span().byte_start, |run| run.marker.span().byte_start)
                ..lehai.span().byte_end,
        ),
        WordLikeData::LerfuWord { .. } | WordLikeData::ZeiCompound { .. } => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub(crate) fn map_spans<F>(word_like: WordLike, map_span: &F) -> Result<WordLike, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    let mut stack = vec![new!(MapTask::Enter(word_like))];
    let mut results = Vec::new();
    while let Some(task) = stack.pop() {
        match task.into_data() {
            MapTaskData::FinishLerfu { bu } => {
                let base = results.pop().expect("lerfu base");
                let bu = crate::map_word_spans(bu, map_span)?;
                results.push(WordLike::letter(base, bu));
            }
            MapTaskData::FinishZei { zei, right } => {
                let left = results.pop().expect("zei left");
                let zei = crate::map_word_spans(zei, map_span)?;
                let right = crate::map_word_spans(right, map_span)?;
                results.push(WordLike::zei_lujvo(left, zei, right));
            }
            MapTaskData::Quote {
                lehai,
                mut pending,
                mut rest,
                marker,
                mut got,
                mut built,
            } => {
                if let Some(marker) = marker {
                    let child = results.pop().expect("quote child");
                    got.push(child);
                    if let Some(next) = rest.pop() {
                        stack.push(new!(MapTask::Quote {
                            lehai,
                            pending,
                            rest,
                            marker: Some(marker),
                            got,
                            built,
                        }));
                        stack.push(new!(MapTask::Enter(next)));
                    } else {
                        let words = got
                            .into_iter()
                            .map(|word| {
                                new!(ReplacementQuoteWord {
                                    word: TreeSlot::new(word),
                                })
                            })
                            .collect();
                        built.push(new!(ReplacementQuoteRun { marker, words }));
                        stack.push(new!(MapTask::Quote {
                            lehai,
                            pending,
                            rest,
                            marker: None,
                            got: Vec::new(),
                            built,
                        }));
                    }
                } else if let Some(run) = pending.pop() {
                    let run = run.into_data();
                    let marker = crate::map_word_spans(run.marker, map_span)?;
                    let mut children = run
                        .words
                        .into_iter()
                        .map(|word| word.into_data().word.into_inner())
                        .collect::<Vec<_>>();
                    children.reverse();
                    if let Some(next) = children.pop() {
                        stack.push(new!(MapTask::Quote {
                            lehai,
                            pending,
                            rest: children,
                            marker: Some(marker),
                            got: Vec::new(),
                            built,
                        }));
                        stack.push(new!(MapTask::Enter(next)));
                    } else {
                        built.push(new!(ReplacementQuoteRun {
                            marker,
                            words: Vec::new(),
                        }));
                        stack.push(new!(MapTask::Quote {
                            lehai,
                            pending,
                            rest: Vec::new(),
                            marker: None,
                            got: Vec::new(),
                            built,
                        }));
                    }
                } else {
                    let lehai = crate::map_word_spans(lehai, map_span)?;
                    let quoted = try_new!(WordLike::ReplacementQuote {
                        runs: built,
                        lehai: Box::new(lehai),
                    })
                    .map_err(|_| {
                        "Source spans in the replacement quote are out of order.".to_owned()
                    })?;
                    results.push(quoted);
                }
            }
            MapTaskData::Enter(word) => match word.into_data() {
                WordLikeData::PlainWord(word) => {
                    results.push(WordLike::bare(crate::map_word_spans(word, map_span)?));
                }
                WordLikeData::QuotedWord { zo, word } => {
                    results.push(WordLike::zo_quote(
                        crate::map_word_spans(*zo, map_span)?,
                        crate::map_word_spans(*word, map_span)?,
                    ));
                }
                WordLikeData::SelmahoQuotedWord { mahoi, word } => {
                    results.push(WordLike::mahoi_quote(
                        crate::map_word_spans(*mahoi, map_span)?,
                        crate::map_word_spans(*word, map_span)?,
                    ));
                }
                WordLikeData::DelimitedNonLojbanQuote {
                    zoi,
                    opening_delimiter,
                    quoted_text,
                    closing_delimiter,
                } => {
                    results.push(WordLike::zoi_quote(
                        crate::map_word_spans(*zoi, map_span)?,
                        crate::map_word_spans(*opening_delimiter, map_span)?,
                        crate::map_verbatim_span(*quoted_text, map_span)?,
                        crate::map_word_spans(*closing_delimiter, map_span)?,
                    ));
                }
                WordLikeData::QuotedWords {
                    lohu,
                    quoted_words,
                    lehu,
                } => {
                    let lohu = crate::map_word_spans(*lohu, map_span)?;
                    let mut words = Vec::with_capacity(quoted_words.len());
                    for word in quoted_words {
                        words.push(crate::map_word_spans(word, map_span)?);
                    }
                    let lehu = crate::map_word_spans(*lehu, map_span)?;
                    results.push(WordLike::lohu_quote(lohu, words, lehu));
                }
                WordLikeData::DelimitedWordQuote {
                    marker,
                    quoted_text,
                } => {
                    results.push(WordLike::single_word_quote(
                        crate::map_word_spans(*marker, map_span)?,
                        crate::map_verbatim_span(*quoted_text, map_span)?,
                    ));
                }
                WordLikeData::LerfuWord { base, bu } => {
                    stack.push(new!(MapTask::FinishLerfu { bu: *bu }));
                    stack.push(new!(MapTask::Enter(base.into_inner())));
                }
                WordLikeData::ZeiCompound { left, zei, right } => {
                    stack.push(new!(MapTask::FinishZei {
                        zei: *zei,
                        right: *right,
                    }));
                    stack.push(new!(MapTask::Enter(left.into_inner())));
                }
                WordLikeData::ReplacementQuote { runs, lehai } => {
                    let mut pending = runs;
                    pending.reverse();
                    stack.push(new!(MapTask::Quote {
                        lehai: *lehai,
                        pending,
                        rest: Vec::new(),
                        marker: None,
                        got: Vec::new(),
                        built: Vec::new(),
                    }));
                }
            },
        }
    }
    Ok(results.pop().expect("mapped word"))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fmt;

    use bityzba::{data, invariant, requires, try_new};
    use jbotci_source::SourceSpan;

    use super::*;

    /// A standard `Debug` painter for one borrowed value.
    ///
    /// The borrowed value already checks its own shape. This struct holds no
    /// extra state, so it has no tighter invariant.
    #[invariant(true)]
    struct StdView<'a, T: ?Sized> {
        value: &'a T,
        paint: fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
    }

    impl<T: ?Sized> fmt::Debug for StdView<'_, T> {
        #[requires(true)]
        #[ensures(true)]
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            (self.paint)(self.value, formatter)
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_word(word: &WordLike, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match word.as_data() {
            data!(WordLike::PlainWord(inner)) => {
                formatter.debug_tuple("PlainWord").field(inner).finish()
            }
            data!(WordLike::QuotedWord { zo, word }) => formatter
                .debug_struct("QuotedWord")
                .field("zo", zo)
                .field("word", word)
                .finish(),
            data!(WordLike::SelmahoQuotedWord { mahoi, word }) => formatter
                .debug_struct("SelmahoQuotedWord")
                .field("mahoi", mahoi)
                .field("word", word)
                .finish(),
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => formatter
                .debug_struct("DelimitedNonLojbanQuote")
                .field("zoi", zoi)
                .field("opening_delimiter", opening_delimiter)
                .field("quoted_text", quoted_text)
                .field("closing_delimiter", closing_delimiter)
                .finish(),
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => formatter
                .debug_struct("QuotedWords")
                .field("lohu", lohu)
                .field("quoted_words", quoted_words)
                .field("lehu", lehu)
                .finish(),
            data!(WordLike::DelimitedWordQuote {
                marker,
                quoted_text,
            }) => formatter
                .debug_struct("DelimitedWordQuote")
                .field("marker", marker)
                .field("quoted_text", quoted_text)
                .finish(),
            data!(WordLike::LerfuWord { base, bu }) => formatter
                .debug_struct("LerfuWord")
                .field(
                    "base",
                    &StdView {
                        value: base.as_ref(),
                        paint: paint_word,
                    },
                )
                .field("bu", bu)
                .finish(),
            data!(WordLike::ZeiCompound { left, zei, right }) => formatter
                .debug_struct("ZeiCompound")
                .field(
                    "left",
                    &StdView {
                        value: left.as_ref(),
                        paint: paint_word,
                    },
                )
                .field("zei", zei)
                .field("right", right)
                .finish(),
            data!(WordLike::ReplacementQuote { runs, lehai }) => formatter
                .debug_struct("ReplacementQuote")
                .field(
                    "runs",
                    &StdView {
                        value: runs.as_slice(),
                        paint: paint_runs,
                    },
                )
                .field("lehai", lehai)
                .finish(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_runs(runs: &[ReplacementQuoteRun], formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = formatter.debug_list();
        for run in runs {
            list.entry(&StdView {
                value: run,
                paint: paint_run,
            });
        }
        list.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_run(run: &ReplacementQuoteRun, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplacementQuoteRun")
            .field("marker", &run.marker)
            .field(
                "words",
                &StdView {
                    value: run.words.as_slice(),
                    paint: paint_quote_words,
                },
            )
            .finish()
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_quote_words(
        words: &[ReplacementQuoteWord],
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let mut list = formatter.debug_list();
        for word in words {
            list.entry(&StdView {
                value: word,
                paint: paint_quote_word,
            });
        }
        list.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_quote_word(
        word: &ReplacementQuoteWord,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter
            .debug_struct("ReplacementQuoteWord")
            .field(
                "word",
                &StdView {
                    value: word.word.as_ref(),
                    paint: paint_word,
                },
            )
            .finish()
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_std_word(input: &str, kind: &str) {
        let words = raw_words(input);
        assert_eq!(words.len(), 1, "{input}");
        let word = &words[0];
        match (kind, word.as_data()) {
            ("PlainWord", data!(WordLike::PlainWord(_)))
            | ("QuotedWord", data!(WordLike::QuotedWord { .. }))
            | ("SelmahoQuotedWord", data!(WordLike::SelmahoQuotedWord { .. }))
            | ("DelimitedNonLojbanQuote", data!(WordLike::DelimitedNonLojbanQuote { .. }))
            | ("QuotedWords", data!(WordLike::QuotedWords { .. }))
            | ("DelimitedWordQuote", data!(WordLike::DelimitedWordQuote { .. }))
            | ("LerfuWord", data!(WordLike::LerfuWord { .. }))
            | ("ZeiCompound", data!(WordLike::ZeiCompound { .. })) => {}
            ("ReplacementQuote", data!(WordLike::ReplacementQuote { runs, .. })) => {
                assert_eq!(runs.len(), 1, "{input}");
            }
            ("ReplacementQuoteEmpty", data!(WordLike::ReplacementQuote { runs, .. })) => {
                assert!(runs.is_empty(), "{input}");
            }
            ("ReplacementQuoteTwo", data!(WordLike::ReplacementQuote { runs, .. })) => {
                assert_eq!(runs.len(), 2, "{input}");
            }
            _ => panic!("{input} is not {kind}: {word:?}"),
        }
        let view = StdView {
            value: word,
            paint: paint_word,
        };
        assert_eq!(format!("{word:?}"), format!("{view:?}"), "compact {input}");
        assert_eq!(format!("{word:#?}"), format!("{view:#?}"), "pretty {input}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn gismu() -> Word {
        let phonemes = crate::Phonemes::from_canonical("broda".to_owned()).expect("phonemes");
        let span = SourceSpan::new(None, 0, 5, 0, 5).expect("span");
        Word::from_kind(crate::WordKind::Gismu, phonemes, span)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_clone_lerfu_frame_rejects_a_gismu_bu() {
        assert!(
            try_new!(CloneTask::FinishLerfu {
                bu: gismu(),
                base: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_clone_zei_frame_rejects_a_gismu_link() {
        assert!(
            try_new!(CloneTask::FinishZei {
                zei: gismu(),
                right: gismu(),
                left: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_clone_quote_frame_rejects_unequal_marker_and_count_lists() {
        assert!(
            try_new!(CloneTask::FinishQuote {
                lehai: gismu(),
                markers: Vec::new(),
                counts: vec![1],
                got: Vec::new(),
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_map_lerfu_frame_rejects_a_gismu_bu() {
        assert!(try_new!(MapTask::FinishLerfu { bu: gismu() }).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_map_zei_frame_rejects_a_gismu_link() {
        assert!(
            try_new!(MapTask::FinishZei {
                zei: gismu(),
                right: gismu(),
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_map_quote_frame_rejects_a_gismu_lehai() {
        assert!(
            try_new!(MapTask::Quote {
                lehai: gismu(),
                pending: Vec::new(),
                rest: Vec::new(),
                marker: None,
                got: Vec::new(),
                built: Vec::new(),
            })
            .is_err()
        );
    }

    /// The callback order matches the recursive mapper on main.
    /// An error returns at that span, and no later callback runs.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn map_span_order_matches_the_recursive_walk() {
        let samples: &[(&str, &[usize])] = &[
            ("mi bu", &[0, 3]),
            ("mi bu bu", &[0, 3, 6]),
            ("mi bu bu bu", &[0, 3, 6, 9]),
            ("broda zei broda", &[0, 6, 10]),
            ("broda zei broda zei broda", &[0, 6, 10, 16, 20]),
            ("lo'ai mi le'ai", &[0, 6, 9]),
            ("lo'ai mi sa'ai do le'ai", &[0, 6, 9, 15, 18]),
            ("lo'ai y bu le'ai", &[0, 6, 8, 11]),
        ];
        for (input, expected) in samples {
            let word = one_mapped_word(input);
            let (seen, result) = map_visits(word.clone(), None);
            assert!(result.is_ok(), "{input}: {result:?}");
            assert_eq!(seen, *expected, "{input}");
            for (index, start) in expected.iter().enumerate() {
                let (seen, result) = map_visits(word.clone(), Some(index));
                assert_eq!(seen, expected[..=index], "{input} fail {index}");
                assert_eq!(result.expect_err("error"), format!("span {start}"));
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn one_mapped_word(input: &str) -> WordLike {
        let words = raw_words(input);
        assert_eq!(words.len(), 1, "{input} {words:?}");
        words.into_iter().next().expect("one word")
    }

    #[requires(true)]
    #[ensures(true)]
    fn map_visits(
        word: WordLike,
        fail_at: Option<usize>,
    ) -> (Vec<usize>, Result<WordLike, String>) {
        let seen = RefCell::new(Vec::new());
        let result = crate::map_word_like_spans(word, &|span| {
            let mut seen = seen.borrow_mut();
            let index = seen.len();
            seen.push(span.byte_start);
            if fail_at == Some(index) {
                return Err(format!("span {}", span.byte_start));
            }
            Ok(span)
        });
        (seen.into_inner(), result)
    }

    /// `vlasei --format raw` prints `{words:#?}`, or `{words:?}` when the
    /// indent is 0. These files are that output on main. A hand-written
    /// `Debug` that drops the alternate flag fails the pretty file.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_debug_matches_the_derived_form() {
        let samples = [
            (
                "mi bu",
                include_str!("testdata/mi-bu.compact.txt"),
                include_str!("testdata/mi-bu.pretty.txt"),
            ),
            (
                "mi bu bu",
                include_str!("testdata/mi-bu-bu.compact.txt"),
                include_str!("testdata/mi-bu-bu.pretty.txt"),
            ),
            (
                "broda zei broda",
                include_str!("testdata/broda-zei.compact.txt"),
                include_str!("testdata/broda-zei.pretty.txt"),
            ),
            (
                "broda zei broda zei broda",
                include_str!("testdata/broda-zei-zei.compact.txt"),
                include_str!("testdata/broda-zei-zei.pretty.txt"),
            ),
            (
                "mi ui",
                include_str!("testdata/mi-ui.compact.txt"),
                include_str!("testdata/mi-ui.pretty.txt"),
            ),
            (
                "zo mi",
                include_str!("testdata/zo-mi.compact.txt"),
                include_str!("testdata/zo-mi.pretty.txt"),
            ),
            (
                "lo'u mi le'u",
                include_str!("testdata/lohu.compact.txt"),
                include_str!("testdata/lohu.pretty.txt"),
            ),
            (
                "zoi gy. foo gy.",
                include_str!("testdata/zoi.compact.txt"),
                include_str!("testdata/zoi.pretty.txt"),
            ),
            (
                "lo'ai coi le'ai",
                include_str!("testdata/lohai.compact.txt"),
                include_str!("testdata/lohai.pretty.txt"),
            ),
        ];
        for (input, compact, pretty) in samples {
            let words = raw_words(input);
            let got_compact = format!("{words:?}\n");
            let got_pretty = format!("{words:#?}\n");
            assert_eq!(got_compact, compact, "compact {input}");
            assert_eq!(got_pretty, pretty, "pretty {input}");
            assert_ne!(compact, pretty, "{input}");
        }
        let variants = [
            ("mi", "PlainWord"),
            ("zo mi", "QuotedWord"),
            ("ma'oi ba", "SelmahoQuotedWord"),
            ("zoi gy. foo gy.", "DelimitedNonLojbanQuote"),
            ("lo'u mi le'u", "QuotedWords"),
            ("lo'u le'u", "QuotedWords"),
            ("zo'oi broda", "DelimitedWordQuote"),
            ("lo'ai coi le'ai", "ReplacementQuote"),
            ("le'ai", "ReplacementQuoteEmpty"),
            ("lo'ai mi sa'ai do le'ai", "ReplacementQuoteTwo"),
            ("mi bu", "LerfuWord"),
            ("broda zei broda", "ZeiCompound"),
        ];
        for (input, kind) in variants {
            assert_std_word(input, kind);
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn raw_words(input: &str) -> Vec<crate::WordLike> {
        let recovered = crate::segment_words_with_modifiers_recovered_with_options_and_source_id(
            input,
            &crate::MorphologyOptions::default(),
            Some(jbotci_source::SourceId("<input>".to_owned())),
        )
        .into_data();
        assert!(
            recovered.errors.is_empty(),
            "{input} {:?}",
            recovered.errors
        );
        recovered.words
    }
}
