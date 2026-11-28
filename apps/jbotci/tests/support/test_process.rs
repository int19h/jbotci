//! Own a test child and the threads that read its pipes.
//!
//! `Drop` reaps the child and joins those threads.
//! The same cleanup runs on success and on unwind.
//! A second call does nothing harmful.
//! Each integration test crate uses the part it needs.

#![allow(dead_code)]

use std::io;
use std::process::{Child, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

/// A child process and the threads that read its pipes.
///
/// The child stays stored after it is reaped.
/// Its process id does not change.
#[invariant(true)]
pub(crate) struct TestProcess {
    child: Child,
    readers: Vec<JoinHandle<()>>,
    /// Set only after `join` returns for the readers owned at cleanup.
    joined: Arc<AtomicBool>,
    /// Set only after `wait` reaps the child.
    reaped: Arc<AtomicBool>,
}

impl TestProcess {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child,
            readers: Vec::new(),
            joined: Arc::new(AtomicBool::new(false)),
            reaped: Arc::new(AtomicBool::new(false)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn add_reader(&mut self, reader: JoinHandle<()>) {
        self.readers.push(reader);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn id(&self) -> u32 {
        self.child.id()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn joined_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.joined)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn reaped_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.reaped)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped.store(true, Ordering::SeqCst);
        }
        Ok(status)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = self.child.wait()?;
        self.reaped.store(true, Ordering::SeqCst);
        Ok(status)
    }

    /// Kill the child and wait for it.
    ///
    /// The caller uses this when the test is finished with a live server.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn kill_and_wait(&mut self) {
        self.child.kill().expect("kill server");
        if self.child.wait().is_ok() {
            self.reaped.store(true, Ordering::SeqCst);
        }
    }

    /// Join every reader.
    ///
    /// A reader panic fails the caller.
    #[requires(true)]
    #[ensures(self.readers.is_empty())]
    pub(crate) fn join_readers_strict(&mut self) {
        let readers = std::mem::take(&mut self.readers);
        for reader in readers {
            reader.join().expect("LSP stdout reader finishes");
        }
        self.joined.store(true, Ordering::SeqCst);
    }

    #[requires(true)]
    #[ensures(self.readers.is_empty())]
    pub(crate) fn join_readers_quiet(&mut self) {
        let readers = std::mem::take(&mut self.readers);
        for reader in readers {
            let _ = reader.join();
        }
        self.joined.store(true, Ordering::SeqCst);
    }

    /// Reap the child when this owner has not reaped it, then join the readers.
    ///
    /// `try_wait` stores the status after the first reap.
    /// A kill after that reap can hit a pid that the OS reused.
    /// Join after the reap.
    /// A join before the reap can wait forever on a blocked read.
    /// Errors are ignored so cleanup during unwind does not panic again.
    #[requires(true)]
    #[ensures(self.readers.is_empty())]
    pub(crate) fn finish_quiet(&mut self) {
        self.reap_quiet();
        self.join_readers_quiet();
    }

    #[requires(true)]
    #[ensures(true)]
    fn reap_quiet(&mut self) {
        match self.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = self.child.kill();
                if self.child.wait().is_ok() {
                    self.reaped.store(true, Ordering::SeqCst);
                }
            }
            Err(_) => {
                if self.child.wait().is_ok() {
                    self.reaped.store(true, Ordering::SeqCst);
                }
            }
        }
    }
}

impl Drop for TestProcess {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.finish_quiet();
    }
}

/// Close an input and join the reader thread that parses it.
///
/// The two-frame test has no child process.
/// `finish` reports a reader panic.
/// `Drop` ignores a join error so unwind does not panic again.
#[invariant(true)]
pub(crate) struct ReaderGuard {
    close_input: Option<Box<dyn FnOnce() + Send>>,
    reader: Option<JoinHandle<()>>,
}

impl ReaderGuard {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(input: impl Send + 'static, reader: JoinHandle<()>) -> Self {
        Self {
            close_input: Some(Box::new(move || drop(input))),
            reader: Some(reader),
        }
    }

    #[requires(true)]
    #[ensures(self.reader.is_none())]
    pub(crate) fn finish(&mut self) {
        self.close_and_join(true);
    }

    #[requires(true)]
    #[ensures(self.reader.is_none())]
    fn close_and_join(&mut self, strict: bool) {
        if let Some(close_input) = self.close_input.take() {
            close_input();
        }
        if let Some(reader) = self.reader.take() {
            let joined = reader.join();
            if strict {
                joined.expect("the reader thread finishes");
            }
        }
    }
}

impl Drop for ReaderGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        // `finish` already closed the input when the test passed.
        self.close_and_join(false);
    }
}
