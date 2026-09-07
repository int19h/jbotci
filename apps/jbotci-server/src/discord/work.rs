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
use tokio::sync::{Semaphore, TryAcquireError};
use tokio::time::Instant;

/// Numeric admission limits. `*_workers` is the number of jobs a lane runs at
/// once; `*_queue` is how many further callers may wait for a worker (a
/// caller that finds a worker idle never queues, so zero means "no backlog":
/// idle workers admit, everything else is refused).
///
/// The defaults are informed by local workload measurements against the
/// deployed service memory budget rather than guessed: with the embedding
/// model resident the process holds about 380 MiB, and rendering a diagram
/// near its cap costs another 50 while it runs. Two analysis workers at the
/// previous sixteen-megapixel cap peaked at 616 MiB against a 512 MiB budget,
/// so analysis runs one job at a time and the image itself is capped smaller
/// (see `DiagramLimits`); that run peaks at 429 MiB. Network work is cheap and
/// keeps four. `docs/discord-app.md` records the workload and the architecture
/// the figures come from. Tests construct their own.
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
            compute_workers: 1,
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
    /// The single embedding worker thread (meaning search); admitted by
    /// `ToolServices`, reported with the same error type.
    Embedding,
}

impl WorkLane {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Compute => "analysis",
            Self::Fetch => "attachment download",
            Self::Embedding => "meaning search",
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
    /// `keepalive` is carried into the worker and dropped when the job ends,
    /// so a caller that gave up waiting does not release anything the running
    /// work still stands for.
    #[requires(true)]
    #[ensures(true)]
    async fn run<T, F>(
        &self,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        job: F,
    ) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let lane = self.kind;
        // A deadline that has already passed admits no new work, even when a
        // worker is idle: nobody could use the result.
        if Instant::now() >= deadline {
            return Err(WorkError::WaitTimedOut { lane });
        }
        // An idle worker admits immediately; the waiting queue counts only the
        // contenders beyond the idle workers.
        let permit = match Arc::clone(&self.permits).try_acquire_owned() {
            Ok(permit) => permit,
            Err(TryAcquireError::Closed) => return Err(WorkError::Overloaded { lane }),
            Err(TryAcquireError::NoPermits) => {
                // Count ourselves as waiting before touching the semaphore so
                // the queue bound covers the race between many arrivals. The
                // slot is released by RAII: a caller dropped while it waits
                // (aborted task, client gone) gives the slot back the same
                // way a timed-out one does.
                let waiting_before = self.waiting.fetch_add(1, Ordering::AcqRel);
                if waiting_before >= self.max_waiting {
                    self.waiting.fetch_sub(1, Ordering::AcqRel);
                    return Err(WorkError::Overloaded { lane });
                }
                let queue_slot = QueueSlot {
                    counter: &self.waiting,
                };
                let acquired =
                    tokio::time::timeout_at(deadline, Arc::clone(&self.permits).acquire_owned())
                        .await;
                drop(queue_slot);
                match acquired {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(_closed)) => return Err(WorkError::Overloaded { lane }),
                    Err(_elapsed) => return Err(WorkError::WaitTimedOut { lane }),
                }
            }
        };
        // `timeout_at` polls what it wraps before it checks its timer, so a
        // permit that frees at or after the deadline still arrives as `Ok`.
        // Nobody can use that result, so hand the permit straight back.
        if Instant::now() >= deadline {
            return Err(WorkError::WaitTimedOut { lane });
        }
        let handle = tokio::task::spawn_blocking(move || {
            // The permit and whatever the caller tied to this work live
            // exactly as long as the worker.
            let _permit = permit;
            let _keepalive = keepalive;
            // The blocking pool has a queue of its own, and a caller that
            // already gave up cannot be told about a late result. Starting
            // expensive work here would only burn a worker, so check the
            // deadline once more at the moment the job really starts. Work
            // that has begun is never abandoned: the permit stays with it.
            if Instant::now() >= deadline {
                return None;
            }
            Some(job())
        });
        match tokio::time::timeout_at(deadline, handle).await {
            Ok(Ok(Some(value))) => Ok(value),
            Ok(Ok(None)) => Err(WorkError::WaitTimedOut { lane }),
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

/// One occupied place in a lane's waiting queue, released on drop.
#[invariant(true)]
struct QueueSlot<'a> {
    counter: &'a AtomicUsize,
}

