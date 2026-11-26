//! Exercise installed-index updates through the same setup entry point as the CLI and desktop.
//
// Deadline, wake, and teardown rules for these tests.
// A deadline is checked before a result is accepted.
// Every sleep or wait stops at the remaining time.
// An exit first observed after the deadline is a timeout.
// Every pending operation uses that same deadline.
// An interrupt keeps the byte offset and the bytes already read.
// A waiter reads the milestone and the worker result under one mutex.
// That mutex is the mutex of the condition variable.
// Teardown joins or awaits every task before it reports another error.
// Cancellation is a normal stop.
// A panic keeps its message.
// A receive loop accepts end of file or the expected reset as closure.
// Any other socket error reports its cause.

use super::*;
use std::io::{Seek, SeekFrom};
use std::net::TcpListener;
use std::sync::Condvar;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread::JoinHandle;

#[requires(true)]
#[ensures(true)]
fn setup_test_dir() -> tempfile::TempDir {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tempfile::tempdir_in(root).unwrap()
}

const PACK_SOCKET_POLL: std::time::Duration = std::time::Duration::from_millis(100);
const PACK_HANG_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

/// One read from the pack server socket.
/// `Line` keeps the bytes through the newline, or through end of file.
/// That text is not empty.
/// `End` means the client closed the socket before the next byte.
/// `Stopped` means the stop flag is true.
/// `Broken` carries the socket error text.
/// That text is not empty.
#[invariant(true)]
#[invariant(::Line(text) => !text.is_empty())]
#[invariant(::Broken(message) => !message.is_empty())]
enum PackRead {
    Line(String),
    End,
    Stopped,
    Broken(String),
}

/// One write to the pack server socket.
/// `Done` means every byte was written.
/// `Stopped` means the stop flag is true.
/// `Broken` carries the socket error text.
/// That text is not empty.
#[invariant(true)]
#[invariant(::Broken(message) => !message.is_empty())]
enum PackWrite {
    Done,
    Stopped,
    Broken(String),
}

/// The text of a socket error.
/// The text is not empty.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn socket_error_text(error: &std::io::Error) -> String {
    let message = error.to_string();
    if !message.is_empty() {
        return message;
    }
    let kind = format!("{:?}", error.kind());
    if !kind.is_empty() {
        return kind;
    }
    "socket error".to_owned()
}

#[requires(true)]
#[ensures(true)]
fn pack_io_is_a_stall(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock || error.kind() == std::io::ErrorKind::TimedOut
}

/// Read one request line.
/// An interrupt returns to this loop.
/// The stop flag and the hang limit are checked on every pass.
/// The clock is read after each read returns.
/// A byte or end of file at the hang limit or later is a hang.
/// A stall is tried again while the stop flag is false.
/// A stall of 60 seconds with a false stop flag fails the hang limit.
/// The failure text says that this limit only stops a hang.
/// A true stop flag ends the read.
#[requires(true)]
#[ensures(true)]
fn read_pack_line(stream: &mut impl std::io::Read, stop: &AtomicBool) -> PackRead {
    read_pack_line_for(stream, stop, PACK_HANG_LIMIT)
}

/// Read one request line with an explicit hang limit.
/// The clock is read after each read returns.
/// A byte or end of file at the limit or later is a hang.
#[requires(!limit.is_zero())]
#[ensures(true)]
fn read_pack_line_for(
    stream: &mut impl std::io::Read,
    stop: &AtomicBool,
    limit: std::time::Duration,
) -> PackRead {
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    let mut progressed = std::time::Instant::now();
    loop {
        if stop.load(AtomicOrdering::Acquire) {
            return new!(PackRead::Stopped);
        }
        if progressed.elapsed() >= limit {
            panic!("this limit only stops a hang");
        }
        match std::io::Read::read(stream, &mut byte) {
            Ok(0) => {
                if progressed.elapsed() >= limit {
                    panic!("this limit only stops a hang");
                }
                if line.is_empty() {
                    return new!(PackRead::End);
                }
                return new!(PackRead::Line(String::from_utf8_lossy(&line).into_owned(),));
            }
            Ok(_) => {
                if progressed.elapsed() >= limit {
                    panic!("this limit only stops a hang");
                }
                progressed = std::time::Instant::now();
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    return new!(PackRead::Line(String::from_utf8_lossy(&line).into_owned(),));
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::Interrupted
                    || pack_io_is_a_stall(&error) =>
            {
                continue;
            }
            Err(error) => return new!(PackRead::Broken(socket_error_text(&error))),
        }
    }
}

/// Milestones and the worker result for one pack server.
/// One mutex holds every value that a waiter tests.
#[invariant(true)]
struct PackProgress {
    accept_ready: bool,
    requests: Vec<String>,
    response_started: bool,
    write_stalled: bool,
    worker_result: Option<Result<(), String>>,
}

/// A progress record with no milestones and no worker result.
#[requires(true)]
#[ensures(
    !ret.accept_ready
        && ret.requests.is_empty()
        && !ret.response_started
        && !ret.write_stalled
        && ret.worker_result.is_none()
)]
fn fresh_pack_progress() -> PackProgress {
    PackProgress {
        accept_ready: false,
        requests: Vec::new(),
        response_started: false,
        write_stalled: false,
        worker_result: None,
    }
}

/// Write every byte.
/// The socket timeout is 100 milliseconds.
/// An interrupt leaves the byte offset unchanged.
/// The stop flag and the hang limit are checked on every pass.
/// The clock is read after each write returns.
/// A positive write at the hang limit or later is a hang.
/// A stall is tried again while the stop flag is false.
/// The first stall before the stop flag records that the write blocked.
/// A stall of 60 seconds with a false stop flag fails the hang limit.
/// The failure text says that this limit only stops a hang.
/// A true stop flag ends the write.
#[requires(true)]
#[ensures(true)]
fn write_pack_bytes(
    stream: &mut impl std::io::Write,
    bytes: &[u8],
    stop: &AtomicBool,
    progress: &Mutex<PackProgress>,
    wake: &Condvar,
) -> PackWrite {
    write_pack_bytes_for(stream, bytes, stop, progress, wake, PACK_HANG_LIMIT)
}

