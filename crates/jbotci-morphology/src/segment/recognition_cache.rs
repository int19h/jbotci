//! Packrat tables for the morphology recognizers that call one another.
//!
//! The tables store the first result of each recognizer. Recursive calls move
//! the start index forward, so a call does not re-enter its own key before
//! that result is stored. A stored result is therefore the same value the body
//! would have returned, and nothing is cut off by depth, steps, or time.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use bityzba::{data, invariant, new, requires};

#[cfg(test)]
thread_local! {
    static BODY_COUNT: Cell<u64> = const { Cell::new(0) };
    static BODY_CAP: Cell<u64> = const { Cell::new(u64::MAX) };
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

#[requires(char_count < usize::MAX)]
#[requires((char_count + 1).checked_mul(char_count + 1).is_some())]
#[ensures(ret == (char_count + 1) * (char_count + 1))]
fn span_area(char_count: usize) -> usize {
    let stride = char_count + 1;
    stride * stride
}

#[requires(len > 0)]
#[ensures(ret.len() == len)]
#[ensures(ret.iter().all(|slot| slot.get().is_none()))]
fn empty_bool_slots(len: usize) -> Vec<Cell<Option<bool>>> {
    (0..len).map(|_| Cell::new(None)).collect()
}

#[requires(len > 0)]
#[ensures(ret.len() == len)]
#[ensures(ret.iter().all(|slot| slot.borrow().is_none()))]
fn empty_head_slots(len: usize) -> Vec<RefCell<Option<Vec<usize>>>> {
    (0..len).map(|_| RefCell::new(None)).collect()
}

/// `(start, end)` results. `start <= end < stride`, and slot `(end, start)` is
/// `end * stride + start`. `None` means the body has not run.
#[invariant(*stride > 0)]
#[invariant(stride.checked_mul(*stride) == Some(slots.len()))]
#[derive(Debug)]
struct DenseBoolTable {
    stride: usize,
    slots: Vec<Cell<Option<bool>>>,
}

impl DenseBoolTable {
    #[requires(char_count < usize::MAX)]
    #[requires((char_count + 1).checked_mul(char_count + 1).is_some())]
    #[ensures(ret.stride == char_count + 1)]
    #[ensures(ret.slots.len() == ret.stride * ret.stride)]
    fn new(char_count: usize) -> Self {
        let stride = char_count + 1;
        Self::from_data(data!(DenseBoolTable {
            stride,
            slots: empty_bool_slots(span_area(char_count)),
        }))
    }

    #[requires(start <= end && end < self.stride)]
    #[ensures(true)]
    fn get(&self, start: usize, end: usize) -> Option<bool> {
        self.slots[end * self.stride + start].get()
    }

    #[requires(start <= end && end < self.stride)]
    #[ensures(self.get(start, end) == Some(value))]
    fn set(&self, start: usize, end: usize, value: bool) {
        self.slots[end * self.stride + start].set(Some(value));
    }
}

/// Two results for one `(start, end)`, selected by a flag in `0..2`.
///
/// The flag is the initial-rafsi bit, the y-policy bit, or the fu'ivla head
/// policy. The caller owns that mapping.
#[invariant(*stride > 0)]
#[invariant(stride.checked_mul(*stride).and_then(|area| area.checked_mul(2)) == Some(slots.len()))]
#[derive(Debug)]
struct DenseFlagTable {
    stride: usize,
    slots: Vec<Cell<Option<bool>>>,
}

impl DenseFlagTable {
    #[requires(char_count < usize::MAX)]
    #[requires((char_count + 1).checked_mul(char_count + 1).and_then(|area| area.checked_mul(2)).is_some())]
    #[ensures(ret.stride == char_count + 1)]
    #[ensures(ret.slots.len() == ret.stride * ret.stride * 2)]
    fn new(char_count: usize) -> Self {
        let stride = char_count + 1;
        let slots = span_area(char_count)
            .checked_mul(2)
            .expect("flag-table size checked by the precondition");
        Self::from_data(data!(DenseFlagTable {
            stride,
            slots: empty_bool_slots(slots),
        }))
    }

    #[requires(start <= end && end < self.stride && flag < 2)]
    #[ensures(true)]
    fn get(&self, start: usize, end: usize, flag: usize) -> Option<bool> {
        self.slots[(end * self.stride + start) * 2 + flag].get()
    }

    #[requires(start <= end && end < self.stride && flag < 2)]
    #[ensures(self.get(start, end, flag) == Some(value))]
    fn set(&self, start: usize, end: usize, flag: usize, value: bool) {
        self.slots[(end * self.stride + start) * 2 + flag].set(Some(value));
    }
}

/// Head-end lists. `None` means the body has not run, and `Some` of an empty
/// list is a computed empty result. Those two states have to stay distinct.
#[invariant(*stride > 0)]
#[invariant(stride.checked_mul(*stride) == Some(slots.len()))]
#[derive(Debug)]
struct DenseHeadTable {
    stride: usize,
    slots: Vec<RefCell<Option<Vec<usize>>>>,
}

impl DenseHeadTable {
    #[requires(char_count < usize::MAX)]
    #[requires((char_count + 1).checked_mul(char_count + 1).is_some())]
    #[ensures(ret.stride == char_count + 1)]
    #[ensures(ret.slots.len() == ret.stride * ret.stride)]
    fn new(char_count: usize) -> Self {
        let stride = char_count + 1;
        Self::from_data(data!(DenseHeadTable {
            stride,
            slots: empty_head_slots(span_area(char_count)),
        }))
    }

    #[requires(start <= end && end < self.stride)]
    #[ensures(true)]
    fn get(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        self.slots[end * self.stride + start].borrow().clone()
    }

    #[requires(start <= end && end < self.stride)]
    #[ensures(self.get(start, end).is_some())]
    fn set(&self, start: usize, end: usize, value: &[usize]) {
        *self.slots[end * self.stride + start].borrow_mut() = Some(value.to_vec());
    }
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
/// `chars.len()` is not bounded. `Grammar::candidate_end` walks forward until
/// it sees a word separator, and `classify_word` sizes one cache to the whole
/// comma-stripped string. A candidate can be as long as the input.
///
/// Two-index tables stay dense. One `Option<bool>` slot is 1 byte, and the
/// sixteen byte-tables (ten plain spans and three two-slot flags) are 28,224
/// bytes at n = 41, the reported word, about 1.0 MB at n = 256, 16 MB at
/// n = 1024, and 256 MB at n = 4096. A head-end slot is `Option<Vec<usize>>`
/// (24 bytes). The two head tables are 84,672 bytes at n = 41, about 3.0 MB
/// at n = 256, 48 MB at n = 1024, and 768 MB at n = 4096. The worst case below
/// already cannot finish for an n of a few hundred, so a dense square is the
/// compact layout for the keys this cycle fills. An unbounded token of several
/// thousand characters would make the squares large, and that allocation is
/// the cost of indexing the candidate the lexer actually has.
///
/// Three-index and four-index results are sparse. One dense three-index byte
/// table is 74,088 bytes at n = 41, 16 MB at n = 256, and 1 GB at n = 1024.
/// One dense four-index table is 3.0 MB at n = 41, 99 MB at n = 100, and 4 GB
/// at n = 256. An eager cube would allocate more memory than a run that long
/// can use. The maps store only keys whose bodies have run.
///
/// Worst-case time is O(n^6). Each body runs once per key. Two-index keys are
/// O(n^2) and three-index keys are O(n^3). The heavy body is
/// `rafsi_string_from_with_cache`: the greedy scan takes O(n) steps, and each
/// step asks whether a nucleus is stressed. That walk keeps a cursor stack and
/// tests membership in a vector, so one check is O(n^2) on a consonant run and
/// one body is O(n^3). O(n^3) keys times an O(n^3) body is O(n^6).
/// `coda_ends` returns the current index and at most the next index, so the
/// consonantal-chain recursion inside a fu'ivla head parse has one successor
/// and is O(n) per call. Head parses are two-index keys, so they are inside
/// this bound. The bound counts bodies, not a cutoff: every key still returns
/// the body's own answer.
#[invariant(char_count.checked_add(1) == Some(cmavo_word.stride))]
#[invariant(cmavo_word.stride == lojban_word.stride
    && cmavo_word.stride == brivla_word.stride
    && cmavo_word.stride == lujvo_slice.stride
    && cmavo_word.stride == slinkuhi.stride
    && cmavo_word.stride == rafsi_string_starts.stride
    && cmavo_word.stride == indicator_cmavo_starts.stride
    && cmavo_word.stride == post_word.stride
    && cmavo_word.stride == fuhivla_shape.stride
    && cmavo_word.stride == experimental_cgv.stride
    && cmavo_word.stride == brivla_head_ends.stride
    && cmavo_word.stride == fuhivla_head_ends.stride
    && cmavo_word.stride == lujvo_from.stride
    && cmavo_word.stride == fuhivla_shape_y.stride
    && cmavo_word.stride == fuhivla_head_policy.stride)]
