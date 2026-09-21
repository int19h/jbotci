//! Ingress deduplication of interaction deliveries.
//!
//! Discord identifies every interaction with a snowflake. A redelivered or
//! replayed interaction must not schedule expensive work or produce a second
//! response, so the first thing a handler does is admit the id here; only the
//! first admission proceeds, and it stays protected until the work it started
//! has settled (the ticket is dropped). Settled ids are remembered in a
//! bounded set that forgets the oldest; correctness after that horizon (or a
//! restart) rests on the published revision and the reconcile-by-reading
//! rule, not on this cache. An active delivery is never evicted: when the set
//! is full of active work, new admission is refused.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use super::request::Snowflake;

#[invariant(settled_order.len() == settled.len())]
#[invariant(active.len() + settled.len() <= *capacity)]
#[invariant(active.iter().all(|id| !settled.contains(id)))]
#[derive(Debug)]
struct Inner {
    /// Deliveries whose work has not settled; never evicted.
    active: HashSet<Snowflake>,
    /// Deliveries whose work settled, oldest first in `settled_order`.
    settled: HashSet<Snowflake>,
    settled_order: VecDeque<Snowflake>,
    capacity: usize,
}

#[invariant(true)]
#[derive(Debug)]
pub(crate) struct RecentInteractions {
    inner: Mutex<Inner>,
}

/// The outcome of admitting a delivery.
#[invariant(::First(_) => true)]
#[derive(Debug)]
pub(crate) enum Admission {
    /// First delivery of this id within the horizon; the ticket keeps the
    /// id protected until dropped.
    First(DeliveryTicket),
    /// The id is active or recently settled.
    Duplicate,
    /// Every remembered id is still active; nothing can be evicted.
    AtCapacity,
}

/// Proof that a delivery is being handled. Dropping it settles the id.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct DeliveryTicket {
    recent: Arc<RecentInteractions>,
    id: Snowflake,
}

impl Drop for DeliveryTicket {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.recent.settle(&self.id);
    }
}

impl RecentInteractions {
    #[requires(capacity > 0)]
    #[ensures(true)]
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(new!(Inner {
                active: HashSet::new(),
                settled: HashSet::new(),
                settled_order: VecDeque::new(),
                capacity,
            })),
        }
    }

    /// Admit a delivery. The ticket of a first admission must live for as
    /// long as the delivery's work and response are in progress.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn admit(self: &Arc<Self>, interaction_id: &Snowflake) -> Admission {
        let mut inner = self.lock();
        if inner.active.contains(interaction_id) || inner.settled.contains(interaction_id) {
            return Admission::Duplicate;
        }
        let mut data = std::mem::replace(&mut *inner, Inner::empty(1)).into_data();
        if data.active.len() + data.settled.len() == data.capacity {
            match data.settled_order.pop_front() {
                Some(oldest) => {
                    data.settled.remove(&oldest);
                }
                None => {
                    *inner = Inner::from_data(data);
                    return Admission::AtCapacity;
                }
            }
        }
        data.active.insert(interaction_id.clone());
        *inner = Inner::from_data(data);
        Admission::First(DeliveryTicket {
            recent: Arc::clone(self),
            id: interaction_id.clone(),
        })
    }

    /// Number of deliveries currently protected.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn active_count(&self) -> usize {
        self.lock().active.len()
    }

    #[requires(true)]
    #[ensures(true)]
    fn settle(&self, interaction_id: &Snowflake) {
        let mut inner = self.lock();
        let mut data = std::mem::replace(&mut *inner, Inner::empty(1)).into_data();
        if data.active.remove(interaction_id) {
            data.settled.insert(interaction_id.clone());
            data.settled_order.push_back(interaction_id.clone());
        }
        *inner = Inner::from_data(data);
    }

    #[requires(true)]
    #[ensures(true)]
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Inner {
    /// A placeholder value swapped in while the real state is being edited
    /// as data; it is never observed.
    #[requires(capacity > 0)]
    #[ensures(true)]
    fn empty(capacity: usize) -> Self {
        new!(Inner {
            active: HashSet::new(),
            settled: HashSet::new(),
            settled_order: VecDeque::new(),
            capacity,
        })
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

    #[requires(true)]
    #[ensures(true)]
    fn first(admission: Admission) -> DeliveryTicket {
        match admission {
            Admission::First(ticket) => ticket,
            other => panic!("expected a first delivery, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn duplicates_are_recognized_while_active_and_after_settling() {
        let recent = Arc::new(RecentInteractions::new(2));
        let ticket = first(recent.admit(&id("1")));
        assert!(matches!(recent.admit(&id("1")), Admission::Duplicate));
        drop(ticket);
        assert_eq!(recent.active_count(), 0);
        assert!(matches!(recent.admit(&id("1")), Admission::Duplicate));
        drop(first(recent.admit(&id("2"))));
        // Capacity two, both settled: a third forgets the oldest.
        drop(first(recent.admit(&id("3"))));
        first(recent.admit(&id("1")));
        assert!(matches!(recent.admit(&id("3")), Admission::Duplicate));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn active_deliveries_survive_a_burst_and_capacity_refuses_rather_than_evicts() {
        let recent = Arc::new(RecentInteractions::new(2));
        // Delivery 1's work is still running (ticket held).
        let ticket_one = first(recent.admit(&id("1")));
        let ticket_two = first(recent.admit(&id("2")));
        // Both remembered ids are active: nothing can be evicted for 3.
        assert!(matches!(recent.admit(&id("3")), Admission::AtCapacity));
        // A replay of 1 during the burst is still a duplicate, so no second
        // expensive run starts before the first publication.
        assert!(matches!(recent.admit(&id("1")), Admission::Duplicate));
        drop(ticket_two);
        // Only the settled delivery is evictable.
        let ticket_three = first(recent.admit(&id("3")));
        assert!(matches!(recent.admit(&id("1")), Admission::Duplicate));
        assert_eq!(recent.active_count(), 2);
        drop(ticket_one);
        drop(ticket_three);
        assert_eq!(recent.active_count(), 0);
    }
}
