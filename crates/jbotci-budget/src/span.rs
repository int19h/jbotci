//! One budget for a bounded document.
//!
//! A fit decision reads the original span and does not copy it. A copy is at
//! most the remaining budget plus one byte. `charge` accumulates one document.
//! It does not restart on each leaf.
use crate::{ByteLimit, OutputLimitError, bounded_json, bounded_json_size};
use bityzba::{data, invariant, new, requires};
use std::cell::Cell;
use std::rc::Rc;

#[invariant(*charged <= limit.get())]
#[derive(Debug)]
pub struct SpanBudget {
    limit: ByteLimit,
    charged: usize,
    examined: usize,
}

impl SpanBudget {
    #[requires(true)]
    #[ensures(ret.remaining() == limit.get())]
    #[ensures(ret.charged() == 0)]
    pub fn new(limit: ByteLimit) -> Self {
        new!(Self {
            limit,
            charged: 0,
            examined: 0,
        })
    }

    #[requires(true)]
    #[ensures(ret == self.limit.get() - self.charged)]
    pub fn remaining(&self) -> usize {
        self.limit.get() - self.charged
    }

    #[requires(true)]
    #[ensures(ret == self.limit)]
    pub fn limit(&self) -> ByteLimit {
        self.limit
    }

    #[requires(true)]
    #[ensures(ret == self.charged)]
    pub fn charged(&self) -> usize {
        self.charged
    }

    #[requires(true)]
    #[ensures(ret == self.examined)]
    pub fn examined(&self) -> usize {
        self.examined
    }

    /// `text` fits in the bytes still available. This does not copy `text`.
    #[requires(true)]
    #[ensures(ret == (text.len() <= self.remaining()))]
    pub fn fits(&self, text: &str) -> bool {
        text.len() <= self.remaining()
    }

    /// Add `bytes` to this document. The same limit covers every later charge.
    #[requires(true)]
    #[ensures(ret.is_ok() == (bytes <= old(self.remaining())))]
    #[ensures(ret.is_err() || self.charged() == old(self.charged()) + bytes)]
    pub fn charge(&mut self, bytes: usize) -> Result<(), OutputLimitError> {
        let limit = self.limit;
        match self.charged.checked_add(bytes) {
            Some(next) if next <= limit.get() => {
                *self = std::mem::replace(self, SpanBudget::new(limit))
                    .with_data(data! { charged: next });
                Ok(())
            }
            _ => Err(OutputLimitError { limit }),
        }
    }

    /// Copy `text` after it fits. The copy length is the original length.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|copied| copied.len() == text.len()) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|copied| copied.len() <= old(self.remaining()).saturating_add(1)) || ret.is_err())]
    pub fn copy(&mut self, text: &str) -> Result<String, OutputLimitError> {
        if !self.fits(text) {
            return Err(OutputLimitError { limit: self.limit });
        }
        self.charge(text.len())?;
        Ok(text.to_owned())
    }

    /// Measure `text` after edge whitespace is removed, then copy that span.
    ///
    /// The trim is a borrow. An oversized trimmed span copies nothing, so a
    /// later trim cannot drop the byte that shows the span was too long.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|copied| copied.as_str() == text.trim()) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|copied| copied.len() <= old(self.remaining()).saturating_add(1)) || ret.is_err())]
    pub fn copy_trimmed(&mut self, text: &str) -> Result<String, OutputLimitError> {
        self.copy(text.trim())
    }

    /// Copy at most the remaining budget plus one byte, on a character boundary.
    ///
    /// The decision uses `text` before the copy. The result is not trimmed.
    #[requires(true)]
    #[ensures(ret.len() <= text.len())]
    #[ensures(ret.len() <= old(self.remaining()).saturating_add(1))]
    pub fn copy_bounded(&mut self, text: &str) -> String {
        if self.fits(text) {
            return self.copy(text).expect("a fitting span copies");
        }
        let room = self.remaining().saturating_add(1).min(text.len());
        let mut end = room;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        let prefix = &text[..end];
        let charge = prefix.len().min(self.remaining());
        let _ = self.charge(charge);
        prefix.to_owned()
    }

