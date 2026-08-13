#![recursion_limit = "1024"]

//! Transport-agnostic analysis snapshots for editor and indexing integrations.

mod line_index;
mod snapshot;

pub use line_index::{
    LineIndex, MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange, TextOffsets,
};
pub use snapshot::{
    CompletionCancellationToken, CompletionDocumentationHandle, CompletionInterpretation,
    CompletionItem, CompletionKind, CompletionProvenance, DecorationProfile, DiagnosticSnapshot,
    DocumentSnapshot, FoldingRange, FoldingRangeKind, HoverContent, IncrementalAnalysisTimings,
    IncrementalDiagnosticGate, Inlay, InlayKind, InlayOptions, PreparedDocumentAnalysis,
    RawBracketsOptions, ResolvedDiagnostic, ResolvedLabel, SelectionRangeChain, SemanticToken,
    SemanticTokenKind, StructureBracketInlayOptions, StructureConstructFilter, StructureInlay,
    StructureInlayKind, WordAt, completion_documentation_markdown,
};
