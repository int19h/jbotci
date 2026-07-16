//! Transport-agnostic analysis snapshots for editor and indexing integrations.

mod line_index;
mod snapshot;

pub use line_index::{LineIndex, Position, PositionEncoding, PositionRange, TextOffsets};
pub use snapshot::{DocumentSnapshot, ResolvedDiagnostic, ResolvedLabel, WordAt};
