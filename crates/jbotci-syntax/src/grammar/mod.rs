#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use jbotci_diagnostics::{
    TraceEventKind, TraceFailureSummary, TraceLevel, TracePhase, TraceRecorder, TraceReport,
    source_span_from_byte_offsets,
};
use jbotci_dialect::DialectFeature;
use jbotci_morphology::{Cmavo, Phonemes, Selmaho, Word, WordKind, WordLike};
use jbotci_source::SourceSpan;
use jbotci_tree::{RecoveryItemState, TreeVisitor};
use rustc_hash::{FxHashMap, FxHashSet};
use vec1::Vec1;

use crate::tree::{SyntaxRecoveryItem, SyntaxRecoveryItemData, TokenIdentity};
use crate::{
    ExperimentalConstruct, ParseOptions, RecoveredSyntaxParse, RecoveredSyntaxParseAttempt,
    SyntaxError, SyntaxExpectation, SyntaxParse, SyntaxParseAttempt, SyntaxRecoveryParse,
    SyntaxRecoveryParseAttempt, SyntaxRecoveryParseData, SyntaxWarning, Token, WithIndicators,
    WithIndicatorsData, syntax_construct_is_descendant_of, syntax_immediate_child_under,
};

mod baseline_mex;
mod baseline_relative;
mod baseline_selbri;
mod baseline_tag;
mod baseline_termset;
mod generated;
mod generated_runtime;
mod parse_error;
mod parser_core;
mod selbri_boundary;
pub(crate) mod tokens;
use parse_error::{
    SharedStack, SyntaxFound, SyntaxFoundData, SyntaxParseCustomKind, SyntaxParseError,
};
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryReachabilityKindTelemetry {
    pub exact_considered: u64,
    pub exact_run: u64,
    pub exact_skipped: u64,
    pub exact_wins: u64,
    pub natural_wins: u64,
    pub both_fail: u64,
    pub exact_run_rejected: u64,
    pub skip_verified_rejected: u64,
    pub skip_false_positive: u64,
    pub cap_retained_away: u64,
}

impl RecoveryReachabilityKindTelemetry {
    #[requires(true)]
    #[ensures(true)]
    pub fn add_assign(&mut self, other: Self) {
        self.exact_considered += other.exact_considered;
        self.exact_run += other.exact_run;
        self.exact_skipped += other.exact_skipped;
        self.exact_wins += other.exact_wins;
        self.natural_wins += other.natural_wins;
        self.both_fail += other.both_fail;
        self.exact_run_rejected += other.exact_run_rejected;
        self.skip_verified_rejected += other.skip_verified_rejected;
        self.skip_false_positive += other.skip_false_positive;
        self.cap_retained_away += other.cap_retained_away;
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecoveryReachabilityTelemetry {
    pub local: RecoveryReachabilityKindTelemetry,
    pub boundary_resync: RecoveryReachabilityKindTelemetry,
}

impl RecoveryReachabilityTelemetry {
    #[requires(true)]
    #[ensures(true)]
    pub fn add_assign(&mut self, other: Self) {
        self.local.add_assign(other.local);
        self.boundary_resync.add_assign(other.boundary_resync);
    }
}

#[cfg(feature = "expensive_contracts")]
thread_local! {
    static RECOVERY_REACHABILITY_FILTER_DISABLED: Cell<bool> = const { Cell::new(false) };
    static RECOVERY_REACHABILITY_TELEMETRY:
        RefCell<Option<RecoveryReachabilityTelemetry>> = const { RefCell::new(None) };
}

#[cfg(feature = "expensive_contracts")]
#[requires(true)]
#[ensures(true)]
pub fn with_recovery_reachability_instrumentation<T>(
    filter_enabled: bool,
    operation: impl FnOnce() -> T,
) -> (T, RecoveryReachabilityTelemetry) {
    RECOVERY_REACHABILITY_FILTER_DISABLED.with(|disabled| {
        assert!(
            !disabled.replace(!filter_enabled),
            "recovery reachability instrumentation cannot be nested"
        );
    });
    RECOVERY_REACHABILITY_TELEMETRY.with(|telemetry| {
        assert!(
            telemetry
                .replace(Some(RecoveryReachabilityTelemetry::default()))
                .is_none(),
            "recovery reachability telemetry capture cannot be nested"
        );
    });
    let result = operation();
    let telemetry = RECOVERY_REACHABILITY_TELEMETRY.with(|telemetry| {
        telemetry
            .take()
            .expect("recovery reachability telemetry capture is active")
    });
    RECOVERY_REACHABILITY_FILTER_DISABLED.with(|disabled| disabled.set(false));
    (result, telemetry)
}

#[invariant(true)]
#[cfg_attr(not(feature = "expensive_contracts"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryReachabilityTelemetryEvent {
    ExactConsidered,
    ExactRun,
    ExactSkipped,
    ExactWins,
    NaturalWins,
    BothFail,
    ExactRunRejected,
    SkipVerifiedRejected,
    SkipFalsePositive,
    CapRetainedAway,
}

#[requires(true)]
#[ensures(true)]
fn record_recovery_reachability_telemetry(
    kind: RecoveryDirectiveKind,
    event: RecoveryReachabilityTelemetryEvent,
    count: usize,
) {
    #[cfg(feature = "expensive_contracts")]
    RECOVERY_REACHABILITY_TELEMETRY.with(|telemetry| {
        let mut telemetry = telemetry.borrow_mut();
        let Some(telemetry) = telemetry.as_mut() else {
            return;
        };
        let counters = match kind {
            RecoveryDirectiveKind::Local => &mut telemetry.local,
            RecoveryDirectiveKind::BoundaryResync => &mut telemetry.boundary_resync,
        };
        let count = u64::try_from(count).expect("telemetry counts fit in u64");
        let counter = match event {
            RecoveryReachabilityTelemetryEvent::ExactConsidered => &mut counters.exact_considered,
            RecoveryReachabilityTelemetryEvent::ExactRun => &mut counters.exact_run,
            RecoveryReachabilityTelemetryEvent::ExactSkipped => &mut counters.exact_skipped,
            RecoveryReachabilityTelemetryEvent::ExactWins => &mut counters.exact_wins,
            RecoveryReachabilityTelemetryEvent::NaturalWins => &mut counters.natural_wins,
            RecoveryReachabilityTelemetryEvent::BothFail => &mut counters.both_fail,
            RecoveryReachabilityTelemetryEvent::ExactRunRejected => {
                &mut counters.exact_run_rejected
            }
            RecoveryReachabilityTelemetryEvent::SkipVerifiedRejected => {
                &mut counters.skip_verified_rejected
            }
            RecoveryReachabilityTelemetryEvent::SkipFalsePositive => {
                &mut counters.skip_false_positive
            }
            RecoveryReachabilityTelemetryEvent::CapRetainedAway => &mut counters.cap_retained_away,
        };
        *counter = counter
            .checked_add(count)
            .expect("recovery reachability telemetry does not overflow");
    });

    #[cfg(not(feature = "expensive_contracts"))]
    let _ = (kind, event, count);
}

#[invariant(!duration.is_zero(), "an active continuation time limit is nonzero")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContinuationTimeLimit {
    started: Instant,
    duration: Duration,
}

impl ContinuationTimeLimit {
    #[requires(!duration.is_zero())]
    #[ensures(ret.duration == duration)]
    fn new(duration: Duration) -> Self {
        new!(ContinuationTimeLimit {
            started: Instant::now(),
            duration,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn exhausted(self) -> bool {
        self.started.elapsed() >= self.duration
    }
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct ParserStateFinish {
    pub warnings: Vec<SyntaxWarning>,
    pub trace: Option<TraceReport>,
    pub unconsumed_recovery_directives: usize,
    pub recovery_directives: Vec<RecoveryDirective>,
    pub effective_fail_token_indices: Vec<usize>,
    pub completed_recovery_boundary_location: Option<usize>,
    pub recovery_checkpoints: Vec<RecoveryCheckpoint>,
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
    abandoned_range_count: usize,
    completed_recovery_boundary_location: Option<usize>,
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
    recovery_enabled: bool,
}

impl SyntaxRuleFrame {
    #[requires(!rule.is_empty())]
    #[ensures(ret.rule == rule)]
    #[ensures(ret.byte_start == byte_start)]
    #[ensures(ret.recovery_enabled == recovery_enabled)]
    pub(super) fn new(rule: &'static str, byte_start: usize, recovery_enabled: bool) -> Self {
        new!(SyntaxRuleFrame {
            rule,
            byte_start,
            recovery_enabled,
        })
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

    #[requires(true)]
    #[ensures(ret == self.recovery_enabled)]
    pub(super) fn recovery_enabled(&self) -> bool {
        self.recovery_enabled
    }
}

#[invariant(!rule.is_empty())]
#[invariant(fail_token_index <= resume_token_index)]
#[invariant(match (kind, boundary_unwind_start_token_index) {
    (RecoveryDirectiveKind::Local, None) => true,
    (RecoveryDirectiveKind::BoundaryResync, Some(start)) => *start < *resume_token_index,
    _ => false,
})]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecoveryDirective {
    kind: RecoveryDirectiveKind,
    boundary_unwind_start_token_index: Option<usize>,
    rule: &'static str,
    instance_byte_start: usize,
    fail_token_index: usize,
    natural_stop_enabled: bool,
    resume_token_index: usize,
    resume_field: usize,
    error_index: usize,
    error: SyntaxError,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryDirectiveKind {
    Local,
    BoundaryResync,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum RecoveryCheckpointKind {
    FieldStart,
    Trailing,
}

#[invariant(!rule.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct RecoveryCheckpoint {
    rule: &'static str,
    instance_byte_start: usize,
    token_index: usize,
    field_index: usize,
    kind: RecoveryCheckpointKind,
}

impl RecoveryCheckpoint {
    #[requires(!rule.is_empty())]
    #[ensures(ret.rule == rule)]
    #[ensures(ret.instance_byte_start == instance_byte_start)]
    #[ensures(ret.token_index == token_index)]
    #[ensures(ret.field_index == field_index)]
    #[ensures(ret.kind == kind)]
    fn new(
        rule: &'static str,
        instance_byte_start: usize,
        token_index: usize,
        field_index: usize,
        kind: RecoveryCheckpointKind,
    ) -> Self {
        new!(RecoveryCheckpoint {
            rule,
            instance_byte_start,
            token_index,
            field_index,
            kind,
        })
    }

    #[requires(true)]
    #[ensures(ret.rule == self.rule)]
    #[ensures(ret.instance_byte_start == self.instance_byte_start)]
    #[ensures(ret.token_index == self.token_index)]
    fn site(&self) -> RecoveryCheckpointSite {
        new!(RecoveryCheckpointSite {
            rule: self.rule,
            instance_byte_start: self.instance_byte_start,
            token_index: self.token_index,
        })
    }
}

#[invariant(!rule.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecoveryCheckpointSite {
    rule: &'static str,
    instance_byte_start: usize,
    token_index: usize,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct RecoveryCheckpointIndex {
    minimum_field_indices: FxHashMap<RecoveryCheckpointSite, usize>,
    #[cfg(feature = "expensive_contracts")]
    checkpoints: Vec<RecoveryCheckpoint>,
}

impl RecoveryCheckpointIndex {
    #[requires(true)]
    #[ensures(true)]
    #[cfg_attr(feature = "expensive_contracts", expensive_ensures(
        ret.checkpoints.iter().all(|checkpoint| {
            ret.minimum_field_indices
                .get(&checkpoint.site())
                .is_some_and(|minimum| *minimum <= checkpoint.field_index)
        })
    ))]
    pub(super) fn from_checkpoints(checkpoints: Vec<RecoveryCheckpoint>) -> Self {
        let mut minimum_field_indices = FxHashMap::default();
        for checkpoint in &checkpoints {
            minimum_field_indices
                .entry(checkpoint.site())
                .and_modify(|minimum: &mut usize| {
                    *minimum = (*minimum).min(checkpoint.field_index);
                })
                .or_insert(checkpoint.field_index);
        }
        Self {
            minimum_field_indices,
            #[cfg(feature = "expensive_contracts")]
            checkpoints,
        }
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    fn contains_local_exact_site(
        &self,
        rule: &'static str,
        instance_byte_start: usize,
        token_index: usize,
        resume_field: usize,
    ) -> bool {
        let site = new!(RecoveryCheckpointSite {
            rule,
            instance_byte_start,
            token_index,
        });
        self.minimum_field_indices
            .get(&site)
            .is_some_and(|minimum| *minimum <= resume_field)
    }

    #[cfg(feature = "expensive_contracts")]
    #[requires(true)]
    #[ensures(true)]
    fn iter(&self) -> impl Iterator<Item = &RecoveryCheckpoint> {
        self.checkpoints.iter()
    }
}

#[invariant(*effective_fail_token_index <= directive.fail_token_index)]
#[invariant(*effective_fail_token_index <= directive.resume_token_index)]
#[invariant(matches!(directive.kind, RecoveryDirectiveKind::Local) || *effective_fail_token_index < directive.resume_token_index)]
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
    BoundaryResync,
    Resume,
}

#[invariant(item.as_ref().is_none_or(|item| item.recovery_error_index().is_some()))]
#[invariant(match kind {
    RecoveryFieldActionKind::Abandon => resume_token_index.is_none(),
    RecoveryFieldActionKind::BoundaryResync | RecoveryFieldActionKind::Resume =>
        resume_token_index.is_some_and(|index| index < usize::MAX),
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
    #[ensures(ret.kind == RecoveryFieldActionKind::BoundaryResync)]
    pub(super) fn boundary_resync(item: SyntaxRecoveryItem, resume_token_index: usize) -> Self {
        new!(RecoveryFieldAction {
            kind: RecoveryFieldActionKind::BoundaryResync,
            item: Some(item),
            resume_token_index: Some(resume_token_index),
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
            kind: RecoveryDirectiveKind::Local,
            boundary_unwind_start_token_index: None,
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

    #[requires(unwind_start_token_index < self.resume_token_index)]
    #[ensures(ret.kind == RecoveryDirectiveKind::BoundaryResync)]
    #[ensures(ret.boundary_unwind_start_token_index == Some(unwind_start_token_index))]
    fn into_boundary_resync(self, unwind_start_token_index: usize) -> Self {
        self.with_data(data! {
            kind: RecoveryDirectiveKind::BoundaryResync,
            boundary_unwind_start_token_index: Some(unwind_start_token_index),
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
    value: Rc<dyn Any>,
}

#[invariant(true)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SyntaxMemoScope {
    #[default]
    Ordinary,
    CeiFree,
    DescriptionRelative,
    CeiFreeDescriptionRelative,
}

impl SyntaxMemoScope {
    #[requires(true)]
    #[ensures(true)]
    fn nested(self, nested: Self) -> Self {
        match (self, nested) {
            (Self::Ordinary, nested) | (nested, Self::Ordinary) => nested,
            (Self::CeiFree, Self::DescriptionRelative)
            | (Self::DescriptionRelative, Self::CeiFree)
            | (Self::CeiFreeDescriptionRelative, _)
            | (_, Self::CeiFreeDescriptionRelative) => Self::CeiFreeDescriptionRelative,
            (scope, _) => scope,
        }
    }
}

type StrictSyntaxMemoKey = (&'static str, usize, SyntaxMemoScope);
type RecoverySyntaxMemoKey = (&'static str, usize, SyntaxMemoScope, usize, usize);
type RecoverySyntaxMemoInProgressKey = (&'static str, usize, SyntaxMemoScope, usize);

impl fmt::Debug for SyntaxMemoValue {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SyntaxMemoValue(..)")
    }
}

impl SyntaxMemoValue {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn from_shared(value: Rc<dyn Any>) -> Self {
        Self { value }
    }
}

#[invariant(start_location <= end_location, "memo success must not rewind input")]
#[invariant(recovery_index <= consumed_recovery_directives)]
#[invariant(effective_fail_token_indices.len() == consumed_recovery_directives - recovery_index)]
#[derive(Debug, Clone)]
pub(super) struct SyntaxMemoSuccess<'tokens> {
    start_location: usize,
    end_location: usize,
    recovery_index: usize,
    consumed_recovery_directives: usize,
    effective_fail_token_indices: Vec<usize>,
    value: SyntaxMemoValue,
    side_effects: SyntaxMemoSideEffects<'tokens>,
    rule_observation_node: Option<usize>,
}

#[invariant(start_location <= end_location)]
#[derive(Debug, Clone)]
struct SyntaxMemoFailure<'tokens> {
    start_location: usize,
    end_location: usize,
    error: SyntaxParseError<'tokens>,
    recovery_checkpoint_observations: Option<Rc<SyntaxRecoveryCheckpointObservations>>,
    diagnostic_observations: Option<Rc<SyntaxDiagnosticObservations<'tokens>>>,
    rule_observation_node: Option<usize>,
}

impl<'tokens> SyntaxMemoFailure<'tokens> {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn into_error(self) -> SyntaxParseError<'tokens> {
        self.into_data().error
    }
}

#[invariant(start_location < end_location)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundaryAbandonedRange {
    start_location: usize,
    end_location: usize,
}

impl BoundaryAbandonedRange {
    #[requires(start_location < end_location)]
    #[ensures(ret.start_location == start_location)]
    #[ensures(ret.end_location == end_location)]
    fn new(start_location: usize, end_location: usize) -> Self {
        new!(BoundaryAbandonedRange {
            start_location,
            end_location,
        })
    }

    #[requires(start_location <= end_location)]
    #[ensures(true)]
    fn intersects(self, start_location: usize, end_location: usize) -> bool {
        if start_location == end_location {
            self.start_location <= start_location && start_location < self.end_location
        } else {
            start_location < self.end_location && self.start_location < end_location
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct SyntaxMemoSideEffects<'tokens> {
    warnings: Rc<[SyntaxWarning]>,
    recovery_checkpoint_observations: Option<Rc<SyntaxRecoveryCheckpointObservations>>,
    diagnostic_observations: Option<Rc<SyntaxDiagnosticObservations<'tokens>>>,
}

#[invariant(!rule.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SyntaxRuleObservation {
    rule: &'static str,
    instance_byte_start: usize,
}

#[invariant(true)]
#[derive(Debug)]
struct SyntaxRuleObservationNode {
    observation: SyntaxRuleObservation,
    children: Vec<usize>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SyntaxDiagnosticObservationId {
    trial_id: NonZeroUsize,
    frame_id: NonZeroUsize,
}

#[invariant(!observations.is_empty())]
#[derive(Debug)]
struct SyntaxDiagnosticObservations<'tokens> {
    id: SyntaxDiagnosticObservationId,
    observations: Rc<[SyntaxDiagnosticObservation<'tokens>]>,
}

#[invariant(::Candidate(_) => true)]
#[invariant(::Nested(observations) => !observations.observations.is_empty())]
#[derive(Debug, Clone)]
enum SyntaxDiagnosticObservation<'tokens> {
    Candidate(SyntaxParseError<'tokens>),
    Nested(Rc<SyntaxDiagnosticObservations<'tokens>>),
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct SyntaxMemoRuleFrame<'tokens> {
    recovery_sensitive: bool,
    rule_observation: Option<SyntaxRuleObservation>,
    child_rule_observation_nodes: Vec<usize>,
    finalized_rule_observation_node: Option<usize>,
    recovery_checkpoint_observation_range: RecoveryCheckpointObservationRange,
    child_recovery_checkpoint_observations: Vec<ChildRecoveryCheckpointObservations>,
    finalized_recovery_checkpoint_observations: Option<Rc<SyntaxRecoveryCheckpointObservations>>,
    diagnostic_observation_id: Option<SyntaxDiagnosticObservationId>,
    diagnostic_observations: Vec<SyntaxDiagnosticObservation<'tokens>>,
    finalized_diagnostic_observations: Option<Rc<SyntaxDiagnosticObservations<'tokens>>>,
}

#[invariant(start <= end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryCheckpointObservationRange {
    start: usize,
    end: usize,
}

#[invariant(!checkpoints.is_empty() || !children.is_empty())]
#[derive(Debug, Clone)]
struct SyntaxRecoveryCheckpointObservations {
    checkpoints: Rc<[RecoveryCheckpoint]>,
    children: Rc<[Rc<SyntaxRecoveryCheckpointObservations>]>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct ChildRecoveryCheckpointObservations {
    range: RecoveryCheckpointObservationRange,
    observations: Rc<SyntaxRecoveryCheckpointObservations>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecoveryCheckpointId {
    index: usize,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct RecoveryCheckpointCollection {
    checkpoints: Vec<RecoveryCheckpoint>,
    checkpoint_ids: FxHashMap<RecoveryCheckpoint, RecoveryCheckpointId>,
    observations: Vec<RecoveryCheckpointId>,
    last_observation_indices: Vec<Option<usize>>,
    last_checkpoint_id: Option<RecoveryCheckpointId>,
    registered_observation_node_pointers: FxHashSet<*const SyntaxRecoveryCheckpointObservations>,
    registered_observation_nodes: Vec<Rc<SyntaxRecoveryCheckpointObservations>>,
    snapshot_marks: Vec<usize>,
    next_snapshot_mark: usize,
}

// Keep the mutable arena's contracts incremental. Whole-arena scans after
// every `record` would recreate quadratic work in expensive-contract builds;
// private construction plus the mutators' local postconditions preserve the
// identity and pointer relationships inductively.
#[invariant(self.checkpoints.len() == self.checkpoint_ids.len())]
#[invariant(self.checkpoints.len() == self.last_observation_indices.len())]
#[invariant(self.checkpoints.len() == self.snapshot_marks.len())]
#[invariant(
    self.registered_observation_node_pointers.len()
        == self.registered_observation_nodes.len()
)]
#[invariant(self.next_snapshot_mark != 0)]
impl RecoveryCheckpointCollection {
    #[requires(true)]
    #[ensures(ret.checkpoints.is_empty())]
    #[ensures(ret.observations.is_empty())]
    fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            checkpoint_ids: FxHashMap::default(),
            observations: Vec::new(),
            last_observation_indices: Vec::new(),
            last_checkpoint_id: None,
            registered_observation_node_pointers: FxHashSet::default(),
            registered_observation_nodes: Vec::new(),
            snapshot_marks: Vec::new(),
            next_snapshot_mark: 1,
        }
    }

    #[requires(true)]
    #[ensures(ret == self.observations.len())]
    fn observation_count(&self) -> usize {
        self.observations.len()
    }

    #[requires(active_frame_start.is_none_or(|start| start <= self.observations.len()))]
    #[ensures(self.observations.len() >= old(self.observations.len()))]
    #[ensures(self.observations.len() <= old(self.observations.len()) + 1)]
    fn record(&mut self, checkpoint: RecoveryCheckpoint, active_frame_start: Option<usize>) {
        let checkpoint_id = self.intern(checkpoint);

        let active_frame_start = active_frame_start.unwrap_or(0);
        if self.last_observation_indices[checkpoint_id.index]
            .is_some_and(|last| last >= active_frame_start)
        {
            return;
        }
        let observation_index = self.observations.len();
        self.observations.push(checkpoint_id);
        self.last_observation_indices[checkpoint_id.index] = Some(observation_index);
    }

    #[requires(true)]
    #[ensures(ret.index < self.checkpoints.len())]
    #[ensures(self.checkpoint_ids.get(&self.checkpoints[ret.index]) == Some(&ret))]
    fn intern(&mut self, checkpoint: RecoveryCheckpoint) -> RecoveryCheckpointId {
        let checkpoint_id = self
            .last_checkpoint_id
            .filter(|id| self.checkpoints[id.index] == checkpoint)
            .unwrap_or_else(|| match self.checkpoint_ids.entry(checkpoint) {
                std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let checkpoint_id = RecoveryCheckpointId {
                        index: self.checkpoints.len(),
                    };
                    self.checkpoints.push(entry.key().clone());
                    self.last_observation_indices.push(None);
                    self.snapshot_marks.push(0);
                    entry.insert(checkpoint_id);
                    checkpoint_id
                }
            });
        self.last_checkpoint_id = Some(checkpoint_id);
        checkpoint_id
    }

    #[requires(true)]
    #[ensures(
        self.registered_observation_node_pointers
            .contains(&Rc::as_ptr(observations))
    )]
    fn register_observation_node(
        &mut self,
        observations: &Rc<SyntaxRecoveryCheckpointObservations>,
    ) {
        let mut pending = vec![Rc::clone(observations)];
        while let Some(observations) = pending.pop() {
            let pointer = Rc::as_ptr(&observations);
            if !self.registered_observation_node_pointers.insert(pointer) {
                continue;
            }
            self.registered_observation_nodes
                .push(Rc::clone(&observations));
            for checkpoint in observations.checkpoints.iter() {
                self.intern(checkpoint.clone());
            }
            pending.extend(observations.children.iter().rev().cloned());
        }
    }

    #[cfg(test)]
    #[requires(start <= end)]
    #[requires(end <= self.observations.len())]
    #[ensures(true)]
    fn capture_range(&mut self, start: usize, end: usize) -> Rc<[RecoveryCheckpoint]> {
        let snapshot_mark = self.begin_snapshot();
        let mut checkpoints = Vec::new();
        self.append_unique_range_to(start, end, snapshot_mark, &mut checkpoints);
        checkpoints.into()
    }

    #[requires(true)]
    #[ensures(ret != 0)]
    fn begin_snapshot(&mut self) -> usize {
        let snapshot_mark = self.next_snapshot_mark;
        self.next_snapshot_mark = self.next_snapshot_mark.checked_add(1).unwrap_or_else(|| {
            self.snapshot_marks.fill(0);
            1
        });
        snapshot_mark
    }

    #[requires(start <= end)]
    #[requires(end <= self.observations.len())]
    #[requires(snapshot_mark != 0)]
    #[ensures(checkpoints.len() >= old(checkpoints.len()))]
    fn append_unique_range_to(
        &mut self,
        start: usize,
        end: usize,
        snapshot_mark: usize,
        checkpoints: &mut Vec<RecoveryCheckpoint>,
    ) {
        for checkpoint_id in &self.observations[start..end] {
            let mark = &mut self.snapshot_marks[checkpoint_id.index];
            if *mark == snapshot_mark {
                continue;
            }
            *mark = snapshot_mark;
            checkpoints.push(self.checkpoints[checkpoint_id.index].clone());
        }
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(self.checkpoints.is_empty())]
    #[ensures(self.checkpoint_ids.is_empty())]
    #[ensures(self.observations.is_empty())]
    #[ensures(self.last_observation_indices.is_empty())]
    #[ensures(self.registered_observation_node_pointers.is_empty())]
    #[ensures(self.registered_observation_nodes.is_empty())]
    #[ensures(self.snapshot_marks.is_empty())]
    fn clear(&mut self) {
        self.checkpoints.clear();
        self.checkpoint_ids.clear();
        self.observations.clear();
        self.last_observation_indices.clear();
        self.last_checkpoint_id = None;
        self.registered_observation_node_pointers.clear();
        self.registered_observation_nodes.clear();
        self.snapshot_marks.clear();
        self.next_snapshot_mark = 1;
    }
}

