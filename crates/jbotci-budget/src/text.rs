//! Bounded text preparation for renderers with infallible formatting callbacks.
use crate::{BudgetedWriter, ByteLimit, OutputBudget, OutputLimitError};
use bityzba::{invariant, requires};
use std::{
    fmt,
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Every temporary buffer has the same byte bound. One failure rejects the render.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct TextScope {
    limit: ByteLimit,
    failed: Arc<AtomicBool>,
}
impl TextScope {
    #[requires(true)]
    #[ensures(ret.limit == limit)]
    pub fn new(limit: ByteLimit) -> Self {
        Self {
            limit,
            failed: Arc::new(AtomicBool::new(false)),
        }
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn is_exceeded(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn buffer(&self) -> TextBuffer {
        TextBuffer {
            output: OutputBudget::new(self.limit).writer(),
            scope: self.clone(),
        }
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn buffer_with(&self, text: &str) -> TextBuffer {
        let mut output = self.buffer();
        output.push_str(text);
        output
    }
    #[requires(true)]
    #[ensures(ret.len() <= self.limit.get())]
    pub fn format(&self, arguments: fmt::Arguments<'_>) -> String {
        if self.failed.load(Ordering::Acquire) {
            return String::new();
        }
        let mut output = self.buffer();
        if output.output.write_fmt(arguments).is_err() {
            self.failed.store(true, Ordering::Release);
        }
        output.into_piece()
    }
    #[requires(true)]
    #[ensures(ret.len() <= self.limit.get())]
    pub fn copy(&self, text: &str) -> String {
        let mut output = self.buffer();
        output.push_str(text);
        output.into_piece()
    }
    #[requires(true)]
    #[ensures(ret.len() <= self.limit.get())]
    pub fn repeat(&self, text: &str, count: usize) -> String {
        let mut output = self.buffer();
        if text
            .len()
            .checked_mul(count)
            .is_none_or(|bytes| bytes > self.limit.get())
        {
            self.failed.store(true, Ordering::Release);
        } else {
            for _ in 0..count {
                output.push_str(text);
            }
        }
        output.into_piece()
    }
    #[requires(true)]
    #[ensures(ret.len() <= self.limit.get())]
    pub fn join<'a>(&self, pieces: impl IntoIterator<Item = &'a str>, separator: &str) -> String {
        let mut output = self.buffer();
        for (index, piece) in pieces.into_iter().enumerate() {
            if index != 0 {
                output.push_str(separator);
            }
            output.push_str(piece);
        }
        output.into_piece()
    }
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| text.len() <= self.limit.get()) || ret.is_err())]
    pub fn finish(&self, text: String) -> Result<String, OutputLimitError> {
        if self.failed.load(Ordering::Acquire) || text.len() > self.limit.get() {
            Err(OutputLimitError { limit: self.limit })
        } else {
            Ok(text)
        }
    }
}

/// A failed buffer stops accepting text. The scope reports the failure at its boundary.
#[invariant(true)]
#[derive(Debug)]
pub struct TextBuffer {
    output: BudgetedWriter,
    scope: TextScope,
}
impl TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    pub fn push_str(&mut self, text: &str) {
        if !self.scope.failed.load(Ordering::Acquire)
            && self.output.write_all(text.as_bytes()).is_err()
        {
            self.scope.failed.store(true, Ordering::Release);
        }
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn push(&mut self, character: char) {
        let mut bytes = [0; 4];
        self.push_str(character.encode_utf8(&mut bytes));
    }
    #[requires(true)]
    #[ensures(ret.len() <= self.scope.limit.get())]
    pub fn into_piece(self) -> String {
        self.output
            .into_string()
            .expect("text preparation writes UTF-8")
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn scope(&self) -> TextScope {
        self.scope.clone()
    }
}
impl std::ops::Deref for TextBuffer {
    type Target = str;
    #[requires(true)]
    #[ensures(true)]
    fn deref(&self) -> &str {
        std::str::from_utf8(self.output.as_bytes()).expect("text preparation writes UTF-8")
    }
}
impl fmt::Write for TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.push_str(text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_text_refuses_an_overflow_without_a_truncated_success() {
        let scope = TextScope::new(ByteLimit::new(5).unwrap());
        let mut parent = scope.buffer();
        parent.push_str("a");
        parent.push_str(&scope.format(format_args!("{}", "😀")));
        assert_eq!(scope.finish(parent.into_piece()).unwrap(), "a😀");
        let scope = TextScope::new(ByteLimit::new(4).unwrap());
        let mut parent = scope.buffer();
        parent.push_str("a");
        parent.push_str(&scope.format(format_args!("{}", "😀")));
        assert!(scope.finish(parent.into_piece()).is_err());
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_failed_temporary_rejects_the_whole_render() {
        let scope = TextScope::new(ByteLimit::new(4).unwrap());
        let _temporary = scope.copy("12345");
        let mut parent = scope.buffer();
        parent.push_str("ok");
        assert!(scope.finish(parent.into_piece()).is_err());
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joining_charges_utf8_separators() {
        let scope = TextScope::new(ByteLimit::new(6).unwrap());
        assert_eq!(scope.finish(scope.join(["a", "b"], "😀")).unwrap(), "a😀b");
        let scope = TextScope::new(ByteLimit::new(5).unwrap());
        assert!(scope.finish(scope.join(["a", "b"], "😀")).is_err());
    }
}
