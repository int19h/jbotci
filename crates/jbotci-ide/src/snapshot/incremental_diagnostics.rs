use std::{
    cell::RefCell,
    cmp::Ordering,
    sync::Arc,
    time::{Duration, Instant},
};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticPhase, DiagnosticSeverity};
use jbotci_morphology::{WordLike, map_word_like_spans};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    ParseOptions, SyntaxRecoveryItem, SyntaxRecoveryParse, SyntaxRecoveryParseData,
    SyntaxTextUnitGranularity, Token, generated_model, partition_syntax_text_units,
    syntax_text_structure, syntax_tokens_with_options,
};
use jbotci_tree::TreeVisitor;
use jbotci_web_core::{
    GentufaMorphologyAnalysis, GentufaWebOptions, analyze_gentufa_morphology_source,
    analyze_gentufa_syntax_diagnostics_for_words, complete_gentufa_source_analysis,
};

use super::{DocumentSnapshot, ResolvedDiagnostic};
use crate::{LineIndex, PositionEncoding};

/// Why an edit did or did not qualify for diagnostics-only paragraph reparsing.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalDiagnosticGate {
    Passed,
    NoConfirmedParagraph,
    FlankMismatch,
    BoundaryStructureChanged,
    CrossParagraphDiagnostic,
}

/// Wall-clock phase measurements for one prepared document generation.
#[invariant(*morphology <= *total)]
#[invariant(provisional.is_none_or(|duration| duration <= *total))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncrementalAnalysisTimings {
    pub morphology: Duration,
    pub provisional: Option<Duration>,
    pub total: Duration,
}

/// Immutable diagnostics-only view of one document version.
#[invariant(text.len() == line_index.byte_len())]
#[expensive_invariant(text.as_ref() == line_index.text())]
#[derive(Debug, Clone)]
pub struct DiagnosticSnapshot {
    pub text: Arc<str>,
    pub version: i64,
    pub line_index: LineIndex,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSnapshot {
    #[requires(true)]
    #[ensures(ret.version == snapshot.version)]
    #[ensures(ret.diagnostics == snapshot.diagnostics)]
    pub fn from_confirmed(snapshot: &DocumentSnapshot) -> Self {
        new!(DiagnosticSnapshot {
            text: Arc::clone(&snapshot.text),
            version: snapshot.version,
            line_index: snapshot.line_index.clone(),
            diagnostics: snapshot.diagnostics.clone(),
        })
    }

    #[requires(true)]
    #[ensures(ret.len() == self.diagnostics.len())]
    pub fn diagnostics(
        &self,
        encoding: PositionEncoding,
    ) -> impl ExactSizeIterator<Item = ResolvedDiagnostic<'_>> + '_ {
        self.diagnostics.iter().map(move |diagnostic| {
            new!(ResolvedDiagnostic {
                diagnostic,
                line_index: &self.line_index,
                encoding,
            })
        })
    }
}

/// Morphology plus an optional diagnostics-only optimistic result for one edit.
#[invariant(provisional.is_some() == (*gate == IncrementalDiagnosticGate::Passed))]
#[derive(Debug)]
pub struct PreparedDocumentAnalysis {
    text: Arc<str>,
    version: i64,
    morphology: GentufaMorphologyAnalysis,
    provisional: Option<Arc<DiagnosticSnapshot>>,
    gate: IncrementalDiagnosticGate,
    timings: IncrementalAnalysisTimings,
}

impl PreparedDocumentAnalysis {
    /// Run whole-document morphology and attempt a proven paragraph-local diagnostic update.
    #[requires(confirmed.is_none_or(|snapshot| snapshot.version < version))]
    #[ensures(ret.version == version)]
    pub fn prepare(confirmed: Option<&DocumentSnapshot>, text: String, version: i64) -> Self {
        let started = Instant::now();
        let text: Arc<str> = Arc::from(text);
        let options = GentufaWebOptions::default();
        let morphology_started = Instant::now();
        let morphology = analyze_gentufa_morphology_source(&text, &options)
            .expect("the built-in default dialect must always compile");
        let morphology_duration = morphology_started.elapsed();
        let (gate, provisional) = confirmed.map_or(
            (IncrementalDiagnosticGate::NoConfirmedParagraph, None),
            |confirmed| provisional_diagnostics(confirmed, Arc::clone(&text), version, &morphology),
        );
        let total = started.elapsed();
        let provisional_duration = provisional.as_ref().map(|_| total);
        let timings = new!(IncrementalAnalysisTimings {
            morphology: morphology_duration,
            provisional: provisional_duration,
            total,
        });
        new!(PreparedDocumentAnalysis {
            text,
            version,
            morphology,
            provisional,
            gate,
            timings,
        })
    }