/// Write every byte with an explicit hang limit.
/// The clock is read after each write returns.
/// A positive write at the limit or later is a hang.
#[requires(!limit.is_zero())]
#[ensures(true)]
fn write_pack_bytes_for(
    stream: &mut impl std::io::Write,
    bytes: &[u8],
    stop: &AtomicBool,
    progress: &Mutex<PackProgress>,
    wake: &Condvar,
    limit: std::time::Duration,
) -> PackWrite {
    let mut offset = 0;
    let mut progressed = std::time::Instant::now();
    while offset < bytes.len() {
        if stop.load(AtomicOrdering::Acquire) {
            return new!(PackWrite::Stopped);
        }
        if progressed.elapsed() >= limit {
            panic!("this limit only stops a hang");
        }
        match std::io::Write::write(stream, &bytes[offset..]) {
            Ok(0) => {
                return new!(PackWrite::Broken(
                    "the pack server write returned no bytes".to_owned(),
                ));
            }
            Ok(count) => {
                if progressed.elapsed() >= limit {
                    panic!("this limit only stops a hang");
                }
                offset += count;
                progressed = std::time::Instant::now();
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if pack_io_is_a_stall(&error) => {
                if !stop.load(AtomicOrdering::Acquire) {
                    record_pack_write_stall(progress, wake);
                }
                continue;
            }
            Err(error) => return new!(PackWrite::Broken(socket_error_text(&error))),
        }
    }
    new!(PackWrite::Done)
}

/// Record that a write stalled before the stop flag.
/// The first record wakes a test that waits for it.
#[requires(true)]
#[ensures(true)]
fn record_pack_write_stall(progress: &Mutex<PackProgress>, wake: &Condvar) {
    let mut guard = progress.lock().expect("the pack server stall flag is open");
    if !guard.write_stalled {
        guard.write_stalled = true;
        wake.notify_all();
    }
}

/// Record that the response header write is about to start.
/// The first record wakes a test that waits for it.
#[requires(true)]
#[ensures(true)]
fn record_pack_response_start(progress: &Mutex<PackProgress>, wake: &Condvar) {
    let mut guard = progress
        .lock()
        .expect("the pack server response flag is open");
    if !guard.response_started {
        guard.response_started = true;
        wake.notify_all();
    }
}

/// Read bytes until the response header ends.
/// The header ends at a blank line.
/// The socket timeout is 100 milliseconds.
/// An interrupt returns to this loop.
/// The hang limit is checked on every pass.
/// A stall is tried again for the same limit.
/// The failure text says that this limit only stops a hang.
/// End of file returns the bytes read so far.
#[requires(!limit.is_zero())]
#[ensures(true)]
fn read_pack_response_header_for(
    stream: &mut impl std::io::Read,
    limit: std::time::Duration,
) -> String {
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 1];
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() >= limit {
            panic!("this limit only stops a hang");
        }
        match std::io::Read::read(stream, &mut buf) {
            Ok(0) => {
                if started.elapsed() >= limit {
                    panic!("this limit only stops a hang");
                }
                return String::from_utf8_lossy(&bytes).into_owned();
            }
            Ok(_) => {
                if started.elapsed() >= limit {
                    panic!("this limit only stops a hang");
                }
                bytes.push(buf[0]);
                if bytes.ends_with(b"\r\n\r\n") {
                    return String::from_utf8_lossy(&bytes).into_owned();
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::Interrupted
                    || pack_io_is_a_stall(&error) =>
            {
                continue;
            }
            Err(error) => panic!("the pack response header failed: {error}"),
        }
    }
}

/// Read the response header from a pack client socket.
/// The hang limit is 60 seconds.
#[requires(true)]
#[ensures(true)]
fn read_pack_response_header(stream: &mut std::net::TcpStream) -> String {
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() >= PACK_HANG_LIMIT {
            panic!("this limit only stops a hang");
        }
        let remaining = PACK_HANG_LIMIT.saturating_sub(started.elapsed());
        let poll = PACK_SOCKET_POLL.min(remaining);
        if poll.is_zero() {
            panic!("this limit only stops a hang");
        }
        match stream.set_read_timeout(Some(poll)) {
            Ok(()) => break,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("the pack client accepts a read timeout: {error}"),
        }
    }
    let remaining = PACK_HANG_LIMIT.saturating_sub(started.elapsed());
    if remaining.is_zero() {
        panic!("this limit only stops a hang");
    }
    read_pack_response_header_for(stream, remaining)
}

/// Read one file chunk.
/// An interrupt returns to this loop.
/// The stop flag is checked on every pass.
/// `Ok(None)` means the stop flag is true.
#[requires(true)]
#[ensures(true)]
fn read_pack_file_chunk(
    file: &mut impl std::io::Read,
    buf: &mut [u8],
    stop: &AtomicBool,
) -> Result<Option<usize>, String> {
    loop {
        if stop.load(AtomicOrdering::Acquire) {
            return Ok(None);
        }
        match file.read(buf) {
            Ok(count) => return Ok(Some(count)),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("the pack server file read failed: {error}")),
        }
    }
}

/// Bind one local port.
/// An interrupt retries the same port.
/// An address that is already in use advances to the next port.
/// Any other error names the address and the cause.
#[requires(start <= end)]
#[ensures(true)]
fn select_local_port<T>(
    start: u16,
    end: u16,
    mut bind_port: impl FnMut(u16) -> std::io::Result<T>,
) -> Result<T, String> {
    let mut port = start;
    loop {
        if port > end {
            return Err(format!("no free port in {start}..={end}"));
        }
        match bind_port(port) {
            Ok(value) => return Ok(value),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                port = match port.checked_add(1) {
                    Some(next) => next,
                    None => return Err(format!("no free port in {start}..={end}")),
                };
            }
            Err(error) => return Err(format!("cannot bind 127.0.0.1:{port}: {error}")),
        }
    }
}

/// Apply one socket call.
/// An interrupt returns to this loop.
/// The stop flag is checked on every pass.
/// `Ok(false)` means the stop flag is true.
#[requires(true)]
#[ensures(true)]
fn call_until_stop(
    stop: &AtomicBool,
    mut operation: impl FnMut() -> std::io::Result<()>,
) -> Result<bool, std::io::Error> {
    loop {
        if stop.load(AtomicOrdering::Acquire) {
            return Ok(false);
        }
        match operation() {
            Ok(()) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

/// A reader that returns Interrupted before each byte.
/// The index is never past the end of the byte slice.
#[invariant(self.index <= self.bytes.len())]
#[derive(Clone)]
struct InterruptBeforeEachByte {
    bytes: &'static [u8],
    index: usize,
    interrupt: bool,
}

impl std::io::Read for InterruptBeforeEachByte {
    #[requires(true)]
    #[ensures(
        ret.as_ref()
            .map(|count| *count <= 1)
            .unwrap_or_else(|error| error.kind() == std::io::ErrorKind::Interrupted)
    )]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.interrupt {
            *self = self.clone().with_data(data! { interrupt: false });
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "injected interrupt",
            ));
        }
        if self.index >= self.bytes.len() {
            return Ok(0);
        }
        let byte = self.bytes[self.index];
        let index = self.index + 1;
        *self = self.clone().with_data(data! {
            index: index,
            interrupt: true,
        });
        buf[0] = byte;
        Ok(1)
    }
}

/// A reader that fails once with a reset.
#[invariant(true)]
struct ResetOnce {
    calls: usize,
}

impl std::io::Read for ResetOnce {
    #[requires(true)]
    #[ensures(ret.is_err())]
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        self.calls += 1;
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "injected reset",
        ))
    }
}

/// A reader that sets the stop flag and then always returns Interrupted.
#[invariant(true)]
struct StopThenInterrupt {
    stop: Arc<AtomicBool>,
    calls: usize,
}

impl std::io::Read for StopThenInterrupt {
    #[requires(true)]
    #[ensures(ret.is_err())]
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        self.calls += 1;
        self.stop.store(true, AtomicOrdering::Release);
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "injected interrupt",
        ))
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_line_returns_to_the_stop_flag_on_persistent_interrupts() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut reader = StopThenInterrupt {
        stop: Arc::clone(&stop),
        calls: 0,
    };
    match read_pack_line(&mut reader, &stop).into_data() {
        data!(PackRead::Stopped) => {}
        _ => panic!("the pack line returns to the stop flag"),
    }
    assert!(reader.calls < 5, "calls={}", reader.calls);
}

/// A reader that always returns Interrupted.
#[invariant(true)]
struct AlwaysInterrupt {
    calls: usize,
}

impl std::io::Read for AlwaysInterrupt {
    #[requires(true)]
    #[ensures(ret.is_err())]
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        self.calls += 1;
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "injected interrupt",
        ))
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_response_header_reaches_a_100_ms_limit_on_persistent_interrupts() {
    let mut reader = AlwaysInterrupt { calls: 0 };
    let started = std::time::Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_pack_response_header_for(&mut reader, std::time::Duration::from_millis(100));
    }));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    let message = panic_payload_text(result.expect_err("the hang limit fires"));
    assert!(
        message.contains("this limit only stops a hang"),
        "{message}"
    );
    assert!(reader.calls > 0);
}

