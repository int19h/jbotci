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

/// Static projection of strict and recovered slice elements.
#[bityzba::contract_trait]
pub trait Projection<'tree, S: 'tree, R: 'tree> {
    type View;
    #[requires(true)]
    #[ensures(true)]
    fn strict(value: &'tree S) -> Self::View;
    #[requires(true)]
    #[ensures(true)]
    fn recovered(value: &'tree R) -> Self::View;
}

/// A borrowed slice. Its type selects an inlinable element projection.
#[invariant(true)]
#[invariant(::Strict => true)]
#[invariant(::Recovered => true)]
#[derive(Debug)]
pub enum Sequence<'tree, S, R, V, P> {
    Strict {
        values: &'tree [S],
        marker: std::marker::PhantomData<(V, P)>,
    },
    Recovered {
        values: &'tree [R],
        marker: std::marker::PhantomData<(V, P)>,
    },
}
impl<S, R, V, P> Copy for Sequence<'_, S, R, V, P> {}
impl<S, R, V, P> Clone for Sequence<'_, S, R, V, P> {
    #[requires(true)]
    #[ensures(true)]
    fn clone(&self) -> Self {
        *self
    }
}
impl<'tree, S, R, V, P: Projection<'tree, S, R, View = V>> Sequence<'tree, S, R, V, P> {
    #[requires(true)]
    #[ensures(ret.len() == values.len())]
    pub fn strict(values: &'tree [S]) -> Self {
        Self::Strict {
            values,
            marker: std::marker::PhantomData,
        }
    }
    #[requires(true)]
    #[ensures(ret.len() == values.len())]
    pub fn recovered(values: &'tree [R]) -> Self {
        Self::Recovered {
            values,
            marker: std::marker::PhantomData,
        }
    }
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
    #[inline]
    #[requires(true)]
    #[ensures(ret.is_some() == (index < self.len()))]
    pub fn get(self, index: usize) -> Option<V> {
        match self {
            Self::Strict { values, .. } => values.get(index).map(P::strict),
            Self::Recovered { values, .. } => values.get(index).map(P::recovered),
        }
    }
    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    pub fn iter(self) -> SequenceIter<'tree, S, R, V, P> {
        match self {
            Self::Strict { values, .. } => SequenceIter::Strict {
                iter: values.iter(),
                marker: std::marker::PhantomData,
            },
            Self::Recovered { values, .. } => SequenceIter::Recovered {
                iter: values.iter(),
                marker: std::marker::PhantomData,
            },
        }
    }
}
/// A borrowed iterator with static element projection.
#[invariant(true)]
#[invariant(::Strict => true)]
#[invariant(::Recovered => true)]
#[derive(Debug)]
pub enum SequenceIter<'tree, S: 'tree, R: 'tree, V, P> {
    Strict {
        iter: std::slice::Iter<'tree, S>,
        marker: std::marker::PhantomData<(V, P)>,
    },
    Recovered {
        iter: std::slice::Iter<'tree, R>,
        marker: std::marker::PhantomData<(V, P)>,
    },
}
impl<'tree, S, R, V, P: Projection<'tree, S, R, View = V>> Iterator
    for SequenceIter<'tree, S, R, V, P>
{
    type Item = V;
    #[inline]
    #[requires(true)]
    #[ensures(true)]
    fn next(&mut self) -> Option<V> {
        match self {
            Self::Strict { iter, .. } => iter.next().map(P::strict),
            Self::Recovered { iter, .. } => iter.next().map(P::recovered),
        }
    }
    #[requires(true)]
    #[ensures(ret.1 == Some(ret.0))]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Strict { iter, .. } => iter.size_hint(),
            Self::Recovered { iter, .. } => iter.size_hint(),
        }
    }
}
impl<'tree, S, R, V, P: Projection<'tree, S, R, View = V>> ExactSizeIterator
    for SequenceIter<'tree, S, R, V, P>
{
}
impl<'tree, S, R, V, P: Projection<'tree, S, R, View = V>> IntoIterator
    for Sequence<'tree, S, R, V, P>
{
    type Item = V;
    type IntoIter = SequenceIter<'tree, S, R, V, P>;
    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// The source model supplies its validity guarantee at compile time.
#[bityzba::contract_trait]
pub trait SourceKind {
    const STRICT: bool;
}
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct StrictSource;
#[bityzba::contract_trait]
impl SourceKind for StrictSource {
    const STRICT: bool = true;
}
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct RecoveredSource;
#[bityzba::contract_trait]
impl SourceKind for RecoveredSource {
    const STRICT: bool = false;
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
