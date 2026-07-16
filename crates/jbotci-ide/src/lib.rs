//! Transport-agnostic analysis snapshots for editor and indexing integrations.

mod line_index;
mod snapshot;

pub use line_index::{
    LineIndex, MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange, TextOffsets,
};
pub use snapshot::{
    DocumentSnapshot, HoverContent, ResolvedDiagnostic, ResolvedLabel, SemanticToken,
    SemanticTokenKind, WordAt,
};