/// One scripted write result.
/// `Bytes` carries a positive count.
#[invariant(true)]
#[invariant(::Bytes(count) => *count > 0)]
enum PackWriteStep {
    Interrupt,
    Bytes(usize),
}

/// A writer that follows a script.
#[invariant(true)]
struct ScriptedPackWrite {
    steps: Vec<PackWriteStep>,
    cursor: usize,
    written: Vec<u8>,
}

impl std::io::Write for ScriptedPackWrite {
    #[requires(true)]
    #[ensures(true)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let copied = match self.steps[self.cursor].as_data() {
            data!(PackWriteStep::Interrupt) => None,
            data!(PackWriteStep::Bytes(count)) => Some(*count),
        };
        self.cursor += 1;
        let Some(count) = copied else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "injected interrupt",
            ));
        };
        let count = count.min(buf.len());
        self.written.extend_from_slice(&buf[..count]);
        Ok(count)
    }

    #[requires(true)]
    #[ensures(true)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn write_pack_bytes_keeps_the_offset_across_interrupts() {
    let mut writer = ScriptedPackWrite {
        steps: vec![
            new!(PackWriteStep::Interrupt),
            new!(PackWriteStep::Bytes(1)),
            new!(PackWriteStep::Interrupt),
            new!(PackWriteStep::Bytes(3)),
        ],
        cursor: 0,
        written: Vec::new(),
    };
    let stop = AtomicBool::new(false);
    let progress = Mutex::new(fresh_pack_progress());
    let wake = Condvar::new();
    match write_pack_bytes(&mut writer, b"abcd", &stop, &progress, &wake).into_data() {
        data!(PackWrite::Done) => {}
        _ => panic!("the pack write finishes"),
    }
    assert_eq!(writer.written, b"abcd");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn write_pack_bytes_rejects_a_write_after_the_deadline() {
    // The pre-write check passes.
    // The write finishes after the limit.
    // The result is a hang.
    let limit = std::time::Duration::from_millis(40);
    let stop = AtomicBool::new(false);
    let progress = Mutex::new(fresh_pack_progress());
    let wake = Condvar::new();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut writer = LatePackBytes {
            delay: limit + std::time::Duration::from_millis(80),
            paused: false,
            payload: 0,
        };
        let _ = write_pack_bytes_for(&mut writer, b"abcd", &stop, &progress, &wake, limit);
    }));
    let message = panic_payload_text(caught.expect_err("a late write is a hang"));
    assert!(
        message.contains("this limit only stops a hang"),
        "{message}"
    );
}

/// A writer that sets the stop flag and then always returns Interrupted.
#[invariant(true)]
struct StopThenInterruptWrite {
    stop: Arc<AtomicBool>,
    calls: usize,
}

impl std::io::Write for StopThenInterruptWrite {
    #[requires(true)]
    #[ensures(ret.is_err())]
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        self.calls += 1;
        self.stop.store(true, AtomicOrdering::Release);
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "injected interrupt",
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn write_pack_bytes_returns_to_the_stop_flag_on_persistent_interrupts() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut writer = StopThenInterruptWrite {
        stop: Arc::clone(&stop),
        calls: 0,
    };
    let progress = Mutex::new(fresh_pack_progress());
    let wake = Condvar::new();
    match write_pack_bytes(&mut writer, b"abcd", &stop, &progress, &wake).into_data() {
        data!(PackWrite::Stopped) => {}
        _ => panic!("the pack write returns to the stop flag"),
    }
    assert!(writer.calls < 5, "calls={}", writer.calls);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_file_chunk_returns_to_the_stop_flag_on_persistent_interrupts() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut reader = StopThenInterrupt {
        stop: Arc::clone(&stop),
        calls: 0,
    };
    let mut buf = [0_u8; 8];
    let chunk = read_pack_file_chunk(&mut reader, &mut buf, &stop).expect("the file read returns");
    assert!(chunk.is_none());
    assert!(reader.calls < 5, "calls={}", reader.calls);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn call_until_stop_returns_on_persistent_interrupts() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut calls = 0_u32;
    let finished = call_until_stop(&stop, || {
        calls += 1;
        stop.store(true, AtomicOrdering::Release);
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "injected interrupt",
        ))
    })
    .expect("the call returns to the stop flag");
    assert!(!finished);
    assert!(calls < 5, "calls={calls}");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn port_scan_reports_permission_denied_with_the_address() {
    let mut calls = 0_u32;
    let error = select_local_port(18800, 18899, |port| {
        calls += 1;
        assert_eq!(port, 18800);
        Err::<(), _>(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "injected permission",
        ))
    })
    .expect_err("permission denied is not a missing port");
    assert_eq!(calls, 1);
    assert!(error.contains("127.0.0.1:18800"), "{error}");
    assert!(error.contains("injected permission"), "{error}");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn port_scan_retries_the_same_port_after_an_interrupt() {
    let mut calls = 0_u32;
    let listener = select_local_port(18800, 18899, |port| {
        calls += 1;
        assert_eq!(port, 18800);
        if calls == 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "injected interrupt",
            ));
        }
        Ok(port)
    })
    .expect("an interrupt retries the same port");
    assert_eq!(listener, 18800);
    assert_eq!(calls, 2);
}

/// A reader or writer that pauses once and then returns one scripted result.
/// `payload` 0 means end of file.
/// Any other payload is one byte.
#[invariant(true)]
struct LatePackBytes {
    delay: std::time::Duration,
    paused: bool,
    payload: u8,
}

impl std::io::Read for LatePackBytes {
    #[requires(true)]
    #[ensures(true)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if !self.paused {
            self.paused = true;
            std::thread::sleep(self.delay);
        }
        if self.payload == 0 {
            return Ok(0);
        }
        buf[0] = self.payload;
        self.payload = 0;
        Ok(1)
    }
}

impl std::io::Write for LatePackBytes {
    #[requires(true)]
    #[ensures(true)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if !self.paused {
            self.paused = true;
            std::thread::sleep(self.delay);
        }
        Ok(buf.len())
    }

    #[requires(true)]
    #[ensures(true)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_line_retries_an_interrupted_read() {
    // One interrupt comes before each byte.
    // A read path that stops on Interrupted fails this test.
    let head = b"GET /pack HTTP/1.1\r\n";
    let mut reader = new!(InterruptBeforeEachByte {
        bytes: head,
        index: 0,
        interrupt: true,
    });
    let stop = AtomicBool::new(false);
    match read_pack_line(&mut reader, &stop).into_data() {
        data!(PackRead::Line(text)) => assert_eq!(text, "GET /pack HTTP/1.1\r\n"),
        _ => panic!("the pack line is complete"),
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_line_rejects_a_byte_after_the_deadline() {
    // The pre-read check passes.
    // The byte arrives after the limit.
    // The line is a hang.
    let limit = std::time::Duration::from_millis(40);
    let stop = AtomicBool::new(false);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut reader = LatePackBytes {
            delay: limit + std::time::Duration::from_millis(80),
            paused: false,
            payload: b'\n',
        };
        let _ = read_pack_line_for(&mut reader, &stop, limit);
    }));
    let message = panic_payload_text(caught.expect_err("a late byte is a hang"));
    assert!(
        message.contains("this limit only stops a hang"),
        "{message}"
    );

    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut reader = LatePackBytes {
            delay: limit + std::time::Duration::from_millis(80),
            paused: false,
            payload: 0,
        };
        let _ = read_pack_line_for(&mut reader, &stop, limit);
    }));
    let message = panic_payload_text(caught.expect_err("late end of file is a hang"));
    assert!(
        message.contains("this limit only stops a hang"),
        "{message}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn read_pack_line_reports_one_reset() {
    // The reset is not an interrupt.
    // The helper must not call the reader again.
    let mut reader = ResetOnce { calls: 0 };
    let stop = AtomicBool::new(false);
    match read_pack_line(&mut reader, &stop).into_data() {
        data!(PackRead::Broken(message)) => assert!(message.contains("injected reset")),
        _ => panic!("the pack line reports the reset"),
    }
    assert_eq!(reader.calls, 1);
}

