//! Owners for recursive morphology edges.
//!
//! A `TreeSlot` holds one child behind `Option<Box<_>>`.
//! `ClassWords` holds the word list of a classification quote.
//! Each destructor removes every nested edge, then drops the empty owner.
//! The validated word wrappers stay movable and have no `Drop`.
//!
//! A caller can observe an empty slot. `Debug` and `Display` print
//! `TreeSlot(empty)` for that state. `Deref`, `as_ref`, and `Serialize`
//! require a filled slot. `is_empty` and `as_deref` accept both states.

use std::fmt;
use std::ops::Deref;

use bityzba::{contract_trait, invariant, requires};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize, Serializer};

#[contract_trait]
pub trait SlotWalk: Sized {
    type Piece;

    #[requires(true)]
    #[ensures(true)]
    fn into_piece(self) -> Self::Piece;

    #[requires(true)]
    #[ensures(true)]
    fn detach_piece(piece: Self::Piece, pending: &mut Vec<Self::Piece>);
}

#[invariant(true)]
#[derive(Clone, PartialEq, Eq)]
pub struct TreeSlot<T: SlotWalk> {
    slot: Option<Box<T>>,
}

impl<T: SlotWalk> TreeSlot<T> {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn new(value: T) -> Self {
        Self {
            slot: Some(Box::new(value)),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.slot.is_none())]
    pub fn is_empty(&self) -> bool {
        self.slot.is_none()
    }

    #[requires(true)]
    #[ensures(ret.is_some() == !self.is_empty())]
    pub(crate) fn as_deref(&self) -> Option<&T> {
        self.slot.as_deref()
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    pub fn as_ref(&self) -> &T {
        self.slot.as_deref().expect("filled tree slot")
    }

    #[requires(!self.is_empty())]
    #[ensures(true)]
    pub fn into_inner(mut self) -> T {
        *self.take().expect("filled tree slot")
    }

    /// `mem::replace` with `None`. The caller owns the child. This slot stays empty.
    #[requires(true)]
    #[ensures(self.is_empty())]
    pub(crate) fn take(&mut self) -> Option<Box<T>> {
        std::mem::replace(&mut self.slot, None)
    }
}

impl<T: SlotWalk> Deref for TreeSlot<T> {
    type Target = T;

    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: SlotWalk + fmt::Debug> fmt::Debug for TreeSlot<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.slot.as_deref() {
            Some(value) => fmt::Debug::fmt(value, formatter),
            None => formatter.write_str("TreeSlot(empty)"),
        }
    }
}

impl<T: SlotWalk + fmt::Display> fmt::Display for TreeSlot<T> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.slot.as_deref() {
            Some(value) => fmt::Display::fmt(value, formatter),
            None => formatter.write_str("TreeSlot(empty)"),
        }
    }
}

impl<T: SlotWalk + Serialize> Serialize for TreeSlot<T> {
    #[requires(!self.is_empty())]
    #[ensures(true)]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().serialize(serializer)
    }
}

impl<'de, T: SlotWalk + Deserialize<'de>> Deserialize<'de> for TreeSlot<T> {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::new)
    }
}

#[invariant(true)]
#[derive(Clone, PartialEq, Eq)]
pub struct ClassWords {
    words: Vec<crate::ValsiClassification>,
}

impl ClassWords {
    #[requires(true)]
    #[ensures(ret.len() == old(words.len()))]
    pub fn new(words: Vec<crate::ValsiClassification>) -> Self {
        Self { words }
    }

    #[requires(true)]
    #[ensures(ret == self.words.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    #[requires(true)]
    #[ensures(ret == self.words.len())]
    pub fn len(&self) -> usize {
        self.words.len()
    }

    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    pub fn as_slice(&self) -> &[crate::ValsiClassification] {
        &self.words
    }

    /// `mem::replace` with an empty list. The caller owns the words. This owner stays empty.
    #[requires(true)]
    #[ensures(self.is_empty())]
    pub(crate) fn take_all(&mut self) -> Vec<crate::ValsiClassification> {
        std::mem::replace(&mut self.words, Vec::new())
    }
}

impl Deref for ClassWords {
    type Target = [crate::ValsiClassification];

    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    fn deref(&self) -> &[crate::ValsiClassification] {
        self.as_slice()
    }
}

impl<T: SlotWalk> Drop for TreeSlot<T> {
    #[requires(true)]
    #[ensures(self.is_empty())]
    fn drop(&mut self) {
        let Some(node) = self.take() else {
            return;
        };
        let mut pending = vec![T::into_piece(*node)];
        while let Some(piece) = pending.pop() {
            T::detach_piece(piece, &mut pending);
        }
    }
}

impl fmt::Debug for ClassWords {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.words, formatter)
    }
}
