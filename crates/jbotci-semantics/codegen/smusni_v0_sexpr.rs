//! Shared compilation view of the version-0 S-expression layer.
//!
//! This shim exists only so the build script sees the same module nesting the
//! crate has: `notation::sexpr::*` beside `notation::kernel`, so the relative
//! `super::super::kernel` paths inside the shared sources resolve identically.

#[path = "../src/notation/sexpr/datum.rs"]
pub mod datum;
#[path = "../src/notation/sexpr/syntax.rs"]
pub mod syntax;
#[path = "../src/notation/sexpr/type_syntax.rs"]
pub mod type_syntax;
