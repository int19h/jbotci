//! Packrat tables for the morphology recognizers that call one another.
//!
//! A stored result is the value the body returned. Nothing is cut off by
//! depth, steps, or time. A recognizer does not call itself on the same key
//! before the result is stored. A later call to the same function either uses
//! a later start, an earlier end, or it runs after the first call has stored
//! its result. The tables do not record an in-progress mark. `get` returns
//! `None` both before the body runs and while the body is still running.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_interrupt::CancellationToken;

use super::{FuhivlaHeadPolicy, LujvoParseFailure, LujvoPartRange};

#[cfg(test)]
thread_local! {
    static BODY_COUNT: Cell<u64> = const { Cell::new(0) };
    static BODY_CAP: Cell<u64> = const { Cell::new(u64::MAX) };
    static SYLLABLE_STEPS: Cell<u64> = const { Cell::new(0) };
    static SYLLABLE_CAP: Cell<u64> = const { Cell::new(u64::MAX) };
    static LIVE_SLOTS: Cell<u64> = const { Cell::new(0) };
    static PEAK_SLOTS: Cell<u64> = const { Cell::new(0) };
}

#[requires(true)]
#[ensures(true)]
fn note_recognition_body() {
    // The counter exists so a test can prove the recognizer stays polynomial.
    // Production builds do not read it. The cap is a test assertion checked at
    // the moment a body finishes: an exponential run fails there instead of
    // continuing. It is not a limit on the recognizer.
    #[cfg(test)]
    {
        let next = BODY_COUNT.with(|count| {
            let next = count.get().saturating_add(1);
            count.set(next);
            next
        });
        let cap = BODY_CAP.with(|cap| cap.get());
        if next > cap {
            panic!(
                "morphology recognition executed {next} memoized bodies, above the polynomial cap {cap}"
            );
        }
    }
}

/// Counts one index visited by the descending syllable scan.
///
/// The cap is a test assertion, like [`note_recognition_body`]. Production
/// builds do not read it, and it is not a limit on the scan.
#[requires(true)]
#[ensures(true)]
pub(crate) fn note_syllable_step() {
    #[cfg(test)]
    {
        let next = SYLLABLE_STEPS.with(|count| {
            let next = count.get().saturating_add(1);
            count.set(next);
            next
        });
        let cap = SYLLABLE_CAP.with(|cap| cap.get());
        if next > cap {
            panic!("syllable scan visited {next} indexes, above the polynomial cap {cap}");
        }
    }
}

/// Test guard for the body-execution counter.
///
/// The cap is restored when the guard drops, including after a panic, so a
/// later test on the same thread is not held to this test's budget.
#[cfg(test)]
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct RecognitionBodyCount {
    previous_cap: u64,
}

#[cfg(test)]
impl RecognitionBodyCount {
    #[requires(true)]
    #[ensures(ret.get() == 0)]
    pub(crate) fn begin(cap: u64) -> Self {
        let previous_cap = BODY_CAP.with(|cap_cell| {
            let previous = cap_cell.get();
            cap_cell.set(cap);
            previous
        });
        BODY_COUNT.with(|count| count.set(0));
        // `#[invariant(true)]` leaves this as an ordinary struct: every saved cap is valid.
        Self { previous_cap }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> u64 {
        BODY_COUNT.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for RecognitionBodyCount {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let previous_cap = self.previous_cap;
        BODY_CAP.with(|cap| cap.set(previous_cap));
        BODY_COUNT.with(|count| count.set(0));
    }
}

/// Test guard for the syllable-scan counter.
#[cfg(test)]
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct SyllableStepCount {
    previous_cap: u64,
}

#[cfg(test)]
impl SyllableStepCount {
    #[requires(true)]
    #[ensures(ret.get() == 0)]
    pub(crate) fn begin(cap: u64) -> Self {
        let previous_cap = SYLLABLE_CAP.with(|cap_cell| {
            let previous = cap_cell.get();
            cap_cell.set(cap);
            previous
        });
        SYLLABLE_STEPS.with(|count| count.set(0));
        Self { previous_cap }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> u64 {
        SYLLABLE_STEPS.with(|count| count.get())
    }
}

#[cfg(test)]
impl Drop for SyllableStepCount {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let previous_cap = self.previous_cap;
        SYLLABLE_CAP.with(|cap| cap.set(previous_cap));
        SYLLABLE_STEPS.with(|count| count.set(0));
    }
}

/// Test guard for the live hash-map capacity of every cache on this thread.
///
/// `begin` sets the peak watermark to the capacity already live, and `get`
/// returns the high-water mark after that. Drop restores the previous
/// watermark. It does not zero the live total, because a cache created
/// outside the guard is still alive.
#[cfg(test)]
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct RecognitionSlotPeak {
    previous_peak: u64,
}

#[cfg(test)]
impl RecognitionSlotPeak {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn begin() -> Self {
        let previous_peak = PEAK_SLOTS.with(|peak| {
            let previous = peak.get();
            let live = LIVE_SLOTS.with(|live| live.get());
            peak.set(live);
            previous
        });
        Self { previous_peak }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> u64 {
        PEAK_SLOTS.with(|peak| peak.get())
    }
}

#[cfg(test)]
impl Drop for RecognitionSlotPeak {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let previous_peak = self.previous_peak;
        PEAK_SLOTS.with(|peak| peak.set(previous_peak));
    }
}

#[invariant(start <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SpanKey {
    start: usize,
    end: usize,
}

#[invariant(start <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct InitialRafsiKey {
    start: usize,
    end: usize,
    has_initial_rafsi: bool,
}