#[invariant(true)]
struct PackServer {
    address: std::net::SocketAddr,
    progress: Arc<Mutex<PackProgress>>,
    wake: Arc<Condvar>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl PackServer {
    #[requires(root.is_dir())]
    #[ensures(ret.worker.is_some())]
    fn start(root: PathBuf) -> Self {
        let listener = select_local_port(18800, 18899, |port| {
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        })
        .unwrap_or_else(|error| panic!("{error}"));
        loop {
            match listener.set_nonblocking(true) {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => panic!("the pack server socket accepts nonblocking mode: {error}"),
            }
        }
        let address = listener
            .local_addr()
            .expect("the pack server has a local address");
        let progress = Arc::new(Mutex::new(fresh_pack_progress()));
        let wake = Arc::new(Condvar::new());
        let stop = Arc::new(AtomicBool::new(false));
        let worker_progress = Arc::clone(&progress);
        let worker_wake = Arc::clone(&wake);
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || -> Result<(), String> {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || -> Result<(), String> {
                    // The listener does not block in accept.
                    // The first WouldBlock poll records that the accept loop is running.
                    // Each pass reads the stop flag.
                    // Shutdown can join without a wakeup connection.
                    // An interrupted accept is tried again.
                    // Any other socket error stops the worker and names the error.
                    'accept: while !worker_stop.load(AtomicOrdering::Acquire) {
                        let (mut stream, _) = match listener.accept() {
                            Ok(connection) => connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                {
                                    let mut progress = worker_progress
                                        .lock()
                                        .expect("the pack server accept flag is open");
                                    if !progress.accept_ready {
                                        progress.accept_ready = true;
                                        worker_wake.notify_all();
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(2));
                                continue;
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                continue;
                            }
                            Err(error) => {
                                return Err(format!("the pack server accept failed: {error}"));
                            }
                        };
                        // On macOS and BSD, an accepted socket keeps the nonblocking mode of the listener.
                        // A nonblocking read returns WouldBlock.
                        // Set this socket to blocking mode before the read.
                        // The socket timeout is 100 milliseconds.
                        // An interrupt returns to the stop flag.
                        if !call_until_stop(&worker_stop, || stream.set_nonblocking(false)).map_err(
                    |error| {
                        format!("the pack server accepted socket did not become blocking: {error}")
                    },
                )? {
                    break;
                }
                        if !call_until_stop(&worker_stop, || {
                            stream.set_read_timeout(Some(PACK_SOCKET_POLL))
                        })
                        .map_err(|error| {
                            format!("the pack server accepted socket has no read timeout: {error}")
                        })? {
                            break;
                        }
                        if !call_until_stop(&worker_stop, || {
                            stream.set_write_timeout(Some(PACK_SOCKET_POLL))
                        })
                        .map_err(|error| {
                            format!("the pack server accepted socket has no write timeout: {error}")
                        })? {
                            break;
                        }
                        let line = match read_pack_line(&mut stream, &worker_stop).into_data() {
                            data!(PackRead::Line(line)) => line,
                            data!(PackRead::Stopped) => break,
                            data!(PackRead::End) => continue,
                            data!(PackRead::Broken(message)) => {
                                return Err(format!("the pack server read failed: {message}"));
                            }
                        };
                        let mut parts = line.split_whitespace();
                        let method = parts.next().unwrap_or("");
                        if method != "GET" {
                            return Err(format!(
                                "the pack server saw a request that is not GET: {line}"
                            ));
                        }
                        let Some(relative) = parts.next() else {
                            return Err(format!("the pack server request has no path: {line}"));
                        };
                        let relative = relative.trim_start_matches('/').to_owned();
                        // Record the request, then wake a test that waits for it.
                        // The request list and the worker result share one mutex.
                        {
                            let mut progress = worker_progress
                                .lock()
                                .expect("the pack server request list is open");
                            progress.requests.push(relative.clone());
                            worker_wake.notify_one();
                        }
                        loop {
                            match read_pack_line(&mut stream, &worker_stop).into_data() {
                                data!(PackRead::Stopped) => break 'accept,
                                data!(PackRead::Broken(message)) => {
                                    return Err(format!("the pack server read failed: {message}"));
                                }
                                data!(PackRead::End) => break,
                                data!(PackRead::Line(header)) if header == "\r\n" => break,
                                data!(PackRead::Line(_)) => {}
                            }
                        }
                        let path =
                            root.join(safe_pack_relative_path(&relative).map_err(|error| {
                                format!("the pack server path is not safe: {error}")
                            })?);
                        let mut file = match File::open(&path) {
                    Ok(file) => file,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        match write_pack_bytes(
                            &mut stream,
                            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            &worker_stop,
                            &worker_progress,
                            &worker_wake,
                        )
                        .into_data()
                        {
                            data!(PackWrite::Stopped) => break,
                            data!(PackWrite::Done) => continue,
                            data!(PackWrite::Broken(message)) => {
                                return Err(format!("the pack server write failed: {message}"));
                            }
                        }
                    }
                    Err(error) => {
                        return Err(format!(
                            "the pack server cannot open {}: {error}",
                            path.display()
                        ));
                    }
                };
                        let length = file.metadata().map_err(|error| {
                            format!("the pack server file length failed: {error}")
                        })?;
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            length.len()
                        );
                        // io::copy uses sendfile for a file and a socket.
                        // sendfile ignores the write timeout.
                        // Each chunk uses the 100 millisecond timeout.
                        // The record is set immediately before the first header write.
                        record_pack_response_start(&worker_progress, &worker_wake);
                        match write_pack_bytes(
                            &mut stream,
                            header.as_bytes(),
                            &worker_stop,
                            &worker_progress,
                            &worker_wake,
                        )
                        .into_data()
                        {
                            data!(PackWrite::Done) => {}
                            data!(PackWrite::Stopped) => break,
                            data!(PackWrite::Broken(message)) => {
                                return Err(format!("the pack server write failed: {message}"));
                            }
                        }
                        let mut buf = [0_u8; 64 * 1024];
                        loop {
                            let count =
                                match read_pack_file_chunk(&mut file, &mut buf, &worker_stop)? {
                                    Some(0) => break,
                                    Some(count) => count,
                                    None => break 'accept,
                                };
                            match write_pack_bytes(
                                &mut stream,
                                &buf[..count],
                                &worker_stop,
                                &worker_progress,
                                &worker_wake,
                            )
                            .into_data()
                            {
                                data!(PackWrite::Done) => {}
                                data!(PackWrite::Stopped) => break 'accept,
                                data!(PackWrite::Broken(message)) => {
                                    return Err(format!("the pack server write failed: {message}"));
                                }
                            }
                        }
                    }
                    Ok(())
                },
            ));
            let result = match outcome {
                Ok(result) => result,
                Err(payload) => Err(pack_server_panic_message(payload)),
            };
            publish_pack_worker_result(&worker_progress, &worker_wake, &result);
            result
        });
        Self {
            address,
            progress,
            wake,
            stop,
            worker: Some(worker),
        }
    }

    /// Stop the worker and return its result.
    /// The accept loop reads the stop flag.
    /// A side thread joins the worker.
    /// If that join does not finish in 60 seconds, return an error.
    /// The error says that this limit only stops a hang.
    /// On that failure the side thread keeps waiting until the process exits.
    /// This path does not open a wakeup connection.
    #[requires(true)]
    #[ensures(self.worker.is_none())]
    fn shutdown(&mut self) -> Result<(), String> {
        self.stop.store(true, AtomicOrdering::Release);
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = match worker.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(error),
                Err(payload) => Err(pack_server_panic_message(payload)),
            };
            let _ = result_tx.send(result);
        });
        match result_rx.recv_timeout(PACK_HANG_LIMIT) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err("this limit only stops a hang".to_owned())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("the hang limit thread stopped".to_owned())
            }
        }
    }
}