    #[requires(true)]
    #[ensures(ret.is_some() == (self.gate == IncrementalDiagnosticGate::Passed))]
    pub fn provisional(&self) -> Option<Arc<DiagnosticSnapshot>> {
        self.provisional.as_ref().map(Arc::clone)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn gate(&self) -> IncrementalDiagnosticGate {
        self.gate
    }

    #[requires(true)]
    #[ensures(ret == self.timings)]
    pub fn timings(&self) -> IncrementalAnalysisTimings {
        self.timings
    }

    /// Complete the authoritative full syntax parse, consuming the morphology result.
    #[requires(true)]
    #[ensures(true)]
    pub fn confirm(self) -> DocumentSnapshot {
        let data!(PreparedDocumentAnalysis {
            text,
            version,
            morphology,
            ..
        }) = self.into_data();
        let options = GentufaWebOptions::default();
        let analysis = complete_gentufa_source_analysis(&text, &options, morphology);
        DocumentSnapshot::from_analysis(text, version, analysis)
    }
}

#[invariant(old_byte_start <= old_byte_end)]
#[invariant(new_byte_start <= new_byte_end)]
#[invariant(old_char_start <= old_char_end)]
#[invariant(new_char_start <= new_char_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocumentEdit {
    old_byte_start: usize,
    old_byte_end: usize,
    new_byte_start: usize,
    new_byte_end: usize,
    old_char_start: usize,
    old_char_end: usize,
    new_char_start: usize,
    new_char_end: usize,
}

#[invariant(token_start < token_end)]
#[invariant(byte_start < byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConfirmedParagraph {
    token_start: usize,
    token_end: usize,
    byte_start: usize,
    byte_end: usize,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticParagraphSide {
    Before,
    Inside,
    After,
    CrossesBoundary,
}

#[invariant(true)]
struct ValidTokenCollector {
    tokens: Vec<Token>,
}

impl<'tree> TreeVisitor<'tree> for ValidTokenCollector {
    type Node = generated_model::NodeRef<'tree>;
    type Atom = generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.tokens.len() == old(self.tokens.len()) + 1)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::AtomRef::Token(token) = atom;
        self.tokens.push(token.clone());
    }
}

#[invariant(true)]
struct RecoveredTokenCollector {
    tokens: Vec<Token>,
}

impl<'tree> TreeVisitor<'tree> for RecoveredTokenCollector {
    type Node = generated_model::recovered::NodeRef<'tree>;
    type Atom = generated_model::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.tokens.len() == old(self.tokens.len()) + 1)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::recovered::AtomRef::Token(token) = atom;
        self.tokens.push(token.clone());
    }
}

#[invariant(runs.borrow().iter().all(|run| !run.is_empty()))]
struct SkippedTokenCollector<'tree> {
    runs: RefCell<Vec<&'tree [Token]>>,
}

impl<'tree> generated_model::recovered::TreeWalker<'tree> for SkippedTokenCollector<'tree> {
    #[requires(true)]
    #[ensures(true)]
    fn walk_recovered_error(&mut self, item: &'tree SyntaxRecoveryItem) {
        if let Some(tokens) = item.skipped_tokens() {
            self.runs.borrow_mut().push(tokens);
        }
    }
}

