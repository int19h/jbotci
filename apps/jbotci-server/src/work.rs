//! Shared work admission for server adapters.
//!
//! Two lanes bound the blocking pool: `compute` (analysis and rendering) and
//! `fetch` (attachment downloads). Each lane has a fixed number of permits and
//! a bounded waiting queue. A permit is *moved into* the blocking worker and
//! released only when the worker exits, so an awaiter that gives up never
//! frees the slot early. When a deadline passes, the server sets that job's
//! cancellation flag. The job stops at its next check and returns, and the
//! permit returns with that exit. The lane stays honest about work that is
//! still running. Fetch transfers are bounded by the time left on their own
//! deadline. Preflight caps still bound input size and result counts.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::time::Instant;

/// Numeric admission limits. `*_workers` is the number of jobs a lane runs at
/// once; `*_queue` is how many further callers may wait for a worker (a
/// caller that finds a worker idle never queues, so zero means "no backlog":
/// idle workers admit, everything else is refused).
///
/// Earlier measurements used a 512 MiB deployment with the embedding model loaded.
/// The idle process used about 380 MiB. A diagram added about 50 MiB.
/// Two analysis workers at the earlier 16-megapixel cap peaked at 616 MiB.
/// One worker at the 8-megapixel cap peaked at 429 MiB.
/// These measurements support the retained defaults. They do not describe current production usage.
/// `docs/discord-app.md` records the workload and build details.
#[invariant(*compute_workers > 0 && *compute_workers <= Semaphore::MAX_PERMITS)]
#[invariant(*fetch_workers > 0 && *fetch_workers <= Semaphore::MAX_PERMITS)]
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

#[invariant(::Overloaded { .. } => true)]
#[invariant(::WaitTimedOut { .. } => true)]
#[invariant(::TimedOut { .. } => true)]
#[invariant(::Panicked { .. } => true)]
#[invariant(::OutputLimit => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkError {
    OutputLimit,
    /// The waiting queue is full; the request was refused without waiting.
    Overloaded {
        lane: WorkLane,
    },
    /// No permit became free before the deadline.
    WaitTimedOut {
        lane: WorkLane,
    },
    /// The worker did not finish before the deadline. The deadline sets the
    /// job's cancellation flag. The worker stops at its next check, then
    /// exits, and only then releases its permit.
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
        formatter.write_str(crate::error::ServerErrorCode::from(self).message())
    }
}

impl std::error::Error for WorkError {}

#[invariant(*capacity > 0 && *capacity <= Semaphore::MAX_PERMITS)]
#[invariant(permits.available_permits() <= *capacity)]
#[invariant(waiting.load(Ordering::Acquire) <= *max_waiting)]
#[derive(Debug)]
struct Lane {
    kind: WorkLane,
    permits: Arc<Semaphore>,
    capacity: usize,
    waiting: AtomicUsize,
    max_waiting: usize,
}

impl Lane {
    #[requires(capacity > 0 && capacity <= Semaphore::MAX_PERMITS)]
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

    #[requires(true)]
    #[ensures(true)]
    async fn acquire(&self, deadline: Instant) -> Result<WorkLease, WorkError> {
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
                self.waiting
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |waiting| {
                        (waiting < self.max_waiting).then(|| waiting + 1)
                    })
                    .map_err(|_| WorkError::Overloaded { lane })?;
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
        Ok(new!(WorkLease {
            permit: Arc::new(permit)
        }))
    }

    /// Run `job` on the blocking pool under one of this lane's permits.
    /// `keepalive` is carried into the worker and dropped when the job ends,
    /// so a caller that gave up waiting does not release anything the running
    /// work still stands for. The job receives a cancellation token. This
    /// function sets that token when `deadline` passes. The permit stays with
    /// the worker until `job` returns.
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
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        let lane = self.kind;
        let permit = self.acquire(deadline).await?;
        // One flag for this job. Dropping the source does not set it, so a
        // job that finishes on time is not cancelled by this value going away.
        let source = jbotci_interrupt::CancellationSource::new();
        let token = source.token();
        let handle = tokio::task::spawn_blocking(move || {
            // The permit and whatever the caller tied to this work live
            // exactly as long as the worker.
            let _permit = permit;
            let _keepalive = keepalive;
            // The blocking pool has a queue of its own, and a caller that
            // already gave up cannot be told about a late result. Starting
            // expensive work here would only burn a worker, so check the
            // deadline once more at the moment the job really starts. A job
            // that has begun keeps the permit until it returns.
            if Instant::now() >= deadline {
                return None;
            }
            let value = job(token.clone());
            trace_permit_release(&token, deadline);
            Some(value)
        });
        match tokio::time::timeout_at(deadline, handle).await {
            Ok(Ok(Some(value))) if Instant::now() < deadline => Ok(value),
            // A result that lands as the deadline passes is still a timeout.
            // Set the flag so a job that has not returned yet stops.
            Ok(Ok(Some(_value))) => {
                source.cancel();
                Err(WorkError::TimedOut { lane })
            }
            Ok(Ok(None)) => Err(WorkError::WaitTimedOut { lane }),
            Ok(Err(_join)) => Err(WorkError::Panicked { lane }),
            Err(_elapsed) => {
                source.cancel();
                Err(WorkError::TimedOut { lane })
            }
        }
    }

    #[cfg(test)]
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

