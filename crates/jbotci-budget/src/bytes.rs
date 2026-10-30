//! Byte budgets checked before output buffers grow.
use bityzba::{invariant, new, requires};
use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use thiserror::Error;

#[invariant(*bytes > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteLimit {
    bytes: usize,
}
impl ByteLimit {
    #[requires(true)]
    #[ensures(ret.is_some() == (bytes > 0))]
    pub fn new(bytes: usize) -> Option<Self> {
        (bytes > 0).then(|| new!(ByteLimit { bytes }))
    }
    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn get(self) -> usize {
        self.bytes
    }
}

#[invariant(used.load(Ordering::Relaxed) <= limit.get())]
#[derive(Debug)]
struct BudgetState {
    limit: ByteLimit,
    used: AtomicUsize,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub struct OutputBudget {
    state: Arc<BudgetState>,
}
impl OutputBudget {
    #[requires(true)]
    #[ensures(ret.remaining() == limit.get())]
    pub fn new(limit: ByteLimit) -> Self {
        Self {
            state: Arc::new(new!(BudgetState {
                limit,
                used: AtomicUsize::new(0)
            })),
        }
    }
    #[requires(true)]
    #[ensures(ret <= self.state.limit.get())]
    pub fn remaining(&self) -> usize {
        self.state.limit.get() - self.state.used.load(Ordering::Acquire)
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn charge(&self, bytes: usize) -> Result<(), OutputLimitError> {
        self.state
            .used
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes)
                    .filter(|next| *next <= self.state.limit.get())
            })
            .map(|_| ())
            .map_err(|_| OutputLimitError {
                limit: self.state.limit,
            })
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn writer(&self) -> BudgetedWriter {
        BudgetedWriter {
            state: Some(new!(WriterState {
                bytes: Vec::new(),
                budget: self.clone()
            })),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Error)]
#[error("The generated output exceeds the {}-byte limit.", limit.get())]
pub struct OutputLimitError {
    pub limit: ByteLimit,
}

#[invariant(bytes.len() <= budget.state.limit.get())]
#[derive(Debug)]
struct WriterState {
    bytes: Vec<u8>,
    budget: OutputBudget,
}

/// The optional state lets writes move the validated buffer without copying it.
/// A panic closes the writer and releases its buffer.
#[invariant(true)]
#[derive(Debug)]
pub struct BudgetedWriter {
    state: Option<WriterState>,
}
impl BudgetedWriter {
    #[requires(true)]
    #[ensures(ret.len() <= old(self.state.as_ref().map_or(0, |state| state.budget.state.limit.get())))]
    pub fn into_bytes(self) -> Vec<u8> {
        self.state
            .map_or_else(Vec::new, |state| state.into_data().bytes)
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn into_string(self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.into_bytes())
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn repeat(&mut self, byte: u8, count: usize) -> io::Result<()> {
        let Some(state) = self.state.take() else {
            return Err(io::Error::other("output writer is closed"));
        };
        let mut state = state.into_data();
        let result = (|| {
            state.budget.charge(count).map_err(io::Error::other)?;
            state
                .bytes
                .try_reserve_exact(count)
                .map_err(io::Error::other)?;
            state.bytes.resize(state.bytes.len() + count, byte);
            Ok(())
        })();
        self.state = Some(WriterState::from_data(state));
        result
    }

    #[requires(true)]
    #[ensures(self.state.is_none() -> ret.is_empty())]
    pub fn as_bytes(&self) -> &[u8] {
        self.state
            .as_ref()
            .map_or(&[], |state| state.bytes.as_slice())
    }
}
impl Write for BudgetedWriter {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|written| *written == bytes.len()) || ret.is_err())]
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(state) = self.state.take() else {
            return Err(io::Error::other("output writer is closed"));
        };
        let mut state = state.into_data();
        let result = (|| {
            state.budget.charge(bytes.len()).map_err(io::Error::other)?;
            state
                .bytes
                .try_reserve_exact(bytes.len())
                .map_err(io::Error::other)?;
            state.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        })();
        self.state = Some(WriterState::from_data(state));
        result
    }
    #[requires(true)]
    #[ensures(ret.is_ok() == self.state.is_some())]
    fn flush(&mut self) -> io::Result<()> {
        if self.state.is_some() {
            Ok(())
        } else {
            Err(io::Error::other("output writer is closed"))
        }
    }
}

