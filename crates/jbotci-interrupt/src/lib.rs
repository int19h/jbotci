//! Cooperative cancellation shared by morphology, syntax, semantics, and rendering.
//!
//! The value those crates load is an atomic flag. It does not read a clock:
//! `std::time::Instant` panics on `wasm32-unknown-unknown`. The owner sets the
//! flag when a deadline passes. A check in a hot loop is one acquire load.
//! Callers that have no deadline hold [`CancellationToken::never`], whose flag
//! stays clear for the life of the process.

use std::fmt;
#[cfg(feature = "cancellation_test_hook")]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};

/// Flag shared by one computation and the owner that may stop it.
///
/// Equality is pointer equality. Two tokens are the same admission only when
/// they share one flag. Every [`CancellationToken::never`] value shares one
/// static flag, so default options stay equal.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
    /// After this many clear loads, store the flag. The load that stores it
    /// still returns clear, so the following load is the one that stops.
    /// Absent on a production token. The field itself is absent unless the
    /// test feature is on, so a production load does not branch.
    #[cfg(feature = "cancellation_test_hook")]
    trip_after: Option<Arc<AtomicUsize>>,
    /// Counts clear loads. Tests read it to learn which load is last.
    #[cfg(feature = "cancellation_test_hook")]
    seen: Option<Arc<AtomicUsize>>,
}

impl PartialEq for CancellationToken {
    #[requires(true)]
    #[ensures(true)]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.flag, &other.flag)
    }
}

impl Eq for CancellationToken {}

impl Default for CancellationToken {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    fn default() -> Self {
        Self::never()
    }
}

impl CancellationToken {
    /// A token that no owner can set.
    ///
    /// The CLI, the browser compute worker, and tests pass this value. Their
    /// results stay the results of an uninterrupted computation.
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    pub fn never() -> Self {
        static FLAG: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));
        Self {
            flag: Arc::clone(&FLAG),
            #[cfg(feature = "cancellation_test_hook")]
            trip_after: None,
            #[cfg(feature = "cancellation_test_hook")]
            seen: None,
        }
    }

    /// A token whose Nth clear load stores the flag after returning clear.
    ///
    /// The next load then reports cancellation. A test picks N so that load is
    /// the checkpoint under test. Removing that load leaves the flag clear for
    /// the return, and the test stops seeing an interrupted result.
    #[cfg(feature = "cancellation_test_hook")]
    #[requires(checks > 0)]
    #[ensures(!ret.is_cancelled())]
    pub fn trip_after_checks(checks: usize) -> Self {
        let mut token = CancellationSource::new().token();
        token.trip_after = Some(Arc::new(AtomicUsize::new(checks)));
        token
    }

    /// A clear token and a counter of its clear `is_cancelled` loads.
    #[cfg(feature = "cancellation_test_hook")]
    #[requires(true)]
    #[ensures(!ret.0.is_cancelled())]
    pub fn counting() -> (Self, Arc<AtomicUsize>) {
        let seen = Arc::new(AtomicUsize::new(0));
        let mut token = CancellationSource::new().token();
        token.seen = Some(Arc::clone(&seen));
        (token, seen)
    }

    /// One acquire load of the shared flag.
    ///
    /// Hot loops call this and nothing else. The load is acquire so it sees a
    /// release store from [`CancellationSource::cancel`]. With the test
    /// feature off, this function is that load and nothing more.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_cancelled(&self) -> bool {
        let cancelled = self.flag.load(Ordering::Acquire);
        #[cfg(feature = "cancellation_test_hook")]
        if !cancelled {
            if let Some(seen) = &self.seen {
                seen.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(remaining) = &self.trip_after {
                let previous = remaining.fetch_sub(1, Ordering::Relaxed);
                if previous == 1 {
                    self.flag.store(true, Ordering::Release);
                }
            }
        }
        cancelled
    }

    /// The interrupted outcome when the flag is set, otherwise success.
    ///
    /// A hot loop that only needs the load calls [`Self::is_cancelled`]. This
    /// helper is for a checkpoint that leaves the function.
    #[requires(true)]
    #[ensures(ret.is_ok() || self.is_cancelled())]
    pub fn check(&self) -> Result<(), Interrupted> {
        if self.is_cancelled() {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

/// Owner of a cancellation flag.
///
/// The server creates one source for each admitted computation and sets it
/// when that computation's deadline passes. Clones share the flag. Dropping a
/// source does not set the flag: a finished owner must not cancel a token that
/// a caller still holds from an earlier clone that outlived this value.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct CancellationSource {
    flag: Arc<AtomicBool>,
}

impl CancellationSource {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_cancelled() || self.is_cancelled())]
    pub fn token(&self) -> CancellationToken {
        CancellationToken {
            flag: Arc::clone(&self.flag),
            #[cfg(feature = "cancellation_test_hook")]
            trip_after: None,
            #[cfg(feature = "cancellation_test_hook")]
            seen: None,
        }
    }

    #[requires(true)]
    #[ensures(self.is_cancelled())]
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