#[invariant(start <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct YPolicyKey {
    start: usize,
    end: usize,
    allows_y: bool,
}

#[invariant(start <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FuhivlaHeadKey {
    start: usize,
    end: usize,
    policy: FuhivlaHeadPolicy,
}

#[invariant(prefix_start <= prefix_end && prefix_end <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PrefixEndKey {
    prefix_start: usize,
    prefix_end: usize,
    end: usize,
}

#[invariant(start <= end && end <= lookahead_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LookaheadKey {
    start: usize,
    end: usize,
    lookahead_end: usize,
}

#[invariant(start <= end && end <= context_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StressContextKey {
    start: usize,
    end: usize,
    context_end: usize,
}

/// Ranges from one `lujvo_part_ranges_from_with_cache` call, plus the
/// greatest-index failure that call recorded on its own.
///
/// The failure is the subtree's own maximum. A hit merges it into the caller.
/// It does not include a failure the caller already held.
#[invariant(ranges.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[derive(Debug, Clone)]
pub(super) struct LujvoPartRangeMemo {
    pub(super) ranges: Option<Rc<[LujvoPartRange]>>,
    pub(super) failure: Option<LujvoParseFailure>,
}

#[invariant(prefix_start <= prefix_end && prefix_end <= index && index <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct IndicatorBoundaryKey {
    prefix_start: usize,
    prefix_end: usize,
    index: usize,
    end: usize,
}

