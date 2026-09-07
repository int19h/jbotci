//! Work admission for Discord handlers.
//!
//! Two lanes bound the blocking pool: `compute` (analysis and rendering) and
//! `fetch` (attachment downloads). Each lane has a fixed number of permits and
//! a bounded waiting queue. A permit is *moved into* the blocking worker and
//! released only when the worker exits, so an awaiter that gives up (deadline)
//! never frees the slot early: timing out stops the wait, not the work, and
//! the lane stays honest about how much work is really running. Preflight caps
//! on input size and result counts are what keep individual jobs short.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use tokio::sync::Semaphore;
use tokio::time::Instant;

/// Numeric admission limits. Defaults are the measured operating values for
/// the single-instance service (#902); tests construct their own.
#[invariant(*compute_workers > 0 && *fetch_workers > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkLimits {
    pub(crate) compute_workers: usize,
    pub(crate) compute_queue: usize,
    pub(crate) fetch_workers: usize,
    pub(crate) fetch_queue: usize,
}

impl Default for WorkLimits {
    #[requires(true)]
    #[ensures(ret.compute_workers > 0 && ret.fetch_workers > 0)]
    fn default() -> Self {
        new!(WorkLimits {
            compute_workers: 2,
            compute_queue: 8,
            fetch_workers: 4,
            fetch_queue: 8,
        })
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkLane {
    Compute,
    Fetch,
}

impl WorkLane {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Compute => "analysis",
            Self::Fetch => "attachment download",
        }
    }
}

#[invariant(::Overloaded { .. } => true)]
#[invariant(::WaitTimedOut { .. } => true)]
#[invariant(::TimedOut { .. } => true)]
#[invariant(::Panicked { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkError {
    /// The waiting queue is full; the request was refused without waiting.
    Overloaded {
        lane: WorkLane,
    },
    /// No permit became free before the deadline.
    WaitTimedOut {
        lane: WorkLane,
    },
    /// The worker did not finish before the deadline; it keeps running (and
    /// its permit) until it exits on its own.
    TimedOut {
        lane: WorkLane,
    },
    Panicked {
        lane: WorkLane,
    },
}

impl fmt::Display for WorkError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overloaded { lane } => write!(
                formatter,
                "the server is busy; too many {} requests are queued",
                lane.name()
            ),
            Self::WaitTimedOut { lane } => write!(
                formatter,
                "the server is busy; no {} slot became free in time",
                lane.name()
            ),
            Self::TimedOut { lane } => {
                write!(formatter, "gave up waiting for the {}", lane.name())
            }
            Self::Panicked { lane } => write!(formatter, "the {} failed", lane.name()),
        }
    }
}

impl std::error::Error for WorkError {}

#[invariant(*max_waiting <= 1024)]
#[derive(Debug)]
struct Lane {
    kind: WorkLane,
    permits: Arc<Semaphore>,
    capacity: usize,
    waiting: AtomicUsize,
    max_waiting: usize,
}

impl Lane {
    #[requires(capacity > 0 && max_waiting <= 1024)]
    #[ensures(ret.capacity == capacity)]
    fn new(kind: WorkLane, capacity: usize, max_waiting: usize) -> Self {
        new!(Lane {
            kind,
            permits: Arc::new(Semaphore::new(capacity)),
            capacity,
            waiting: AtomicUsize::new(0),
            max_waiting,
        })
    }

    /// Run `job` on the blocking pool under one of this lane's permits.
    #[requires(true)]
    #[ensures(true)]
    async fn run<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let lane = self.kind;
        // Admission: count ourselves as waiting before touching the semaphore
        // so the queue bound covers the race between many arrivals.
        let waiting_before = self.waiting.fetch_add(1, Ordering::AcqRel);
        if waiting_before >= self.max_waiting {
            self.waiting.fetch_sub(1, Ordering::AcqRel);
            return Err(WorkError::Overloaded { lane });
        }
        let acquired =
            tokio::time::timeout_at(deadline, Arc::clone(&self.permits).acquire_owned()).await;
        self.waiting.fetch_sub(1, Ordering::AcqRel);
        let permit = match acquired {
            Ok(Ok(permit)) => permit,
            Ok(Err(_closed)) => return Err(WorkError::Overloaded { lane }),
            Err(_elapsed) => return Err(WorkError::WaitTimedOut { lane }),
        };
        let handle = tokio::task::spawn_blocking(move || {
            // The permit lives exactly as long as the worker.
            let _permit = permit;
            job()
        });
        match tokio::time::timeout_at(deadline, handle).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_join)) => Err(WorkError::Panicked { lane }),
            Err(_elapsed) => Err(WorkError::TimedOut { lane }),
        }
    }

    #[requires(true)]
    #[ensures(ret <= self.capacity)]
    fn active(&self) -> usize {
        self.capacity - self.permits.available_permits()
    }
}