impl Default for CancellationSource {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    fn default() -> Self {
        Self::new()
    }
}

/// A computation stopped because its cancellation flag was set.
///
/// This is not a morphology error and not a syntax error. Callers match it by
/// type. Layers that were cancelled return this value instead of a partial
/// word, parse, or rendering.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("computation interrupted")
    }
}

impl std::error::Error for Interrupted {}

/// Prove that the last clear load turns a late flag into an interrupt.
///
/// `counting` records N on a clear run. `trip_after_checks(N - 1)` must
/// return the interrupted error. `trip_after_checks(N)` must succeed.
/// N includes the constructor load, so a call that never loads the token
/// fails the `N >= 2` assert.
#[cfg(feature = "cancellation_test_hook")]
#[requires(true)]
#[ensures(true)]
pub fn assert_exit_check<T: fmt::Debug, E: fmt::Debug>(
    run: impl Fn(&CancellationToken) -> Result<T, E>,
    interrupted: impl Fn(&E) -> bool,
) -> T {
    let (token, seen) = CancellationToken::counting();
    run(&token).expect("an untripped token finishes");
    let n = seen.load(Ordering::Relaxed);
    assert!(n >= 2, "the call never loaded the token");
    let error = run(&CancellationToken::trip_after_checks(n - 1))
        .expect_err("the last load stops the call");
    assert!(interrupted(&error));
    run(&CancellationToken::trip_after_checks(n)).expect("one more clear load finishes the call")
}

/// Time after a due command deadline.
/// A killed pipe uses this time to deliver stderr.
/// A killed child uses this time to reap.
/// A pipe that stays open hits this bound.
/// A child that does not reap hits this bound.
#[cfg(not(target_arch = "wasm32"))]
pub const READBACK_DRAIN_AFTER_DEADLINE: std::time::Duration = std::time::Duration::from_secs(1);

/// The wait after a command deadline.
/// The result is at least the time that remains.
/// The result is at least [`READBACK_DRAIN_AFTER_DEADLINE`].
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret >= remaining)]
#[ensures(ret >= READBACK_DRAIN_AFTER_DEADLINE)]
pub fn bound_after_deadline(remaining: std::time::Duration) -> std::time::Duration {
    remaining.max(READBACK_DRAIN_AFTER_DEADLINE)
}

/// The time left until one shared drain deadline.
/// The deadline is `start + timeout + READBACK_DRAIN_AFTER_DEADLINE`.
/// The reap and the stderr drain share that deadline.
/// A clock that cannot add the span uses the drain constant.
/// The result is never longer than that span.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret <= timeout.saturating_add(READBACK_DRAIN_AFTER_DEADLINE))]
pub fn drain_window(
    start: std::time::Instant,
    timeout: std::time::Duration,
    now: std::time::Instant,
) -> std::time::Duration {
    let span = timeout.saturating_add(READBACK_DRAIN_AFTER_DEADLINE);
    let Some(deadline) = start.checked_add(span) else {
        return READBACK_DRAIN_AFTER_DEADLINE.min(span);
    };
    if now <= start {
        return span;
    }
    deadline.saturating_duration_since(now).min(span)
}

/// The `kill` argument for a process group.
/// The argument is negative and less than `-1`.
/// A pid of 0, a pid of 1, and a pid that does not fit in `i32` are errors.
/// This function does not send a signal.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(match &ret {
    Ok(argument) => *argument < -1,
    Err(_) => true,
})]
pub fn group_kill_argument(pid: u32) -> std::io::Result<i32> {
    let Ok(pid) = i32::try_from(pid) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the process id does not fit in a group signal",
        ));
    };
    if pid <= 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the process id is not a process group",
        ));
    }
    let Some(argument) = pid.checked_neg() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the process id does not negate",
        ));
    };
    if argument >= -1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the group signal is not a single group",
        ));
    }
    Ok(argument)
}

/// Send `SIGKILL` to the process group of `pid`.
/// The signal argument comes from [`group_kill_argument`].
/// A missing group is success.
/// A non-Unix host does not send a signal.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub fn kill_process_group(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let argument = group_kill_argument(pid)?;
        debug_assert!(argument < -1);
        let rc = unsafe { libc::kill(argument, libc::SIGKILL) };
        if rc == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        return Err(error);
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        Ok(())
    }
}

