//! Experimental typed human-readable S-expression notation.

pub mod datum;
pub mod elaborate;
pub mod planner;
pub mod renderer;
pub mod structural;

pub use datum::{Datum, ParseError, parse_document};
pub use renderer::{DocumentMode, SmusniRender, SmusniRenderStats, render_document};
pub use structural::word_card_datum;
