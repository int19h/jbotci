//! Character tables for one input.
//!
//! Each buffer owns its tables. Each parse owns its grammar flags.
//! The recognizer runs on a heap work stack.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use bityzba::{invariant, new, requires};

/// Nucleus bits and cmavo-form answers for one character buffer.
///
/// The map key is a prefix length inside that buffer. The table lives in the
/// cache or the guard that owns the buffer. Nothing here stores an address.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct CharTables {
    len: usize,
    nucleus: HashMap<usize, Vec<bool>>,
    /// How many one-bit queries this scope has seen for each prefix length.
    /// The third query stores the prefix. The earlier queries do not.
    nucleus_sights: HashMap<usize, u8>,
    cmavo: CmavoFormCache,
}

impl CharTables {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(len: usize) -> Self {
        Self {
            len,
            nucleus: HashMap::new(),
            nucleus_sights: HashMap::new(),
            cmavo: CmavoFormCache::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.len)]
    pub(crate) fn owner_len(&self) -> usize {
        self.len
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn nucleus_bit(&self, limit: usize, index: usize) -> Option<bool> {
        self.nucleus
            .get(&limit)
            .and_then(|bits| bits.get(index).copied())
    }

    /// `true` means the caller computes one bit and does not store the prefix.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn claim_nucleus_limit(&mut self, limit: usize) -> bool {
        let count = self.nucleus_sights.entry(limit).or_insert(0);
        *count = count.saturating_add(1);
        *count < 3
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn nucleus_ready(&self, limit: usize) -> bool {
        self.nucleus.contains_key(&limit)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn clone_nucleus_bits(&self, limit: usize) -> Vec<bool> {
        self.nucleus.get(&limit).cloned().unwrap_or_default()
    }

    #[requires(bits.len() == limit + 1)]
    #[ensures(true)]
    pub(crate) fn store_nucleus_bits(&mut self, limit: usize, bits: Vec<bool>) {
        self.nucleus.insert(limit, bits);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn cmavo_get(&self, start: usize, end: usize, lookahead: usize) -> Option<bool> {
        self.cmavo.get(start, end, lookahead)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn cmavo_set(&mut self, start: usize, end: usize, lookahead: usize, value: bool) {
        let owner_len = self.len;
        self.cmavo.insert(owner_len, start, end, lookahead, value);
    }
}

/// A window of a shared character buffer.
///
/// The window clones the `Rc` pointer. It does not copy the characters.
#[invariant(*origin + *len <= chars.len())]
#[derive(Clone, Debug)]
pub(crate) struct CharBuf {
    chars: Rc<[char]>,
    origin: usize,
    len: usize,
}

#[cfg(test)]
impl CharBuf {
    /// The two windows share one allocation.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn same_buffer(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.as_data().chars, &other.as_data().chars)
    }
}

impl AsRef<[char]> for CharBuf {
    #[requires(true)]
    #[ensures(ret.len() == self.as_data().len)]
    fn as_ref(&self) -> &[char] {
        let data = self.as_data();
        &data.chars[data.origin..data.origin + data.len]
    }
}

/// Nucleus bits and cmavo answers for one character buffer.
///
/// The buffer and the tables are fields of this value. A call receives this
/// value. It cannot pass another buffer's tables. The type invariant makes
/// this a wrapper, and the wrapper has no mutable field access. The tables
/// and the scratch buffer sit in cells so a shared borrow can update them.
/// The character buffer is fixed when the value is built.
/// `origin` and `view_len` select a span of `chars`. A subspan shares `chars`.
#[invariant(*origin + *view_len <= chars.len())]
#[invariant(tables.borrow().owner_len() == *view_len)]
#[derive(Debug)]
pub(crate) struct CharState {
    chars: Rc<[char]>,
    origin: usize,
    view_len: usize,
    tables: RefCell<CharTables>,
    scratch: RefCell<Vec<bool>>,
}

impl CharState {
    #[requires(true)]
    #[ensures(ret.len() == chars.len())]
    pub(crate) fn new(chars: &[char]) -> Self {
        Self::from_rc(Rc::from(chars))
    }

    #[requires(true)]
    #[ensures(ret.len() == chars.len())]
    pub(crate) fn from_rc(chars: Rc<[char]>) -> Self {
        let len = chars.len();
        // `ensures` names `chars` after this body. `new!` moves its fields
        // into a closure, so the stored handle is a clone of the parameter.
        let owned = Rc::clone(&chars);
        new!(Self {
            chars: owned,
            origin: 0,
            view_len: len,
            tables: RefCell::new(CharTables::new(len)),
            scratch: RefCell::new(Vec::new()),
        })
    }

    /// A mid-buffer span of this state. The tables are new.
    ///
    /// The slice is this state's characters from `start` to `end`. The span
    /// shares the parent buffer. It does not copy the characters.
    #[requires(start <= end && end <= self.len())]
    #[ensures(ret.len() == end - start)]
    pub(crate) fn subspan(&self, start: usize, end: usize) -> Self {
        let view_len = end - start;
        new!(Self {
            chars: Rc::clone(&self.chars),
            origin: self.origin + start,
            view_len: view_len,
            tables: RefCell::new(CharTables::new(view_len)),
            scratch: RefCell::new(Vec::new()),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.view_len)]
    pub(crate) fn len(&self) -> usize {
        self.view_len
    }

    /// A clone of the buffer pointer. The clone does not borrow `self`.
    /// The slice is this state's window. The clone does not copy characters.
    #[requires(true)]
    #[ensures(ret.as_data().len == self.len())]
    pub(crate) fn chars_rc(&self) -> CharBuf {
        new!(CharBuf {
            chars: Rc::clone(&self.chars),
            origin: self.origin,
            len: self.view_len,
        })
    }

    /// The characters of this state. The borrow ends with the caller's use.
    #[requires(true)]
    #[ensures(ret.len() == self.len())]
    pub(crate) fn chars(&self) -> &[char] {
        &self.chars[self.origin..self.origin + self.view_len]
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn nucleus_bit(&self, limit: usize, index: usize) -> Option<bool> {
        self.tables.borrow().nucleus_bit(limit, index)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn claim_nucleus_limit(&self, limit: usize) -> bool {
        self.tables.borrow_mut().claim_nucleus_limit(limit)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn nucleus_ready(&self, limit: usize) -> bool {
        self.tables.borrow().nucleus_ready(limit)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn clone_nucleus_bits(&self, limit: usize) -> Vec<bool> {
        self.tables.borrow().clone_nucleus_bits(limit)
    }

    #[requires(bits.len() == limit + 1)]
    #[ensures(true)]
    pub(crate) fn store_nucleus_bits(&self, limit: usize, bits: Vec<bool>) {
        self.tables.borrow_mut().store_nucleus_bits(limit, bits);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn cmavo_get(&self, start: usize, end: usize, lookahead: usize) -> Option<bool> {
        self.tables.borrow().cmavo_get(start, end, lookahead)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn cmavo_set(&self, start: usize, end: usize, lookahead: usize, value: bool) {
        let mut tables = self.tables.borrow_mut();
        tables.cmavo_set(start, end, lookahead, value);
    }

    /// Runs `body` on the scratch buffer of this state.
    ///
    /// The borrow ends when `body` returns. The caller does not keep it.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn with_scratch<R>(&self, body: impl FnOnce(&mut Vec<bool>) -> R) -> R {
        body(&mut self.scratch.borrow_mut())
    }
}

/// Grammar-driver flags for one parse.
///
/// A segmenter owns this value. Each region cache of that parse holds the
/// same `Rc`. A nested parse builds a new value. It cannot clear the outer one.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct ParseFlags {
    depth: Cell<u32>,
    eval: Cell<Option<usize>>,
    block: Cell<Option<usize>>,
    /// Word-start answers for strict boundaries. The key is the source index.
    resolved_strict: RefCell<HashMap<usize, bool>>,
    /// Word-start answers for permissive boundaries. The key is the source index.
    ///
    /// A zoi probe turns permissive boundaries off and back on. The two tables
    /// keep those answers apart. One index is not enough.
    resolved_permissive: RefCell<HashMap<usize, bool>>,
    region_hold: Cell<u32>,
}

impl ParseFlags {
    #[requires(true)]
    #[ensures(!ret.in_driver())]
    #[ensures(!ret.blocked())]
    #[ensures(!ret.region_held())]
    pub(crate) fn new() -> Self {
        Self {
            depth: Cell::new(0),
            eval: Cell::new(None),
            block: Cell::new(None),
            resolved_strict: RefCell::new(HashMap::new()),
            resolved_permissive: RefCell::new(HashMap::new()),
            region_hold: Cell::new(0),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn in_driver(&self) -> bool {
        self.depth.get() > 0
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn enter_driver(&self) {
        self.depth.set(self.depth.get().saturating_add(1));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn leave_driver(&self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn evaluating(&self, index: usize) -> bool {
        self.eval.get() == Some(index)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn take_eval(&self) -> Option<usize> {
        self.eval.take()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn set_eval(&self, index: Option<usize>) {
        self.eval.set(index);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn blocked(&self) -> bool {
        self.block.get().is_some()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn block_at(&self, index: usize) -> crate::segment::drive::Blocked {
        crate::segment::drive::block_source(&self.block, index)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn take_block(&self) -> Option<usize> {
        self.block.take()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn resolved(&self, index: usize, permissive: bool) -> Option<bool> {
        let table = if permissive {
            &self.resolved_permissive
        } else {
            &self.resolved_strict
        };
        table.borrow().get(&index).copied()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn note_resolved(&self, index: usize, permissive: bool, value: bool) {
        let table = if permissive {
            &self.resolved_permissive
        } else {
            &self.resolved_strict
        };
        table.borrow_mut().insert(index, value);
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn region_held(&self) -> bool {
        self.region_hold.get() > 0
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn region_hold(&self) -> u32 {
        self.region_hold.get()
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn note_region_hold(&self) {
        self.region_hold
            .set(self.region_hold.get().saturating_add(1));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn release_region_holds(&self, count: u32) {
        self.region_hold
            .set(self.region_hold.get().saturating_sub(count));
    }

    #[requires(true)]
    #[ensures(self.region_hold() == count)]
    pub(crate) fn set_region_hold(&self, count: u32) {
        self.region_hold.set(count);
    }
}

/// How many lookahead grids one buffer keeps before the spill map.
const CMAVO_GRIDS: usize = 2;

/// Cells in one grid. A longer buffer uses the spill map.
///
/// One cell is one `(start, end)` pair. `2^26` cells is a stride of 8,192.
/// Two grids of that size are 32 MiB of bitsets. A longer buffer uses the spill map.
/// The spill map keeps the same bool, so the bound does not change an answer.
const CMAVO_GRID_CELL_BUDGET: usize = 1 << 26;

/// Present bit and value bit for one `(start, end)` pair.
///
/// The hash map stored a 24-byte key for every answer. On one unspaced word
/// the lookahead is a single index, so the answers are a square on that buffer.
/// A missing cell is not the same as a stored `false`.
#[invariant(true)]
#[derive(Debug)]
struct CmavoGrid {
    stride: usize,
    present: Vec<u64>,
    values: Vec<u64>,
}

impl CmavoGrid {
    #[requires(true)]
    #[ensures(true)]
    fn new(stride: usize) -> Option<Self> {
        let cells = stride.checked_mul(stride)?;
        if cells > CMAVO_GRID_CELL_BUDGET {
            return None;
        }
        let words = cells.div_ceil(64);
        Some(Self {
            stride,
            present: vec![0; words],
            values: vec![0; words],
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self, start: usize, end: usize) -> Option<bool> {
        let bit = self.bit(start, end)?;
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);
        if self.present.get(word).copied().unwrap_or(0) & mask == 0 {
            return None;
        }
        Some(self.values.get(word).copied().unwrap_or(0) & mask != 0)
    }

    #[requires(true)]
    #[ensures(true)]
    fn set(&mut self, start: usize, end: usize, value: bool) -> bool {
        let Some(bit) = self.bit(start, end) else {
            return false;
        };
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);
        let Some(present) = self.present.get_mut(word) else {
            return false;
        };
        let was_present = *present & mask != 0;
        *present |= mask;
        let Some(slot) = self.values.get_mut(word) else {
            return false;
        };
        if value {
            *slot |= mask;
        } else {
            *slot &= !mask;
        }
        !was_present
    }

    #[requires(true)]
    #[ensures(true)]
    fn bit(&self, start: usize, end: usize) -> Option<usize> {
        if start >= self.stride || end >= self.stride {
            return None;
        }
        start.checked_mul(self.stride)?.checked_add(end)
    }
}

/// Cmavo-form answers for one character buffer.
///
/// The first two lookaheads use a bit grid. Later lookaheads use the spill map.
#[invariant(true)]
#[derive(Debug)]
struct CmavoFormCache {
    grids: HashMap<usize, CmavoGrid>,
    spill: HashMap<(usize, usize, usize), bool>,
}

impl CmavoFormCache {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        Self {
            grids: HashMap::new(),
            spill: HashMap::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self, start: usize, end: usize, lookahead: usize) -> Option<bool> {
        if let Some(grid) = self.grids.get(&lookahead) {
            return grid.get(start, end);
        }
        self.spill.get(&(start, end, lookahead)).copied()
    }

    #[requires(true)]
    #[ensures(true)]
    fn insert(
        &mut self,
        owner_len: usize,
        start: usize,
        end: usize,
        lookahead: usize,
        value: bool,
    ) {
        if let Some(grid) = self.grids.get_mut(&lookahead) {
            let _ = grid.set(start, end, value);
            return;
        }
        if self.grids.len() < CMAVO_GRIDS {
            let stride = owner_len.saturating_add(1);
            if let Some(mut grid) = CmavoGrid::new(stride) {
                let _ = grid.set(start, end, value);
                self.grids.insert(lookahead, grid);
                return;
            }
        }
        self.spill.insert((start, end, lookahead), value);
    }
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn cmavo_form_grid_above_the_cell_budget_uses_the_spill_map() {
    // `stride` is `owner_len + 1`. `8193 * 8193` is above `2^26` cells.
    // `CmavoGrid::new` returns before it allocates that square.
    let mut cache = CmavoFormCache::new();
    cache.insert(8192, 0, 1, 7, true);
    cache.insert(8192, 2, 3, 7, false);
    assert!(cache.grids.is_empty());
    assert_eq!(cache.get(0, 1, 7), Some(true));
    assert_eq!(cache.get(2, 3, 7), Some(false));
    assert_eq!(cache.spill.get(&(0, 1, 7)), Some(&true));
    assert_eq!(cache.spill.get(&(2, 3, 7)), Some(&false));

    let mut small = CmavoFormCache::new();
    small.insert(4, 0, 1, 3, true);
    small.insert(4, 0, 1, 4, false);
    small.insert(4, 0, 2, 5, true);
    assert_eq!(small.grids.len(), CMAVO_GRIDS);
    assert_eq!(small.spill.len(), 1);
    assert_eq!(small.get(0, 1, 3), Some(true));
    assert_eq!(small.get(0, 1, 4), Some(false));
    assert_eq!(small.get(0, 2, 5), Some(true));
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn r23_resolved_keeps_the_permissive_flag() {
    let flags = ParseFlags::new();
    flags.note_resolved(3, false, true);
    assert_eq!(flags.resolved(3, false), Some(true));
    assert_eq!(flags.resolved(3, true), None);
    assert_eq!(flags.resolved(4, false), None);
    flags.note_resolved(3, true, false);
    assert_eq!(flags.resolved(3, true), Some(false));
    assert_eq!(flags.resolved(3, false), Some(true));
    assert_eq!(flags.resolved(4, true), None);
}

#[cfg(test)]
#[test]
#[requires(true)]
#[ensures(true)]
fn r23_char_tables_keep_lookahead_and_nucleus_limit() {
    let chars = ['a', 'b', 'c', 'd'];
    let state = CharState::new(&chars);
    state.cmavo_set(0, 1, 2, true);
    assert_eq!(state.cmavo_get(0, 1, 2), Some(true));
    assert_eq!(state.cmavo_get(0, 1, 3), None);
    assert_eq!(state.cmavo_get(1, 2, 2), None);
    assert_eq!(state.cmavo_get(0, 2, 2), None);
    state.store_nucleus_bits(1, vec![true, false]);
    state.store_nucleus_bits(2, vec![false, true, false]);
    assert_eq!(state.nucleus_bit(1, 0), Some(true));
    assert_eq!(state.nucleus_bit(1, 1), Some(false));
    assert_eq!(state.nucleus_bit(2, 0), Some(false));
    let child = state.subspan(0, 2);
    assert_eq!(child.len(), 2);
    assert_eq!(child.chars(), &chars[..2]);
    assert!(state.chars_rc().same_buffer(&child.chars_rc()));
    assert_eq!(child.cmavo_get(0, 1, 2), None);
    assert_eq!(state.cmavo_get(0, 1, 2), Some(true));
}