/// The text of a panic payload.
/// The text is not empty.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn panic_payload_text(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        if !message.is_empty() {
            return message.clone();
        }
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        if !message.is_empty() {
            return (*message).to_owned();
        }
    }
    "the worker panicked".to_owned()
}

/// Publish a worker result and wake every pack wait.
/// The result is stored in the same mutex as the milestones.
#[requires(true)]
#[ensures(true)]
fn publish_pack_worker_result(
    progress: &Mutex<PackProgress>,
    wake: &Condvar,
    result: &Result<(), String>,
) {
    let mut guard = progress
        .lock()
        .expect("the pack server worker result is open");
    guard.worker_result = Some(result.clone());
    wake.notify_all();
}

/// The text for a worker result that ends a wait.
/// An error uses the worker text.
/// A clean finish uses the hang text.
/// The text is not empty.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn pack_wait_failure_text(result: &Result<(), String>) -> String {
    match result {
        Err(error) if !error.is_empty() => error.clone(),
        Err(_) | Ok(()) => "this limit only stops a hang".to_owned(),
    }
}

/// Wait until a milestone is set.
/// The milestone and the worker result share one mutex.
/// The clock is read before the milestone is tested.
/// A milestone at the limit or later is a hang.
/// The hook runs after the predicate and before the sleep.
/// The wait reads the clock and both values again before it sleeps.
/// A worker error fails this wait only while the milestone is unset.
/// The sleep stops at the remaining time.
#[requires(!limit.is_zero())]
#[ensures(true)]
fn wait_for_pack_milestone(
    progress: &Mutex<PackProgress>,
    wake: &Condvar,
    limit: std::time::Duration,
    still_waiting: impl Fn(&PackProgress) -> bool,
    mut before_sleep: impl FnMut(&mut PackProgress),
) {
    let started = std::time::Instant::now();
    let mut guard = progress.lock().expect("the pack server milestone is open");
    loop {
        if started.elapsed() >= limit {
            panic!("this limit only stops a hang");
        }
        if !still_waiting(&guard) {
            return;
        }
        if let Some(result) = guard.worker_result.clone() {
            panic!("{}", pack_wait_failure_text(&result));
        }
        before_sleep(&mut guard);
        if started.elapsed() >= limit {
            panic!("this limit only stops a hang");
        }
        if !still_waiting(&guard) {
            return;
        }
        if let Some(result) = guard.worker_result.clone() {
            panic!("{}", pack_wait_failure_text(&result));
        }
        let remaining = limit.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            panic!("this limit only stops a hang");
        }
        let (next, _) = wake
            .wait_timeout(guard, remaining)
            .expect("the pack server milestone is open");
        guard = next;
    }
}

#[requires(true)]
#[ensures(ret.starts_with("the pack server thread panicked"))]
fn pack_server_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return format!("the pack server thread panicked: {message}");
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return format!("the pack server thread panicked: {message}");
    }
    "the pack server thread panicked".to_owned()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_shutdown_joins_without_a_client() {
    // No client connects.
    // The test waits until the accept loop records its first poll.
    // The accept loop reads the stop flag.
    // Shutdown returns without a wakeup connection.
    let root = setup_test_dir();
    let mut server = PackServer::start(root.path().to_owned());
    wait_until_pack_accept_is_ready(&server);
    server
        .shutdown()
        .expect("the pack server thread finishes cleanly");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_worker_panic_wakes_a_milestone_wait() {
    let progress = Arc::new(Mutex::new(fresh_pack_progress()));
    let wake = Arc::new(Condvar::new());
    let progress_worker = Arc::clone(&progress);
    let wake_worker = Arc::clone(&wake);
    let publisher = std::thread::spawn(move || {
        let payload = std::panic::catch_unwind(|| panic!("pack worker sentinel"))
            .expect_err("the worker panics");
        let result = Err(pack_server_panic_message(payload));
        publish_pack_worker_result(&progress_worker, &wake_worker, &result);
    });
    let started = std::time::Instant::now();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_pack_milestone(
            &progress,
            &wake,
            std::time::Duration::from_secs(5),
            |state| !state.accept_ready,
            |_| {},
        );
    }));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    let message = panic_payload_text(caught.expect_err("the wait reports the worker panic"));
    assert!(message.contains("pack worker sentinel"), "{message}");
    assert!(
        !message.contains("this limit only stops a hang"),
        "{message}"
    );
    publisher.join().expect("the publisher finishes");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_wait_sees_a_result_published_before_sleep() {
    // The predicate sees an unset milestone and no worker result.
    // The hook then stores the worker error.
    // The wait reads that error before it sleeps.
    let progress = Mutex::new(fresh_pack_progress());
    let wake = Condvar::new();
    let started = std::time::Instant::now();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_pack_milestone(
            &progress,
            &wake,
            std::time::Duration::from_secs(60),
            |state| !state.accept_ready,
            |state| {
                state.worker_result = Some(Err("connection reset".to_owned()));
            },
        );
    }));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    let message = panic_payload_text(caught.expect_err("the wait reports the worker error"));
    assert!(message.contains("connection reset"), "{message}");
    assert!(
        !message.contains("this limit only stops a hang"),
        "{message}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_wait_rejects_a_milestone_after_the_deadline() {
    // The first pass sees no milestone.
    // The hook then sets the milestone after the limit.
    // The next pass reads the clock before it tests the milestone.
    // The milestone is a hang.
    let progress = Mutex::new(fresh_pack_progress());
    let wake = Condvar::new();
    let limit = std::time::Duration::from_millis(40);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wait_for_pack_milestone(
            &progress,
            &wake,
            limit,
            |state| !state.accept_ready,
            |state| {
                std::thread::sleep(limit + std::time::Duration::from_millis(80));
                state.accept_ready = true;
            },
        );
    }));
    let message = panic_payload_text(caught.expect_err("a late milestone is a hang"));
    assert!(
        message.contains("this limit only stops a hang"),
        "{message}"
    );
}

/// Close the socket with a reset.
/// Linux linger of zero sends that reset.
#[cfg(target_os = "linux")]
#[requires(true)]
#[ensures(true)]
fn close_with_reset(stream: &std::net::TcpStream) {
    use std::os::fd::AsRawFd;
    unsafe extern "C" {
        fn setsockopt(
            fd: i32,
            level: i32,
            name: i32,
            value: *const core::ffi::c_void,
            len: u32,
        ) -> i32;
    }
    // Linux SOL_SOCKET is 1. Linux SO_LINGER is 13.
    // Two i32 values match the Linux linger record.
    let linger = [1_i32, 0_i32];
    let result = unsafe {
        setsockopt(
            stream.as_raw_fd(),
            1,
            13,
            linger.as_ptr().cast(),
            std::mem::size_of_val(&linger) as u32,
        )
    };
    assert_eq!(result, 0, "the pack client sets linger");
}