/// A child the caller can kill and poll.
/// `try_wait` does not block.
#[cfg(not(target_arch = "wasm32"))]
#[contract_trait]
pub trait PollableChild {
    type Status;

    /// Signal the process group.
    /// On Unix the argument is less than `-1`.
    /// A missing group is success.
    /// A pid of 0, a pid of 1, and a pid that does not fit in `i32` are errors.
    #[requires(true)]
    #[ensures(true)]
    fn kill_group(&mut self) -> std::io::Result<()>;

    #[requires(true)]
    #[ensures(true)]
    fn kill(&mut self) -> std::io::Result<()>;

    #[requires(true)]
    #[ensures(true)]
    fn try_wait(&mut self) -> std::io::Result<Option<Self::Status>>;
}

#[cfg(not(target_arch = "wasm32"))]
#[contract_trait]
impl PollableChild for std::process::Child {
    type Status = std::process::ExitStatus;

    #[requires(true)]
    #[ensures(true)]
    fn kill_group(&mut self) -> std::io::Result<()> {
        kill_process_group(std::process::Child::id(self))
    }

    #[requires(true)]
    #[ensures(true)]
    fn kill(&mut self) -> std::io::Result<()> {
        std::process::Child::kill(self)
    }

    #[requires(true)]
    #[ensures(true)]
    fn try_wait(&mut self) -> std::io::Result<Option<Self::Status>> {
        std::process::Child::try_wait(self)
    }
}

/// The result of a bounded reap.
/// `status` is present after the leader is reaped.
/// `error` is a failed kill, a failed poll, or a failed group kill.
/// A group kill that fails after the leader is already reaped keeps both fields.
/// `group_error_after_reap` is set only for that pair.
/// A later poll that reaps the leader drops an earlier error.
/// An unreaped leader can have neither field.
#[cfg(not(target_arch = "wasm32"))]
#[invariant(self.group_error_after_reap == (self.status.is_some() && self.error.is_some()))]
#[derive(Debug)]
pub struct ReapAfterKill<S> {
    pub status: Option<S>,
    pub error: Option<std::io::Error>,
    /// True only when `error` is a group-kill failure after a reaped leader.
    pub group_error_after_reap: bool,
}

/// Kill `child`, then poll `try_wait` until `limit`.
/// A leader that is already reaped keeps that status.
/// The process kill does not run again for that leader.
/// A group kill still runs when `kill_group` is set.
/// A group-kill error stays with the reaped status.
/// A recorded error stays with an unreaped child.
/// A later poll that reaps the child drops that error.
/// The caller leaves an unreaped child unreaped.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub fn reap_after_kill<C: PollableChild>(
    child: &mut C,
    limit: std::time::Duration,
    kill_group: bool,
) -> ReapAfterKill<C::Status> {
    let mut failure = None;
    let reaped = match poll_reap(child) {
        Ok(status) => status,
        Err(error) => {
            failure = Some(error);
            None
        }
    };
    if reaped.is_some() {
        if kill_group {
            failure = merge_reap_error(failure, child.kill_group().err());
        }
        let group_error_after_reap = failure.is_some();
        return new!(ReapAfterKill {
            status: reaped,
            error: failure,
            group_error_after_reap,
        });
    }
    failure = merge_reap_error(failure, signal_for_reap(child, kill_group));
    let start = std::time::Instant::now();
    loop {
        match poll_reap(child) {
            Ok(Some(status)) => {
                return new!(ReapAfterKill {
                    status: Some(status),
                    error: None,
                    group_error_after_reap: false,
                });
            }
            Ok(None) => {}
            Err(error) => failure = merge_reap_error(failure, Some(error)),
        }
        let elapsed = start.elapsed();
        if elapsed >= limit {
            return new!(ReapAfterKill {
                status: None,
                error: failure,
                group_error_after_reap: false,
            });
        }
        let pause = std::time::Duration::from_millis(20).min(limit.saturating_sub(elapsed));
        if pause.is_zero() {
            return new!(ReapAfterKill {
                status: None,
                error: failure,
                group_error_after_reap: false,
            });
        }
        std::thread::sleep(pause);
    }
}

/// One non-blocking poll.
/// `Ok(None)` means the child is still unreaped.
/// An interrupt stays unreaped.
/// Any other poll error is returned.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn poll_reap<C: PollableChild>(child: &mut C) -> std::io::Result<Option<C::Status>> {
    match child.try_wait() {
        Ok(status) => Ok(status),
        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => Ok(None),
        Err(error) => Err(error),
    }
}

