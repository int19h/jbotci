//! Iterative glue for `ValsiClassification`.
//!
//! A `bu` or `zei` chain is one classification node per link. Clone, equality,
//! and debug stay on a heap stack. `Serialize` keeps the nested field format
//! and calls the serializer once per level.
//! `Drop` lives on `TreeSlot<ValsiClassification>` and `ClassWords`, not on the wrapper.

use std::fmt;

use bityzba::{contract_trait, data, invariant, new, requires};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::tree::{ClassWords, Emit, SlotWalk, TreeSlot};
use crate::{
    PlainWordClassification, ValsiClassification, ValsiClassificationData, WordLike, WordLikeData,
    plain_word_classification,
};

/// A finish frame owns the children it has received.
/// `base` and `left` are absent while that child is still on the stack.
/// `need` is the hole count in `pieces`. `got` stays at most `need`.
#[invariant(::Enter(_) => true)]
#[invariant(::FinishLerfu { suffix, .. } => suffix.category == crate::WordKind::Cmavo)]
#[invariant(::FinishZei { link, right, .. } =>
    link.category == crate::WordKind::Cmavo && !right.phonemes.is_empty())]
#[invariant(::FinishQuote { pieces, need, got } =>
    !pieces.is_empty() && *need <= pieces.len() && got.len() <= *need)]
enum ClassTask<'a> {
    Enter(&'a WordLike),
    FinishLerfu {
        suffix: PlainWordClassification,
        base: Option<ValsiClassification>,
    },
    FinishZei {
        link: PlainWordClassification,
        right: PlainWordClassification,
        left: Option<ValsiClassification>,
    },
    FinishQuote {
        pieces: Vec<ClassPiece>,
        need: usize,
        got: Vec<ValsiClassification>,
    },
    /// Placeholder while a collector frame is updated.
    Slot,
}

#[invariant(true)]
#[invariant(::Ready(_) => true)]
#[invariant(::Hole => true)]
enum ClassPiece {
    Ready(ValsiClassification),
    Hole,
}

/// A finish frame owns the children it has received.
/// `base` and `left` are absent while that child is still on the stack.
/// A quote owns one slot per raw word. `got` stays within that list.
#[invariant(::Enter(_) => true)]
#[invariant(::FinishLerfu { suffix, .. } => suffix.category == crate::WordKind::Cmavo)]
#[invariant(::FinishZei { link, right, .. } =>
    link.category == crate::WordKind::Cmavo && !right.phonemes.is_empty())]
#[invariant(::FinishQuote { pieces, got } => !pieces.is_empty() && got.len() <= pieces.len())]
enum OwnTask<'a, T> {
    Enter(&'a ValsiClassification),
    FinishLerfu {
        suffix: PlainWordClassification,
        base: Option<T>,
    },
    FinishZei {
        link: PlainWordClassification,
        right: PlainWordClassification,
        left: Option<T>,
    },
    FinishQuote {
        pieces: Vec<OwnPiece>,
        got: Vec<T>,
    },
    /// Placeholder while a collector frame is updated.
    Slot,
}

#[invariant(true)]
enum OwnPiece {
    Hole,
}