impl RecoveryCheckpointCollection {
    #[requires(true)]
    #[ensures(true)]
    fn into_checkpoints(self) -> Vec<RecoveryCheckpoint> {
        self.checkpoints
    }
}

#[invariant(true)]
#[derive(Debug, Default)]
struct SyntaxRecoveryMemoStore<'tokens> {
    insensitive_successes: HashMap<StrictSyntaxMemoKey, SyntaxMemoSuccess<'tokens>>,
    sensitive_successes: HashMap<RecoverySyntaxMemoKey, SyntaxMemoSuccess<'tokens>>,
    insensitive_failures: HashMap<StrictSyntaxMemoKey, SyntaxMemoFailure<'tokens>>,
    sensitive_failures: HashMap<RecoverySyntaxMemoKey, SyntaxMemoFailure<'tokens>>,
    rule_observation_nodes: Vec<SyntaxRuleObservationNode>,
    rule_sensitivity_cache: RefCell<FxHashMap<SyntaxRuleObservation, Vec<Option<bool>>>>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct SyntaxRecoveryMemoTrial<'tokens> {
    trial_id: NonZeroUsize,
    store: Rc<RefCell<SyntaxRecoveryMemoStore<'tokens>>>,
}

#[invariant(true)]
#[derive(Debug)]
pub(super) struct SyntaxRecoveryMemoSession<'tokens> {
    next_trial_id: NonZeroUsize,
    store: Rc<RefCell<SyntaxRecoveryMemoStore<'tokens>>>,
}

impl<'tokens> SyntaxRecoveryMemoSession<'tokens> {
    #[requires(true)]
    #[ensures(ret.next_trial_id.get() == 1)]
    #[ensures(ret.store.borrow().insensitive_successes.is_empty())]
    pub(super) fn new() -> Self {
        Self {
            next_trial_id: NonZeroUsize::MIN,
            store: Rc::new(RefCell::new(SyntaxRecoveryMemoStore::default())),
        }
    }

    #[requires(self.next_trial_id.get() < usize::MAX)]
    #[ensures(ret.trial_id == old(self.next_trial_id))]
    #[ensures(self.next_trial_id.get() == old(self.next_trial_id.get()) + 1)]
    fn begin_trial(&mut self) -> SyntaxRecoveryMemoTrial<'tokens> {
        let trial = SyntaxRecoveryMemoTrial {
            trial_id: self.next_trial_id,
            store: Rc::clone(&self.store),
        };
        self.next_trial_id = NonZeroUsize::new(
            self.next_trial_id
                .get()
                .checked_add(1)
                .expect("recovery memo trial identity does not overflow"),
        )
        .expect("a positive recovery memo trial identity stays nonzero");
        trial
    }

    #[requires(trial_id > 0)]
    #[ensures(!self.store.borrow().sensitive_successes.keys().any(|(_, _, _, entry_trial_id, _)| *entry_trial_id == trial_id))]
    #[ensures(!self.store.borrow().sensitive_failures.keys().any(|(_, _, _, entry_trial_id, _)| *entry_trial_id == trial_id))]
    fn finish_trial(&mut self, trial_id: usize) {
        let mut store = self.store.borrow_mut();
        store
            .sensitive_successes
            .retain(|(_, _, _, entry_trial_id, _), _| *entry_trial_id != trial_id);
        store
            .sensitive_failures
            .retain(|(_, _, _, entry_trial_id, _), _| *entry_trial_id != trial_id);
    }

    #[requires(true)]
    #[ensures(self.store.borrow().insensitive_successes.is_empty())]
    #[ensures(self.store.borrow().insensitive_failures.is_empty())]
    fn clear(&mut self) {
        *self.store.borrow_mut() = SyntaxRecoveryMemoStore::default();
    }
}

#[invariant(recovery_trial_id.is_none() -> *recovery_index == 0)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SyntaxMemoContext {
    recovery_trial_id: Option<usize>,
    recovery_index: usize,
    scope: SyntaxMemoScope,
}

#[invariant(true)]
pub(super) struct SyntaxMemoSuccessHit<'tokens> {
    memo: SyntaxMemoSuccess<'tokens>,
    sensitive: bool,
}

impl SyntaxMemoSuccessHit<'_> {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn value(&self) -> Rc<dyn Any> {
        Rc::clone(&self.memo.value.value)
    }
}

