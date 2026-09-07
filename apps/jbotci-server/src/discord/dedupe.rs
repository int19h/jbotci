//! Ingress deduplication of interaction deliveries.
//!
//! Discord identifies every interaction with a snowflake. A redelivered or
//! replayed interaction must not schedule expensive work or produce a second
//! response, so the first thing a handler does is register the id here; only
//! the first registration proceeds. The set is bounded and forgets the oldest
//! ids; correctness after that horizon (or a restart) rests on the published
//! revision and the reconcile-by-reading rule, not on this cache.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use super::request::Snowflake;

#[invariant(order.len() == seen.len() && order.len() <= *capacity)]
#[derive(Debug)]
struct Inner {
    seen: HashSet<Snowflake>,
    order: VecDeque<Snowflake>,
    capacity: usize,
}

#[invariant(true)]
#[derive(Debug)]
pub(crate) struct RecentInteractions {
    inner: Mutex<Inner>,
}

impl RecentInteractions {
    #[requires(capacity > 0)]
    #[ensures(true)]
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(new!(Inner {
                seen: HashSet::new(),
                order: VecDeque::new(),
                capacity,
            })),
        }
    }

    /// Register a delivery. Returns `true` for the first delivery of this id
    /// within the remembered horizon, `false` for a duplicate.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn first_delivery(&self, interaction_id: &Snowflake) -> bool {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if inner.seen.contains(interaction_id) {
            return false;
        }
        let capacity = inner.capacity;
        let mut data = std::mem::replace(
            &mut *inner,
            new!(Inner {
                seen: HashSet::new(),
                order: VecDeque::new(),
                capacity,
            }),
        )
        .into_data();
        if data.order.len() == data.capacity
            && let Some(oldest) = data.order.pop_front()
        {
            data.seen.remove(&oldest);
        }
        data.seen.insert(interaction_id.clone());
        data.order.push_back(interaction_id.clone());
        *inner = Inner::from_data(data);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn id(value: &str) -> Snowflake {
        Snowflake::parse(value).expect("snowflake")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicates_are_recognized_within_the_horizon() {
        let recent = RecentInteractions::new(2);
        assert!(recent.first_delivery(&id("1")));
        assert!(!recent.first_delivery(&id("1")));
        assert!(recent.first_delivery(&id("2")));
        assert!(!recent.first_delivery(&id("2")));
        // Capacity two: registering a third forgets the first.
        assert!(recent.first_delivery(&id("3")));
        assert!(recent.first_delivery(&id("1")));
        assert!(!recent.first_delivery(&id("3")));
    }
}