#[requires(confirmed.version < version)]
#[ensures(ret.1.is_some() == (ret.0 == IncrementalDiagnosticGate::Passed))]
fn provisional_diagnostics(
    confirmed: &DocumentSnapshot,
    text: Arc<str>,
    version: i64,
    morphology: &GentufaMorphologyAnalysis,
) -> (IncrementalDiagnosticGate, Option<Arc<DiagnosticSnapshot>>) {
    let edit = document_edit(&confirmed.text, &text);
    let old_tokens = confirmed_tree_tokens(&confirmed.parse);
    let Some(paragraph) = confirmed_paragraph_covering_edit(&old_tokens, edit) else {
        return (IncrementalDiagnosticGate::NoConfirmedParagraph, None);
    };
    let Some(old_word_range) = word_range_for_bytes(
        &confirmed.words.words,
        paragraph.byte_start,
        paragraph.byte_end,
    ) else {
        return (IncrementalDiagnosticGate::NoConfirmedParagraph, None);
    };
    let Some(new_word_range) = exact_flank_gate(
        &confirmed.words.words,
        &morphology.morphology().words,
        old_word_range.clone(),
        edit,
    ) else {
        return (IncrementalDiagnosticGate::FlankMismatch, None);
    };
    let old_paragraph_tokens = &old_tokens[paragraph.token_start..paragraph.token_end];
    let new_paragraph_tokens = syntax_tokens_with_options(
        &morphology.morphology().words[new_word_range.clone()],
        &ParseOptions::default(),
    );
    if syntax_text_structure(old_paragraph_tokens) != syntax_text_structure(&new_paragraph_tokens) {
        return (IncrementalDiagnosticGate::BoundaryStructureChanged, None);
    }

    let word_delta = signed_delta(new_word_range.len(), old_word_range.len());
    let Some(word_delta) = word_delta else {
        return (IncrementalDiagnosticGate::FlankMismatch, None);
    };
    let mut retained_syntax = Vec::new();
    for diagnostic in confirmed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.phase == DiagnosticPhase::Syntax)
    {
        let side = diagnostic_side(diagnostic, paragraph.byte_start, paragraph.byte_end);
        // Multi-error recovery is a document-wide directive chain. A local
        // parse starts with an empty chain, so it cannot soundly reproduce a
        // paragraph reached through an earlier syntax error even when the
        // recovered tree exposes a precise paragraph boundary.
        if side == DiagnosticParagraphSide::Before
            && diagnostic.severity == DiagnosticSeverity::Error
        {
            return (IncrementalDiagnosticGate::CrossParagraphDiagnostic, None);
        }
        if side == DiagnosticParagraphSide::Inside {
            continue;
        }
        if side == DiagnosticParagraphSide::CrossesBoundary {
            return (IncrementalDiagnosticGate::CrossParagraphDiagnostic, None);
        }
        let word_index_delta = (side == DiagnosticParagraphSide::After)
            .then_some(word_delta)
            .unwrap_or(0);
        let Some(shifted) = shift_diagnostic(diagnostic.clone(), edit, word_index_delta) else {
            return (IncrementalDiagnosticGate::CrossParagraphDiagnostic, None);
        };
        retained_syntax.push(shifted);
    }

    let local_syntax = if morphology.morphology().errors.is_empty() {
        analyze_gentufa_syntax_diagnostics_for_words(
            &text,
            &GentufaWebOptions::default(),
            morphology,
            &morphology.morphology().words[new_word_range.clone()],
        )
    } else {
        Vec::new()
    };
    let mut global_local_syntax = Vec::with_capacity(local_syntax.len());
    for diagnostic in local_syntax {
        let Some(word_index) = diagnostic.word_index else {
            global_local_syntax.push(diagnostic);
            continue;
        };
        let Some(global_word_index) = word_index.checked_add(new_word_range.start) else {
            return (IncrementalDiagnosticGate::FlankMismatch, None);
        };
        global_local_syntax.push(diagnostic.with_data(data! {
            word_index: Some(global_word_index),
        }));
    }

    let diagnostics = if morphology.morphology().errors.is_empty() {
        splice_diagnostics(
            retained_syntax,
            global_local_syntax,
            morphology.diagnostics().to_vec(),
        )
    } else {
        morphology.diagnostics().to_vec()
    };
    let line_index = LineIndex::new(Arc::clone(&text));
    let provisional = Arc::new(new!(DiagnosticSnapshot {
        text,
        version,
        line_index,
        diagnostics,
    }));
    (IncrementalDiagnosticGate::Passed, Some(provisional))
}