impl Drop for QueueSlot<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Something a running job keeps alive: the work has started, so whatever it
/// stands for is still in progress even after its caller stopped waiting.
pub(crate) type WorkKeepalive = Arc<dyn std::any::Any + Send + Sync>;

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
        self.compute.run(deadline, None, job).await
    }

    /// Run a CPU-bound job that keeps `keepalive` for as long as it runs.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_compute_keeping<T, F>(
        &self,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        job: F,
    ) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.compute.run(deadline, keepalive, job).await
    }

    /// Run a blocking network fetch.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_fetch<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.fetch.run(deadline, None, job).await
    }

    /// Run a network job that keeps `keepalive` for as long as it runs, so a
    /// request still in flight keeps standing for its delivery.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_fetch_keeping<T, F>(
        &self,
        deadline: Instant,
        keepalive: Option<WorkKeepalive>,
        job: F,
    ) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.fetch.run(deadline, keepalive, job).await
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn zero_queue_admits_idle_workers_and_refuses_backlog() {
        let governor = governor(1, 0);
        assert_eq!(
            governor
                .run_compute(Instant::now() + Duration::from_secs(1), || 1)
                .await,
            Ok(1)
        );
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let busy_governor = Arc::clone(&governor);
        let busy = tokio::spawn(async move {
            busy_governor
                .run_compute(Instant::now() + Duration::from_secs(5), move || {
                    let _ = release_rx.recv();
                    2
                })
                .await
        });
        for _ in 0..200 {
            if governor.snapshot().compute_active == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(governor.snapshot().compute_active, 1);
        assert_eq!(
            governor
                .run_compute(Instant::now() + Duration::from_secs(1), || 3)
                .await,
            Err(WorkError::Overloaded {
                lane: WorkLane::Compute
            }),
            "no backlog is allowed once the only worker is busy"
        );
        release_tx.send(()).expect("worker waiting");
        assert_eq!(busy.await.expect("task"), Ok(2));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn work_queued_past_its_deadline_never_starts() {
        // One blocking thread, so the second job is stuck in the blocking
        // pool's queue behind the first with no timing race: its caller times
        // out while it waits there. When the thread frees, the job must not
        // run, because its result can no longer reach anyone.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .expect("runtime");
        let governor = governor(2, 4);
        let ran = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel::<()>();
        runtime.block_on(async {
            let occupied = {
                let governor = Arc::clone(&governor);
                let started = Arc::clone(&started);
                tokio::spawn(async move {
                    governor
                        .run_compute(Instant::now() + Duration::from_secs(30), move || {
                            started.fetch_add(1, Ordering::SeqCst);
                            release_rx.recv().expect("release");
                            "occupied"
                        })
                        .await
                })
            };
            // Wait for the occupying job without blocking a worker thread.
            for _ in 0..400 {
                if started.load(Ordering::SeqCst) == 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert_eq!(
                started.load(Ordering::SeqCst),
                1,
                "the only blocking thread is taken"
            );
            let counter = Arc::clone(&ran);
            let queued = governor
                .run_compute(Instant::now() + Duration::from_millis(30), move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                    "queued"
                })
                .await;
            assert_eq!(
                queued,
                Err(WorkError::TimedOut {
                    lane: WorkLane::Compute
                }),
                "the caller gives up while the job waits for a thread"
            );
            release_tx.send(()).expect("worker waiting");
            assert_eq!(occupied.await.expect("task"), Ok("occupied"));
            // Let the freed thread pick up the abandoned job.
            for _ in 0..200 {
                if governor.snapshot().compute_active == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert_eq!(
                governor.snapshot().compute_active,
                0,
                "both permits returned"
            );
            assert_eq!(
                ran.load(Ordering::SeqCst),
                0,
                "the expired job returned without doing its work"
            );
        });
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn an_expired_deadline_admits_no_work_even_with_idle_workers() {
        let governor = governor(2, 4);
        let ran = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&ran);
        let result = governor
            .run_compute(Instant::now(), move || {
                counter.fetch_add(1, Ordering::SeqCst);
                1
            })
            .await;
        assert_eq!(
            result,
            Err(WorkError::WaitTimedOut {
                lane: WorkLane::Compute
            })
        );
        assert_eq!(ran.load(Ordering::SeqCst), 0);
        assert_eq!(governor.snapshot().compute_active, 0);
    }

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
    async fn an_aborted_waiter_gives_its_queue_slot_back() {
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
        for _ in 0..50 {
            if governor.snapshot().compute_active == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let waiter = {
            let governor = Arc::clone(&governor);
            tokio::spawn(async move {
                governor
                    .run_compute(Instant::now() + Duration::from_secs(5), || "never")
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
        // Abort the waiter while it is blocked on the permit.
        waiter.abort();
        assert!(waiter.await.is_err(), "aborted task reports cancellation");
        for _ in 0..50 {
            if governor.snapshot().compute_waiting == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(
            governor.snapshot().compute_waiting,
            0,
            "queue slot returned"
        );
        // The queue is free again for a new arrival.
        let next = {
            let governor = Arc::clone(&governor);
            tokio::spawn(async move {
                governor
                    .run_compute(Instant::now() + Duration::from_secs(5), || "admitted")
                    .await
            })
        };
        release_tx.send(()).expect("holder waiting");
        holder.await.expect("holder task").expect("holder ran");
        assert_eq!(next.await.expect("next task"), Ok("admitted"));
        assert_eq!(governor.snapshot().compute_active, 0);
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
