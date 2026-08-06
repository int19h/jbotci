//! The typed smusni kernel.
//!
//! The kernel is the semantic core of the notation: a strongly typed value
//! model whose constructors validate through the application kernels in
//! [`apply`], so a constructed value is well-typed by construction rather than
//! by a later acceptance parse. It is deliberately notation-independent —
//! nothing under this module mentions `Datum` or any other serialization — so
//! that the version-0 S-expression printer and any future notation are printers
//! over the same value.

pub mod apply;
pub mod lexicon;
pub mod types;
