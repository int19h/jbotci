//! Borrowed slots and sequences for generated read-only model views.

use bityzba::{invariant, requires};

/// A borrowed child selection. Only `Valid` supplies a valid child.
#[invariant(true)]
#[invariant(::Valid(_) => true)]
#[invariant(::Prefix(_) => true)]
#[derive(Debug, Clone, Copy)]
pub enum Slot<V> {
    Valid(V),
    Prefix(V),
    Error,
}

impl<V: Copy> Slot<V> {
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Valid(_)))]
    pub fn valid(self) -> Option<V> {
        match self {
            Self::Valid(value) => Some(value),
            Self::Prefix(_) | Self::Error => None,
        }
    }
}

/// A slice and its projection function. Projection never owns an element.
#[invariant(true)]
#[invariant(::Strict => true)]
#[invariant(::Recovered => true)]
#[derive(Debug)]
pub enum Sequence<'tree, S, R, V> {
    Strict {
        values: &'tree [S],
        project: fn(&'tree S) -> V,
    },
    Recovered {
        values: &'tree [R],
        project: fn(&'tree R) -> V,
    },
}

impl<S, R, V> Copy for Sequence<'_, S, R, V> {}
impl<S, R, V> Clone for Sequence<'_, S, R, V> {
    #[requires(true)]
    #[ensures(true)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'tree, S, R, V> Sequence<'tree, S, R, V> {
    #[requires(true)]
    #[ensures(true)]
    pub fn len(self) -> usize {
        match self {
            Self::Strict { values, .. } => values.len(),
            Self::Recovered { values, .. } => values.len(),
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.len() == 0))]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (index < self.len()))]
    pub fn get(self, index: usize) -> Option<V> {
        match self {
            Self::Strict { values, project } => values.get(index).map(project),
            Self::Recovered { values, project } => values.get(index).map(project),
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    pub fn iter(self) -> SequenceIter<'tree, S, R, V> {
        match self {
            Self::Strict { values, project } => SequenceIter::Strict(values.iter().map(project)),
            Self::Recovered { values, project } => {
                SequenceIter::Recovered(values.iter().map(project))
            }
        }
    }
}

/// An iterator that projects one borrowed element at a time.
#[invariant(true)]
#[invariant(::Strict(_) => true)]
#[invariant(::Recovered(_) => true)]
#[derive(Debug)]
pub enum SequenceIter<'tree, S, R, V> {
    Strict(std::iter::Map<std::slice::Iter<'tree, S>, fn(&'tree S) -> V>),
    Recovered(std::iter::Map<std::slice::Iter<'tree, R>, fn(&'tree R) -> V>),
}

impl<S, R, V> Iterator for SequenceIter<'_, S, R, V> {
    type Item = V;
    #[requires(true)]
    #[ensures(true)]
    fn next(&mut self) -> Option<V> {
        match self {
            Self::Strict(iter) => iter.next(),
            Self::Recovered(iter) => iter.next(),
        }
    }
    #[requires(true)]
    #[ensures(ret.1 == Some(ret.0))]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Strict(iter) => iter.size_hint(),
            Self::Recovered(iter) => iter.size_hint(),
        }
    }
}
impl<S, R, V> ExactSizeIterator for SequenceIter<'_, S, R, V> {}

impl<'tree, S, R, V> IntoIterator for Sequence<'tree, S, R, V> {
    type Item = V;
    type IntoIter = SequenceIter<'tree, S, R, V>;
    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A chain view with the original first element and link sequence.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct ChainView<E, L> {
    pub first: E,
    pub links: L,
}

/// A word or node view with its original trailing modifier sequence.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct WithFreeModifiersView<T, F> {
    pub value: T,
    pub free_modifiers: F,
}