#[requires(true)]
#[ensures(true)]
fn document_edit(old: &str, new: &str) -> DocumentEdit {
    let prefix_chars = old
        .chars()
        .zip(new.chars())
        .take_while(|(left, right)| left == right)
        .count();
    let old_char_len = old.chars().count();
    let new_char_len = new.chars().count();
    let suffix_limit = old_char_len
        .saturating_sub(prefix_chars)
        .min(new_char_len.saturating_sub(prefix_chars));
    let suffix_chars = old
        .chars()
        .rev()
        .zip(new.chars().rev())
        .take(suffix_limit)
        .take_while(|(left, right)| left == right)
        .count();
    let old_char_end = old_char_len - suffix_chars;
    let new_char_end = new_char_len - suffix_chars;
    new!(DocumentEdit {
        old_byte_start: byte_offset_for_char(old, prefix_chars),
        old_byte_end: byte_offset_for_char(old, old_char_end),
        new_byte_start: byte_offset_for_char(new, prefix_chars),
        new_byte_end: byte_offset_for_char(new, new_char_end),
        old_char_start: prefix_chars,
        old_char_end,
        new_char_start: prefix_chars,
        new_char_end,
    })
}

#[requires(char_offset <= text.chars().count())]
#[ensures(ret <= text.len() && text.is_char_boundary(ret))]
fn byte_offset_for_char(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map_or(text.len(), |(byte, _)| byte)
}

#[requires(true)]
#[ensures(ret.windows(2).all(|tokens| token_byte_start(&tokens[0]) <= token_byte_start(&tokens[1])))]
fn confirmed_tree_tokens(parse: &SyntaxRecoveryParse) -> Vec<Token> {
    let mut tokens = match parse.as_data() {
        SyntaxRecoveryParseData::Valid { parse } => {
            let mut collector = ValidTokenCollector { tokens: Vec::new() };
            generated_model::TreeNode::visit_in_order(&parse.parse_tree, &mut collector);
            collector.tokens
        }
        SyntaxRecoveryParseData::Recovered { parse } => {
            let mut collector = RecoveredTokenCollector { tokens: Vec::new() };
            generated_model::recovered::TreeNode::visit_in_order(&parse.parse_tree, &mut collector);
            let mut skipped = new!(SkippedTokenCollector {
                runs: RefCell::new(Vec::new()),
            });
            generated_model::recovered::TreeWalkable::walk_with(
                parse.parse_tree.as_ref(),
                &mut skipped,
            );
            let data!(SkippedTokenCollector { runs }) = skipped.into_data();
            let mut tokens = collector.tokens;
            tokens.extend(runs.into_inner().into_iter().flatten().cloned());
            tokens
        }
    };
    tokens.sort_by_key(token_byte_start);
    tokens
}

#[requires(true)]
#[ensures(true)]
fn token_byte_start(token: &Token) -> usize {
    token
        .source_spans()
        .first()
        .map_or(usize::MAX, |span| span.byte_start)
}

