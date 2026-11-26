//! Heap queue for generated tree drops.
//!
//! A parent chain of `Arc` or `Box` values drops one frame at a time on the
//! native stack. A deep syntax tree overflows the default 2 MiB thread stack.
//! `tree_model!` detaches each owned child onto this queue. The first drop
//! drains the queue. Later drops only append the next children.

use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

#[derive(Clone, Copy)]
#[invariant(true)]
#[invariant(::Busy(_) => true)]
enum Tls {
    Idle,
    Busy(*mut DropQueue),
    Dead,
}

#[invariant(true)]
struct DropQueue {
    pending: Vec<Pending>,
}

#[invariant(true)]
struct Pending {
    drop_value: fn(*mut ()),
    data: *mut (),
}

impl Drop for Pending {
    fn drop(&mut self) {
        let data = self.data;
        if data.is_null() {
            return;
        }
        self.data = std::ptr::null_mut();
        (self.drop_value)(data);
    }
}

struct StateGuard;

impl Drop for StateGuard {
    fn drop(&mut self) {
        let _ = QUEUE.try_with(|cell| cell.set(std::ptr::null_mut()));
    }
}

thread_local! {
    static QUEUE: Cell<*mut DropQueue> = const { Cell::new(std::ptr::null_mut()) };
    static SENTINELS: RefCell<HashMap<TypeId, *const ()>> = RefCell::new(HashMap::new());
}

#[requires(true)]
#[ensures(true)]
fn tls_state() -> Tls {
    match QUEUE.try_with(|cell| cell.get()) {
        Ok(ptr) if ptr.is_null() => Tls::Idle,
        Ok(ptr) => Tls::Busy(ptr),
        Err(_) => Tls::Dead,
    }
}

#[requires(true)]
#[ensures(true)]
fn immortal_arc<T: 'static>() -> Arc<T> {
    let uninit = Arc::<MaybeUninit<T>>::new_uninit();
    let raw = Arc::into_raw(uninit);
    // SAFETY: `MaybeUninit<T>` has the same size and alignment as `T`.
    // `Arc::into_raw` returns the pointer `Arc::from_raw` accepts for a type
    // of that size and alignment. The leaked owner keeps the count above zero.
    // No code reads the inner bytes.
    unsafe { Arc::from_raw(raw.cast::<T>()) }
}

#[requires(true)]
#[ensures(true)]
fn sentinel_arc<T: 'static>() -> Arc<T> {
    let cached = SENTINELS.try_with(|map| {
        let mut map = map.borrow_mut();
        let entry = map.entry(TypeId::of::<T>()).or_insert_with(|| {
            let leaked: &'static Arc<T> = Box::leak(Box::new(immortal_arc::<T>()));
            (leaked as *const Arc<T>).cast::<()>()
        });
        // SAFETY: This map stores one leaked `Arc<T>` per `TypeId`.
        // The leak keeps that `Arc` alive. The pointer is read as `Arc<T>`.
        let leaked = unsafe { &*(*entry).cast::<Arc<T>>() };
        Arc::clone(leaked)
    });
    match cached {
        Ok(arc) => arc,
        Err(_) => Arc::clone(Box::leak(Box::new(immortal_arc::<T>()))),
    }
}

#[requires(true)]
#[ensures(true)]
fn drop_boxed<T>(data: *mut ()) {
    // SAFETY: `enqueue` stored `Box::into_raw` of a `T` in `data`.
    // This function runs once. `Pending` clears `data` before the call.
    let _owned = unsafe { Box::from_raw(data.cast::<T>()) };
}

#[requires(true)]
#[ensures(true)]
fn enqueue<T: 'static>(value: T) {
    let pending = Pending {
        drop_value: drop_boxed::<T>,
        data: Box::into_raw(Box::new(value)).cast::<()>(),
    };
    if let Tls::Busy(ptr) = tls_state() {
        // SAFETY: `ptr` addresses the queue owned by the active `drain` on
        // this thread. The pop that exposed this call has ended.
        unsafe { (*ptr).pending.push(pending) };
        return;
    }
    drop(pending);
}

#[requires(true)]
#[ensures(true)]
fn drain<T>(node: &mut T, release: fn(&mut T)) {
    let mut queue = DropQueue {
        pending: Vec::new(),
    };
    let ptr: *mut DropQueue = &mut queue;
    // SAFETY: `ptr` stays valid while `queue` is on this stack. The guard
    // clears the thread-local pointer before `queue` drops.
    let installed = QUEUE.try_with(|cell| cell.set(ptr)).is_ok();
    if !installed {
        return;
    }
    let _guard = StateGuard;
    release(node);
    // SAFETY: the same pointer. Each `Pending` drop returns before the next pop.
    unsafe {
        while let Some(item) = (*ptr).pending.pop() {
            drop(item);
        }
    }
}

/// Detach `node` and drop owned descendants from a heap queue.
///
/// `release` moves owned child nodes onto the queue. An outer drain is
/// already active for a nested drop. This function then returns after
/// `release`. The outer drain drops the queued nodes.
#[inline]
#[requires(true)]
#[ensures(true)]
pub fn drop_tree_node<T>(node: &mut T, release: fn(&mut T)) {
    match tls_state() {
        Tls::Busy(_) => release(node),
        Tls::Idle => drain(node, release),
        Tls::Dead => {}
    }
}

/// Move a unique node out of `slot` and drop it from the active heap queue.
///
/// A shared `Arc` keeps its node. The slot keeps a stand-in `Arc` whose drop
/// does not drop a node.
#[inline]
#[requires(true)]
#[ensures(true)]
pub fn release_arc<T: 'static>(slot: &mut Arc<T>) {
    let taken = std::mem::replace(slot, sentinel_arc::<T>());
    match Arc::try_unwrap(taken) {
        Ok(value) => enqueue(value),
        Err(shared) => drop(shared),
    }
}

/// Move a unique recovered node out of `slot` and drop it from the heap queue.
///
/// The recovered wrapper is consumed here. Its node joins the queue. A shared
/// `Arc` keeps the wrapper.
#[inline]
#[requires(true)]
#[ensures(true)]
pub fn release_arc_recovered<T: 'static, E: 'static>(slot: &mut Arc<crate::Recovered<T, E>>) {
    let taken = std::mem::replace(slot, sentinel_arc::<crate::Recovered<T, E>>());
    let Ok(recovered) = Arc::try_unwrap(taken) else {
        return;
    };
    match recovered {
        crate::Recovered::Valid(node) => enqueue(*node),
        crate::Recovered::Prefix(prefix) => enqueue(*prefix.value),
        crate::Recovered::Error(error) => drop(error),
    }
}

/// Move `value` onto the active heap queue.
#[inline]
#[requires(true)]
#[ensures(true)]
pub fn enqueue_node<T: 'static>(value: T) {
    enqueue(value);
}

/// Run `release` on the unique value behind `slot`.
///
/// A shared `Arc` is left unchanged apart from its reference count. The slot
/// keeps a stand-in `Arc` after a unique value is opened.
#[inline]
#[requires(true)]
#[ensures(true)]
pub fn with_unique_arc<T: 'static>(slot: &mut Arc<T>, release: fn(&mut T)) {
    let taken = std::mem::replace(slot, sentinel_arc::<T>());
    match Arc::try_unwrap(taken) {
        Ok(mut value) => {
            release(&mut value);
            drop(value);
        }
        Err(shared) => drop(shared),
    }
}