#[invariant(cmavo_word.stride.checked_mul(cmavo_word.stride) == Some(cmavo_word.slots.len()))]
#[invariant(lojban_word.slots.len() == cmavo_word.slots.len()
    && brivla_word.slots.len() == cmavo_word.slots.len()
    && lujvo_slice.slots.len() == cmavo_word.slots.len()
    && slinkuhi.slots.len() == cmavo_word.slots.len()
    && rafsi_string_starts.slots.len() == cmavo_word.slots.len()
    && indicator_cmavo_starts.slots.len() == cmavo_word.slots.len()
    && post_word.slots.len() == cmavo_word.slots.len()
    && fuhivla_shape.slots.len() == cmavo_word.slots.len()
    && experimental_cgv.slots.len() == cmavo_word.slots.len()
    && brivla_head_ends.slots.len() == cmavo_word.slots.len()
    && fuhivla_head_ends.slots.len() == cmavo_word.slots.len())]
#[invariant(lujvo_from.stride.checked_mul(lujvo_from.stride).and_then(|area| area.checked_mul(2)) == Some(lujvo_from.slots.len()))]
#[invariant(fuhivla_shape_y.slots.len() == lujvo_from.slots.len())]
#[invariant(fuhivla_head_policy.slots.len() == lujvo_from.slots.len())]
#[derive(Debug)]
pub(crate) struct MorphologyRecognitionCache {
    pub(crate) char_count: usize,
    lujvo_from: DenseFlagTable,
    cmavo_word: DenseBoolTable,
    lojban_word: DenseBoolTable,
    brivla_word: DenseBoolTable,
    lujvo_slice: DenseBoolTable,
    fuhivla_shape_y: DenseFlagTable,
    fuhivla_shape: DenseBoolTable,
    fuhivla_head_policy: DenseFlagTable,
    experimental_cgv: DenseBoolTable,
    fuhivla_head_ends: DenseHeadTable,
    brivla_head_ends: DenseHeadTable,
    slinkuhi: DenseBoolTable,
    rafsi_string_starts: DenseBoolTable,
    rafsi_string_from: RefCell<HashMap<LookaheadKey, bool>>,
    rafsi_string_ending: RefCell<HashMap<LookaheadKey, bool>>,
    rafsi_string_lookahead: RefCell<HashMap<LookaheadKey, bool>>,
    post_word: DenseBoolTable,
    indicator_cmavo_starts: DenseBoolTable,
    cmavo_post_word: RefCell<HashMap<PrefixEndKey, bool>>,
    indicator_boundary: RefCell<HashMap<IndicatorBoundaryKey, bool>>,
}