#[invariant(true)]
pub(super) struct SyntaxMemoReplayEffects<'tokens> {
    end_location: usize,
    side_effects: SyntaxMemoSideEffects<'tokens>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct ParserState<'tokens> {
    anchor_token_identities: Vec<TokenIdentity>,
    syntax_location_byte_offsets: Vec<usize>,
    // A source range is attribution, not token identity: dialect expansion
    // siblings deliberately share a range. Token clones share one stable Arc
    // allocation, while distinct expansion siblings do not, making this the
    // exact identity needed by the parser-local classification cache.
    cmavo_cache: HashMap<TokenIdentity, Option<Cmavo>>,
    syntax_memo: HashMap<StrictSyntaxMemoKey, SyntaxMemoSuccess<'tokens>>,
    syntax_failure_memo: HashMap<StrictSyntaxMemoKey, SyntaxParseError<'tokens>>,
    syntax_memo_in_progress: HashSet<StrictSyntaxMemoKey>,
    syntax_recovery_memo_in_progress: HashSet<RecoverySyntaxMemoInProgressKey>,
    recovery_memo_trial: Option<SyntaxRecoveryMemoTrial<'tokens>>,
    syntax_memo_rule_frames: Vec<SyntaxMemoRuleFrame<'tokens>>,
    syntax_memo_scope: SyntaxMemoScope,
    next_syntax_diagnostic_observation_frame_id: NonZeroUsize,
    replayed_syntax_diagnostic_observations: HashSet<SyntaxDiagnosticObservationId>,
    diagnostic_candidates: Vec<SyntaxParseError<'tokens>>,
    diagnostic_candidate_hash_buckets: HashMap<u64, Vec<usize>>,
    continuation_diagnostic_candidates: Vec<SyntaxParseError<'tokens>>,
    warnings: Vec<SyntaxWarning>,
    trace: TraceRecorder,
    active_syntax_contexts: Vec<SyntaxContextFrame>,
    active_syntax_rules: Vec<SyntaxRuleFrame>,
    active_syntax_context_stack: SharedStack<SyntaxContextFrame>,
    active_syntax_rule_stack: SharedStack<SyntaxRuleFrame>,
    recovery_directives: Vec<RecoveryDirective>,
    recovery_rule_parser_targets: HashSet<(&'static str, usize)>,
    recovery_rule_target_last_indices: FxHashMap<SyntaxRuleObservation, usize>,
    syntax_rule_observation_latest_recovery_target_indices:
        RefCell<FxHashMap<usize, Option<usize>>>,
    consumed_recovery_directives: usize,
    // This is a consumption stack parallel to `recovery_directives`; each
    // entry records where its directive actually fired. Checkpoint rewind can
    // therefore restore recovery state by truncating instead of cloning and
    // rewriting every remaining directive.
    effective_fail_token_indices: Vec<usize>,
    active_recovery_directive: Option<ActiveRecoveryDirective>,
    abandoned_recovery_ranges: Vec<BoundaryAbandonedRange>,
    completed_recovery_boundary_location: Option<usize>,
    recovery_checkpoint_collection: Option<RecoveryCheckpointCollection>,
    recovery_tokens: Vec<Token>,
    recovery_source: Option<Arc<str>>,
    track_recovery_branches: bool,
    syntax_grammar_env: generated_runtime::SyntaxGrammarEnv,
    continuation_sentinel_index: Option<usize>,
    continuation_time_limit: Option<ContinuationTimeLimit>,
    _tokens: PhantomData<&'tokens ()>,
}

#[invariant(
    self.syntax_location_byte_offsets.is_empty()
        || self.syntax_location_byte_offsets.len() == self.anchor_token_identities.len() + 1,
    "syntax location offsets include one EOF offset after token anchors"
)]
#[invariant(self.effective_fail_token_indices.len() == self.consumed_recovery_directives)]
#[invariant(self.continuation_sentinel_index.is_some() || self.continuation_diagnostic_candidates.is_empty())]
#[invariant(self.recovery_checkpoint_collection.is_none() || self.track_recovery_branches)]
#[expensive_invariant(
    true,
    "syntax memo keys are protected by ParserState's private mutation APIs"
)]
#[expensive_invariant(
    self.abandoned_recovery_ranges
        .windows(2)
        .all(|ranges| ranges[0].end_location <= ranges[1].start_location),
    "abandoned recovery ranges are ordered and disjoint"
)]
impl<'tokens> ParserState<'tokens> {
    #[requires(true)]
    #[ensures(ret.anchor_token_identities.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    pub(super) fn new(words: &[Token], options: &ParseOptions) -> Self {
        Self {
            anchor_token_identities: words.iter().map(Token::identity).collect(),
            syntax_location_byte_offsets: syntax_location_byte_offsets(words),
            cmavo_cache: HashMap::new(),
            syntax_memo: HashMap::new(),
            syntax_failure_memo: HashMap::new(),
            syntax_memo_in_progress: HashSet::new(),
            syntax_recovery_memo_in_progress: HashSet::new(),
            recovery_memo_trial: None,
            syntax_memo_rule_frames: Vec::new(),
            syntax_memo_scope: SyntaxMemoScope::Ordinary,
            next_syntax_diagnostic_observation_frame_id: NonZeroUsize::MIN,
            replayed_syntax_diagnostic_observations: HashSet::new(),
            diagnostic_candidates: Vec::new(),
            diagnostic_candidate_hash_buckets: HashMap::new(),
            continuation_diagnostic_candidates: Vec::new(),
            warnings: Vec::new(),
            trace: TraceRecorder::new(options.trace.clone(), TracePhase::Syntax),
            active_syntax_contexts: Vec::new(),
            active_syntax_rules: Vec::new(),
            active_syntax_context_stack: SharedStack::empty(),
            active_syntax_rule_stack: SharedStack::empty(),
            recovery_directives: Vec::new(),
            recovery_rule_parser_targets: HashSet::new(),
            recovery_rule_target_last_indices: FxHashMap::default(),
            syntax_rule_observation_latest_recovery_target_indices: RefCell::new(
                FxHashMap::default(),
            ),
            consumed_recovery_directives: 0,
            effective_fail_token_indices: Vec::new(),
            active_recovery_directive: None,
            abandoned_recovery_ranges: Vec::new(),
            completed_recovery_boundary_location: None,
            recovery_checkpoint_collection: None,
            recovery_tokens: Vec::new(),
            recovery_source: None,
            track_recovery_branches: false,
            syntax_grammar_env: generated_runtime::SyntaxGrammarEnv::from_options(options),
            continuation_sentinel_index: None,
            continuation_time_limit: None,
            _tokens: PhantomData,
        }
    }

    #[requires(sentinel_index < words.len())]
    #[ensures(ret.continuation_sentinel_index == Some(sentinel_index))]
    #[ensures(ret.continuation_time_limit == continuation_time_limit)]
    pub(super) fn new_for_expected_continuations(
        words: &[Token],
        options: &ParseOptions,
        sentinel_index: usize,
        continuation_time_limit: Option<ContinuationTimeLimit>,
    ) -> Self {
        let mut state = Self::new(words, options);
        state.track_recovery_branches = true;
        state.continuation_sentinel_index = Some(sentinel_index);
        state.continuation_time_limit = continuation_time_limit;
        state
    }

    #[requires(true)]
    #[ensures(ret.anchor_token_identities.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    pub(super) fn new_with_recovery_branches(words: &[Token], options: &ParseOptions) -> Self {
        let mut state = Self::new(words, options);
        state.track_recovery_branches = true;
        state.recovery_checkpoint_collection = Some(RecoveryCheckpointCollection::new());
        state
    }

    #[requires(continuation_sentinel_index.is_none_or(|index| index < words.len()))]
    #[ensures(ret.anchor_token_identities.len() == words.len())]
    #[ensures(ret.syntax_location_byte_offsets.len() == words.len() + 1)]
    #[ensures(ret.continuation_sentinel_index == continuation_sentinel_index)]
    pub(super) fn new_with_recovery(
        words: &[Token],
        source: Option<&str>,
        options: &ParseOptions,
        directives: &[RecoveryDirective],
        memo_trial: SyntaxRecoveryMemoTrial<'tokens>,
        continuation_sentinel_index: Option<usize>,
        continuation_time_limit: Option<ContinuationTimeLimit>,
    ) -> Self {
        let mut state = Self::new_with_recovery_branches(words, options);
        state.recovery_directives = directives.to_vec();
        state.recovery_rule_parser_targets = directives
            .iter()
            .map(|directive| (directive.rule, directive.instance_byte_start))
            .collect();
        for (index, directive) in directives.iter().enumerate() {
            state.recovery_rule_target_last_indices.insert(
                new!(SyntaxRuleObservation {
                    rule: directive.rule,
                    instance_byte_start: directive.instance_byte_start,
                }),
                index,
            );
        }
        state.recovery_tokens = words.to_vec();
        state.recovery_source = source.map(Arc::<str>::from);
        state.recovery_memo_trial = Some(memo_trial);
        state.continuation_sentinel_index = continuation_sentinel_index;
        state.continuation_time_limit = continuation_time_limit;
        state
    }

    #[requires(true)]
    #[ensures(ret == (self.continuation_sentinel_index == Some(location)))]
    pub(super) fn is_continuation_sentinel_location(&self, location: usize) -> bool {
        self.continuation_sentinel_index == Some(location)
    }

    #[requires(true)]
    #[ensures(ret == token.cmavo())]
    pub(super) fn token_cmavo(&mut self, token: &Token) -> Option<Cmavo> {
        let key = token.identity();
        if let Some(cmavo) = self.cmavo_cache.get(&key) {
            *cmavo
        } else {
            let cmavo = token.cmavo();
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

    #[requires(!rule.is_empty())]
    #[ensures(ret == (self.active_recovery_directive.as_ref().is_some_and(|active| active.directive.rule == rule && active.directive.instance_byte_start == instance_byte_start) || self.recovery_directives[self.consumed_recovery_directives..].iter().any(|directive| directive.rule == rule && directive.instance_byte_start == instance_byte_start)))]
    pub(super) fn recovery_rule_parser_enabled(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
    ) -> bool {
        if !self
            .recovery_rule_parser_targets
            .contains(&(rule, instance_byte_start))
        {
            return false;
        }
        self.observe_recovery_directive_state();
        self.active_recovery_directive
            .as_ref()
            .is_some_and(|active| {
                active.directive.rule == rule
                    && active.directive.instance_byte_start == instance_byte_start
            })
            || self.recovery_directives[self.consumed_recovery_directives..]
                .iter()
                .any(|directive| {
                    directive.rule == rule && directive.instance_byte_start == instance_byte_start
                })
    }

    #[requires(true)]
    #[ensures(ret == self.track_recovery_branches)]
    pub(super) fn recovery_branch_tracking_enabled(&self) -> bool {
        self.track_recovery_branches
    }

    #[requires(true)]
    #[ensures(ret.recovery_index == self.consumed_recovery_directives)]
    #[ensures(ret.recovery_trial_id.is_some() == self.recovery_enabled())]
    pub(super) fn syntax_memo_context(&self) -> SyntaxMemoContext {
        new!(SyntaxMemoContext {
            recovery_trial_id: self
                .recovery_memo_trial
                .as_ref()
                .map(|trial| trial.trial_id.get()),
            recovery_index: self.consumed_recovery_directives,
            scope: self.syntax_memo_scope,
        })
    }

    #[requires(true)]
    #[ensures(self.syntax_memo_scope == old(self.syntax_memo_scope).nested(scope))]
    pub(super) fn enter_syntax_memo_scope(&mut self, scope: SyntaxMemoScope) -> SyntaxMemoScope {
        let previous = self.syntax_memo_scope;
        self.syntax_memo_scope = previous.nested(scope);
        previous
    }

    #[requires(true)]
    #[ensures(self.syntax_memo_scope == scope)]
    pub(super) fn restore_syntax_memo_scope(&mut self, scope: SyntaxMemoScope) {
        self.syntax_memo_scope = scope;
    }

    #[requires(true)]
    #[ensures(self.syntax_memo_rule_frames.len() == old(self.syntax_memo_rule_frames.len()) + 1)]
    #[ensures(!self.syntax_memo_rule_frames.last().map_or(true, |frame| frame.recovery_sensitive))]
    pub(super) fn begin_syntax_memo_rule_frame(&mut self) {
        let recovery_checkpoint_observation_start = self
            .recovery_checkpoint_collection
            .as_ref()
            .map_or(0, RecoveryCheckpointCollection::observation_count);
        let diagnostic_observation_id = self.recovery_memo_trial.as_ref().map(|trial| {
            let frame_id = self.next_syntax_diagnostic_observation_frame_id;
            self.next_syntax_diagnostic_observation_frame_id = NonZeroUsize::new(
                frame_id
                    .get()
                    .checked_add(1)
                    .expect("syntax diagnostic observation identity does not overflow"),
            )
            .expect("a positive syntax diagnostic observation identity stays nonzero");
            SyntaxDiagnosticObservationId {
                trial_id: trial.trial_id,
                frame_id,
            }
        });
        self.syntax_memo_rule_frames.push(SyntaxMemoRuleFrame {
            recovery_sensitive: false,
            rule_observation: None,
            child_rule_observation_nodes: Vec::new(),
            finalized_rule_observation_node: None,
            recovery_checkpoint_observation_range: new!(RecoveryCheckpointObservationRange {
                start: recovery_checkpoint_observation_start,
                end: recovery_checkpoint_observation_start,
            }),
            child_recovery_checkpoint_observations: Vec::new(),
            finalized_recovery_checkpoint_observations: None,
            diagnostic_observation_id,
            diagnostic_observations: Vec::new(),
            finalized_diagnostic_observations: None,
        });
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(self.syntax_memo_rule_frames.last().is_some_and(|frame| frame.recovery_sensitive))]
    pub(super) fn mark_syntax_memo_rule_recovery_sensitive(&mut self) {
        self.syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active")
            .recovery_sensitive = true;
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(ret == self.syntax_memo_rule_frames.last().is_some_and(|frame| frame.recovery_sensitive))]
    fn syntax_memo_rule_is_recovery_sensitive(&self) -> bool {
        self.syntax_memo_rule_frames
            .last()
            .expect("syntax memo rule frame is active")
            .recovery_sensitive
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(self.syntax_memo_rule_frames.len() + 1 == old(self.syntax_memo_rule_frames.len()))]
    pub(super) fn finish_syntax_memo_rule_frame(&mut self) -> bool {
        let mut frame = self
            .syntax_memo_rule_frames
            .pop()
            .expect("syntax memo rule frame is active");
        let recovery_checkpoint_observation_end = self
            .recovery_checkpoint_collection
            .as_ref()
            .map_or(0, RecoveryCheckpointCollection::observation_count);
        debug_assert!(
            frame.recovery_checkpoint_observation_range.start
                <= recovery_checkpoint_observation_end,
            "a rule's checkpoint observations form a forward range",
        );
        debug_assert!(
            frame
                .finalized_recovery_checkpoint_observations
                .as_ref()
                .is_none_or(|_| {
                    frame.recovery_checkpoint_observation_range.end
                        == recovery_checkpoint_observation_end
                }),
            "memo checkpoint observations do not change after finalization",
        );
        frame.recovery_checkpoint_observation_range = new!(RecoveryCheckpointObservationRange {
            start: frame.recovery_checkpoint_observation_range.start,
            end: recovery_checkpoint_observation_end,
        });
        debug_assert!(
            self.recovery_memo_trial.is_none()
                || frame.finalized_recovery_checkpoint_observations.is_some()
                || (frame.recovery_checkpoint_observation_range.start
                    == frame.recovery_checkpoint_observation_range.end
                    && frame.child_recovery_checkpoint_observations.is_empty()),
            "a recovered memo frame finalizes every checkpoint observation",
        );
        let rule_observation_node = if frame.recovery_sensitive {
            None
        } else {
            self.recovery_memo_trial.as_ref().and_then(|trial| {
                let store = Rc::clone(&trial.store);
                let mut store = store.borrow_mut();
                Self::finalize_syntax_rule_observation(&mut frame, &mut store)
            })
        };
        let diagnostic_observations = Self::finalize_syntax_diagnostic_observations(&mut frame);
        if let Some(observations) = &diagnostic_observations {
            self.replayed_syntax_diagnostic_observations
                .insert(observations.id);
        }
        if let Some(parent) = self.syntax_memo_rule_frames.last_mut() {
            if frame.recovery_sensitive {
                parent.recovery_sensitive = true;
            }
            if let Some(node) = rule_observation_node {
                parent.child_rule_observation_nodes.push(node);
            }
            if let Some(observations) = frame.finalized_recovery_checkpoint_observations {
                parent.child_recovery_checkpoint_observations.push(
                    ChildRecoveryCheckpointObservations {
                        range: frame.recovery_checkpoint_observation_range,
                        observations,
                    },
                );
            }
            if let Some(observations) = diagnostic_observations {
                parent
                    .diagnostic_observations
                    .push(new!(SyntaxDiagnosticObservation::Nested(observations)));
            }
        }
        frame.recovery_sensitive
    }

    #[requires(true)]
    #[ensures(!self.syntax_memo_rule_frames.is_empty() -> self.syntax_memo_rule_frames.last().is_some_and(|frame| frame.recovery_sensitive))]
    fn observe_recovery_directive_state(&mut self) {
        if !self.syntax_memo_rule_frames.is_empty() {
            self.mark_syntax_memo_rule_recovery_sensitive();
        }
    }

    #[requires(!rule.is_empty())]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    pub(super) fn observe_syntax_rule(&mut self, rule: &'static str, instance_byte_start: usize) {
        if self.recovery_memo_trial.is_none() {
            return;
        }
        let frame = self
            .syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active");
        debug_assert!(frame.rule_observation.is_none());
        frame.rule_observation = Some(new!(SyntaxRuleObservation {
            rule,
            instance_byte_start,
        }));
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    fn replay_syntax_rule_observation(&mut self, node: usize) {
        if self.recovery_memo_trial.is_none() {
            return;
        }
        self.syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active")
            .child_rule_observation_nodes
            .push(node);
    }

    #[requires(node < store.rule_observation_nodes.len())]
    #[ensures(true)]
    fn syntax_rule_observations_are_insensitive(
        &self,
        store: &SyntaxRecoveryMemoStore<'tokens>,
        node: usize,
    ) -> bool {
        if let Some(active) = &self.active_recovery_directive
            && self.syntax_rule_observation_contains(
                store,
                node,
                active.directive.rule,
                active.directive.instance_byte_start,
            )
        {
            return false;
        }
        self.syntax_rule_observation_latest_recovery_target_index(store, node)
            .is_none_or(|index| index < self.consumed_recovery_directives)
    }

    #[requires(node < store.rule_observation_nodes.len())]
    #[ensures(ret.is_none_or(|index| index < self.recovery_directives.len()))]
    fn syntax_rule_observation_latest_recovery_target_index(
        &self,
        store: &SyntaxRecoveryMemoStore<'tokens>,
        node: usize,
    ) -> Option<usize> {
        if let Some(cached) = self
            .syntax_rule_observation_latest_recovery_target_indices
            .borrow()
            .get(&node)
        {
            return *cached;
        }

        let observation = &store.rule_observation_nodes[node].observation;
        let mut latest = self
            .recovery_rule_target_last_indices
            .get(observation)
            .copied();
        for child_index in 0..store.rule_observation_nodes[node].children.len() {
            let child = store.rule_observation_nodes[node].children[child_index];
            if let Some(child_latest) =
                self.syntax_rule_observation_latest_recovery_target_index(store, child)
            {
                latest = Some(latest.map_or(child_latest, |index| index.max(child_latest)));
            }
        }
        self.syntax_rule_observation_latest_recovery_target_indices
            .borrow_mut()
            .insert(node, latest);
        latest
    }

    #[requires(start_location <= end_location)]
    #[ensures(true)]
    fn memo_range_is_reusable(&self, start_location: usize, end_location: usize) -> bool {
        let first_possible = self
            .abandoned_recovery_ranges
            .partition_point(|range| range.end_location <= start_location);
        self.abandoned_recovery_ranges
            .get(first_possible)
            .is_none_or(|range| !range.intersects(start_location, end_location))
    }

    #[requires(self.abandoned_recovery_ranges.last().is_none_or(|previous| previous.end_location <= range.start_location))]
    #[ensures(self.abandoned_recovery_ranges.last() == Some(&range))]
    fn record_abandoned_recovery_range(&mut self, range: BoundaryAbandonedRange) {
        self.abandoned_recovery_ranges.push(range);
    }

    #[requires(location < self.anchor_token_identities.len())]
    #[ensures(self.completed_recovery_boundary_location.is_some_and(|completed| completed >= location))]
    pub(super) fn record_completed_recovery_boundary(&mut self, location: usize) {
        self.completed_recovery_boundary_location = Some(
            self.completed_recovery_boundary_location
                .map_or(location, |completed| completed.max(location)),
        );
    }

    #[requires(node < store.rule_observation_nodes.len())]
    #[requires(!rule.is_empty())]
    #[ensures(true)]
    fn syntax_rule_observation_contains(
        &self,
        store: &SyntaxRecoveryMemoStore<'tokens>,
        node: usize,
        rule: &'static str,
        instance_byte_start: usize,
    ) -> bool {
        let target = new!(SyntaxRuleObservation {
            rule,
            instance_byte_start,
        });
        let mut cache = store.rule_sensitivity_cache.borrow_mut();
        let node_results = cache.entry(target).or_default();
        node_results.resize(store.rule_observation_nodes.len(), None);
        if let Some(cached) = node_results[node] {
            return cached;
        }

        for current in 0..=node {
            if node_results[current].is_some() {
                continue;
            }
            let observation_node = &store.rule_observation_nodes[current];
            debug_assert!(
                observation_node
                    .children
                    .iter()
                    .all(|child| *child < current),
                "observation children are finalized before their parent"
            );
            let own_match = observation_node.observation.rule == rule
                && observation_node.observation.instance_byte_start == instance_byte_start;
            let child_match = !own_match
                && observation_node
                    .children
                    .iter()
                    .any(|child| node_results[*child].expect("child containment was computed"));
            node_results[current] = Some(own_match || child_match);
        }
        node_results[node].expect("observation containment was computed")
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    pub(super) fn replay_syntax_diagnostic_observations(
        &mut self,
        observations: Option<&Rc<SyntaxDiagnosticObservations<'tokens>>>,
    ) {
        let Some(observations) = observations else {
            return;
        };
        self.syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active")
            .diagnostic_observations
            .push(new!(SyntaxDiagnosticObservation::Nested(Rc::clone(
                observations
            ))));

        if !self
            .replayed_syntax_diagnostic_observations
            .insert(observations.id)
        {
            return;
        }
        let mut pending = observations.observations.iter().rev().collect::<Vec<_>>();
        while let Some(observation) = pending.pop() {
            match observation.as_data() {
                data!(SyntaxDiagnosticObservation::Candidate(error)) => {
                    self.merge_diagnostic_candidate(error.clone());
                }
                data!(SyntaxDiagnosticObservation::Nested(observations)) => {
                    if self
                        .replayed_syntax_diagnostic_observations
                        .insert(observations.id)
                    {
                        pending.extend(observations.observations.iter().rev());
                    }
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none_or(|node| node < store.rule_observation_nodes.len()))]
    fn finalize_syntax_rule_observation(
        frame: &mut SyntaxMemoRuleFrame<'tokens>,
        store: &mut SyntaxRecoveryMemoStore<'tokens>,
    ) -> Option<usize> {
        if let Some(node) = frame.finalized_rule_observation_node {
            return Some(node);
        }
        let Some(observation) = frame.rule_observation.clone() else {
            debug_assert!(frame.child_rule_observation_nodes.len() <= 1);
            return frame.child_rule_observation_nodes.first().copied();
        };
        let node = store.rule_observation_nodes.len();
        store
            .rule_observation_nodes
            .push(SyntaxRuleObservationNode {
                observation,
                children: frame.child_rule_observation_nodes.clone(),
            });
        frame.finalized_rule_observation_node = Some(node);
        Some(node)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|observations| !observations.observations.is_empty()))]
    fn finalize_syntax_diagnostic_observations(
        frame: &mut SyntaxMemoRuleFrame<'tokens>,
    ) -> Option<Rc<SyntaxDiagnosticObservations<'tokens>>> {
        if let Some(observations) = &frame.finalized_diagnostic_observations {
            return Some(Rc::clone(observations));
        }
        if frame.diagnostic_observations.len() == 1
            && let data!(SyntaxDiagnosticObservation::Nested(observations)) =
                frame.diagnostic_observations[0].as_data()
        {
            return Some(Rc::clone(observations));
        }
        if frame.diagnostic_observations.is_empty() {
            return None;
        }
        let observations = Rc::new(new!(SyntaxDiagnosticObservations {
            id: frame
                .diagnostic_observation_id
                .expect("recovered memo frames have diagnostic observation identities"),
            observations: Rc::from(frame.diagnostic_observations.clone()),
        }));
        frame.finalized_diagnostic_observations = Some(Rc::clone(&observations));
        Some(observations)
    }

    #[requires(self.recovery_memo_trial.is_some())]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(ret.0.is_some() == !self.syntax_memo_rule_is_recovery_sensitive())]
    fn current_syntax_memo_observations(
        &mut self,
    ) -> (
        Option<usize>,
        Option<Rc<SyntaxDiagnosticObservations<'tokens>>>,
    ) {
        let store = Rc::clone(
            &self
                .recovery_memo_trial
                .as_ref()
                .expect("recovery memo trial is active")
                .store,
        );
        let frame = self
            .syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active");
        let rule_observation_node = if frame.recovery_sensitive {
            None
        } else {
            Some(
                Self::finalize_syntax_rule_observation(frame, &mut store.borrow_mut())
                    .expect("a freshly evaluated insensitive recovery rule records itself"),
            )
        };
        let diagnostic_observations = Self::finalize_syntax_diagnostic_observations(frame);
        if let Some(observations) = &diagnostic_observations {
            self.replayed_syntax_diagnostic_observations
                .insert(observations.id);
        }
        (rule_observation_node, diagnostic_observations)
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    fn current_syntax_memo_recovery_checkpoint_observations(
        &mut self,
    ) -> Option<Rc<SyntaxRecoveryCheckpointObservations>> {
        let collection = self.recovery_checkpoint_collection.as_mut()?;
        let observation_end = collection.observation_count();
        let frame = self
            .syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active");
        if let Some(observations) = &frame.finalized_recovery_checkpoint_observations {
            return Some(Rc::clone(observations));
        }
        let observation_start = frame.recovery_checkpoint_observation_range.start;
        debug_assert!(
            observation_start <= observation_end,
            "a rule's checkpoint observations form a forward range",
        );
        frame.recovery_checkpoint_observation_range = new!(RecoveryCheckpointObservationRange {
            start: observation_start,
            end: observation_end,
        });

        let snapshot_mark = collection.begin_snapshot();
        let mut checkpoints = Vec::new();
        let mut direct_observation_start = observation_start;
        for child in &frame.child_recovery_checkpoint_observations {
            debug_assert!(
                direct_observation_start <= child.range.start && child.range.end <= observation_end,
                "child checkpoint observation ranges are ordered within their parent",
            );
            collection.append_unique_range_to(
                direct_observation_start,
                child.range.start,
                snapshot_mark,
                &mut checkpoints,
            );
            direct_observation_start = child.range.end;
        }
        collection.append_unique_range_to(
            direct_observation_start,
            observation_end,
            snapshot_mark,
            &mut checkpoints,
        );

        let observations = if checkpoints.is_empty()
            && frame.child_recovery_checkpoint_observations.len() == 1
        {
            Some(Rc::clone(
                &frame.child_recovery_checkpoint_observations[0].observations,
            ))
        } else if checkpoints.is_empty() && frame.child_recovery_checkpoint_observations.is_empty()
        {
            None
        } else {
            Some(Rc::new(new!(SyntaxRecoveryCheckpointObservations {
                checkpoints: checkpoints.into(),
                children: frame
                    .child_recovery_checkpoint_observations
                    .iter()
                    .map(|child| Rc::clone(&child.observations))
                    .collect::<Vec<_>>()
                    .into(),
            })))
        };
        frame.finalized_recovery_checkpoint_observations = observations.as_ref().map(Rc::clone);
        observations
    }

    #[requires(!rule_name.is_empty())]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    // This method is called immediately before recursive rule descent. Keeping
    // recovery memo lookup out of the caller bounds every rule frame while
    // compiling the type-independent bookkeeping only once.
    #[inline(never)]
    pub(super) fn syntax_memo_success(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
    ) -> Option<SyntaxMemoSuccessHit<'tokens>> {
        let (memo, sensitive) = if let Some(trial) = &self.recovery_memo_trial {
            let store = trial.store.borrow();
            let insensitive = (!self.syntax_memo_rule_is_recovery_sensitive())
                .then(|| {
                    store
                        .insensitive_successes
                        .get(&(rule_name, start_location, context.scope))
                        .filter(|memo| {
                            self.memo_range_is_reusable(memo.start_location, memo.end_location)
                                && self.syntax_rule_observations_are_insensitive(
                                    &store,
                                    memo.rule_observation_node
                                        .expect("recovery memo entries record rule observations"),
                                )
                        })
                        .cloned()
                })
                .flatten();
            if let Some(memo) = insensitive {
                (memo, false)
            } else {
                let trial_id = context
                    .recovery_trial_id
                    .expect("recovered memo context has a trial identity");
                let memo = store.sensitive_successes.get(&(
                    rule_name,
                    start_location,
                    context.scope,
                    trial_id,
                    context.recovery_index,
                ))?;
                (memo.clone(), true)
            }
        } else {
            (
                self.syntax_memo
                    .get(&(rule_name, start_location, context.scope))?
                    .clone(),
                false,
            )
        };
        Some(SyntaxMemoSuccessHit { memo, sensitive })
    }

    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    #[inline(never)]
    pub(super) fn apply_syntax_memo_success(
        &mut self,
        hit: SyntaxMemoSuccessHit<'tokens>,
    ) -> SyntaxMemoReplayEffects<'tokens> {
        let SyntaxMemoSuccessHit { memo, sensitive } = hit;
        let data!(SyntaxMemoSuccess {
            end_location,
            consumed_recovery_directives,
            effective_fail_token_indices,
            side_effects,
            rule_observation_node,
            ..
        }) = memo.into_data();
        if sensitive {
            self.mark_syntax_memo_rule_recovery_sensitive();
            self.effective_fail_token_indices
                .extend_from_slice(&effective_fail_token_indices);
            self.consumed_recovery_directives = consumed_recovery_directives;
        }
        if let Some(node) = rule_observation_node {
            self.replay_syntax_rule_observation(node);
        }
        SyntaxMemoReplayEffects {
            end_location,
            side_effects,
        }
    }

    #[requires(!rule_name.is_empty())]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[ensures(true)]
    pub(super) fn syntax_memo_failure(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
    ) -> Option<SyntaxMemoFailure<'tokens>> {
        if let Some(trial) = &self.recovery_memo_trial {
            let hit = {
                let store = trial.store.borrow();
                if !self.syntax_memo_rule_is_recovery_sensitive()
                    && let Some(failure) = store
                        .insensitive_failures
                        .get(&(rule_name, start_location, context.scope))
                        .filter(|failure| {
                            self.memo_range_is_reusable(
                                failure.start_location,
                                failure.end_location,
                            ) && self.syntax_rule_observations_are_insensitive(
                                &store,
                                failure
                                    .rule_observation_node
                                    .expect("recovery memo entries record rule observations"),
                            )
                        })
                {
                    Some((failure.clone(), false))
                } else {
                    let trial_id = context
                        .recovery_trial_id
                        .expect("recovered memo context has a trial identity");
                    store
                        .sensitive_failures
                        .get(&(
                            rule_name,
                            start_location,
                            context.scope,
                            trial_id,
                            context.recovery_index,
                        ))
                        .cloned()
                        .map(|failure| (failure, true))
                }
            }?;
            let (failure, sensitive) = hit;
            if sensitive {
                self.mark_syntax_memo_rule_recovery_sensitive();
            }
            if let Some(node) = failure.rule_observation_node {
                self.replay_syntax_rule_observation(node);
            } else {
                debug_assert!(
                    sensitive,
                    "only sensitive memo entries omit rule observations"
                );
            }
            if let Some(observations) = &failure.recovery_checkpoint_observations {
                self.replay_recovery_checkpoint_observations(observations);
            }
            return Some(failure);
        }
        self.syntax_failure_memo
            .get(&(rule_name, start_location, context.scope))
            .cloned()
            .map(|error| {
                let end_location = self.memo_failure_end_location(start_location, &error);
                new!(SyntaxMemoFailure {
                    start_location,
                    end_location,
                    error,
                    recovery_checkpoint_observations: None,
                    diagnostic_observations: None,
                    rule_observation_node: None,
                })
            })
    }

    #[requires(!rule_name.is_empty())]
    #[requires(end_location >= start_location)]
    #[requires(context.recovery_index <= self.consumed_recovery_directives)]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || end_location < self.syntax_location_byte_offsets.len())]
    #[ensures(self.recovery_enabled() || self.syntax_memo.contains_key(&(rule_name, start_location, context.scope)))]
    // Do not fold recovery side-effect snapshots into the recursive wasm rule
    // wrapper; V8 reserves frame space for inlined locals across the descent.
    #[inline(never)]
    pub(super) fn store_syntax_memo_success(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
        end_location: usize,
        value: SyntaxMemoValue,
        warnings: Vec<SyntaxWarning>,
    ) {
        let sensitive = self.syntax_memo_rule_is_recovery_sensitive();
        let effective_fail_token_indices = if sensitive {
            self.effective_fail_token_indices
                [context.recovery_index..self.consumed_recovery_directives]
                .to_vec()
        } else {
            debug_assert_eq!(
                context.recovery_index, self.consumed_recovery_directives,
                "an insensitive rule cannot consume recovery directives"
            );
            Vec::new()
        };
        let (rule_observation_node, diagnostic_observations) = self
            .recovery_memo_trial
            .is_some()
            .then(|| self.current_syntax_memo_observations())
            .unwrap_or((None, None));
        let recovery_checkpoint_observations =
            self.current_syntax_memo_recovery_checkpoint_observations();
        let success = new!(SyntaxMemoSuccess {
            start_location,
            end_location,
            recovery_index: context.recovery_index,
            consumed_recovery_directives: self.consumed_recovery_directives,
            effective_fail_token_indices,
            value,
            side_effects: SyntaxMemoSideEffects {
                warnings: warnings.into(),
                recovery_checkpoint_observations,
                diagnostic_observations,
            },
            rule_observation_node,
        });
        if let Some(trial) = &self.recovery_memo_trial {
            let mut store = trial.store.borrow_mut();
            if sensitive {
                let trial_id = context
                    .recovery_trial_id
                    .expect("recovered memo context has a trial identity");
                store.sensitive_successes.insert(
                    (
                        rule_name,
                        start_location,
                        context.scope,
                        trial_id,
                        context.recovery_index,
                    ),
                    success,
                );
            } else {
                store
                    .insensitive_successes
                    .insert((rule_name, start_location, context.scope), success);
            }
        } else {
            self.syntax_memo
                .insert((rule_name, start_location, context.scope), success);
        }
    }

    #[requires(!rule_name.is_empty())]
    #[requires(!self.syntax_memo_rule_frames.is_empty())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[ensures(self.recovery_enabled() || self.syntax_failure_memo.contains_key(&(rule_name, start_location, context.scope)))]
    pub(super) fn store_syntax_memo_failure(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
        error: SyntaxParseError<'tokens>,
    ) {
        let sensitive = self.syntax_memo_rule_is_recovery_sensitive();
        let observations = self
            .recovery_memo_trial
            .is_some()
            .then(|| self.current_syntax_memo_observations());
        let recovery_checkpoint_observations = if self.recovery_memo_trial.is_some() {
            self.current_syntax_memo_recovery_checkpoint_observations()
        } else {
            None
        };
        if let Some(trial) = &self.recovery_memo_trial {
            let (rule_observation_node, diagnostic_observations) =
                observations.expect("recovery memo observations were finalized");
            let end_location = self.memo_failure_end_location(start_location, &error);
            let failure = new!(SyntaxMemoFailure {
                start_location,
                end_location,
                error,
                recovery_checkpoint_observations,
                diagnostic_observations,
                rule_observation_node,
            });
            let mut store = trial.store.borrow_mut();
            if sensitive {
                let trial_id = context
                    .recovery_trial_id
                    .expect("recovered memo context has a trial identity");
                store.sensitive_failures.insert(
                    (
                        rule_name,
                        start_location,
                        context.scope,
                        trial_id,
                        context.recovery_index,
                    ),
                    failure,
                );
            } else {
                store
                    .insensitive_failures
                    .insert((rule_name, start_location, context.scope), failure);
            }
            return;
        }
        self.syntax_failure_memo
            .insert((rule_name, start_location, context.scope), error);
    }

    #[requires(!rule_name.is_empty())]
    #[requires(self.syntax_location_byte_offsets.is_empty() || start_location < self.syntax_location_byte_offsets.len())]
    #[ensures(ret && !self.recovery_enabled() -> self.syntax_memo_in_progress.contains(&(rule_name, start_location, context.scope)))]
    #[ensures(ret && self.recovery_enabled() -> self.syntax_recovery_memo_in_progress.contains(&(rule_name, start_location, context.scope, context.recovery_index)))]
    // A completion deadline must be observable inside a single recovery
    // trial. Keep the query in this existing non-generic descent boundary so
    // ordinary generated rule frames retain their original stack shape.
    #[inline(never)]
    pub(super) fn enter_syntax_memo_rule(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
    ) -> bool {
        if self
            .continuation_time_limit
            .is_some_and(ContinuationTimeLimit::exhausted)
        {
            return false;
        }
        let entered = if self.recovery_enabled() {
            self.syntax_recovery_memo_in_progress.insert((
                rule_name,
                start_location,
                context.scope,
                context.recovery_index,
            ))
        } else {
            self.syntax_memo_in_progress
                .insert((rule_name, start_location, context.scope))
        };
        entered
    }

    #[requires(!rule_name.is_empty())]
    #[ensures(!self.syntax_memo_in_progress.contains(&(rule_name, start_location, context.scope)))]
    #[ensures(!self.syntax_recovery_memo_in_progress.contains(&(rule_name, start_location, context.scope, context.recovery_index)))]
    pub(super) fn exit_syntax_memo_rule(
        &mut self,
        rule_name: &'static str,
        start_location: usize,
        context: SyntaxMemoContext,
    ) {
        if self.recovery_enabled() {
            self.syntax_recovery_memo_in_progress.remove(&(
                rule_name,
                start_location,
                context.scope,
                context.recovery_index,
            ));
        } else {
            self.syntax_memo_in_progress
                .remove(&(rule_name, start_location, context.scope));
        }
    }

    #[requires(true)]
    #[ensures(self.warnings.len() == old(self.warnings.len()) + side_effects.warnings.len())]
    pub(super) fn replay_syntax_memo_side_effects(
        &mut self,
        side_effects: &SyntaxMemoSideEffects<'tokens>,
    ) {
        self.warnings.extend_from_slice(&side_effects.warnings);
        if let Some(observations) = &side_effects.recovery_checkpoint_observations {
            self.replay_recovery_checkpoint_observations(observations);
        }
        self.replay_syntax_diagnostic_observations(side_effects.diagnostic_observations.as_ref());
    }

    #[requires(true)]
    #[ensures(true)]
    fn replay_recovery_checkpoint_observations(
        &mut self,
        observations: &Rc<SyntaxRecoveryCheckpointObservations>,
    ) {
        let collection = self
            .recovery_checkpoint_collection
            .as_mut()
            .expect("recovered memo trials collect checkpoints");
        collection.register_observation_node(observations);
        let observation_end = collection.observation_count();
        let frame = self
            .syntax_memo_rule_frames
            .last_mut()
            .expect("syntax memo rule frame is active");
        debug_assert_eq!(
            frame.recovery_checkpoint_observation_range.start, observation_end,
            "memo replay precedes fresh checkpoint observations",
        );
        frame.recovery_checkpoint_observation_range = new!(RecoveryCheckpointObservationRange {
            start: observation_end,
            end: observation_end,
        });
        frame.finalized_recovery_checkpoint_observations = Some(Rc::clone(observations));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn record_diagnostic_candidate(&mut self, error: SyntaxParseError<'tokens>) {
        let error = error
            .with_active_contexts(self.active_syntax_context_stack.clone())
            .with_active_rule_contexts(self.active_syntax_rule_stack.clone());
        if self.diagnostic_candidate_is_at_continuation_sentinel(&error)
            && !self
                .continuation_diagnostic_candidates
                .iter()
                .any(|candidate| candidate.same_report_content(&error))
        {
            self.continuation_diagnostic_candidates.push(error.clone());
        }
        if self.recovery_memo_trial.is_none() {
            self.merge_strict_diagnostic_candidate(error);
            return;
        }
        if let Some(frame) = self.syntax_memo_rule_frames.last_mut() {
            frame
                .diagnostic_observations
                .push(new!(SyntaxDiagnosticObservation::Candidate(error.clone())));
        }
        self.merge_diagnostic_candidate(error);
    }

    #[requires(self.recovery_memo_trial.is_none())]
    #[ensures(true)]
    fn merge_strict_diagnostic_candidate(&mut self, error: SyntaxParseError<'tokens>) {
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

    #[requires(self.recovery_memo_trial.is_some())]
    #[ensures(true)]
    fn merge_diagnostic_candidate(&mut self, error: SyntaxParseError<'tokens>) {
        let Some(farthest_start) = self
            .diagnostic_candidates
            .first()
            .map(|candidate| candidate.span().start)
        else {
            self.push_diagnostic_candidate(error);
            return;
        };
        match error.span().start.cmp(&farthest_start) {
            std::cmp::Ordering::Greater => {
                self.diagnostic_candidates.clear();
                self.diagnostic_candidate_hash_buckets.clear();
                self.push_diagnostic_candidate(error);
            }
            std::cmp::Ordering::Equal => {
                let hash = error.report_content_hash_for_dedup();
                let duplicate = hash.is_some_and(|hash| {
                    self.diagnostic_candidate_hash_buckets
                        .get(&hash)
                        .is_some_and(|indices| {
                            indices.iter().any(|index| {
                                self.diagnostic_candidates[*index].same_report_content(&error)
                            })
                        })
                });
                if !duplicate {
                    self.push_diagnostic_candidate(error);
                }
            }
            std::cmp::Ordering::Less => {}
        }
    }

    #[requires(self.recovery_memo_trial.is_some())]
    #[ensures(self.diagnostic_candidates.len() == old(self.diagnostic_candidates.len()) + 1)]
    fn push_diagnostic_candidate(&mut self, error: SyntaxParseError<'tokens>) {
        let hash = error.report_content_hash_for_dedup();
        let index = self.diagnostic_candidates.len();
        self.diagnostic_candidates.push(error);
        if let Some(hash) = hash {
            self.diagnostic_candidate_hash_buckets
                .entry(hash)
                .or_default()
                .push(index);
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == self.diagnostic_candidates.len())]
    pub(super) fn diagnostic_candidates_snapshot(&self) -> Vec<SyntaxParseError<'tokens>> {
        self.diagnostic_candidates.clone()
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
    pub(super) fn continuation_expectations(&self) -> Vec<SyntaxExpectation> {
        self.continuation_diagnostic_candidates
            .iter()
            .cloned()
            .flat_map(|error| error.into_report_error().expectations())
            .collect()
    }

    #[requires(start_location <= failure_location)]
    #[ensures(self.continuation_sentinel_index.is_none() -> self.continuation_diagnostic_candidates.is_empty())]
    pub(super) fn record_continuation_rule_failure(
        &mut self,
        start_location: usize,
        failure_location: usize,
        error: &SyntaxParseError<'tokens>,
    ) {
        if !self.is_continuation_sentinel_location(failure_location)
            || !self
                .continuation_rule_or_enclosing_context_progressed(start_location, failure_location)
            || self
                .continuation_diagnostic_candidates
                .iter()
                .any(|candidate| candidate.same_report_content(&error))
        {
            return;
        }
        self.continuation_diagnostic_candidates.push(error.clone());
    }

    #[requires(start_location <= failure_location)]
    #[ensures(ret -> self.continuation_sentinel_index == Some(failure_location))]
    fn continuation_rule_or_enclosing_context_progressed(
        &self,
        start_location: usize,
        failure_location: usize,
    ) -> bool {
        if failure_location > start_location {
            return true;
        }
        let Some(sentinel_index) = self
            .continuation_sentinel_index
            .filter(|index| *index == failure_location)
        else {
            return false;
        };
        // At an empty document the grammar cannot consume a token before the
        // sentinel, but descending from the root into a start construct is
        // still meaningful grammar progress. Retain those rule failures so
        // expected continuations cover ordinary statement starts such as a
        // brivla, not only failures recorded directly by terminal parsers.
        if sentinel_index == 0 {
            return true;
        }
        let Some(cut_byte) = self.syntax_location_byte_offsets.get(sentinel_index) else {
            return false;
        };
        self.active_syntax_contexts
            .iter()
            .any(|context| context.byte_start() < *cut_byte)
    }

    #[requires(true)]
    #[ensures(ret -> self.continuation_sentinel_index.is_some())]
    fn diagnostic_candidate_is_at_continuation_sentinel(
        &self,
        error: &SyntaxParseError<'_>,
    ) -> bool {
        self.continuation_sentinel_index
            .and_then(|index| self.syntax_location_byte_offsets.get(index))
            .is_some_and(|byte_offset| error.span().start == *byte_offset)
    }

    #[requires(true)]
    #[ensures(self.diagnostic_candidates.len() == old(snapshot.len()))]
    pub(super) fn restore_diagnostic_candidates(
        &mut self,
        snapshot: Vec<SyntaxParseError<'tokens>>,
    ) {
        self.diagnostic_candidates = snapshot;
        self.diagnostic_candidate_hash_buckets.clear();
        if self.recovery_memo_trial.is_some() {
            for (index, candidate) in self.diagnostic_candidates.iter().enumerate() {
                if let Some(hash) = candidate.report_content_hash_for_dedup() {
                    self.diagnostic_candidate_hash_buckets
                        .entry(hash)
                        .or_default()
                        .push(index);
                }
            }
        }
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
        self.restore_diagnostic_candidates(snapshot);
        for candidate in preserved {
            self.record_diagnostic_candidate(candidate);
        }
    }

    #[requires(!construct.is_empty())]
    #[ensures(self.active_syntax_contexts.len() == old(self.active_syntax_contexts.len()) + 1)]
    pub(super) fn push_syntax_context(&mut self, construct: &'static str, byte_start: usize) {
        let frame = SyntaxContextFrame::new(construct, byte_start);
        self.active_syntax_context_stack = self.active_syntax_context_stack.pushed(frame.clone());
        self.active_syntax_contexts.push(frame);
    }

    #[requires(!self.active_syntax_contexts.is_empty())]
    #[ensures(self.active_syntax_contexts.len() + 1 == old(self.active_syntax_contexts.len()))]
    pub(super) fn pop_syntax_context(&mut self) {
        self.active_syntax_contexts
            .pop()
            .expect("syntax context stack is non-empty");
        self.active_syntax_context_stack = self.active_syntax_context_stack.popped();
    }

    #[requires(!rule.is_empty())]
    #[ensures(self.active_syntax_rules.len() == old(self.active_syntax_rules.len()) + 1)]
    pub(super) fn push_syntax_rule(&mut self, rule: &'static str, byte_start: usize) {
        let recovery_enabled = self.recovery_rule_parser_enabled(rule, byte_start);
        let frame = SyntaxRuleFrame::new(rule, byte_start, recovery_enabled);
        self.active_syntax_rule_stack = self.active_syntax_rule_stack.pushed(frame.clone());
        self.active_syntax_rules.push(frame);
    }

    #[requires(!self.active_syntax_rules.is_empty())]
    #[ensures(self.active_syntax_rules.len() + 1 == old(self.active_syntax_rules.len()))]
    pub(super) fn pop_syntax_rule(&mut self) {
        self.active_syntax_rules
            .pop()
            .expect("syntax rule stack is non-empty");
        self.active_syntax_rule_stack = self.active_syntax_rule_stack.popped();
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
    #[ensures(ret < self.syntax_location_byte_offsets.len().max(1))]
    fn location_for_byte_offset(&self, byte_offset: usize) -> usize {
        self.syntax_location_byte_offsets
            .partition_point(|offset| *offset < byte_offset)
            .min(self.syntax_location_byte_offsets.len().saturating_sub(1))
    }

    #[requires(true)]
    #[ensures(ret >= start_location)]
    fn memo_failure_end_location(
        &self,
        start_location: usize,
        error: &SyntaxParseError<'_>,
    ) -> usize {
        self.location_for_byte_offset(error.span().start.max(error.span().end))
            .max(start_location)
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

    #[requires(true)]
    #[ensures(ret.len() == self.active_syntax_contexts.len())]
    pub(super) fn active_syntax_context_stack(&self) -> SharedStack<SyntaxContextFrame> {
        self.active_syntax_context_stack.clone()
    }

    #[requires(true)]
    #[ensures(ret.len() == self.active_syntax_rules.len())]
    pub(super) fn active_syntax_rule_stack(&self) -> SharedStack<SyntaxRuleFrame> {
        self.active_syntax_rule_stack.clone()
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
    #[ensures(ret -> self.active_recovery_directive.is_none())]
    pub(super) fn boundary_resync_catches_field_failure(
        &self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        field_start_location: usize,
    ) -> bool {
        if self.active_recovery_directive.is_some() {
            return false;
        }
        self.recovery_directives
            .get(self.consumed_recovery_directives)
            .is_some_and(|directive| {
                directive.kind == RecoveryDirectiveKind::BoundaryResync
                    && directive.rule == rule
                    && directive.instance_byte_start == instance_byte_start
                    && field_index <= directive.resume_field
                    && directive
                        .boundary_unwind_start_token_index
                        .is_some_and(|unwind_start| unwind_start <= field_start_location)
                    && field_start_location < directive.resume_token_index
                    && field_start_location <= directive.fail_token_index
            })
    }

    #[requires(!rule.is_empty())]
    #[requires(self.boundary_resync_catches_field_failure(
        rule,
        instance_byte_start,
        field_index,
        field_start_location,
    ))]
    #[ensures(matches!(
        ret.kind,
        RecoveryFieldActionKind::BoundaryResync | RecoveryFieldActionKind::Resume
    ))]
    pub(super) fn boundary_resync_field_action_after_failure(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        field_index: usize,
        field_start_location: usize,
    ) -> RecoveryFieldAction {
        self.observe_recovery_directive_state();
        let directive = self.recovery_directives[self.consumed_recovery_directives].clone();
        self.effective_fail_token_indices.push(field_start_location);
        self.consumed_recovery_directives += 1;
        self.active_recovery_directive = Some(new!(ActiveRecoveryDirective {
            directive: directive.clone(),
            effective_fail_token_index: field_start_location,
            skipped_item_emitted: true,
        }));
        self.record_abandoned_recovery_range(BoundaryAbandonedRange::new(
            field_start_location,
            directive.resume_token_index,
        ));
        let item = self.recovery_item_for_directive(&directive, field_start_location);
        if field_index < directive.resume_field {
            RecoveryFieldAction::boundary_resync(item, directive.resume_token_index)
        } else {
            self.active_recovery_directive = None;
            RecoveryFieldAction::resume(Some(item), directive.resume_token_index)
        }
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
        self.observe_recovery_directive_state();
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
        self.observe_recovery_directive_state();
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
    #[requires(effective_fail_token_index <= directive.resume_token_index)]
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
        if self
            .active_recovery_directive
            .as_ref()
            .is_some_and(|active| active.directive.kind == RecoveryDirectiveKind::BoundaryResync)
        {
            self.active_recovery_directive = None;
            self.consumed_recovery_directives = self
                .consumed_recovery_directives
                .checked_sub(1)
                .expect("an active boundary directive has fired");
            self.effective_fail_token_indices
                .pop()
                .expect("a fired boundary directive records its effective failure");
        }
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
        let recovery_checkpoints = self
            .recovery_checkpoint_collection
            .take()
            .map_or_else(Vec::new, RecoveryCheckpointCollection::into_checkpoints);
        ParserStateFinish {
            warnings: deduped,
            trace,
            unconsumed_recovery_directives,
            recovery_directives: self.recovery_directives,
            effective_fail_token_indices,
            completed_recovery_boundary_location: self.completed_recovery_boundary_location,
            recovery_checkpoints,
        }
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    pub(super) fn recovery_rule_instance_byte_start(
        &self,
        rule: &'static str,
        token_index: usize,
    ) -> usize {
        self.active_syntax_rules
            .iter()
            .rev()
            .find(|frame| frame.rule() == rule)
            .map_or_else(
                || self.byte_offset_for_location(token_index),
                SyntaxRuleFrame::byte_start,
            )
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    pub(super) fn record_recovery_checkpoint(
        &mut self,
        rule: &'static str,
        instance_byte_start: usize,
        token_index: usize,
        field_index: usize,
        kind: RecoveryCheckpointKind,
    ) {
        let active_frame_start = self
            .syntax_memo_rule_frames
            .last()
            .map(|frame| frame.recovery_checkpoint_observation_range.start);
        if let Some(collection) = &mut self.recovery_checkpoint_collection {
            // Interning keeps the flat observation log compact while still
            // retaining an observation in every dynamic subtree that needs
            // one. A memo entry must replay a checkpoint even when an
            // ancestor or an earlier trial already observed the same site.
            // Rule frames capture ranges of typed IDs, so recording never
            // propagates checkpoint data through ancestors.
            collection.record(
                RecoveryCheckpoint::new(rule, instance_byte_start, token_index, field_index, kind),
                active_frame_start,
            );
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
    #[ensures(ret < self.anchor_token_identities.len() || self.anchor_token_identities.is_empty())]
    fn anchor_index(&self, anchor: &Token) -> usize {
        let identity = anchor.identity();
        if let Some(index) = self
            .anchor_token_identities
            .iter()
            .position(|candidate| candidate == &identity)
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
                    abandoned_range_count: self.abandoned_recovery_ranges.len(),
                    completed_recovery_boundary_location: self.completed_recovery_boundary_location,
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
        self.active_syntax_context_stack
            .truncate(marker.inspector().syntax_context_count);
        self.active_syntax_rule_stack
            .truncate(marker.inspector().syntax_rule_count);
        if let Some(recovery) = &marker.inspector().recovery {
            self.consumed_recovery_directives = recovery.consumed_recovery_directives;
            self.effective_fail_token_indices
                .truncate(self.consumed_recovery_directives);
            self.active_recovery_directive = recovery.active_recovery_directive.clone();
            self.abandoned_recovery_ranges
                .truncate(recovery.abandoned_range_count);
            self.completed_recovery_boundary_location =
                recovery.completed_recovery_boundary_location;
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_word_label(token: &Token) -> String {
    token.core_word().to_string()
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
        add_generated_construct_warnings(
            &parsed.text,
            &tokens,
            options.dialect.features.contains(&DialectFeature::Cbm),
            &mut warnings,
        );
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
#[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
pub(crate) fn expected_continuations(
    words: &[WordLike],
    options: &ParseOptions,
    time_limit: Option<Duration>,
) -> Vec<SyntaxExpectation> {
    if time_limit.is_some_and(|duration| duration.is_zero()) {
        return Vec::new();
    }
    let time_limit = time_limit.map(ContinuationTimeLimit::new);
    if time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
        return Vec::new();
    }
    let cut_byte = words
        .last()
        .and_then(WordLike::byte_range)
        .map_or(0, |range| range.end);
    let mut bare_tokens = words.iter().cloned().map(Token::bare).collect::<Vec<_>>();
    bare_tokens.push(expected_continuation_sentinel(cut_byte));
    // The sentinel stands for the word that would follow this prefix, so it
    // must participate in the same modifier preparation as a real word.
    // In particular, trailing BAhE decorates the sentinel without changing
    // which grammar terminals are tested at the cut.
    let mut tokens = prepare_syntax_tokens(bare_tokens, options);
    let sentinel_index = tokens.len() - 1;
    // BAhE has now been consumed as the sentinel's prefix. Its source span
    // must not widen the synthetic zero-width parser token back over the
    // modifier, and CLL 19.11 guarantees that BAhE does not change its target
    // word's grammar, so the parser-facing sentinel can remain bare.
    tokens[sentinel_index] = Token::bare(tokens[sentinel_index].core_word().clone());

    let tracked_attempt =
        generated::generated_model::parse_text_detailed_tracked_attempt_for_expected_continuations(
            &tokens,
            options,
            sentinel_index,
            time_limit,
        );
    let data!(generated::generated_model::GeneratedParsedTextDetailedAttempt {
        result,
        trace,
        mut continuation_expectations,
    }) = tracked_attempt.into_data();
    if time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
        return Vec::new();
    }
    let failure = match result {
        Ok(_) => unreachable!("the completion sentinel cannot satisfy the root EOF parser"),
        Err(failure) => failure,
    };
    if syntax_error_start(&failure.public_error) == cut_byte {
        continuation_expectations.extend(syntax_error_expectations(&failure.public_error));
        return continuation_expectations;
    }
    let recovered = recover_after_strict_failure(
        tokens,
        None,
        options,
        failure,
        trace,
        Some(sentinel_index),
        time_limit,
    );
    let data!(RecoveryParseOutcome {
        recovered: _,
        continuation_expectations,
        continuation_cut_reached: _,
        continuation_time_limit_exhausted,
    }) = recovered.into_data();
    if continuation_time_limit_exhausted {
        Vec::new()
    } else {
        continuation_expectations
    }
}

#[requires(true)]
#[ensures(ret.core_word().byte_range().is_some_and(|range| range.start == byte_offset && range.end == byte_offset))]
fn expected_continuation_sentinel(byte_offset: usize) -> Token {
    let span = SourceSpan::new(None, byte_offset, byte_offset, 0, 1)
        .expect("the completion sentinel has ordered byte and character spans");
    let phonemes = Phonemes::from_canonical(Cmavo::Faho.canonical_text().to_owned())
        .expect("the canonical FAhO spelling is valid phoneme text");
    Token::bare(WordLike::bare(Word::from_kind(
        WordKind::Cmavo,
        phonemes,
        span,
    )))
}

#[requires(true)]
#[ensures(ret.iter().all(|expectation| !expectation.tokens.is_empty()))]
fn syntax_error_expectations(error: &SyntaxError) -> Vec<SyntaxExpectation> {
    match error {
        SyntaxError::Parse { expectations, .. } => expectations.clone(),
        SyntaxError::NotImplemented => Vec::new(),
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
    parse_generated_model_syntax_tokens_with_recovery_attempt(tokens, source, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_generated_model_syntax_tokens_with_recovery_attempt(
    tokens: Vec<Token>,
    source: Option<&str>,
    options: &ParseOptions,
) -> SyntaxRecoveryParseAttempt {
    let strict_attempt = generated::generated_model::parse_text_attempt(&tokens, options);
    if let Ok(parsed) = strict_attempt.result {
        return valid_syntax_recovery_attempt(parsed, &tokens, options, strict_attempt.trace);
    }

    let tracked_attempt =
        generated::generated_model::parse_text_detailed_tracked_attempt(&tokens, options);
    let data!(
        generated::generated_model::GeneratedParsedTextDetailedAttempt {
            result,
            trace,
            continuation_expectations: _,
        }
    ) = tracked_attempt.into_data();
    let failure = match result {
        Ok(parsed) => {
            return valid_syntax_recovery_attempt(parsed, &tokens, options, trace);
        }
        Err(failure) => failure,
    };

    let recovered =
        recover_after_strict_failure(tokens, source, options, failure, trace, None, None);
    let data!(RecoveryParseOutcome {
        recovered,
        continuation_expectations: _,
        continuation_cut_reached: _,
        continuation_time_limit_exhausted: _,
    }) = recovered.into_data();
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
    options: &ParseOptions,
    trace: Option<TraceReport>,
) -> SyntaxRecoveryParseAttempt {
    let mut warnings = parsed.warnings;
    add_generated_construct_warnings(
        &parsed.text,
        tokens,
        options.dialect.features.contains(&DialectFeature::Cbm),
        &mut warnings,
    );
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

#[requires(continuation_sentinel_index.is_none_or(|index| index < tokens.len()))]
#[ensures(!ret.recovered.result.errors.is_empty())]
#[ensures(ret.continuation_expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
fn recover_after_strict_failure(
    tokens: Vec<Token>,
    source: Option<&str>,
    options: &ParseOptions,
    mut failure: generated::generated_model::GeneratedParseFailure,
    mut trace: Option<TraceReport>,
    continuation_sentinel_index: Option<usize>,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> RecoveryParseOutcome {
    let global_hard_cap = options.recovery_error_policy.global_hard_cap().get();
    let per_statement_cap = options.recovery_error_policy.per_statement().get();
    let parser_tokens = tokens::spanned_tokens(&tokens);
    let recovery_token_scan = RecoveryTokenScan::new(&tokens);
    // Recovery selection depends only on parse results. Keep its memo session
    // free of completion-specific observation state so recovery-insensitive
    // entries can be shared across every trial. Once one directive chain
    // reaches the requested cut, replay only that chain in a fresh session to
    // capture its expectations in isolation.
    let mut recovery_session =
        generated::generated_model::GeneratedRecoveryParseSession::new_with_continuation_time_limit(
            continuation_time_limit,
        );
    let mut errors = vec![failure.public_error.clone()];
    let mut directives = Vec::new();
    let mut continuation_expectations = Vec::new();
    let mut continuation_cut_reached = false;
    let mut continuation_time_limit_exhausted = false;
    let mut errors_in_statement = 1usize;
    let reachability_filter_enabled =
        recovery_reachability_filter_enabled(options, continuation_time_limit);

    'recovery_errors: while errors.len() < global_hard_cap {
        if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
            continuation_time_limit_exhausted = true;
            break;
        }
        let mut candidates = select_recovery_directives(
            &tokens,
            &recovery_token_scan,
            &failure,
            options,
            errors.len() - 1,
        );
        let local_cap_exhausted = errors_in_statement >= per_statement_cap;
        if local_cap_exhausted {
            let local_candidates = candidates
                .iter()
                .filter(|directive| directive.kind == RecoveryDirectiveKind::Local)
                .count();
            record_recovery_reachability_telemetry(
                RecoveryDirectiveKind::Local,
                RecoveryReachabilityTelemetryEvent::CapRetainedAway,
                local_candidates,
            );
            candidates.retain(|directive| directive.kind == RecoveryDirectiveKind::BoundaryResync);
        }
        let candidates = if candidates.is_empty() {
            if local_cap_exhausted {
                Vec::new()
            } else {
                select_final_recovery_directives(&tokens, &failure, errors.len() - 1)
            }
        } else {
            candidates
        };
        if candidates.is_empty() {
            break;
        }

        let mut accepted_progress = None;
        let mut exact_position_success = None;
        let mut rejected_exact_sites = Vec::new();
        'recovery_phases: for natural_stop_enabled in [false, true] {
            let trial_limit = if natural_stop_enabled {
                MAX_NATURAL_STOP_DIRECTIVE_TRIALS_PER_ERROR
            } else {
                LEGACY_RECOVERY_DIRECTIVE_TRIALS_PER_ERROR
            };
            let mut trial_count = 0usize;
            for directive in candidates.iter().cloned() {
                if directives.iter().any(|existing| {
                    same_recovery_site(existing, &directive)
                        || same_boundary_resync_group(existing, &directive)
                }) {
                    continue;
                }
                if trial_count >= trial_limit {
                    break;
                }
                trial_count += 1;
                if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
                    continuation_time_limit_exhausted = true;
                    break 'recovery_errors;
                }
                let directive = if natural_stop_enabled {
                    directive.with_natural_stop_enabled()
                } else {
                    directive
                };
                let mut trial_directives = directives.clone();
                trial_directives.push(directive.clone());

                if !natural_stop_enabled {
                    record_recovery_reachability_telemetry(
                        directive.kind,
                        RecoveryReachabilityTelemetryEvent::ExactConsidered,
                        1,
                    );
                }
                let paired_rejected_exact = natural_stop_enabled
                    && rejected_exact_sites
                        .iter()
                        .any(|exact| same_recovery_site(exact, &directive));
                if !natural_stop_enabled
                    && reachability_filter_enabled
                    && !exact_trial_reachable(&failure, &directive)
                {
                    record_recovery_reachability_telemetry(
                        directive.kind,
                        RecoveryReachabilityTelemetryEvent::ExactSkipped,
                        1,
                    );
                    #[cfg(feature = "expensive_contracts")]
                    {
                        let verification = run_recovery_trial(
                            &tokens,
                            &parser_tokens,
                            source,
                            options,
                            &trial_directives,
                            &directive,
                            &mut recovery_session,
                            errors.len(),
                            global_hard_cap,
                            errors.last().map_or(0, syntax_error_start),
                        );
                        if matches!(verification, RecoveryTrialClassification::Rejected { .. }) {
                            record_recovery_reachability_telemetry(
                                directive.kind,
                                RecoveryReachabilityTelemetryEvent::SkipVerifiedRejected,
                                1,
                            );
                        } else {
                            record_recovery_reachability_telemetry(
                                directive.kind,
                                RecoveryReachabilityTelemetryEvent::SkipFalsePositive,
                                1,
                            );
                            let related_checkpoints = failure
                                .checkpoints
                                .iter()
                                .filter(|checkpoint| checkpoint.rule == directive.rule)
                                .collect::<Vec<_>>();
                            panic!(
                                "exact-site reachability skipped a trial accepted by the #533 classifier: {directive:?}; same-rule checkpoints: {related_checkpoints:?}"
                            );
                        }
                    }
                    rejected_exact_sites.push(directive);
                    continue;
                }
                if !natural_stop_enabled {
                    record_recovery_reachability_telemetry(
                        directive.kind,
                        RecoveryReachabilityTelemetryEvent::ExactRun,
                        1,
                    );
                }
                let classification = run_recovery_trial(
                    &tokens,
                    &parser_tokens,
                    source,
                    options,
                    &trial_directives,
                    &directive,
                    &mut recovery_session,
                    errors.len(),
                    global_hard_cap,
                    errors.last().map_or(0, syntax_error_start),
                );
                if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
                    continuation_time_limit_exhausted = true;
                    break 'recovery_errors;
                }
                match classification {
                    RecoveryTrialClassification::AcceptedSuccess {
                        trial,
                        fired_left_of_declared_failure,
                    } => {
                        if natural_stop_enabled {
                            if paired_rejected_exact {
                                record_recovery_reachability_telemetry(
                                    directive.kind,
                                    RecoveryReachabilityTelemetryEvent::NaturalWins,
                                    1,
                                );
                            }
                        } else {
                            record_recovery_reachability_telemetry(
                                directive.kind,
                                RecoveryReachabilityTelemetryEvent::ExactWins,
                                1,
                            );
                        }
                        let data!(RecoverySuccessTrial {
                            parsed,
                            trace: attempt_trace,
                            directives: applied_directives,
                            effective_fail_token_indices: applied_effective_fail_token_indices,
                        }) = trial.into_data();
                        if !natural_stop_enabled || fired_left_of_declared_failure {
                            let winning_expectations =
                                if let Some(sentinel_index) = continuation_sentinel_index {
                                    let expectations =
                                        replay_winning_continuation_success_expectations(
                                            &tokens,
                                            &parser_tokens,
                                            source,
                                            options,
                                            &applied_directives,
                                            0,
                                            &applied_effective_fail_token_indices,
                                            sentinel_index,
                                            continuation_time_limit,
                                        )
                                        .unwrap_or_default();
                                    if continuation_time_limit
                                        .is_some_and(ContinuationTimeLimit::exhausted)
                                    {
                                        continuation_time_limit_exhausted = true;
                                        break 'recovery_errors;
                                    }
                                    expectations
                                } else {
                                    continuation_expectations
                                };
                            recovery_session.clear_memo();
                            return recovered_success(
                                parsed,
                                &errors,
                                attempt_trace,
                                winning_expectations,
                                continuation_sentinel_index.is_some(),
                            );
                        }
                        if !directives.is_empty() && exact_position_success.is_none() {
                            exact_position_success = Some(new!(RecoverySuccessTrial {
                                parsed,
                                trace: attempt_trace,
                                directives: applied_directives,
                                effective_fail_token_indices: applied_effective_fail_token_indices,
                            }));
                        }
                    }
                    RecoveryTrialClassification::AcceptedProgress { trial } => {
                        if natural_stop_enabled {
                            if paired_rejected_exact {
                                record_recovery_reachability_telemetry(
                                    directive.kind,
                                    RecoveryReachabilityTelemetryEvent::NaturalWins,
                                    1,
                                );
                            }
                        } else {
                            record_recovery_reachability_telemetry(
                                directive.kind,
                                RecoveryReachabilityTelemetryEvent::ExactWins,
                                1,
                            );
                        }
                        if accepted_progress.is_none() {
                            accepted_progress = Some(trial);
                        }
                        if !natural_stop_enabled {
                            break 'recovery_phases;
                        }
                    }
                    RecoveryTrialClassification::Rejected {
                        trace: attempt_trace,
                    } => {
                        if natural_stop_enabled {
                            if paired_rejected_exact {
                                record_recovery_reachability_telemetry(
                                    directive.kind,
                                    RecoveryReachabilityTelemetryEvent::BothFail,
                                    1,
                                );
                            }
                        } else {
                            record_recovery_reachability_telemetry(
                                directive.kind,
                                RecoveryReachabilityTelemetryEvent::ExactRunRejected,
                                1,
                            );
                            rejected_exact_sites.push(directive);
                        }
                        trace = attempt_trace;
                    }
                }
            }
        }

        // The wider phase preserves natural-stop recoveries as its first
        // priority. After prior progress, if none fires left of the next
        // declared failure, a late exact-site success is still a complete
        // recovery and is preferable to degrading the entire parse.
        if let Some(success) = exact_position_success {
            let data!(RecoverySuccessTrial {
                parsed,
                trace: attempt_trace,
                directives: success_directives,
                effective_fail_token_indices,
            }) = success.into_data();
            let winning_expectations = if let Some(sentinel_index) = continuation_sentinel_index {
                let expectations = replay_winning_continuation_success_expectations(
                    &tokens,
                    &parser_tokens,
                    source,
                    options,
                    &success_directives,
                    0,
                    &effective_fail_token_indices,
                    sentinel_index,
                    continuation_time_limit,
                )
                .unwrap_or_default();
                if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
                    continuation_time_limit_exhausted = true;
                    break 'recovery_errors;
                }
                expectations
            } else {
                continuation_expectations
            };
            recovery_session.clear_memo();
            return recovered_success(
                parsed,
                &errors,
                attempt_trace,
                winning_expectations,
                continuation_sentinel_index.is_some(),
            );
        }

        let Some(progress) = accepted_progress else {
            break;
        };
        let data!(RecoveryProgressTrial {
            directives: trial_directives,
            failure: next_failure,
            trace: progress_trace,
            effective_fail_token_indices,
            completed_recovery_boundary_location,
        }) = progress.into_data();
        let trial_reached_continuation_cut =
            continuation_sentinel_index.is_some_and(|sentinel_index| {
                syntax_error_start(&next_failure.public_error)
                    == recovery_byte_at(&tokens, sentinel_index)
            });
        if let Some(sentinel_index) =
            continuation_sentinel_index.filter(|_| trial_reached_continuation_cut)
        {
            let replayed_expectations = replay_winning_continuation_expectations(
                &tokens,
                &parser_tokens,
                source,
                options,
                &trial_directives,
                0,
                &effective_fail_token_indices,
                &next_failure,
                sentinel_index,
                continuation_time_limit,
            );
            continuation_expectations = replayed_expectations
                .unwrap_or_else(|| syntax_error_expectations(&next_failure.public_error));
            if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
                continuation_time_limit_exhausted = true;
                break 'recovery_errors;
            }
        }
        directives = trial_directives;
        errors.push(next_failure.public_error.clone());
        let previous_failure_token_index =
            token_index_for_byte_start(&tokens, syntax_error_start(&failure.public_error));
        let crossed_completed_boundary = completed_recovery_boundary_location
            .is_some_and(|location| location >= previous_failure_token_index);
        if crossed_completed_boundary
            || directives
                .last()
                .is_some_and(|directive| directive.kind == RecoveryDirectiveKind::BoundaryResync)
        {
            errors_in_statement = 1;
        } else {
            errors_in_statement = errors_in_statement
                .checked_add(1)
                .expect("the per-statement recovery error count does not overflow");
        }
        failure = next_failure;
        trace = progress_trace;
        if trial_reached_continuation_cut {
            continuation_cut_reached = true;
            break;
        }
    }

    if !continuation_time_limit_exhausted
        && !continuation_cut_reached
        && !directives.is_empty()
        && errors_in_statement < per_statement_cap
    {
        if let Some(recovered) = try_final_recovery_from_current_failure(
            &tokens,
            source,
            options,
            &failure,
            &directives,
            &errors,
            &parser_tokens,
            &mut recovery_session,
            continuation_sentinel_index,
            continuation_time_limit,
        ) {
            return recovered;
        }
        continuation_time_limit_exhausted =
            continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted);
    }

    recovery_session.clear_memo();
    let parse_tree = degraded_recovered_text(&tokens, source, &errors);
    if !continuation_time_limit_exhausted
        && continuation_time_limit.is_none()
        && continuation_sentinel_index.is_some()
        && continuation_expectations.is_empty()
    {
        continuation_expectations = errors
            .last()
            .map_or_else(Vec::new, syntax_error_expectations);
    }
    new!(RecoveryParseOutcome {
        recovered: RecoveredSyntaxParseAttempt {
            result: new!(RecoveredSyntaxParse {
                parse_tree: Box::new(parse_tree),
                errors,
                warnings: Vec::new(),
            }),
            trace,
        },
        continuation_expectations,
        continuation_cut_reached,
        continuation_time_limit_exhausted,
    })
}

#[requires(!errors.is_empty())]
#[ensures(!ret.recovered.result.errors.is_empty())]
#[ensures(ret.continuation_expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
#[ensures(ret.continuation_cut_reached == continuation_cut_reached)]
fn recovered_success(
    parsed: generated::generated_model::GeneratedRecoveredParsedText,
    errors: &[SyntaxError],
    trace: Option<TraceReport>,
    continuation_expectations: Vec<SyntaxExpectation>,
    continuation_cut_reached: bool,
) -> RecoveryParseOutcome {
    new!(RecoveryParseOutcome {
        recovered: RecoveredSyntaxParseAttempt {
            result: new!(RecoveredSyntaxParse {
                parse_tree: Box::new(parsed.text.into_owned()),
                errors: errors.to_vec(),
                warnings: parsed.warnings,
            }),
            trace,
        },
        continuation_expectations,
        continuation_cut_reached,
        continuation_time_limit_exhausted: false,
    })
}

#[requires(!directives.is_empty())]
#[requires(expected_effective_fail_token_indices.len() + expected_unconsumed_directives == directives.len())]
#[requires(continuation_sentinel_index < tokens.len())]
#[ensures(ret.as_ref().is_none_or(|expectations| expectations.iter().all(|expectation| !expectation.tokens.is_empty())))]
fn replay_winning_continuation_expectations<'tokens>(
    tokens: &[Token],
    parser_tokens: &'tokens [SpannedToken],
    source: Option<&str>,
    options: &ParseOptions,
    directives: &[RecoveryDirective],
    expected_unconsumed_directives: usize,
    expected_effective_fail_token_indices: &[usize],
    expected_failure: &generated::generated_model::GeneratedParseFailure,
    continuation_sentinel_index: usize,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> Option<Vec<SyntaxExpectation>> {
    let attempt = replay_winning_continuation_attempt(
        tokens,
        parser_tokens,
        source,
        options,
        directives,
        continuation_sentinel_index,
        continuation_time_limit,
    );
    let data!(generated::generated_model::GeneratedRecoveredParsedTextAttempt {
        result,
        mut continuation_expectations,
        unconsumed_directives,
        recovery_directives,
        effective_fail_token_indices,
        ..
    }) = attempt.into_data();
    let replay_matches = result.as_ref().is_err_and(|failure| {
        failure.public_error == expected_failure.public_error
            && syntax_error_start(&failure.public_error)
                == recovery_byte_at(tokens, continuation_sentinel_index)
            && unconsumed_directives == expected_unconsumed_directives
            && recovery_directives == directives
            && effective_fail_token_indices == expected_effective_fail_token_indices
    });
    if !replay_matches {
        return None;
    }
    let Err(failure) = result else {
        unreachable!("matching continuation replay is a failure");
    };
    continuation_expectations.extend(syntax_error_expectations(&failure.public_error));
    Some(continuation_expectations)
}

#[requires(!directives.is_empty())]
#[requires(expected_effective_fail_token_indices.len() + expected_unconsumed_directives == directives.len())]
#[requires(continuation_sentinel_index < tokens.len())]
#[ensures(ret.as_ref().is_none_or(|expectations| expectations.iter().all(|expectation| !expectation.tokens.is_empty())))]
fn replay_winning_continuation_success_expectations<'tokens>(
    tokens: &[Token],
    parser_tokens: &'tokens [SpannedToken],
    source: Option<&str>,
    options: &ParseOptions,
    directives: &[RecoveryDirective],
    expected_unconsumed_directives: usize,
    expected_effective_fail_token_indices: &[usize],
    continuation_sentinel_index: usize,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> Option<Vec<SyntaxExpectation>> {
    let attempt = replay_winning_continuation_attempt(
        tokens,
        parser_tokens,
        source,
        options,
        directives,
        continuation_sentinel_index,
        continuation_time_limit,
    );
    let data!(
        generated::generated_model::GeneratedRecoveredParsedTextAttempt {
            result,
            continuation_expectations,
            unconsumed_directives,
            recovery_directives,
            effective_fail_token_indices,
            ..
        }
    ) = attempt.into_data();
    (result.is_ok()
        && unconsumed_directives == expected_unconsumed_directives
        && recovery_directives == directives
        && effective_fail_token_indices == expected_effective_fail_token_indices)
        .then_some(continuation_expectations)
}

#[requires(!directives.is_empty())]
#[requires(continuation_sentinel_index < tokens.len())]
#[ensures(ret.recovery_directives == directives)]
fn replay_winning_continuation_attempt<'tokens>(
    tokens: &[Token],
    parser_tokens: &'tokens [SpannedToken],
    source: Option<&str>,
    options: &ParseOptions,
    directives: &[RecoveryDirective],
    continuation_sentinel_index: usize,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> generated::generated_model::GeneratedRecoveredParsedTextAttempt {
    let mut recovery_session =
        generated::generated_model::GeneratedRecoveryParseSession::new_for_expected_continuations(
            continuation_sentinel_index,
            continuation_time_limit,
        );
    generated::generated_model::parse_recovered_text_attempt_with_session(
        tokens,
        parser_tokens,
        source,
        options,
        directives,
        &mut recovery_session,
    )
}

#[requires(!trial_directives.is_empty())]
#[requires(current_error_count > 0)]
#[ensures(true)]
fn run_recovery_trial<'tokens>(
    tokens: &[Token],
    parser_tokens: &'tokens [SpannedToken],
    source: Option<&str>,
    options: &ParseOptions,
    trial_directives: &[RecoveryDirective],
    directive: &RecoveryDirective,
    recovery_session: &mut generated::generated_model::GeneratedRecoveryParseSession<'tokens>,
    current_error_count: usize,
    global_hard_cap: usize,
    previous_error_start: usize,
) -> RecoveryTrialClassification {
    let attempt = generated::generated_model::parse_recovered_text_attempt_with_session(
        tokens,
        parser_tokens,
        source,
        options,
        trial_directives,
        recovery_session,
    );
    classify_recovery_trial(
        tokens,
        directive,
        attempt,
        current_error_count,
        global_hard_cap,
        previous_error_start,
    )
}

#[requires(current_error_count > 0)]
#[ensures(true)]
fn classify_recovery_trial(
    tokens: &[Token],
    directive: &RecoveryDirective,
    attempt: generated::generated_model::GeneratedRecoveredParsedTextAttempt,
    current_error_count: usize,
    global_hard_cap: usize,
    previous_error_start: usize,
) -> RecoveryTrialClassification {
    let data!(
        generated::generated_model::GeneratedRecoveredParsedTextAttempt {
            result,
            trace,
            unconsumed_directives,
            recovery_directives,
            effective_fail_token_indices,
            completed_recovery_boundary_location,
            continuation_expectations: _,
        }
    ) = attempt.into_data();
    let fired_left_of_declared_failure = recovery_directives
        .last()
        .zip(effective_fail_token_indices.last())
        .is_some_and(|(directive, effective_fail_token_index)| {
            effective_fail_token_index < &directive.fail_token_index
        });
    match result {
        Ok(parsed) if unconsumed_directives == 0 => RecoveryTrialClassification::AcceptedSuccess {
            trial: new!(RecoverySuccessTrial {
                parsed,
                trace,
                directives: recovery_directives,
                effective_fail_token_indices,
            }),
            fired_left_of_declared_failure,
        },
        Ok(_) => RecoveryTrialClassification::Rejected { trace },
        Err(failure) => {
            let next_error_start = syntax_error_start(&failure.public_error);
            if unconsumed_directives == 0
                && current_error_count < global_hard_cap
                && next_error_start > recovery_byte_at(tokens, directive.resume_token_index)
                && next_error_start > previous_error_start
            {
                RecoveryTrialClassification::AcceptedProgress {
                    trial: new!(RecoveryProgressTrial {
                        directives: recovery_directives,
                        failure,
                        trace,
                        effective_fail_token_indices,
                        completed_recovery_boundary_location,
                    }),
                }
            } else {
                RecoveryTrialClassification::Rejected { trace }
            }
        }
    }
}

#[requires(!errors.is_empty())]
#[requires(!directives.is_empty())]
#[requires(directives.len() + 1 == errors.len())]
#[requires(errors.last().is_some_and(|error| error == &failure.public_error))]
#[ensures(ret.as_ref().is_none_or(|attempt| !attempt.recovered.result.errors.is_empty()))]
fn try_final_recovery_from_current_failure<'tokens>(
    tokens: &[Token],
    source: Option<&str>,
    options: &ParseOptions,
    failure: &generated::generated_model::GeneratedParseFailure,
    directives: &[RecoveryDirective],
    errors: &[SyntaxError],
    parser_tokens: &'tokens [SpannedToken],
    recovery_session: &mut generated::generated_model::GeneratedRecoveryParseSession<'tokens>,
    continuation_sentinel_index: Option<usize>,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> Option<RecoveryParseOutcome> {
    for directive in select_final_recovery_directives(tokens, failure, errors.len() - 1) {
        if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
            return None;
        }
        let mut trial_directives = directives.to_vec();
        trial_directives.push(directive.clone());
        let classification = run_recovery_trial(
            tokens,
            parser_tokens,
            source,
            options,
            &trial_directives,
            &directive,
            recovery_session,
            errors.len(),
            options.recovery_error_policy.global_hard_cap().get(),
            errors.last().map_or(0, syntax_error_start),
        );
        if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
            return None;
        }
        let RecoveryTrialClassification::AcceptedSuccess { trial, .. } = classification else {
            continue;
        };
        if trial.directives == trial_directives {
            let data!(RecoverySuccessTrial {
                parsed,
                trace,
                directives: recovery_directives,
                effective_fail_token_indices,
            }) = trial.into_data();
            let continuation_expectations =
                if let Some(sentinel_index) = continuation_sentinel_index {
                    let expectations = replay_winning_continuation_success_expectations(
                        tokens,
                        parser_tokens,
                        source,
                        options,
                        &recovery_directives,
                        0,
                        &effective_fail_token_indices,
                        sentinel_index,
                        continuation_time_limit,
                    )
                    .unwrap_or_default();
                    if continuation_time_limit.is_some_and(ContinuationTimeLimit::exhausted) {
                        return None;
                    }
                    expectations
                } else {
                    Vec::new()
                };
            recovery_session.clear_memo();
            return Some(recovered_success(
                parsed,
                errors,
                trace,
                continuation_expectations,
                continuation_sentinel_index.is_some(),
            ));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn same_recovery_site(left: &RecoveryDirective, right: &RecoveryDirective) -> bool {
    left.kind == right.kind
        && left.fail_token_index == right.fail_token_index
        && left.resume_token_index == right.resume_token_index
        && left.resume_field == right.resume_field
        && left.rule == right.rule
        && left.instance_byte_start == right.instance_byte_start
        && left.boundary_unwind_start_token_index == right.boundary_unwind_start_token_index
}

#[requires(true)]
#[ensures(ret -> left.kind == RecoveryDirectiveKind::BoundaryResync)]
#[ensures(ret -> right.kind == RecoveryDirectiveKind::BoundaryResync)]
fn same_boundary_resync_group(left: &RecoveryDirective, right: &RecoveryDirective) -> bool {
    left.kind == RecoveryDirectiveKind::BoundaryResync
        && right.kind == RecoveryDirectiveKind::BoundaryResync
        && left.rule == right.rule
        && left.instance_byte_start == right.instance_byte_start
        && left.resume_token_index == right.resume_token_index
        && left.resume_field == right.resume_field
}

#[requires(true)]
#[ensures(continuation_time_limit.is_some() -> !ret)]
fn recovery_reachability_filter_enabled(
    options: &ParseOptions,
    continuation_time_limit: Option<ContinuationTimeLimit>,
) -> bool {
    if options.trace.includes(TracePhase::Syntax) || continuation_time_limit.is_some() {
        return false;
    }

    #[cfg(feature = "expensive_contracts")]
    if RECOVERY_REACHABILITY_FILTER_DISABLED.with(Cell::get) {
        return false;
    }

    true
}

#[requires(true)]
#[ensures(directive.kind == RecoveryDirectiveKind::BoundaryResync -> ret)]
fn exact_trial_reachable(
    failure: &generated::generated_model::GeneratedParseFailure,
    directive: &RecoveryDirective,
) -> bool {
    directive.kind == RecoveryDirectiveKind::BoundaryResync
        || failure.checkpoints.contains_local_exact_site(
            directive.rule,
            directive.instance_byte_start,
            directive.fail_token_index,
            directive.resume_field,
        )
}

#[invariant(!rule.is_empty())]
#[invariant(match (kind, boundary_unwind_start_token_index) {
    (RecoveryDirectiveKind::Local, None) => true,
    (RecoveryDirectiveKind::BoundaryResync, Some(start)) => *start < *resume_token_index,
    _ => false,
})]
#[derive(Debug, Clone)]
struct RecoveryClaim {
    branch_index: usize,
    inner_rank: usize,
    kind: RecoveryDirectiveKind,
    boundary_unwind_start_token_index: Option<usize>,
    rule: &'static str,
    instance_byte_start: usize,
    resume_token_index: usize,
    resume_field: usize,
}