    /// Borrow `source[start..end]` when `start` and `end` are byte indexes.
    ///
    /// The work counter grows by the span length. It does not walk from the
    /// first character of `source`.
    #[requires(true)]
    #[ensures(ret.is_some_and(|slice| slice.len() == end - start) || ret.is_none())]
    pub fn slice<'source>(
        &mut self,
        source: &'source str,
        start: usize,
        end: usize,
    ) -> Option<&'source str> {
        if start > end || end > source.len() {
            return None;
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return None;
        }
        let span = end - start;
        let examined = self.examined.saturating_add(span);
        let limit = self.limit;
        *self =
            std::mem::replace(self, SpanBudget::new(limit)).with_data(data! { examined: examined });
        record_slice_work(span);
        source.get(start..end)
    }

    /// Charge the serialized form of `value` against this document.
    ///
    /// The serializer stops when the encoded document passes the remaining
    /// budget. Leaf text is not given a fresh limit.
    #[requires(true)]
    #[ensures(true)]
    pub fn charge_json<T: serde::Serialize>(&mut self, value: &T) -> Result<(), OutputLimitError> {
        let room = self.remaining();
        let Some(limit) = ByteLimit::new(room) else {
            return Err(OutputLimitError { limit: self.limit });
        };
        let size =
            bounded_json_size(value, limit).map_err(|_| OutputLimitError { limit: self.limit })?;
        self.charge(size)
    }

    /// Collect the serialized form when it fits in the remaining budget.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|bytes| bytes.len() <= old(self.remaining())) || ret.is_err())]
    pub fn collect_json<T: serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<Vec<u8>, serde_json::Error> {
        let room = self.remaining();
        let Some(limit) = ByteLimit::new(room) else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                OutputLimitError { limit: self.limit },
            )));
        };
        let bytes = bounded_json(value, limit)?;
        self.charge(bytes.len()).map_err(|error| {
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::FileTooLarge, error))
        })?;
        Ok(bytes)
    }
}

thread_local! {
    static SLICE_WORK: Cell<usize> = const { Cell::new(0) };
}

#[requires(true)]
#[ensures(true)]
fn record_slice_work(bytes: usize) {
    SLICE_WORK.with(|work| work.set(work.get().saturating_add(bytes)));
}

/// Bytes examined by [`SpanBudget::slice`] since the last reset.
#[requires(true)]
#[ensures(true)]
pub fn slice_work() -> usize {
    SLICE_WORK.with(|work| work.get())
}

#[requires(true)]
#[ensures(slice_work() == 0)]
pub fn reset_slice_work() {
    SLICE_WORK.with(|work| work.set(0));
}

/// Reject a leaf that does not fit, or copy a short preview prefix.
#[invariant(::Reject { max_bytes } => *max_bytes > 0)]
#[invariant(::Preview { max_chars } => *max_chars > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafCopyPolicy {
    /// The leaf must fit in this many bytes. A longer leaf stops the render.
    Reject { max_bytes: usize },
    /// Copy at most this many characters plus one. The copy does not stop the render.
    Preview { max_chars: usize },
}

#[invariant(noted.get().is_none_or(|bytes| bytes > 0))]
#[derive(Debug)]
struct LeafLedger {
    noted: Cell<Option<usize>>,
    depth: Cell<usize>,
}

/// One leaf policy and the overflow note for the builds that share it.
///
/// A preview value keeps a separate note.
#[invariant(matches!(*policy.as_data(), LeafCopyPolicyData::Reject { max_bytes } if max_bytes > 0) || matches!(*policy.as_data(), LeafCopyPolicyData::Preview { max_chars } if max_chars > 0))]
#[invariant(ledger.noted.get().is_none_or(|bytes| bytes > 0))]
#[derive(Debug, Clone)]
pub struct LeafCopy {
    policy: LeafCopyPolicy,
    ledger: Rc<LeafLedger>,
}

/// The outermost build reports a noted overflow. An inner build leaves it.
///
/// This guard is a tuple. A named field needs an invariant.
/// Drop on that wrapper does not compile.
#[derive(Debug)]
pub struct LimitScope(Rc<LeafLedger>);

impl LeafCopy {
    #[requires(true)]
    #[ensures(ret.is_some())]
    fn fresh(policy: LeafCopyPolicy) -> Option<Self> {
        Some(new!(Self {
            policy,
            ledger: Rc::new(new!(LeafLedger {
                noted: Cell::new(None),
                depth: Cell::new(0),
            })),
        }))
    }

    /// A reject policy for `max_bytes`. Zero is not a budget.
    #[requires(true)]
    #[ensures(ret.is_some() == (max_bytes > 0))]
    pub fn reject(max_bytes: usize) -> Option<Self> {
        let limit = ByteLimit::new(max_bytes)?;
        let policy = new!(LeafCopyPolicy::Reject {
            max_bytes: limit.get(),
        });
        Self::fresh(policy)
    }

    /// The reject policy for an existing byte budget.
    #[requires(true)]
    #[ensures(true)]
    pub fn reject_limit(limit: ByteLimit) -> Self {
        Self::reject(limit.get()).expect("a byte limit is positive")
    }

