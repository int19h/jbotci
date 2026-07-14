#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    sync::Arc,
};

use jbotci_diagnostics::{
    TraceEventKind, TraceFailureSummary, TraceLevel, TracePhase, TraceRecorder, TraceReport,
    source_span_from_byte_offsets,
};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Selmaho, Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_tree::{RecoveryItemState, TreeVisitor};
use vec1::Vec1;

use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData};
use crate::{
    ExperimentalConstruct, ParseOptions, RecoveredSyntaxParse, RecoveredSyntaxParseAttempt,
    SyntaxError, SyntaxParse, SyntaxParseAttempt, SyntaxRecoveryParse, SyntaxRecoveryParseAttempt,
    SyntaxRecoveryParseData, SyntaxWarning, Token, WithIndicators, WithIndicatorsData,
    syntax_construct_is_descendant_of, syntax_immediate_child_under,
};

mod generated;
mod generated_runtime;
mod parse_error;
mod parser_core;
pub(crate) mod tokens;
use parse_error::{SyntaxFound, SyntaxFoundData, SyntaxParseCustomKind, SyntaxParseError};
use parser_core::{Boxed, Checkpoint, Cursor, Inspector, MappedInput, SimpleSpan, Spanned};

#[doc(hidden)]
pub mod generated_model {
    pub use super::generated::generated_model::*;
}

type Span = SimpleSpan;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens>;
type BoxedParser<'tokens, O> = Boxed<'tokens, O>;

// Candidate directives are already sorted by the spec's priority order. Keep
// the legacy bound and ordering intact for inputs that already recover. The
// fallback phase needs a wider search because only some owning-rule candidates
// encounter a natural stop before the declared failure position.
const LEGACY_RECOVERY_DIRECTIVE_TRIALS_PER_ERROR: usize = 8;
const MAX_NATURAL_STOP_DIRECTIVE_TRIALS_PER_ERROR: usize = 64;

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct ParserStateFinish {
    pub warnings: Vec<SyntaxWarning>,
    pub trace: Option<TraceReport>,
    pub unconsumed_recovery_directives: usize,
    pub recovery_directives: Vec<RecoveryDirective>,
    pub effective_fail_token_indices: Vec<usize>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParserCheckpoint {
    warning_count: usize,
    syntax_context_count: usize,
    syntax_rule_count: usize,
    recovery: Option<ParserRecoveryCheckpoint>,
    trace_save: bool,
}

#[invariant(active_recovery_directive.as_ref().is_none_or(|active| active.directive.fail_token_index <= active.directive.resume_token_index))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParserRecoveryCheckpoint {
    consumed_recovery_directives: usize,
    active_recovery_directive: Option<ActiveRecoveryDirective>,
}

#[invariant(!construct.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SyntaxContextFrame {
    construct: &'static str,
    byte_start: usize,
}

impl SyntaxContextFrame {
    #[requires(!construct.is_empty())]
    #[ensures(ret.construct == construct)]
    #[ensures(ret.byte_start == byte_start)]
    pub(super) fn new(construct: &'static str, byte_start: usize) -> Self {
        new!(SyntaxContextFrame {
            construct,
            byte_start,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(super) fn construct(&self) -> &'static str {
        self.construct
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn byte_start(&self) -> usize {
        self.byte_start
    }
}

#[invariant(!rule.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SyntaxRuleFrame {
    rule: &'static str,
    byte_start: usize,
}

impl SyntaxRuleFrame {
    #[requires(!rule.is_empty())]
    #[ensures(ret.rule == rule)]
    #[ensures(ret.byte_start == byte_start)]
    pub(super) fn new(rule: &'static str, byte_start: usize) -> Self {
        new!(SyntaxRuleFrame { rule, byte_start })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(super) fn rule(&self) -> &'static str {
        self.rule
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn byte_start(&self) -> usize {
        self.byte_start
    }
}

#[invariant(!rule.is_empty())]
#[invariant(fail_token_index <= resume_token_index)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecoveryDirective {
    rule: &'static str,
    instance_byte_start: usize,
    fail_token_index: usize,
    natural_stop_enabled: bool,
    resume_token_index: usize,
    resume_field: usize,
    error_index: usize,
    error: SyntaxError,
}

#[invariant(*effective_fail_token_index <= directive.fail_token_index)]
#[invariant(directive.fail_token_index < directive.resume_token_index || *effective_fail_token_index == directive.fail_token_index)]
#[invariant(*skipped_item_emitted -> directive.error_index < usize::MAX)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveRecoveryDirective {
    directive: RecoveryDirective,
    effective_fail_token_index: usize,
    skipped_item_emitted: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RecoveryFieldActionKind {
    Abandon,
    Resume,
}

#[invariant(item.as_ref().is_none_or(|item| item.recovery_error_index().is_some()))]
#[invariant(match kind {
    RecoveryFieldActionKind::Abandon => resume_token_index.is_none(),
    RecoveryFieldActionKind::Resume => resume_token_index.is_some_and(|index| index < usize::MAX),
})]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecoveryFieldAction {
    kind: RecoveryFieldActionKind,
    item: Option<SyntaxRecoveryItem>,
    resume_token_index: Option<usize>,
}

impl RecoveryFieldAction {
    #[requires(true)]
    #[ensures(ret.kind == RecoveryFieldActionKind::Abandon)]
    pub(super) fn abandon(item: Option<SyntaxRecoveryItem>) -> Self {
        new!(RecoveryFieldAction {
            kind: RecoveryFieldActionKind::Abandon,
            item,
            resume_token_index: None,
        })
    }