impl std::fmt::Write for BudgetedWriter {
    #[requires(true)]
    #[ensures(true)]
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.write_all(text.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn bounded_format(arguments: std::fmt::Arguments<'_>, limit: ByteLimit) -> io::Result<String> {
    let mut output = OutputBudget::new(limit).writer();
    std::io::Write::write_fmt(&mut output, arguments)?;
    Ok(output.into_string().expect("formatting writes UTF-8"))
}

impl BudgetedWriter {
    #[requires(true)]
    #[ensures(self.state.as_ref().is_none_or(|state| ret <= state.budget.state.limit.get()))]
    pub fn remaining(&self) -> usize {
        self.state
            .as_ref()
            .map_or(0, |state| state.budget.remaining())
    }

    #[requires(true)]
    #[ensures(ret.is_none() == self.as_bytes().is_empty())]
    pub fn last_byte(&self) -> Option<u8> {
        self.as_bytes().last().copied()
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() <= limit.get()) || ret.is_err())]
pub fn bounded_json<T: serde::Serialize>(
    value: &T,
    limit: ByteLimit,
) -> Result<Vec<u8>, serde_json::Error> {
    let budget = OutputBudget::new(limit);
    let mut writer = budget.writer();
    let serializer = serde_json::Serializer::new(&mut writer);
    let mut serializer = serializer;
    let serializer = serde_stacker::Serializer::new(&mut serializer);
    value.serialize(serializer)?;
    Ok(writer.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn utf8_and_diagnostics_share_one_exact_byte_budget() {
        let budget = OutputBudget::new(ByteLimit::new(6).unwrap());
        let mut stdout = budget.writer();
        let mut stderr = budget.writer();
        stdout.write_all("😀".as_bytes()).unwrap();
        stderr.write_all(b"ok").unwrap();
        assert_eq!(budget.remaining(), 0);
        let stdout_capacity = stdout.state.as_ref().unwrap().bytes.capacity();
        assert!(stdout.write_all(b"x").is_err());
        assert_eq!(
            stdout.state.as_ref().unwrap().bytes.capacity(),
            stdout_capacity
        );
        assert_eq!(stdout.into_bytes(), "😀".as_bytes());
        assert_eq!(stderr.into_bytes(), b"ok");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_charges_escaping_before_the_encoded_buffer_grows() {
        assert_eq!(
            bounded_json(&"\n", ByteLimit::new(4).unwrap()).unwrap(),
            b"\"\\n\""
        );
        assert!(bounded_json(&"\n", ByteLimit::new(3).unwrap()).is_err());
        assert_eq!(
            bounded_json(&"😀", ByteLimit::new(6).unwrap())
                .unwrap()
                .len(),
            6
        );
        assert!(bounded_json(&"😀", ByteLimit::new(5).unwrap()).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_formatting_stops_at_the_exact_utf8_boundary() {
        let expected = "[\n    \"😀\",\n]";
        let value = ["😀"];
        assert_eq!(
            bounded_format(
                format_args!("{value:#?}"),
                ByteLimit::new(expected.len()).unwrap()
            )
            .unwrap(),
            expected
        );
        assert!(
            bounded_format(
                format_args!("{value:#?}"),
                ByteLimit::new(expected.len() - 1).unwrap()
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn giant_repetition_is_refused_in_a_constrained_child() {
        if std::env::var_os("JBOTCI_BUDGET_CHILD").is_some() {
            let mut writer = OutputBudget::new(ByteLimit::new(8).unwrap()).writer();
            assert!(writer.repeat(b' ', usize::MAX / 2).is_err());
            assert_eq!(writer.state.as_ref().unwrap().bytes.capacity(), 0);
            assert!(writer.into_bytes().is_empty());
            return;
        }
        let status = std::process::Command::new("bash")
            .arg("-c")
            .arg("ulimit -v 524288; exec \"$1\" --exact bytes::tests::giant_repetition_is_refused_in_a_constrained_child --nocapture")
            .arg("budget-test")
            .arg(std::env::current_exe().unwrap())
            .env("JBOTCI_BUDGET_CHILD", "1")
            .status().unwrap();
        assert!(
            status.success(),
            "bounded repetition failed under the child memory limit"
        );
    }
}