#[requires(true)]
#[ensures(true)]
fn confirmed_paragraph_covering_edit(
    tokens: &[Token],
    edit: DocumentEdit,
) -> Option<ConfirmedParagraph> {
    let mut matches = partition_syntax_text_units(tokens, SyntaxTextUnitGranularity::Paragraph)
        .into_iter()
        .filter_map(|unit| {
            let first = tokens
                .get(unit.token_start)?
                .source_spans()
                .first()?
                .byte_start;
            let last = tokens
                .get(unit.token_end - 1)?
                .source_spans()
                .last()?
                .byte_end;
            let covers = if edit.old_byte_start == edit.old_byte_end {
                first <= edit.old_byte_start && edit.old_byte_start <= last
            } else {
                first <= edit.old_byte_start && edit.old_byte_end <= last
            };
            covers.then(|| {
                new!(ConfirmedParagraph {
                    token_start: unit.token_start,
                    token_end: unit.token_end,
                    byte_start: first,
                    byte_end: last,
                })
            })
        });
    let paragraph = matches.next()?;
    matches.next().is_none().then_some(paragraph)
}

#[requires(byte_start < byte_end)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= words.len()))]
fn word_range_for_bytes(
    words: &[WordLike],
    byte_start: usize,
    byte_end: usize,
) -> Option<std::ops::Range<usize>> {
    let start = words.partition_point(|word| {
        word.byte_range()
            .is_none_or(|range| range.end <= byte_start)
    });
    let end = words.partition_point(|word| {
        word.byte_range()
            .is_some_and(|range| range.start < byte_end)
    });
    (start < end).then_some(start..end)
}

#[requires(old_range.end <= old_words.len())]
#[ensures(ret.as_ref().is_none_or(|range| range.end <= new_words.len()))]
fn exact_flank_gate(
    old_words: &[WordLike],
    new_words: &[WordLike],
    old_range: std::ops::Range<usize>,
    edit: DocumentEdit,
) -> Option<std::ops::Range<usize>> {
    let suffix_len = old_words.len() - old_range.end;
    if new_words.len() < old_range.start + suffix_len
        || old_words[..old_range.start] != new_words[..old_range.start]
    {
        return None;
    }
    let byte_delta = signed_delta(
        edit.new_byte_end - edit.new_byte_start,
        edit.old_byte_end - edit.old_byte_start,
    )?;
    let char_delta = signed_delta(
        edit.new_char_end - edit.new_char_start,
        edit.old_char_end - edit.old_char_start,
    )?;
    let new_suffix_start = new_words.len() - suffix_len;
    let shifted_suffix = old_words[old_range.end..]
        .iter()
        .cloned()
        .map(|word| map_word_like_spans(word, &|span| shift_span(span, byte_delta, char_delta)))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if shifted_suffix != new_words[new_suffix_start..] {
        return None;
    }
    Some(old_range.start..new_suffix_start)
}

#[requires(true)]
#[ensures(true)]
fn signed_delta(new: usize, old: usize) -> Option<isize> {
    if new >= old {
        isize::try_from(new - old).ok()
    } else {
        isize::try_from(old - new).ok()?.checked_neg()
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|shifted| shifted.byte_len() == span.byte_len() && shifted.char_len() == span.char_len()) || ret.is_err())]
fn shift_span(
    span: SourceSpan,
    byte_delta: isize,
    char_delta: isize,
) -> Result<SourceSpan, String> {
    SourceSpan::new(
        span.source_id.clone(),
        span.byte_start
            .checked_add_signed(byte_delta)
            .ok_or_else(|| "shifted byte start is out of range".to_owned())?,
        span.byte_end
            .checked_add_signed(byte_delta)
            .ok_or_else(|| "shifted byte end is out of range".to_owned())?,
        span.char_start
            .checked_add_signed(char_delta)
            .ok_or_else(|| "shifted character start is out of range".to_owned())?,
        span.char_end
            .checked_add_signed(char_delta)
            .ok_or_else(|| "shifted character end is out of range".to_owned())?,
    )
    .map_err(|error| error.to_string())
}