#[invariant(!directives.is_empty())]
#[invariant(effective_fail_token_indices.len() == directives.len())]
struct RecoveryProgressTrial {
    directives: Vec<RecoveryDirective>,
    failure: generated::generated_model::GeneratedParseFailure,
    trace: Option<TraceReport>,
    effective_fail_token_indices: Vec<usize>,
    completed_recovery_boundary_location: Option<usize>,
}

#[invariant(!directives.is_empty())]
#[invariant(effective_fail_token_indices.len() == directives.len())]
struct RecoverySuccessTrial {
    parsed: generated::generated_model::GeneratedRecoveredParsedText,
    trace: Option<TraceReport>,
    directives: Vec<RecoveryDirective>,
    effective_fail_token_indices: Vec<usize>,
}

#[invariant(::AcceptedSuccess => true)]
#[invariant(::AcceptedProgress => true)]
#[invariant(::Rejected => true)]
enum RecoveryTrialClassification {
    AcceptedSuccess {
        trial: RecoverySuccessTrial,
        fired_left_of_declared_failure: bool,
    },
    AcceptedProgress {
        trial: RecoveryProgressTrial,
    },
    Rejected {
        trace: Option<TraceReport>,
    },
}

#[invariant(continuation_expectations.iter().all(|expectation| !expectation.tokens.is_empty()))]
#[invariant(!*continuation_cut_reached || !*continuation_time_limit_exhausted)]
struct RecoveryParseOutcome {
    recovered: RecoveredSyntaxParseAttempt,
    continuation_expectations: Vec<SyntaxExpectation>,
    continuation_cut_reached: bool,
    continuation_time_limit_exhausted: bool,
}