/// Snapshot of lane occupancy, for diagnostics and tests.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkSnapshot {
    pub(crate) compute_active: usize,
    pub(crate) compute_waiting: usize,
    pub(crate) fetch_active: usize,
    pub(crate) fetch_waiting: usize,
}

/// Admission governor shared by all Discord handlers.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct WorkGovernor {
    compute: Lane,
    fetch: Lane,
}

impl WorkGovernor {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(limits: WorkLimits) -> Self {
        Self {
            compute: Lane::new(
                WorkLane::Compute,
                limits.compute_workers,
                limits.compute_queue,
            ),
            fetch: Lane::new(WorkLane::Fetch, limits.fetch_workers, limits.fetch_queue),
        }
    }

    /// Run a CPU-bound analysis/rendering job.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_compute<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.compute.run(deadline, job).await
    }

    /// Run a blocking network fetch.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_fetch<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.fetch.run(deadline, job).await
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn snapshot(&self) -> WorkSnapshot {
        WorkSnapshot {
            compute_active: self.compute.active(),
            compute_waiting: self.compute.waiting.load(Ordering::Acquire),
            fetch_active: self.fetch.active(),
            fetch_waiting: self.fetch.waiting.load(Ordering::Acquire),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[requires(true)]
    #[ensures(true)]
    fn governor(workers: usize, queue: usize) -> Arc<WorkGovernor> {
        Arc::new(WorkGovernor::new(new!(WorkLimits {
            compute_workers: workers,
            compute_queue: queue,
            fetch_workers: 1,
            fetch_queue: 1,
        })))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_timed_out_worker_keeps_its_permit_until_it_exits() {
        let governor = governor(1, 4);
        let (release_tx, release_rx) = mpsc::channel::<()>();
        // The worker blocks until told to finish.
        let slow = governor.run_compute(Instant::now() + Duration::from_millis(50), move || {
            let _ = release_rx.recv();
            "slow"
        });
        assert_eq!(
            slow.await,
            Err(WorkError::TimedOut {
                lane: WorkLane::Compute
            })
        );
        assert_eq!(
            governor.snapshot().compute_active,
            1,
            "the permit is still held"
        );

        // A second job cannot start while the abandoned worker holds the only
        // permit: it waits until its own deadline and reports that.
        let blocked = governor
            .run_compute(Instant::now() + Duration::from_millis(50), || "second")
            .await;
        assert_eq!(
            blocked,
            Err(WorkError::WaitTimedOut {
                lane: WorkLane::Compute
            })
        );

        release_tx.send(()).expect("worker still waiting");
        // Once the worker exits the permit returns and new work runs.
        let mut ran = Err(WorkError::TimedOut {
            lane: WorkLane::Compute,
        });
        for _ in 0..50 {
            ran = governor
                .run_compute(Instant::now() + Duration::from_millis(200), || "third")
                .await;
            if ran.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(ran, Ok("third"));
        assert_eq!(governor.snapshot().compute_active, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn the_waiting_queue_is_bounded() {
        let governor = governor(1, 1);
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder = {
            let governor = Arc::clone(&governor);
            tokio::spawn(async move {
                governor
                    .run_compute(Instant::now() + Duration::from_secs(5), move || {
                        let _ = release_rx.recv();
                    })
                    .await
            })
        };
        // Let the holder take the permit.
        for _ in 0..50 {
            if governor.snapshot().compute_active == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(governor.snapshot().compute_active, 1);
        let waiter = {
            let governor = Arc::clone(&governor);
            tokio::spawn(async move {
                governor
                    .run_compute(Instant::now() + Duration::from_secs(5), || "waited")
                    .await
            })
        };
        for _ in 0..50 {
            if governor.snapshot().compute_waiting == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(governor.snapshot().compute_waiting, 1);
        // The queue holds one waiter; the next arrival is refused at once.
        let refused = governor
            .run_compute(Instant::now() + Duration::from_secs(5), || "refused")
            .await;
        assert_eq!(
            refused,
            Err(WorkError::Overloaded {
                lane: WorkLane::Compute
            })
        );
        release_tx.send(()).expect("holder waiting");
        holder.await.expect("holder task").expect("holder ran");
        assert_eq!(waiter.await.expect("waiter task"), Ok("waited"));
        assert_eq!(
            governor.snapshot(),
            WorkSnapshot {
                compute_active: 0,
                compute_waiting: 0,
                fetch_active: 0,
                fetch_waiting: 0,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn panicking_workers_release_their_permit() {
        let governor = governor(1, 1);
        let result = governor
            .run_compute(Instant::now() + Duration::from_secs(5), || -> u8 {
                panic!("deterministic worker failure")
            })
            .await;
        assert_eq!(
            result,
            Err(WorkError::Panicked {
                lane: WorkLane::Compute
            })
        );
        assert_eq!(governor.snapshot().compute_active, 0);
        assert_eq!(
            governor
                .run_fetch(Instant::now() + Duration::from_secs(5), || 7)
                .await,
            Ok(7)
        );
    }
}
