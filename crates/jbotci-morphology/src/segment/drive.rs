//! Heap driver for the recognition cycles.
//!
//! Each cache owns one driver. A call that needs another uncached key of that
//! cache suspends onto that cache's heap stack. A different cache has its own
//! driver, so a nested parse does not share this stack.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[cfg(test)]
use bityzba::try_new;
use bityzba::{data, invariant, new, requires};

use super::recognition_cache::MorphologyRecognitionCache;
use super::{LujvoParseFailure, LujvoPartRange};

/// A recognition answer is waiting on another key.
///
/// This value is not `false` and not `None`. The driver runs the other key
/// and then retries this one. A caller uses `?`.
#[invariant(true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Blocked {
    _private: (),
}

/// The only way to build `Blocked` outside this module.
/// The call sets `cell` before it returns the token.
#[requires(true)]
#[ensures(cell.get() == Some(index))]
pub(crate) fn block_source(cell: &Cell<Option<usize>>, index: usize) -> Blocked {
    cell.set(Some(index));
    mint_blocked()
}

#[requires(true)]
#[ensures(true)]
fn mint_blocked() -> Blocked {
    Blocked { _private: () }
}

#[invariant(::Cmavo { start, end } => start <= end)]
#[invariant(::Post { prefix_start, prefix_end, end } => prefix_start <= prefix_end && prefix_end <= end)]
#[invariant(::Lojban { start, end } => start <= end)]
#[invariant(::PrefixPost { index, end } => index <= end)]
#[invariant(::PrefixLojban { start, end } => start <= end)]
#[invariant(::LujvoFrom { index, end, .. } => index <= end)]
#[invariant(::LujvoParts { index, end, .. } => index <= end)]
#[invariant(::Cvcy { index, end } => index <= end)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RecKey {
    Cmavo {
        start: usize,
        end: usize,
    },
    Post {
        prefix_start: usize,
        prefix_end: usize,
        end: usize,
    },
    Lojban {
        start: usize,
        end: usize,
    },
    PrefixPost {
        index: usize,
        end: usize,
    },
    PrefixLojban {
        start: usize,
        end: usize,
    },
    LujvoFrom {
        index: usize,
        end: usize,
        flag: bool,
    },
    LujvoParts {
        index: usize,
        end: usize,
        flag: bool,
    },
    Cvcy {
        index: usize,
        end: usize,
    },
}

/// Cmavo resume. `PostWordFrom` means the cvcy arm already returned false.
/// The cursor is the post-word end that this attempt starts at.
/// The frame checks that the cursor stays inside the key.
#[invariant(true)]
#[invariant(::PostWordFrom(_) => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CmavoResume {
    Start,
    PostWordFrom(usize),
}

/// Prefix-lojban resume. The cmavo arm and the brivla arm share one frame.
/// `CmavoAt` means the cvcy arm already returned false.
/// `BrivlaWhole` means the cmavo arm already returned false.
/// `BrivlaShaped` still runs the shape check.
/// `BrivlaPost` means the shaped brivla already matched.
/// The frame checks that a cursor stays inside the key.
#[invariant(true)]
#[invariant(::CmavoAt(_) => true)]
#[invariant(::BrivlaShaped(_) => true)]
#[invariant(::BrivlaPost(_) => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrefixLojbanResume {
    Start,
    CmavoAt(usize),
    BrivlaWhole,
    BrivlaShaped(usize),
    BrivlaPost(usize),
}

/// Lujvo-from resume. Earlier ordinals returned false.
/// The frame checks that the ordinal stays inside the key.
#[invariant(true)]
#[invariant(::AtOrdinal(_) => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LujvoFromResume {
    Start,
    AtOrdinal(usize),
}

/// Lujvo-part resume.
/// `failure` is the best failure from ordinals that already finished.
/// The ordinal in this arm has not finished. Its body runs again.
/// The frame checks that the ordinal stays inside the key.
/// `failure` keeps the invariant of `LujvoParseFailure`.
#[invariant(true)]
#[invariant(::AtOrdinal { .. } => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LujvoPartsResume {
    Start,
    AtOrdinal {
        ordinal: usize,
        failure: Option<LujvoParseFailure>,
    },
}

/// Cvcy-star resume. The core arm already returned false.
/// The frame checks that the ordinal stays inside the key.
#[invariant(true)]
#[invariant(::AfterCore(_) => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CvcyResume {
    Start,
    AfterCore(usize),
}

/// Resume state for one recognition key.
/// Each arm belongs to one `RecKey` variant.
/// The frame checks that the arm matches that key.
#[invariant(true)]
#[invariant(::Cmavo(_) => true)]
#[invariant(::PrefixLojban(_) => true)]
#[invariant(::LujvoFrom(_) => true)]
#[invariant(::LujvoParts(_) => true)]
#[invariant(::Cvcy(_) => true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Resume {
    Cmavo(CmavoResume),
    Post,
    Lojban,
    PrefixPost,
    PrefixLojban(PrefixLojbanResume),
    LujvoFrom(LujvoFromResume),
    LujvoParts(LujvoPartsResume),
    Cvcy(CvcyResume),
}

/// `resume` matches `key`. A cursor in an arm stays at most `end`.
#[invariant(resume_matches(key, resume))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    key: RecKey,
    resume: Resume,
}

#[requires(true)]
#[ensures(true)]
fn resume_matches(key: &RecKey, resume: &Resume) -> bool {
    match (key.as_data(), resume) {
        (data!(RecKey::Cmavo { .. }), Resume::Cmavo(CmavoResume::Start)) => true,
        (data!(RecKey::Cmavo { end, .. }), Resume::Cmavo(CmavoResume::PostWordFrom(cursor))) => {
            *cursor <= *end
        }
        (data!(RecKey::Post { .. }), Resume::Post) => true,
        (data!(RecKey::Lojban { .. }), Resume::Lojban) => true,
        (data!(RecKey::PrefixPost { .. }), Resume::PrefixPost) => true,
        (data!(RecKey::PrefixLojban { .. }), Resume::PrefixLojban(PrefixLojbanResume::Start)) => {
            true
        }
        (
            data!(RecKey::PrefixLojban { end, .. }),
            Resume::PrefixLojban(PrefixLojbanResume::CmavoAt(cursor)),
        ) => *cursor <= *end,
        (
            data!(RecKey::PrefixLojban { .. }),
            Resume::PrefixLojban(PrefixLojbanResume::BrivlaWhole),
        ) => true,
        (
            data!(RecKey::PrefixLojban { end, .. }),
            Resume::PrefixLojban(PrefixLojbanResume::BrivlaShaped(ordinal)),
        ) => *ordinal <= *end,
        (
            data!(RecKey::PrefixLojban { end, .. }),
            Resume::PrefixLojban(PrefixLojbanResume::BrivlaPost(ordinal)),
        ) => *ordinal <= *end,
        (data!(RecKey::LujvoFrom { .. }), Resume::LujvoFrom(LujvoFromResume::Start)) => true,
        (
            data!(RecKey::LujvoFrom { end, .. }),
            Resume::LujvoFrom(LujvoFromResume::AtOrdinal(ordinal)),
        ) => *ordinal <= *end,
        (data!(RecKey::LujvoParts { .. }), Resume::LujvoParts(LujvoPartsResume::Start)) => true,
        (
            data!(RecKey::LujvoParts { end, .. }),
            Resume::LujvoParts(LujvoPartsResume::AtOrdinal { ordinal, .. }),
        ) => *ordinal <= *end,
        (data!(RecKey::Cvcy { .. }), Resume::Cvcy(CvcyResume::Start)) => true,
        (data!(RecKey::Cvcy { end, .. }), Resume::Cvcy(CvcyResume::AfterCore(ordinal))) => {
            *ordinal <= *end
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(resume_matches(&key, &ret))]
fn resume_start(key: RecKey) -> Resume {
    match *key.as_data() {
        data!(RecKey::Cmavo { .. }) => Resume::Cmavo(CmavoResume::Start),
        data!(RecKey::Post { .. }) => Resume::Post,
        data!(RecKey::Lojban { .. }) => Resume::Lojban,
        data!(RecKey::PrefixPost { .. }) => Resume::PrefixPost,
        data!(RecKey::PrefixLojban { .. }) => Resume::PrefixLojban(PrefixLojbanResume::Start),
        data!(RecKey::LujvoFrom { .. }) => Resume::LujvoFrom(LujvoFromResume::Start),
        data!(RecKey::LujvoParts { .. }) => Resume::LujvoParts(LujvoPartsResume::Start),
        data!(RecKey::Cvcy { .. }) => Resume::Cvcy(CvcyResume::Start),
    }
}

/// One arm of the cmavo-tail scan.
///
/// `Diphthong` tries a diphthong at this index.
/// `AfterDiphthong` resumes after that diphthong's child.
/// `Vowel` tries one vowel at this index.
/// `AfterVowel` resumes after that vowel's child.
#[invariant(true)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TailStep {
    Diphthong,
    AfterDiphthong,
    Vowel,
    AfterVowel,
}

/// `index` is a cursor in the caller buffer. `saved` is an output length.
/// `step` names the arm. The four arms are the only values.
#[invariant(matches!(
    step,
    &TailStep::Diphthong
        | &TailStep::AfterDiphthong
        | &TailStep::Vowel
        | &TailStep::AfterVowel
))]
#[derive(Clone, Copy)]
struct TailJob {
    index: usize,
    step: TailStep,
    saved: usize,
}

