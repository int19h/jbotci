//! Structured source diagnostics shared by parsers, renderers, and fixtures.

use std::ops::Range;

use bityzba::{data, invariant, new, requires};
use jbotci_source::{LineColumn, SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Advice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticPhase {
    Morphology,
    Syntax,
}

pub const DEFAULT_TRACE_LIMIT: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TracePhase {
    Morphology,
    Syntax,
    All,
}

impl TracePhase {
    #[requires(true)]
    #[ensures(matches!(self, Self::All) -> ret)]
    pub fn includes(self, phase: TracePhase) -> bool {
        self == Self::All || self == phase
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceLevel {
    Top,
    Detailed,
    All,
    Primitives,
}

impl TraceLevel {
    #[requires(true)]
    #[ensures((1..=4).contains(&ret))]
    pub fn number(self) -> u8 {
        match self {
            Self::Top => 1,
            Self::Detailed => 2,
            Self::All => 3,
            Self::Primitives => 4,
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|level| level.number() == value) || ret.is_err())]
    pub fn from_number(value: u8) -> Result<Self, TraceOptionError> {
        match value {
            1 => Ok(Self::Top),
            2 => Ok(Self::Detailed),
            3 => Ok(Self::All),
            4 => Ok(Self::Primitives),
            _ => Err(TraceOptionError::InvalidLevel { value }),
        }
    }
}

impl Default for TraceLevel {
    #[requires(true)]
    #[ensures(ret == TraceLevel::Top)]
    fn default() -> Self {
        Self::Top
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::InvalidLevel => true)]
pub enum TraceOptionError {
    #[error("invalid trace level {value}; expected 1, 2, 3, or 4")]
    InvalidLevel { value: u8 },
}

#[invariant(!name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceFilter {
    pub name: String,
}

impl TraceFilter {
    #[requires(!name.is_empty())]
    #[ensures(true)]
    pub fn new(name: String) -> Self {
        new!(TraceFilter { name })
    }
}

#[invariant(*limit > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceOptions {
    pub enabled: bool,
    pub level: TraceLevel,
    pub filter: Option<TraceFilter>,
    pub phase: TracePhase,
    pub limit: usize,
}

impl Default for TraceOptions {
    #[requires(true)]
    #[ensures(!ret.enabled)]
    #[ensures(ret.limit == DEFAULT_TRACE_LIMIT)]
    fn default() -> Self {
        new!(TraceOptions {
            enabled: false,
            level: TraceLevel::Top,
            filter: None,
            phase: TracePhase::All,
            limit: DEFAULT_TRACE_LIMIT,
        })
    }
}

impl TraceOptions {
    #[requires(true)]
    #[ensures(!ret.enabled)]
    pub fn disabled() -> Self {
        Self::default()
    }