/// Memoized results for every recognizer in the stressed-word cycle.
///
/// Every table is a hash map. `new` stores no keys and allocates no square.
/// A map grows only when a body stores a key, so the memory of one cache is
/// proportional to the entries that run fills. The time bounds in this comment
/// are expected time: hash map lookup and insert are expected constant time,
/// not a worst-case bound.
///
/// `chars.len()` is not bounded. `Grammar::candidate_end` walks forward until
/// it sees a word separator, and one cache is sized to that whole span. The
/// span can be as long as the input. Callers reuse one cache across the
/// suffixes of an unspaced span, so a long run does not keep a cache per
/// suffix.
///
/// Key space for a candidate of `n` characters, with `m = n + 1`:
///
/// - Two-index keys are `O(m^2)`. The plain maps are `cmavo_word`,
///   `lojban_word`, `brivla_word`, `lujvo_slice`, `fuhivla_shape`,
///   `experimental_cgv`, `slinkuhi`, `rafsi_string_starts`, `post_word`,
///   `indicator_cmavo_starts`, `initial_rafsi_ends`, `extended_rafsi_ends`,
///   `any_extended_rafsi_starts`, `fuhivla_rafsi_base_ends`,
///   `stressed_extended_rafsi_ends`, `nucleus_has_implicit_stress`,
///   `stress_tail_reaches_boundary`, `greedy_rafsi_chain`, `prefix_post_word`,
///   `prefix_lojban_word`, and `cvcy_core_before`.
/// - `lujvo_from` and `lujvo_part_ranges` add `has_initial_rafsi`,
///   `fuhivla_shape_y` adds `allows_y`, and `fuhivla_head_policy` adds
///   `FuhivlaHeadPolicy`. Each flag has a constant number of values, so those
///   maps are still `O(m^2)`.
/// - Three-index keys are `O(m^3)`: `rafsi_string_from`, `rafsi_string_ending`,
///   `cmavo_post_word`, `nucleus_is_stressed`, `y_less_rafsi_unstressed`,
///   `cvv_rafsi_unstressed`, `brivla_head_ends`, and `fuhivla_head_ends`.
///   The two head maps store the span end and the lookahead end. A fu'ivla
///   rafsi lookahead reads past its `y`.
/// - Four-index keys are `O(m^4)`: `indicator_boundary`.
///
/// Those sizes are the key domains: every index combination a map can store.
/// A run stores the keys it reaches. In the measured families that reached
/// set stays quadratic. A stress-context span is one nucleus or one rafsi.
/// A `cmavo_post_word` prefix is one dictionary cmavo. An
/// `indicator_boundary` prefix is one indicator cmavo, and its index is the
/// next non-comma position. `rafsi_string_from` and `rafsi_string_ending`
/// are reached with the span end as the lookahead. The domain of those maps
/// stays cubic or quartic. The entries one run writes do not fill that domain.
///
/// Each memoized body runs once per reached key.
///
/// `lujvo_part_ranges_from_with_cache` is memoized on
/// `(index, end, has_initial_rafsi)`. The stored value is the range list and
/// the greatest-index failure that call recorded. A hit merges that failure
/// into the caller with the same greater-index rule. The bound is the key
/// space. One body runs per key, and that body reads the memoized initial
/// rafsi list.
///
/// `greedy_rafsi_chain` stores the greedy y-less cursors for
/// `(start, lookahead)`. `rafsi_string_from_with_cache` reads the cursor that
/// the same walk reaches for its own `end`. One chain is built per pair. A
/// later word end with that lookahead does not repeat the greedy steps.
///
/// `nucleus_is_stressed_in_context`, `y_less_rafsi_is_unstressed_in_context`,
/// and `cvv_rafsi_is_unstressed_in_context` are three-index keys.
/// `nucleus_has_implicit_stress` and `stress_tail_reaches_boundary` are
/// two-index keys. A coda query visits the syllabic-consonant run its flags
/// depend on, then stops. On `lo` that run is one character. A long run still
/// visits each character of the run once per uncached query.
/// `consonantal_chain_then_final` and `consonantal_chain_then_onset` follow
/// one successor per step. One chain takes `O(n)` steps, and each step scans
/// only its own run. A run as long as the candidate makes one chain `O(n^2)`.
///
/// The initial, extended, and stressed rafsi lists are memoized on their full
/// arguments. One body runs per key. The body walks those arguments once and
/// then reads the other memoized tables.
///
/// The bounds count work. They are not a cutoff. Every key still returns the
/// body's own answer.
#[invariant(*char_count < usize::MAX)]
#[derive(Debug)]
pub(crate) struct MorphologyRecognitionCache {
    pub(crate) char_count: usize,
    /// Drop hook for the test slot counter.
    ///
    /// The cache wrapper cannot implement `Drop`. Its invariant generates
    /// `into_data`, and that moves the inner value out of the wrapper.
    accounted: CacheSlotAccount,
    lujvo_from: RefCell<HashMap<InitialRafsiKey, bool>>,
    cmavo_word: RefCell<HashMap<SpanKey, bool>>,
    lojban_word: RefCell<HashMap<SpanKey, bool>>,
    brivla_word: RefCell<HashMap<SpanKey, bool>>,
    lujvo_slice: RefCell<HashMap<SpanKey, bool>>,
    fuhivla_shape_y: RefCell<HashMap<YPolicyKey, bool>>,
    fuhivla_shape: RefCell<HashMap<SpanKey, bool>>,
    fuhivla_head_policy: RefCell<HashMap<FuhivlaHeadKey, bool>>,
    experimental_cgv: RefCell<HashMap<SpanKey, bool>>,
    fuhivla_head_ends: RefCell<HashMap<LookaheadKey, Rc<[usize]>>>,
    brivla_head_ends: RefCell<HashMap<LookaheadKey, Rc<[usize]>>>,
    slinkuhi: RefCell<HashMap<SpanKey, bool>>,
    rafsi_string_starts: RefCell<HashMap<SpanKey, bool>>,
    rafsi_string_from: RefCell<HashMap<LookaheadKey, bool>>,
    rafsi_string_ending: RefCell<HashMap<LookaheadKey, bool>>,
    post_word: RefCell<HashMap<SpanKey, bool>>,
    indicator_cmavo_starts: RefCell<HashMap<SpanKey, bool>>,
    cmavo_post_word: RefCell<HashMap<PrefixEndKey, bool>>,
    indicator_boundary: RefCell<HashMap<IndicatorBoundaryKey, bool>>,
    initial_rafsi_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    extended_rafsi_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    any_extended_rafsi_starts: RefCell<HashMap<SpanKey, bool>>,
    fuhivla_rafsi_base_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    stressed_extended_rafsi_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    nucleus_has_implicit_stress: RefCell<HashMap<SpanKey, bool>>,
    stress_tail_reaches_boundary: RefCell<HashMap<SpanKey, bool>>,
    greedy_rafsi_chain: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    lujvo_part_ranges: RefCell<HashMap<InitialRafsiKey, LujvoPartRangeMemo>>,
    nucleus_is_stressed: RefCell<HashMap<StressContextKey, bool>>,
    y_less_rafsi_unstressed: RefCell<HashMap<StressContextKey, bool>>,
    cvv_rafsi_unstressed: RefCell<HashMap<StressContextKey, bool>>,
    /// Prefix `post_word`: `pause / !nucleus lojban_word`.
    /// A brivla in that word can end before the slice. This is not `post_word`.
    /// A true result here must not satisfy a caller that needs the next word
    /// to consume the whole slice.
    prefix_post_word: RefCell<HashMap<SpanKey, bool>>,
    /// Prefix `lojban_word`: `cmevla / cmavo / brivla`.
    /// The cmavo arm continues with `prefix_post_word`. This is not `lojban_word`.
    prefix_lojban_word: RefCell<HashMap<SpanKey, bool>>,
    /// `initial_rafsi* brivla_core` ending strictly before this span.
    cvcy_core_before: RefCell<HashMap<SpanKey, bool>>,
    /// Shared with the admission that owns this segmentation.
    ///
    /// A recognizer loads this flag once at the start of each memoized body.
    /// `record_*` refuses to insert after the flag is set, so a cancelled
    /// answer is not stored even in this per-call table.
    cancellation: CancellationToken,
}

impl MorphologyRecognitionCache {
    #[requires(char_count < usize::MAX)]
    #[ensures(ret.char_count == char_count)]
    #[ensures(!ret.is_cancelled())]
    #[ensures(ret.allocated_slots_for_contract() == 0)]
    pub(crate) fn new(char_count: usize) -> Self {
        Self::with_cancellation(char_count, CancellationToken::never())
    }