    /// A reject policy that accepts every leaf.
    #[requires(true)]
    #[ensures(true)]
    pub fn unlimited() -> Self {
        Self::reject(usize::MAX).expect("the maximum byte count is positive")
    }

    /// A preview policy for `max_chars`. Zero is not a budget.
    #[requires(true)]
    #[ensures(ret.is_some() == (max_chars > 0))]
    pub fn preview(max_chars: usize) -> Option<Self> {
        let limit = ByteLimit::new(max_chars)?;
        let policy = new!(LeafCopyPolicy::Preview {
            max_chars: limit.get(),
        });
        Self::fresh(policy)
    }

    /// Start one build on this shared note.
    ///
    /// The first build clears an old note. A nested build keeps it.
    #[requires(true)]
    #[ensures(true)]
    pub fn enter(&self) -> LimitScope {
        let depth = self.ledger.depth.get();
        if depth == 0 {
            self.ledger.noted.set(None);
        }
        self.ledger.depth.set(depth + 1);
        LimitScope(Rc::clone(&self.ledger))
    }

    /// Record the first reject overflow. A preview records nothing.
    #[requires(true)]
    #[ensures(true)]
    pub fn note(&self, limit: ByteLimit) {
        if !matches!(*self.policy.as_data(), LeafCopyPolicyData::Reject { .. }) {
            return;
        }
        if self.ledger.noted.get().is_none() {
            self.ledger.noted.set(Some(limit.get()));
        }
    }

    /// Copy `text` after edge whitespace is removed.
    ///
    /// A reject copy is the whole trimmed span, or an error. A preview copy is
    /// a prefix of the trimmed span. The prefix is one character past the budget.
    #[requires(true)]
    #[ensures(ret.is_ok() || matches!(*self.policy.as_data(), LeafCopyPolicyData::Reject { .. }))]
    #[ensures(match (self.policy.as_data(), &ret) {
        (LeafCopyPolicyData::Preview { max_chars }, Ok(copied)) => {
            copied.chars().count() <= max_chars.saturating_add(1)
        }
        (LeafCopyPolicyData::Reject { .. }, Ok(copied)) => copied.len() <= text.trim().len(),
        (_, Err(_)) => true,
    })]
    pub fn copy_trimmed(&self, text: &str) -> Result<String, OutputLimitError> {
        let trimmed = text.trim();
        match *self.policy.as_data() {
            LeafCopyPolicyData::Preview { max_chars } => {
                Ok(char_prefix(trimmed, max_chars.saturating_add(1)))
            }
            LeafCopyPolicyData::Reject { .. } => self.copy_reject(trimmed),
        }
    }

    /// Copy `text` with no trim.
    ///
    /// A preview keeps line breaks in the prefix. The prefix length is the
    /// character budget plus one.
    #[requires(true)]
    #[ensures(ret.is_ok() || matches!(*self.policy.as_data(), LeafCopyPolicyData::Reject { .. }))]
    #[ensures(match (self.policy.as_data(), &ret) {
        (LeafCopyPolicyData::Preview { max_chars }, Ok(copied)) => {
            copied.chars().count() <= max_chars.saturating_add(1)
        }
        (LeafCopyPolicyData::Reject { .. }, Ok(copied)) => copied.len() == text.len(),
        (_, Err(_)) => true,
    })]
    pub fn copy_exact(&self, text: &str) -> Result<String, OutputLimitError> {
        match *self.policy.as_data() {
            LeafCopyPolicyData::Preview { max_chars } => {
                Ok(char_prefix(text, max_chars.saturating_add(1)))
            }
            LeafCopyPolicyData::Reject { .. } => self.copy_reject(text),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|copied| copied.len() == text.len()) || ret.is_err())]
    fn copy_reject(&self, text: &str) -> Result<String, OutputLimitError> {
        let LeafCopyPolicyData::Reject { max_bytes } = *self.policy.as_data() else {
            return Err(OutputLimitError {
                limit: ByteLimit::new(usize::MAX).expect("the maximum byte count is positive"),
            });
        };
        let limit = ByteLimit::new(max_bytes).expect("a reject budget is positive");
        let mut budget = SpanBudget::new(limit);
        match budget.copy(text) {
            Ok(copied) => Ok(copied),
            Err(error) => {
                self.note(error.limit);
                Err(error)
            }
        }
    }
}

impl LimitScope {
    #[requires(true)]
    #[ensures(true)]
    pub fn failure(&self) -> Option<OutputLimitError> {
        if self.0.depth.get() != 1 {
            return None;
        }
        let bytes = self.0.noted.take()?;
        ByteLimit::new(bytes).map(|limit| OutputLimitError { limit })
    }
}