    #[requires(limit > 0)]
    #[ensures(ret.enabled)]
    #[ensures(ret.level == level)]
    #[ensures(ret.phase == phase)]
    #[ensures(ret.limit == limit)]
    pub fn enabled(
        level: TraceLevel,
        filter: Option<TraceFilter>,
        phase: TracePhase,
        limit: usize,
    ) -> Self {
        new!(TraceOptions {
            enabled: true,
            level,
            filter,
            phase,
            limit,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_phase(self, phase: TracePhase) -> Self {
        self.with_data(data! { phase: phase })
    }

    #[requires(limit > 0)]
    #[ensures(ret.limit == limit)]
    pub fn with_limit(self, limit: usize) -> Self {
        self.with_data(data! { limit: limit })
    }

    #[requires(true)]
    #[ensures(!self.enabled -> !ret)]
    pub fn includes(&self, phase: TracePhase) -> bool {
        self.enabled && self.phase.includes(phase)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceEventKind {
    ConstructEnter,
    ConstructSuccess,
    ConstructFailure,
    TerminalAttempt,
    TerminalSuccess,
    TerminalFailure,
    Token,
    Save,
    Rewind,
    MorphologyStep,
    MorphologyFailure,
}

#[invariant(*phase != TracePhase::All)]
#[invariant(!label.is_empty())]
#[invariant(*byte_start <= *byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub phase: TracePhase,
    pub level: TraceLevel,
    pub depth: usize,
    pub kind: TraceEventKind,
    pub label: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub detail: Option<String>,
}

#[invariant(!construct.is_empty())]
#[invariant(*byte_start <= *byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub construct: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

impl TraceContext {
    #[requires(!construct.is_empty())]
    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub fn new(construct: String, byte_start: usize, byte_end: usize) -> Self {
        new!(TraceContext {
            construct,
            byte_start,
            byte_end,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TraceFailureBranch {
    pub contexts: Vec<TraceContext>,
    pub expected: Vec<String>,
}

#[invariant(!reason.is_empty())]
#[invariant(*byte_start <= *byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceFailureSummary {
    pub byte_start: usize,
    pub byte_end: usize,
    pub reason: String,
    pub branches: Vec<TraceFailureBranch>,
    pub current_context: Option<TraceContext>,
}

#[invariant(*phase != TracePhase::All)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceReport {
    pub phase: TracePhase,
    pub events: Vec<TraceEvent>,
    pub truncated: bool,
    pub failure: Option<TraceFailureSummary>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
#[invariant(::Active(_) => true)]
pub enum TraceRecorder {
    Disabled,
    Active(Box<TraceRecorderState>),
}

impl Default for TraceRecorder {
    #[requires(true)]
    #[ensures(matches!(ret, TraceRecorder::Disabled))]
    fn default() -> Self {
        Self::Disabled
    }
}

impl TraceRecorder {
    #[requires(phase != TracePhase::All)]
    #[ensures(true)]
    pub fn new(options: TraceOptions, phase: TracePhase) -> Self {
        if options.includes(phase) {
            Self::Active(Box::new(TraceRecorderState::new(options, phase)))
        } else {
            Self::Disabled
        }
    }

    #[requires(true)]
    #[ensures(matches!(self, Self::Disabled) -> !ret)]
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Active(_))
    }

    #[requires(true)]
    #[ensures(matches!(self, Self::Disabled) -> !ret)]
    pub fn should_record(&self, level: TraceLevel, label: &str) -> bool {
        match self {
            Self::Disabled => false,
            Self::Active(state) => state.should_record(level, label),
        }
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub fn record_with_detail(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        if let Self::Active(state) = self {
            state.record_with_detail(level, kind, label, byte_start, byte_end, detail);
        }
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub fn enter_construct(
        &mut self,
        level: TraceLevel,
        label: &str,
        byte_start: usize,
        byte_end: usize,
    ) {
        if let Self::Active(state) = self {
            state.enter_construct(level, label, byte_start, byte_end);
        }
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub fn exit_construct(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        if let Self::Active(state) = self {
            state.exit_construct(level, kind, label, byte_start, byte_end, detail);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn set_failure(&mut self, failure: TraceFailureSummary) {
        if let Self::Active(state) = self {
            state.failure = Some(failure);
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|report| report.phase != TracePhase::All))]
    pub fn finish(self) -> Option<TraceReport> {
        match self {
            Self::Disabled => None,
            Self::Active(state) => Some((*state).finish()),
        }
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct TraceRecorderState {
    options: TraceOptions,
    phase: TracePhase,
    events: Vec<TraceEvent>,
    depth: usize,
    trigger_depth: Option<usize>,
    truncated: bool,
    failure: Option<TraceFailureSummary>,
}

impl TraceRecorderState {
    #[requires(options.enabled)]
    #[requires(phase != TracePhase::All)]
    #[requires(options.limit > 0)]
    #[ensures(true)]
    fn new(options: TraceOptions, phase: TracePhase) -> Self {
        TraceRecorderState {
            options,
            phase,
            events: Vec::new(),
            depth: 0,
            trigger_depth: None,
            truncated: false,
            failure: None,
        }
    }

    #[requires(true)]
    #[ensures(!label.is_empty() && level <= self.options.level && self.filter_active_or_matches(label) -> ret)]
    fn should_record(&self, level: TraceLevel, label: &str) -> bool {
        !label.is_empty() && level <= self.options.level && self.filter_active_or_matches(label)
    }

    #[requires(!label.is_empty())]
    #[ensures(self.options.filter.is_none() -> ret)]
    fn filter_active_or_matches(&self, label: &str) -> bool {
        match &self.options.filter {
            None => true,
            Some(filter) => self.trigger_depth.is_some() || filter.name == label,
        }
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    fn record_with_detail(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        if !self.should_record(level, label) {
            return;
        }
        let depth = self.depth;
        let detail = detail();
        self.push_event(level, kind, label, byte_start..byte_end, depth, detail);
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    fn enter_construct(
        &mut self,
        level: TraceLevel,
        label: &str,
        byte_start: usize,
        byte_end: usize,
    ) {
        let starts_filter = self
            .options
            .filter
            .as_ref()
            .is_some_and(|filter| filter.name == label && self.trigger_depth.is_none());
        let should_record = !label.is_empty()
            && level <= self.options.level
            && (self.options.filter.is_none() || self.trigger_depth.is_some() || starts_filter);
        if !should_record {
            return;
        }
        if starts_filter {
            self.trigger_depth = Some(self.depth);
        }
        self.push_event(
            level,
            TraceEventKind::ConstructEnter,
            label,
            byte_start..byte_end,
            self.depth,
            None,
        );
        self.depth += 1;
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    fn exit_construct(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        if !self.should_record(level, label) {
            return;
        }
        self.depth = self.depth.saturating_sub(1);
        let depth = self.depth;
        let detail = detail();
        self.push_event(level, kind, label, byte_start..byte_end, depth, detail);
        if self.trigger_depth == Some(depth)
            && self
                .options
                .filter
                .as_ref()
                .is_some_and(|filter| filter.name == label)
        {
            self.trigger_depth = None;
        }
    }

    #[requires(!label.is_empty())]
    #[requires(span.start <= span.end)]
    #[ensures(true)]
    fn push_event(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        span: Range<usize>,
        depth: usize,
        detail: Option<String>,
    ) {
        if self.events.len() >= self.options.limit {
            self.truncated = true;
            return;
        }
        self.events.push(new!(TraceEvent {
            phase: self.phase,
            level,
            depth,
            kind,
            label: label.to_owned(),
            byte_start: span.start,
            byte_end: span.end,
            detail,
        }));
    }

    #[requires(true)]
    #[ensures(ret.phase == self.phase)]
    fn finish(self) -> TraceReport {
        new!(TraceReport {
            phase: self.phase,
            events: self.events,
            truncated: self.truncated,
            failure: self.failure,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticDetailMode {
    Summary,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticNoteMode {
    Always,
    Summary,
    Detailed,
}

impl DiagnosticNoteMode {
    #[requires(true)]
    #[ensures(matches!(self, Self::Always) -> ret)]
    pub fn visible_in(self, detail: DiagnosticDetailMode) -> bool {
        matches!(
            (self, detail),
            (Self::Always, _)
                | (Self::Summary, DiagnosticDetailMode::Summary)
                | (Self::Detailed, DiagnosticDetailMode::Detailed)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticTextRole {
    Construct,
    SpecificWord,
    Selmaho,
    WordCategory,
    Keyword,
    Punctuation,
    Plain,
}

#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticTextSegment {
    pub role: DiagnosticTextRole,
    pub text: String,
}

impl DiagnosticTextSegment {
    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn new(role: DiagnosticTextRole, text: String) -> Self {
        new!(DiagnosticTextSegment { role, text })
    }
}

#[invariant(!segments.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticStyledNote {
    pub mode: DiagnosticNoteMode,
    pub segments: Vec<DiagnosticTextSegment>,
}

impl DiagnosticStyledNote {
    #[requires(!segments.is_empty())]
    #[ensures(true)]
    pub fn new(mode: DiagnosticNoteMode, segments: Vec<DiagnosticTextSegment>) -> Self {
        new!(DiagnosticStyledNote { mode, segments })
    }
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    pub span: SourceSpan,
    pub message: String,
    pub primary: bool,
}

impl DiagnosticLabel {
    #[requires(!message.is_empty())]
    #[ensures(true)]
    pub fn new(span: SourceSpan, message: String, primary: bool) -> Self {
        new!(DiagnosticLabel {
            span,
            message,
            primary,
        })
    }
}

#[invariant(!code.is_empty())]
#[invariant(!message.is_empty())]
#[invariant(!labels.is_empty())]
#[invariant(labels.iter().any(|label| label.primary))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub phase: DiagnosticPhase,
    pub code: String,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    #[serde(default)]
    pub styled_notes: Vec<DiagnosticStyledNote>,
    pub word_index: Option<usize>,
}

impl Diagnostic {
    #[requires(!code.is_empty())]
    #[requires(!message.is_empty())]
    #[requires(!labels.is_empty())]
    #[requires(labels.iter().any(|label| label.primary))]
    #[ensures(true)]
    pub fn new(
        severity: DiagnosticSeverity,
        phase: DiagnosticPhase,
        code: String,
        message: String,
        labels: Vec<DiagnosticLabel>,
        notes: Vec<String>,
        word_index: Option<usize>,
    ) -> Self {
        new!(Diagnostic {
            severity,
            phase,
            code,
            message,
            labels,
            notes,
            styled_notes: Vec::new(),
            word_index,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_styled_notes(self, styled_notes: Vec<DiagnosticStyledNote>) -> Self {
        self.with_data(data! { styled_notes: styled_notes })
    }

    #[requires(true)]
    #[ensures(ret.primary)]
    pub fn primary_label(&self) -> &DiagnosticLabel {
        self.labels
            .iter()
            .find(|label| label.primary)
            .expect("diagnostic invariant guarantees a primary label")
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::CharOffsetOutOfBounds => true)]
#[invariant(::ByteOffsetOutOfBounds => true)]
#[invariant(::ByteOffsetNotCharBoundary => true)]
#[invariant(::SourceLocation(..) => true)]
pub enum DiagnosticSpanError {
    #[error("character offset {offset} exceeds source character length {source_len}")]
    CharOffsetOutOfBounds { offset: usize, source_len: usize },
    #[error("byte offset {offset} exceeds source byte length {source_len}")]
    ByteOffsetOutOfBounds { offset: usize, source_len: usize },
    #[error("byte offset {offset} is not a UTF-8 character boundary")]
    ByteOffsetNotCharBoundary { offset: usize },
    #[error("invalid source span: {0}")]
    SourceLocation(#[from] SourceLocationError),
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|span| span.char_start == char_start) || ret.is_err())]
pub fn source_span_from_char_offsets(
    source_id: Option<SourceId>,
    source: &str,
    char_start: usize,
    char_end: usize,
) -> Result<SourceSpan, DiagnosticSpanError> {
    let source_len = source.chars().count();
    if char_start > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_start,
            source_len,
        });
    }
    if char_end > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_end,
            source_len,
        });
    }
    let byte_start = byte_offset_for_char_offset(source, char_start)?;
    let byte_end = byte_offset_for_char_offset(source, char_end)?;
    SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end)
        .map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|span| span.byte_start == byte_start) || ret.is_err())]
pub fn source_span_from_byte_offsets(
    source_id: Option<SourceId>,
    source: &str,
    byte_start: usize,
    byte_end: usize,
) -> Result<SourceSpan, DiagnosticSpanError> {
    validate_byte_offset(source, byte_start)?;
    validate_byte_offset(source, byte_end)?;
    let char_start = char_offset_for_byte_offset(source, byte_start)?;
    let char_end = char_offset_for_byte_offset(source, byte_end)?;
    SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end)
        .map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(char_offset <= source.chars().count())]
#[ensures(ret.as_ref().is_ok_and(|offset| *offset <= source.len()) || ret.is_err())]
pub fn byte_offset_for_char_offset(
    source: &str,
    char_offset: usize,
) -> Result<usize, DiagnosticSpanError> {
    let source_len = source.chars().count();
    if char_offset > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_offset,
            source_len,
        });
    }
    Ok(source
        .char_indices()
        .map(|(index, _)| index)
        .nth(char_offset)
        .unwrap_or(source.len()))
}

#[requires(byte_offset <= source.len())]
#[requires(source.is_char_boundary(byte_offset))]
#[ensures(ret.as_ref().is_ok_and(|offset| *offset <= source.chars().count()) || ret.is_err())]
pub fn char_offset_for_byte_offset(
    source: &str,
    byte_offset: usize,
) -> Result<usize, DiagnosticSpanError> {
    validate_byte_offset(source, byte_offset)?;
    Ok(source[..byte_offset].chars().count())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|line_column| line_column.line > 0 && line_column.column > 0) || ret.is_err())]
pub fn line_column_for_byte_offset(
    source: &str,
    byte_offset: usize,
) -> Result<LineColumn, DiagnosticSpanError> {
    validate_byte_offset(source, byte_offset)?;
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in source[..byte_offset].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    LineColumn::new(line, column).map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| source.contains(text) || text.is_empty()) || ret.is_err())]
pub fn source_text_for_span(
    source: &str,
    span: &SourceSpan,
) -> Result<String, DiagnosticSpanError> {
    validate_byte_offset(source, span.byte_start)?;
    validate_byte_offset(source, span.byte_end)?;
    Ok(source[span.byte_start..span.byte_end].to_owned())
}

#[requires(true)]
#[ensures(ret.is_ok() -> offset <= source.len())]
fn validate_byte_offset(source: &str, offset: usize) -> Result<(), DiagnosticSpanError> {
    if offset > source.len() {
        return Err(DiagnosticSpanError::ByteOffsetOutOfBounds {
            offset,
            source_len: source.len(),
        });
    }
    if !source.is_char_boundary(offset) {
        return Err(DiagnosticSpanError::ByteOffsetNotCharBoundary { offset });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;
    use std::cell::Cell;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn converts_non_ascii_character_offsets_to_byte_spans() {
        let source = "coi gleki ĭa";
        let span = source_span_from_char_offsets(None, source, 10, 12).expect("span");
        assert_eq!(span.byte_start, 10);
        assert_eq!(span.byte_end, 13);
        assert_eq!(
            source_text_for_span(source, &span).expect("source text"),
            "ĭa"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_byte_offsets_inside_a_codepoint() {
        let error = source_span_from_byte_offsets(None, "ĭa", 1, 2).expect_err("bad boundary");
        assert!(matches!(
            error,
            DiagnosticSpanError::ByteOffsetNotCharBoundary { offset: 1 }
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn disabled_trace_recorder_does_not_evaluate_detail() {
        let detail_called = Cell::new(false);
        let mut recorder = TraceRecorder::new(TraceOptions::disabled(), TracePhase::Syntax);
        recorder.record_with_detail(
            TraceLevel::Top,
            TraceEventKind::ConstructEnter,
            "text",
            0,
            0,
            || {
                detail_called.set(true);
                Some("expensive detail".to_owned())
            },
        );

        assert!(!detail_called.get());
        assert!(recorder.finish().is_none());
    }
}