#[cfg(test)]
/// Snapshot of lane occupancy, for diagnostics and tests.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkSnapshot {
    pub(crate) compute_active: usize,
    pub(crate) compute_waiting: usize,
    pub(crate) fetch_active: usize,
    pub(crate) fetch_waiting: usize,
}

/// Admission governor shared by all server adapters.
#[invariant(compute.kind == WorkLane::Compute && fetch.kind == WorkLane::Fetch)]
#[derive(Debug)]
pub(crate) struct WorkGovernor {
    compute: Lane,
    fetch: Lane,
}

impl WorkGovernor {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(limits: WorkLimits) -> Self {
        new!(WorkGovernor {
            compute: Lane::new(
                WorkLane::Compute,
                limits.compute_workers,
                limits.compute_queue,
            ),
            fetch: Lane::new(WorkLane::Fetch, limits.fetch_workers, limits.fetch_queue),
        })
    }

    #[requires(true)]
    #[ensures(ret.compute_workers == self.compute.capacity && ret.fetch_workers == self.fetch.capacity)]
    pub(crate) fn limits(&self) -> WorkLimits {
        new!(WorkLimits {
            compute_workers: self.compute.capacity,
            compute_queue: self.compute.max_waiting,
            fetch_workers: self.fetch.capacity,
            fetch_queue: self.fetch.max_waiting
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn owns_compute(&self, lease: &WorkLease) -> bool {
        Arc::ptr_eq(&self.compute.permits, lease.permit.semaphore())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|lease| self.owns_compute(lease)) || ret.is_err())]
    pub(crate) async fn acquire_compute(&self, deadline: Instant) -> Result<WorkLease, WorkError> {
        self.compute.acquire(deadline).await
    }

    /// Run a CPU-bound analysis/rendering job.
    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_compute<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        self.compute.run(deadline, None, job).await
    }