#[cfg(target_os = "linux")]
#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_read_error_wakes_the_request_wait() {
    let root = setup_test_dir();
    let server = PackServer::start(root.path().to_owned());
    wait_until_pack_accept_is_ready(&server);
    let client = std::net::TcpStream::connect(server.address).expect("the pack client connects");
    std::thread::sleep(std::time::Duration::from_millis(50));
    close_with_reset(&client);
    drop(client);
    let (sender, receiver) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wait_for_pack_request(&server);
        }));
        let _ = sender.send(caught);
    });
    let caught = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the request wait observes the worker error");
    let message = panic_payload_text(caught.expect_err("the worker error reaches the wait"));
    assert!(message.contains("the pack server read failed"), "{message}");
    assert!(
        !message.contains("this limit only stops a hang"),
        "{message}"
    );
    waiter.join().expect("the waiter finishes");
}

/// True for the root user.
/// Root can read a file whose mode is 000.
#[cfg(unix)]
#[requires(true)]
#[ensures(true)]
fn pack_test_euid_is_root() -> bool {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

/// One step of the pack client receive loop.
/// `Closed` is end of file or the expected reset.
/// `Bytes` is a nonempty read.
/// `Retry` is an interrupt, a timeout, or a would-block result.
/// `Failed` carries any other socket error.
/// That text is not empty.
#[invariant(true)]
#[invariant(::Bytes(count) => *count > 0)]
#[invariant(::Failed(message) => !message.is_empty())]
enum PackReceiveStep {
    Closed,
    Bytes(usize),
    Retry,
    Failed(String),
}

/// Classify one pack client read.
/// End of file and the expected reset are closure.
/// Any other socket error keeps its cause.
#[requires(true)]
#[ensures(true)]
fn pack_receive_step(read: std::io::Result<usize>) -> PackReceiveStep {
    match read {
        Ok(0) => new!(PackReceiveStep::Closed),
        Ok(count) => new!(PackReceiveStep::Bytes(count)),
        Err(error)
            if error.kind() == std::io::ErrorKind::Interrupted || pack_io_is_a_stall(&error) =>
        {
            new!(PackReceiveStep::Retry)
        }
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
            new!(PackReceiveStep::Closed)
        }
        Err(error) => new!(PackReceiveStep::Failed(socket_error_text(&error))),
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_receive_reports_a_socket_error_with_its_cause() {
    match pack_receive_step(Err(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "injected permission",
    )))
    .into_data()
    {
        data!(PackReceiveStep::Failed(message)) => {
            assert!(message.contains("injected permission"), "{message}");
        }
        _ => panic!("the socket error keeps its cause"),
    }
    match pack_receive_step(Ok(0)).into_data() {
        data!(PackReceiveStep::Closed) => {}
        _ => panic!("end of file is closure"),
    }
    match pack_receive_step(Ok(4)).into_data() {
        data!(PackReceiveStep::Bytes(4)) => {}
        _ => panic!("a nonempty read keeps its count"),
    }
    match pack_receive_step(Err(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "reset",
    )))
    .into_data()
    {
        data!(PackReceiveStep::Closed) => {}
        _ => panic!("the expected reset is closure"),
    }
    for kind in [
        std::io::ErrorKind::WouldBlock,
        std::io::ErrorKind::TimedOut,
        std::io::ErrorKind::Interrupted,
    ] {
        match pack_receive_step(Err(std::io::Error::new(kind, "again"))).into_data() {
            data!(PackReceiveStep::Retry) => {}
            _ => panic!("the receive loop retries {kind:?}"),
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_client_rejects_a_read_after_the_deadline() {
    // The pre-read check passes.
    // The byte arrives after the deadline.
    // End of file after the deadline is the same hang.
    let limit = std::time::Duration::from_millis(40);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut reader = LatePackBytes {
            delay: limit + std::time::Duration::from_millis(80),
            paused: false,
            payload: b'x',
        };
        let deadline = std::time::Instant::now() + limit;
        let _ = read_pack_client_until_close(&mut reader, deadline, |_stream, _remaining| Ok(()));
    }));
    let message = panic_payload_text(caught.expect_err("late bytes are a hang"));
    assert!(
        message.contains("the pack server did not close the socket"),
        "{message}"
    );

    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut reader = LatePackBytes {
            delay: limit + std::time::Duration::from_millis(80),
            paused: false,
            payload: 0,
        };
        let deadline = std::time::Instant::now() + limit;
        let _ = read_pack_client_until_close(&mut reader, deadline, |_stream, _remaining| Ok(()));
    }));
    let message = panic_payload_text(caught.expect_err("late end of file is a hang"));
    assert!(
        message.contains("the pack server did not close the socket"),
        "{message}"
    );
}

/// Read until the pack server closes the socket.
/// The clock is read after each read returns.
/// End of file or bytes at the deadline or later are a hang.
/// An interrupt, a timeout, or a would-block result tries again.
/// Any other socket error reports its cause.
#[requires(true)]
#[ensures(true)]
fn read_pack_client_until_close<S, F>(
    stream: &mut S,
    deadline: std::time::Instant,
    mut set_timeout: F,
) -> Vec<u8>
where
    S: std::io::Read,
    F: FnMut(&mut S, std::time::Duration) -> std::io::Result<()>,
{
    let mut buf = [0_u8; 128];
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            panic!("the pack server did not close the socket");
        }
        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            panic!("the pack server did not close the socket");
        }
        match set_timeout(stream, remaining) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!(
                "the pack client sets a read timeout: {}",
                socket_error_text(&error)
            ),
        }
        match pack_receive_step(std::io::Read::read(stream, &mut buf)).into_data() {
            data!(PackReceiveStep::Closed) => {
                if std::time::Instant::now() >= deadline {
                    panic!("the pack server did not close the socket");
                }
                return Vec::new();
            }
            data!(PackReceiveStep::Bytes(count)) => {
                if std::time::Instant::now() >= deadline {
                    panic!("the pack server did not close the socket");
                }
                return buf[..count].to_vec();
            }
            data!(PackReceiveStep::Retry) => continue,
            data!(PackReceiveStep::Failed(message)) => {
                panic!("the pack client read failed: {message}");
            }
        }
    }
}

