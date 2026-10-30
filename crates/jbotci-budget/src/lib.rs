//! Byte budgets shared by renderers and server adapters.
mod bytes;
pub use bytes::*;

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
