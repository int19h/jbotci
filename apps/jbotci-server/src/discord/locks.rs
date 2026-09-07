//! Per-message coordination.
//!
//! Every update of a published message runs under that message's lock so a
//! slower, older submission cannot overwrite a newer one and so the
//! read-revision / compute / publish / reconcile sequence is atomic within
//! this instance. The registry keeps exactly one lock identity per message
//! for as long as anyone holds or waits for it: entries are reference counted
//! by holders plus waiters and reclaimed under the registry mutex only when
//! that count reaches zero, so a late waiter can never end up on a second
//! mutex for the same message. Admission is bounded per message and overall.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use tokio::sync::OwnedMutexGuard;
use tokio::time::Instant;

use super::request::Snowflake;

#[invariant(true)]
#[derive(Debug)]
struct Entry {
    mutex: Arc<tokio::sync::Mutex<()>>,
    /// Holders plus waiters currently referencing this entry.
    interested: usize,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockError {
    /// Too many submissions are already queued for this message.
    Busy,
    /// Too many messages are being updated at once.
    Overloaded,
    /// The current holder did not finish before the deadline.
    WaitTimedOut,
}

impl fmt::Display for LockError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy => formatter.write_str(
                "another change to this result is still being applied; try again in a moment",
            ),
            Self::Overloaded => {
                formatter.write_str("the server is applying too many changes right now; try again")
            }
            Self::WaitTimedOut => {
                formatter.write_str("the previous change to this result took too long; try again")
            }
        }
    }
}

impl std::error::Error for LockError {}

/// Registry of per-message locks.
#[invariant(*max_entries > 0 && *max_waiters_per_message > 0)]
#[derive(Debug)]
pub(crate) struct MessageLocks {
    entries: Mutex<HashMap<Snowflake, Entry>>,
    max_entries: usize,
    max_waiters_per_message: usize,
}

impl MessageLocks {
    #[requires(max_entries > 0 && max_waiters_per_message > 0)]
    #[ensures(true)]
    pub(crate) fn new(max_entries: usize, max_waiters_per_message: usize) -> Self {
        new!(MessageLocks {
            entries: Mutex::new(HashMap::new()),
            max_entries,
            max_waiters_per_message,
        })
    }

    /// Acquire the lock for `message_id`, waiting at most until `deadline`.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn acquire(
        self: &Arc<Self>,
        message_id: &Snowflake,
        deadline: Instant,
    ) -> Result<MessageGuard, LockError> {
        let mutex = {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(entry) = entries.get_mut(message_id) {
                // One holder plus the allowed waiters.
                if entry.interested > self.max_waiters_per_message {
                    return Err(LockError::Busy);
                }
                entry.interested += 1;
                Arc::clone(&entry.mutex)
            } else {
                if entries.len() >= self.max_entries {
                    return Err(LockError::Overloaded);
                }
                let mutex = Arc::new(tokio::sync::Mutex::new(()));
                entries.insert(
                    message_id.clone(),
                    Entry {
                        mutex: Arc::clone(&mutex),
                        interested: 1,
                    },
                );
                mutex
            }
        };
        // Interest is released by RAII from here on: a waiter that times out
        // or is dropped mid-wait releases it exactly like a finished holder.
        let interest = InterestGuard {
            registry: Arc::clone(self),
            message_id: message_id.clone(),
        };
        match tokio::time::timeout_at(deadline, mutex.lock_owned()).await {
            Ok(guard) => Ok(MessageGuard {
                _guard: guard,
                _interest: interest,
            }),
            Err(_elapsed) => Err(LockError::WaitTimedOut),
        }
    }

    /// Drop one holder/waiter reference and reclaim the entry when it was the
    /// last one.
    #[requires(true)]
    #[ensures(true)]
    fn release_interest(&self, message_id: &Snowflake) {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let remove = match entries.get_mut(message_id) {
            Some(entry) => {
                entry.interested = entry.interested.saturating_sub(1);
                entry.interested == 0
            }
            None => false,
        };
        if remove {
            entries.remove(message_id);
        }
    }

    /// Number of messages with a live lock entry (diagnostics/tests).
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn live_entries(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

/// One holder's or waiter's reference to a message entry, released on drop.
#[invariant(true)]
#[derive(Debug)]
struct InterestGuard {
    registry: Arc<MessageLocks>,
    message_id: Snowflake,
}

impl Drop for InterestGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.registry.release_interest(&self.message_id);
    }
}