    /// Run a CPU-bound job that keeps `keepalive` for as long as it runs.
    #[cfg(test)]
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
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        self.compute.run(deadline, keepalive, job).await
    }

    /// Run a blocking network fetch.
    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_fetch<T, F>(&self, deadline: Instant, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
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
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        self.fetch.run(deadline, keepalive, job).await
    }

    #[cfg(test)]
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
    use tokio::sync::oneshot;

    /// Lane for a test job that must enter its closure.
    #[invariant(true)]
    enum GatedLane {
        Compute,
        Fetch,
    }

    /// A submitted job plus the signal from the first line of its closure.
    #[invariant(true)]
    struct GatedJob<T> {
        start_rx: oneshot::Receiver<()>,
        task: tokio::task::JoinHandle<Result<T, WorkError>>,
    }

    /// Wait until the closure has entered.
    /// The paused clock stays still during the wait.
    #[requires(true)]
    #[ensures(true)]
    async fn until_started<T>(job: GatedJob<T>) -> tokio::task::JoinHandle<Result<T, WorkError>> {
        let GatedJob { start_rx, task } = job;
        start_rx
            .await
            .expect("the worker starts before the deadline");
        task
    }

    /// Wait for closure entry, then return the job result.
    #[requires(true)]
    #[ensures(true)]
    async fn finish_gated<T>(job: GatedJob<T>) -> Result<T, WorkError> {
        until_started(job)
            .await
            .await
            .expect("the job task finishes")
    }

    /// A deadline far ahead of the paused clock.
    /// Call this inside a paused runtime.
    /// Advance time only for a timeout case.
    #[requires(true)]
    #[ensures(true)]
    fn far_deadline() -> Instant {
        Instant::now() + Duration::from_secs(3600)
    }

    /// Submit `body` and keep the start signal for the caller.
    /// The signal is the first act of the closure.
    #[requires(true)]
    #[ensures(true)]
    fn open_gated<T, F>(
        governor: &Arc<WorkGovernor>,
        lane: GatedLane,
        deadline: Instant,
        body: F,
    ) -> GatedJob<T>
    where
        T: Send + 'static,
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        let (start_tx, start_rx) = oneshot::channel();
        let governor = Arc::clone(governor);
        let task = tokio::spawn(async move {
            let body = move |token| {
                start_tx
                    .send(())
                    .expect("the test waits for the worker start");
                body(token)
            };
            match lane {
                GatedLane::Compute => governor.run_compute(deadline, body).await,
                GatedLane::Fetch => governor.run_fetch(deadline, body).await,
            }
        });
        GatedJob { start_rx, task }
    }

    /// Submit a compute job.
    /// Call `until_started` or `finish_gated` to wait for closure entry.
    #[requires(true)]
    #[ensures(true)]
    fn gate_compute<T, F>(governor: &Arc<WorkGovernor>, deadline: Instant, body: F) -> GatedJob<T>
    where
        T: Send + 'static,
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        open_gated(governor, GatedLane::Compute, deadline, body)
    }

    /// Submit a fetch job.
    /// Call `until_started` or `finish_gated` to wait for closure entry.
    #[requires(true)]
    #[ensures(true)]
    fn gate_fetch<T, F>(governor: &Arc<WorkGovernor>, deadline: Instant, body: F) -> GatedJob<T>
    where
        T: Send + 'static,
        F: FnOnce(jbotci_interrupt::CancellationToken) -> T + Send + 'static,
    {
        open_gated(governor, GatedLane::Fetch, deadline, body)
    }

    /// Yield until `ready` is true.
    /// Give up after 1000 yields.
    #[requires(true)]
    #[ensures(true)]
    async fn yield_until(mut ready: impl FnMut() -> bool) -> bool {
        for _ in 0..1000 {
            if ready() {
                return true;
            }
            tokio::task::yield_now().await;
        }
        ready()
    }

    /// A paused current-thread runtime with one blocking thread.
    #[requires(true)]
    #[ensures(true)]
    fn paused_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .max_blocking_threads(1)
            .build()
            .expect("runtime")
    }

    /// Run one empty blocking task.
    /// The runtime has one blocking thread.
    /// This task starts after the previous blocking task has stored its result.
    #[requires(true)]
    #[ensures(true)]
    async fn drain_blocking_thread() {
        tokio::task::spawn_blocking(|| ()).await.expect("drain");
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn zero_queue_admits_idle_workers_and_refuses_backlog() {
        let governor = governor(1, 0);
        assert_eq!(
            finish_gated(gate_compute(&governor, far_deadline(), |_| 1)).await,
            Ok(1)
        );
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let busy = until_started(gate_compute(&governor, far_deadline(), move |_| {
            let _ = release_rx.recv();
            2
        }))
        .await;
        assert_eq!(governor.snapshot().compute_active, 1);
        assert_eq!(
            governor.run_compute(far_deadline(), |_| 3).await,
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
        // One blocking thread holds the pool.
        // The second job takes a permit and waits for that thread.
        // The test then moves the paused clock past the second deadline.
        // The caller times out.
        // The freed thread later runs the closure.
        // The closure returns and skips the job body.
        let runtime = paused_runtime();
        let governor = governor(2, 4);
        let ran = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel::<()>();
        runtime.block_on(async {
            let occupied = until_started(gate_compute(&governor, far_deadline(), move |_| {
                release_rx.recv().expect("release");
                "occupied"
            }))
            .await;
            let counter = Arc::clone(&ran);
            let deadline = Instant::now() + Duration::from_secs(1);
            let queued = {
                let governor = Arc::clone(&governor);
                tokio::spawn(async move {
                    governor
                        .run_compute(deadline, move |_| {
                            counter.fetch_add(1, Ordering::SeqCst);
                            "queued"
                        })
                        .await
                })
            };
            assert!(
                yield_until(|| governor.snapshot().compute_active == 2).await,
                "the second permit is taken before its closure runs"
            );
            tokio::time::advance(Duration::from_secs(2)).await;
            assert_eq!(
                queued.await.expect("task"),
                Err(WorkError::TimedOut {
                    lane: WorkLane::Compute
                }),
                "the caller gives up during the wait for a thread"
            );
            release_tx.send(()).expect("worker waiting");
            assert_eq!(occupied.await.expect("task"), Ok("occupied"));
            drain_blocking_thread().await;
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_job_dropped_before_its_closure_starts_reports_wait_timed_out() {
        // A fresh paused runtime with one blocking thread.
        // The second closure waits behind the holder.
        // The clock passes the second deadline.
        // The test leaves that caller unpolled.
        // The holder exits.
        // An empty blocking task then runs.
        // That task starts only after the late closure has stored its result.
        // The caller then observes WaitTimedOut.
        // The job body does not run.
        let runtime = paused_runtime();
        let governor = governor(2, 4);
        let ran = Arc::new(AtomicUsize::new(0));
        let (release_tx, release_rx) = mpsc::channel::<()>();
        runtime.block_on(async {
            let occupied = until_started(gate_compute(&governor, far_deadline(), move |_| {
                release_rx.recv().expect("release");
                "occupied"
            }))
            .await;
            let counter = Arc::clone(&ran);
            let deadline = Instant::now() + Duration::from_secs(1);
            let queued = governor.run_compute(deadline, move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
                "queued"
            });
            tokio::pin!(queued);
            // Poll the caller until it parks on the deadline.
            // The next poll of this caller waits for the late closure to return.
            let mut parked = false;
            for _ in 0..1000 {
                if parked {
                    break;
                }
                tokio::select! {
                    biased;
                    _ = &mut queued => panic!("the queued job finished before its deadline"),
                    _ = tokio::task::yield_now() => {}
                }
                parked = governor.snapshot().compute_active == 2;
            }
            assert!(
                parked,
                "the queued job parks after it takes the second permit"
            );
            tokio::time::advance(Duration::from_secs(2)).await;
            release_tx.send(()).expect("worker waiting");
            assert_eq!(occupied.await.expect("task"), Ok("occupied"));
            drain_blocking_thread().await;
            assert_eq!(
                governor.snapshot().compute_active,
                0,
                "the late closure returned its permit"
            );
            assert_eq!(ran.load(Ordering::SeqCst), 0);
            assert_eq!(
                queued.await,
                Err(WorkError::WaitTimedOut {
                    lane: WorkLane::Compute
                }),
                "the closure returns ahead of the caller poll"
            );
            assert_eq!(ran.load(Ordering::SeqCst), 0, "the job body stays at zero");
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
            .run_compute(Instant::now(), move |_| {
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_timed_out_worker_keeps_its_permit_until_it_exits() {
        // The worker blocks on a channel.
        // The test sends on that channel after the caller returns.
        // The flag is true while the worker still holds the permit.
        // A late flag fails the test.
        let runtime = paused_runtime();
        let governor = governor(1, 4);
        runtime.block_on(async {
            let (start_tx, start_rx) = oneshot::channel();
            let (check_tx, check_rx) = mpsc::channel();
            let (flag_tx, flag_rx) = oneshot::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let deadline = Instant::now() + Duration::from_millis(500);
            let slow = {
                let governor = Arc::clone(&governor);
                tokio::spawn(async move {
                    governor
                        .run_compute(deadline, move |cancellation| {
                            start_tx
                                .send(())
                                .expect("the test waits for the worker start");
                            check_rx.recv().expect("the test asks for the flag");
                            flag_tx
                                .send(cancellation.is_cancelled())
                                .expect("the test reads the flag");
                            let _ = release_rx.recv();
                            "slow"
                        })
                        .await
                })
            };
            start_rx
                .await
                .expect("the worker starts before the deadline");
            tokio::time::advance(Duration::from_secs(1)).await;
            assert_eq!(
                slow.await.expect("task"),
                Err(WorkError::TimedOut {
                    lane: WorkLane::Compute
                })
            );
            check_tx
                .send(())
                .expect("the worker is waiting for the flag check");
            assert!(
                flag_rx.await.expect("the worker sends the flag"),
                "the flag is set before the worker returns the permit"
            );
            assert_eq!(
                governor.snapshot().compute_active,
                1,
                "the permit is still held"
            );

            // The second job waits on the only permit.
            // Its closure must not start.
            // `finish_gated` waits for that start, so this job does not use it.
            let blocked = gate_compute(&governor, far_deadline(), |_| "second");
            assert!(
                yield_until(|| governor.snapshot().compute_waiting == 1).await,
                "the second job waits for the only permit"
            );
            tokio::time::advance(Duration::from_secs(3601)).await;
            let GatedJob {
                start_rx: mut blocked_start,
                task: blocked_task,
            } = blocked;
            assert_eq!(
                blocked_task.await.expect("task"),
                Err(WorkError::WaitTimedOut {
                    lane: WorkLane::Compute
                })
            );
            assert!(
                blocked_start.try_recv().is_err(),
                "the waiting job does not enter its closure"
            );

            release_tx
                .send(())
                .expect("the worker is waiting for release");
            drain_blocking_thread().await;
            assert_eq!(governor.snapshot().compute_active, 0);
            assert_eq!(
                finish_gated(gate_compute(&governor, far_deadline(), |_| "third")).await,
                Ok("third")
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_cancelled_worker_returns_its_permit_shortly_after_the_deadline() {
        // The worker blocks on a channel.
        // The test reads the flag after the caller returns.
        // The worker still holds the permit at that moment.
        // The worker then returns.
        // A late flag fails the test.
        let runtime = paused_runtime();
        let governor = governor(1, 4);
        runtime.block_on(async {
            let (start_tx, start_rx) = oneshot::channel();
            let (check_tx, check_rx) = mpsc::channel();
            let (flag_tx, flag_rx) = oneshot::channel();
            let deadline = Instant::now() + Duration::from_millis(500);
            let result = {
                let governor = Arc::clone(&governor);
                tokio::spawn(async move {
                    governor
                        .run_compute(deadline, move |cancellation| {
                            start_tx
                                .send(())
                                .expect("the test waits for the worker start");
                            check_rx.recv().expect("the test asks for the flag");
                            flag_tx
                                .send(cancellation.is_cancelled())
                                .expect("the test reads the flag");
                            "stopped"
                        })
                        .await
                })
            };
            start_rx
                .await
                .expect("the worker starts before the deadline");
            tokio::time::advance(Duration::from_secs(1)).await;
            assert_eq!(
                result.await.expect("task"),
                Err(WorkError::TimedOut {
                    lane: WorkLane::Compute
                })
            );
            assert_eq!(
                governor.snapshot().compute_active,
                1,
                "the permit is still held"
            );
            check_tx
                .send(())
                .expect("the worker is waiting for the flag check");
            assert!(
                flag_rx.await.expect("the worker sends the flag"),
                "the flag is set before the worker returns the permit"
            );
            drain_blocking_thread().await;
            assert_eq!(governor.snapshot().compute_active, 0);
            assert_eq!(
                finish_gated(gate_compute(&governor, far_deadline(), |_| "second")).await,
                Ok("second")
            );
        });
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn the_waiting_queue_is_bounded() {
        let governor = governor(1, 1);
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder = until_started(gate_compute(&governor, far_deadline(), move |_| {
            let _ = release_rx.recv();
        }))
        .await;
        assert_eq!(governor.snapshot().compute_active, 1);
        let waiter = gate_compute(&governor, far_deadline(), |_| "waited");
        assert!(
            yield_until(|| governor.snapshot().compute_waiting == 1).await,
            "one caller waits for the only permit"
        );
        // The queue holds one waiter. The next arrival is refused at once.
        let refused = governor.run_compute(far_deadline(), |_| "refused").await;
        assert_eq!(
            refused,
            Err(WorkError::Overloaded {
                lane: WorkLane::Compute
            })
        );
        release_tx.send(()).expect("holder waiting");
        assert_eq!(finish_gated(waiter).await, Ok("waited"));
        holder.await.expect("holder task").expect("holder ran");
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

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_aborted_waiter_gives_its_queue_slot_back() {
        let governor = governor(1, 1);
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let holder = until_started(gate_compute(&governor, far_deadline(), move |_| {
            let _ = release_rx.recv();
        }))
        .await;
        assert_eq!(governor.snapshot().compute_active, 1);
        // This body stays out of the closure. The test aborts the waiter
        // during the permit wait.
        let waiter = {
            let governor = Arc::clone(&governor);
            tokio::spawn(async move { governor.run_compute(far_deadline(), |_| "never").await })
        };
        assert!(
            yield_until(|| governor.snapshot().compute_waiting == 1).await,
            "the waiter holds the only queue slot"
        );
        waiter.abort();
        assert!(waiter.await.is_err(), "aborted task reports cancellation");
        assert!(
            yield_until(|| governor.snapshot().compute_waiting == 0).await,
            "queue slot returned"
        );
        let next = gate_compute(&governor, far_deadline(), |_| "admitted");
        // The next job sits in the queue before the holder leaves.
        // A free permit skips the queue.
        assert!(
            yield_until(|| governor.snapshot().compute_waiting == 1).await,
            "the next job waits in the queue"
        );
        release_tx.send(()).expect("holder waiting");
        holder.await.expect("holder task").expect("holder ran");
        assert_eq!(finish_gated(next).await, Ok("admitted"));
        assert_eq!(governor.snapshot().compute_active, 0);
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn panicking_workers_release_their_permit() {
        let governor = governor(1, 1);
        let result = finish_gated(gate_compute(&governor, far_deadline(), |_| -> u8 {
            panic!("deterministic worker failure")
        }))
        .await;
        assert_eq!(
            result,
            Err(WorkError::Panicked {
                lane: WorkLane::Compute
            })
        );
        assert_eq!(governor.snapshot().compute_active, 0);
        assert_eq!(
            finish_gated(gate_fetch(&governor, far_deadline(), |_| 7)).await,
            Ok(7)
        );
    }
}

/// Clones retain one admitted slot. They never admit another operation.
#[invariant(permit.num_permits() == 1)]
#[derive(Debug, Clone)]
pub(crate) struct WorkLease {
    permit: Arc<OwnedSemaphorePermit>,
}

#[invariant(true)]
struct CompletedWork<T> {
    value: T,
    _lease: WorkLease,
}

/// Record how long after `deadline` a cancelled worker released its permit.
///
/// Set `JBOTCI_WORK_TRACE` to print one line. The permit drops when the
/// worker closure returns, which is where this is called. The variable is
/// off unless a measurement run sets it, so production stays quiet.
#[requires(true)]
#[ensures(true)]
fn trace_permit_release(cancellation: &jbotci_interrupt::CancellationToken, deadline: Instant) {
    if cancellation.is_cancelled() && std::env::var_os("JBOTCI_WORK_TRACE").is_some() {
        let late = Instant::now().saturating_duration_since(deadline);
        eprintln!(
            "jbotci work trace: permit release {} ms after deadline",
            late.as_millis()
        );
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn run_admitted<T, F>(
    lease: WorkLease,
    deadline: Instant,
    cancellation: &jbotci_interrupt::CancellationSource,
    job: F,
) -> Result<T, WorkError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    if Instant::now() >= deadline {
        // The sleeper may not have been polled yet. Set the flag before
        // returning so a worker that already started still stops.
        cancellation.cancel();
        return Err(WorkError::TimedOut {
            lane: WorkLane::Compute,
        });
    }
    let token = cancellation.token();
    let flag = cancellation.clone();
    let handle = tokio::task::spawn_blocking(move || {
        if Instant::now() >= deadline {
            flag.cancel();
            None
        } else {
            let value = job();
            trace_permit_release(&token, deadline);
            Some(CompletedWork {
                value,
                _lease: lease,
            })
        }
    });
    match tokio::time::timeout_at(deadline, handle).await {
        // A job that returns while the caller's deadline still has time is a
        // result, even when the flag is already set. Error delivery extends
        // the deadline of a later encode, and that encode must still finish.
        Ok(Ok(Some(completed))) if Instant::now() < deadline => Ok(completed.value),
        Ok(Ok(Some(_completed))) => {
            cancellation.cancel();
            Err(WorkError::TimedOut {
                lane: WorkLane::Compute,
            })
        }
        Ok(Ok(None)) => Err(WorkError::TimedOut {
            lane: WorkLane::Compute,
        }),
        // A panic is a worker failure. A cancellation returns a normal value
        // and does not reach this arm.
        Ok(Err(error)) => {
            eprintln!("server compute worker failed: {error}");
            Err(WorkError::Panicked {
                lane: WorkLane::Compute,
            })
        }
        Err(_) => {
            cancellation.cancel();
            Err(WorkError::TimedOut {
                lane: WorkLane::Compute,
            })
        }
    }
}
