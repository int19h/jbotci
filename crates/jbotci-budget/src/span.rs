//! One budget for a bounded document.
//!
//! A fit decision reads the original span and does not copy it. A copy is at
//! most the remaining budget plus one byte. `charge` accumulates one document.
//! It does not restart on each leaf.
use crate::{ByteLimit, OutputLimitError, bounded_json, bounded_json_size};
use bityzba::{data, invariant, new, requires};
use std::cell::Cell;

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
            return bounded_json(value, self.limit);
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
    static NOTED_LIMIT: Cell<Option<usize>> = const { Cell::new(None) };
    static BUILD_DEPTH: Cell<usize> = const { Cell::new(0) };
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

/// Remember the first overflow so an outer build can report it.
#[requires(true)]
#[ensures(true)]
pub fn note_limit(limit: ByteLimit) {
    NOTED_LIMIT.with(|noted| {
        if noted.get().is_none() {
            noted.set(Some(limit.get()));
        }
    });
}

/// The outermost build reports a noted overflow. An inner build leaves it.
#[derive(Debug)]
pub struct LimitScope;

impl LimitScope {
    #[requires(true)]
    #[ensures(true)]
    pub fn enter() -> Self {
        BUILD_DEPTH.with(|depth| {
            if depth.get() == 0 {
                NOTED_LIMIT.with(|noted| noted.set(None));
            }
            depth.set(depth.get() + 1);
        });
        Self
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn failure(&self) -> Option<OutputLimitError> {
        let depth = BUILD_DEPTH.with(|depth| depth.get());
        if depth != 1 {
            return None;
        }
        let bytes = NOTED_LIMIT.with(|noted| noted.take())?;
        ByteLimit::new(bytes).map(|limit| OutputLimitError { limit })
    }
}

impl Drop for LimitScope {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        BUILD_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
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
