//! Byte budgets shared by renderers and server adapters.
mod bytes;
mod json_text;
mod text;
pub use bytes::*;
pub use json_text::{bounded_json_with_text_budget, check_json_text_budget};
pub use text::{TextBuffer, TextScope};

use bityzba::{invariant, requires};

/// Separate limits for human text and an intermediate serialized projection.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderLimits {
    pub text: ByteLimit,
    pub projection: ByteLimit,
}
impl RenderLimits {
    #[requires(true)]
    #[ensures(ret.text == text && ret.projection == projection)]
    pub fn new(text: ByteLimit, projection: ByteLimit) -> Self {
        Self { text, projection }
    }
}