/// Send the stop signal.
/// A group kill uses [`PollableChild::kill_group`].
/// The child kill always runs.
/// The first error is kept, and a second error is appended.
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn signal_for_reap<C: PollableChild>(child: &mut C, kill_group: bool) -> Option<std::io::Error> {
    let mut error = None;
    if kill_group && let Err(group_error) = child.kill_group() {
        error = Some(group_error);
    }
    if let Err(kill_error) = child.kill() {
        error = merge_reap_error(error, Some(kill_error));
    }
    error
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn merge_reap_error(
    current: Option<std::io::Error>,
    next: Option<std::io::Error>,
) -> Option<std::io::Error> {
    match (current, next) {
        (None, next) => next,
        (current, None) => current,
        (Some(current), Some(next)) => Some(std::io::Error::other(format!("{current}. {next}"))),
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn never_stays_clear_and_a_source_sets_only_its_own_flag() {
        let never = CancellationToken::never();
        assert!(!never.is_cancelled());
        let source = CancellationSource::new();
        let token = source.token();
        assert!(!token.is_cancelled());
        assert_ne!(token, never);
        source.cancel();
        assert!(token.is_cancelled());
        assert!(source.token().is_cancelled());
        assert!(!CancellationToken::never().is_cancelled());
        assert_eq!(CancellationToken::never(), CancellationToken::default());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_flag_set_on_another_thread_is_visible_to_the_token() {
        let source = CancellationSource::new();
        let token = source.token();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            source.cancel();
        });
        let started = std::time::Instant::now();
        while !token.is_cancelled() {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "the token did not observe the other thread"
            );
            thread::sleep(Duration::from_millis(5));
        }
        handle.join().expect("canceller thread");
    }

    /// A child whose poll never returns a status.
    /// Any field combination is a valid test control.
    #[cfg(not(target_arch = "wasm32"))]
    #[invariant(true)]
    struct StuckChild {
        kill_error: Option<std::io::ErrorKind>,
        kills: u32,
        polls: u32,
        status: Option<u8>,
        group_id: u32,
        group_signals: u32,
        group_argument: Option<i32>,
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl StuckChild {
        #[requires(true)]
        #[ensures(true)]
        fn stuck() -> Self {
            Self {
                kill_error: None,
                kills: 0,
                polls: 0,
                status: None,
                group_id: 1,
                group_signals: 0,
                group_argument: None,
            }
        }

        #[requires(true)]
        #[ensures(true)]
        fn refused() -> Self {
            Self {
                kill_error: Some(std::io::ErrorKind::PermissionDenied),
                kills: 0,
                polls: 0,
                status: None,
                group_id: 1,
                group_signals: 0,
                group_argument: None,
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[contract_trait]
    impl super::PollableChild for StuckChild {
        type Status = u8;

        fn kill_group(&mut self) -> std::io::Result<()> {
            // The model records the argument.
            // It does not call `kill`.
            // A pid of 1 must not become `-1`.
            self.group_signals += 1;
            match super::group_kill_argument(self.group_id) {
                Ok(argument) => {
                    self.group_argument = Some(argument);
                    debug_assert!(argument < -1);
                    Ok(())
                }
                Err(error) => {
                    self.group_argument = None;
                    Err(error)
                }
            }
        }

        fn kill(&mut self) -> std::io::Result<()> {
            self.kills += 1;
            match self.kill_error {
                Some(kind) => Err(std::io::Error::new(kind, "kill failed")),
                None => Ok(()),
            }
        }

        fn try_wait(&mut self) -> std::io::Result<Option<u8>> {
            self.polls += 1;
            Ok(self.status)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reap_returns_while_a_child_never_reaps() {
        let mut child = StuckChild::stuck();
        let limit = Duration::from_millis(100);
        let started = std::time::Instant::now();
        let reap = super::reap_after_kill(&mut child, limit, false);
        let elapsed = started.elapsed();
        assert!(reap.status.is_none(), "the stuck child has no status");
        assert!(reap.error.is_none(), "the kill was accepted");
        assert!(child.kills == 1, "the kill ran once");
        assert!(child.polls > 1, "the reap polled after the kill");
        assert!(
            elapsed < Duration::from_secs(2),
            "the reap ran for {elapsed:?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reap_reports_a_kill_error_inside_the_bound() {
        let mut child = StuckChild::refused();
        let limit = Duration::from_millis(100);
        let started = std::time::Instant::now();
        let reap = super::reap_after_kill(&mut child, limit, false);
        let elapsed = started.elapsed();
        assert!(reap.status.is_none(), "a refused kill does not reap");
        let error = reap.error.as_ref().expect("the kill error is kept");
        assert!(error.to_string().contains("kill failed"), "{error}");
        assert!(
            elapsed < Duration::from_secs(2),
            "the reap ran for {elapsed:?}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reap_keeps_a_status_without_killing_again() {
        let mut child = StuckChild::stuck();
        child.status = Some(7);
        let reap = super::reap_after_kill(&mut child, Duration::from_secs(5), false);
        assert_eq!(reap.status, Some(7));
        assert!(reap.error.is_none());
        assert!(!reap.group_error_after_reap);
        assert_eq!(child.kills, 0, "an exited child is not killed");
        assert_eq!(child.group_signals, 0, "the group kill is off");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_group_id_of_one_does_not_form_a_signal_for_every_process() {
        let mut child = StuckChild::stuck();
        child.status = Some(7);
        let reap = super::reap_after_kill(&mut child, Duration::from_secs(5), true);
        assert_eq!(reap.status, Some(7));
        assert!(reap.group_error_after_reap);
        let error = reap.error.as_ref().expect("the group id is refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("not a process group"), "{error}");
        assert_eq!(child.kills, 0, "an exited child is not killed");
        assert_eq!(child.group_signals, 1);
        assert_eq!(child.group_argument, None);
        assert_ne!(child.group_argument, Some(-1));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_reaped_leader_still_signals_its_group() {
        let mut child = StuckChild::stuck();
        child.group_id = 2;
        child.status = Some(7);
        let reap = super::reap_after_kill(&mut child, Duration::from_secs(5), true);
        assert_eq!(reap.status, Some(7));
        assert!(reap.error.is_none());
        assert!(!reap.group_error_after_reap);
        assert_eq!(child.kills, 0, "an exited child is not killed");
        assert_eq!(child.group_signals, 1);
        assert_eq!(child.group_argument, Some(-2));
        assert!(child.group_argument.is_some_and(|argument| argument < -1));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_group_id_that_does_not_fit_is_refused() {
        for pid in [0_u32, 1, 0x8000_0000, u32::MAX] {
            let error = super::group_kill_argument(pid).expect_err("the id is refused");
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{pid}");
        }
        let argument = super::group_kill_argument(2).expect("pid 2 is a group");
        assert_eq!(argument, -2);
        let wide = super::group_kill_argument(i32::MAX as u32).expect("i32::MAX fits");
        assert_eq!(wide, -i32::MAX);
        assert!(wide < -1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unreaped_bad_group_id_still_kills_the_process() {
        let mut child = StuckChild::stuck();
        let reap = super::reap_after_kill(&mut child, Duration::from_millis(40), true);
        assert!(reap.status.is_none());
        assert!(!reap.group_error_after_reap);
        let error = reap.error.as_ref().expect("the group id is refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(child.kills, 1, "the process kill still runs");
        assert_eq!(child.group_argument, None);
        assert_ne!(child.group_argument, Some(-1));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_drain_window_stays_open_just_before_the_deadline() {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(50);
        let near = start + timeout - Duration::from_micros(20);
        assert_eq!(
            super::drain_window(start, timeout, near),
            super::READBACK_DRAIN_AFTER_DEADLINE + Duration::from_micros(20)
        );
        let at_deadline = start + timeout;
        assert_eq!(
            super::drain_window(start, timeout, at_deadline),
            super::READBACK_DRAIN_AFTER_DEADLINE
        );
        let during_drain = at_deadline + Duration::from_millis(200);
        assert_eq!(
            super::drain_window(start, timeout, during_drain),
            Duration::from_millis(800)
        );
        let drain_deadline = start + timeout + super::READBACK_DRAIN_AFTER_DEADLINE;
        assert_eq!(
            super::drain_window(start, timeout, drain_deadline),
            Duration::ZERO
        );
        assert_eq!(
            super::drain_window(start, timeout, drain_deadline + Duration::from_secs(5)),
            Duration::ZERO
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bound_after_deadline_is_at_least_the_drain_constant() {
        let small = Duration::from_micros(20);
        assert_eq!(
            super::bound_after_deadline(small),
            super::READBACK_DRAIN_AFTER_DEADLINE
        );
        assert!(super::bound_after_deadline(small) >= small);
        assert_eq!(
            super::bound_after_deadline(Duration::ZERO),
            super::READBACK_DRAIN_AFTER_DEADLINE
        );
        assert_eq!(
            super::bound_after_deadline(Duration::from_secs(2)),
            Duration::from_secs(2)
        );
    }
}
