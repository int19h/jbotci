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

use bityzba::{invariant, new, requires};

use super::FuhivlaHeadPolicy;

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
///   `indicator_cmavo_starts`, `brivla_head_ends`, and `fuhivla_head_ends`.
/// - `lujvo_from` adds `has_initial_rafsi`, `fuhivla_shape_y` adds `allows_y`,
///   and `fuhivla_head_policy` adds `FuhivlaHeadPolicy`. Each flag has a
///   constant number of values, so those maps are still `O(m^2)`.
/// - Three-index keys are `O(m^3)`: `rafsi_string_from`, `rafsi_string_ending`,
///   and `cmavo_post_word`.
/// - Four-index keys are `O(m^4)`: `indicator_boundary`.
///
/// Each memoized body runs once per key.
///
/// `rafsi_string_from_with_cache` is the heavy memoized body. Its greedy scan
/// takes `O(n)` steps, and each step calls `nucleus_is_stressed_in_context`.
/// That walk keeps a cursor stack and tests membership in a vector, so one
/// check is `O(n^2)` on a consonant run and one body is `O(n^3)`. `O(n^3)`
/// three-index keys times an `O(n^3)` body is expected `O(n^6)`.
///
/// These helpers are not memoized:
///
/// - `initial_rafsi_ends_until_with_cache` calls
///   `extended_rafsi_ends_until_with_cache` once, then the y-rafsi ends, then
///   the y-less ends. A y-less end has a fixed shape, so that branch calls
///   `any_extended_rafsi_starts_until_with_cache` a bounded number of extra
///   times. The extended walk below can make one call `O(n^2)`. The
///   `lujvo_from` body calls this once per two-index key.
/// - `extended_rafsi_ends_until_with_cache` walks the fu'ivla rafsi bases and
///   the memoized brivla head ends. One base can run an `O(n)` syllable scan,
///   and there can be `O(n)` bases, so one call can be `O(n^2)`. Across
///   `O(n^2)` `lujvo_from` keys that part is expected `O(n^4)`.
/// - `any_extended_rafsi_starts_until_with_cache` calls the memoized fu'ivla
///   shape test, then the unmemoized extended walk, then
///   `stressed_extended_rafsi_ends_until_with_cache`, which loops over `O(n)`
///   ends. On a span of `lo` the stressed-brivla and stressed-fu'ivla shape
///   checks fail in constant time because those shapes require `'` or `y`.
///   On a span that matches, the syllable scans make one call `O(n^2)`. A
///   caller that does this for every two-index key pays expected `O(n^4)` in
///   that branch.
/// - `lujvo_core_part_ranges_until` calls the memoized fu'ivla shape test and
///   scans for one stressed split. The split scan can call the extended-rafsi
///   checks, so one call can be `O(n^2)`. The `lujvo_from` body calls it once.
/// - `coda_ends`, `starts_any_syllable`, and `consonantal_syllable_ends` scan
///   from the index down to the end of the candidate. The onset check looks
///   at a bounded initial and one nucleus, so one scan is `O(end - index)`.
///   `consonantal_chain_then_final` still follows one successor per step. One
///   chain is `O(n^2)` when those scans are not memoized across calls.
/// - `lujvo_part_ranges_from_with_cache` is an unmemoized branching recursion
///   with a `failure` side effect. It has no proven polynomial bound.
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
    fuhivla_head_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    brivla_head_ends: RefCell<HashMap<SpanKey, Rc<[usize]>>>,
    slinkuhi: RefCell<HashMap<SpanKey, bool>>,
    rafsi_string_starts: RefCell<HashMap<SpanKey, bool>>,
    rafsi_string_from: RefCell<HashMap<LookaheadKey, bool>>,
    rafsi_string_ending: RefCell<HashMap<LookaheadKey, bool>>,
    post_word: RefCell<HashMap<SpanKey, bool>>,
    indicator_cmavo_starts: RefCell<HashMap<SpanKey, bool>>,
    cmavo_post_word: RefCell<HashMap<PrefixEndKey, bool>>,
    indicator_boundary: RefCell<HashMap<IndicatorBoundaryKey, bool>>,
}

impl MorphologyRecognitionCache {
    #[requires(char_count < usize::MAX)]
    #[ensures(ret.char_count == char_count)]
    #[ensures(ret.allocated_slots_for_contract() == 0)]
    pub(crate) fn new(char_count: usize) -> Self {
        let cache = new!(MorphologyRecognitionCache {
            char_count: char_count,
            accounted: CacheSlotAccount::new(),
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
        });
        cache.sync_accounted();
        cache
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

    #[cfg(test)]
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
    }