    #[requires(char_count < usize::MAX)]
    #[ensures(ret.char_count == char_count)]
    #[ensures(ret.allocated_slots_for_contract() == 0)]
    pub(crate) fn with_cancellation(char_count: usize, cancellation: CancellationToken) -> Self {
        let cache = new!(MorphologyRecognitionCache {
            char_count: char_count,
            accounted: CacheSlotAccount::new(),
            cancellation: cancellation,
            lujvo_from: RefCell::new(HashMap::new()),
            cmavo_word: RefCell::new(HashMap::new()),
            lojban_word: RefCell::new(HashMap::new()),
            brivla_word: RefCell::new(HashMap::new()),
            lujvo_slice: RefCell::new(HashMap::new()),
            fuhivla_shape_y: RefCell::new(HashMap::new()),
            fuhivla_shape: RefCell::new(HashMap::new()),
            fuhivla_head_policy: RefCell::new(HashMap::new()),
            experimental_cgv: RefCell::new(HashMap::new()),
            fuhivla_head_ends: RefCell::new(HashMap::new()),
            brivla_head_ends: RefCell::new(HashMap::new()),
            slinkuhi: RefCell::new(HashMap::new()),
            rafsi_string_starts: RefCell::new(HashMap::new()),
            rafsi_string_from: RefCell::new(HashMap::new()),
            rafsi_string_ending: RefCell::new(HashMap::new()),
            post_word: RefCell::new(HashMap::new()),
            indicator_cmavo_starts: RefCell::new(HashMap::new()),
            cmavo_post_word: RefCell::new(HashMap::new()),
            indicator_boundary: RefCell::new(HashMap::new()),
            initial_rafsi_ends: RefCell::new(HashMap::new()),
            extended_rafsi_ends: RefCell::new(HashMap::new()),
            any_extended_rafsi_starts: RefCell::new(HashMap::new()),
            fuhivla_rafsi_base_ends: RefCell::new(HashMap::new()),
            stressed_extended_rafsi_ends: RefCell::new(HashMap::new()),
            nucleus_has_implicit_stress: RefCell::new(HashMap::new()),
            stress_tail_reaches_boundary: RefCell::new(HashMap::new()),
            greedy_rafsi_chain: RefCell::new(HashMap::new()),
            lujvo_part_ranges: RefCell::new(HashMap::new()),
            nucleus_is_stressed: RefCell::new(HashMap::new()),
            y_less_rafsi_unstressed: RefCell::new(HashMap::new()),
            cvv_rafsi_unstressed: RefCell::new(HashMap::new()),
            prefix_post_word: RefCell::new(HashMap::new()),
            prefix_lojban_word: RefCell::new(HashMap::new()),
            cvcy_core_before: RefCell::new(HashMap::new()),
        });
        cache.sync_accounted();
        cache
    }