#[invariant(cmavo.len() == opens_subtext_container.len())]
#[invariant(cmavo.len() == closes_subtext_container.len())]
#[invariant(container_depth_before.len() == cmavo.len() + 1)]
struct RecoveryTokenScan {
    cmavo: Vec<Option<Cmavo>>,
    opens_subtext_container: Vec<bool>,
    closes_subtext_container: Vec<bool>,
    container_depth_before: Vec<usize>,
}

impl RecoveryTokenScan {
    #[requires(true)]
    #[ensures(ret.cmavo.len() == tokens.len())]
    #[ensures(ret.container_depth_before.len() == tokens.len() + 1)]
    fn new(tokens: &[Token]) -> Self {
        let cmavo = tokens.iter().map(Token::cmavo).collect::<Vec<_>>();
        let opens_subtext_container = cmavo
            .iter()
            .map(|cmavo| token_cmavo_opens_subtext_container(*cmavo))
            .collect::<Vec<_>>();
        let closes_subtext_container = cmavo
            .iter()
            .map(|cmavo| token_cmavo_closes_subtext_container(*cmavo))
            .collect::<Vec<_>>();
        let mut container_depth_before = Vec::with_capacity(tokens.len() + 1);
        let mut depth = 0usize;
        container_depth_before.push(depth);
        for (opens, closes) in opens_subtext_container
            .iter()
            .zip(&closes_subtext_container)
        {
            if *closes && depth > 0 {
                depth -= 1;
            }
            if *opens {
                depth += 1;
            }
            container_depth_before.push(depth);
        }
        new!(RecoveryTokenScan {
            cmavo,
            opens_subtext_container,
            closes_subtext_container,
            container_depth_before,
        })
    }