impl MorphologyRecognitionCache {
    #[requires(char_count < usize::MAX && (char_count + 1).checked_mul(char_count + 1).and_then(|area| area.checked_mul(2)).is_some())]
    #[ensures(ret.char_count == char_count)]
    #[ensures(ret.cmavo_word.stride == char_count + 1)]
    pub(crate) fn new(char_count: usize) -> Self {
        Self::from_data(data!(MorphologyRecognitionCache {
            char_count,
            lujvo_from: DenseFlagTable::new(char_count),
            cmavo_word: DenseBoolTable::new(char_count),
            lojban_word: DenseBoolTable::new(char_count),
            brivla_word: DenseBoolTable::new(char_count),
            lujvo_slice: DenseBoolTable::new(char_count),
            fuhivla_shape_y: DenseFlagTable::new(char_count),
            fuhivla_shape: DenseBoolTable::new(char_count),
            fuhivla_head_policy: DenseFlagTable::new(char_count),
            experimental_cgv: DenseBoolTable::new(char_count),
            fuhivla_head_ends: DenseHeadTable::new(char_count),
            brivla_head_ends: DenseHeadTable::new(char_count),
            slinkuhi: DenseBoolTable::new(char_count),
            rafsi_string_starts: DenseBoolTable::new(char_count),
            rafsi_string_from: RefCell::new(HashMap::new()),
            rafsi_string_ending: RefCell::new(HashMap::new()),
            rafsi_string_lookahead: RefCell::new(HashMap::new()),
            post_word: DenseBoolTable::new(char_count),
            indicator_cmavo_starts: DenseBoolTable::new(char_count),
            cmavo_post_word: RefCell::new(HashMap::new()),
            indicator_boundary: RefCell::new(HashMap::new()),
        }))
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lujvo_from(
        &self,
        index: usize,
        end: usize,
        has_initial_rafsi: bool,
    ) -> Option<bool> {
        self.lujvo_from
            .get(index, end, usize::from(has_initial_rafsi))
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
        self.lujvo_from
            .set(index, end, usize::from(has_initial_rafsi), result);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn cmavo_word(&self, start: usize, end: usize) -> Option<bool> {
        self.cmavo_word.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.cmavo_word(start, end) == Some(value))]
    pub(crate) fn record_cmavo_word(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.cmavo_word.set(start, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lojban_word(&self, start: usize, end: usize) -> Option<bool> {
        self.lojban_word.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.lojban_word(start, end) == Some(value))]
    pub(crate) fn record_lojban_word(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.lojban_word.set(start, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn brivla_word(&self, start: usize, end: usize) -> Option<bool> {
        self.brivla_word.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.brivla_word(start, end) == Some(value))]
    pub(crate) fn record_brivla_word(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.brivla_word.set(start, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn lujvo_slice(&self, start: usize, end: usize) -> Option<bool> {
        self.lujvo_slice.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.lujvo_slice(start, end) == Some(value))]
    pub(crate) fn record_lujvo_slice(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.lujvo_slice.set(start, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_shape_y(&self, start: usize, end: usize, allows_y: bool) -> Option<bool> {
        self.fuhivla_shape_y.get(start, end, usize::from(allows_y))
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
        self.fuhivla_shape_y
            .set(start, end, usize::from(allows_y), value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_shape(&self, start: usize, end: usize) -> Option<bool> {
        self.fuhivla_shape.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.fuhivla_shape(start, end) == Some(value))]
    pub(crate) fn record_fuhivla_shape(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.fuhivla_shape.set(start, end, value);
    }

    /// `policy_index` is 0 for the standard fu'ivla head and 1 for the
    /// experimental consonant-glide-vowel head.
    #[requires(start <= end && end <= self.char_count && policy_index < 2)]
    #[ensures(true)]
    pub(crate) fn fuhivla_head_policy(
        &self,
        start: usize,
        end: usize,
        policy_index: usize,
    ) -> Option<bool> {
        self.fuhivla_head_policy.get(start, end, policy_index)
    }

    #[requires(start <= end && end <= self.char_count && policy_index < 2)]
    #[ensures(self.fuhivla_head_policy(start, end, policy_index) == Some(value))]
    pub(crate) fn record_fuhivla_head_policy(
        &self,
        start: usize,
        end: usize,
        policy_index: usize,
        value: bool,
    ) {
        note_recognition_body();
        self.fuhivla_head_policy
            .set(start, end, policy_index, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn experimental_cgv(&self, start: usize, end: usize) -> Option<bool> {
        self.experimental_cgv.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.experimental_cgv(start, end) == Some(value))]
    pub(crate) fn record_experimental_cgv(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.experimental_cgv.set(start, end, value);
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn fuhivla_head_ends(&self, index: usize, end: usize) -> Option<Vec<usize>> {
        self.fuhivla_head_ends.get(index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.fuhivla_head_ends(index, end).is_some())]
    pub(crate) fn record_fuhivla_head_ends(&self, index: usize, end: usize, value: &[usize]) {
        note_recognition_body();
        self.fuhivla_head_ends.set(index, end, value);
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn brivla_head_ends(&self, index: usize, end: usize) -> Option<Vec<usize>> {
        self.brivla_head_ends.get(index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.brivla_head_ends(index, end).is_some())]
    pub(crate) fn record_brivla_head_ends(&self, index: usize, end: usize, value: &[usize]) {
        note_recognition_body();
        self.brivla_head_ends.set(index, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn slinkuhi(&self, start: usize, end: usize) -> Option<bool> {
        self.slinkuhi.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.slinkuhi(start, end) == Some(value))]
    pub(crate) fn record_slinkuhi(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.slinkuhi.set(start, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn rafsi_string_starts(&self, start: usize, end: usize) -> Option<bool> {
        self.rafsi_string_starts.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.rafsi_string_starts(start, end) == Some(value))]
    pub(crate) fn record_rafsi_string_starts(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.rafsi_string_starts.set(start, end, value);
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
            start,
            end,
            lookahead_end
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
            start,
            end,
            lookahead_end
        });
        self.rafsi_string_from.borrow_mut().insert(key, value);
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
            start,
            end,
            lookahead_end
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
            start,
            end,
            lookahead_end
        });
        self.rafsi_string_ending.borrow_mut().insert(key, value);
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn rafsi_string_lookahead(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
    ) -> Option<bool> {
        let key = new!(LookaheadKey {
            start,
            end,
            lookahead_end
        });
        self.rafsi_string_lookahead.borrow().get(&key).copied()
    }

    #[requires(start <= end && end <= lookahead_end && lookahead_end <= self.char_count)]
    #[ensures(self.rafsi_string_lookahead(start, end, lookahead_end) == Some(value))]
    pub(crate) fn record_rafsi_string_lookahead(
        &self,
        start: usize,
        end: usize,
        lookahead_end: usize,
        value: bool,
    ) {
        note_recognition_body();
        let key = new!(LookaheadKey {
            start,
            end,
            lookahead_end
        });
        self.rafsi_string_lookahead.borrow_mut().insert(key, value);
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn post_word(&self, index: usize, end: usize) -> Option<bool> {
        self.post_word.get(index, end)
    }

    #[requires(index <= end && end <= self.char_count)]
    #[ensures(self.post_word(index, end) == Some(value))]
    pub(crate) fn record_post_word(&self, index: usize, end: usize, value: bool) {
        note_recognition_body();
        self.post_word.set(index, end, value);
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(true)]
    pub(crate) fn indicator_cmavo_starts(&self, start: usize, end: usize) -> Option<bool> {
        self.indicator_cmavo_starts.get(start, end)
    }

    #[requires(start <= end && end <= self.char_count)]
    #[ensures(self.indicator_cmavo_starts(start, end) == Some(value))]
    pub(crate) fn record_indicator_cmavo_starts(&self, start: usize, end: usize, value: bool) {
        note_recognition_body();
        self.indicator_cmavo_starts.set(start, end, value);
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
            prefix_start,
            prefix_end,
            end
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
            prefix_start,
            prefix_end,
            end
        });
        self.cmavo_post_word.borrow_mut().insert(key, value);
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
            prefix_start,
            prefix_end,
            index,
            end
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
            prefix_start,
            prefix_end,
            index,
            end
        });
        self.indicator_boundary.borrow_mut().insert(key, value);
    }

    #[cfg(test)]
    #[requires((self.char_count + 1).checked_mul(self.char_count + 1).is_some())]
    #[ensures(true)]
    pub(crate) fn assert_dense_layout(&self) {
        let stride = self.char_count + 1;
        let area = stride * stride;
        assert_eq!(self.cmavo_word.stride, stride);
        assert_eq!(self.cmavo_word.slots.len(), area);
        assert_eq!(self.lojban_word.slots.len(), area);
        assert_eq!(self.brivla_word.slots.len(), area);
        assert_eq!(self.lujvo_slice.slots.len(), area);
        assert_eq!(self.slinkuhi.slots.len(), area);
        assert_eq!(self.rafsi_string_starts.slots.len(), area);
        assert_eq!(self.indicator_cmavo_starts.slots.len(), area);
        assert_eq!(self.post_word.slots.len(), area);
        assert_eq!(self.fuhivla_shape.slots.len(), area);
        assert_eq!(self.experimental_cgv.slots.len(), area);
        assert_eq!(self.brivla_head_ends.slots.len(), area);
        assert_eq!(self.fuhivla_head_ends.slots.len(), area);
        assert_eq!(self.lujvo_from.slots.len(), area * 2);
        assert_eq!(self.fuhivla_shape_y.slots.len(), area * 2);
        assert_eq!(self.fuhivla_head_policy.slots.len(), area * 2);
        assert!(self.rafsi_string_from.borrow().is_empty());
        assert!(self.rafsi_string_ending.borrow().is_empty());
        assert!(self.rafsi_string_lookahead.borrow().is_empty());
        assert!(self.cmavo_post_word.borrow().is_empty());
        assert!(self.indicator_boundary.borrow().is_empty());
    }
}