#[requires(byte_start < byte_end)]
#[ensures(true)]
fn diagnostic_side(
    diagnostic: &Diagnostic,
    byte_start: usize,
    byte_end: usize,
) -> DiagnosticParagraphSide {
    if diagnostic
        .labels
        .iter()
        .all(|label| byte_start <= label.span.byte_start && label.span.byte_end <= byte_end)
    {
        DiagnosticParagraphSide::Inside
    } else if diagnostic
        .labels
        .iter()
        .all(|label| label.span.byte_end <= byte_start)
    {
        DiagnosticParagraphSide::Before
    } else if diagnostic
        .labels
        .iter()
        .all(|label| byte_end <= label.span.byte_start)
    {
        DiagnosticParagraphSide::After
    } else {
        DiagnosticParagraphSide::CrossesBoundary
    }
}

#[requires(true)]
#[ensures(true)]
fn shift_diagnostic(
    diagnostic: Diagnostic,
    edit: DocumentEdit,
    word_index_delta: isize,
) -> Option<Diagnostic> {
    let byte_delta = signed_delta(
        edit.new_byte_end - edit.new_byte_start,
        edit.old_byte_end - edit.old_byte_start,
    )?;
    let char_delta = signed_delta(
        edit.new_char_end - edit.new_char_start,
        edit.old_char_end - edit.old_char_start,
    )?;
    let labels = diagnostic
        .labels
        .iter()
        .cloned()
        .map(|label| shift_label(label, edit, byte_delta, char_delta))
        .collect::<Option<Vec<_>>>()?;
    let word_index = match diagnostic.word_index {
        Some(index) => Some(index.checked_add_signed(word_index_delta)?),
        None => None,
    };
    Some(diagnostic.with_data(data! {
        labels: labels,
        word_index: word_index,
    }))
}

#[requires(true)]
#[ensures(true)]
fn shift_label(
    label: DiagnosticLabel,
    edit: DocumentEdit,
    byte_delta: isize,
    char_delta: isize,
) -> Option<DiagnosticLabel> {
    if label.span.byte_end <= edit.old_byte_start {
        return Some(label);
    }
    if edit.old_byte_end <= label.span.byte_start {
        let span = shift_span(label.span.clone(), byte_delta, char_delta).ok()?;
        return Some(label.with_data(data! { span: span }));
    }
    None
}

