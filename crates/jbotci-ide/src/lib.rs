//! Transport-agnostic analysis snapshots for editor and indexing integrations.

mod line_index;
mod snapshot;

pub use line_index::{
    LineIndex, MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange, TextOffsets,
};
pub use snapshot::{
    CompletionCancellationToken, CompletionDocumentationHandle, CompletionInterpretation,
    CompletionItem, CompletionKind, CompletionProvenance, DecorationProfile, DiagnosticSnapshot,
    DocumentSnapshot, HoverContent, IncrementalAnalysisTimings, IncrementalDiagnosticGate,
    PreparedDocumentAnalysis, RawBracketsOptions, ResolvedDiagnostic, ResolvedLabel, SemanticToken,
    SemanticTokenKind, StructureConstructFilter, StructureInlay, StructureInlayKind, WordAt,
    completion_documentation_markdown,
};
