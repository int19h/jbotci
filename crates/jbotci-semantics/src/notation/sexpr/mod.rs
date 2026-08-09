//! Experimental typed human-readable S-expression notation.

pub mod datum;
pub mod elaborate;
mod identity;
/// The jbotci-internal raw debug codec. Not smusni, not a public API, and not
/// a compatibility surface; see `docs/smusni/internal-raw.md`.
#[doc(hidden)]
pub mod internal_raw;
pub mod kernel_printer;
pub mod planner;
mod purity;
pub mod renderer;
pub mod structural;
pub mod syntax;
pub mod type_syntax;

pub use datum::{Datum, ParseError, parse_document};
pub use renderer::{
    FailureSpanSource, SmusniDiagnostic, SmusniProjection, SmusniProjectionFailed,
    SmusniProjectionFailure, SmusniRender, SmusniRenderStats, render_document,
};
pub use structural::word_card_datum;
pub use syntax::{
    V0Document, V0Expr, V0ParseError, parse_v0_document, parse_v0_expression, parse_v0_expressions,
    print_v0_document,
};