    #[requires(index <= self.cmavo.len())]
    #[ensures(true)]
    fn container_depth_before(&self, index: usize) -> usize {
        self.container_depth_before[index]
    }
}

#[requires(true)]
#[requires(scan.cmavo.len() == tokens.len())]
#[ensures(ret.iter().all(|directive| directive.fail_token_index <= directive.resume_token_index))]
fn select_recovery_directives(
    tokens: &[Token],
    scan: &RecoveryTokenScan,
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
                    let frame_container_depth = scan.container_depth_before(frame_token_index);
                    let Some(resume_token_index) = scan_for_recovery_anchor(
                        scan,
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
                        kind: if anchor.boundary_resync && frame_token_index < resume_token_index {
                            RecoveryDirectiveKind::BoundaryResync
                        } else {
                            RecoveryDirectiveKind::Local
                        },
                        boundary_unwind_start_token_index: (anchor.boundary_resync
                            && frame_token_index < resume_token_index)
                            .then_some(frame_token_index),
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
        let directive = match selected.kind {
            RecoveryDirectiveKind::Local => directive,
            RecoveryDirectiveKind::BoundaryResync => directive.into_boundary_resync(
                selected
                    .boundary_unwind_start_token_index
                    .expect("boundary claims carry their owning rule start"),
            ),
        };
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
                kind: RecoveryDirectiveKind::Local,
                boundary_unwind_start_token_index: None,
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
fn recovery_claim_sort_key(claim: &RecoveryClaim) -> (usize, usize, bool, usize) {
    // A boundary at an enclosing text layer must not preempt a more precise
    // claim from the grammar construct that actually owns the same token
    // (notably connective I inside a statement). Boundary resync wins only
    // after locality has selected the innermost active owner.
    (
        claim.resume_token_index,
        claim.inner_rank,
        matches!(claim.kind, RecoveryDirectiveKind::Local),
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
        "Cbm" => dialect.cbm_enabled,
        "UnrestrictedFree" => dialect.unrestricted_free_enabled,
        "ZantufaAdverbials" => dialect.zantufa_adverbials_enabled,
        "ZantufaConnectives" => dialect.zantufa_connectives_enabled,
        "ZantufaMex" => dialect.zantufa_mex_enabled,
        "ZantufaMexReinterpretation" => dialect.zantufa_mex_reinterpretation_enabled,
        "ZantufaSelbriReinterpretation" => dialect.zantufa_selbri_reinterpretation_enabled,
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

#[requires(start <= scan.cmavo.len())]
#[ensures(ret.is_none_or(|index| index >= start && index < scan.cmavo.len()))]
fn scan_for_recovery_anchor(
    scan: &RecoveryTokenScan,
    start: usize,
    base_depth: usize,
    anchors: &[generated::generated_model::SyntaxGrammarAnchorToken],
) -> Option<usize> {
    let mut depth = scan
        .container_depth_before(start)
        .saturating_sub(base_depth);
    for (index, cmavo) in scan.cmavo.iter().enumerate().skip(start) {
        let opens = scan.opens_subtext_container[index];
        let closes = scan.closes_subtext_container[index];
        if closes {
            if depth == 0 {
                return None;
            }
            depth -= 1;
        }
        if depth == 0
            && anchors
                .iter()
                .any(|anchor| recovery_anchor_matches_cmavo(*anchor, *cmavo))
        {
            return Some(index);
        }
        if opens {
            depth += 1;
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn token_cmavo_opens_subtext_container(cmavo: Option<Cmavo>) -> bool {
    generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS
        .iter()
        .any(|container| {
            container
                .opener_tokens
                .iter()
                .any(|anchor| recovery_anchor_matches_cmavo(*anchor, cmavo))
        })
}

#[requires(true)]
#[ensures(true)]
fn token_cmavo_closes_subtext_container(cmavo: Option<Cmavo>) -> bool {
    generated::generated_model::SYNTAX_GRAMMAR_SUBTEXT_CONTAINERS
        .iter()
        .any(|container| {
            container
                .closer_tokens
                .iter()
                .any(|anchor| recovery_anchor_matches_cmavo(*anchor, cmavo))
        })
}

#[requires(true)]
#[ensures(true)]
fn recovery_anchor_matches_cmavo(
    anchor: generated::generated_model::SyntaxGrammarAnchorToken,
    actual: Option<Cmavo>,
) -> bool {
    match anchor {
        generated::generated_model::SyntaxGrammarAnchorToken::Cmavo(cmavo) => actual == Some(cmavo),
        generated::generated_model::SyntaxGrammarAnchorToken::Selmaho(selmaho) => {
            actual.is_some_and(|cmavo| selmaho.contains(cmavo))
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
#[requires(effective_fail_token_index <= directive.resume_token_index)]
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
    cbm_enabled: bool,
    warnings: &mut Vec<SyntaxWarning>,
) {
    let mut visitor = new!(GeneratedConstructWarningVisitor {
        tokens,
        cbm_enabled,
        zantufa_tag_depth: Cell::new(0),
        warnings: RefCell::new(warnings),
    });
    generated::generated_model::TreeNode::visit_in_order(text, &mut visitor);
    drop(visitor);
    // Parser-attached warnings are collected before structural warnings. Restore source order
    // after combining both streams; the stable sort preserves their existing order at one token.
    warnings.sort_by_key(|warning| warning.anchor_index);
}

#[invariant(
    tokens
        .iter()
        .all(|token| token.core_word().byte_range().is_some()),
    "generated warning anchors require source-backed syntax tokens"
)]
struct GeneratedConstructWarningVisitor<'a> {
    tokens: &'a [Token],
    cbm_enabled: bool,
    zantufa_tag_depth: Cell<usize>,
    warnings: RefCell<&'a mut Vec<SyntaxWarning>>,
}

impl GeneratedConstructWarningVisitor<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn warn_first_token<T>(&mut self, construct: ExperimentalConstruct, node: &T)
    where
        T: generated::generated_model::TreeNode,
    {
        if construct == ExperimentalConstruct::ExperimentalZantufaMex
            && self.zantufa_tag_depth.get() > 0
        {
            return;
        }
        let mut visitor = new!(FirstTokenVisitor {
            token: Cell::new(None),
        });
        node.visit_in_order(&mut visitor);
        if let Some(anchor) = visitor.token.get() {
            let mut warnings = self.warnings.borrow_mut();
            push_generated_construct_warning(&mut warnings, self.tokens, construct, anchor);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn remove_nested_zantufa_warnings<T>(&mut self, node: &T)
    where
        T: generated::generated_model::TreeNode,
    {
        let mut visitor = new!(TokenRangeVisitor {
            first: Cell::new(None),
            last: Cell::new(None),
        });
        node.visit_in_order(&mut visitor);
        let (Some(first), Some(last)) = (visitor.first.get(), visitor.last.get()) else {
            return;
        };
        let first = generated_warning_anchor_index(self.tokens, first);
        let last = generated_warning_anchor_index(self.tokens, last);
        self.warnings.borrow_mut().retain(|warning| {
            !matches!(
                warning.kind,
                ExperimentalConstruct::ExperimentalZantufaMex
                    | ExperimentalConstruct::ExperimentalZantufaCmavo
            ) || warning.anchor_index < first
                || warning.anchor_index > last
        });
    }

    #[requires(description.0.value.is_cmavo(Cmavo::La))]
    #[ensures(true)]
    fn warn_cbm_la_name_form(
        &mut self,
        description: &generated::generated_model::DescriptionHeadSyntax,
        tail: &generated::generated_model::DescriptionTailSyntax,
    ) {
        let mut visitor = new!(FirstTokenVisitor {
            token: Cell::new(None),
        });
        generated::generated_model::TreeNode::visit_in_order(&tail.tail, &mut visitor);
        if visitor.token.get().is_some_and(tokens::is_cmevla_word) {
            let anchor = &description.0.value;
            let mut warnings = self.warnings.borrow_mut();
            push_generated_construct_warning(
                &mut warnings,
                self.tokens,
                ExperimentalConstruct::ExperimentalCbmLaNameAsDescriptor,
                anchor,
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_exp_run_is_single_unprefixed_fa(
    run: &generated::generated_model::ExpTagAtomRunSyntax,
) -> bool {
    let generated::generated_model::ExpTagAtomRunSyntax(run) = run;
    let generated::generated_model::ExpTagAtomRunBodySyntax { first, additional } = run.as_ref();
    let generated::generated_model::ExpPrefixedTagAtomSyntax { nahe, se, atom } = first.as_ref();
    additional.is_empty()
        && nahe.is_none()
        && se.is_none()
        && matches!(
            atom.value.as_ref(),
            generated::generated_model::ExpTagAtomSyntax::ExpFaTagAtom(_)
        )
}

impl<'tree> TreeVisitor<'tree> for GeneratedConstructWarningVisitor<'_> {
    type Node = generated::generated_model::NodeRef<'tree>;
    type Atom = generated::generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        match node {
            generated::generated_model::NodeRef::ExpTagAtomRunSyntax(run)
                if !generated_exp_run_is_single_unprefixed_fa(run) =>
            {
                self.warn_first_token(ExperimentalConstruct::ExperimentalFlattenedTag, run);
            }
            // The loose (T3) term tier is a diagnosed extension: camxes-standard has no
            // term-level connective at all. Its continuations own no token of their own -- the
            // connective belongs to the `joik_connective` / `ek_connective` nodes the sumti and
            // statement tiers share -- so the tier is diagnosed post-parse here, where the
            // continuation node is complete and its first token is exactly its connective. The
            // BO (T4) tier keeps its in-parser `warn` on the `bo` token it does own.
            generated::generated_model::NodeRef::ConnectedTermContinuationSyntax(continuation) => {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalTermLooseConnection,
                    continuation,
                );
            }
            // Rolling Zantufa's NUhI-less termset owns no token of its own -- its GEK and its
            // first GIK are the shapes every other forethought connection spells, and its GIhI is
            // elidable -- so the arm is diagnosed post-parse here, anchored at the GEK that opens
            // it. Branches beyond the first keep the in-parser n-ary warning on their own GI.
            generated::generated_model::NodeRef::ZantufaGekTermsetSyntax(termset) => {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalZantufaGekTermset,
                    termset,
                );
            }
            generated::generated_model::NodeRef::ConnectedLinkedTermContinuationSyntax(
                continuation,
            ) => {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalTermLooseConnection,
                    continuation,
                );
            }
            generated::generated_model::NodeRef::ZantufaTagSyntax(tag) => {
                self.remove_nested_zantufa_warnings(tag);
                self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaTag, tag);
                self.zantufa_tag_depth
                    .set(self.zantufa_tag_depth.get() + 1);
            }
            generated::generated_model::NodeRef::ZantufaRelativeSelbriSyntax(selbri) => {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalZantufaSelbriRelativePlacement,
                    selbri.relative_clauses.as_ref(),
                );
            }
            generated::generated_model::NodeRef::ZantufaBareRelativeClauseTailSyntax(tail) => {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalZantufaSelbriRelativePlacement,
                    tail.0.as_ref(),
                );
            }
            generated::generated_model::NodeRef::FragmentStatementSyntaxZantufaMeksoFragment(
                fragment,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, fragment),
            generated::generated_model::NodeRef::QuantifierSyntaxZantufaRawMeksoQuantifier(
                quantifier,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, quantifier),
            generated::generated_model::NodeRef::QuantifierSyntaxZantufaPriorityRawMeksoQuantifier(
                quantifier,
            ) => self.warn_first_token(ExperimentalConstruct::ExperimentalZantufaMex, quantifier),
            generated::generated_model::NodeRef::ZantufaMexContinuationSyntax(continuation)
                if continuation.right_expression.is_none()
                    && matches!(
                        continuation.operators.last().as_ref(),
                        generated::generated_model::ZantufaOperatorSyntax::ZantufaConnectiveMeksoOperator(_)
                    ) =>
            {
                self.warn_first_token(
                    ExperimentalConstruct::ExperimentalZantufaMex,
                    continuation,
                );
            }
            generated::generated_model::NodeRef::AtomicMeksoOperatorSyntaxExperimentalConnectiveMeksoOperator(
                operator,
            ) => self.warn_first_token(
                ExperimentalConstruct::ExperimentalMexOperatorConnective,
                operator,
            ),
            generated::generated_model::NodeRef::SumtiBaseSyntaxDescriptorWithGadriSumti(base) => {
                if let generated::generated_model::SumtiBaseSyntax::DescriptorWithGadriSumti(
                    description,
                ) = base
                    && self.cbm_enabled
                    && description.description.0.value.is_cmavo(Cmavo::La)
                {
                    self.warn_cbm_la_name_form(&description.description, &description.tail);
                }
            }
            generated::generated_model::NodeRef::SumtiBaseSyntaxDescriptorWithOuterQuantifierSumti(
                base,
            ) => {
                if let generated::generated_model::SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(
                    description,
                ) = base
                    && self.cbm_enabled
                    && description.description.0.value.is_cmavo(Cmavo::La)
                {
                    self.warn_cbm_la_name_form(&description.description, &description.tail);
                }
            }
            generated::generated_model::NodeRef::SumtiBaseSyntaxDescriptionConnectionSumti(base) => {
                if let generated::generated_model::SumtiBaseSyntax::DescriptionConnectionSumti(
                    description,
                ) = base
                    && self.cbm_enabled
                    && description.leading_description_head.0.value.is_cmavo(Cmavo::La)
                {
                    self.warn_cbm_la_name_form(
                        &description.leading_description_head,
                        &description.tail,
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        if matches!(
            node,
            generated::generated_model::NodeRef::ZantufaTagSyntax(_)
        ) {
            assert!(
                self.zantufa_tag_depth.get() > 0,
                "Zantufa tag traversal exit must follow its matching entry"
            );
            self.zantufa_tag_depth.set(self.zantufa_tag_depth.get() - 1);
        }
    }
}

#[invariant(
    first
        .get()
        .is_none_or(|token| token.core_word().byte_range().is_some()),
    "captured first token must be source-backed"
)]
#[invariant(
    last
        .get()
        .is_none_or(|token| token.core_word().byte_range().is_some()),
    "captured last token must be source-backed"
)]
struct TokenRangeVisitor<'tree> {
    first: Cell<Option<&'tree Token>>,
    last: Cell<Option<&'tree Token>>,
}

impl<'tree> TreeVisitor<'tree> for TokenRangeVisitor<'tree> {
    type Node = generated::generated_model::NodeRef<'tree>;
    type Atom = generated::generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated::generated_model::AtomRef::Token(token) = atom;
        if self.first.get().is_none() {
            self.first.set(Some(token));
        }
        self.last.set(Some(token));
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
    tokens
        .iter()
        .position(|token| Token::ptr_eq(token, anchor))
        .unwrap_or(tokens.len())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn syntax_tokens(words: &[WordLike], options: &ParseOptions) -> Vec<Token> {
    prepare_syntax_tokens(words.iter().cloned().map(Token::bare).collect(), options)
}

#[requires(true)]
#[ensures(true)]
fn prepare_syntax_tokens(tokens: Vec<Token>, options: &ParseOptions) -> Vec<Token> {
    attach_indicators(
        attach_bahe(tokens),
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
    use jbotci_morphology::{
        MorphologyOptions, WordLikeData, segment_words_with_modifiers,
        segment_words_with_modifiers_with_options_and_source_id,
    };
    use jbotci_tree::RecoveredFieldState;
    use std::{
        fmt::Write as _,
        fs,
        path::Path,
        rc::Rc,
        time::{Duration, Instant},
    };
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
    fn reachability_filter_observability_guards_are_explicit() {
        let ordinary = ParseOptions::default();
        assert!(recovery_reachability_filter_enabled(&ordinary, None));

        let traced = ordinary
            .clone()
            .with_trace_options(crate::TraceOptions::enabled(
                TraceLevel::Top,
                None,
                TracePhase::Syntax,
                1,
            ));
        assert!(!recovery_reachability_filter_enabled(&traced, None));

        let deadline = Some(ContinuationTimeLimit::new(Duration::from_secs(1)));
        assert!(!recovery_reachability_filter_enabled(&ordinary, deadline));
    }

    #[requires(true)]
    #[ensures(ret.0.recovery_directives.len() == 1)]
    fn boundary_recovery_test_state() -> (
        ParserState<'static>,
        Rc<RefCell<SyntaxRecoveryMemoStore<'static>>>,
    ) {
        let source = "mi zo'u do .i mi klama";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let directive = RecoveryDirective::new("owner", 0, 3, 3, 2, 0, SyntaxError::NotImplemented)
            .into_boundary_resync(0);
        let mut session = SyntaxRecoveryMemoSession::new();
        let trial = session.begin_trial();
        let store = Rc::clone(&trial.store);
        (
            ParserState::new_with_recovery(
                &tokens,
                Some(source),
                &ParseOptions::default(),
                &[directive],
                trial,
                None,
                None,
            ),
            store,
        )
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn incomplete_boundary_resync_is_retracted_before_driver_acceptance() {
        let (mut state, _store) = boundary_recovery_test_state();
        let action = state.boundary_resync_field_action_after_failure("owner", 0, 0, 0);
        let (kind, item, resume_token_index) = action.into_parts();
        assert_eq!(kind, RecoveryFieldActionKind::BoundaryResync);
        assert!(item.is_some());
        assert_eq!(resume_token_index, Some(3));
        assert_eq!(state.consumed_recovery_directives, 1);
        assert!(state.active_recovery_directive.is_some());

        // Model a second failure before generated traversal reaches the resume
        // field. Finishing the failed attempt makes the fired directive pending
        // again, so #533 cannot accept a stranded half-consumed chain.
        let finish = state.finish();
        assert_eq!(finish.unconsumed_recovery_directives, 1);
        assert!(finish.effective_fail_token_indices.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn consumed_boundary_resync_rejects_intersecting_insensitive_sibling_memo() {
        let (mut state, store) = boundary_recovery_test_state();
        let mut store = store.borrow_mut();
        store
            .rule_observation_nodes
            .push(SyntaxRuleObservationNode {
                observation: new!(SyntaxRuleObservation {
                    rule: "sibling",
                    instance_byte_start: 0,
                }),
                children: Vec::new(),
            });
        let memo = |start_location, end_location| {
            new!(SyntaxMemoSuccess {
                start_location,
                end_location,
                recovery_index: 0,
                consumed_recovery_directives: 0,
                effective_fail_token_indices: Vec::new(),
                value: SyntaxMemoValue::from_shared(Rc::new(())),
                side_effects: SyntaxMemoSideEffects {
                    warnings: Rc::from([]),
                    recovery_checkpoint_observations: None,
                    diagnostic_observations: None,
                },
                rule_observation_node: Some(0),
            })
        };
        store
            .insensitive_successes
            .insert(("intersecting", 1, SyntaxMemoScope::Ordinary), memo(1, 2));
        store
            .insensitive_successes
            .insert(("disjoint", 3, SyntaxMemoScope::Ordinary), memo(3, 4));
        drop(store);

        state.consumed_recovery_directives = 1;
        state.effective_fail_token_indices.push(0);
        state
            .abandoned_recovery_ranges
            .push(BoundaryAbandonedRange::new(0, 3));
        state.begin_syntax_memo_rule_frame();
        let context = state.syntax_memo_context();
        assert!(
            state
                .syntax_memo_success("intersecting", 1, context)
                .is_none(),
            "a sibling memo evaluated inside the abandoned range is stale",
        );
        assert!(
            state.syntax_memo_success("disjoint", 3, context).is_some(),
            "memo reuse outside the abandoned range remains available",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parameterized_rule_memos_are_isolated_by_scope() {
        let mut state = ParserState::new(&[], &ParseOptions::default());

        state.begin_syntax_memo_rule_frame();
        let ordinary_context = state.syntax_memo_context();
        state.store_syntax_memo_success(
            "parameterized",
            0,
            ordinary_context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(1_u8)),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();

        let previous = state.enter_syntax_memo_scope(SyntaxMemoScope::CeiFree);
        state.begin_syntax_memo_rule_frame();
        let cei_free_context = state.syntax_memo_context();
        state.store_syntax_memo_success(
            "parameterized",
            0,
            cei_free_context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(2_u8)),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();
        state.restore_syntax_memo_scope(previous);

        let previous = state.enter_syntax_memo_scope(SyntaxMemoScope::DescriptionRelative);
        state.begin_syntax_memo_rule_frame();
        let description_context = state.syntax_memo_context();
        state.store_syntax_memo_success(
            "parameterized",
            0,
            description_context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(3_u8)),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();
        state.restore_syntax_memo_scope(previous);

        for (scope, expected) in [
            (SyntaxMemoScope::Ordinary, 1_u8),
            (SyntaxMemoScope::CeiFree, 2_u8),
            (SyntaxMemoScope::DescriptionRelative, 3_u8),
        ] {
            let previous = state.enter_syntax_memo_scope(scope);
            state.begin_syntax_memo_rule_frame();
            let context = state.syntax_memo_context();
            let value = state
                .syntax_memo_success("parameterized", 0, context)
                .expect("the scoped memo is present")
                .value()
                .downcast::<u8>()
                .expect("the scoped memo retains its typed value");
            state.finish_syntax_memo_rule_frame();
            state.restore_syntax_memo_scope(previous);
            assert_eq!(*value, expected);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_checkpoint_index_preserves_exact_site_semantics() {
        let checkpoints = vec![
            RecoveryCheckpoint::new("observed", 7, 11, 4, RecoveryCheckpointKind::Trailing),
            RecoveryCheckpoint::new("observed", 7, 11, 1, RecoveryCheckpointKind::FieldStart),
        ];
        let index = RecoveryCheckpointIndex::from_checkpoints(checkpoints);

        assert!(!index.contains_local_exact_site("observed", 7, 11, 0));
        assert!(index.contains_local_exact_site("observed", 7, 11, 1));
        assert!(index.contains_local_exact_site("observed", 7, 11, 4));
        assert!(!index.contains_local_exact_site("other", 7, 11, 4));
        assert!(!index.contains_local_exact_site("observed", 8, 11, 4));
        assert!(!index.contains_local_exact_site("observed", 7, 12, 4));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_checkpoint_collection_deduplicates_captured_ranges() {
        let first = RecoveryCheckpoint::new("first", 0, 1, 2, RecoveryCheckpointKind::FieldStart);
        let second = RecoveryCheckpoint::new("second", 0, 2, 3, RecoveryCheckpointKind::Trailing);
        let mut collection = RecoveryCheckpointCollection::new();
        collection.record(first.clone(), None);
        let frame_start = collection.observation_count();
        collection.record(first.clone(), Some(frame_start));
        collection.record(first.clone(), Some(frame_start));
        collection.record(second.clone(), Some(frame_start));

        assert_eq!(collection.observation_count(), 3);
        assert_eq!(
            collection.capture_range(frame_start, 3).as_ref(),
            [first.clone(), second.clone()],
            "an observation before the frame does not suppress the frame's copy",
        );
        let sibling_start = collection.observation_count();
        collection.record(first.clone(), Some(sibling_start));
        assert_eq!(
            collection.capture_range(sibling_start, 4).as_ref(),
            [first.clone()],
            "a sibling frame receives its own observation range",
        );
        assert_eq!(collection.into_checkpoints(), [first, second]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn memo_checkpoint_observations_share_child_nodes_and_replay_recursively() {
        let (mut state, store) = boundary_recovery_test_state();
        let parent_checkpoint =
            RecoveryCheckpoint::new("parent", 0, 1, 1, RecoveryCheckpointKind::FieldStart);
        let child_checkpoint =
            RecoveryCheckpoint::new("child", 0, 2, 2, RecoveryCheckpointKind::Trailing);

        state.begin_syntax_memo_rule_frame();
        let parent_context = state.syntax_memo_context();
        state.observe_syntax_rule("parent-memo", 0);
        state.record_recovery_checkpoint(
            parent_checkpoint.rule,
            parent_checkpoint.instance_byte_start,
            parent_checkpoint.token_index,
            parent_checkpoint.field_index,
            parent_checkpoint.kind,
        );

        state.begin_syntax_memo_rule_frame();
        let child_context = state.syntax_memo_context();
        state.observe_syntax_rule("child-memo", 0);
        state.record_recovery_checkpoint(
            child_checkpoint.rule,
            child_checkpoint.instance_byte_start,
            child_checkpoint.token_index,
            child_checkpoint.field_index,
            child_checkpoint.kind,
        );
        state.store_syntax_memo_success(
            "child-memo",
            0,
            child_context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(())),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();

        state.store_syntax_memo_success(
            "parent-memo",
            0,
            parent_context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(())),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();

        let store_ref = store.borrow();
        let child_observations = Rc::clone(
            store_ref
                .insensitive_successes
                .get(&("child-memo", 0, SyntaxMemoScope::Ordinary))
                .and_then(|memo| memo.side_effects.recovery_checkpoint_observations.as_ref())
                .expect("the child memo retained checkpoint observations"),
        );
        let parent_observations = Rc::clone(
            store_ref
                .insensitive_successes
                .get(&("parent-memo", 0, SyntaxMemoScope::Ordinary))
                .and_then(|memo| memo.side_effects.recovery_checkpoint_observations.as_ref())
                .expect("the parent memo retained checkpoint observations"),
        );
        assert_eq!(
            parent_observations.checkpoints.as_ref(),
            [parent_checkpoint.clone()],
        );
        assert_eq!(parent_observations.children.len(), 1);
        assert!(Rc::ptr_eq(
            &parent_observations.children[0],
            &child_observations,
        ));
        assert_eq!(
            child_observations.checkpoints.as_ref(),
            [child_checkpoint.clone()],
        );
        drop(store_ref);

        state
            .recovery_checkpoint_collection
            .as_mut()
            .expect("the recovery state tracks checkpoints")
            .clear();
        state.begin_syntax_memo_rule_frame();
        let replay_context = state.syntax_memo_context();
        let hit = state
            .syntax_memo_success("parent-memo", 0, replay_context)
            .expect("the parent memo can be reused");
        let replay = state.apply_syntax_memo_success(hit);
        state.replay_syntax_memo_side_effects(&replay.side_effects);
        state.finish_syntax_memo_rule_frame();
        assert_eq!(
            state
                .recovery_checkpoint_collection
                .expect("the recovery state tracks checkpoint observations")
                .into_checkpoints(),
            [parent_checkpoint, child_checkpoint],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn memo_side_effects_retain_duplicate_checkpoint_observations() {
        let (mut state, store) = boundary_recovery_test_state();
        let checkpoint =
            RecoveryCheckpoint::new("observed", 0, 1, 2, RecoveryCheckpointKind::FieldStart);
        state.record_recovery_checkpoint(
            checkpoint.rule,
            checkpoint.instance_byte_start,
            checkpoint.token_index,
            checkpoint.field_index,
            checkpoint.kind,
        );

        state.begin_syntax_memo_rule_frame();
        let context = state.syntax_memo_context();
        state.observe_syntax_rule("memo", 0);
        state.record_recovery_checkpoint(
            checkpoint.rule,
            checkpoint.instance_byte_start,
            checkpoint.token_index,
            checkpoint.field_index,
            checkpoint.kind,
        );
        state.store_syntax_memo_success(
            "memo",
            0,
            context,
            0,
            SyntaxMemoValue::from_shared(Rc::new(())),
            Vec::new(),
        );
        state.finish_syntax_memo_rule_frame();

        let store = store.borrow();
        let memo = store
            .insensitive_successes
            .get(&("memo", 0, SyntaxMemoScope::Ordinary))
            .expect("the insensitive memo was stored");
        assert_eq!(
            memo.side_effects
                .recovery_checkpoint_observations
                .as_ref()
                .expect("the memo retained its checkpoint observations")
                .checkpoints
                .as_ref(),
            [checkpoint.clone()],
            "the memo must replay observations that were already globally deduplicated",
        );
        assert!(
            memo.side_effects
                .recovery_checkpoint_observations
                .as_ref()
                .expect("the memo retained its checkpoint observations")
                .children
                .is_empty(),
        );
        drop(store);

        state
            .recovery_checkpoint_collection
            .as_mut()
            .expect("the recovery state tracks checkpoints")
            .clear();
        state.begin_syntax_memo_rule_frame();
        let replay_context = state.syntax_memo_context();
        let hit = state
            .syntax_memo_success("memo", 0, replay_context)
            .expect("the insensitive memo can be reused");
        let replay = state.apply_syntax_memo_success(hit);
        state.replay_syntax_memo_side_effects(&replay.side_effects);
        state.finish_syntax_memo_rule_frame();
        assert_eq!(
            state
                .recovery_checkpoint_collection
                .expect("the recovery state tracks checkpoint observations")
                .into_checkpoints(),
            [checkpoint],
            "a fresh trial receives the checkpoint from the memo replay",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovery_observation_target_index_cache_tracks_pending_suffix() {
        let source = "mi klama";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let directives = [
            RecoveryDirective::new("a", 0, 0, 1, 0, 0, SyntaxError::NotImplemented),
            RecoveryDirective::new("b", 0, 0, 1, 0, 1, SyntaxError::NotImplemented),
            RecoveryDirective::new("a", 0, 0, 1, 0, 2, SyntaxError::NotImplemented),
        ];
        let mut session = SyntaxRecoveryMemoSession::new();
        let trial = session.begin_trial();
        let store = Rc::clone(&trial.store);
        let mut state = ParserState::new_with_recovery(
            &tokens,
            Some(source),
            &ParseOptions::default(),
            &directives,
            trial,
            None,
            None,
        );
        let mut store = store.borrow_mut();
        store.rule_observation_nodes.extend([
            SyntaxRuleObservationNode {
                observation: new!(SyntaxRuleObservation {
                    rule: "a",
                    instance_byte_start: 0,
                }),
                children: vec![1],
            },
            SyntaxRuleObservationNode {
                observation: new!(SyntaxRuleObservation {
                    rule: "b",
                    instance_byte_start: 0,
                }),
                children: Vec::new(),
            },
            SyntaxRuleObservationNode {
                observation: new!(SyntaxRuleObservation {
                    rule: "c",
                    instance_byte_start: 0,
                }),
                children: Vec::new(),
            },
        ]);

        assert_eq!(
            state.syntax_rule_observation_latest_recovery_target_index(&store, 0),
            Some(2),
        );
        assert_eq!(
            state.syntax_rule_observation_latest_recovery_target_index(&store, 1),
            Some(1),
        );
        assert_eq!(
            state.syntax_rule_observation_latest_recovery_target_index(&store, 2),
            None,
        );
        assert!(!state.syntax_rule_observations_are_insensitive(&store, 0));
        state.consumed_recovery_directives = 3;
        state.effective_fail_token_indices = vec![0; 3];
        assert!(state.syntax_rule_observations_are_insensitive(&store, 0));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn boundary_scan_does_not_escape_through_nested_tuhe_i() {
        let source = "tu'e mi ku .i do tu'u .i mi klama";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let tokens = syntax_tokens(&words, &ParseOptions::default());
        let scan = RecoveryTokenScan::new(&tokens);
        let i_anchor = [generated::generated_model::SyntaxGrammarAnchorToken::Cmavo(
            Cmavo::I,
        )];
        let inner_i = tokens
            .iter()
            .position(|token| token.is_cmavo(Cmavo::I))
            .expect("nested I exists");
        let outer_i = tokens
            .iter()
            .rposition(|token| token.is_cmavo(Cmavo::I))
            .expect("outer I exists");
        assert_ne!(inner_i, outer_i);

        assert_eq!(
            scan_for_recovery_anchor(&scan, 1, 0, &i_anchor),
            Some(outer_i),
            "an outer owner must ignore the I inside TUhE",
        );
        assert_eq!(
            scan_for_recovery_anchor(&scan, 1, 1, &i_anchor),
            Some(inner_i),
            "the nested text owner may use its same-depth I",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn co_spanned_dialect_tokens_retain_distinct_warning_anchor_indices() {
        let dialect = parse_dialect_definition("(jboponei)").expect("built-in dialect parses");
        let source = "mi cusku po do klama";
        let words = segment_words_with_modifiers_with_options_and_source_id(
            source,
            &MorphologyOptions::default().with_dialect_definition(&dialect),
            None,
        )
        .expect("valid dialect morphology");
        let options = ParseOptions::default().with_dialect_definition(&dialect);
        let tokens = syntax_tokens(&words, &options);
        let first_sibling_index = tokens
            .windows(2)
            .position(|pair| pair[0].source_spans() == pair[1].source_spans())
            .expect("jboponei po expands to co-spanned lo and su'u");
        let second_sibling_index = first_sibling_index + 1;

        assert_eq!(
            generated_warning_anchor_index(&tokens, &tokens[second_sibling_index]),
            second_sibling_index,
        );
        let state = ParserState::new(&tokens, &options);
        assert_eq!(
            state.anchor_index(&tokens[second_sibling_index]),
            second_sibling_index,
        );
    }

    #[test]
    #[ignore = "requires the owner document path in JBOTCI_ISSUE_463_REPRO"]
    #[requires(true)]
    #[ensures(true)]
    fn owner_repro_unbounded_completion_reaches_the_deep_cut() {
        run_on_fixture_worker_stack(|| {
            let path = std::env::var_os("JBOTCI_ISSUE_463_REPRO")
                .expect("set JBOTCI_ISSUE_463_REPRO to the private owner-document path");
            let source = fs::read_to_string(path).expect("owner document should be readable");
            let phrase_start = source
                .find("mukti lo nu")
                .expect("owner document should contain the issue #463 completion phrase");
            let prefix_end = phrase_start + "mukti ".len();
            let prefix = &source[..prefix_end];
            let words = segment_words_with_modifiers(prefix)
                .expect("the owner-document prefix should have valid morphology");
            let options = ParseOptions::default();
            let mut tokens = syntax_tokens(&words, &options);
            let cut_byte = tokens
                .last()
                .and_then(tokens::word_byte_range)
                .map(|range| range.end)
                .expect("the non-empty prefix should have a final syntax token");
            assert_eq!(
                cut_byte,
                phrase_start + "mukti".len(),
                "the syntax cut must follow mukti immediately before the current lo seed",
            );
            tokens.push(expected_continuation_sentinel(cut_byte));
            let sentinel_index = tokens.len() - 1;

            let started = Instant::now();
            let tracked_attempt = generated::generated_model::parse_text_detailed_tracked_attempt_for_expected_continuations(
                &tokens,
                &options,
                sentinel_index,
                None,
            );
            let data!(
                generated::generated_model::GeneratedParsedTextDetailedAttempt {
                    result,
                    trace,
                    continuation_expectations: _,
                }
            ) = tracked_attempt.into_data();
            let failure = match result {
                Ok(_) => panic!("the sentinel should fail the strict root parser"),
                Err(failure) => failure,
            };
            let recovered = recover_after_strict_failure(
                tokens,
                None,
                &options,
                failure,
                trace,
                Some(sentinel_index),
                None,
            );
            let elapsed = started.elapsed();

            assert!(
                recovered.continuation_cut_reached,
                "unbounded recovery must reach the requested completion cut",
            );
            assert_eq!(
                recovered.recovered.result.errors.len(),
                14,
                "the winning recovery must retain every prior owner-document error",
            );
            assert!(
                elapsed <= Duration::from_secs(30),
                "unbounded issue #463 recovery took {elapsed:?}; the generous ceiling guards memo-retention regressions",
            );
        });
    }

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
                    let boundary = if anchor.boundary_resync {
                        " boundary-resync"
                    } else {
                        ""
                    };
                    writeln!(
                        &mut snapshot,
                        "    resume {} origin {:?}{} start {} conditions {}",
                        anchor.resume_field,
                        anchor.origin,
                        boundary,
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
    #[requires(!field.is_empty())]
    #[ensures(true)]
    fn assert_field_anchor_is_boundary(
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
            field_metadata.anchors.iter().any(|anchor| {
                anchor.boundary_resync && anchor_tokens_contain(anchor.start_tokens, token)
            }),
            "{rule}.{field} does not mark anchor token {token:?} as a recovery boundary",
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
        assert_field_anchor_is_boundary(
            "paragraph_statement_sequence",
            "initial",
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
    fn parses_v0_joik_and_cehe_connective_cases() {
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
    fn sumti_connective_excludes_cehe_at_all_three_levels() {
        run_on_normal_stack(|| {
            for source in [
                "tu'a mi ce'e do lu'u cu broda",
                "tu'a mi ce'e bo do lu'u cu broda",
                "tu'a mi ce'e ke do ke'e lu'u cu broda",
            ] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                assert!(
                    parse_syntax_tree(&words, &ParseOptions::default()).is_err(),
                    "CEhE must not parse as a sumti connective: {source}",
                );
            }

            let tree = parse_tree_debug("mi ce'e do cu broda", &ParseOptions::default());
            assert!(tree.contains("TermsetGroup"));
            assert!(!tree.contains("SumtiConnectiveSyntax::CeheConnective"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sumti_connective_retains_all_jehi_spellings_without_warnings() {
        run_on_normal_stack(|| {
            for spelling in ["ja", "je", "je'i", "jo", "ju"] {
                let source = format!("tu'a mi {spelling} do lu'u cu broda");
                let parsed = parse_source(&source, &ParseOptions::default());
                assert!(
                    parsed.warnings.is_empty(),
                    "JEhI vocabulary-waiver spelling must remain warning-free: {source}",
                );
                assert!(
                    format!("{:?}", parsed.parse_tree).contains("JehiConnective"),
                    "JEhI spelling must retain the JEhI sumti-connective arm: {source}",
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vuhu_sumti_connective_is_warning_gated() {
        run_on_normal_stack(|| {
            let source = "tu'a mi su'i do lu'u cu broda";
            let parsed = parse_source(source, &ParseOptions::default());
            assert_warning_kind(
                source,
                &ParseOptions::default(),
                ExperimentalConstruct::ExperimentalVuhuConnective,
            );
            assert!(format!("{:?}", parsed.parse_tree).contains("ExperimentalVuhuSumtiConnective"));
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
    fn bare_nahe_sumti_without_bo_warns() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi viska na'e lo mlatu", &ParseOptions::default());
            assert!(format!("{:?}", parsed.parse_tree).contains("ScalarNegatedSumti"));
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo
            ));
            // The bare form is the sumti-oriented extension, not a term wrapper.
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalLaheNaheTermWrapper
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nahe_bo_sumti_does_not_warn() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi viska na'e bo lo mlatu", &ParseOptions::default());
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo
            ));
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalLaheNaheTermWrapper
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_nahe_sumti_without_bo_warning_anchors_nahe() {
        run_on_normal_stack(|| {
            let parsed = parse_source("mi viska na'e lo mlatu", &ParseOptions::default());
            let warning = parsed
                .warnings
                .iter()
                .find(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo
                })
                .expect("NAhE-without-BO warning");

            // Anchor covers the `na'e` token at offset 9..13 in "mi viska na'e lo mlatu".
            assert_eq!(warning_span(warning), [9, 13]);
            assert!(warning.anchor.is_selmaho(Selmaho::Nahe));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lahe_nahe_term_wrappers_warn() {
        run_on_normal_stack(|| {
            // Each of these wraps a bare term (a FA-tagged sumti or a termset) rather
            // than a sumti, which is the non-CLL term-wrapper extension. Following v0,
            // the no-`bo` NAhE term wrapper carries only the term-wrapper warning, never
            // the sumti-oriented without-`bo` warning.
            //
            // The no-`bo` case uses a termset payload (`nu'i ... nu'u`): unlike v0, v1
            // routes bare `na'e` before a FA-tagged sumti (`na'e fa do`) through the
            // flattened-tag path rather than the term wrapper, so the term wrapper is
            // reached here via a term that cannot be reinterpreted as a tag.
            for (source, anchor) in [
                ("mi tavla la'e fa do", Selmaho::Lahe),
                ("mi tavla na'e bo fa do", Selmaho::Nahe),
                ("mi tavla na'e nu'i do de nu'u", Selmaho::Nahe),
            ] {
                let parsed = parse_source(source, &ParseOptions::default());
                assert!(
                    has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalLaheNaheTermWrapper
                    ),
                    "{source}"
                );
                assert!(
                    !has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo
                    ),
                    "{source}"
                );

                let warning = parsed
                    .warnings
                    .iter()
                    .find(|warning| {
                        warning.kind == ExperimentalConstruct::ExperimentalLaheNaheTermWrapper
                    })
                    .unwrap_or_else(|| panic!("term-wrapper warning for {source}"));
                assert!(warning.anchor.is_selmaho(anchor), "{source}");
            }
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
    fn gates_zantufa_gek_termset_arm() {
        run_on_normal_stack(|| {
            // The operand runs are unbalanced, so the sourced NUhI-less `gek_termset` -- which
            // pairs one term per position -- cannot take this surface and neither can the
            // NUhI-mandatory arm. Only rolling Zantufa's `gek_term` admits it.
            let source = "ge mi ko'a gi do klama";
            let words = segment_words_with_modifiers(source).expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_source(source, &options);
            let debug_tree = format!("{:?}", parsed.parse_tree);

            assert!(debug_tree.contains("ZantufaGekTermset"), "{debug_tree}");
            assert!(has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalZantufaGekTermset
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gates_zantufa_gek_termset_arm_on_connectives_not_terms() {
        run_on_normal_stack(|| {
            let source = "ge mi ko'a gi do klama";
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let dialect =
                parse_dialect_definition("(+ZANTUFA-TERMS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            assert!(parse_syntax_tree(&words, &options).is_err());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unsourced_nuhi_termset_widenings() {
        run_on_normal_stack(|| {
            // Both widenings the optional-NUhI termset node used to carry. Neither camxes parser
            // nor rolling Zantufa accepts either surface, and rolling Zantufa has no NUhI or NUhU
            // selma'o at all, so enabling its arm does not restore them.
            for source in ["ge mi nu'u gi do klama", "nu'i ge mi gi do gi ti klama"] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                assert!(
                    parse_syntax_tree(&words, &ParseOptions::default()).is_err(),
                    "{source}"
                );

                let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                    .expect("valid dialect definition");
                let options = ParseOptions::default().with_dialect_definition(&dialect);
                assert!(parse_syntax_tree(&words, &options).is_err(), "{source}");
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_gek_termset_option_grid() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(+ZANTUFA-CONNECTIVES)")
                .expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);

            // Every row keeps the leading run unbalanced so the sourced `gek_termset` cannot
            // claim it; what varies is the branch count and the GIhI terminator.
            for (source, extra_branch_count, has_gihi) in [
                ("ge mi ko'a gi do klama", 0, false),
                ("ge mi ko'a gi do gi ti klama", 1, false),
                ("ge mi ko'a gi do gi ti gi'i klama", 1, true),
                ("ge mi ko'a gi do gi ti gi ta gi'i klama", 2, true),
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
                segment_words_with_modifiers("mi cu na'e fa klama").expect("valid morphology");

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
            let words = segment_words_with_modifiers("mi cu roi klama").expect("valid morphology");

            assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());

            let dialect =
                parse_dialect_definition("(+ZANTUFA-TAGS)").expect("valid dialect definition");
            let options = ParseOptions::default().with_dialect_definition(&dialect);
            let parsed = parse_syntax_tree(&words, &options).expect("valid recursive tag");

            assert!(
                parsed.warnings.iter().any(|warning| {
                    warning.kind == ExperimentalConstruct::ExperimentalZantufaTag
                })
            );
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
    fn zantufa_mex_priority_reaches_non_ke_extensions_without_stealing_baseline() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);
            for source in ["li pa su'i", "li pa bo re", "li pa bi'e su'i"] {
                let parsed = parse_source(source, &zantufa);
                assert!(
                    parse_tree_debug(source, &zantufa).contains("ZantufaPriorityMex"),
                    "{source}"
                );
                assert!(
                    has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalZantufaMex),
                    "{source}"
                );
            }

            let source = "li pa su'i re";
            let baseline = parse_source(source, &ParseOptions::default());
            let extended = parse_source(source, &zantufa);
            assert_eq!(extended.parse_tree, baseline.parse_tree);
            assert!(
                !has_warning_kind(&extended, ExperimentalConstruct::ExperimentalZantufaMex),
                "baseline-owned MEX must not carry a Zantufa warning"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_selbri_assignment_priority_preserves_whole_baseline_candidates() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);
            for source in ["mi broda cei brode brodi", "mi broda cei brode cei brodi"] {
                let baseline = parse_source(source, &ParseOptions::default());
                let extended = parse_source(source, &zantufa);
                assert_eq!(extended.parse_tree, baseline.parse_tree, "{source}");
                assert!(
                    !has_warning_kind(
                        &extended,
                        ExperimentalConstruct::ExperimentalZantufaSelbriAssignment,
                    ),
                    "baseline-owned CEI candidate must not warn: {source}"
                );
            }

            for source in [
                "mi broda cei brode cei na brodi",
                "mi broda cei na brode",
                "mi broda cei pu brode",
            ] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                assert!(
                    parse_syntax_tree(&words, &ParseOptions::default()).is_err(),
                    "{source}"
                );
                let parsed = parse_source(source, &zantufa);
                assert!(
                    parse_tree_debug(source, &zantufa).contains("ZantufaPriorityAssignedSelbri"),
                    "{source}"
                );
                assert!(
                    has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalZantufaSelbriAssignment,
                    ),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_selbri_relative_boundaries_and_reinterpretation_are_explicit() {
        run_on_normal_stack(|| {
            let zantufa_definition = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let fidelity_definition =
                parse_dialect_definition("(zantufa +zantufa-selbri-reinterpretation)")
                    .expect("valid fidelity dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&zantufa_definition);
            let fidelity = ParseOptions::default().with_dialect_definition(&fidelity_definition);

            let baseline_owned = parse_source("lo broda poi brode ku", &zantufa);
            assert!(!format!("{:?}", baseline_owned.parse_tree).contains("ZantufaRelativeSelbri"));
            assert!(!has_warning_kind(
                &baseline_owned,
                ExperimentalConstruct::ExperimentalZantufaSelbriRelativePlacement,
            ));

            let reinterpreted = parse_source("lo broda poi brode ku", &fidelity);
            assert!(format!("{:?}", reinterpreted.parse_tree).contains("ZantufaRelativeSelbri"));
            assert_warning_kind(
                "lo broda poi brode ku",
                &fidelity,
                ExperimentalConstruct::ExperimentalZantufaSelbriRelativePlacement,
            );

            let assignment_gap = parse_source("lo broda cei brode brodi ku", &zantufa);
            assert!(
                !format!("{:?}", assignment_gap.parse_tree)
                    .contains("ReinterpretZantufaAssignedSelbri")
            );
            let faithful_assignment = parse_source("lo broda cei brode brodi ku", &fidelity);
            let faithful_assignment_tree = format!("{:?}", faithful_assignment.parse_tree);
            assert!(
                faithful_assignment_tree.contains("ReinterpretZantufaAssignedSelbri"),
                "{faithful_assignment_tree}"
            );

            let explicit_ku = parse_source("re broda poi brode ku", &zantufa);
            assert!(format!("{:?}", explicit_ku.parse_tree).contains("ZantufaRelativeSelbri"));
            let elided_ku = parse_source("re broda poi brode", &zantufa);
            assert!(!format!("{:?}", elided_ku.parse_tree).contains("ZantufaRelativeSelbri"));

            for source in [
                "mi broda noi brode",
                "lo broda poi brode poi brodi ku",
                "coi broda poi brode do'u",
            ] {
                parse_source(source, &zantufa);
                parse_source(source, &fidelity);
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_ke_co_group_is_flat_disjoint_and_warning_bearing() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);

            // A description isolates the selbri atom from the pre-existing
            // Zantufa grouped-bridi-tail owner of top-level KE.
            let control = "lo ke broda brode ke'e ku";
            assert_eq!(
                parse_source(control, &zantufa).parse_tree,
                parse_source(control, &ParseOptions::default()).parse_tree,
            );

            for source in [
                "lo ke broda co brode co brodi ke'e ku",
                "lo ke broda co brode ke'e cei na brodi ku",
                "lo na'e ke broda co brode ke'e ku",
            ] {
                let words = segment_words_with_modifiers(source).expect("valid morphology");
                assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());
                let parsed = parse_source(source, &zantufa);
                let tree = parse_tree_debug(source, &zantufa);
                assert!(
                    tree.contains("ZantufaKeCoGroupedTanruUnit"),
                    "{source}: {tree}"
                );
                assert!(
                    has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalZantufaKeCoGrouping,
                    ),
                    "{source}"
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_mex_priority_preserves_full_baseline_operand_width() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);
            for source in [
                "li la'e pa .e re lu'u",
                "li na'e bo pa .e re lu'u",
                "li pa .e ke re ke'e",
                "li pa .e bo re",
                "li pa joi bo re",
                "li pa .e pu bo re",
            ] {
                let baseline = parse_source(source, &ParseOptions::default());
                let extended = parse_source(source, &zantufa);
                assert_eq!(extended.parse_tree, baseline.parse_tree, "{source}");
                assert!(
                    !has_warning_kind(&extended, ExperimentalConstruct::ExperimentalZantufaMex),
                    "baseline-owned operand must not carry a Zantufa warning: {source}"
                );
            }

            let trailing = "li pa .e";
            let parsed = parse_source(trailing, &zantufa);
            assert!(
                parse_tree_debug(trailing, &zantufa).contains("ZantufaPriorityMex"),
                "{trailing}"
            );
            assert!(
                has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalZantufaMex),
                "{trailing}"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_mex_priority_enforces_wide_qualified_union_policy() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);

            let elided = "li lu'e pa su'i re lo'o";
            let baseline = parse_source(elided, &ParseOptions::default());
            let extended = parse_source(elided, &zantufa);
            assert_eq!(extended.parse_tree, baseline.parse_tree);
            assert!(
                !has_warning_kind(&extended, ExperimentalConstruct::ExperimentalZantufaMex),
                "the warning union must retain the narrow baseline reading"
            );

            let explicit = segment_words_with_modifiers("li lu'e pa su'i re lu'u lo'o")
                .expect("valid morphology");
            assert!(parse_syntax_tree(&explicit, &zantufa).is_err());

            let zantufa_first = "li lu'e ke pa ke'e lo'o";
            let parsed = parse_source(zantufa_first, &zantufa);
            assert!(
                parse_tree_debug(zantufa_first, &zantufa).contains("ZantufaPriorityMex"),
                "{zantufa_first}"
            );
            assert!(
                has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalZantufaMex),
                "wide ownership must remain available when the inner starts Zantufa-only"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_mex_priority_keeps_wide_qualifiers_with_zantufa_only_inner_material() {
        run_on_normal_stack(|| {
            let dialect = parse_dialect_definition("(zantufa)").expect("valid dialect");
            let zantufa = ParseOptions::default().with_dialect_definition(&dialect);

            for source in [
                "li lu'e pa bo ci lo'o",
                "li lu'e pa su'i re bo ci lo'o",
                "li lu'e pa su'i re .e lo'o",
                "li na'e bo pa bo ci lo'o",
            ] {
                let parsed = parse_source(source, &zantufa);
                let tree = parse_tree_debug(source, &zantufa);
                assert!(tree.contains("ZantufaPriorityMex"), "{source}: {tree}");
                assert!(
                    tree.contains("ZantufaLaheQualifiedMeksoOperand")
                        || tree.contains("ZantufaNaheBoQualifiedMeksoOperand"),
                    "wide qualifier must retain ownership: {source}: {tree}"
                );
                assert!(
                    has_warning_kind(&parsed, ExperimentalConstruct::ExperimentalZantufaMex),
                    "wide Zantufa reading must warn: {source}"
                );
            }
        });
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
    fn relative_continuation_classifier_preserves_baseline_zihe_ownership() {
        run_on_normal_stack(|| {
            let source = "lo gerku poi ke'a barda zi'e noi ke'a melbi cu klama";
            let parsed = parse_source(source, &ParseOptions::default());
            let tree = format!("{:?}", parsed.parse_tree);
            assert!(tree.contains("JoinedRelativeClauseTail"), "{tree}");
            assert!(!tree.contains("RelativeClauseExpContinuation"), "{tree}");
            assert!(!has_warning_kind(
                &parsed,
                ExperimentalConstruct::ExperimentalRelativeClauseConnective,
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_relative_continuation_accepts_exact_connective_shape() {
        run_on_normal_stack(|| {
            for source in [
                "lo gerku poi ke'a barda ja noi ke'a melbi cu klama",
                "lo gerku poi ke'a barda .e noi ke'a melbi cu klama",
                "lo gerku poi ke'a barda na ja nai noi ke'a melbi cu klama",
                "lo gerku poi ke'a barda ja voi'e lo mlatu cu klama",
                "lo gerku poi ke'a barda ja po'oi lo mlatu cu klama",
                "lo gerku poi ke'a barda ja sei mi cusku noi ke'a melbi cu klama",
            ] {
                let parsed = parse_source(source, &ParseOptions::default());
                let tree = format!("{:?}", parsed.parse_tree);
                assert!(
                    tree.contains("RelativeClauseExpContinuation"),
                    "{source}: {tree}"
                );
                assert!(
                    has_warning_kind(
                        &parsed,
                        ExperimentalConstruct::ExperimentalRelativeClauseConnective,
                    ),
                    "{source}",
                );
            }
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vuho_ownership_is_consumer_sensitive_and_warning_gated() {
        run_on_normal_stack(|| {
            let baseline = parse_source(
                "mi viska lo gerku vu'o poi ke'a barda",
                &ParseOptions::default(),
            );
            let baseline_tree = format!("{:?}", baseline.parse_tree);
            assert!(
                baseline_tree.contains("VuhoRelativeSumtiAttachmentTail"),
                "{baseline_tree}",
            );
            assert!(!has_warning_kind(
                &baseline,
                ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
            ));

            let top_level = parse_source(
                "mi viska lo gerku vu'o poi ke'a barda ku'o .e lo mlatu",
                &ParseOptions::default(),
            );
            let top_level_tree = format!("{:?}", top_level.parse_tree);
            assert!(
                top_level_tree.contains("VuhoRelativeSumtiAttachmentTail"),
                "{top_level_tree}",
            );
            assert!(top_level_tree.contains("ConnectedTerm"), "{top_level_tree}");
            assert!(!has_warning_kind(
                &top_level,
                ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
            ));

            let elided = parse_source(
                "mi viska la'e lo gerku vu'o poi ke'a barda ku'o .e lo mlatu",
                &ParseOptions::default(),
            );
            let elided_tree = format!("{:?}", elided.parse_tree);
            assert!(
                elided_tree.contains("VuhoRelativeSumtiAttachmentTail"),
                "{elided_tree}",
            );
            assert!(!has_warning_kind(
                &elided,
                ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
            ));

            let explicit_source =
                "mi viska la'e lo gerku vu'o poi ke'a barda ku'o .e lo mlatu lu'u";
            let explicit = parse_source(explicit_source, &ParseOptions::default());
            let explicit_tree = format!("{:?}", explicit.parse_tree);
            assert!(
                explicit_tree.contains("ExperimentalVuhoScopedSumtiAttachmentTail"),
                "{explicit_tree}",
            );
            assert!(
                has_warning_kind(
                    &explicit,
                    ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
                ),
                "{explicit_source}",
            );

            let bare = parse_source("mi viska lo gerku vu'o", &ParseOptions::default());
            assert!(
                format!("{:?}", bare.parse_tree)
                    .contains("ExperimentalBareVuhoSumtiAttachmentTail")
            );
            assert!(has_warning_kind(
                &bare,
                ExperimentalConstruct::ExperimentalVuhoScopedAttachment,
            ));

            let direct =
                segment_words_with_modifiers("mi viska la'e lo gerku vu'o .e lo mlatu lu'u")
                    .expect("valid morphology");
            assert!(
                parse_syntax_tree(&direct, &ParseOptions::default()).is_err(),
                "camxes-exp does not source a VUhO continuation without preceding relatives",
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nahe_bo_accepts_pre_inner_relative_clauses_without_changing_grouping() {
        run_on_normal_stack(|| {
            let nahe = parse_tree_debug(
                "mi viska na'e bo poi ke'a barda ku'o lo gerku lu'u",
                &ParseOptions::default(),
            );
            let lahe = parse_tree_debug(
                "mi viska la'e poi ke'a barda ku'o lo gerku lu'u",
                &ParseOptions::default(),
            );
            assert!(nahe.contains("ScalarNegatedSumtiWithBo"), "{nahe}");
            assert!(nahe.contains("RelativeClauseListSyntax"), "{nahe}");
            assert!(lahe.contains("LaheSumti"), "{lahe}");
            assert!(lahe.contains("RelativeClauseListSyntax"), "{lahe}");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn forethought_relative_clause_connective_remains_rejected() {
        let words =
            segment_words_with_modifiers("lo gerku ge poi ke'a barda gi poi ke'a melbi cu klama")
                .expect("valid morphology");
        assert!(parse_syntax_tree(&words, &ParseOptions::default()).is_err());
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