#[requires(morphology.iter().all(|diagnostic| diagnostic.phase == DiagnosticPhase::Morphology))]
#[ensures(ret.len() == old(retained.len()) + old(local.len()) + old(morphology.len()))]
fn splice_diagnostics(
    retained: Vec<Diagnostic>,
    local: Vec<Diagnostic>,
    morphology: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let (mut syntax_errors, mut syntax_warnings): (Vec<_>, Vec<_>) = retained
        .into_iter()
        .chain(local)
        .partition(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    syntax_errors.sort_by(diagnostic_source_order);
    syntax_warnings.sort_by(diagnostic_source_order);
    syntax_errors.extend(morphology);
    syntax_errors.extend(syntax_warnings);
    syntax_errors
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_source_order(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    let left = left.primary_label();
    let right = right.primary_label();
    (left.span.byte_start, left.span.byte_end, &left.message).cmp(&(
        right.span.byte_start,
        right.span.byte_end,
        &right.message,
    ))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    const DOCUMENT_SCALE_RECOVERED_SOURCE: &str = concat!(
        "ni'o\n",
        ".i mi cusku lu do cusku lu mi klama li'u li'u\n",
        "ni'o\n",
        ".i do ku viska le mlatu\n",
        "ni'o\n",
        ".i mi tavla lu do klama li'u\n",
    );

    #[requires(old_source.matches(old_fragment).count() == 1)]
    #[requires(!new_fragment.is_empty())]
    #[ensures(ret.gate() == IncrementalDiagnosticGate::Passed -> ret.provisional().is_some())]
    fn prepare_replacement(
        old_source: &str,
        old_fragment: &str,
        new_fragment: &str,
    ) -> PreparedDocumentAnalysis {
        let confirmed = DocumentSnapshot::new(old_source.to_owned(), 1);
        let new_source = old_source.replacen(old_fragment, new_fragment, 1);
        PreparedDocumentAnalysis::prepare(Some(&confirmed), new_source, 2)
    }

    #[requires(prepared.gate() == IncrementalDiagnosticGate::Passed)]
    #[ensures(true)]
    fn assert_provisional_matches_confirmation(prepared: PreparedDocumentAnalysis) {
        let provisional = prepared
            .provisional()
            .expect("the precondition requires a provisional diagnostic layer");
        let confirmed = prepared.confirm();
        assert_eq!(provisional.diagnostics, confirmed.diagnostics);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordinary_paragraph_edit_passes_and_matches_confirmation() {
        let prepared = prepare_replacement(
            "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do",
            "do cadzu",
            "do cadzu le zarci",
        );
        assert_eq!(prepared.gate(), IncrementalDiagnosticGate::Passed);
        assert_provisional_matches_confirmation(prepared);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn assumption_breaking_edits_fail_the_gate() {
        let cases = [
            (
                "typing-niho",
                "mi klama i do cadzu",
                "do cadzu",
                "ni'o do cadzu",
                IncrementalDiagnosticGate::BoundaryStructureChanged,
            ),
            (
                "deleting-lihu",
                "mi cusku lu do cadzu li'u i mi klama",
                " li'u",
                " ",
                IncrementalDiagnosticGate::BoundaryStructureChanged,
            ),
            (
                "prefixing-zo",
                "mi klama i do cadzu",
                " i ",
                " zo i ",
                IncrementalDiagnosticGate::BoundaryStructureChanged,
            ),
            (
                "su-erasure",
                "mi klama ni'o do cadzu ni'o mi tavla",
                " ni'o do",
                " su ni'o do",
                IncrementalDiagnosticGate::FlankMismatch,
            ),
        ];
        for (name, old_source, old_fragment, new_fragment, expected) in cases {
            let prepared = prepare_replacement(old_source, old_fragment, new_fragment);
            if name == "su-erasure" {
                assert_ne!(prepared.gate(), IncrementalDiagnosticGate::Passed, "{name}");
            } else {
                assert_eq!(prepared.gate(), expected, "{name}");
            }
            assert!(prepared.provisional().is_none(), "{name}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_single_skipped_run_still_finds_paragraph_anchor() {
        let prepared = prepare_replacement(
            DOCUMENT_SCALE_RECOVERED_SOURCE,
            "do ku viska le mlatu",
            "do ku viska le mlatu ui",
        );
        assert_eq!(
            prepared.gate(),
            IncrementalDiagnosticGate::CrossParagraphDiagnostic
        );
        assert!(prepared.provisional().is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn retained_multibyte_diagnostic_spans_match_in_all_encodings() {
        let prepared = prepare_replacement(
            "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do",
            "do cadzu",
            "do cádzu le zarci",
        );
        assert_eq!(prepared.gate(), IncrementalDiagnosticGate::Passed);
        let provisional = prepared
            .provisional()
            .expect("ordinary paragraph edit has a provisional layer");
        let confirmed = prepared.confirm();
        for encoding in [
            PositionEncoding::Utf8,
            PositionEncoding::Utf16,
            PositionEncoding::Utf32,
        ] {
            let provisional_positions = provisional
                .diagnostics(encoding)
                .flat_map(|diagnostic| {
                    diagnostic
                        .labels()
                        .map(|label| label.positions)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let confirmed_positions = confirmed
                .diagnostics(encoding)
                .flat_map(|diagnostic| {
                    diagnostic
                        .labels()
                        .map(|label| label.positions)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            assert_eq!(provisional_positions, confirmed_positions, "{encoding:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fixture_sample_gate_passes_imply_confirmation_equivalence() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/zantufa/upstream-parity.json"
        ))
        .expect("checked-in fixture JSON");
        let mut passed = 0;
        for case in fixture["cases"]
            .as_array()
            .expect("fixture cases")
            .iter()
            .take(32)
        {
            let sample = case["source"].as_str().expect("fixture source");
            let old_source = format!("mi klama\nni'o\n{sample}\nni'o\nmi ku i do");
            let new_source = format!("mi klama\nni'o\n{sample} ui\nni'o\nmi ku i do");
            let confirmed = DocumentSnapshot::new(old_source, 1);
            let prepared = PreparedDocumentAnalysis::prepare(Some(&confirmed), new_source, 2);
            if prepared.gate() == IncrementalDiagnosticGate::Passed {
                passed += 1;
                assert_provisional_matches_confirmation(prepared);
            }
        }
        assert!(
            passed >= 5,
            "too few fixture-derived edits passed: {passed}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_provisional_timing(assert_literal_boundary: bool) {
        let old_source = format!(
            "{}do cadzu\nni'o\nmi ku i do",
            "mi klama\nni'o\n".repeat(20),
        );
        let confirmed = DocumentSnapshot::new(old_source.clone(), 1);
        let new_source = old_source.replacen("do cadzu", "do cadzu le zarci", 1);
        let started = Instant::now();
        let prepared = PreparedDocumentAnalysis::prepare(Some(&confirmed), new_source, 2);
        let elapsed = started.elapsed();
        assert_eq!(prepared.gate(), IncrementalDiagnosticGate::Passed);
        assert_provisional_matches_confirmation(prepared);
        if assert_literal_boundary {
            assert!(
                elapsed < Duration::from_millis(500),
                "provisional diagnostics took {elapsed:?}",
            );
        }
    }

    #[cfg(not(feature = "expensive_contracts"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn provisional_diagnostics_meet_local_latency_boundary() {
        assert_provisional_timing(std::env::var_os("CI").is_none());
    }

    #[cfg(feature = "expensive_contracts")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn provisional_diagnostics_timing_functional_twin() {
        assert_provisional_timing(false);
    }

    #[test]
    #[ignore = "requires an external reference document and explicit edit fragments"]
    #[requires(true)]
    #[ensures(true)]
    fn report_external_reference_document_timings() {
        let path = std::env::var("JBOTCI_INCREMENTAL_REFERENCE_DOCUMENT")
            .expect("set JBOTCI_INCREMENTAL_REFERENCE_DOCUMENT");
        let old_fragment = std::env::var("JBOTCI_INCREMENTAL_REFERENCE_OLD")
            .expect("set JBOTCI_INCREMENTAL_REFERENCE_OLD");
        let new_fragment = std::env::var("JBOTCI_INCREMENTAL_REFERENCE_NEW")
            .expect("set JBOTCI_INCREMENTAL_REFERENCE_NEW");
        let source = std::fs::read_to_string(path).expect("read external reference document");
        assert_eq!(
            source.matches(&old_fragment).count(),
            1,
            "the measured edit must identify exactly one source fragment",
        );

        let first_open_started = Instant::now();
        let confirmed = DocumentSnapshot::new(source.clone(), 1);
        let first_open = first_open_started.elapsed();
        let edited = source.replacen(&old_fragment, &new_fragment, 1);
        let prepared = PreparedDocumentAnalysis::prepare(Some(&confirmed), edited, 2);
        let timings = prepared.timings();
        assert_eq!(prepared.gate(), IncrementalDiagnosticGate::Passed);
        assert!(
            timings.total < Duration::from_millis(500),
            "reference provisional diagnostics took {:?}",
            timings.total,
        );
        let confirmation_started = Instant::now();
        assert_provisional_matches_confirmation(prepared);
        let confirmation = confirmation_started.elapsed();

        eprintln!(
            "reference-bytes={} morphology={:?} edit-to-provisional={:?} confirmation={:?} first-open={:?}",
            source.len(),
            timings.morphology,
            timings
                .provisional
                .expect("gate pass records provisional time"),
            confirmation,
            first_open,
        );
    }
}