#[requires(true)]
#[ensures(true)]
fn detach_class(word: ValsiClassification, pending: &mut Vec<ValsiClassification>) {
    match word.into_data() {
        ValsiClassificationData::LerfuWord { mut base, suffix } => {
            drop(suffix);
            if let Some(child) = base.take() {
                pending.push(*child);
            }
        }
        ValsiClassificationData::ZeiCompound {
            mut left,
            link,
            right,
        } => {
            drop(right);
            drop(link);
            if let Some(child) = left.take() {
                pending.push(*child);
            }
        }
        ValsiClassificationData::ReplacementQuote { mut words } => {
            // Push first to last. Pop then yields the last word first.
            // A later word's children sit above the earlier words.
            for word in words.take_all() {
                pending.push(word);
            }
        }
        other => drop(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn walk_class(mut pending: Vec<ValsiClassification>) {
    while let Some(word) = pending.pop() {
        detach_class(word, &mut pending);
    }
}

#[contract_trait]
impl SlotWalk for ValsiClassification {
    type Piece = Self;

    fn into_piece(self) -> Self {
        self
    }

    fn detach_piece(piece: Self, pending: &mut Vec<Self>) {
        detach_class(piece, pending);
    }
}

impl Drop for ClassWords {
    #[requires(true)]
    #[ensures(self.is_empty())]
    fn drop(&mut self) {
        let words = self.take_all();
        if words.is_empty() {
            return;
        }
        walk_class(words);
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn classify(word: &WordLike) -> ValsiClassification {
    let mut stack = vec![new!(ClassTask::Enter(word))];
    let mut open = Vec::new();
    let mut built = Vec::new();
    while let Some(task) = stack.pop() {
        match task.into_data() {
            ClassTaskData::Slot => {}
            ClassTaskData::FinishLerfu { suffix, base } => {
                let base = base.expect("lerfu base");
                deliver_class(
                    &mut stack,
                    &mut open,
                    &mut built,
                    new!(ValsiClassification::LerfuWord {
                        base: TreeSlot::new(base),
                        suffix,
                    }),
                );
            }
            ClassTaskData::FinishZei { link, right, left } => {
                let left = left.expect("zei left");
                deliver_class(
                    &mut stack,
                    &mut open,
                    &mut built,
                    new!(ValsiClassification::ZeiCompound {
                        left: TreeSlot::new(left),
                        link,
                        right,
                    }),
                );
            }
            ClassTaskData::FinishQuote { pieces, got, .. } => {
                let mut kids = got.into_iter();
                let mut words = Vec::with_capacity(pieces.len());
                for piece in pieces {
                    match piece {
                        ClassPiece::Ready(word) => words.push(word),
                        ClassPiece::Hole => words.push(kids.next().expect("quote child")),
                    }
                }
                deliver_class(
                    &mut stack,
                    &mut open,
                    &mut built,
                    new!(ValsiClassification::ReplacementQuote {
                        words: ClassWords::new(words),
                    }),
                );
            }
            ClassTaskData::Enter(word) => push_class(&mut stack, &mut open, word, &mut built),
        }
    }
    built.pop().expect("classification")
}

#[requires(true)]
#[ensures(true)]
fn deliver_class(
    stack: &mut Vec<ClassTask<'_>>,
    open: &mut Vec<usize>,
    built: &mut Vec<ValsiClassification>,
    child: ValsiClassification,
) {
    let Some(&index) = open.last() else {
        built.push(child);
        return;
    };
    let task = std::mem::replace(&mut stack[index], new!(ClassTask::Slot));
    let (task, full) = match task.into_data() {
        ClassTaskData::FinishLerfu { suffix, .. } => (
            new!(ClassTask::FinishLerfu {
                suffix,
                base: Some(child),
            }),
            true,
        ),
        ClassTaskData::FinishZei { link, right, .. } => (
            new!(ClassTask::FinishZei {
                link,
                right,
                left: Some(child),
            }),
            true,
        ),
        ClassTaskData::FinishQuote {
            pieces,
            need,
            mut got,
        } => {
            got.push(child);
            let full = got.len() == need;
            (new!(ClassTask::FinishQuote { pieces, need, got }), full)
        }
        other => (ClassTask::from_data(other), false),
    };
    stack[index] = task;
    if full {
        open.pop();
    }
}

#[requires(true)]
#[ensures(true)]
fn push_class<'a>(
    stack: &mut Vec<ClassTask<'a>>,
    open: &mut Vec<usize>,
    word: &'a WordLike,
    built: &mut Vec<ValsiClassification>,
) {
    match word.as_data() {
        data!(WordLike::PlainWord(word)) => deliver_class(
            stack,
            open,
            built,
            new!(ValsiClassification::PlainWord {
                word: plain_word_classification(word),
            }),
        ),
        data!(WordLike::QuotedWord { zo, word }) => {
            deliver_class(
                stack,
                open,
                built,
                new!(ValsiClassification::QuotedWord {
                    marker: plain_word_classification(zo),
                    quoted_word: plain_word_classification(word),
                }),
            );
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            deliver_class(
                stack,
                open,
                built,
                new!(ValsiClassification::QuotedWord {
                    marker: plain_word_classification(mahoi),
                    quoted_word: plain_word_classification(word),
                }),
            );
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            ..
        }) => {
            deliver_class(
                stack,
                open,
                built,
                new!(ValsiClassification::DelimitedNonLojbanQuote {
                    marker: plain_word_classification(zoi),
                    delimiter: opening_delimiter.phonemes().into_string(),
                }),
            );
        }
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            ..
        }) => {
            deliver_class(
                stack,
                open,
                built,
                new!(ValsiClassification::QuotedWords {
                    marker: plain_word_classification(lohu),
                    quoted_words: quoted_words.iter().map(plain_word_classification).collect(),
                }),
            );
        }
        data!(WordLike::DelimitedWordQuote { marker, .. }) => {
            deliver_class(
                stack,
                open,
                built,
                new!(ValsiClassification::DelimitedWordQuote {
                    marker_text: marker.phonemes().into_string(),
                }),
            );
        }
        data!(WordLike::LerfuWord { base, bu }) => {
            stack.push(new!(ClassTask::FinishLerfu {
                suffix: plain_word_classification(bu),
                base: None,
            }));
            open.push(stack.len() - 1);
            stack.push(new!(ClassTask::Enter(base.as_ref())));
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            stack.push(new!(ClassTask::FinishZei {
                link: plain_word_classification(zei),
                right: plain_word_classification(right),
                left: None,
            }));
            open.push(stack.len() - 1);
            stack.push(new!(ClassTask::Enter(left.as_ref())));
        }
        data!(WordLike::ReplacementQuote { runs, lehai }) => {
            let mut pieces = Vec::new();
            let mut children = Vec::new();
            for run in runs {
                pieces.push(ClassPiece::Ready(new!(ValsiClassification::PlainWord {
                    word: plain_word_classification(&run.marker),
                })));
                for word in &run.words {
                    pieces.push(ClassPiece::Hole);
                    children.push(word.word.as_ref());
                }
            }
            pieces.push(ClassPiece::Ready(new!(ValsiClassification::PlainWord {
                word: plain_word_classification(lehai),
            })));
            let need = children.len();
            stack.push(new!(ClassTask::FinishQuote {
                pieces,
                need,
                got: Vec::new(),
            }));
            if need > 0 {
                open.push(stack.len() - 1);
            }
            for child in children.into_iter().rev() {
                stack.push(new!(ClassTask::Enter(child)));
            }
        }
    }
}

impl Clone for ValsiClassification {
    #[requires(true)]
    #[ensures(true)]
    fn clone(&self) -> Self {
        let mut stack = vec![new!(OwnTask::Enter(self))];
        let mut open = Vec::new();
        let mut built = Vec::new();
        while let Some(task) = stack.pop() {
            match task.into_data() {
                OwnTaskData::Slot => {}
                OwnTaskData::FinishLerfu { suffix, base } => {
                    let base = base.expect("lerfu base");
                    deliver_own(
                        &mut stack,
                        &mut open,
                        &mut built,
                        new!(ValsiClassification::LerfuWord {
                            base: TreeSlot::new(base),
                            suffix,
                        }),
                    );
                }
                OwnTaskData::FinishZei { link, right, left } => {
                    let left = left.expect("zei left");
                    deliver_own(
                        &mut stack,
                        &mut open,
                        &mut built,
                        new!(ValsiClassification::ZeiCompound {
                            left: TreeSlot::new(left),
                            link,
                            right,
                        }),
                    );
                }
                OwnTaskData::FinishQuote { pieces, got } => {
                    let mut kids = got.into_iter();
                    let mut words = Vec::with_capacity(pieces.len());
                    for _piece in pieces {
                        words.push(kids.next().expect("quote child"));
                    }
                    deliver_own(
                        &mut stack,
                        &mut open,
                        &mut built,
                        new!(ValsiClassification::ReplacementQuote {
                            words: ClassWords::new(words),
                        }),
                    );
                }
                OwnTaskData::Enter(word) => push_own(&mut stack, &mut open, word, &mut built),
            }
        }
        built.pop().expect("clone")
    }
}

#[requires(true)]
#[ensures(true)]
fn push_own<'a>(
    stack: &mut Vec<OwnTask<'a, ValsiClassification>>,
    open: &mut Vec<usize>,
    word: &'a ValsiClassification,
    built: &mut Vec<ValsiClassification>,
) {
    match word.as_data() {
        data!(ValsiClassification::LerfuWord { base, suffix }) => {
            push_own_lerfu(stack, open, suffix.clone());
            stack.push(new!(OwnTask::Enter(base.as_ref())));
        }
        data!(ValsiClassification::ZeiCompound { left, link, right }) => {
            push_own_zei(stack, open, link.clone(), right.clone());
            stack.push(new!(OwnTask::Enter(left.as_ref())));
        }
        data!(ValsiClassification::ReplacementQuote { words }) => {
            let pieces = words.iter().map(|_| OwnPiece::Hole).collect();
            push_own_quote(stack, open, pieces);
            for word in words.iter().rev() {
                stack.push(new!(OwnTask::Enter(word)));
            }
        }
        data!(ValsiClassification::PlainWord { word }) => {
            deliver_own(
                stack,
                open,
                built,
                new!(ValsiClassification::PlainWord { word: word.clone() }),
            );
        }
        data!(ValsiClassification::QuotedWord {
            marker,
            quoted_word
        }) => {
            deliver_own(
                stack,
                open,
                built,
                new!(ValsiClassification::QuotedWord {
                    marker: marker.clone(),
                    quoted_word: quoted_word.clone(),
                }),
            );
        }
        data!(ValsiClassification::DelimitedNonLojbanQuote { marker, delimiter }) => {
            deliver_own(
                stack,
                open,
                built,
                new!(ValsiClassification::DelimitedNonLojbanQuote {
                    marker: marker.clone(),
                    delimiter: delimiter.clone(),
                }),
            );
        }
        data!(ValsiClassification::QuotedWords {
            marker,
            quoted_words
        }) => {
            deliver_own(
                stack,
                open,
                built,
                new!(ValsiClassification::QuotedWords {
                    marker: marker.clone(),
                    quoted_words: quoted_words.clone(),
                }),
            );
        }
        data!(ValsiClassification::DelimitedWordQuote { marker_text }) => {
            deliver_own(
                stack,
                open,
                built,
                new!(ValsiClassification::DelimitedWordQuote {
                    marker_text: marker_text.clone(),
                }),
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_own_lerfu<'a, T>(
    stack: &mut Vec<OwnTask<'a, T>>,
    open: &mut Vec<usize>,
    suffix: PlainWordClassification,
) {
    stack.push(new!(OwnTask::FinishLerfu { suffix, base: None }));
    open.push(stack.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn push_own_zei<'a, T>(
    stack: &mut Vec<OwnTask<'a, T>>,
    open: &mut Vec<usize>,
    link: PlainWordClassification,
    right: PlainWordClassification,
) {
    stack.push(new!(OwnTask::FinishZei {
        link,
        right,
        left: None,
    }));
    open.push(stack.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn push_own_quote<'a, T>(
    stack: &mut Vec<OwnTask<'a, T>>,
    open: &mut Vec<usize>,
    pieces: Vec<OwnPiece>,
) {
    let need = pieces.len();
    stack.push(new!(OwnTask::FinishQuote {
        pieces,
        got: Vec::new(),
    }));
    if need > 0 {
        open.push(stack.len() - 1);
    }
}

#[requires(true)]
#[ensures(true)]
fn deliver_own<'a, T>(
    stack: &mut Vec<OwnTask<'a, T>>,
    open: &mut Vec<usize>,
    built: &mut Vec<T>,
    child: T,
) {
    let Some(&index) = open.last() else {
        built.push(child);
        return;
    };
    let task = std::mem::replace(&mut stack[index], new!(OwnTask::Slot));
    let (task, full) = match task.into_data() {
        OwnTaskData::FinishLerfu { suffix, .. } => (
            new!(OwnTask::FinishLerfu {
                suffix,
                base: Some(child),
            }),
            true,
        ),
        OwnTaskData::FinishZei { link, right, .. } => (
            new!(OwnTask::FinishZei {
                link,
                right,
                left: Some(child),
            }),
            true,
        ),
        OwnTaskData::FinishQuote { pieces, mut got } => {
            got.push(child);
            let full = got.len() == pieces.len();
            (new!(OwnTask::FinishQuote { pieces, got }), full)
        }
        other => (OwnTask::from_data(other), false),
    };
    stack[index] = task;
    if full {
        open.pop();
    }
}

impl PartialEq for ValsiClassification {
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

impl Eq for ValsiClassification {}

#[requires(true)]
#[ensures(true)]
fn eq_step<'a>(
    stack: &mut Vec<(&'a ValsiClassification, &'a ValsiClassification)>,
    left: &'a ValsiClassification,
    right: &'a ValsiClassification,
) -> bool {
    match (left.as_data(), right.as_data()) {
        (
            data!(ValsiClassification::PlainWord { word: left }),
            data!(ValsiClassification::PlainWord { word: right }),
        ) => left == right,
        (
            data!(ValsiClassification::QuotedWord {
                marker: left_marker,
                quoted_word: left_word
            }),
            data!(ValsiClassification::QuotedWord {
                marker: right_marker,
                quoted_word: right_word
            }),
        ) => left_marker == right_marker && left_word == right_word,
        (
            data!(ValsiClassification::DelimitedNonLojbanQuote {
                marker: left_marker,
                delimiter: left_delimiter
            }),
            data!(ValsiClassification::DelimitedNonLojbanQuote {
                marker: right_marker,
                delimiter: right_delimiter
            }),
        ) => left_marker == right_marker && left_delimiter == right_delimiter,
        (
            data!(ValsiClassification::QuotedWords {
                marker: left_marker,
                quoted_words: left_words
            }),
            data!(ValsiClassification::QuotedWords {
                marker: right_marker,
                quoted_words: right_words
            }),
        ) => left_marker == right_marker && left_words == right_words,
        (
            data!(ValsiClassification::DelimitedWordQuote { marker_text: left }),
            data!(ValsiClassification::DelimitedWordQuote { marker_text: right }),
        ) => left == right,
        (
            data!(ValsiClassification::LerfuWord {
                base: left_base,
                suffix: left_suffix
            }),
            data!(ValsiClassification::LerfuWord {
                base: right_base,
                suffix: right_suffix
            }),
        ) => {
            if left_suffix != right_suffix {
                return false;
            }
            stack.push((left_base.as_ref(), right_base.as_ref()));
            true
        }
        (
            data!(ValsiClassification::ZeiCompound {
                left: left_left,
                link: left_link,
                right: left_right
            }),
            data!(ValsiClassification::ZeiCompound {
                left: right_left,
                link: right_link,
                right: right_right
            }),
        ) => {
            if left_link != right_link || left_right != right_right {
                return false;
            }
            stack.push((left_left.as_ref(), right_left.as_ref()));
            true
        }
        (
            data!(ValsiClassification::ReplacementQuote { words: left }),
            data!(ValsiClassification::ReplacementQuote { words: right }),
        ) => {
            if left.len() != right.len() {
                return false;
            }
            for (left, right) in left.iter().zip(right.iter()) {
                stack.push((left, right));
            }
            true
        }
        _ => false,
    }
}

impl fmt::Debug for ValsiClassification {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&debug_text(self, formatter.alternate()))
    }
}

#[invariant(::BeginStruct(name) => !name.is_empty())]
#[invariant(::Field(name) => !name.is_empty())]
#[invariant(::Node(_) => true)]
#[invariant(::Leaf(_) => true)]
enum ClassStep<'a> {
    Node(&'a ValsiClassification),
    Leaf(&'a dyn fmt::Debug),
    BeginStruct(&'static str),
    BeginList,
    Field(&'static str),
    EndField,
    Item,
    EndItem,
    EndStruct,
    EndList,
}

#[requires(true)]
#[ensures(true)]
fn apply_class_step<'a>(emit: &mut Emit, stack: &mut Vec<ClassStep<'a>>, step: ClassStep<'a>) {
    match step.as_data() {
        data!(ClassStep::Node(word)) => expand_class(stack, word),
        data!(ClassStep::Leaf(value)) => emit.debug_value(value),
        data!(ClassStep::BeginStruct(name)) => emit.begin_struct(name),
        data!(ClassStep::BeginList) => emit.begin_list(),
        data!(ClassStep::Field(name)) => emit.begin_field(name),
        data!(ClassStep::EndField) => emit.end_field(),
        data!(ClassStep::Item) => emit.begin_item(),
        data!(ClassStep::EndItem) => emit.end_item(),
        data!(ClassStep::EndStruct) => emit.end_struct(),
        data!(ClassStep::EndList) => emit.end_list(),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_class_steps<'a>(stack: &mut Vec<ClassStep<'a>>, steps: Vec<ClassStep<'a>>) {
    stack.extend(steps.into_iter().rev());
}

#[requires(true)]
#[ensures(true)]
fn expand_class<'a>(stack: &mut Vec<ClassStep<'a>>, word: &'a ValsiClassification) {
    match word.as_data() {
        data!(ValsiClassification::PlainWord { word }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("PlainWord")),
                new!(ClassStep::Field("word")),
                new!(ClassStep::Leaf(word)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::QuotedWord {
            marker,
            quoted_word
        }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("QuotedWord")),
                new!(ClassStep::Field("marker")),
                new!(ClassStep::Leaf(marker)),
                new!(ClassStep::EndField),
                new!(ClassStep::Field("quoted_word")),
                new!(ClassStep::Leaf(quoted_word)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::DelimitedNonLojbanQuote { marker, delimiter }) => {
            push_class_steps(
                stack,
                vec![
                    new!(ClassStep::BeginStruct("DelimitedNonLojbanQuote")),
                    new!(ClassStep::Field("marker")),
                    new!(ClassStep::Leaf(marker)),
                    new!(ClassStep::EndField),
                    new!(ClassStep::Field("delimiter")),
                    new!(ClassStep::Leaf(delimiter)),
                    new!(ClassStep::EndField),
                    new!(ClassStep::EndStruct),
                ],
            )
        }
        data!(ValsiClassification::QuotedWords {
            marker,
            quoted_words
        }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("QuotedWords")),
                new!(ClassStep::Field("marker")),
                new!(ClassStep::Leaf(marker)),
                new!(ClassStep::EndField),
                new!(ClassStep::Field("quoted_words")),
                new!(ClassStep::Leaf(quoted_words)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::DelimitedWordQuote { marker_text }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("DelimitedWordQuote")),
                new!(ClassStep::Field("marker_text")),
                new!(ClassStep::Leaf(marker_text)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::LerfuWord { base, suffix }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("LerfuWord")),
                new!(ClassStep::Field("base")),
                new!(ClassStep::Node(base.as_ref())),
                new!(ClassStep::EndField),
                new!(ClassStep::Field("suffix")),
                new!(ClassStep::Leaf(suffix)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::ZeiCompound { left, link, right }) => push_class_steps(
            stack,
            vec![
                new!(ClassStep::BeginStruct("ZeiCompound")),
                new!(ClassStep::Field("left")),
                new!(ClassStep::Node(left.as_ref())),
                new!(ClassStep::EndField),
                new!(ClassStep::Field("link")),
                new!(ClassStep::Leaf(link)),
                new!(ClassStep::EndField),
                new!(ClassStep::Field("right")),
                new!(ClassStep::Leaf(right)),
                new!(ClassStep::EndField),
                new!(ClassStep::EndStruct),
            ],
        ),
        data!(ValsiClassification::ReplacementQuote { words }) => {
            let mut steps = vec![
                new!(ClassStep::BeginStruct("ReplacementQuote")),
                new!(ClassStep::Field("words")),
                new!(ClassStep::BeginList),
            ];
            for word in words.iter() {
                steps.push(new!(ClassStep::Item));
                steps.push(new!(ClassStep::Node(word)));
                steps.push(new!(ClassStep::EndItem));
            }
            steps.push(new!(ClassStep::EndList));
            steps.push(new!(ClassStep::EndField));
            steps.push(new!(ClassStep::EndStruct));
            push_class_steps(stack, steps);
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn debug_text(word: &ValsiClassification, alternate: bool) -> String {
    let mut emit = Emit::new(alternate);
    let mut stack = vec![new!(ClassStep::Node(word))];
    while let Some(step) = stack.pop() {
        apply_class_step(&mut emit, &mut stack, step);
    }
    emit.finish()
}

impl Serialize for ValsiClassification {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let data!(ValsiClassification::ReplacementQuote { words }) = self.as_data() {
            let mut state = serializer.serialize_struct("ValsiClassification", 2)?;
            state.serialize_field("kind", &self.kind())?;
            state.serialize_field("words", words.as_slice())?;
            return state.end();
        }
        let mut state = serializer.serialize_struct("ValsiClassification", 12)?;
        state.serialize_field("kind", &self.kind())?;
        state.serialize_field("word", &self.word())?;
        state.serialize_field("marker", &self.marker())?;
        state.serialize_field("quoted-word", &self.quoted_word())?;
        state.serialize_field("quoted-words", &self.quoted_words())?;
        state.serialize_field("marker-text", &self.marker_text())?;
        state.serialize_field("delimiter", &self.delimiter())?;
        state.serialize_field("base", &self.base())?;
        state.serialize_field("left", &self.left())?;
        state.serialize_field("link", &self.link())?;
        state.serialize_field("right", &self.right())?;
        state.serialize_field("suffix", &self.suffix())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use bityzba::{data, invariant, requires, try_new};

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
    fn paint_class(word: &ValsiClassification, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match word.as_data() {
            data!(ValsiClassification::PlainWord { word }) => formatter
                .debug_struct("PlainWord")
                .field("word", word)
                .finish(),
            data!(ValsiClassification::QuotedWord {
                marker,
                quoted_word,
            }) => formatter
                .debug_struct("QuotedWord")
                .field("marker", marker)
                .field("quoted_word", quoted_word)
                .finish(),
            data!(ValsiClassification::DelimitedNonLojbanQuote { marker, delimiter }) => formatter
                .debug_struct("DelimitedNonLojbanQuote")
                .field("marker", marker)
                .field("delimiter", delimiter)
                .finish(),
            data!(ValsiClassification::QuotedWords {
                marker,
                quoted_words,
            }) => formatter
                .debug_struct("QuotedWords")
                .field("marker", marker)
                .field("quoted_words", quoted_words)
                .finish(),
            data!(ValsiClassification::DelimitedWordQuote { marker_text }) => formatter
                .debug_struct("DelimitedWordQuote")
                .field("marker_text", marker_text)
                .finish(),
            data!(ValsiClassification::ReplacementQuote { words }) => formatter
                .debug_struct("ReplacementQuote")
                .field(
                    "words",
                    &StdView {
                        value: words.as_slice(),
                        paint: paint_class_list,
                    },
                )
                .finish(),
            data!(ValsiClassification::LerfuWord { base, suffix }) => formatter
                .debug_struct("LerfuWord")
                .field(
                    "base",
                    &StdView {
                        value: base.as_ref(),
                        paint: paint_class,
                    },
                )
                .field("suffix", suffix)
                .finish(),
            data!(ValsiClassification::ZeiCompound { left, link, right }) => formatter
                .debug_struct("ZeiCompound")
                .field(
                    "left",
                    &StdView {
                        value: left.as_ref(),
                        paint: paint_class,
                    },
                )
                .field("link", link)
                .field("right", right)
                .finish(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn paint_class_list(
        words: &[ValsiClassification],
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        let mut list = formatter.debug_list();
        for word in words {
            list.entry(&StdView {
                value: word,
                paint: paint_class,
            });
        }
        list.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_std_class(input: &str, kind: &str) {
        let class = one_classification(input);
        match (kind, class.as_data()) {
            ("PlainWord", data!(ValsiClassification::PlainWord { .. }))
            | ("QuotedWord", data!(ValsiClassification::QuotedWord { .. }))
            | (
                "DelimitedNonLojbanQuote",
                data!(ValsiClassification::DelimitedNonLojbanQuote { .. }),
            )
            | ("QuotedWords", data!(ValsiClassification::QuotedWords { .. }))
            | ("DelimitedWordQuote", data!(ValsiClassification::DelimitedWordQuote { .. }))
            | ("LerfuWord", data!(ValsiClassification::LerfuWord { .. }))
            | ("ZeiCompound", data!(ValsiClassification::ZeiCompound { .. }))
            | ("ReplacementQuote", data!(ValsiClassification::ReplacementQuote { .. })) => {}
            _ => panic!("{input} is not {kind}: {class:?}"),
        }
        if kind == "ReplacementQuote" {
            let data!(ValsiClassification::ReplacementQuote { words }) = class.as_data() else {
                panic!("{input} is not ReplacementQuote");
            };
            assert!(!words.is_empty(), "{input}");
        }
        let view = StdView {
            value: &class,
            paint: paint_class,
        };
        assert_eq!(format!("{class:?}"), format!("{view:?}"), "compact {input}");
        assert_eq!(
            format!("{class:#?}"),
            format!("{view:#?}"),
            "pretty {input}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn gismu() -> PlainWordClassification {
        new!(PlainWordClassification {
            category: crate::WordKind::Gismu,
            phonemes: "broda".to_owned(),
            selmaho: None,
            split: None,
            parts: Vec::new(),
            stage: None,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_class_lerfu_frame_rejects_a_gismu_suffix() {
        assert!(
            try_new!(ClassTask::FinishLerfu {
                suffix: gismu(),
                base: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_class_zei_frame_rejects_a_gismu_link() {
        assert!(
            try_new!(ClassTask::FinishZei {
                link: gismu(),
                right: gismu(),
                left: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_class_quote_frame_rejects_a_need_past_its_pieces() {
        assert!(
            try_new!(ClassTask::FinishQuote {
                pieces: vec![ClassPiece::Hole],
                need: 2,
                got: Vec::new(),
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_owned_lerfu_frame_rejects_a_gismu_suffix() {
        assert!(
            try_new!(OwnTask::<'_, ()>::FinishLerfu {
                suffix: gismu(),
                base: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_owned_zei_frame_rejects_a_gismu_link() {
        assert!(
            try_new!(OwnTask::<'_, ()>::FinishZei {
                link: gismu(),
                right: gismu(),
                left: None,
            })
            .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_owned_quote_frame_rejects_an_empty_piece_list() {
        let pieces: Vec<OwnPiece> = Vec::new();
        assert!(
            try_new!(OwnTask::<'_, ()>::FinishQuote {
                pieces,
                got: Vec::new(),
            })
            .is_err()
        );
    }

    /// Raw output prints one classification with `{class:#?}`, or with
    /// `{class:?}` at indent 0. These files are that output on main.
    /// A hand-written `Debug` that drops the alternate flag fails the pretty file.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_debug_matches_the_derived_form() {
        let samples = [
            (
                "mi bu",
                "LerfuWord",
                include_str!("testdata/valsi-mi-bu.compact.txt"),
                include_str!("testdata/valsi-mi-bu.pretty.txt"),
            ),
            (
                "broda zei broda",
                "ZeiCompound",
                include_str!("testdata/valsi-broda-zei.compact.txt"),
                include_str!("testdata/valsi-broda-zei.pretty.txt"),
            ),
            (
                "lo'ai coi le'ai",
                "ReplacementQuote",
                include_str!("testdata/valsi-lohai.compact.txt"),
                include_str!("testdata/valsi-lohai.pretty.txt"),
            ),
        ];
        for (input, kind, compact, pretty) in samples {
            let class = one_classification(input);
            match (kind, class.as_data()) {
                ("LerfuWord", data!(ValsiClassification::LerfuWord { .. }))
                | ("ZeiCompound", data!(ValsiClassification::ZeiCompound { .. }))
                | ("ReplacementQuote", data!(ValsiClassification::ReplacementQuote { .. })) => {}
                _ => panic!("{input} is not {kind}"),
            }
            let got_compact = format!("{class:?}\n");
            let got_pretty = format!("{class:#?}\n");
            assert_ne!(got_compact, got_pretty, "{input}");
            assert_eq!(got_compact, compact, "compact {input}");
            assert_eq!(got_pretty, pretty, "pretty {input}");
        }
        // An empty word list cannot be a replacement quote. The closer of an
        // empty run is still one classified word, so `le'ai` stays legal.
        assert!(
            try_new!(ValsiClassification::ReplacementQuote {
                words: ClassWords::new(Vec::new()),
            })
            .is_err()
        );
        let variants = [
            ("broda", "PlainWord"),
            ("zo mi", "QuotedWord"),
            ("ma'oi ba", "QuotedWord"),
            ("zoi gy. foo gy.", "DelimitedNonLojbanQuote"),
            ("lo'u mi le'u", "QuotedWords"),
            ("lo'u le'u", "QuotedWords"),
            ("zo'oi broda", "DelimitedWordQuote"),
            ("lo'ai coi le'ai", "ReplacementQuote"),
            ("le'ai", "ReplacementQuote"),
            ("lo'ai mi sa'ai do le'ai", "ReplacementQuote"),
            ("mi bu", "LerfuWord"),
            ("broda zei broda", "ZeiCompound"),
        ];
        for (input, kind) in variants {
            assert_std_class(input, kind);
        }
    }

    #[requires(!input.is_empty())]
    #[ensures(true)]
    fn one_classification(input: &str) -> ValsiClassification {
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
        assert_eq!(recovered.words.len(), 1, "{input}");
        classify(&recovered.words[0])
    }
}