    #[requires(resume_token_index < usize::MAX)]
    #[ensures(ret.kind == RecoveryFieldActionKind::Resume)]
    pub(super) fn resume(item: Option<SyntaxRecoveryItem>, resume_token_index: usize) -> Self {
        new!(RecoveryFieldAction {
            kind: RecoveryFieldActionKind::Resume,
            item,
            resume_token_index: Some(resume_token_index),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn into_parts(
        self,
    ) -> (
        RecoveryFieldActionKind,
        Option<SyntaxRecoveryItem>,
        Option<usize>,
    ) {
        let data!(RecoveryFieldAction {
            kind,
            item,
            resume_token_index,
        }) = self.into_data();
        (kind, item, resume_token_index)
    }
}

impl RecoveryDirective {
    #[requires(!rule.is_empty())]
    #[requires(fail_token_index <= resume_token_index)]
    #[ensures(ret.rule == rule)]
    #[ensures(ret.fail_token_index == fail_token_index)]
    #[ensures(ret.resume_token_index == resume_token_index)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        rule: &'static str,
        instance_byte_start: usize,
        fail_token_index: usize,
        resume_token_index: usize,
        resume_field: usize,
        error_index: usize,
        error: SyntaxError,
    ) -> Self {
        new!(RecoveryDirective {
            rule,
            instance_byte_start,
            fail_token_index,
            natural_stop_enabled: false,
            resume_token_index,
            resume_field,
            error_index,
            error,
        })
    }

    #[requires(true)]
    #[ensures(ret.natural_stop_enabled)]
    fn with_natural_stop_enabled(self) -> Self {
        self.with_data(data! { natural_stop_enabled: true })
    }

    #[requires(true)]
    #[ensures(ret -> input_location <= self.fail_token_index)]
    #[ensures(ret -> (self.resume_token_index > input_location || (self.resume_token_index == self.fail_token_index && input_location == self.fail_token_index)))]
    fn can_fire_at(&self, input_location: usize) -> bool {
        input_location <= self.fail_token_index
            && (self.fail_token_index < self.resume_token_index
                || input_location == self.fail_token_index)
    }
}

#[derive(Clone)]
#[invariant(true)]
pub(super) struct SyntaxMemoValue {
    value: Arc<dyn Any>,
}

type StrictSyntaxMemoKey = (&'static str, usize);
type RecoverySyntaxMemoKey = (&'static str, usize, usize);

impl fmt::Debug for SyntaxMemoValue {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SyntaxMemoValue(..)")
    }
}

#[invariant(start_location <= end_location, "memo success must not rewind input")]
#[invariant(recovery_index <= consumed_recovery_directives)]
#[invariant(effective_fail_token_indices.len() == consumed_recovery_directives - recovery_index)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxMemoSuccess {
    start_location: usize,
    end_location: usize,
    recovery_index: usize,
    consumed_recovery_directives: usize,
    effective_fail_token_indices: Vec<usize>,
    value: SyntaxMemoValue,
    warnings: Arc<[SyntaxWarning]>,
}

#[derive(Debug, Clone, Default)]
#[invariant(true)]
pub(super) struct ParserState<'tokens> {
    anchor_byte_starts: Vec<Option<usize>>,
    syntax_location_byte_offsets: Vec<usize>,
    cmavo_cache: HashMap<(usize, usize), Option<Cmavo>>,
    syntax_memo: HashMap<StrictSyntaxMemoKey, SyntaxMemoSuccess>,
    syntax_recovery_memo: HashMap<RecoverySyntaxMemoKey, SyntaxMemoSuccess>,
    syntax_failure_memo: HashMap<StrictSyntaxMemoKey, SyntaxParseError<'tokens>>,
    // Failed recovered rules depend on the same directive-consumption index as
    // successful recovered rules; caching them prevents exponential retries.
    syntax_recovery_failure_memo: HashMap<RecoverySyntaxMemoKey, SyntaxParseError<'tokens>>,
    syntax_memo_in_progress: HashSet<StrictSyntaxMemoKey>,
    syntax_recovery_memo_in_progress: HashSet<RecoverySyntaxMemoKey>,
    diagnostic_candidates: Vec<SyntaxParseError<'tokens>>,
    warnings: Vec<SyntaxWarning>,
    trace: TraceRecorder,
    active_syntax_contexts: Vec<SyntaxContextFrame>,
    active_syntax_rules: Vec<SyntaxRuleFrame>,
    recovery_directives: Vec<RecoveryDirective>,
    consumed_recovery_directives: usize,
    // This is a consumption stack parallel to `recovery_directives`; each
    // entry records where its directive actually fired. Checkpoint rewind can
    // therefore restore recovery state by truncating instead of cloning and
    // rewriting every remaining directive.
    effective_fail_token_indices: Vec<usize>,
    active_recovery_directive: Option<ActiveRecoveryDirective>,
    recovery_tokens: Vec<Token>,
    recovery_source: Option<Arc<str>>,
    track_recovery_branches: bool,
    syntax_grammar_env: generated_runtime::SyntaxGrammarEnv,
    _tokens: PhantomData<&'tokens ()>,
}

#[invariant(
    self.syntax_location_byte_offsets.is_empty()
        || self.syntax_location_byte_offsets.len() == self.anchor_byte_starts.len() + 1,
    "syntax location offsets include one EOF offset after token anchors"
)]
#[invariant(self.effective_fail_token_indices.len() == self.consumed_recovery_directives)]
#[expensive_invariant(
    true,
    "syntax memo keys are protected by ParserState's private mutation APIs"
)]
impl<'tokens> ParserState<'tokens> {
    #[requires(true)]
    #[ensures(ret.anchor_byte_starts.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    pub(super) fn new(words: &[Token], options: &ParseOptions) -> Self {
        Self {
            anchor_byte_starts: words.iter().map(word_anchor_byte_start).collect(),
            syntax_location_byte_offsets: syntax_location_byte_offsets(words),
            cmavo_cache: HashMap::new(),
            syntax_memo: HashMap::new(),
            syntax_recovery_memo: HashMap::new(),
            syntax_failure_memo: HashMap::new(),
            syntax_recovery_failure_memo: HashMap::new(),
            syntax_memo_in_progress: HashSet::new(),
            syntax_recovery_memo_in_progress: HashSet::new(),
            diagnostic_candidates: Vec::new(),
            warnings: Vec::new(),
            trace: TraceRecorder::new(options.trace.clone(), TracePhase::Syntax),
            active_syntax_contexts: Vec::new(),
            active_syntax_rules: Vec::new(),
            recovery_directives: Vec::new(),
            consumed_recovery_directives: 0,
            effective_fail_token_indices: Vec::new(),
            active_recovery_directive: None,
            recovery_tokens: Vec::new(),
            recovery_source: None,
            track_recovery_branches: false,
            syntax_grammar_env: generated_runtime::SyntaxGrammarEnv::from_options(options),
            _tokens: PhantomData,
        }
    }

    #[requires(true)]
    #[ensures(ret.anchor_byte_starts.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    pub(super) fn new_with_recovery_branches(words: &[Token], options: &ParseOptions) -> Self {
        let mut state = Self::new(words, options);
        state.track_recovery_branches = true;
        state
    }

    #[requires(true)]
    #[ensures(ret.anchor_byte_starts.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    pub(super) fn new_with_recovery(
        words: &[Token],
        source: Option<&str>,
        options: &ParseOptions,
        directives: &[RecoveryDirective],
    ) -> Self {
        let mut state = Self::new_with_recovery_branches(words, options);
        state.recovery_directives = directives.to_vec();
        state.recovery_tokens = words.to_vec();
        state.recovery_source = source.map(Arc::<str>::from);
        state
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn token_cmavo(&mut self, token: &Token) -> Option<Cmavo> {
        let range = token
            .core_word()
            .byte_range()
            .expect("syntax tokens have source byte ranges");
        let key = (range.start, range.end);
        if let Some(cmavo) = self.cmavo_cache.get(&key) {
            *cmavo
        } else {
            let cmavo = token.core_word().cmavo();
            self.cmavo_cache.insert(key, cmavo);
            cmavo
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn syntax_grammar_env(&self) -> generated_runtime::SyntaxGrammarEnv {
        self.syntax_grammar_env
    }

    #[requires(true)]
    #[ensures(ret == !self.recovery_directives.is_empty())]
    pub(super) fn recovery_enabled(&self) -> bool {
        !self.recovery_directives.is_empty()
    }

    #[requires(true)]
    #[ensures(ret == self.track_recovery_branches)]
    pub(super) fn recovery_branch_tracking_enabled(&self) -> bool {
        self.track_recovery_branches
    }

    #[requires(true)]
    #[ensures(ret == self.consumed_recovery_directives)]
    pub(super) fn syntax_recovery_memo_index(&self) -> usize {
        self.consumed_recovery_directives
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(true)]
    // This method is called immediately before recursive rule descent. Keeping
    // recovery memo replay out of the wasm caller bounds every rule frame even
    // as replay bookkeeping evolves.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    pub(super) fn syntax_memo_success<O: 'static>(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
    ) -> Option<(
        parser_core::SharedSyntaxOutput<O>,
        usize,
        Arc<[SyntaxWarning]>,
    )> {
        if self.recovery_enabled() {
            let memo =
                self.syntax_recovery_memo
                    .get(&(rule_name, start_location, recovery_index))?;
            let value = memo
                .value
                .value
                .downcast_ref::<parser_core::SharedSyntaxOutput<O>>()?
                .clone();
            let end_location = memo.end_location;
            let warnings = Arc::clone(&memo.warnings);
            let consumed_recovery_directives = memo.consumed_recovery_directives;
            self.effective_fail_token_indices
                .extend_from_slice(&memo.effective_fail_token_indices);
            self.consumed_recovery_directives = consumed_recovery_directives;
            return Some((value, end_location, warnings));
        }
        let (value, end_location, warnings) = {
            let memo = self.syntax_memo.get(&(rule_name, start_location))?;
            let value = memo
                .value
                .value
                .downcast_ref::<parser_core::SharedSyntaxOutput<O>>()?
                .clone();
            (value, memo.end_location, Arc::clone(&memo.warnings))
        };
        Some((value, end_location, warnings))
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(true)]
    pub(super) fn syntax_memo_failure(
        &self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
    ) -> Option<SyntaxParseError<'tokens>> {
        if self.recovery_enabled() {
            return self
                .syntax_recovery_failure_memo
                .get(&(rule_name, start_location, recovery_index))
                .cloned();
        }
        let _ = recovery_index;
        self.syntax_failure_memo
            .get(&(rule_name, start_location))
            .cloned()
    }

    #[requires(!rule_name.is_empty())]
    #[requires(end_location >= start_location)]
    #[requires(recovery_index <= self.consumed_recovery_directives)]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || end_location < self.syntax_location_byte_offsets.len())]
    #[ensures(self.recovery_enabled() || self.syntax_memo.contains_key(&(rule_name, start_location)))]
    #[ensures(!self.recovery_enabled() || self.syntax_recovery_memo.contains_key(&(rule_name, start_location, recovery_index)))]
    // Do not fold recovery side-effect snapshots into the recursive wasm rule
    // wrapper; V8 reserves frame space for inlined locals across the descent.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    pub(super) fn store_syntax_memo_success<O: 'static>(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
        end_location: usize,
        value: parser_core::SharedSyntaxOutput<O>,
        warnings: Vec<SyntaxWarning>,
    ) {
        let effective_fail_token_indices = self.effective_fail_token_indices
            [recovery_index..self.consumed_recovery_directives]
            .to_vec();
        let success = new!(SyntaxMemoSuccess {
            start_location,
            end_location,
            recovery_index,
            consumed_recovery_directives: self.consumed_recovery_directives,
            effective_fail_token_indices,
            value: SyntaxMemoValue {
                value: Arc::new(value),
            },
            warnings: warnings.into(),
        });
        if self.recovery_enabled() {
            self.syntax_recovery_memo
                .insert((rule_name, start_location, recovery_index), success);
        } else {
            let _ = recovery_index;
            self.syntax_memo
                .insert((rule_name, start_location), success);
        }
    }

    #[requires(!rule_name.is_empty())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[ensures(self.recovery_enabled() || self.syntax_failure_memo.contains_key(&(rule_name, start_location)))]
    #[ensures(!self.recovery_enabled() || self.syntax_recovery_failure_memo.contains_key(&(rule_name, start_location, recovery_index)))]
    pub(super) fn store_syntax_memo_failure(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
        error: SyntaxParseError<'tokens>,
    ) {
        if self.recovery_enabled() {
            self.syntax_recovery_failure_memo
                .insert((rule_name, start_location, recovery_index), error);
            return;
        }
        let _ = recovery_index;
        self.syntax_failure_memo
            .insert((rule_name, start_location), error);
    }

    #[requires(!rule_name.is_empty())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[ensures(ret && !self.recovery_enabled() -> self.syntax_memo_in_progress.contains(&(rule_name, start_location)))]
    #[ensures(ret && self.recovery_enabled() -> self.syntax_recovery_memo_in_progress.contains(&(rule_name, start_location, recovery_index)))]
    pub(super) fn enter_syntax_memo_rule(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
    ) -> bool {
        if self.recovery_enabled() {
            self.syntax_recovery_memo_in_progress.insert((
                rule_name,
                start_location,
                recovery_index,
            ))
        } else {
            let _ = recovery_index;
            self.syntax_memo_in_progress
                .insert((rule_name, start_location))
        }
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(!self.syntax_memo_in_progress.contains(&(rule_name, start_location)))]
    #[ensures(!self.syntax_recovery_memo_in_progress.contains(&(rule_name, start_location, recovery_index)))]
    pub(super) fn exit_syntax_memo_rule(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        recovery_index: usize,
    ) {
        if self.recovery_enabled() {
            self.syntax_recovery_memo_in_progress.remove(&(
                rule_name,
                start_location,
                recovery_index,
            ));
        } else {
            let _ = recovery_index;
            self.syntax_memo_in_progress
                .remove(&(rule_name, start_location));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_diagnostic_candidate(&mut self, error: SyntaxParseError<'tokens>) {
        let error = error
            .with_active_contexts(&self.active_syntax_contexts)
            .with_active_rule_contexts(&self.active_syntax_rules);
        let Some(farthest_start) = self
            .diagnostic_candidates
            .first()
            .map(|candidate| candidate.span().start)
        else {
            self.diagnostic_candidates.push(error);
            return;
        };
        match error.span().start.cmp(&farthest_start) {
            std::cmp::Ordering::Greater => {
                self.diagnostic_candidates.clear();
                self.diagnostic_candidates.push(error);
            }
            std::cmp::Ordering::Equal => {
                if !self
                    .diagnostic_candidates
                    .iter()
                    .any(|candidate| candidate.same_report_content(&error))
                {
                    self.diagnostic_candidates.push(error);
                }
            }
            std::cmp::Ordering::Less => {}
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == self.diagnostic_candidates.len())]
    pub(super) fn diagnostic_candidates_snapshot(&self) -> Vec<SyntaxParseError<'tokens>> {
        self.diagnostic_candidates.clone()
    }

    #[requires(true)]
    #[ensures(self.diagnostic_candidates.len() == old(snapshot.len()))]
    pub(super) fn restore_diagnostic_candidates(
        &mut self,
        snapshot: Vec<SyntaxParseError<'tokens>>,
    ) {
        self.diagnostic_candidates = snapshot;
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn restore_diagnostic_candidates_preserving_start(
        &mut self,
        snapshot: Vec<SyntaxParseError<'tokens>>,
        start: usize,
    ) {
        let preserved = self
            .diagnostic_candidates
            .iter()
            .filter(|candidate| candidate.span().start == start)
            .cloned()
            .collect::<Vec<_>>();
        self.diagnostic_candidates = snapshot;
        for candidate in preserved {
            self.record_diagnostic_candidate(candidate);
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(self.active_syntax_contexts.len() == old(self.active_syntax_contexts.len()) + 1)]
    pub(super) fn push_syntax_context(&mut self, construct: &'static str, byte_start: usize) {
        self.active_syntax_contexts
            .push(SyntaxContextFrame::new(construct, byte_start));
    }

    #[requires(!self.active_syntax_contexts.is_empty())]
    #[ensures(self.active_syntax_contexts.len() + 1 == old(self.active_syntax_contexts.len()))]
    pub(super) fn pop_syntax_context(&mut self) {
        self.active_syntax_contexts
            .pop()
            .expect("syntax context stack is non-empty");
    }

    #[requires(!rule.is_empty())]
    #[ensures(self.active_syntax_rules.len() == old(self.active_syntax_rules.len()) + 1)]
    pub(super) fn push_syntax_rule(&mut self, rule: &'static str, byte_start: usize) {
        self.active_syntax_rules
            .push(SyntaxRuleFrame::new(rule, byte_start));
    }

    #[requires(!self.active_syntax_rules.is_empty())]
    #[ensures(self.active_syntax_rules.len() + 1 == old(self.active_syntax_rules.len()))]
    pub(super) fn pop_syntax_rule(&mut self) {
        self.active_syntax_rules
            .pop()
            .expect("syntax rule stack is non-empty");
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn diagnostic_candidate(&self) -> Option<SyntaxParseError<'tokens>> {
        self.diagnostic_candidates
            .clone()
            .into_iter()
            .reduce(SyntaxParseError::merge_for_report)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn byte_offset_for_location(&self, location: usize) -> usize {
        self.syntax_location_byte_offsets
            .get(location)
            .copied()
            .unwrap_or_else(|| {
                self.syntax_location_byte_offsets
                    .last()
                    .copied()
                    .unwrap_or(0)
            })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn active_syntax_contexts(&self) -> &[SyntaxContextFrame] {
        &self.active_syntax_contexts
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn active_syntax_rules(&self) -> &[SyntaxRuleFrame] {
        &self.active_syntax_rules
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    pub(super) fn recovery_field_action(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        input_location: usize,
        field_can_be_absent: bool,
    ) -> Option<RecoveryFieldAction> {
        self.recovery_field_action_with_match(
            rule,
            instance_byte_start,
            field_index,
            input_location,
            field_can_be_absent,
            false,
        )
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    // Natural-stop matching runs only after the repeated item parser has
    // returned. Keep it out of that parser's live wasm recursion frame.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    pub(super) fn recovery_field_action_at_natural_stop(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        input_location: usize,
        field_can_be_absent: bool,
    ) -> Option<RecoveryFieldAction> {
        self.recovery_field_action_with_match(
            rule,
            instance_byte_start,
            field_index,
            input_location,
            field_can_be_absent,
            true,
        )
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    fn recovery_field_action_with_match(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        input_location: usize,
        field_can_be_absent: bool,
        allow_earlier_natural_stop: bool,
    ) -> Option<RecoveryFieldAction> {
        if self.active_recovery_directive.is_none() {
            let directive_index = self.consumed_recovery_directives;
            let directive = self.recovery_directives.get(directive_index)?.clone();
            if directive.rule != rule
                || directive.instance_byte_start != instance_byte_start
                || if allow_earlier_natural_stop {
                    !directive.natural_stop_enabled
                        || input_location >= directive.fail_token_index
                        || !directive.can_fire_at(input_location)
                } else {
                    directive.fail_token_index != input_location
                }
                || if allow_earlier_natural_stop {
                    self.byte_offset_for_location(input_location) <= directive.instance_byte_start
                        || (directive.resume_field != usize::MAX
                            && field_index != directive.resume_field)
                } else {
                    field_index > directive.resume_field
                }
            {
                return None;
            }
            self.effective_fail_token_indices.push(input_location);
            self.consumed_recovery_directives += 1;
            self.active_recovery_directive = Some(new!(ActiveRecoveryDirective {
                directive,
                effective_fail_token_index: input_location,
                skipped_item_emitted: false,
            }));
        }

        let active = self.active_recovery_directive.as_ref()?;
        if active.directive.rule != rule
            || active.directive.instance_byte_start != instance_byte_start
        {
            return None;
        }
        if active.directive.resume_field == usize::MAX
            && active.effective_fail_token_index < active.directive.fail_token_index
        {
            let active = self
                .active_recovery_directive
                .take()
                .expect("active directive was just inspected");
            let item = self
                .recovery_item_for_directive(&active.directive, active.effective_fail_token_index);
            return Some(RecoveryFieldAction::resume(
                Some(item),
                active.directive.resume_token_index,
            ));
        }
        if field_index < active.directive.resume_field {
            if field_can_be_absent {
                return Some(RecoveryFieldAction::abandon(None));
            }
            let directive = active.directive.clone();
            let effective_fail_token_index = active.effective_fail_token_index;
            let skipped_item_emitted = active.skipped_item_emitted;
            if let Some(active) = self.active_recovery_directive.take() {
                self.active_recovery_directive = Some(active.with_data(data! {
                    skipped_item_emitted: true,
                }));
            }
            let item = if skipped_item_emitted {
                self.missing_recovery_item_for_directive(&directive)
            } else {
                self.recovery_item_for_directive(&directive, effective_fail_token_index)
            };
            return Some(RecoveryFieldAction::abandon(Some(item)));
        }
        if field_index == active.directive.resume_field {
            let directive = active.directive.clone();
            let effective_fail_token_index = active.effective_fail_token_index;
            let skipped_item_emitted = active.skipped_item_emitted;
            self.active_recovery_directive = None;
            let item = (!skipped_item_emitted)
                .then(|| self.recovery_item_for_directive(&directive, effective_fail_token_index));
            return Some(RecoveryFieldAction::resume(
                item,
                directive.resume_token_index,
            ));
        }
        None
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    pub(super) fn trailing_recovery_field_action(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        input_location: usize,
    ) -> Option<(SyntaxRecoveryItem, usize)> {
        if self.active_recovery_directive.is_some() {
            return None;
        }
        let directive = self
            .recovery_directives
            .get(self.consumed_recovery_directives)?
            .clone();
        if directive.rule != rule
            || directive.instance_byte_start != instance_byte_start
            || directive.fail_token_index != input_location
            || field_index > directive.resume_field
        {
            return None;
        }
        let item = skipped_recovery_item(
            directive.error_index,
            &self.recovery_tokens,
            &directive,
            input_location,
        )?;
        self.effective_fail_token_indices.push(input_location);
        self.consumed_recovery_directives += 1;
        if field_index < directive.resume_field {
            self.active_recovery_directive = Some(new!(ActiveRecoveryDirective {
                directive: directive.clone(),
                effective_fail_token_index: input_location,
                skipped_item_emitted: true,
            }));
        }
        Some((item, directive.resume_token_index))
    }

    #[requires(true)]
    #[ensures(ret <= self.recovery_directives.len())]
    pub(super) fn unconsumed_recovery_directives(&self) -> usize {
        self.recovery_directives
            .len()
            .saturating_sub(self.consumed_recovery_directives)
    }

    #[requires(directive.fail_token_index <= directive.resume_token_index)]
    #[requires(effective_fail_token_index <= directive.fail_token_index)]
    #[requires(directive.fail_token_index < directive.resume_token_index || effective_fail_token_index == directive.fail_token_index)]
    #[ensures(true)]
    pub(super) fn recovery_item_for_directive(
        &self,
        directive: &RecoveryDirective,
        effective_fail_token_index: usize,
    ) -> SyntaxRecoveryItem {
        skipped_recovery_item(
            directive.error_index,
            &self.recovery_tokens,
            directive,
            effective_fail_token_index,
        )
        .unwrap_or_else(|| {
            missing_recovery_item(
                directive.error_index,
                &self.recovery_tokens,
                self.recovery_source.as_deref(),
                directive,
            )
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_recovery_item_for_directive(
        &self,
        directive: &RecoveryDirective,
    ) -> SyntaxRecoveryItem {
        missing_recovery_item(
            directive.error_index,
            &self.recovery_tokens,
            self.recovery_source.as_deref(),
            directive,
        )
    }

    #[requires(true)]
    #[ensures(ret <= self.warnings.len())]
    pub(super) fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    #[requires(start <= self.warnings.len())]
    #[ensures(ret.len() + start == self.warnings.len())]
    pub(super) fn warnings_since(&self, start: usize) -> Vec<SyntaxWarning> {
        self.warnings[start..].to_vec()
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + warnings.len())]
    pub(super) fn extend_warnings(&mut self, warnings: &[SyntaxWarning]) {
        self.warnings.extend_from_slice(warnings);
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + 1)]
    pub(super) fn warn(&mut self, construct: ExperimentalConstruct, anchor: &Token) {
        let anchor_index = self.anchor_index(anchor);
        let anchor = Token::bare(anchor.core_word().clone());
        self.warnings.push(SyntaxWarning::experimental_construct(
            construct,
            anchor_index,
            anchor,
        ));
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + 1)]
    pub(super) fn warn_word(
        &mut self,
        construct: ExperimentalConstruct,
        context: &Token,
        anchor: &Word,
    ) {
        let anchor_index = self.anchor_index(context);
        self.warnings.push(SyntaxWarning::experimental_construct(
            construct,
            anchor_index,
            Token::bare(WordLike::bare(anchor.clone())),
        ));
    }

    #[requires(true)]
    #[ensures(ret.trace.as_ref().is_none_or(|report| report.phase == TracePhase::Syntax))]
    #[ensures(ret.effective_fail_token_indices.len() + ret.unconsumed_recovery_directives == ret.recovery_directives.len())]
    pub(super) fn finish(mut self) -> ParserStateFinish {
        let unconsumed_recovery_directives = self.unconsumed_recovery_directives();
        let effective_fail_token_indices = std::mem::take(&mut self.effective_fail_token_indices);
        self.consumed_recovery_directives = 0;
        let trace = self.trace.finish();
        let mut deduped = Vec::new();
        for warning in self.warnings {
            if !deduped.contains(&warning) {
                deduped.push(warning);
            }
        }
        ParserStateFinish {
            warnings: deduped,
            trace,
            unconsumed_recovery_directives,
            recovery_directives: self.recovery_directives,
            effective_fail_token_indices,
        }
    }

    #[requires(true)]
    #[ensures(matches!(self.trace, TraceRecorder::Disabled) -> !ret)]
    pub(super) fn trace_enabled(&self) -> bool {
        self.trace.is_enabled()
    }

    #[requires(true)]
    #[ensures(matches!(self.trace, TraceRecorder::Disabled) -> !ret)]
    pub(super) fn trace_should_record(&self, level: TraceLevel, label: &str) -> bool {
        self.trace.should_record(level, label)
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_event(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        self.trace
            .record_with_detail(level, kind, label, byte_start, byte_end, detail);
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_enter_construct(
        &mut self,
        level: TraceLevel,
        label: &str,
        byte_start: usize,
        byte_end: usize,
    ) {
        self.trace
            .enter_construct(level, label, byte_start, byte_end);
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(true)]
    pub(super) fn trace_exit_construct(
        &mut self,
        level: TraceLevel,
        kind: TraceEventKind,
        label: &str,
        byte_start: usize,
        byte_end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        self.trace
            .exit_construct(level, kind, label, byte_start, byte_end, detail);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn trace_failure_summary(&mut self, failure: TraceFailureSummary) {
        self.trace.set_failure(failure);
    }

    #[requires(true)]
    #[ensures(ret < self.anchor_byte_starts.len() || self.anchor_byte_starts.is_empty())]
    fn anchor_index(&self, anchor: &Token) -> usize {
        if let Some(anchor_start) = word_anchor_byte_start(anchor)
            && let Some(index) = self
                .anchor_byte_starts
                .iter()
                .position(|candidate| *candidate == Some(anchor_start))
        {
            return index;
        }
        0
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_contexts_are_compatible(
    left: &SyntaxParseError<'_>,
    right: &SyntaxParseError<'_>,
) -> bool {
    match (left.preferred_context(), right.preferred_context()) {
        (None, _) | (_, None) => true,
        (Some(left), Some(right)) => left.construct == right.construct,
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_context_can_refine(
    current: &SyntaxParseError<'_>,
    candidate: &SyntaxParseError<'_>,
) -> bool {
    let Some(current_context) = current.preferred_context() else {
        return true;
    };
    let Some(candidate_context) = candidate.preferred_context() else {
        return false;
    };
    if !syntax_construct_is_descendant_of(&current_context.construct, &candidate_context.construct)
    {
        return false;
    }
    let Some(child) =
        syntax_immediate_child_under(&current_context.construct, &candidate_context.construct)
    else {
        return false;
    };
    !diagnostic_expectations_include_construct(current, &child)
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_context_covers_descendant(
    candidate: &SyntaxParseError<'_>,
    current: &SyntaxParseError<'_>,
) -> bool {
    let Some(candidate_context) = candidate.preferred_context() else {
        return false;
    };
    let Some(current_context) = current.preferred_context() else {
        return false;
    };
    if !syntax_construct_is_descendant_of(&candidate_context.construct, &current_context.construct)
    {
        return false;
    }
    let Some(child) =
        syntax_immediate_child_under(&candidate_context.construct, &current_context.construct)
    else {
        return false;
    };
    if current_context.construct != child {
        return false;
    }
    diagnostic_expectations_include_construct(candidate, &child)
}

#[requires(!construct.is_empty())]
#[ensures(true)]
fn diagnostic_expectations_include_construct(
    error: &SyntaxParseError<'_>,
    construct: &str,
) -> bool {
    error
        .clone()
        .into_report_error()
        .expectations()
        .iter()
        .any(|expectation| expectation.reason.construct() == construct)
}

#[bityzba::contract_trait]
impl<'tokens> Inspector<'tokens> for ParserState<'tokens> {
    #[requires(true)]
    #[ensures(true)]
    fn on_token(&mut self, token: &Token) {
        if !self.trace_should_record(TraceLevel::Primitives, "token") {
            return;
        }
        let span = token
            .source_spans()
            .into_iter()
            .next()
            .map(|span| span.byte_start..span.byte_end)
            .expect("syntax tokens have source byte ranges");
        self.trace_event(
            TraceLevel::Primitives,
            TraceEventKind::Token,
            "token",
            span.start,
            span.end,
            || Some(trace_word_label(token)),
        );
    }

    #[requires(true)]
    #[ensures(ret.warning_count == self.warnings.len())]
    fn on_save<'parse>(&self, _cursor: &Cursor<'tokens, 'parse>) -> ParserCheckpoint {
        ParserCheckpoint {
            warning_count: self.warnings.len(),
            syntax_context_count: self.active_syntax_contexts.len(),
            syntax_rule_count: self.active_syntax_rules.len(),
            recovery: self.recovery_enabled().then(|| {
                new!(ParserRecoveryCheckpoint {
                    consumed_recovery_directives: self.consumed_recovery_directives,
                    active_recovery_directive: self.active_recovery_directive.clone(),
                })
            }),
            trace_save: self.trace_should_record(TraceLevel::Primitives, "save"),
        }
    }

    #[requires(true)]
    #[ensures(self.warnings.len() <= old(self.warnings.len()))]
    fn on_rewind<'parse>(&mut self, marker: &Checkpoint<'tokens, 'parse>) {
        if marker.inspector().trace_save {
            self.trace_event(
                TraceLevel::Primitives,
                TraceEventKind::Save,
                "save",
                0,
                0,
                || None,
            );
        }
        self.trace_event(
            TraceLevel::Primitives,
            TraceEventKind::Rewind,
            "rewind",
            0,
            0,
            || None,
        );
        self.warnings.truncate(marker.inspector().warning_count);
        self.active_syntax_contexts
            .truncate(marker.inspector().syntax_context_count);
        self.active_syntax_rules
            .truncate(marker.inspector().syntax_rule_count);
        if let Some(recovery) = &marker.inspector().recovery {
            self.consumed_recovery_directives = recovery.consumed_recovery_directives;
            self.effective_fail_token_indices
                .truncate(self.consumed_recovery_directives);
            self.active_recovery_directive = recovery.active_recovery_directive.clone();
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_word_label(token: &Token) -> String {
    token.core_word().to_string()
}

#[requires(true)]
#[ensures(true)]
fn word_anchor_byte_start(word: &Token) -> Option<usize> {
    word.core_word()
        .source_spans()
        .into_iter()
        .map(|span| span.byte_start)
        .min()
}

#[requires(true)]
#[ensures(ret.len() == words.len() + 1)]
fn syntax_location_byte_offsets(words: &[Token]) -> Vec<usize> {
    let mut offsets = words
        .iter()
        .map(|word| word.core_word().byte_range().map_or(0, |range| range.start))
        .collect::<Vec<_>>();
    offsets.push(
        words
            .last()
            .and_then(|word| word.core_word().byte_range())
            .map_or(0, |range| range.end),
    );
    offsets
}

#[requires(true)]
#[ensures(true)]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    crate::generated_model_text_syntax_leaf_spans_match_words(words, &parse.parse_tree)
}))]
#[expensive_ensures(ret.as_ref().map_or(true, |parse| {
    crate::generated_model_recovered_round_trip_matches_valid(&parse.parse_tree)
}))]
pub(crate) fn parse_syntax_tree(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    parse_generated_model_syntax_tree_with_source_attempt(words, None, options).result
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_generated_model_syntax_tree_with_source(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> Result<Box<generated::generated_model::TextSyntax>, SyntaxError> {
    parse_generated_model_syntax_tree_with_source_attempt(words, source, options)
        .result
        .map(|parsed| parsed.into_data().parse_tree)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_generated_model_syntax_tree_with_source_attempt(
    words: &[WordLike],
    _source: Option<&str>,
    options: &ParseOptions,
) -> SyntaxParseAttempt {
    let tokens = syntax_tokens(words, options);
    let parsed = generated::generated_model::parse_text_attempt(&tokens, options);
    let result = parsed.result.map(|parsed| {
        let mut warnings = parsed.warnings;
        add_generated_construct_warnings(&parsed.text, &tokens, &mut warnings);
        new!(SyntaxParse {
            parse_tree: Box::new(parsed.text),
            warnings,
        })
    });
    SyntaxParseAttempt {
        result,
        trace: parsed.trace,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_recovered_generated_model_syntax_tree_with_source_attempt(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> RecoveredSyntaxParseAttempt {
    let attempt = parse_generated_model_syntax_tree_with_recovery_attempt(words, source, options);
    let result = match attempt.result.into_data() {
        data!(SyntaxRecoveryParse::Valid { parse }) => {
            let parse = parse.into_data();
            new!(RecoveredSyntaxParse {
                parse_tree: Box::new(
                    generated::generated_model::recovered::TextSyntax::from_valid(
                        *parse.parse_tree,
                    ),
                ),
                errors: Vec::new(),
                warnings: parse.warnings,
            })
        }
        data!(SyntaxRecoveryParse::Recovered { parse }) => parse,
    };
    RecoveredSyntaxParseAttempt {
        result,
        trace: attempt.trace,
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_generated_model_syntax_tree_with_recovery_attempt(
    words: &[WordLike],
    source: Option<&str>,
    options: &ParseOptions,
) -> SyntaxRecoveryParseAttempt {
    let tokens = syntax_tokens(words, options);
    let strict_attempt = generated::generated_model::parse_text_attempt(&tokens, options);
    if let Ok(parsed) = strict_attempt.result {
        return valid_syntax_recovery_attempt(parsed, &tokens, strict_attempt.trace);
    }

    let tracked_attempt =
        generated::generated_model::parse_text_detailed_tracked_attempt(&tokens, options);
    let failure = match tracked_attempt.result {
        Ok(parsed) => {
            return valid_syntax_recovery_attempt(parsed, &tokens, tracked_attempt.trace);
        }
        Err(failure) => failure,
    };

    let recovered =
        recover_after_strict_failure(tokens, source, options, failure, tracked_attempt.trace);
    SyntaxRecoveryParseAttempt {
        result: new!(SyntaxRecoveryParse::Recovered {
            parse: recovered.result,
        }),
        trace: recovered.trace,
    }
}

#[requires(true)]
#[ensures(matches!(ret.result.as_data(), SyntaxRecoveryParseData::Valid { .. }))]
fn valid_syntax_recovery_attempt(
    parsed: generated::generated_model::GeneratedParsedText,
    tokens: &[Token],
    trace: Option<TraceReport>,
) -> SyntaxRecoveryParseAttempt {
    let mut warnings = parsed.warnings;
    add_generated_construct_warnings(&parsed.text, tokens, &mut warnings);
    SyntaxRecoveryParseAttempt {
        result: new!(SyntaxRecoveryParse::Valid {
            parse: new!(SyntaxParse {
                parse_tree: Box::new(parsed.text),
                warnings,
            }),
        }),
        trace,
    }
}

#[requires(true)]
#[ensures(true)]
fn recover_after_strict_failure(
    tokens: Vec<Token>,
    source: Option<&str>,
    options: &ParseOptions,
    mut failure: generated::generated_model::GeneratedParseFailure,
    mut trace: Option<TraceReport>,
) -> RecoveredSyntaxParseAttempt {
    let cap = options.max_recovery_errors.get();
    let initial_failure = failure.clone();
    let mut initial_has_anchor_candidates = false;
    let mut errors = vec![failure.public_error.clone()];
    let mut directives = Vec::new();

    while errors.len() < cap {
        let candidates = select_recovery_directives(&tokens, &failure, options, errors.len() - 1);
        if errors.len() == 1 {
            initial_has_anchor_candidates = !candidates.is_empty();
        }
        let candidates = if candidates.is_empty() {
            select_final_recovery_directives(&tokens, &failure, errors.len() - 1)
        } else {
            candidates
        };
        if candidates.is_empty() {
            break;
        }

        let mut accepted_progress = None;
        let mut exact_position_success = None;
        'recovery_phases: for natural_stop_enabled in [false, true] {
            let trial_limit = if natural_stop_enabled {
                MAX_NATURAL_STOP_DIRECTIVE_TRIALS_PER_ERROR
            } else {
                LEGACY_RECOVERY_DIRECTIVE_TRIALS_PER_ERROR
            };
            let mut trial_count = 0usize;
            for directive in candidates.iter().cloned() {
                if directives
                    .iter()
                    .any(|existing| same_recovery_site(existing, &directive))
                {
                    continue;
                }
                if trial_count >= trial_limit {
                    break;
                }
                trial_count += 1;
                let directive = if natural_stop_enabled {
                    directive.with_natural_stop_enabled()
                } else {
                    directive
                };
                let mut trial_directives = directives.clone();
                trial_directives.push(directive.clone());

                let attempt = generated::generated_model::parse_recovered_text_attempt(
                    &tokens,
                    source,
                    options,
                    &trial_directives,
                );
                let generated::generated_model::GeneratedRecoveredParsedTextAttempt {
                    result,
                    trace: attempt_trace,
                    unconsumed_directives,
                    recovery_directives: applied_directives,
                    effective_fail_token_indices: applied_effective_fail_token_indices,
                } = attempt;
                let fired_left_of_declared_failure = applied_directives
                    .last()
                    .zip(applied_effective_fail_token_indices.last())
                    .is_some_and(|(directive, effective_fail_token_index)| {
                        effective_fail_token_index < &directive.fail_token_index
                    });
                match result {
                    Ok(parsed) if unconsumed_directives == 0 => {
                        let success = RecoveredSyntaxParseAttempt {
                            result: new!(RecoveredSyntaxParse {
                                parse_tree: Box::new(parsed.text),
                                errors: errors.clone(),
                                warnings: parsed.warnings,
                            }),
                            trace: attempt_trace,
                        };
                        if !natural_stop_enabled || fired_left_of_declared_failure {
                            return success;
                        }
                        if !directives.is_empty() && exact_position_success.is_none() {
                            exact_position_success = Some(success);
                        }
                    }
                    Ok(_) => {
                        trace = attempt_trace;
                    }
                    Err(next_failure) => {
                        if errors.len() >= cap {
                            break;
                        }
                        let next_error_start = syntax_error_start(&next_failure.public_error);
                        if next_error_start
                            <= recovery_byte_at(&tokens, directive.resume_token_index)
                            || next_error_start <= errors.last().map_or(0, syntax_error_start)
                        {
                            trace = attempt_trace;
                            continue;
                        }
                        if accepted_progress.is_none() {
                            accepted_progress = Some(new!(RecoveryProgressTrial {
                                directives: applied_directives,
                                failure: next_failure,
                                trace: attempt_trace,
                            }));
                        }
                        if !natural_stop_enabled {
                            break 'recovery_phases;
                        }
                    }
                }
            }
        }

        // The wider phase preserves natural-stop recoveries as its first
        // priority. After prior progress, if none fires left of the next
        // declared failure, a late exact-site success is still a complete
        // recovery and is preferable to degrading the entire parse.
        if let Some(success) = exact_position_success {
            return success;
        }

        let Some(progress) = accepted_progress else {
            break;
        };
        let data!(RecoveryProgressTrial {
            directives: trial_directives,
            failure: next_failure,
            trace: progress_trace,
        }) = progress.into_data();
        directives = trial_directives;
        errors.push(next_failure.public_error.clone());
        failure = next_failure;
        trace = progress_trace;
    }

    if initial_has_anchor_candidates && errors.len() > 1 {
        if let Some(recovered) = try_final_recovery_from_initial_failure(
            &tokens,
            source,
            options,
            &initial_failure,
            &errors,
        ) {
            return recovered;
        }
    }

    let parse_tree = degraded_recovered_text(&tokens, source, &errors);
    RecoveredSyntaxParseAttempt {
        result: new!(RecoveredSyntaxParse {
            parse_tree: Box::new(parse_tree),
            errors,
            warnings: Vec::new(),
        }),
        trace,
    }
}

#[requires(!errors.is_empty())]
#[ensures(ret.as_ref().is_none_or(|attempt| !attempt.result.errors.is_empty()))]
fn try_final_recovery_from_initial_failure(
    tokens: &[Token],
    source: Option<&str>,
    options: &ParseOptions,
    initial_failure: &generated::generated_model::GeneratedParseFailure,
    errors: &[SyntaxError],
) -> Option<RecoveredSyntaxParseAttempt> {
    for directive in select_final_recovery_directives(tokens, initial_failure, 0) {
        let attempt = generated::generated_model::parse_recovered_text_attempt(
            tokens,
            source,
            options,
            std::slice::from_ref(&directive),
        );
        let generated::generated_model::GeneratedRecoveredParsedTextAttempt {
            result,
            trace,
            unconsumed_directives,
            ..
        } = attempt;
        if let Ok(parsed) = result
            && unconsumed_directives == 0
        {
            return Some(RecoveredSyntaxParseAttempt {
                result: new!(RecoveredSyntaxParse {
                    parse_tree: Box::new(parsed.text),
                    errors: errors.to_vec(),
                    warnings: parsed.warnings,
                }),
                trace,
            });
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn same_recovery_site(left: &RecoveryDirective, right: &RecoveryDirective) -> bool {
    left.fail_token_index == right.fail_token_index
        && left.resume_token_index == right.resume_token_index
        && left.resume_field == right.resume_field
        && left.rule == right.rule
        && left.instance_byte_start == right.instance_byte_start
}

#[invariant(!rule.is_empty())]
#[derive(Debug, Clone)]
struct RecoveryClaim {
    branch_index: usize,
    inner_rank: usize,
    rule: &'static str,
    instance_byte_start: usize,
    resume_token_index: usize,
    resume_field: usize,
}

#[invariant(!directives.is_empty())]
struct RecoveryProgressTrial {
    directives: Vec<RecoveryDirective>,
    failure: generated::generated_model::GeneratedParseFailure,
    trace: Option<TraceReport>,
}

#[requires(true)]
#[ensures(ret.iter().all(|directive| directive.fail_token_index <= directive.resume_token_index))]
fn select_recovery_directives(
    tokens: &[Token],
    failure: &generated::generated_model::GeneratedParseFailure,
    options: &ParseOptions,
    error_index: usize,
) -> Vec<RecoveryDirective> {
    let fail_token_index = token_index_for_byte_start(
        tokens,
        failure.branches.first().map_or_else(
            || syntax_error_start(&failure.public_error),
            |branch| branch.span_start,
        ),
    );
    let fail_byte_start = recovery_byte_at(tokens, fail_token_index);
    let env = generated_runtime::SyntaxGrammarEnv::from_options(options);
    let mut claims = Vec::new();

    for (branch_index, branch) in failure.branches.iter().enumerate() {
        for (inner_rank, frame) in branch.active_rule_contexts.iter().rev().enumerate() {
            let Some(metadata) =
                generated::generated_model::syntax_grammar_anchor_metadata_by_rule_name(
                    frame.rule(),
                )
            else {
                continue;
            };
            for field in metadata.fields {
                for anchor in field.anchors {
                    if !recovery_anchor_origin_is_v1(anchor.origin)
                        || !recovery_conditions_match(anchor.conditions, env)
                    {
                        continue;
                    }
                    let frame_token_index = token_index_for_byte_start(tokens, frame.byte_start());
                    let frame_container_depth =
                        subtext_container_depth_before(tokens, frame_token_index);
                    let Some(resume_token_index) = scan_for_recovery_anchor(
                        tokens,
                        fail_token_index,
                        frame_container_depth,
                        anchor.start_tokens,
                    ) else {
                        continue;
                    };
                    if frame.byte_start() >= fail_byte_start
                        && resume_token_index > fail_token_index
                    {
                        continue;
                    }
                    let claim = new!(RecoveryClaim {
                        branch_index,
                        inner_rank,
                        rule: metadata.rule,
                        instance_byte_start: frame.byte_start(),
                        resume_token_index,
                        resume_field: anchor.resume_field,
                    });
                    claims.push(claim);
                }
            }
        }
    }

    claims.sort_by_key(recovery_claim_sort_key);
    let mut directives = Vec::new();
    for selected in claims {
        let directive = RecoveryDirective::new(
            selected.rule,
            selected.instance_byte_start,
            fail_token_index,
            selected.resume_token_index,
            selected.resume_field,
            error_index,
            failure.public_error.clone(),
        );
        if directives
            .iter()
            .any(|existing| same_recovery_site(existing, &directive))
        {
            continue;
        }
        directives.push(directive);
    }
    directives
}

#[requires(true)]
#[ensures(ret.iter().all(|directive| directive.resume_token_index == tokens.len()))]
fn select_final_recovery_directives(
    tokens: &[Token],
    failure: &generated::generated_model::GeneratedParseFailure,
    error_index: usize,
) -> Vec<RecoveryDirective> {
    let fail_token_index =
        token_index_for_byte_start(tokens, syntax_error_start(&failure.public_error));
    let fail_byte_start = recovery_byte_at(tokens, fail_token_index);
    let mut claims = Vec::new();
    for (branch_index, branch) in failure.branches.iter().enumerate() {
        for (inner_rank, frame) in branch.active_rule_contexts.iter().rev().enumerate() {
            if frame.byte_start() >= fail_byte_start {
                continue;
            }
            let Some(metadata) =
                generated::generated_model::syntax_grammar_anchor_metadata_by_rule_name(
                    frame.rule(),
                )
            else {
                continue;
            };
            claims.push(new!(RecoveryClaim {
                branch_index,
                inner_rank,
                rule: metadata.rule,
                instance_byte_start: frame.byte_start(),
                resume_token_index: tokens.len(),
                resume_field: usize::MAX,
            }));
        }
    }
    claims.sort_by_key(recovery_claim_sort_key);
    let mut directives = Vec::new();
    for claim in claims {
        let directive = RecoveryDirective::new(
            claim.rule,
            claim.instance_byte_start,
            fail_token_index,
            tokens.len(),
            claim.resume_field,
            error_index,
            failure.public_error.clone(),
        );
        if directives
            .iter()
            .any(|existing| same_recovery_site(existing, &directive))
        {
            continue;
        }
        directives.push(directive);
    }
    directives
}

#[requires(true)]
#[ensures(true)]
fn recovery_claim_sort_key(claim: &RecoveryClaim) -> (usize, usize, usize) {
    (
        claim.resume_token_index,
        claim.inner_rank,
        claim.branch_index,
    )
}

#[requires(true)]
#[ensures(true)]
fn syntax_error_start(error: &SyntaxError) -> usize {
    match error {
        SyntaxError::Parse { byte_start, .. } => *byte_start,
        SyntaxError::NotImplemented => 0,
    }
}

#[requires(true)]
#[ensures(ret <= tokens.len())]
fn token_index_for_byte_start(tokens: &[Token], byte_start: usize) -> usize {
    tokens
        .iter()
        .position(|token| {
            token
                .core_word()
                .byte_range()
                .is_some_and(|range| range.start >= byte_start)
        })
        .unwrap_or(tokens.len())
}

#[requires(true)]
#[ensures(true)]
fn recovery_anchor_origin_is_v1(
    origin: generated::generated_model::SyntaxGrammarAnchorOrigin,
) -> bool {
    use generated::generated_model::SyntaxGrammarAnchorOrigin::{
        FieldFirst, LiteralRun, RepetitionElementFirst,
    };
    match origin {
        LiteralRun | RepetitionElementFirst => true,
        FieldFirst => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovery_conditions_match(
    conditions: &[generated::generated_model::SyntaxGrammarCondition],
    env: generated_runtime::SyntaxGrammarEnv,
) -> bool {
    conditions
        .iter()
        .all(|condition| recovery_condition_matches(*condition, env))
}

#[requires(true)]
#[ensures(true)]
fn recovery_condition_matches(
    condition: generated::generated_model::SyntaxGrammarCondition,
    env: generated_runtime::SyntaxGrammarEnv,
) -> bool {
    use generated::generated_model::SyntaxGrammarConditionKind::{Feature, Policy};
    match condition.kind {
        Feature => recovery_feature_condition_matches(condition.name, env.dialect),
        Policy => recovery_policy_condition_matches(condition.name, env.policy),
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn recovery_feature_condition_matches(
    name: &str,
    dialect: generated_runtime::SyntaxGrammarDialect,
) -> bool {
    match name {
        "TermHierarchy" => dialect.term_hierarchy_enabled,
        "Cbm" => dialect.cbm_enabled,
        "ZantufaAdverbials" => dialect.zantufa_adverbials_enabled,
        "ZantufaConnectives" => dialect.zantufa_connectives_enabled,
        "ZantufaMex" => dialect.zantufa_mex_enabled,
        "ZantufaQuotes" => dialect.zantufa_quotes_enabled,
        "ZantufaTags" => dialect.zantufa_tags_enabled,
        "ZantufaTerms" => dialect.zantufa_terms_enabled,
        _ => false,
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn recovery_policy_condition_matches(
    name: &str,
    policy: generated_runtime::SyntaxGrammarPolicy,
) -> bool {
    match name {
        "SoiAdverbials" => policy.soi_adverbials_enabled,
        "ZantufaAdverbials" => policy.zantufa_adverbials_enabled,
        "ZantufaQuotes" => policy.zantufa_quotes_enabled,
        _ => false,
    }
}

#[requires(start <= tokens.len())]
#[ensures(ret.is_none_or(|index| index >= start && index < tokens.len()))]
fn scan_for_recovery_anchor(
    tokens: &[Token],
    start: usize,
    base_depth: usize,
    anchors: &[generated::generated_model::SyntaxGrammarAnchorToken],
) -> Option<usize> {
    let mut depth = subtext_container_depth_before(tokens, start).saturating_sub(base_depth);
    for (index, token) in tokens.iter().enumerate().skip(start) {
        let opens = token_opens_subtext_container(token);
        let closes = token_closes_subtext_container(token);
        if closes {
            if depth == 0 {
                return None;
            }
            depth -= 1;
        }
        if depth == 0
            && anchors
                .iter()
                .any(|anchor| recovery_anchor_matches(*anchor, token))
        {
            return Some(index);
        }
        if opens {
            depth += 1;
        }
    }
    None
}

#[requires(start <= tokens.len())]
#[ensures(true)]
fn subtext_container_depth_before(tokens: &[Token], start: usize) -> usize {
    let mut depth = 0usize;
    for token in tokens.iter().take(start) {
        if token_closes_subtext_container(token) && depth > 0 {
            depth -= 1;
        }
        if token_opens_subtext_container(token) {
            depth += 1;
        }
    }
    depth
}

#[requires(true)]
#[ensures(true)]
fn token_opens_subtext_container(token: &Token) -> bool {
    generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS
        .iter()
        .any(|container| {
            container
                .opener_tokens
                .iter()
                .any(|anchor| recovery_anchor_matches(*anchor, token))
        })
}

#[requires(true)]
#[ensures(true)]
fn token_closes_subtext_container(token: &Token) -> bool {
    generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS
        .iter()
        .any(|container| {
            container
                .closer_tokens
                .iter()
                .any(|anchor| recovery_anchor_matches(*anchor, token))
        })
}

#[requires(true)]
#[ensures(true)]
fn recovery_anchor_matches(
    anchor: generated::generated_model::SyntaxGrammarAnchorToken,
    token: &Token,
) -> bool {
    match anchor {
        generated::generated_model::SyntaxGrammarAnchorToken::Cmavo(cmavo) => token.is_cmavo(cmavo),
        generated::generated_model::SyntaxGrammarAnchorToken::Selmaho(selmaho) => {
            token.is_selmaho(selmaho)
        }
    }
}

#[requires(!errors.is_empty())]
#[ensures(true)]
fn degraded_recovered_text(
    tokens: &[Token],
    source: Option<&str>,
    errors: &[SyntaxError],
) -> generated::generated_model::recovered::TextSyntax {
    let mut tree = empty_recovered_text();
    let item = all_tokens_recovery_item(0, tokens)
        .unwrap_or_else(|| fallback_recovery_item(tokens, source, 0, &errors[0]));
    insert_leading_recovery_item(&mut tree, item);
    tree
}

#[requires(error_index < usize::MAX)]
#[ensures(ret.as_ref().is_none_or(|item| item.recovery_error_index() == Some(error_index)))]
fn all_tokens_recovery_item(error_index: usize, tokens: &[Token]) -> Option<SyntaxRecoveryItem> {
    let skipped = Vec1::try_from_vec(tokens.to_vec()).ok()?;
    Some(new!(SyntaxRecoveryItem::SkippedTokens {
        error_index,
        tokens: skipped,
    }))
}

#[requires(error_index < usize::MAX)]
#[ensures(true)]
fn fallback_recovery_item(
    tokens: &[Token],
    source: Option<&str>,
    error_index: usize,
    error: &SyntaxError,
) -> SyntaxRecoveryItem {
    let fail_token_index = token_index_for_byte_start(tokens, syntax_error_start(error));
    let directive = RecoveryDirective::new(
        "text",
        0,
        fail_token_index,
        fail_token_index,
        usize::MAX,
        error_index,
        error.clone(),
    );
    missing_recovery_item(error_index, tokens, source, &directive)
}

#[requires(effective_fail_token_index <= directive.fail_token_index)]
#[requires(directive.fail_token_index < directive.resume_token_index || effective_fail_token_index == directive.fail_token_index)]
#[ensures(true)]
fn skipped_recovery_item(
    error_index: usize,
    tokens: &[Token],
    directive: &RecoveryDirective,
    effective_fail_token_index: usize,
) -> Option<SyntaxRecoveryItem> {
    let start = effective_fail_token_index.min(tokens.len());
    let end = directive.resume_token_index.min(tokens.len());
    if start >= end {
        return None;
    }
    let skipped = Vec1::try_from_vec(tokens[start..end].to_vec()).ok()?;
    Some(new!(SyntaxRecoveryItem::SkippedTokens {
        error_index,
        tokens: skipped,
    }))
}

#[requires(true)]
#[ensures(true)]
fn missing_recovery_item(
    error_index: usize,
    tokens: &[Token],
    source: Option<&str>,
    directive: &RecoveryDirective,
) -> SyntaxRecoveryItem {
    let byte = recovery_byte_at(tokens, directive.fail_token_index);
    let span = source
        .and_then(|source| source_span_from_byte_offsets(None, source, byte, byte).ok())
        .unwrap_or_else(|| {
            SourceSpan::new(None, byte, byte, byte, byte)
                .expect("zero-width byte and char span is valid")
        });
    new!(SyntaxRecoveryItem::MissingRequiredField {
        error_index,
        span: Arc::new(span),
        expected: directive.rule.to_owned(),
    })
}

#[requires(true)]
#[ensures(true)]
fn recovery_byte_at(tokens: &[Token], index: usize) -> usize {
    tokens
        .get(index)
        .and_then(|token| token.core_word().byte_range().map(|range| range.start))
        .or_else(|| {
            tokens
                .last()
                .and_then(|token| token.core_word().byte_range().map(|range| range.end))
        })
        .unwrap_or(0)
}

#[requires(true)]
#[ensures(true)]
fn empty_recovered_text() -> generated::generated_model::recovered::TextSyntax {
    generated::generated_model::recovered::TextSyntax::RegularText(
        generated::generated_model::recovered::Recovered::valid(
            generated::generated_model::recovered::RegularTextSyntax {
                leading_nai: Vec::new(),
                leading_cmevla: Vec::new(),
                leading_indicators: Vec::new(),
                leading_free_modifiers: Vec::new(),
                leading_connective: None,
                leading_i_statements: Vec::new(),
                paragraphs: None,
            },
        ),
    )
}

#[requires(true)]
#[ensures(true)]
fn insert_leading_recovery_item(
    tree: &mut generated::generated_model::recovered::TextSyntax,
    item: SyntaxRecoveryItem,
) {
    if regular_text_mut(tree).is_none() {
        *tree = empty_recovered_text();
    }
    let Some(regular_text) = regular_text_mut(tree) else {
        return;
    };
    regular_text
        .leading_nai
        .push(generated::generated_model::recovered::Recovered::error(
            item,
        ));
}

#[requires(true)]
#[ensures(true)]
fn regular_text_mut(
    tree: &mut generated::generated_model::recovered::TextSyntax,
) -> Option<&mut generated::generated_model::recovered::RegularTextSyntax> {
    match tree {
        generated::generated_model::recovered::TextSyntax::RegularText(regular_text) => {
            recovered_value_mut(regular_text)
        }
        generated::generated_model::recovered::TextSyntax::ExplicitXauhaLohoiText(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_value_mut<T>(
    value: &mut generated::generated_model::recovered::Recovered<T>,
) -> Option<&mut T> {
    match value {
        generated::generated_model::recovered::Recovered::Valid(value) => Some(value.as_mut()),
        generated::generated_model::recovered::Recovered::Prefix(prefix) => {
            Some(prefix.value.as_mut())
        }
        generated::generated_model::recovered::Recovered::Error(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn add_generated_construct_warnings(
    text: &generated::generated_model::TextSyntax,
    tokens: &[Token],
    warnings: &mut Vec<SyntaxWarning>,
) {
    let mut visitor = new!(GeneratedConstructWarningVisitor {
        tokens,
        warnings: RefCell::new(warnings),
    });
    generated::generated_model::TreeNode::visit_in_order(text, &mut visitor);
}

#[invariant(
    tokens
        .iter()
        .all(|token| token.core_word().byte_range().is_some()),
    "generated warning anchors require source-backed syntax tokens"
)]
struct GeneratedConstructWarningVisitor<'a> {
    tokens: &'a [Token],
    warnings: RefCell<&'a mut Vec<SyntaxWarning>>,
}

impl GeneratedConstructWarningVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn warn_first_token<T>(&mut self, construct: ExperimentalConstruct, node: &T)
    where
        T: generated::generated_model::TreeNode,
    {
        let mut visitor = new!(FirstTokenVisitor {
            token: Cell::new(None),
        });
        node.visit_in_order(&mut visitor);
        if let Some(anchor) = visitor.token.get() {
            let mut warnings = self.warnings.borrow_mut();
            push_generated_construct_warning(&mut warnings, self.tokens, construct, anchor);
        }
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedConstructWarningVisitor<'_> {
    type Node = generated::generated_model::NodeRef<'tree>;
    type Atom = generated::generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        match node {
            generated::generated_model::NodeRef::FragmentStatementSyntaxZantufaMeksoFragment(
                fragment,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, fragment),
            generated::generated_model::NodeRef::QuantifierSyntaxZantufaRawMeksoQuantifier(
                quantifier,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, quantifier),
            generated::generated_model::NodeRef::QuantifierSyntaxZantufaPriorityRawMeksoQuantifier(
                quantifier,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, quantifier),
            _ => {}
        }
    }
}

#[invariant(
    token
        .get()
        .is_none_or(|token| token.core_word().byte_range().is_some()),
    "captured warning anchor token must be source-backed"
)]
struct FirstTokenVisitor<'tree> {
    token: Cell<Option<&'tree Token>>,
}

impl<'tree> TreeVisitor<'tree> for FirstTokenVisitor<'tree> {
    type Node = generated::generated_model::NodeRef<'tree>;
    type Atom = generated::generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        if self.token.get().is_some() {
            return;
        }
        let generated::generated_model::AtomRef::Token(token) = atom;
        self.token.set(Some(token));
    }
}

#[requires(true)]
#[ensures(warnings.len() == old(warnings.len()) || warnings.len() == old(warnings.len()) + 1)]
fn push_generated_construct_warning(
    warnings: &mut Vec<SyntaxWarning>,
    tokens: &[Token],
    construct: ExperimentalConstruct,
    anchor: &Token,
) {
    let anchor_index = generated_warning_anchor_index(tokens, anchor);
    let warning = SyntaxWarning::experimental_construct(
        construct,
        anchor_index,
        Token::bare(anchor.core_word().clone()),
    );
    if !warnings.contains(&warning) {
        warnings.push(warning);
    }
}

#[requires(true)]
#[ensures(ret <= tokens.len())]
fn generated_warning_anchor_index(tokens: &[Token], anchor: &Token) -> usize {
    let anchor_range = anchor.core_word().byte_range();
    tokens
        .iter()
        .position(|token| token.core_word().byte_range() == anchor_range)
        .unwrap_or(tokens.len())
}

#[requires(true)]
#[ensures(true)]
fn syntax_tokens(words: &[WordLike], options: &ParseOptions) -> Vec<Token> {
    attach_indicators(
        attach_bahe(words.iter().cloned().map(Token::bare).collect()),
        options
            .dialect
            .features
            .contains(&DialectFeature::ZantufaTerms),
    )
}

#[requires(true)]
#[ensures(true)]
fn attach_bahe(words: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::with_capacity(words.len());
    let mut pending_bahe = Vec::new();
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if iter.peek().is_some()
            && is_bahe_word(&word)
            && let Some(bahe) = modifier_word(&word).cloned()
        {
            pending_bahe.push(bahe);
            continue;
        }

        let mut word = word;
        while let Some(bahe) = pending_bahe.pop() {
            word = word.with_prepended_bahe(bahe);
        }
        out.push(word);
    }
    debug_assert!(pending_bahe.is_empty());
    out
}

#[requires(true)]
#[ensures(true)]
fn is_bahe_word(word: &Token) -> bool {
    modifier_word(word).is_some_and(|word| word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe]))
}

#[requires(true)]
#[ensures(true)]
fn attach_indicators(words: Vec<Token>, preserve_zantufa_iau: bool) -> Vec<Token> {
    let mut out = Vec::with_capacity(words.len());
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if modifier_word(&word).is_some_and(is_indicator_word) {
            let indicator = modifier_word_with_bahe(&word);
            let nai = if iter
                .peek()
                .and_then(modifier_word)
                .is_some_and(|next| next.is_cmavo(Cmavo::Nai))
            {
                iter.next().and_then(|next| modifier_word_with_bahe(&next))
            } else {
                None
            };
            if let (Some(prev), Some((indicator_bahe, indicator))) = (out.pop(), indicator) {
                let prev_is_leading_indicator_nai = modifier_word(&prev)
                    .is_some_and(|word| word.is_cmavo(Cmavo::Nai))
                    && out
                        .last()
                        .and_then(modifier_word)
                        .is_some_and(is_indicator_word);
                if prev_is_leading_indicator_nai
                    || !should_attach_indicator(&prev, &indicator, preserve_zantufa_iau)
                {
                    out.push(prev);
                    out.push(word);
                    if let Some((nai_bahe, nai)) = nai {
                        out.push(token_from_modifier_parts(nai_bahe, nai));
                    }
                } else {
                    let (nai_bahe, nai) = nai
                        .map(|(bahe, word)| (bahe, Some(word)))
                        .unwrap_or((Vec::new(), None));
                    out.push(Token::with_indicator_with_modifiers(
                        prev,
                        indicator_bahe,
                        indicator,
                        nai_bahe,
                        nai,
                    ));
                }
            } else {
                out.push(word);
                if let Some((nai_bahe, nai)) = nai {
                    out.push(token_from_modifier_parts(nai_bahe, nai));
                }
            }
        } else {
            out.push(word);
        }
    }
    out
}

#[requires(true)]
#[ensures(true)]
fn modifier_word(word: &Token) -> Option<&Word> {
    word.core_word().bare_word()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(bahe, _)| bahe.iter().all(|word| word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe]))))]
fn modifier_word_with_bahe(word: &Token) -> Option<(Vec<Word>, Word)> {
    match word.as_indicators().as_data() {
        data!(WithIndicators::Plain(word_like)) => word_like
            .bare_word()
            .cloned()
            .map(|word| (Vec::new(), word)),
        data!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }) => word_like.bare_word().cloned().map(|word| {
            let mut bahes = Vec::with_capacity(extra_bahe.len() + 1);
            bahes.push(bahe.clone());
            bahes.extend(extra_bahe.iter().cloned());
            (bahes, word)
        }),
        data!(WithIndicators::WithIndicator { .. }) => {
            modifier_word(word).cloned().map(|word| (Vec::new(), word))
        }
    }
}

#[requires(bahe.iter().all(|word| word.is_one_of_cmavo(&[Cmavo::Bahe, Cmavo::Zahe])))]
#[ensures(true)]
fn token_from_modifier_parts(mut bahe: Vec<Word>, word: Word) -> Token {
    if bahe.is_empty() {
        Token::bare(WordLike::bare(word))
    } else {
        let first_bahe = bahe.remove(0);
        Token::from_indicators(WithIndicators::emphasized_with_extra_bahe(
            first_bahe,
            bahe,
            WordLike::bare(word),
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    word.cmavo().is_some_and(|cmavo| {
        cmavo.is_selmaho(Selmaho::Ui) || cmavo.is_selmaho(Selmaho::Cai) || cmavo == Cmavo::Y
    })
}

#[requires(true)]
#[ensures(true)]
fn should_attach_indicator(prev: &Token, indicator: &Word, preserve_zantufa_iau: bool) -> bool {
    if preserve_zantufa_iau && indicator.is_cmavo(Cmavo::Ihau) {
        return false;
    }
    !(indicator.is_selmaho(Selmaho::Roi)
        && modifier_word(prev).is_some_and(|prev| prev.is_selmaho(Selmaho::Pa)))
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, ensures, new, requires};
    use jbotci_dialect::parse_dialect_definition;
    use jbotci_morphology::{WordLikeData, segment_words_with_modifiers};
    use jbotci_tree::RecoveredFieldState;
    use std::{fmt::Write as _, fs, path::Path};
    use vec1::Vec1;

    use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData, WithFreeModifiers};

    use super::*;

    const RECOVERY_ANCHOR_SNAPSHOT_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/recovery-anchor-metadata.snapshot.txt"
    );

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_basic_predicate_with_leading_and_tail_terms() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("do mamta mi").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:?}", parsed.parse_tree).contains("Paragraph"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_model_strict_parser_parses_basic_text() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi klama").expect("valid morphology");
            let tokens = syntax_tokens(&words, &ParseOptions::default());

            let parsed = generated::generated_model::parse_text(&tokens, &ParseOptions::default())
                .expect("valid generated-model syntax");
            let mut visitor = GeneratedModelNoopVisitor;
            generated::generated_model::TreeNode::visit_in_order(&parsed, &mut visitor);

            let generated::generated_model::TextSyntax::RegularText(regular_text) = parsed else {
                panic!("basic text should parse as regular generated-model text");
            };
            assert!(regular_text.paragraphs.is_some());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_model_strict_parser_keeps_leading_i_statement_marker() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("i mi klama").expect("valid morphology");
            let tokens = syntax_tokens(&words, &ParseOptions::default());

            let parsed = generated::generated_model::parse_text(&tokens, &ParseOptions::default())
                .expect("valid generated-model syntax");
            let generated::generated_model::TextSyntax::RegularText(regular_text) = parsed else {
                panic!("basic text should parse as regular generated-model text");
            };

            assert_eq!(regular_text.leading_i_statements.len(), 1);
            assert!(regular_text.paragraphs.is_some());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_generated_model_round_trips_representative_valid_sources() {
        run_on_normal_stack(|| {
            for source in [
                "mi klama",
                "i mi klama",
                "do mamta mi",
                "lo gerku cu batci lo nanmu",
            ] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                let parsed =
                    parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
                let valid = parsed.parse_tree.as_ref().clone();
                let recovered =
                    generated::generated_model::recovered::TextSyntax::from_valid(valid.clone());

                assert_eq!(recovered.recovery_error_slots(), 0);
                assert_eq!(recovered.clone().try_into_valid(), Ok(valid.clone()));

                let mut valid_spans = Vec::new();
                valid.visit_source_spans(&mut |span| valid_spans.push(span.clone()));
                let mut recovered_spans = Vec::new();
                recovered.visit_source_spans(&mut |span| recovered_spans.push(span.clone()));
                assert_eq!(recovered_spans, valid_spans);

                let json = serde_json::to_string(&recovered)
                    .expect("recovered valid tree should serialize");
                let decoded: generated::generated_model::recovered::TextSyntax =
                    serde_json::from_str(&json).expect("recovered valid tree should deserialize");
                assert_eq!(decoded, recovered);
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_generated_model_error_slots_block_strict_conversion() {
        run_on_normal_stack(|| {
            let (tree, skipped_item, _missing_item, _missing_span) =
                recovered_text_with_skipped_and_missing_slots();

            assert_eq!(tree.recovery_error_slots(), 2);
            assert_eq!(tree.invalid_error_slots(), 1);
            assert_eq!(tree.missing_error_slots(), 1);
            let error = tree
                .try_into_valid()
                .expect_err("error slots must prevent strict conversion");
            assert_eq!(error.item, skipped_item);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_generated_model_visits_recovery_spans_in_tree_order() {
        run_on_normal_stack(|| {
            let (tree, _skipped_item, _missing_item, missing_span) =
                recovered_text_with_skipped_and_missing_slots();

            let mut spans = Vec::new();
            tree.visit_source_spans(&mut |span| spans.push(span.clone()));

            assert_eq!(spans.len(), 2);
            assert_eq!((spans[0].byte_start, spans[0].byte_end), (0, 2));
            assert_eq!(spans[1], missing_span);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_generated_model_error_slots_serde_round_trip() {
        run_on_normal_stack(|| {
            let (tree, _skipped_item, _missing_item, _missing_span) =
                recovered_text_with_skipped_and_missing_slots();

            let json =
                serde_json::to_string(&tree).expect("recovered tree with errors should serialize");
            let decoded: generated::generated_model::recovered::TextSyntax =
                serde_json::from_str(&json).expect("recovered tree with errors should deserialize");

            assert_eq!(decoded, tree);
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn recovered_text_with_skipped_and_missing_slots() -> (
        generated::generated_model::recovered::TextSyntax,
        SyntaxRecoveryItem,
        SyntaxRecoveryItem,
        jbotci_source::SourceSpan,
    ) {
        let source = "mi";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let skipped_token = tokens[0].clone();
        let skipped_item = new!(SyntaxRecoveryItem::SkippedTokens {
            error_index: 0,
            tokens: Vec1::try_from_vec(vec![skipped_token]).expect("one skipped token"),
        });
        let missing_span = jbotci_diagnostics::source_span_from_byte_offsets(None, source, 2, 2)
            .expect("valid zero-width source span");
        let missing_item = new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index: 1,
            span: Arc::new(missing_span.clone()),
            expected: "paragraphs".to_owned(),
        });

        let skipped_slot =
            generated::generated_model::recovered::Recovered::error(skipped_item.clone());
        let missing_paragraphs =
            generated::generated_model::recovered::Recovered::error(missing_item.clone());
        let tree = generated::generated_model::recovered::TextSyntax::RegularText(
            generated::generated_model::recovered::Recovered::valid(
                generated::generated_model::recovered::RegularTextSyntax {
                    leading_nai: vec![skipped_slot],
                    leading_cmevla: Vec::new(),
                    leading_indicators: Vec::new(),
                    leading_free_modifiers: Vec::new(),
                    leading_connective: None,
                    leading_i_statements: Vec::new(),
                    paragraphs: Some(Arc::new(missing_paragraphs)),
                },
            ),
        );
        (tree, skipped_item, missing_item, missing_span)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_model_reports_farthest_soft_failure_before_eof() {
        run_on_fixture_worker_stack(|| {
            let source = "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti lo ko ce'u xendo ije cnikansa ro lo jmive ta'i lo racli";
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let tokens = syntax_tokens(&words, &ParseOptions::default());

            let error = generated::generated_model::parse_text(&tokens, &ParseOptions::default())
                .expect_err("syntax should reject the malformed description tail");
            let SyntaxError::Parse {
                byte_start,
                byte_end,
                expected,
                contexts,
                ..
            } = error
            else {
                panic!("expected syntax parse error");
            };

            assert_eq!(byte_start, 68);
            assert_eq!(byte_end, 72);
            assert!(!expected.iter().any(|item| item == "end of input"));
            assert_eq!(
                contexts.first().map(|context| context.construct.as_str()),
                Some("description tail")
            );
        });
    }

    #[invariant(true)]
    struct GeneratedModelNoopVisitor;

    impl<'tree> jbotci_tree::TreeVisitor<'tree> for GeneratedModelNoopVisitor {
        type Node = generated::generated_model::NodeRef<'tree>;
        type Atom = generated::generated_model::AtomRef<'tree>;
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn recovery_anchor_metadata_snapshot() -> String {
        let mut snapshot = String::new();
        writeln!(
            &mut snapshot,
            "rules: {}",
            generated::generated_model::SYNTAX_GRAMMAR_RECOVERY_ANCHORS.len()
        )
        .expect("writing to string cannot fail");
        for metadata in generated::generated_model::SYNTAX_GRAMMAR_RECOVERY_ANCHORS {
            writeln!(&mut snapshot, "rule {}", metadata.rule)
                .expect("writing to string cannot fail");
            for first in metadata.first {
                writeln!(
                    &mut snapshot,
                    "  first {} conditions {}",
                    format_anchor_tokens(first.tokens),
                    format_anchor_conditions(first.conditions),
                )
                .expect("writing to string cannot fail");
            }
            for field in metadata.fields {
                writeln!(
                    &mut snapshot,
                    "  field {} {}",
                    field.field_index, field.field_name
                )
                .expect("writing to string cannot fail");
                for anchor in field.anchors {
                    writeln!(
                        &mut snapshot,
                        "    resume {} origin {:?} start {} conditions {}",
                        anchor.resume_field,
                        anchor.origin,
                        format_anchor_tokens(anchor.start_tokens),
                        format_anchor_conditions(anchor.conditions),
                    )
                    .expect("writing to string cannot fail");
                }
            }
        }
        writeln!(&mut snapshot, "subtext-containers").expect("writing to string cannot fail");
        for container in generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS {
            writeln!(
                &mut snapshot,
                "  {} opener {} {} text {} closer {} {}",
                container.rule,
                container.opener_field,
                format_anchor_tokens(container.opener_tokens),
                container.text_field,
                container.closer_field,
                format_anchor_tokens(container.closer_tokens),
            )
            .expect("writing to string cannot fail");
        }
        snapshot
    }

    #[requires(true)]
    #[ensures(ret.starts_with('['))]
    fn format_anchor_tokens(
        tokens: &[generated::generated_model::SyntaxGrammarAnchorToken],
    ) -> String {
        let mut rendered = String::new();
        rendered.push('[');
        for (index, token) in tokens.iter().enumerate() {
            if index > 0 {
                rendered.push_str(", ");
            }
            match token {
                generated::generated_model::SyntaxGrammarAnchorToken::Cmavo(cmavo) => {
                    write!(&mut rendered, "Cmavo({cmavo:?})")
                        .expect("writing to string cannot fail");
                }
                generated::generated_model::SyntaxGrammarAnchorToken::Selmaho(selmaho) => {
                    write!(&mut rendered, "Selmaho({selmaho:?})")
                        .expect("writing to string cannot fail");
                }
            }
        }
        rendered.push(']');
        rendered
    }

    #[requires(true)]
    #[ensures(ret.starts_with('['))]
    fn format_anchor_conditions(
        conditions: &[generated::generated_model::SyntaxGrammarCondition],
    ) -> String {
        let mut rendered = String::new();
        rendered.push('[');
        for (index, condition) in conditions.iter().enumerate() {
            if index > 0 {
                rendered.push_str(", ");
            }
            write!(&mut rendered, "{:?}({})", condition.kind, condition.name)
                .expect("writing to string cannot fail");
        }
        rendered.push(']');
        rendered
    }

    #[requires(!rule.is_empty())]
    #[ensures(ret.rule == rule)]
    fn generated_anchor_metadata(
        rule: &str,
    ) -> &'static generated::generated_model::SyntaxGrammarRuleAnchorMetadata {
        generated::generated_model::syntax_grammar_anchor_metadata_by_rule_name(rule)
            .expect("generated anchor metadata exists")
    }

    #[requires(true)]
    #[ensures(true)]
    fn anchor_tokens_contain(
        tokens: &[generated::generated_model::SyntaxGrammarAnchorToken],
        token: generated::generated_model::SyntaxGrammarAnchorToken,
    ) -> bool {
        tokens.contains(&token)
    }

    #[requires(!rule.is_empty())]
    #[requires(!field.is_empty())]
    #[ensures(true)]
    fn assert_field_anchor_contains(
        rule: &str,
        field: &str,
        token: generated::generated_model::SyntaxGrammarAnchorToken,
    ) {
        let metadata = generated_anchor_metadata(rule);
        let field_metadata = metadata
            .fields
            .iter()
            .find(|field_metadata| field_metadata.field_name == field)
            .expect("field metadata exists");
        assert!(
            field_metadata
                .anchors
                .iter()
                .any(|anchor| anchor_tokens_contain(anchor.start_tokens, token)),
            "{rule}.{field} does not contain anchor token {token:?}",
        );
    }

    #[requires(!rule.is_empty())]
    #[requires(!field.is_empty())]
    #[ensures(true)]
    fn assert_field_anchor_contains_origin(
        rule: &str,
        field: &str,
        token: generated::generated_model::SyntaxGrammarAnchorToken,
        origin: generated::generated_model::SyntaxGrammarAnchorOrigin,
    ) {
        let metadata = generated_anchor_metadata(rule);
        let field_metadata = metadata
            .fields
            .iter()
            .find(|field_metadata| field_metadata.field_name == field)
            .expect("field metadata exists");
        assert!(
            field_metadata.anchors.iter().any(|anchor| {
                anchor.origin == origin && anchor_tokens_contain(anchor.start_tokens, token)
            }),
            "{rule}.{field} does not contain {origin:?} anchor token {token:?}",
        );
    }

    #[requires(!rule.is_empty())]
    #[requires(!condition.is_empty())]
    #[ensures(true)]
    fn assert_first_contains_condition(rule: &str, condition: &str) {
        let metadata = generated_anchor_metadata(rule);
        assert!(
            metadata.first.iter().any(|entry| entry
                .conditions
                .iter()
                .any(|entry_condition| entry_condition.name == condition)),
            "{rule} has no FIRST entry conditioned on {condition}",
        );
    }

    #[requires(!rule.is_empty())]
    #[ensures(ret.rule == rule)]
    fn subtext_container(
        rule: &str,
    ) -> &'static generated::generated_model::SyntaxGrammarSubtextContainer {
        generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS
            .iter()
            .find(|container| container.rule == rule)
            .expect("subtext container exists")
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    fn assert_subtext_container(
        rule: &str,
        opener_field: usize,
        opener: generated::generated_model::SyntaxGrammarAnchorToken,
        text_field: usize,
        closer_field: usize,
        closer: generated::generated_model::SyntaxGrammarAnchorToken,
    ) {
        let container = subtext_container(rule);
        assert_eq!(container.opener_field, opener_field);
        assert_eq!(container.text_field, text_field);
        assert_eq!(container.closer_field, closer_field);
        assert!(anchor_tokens_contain(container.opener_tokens, opener));
        assert!(anchor_tokens_contain(container.closer_tokens, closer));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_recovery_anchor_metadata_snapshot_matches() {
        let snapshot = recovery_anchor_metadata_snapshot();
        let path = Path::new(RECOVERY_ANCHOR_SNAPSHOT_PATH);
        if std::env::var_os("JBOTCI_UPDATE_RECOVERY_ANCHOR_SNAPSHOT").is_some() {
            fs::write(path, &snapshot).expect("snapshot can be updated");
        }
        let expected = fs::read_to_string(path).expect("checked-in recovery anchor snapshot");
        assert_eq!(snapshot, expected);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_recovery_anchor_metadata_spot_checks() {
        use generated::generated_model::SyntaxGrammarAnchorOrigin::{
            FieldFirst, LiteralRun, RepetitionElementFirst,
        };
        use generated::generated_model::SyntaxGrammarAnchorToken::{
            Cmavo as AnchorCmavo, Selmaho as AnchorSelmaho,
        };

        assert_eq!(
            generated::generated_model::SYNTAX_GRAMMAR_RECOVERY_ANCHORS.len(),
            generated::generated_model::SYNTAX_GRAMMAR_RULES.len()
        );

        assert_field_anchor_contains(
            "descriptor_with_gadri_sumti",
            "tail",
            AnchorCmavo(Cmavo::Ku),
        );
        assert_field_anchor_contains(
            "abstraction_tanru_unit",
            "subbridi",
            AnchorCmavo(Cmavo::Kei),
        );
        assert_field_anchor_contains(
            "restrictive_bridi_relative_clause",
            "subbridi",
            AnchorCmavo(Cmavo::Kuho),
        );
        assert_field_anchor_contains("selbri_simple_bridi_tail", "terms", AnchorCmavo(Cmavo::Vau));
        assert_field_anchor_contains(
            "paragraph_statement_sequence",
            "following",
            AnchorCmavo(Cmavo::I),
        );
        assert_field_anchor_contains(
            "text_paragraph_with_additional_niho",
            "additional_niho",
            AnchorSelmaho(Selmaho::Niho),
        );
        assert_field_anchor_contains_origin(
            "descriptor_with_gadri_sumti",
            "tail",
            AnchorCmavo(Cmavo::Ku),
            LiteralRun,
        );
        assert_field_anchor_contains_origin(
            "descriptor_with_gadri_sumti",
            "tail",
            AnchorCmavo(Cmavo::Noi),
            FieldFirst,
        );
        assert_field_anchor_contains_origin(
            "paragraph_statement_sequence",
            "following",
            AnchorCmavo(Cmavo::I),
            RepetitionElementFirst,
        );

        assert_subtext_container(
            "text_quote",
            0,
            AnchorCmavo(Cmavo::Lu),
            1,
            2,
            AnchorCmavo(Cmavo::Lihu),
        );
        assert_subtext_container(
            "parenthetical_text",
            0,
            AnchorSelmaho(Selmaho::To),
            1,
            2,
            AnchorCmavo(Cmavo::Toi),
        );
        assert_subtext_container(
            "text_group_statement",
            1,
            AnchorCmavo(Cmavo::Tuhe),
            2,
            3,
            AnchorCmavo(Cmavo::Tuhu),
        );

        assert_first_contains_condition("statement_base", "ZantufaConnectives");
        for rule in ["text", "statement", "sumti", "selbri"] {
            assert!(
                !generated_anchor_metadata(rule).first.is_empty(),
                "{rule} should have non-empty FIRST metadata",
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_v1_anchor_origin_filter_ignores_field_first() {
        use generated::generated_model::SyntaxGrammarAnchorOrigin::{
            FieldFirst, LiteralRun, RepetitionElementFirst,
        };

        assert!(recovery_anchor_origin_is_v1(LiteralRun));
        assert!(recovery_anchor_origin_is_v1(RepetitionElementFirst));
        assert!(!recovery_anchor_origin_is_v1(FieldFirst));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_anchor_conditions_respect_dialect_features() {
        use generated::generated_model::{SyntaxGrammarCondition, SyntaxGrammarConditionKind};

        let zantufa_terms = [SyntaxGrammarCondition {
            kind: SyntaxGrammarConditionKind::Feature,
            name: "ZantufaTerms",
        }];
        let baseline_env =
            generated_runtime::SyntaxGrammarEnv::from_options(&ParseOptions::default());
        let zantufa = parse_dialect_definition("(zantufa)").expect("zantufa dialect parses");
        let zantufa_env = generated_runtime::SyntaxGrammarEnv::from_options(
            &ParseOptions::default().with_dialect_definition(&zantufa),
        );

        assert!(!recovery_conditions_match(&zantufa_terms, baseline_env));
        assert!(recovery_conditions_match(&zantufa_terms, zantufa_env));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_stray_cu() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("cu").expect("valid morphology");

            let error = parse_syntax_tree(&words, &ParseOptions::default()).expect_err("invalid");

            assert!(matches!(error, SyntaxError::Parse { .. }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_grouped_math_operator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("li re ke su'i ke'e ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("GroupedMeksoOperator"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_bo_connected_math_operator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("li re su'i je bo vu'u ci du li mu")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");

            assert!(format!("{:#?}", parsed.parse_tree).contains("Bo"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_pehe_termset_with_cehe_connectives_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers(
                "mi klama le zarci ce'e le briju pe'e je le zdani ce'e le ckule",
            )
            .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("PeheTermsetConnection"));
            assert!(raw.contains("PeheTermsetConnectionContinuation"));
            assert!(raw.contains("pe'e"));
            assert!(raw.contains("je"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_emphasized_goha_relation_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("le lojbo cu ba'e du le loglo")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("Emphasized"));
            assert!(raw.contains("du"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_statement_connective_with_flattened_fiho_relation_under_contracts() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("i fi'o ke broda brode bo mi klama")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("ITagBoParagraphStatementConnective"));
            assert!(raw.contains("FihoTense"));
            assert!(raw.contains("GroupedTanruUnit"));
            assert!(raw.contains("fi'o"));
            assert!(raw.contains("bróda"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_fiho_modal_with_full_linked_selbri() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi tavla fi'o tavla be do fe'u do")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("FihoTense"));
            assert!(raw.contains("LinkedTanruUnit"));
            assert!(raw.contains("Linkargs"));
            assert!(raw.contains("be"));
            assert!(raw.contains("fe'u"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_connected_fiho_tags_as_one_tagged_term() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers(".e'a casnu fi'o selsnu ja fi'o bangu la lojban")
                    .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
            let raw = format!("{:?}", parsed.parse_tree);

            assert!(raw.contains("TaggedSumtiTerm"));
            assert!(raw.contains("ConnectedTenseModal"));
            assert!(raw.contains("ConnectedTenseModalContinuation"));
            assert!(raw.contains("sél"));
            assert!(raw.contains("snu"));
            assert!(raw.contains("bángu"));
            assert!(!raw.contains("TermConnection"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_i_connectives_out_of_tail_terms() {
        run_on_normal_stack(|| {
            let raw = parse_tree_debug("mi ca pilno .ije ca'o nelci", &ParseOptions::default());

            assert!(raw.contains("StatementConnection"));
            assert!(raw.contains("leading_statement"));
            assert!(raw.contains("trailing_statement"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classifies_mohi_as_spatial_movement_not_koha() {
        run_on_normal_stack(|| {
            let raw = parse_tree_debug(
                "le verba mo'i ri'u cadzu le bisli",
                &ParseOptions::default(),
            );

            assert!(raw.contains("TaggedSelbri"));
            assert!(raw.contains("mo'i"));
            assert!(!raw.contains("ProSumti(WithFreeModifiers { value: Plain(PlainWord(Cmavo { phonemes: Phonemes { text: \"mo'i\" }"));

            let words = segment_words_with_modifiers("da poi palci vimo'i selklama")
                .expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_v0_joik_and_cehe_argument_connective_cases() {
        run_on_normal_stack(|| {
            for source in [
                "la djeimyz. cebo la djordj. bruna remei",
                "mi joibo do cu broda",
                "ju'a nai cy pa ka ce'u ce ke do ke'e simxu cy no kei",
                "ce'e di",
            ] {
                parse_source(source, &ParseOptions::default());
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_nested_descriptor_tail_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = "mi pensi ledu'u mi ba stidi fi la nitcion. fe le pu selsnu be mi joi do poi ckini lei bifce poi pu xabju le mi zdani kei";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_modal_abstraction_tail_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = ".ino'iji'a pa makcu nixli cu pleji fi mi lenu kelci ki'u lenu te cusku fe lesedu'u mi xamgu to malglico toi kelci";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_grouped_argument_recursion_on_fixture_worker_stack() {
        run_on_fixture_worker_stack(|| {
            let source = concat!(
                " i abu zi ba le nu facki le du'u makau drani tadji le nu kurji cy ",
                "to no'u le nu tongau cy ja'e lo jgena gi'e tagji jgari le cy pritu ",
                "kerlo ku joi le cy zunle jamfu ja'e le nu rivbi le nu cy sezytolplo ",
                "toi cu bevri cy le bartu vacri i lu lei du romu'ei le du'u mi na ",
                "lebna le vi cifnu sei la alis pensi cu ba catra cy za lo djedi be ",
                "li ji'ire i xu na zekri fa le nu cliva cy li'u i abu cladu cusku ",
                "lei romoi valsi i le cmalu cu spuda cmoni to cy ca ba'o senci toi ",
                "i lu ko na cmoni sei la alis cusku i nasai drani tadji le nu cusku li'u ",
            );
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            parse_syntax_tree(&words, &ParseOptions::default()).expect("valid syntax");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_vowel_cmavo_are_not_implicit_letters() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("a cmene").expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let raw = parse_tree_debug("a bu cmene", &ParseOptions::default());
            assert!(raw.contains("LerfuWord"));

            let raw = parse_tree_debug("abu cmene", &ParseOptions::default());
            assert!(raw.contains("LerfuWord"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn core_word_strips_syntax_wrappers_but_preserves_word_like_unit() {
        run_on_normal_stack(|| {
            let mut words = segment_words_with_modifiers("zo coi").expect("valid morphology");
            let quote = words.remove(0);
            let wrapped: WithFreeModifiers<Token, generated::generated_model::FreeModifierSyntax> =
                WithFreeModifiers::new(
                    Token::with_indicator(
                        Token::emphasized(single_bare_word("ba'e"), quote.clone()),
                        single_bare_word("ui"),
                        None,
                    ),
                    Vec::new(),
                );

            assert_eq!(wrapped.core_word(), &quote);
            assert_eq!(wrapped.quote_marker_cmavo(), Some(Cmavo::Zo));
            assert!(!wrapped.is_cmavo(Cmavo::Zo));
            assert!(!wrapped.is_selmaho(Selmaho::Zo));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_warning_anchor_covers_whole_core_word_like() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi tavla zo'oi broda", &ParseOptions::default());
            let quote_warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalZohOiQuote)
                .expect("ZOhOI warning");

            assert_eq!(warning_span(quote_warning), [9, 20]);
            assert!(matches!(
                quote_warning.anchor.core_word().as_data(),
                data!(WordLike::DelimitedWordQuote { .. })
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mahoi_quote_warns_on_the_experimental_marker() {
        run_on_normal_stack(|| {
            let parsed = parse_source("ma'oi ba", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalCmavo)
                .expect("experimental MAhOI warning");

            assert_eq!(warning_span(warning), [0, 5]);
            assert!(warning.anchor.is_cmavo(Cmavo::Mahoi));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mehoi_quote_warning_is_distinct_from_selbri_unit_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi me'oi broda", &ParseOptions::default());

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalMehOiQuote
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalMehOiSelbriUnit
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_lu_quotes_do_not_warn_for_quoted_experimental_cmavo() {
        run_on_normal_stack(|| {
            for source in [
                "mi tavla zo li'oi",
                "mi tavla zo'oi li'oi",
                "mi tavla lo'u li'oi le'u",
            ] {
                let parsed = parse_source(source, &ParseOptions::default());
                assert!(
                    !has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalDictionaryUiIndicator
                    ),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lu_quote_warns_for_inner_experimental_cmavo() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi cusku lu li'oi li'u", &ParseOptions::default());
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalDictionaryUiIndicator
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_indicator_warning_anchors_indicator_word() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi li'oi klama", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalDictionaryUiIndicator
                })
                .expect("experimental UI warning");

            assert_eq!(warning.anchor_index, 0);
            assert_eq!(warning_span(warning), [3, 8]);
            assert!(warning.anchor.is_cmavo(Cmavo::Lihoi));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_noi_indicator_uses_noi_warning_context() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi klama no'oi bajra", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo)
                .expect("Zantufa NOI indicator warning");

            assert_eq!(warning.anchor_index, 1);
            assert_eq!(warning_span(warning), [9, 14]);
            assert!(warning.anchor.is_cmavo(Cmavo::Nohoi));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn koha_category_terminal_warns_for_experimental_cmavo() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi'ai klama", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalCmavo)
                .expect("experimental KOhA warning");

            assert_eq!(warning_span(warning), [0, 5]);
            assert!(warning.anchor.is_cmavo(Cmavo::Mihai));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn by_category_terminal_warns_for_experimental_cmavo() {
        run_on_normal_stack(|| {
            let parsed = parse_source("a'y cmene", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| warning.kind == ExperimentalConstruct::ExperimentalCmavo)
                .expect("experimental BY warning");

            assert_eq!(warning_span(warning), [0, 3]);
            assert!(warning.anchor.is_cmavo(Cmavo::Ahy));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_experimental_muhei_roi_tense_with_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi so'emu'ei spuda", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("Composite"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalCmavo
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_additive_zantufa_quote_relation_units_by_default() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("lu'ei mi klama li'au").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid zantufa quote syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaLuheiSelbriUnit
            }));

            let words =
                segment_words_with_modifiers("mi cu mu'oi gy foo gy").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid zantufa MUhOI syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaMuhoiSelbriUnit
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_jai_tag_terms() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("jai pu mi cu klama").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid zantufa JAI tag term");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaJaiTagTerm
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_additive_zantufa_poiha_brigahi_ku_by_default() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("noi'a klama ku mi cu broda")
                .expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid Zantufa POIhA briga'i");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_zantufa_cmavo_table_entries_with_warning() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi cu xe'u").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid Zantufa cmavo syntax");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_gohoi_markers_as_word_quotes() {
        run_on_normal_stack(|| {
            for marker in ["go'oi", "ze'oi", "ta'ai", "bo'ei"] {
                let source = format!("mi cu {marker} coi");
                let words = segment_words_with_modifiers(&source).expect("valid morphology");
                let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                    .expect("valid GOhOI word quote selbri");
                let debug_tree = format!("{:?}", parsed.parse_tree);

                assert!(debug_tree.contains("QuotedBridiSelbri"));
                assert!(parsed.warnings.iter().any(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalGohoiSelbriUnit
                }));
                assert!(!parsed.warnings.iter().any(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalZantufaCmavo
                }));
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_lohoi_bridi_descriptions() {
        run_on_normal_stack(|| {
            for lohoi in ["lo'oi", "xu'u", "xau'a", "mau'a"] {
                let source = format!("{lohoi} mi cu broda ku'au");
                let parsed = parse_source(&source, &ParseOptions::default());

                assert!(format!("{:?}", parsed.parse_tree).contains("BridiDescription"));
                assert!(has_warning_kind(
                    &parsed,
                    ExperimentalConstruct::ExperimentalLohOiBridiDescription
                ));
            }

            let ui_parse = parse_source("xau'a mi cu broda", &ParseOptions::default());
            assert!(!format!("{:?}", ui_parse.parse_tree).contains("BridiDescription"));
            assert!(!has_warning_kind(
                &ui_parse,
                ExperimentalConstruct::ExperimentalLohOiBridiDescription
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_rahoi_quote_warning() {
        run_on_normal_stack(|| {
            let parsed = parse_source("ra'oi broda cu brode", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("DelimitedWordQuote"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaRahoiQuote
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZohOiQuote
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_1_17_xoi_as_adverbial_term() {
        run_on_normal_stack(|| {
            let parsed = parse_source("xoi mi broda", &ParseOptions::default());

            assert!(format!("{:?}", parsed.parse_tree).contains("SoiAdverbialTerm"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalSoiAdverbial
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_xoi_and_fihoi_statement_payloads() {
        run_on_normal_stack(|| {
            for source in [
                "xoi mi broda i je do brode se'u",
                "fi'oi mi broda i je do brode fi'au",
            ] {
                let parsed = parse_source(source, &ParseOptions::default());
                assert!(
                    has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalSoiAdverbial)
                        || has_warning_kind(
                            &parsed,
                            ExperimentalConstruct::ExperimentalFihoiAdverbial
                        ),
                    "{source}"
                );
                assert!(
                    format!("{:?}", parsed.parse_tree).contains("IStatementConnection"),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_poiha_brigahi_with_free_modifiers() {
        run_on_normal_stack(|| {
            let parsed = parse_source(
                "noi'a to mi toi klama ku mi cu broda",
                &ParseOptions::default(),
            );

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi
            ));
            assert!(format!("{:?}", parsed.parse_tree).contains("free_modifiers"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_mex_forms() {
        run_on_normal_stack(|| {
            let dialect =
                parse_dialect_definition("(+ZANTUFA-MEX)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            for source in [
                "li mo'e broda lo'o",
                "li ma'o lo broda te'u pa lo'o",
                "li na'e pa lo'o",
                "li ke pa re ke'e lo'o",
            ] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                assert!(
                    parse_syntax_tree(&words, &ParseOptions::default()).is_err(),
                    "{source}"
                );

                let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa mex");
                assert!(
                    has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalZantufaMex),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_raw_mekso_quantifier_does_not_shadow_lerfu_sumti_sentence() {
        run_on_normal_stack(|| {
            let dialect =
                parse_dialect_definition("(case-insensitive zantufa)").expect("valid dialect");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let words =
                segment_words_with_modifiers("lo cukta poi my tcidu").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa syntax");
            let tree = format!("{:#?}", parsed.parse_tree);

            assert!(tree.contains("LerfuStringSumti"), "{tree}");
            assert!(
                !tree.contains("ZantufaPriorityRawMeksoQuantifier"),
                "{tree}"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_initial_gi_gek() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("gi je mi klama gi do klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GI GEK");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_gihi_forethought_terminator() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("ge mi klama gi do klama gi'i")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid Zantufa GIhI");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaForethoughtGihi
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_nary_forethought_bridi_branches() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("ge mi klama gi do klama gi ti klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed =
                parse_syntax_tree(&words, &options).expect("valid Zantufa n-ary bridi forethought");
            let debug_tree = format!("{:?}", parsed.parse_tree);

            assert!(debug_tree.contains("additional_branches"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaNaryForethought
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_nary_forethought_bridi_branch_count_grid() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            for (source, extra_branch_count) in [
                ("ge mi klama gi do klama", 0),
                ("ge mi klama gi do klama gi ti klama", 1),
                ("ge mi klama gi do klama gi ti klama gi ta klama", 2),
                (
                    "ge mi klama gi do klama gi ti klama gi ta klama gi zo'e klama",
                    3,
                ),
            ] {
                let parsed = parse_source(source, &options);
                assert_eq!(
                    parsed
                        .warnings
                        .iter()
                        .filter(|warning| {
                            warning.kind
                                == ExperimentalConstruct::ExperimentalZantufaNaryForethought
                        })
                        .count(),
                    extra_branch_count,
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_nary_forethought_bridi_with_gihi() {
        run_on_normal_stack(|| {
            let source = "ge mi klama gi do klama gi ti klama gi'i";
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_source(source, &options);

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaNaryForethought
            ));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaForethoughtGihi
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_nary_forethought_termset_branches() {
        run_on_normal_stack(|| {
            let source = "nu'i ge mi gi do gi ti";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_source(source, &options);
            let debug_tree = format!("{:?}", parsed.parse_tree);

            assert!(debug_tree.contains("ForethoughtTermset"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaNaryForethought
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_nary_forethought_termset_option_grid() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            for (source, extra_branch_count, has_gihi) in [
                ("nu'i ge mi gi do", 0, false),
                ("nu'i ge mi gi do gi ti", 1, false),
                ("nu'i ge mi nu'u gi do nu'u gi ti nu'u", 1, false),
                (
                    "nu'i ge mi nu'u gi do nu'u gi ti nu'u gi ta nu'u gi'i",
                    2,
                    true,
                ),
            ] {
                let parsed = parse_source(source, &options);
                assert_eq!(
                    parsed
                        .warnings
                        .iter()
                        .filter(|warning| {
                            warning.kind
                                == ExperimentalConstruct::ExperimentalZantufaNaryForethought
                        })
                        .count(),
                    extra_branch_count,
                    "{source}"
                );
                assert_eq!(
                    has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalZantufaForethoughtGihi
                    ),
                    has_gihi,
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_nary_forethought_sumti_branches() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("ga lo mlatu gi lo gerku gi lo ractu")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed =
                parse_syntax_tree(&words, &options).expect("valid Zantufa n-ary sumti forethought");

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaNaryForethought
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_nary_forethought_selbri_branches() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi gu'e klama gi cadzu gi bajra")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options)
                .expect("valid Zantufa n-ary selbri forethought");

            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaNaryForethought
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_guskant_sourced_nary_juhe_forethought_example() {
        run_on_normal_stack(|| {
            // Source: guskant, "{tu'e...tu'u} in NU", Google Groups, 2015-07-15.
            let source = "lo nu ju'e gi broda gi brode gi brodi gi brodo gi brodu kei";
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options)
                .expect("valid sourced Zantufa n-ary forethought");

            assert_eq!(
                parsed
                    .warnings
                    .iter()
                    .filter(|warning| {
                        warning.kind == ExperimentalConstruct::ExperimentalZantufaNaryForethought
                    })
                    .count(),
                3
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warns_for_jek_gek_and_bo_gek_extensions() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("je gi mi klama gi do klama")
                .expect("valid morphology");
            let parsed =
                parse_syntax_tree(&words, &ParseOptions::default()).expect("valid jek GEK");
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );

            let words = segment_words_with_modifiers("joi gi bo mi klama gi do klama")
                .expect("valid morphology");
            let parsed = parse_syntax_tree(&words, &ParseOptions::default()).expect("valid BO GEK");
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalZantufaGek)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warns_for_flat_tag_forms() {
        run_on_normal_stack(|| {
            let words =
                segment_words_with_modifiers("na'e fa mi cu klama").expect("valid morphology");

            let parsed = parse_syntax_tree(&words, &ParseOptions::default())
                .expect("valid flattened FA tag");

            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFlattenedTag)
            );
            assert!(
                parsed
                    .warnings
                    .iter()
                    .any(|warning| warning.kind == ExperimentalConstruct::ExperimentalFaAsTag)
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_recursive_tags() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("na'e se na'e se fa mi cu klama")
                .expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid recursive tag");

            assert!(parsed.warnings.iter().any(|warning| {
                warning.kind == ExperimentalConstruct::ExperimentalZantufaRecursiveTag
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classifies_v0_dictionary_first_cases_by_dictionary_selmaho() {
        run_on_normal_stack(|| {
            let cases = [
                (
                    "a'oi do klama",
                    ExperimentalConstruct::ExperimentalDictionaryCoiVocative,
                ),
                (
                    "o'ai do klama",
                    ExperimentalConstruct::ExperimentalDictionaryCoiVocative,
                ),
                (
                    "xe'e lo gerku cu klama",
                    ExperimentalConstruct::ExperimentalDictionaryPaNumber,
                ),
                (
                    "su'ai lo gerku cu klama",
                    ExperimentalConstruct::ExperimentalDictionaryPaNumber,
                ),
                (
                    "xei'e lo kibro mi klama",
                    ExperimentalConstruct::ExperimentalDictionaryFahaTag,
                ),
                (
                    "li'oi mi klama",
                    ExperimentalConstruct::ExperimentalDictionaryUiIndicator,
                ),
            ];

            for (source, expected) in cases {
                assert_warning_kind(source, &ParseOptions::default(), expected);
            }

            let xoi = parse_source("mi klama xoi mutce", &ParseOptions::default());
            assert!(has_warning_kind(
                &xoi,
                ExperimentalConstruct::ExperimentalSoiAdverbial
            ));
            assert!(!has_warning_kind(
                &xoi,
                ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cbm_accepts_cmevla_relation_in_descriptor_arguments() {
        run_on_normal_stack(|| {
            let source = "lo .alis. broda cu melbi";
            let baseline_words = segment_words_with_modifiers(source).expect("valid morphology");
            assert!(parse_syntax_tree(&baseline_words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+CBM)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let cbm = parse_tree_debug(source, &options);
            assert!(cbm.contains("DescriptorWithGadriSumti"));
            assert!(cbm.contains("DescriptionTail"));
            assert!(cbm.contains("Cmevla {"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cbm_warns_for_cmevla_relation_words() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(+CBM)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            assert_warning_kind(
                "lo .alis. broda cu melbi",
                &options,
                ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
            );
            assert_warning_kind(
                ".alis. broda",
                &options,
                ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord,
            );
        });
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_warning_kind(source: &str, options: &ParseOptions, expected: ExperimentalConstruct) {
        let parsed = parse_source(source, options);
        assert!(has_warning_kind(&parsed, expected), "{source}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_warning_kind(parsed: &SyntaxParse, expected: ExperimentalConstruct) -> bool {
        parsed
            .warnings
            .iter()
            .any(|warning| warning.kind == expected)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parse_tree_debug(source: &str, options: &ParseOptions) -> String {
        format!("{:?}", parse_source(source, options).parse_tree)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parse_source(source: &str, options: &ParseOptions) -> SyntaxParse {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        parse_syntax_tree(&words, options).expect("valid syntax")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn indefinite_sumti_explicit_ku_precedes_relative_clause() {
        let valid = segment_words_with_modifiers("mi viska ci gerku ku poi barda")
            .expect("valid morphology");
        assert!(parse_syntax_tree(&valid, &ParseOptions::default()).is_ok());

        let invalid = segment_words_with_modifiers("mi viska ci gerku poi barda ku")
            .expect("valid morphology");
        assert!(parse_syntax_tree(&invalid, &ParseOptions::default()).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn voi_relative_bridi_is_syntax_restrictive() {
        let raw = parse_tree_debug("le gerku voi blabi cu klama", &ParseOptions::default());
        assert!(raw.contains("RestrictiveBridiRelativeClause"));
        assert!(!raw.contains("IncidentalBridiRelativeClause"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_cu_terms_selbri_fallback_parses_alice_naku() {
        let parsed = parse_source("mi cu naku naku klama", &ParseOptions::default());
        assert!(has_warning_kind(
            &parsed,
            ExperimentalConstruct::ExperimentalCuTermsSelbri
        ));
        assert!(format!("{:?}", parsed.parse_tree).contains("CuTermsBridiTail"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_cu_terms_selbri_fallback_preserves_existing_cu_parses() {
        for source in [
            "mi cu pu klama",
            "mi cu na klama",
            "mi cu fa klama",
            "cu klama",
            "cu fa klama",
        ] {
            let parsed = parse_source(source, &ParseOptions::default());
            let raw = format!("{:?}", parsed.parse_tree);
            assert!(
                !raw.contains("TermPrefixedBridiTail"),
                "{source} should keep its existing bridi-tail parse"
            );
            assert!(
                !has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalCuTermsSelbri),
                "{source} should not use the CU TERMS fallback"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_statement_i_stag_bo_accepts_free_modifier() {
        parse_source(
            "do tavla .i ca bo sei mi cusku mi klama",
            &ParseOptions::default(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_ke_termset_parses_alice_table_row() {
        let parsed = parse_source(
            "la .alis. cu penmi le cmalu jubme .i cpana le jubme fa ke po'o le cmacma ke solji ckiku",
            &ParseOptions::default(),
        );
        assert!(has_warning_kind(
            &parsed,
            ExperimentalConstruct::ExperimentalKeTermset
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_repeated_cehe_termset_group_parses_forest_row() {
        run_on_normal_stack(|| {
            let parsed = parse_source(
                ".i ko klama doi cilce je ricfoi ninmu .i ko klama .i mi prami do .i .au mi skicu fi le prenu noi ke'a fi do co'u morji ce'e fe le nu do ca'o renvi gi'e ca'o melbi ce'e fe le nu le risna be do ca'o ka'e prami ce'e fe le nu do badri gi'e se betri",
                &ParseOptions::default(),
            );
            let raw = format!("{:?}", parsed.parse_tree);
            assert!(raw.matches("TermsetGroup").count() >= 3);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_forest_split_quote_rows_parse_when_combined() {
        run_on_fixture_worker_stack(|| {
            parse_source(
                ".i fe lu .e'o sai doi do'u .e'o .e'o doi le ricfoi ninmu do'u .e'o mi catlu do cu pikci cusku fa mi .i ba bo go'i lu pu ki ca le po'o nai nu mi zvati le ckana be fi lo'e cifnu cu skicu fi mi fe lo zabna ranmi be do fe la'e lo se sanga poi jufra do .i je mi manci gi'e audji lo ka co'a zgana do .i mi ca le nu mi verba kei so'i roi ku ca lo nicte cu senva tu'a do fe lo nu do sanga fi mi fe lo jai se manci gi'e punji fi le stedu be mi fe lo xrula noi ja'e jadni ri\n.i ca le nu mi cilce verba be pu zi ku do ca'o raktu mi lo ka senva ma kau gi'e jai se senva mi fai lo nu do fagri gi'e kavbu gi'e jgari mi le ka se xance lo milxe glare kei tai lo nu do ralci gi'e milxe satre gi'e se panci lo ricfoi xrula gi'e vindu ja'e lo nu de'a sanji .i mi pu ta'e senva lo nu mi jersi do ije le risna be mi pu ku audji tu'a do gi'e prami do .i pu ta'e ku ca lo nicte mi di'a cikna tai lo da'i nu mi tirna lo nicte se sanga be do gi'e viska lo nu do vofli ni'a lei cizra tsani .i ku'i do .i do pu zvati ma ja'e lo nu mi tu'a do na ku ka'e ku viska gi'a tirna .i ba'e nau ku mi ta'e catlu le ricfoi gi'e zgana ri fau lo nu mi pacna gi'e djica lo nu mi cliva le cladu tcadu te zu'e lo nu mi klama gi'e penmi do li'u",
                &ParseOptions::default(),
            );
            parse_source(
                "lu .ia nai .i mi ba'o xlura ke ricfoi crida .i mi'a ba'o simxu lo ka kansa fi lo ka vofli bu'u lo ricfoi .i mi'a ba'o zukte lo ka gleki jinru lo ve'i rirxe .i mi'a ba'o cilce kelci ca lo nu le lunra cu te gusni .i mi'a ca cu spofu gi'e badri .i do'o pu lebna tu'a le citno dalgidva pe loi cmana zi'e noi se prami mi'a gi'e na'e dunku gi'e zifre .i le zgike poi sance lo flani pe le dalgidva pu je ca nai se minra fo le se stuzi be lo jbini be lo'i su'o cmana .i je le sance be le nu le dalgidva cu cinmo vasxu cu pu je ca nai se bevri ni'a le klina tsani ca lo nicte .i ba'o ku le dalgidva cu klaku fi tu'a mi'a gi'a senva tu'a mi'a gi'a zenba lo ka kandi ri'a tu'a mi'a\n.i do'o ne le za'u tcadu cu gasnu le cnino nabmi e le daspo be ge mi'a gi le dalgidva .i le dalgidva cu canci gi'e canci fau le nu ri te prina fi no da kei gi'e me le na'e cando virnu noi klama fo lu'i le foldi e le cmana fu lo ka se marce lo cilce xirma zi'e noi gasnu lo banli zi'e noi ta'e ku su'o me ke'a co'a morsi gi'a jinga .i nauku so'u roi ku su'o remna cu klama fo lu'i le klaji pe le ricfoi .i ro go'i cu ruble gi'e dunku gi'e du'e va'e pensi gi'e na'e cinmo gi'e to'e ckire gi'e badri .i le'e remna mo'u cliva mi'a gi'e na'e gleki fau le nu le nei na kansa mi'a .i le banli tcadu ku voi cpana le terdi cu cpana le spofu risna be lo remna .i le nurma tcadu cu simsa lo'e muzga be lo morsi .i bu'u le do'o banli malsi ba'o ku su'o da pikci .i mi pu prami le pa citno pe le cmana .i je ku'i ba bo le se go'i co'u prami mi gi'e cliva .i mi badri gi'e spofu .i ca le'e nicte e le'e donri mi klama fo lu'i le za'u ricfoi gi'e lausku le cmene be ra .i ku'i fliba .i le lastu flani be ra no roi se sance to'o su'o da li'u",
                &ParseOptions::default(),
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chrestomathy_kubla_split_poem_rows_parse_when_combined() {
        run_on_normal_stack(|| {
            parse_source(
                "la .alf. noi censa rirxe lei\nnoi so'i mei vau kevna fo",
                &ParseOptions::default(),
            );
            parse_source(
                ".uo li re pi'i mu se minli\nlei ferti dertu joi lei noi cinla\nvau korcu flecu joi lei purdi",
                &ParseOptions::default(),
            );
        });
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn indicated_word(text: &str) -> Token {
        let mut words = segment_words_with_modifiers(text).expect("valid morphology");
        assert_eq!(words.len(), 1, "test helper expects one word");
        Token::bare(words.remove(0))
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn single_bare_word(text: &str) -> Word {
        let mut words = segment_words_with_modifiers(text).expect("valid morphology");
        assert_eq!(words.len(), 1, "test helper expects one word");
        words
            .remove(0)
            .bare_word()
            .expect("test helper expects a bare word")
            .clone()
    }

    #[requires(true)]
    #[ensures(ret[0] <= ret[1])]
    fn warning_span(warning: &SyntaxWarning) -> [usize; 2] {
        let mut spans = warning.anchor.source_spans();
        spans.sort_by_key(|span| span.byte_start);
        let first = spans.first().expect("warning has source spans");
        let last = spans.last().expect("warning has source spans");
        [first.byte_start, last.byte_end]
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_normal_stack(test: impl FnOnce() + Send) {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("jbotci-syntax-test".to_owned())
                .stack_size(16 * 1024 * 1024)
                .spawn_scoped(scope, test)
                .expect("spawn normal-stack syntax test thread")
                .join()
                .expect("normal-stack syntax test thread panicked");
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_fixture_worker_stack(test: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(test)
            .expect("fixture worker stack test thread should spawn");
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    }
}
