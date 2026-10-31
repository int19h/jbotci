//! Bounded text preparation for renderers with infallible formatting callbacks.
use crate::{ByteLimit, OutputBudget, OutputLimitError};
use bityzba::{invariant, new, requires};
use std::{
    fmt,
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
    #[ensures(ret == self.limit)]
    pub fn limit(&self) -> ByteLimit {
        self.limit
    }
    #[requires(true)]
    #[ensures(self.is_exceeded())]
    pub fn reject(&self) {
        self.failed.store(true, Ordering::Release);
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn buffer(&self) -> TextBuffer {
        TextBuffer {
            state: Some(new!(TextState {
                text: String::new(),
                budget: OutputBudget::new(self.limit)
            })),
            failed: Arc::clone(&self.failed),
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
        if fmt::write(&mut output, arguments).is_err() {
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
        if self.is_exceeded() || text.is_empty() || count == 0 {
            return String::new();
        }
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
    pub fn join<T: AsRef<str>>(
        &self,
        pieces: impl IntoIterator<Item = T>,
        separator: &str,
    ) -> String {
        let mut output = self.buffer();
        let mut pieces = pieces.into_iter();
        let mut first = true;
        while !self.is_exceeded() {
            let Some(piece) = pieces.next() else {
                break;
            };
            if !first {
                output.push_str(separator);
            }
            first = false;
            output.push_str(piece.as_ref());
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

#[invariant(text.len() <= budget.used())]
#[invariant(text.capacity() <= budget.limit().get())]
#[derive(Debug)]
struct TextState {
    text: String,
    budget: OutputBudget,
}

/// The optional state closes a buffer after a panic without copying its text.
#[invariant(true)]
#[derive(Debug)]
pub struct TextBuffer {
    state: Option<TextState>,
    failed: Arc<AtomicBool>,
}
impl TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    pub fn push_str(&mut self, text: &str) {
        if self.failed.load(Ordering::Acquire) {
            return;
        }
        let Some(state) = self.state.take() else {
            self.failed.store(true, Ordering::Release);
            return;
        };
        let mut state = state.into_data();
        let result = (|| {
            state.budget.charge(text.len())?;
            let needed = state.text.len() + text.len();
            if needed > state.text.capacity() {
                let capacity = needed
                    .max(state.text.capacity().saturating_mul(2))
                    .min(state.budget.limit().get());
                if state
                    .text
                    .try_reserve_exact(capacity - state.text.len())
                    .is_err()
                {
                    return Err(OutputLimitError {
                        limit: state.budget.limit(),
                    });
                }
            }
            state.text.push_str(text);
            Ok(())
        })();
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        self.state = Some(TextState::from_data(state));
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn push(&mut self, character: char) {
        let mut bytes = [0; 4];
        self.push_str(character.encode_utf8(&mut bytes));
    }
    #[requires(true)]
    #[ensures(ret.len() <= old(self.state.as_ref().map_or(0, |state| state.budget.limit().get())))]
    pub fn into_piece(self) -> String {
        self.state
            .map_or_else(String::new, |state| state.into_data().text)
    }
    #[requires(self.state.is_some())]
    #[ensures(true)]
    pub fn scope(&self) -> TextScope {
        TextScope {
            limit: self
                .state
                .as_ref()
                .expect("open text buffer")
                .budget
                .limit(),
            failed: Arc::clone(&self.failed),
        }
    }
    #[requires(self.state.is_some())]
    #[ensures(true)]
    pub fn take_piece(&mut self) -> String {
        let replacement = self.scope().buffer();
        std::mem::replace(self, replacement).into_piece()
    }
    #[requires(true)]
    #[ensures(true)]
    pub fn pop(&mut self) -> Option<char> {
        let state = self.state.take()?;
        let mut state = state.into_data();
        let character = state.text.pop();
        if let Some(character) = character {
            state.budget.release(character.len_utf8());
        }
        self.state = Some(TextState::from_data(state));
        character
    }
}
impl std::ops::Deref for TextBuffer {
    type Target = str;
    #[requires(true)]
    #[ensures(true)]
    fn deref(&self) -> &str {
        self.state.as_ref().map_or("", |state| state.text.as_str())
    }
}
impl fmt::Write for TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.push_str(text);
        if self.failed.load(Ordering::Acquire) {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}
impl fmt::Display for TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self)
    }
}
impl AsRef<str> for TextBuffer {
    #[requires(true)]
    #[ensures(true)]
    fn as_ref(&self) -> &str {
        self
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
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn removing_a_character_preserves_the_exact_utf8_boundary() {
        let scope = TextScope::new(ByteLimit::new(5).unwrap());
        let mut text = scope.buffer();
        text.push_str("ab\\");
        assert_eq!(text.pop(), Some('\\'));
        text.push('…');
        assert_eq!(scope.finish(text.into_piece()).unwrap(), "ab…");
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn joining_stops_requesting_pieces_after_the_first_overflow() {
        let scope = TextScope::new(ByteLimit::new(4).unwrap());
        let pieces = (0..).map(|index| {
            assert!(
                index < 3,
                "the renderer requested a piece after its budget failed"
            );
            "ab"
        });
        assert!(scope.finish(scope.join(pieces, "")).is_err());
    }
}