    /// One acquire load. Hot recognizers call this before doing any search.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    #[requires(true)]
    #[ensures(true)]
    fn sync_accounted(&self) {
        // Capacity is the number of hash slots, not the number of stored
        // keys. The live total is what a test compares with the old dense
        // squares. Production builds do not read it.
        #[cfg(test)]
        {
            let next = self.total_capacity();
            let previous = self.accounted.slots.get();
            self.accounted.slots.set(next);
            LIVE_SLOTS.with(|live| {
                let updated = live.get().saturating_sub(previous).saturating_add(next);
                live.set(updated);
                PEAK_SLOTS.with(|peak| {
                    if updated > peak.get() {
                        peak.set(updated);
                    }
                });
            });
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn total_capacity(&self) -> u64 {
        #[requires(true)]
        #[ensures(ret == u64::try_from(table.borrow().capacity()).unwrap_or(u64::MAX))]
        fn slots<K, V>(table: &RefCell<HashMap<K, V>>) -> u64 {
            u64::try_from(table.borrow().capacity()).unwrap_or(u64::MAX)
        }
        slots(&self.lujvo_from)
            .saturating_add(slots(&self.cmavo_word))
            .saturating_add(slots(&self.lojban_word))
            .saturating_add(slots(&self.brivla_word))
            .saturating_add(slots(&self.lujvo_slice))
            .saturating_add(slots(&self.fuhivla_shape_y))
            .saturating_add(slots(&self.fuhivla_shape))
            .saturating_add(slots(&self.fuhivla_head_policy))
            .saturating_add(slots(&self.experimental_cgv))
            .saturating_add(slots(&self.fuhivla_head_ends))
            .saturating_add(slots(&self.brivla_head_ends))
            .saturating_add(slots(&self.slinkuhi))
            .saturating_add(slots(&self.rafsi_string_starts))
            .saturating_add(slots(&self.rafsi_string_from))
            .saturating_add(slots(&self.rafsi_string_ending))
            .saturating_add(slots(&self.post_word))
            .saturating_add(slots(&self.indicator_cmavo_starts))
            .saturating_add(slots(&self.cmavo_post_word))
            .saturating_add(slots(&self.indicator_boundary))
            .saturating_add(slots(&self.initial_rafsi_ends))
            .saturating_add(slots(&self.extended_rafsi_ends))
            .saturating_add(slots(&self.any_extended_rafsi_starts))
            .saturating_add(slots(&self.fuhivla_rafsi_base_ends))
            .saturating_add(slots(&self.stressed_extended_rafsi_ends))
            .saturating_add(slots(&self.nucleus_has_implicit_stress))
            .saturating_add(slots(&self.stress_tail_reaches_boundary))
            .saturating_add(slots(&self.greedy_rafsi_chain))
            .saturating_add(slots(&self.lujvo_part_ranges))
            .saturating_add(slots(&self.nucleus_is_stressed))
            .saturating_add(slots(&self.y_less_rafsi_unstressed))
            .saturating_add(slots(&self.cvv_rafsi_unstressed))
            .saturating_add(slots(&self.prefix_post_word))
            .saturating_add(slots(&self.prefix_lojban_word))
            .saturating_add(slots(&self.cvcy_core_before))
    }

    /// Empty maps have capacity 0. The ensures on `new` evaluates this in
    /// every build, so the postcondition is the real slot total.
    #[requires(true)]
    #[ensures(ret == self.total_capacity())]
    fn allocated_slots_for_contract(&self) -> u64 {
        self.total_capacity()
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(ret == self.total_capacity())]
    pub(crate) fn allocated_slots(&self) -> u64 {
        self.total_capacity()
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lujvo_from(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
    ) -> Option<bool> {
        let key = new!(InitialRafsiKey {
            start: index,
            end: end,
            has_initial_rafsi: has_initial_rafsi,
        });
        self.lujvo_from.borrow().get(&key).copied()
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.lujvo_from(index, end, has_initial_rafsi) == Some(result))]
    pub(crate) fn record_lujvo_from(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
        result: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(InitialRafsiKey {
            start: index,
            end: end,
            has_initial_rafsi: has_initial_rafsi,
        });
        self.lujvo_from.borrow_mut().insert(key, result);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn cmavo_word(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.cmavo_word, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.cmavo_word(start, end) == Some(value))]
    pub(crate) fn record_cmavo_word(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.cmavo_word, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lojban_word(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.lojban_word, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.lojban_word(start, end) == Some(value))]
    pub(crate) fn record_lojban_word(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.lojban_word, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn brivla_word(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.brivla_word, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.brivla_word(start, end) == Some(value))]
    pub(crate) fn record_brivla_word(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.brivla_word, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lujvo_slice(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.lujvo_slice, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.lujvo_slice(start, end) == Some(value))]
    pub(crate) fn record_lujvo_slice(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.lujvo_slice, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_shape_y(&self, start: usize, end: usize, allows_y: bool) -> Option<bool> {
        let key = new!(YPolicyKey {
            start: start,
            end: end,
            allows_y: allows_y,
        });
        self.fuhivla_shape_y.borrow().get(&key).copied()
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.fuhivla_shape_y(start, end, allows_y) == Some(value))]
    pub(crate) fn record_fuhivla_shape_y(
        &self,
        start: usize,
        end: usize,
        allows_y: bool,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(YPolicyKey {
            start: start,
            end: end,
            allows_y: allows_y,
        });
        self.fuhivla_shape_y.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_shape(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.fuhivla_shape, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.fuhivla_shape(start, end) == Some(value))]
    pub(crate) fn record_fuhivla_shape(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.fuhivla_shape, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_head_policy(
        &self,
        start: usize,
        end: usize,
        policy: FuhivlaHeadPolicy,
    ) -> Option<bool> {
        let key = new!(FuhivlaHeadKey {
            start: start,
            end: end,
            policy: policy,
        });
        self.fuhivla_head_policy.borrow().get(&key).copied()
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.fuhivla_head_policy(start, end, policy) == Some(value))]
    pub(crate) fn record_fuhivla_head_policy(
        &self,
        start: usize,
        end: usize,
        policy: FuhivlaHeadPolicy,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(FuhivlaHeadKey {
            start: start,
            end: end,
            policy: policy,
        });
        self.fuhivla_head_policy.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn experimental_cgv(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.experimental_cgv, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.experimental_cgv(start, end) == Some(value))]
    pub(crate) fn record_experimental_cgv(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.experimental_cgv, start, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_head_ends(
        &self,
        index: usize,
        end: usize,
        lookahead_end: usize,
    ) -> Option<Rc<[usize]>> {
        lookahead_head_get(&self.fuhivla_head_ends, index, end, lookahead_end)
    }

    #[requires(index <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.fuhivla_head_ends(index, end, lookahead_end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_fuhivla_head_ends(
        &self,
        index: usize,
        end: usize,
        lookahead_end: usize,
        value: &[usize],
    ) -> Rc<[usize]> {
        if self.is_cancelled() {
            // Do not store a head list for a cancelled recognition.
            return Rc::from(value);
        }
        note_recognition_body();
        let stored = lookahead_head_set(&self.fuhivla_head_ends, index, end, lookahead_end, value);
        self.sync_accounted();
        stored
    }

    /// Store `ends` itself. The fu'ivla miss path that copies a brivla head
    /// list uses this so both tables keep one allocation.
    #[requires(index <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(Rc::ptr_eq(&ret, &ends))]
    #[ensures(self.is_cancelled() || self.fuhivla_head_ends(index, end, lookahead_end).is_some_and(|stored| Rc::ptr_eq(&stored, &ends)))]
    pub(crate) fn record_fuhivla_head_ends_shared(
        &self,
        index: usize,
        end: usize,
        lookahead_end: usize,
        ends: Rc<[usize]>,
    ) -> Rc<[usize]> {
        if self.is_cancelled() {
            // Hand the list back without putting it in the table.
            return Rc::clone(&ends);
        }
        note_recognition_body();
        let key = new!(LookaheadKey {
            start: index,
            end: end,
            lookahead_end: lookahead_end,
        });
        self.fuhivla_head_ends
            .borrow_mut()
            .insert(key, Rc::clone(&ends));
        self.sync_accounted();
        Rc::clone(&ends)
    }

    #[requires(index <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn brivla_head_ends(
        &self,
        index: usize,
        end: usize,
        lookahead_end: usize,
    ) -> Option<Rc<[usize]>> {
        lookahead_head_get(&self.brivla_head_ends, index, end, lookahead_end)
    }

    #[requires(index <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.brivla_head_ends(index, end, lookahead_end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_brivla_head_ends(
        &self,
        index: usize,
        end: usize,
        lookahead_end: usize,
        value: &[usize],
    ) -> Rc<[usize]> {
        if self.is_cancelled() {
            return Rc::from(value);
        }
        note_recognition_body();
        let stored = lookahead_head_set(&self.brivla_head_ends, index, end, lookahead_end, value);
        self.sync_accounted();
        stored
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn slinkuhi(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.slinkuhi, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.slinkuhi(start, end) == Some(value))]
    pub(crate) fn record_slinkuhi(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.slinkuhi, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn rafsi_string_starts(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.rafsi_string_starts, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.rafsi_string_starts(start, end) == Some(value))]
    pub(crate) fn record_rafsi_string_starts(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.rafsi_string_starts, start, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn rafsi_string_from(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
    ) -> Option<bool> {
        let key = new!(LookaheadKey {
            start: start,
            end: end,
            lookahead_end: lookahead_end,
        });
        self.rafsi_string_from.borrow().get(&key).copied()
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.rafsi_string_from(start, end, lookahead_end) == Some(value))]
    pub(crate) fn record_rafsi_string_from(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(LookaheadKey {
            start: start,
            end: end,
            lookahead_end: lookahead_end,
        });
        self.rafsi_string_from.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn rafsi_string_ending(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
    ) -> Option<bool> {
        let key = new!(LookaheadKey {
            start: start,
            end: end,
            lookahead_end: lookahead_end,
        });
        self.rafsi_string_ending.borrow().get(&key).copied()
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.rafsi_string_ending(start, end, lookahead_end) == Some(value))]
    pub(crate) fn record_rafsi_string_ending(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(LookaheadKey {
            start: start,
            end: end,
            lookahead_end: lookahead_end,
        });
        self.rafsi_string_ending.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn post_word(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.post_word, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.post_word(index, end) == Some(value))]
    pub(crate) fn record_post_word(&self, index: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.post_word, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn prefix_post_word(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.prefix_post_word, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.prefix_post_word(index, end) == Some(value))]
    pub(crate) fn record_prefix_post_word(&self, index: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.prefix_post_word, index, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn prefix_lojban_word(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.prefix_lojban_word, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.prefix_lojban_word(start, end) == Some(value))]
    pub(crate) fn record_prefix_lojban_word(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.prefix_lojban_word, start, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn cvcy_core_before(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.cvcy_core_before, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.cvcy_core_before(index, end) == Some(value))]
    pub(crate) fn record_cvcy_core_before(&self, index: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.cvcy_core_before, index, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn indicator_cmavo_starts(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.indicator_cmavo_starts, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.indicator_cmavo_starts(start, end) == Some(value))]
    pub(crate) fn record_indicator_cmavo_starts(&self, start: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.indicator_cmavo_starts, start, end, value);
        self.sync_accounted();
    }

    #[requires(prefix_start <= prefix_end && prefix_end <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn cmavo_post_word(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        end: usize,
    ) -> Option<bool> {
        let key = new!(PrefixEndKey {
            prefix_start: prefix_start,
            prefix_end: prefix_end,
            end: end,
        });
        self.cmavo_post_word.borrow().get(&key).copied()
    }

    #[requires(prefix_start <= prefix_end && prefix_end <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.cmavo_post_word(prefix_start, prefix_end, end) == Some(value))]
    pub(crate) fn record_cmavo_post_word(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(PrefixEndKey {
            prefix_start: prefix_start,
            prefix_end: prefix_end,
            end: end,
        });
        self.cmavo_post_word.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(prefix_start <= prefix_end && prefix_end <= index && index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn indicator_boundary(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        index: usize,
        end: usize,
    ) -> Option<bool> {
        let key = new!(IndicatorBoundaryKey {
            prefix_start: prefix_start,
            prefix_end: prefix_end,
            index: index,
            end: end,
        });
        self.indicator_boundary.borrow().get(&key).copied()
    }

    #[requires(prefix_start <= prefix_end && prefix_end <= index && index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.indicator_boundary(prefix_start, prefix_end, index, end) == Some(value))]
    pub(crate) fn record_indicator_boundary(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        index: usize,
        end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(IndicatorBoundaryKey {
            prefix_start: prefix_start,
            prefix_end: prefix_end,
            index: index,
            end: end,
        });
        self.indicator_boundary.borrow_mut().insert(key, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn initial_rafsi_ends(&self, index: usize, end: usize) -> Option<Rc<[usize]>> {
        head_get(&self.initial_rafsi_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.initial_rafsi_ends(index, end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_initial_rafsi_ends(&self, index: usize, end: usize, value: &[usize]) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        head_set(&self.initial_rafsi_ends, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn extended_rafsi_ends(&self, index: usize, end: usize) -> Option<Rc<[usize]>> {
        head_get(&self.extended_rafsi_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.extended_rafsi_ends(index, end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_extended_rafsi_ends(&self, index: usize, end: usize, value: &[usize]) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        head_set(&self.extended_rafsi_ends, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn any_extended_rafsi_starts(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.any_extended_rafsi_starts, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.any_extended_rafsi_starts(index, end) == Some(value))]
    pub(crate) fn record_any_extended_rafsi_starts(&self, index: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.any_extended_rafsi_starts, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_rafsi_base_ends(&self, index: usize, end: usize) -> Option<Rc<[usize]>> {
        head_get(&self.fuhivla_rafsi_base_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.fuhivla_rafsi_base_ends(index, end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_fuhivla_rafsi_base_ends(&self, index: usize, end: usize, value: &[usize]) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        head_set(&self.fuhivla_rafsi_base_ends, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn stressed_extended_rafsi_ends(
        &self,
        index: usize,
        end: usize,
    ) -> Option<Rc<[usize]>> {
        head_get(&self.stressed_extended_rafsi_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(
        self.is_cancelled()
            || self
                .stressed_extended_rafsi_ends(index, end)
                .is_some_and(|ends| ends.len() == value.len())
    )]
    pub(crate) fn record_stressed_extended_rafsi_ends(
        &self,
        index: usize,
        end: usize,
        value: &[usize],
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        head_set(&self.stressed_extended_rafsi_ends, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn nucleus_has_implicit_stress(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.nucleus_has_implicit_stress, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.nucleus_has_implicit_stress(index, end) == Some(value))]
    pub(crate) fn record_nucleus_has_implicit_stress(&self, index: usize, end: usize, value: bool) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.nucleus_has_implicit_stress, index, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn stress_tail_reaches_boundary(&self, index: usize, end: usize) -> Option<bool> {
        span_get(&self.stress_tail_reaches_boundary, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.stress_tail_reaches_boundary(index, end) == Some(value))]
    pub(crate) fn record_stress_tail_reaches_boundary(
        &self,
        index: usize,
        end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        span_set(&self.stress_tail_reaches_boundary, index, end, value);
        self.sync_accounted();
    }

    #[requires(start <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn greedy_rafsi_chain(
        &self,
        start: usize,
        lookahead_end: usize,
    ) -> Option<Rc<[usize]>> {
        head_get(&self.greedy_rafsi_chain, start, lookahead_end)
    }

    #[requires(start <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(
        self.is_cancelled()
            || self
                .greedy_rafsi_chain(start, lookahead_end)
                .is_some_and(|chain| chain.len() == value.len())
    )]
    #[ensures(ret.len() == value.len())]
    pub(crate) fn record_greedy_rafsi_chain(
        &self,
        start: usize,
        lookahead_end: usize,
        value: &[usize],
    ) -> Rc<[usize]> {
        if self.is_cancelled() {
            return Rc::from(value);
        }
        note_recognition_body();
        let stored = head_set(&self.greedy_rafsi_chain, start, lookahead_end, value);
        self.sync_accounted();
        stored
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(super) fn lujvo_part_ranges(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
    ) -> Option<LujvoPartRangeMemo> {
        let key = new!(InitialRafsiKey {
            start: index,
            end: end,
            has_initial_rafsi: has_initial_rafsi,
        });
        self.lujvo_part_ranges.borrow().get(&key).cloned()
    }

    #[requires(index <= end && end <= self.char_count)]
    #[requires(ranges.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
    #[ensures(self.is_cancelled() || self.lujvo_part_ranges(index, end, has_initial_rafsi).is_some())]
    pub(super) fn record_lujvo_part_ranges(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
        ranges: Option<Rc<[LujvoPartRange]>>,
        failure: Option<LujvoParseFailure>,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        let key = new!(InitialRafsiKey {
            start: index,
            end: end,
            has_initial_rafsi: has_initial_rafsi,
        });
        let memo = new!(LujvoPartRangeMemo {
            ranges: ranges,
            failure: failure,
        });
        self.lujvo_part_ranges.borrow_mut().insert(key, memo);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn nucleus_is_stressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
    ) -> Option<bool> {
        stress_get(&self.nucleus_is_stressed, start, end, context_end)
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.nucleus_is_stressed(start, end, context_end) == Some(value))]
    pub(crate) fn record_nucleus_is_stressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        stress_set(&self.nucleus_is_stressed, start, end, context_end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn y_less_rafsi_unstressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
    ) -> Option<bool> {
        stress_get(&self.y_less_rafsi_unstressed, start, end, context_end)
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.y_less_rafsi_unstressed(start, end, context_end) == Some(value))]
    pub(crate) fn record_y_less_rafsi_unstressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        stress_set(
            &self.y_less_rafsi_unstressed,
            start,
            end,
            context_end,
            value,
        );
        self.sync_accounted();
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn cvv_rafsi_unstressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
    ) -> Option<bool> {
        stress_get(&self.cvv_rafsi_unstressed, start, end, context_end)
    }

    #[requires(start <= end && end <= context_end && context_end <= self.char_count)]
    #[ensures(self.is_cancelled() || self.cvv_rafsi_unstressed(start, end, context_end) == Some(value))]
    pub(crate) fn record_cvv_rafsi_unstressed(
        &self,
        start: usize,
        end: usize,
        context_end: usize,
        value: bool,
    ) {
        if self.is_cancelled() {
            return;
        }
        note_recognition_body();
        stress_set(&self.cvv_rafsi_unstressed, start, end, context_end, value);
        self.sync_accounted();
    }
}

/// Owns the slot count so the cache wrapper can stay free of `Drop`.
///
/// Every stored count is a valid account. The type exists only to subtract
/// that count from the test live total when the cache goes away.
#[invariant(true)]
#[derive(Debug)]
struct CacheSlotAccount {
    slots: Cell<u64>,
}

impl CacheSlotAccount {
    #[requires(true)]
    #[ensures(ret.slots.get() == 0)]
    fn new() -> Self {
        Self {
            slots: Cell::new(0),
        }
    }
}

impl Drop for CacheSlotAccount {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        #[cfg(test)]
        {
            let previous = self.slots.get();
            LIVE_SLOTS.with(|live| {
                live.set(live.get().saturating_sub(previous));
            });
        }
    }
}

#[requires(start <= end)]
#[ensures(true)]
fn span_get(table: &RefCell<HashMap<SpanKey, bool>>, start: usize, end: usize) -> Option<bool> {
    let key = new!(SpanKey {
        start: start,
        end: end,
    });
    table.borrow().get(&key).copied()
}

#[requires(start <= end && end <= context_end)]
#[ensures(true)]
fn stress_get(
    table: &RefCell<HashMap<StressContextKey, bool>>,
    start: usize,
    end: usize,
    context_end: usize,
) -> Option<bool> {
    let key = new!(StressContextKey {
        start: start,
        end: end,
        context_end: context_end,
    });
    table.borrow().get(&key).copied()
}

#[requires(start <= end && end <= context_end)]
#[ensures(stress_get(table, start, end, context_end) == Some(value))]
fn stress_set(
    table: &RefCell<HashMap<StressContextKey, bool>>,
    start: usize,
    end: usize,
    context_end: usize,
    value: bool,
) {
    let key = new!(StressContextKey {
        start: start,
        end: end,
        context_end: context_end,
    });
    table.borrow_mut().insert(key, value);
}

#[requires(start <= end)]
#[ensures(span_get(table, start, end) == Some(value))]
fn span_set(table: &RefCell<HashMap<SpanKey, bool>>, start: usize, end: usize, value: bool) {
    let key = new!(SpanKey {
        start: start,
        end: end,
    });
    table.borrow_mut().insert(key, value);
}

#[requires(start <= end)]
#[ensures(true)]
fn head_get(
    table: &RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    start: usize,
    end: usize,
) -> Option<Rc<[usize]>> {
    let key = new!(SpanKey {
        start: start,
        end: end,
    });
    table.borrow().get(&key).cloned()
}

#[requires(start <= end)]
#[ensures(head_get(table, start, end).is_some_and(|ends| ends.len() == value.len()))]
fn head_set(
    table: &RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    start: usize,
    end: usize,
    value: &[usize],
) -> Rc<[usize]> {
    let stored = Rc::<[usize]>::from(value);
    let key = new!(SpanKey {
        start: start,
        end: end,
    });
    table.borrow_mut().insert(key, Rc::clone(&stored));
    stored
}

#[requires(start <= end && end <= lookahead_end)]
#[ensures(true)]
fn lookahead_head_get(
    table: &RefCell<HashMap<LookaheadKey, Rc<[usize]>>>,
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> Option<Rc<[usize]>> {
    let key = new!(LookaheadKey {
        start: start,
        end: end,
        lookahead_end: lookahead_end,
    });
    table.borrow().get(&key).cloned()
}

#[requires(start <= end && end <= lookahead_end)]
#[ensures(lookahead_head_get(table, start, end, lookahead_end).is_some_and(|ends| ends.len() == value.len()))]
fn lookahead_head_set(
    table: &RefCell<HashMap<LookaheadKey, Rc<[usize]>>>,
    start: usize,
    end: usize,
    lookahead_end: usize,
    value: &[usize],
) -> Rc<[usize]> {
    let stored = Rc::<[usize]>::from(value);
    let key = new!(LookaheadKey {
        start: start,
        end: end,
        lookahead_end: lookahead_end,
    });
    table.borrow_mut().insert(key, Rc::clone(&stored));
    stored
}