/// Report a file open error that is not a missing file.
/// A normal user gets that error from a mode of 000.
/// Root can read a mode of 000.
/// For root, the path is a child of a regular file.
/// The response is not HTTP 404.
/// The error names the path and the cause.
#[cfg(unix)]
#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_reports_an_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;
    let root = setup_test_dir();
    let path = root.path().join("big");
    std::fs::write(&path, b"secret").unwrap();
    let (request, cause): (&'static [u8], &'static str) = if pack_test_euid_is_root() {
        (b"GET /big/child HTTP/1.1\r\n\r\n", "directory")
    } else {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        (b"GET /big HTTP/1.1\r\n\r\n", "permission")
    };
    let mut server = PackServer::start(root.path().to_owned());
    wait_until_pack_accept_is_ready(&server);
    let mut client =
        std::net::TcpStream::connect(server.address).expect("the pack client connects");
    client.write_all(request).expect("the pack client writes");
    wait_for_pack_request(&server);
    // The client reads until the worker closes the socket.
    // That close follows the file open.
    // Shutdown starts after the close, so the stop flag does not skip the open.
    // The receive loop stops at its deadline.
    // End of file and the expected reset are closure.
    // Any other socket error reports its cause.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let bytes = read_pack_client_until_close(&mut client, deadline, |stream, remaining| {
        stream.set_read_timeout(Some(remaining))
    });
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("404"), "{text}");
    let started = std::time::Instant::now();
    let error = server
        .shutdown()
        .expect_err("the pack server reports the file error");
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert!(error.contains("big"), "{error}");
    assert!(error.to_lowercase().contains(cause), "{error}");
    drop(client);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_returns_404_for_a_missing_file() {
    let root = setup_test_dir();
    let mut server = PackServer::start(root.path().to_owned());
    wait_until_pack_accept_is_ready(&server);
    let mut client =
        std::net::TcpStream::connect(server.address).expect("the pack client connects");
    client
        .write_all(b"GET /missing HTTP/1.1\r\n\r\n")
        .expect("the pack client writes");
    let header = read_pack_response_header(&mut client);
    assert!(header.starts_with("HTTP/1.1 404"), "{header}");
    server
        .shutdown()
        .expect("the pack server thread finishes cleanly");
}

/// Wait until the accept loop records its first poll.
/// A wall clock of 60 seconds waits with the ready flag.
/// When the flag is true, a correct run returns.
/// When the flag stays false, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_until_pack_accept_is_ready(server: &PackServer) {
    wait_for_pack_milestone(
        &server.progress,
        &server.wake,
        PACK_HANG_LIMIT,
        |progress| !progress.accept_ready,
        |_| {},
    );
}

/// Wait until the worker records one request.
/// A wall clock of 60 seconds waits with the request list.
/// When the record is present, a correct run returns.
/// When the record does not come, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_for_pack_request(server: &PackServer) {
    wait_for_pack_milestone(
        &server.progress,
        &server.wake,
        PACK_HANG_LIMIT,
        |progress| progress.requests.is_empty(),
        |_| {},
    );
}

/// Wait until the worker is about to write the response header.
/// A wall clock of 60 seconds waits with the response flag.
/// If the flag is true, a correct run returns.
/// If the flag stays false, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_until_pack_response_starts(server: &PackServer) {
    wait_for_pack_milestone(
        &server.progress,
        &server.wake,
        PACK_HANG_LIMIT,
        |progress| !progress.response_started,
        |_| {},
    );
}

/// Wait until a write stalls before the stop flag.
/// A wall clock of 60 seconds waits with the stall flag.
/// If the flag is true, a correct run returns.
/// If the flag stays false, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_until_pack_write_stalls(server: &PackServer) {
    wait_for_pack_milestone(
        &server.progress,
        &server.wake,
        PACK_HANG_LIMIT,
        |progress| !progress.write_stalled,
        |_| {},
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_shutdown_returns_during_a_blocked_write() {
    // The file length is 256 MiB.
    // The client sends the request and reads only the response header.
    // The test waits until a write stalls.
    // A missing stall fails after 60 seconds.
    // The failure text says that this limit only stops a hang.
    // The client must receive the response header.
    // The short timeout returns.
    // The worker sees the stop flag.
    // Shutdown returns the worker result.
    let root = setup_test_dir();
    let big = File::create(root.path().join("big")).unwrap();
    big.set_len(256 * 1024 * 1024).unwrap();
    drop(big);
    let mut server = PackServer::start(root.path().to_owned());
    let mut client = loop {
        match std::net::TcpStream::connect(server.address) {
            Ok(stream) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("the pack client connects: {error}"),
        }
    };
    client.write_all(b"GET /big HTTP/1.1\r\n\r\n").unwrap();
    wait_for_pack_request(&server);
    wait_until_pack_response_starts(&server);
    let header = read_pack_response_header(&mut client);
    assert!(
        header.starts_with("HTTP/1.1 200 OK\r\n") && header.ends_with("\r\n\r\n"),
        "the client received the response header"
    );
    eprintln!(
        "PACK_RESPONSE_HEADER={}",
        header.split("\r\n").next().unwrap_or("")
    );
    wait_until_pack_write_stalls(&server);
    let started = std::time::Instant::now();
    let result = server.shutdown();
    let elapsed = started.elapsed();
    drop(client);
    eprintln!(
        "PACK_SHUTDOWN_OK={} PACK_SHUTDOWN_MS={}",
        result.is_ok(),
        elapsed.as_millis()
    );
    result.expect("the pack server thread finishes cleanly");
}

impl Drop for PackServer {
    #[requires(true)]
    #[ensures(self.worker.is_none())]
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            let _ = std::io::Write::write_fmt(
                &mut std::io::stderr(),
                format_args!("the pack server thread failed during drop: {error}\n"),
            );
        }
    }
}

#[requires(rows > 0)]
#[requires([VLACKU_CORPUS_ID, CUKTA_CORPUS_ID].contains(&id))]
#[ensures(true)]
fn write_fixture_items(path: &Path, id: &str, rows: usize, old: bool) {
    let input_hash = sha256_hex_bytes(if old { b"old input" } else { b"current input" });
    if id == VLACKU_CORPUS_ID {
        let items = (0..rows)
            .map(|row| DictionaryEmbeddingItem {
                entry_index: row,
                word: format!("fixture-{row}"),
                definition_id: row as u64,
                input_hash: input_hash.clone(),
                kind: "gismu".to_owned(),
            })
            .collect::<Vec<_>>();
        write_json_file(path, &items).unwrap();
    } else {
        let items = (0..rows)
            .map(|row| CllEmbeddingItem {
                chunk_index: row,
                input_hash: input_hash.clone(),
            })
            .collect::<Vec<_>>();
        write_json_file(path, &items).unwrap();
    }
}

#[requires(root.is_dir())]
#[ensures(ret.model_key == spec.model_key)]
fn current_fixture(root: &Path, spec: &EmbeddingModelSpec) -> EmbeddingPackManifest {
    let identity = current_native_corpus_identity().unwrap();
    let pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &spec.native_sha256,
        &identity.dictionary_fingerprint,
        &identity.cll_fingerprint,
    );
    let pack_dir = pack_root(root, &spec.model_key, &pack_id);
    // Real setup selects the compiled model and corpora. Synthetic unit vectors avoid model inference.
    let mut corpora = Vec::new();
    for (id, rows, fingerprint) in [
        (
            VLACKU_CORPUS_ID,
            identity.dictionary_rows,
            &identity.dictionary_fingerprint,
        ),
        (
            CUKTA_CORPUS_ID,
            identity.cll_rows,
            &identity.cll_fingerprint,
        ),
    ] {
        let prefix = format!("corpora/{id}");
        let items_url = format!("{prefix}/items.json");
        write_fixture_items(&pack_dir.join(&items_url), id, rows, false);
        let mut values = vec![0.0; rows * spec.dimensions];
        for row in values.chunks_exact_mut(spec.dimensions) {
            row[0] = 1.0;
        }
        let shard = write_vector_chunk_file(
            &pack_dir.join(&prefix),
            &prefix,
            "vectors.f32",
            &values,
            spec.dimensions,
        )
        .unwrap();
        corpora.push(CorpusManifest {
            corpus_id: id.to_owned(),
            input_format_version: spec.input_format_version.clone(),
            fingerprint: fingerprint.clone(),
            row_count: rows,
            dimensions: spec.dimensions,
            items_sha256: sha256_hex_file(&pack_dir.join(&items_url)).unwrap(),
            items_url,
            shards: vec![shard],
        });
    }
    let manifest = EmbeddingPackManifest {
        vector_space: native_vector_space_key(),
        model_sha256: Some(spec.native_sha256.clone()),
        build_provenance: None,
        legacy_adoption: None,
        vector_parameters: Some(native_vector_parameters()),
        schema_version: INDEX_SCHEMA_VERSION,
        model_key: spec.model_key.clone(),
        model_revision: spec.model_revision.clone(),
        pack_id,
        input_format_version: spec.input_format_version.clone(),
        built_by: EmbeddingRuntime {
            runtime: "fixture".to_owned(),
            version: "test".to_owned(),
        },
        dimensions: spec.dimensions,
        element_type: "f32le".to_owned(),
        normalized: true,
        distance: "dot".to_owned(),
        compatible_query_runtimes: vec![],
        corpora,
    };
    write_json_file(&pack_dir.join("manifest.json"), &manifest).unwrap();
    write_catalog(root, spec, &manifest.pack_id).unwrap();
    manifest
}

