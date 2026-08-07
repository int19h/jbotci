//! Shared compilation view of the exact v0 syntax kernel for bundle minting.
//!
//! The module nesting mirrors `src/notation` exactly — `kernel` beside an
//! inline `sexpr` — so every relative `super::super::` path inside the shared
//! sources resolves the same way in the build script as it does in the crate.

#![allow(dead_code)]
// The build script compiles only the syntax and type surfaces it mints from, so
// the kernel's own convenience re-exports are unused here.
#![allow(unused_imports)]

#[path = "../src/notation/kernel/mod.rs"]
pub mod kernel;

#[path = "smusni_v0_sexpr.rs"]
pub mod sexpr;
