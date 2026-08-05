//! Experimental typed human-readable S-expression notation.

pub mod datum;
pub mod elaborate;
mod identity;
pub mod planner;
pub mod renderer;
pub mod structural;
pub mod syntax;
pub mod type_system;

pub use datum::{Datum, ParseError, parse_document};
pub use renderer::{
    DocumentMode, SmusniDiagnostic, SmusniDiagnosticData, SmusniRender, SmusniRenderStats,
    render_document,
};
pub use structural::word_card_datum;
pub use syntax::{
    V0Document, V0Expr, V0ParseError, parse_v0_document, parse_v0_expression, parse_v0_expressions,
    print_v0_document,
};