#[test]
#[requires(true)]
#[ensures(true)]
fn public_setup_updates_one_corpus_without_force_and_repairs_damaged_candidates() {
    let remote = setup_test_dir();
    let spec = model_spec(DEFAULT_MODEL_KEY).unwrap();
    let desired = current_fixture(remote.path(), &spec);
    let remote_pack = pack_root(remote.path(), &spec.model_key, &desired.pack_id);
    let mut server = PackServer::start(remote.path().join(INDEX_BASE_VERSION));
    let models = setup_test_dir();
    let model = model_file_path(models.path(), &spec);
    ensure_parent_dir(&model).unwrap();
    // Size-only setup validation accepts a sparse fixture. The precomputed path never opens llama.cpp.
    File::create(&model)
        .unwrap()
        .set_len(spec.native_size_bytes)
        .unwrap();
    for changed in [VLACKU_CORPUS_ID, CUKTA_CORPUS_ID] {
        for damaged in [false, true] {
            let index = setup_test_dir();
            let mut old = desired.clone();
            old.corpora
                .iter_mut()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap()
                .fingerprint = sha256_hex_bytes(b"older corpus");
            old.pack_id = deterministic_pack_id(
                &spec.input_format_version,
                &spec.model_revision,
                &spec.native_sha256,
                &old.corpora[0].fingerprint,
                &old.corpora[1].fingerprint,
            );
            assert_ne!(old.pack_id, desired.pack_id);
            let old_pack = pack_root(index.path(), &spec.model_key, &old.pack_id);
            for relative in native_pack_remote_paths(&desired).unwrap() {
                ensure_parent_dir(&old_pack.join(&relative)).unwrap();
                fs::copy(remote_pack.join(&relative), old_pack.join(relative)).unwrap();
            }
            let changed_corpus = old
                .corpora
                .iter_mut()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap();
            let items_path = old_pack.join(&changed_corpus.items_url);
            write_fixture_items(&items_path, changed, changed_corpus.row_count, true);
            changed_corpus.items_sha256 = sha256_hex_file(&items_path).unwrap();
            let vector_path = old_pack.join(&changed_corpus.shards[0].url);
            let mut file = File::options().write(true).open(&vector_path).unwrap();
            file.write_all(&0.0_f32.to_le_bytes()).unwrap();
            file.write_all(&1.0_f32.to_le_bytes()).unwrap();
            drop(file);
            changed_corpus.shards[0].sha256 = sha256_hex_file(&vector_path).unwrap();
            write_json_file(&old_pack.join("manifest.json"), &old).unwrap();
            write_catalog(index.path(), &spec, &old.pack_id).unwrap();
            validate_pack_dir(&old_pack).unwrap();
            let unchanged = old
                .corpora
                .iter()
                .find(|corpus| corpus.corpus_id != changed)
                .unwrap();
            if damaged {
                let mut file = File::options()
                    .write(true)
                    .open(old_pack.join(&unchanged.shards[0].url))
                    .unwrap();
                file.seek(SeekFrom::Start(0)).unwrap();
                // A one-bit, same-size change stays near unit length but fails the SHA-256 proof.
                file.write_all(&[1]).unwrap();
            }
            server.progress.lock().unwrap().requests.clear();
            assert!(!pack_root(index.path(), &spec.model_key, &desired.pack_id).exists());
            let options = SetupOptions {
                force: false,
                use_precomputed: UsePrecomputed::Always,
                skip_validation: true,
                index_dir: Some(index.path().to_owned()),
                model_dir: Some(models.path().to_owned()),
                precomputed_base_url: format!("http://{}", server.address),
                ..SetupOptions::default()
            };
            if changed == VLACKU_CORPUS_ID && !damaged {
                let path = catalog_path(index.path()).unwrap();
                let original: EmbeddingCatalog = read_json_file(&path).unwrap();
                for corruption in ["schema", "model", "path", "missing", "identity"] {
                    let mut catalog = original.clone();
                    let mut manifest = old.clone();
                    match corruption {
                        "schema" => catalog.schema_version += 1,
                        "model" => catalog.models.clear(),
                        "path" => catalog.models[0].manifest_url = "../manifest.json".to_owned(),
                        "identity" => manifest.pack_id.push_str("-wrong"),
                        "missing" => {}
                        _ => unreachable!(),
                    }
                    write_json_file(&path, &catalog).unwrap();
                    let manifest_path = old_pack.join("manifest.json");
                    write_json_file(&manifest_path, &manifest).unwrap();
                    if corruption == "missing" {
                        fs::remove_file(&manifest_path).unwrap();
                    }
                    assert!(native::setup_embeddings(&options).is_err(), "{corruption}");
                    assert!(
                        server.progress.lock().unwrap().requests.is_empty(),
                        "{corruption}"
                    );
                }
                write_json_file(&path, &original).unwrap();
                write_json_file(&old_pack.join("manifest.json"), &old).unwrap();
            }
            let report = native::setup_embeddings(&options).unwrap();
            assert_eq!(report.pack_id, desired.pack_id);
            assert_eq!(report.index_source, SetupIndexSource::DownloadedPrecomputed);
            let manifest_url = format!(
                "models/{}/packs/{}/manifest.json",
                spec.model_key, desired.pack_id
            );
            let prefix = manifest_url.strip_suffix("manifest.json").unwrap();
            let changed_corpus = desired
                .corpora
                .iter()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap();
            let mut expected = vec![
                "catalog.json".to_owned(),
                manifest_url.clone(),
                format!("{prefix}{}", changed_corpus.items_url),
                format!("{prefix}{}", changed_corpus.shards[0].url),
            ];
            if damaged {
                expected.push(format!("{prefix}{}", unchanged.shards[0].url));
            }
            let mut fetched = server.progress.lock().unwrap().requests.clone();
            expected.sort();
            fetched.sort();
            assert_eq!(fetched, expected, "{changed}, damaged={damaged}");
            let installed = pack_root(index.path(), &spec.model_key, &desired.pack_id);
            for relative in native_pack_remote_paths(&desired).unwrap() {
                assert_eq!(
                    sha256_hex_file(&installed.join(&relative)).unwrap(),
                    sha256_hex_file(&remote_pack.join(&relative)).unwrap()
                );
            }
            validate_pack_dir(&installed).unwrap();
            assert_eq!(
                read_latest_pack_manifest(index.path(), &spec.model_key)
                    .unwrap()
                    .1
                    .pack_id,
                desired.pack_id
            );
        }
    }
    server
        .shutdown()
        .expect("the pack server thread finishes cleanly");
}