/// Exclusive access to one message's update sequence. Fields drop in order:
/// the mutex is released first (waking the next waiter, which holds its own
/// interest), then this holder's interest.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct MessageGuard {
    _guard: OwnedMutexGuard<()>,
    _interest: InterestGuard,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[requires(true)]
    #[ensures(true)]
    fn message(id: &str) -> Snowflake {
        Snowflake::parse(id).expect("snowflake")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn three_overlapping_submissions_serialize_on_one_identity() {
        let locks = Arc::new(MessageLocks::new(16, 4));
        let concurrent = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let locks = Arc::clone(&locks);
            let concurrent = Arc::clone(&concurrent);
            let peak = Arc::clone(&peak);
            tasks.push(tokio::spawn(async move {
                let guard = locks
                    .acquire(&message("42"), Instant::now() + Duration::from_secs(5))
                    .await
                    .expect("lock acquired");
                let now = concurrent.fetch_add(1, Ordering::AcqRel) + 1;
                peak.fetch_max(now, Ordering::AcqRel);
                tokio::time::sleep(Duration::from_millis(30)).await;
                concurrent.fetch_sub(1, Ordering::AcqRel);
                assert_eq!(
                    locks.live_entries(),
                    1,
                    "one identity while anyone is interested"
                );
                drop(guard);
            }));
        }
        for task in tasks {
            task.await.expect("task");
        }
        assert_eq!(peak.load(Ordering::Acquire), 1, "never two holders at once");
        assert_eq!(
            locks.live_entries(),
            0,
            "entry reclaimed after the last holder"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_waiter_that_times_out_leaves_the_holder_and_entry_intact() {
        let locks = Arc::new(MessageLocks::new(16, 4));
        let holder = locks
            .acquire(&message("7"), Instant::now() + Duration::from_secs(5))
            .await
            .expect("holder");
        let waiter = locks
            .acquire(&message("7"), Instant::now() + Duration::from_millis(40))
            .await;
        assert_eq!(waiter.err(), Some(LockError::WaitTimedOut));
        assert_eq!(locks.live_entries(), 1, "the holder still owns the entry");
        // A later arrival waits on the same identity and gets it after the holder.
        let later = {
            let locks = Arc::clone(&locks);
            tokio::spawn(async move {
                locks
                    .acquire(&message("7"), Instant::now() + Duration::from_secs(5))
                    .await
                    .map(|_guard| ())
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(holder);
        assert_eq!(later.await.expect("task"), Ok(()));
        assert_eq!(locks.live_entries(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn admission_is_bounded_per_message_and_overall() {
        let locks = Arc::new(MessageLocks::new(2, 1));
        let _holder = locks
            .acquire(&message("1"), Instant::now() + Duration::from_secs(5))
            .await
            .expect("holder");
        let waiting = {
            let locks = Arc::clone(&locks);
            tokio::spawn(async move {
                locks
                    .acquire(&message("1"), Instant::now() + Duration::from_millis(200))
                    .await
                    .map(|_| ())
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        // One waiter is allowed; a second is refused immediately.
        assert_eq!(
            locks
                .acquire(&message("1"), Instant::now() + Duration::from_secs(5))
                .await
                .err(),
            Some(LockError::Busy)
        );
        let _other = locks
            .acquire(&message("2"), Instant::now() + Duration::from_secs(5))
            .await
            .expect("second message");
        assert_eq!(
            locks
                .acquire(&message("3"), Instant::now() + Duration::from_secs(5))
                .await
                .err(),
            Some(LockError::Overloaded)
        );
        assert_eq!(waiting.await.expect("task"), Err(LockError::WaitTimedOut));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn aborting_a_blocked_waiter_reclaims_the_entry_after_the_holder_exits() {
        let locks = Arc::new(MessageLocks::new(16, 4));
        let holder = locks
            .acquire(&message("11"), Instant::now() + Duration::from_secs(5))
            .await
            .expect("holder");
        let waiter = {
            let locks = Arc::clone(&locks);
            tokio::spawn(async move {
                locks
                    .acquire(&message("11"), Instant::now() + Duration::from_secs(5))
                    .await
                    .map(|_| ())
            })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        waiter.abort();
        assert!(waiter.await.is_err());
        assert_eq!(locks.live_entries(), 1, "the holder still owns the entry");
        drop(holder);
        assert_eq!(locks.live_entries(), 0, "aborted interest was released");
        let again = locks
            .acquire(&message("11"), Instant::now() + Duration::from_secs(5))
            .await;
        assert!(again.is_ok(), "the message is admitted again");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[requires(true)]
    #[ensures(true)]
    async fn cancelling_a_waiter_releases_its_interest() {
        let locks = Arc::new(MessageLocks::new(16, 4));
        let holder = locks
            .acquire(&message("9"), Instant::now() + Duration::from_secs(5))
            .await
            .expect("holder");
        let waiter = {
            let locks = Arc::clone(&locks);
            tokio::spawn(async move {
                let _ = locks
                    .acquire(&message("9"), Instant::now() + Duration::from_millis(30))
                    .await;
            })
        };
        waiter.await.expect("waiter finished by its own deadline");
        drop(holder);
        assert_eq!(locks.live_entries(), 0);
    }
}