    /// Empty maps have capacity 0. The ensures on `new` uses this in every
    /// build, including ones that do not keep the test counter.
    #[requires(true)]
    #[ensures(true)]
    fn allocated_slots_for_contract(&self) -> u64 {
        #[cfg(test)]
        {
            self.total_capacity()
        }
        #[cfg(not(test))]
        {
            0
        }
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
    #[ensures(self.lujvo_from(index, end, has_initial_rafsi) == Some(result))]
    pub(crate) fn record_lujvo_from(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
        result: bool,
    ) {
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
    #[ensures(self.cmavo_word(start, end) == Some(value))]
    pub(crate) fn record_cmavo_word(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.lojban_word(start, end) == Some(value))]
    pub(crate) fn record_lojban_word(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.brivla_word(start, end) == Some(value))]
    pub(crate) fn record_brivla_word(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.lujvo_slice(start, end) == Some(value))]
    pub(crate) fn record_lujvo_slice(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.fuhivla_shape_y(start, end, allows_y) == Some(value))]
    pub(crate) fn record_fuhivla_shape_y(
        &self,
        start: usize,
        end: usize,
        allows_y: bool,
        value: bool,
    ) {
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
    #[ensures(self.fuhivla_shape(start, end) == Some(value))]
    pub(crate) fn record_fuhivla_shape(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.fuhivla_head_policy(start, end, policy) == Some(value))]
    pub(crate) fn record_fuhivla_head_policy(
        &self,
        start: usize,
        end: usize,
        policy: FuhivlaHeadPolicy,
        value: bool,
    ) {
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
    #[ensures(self.experimental_cgv(start, end) == Some(value))]
    pub(crate) fn record_experimental_cgv(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        span_set(&self.experimental_cgv, start, end, value);
        self.sync_accounted();
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_head_ends(&self, index: usize, end: usize) -> Option<Rc<[usize]>> {
        head_get(&self.fuhivla_head_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.fuhivla_head_ends(index, end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_fuhivla_head_ends(
        &self,
        index: usize,
        end: usize,
        value: &[usize],
    ) -> Rc<[usize]> {
        note_recognition_body();
        let stored = head_set(&self.fuhivla_head_ends, index, end, value);
        self.sync_accounted();
        stored
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn brivla_head_ends(&self, index: usize, end: usize) -> Option<Rc<[usize]>> {
        head_get(&self.brivla_head_ends, index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.brivla_head_ends(index, end).is_some_and(|ends| ends.len() == value.len()))]
    pub(crate) fn record_brivla_head_ends(
        &self,
        index: usize,
        end: usize,
        value: &[usize],
    ) -> Rc<[usize]> {
        note_recognition_body();
        let stored = head_set(&self.brivla_head_ends, index, end, value);
        self.sync_accounted();
        stored
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn slinkuhi(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.slinkuhi, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.slinkuhi(start, end) == Some(value))]
    pub(crate) fn record_slinkuhi(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.rafsi_string_starts(start, end) == Some(value))]
    pub(crate) fn record_rafsi_string_starts(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.rafsi_string_from(start, end, lookahead_end) == Some(value))]
    pub(crate) fn record_rafsi_string_from(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
        value: bool,
    ) {
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
    #[ensures(self.rafsi_string_ending(start, end, lookahead_end) == Some(value))]
    pub(crate) fn record_rafsi_string_ending(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
        value: bool,
    ) {
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
    #[ensures(self.post_word(index, end) == Some(value))]
    pub(crate) fn record_post_word(&self, index: usize, end: usize, value: bool) {
        note_recognition_body();
        span_set(&self.post_word, index, end, value);
        self.sync_accounted();
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn indicator_cmavo_starts(&self, start: usize, end: usize) -> Option<bool> {
        span_get(&self.indicator_cmavo_starts, start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.indicator_cmavo_starts(start, end) == Some(value))]
    pub(crate) fn record_indicator_cmavo_starts(&self, start: usize, end: usize, value: bool) {
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
    #[ensures(self.cmavo_post_word(prefix_start, prefix_end, end) == Some(value))]
    pub(crate) fn record_cmavo_post_word(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        end: usize,
        value: bool,
    ) {
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
    #[ensures(self.indicator_boundary(prefix_start, prefix_end, index, end) == Some(value))]
    pub(crate) fn record_indicator_boundary(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        index: usize,
        end: usize,
        value: bool,
    ) {
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
