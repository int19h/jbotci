//! The kernel construction failure.
//!
//! Every kernel constructor is fallible, because rejecting an ill-typed
//! composition is the whole point of the model. A route that the coordinate
//! registry declares carried but which fails to construct is an implementation
//! invariant violation rather than a language gap, so the message has to say
//! what the kernel refused rather than only that it refused.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use super::apply::ApplicationTypeError;
use super::types::TypeParseError;

/// A typed-construction failure with stable human-readable context.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelTypeError {
    message: String,
}

impl KernelTypeError {
    /// Construct a nonempty construction failure.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    pub(super) fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(
            !message.is_empty(),
            "kernel construction errors require a message"
        );
        new!(KernelTypeError { message })
    }

    /// Borrow the failure text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for KernelTypeError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for KernelTypeError {}

impl From<ApplicationTypeError> for KernelTypeError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: ApplicationTypeError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<TypeParseError> for KernelTypeError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: TypeParseError) -> Self {
        Self::new(error.to_string())
    }
}