impl Drop for LimitScope {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let depth = self.0.depth.get();
        self.0.depth.set(depth.saturating_sub(1));
    }
}

#[requires(true)]
#[ensures(ret.chars().count() <= max_chars)]
fn char_prefix(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for character in text.chars().take(max_chars) {
        output.push(character);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_trimmed_quote_is_measured_before_it_is_copied() {
        let mut budget = SpanBudget::new(ByteLimit::new(4_096).unwrap());
        let quote = format!("a{}b", " ".repeat(8_192));
        assert!(budget.copy_trimmed(&quote).is_err());
        assert_eq!(budget.charged(), 0);
        // A prefix of this quote ends inside the spaces. Trimming that prefix
        // would drop the overflow. The original trimmed span is measured first.
        let cut = format!("a{}b", " ".repeat(4_096));
        assert!(budget.copy_trimmed(&cut).is_err());
        assert_eq!(budget.charged(), 0);
        assert_eq!(budget.copy_trimmed("a b").unwrap(), "a b");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_bounded_copy_stops_one_byte_past_the_budget() {
        let mut budget = SpanBudget::new(ByteLimit::new(4).unwrap());
        let copied = budget.copy_bounded("abcdef");
        assert_eq!(copied, "abcde");
        assert!(copied.len() <= 5);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_charges_add_up_across_leaves() {
        let mut budget = SpanBudget::new(ByteLimit::new(16).unwrap());
        budget.charge(2).unwrap();
        budget.charge(8).unwrap();
        assert!(budget.charge(8).is_err());
        assert_eq!(budget.charged(), 10);
    }

    struct SerializeProbe<'probe>(&'probe std::cell::Cell<bool>);

    impl serde::Serialize for SerializeProbe<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.set(true);
            0u8.serialize(serializer)
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_exhausted_budget_rejects_json_without_serializing() {
        let mut budget = SpanBudget::new(ByteLimit::new(1).unwrap());
        budget.charge(1).unwrap();
        let error = budget.collect_json(&0).expect_err("no room remains");
        assert_eq!(
            error.io_error_kind(),
            Some(std::io::ErrorKind::FileTooLarge)
        );
        assert_eq!(budget.charged(), 1);
        let saw = std::cell::Cell::new(false);
        let probe = SerializeProbe(&saw);
        assert!(budget.collect_json(&probe).is_err());
        assert!(!saw.get(), "an exhausted budget must not serialize");
        assert!(budget.charge_json(&probe).is_err());
        assert!(!saw.get(), "an exhausted budget must not measure JSON");
        assert!(!budget.fits("a"));
        assert!(budget.fits(""));
        assert!(budget.copy("a").is_err());
        assert!(budget.copy_trimmed(" a ").is_err());
        assert!(budget.charge(0).is_ok());
        assert!(budget.charge(1).is_err());
        let copied = budget.copy_bounded("abcdef");
        assert!(copied.len() <= 1, "{copied}");
        assert_eq!(budget.charged(), 1);
        assert_eq!(budget.slice("abcdef", 1, 4), Some("bcd"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_preview_prefix_does_not_mark_a_reject_build() {
        let reject = LeafCopy::reject(4).expect("positive reject budget");
        {
            let outer = reject.enter();
            {
                let inner = reject.enter();
                assert!(reject.copy_exact("abcdef").is_err());
                assert!(inner.failure().is_none());
            }
            let error = outer
                .failure()
                .expect("the outer build reports the overflow");
            assert_eq!(error.limit.get(), 4);
        }

        let again = reject.enter();
        let preview = LeafCopy::preview(3).expect("positive preview budget");
        let copied = preview
            .copy_trimmed(&"a".repeat(200))
            .expect("a preview copies a prefix");
        assert_eq!(copied, "a".repeat(4));
        assert_eq!(
            preview.copy_exact("\n\n\n\n").expect("a span keeps breaks"),
            "\n\n\n\n"
        );
        assert!(again.failure().is_none());
        assert!(LeafCopy::reject(0).is_none());
        assert!(LeafCopy::preview(0).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_byte_slice_charges_the_span_length() {
        reset_slice_work();
        let source = "abcdefghijklmnopqrstuvwxyz";
        let mut budget = SpanBudget::new(ByteLimit::new(64).unwrap());
        assert_eq!(budget.slice(source, 3, 7), Some("defg"));
        assert_eq!(budget.slice(source, 20, 24), Some("uvwx"));
        assert_eq!(budget.examined(), 8);
        assert_eq!(slice_work(), 8);
        assert!(slice_work() < 3 + 7 + 20 + 24);
    }
}