/// The job owns its prefix. `cand_i` is the next candidate, or one past the last.
#[invariant(*cand_i <= cands.len())]
struct CmevlaJob {
    prefix: Vec<LujvoPartRange>,
    cand_i: usize,
    cands: Vec<Vec<LujvoPartRange>>,
}

/// The start and the end of one prefix span in the caller buffer.
#[invariant(start <= end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PrefixSpan {
    start: usize,
    end: usize,
}

/// Driver flags for one recognition cache.
///
/// `block` is the key this driver is waiting on. A query on another cache
/// does not hold this cell. The key is present only while the driver is active.
#[invariant(driver.get() || block.get().is_none())]
#[derive(Debug)]
pub(crate) struct DriveState {
    driver: Cell<bool>,
    eval: Cell<Option<RecKey>>,
    block: Cell<Option<RecKey>>,
    resume: Cell<Resume>,
    prefix_ends: RefCell<HashMap<PrefixSpan, Rc<[usize]>>>,
}

impl DriveState {
    #[requires(true)]
    #[ensures(!ret.in_driver())]
    #[ensures(!ret.dep_blocked())]
    pub(super) fn new() -> Self {
        new!(Self {
            driver: Cell::new(false),
            eval: Cell::new(None),
            block: Cell::new(None),
            resume: Cell::new(Resume::Post),
            prefix_ends: RefCell::new(HashMap::new()),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn in_driver(&self) -> bool {
        self.driver.get()
    }

    #[requires(true)]
    #[ensures(self.in_driver() == value)]
    fn set_driver(&self, value: bool) {
        self.driver.set(value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn evaluating(&self, key: RecKey) -> bool {
        self.eval.get() == Some(key)
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_eval(&self, key: Option<RecKey>) {
        self.eval.set(key);
    }

    #[requires(true)]
    #[ensures(true)]
    fn dep_blocked(&self) -> bool {
        self.block.get().is_some()
    }

    #[requires(true)]
    #[ensures(self.dep_blocked() == key.is_some())]
    fn set_block(&self, key: Option<RecKey>) {
        self.block.set(key);
    }

    #[requires(true)]
    #[ensures(!self.dep_blocked())]
    fn take_block(&self) -> Option<RecKey> {
        self.block.take()
    }

    #[requires(true)]
    #[ensures(true)]
    fn resume(&self) -> Resume {
        self.resume.get()
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_resume_cell(&self, resume: Resume) {
        self.resume.set(resume);
    }

    #[requires(true)]
    #[ensures(true)]
    fn clear_prefix_ends(&self) {
        self.prefix_ends.borrow_mut().clear();
    }

    #[requires(true)]
    #[ensures(true)]
    fn prefix_hit(&self, key: PrefixSpan) -> Option<Rc<[usize]>> {
        self.prefix_ends.borrow().get(&key).cloned()
    }

    #[requires(true)]
    #[ensures(true)]
    fn store_prefix_ends(&self, key: PrefixSpan, ends: Rc<[usize]>) {
        self.prefix_ends.borrow_mut().insert(key, ends);
    }

    /// Clears the live driver. The test cycle flag clears with it.
    #[requires(true)]
    #[ensures(!self.in_driver())]
    #[ensures(!self.dep_blocked())]
    fn release(&self) {
        self.set_block(None);
        self.set_eval(None);
        self.set_driver(false);
        self.set_resume_cell(Resume::Post);
        self.clear_prefix_ends();
        #[cfg(test)]
        FORCE_SELF_CYCLE.with(|cell| cell.set(false));
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    fn take_force_cycle(&self) -> bool {
        FORCE_SELF_CYCLE.with(|cell| cell.replace(false))
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn force_cycle(&self) {
        FORCE_SELF_CYCLE.with(|cell| cell.set(true));
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn clear_force(&self) {
        FORCE_SELF_CYCLE.with(|cell| cell.set(false));
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn dep_blocked(cache: &MorphologyRecognitionCache) -> bool {
    cache.drive().dep_blocked()
}

#[requires(start <= end && end <= cache.char_count)]
#[ensures(true)]
fn cached_prefix_ends(cache: &MorphologyRecognitionCache, start: usize, end: usize) -> Rc<[usize]> {
    let key = new!(PrefixSpan {
        start: start,
        end: end
    });
    if let Some(hit) = cache.drive().prefix_hit(key) {
        return hit;
    }
    let ends: Rc<[usize]> = super::cvcy_stressable_nucleus_ends(&cache.char_state(), start, end)
        .into_iter()
        .skip(1)
        .collect();
    cache.drive().store_prefix_ends(key, Rc::clone(&ends));
    ends
}

#[cfg(test)]
thread_local! {
    /// The next dispatch treats its own key as a dependency.
    /// `release` and `clear_force` set the cell back to false.
    static FORCE_SELF_CYCLE: Cell<bool> = const { Cell::new(false) };
}

#[requires(true)]
#[ensures(true)]
fn in_driver(cache: &MorphologyRecognitionCache) -> bool {
    cache.drive().in_driver()
}

#[requires(true)]
#[ensures(true)]
fn evaluating(cache: &MorphologyRecognitionCache, key: RecKey) -> bool {
    cache.drive().evaluating(key)
}

#[requires(true)]
#[ensures(true)]
fn block(cache: &MorphologyRecognitionCache, key: RecKey) {
    cache.drive().set_block(Some(key));
}

/// `Ok(true)` means this call evaluates `key`.
/// `Ok(false)` means no driver is active, so the caller starts one.
/// `Err(Blocked)` means a driver is active on another key. The block is set.
#[requires(true)]
#[ensures(true)]
pub(super) fn claim(cache: &MorphologyRecognitionCache, key: RecKey) -> Result<bool, Blocked> {
    if evaluating(cache, key) {
        if !in_driver(cache) {
            panic!("recognition re-entered {key:?}");
        }
        Ok(true)
    } else if in_driver(cache) {
        block(cache, key);
        Err(mint_blocked())
    } else {
        Ok(false)
    }
}

/// A finished value. A blocked result cannot leave the driver.
#[requires(true)]
#[ensures(true)]
pub(crate) fn ready<T>(result: Result<T, Blocked>) -> T {
    match result {
        Ok(value) => value,
        Err(Blocked { .. }) => panic!("recognition blocked outside the driver"),
    }
}

/// Where one memoized recognition call enters.
///
/// `Cancelled` is a normal miss. `Hit` is the stored answer.
/// `Drive` means no driver is active, so the caller starts one.
/// `Own` means this call evaluates the key.
/// The caller builds `key` first. `RecKey` is `Copy`. The memo read can
/// run while the call is cancelled. A read does not store an answer.
/// `Hit` stores that answer. The answer type checks its own fields.
#[invariant(true)]
#[invariant(::Hit(_) => true)]
pub(super) enum Open<T> {
    Cancelled,
    Hit(T),
    Drive,
    Own,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn open_memo<T>(
    cache: &MorphologyRecognitionCache,
    cached: Option<T>,
    key: RecKey,
) -> Result<Open<T>, Blocked> {
    if cache.is_cancelled() {
        return Ok(Open::Cancelled);
    }
    if let Some(result) = cached {
        return Ok(Open::Hit(result));
    }
    if !claim(cache, key)? {
        return Ok(Open::Drive);
    }
    Ok(Open::Own)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn resume(cache: &MorphologyRecognitionCache) -> Resume {
    cache.drive().resume()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_resume(cache: &MorphologyRecognitionCache, resume: Resume) {
    cache.drive().set_resume_cell(resume);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn lujvo_from_drive(
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    cache: &MorphologyRecognitionCache,
) -> bool {
    drive(
        cache,
        new!(RecKey::LujvoFrom {
            index,
            end,
            flag: has_initial_rafsi,
        }),
    );
    cache
        .lujvo_from(index, end, has_initial_rafsi)
        .unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn lujvo_parts_drive(
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    failure: &mut Option<LujvoParseFailure>,
    cache: &MorphologyRecognitionCache,
) -> Option<Vec<LujvoPartRange>> {
    drive(
        cache,
        new!(RecKey::LujvoParts {
            index,
            end,
            flag: has_initial_rafsi,
        }),
    );
    if cache.is_cancelled() {
        return None;
    }
    restore_parts(cache, index, end, has_initial_rafsi, failure)
}

#[requires(true)]
#[ensures(true)]
fn restore_parts(
    cache: &MorphologyRecognitionCache,
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    failure: &mut Option<LujvoParseFailure>,
) -> Option<Vec<LujvoPartRange>> {
    let memo = cache.lujvo_part_ranges(index, end, has_initial_rafsi)?;
    super::merge_lujvo_failure(failure, memo.failure);
    memo.ranges.as_ref().map(|ranges| ranges.to_vec())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cvcy_drive(
    index: usize,
    context_end: usize,
    cache: &MorphologyRecognitionCache,
) -> bool {
    drive(
        cache,
        new!(RecKey::Cvcy {
            index,
            end: context_end,
        }),
    );
    cache.cvcy_core_before(index, context_end).unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn prefix_drive_lojban(
    index: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> bool {
    drive(cache, new!(RecKey::PrefixLojban { start: index, end }));
    cache.prefix_lojban_word(index, end).unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn prefix_drive_post(
    index: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> bool {
    drive(cache, new!(RecKey::PrefixPost { index, end }));
    cache.prefix_post_word(index, end).unwrap_or(false)
}

/// Clears the recognition driver. Drop runs on unwind.
///
/// The guard holds the driver by `Rc`, so drop does not borrow the cache.
#[invariant(true)]
struct DriveGuard {
    drive: Rc<DriveState>,
}

impl DriveGuard {
    #[requires(true)]
    #[ensures(!cache.drive().dep_blocked())]
    #[ensures(cache.drive().in_driver())]
    fn arm(cache: &MorphologyRecognitionCache) -> Self {
        cache.drive().set_driver(true);
        Self {
            drive: Rc::clone(&cache.drive()),
        }
    }
}

impl Drop for DriveGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.drive.release();
    }
}

#[requires(!cache.drive().in_driver())]
#[ensures(!cache.drive().in_driver())]
#[ensures(!cache.drive().dep_blocked())]
fn drive(cache: &MorphologyRecognitionCache, root: RecKey) {
    if cache.drive().in_driver() {
        panic!("recognition driver re-entered on the call stack for {root:?}");
    }
    cache.drive().clear_prefix_ends();
    let _drive = DriveGuard::arm(cache);
    let mut on_stack = HashSet::new();
    on_stack.insert(root);
    let mut stack = vec![new!(Frame {
        key: root,
        resume: resume_start(root),
    })];
    loop {
        let Some(index) = stack.len().checked_sub(1) else {
            break;
        };
        let frame = stack[index];
        if rec_cached(cache, frame.key) {
            on_stack.remove(&frame.key);
            stack.pop();
            continue;
        }
        if cache.is_cancelled() {
            break;
        }
        cache.drive().set_eval(Some(frame.key));
        cache.drive().set_block(None);
        cache.drive().set_resume_cell(frame.resume);
        dispatch(cache, frame.key);
        cache.drive().set_eval(None);
        if cache.is_cancelled() {
            break;
        }
        if let Some(dep) = cache.drive().take_block() {
            let saved = cache.drive().resume();
            let current = stack[index];
            stack[index] = current.with_data(data! { resume: saved });
            if !on_stack.insert(dep) {
                panic!("recognition re-entered {dep:?}");
            }
            stack.push(new!(Frame {
                key: dep,
                resume: resume_start(dep),
            }));
            continue;
        }
        if !rec_cached(cache, frame.key) {
            panic!(
                "recognition returned without a stored answer for {:?}",
                frame.key
            );
        }
        on_stack.remove(&frame.key);
        stack.pop();
    }
}

#[requires(true)]
#[ensures(true)]
fn dispatch(cache: &MorphologyRecognitionCache, key: RecKey) {
    #[cfg(test)]
    if cache.drive().take_force_cycle() {
        block(cache, key);
        return;
    }
    let blocked = match *key.as_data() {
        data!(RecKey::Cmavo { start, end }) => enter_cmavo(start, end, cache).is_err(),
        data!(RecKey::Post {
            prefix_start,
            prefix_end,
            end,
        }) => enter_cmavo_post(prefix_start, prefix_end, end, cache).is_err(),
        data!(RecKey::Lojban { start, end }) => enter_lojban(start, end, cache).is_err(),
        data!(RecKey::PrefixPost { index, end }) => enter_prefix_post(index, end, cache).is_err(),
        data!(RecKey::PrefixLojban { start, end }) => {
            enter_prefix_lojban(start, end, cache).is_err()
        }
        data!(RecKey::LujvoFrom { index, end, flag }) => {
            super::lujvo_from_until_with_cache(index, end, flag, cache).is_err()
        }
        data!(RecKey::LujvoParts { index, end, flag }) => {
            let mut ignored = None;
            super::lujvo_part_ranges_from_with_cache(index, end, flag, &mut ignored, cache).is_err()
        }
        data!(RecKey::Cvcy { index, end }) => {
            super::cvcy_initial_rafsi_star_core_before_end(index, end, cache).is_err()
        }
    };
    if blocked {
        assert!(
            cache.drive().dep_blocked(),
            "blocked recognition of {key:?} set no dependency"
        );
    } else if cache.drive().dep_blocked() {
        panic!("recognition stored a block after a finished answer for {key:?}");
    }
}

#[requires(true)]
#[ensures(true)]
fn rec_cached(cache: &MorphologyRecognitionCache, key: RecKey) -> bool {
    match *key.as_data() {
        data!(RecKey::Cmavo { start, end }) => cache.cmavo_word(start, end).is_some(),
        data!(RecKey::Post {
            prefix_start,
            prefix_end,
            end,
        }) => cache
            .cmavo_post_word(prefix_start, prefix_end, end)
            .is_some(),
        data!(RecKey::Lojban { start, end }) => cache.lojban_word(start, end).is_some(),
        data!(RecKey::PrefixPost { index, end }) => cache.prefix_post_word(index, end).is_some(),
        data!(RecKey::PrefixLojban { start, end }) => {
            cache.prefix_lojban_word(start, end).is_some()
        }
        data!(RecKey::LujvoFrom { index, end, flag }) => {
            cache.lujvo_from(index, end, flag).is_some()
        }
        data!(RecKey::LujvoParts { index, end, flag }) => {
            cache.lujvo_part_ranges(index, end, flag).is_some()
        }
        data!(RecKey::Cvcy { index, end }) => cache.cvcy_core_before(index, end).is_some(),
    }
}

/// A segment memo whose driver result is one bool.
///
/// Prefix, lujvo, and cvcy keys have their own drivers. This type names only
/// the three keys that `drive_bool` runs.
#[invariant(::Cmavo { start, end } => start <= end)]
#[invariant(::Post { prefix_start, prefix_end, end } => prefix_start <= prefix_end && prefix_end <= end)]
#[invariant(::Lojban { start, end } => start <= end)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentBoolKey {
    Cmavo {
        start: usize,
        end: usize,
    },
    Post {
        prefix_start: usize,
        prefix_end: usize,
        end: usize,
    },
    Lojban {
        start: usize,
        end: usize,
    },
}

#[requires(true)]
#[ensures(true)]
fn drive_bool(cache: &MorphologyRecognitionCache, key: SegmentBoolKey) -> bool {
    let rec = match key.as_data() {
        data!(SegmentBoolKey::Cmavo { start, end }) => new!(RecKey::Cmavo {
            start: *start,
            end: *end,
        }),
        data!(SegmentBoolKey::Post {
            prefix_start,
            prefix_end,
            end,
        }) => new!(RecKey::Post {
            prefix_start: *prefix_start,
            prefix_end: *prefix_end,
            end: *end,
        }),
        data!(SegmentBoolKey::Lojban { start, end }) => new!(RecKey::Lojban {
            start: *start,
            end: *end,
        }),
    };
    drive(cache, rec);
    match key.as_data() {
        data!(SegmentBoolKey::Cmavo { start, end }) => {
            cache.cmavo_word(*start, *end).unwrap_or(false)
        }
        data!(SegmentBoolKey::Post {
            prefix_start,
            prefix_end,
            end,
        }) => cache
            .cmavo_post_word(*prefix_start, *prefix_end, *end)
            .unwrap_or(false),
        data!(SegmentBoolKey::Lojban { start, end }) => {
            cache.lojban_word(*start, *end).unwrap_or(false)
        }
    }
}

#[requires(start <= state.len())]
#[ensures(true)]
pub(super) fn nucleus_at(state: &crate::recog_state::CharState, start: usize) -> bool {
    let len = state.len();
    nucleus_at_limit(state, start, len)
}

#[requires(start <= limit && limit <= state.len())]
#[ensures(true)]
pub(super) fn nucleus_at_limit(
    state: &crate::recog_state::CharState,
    start: usize,
    limit: usize,
) -> bool {
    if start >= limit {
        return false;
    }
    nucleus_followed_limit(state, start, limit)
}

#[requires(limit <= state.len())]
#[ensures(ret.is_none_or(|(_, end)| end > start && end <= limit))]
pub(super) fn diphthong_iter_limit(
    state: &crate::recog_state::CharState,
    start: usize,
    limit: usize,
) -> Option<(char, usize)> {
    if start >= limit {
        return None;
    }
    let chars: &[char] = state.chars();
    let view = &chars[..limit];
    let (semivowel, end) = super::raw_diphthong_end(view, start)?;
    if super::next_non_comma_index(view, end)
        .is_some_and(|next| super::matches_diphthong_semivowel(view[next], semivowel))
    {
        return None;
    }
    // `end` can name a comma. A nucleus after that comma still follows.
    if nucleus_followed_limit(state, end, limit) {
        return None;
    }
    Some((semivowel, end))
}

#[requires(start <= state.len())]
#[ensures(ret.is_none_or(|(_, end)| end > start && end <= state.len()))]
pub(super) fn diphthong_iter(
    state: &crate::recog_state::CharState,
    start: usize,
) -> Option<(char, usize)> {
    let chars: &[char] = state.chars();
    let (semivowel, end) = super::raw_diphthong_end(&chars, start)?;
    if super::next_non_comma_index(&chars, end)
        .is_some_and(|next| super::matches_diphthong_semivowel(chars[next], semivowel))
    {
        return None;
    }
    if nucleus_followed(state, end) {
        return None;
    }
    Some((semivowel, end))
}

#[requires(start <= state.len())]
#[ensures(ret.is_none_or(|end| end == start + 1))]
pub(super) fn single_vowel_iter(
    state: &crate::recog_state::CharState,
    start: usize,
) -> Option<usize> {
    let chars: &[char] = state.chars();
    let value = *chars.get(start)?;
    let end = start + 1;
    let followed = nucleus_followed(state, end);
    if value == 'y' || value == 'ý' {
        return (!followed).then_some(end);
    }
    if !super::is_vowel(value) && !matches!(value, 'ĭ' | 'ŭ') {
        return None;
    }
    (!followed).then_some(end)
}

#[requires(true)]
#[ensures(true)]
fn nucleus_followed(state: &crate::recog_state::CharState, index: usize) -> bool {
    let len = state.len();
    nucleus_followed_limit(state, index, len)
}

#[requires(index <= limit && limit <= state.len())]
#[ensures(true)]
fn nucleus_followed_limit(
    state: &crate::recog_state::CharState,
    index: usize,
    limit: usize,
) -> bool {
    // A vowel or `y` before a consonant needs no table and no cache lookup.
    // The lookup alone dominated `kánykla`. A long cluster still uses the table.
    // Camxes reads `!nucleus` after `comma*`. A comma is not the nucleus.
    let chars: &[char] = state.chars();
    let Some(index) = super::next_non_comma_index(&chars[..limit], index) else {
        return false;
    };
    if let Some(bit) = short_nucleus_bit(&chars, index, limit) {
        return bit;
    }
    if let Some(bit) = state.nucleus_bit(limit, index) {
        return bit;
    }
    if !state.claim_nucleus_limit(limit) {
        return nucleus_bits(state, limit)
            .get(index)
            .copied()
            .unwrap_or(false);
    }
    nucleus_bit_unstored(state, index, limit)
}

/// `Some` when the cluster is at most one character.
/// `None` when the next character can extend the cluster.
#[requires(index <= limit && limit <= chars.len())]
#[ensures(true)]
fn short_nucleus_bit(chars: &[char], index: usize, limit: usize) -> Option<bool> {
    if index >= limit {
        return Some(false);
    }
    let value = chars[index];
    if !nucleus_cluster_char(value) {
        return Some(false);
    }
    if index + 1 < limit && nucleus_cluster_char(chars[index + 1]) {
        return None;
    }
    Some(value != ',')
}

/// One bit of the nucleus table, over the vowel cluster at `index` only.
///
/// A consonant, an apostrophe, or the prefix end has no nucleus bit.
/// The cluster is the vowels, `y`, semivowels, and commas from `index`
/// up to that stop. Cells in the cluster start false, and the cell at the
/// stop stays false. The scan walks down to `index`. A later cell is ready
/// before the cursor reads it. The scratch is taken out for the scan.
#[requires(index <= limit && limit <= state.len())]
#[ensures(true)]
fn nucleus_bit_unstored(state: &crate::recog_state::CharState, index: usize, limit: usize) -> bool {
    if index >= limit {
        return false;
    }
    let chars: &[char] = state.chars();
    if let Some(bit) = short_nucleus_bit(&chars, index, limit) {
        return bit;
    }
    let cluster_end = nucleus_cluster_end(&chars, index, limit);
    let view = &chars[..limit];
    state.with_scratch(|bits| {
        if bits.len() < cluster_end + 1 {
            bits.resize(cluster_end + 1, false);
        }
        for slot in &mut bits[index..=cluster_end] {
            *slot = false;
        }
        for cursor in (index..cluster_end).rev() {
            #[cfg(test)]
            super::recognition_cache::count_nucleus_step();
            let bit = diphthong_here(view, cursor, bits) || single_here(view, cursor, bits);
            bits[cursor] = bit;
        }
        bits[index]
    })
}

#[requires(true)]
#[ensures(true)]
fn nucleus_cluster_char(value: char) -> bool {
    value == ','
        || value == 'y'
        || value == 'ý'
        || value == 'ĭ'
        || value == 'ŭ'
        || super::is_vowel(value)
}

/// First index at or after `index` that cannot extend this nucleus cluster.
#[requires(index <= limit && limit <= chars.len())]
#[ensures(true)]
fn nucleus_cluster_end(chars: &[char], index: usize, limit: usize) -> usize {
    let mut cursor = index;
    while cursor < limit && nucleus_cluster_char(chars[cursor]) {
        cursor += 1;
    }
    cursor
}

#[requires(limit <= state.len())]
#[ensures(ret.len() == limit + 1)]
fn nucleus_bits(state: &crate::recog_state::CharState, limit: usize) -> Vec<bool> {
    if state.nucleus_ready(limit) {
        return state.clone_nucleus_bits(limit);
    }
    let chars: &[char] = state.chars();
    let bits = build_nucleus_bits(&chars, limit);
    state.store_nucleus_bits(limit, bits.clone());
    bits
}

#[requires(limit <= chars.len())]
#[ensures(ret.len() == limit + 1)]
fn build_nucleus_bits(chars: &[char], limit: usize) -> Vec<bool> {
    let view = &chars[..limit];
    let mut bits = vec![false; limit + 1];
    for index in (0..limit).rev() {
        #[cfg(test)]
        super::recognition_cache::count_nucleus_step();
        bits[index] = diphthong_here(view, index, &bits) || single_here(view, index, &bits);
    }
    bits
}

/// The nucleus bit at `index`, after any commas.
///
/// Camxes writes `comma*` before the next vowel. The bit at the comma is
/// false. The bit that `!nucleus` reads is the bit at the next letter.
#[requires(index <= chars.len())]
#[ensures(true)]
fn nucleus_bit_after_commas(chars: &[char], index: usize, bits: &[bool]) -> bool {
    let Some(found) = super::next_non_comma_index(chars, index) else {
        return false;
    };
    bits.get(found).copied().unwrap_or(false)
}

#[requires(index < chars.len())]
#[ensures(true)]
fn diphthong_here(chars: &[char], index: usize, bits: &[bool]) -> bool {
    let Some((semivowel, end)) = super::raw_diphthong_end(chars, index) else {
        return false;
    };
    if super::next_non_comma_index(chars, end)
        .is_some_and(|next| super::matches_diphthong_semivowel(chars[next], semivowel))
    {
        return false;
    }
    !nucleus_bit_after_commas(chars, end, bits)
}

#[requires(index < chars.len())]
#[ensures(true)]
fn single_here(chars: &[char], index: usize, bits: &[bool]) -> bool {
    let value = chars[index];
    let end = index + 1;
    let followed = nucleus_bit_after_commas(chars, end, bits);
    if value == 'y' || value == 'ý' {
        return !followed;
    }
    (super::is_vowel(value) || matches!(value, 'ĭ' | 'ŭ')) && !followed
}

#[requires(nucleus_start < nucleus_end && nucleus_end <= chars.len())]
#[ensures(true)]
fn push_cmavo_diphthong(
    output: &mut Option<&mut String>,
    chars: &[char],
    nucleus_start: usize,
    nucleus_end: usize,
    semivowel: char,
) {
    // The acute sits on the first vowel. A mark on the off-glide moves there.
    let first = super::normalize_vowel(chars[nucleus_start]);
    let first = if super::syllable_has_explicit_stress(chars, nucleus_start, nucleus_end) {
        super::acute_vowel(first)
    } else {
        first
    };
    if let Some(text) = output.as_mut() {
        text.push(first);
        text.push(semivowel);
    }
}

#[requires(index <= cmavo_end && cmavo_end <= lookahead_end && lookahead_end <= state.len())]
#[ensures(true)]
pub(super) fn cmavo_tail(
    state: &crate::recog_state::CharState,
    index: usize,
    cmavo_end: usize,
    lookahead_end: usize,
    output: &mut Option<&mut String>,
) -> Option<()> {
    // One prefix of the caller buffer. The table key is that prefix length.
    // The caller removed every comma in this span before the call.
    let chars: &[char] = state.chars();
    debug_assert!(
        chars[index..lookahead_end]
            .iter()
            .all(|value| *value != ',')
    );
    let limit = lookahead_end;
    let bits = nucleus_bits(state, limit);
    let visible = &chars[..limit];
    let diphthong_end = |start: usize| -> Option<(char, usize)> {
        let first = match visible.get(start).copied() {
            Some('a' | 'á') => 'a',
            Some('e' | 'é') => 'e',
            Some('i' | 'í') => 'i',
            Some('o' | 'ó') => 'o',
            Some('u' | 'ú') => 'u',
            _ => return None,
        };
        let second = start + 1;
        if second >= visible.len() {
            return None;
        }
        let semivowel = match (first, visible[second]) {
            ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
            ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
            _ => return None,
        };
        let end = second + 1;
        let next = end;
        if next < visible.len() {
            let value = visible[next];
            let repeated = match semivowel {
                'ĭ' => matches!(value, 'i' | 'í' | 'ĭ'),
                'ŭ' => matches!(value, 'u' | 'ú' | 'ŭ'),
                _ => false,
            };
            if repeated {
                return None;
            }
        }
        if nucleus_bit_after_commas(visible, end, &bits) {
            return None;
        }
        Some((semivowel, end))
    };
    let single_end = |start: usize| -> Option<usize> {
        let value = *visible.get(start)?;
        let end = start + 1;
        let followed = nucleus_bit_after_commas(visible, end, &bits);
        if value == 'y' || value == 'ý' {
            return (!followed).then_some(end);
        }
        if !matches!(
            value,
            'a' | 'á' | 'e' | 'é' | 'i' | 'í' | 'o' | 'ó' | 'u' | 'ú' | 'ĭ' | 'ŭ'
        ) {
            return None;
        }
        (!followed).then_some(end)
    };
    let push_char = |output: &mut Option<&mut String>, value: char| {
        if let Some(text) = output.as_mut() {
            text.push(value);
        }
    };
    let push_vowel = |output: &mut Option<&mut String>, value: char| {
        let nucleus = match value {
            'á' => 'á',
            'é' => 'é',
            'í' => 'í',
            'ó' => 'ó',
            'ú' => 'ú',
            'ĭ' => 'i',
            'ŭ' => 'u',
            other => other,
        };
        push_char(output, nucleus);
    };
    let mut jobs = Vec::with_capacity(cmavo_end.saturating_sub(index).saturating_add(1));
    jobs.push(new!(TailJob {
        index,
        step: TailStep::Diphthong,
        saved: 0,
    }));
    while let Some(position) = jobs.len().checked_sub(1) {
        let step = jobs[position].step;
        let cursor = jobs[position].index;
        if matches!(step, TailStep::Diphthong | TailStep::Vowel) {
            let diphthong = step == TailStep::Diphthong;
            let production = if diphthong {
                diphthong_end(cursor)
            } else {
                single_end(cursor).map(|end| (' ', end))
            };
            if let Some((semivowel, after)) = production
                && after <= cmavo_end
            {
                if after == cmavo_end
                    && super::last_cmavo_nucleus_is_allowed(chars, cursor, after, lookahead_end)
                {
                    if diphthong {
                        push_cmavo_diphthong(output, chars, cursor, after, semivowel);
                    } else {
                        push_vowel(output, chars[cursor]);
                    }
                    return Some(());
                }
                if after != cmavo_end && chars.get(after) == Some(&'\'') {
                    let saved = output.as_ref().map(|text| text.len()).unwrap_or(0);
                    if diphthong {
                        push_cmavo_diphthong(output, chars, cursor, after, semivowel);
                    } else {
                        push_vowel(output, chars[cursor]);
                    }
                    push_char(output, '\'');
                    let next_step = if diphthong {
                        TailStep::AfterDiphthong
                    } else {
                        TailStep::AfterVowel
                    };
                    jobs[position] =
                        jobs[position].with_data(data! { step: next_step, saved: saved });
                    jobs.push(new!(TailJob {
                        index: after + 1,
                        step: TailStep::Diphthong,
                        saved: 0,
                    }));
                    continue;
                }
            }
            if diphthong {
                jobs[position] = jobs[position].with_data(data! { step: TailStep::Vowel });
                continue;
            }
            jobs.pop();
            continue;
        }
        let saved = jobs[position].saved;
        if let Some(text) = output.as_mut() {
            text.truncate(saved);
        }
        if step == TailStep::AfterDiphthong {
            jobs[position] = jobs[position].with_data(data! { step: TailStep::Vowel });
            continue;
        }
        jobs.pop();
    }
    None
}

#[requires(index <= end && end <= state.len())]
#[ensures(true)]
pub(super) fn cons_final(
    state: &crate::recog_state::CharState,
    mut index: usize,
    end: usize,
) -> bool {
    let mut stack = vec![index];
    while let Some(cursor) = stack.pop() {
        index = cursor;
        if super::final_syllable_slice(state, index, end) {
            return true;
        }
        for next in super::consonantal_syllable_ends(state, index, end, end)
            .into_iter()
            .rev()
        {
            if next > index {
                stack.push(next);
            }
        }
    }
    false
}

#[requires(index <= lookahead_end && lookahead_end <= state.len())]
#[ensures(true)]
pub(super) fn cons_after(
    state: &crate::recog_state::CharState,
    index: usize,
    lookahead_end: usize,
) -> bool {
    let mut stack = vec![index];
    while let Some(cursor) = stack.pop() {
        if super::syllable_ends(state, cursor, lookahead_end, lookahead_end)
            .iter()
            .any(|syllable_end| *syllable_end > cursor)
        {
            return true;
        }
        for next in super::consonantal_syllable_ends(state, cursor, lookahead_end, lookahead_end)
            .into_iter()
            .rev()
        {
            if next > cursor {
                stack.push(next);
            }
        }
    }
    false
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= state.len())]
#[ensures(true)]
pub(super) fn cons_onset(
    state: &crate::recog_state::CharState,
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> bool {
    let chars: &[char] = state.chars();
    let mut stack = vec![index];
    while let Some(cursor) = stack.pop() {
        if chars.get(cursor) != Some(&'\'') && super::has_parse_onset_end(state, cursor, span_end) {
            return true;
        }
        for next in super::consonantal_syllable_ends(state, cursor, span_end, lookahead_end)
            .into_iter()
            .rev()
        {
            if next > cursor {
                stack.push(next);
            }
        }
    }
    false
}

#[requires(index <= state.len())]
#[ensures(true)]
pub(super) fn cmevla_ranges(
    state: &crate::recog_state::CharState,
    index: usize,
    out: &mut Vec<Vec<LujvoPartRange>>,
) {
    let chars: &[char] = state.chars();
    let cands = super::cmevla_lujvo_range_candidates(state, index, &[]);
    let mut stack = vec![new!(CmevlaJob {
        prefix: Vec::new(),
        cand_i: 0,
        cands,
    })];
    while !stack.is_empty() {
        let top = stack.pop().expect("the loop has a top job");
        let cand_i = top.cand_i;
        if cand_i >= top.cands.len() {
            continue;
        }
        let ranges = top.cands[cand_i].clone();
        let top = top.with_data(data! { cand_i: cand_i + 1 });
        let Some(next) = ranges.last().map(|range| range.end) else {
            stack.push(top);
            continue;
        };
        let mut built = top.prefix.clone();
        built.extend(ranges);
        stack.push(top);
        if next == chars.len() {
            if super::cmevla_lujvo_ranges_match_bonded(chars, &built) {
                out.push(built);
            }
            continue;
        }
        let cands = super::cmevla_lujvo_range_candidates(state, next, &built);
        stack.push(new!(CmevlaJob {
            prefix: built,
            cand_i: 0,
            cands,
        }));
    }
}

/// Cursor where the greedy y-less walk from `start` stops under `end`.
///
/// For one lookahead, the first unstressed y-less end that fits is stable as
/// the cap grows. The walk capped at the lookahead is therefore a chain. A
/// shorter cap follows that chain until the next jump would pass the cap. The
/// only departure is a lower-priority end that still fits under the shorter cap.
#[requires(start <= end && end <= lookahead_end && lookahead_end <= cache.char_count)]
#[ensures(ret.is_err() || (*ret.as_ref().unwrap() >= start && *ret.as_ref().unwrap() <= end))]
pub(super) fn greedy_loop(
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<usize, Blocked> {
    let mut cursor = start;
    loop {
        let chain = super::greedy_max_chain(cursor, lookahead_end, cache)?;
        let index = chain
            .partition_point(|point| *point <= end)
            .saturating_sub(1);
        let landed = chain[index];
        if let Some(&jump) = chain.get(index + 1)
            && jump > end
            && let Some(shorter) =
                super::greedy_shorter_rafsi_end(landed, end, lookahead_end, cache)?
        {
            cursor = shorter;
            continue;
        }
        return Ok(landed);
    }
}

#[requires(prefix_start <= prefix_end && prefix_end <= end && end <= cache.char_count)]
#[ensures(true)]
pub(super) fn enter_cmavo_post(
    prefix_start: usize,
    prefix_end: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let chars: &[char] = cache.char_state().chars();
    let key = new!(RecKey::Post {
        prefix_start,
        prefix_end,
        end,
    });
    match open_memo(
        cache,
        cache.cmavo_post_word(prefix_start, prefix_end, end),
        key,
    )? {
        Open::Cancelled => return Ok(false),
        Open::Hit(result) => return Ok(result),
        Open::Drive => {
            return Ok(drive_bool(
                cache,
                new!(SegmentBoolKey::Post {
                    prefix_start,
                    prefix_end,
                    end,
                }),
            ));
        }
        Open::Own => {}
    }

    let result = match super::next_non_comma_index(chars, prefix_end) {
        None => true,
        Some(index) if index >= end => true,
        Some(index) if super::starts_with_nucleus(&cache.char_state(), index) => false,
        Some(index) => enter_lojban(index, end, cache)?,
    };
    cache.record_cmavo_post_word(prefix_start, prefix_end, end, result);
    Ok(result)
}

#[requires(start <= end && end <= cache.char_count)]
#[ensures(true)]
pub(super) fn enter_lojban(
    start: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let key = new!(RecKey::Lojban { start, end });
    match open_memo(cache, cache.lojban_word(start, end), key)? {
        Open::Cancelled => return Ok(false),
        Open::Hit(result) => return Ok(result),
        Open::Drive => {
            return Ok(drive_bool(
                cache,
                new!(SegmentBoolKey::Lojban { start, end }),
            ));
        }
        Open::Own => {}
    }

    let result = if start >= end {
        false
    } else if super::is_cmevla_slice(&cache.char_state(), start, end) {
        true
    } else {
        let cmavo = enter_cmavo(start, end, cache)?;
        if cmavo {
            true
        } else {
            super::brivla_word_slice_with_cache(start, end, cache)?
        }
    };
    cache.record_lojban_word(start, end, result);
    Ok(result)
}

#[requires(start <= end && end <= cache.char_count)]
#[ensures(true)]
pub(super) fn enter_cmavo(
    start: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let key = new!(RecKey::Cmavo { start, end });
    match open_memo(cache, cache.cmavo_word(start, end), key)? {
        Open::Cancelled => return Ok(false),
        Open::Hit(result) => return Ok(result),
        Open::Drive => {
            return Ok(drive_bool(
                cache,
                new!(SegmentBoolKey::Cmavo { start, end }),
            ));
        }
        Open::Own => {}
    }

    // `PostWordFrom` means the cvcy arm already returned false. The cursor is
    // the post-word end that blocked, and this attempt starts there.
    let resumed_at = match resume(cache) {
        Resume::Cmavo(CmavoResume::PostWordFrom(cursor)) => Some(cursor),
        _ => None,
    };
    // A cmevla span and an empty span store no cvcy key. Main tests those
    // two conditions before the cvcy call.
    let result = if start >= end || super::is_cmevla_slice(&cache.char_state(), start, end) {
        false
    } else if resumed_at.is_none()
        && super::starts_with_cvcy_lujvo_chars_with_cache(start, end, cache)?
    {
        false
    } else {
        let mut found = false;
        let first = resumed_at.unwrap_or(start + 1).max(start + 1);
        for word_end in first..=end {
            set_resume(cache, Resume::Cmavo(CmavoResume::PostWordFrom(word_end)));

            if super::is_cmavo_form_span(&cache.char_state(), start, word_end, end)
                && enter_cmavo_post(start, word_end, end, cache)?
            {
                found = true;
                break;
            }
        }
        found
    };
    cache.record_cmavo_word(start, end, result);
    Ok(result)
}

#[requires(index <= end && end <= cache.char_count)]
#[ensures(true)]
pub(super) fn enter_prefix_post(
    index: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let chars: &[char] = cache.char_state().chars();
    // post_word <- pause / !nucleus lojban_word
    // `pause` is a separator or the end of this slice. `end` stays the stress
    // context and the lookahead. This result stays out of the whole-slice
    // `post_word` table. A later call starts at a later index, so one key runs once.
    let key = new!(RecKey::PrefixPost { index, end });
    match open_memo(cache, cache.prefix_post_word(index, end), key)? {
        Open::Cancelled => return Ok(false),
        Open::Hit(result) => return Ok(result),
        Open::Drive => return Ok(prefix_drive_post(index, end, cache)),
        Open::Own => {}
    }

    let result = match super::next_non_comma_index(chars, index) {
        None => true,
        Some(next) if next >= end || super::is_separator(chars[next]) => true,
        Some(next) if super::starts_with_nucleus(&cache.char_state(), next) => false,
        Some(next) => enter_prefix_lojban(next, end, cache)?,
    };
    cache.record_prefix_post_word(index, end, result);
    Ok(result)
}

#[requires(start <= end && end <= cache.char_count)]
#[ensures(true)]
pub(super) fn enter_prefix_lojban(
    start: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let key = new!(RecKey::PrefixLojban { start, end });
    match open_memo(cache, cache.prefix_lojban_word(start, end), key)? {
        Open::Cancelled => return Ok(false),
        Open::Hit(result) => return Ok(result),
        Open::Drive => return Ok(prefix_drive_lojban(start, end, cache)),
        Open::Own => {}
    }

    let past_cmavo = matches!(
        resume(cache),
        Resume::PrefixLojban(
            PrefixLojbanResume::BrivlaWhole
                | PrefixLojbanResume::BrivlaShaped(_)
                | PrefixLojbanResume::BrivlaPost(_)
        )
    );
    let result = if start >= end || super::is_cmevla_slice(&cache.char_state(), start, end) {
        start < end && super::is_cmevla_slice(&cache.char_state(), start, end)
    } else if !past_cmavo {
        let cmavo = prefix_cmavo(start, end, cache)?;
        if cmavo {
            true
        } else {
            prefix_brivla(start, end, cache)?
        }
    } else {
        prefix_brivla(start, end, cache)?
    };
    cache.record_prefix_lojban_word(start, end, result);
    Ok(result)
}

#[requires(true)]
#[ensures(true)]
fn prefix_cmavo(
    start: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    if start >= end || super::is_cmevla_slice(&cache.char_state(), start, end) {
        return Ok(false);
    }
    let resumed_at = match resume(cache) {
        Resume::PrefixLojban(PrefixLojbanResume::CmavoAt(cursor)) => Some(cursor),
        _ => None,
    };
    if resumed_at.is_none() && super::starts_with_cvcy_lujvo_chars_with_cache(start, end, cache)? {
        return Ok(false);
    }
    let first = resumed_at.unwrap_or(start + 1).max(start + 1);
    for word_end in first..=end {
        set_resume(
            cache,
            Resume::PrefixLojban(PrefixLojbanResume::CmavoAt(word_end)),
        );

        if super::is_cmavo_form_span(&cache.char_state(), start, word_end, end)
            && prefix_cmavo_post(start, word_end, end, cache)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[requires(true)]
#[ensures(true)]
fn prefix_cmavo_post(
    _prefix_start: usize,
    prefix_end: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let chars: &[char] = cache.char_state().chars();
    let Some(index) = super::next_non_comma_index(chars, prefix_end) else {
        return Ok(true);
    };
    if index >= end || super::is_separator(chars[index]) {
        return Ok(true);
    }
    if super::starts_with_nucleus(&cache.char_state(), index) {
        return Ok(false);
    }
    enter_prefix_post(prefix_end, end, cache)
}

#[requires(true)]
#[ensures(true)]
fn prefix_brivla(
    start: usize,
    end: usize,
    cache: &MorphologyRecognitionCache,
) -> Result<bool, Blocked> {
    let chars: &[char] = cache.char_state().chars();
    if start >= end {
        return Ok(false);
    }
    let mut shaped_from = None;
    let mut post_only = None;
    match resume(cache) {
        Resume::PrefixLojban(PrefixLojbanResume::BrivlaShaped(ordinal)) => {
            shaped_from = Some(ordinal);
        }
        Resume::PrefixLojban(PrefixLojbanResume::BrivlaPost(ordinal)) => {
            post_only = Some(ordinal);
        }
        _ => {}
    }
    if shaped_from.is_none() && post_only.is_none() {
        set_resume(cache, Resume::PrefixLojban(PrefixLojbanResume::BrivlaWhole));

        let whole = super::brivla_word_slice_with_cache(start, end, cache)?;
        if whole {
            return Ok(true);
        }
        set_resume(
            cache,
            Resume::PrefixLojban(PrefixLojbanResume::BrivlaShaped(0)),
        );
    }
    let ends = cached_prefix_ends(cache, start, end);
    let start_ord = post_only.or(shaped_from).unwrap_or(0);
    for (ordinal, brivla_end) in ends.iter().copied().enumerate() {
        if ordinal < start_ord {
            continue;
        }
        let retry_post = post_only == Some(ordinal);
        if !retry_post {
            let prefix = &chars[start..brivla_end];
            if !(crate::grammar::has_explicit_brivla_stress_chars(prefix)
                && crate::grammar::explicit_brivla_stress_is_valid_chars(prefix)
                && !crate::grammar::boundary_repeats_diphthong_semivowel_chars(
                    prefix,
                    &chars[brivla_end..end],
                ))
            {
                continue;
            }
            set_resume(
                cache,
                Resume::PrefixLojban(PrefixLojbanResume::BrivlaShaped(ordinal)),
            );

            let shaped = super::brivla_word_slice_with_cache(start, brivla_end, cache)?;
            if !shaped {
                continue;
            }
        }
        set_resume(
            cache,
            Resume::PrefixLojban(PrefixLojbanResume::BrivlaPost(ordinal)),
        );

        let posted = enter_prefix_post(brivla_end, end, cache)?;
        if posted {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn nucleus_scope_keeps_one_buffer_until_clear() {
    let chars = ['a', 'a'];
    let state = crate::recog_state::CharState::new(&chars);
    assert!(!state.nucleus_ready(chars.len()));
    assert!(!nucleus_at(&state, 0));
    assert!(!nucleus_at(&state, 0));
    assert!(!state.nucleus_ready(chars.len()));
    assert!(!nucleus_at(&state, 0));
    assert!(state.nucleus_ready(chars.len()));
    assert!(!nucleus_at(&state, 0));
    assert!(nucleus_at(&state, 1));
    let copy = state.clone_nucleus_bits(chars.len());
    assert_eq!(copy.len(), chars.len() + 1);
    assert!(!copy[0]);
    // A second value for the same characters does not see the first row.
    let fresh = crate::recog_state::CharState::new(&chars);
    assert!(!fresh.nucleus_ready(chars.len()));
    assert!(!nucleus_at(&fresh, 0));
    assert!(!fresh.nucleus_ready(chars.len()));
    assert!(state.nucleus_ready(chars.len()));
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn r23_prefix_ends_keep_the_span() {
    let drive = DriveState::new();
    drive.store_prefix_ends(new!(PrefixSpan { start: 0, end: 2 }), Rc::from([1usize]));
    assert!(
        drive
            .prefix_hit(new!(PrefixSpan { start: 0, end: 2 }))
            .is_some()
    );
    assert!(
        drive
            .prefix_hit(new!(PrefixSpan { start: 0, end: 3 }))
            .is_none()
    );
    assert!(
        drive
            .prefix_hit(new!(PrefixSpan { start: 1, end: 2 }))
            .is_none()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn nested_fresh_table_does_not_keep_the_outer_row() {
    let chars = ['y', 'y', 'y'];
    let outer = crate::recog_state::CharState::new(&chars);
    assert!(nucleus_at(&outer, 0));
    assert!(nucleus_at(&outer, 0));
    assert!(nucleus_at(&outer, 0));
    assert!(outer.nucleus_ready(chars.len()));
    let inner = crate::recog_state::CharState::new(&chars);
    assert!(!inner.nucleus_ready(chars.len()));
    assert!(outer.nucleus_ready(chars.len()));
    let short = crate::recog_state::CharState::new(&['k', 'a', 'n']);
    assert!(!short.nucleus_ready(chars.len()));
    assert!(outer.nucleus_ready(chars.len()));
    assert!(!inner.nucleus_ready(chars.len()));
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_cycle_panic_lets_the_same_cache_store_again() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let chars = ['a'];
    let cache = MorphologyRecognitionCache::new(&chars);
    cache.drive().force_cycle();
    let caught = catch_unwind(AssertUnwindSafe(|| {
        drive(
            &cache,
            new!(RecKey::Cmavo {
                start: 0,
                end: chars.len(),
            }),
        );
    }));
    cache.drive().clear_force();
    let message = caught.expect_err("a self-cycle panics");
    let text = message
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or("");
    assert!(text.contains("re-entered"), "{text}");
    assert!(!cache.drive().dep_blocked());
    cache.record_cmavo_word(0, chars.len(), true);
    assert_eq!(cache.cmavo_word(0, chars.len()), Some(true));
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn unstored_nucleus_bit_matches_full_table() {
    let texts = [
        "kánykla",
        "ai",
        "a,i",
        "a,i,i",
        "aii",
        "yy",
        "y",
        "a",
        "ba",
        "aaa",
        "a'a",
        "ĭa",
        "kanbryyyyty",
        "a'a'a",
    ];
    for text in texts {
        let chars: Vec<char> = text.chars().collect();
        let state = crate::recog_state::CharState::new(&chars);
        for limit in 0..=chars.len() {
            let full = build_nucleus_bits(&chars, limit);
            for index in 0..=limit {
                let bit = nucleus_bit_unstored(&state, index, limit);
                assert_eq!(
                    bit, full[index],
                    "{text} limit {limit} index {index} bit {bit} full {}",
                    full[index]
                );
            }
        }
    }
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn bare_rafsi_parses_on_a_mutated_stem() {
    for text in ["spia", "spria", "klia", "kria", "plia", "bria", "kanria"] {
        let _parsed = crate::LujvoPart::parse_bare_rafsi(text);
    }
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_cmavo_key_is_rejected() {
    assert!(try_new!(RecKey::Cmavo { start: 2, end: 1 }).is_err());
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_post_key_is_rejected() {
    assert!(
        try_new!(RecKey::Post {
            prefix_start: 2,
            prefix_end: 1,
            end: 3,
        })
        .is_err()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_lojban_key_is_rejected() {
    assert!(try_new!(RecKey::Lojban { start: 2, end: 1 }).is_err());
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_prefix_post_key_is_rejected() {
    assert!(try_new!(RecKey::PrefixPost { index: 4, end: 2 }).is_err());
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_prefix_lojban_key_is_rejected() {
    assert!(try_new!(RecKey::PrefixLojban { start: 3, end: 1 }).is_err());
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_lujvo_from_key_is_rejected() {
    assert!(
        try_new!(RecKey::LujvoFrom {
            index: 4,
            end: 2,
            flag: false,
        })
        .is_err()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_lujvo_parts_key_is_rejected() {
    assert!(
        try_new!(RecKey::LujvoParts {
            index: 4,
            end: 2,
            flag: true,
        })
        .is_err()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_reversed_cvcy_key_is_rejected() {
    assert!(try_new!(RecKey::Cvcy { index: 4, end: 2 }).is_err());
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_frame_step_past_the_resume_arms_is_rejected() {
    let key = new!(RecKey::Cmavo { start: 0, end: 1 });
    assert!(
        try_new!(Frame {
            key,
            resume: Resume::LujvoFrom(LujvoFromResume::Start),
        })
        .is_err()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_frame_cursor_past_the_key_end_is_rejected() {
    let key = new!(RecKey::Cmavo { start: 0, end: 1 });
    assert!(
        try_new!(Frame {
            key,
            resume: Resume::Cmavo(CmavoResume::PostWordFrom(2)),
        })
        .is_err()
    );
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn a_cmevla_candidate_past_the_list_is_rejected() {
    assert!(
        try_new!(CmevlaJob {
            prefix: Vec::new(),
            cand_i: 1,
            cands: Vec::new(),
        })
        .is_err()
    );
}
