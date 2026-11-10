//! `EpochState<T>`: a small abstraction around arbitrary state with
//! epoch-delimited mutations and borrow-free immutable refs.
//!
//! The idea is that we can keep an `EpochState<T>` and access `T` in
//! three ways:
//!
//! - Immutably, the usual "projected borrow" way,
//!   with `fn get(&self) -> &T`;
//! - Mutably, the usual "projected borrow" way,
//!   with `fn get_mut(&mut self) -> &mut T`;
//! - Immutably *but without holding an immutable borrow
//!   between reference creation and access times* via
//!   `EpochState::handle` and `EpochStateHandle::get`
//!   with the `get` method taking the `&EpochState<T>` again.
//!
//! The idea is that the reference type `EpochStateHandle<T>` does not
//! actually hold a `&self` borrow, so it's fine for the user to use
//! `get_mut` in the meantime; but if the `T` was actually mutated
//! between the time of `handle` creating the handle and `get`
//! trying to use it, the caller gets a `None` back. So this is a
//! "lazily invalidated immutable borrow" of sorts.
//!
//! This is very useful when Wasmtime needs to hand out types in the
//! API that are nominally handles, and are related to the current
//! state of a `Store`, and must be invalidated when that state
//! changes. For example, the GC host API presents `Unrooted`s that
//! refer to GC objects in the current heap, without rooting (for
//! efficiency), such that any GC heap mutation may invalidate
//! them. We do not want these to hold an immutable borrow of the
//! `Store`, because we do not want to infect every `Val` with a
//! lifetime nor actually hold the `Store` borrowed. But we *do* want
//! to invalidate the unrooted refs if anything about the GC heap
//! actually changes. We can do this with an `EpochState` that
//! encapsulates the GC heap. The encapsulation ensures that we do not
//! miss any invalidation points, and makes this pattern
//! safe-by-construction.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

/// An encapsulated bit of state that can be mutated and that can have
/// lazily-invalidated immutable "handles" that refer to a particular
/// epoch or version in time.
///
/// Epoch handles capture a snapshot of an internal epoch counter that
/// increments every time a mutation happens (every time `get_mut()`
/// is called). Dereferencing an epoch handle is allowed to fail (it
/// returns an `Option<&T>`), and it will fail if the epoch has moved
/// on since the handle was created. If a handle dereference succeeds,
/// this container guarantees that the `T` has not been mutated (no
/// `&mut T` has been handed out) since the handle was created.
pub struct EpochState<T> {
    inner: T,
    stamp: Stamp,
    epoch: Epoch,
}

/// Internal epoch counter.
///
/// This is 64 bits wide so that it never wraps around in practice: a
/// narrower counter could, since some users advance it very often
/// (e.g. on every call between Wasm and the host), and then a stale
/// handle would be accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
struct Epoch(u64);
impl Epoch {
    fn increment(&mut self) {
        self.0 += 1;
    }
}

/// Internal "stamp" distinguishing each `EpochState` instance that
/// exists in the program at runtime. This is what ties an
/// `EpochStateHandle` to an `EpochState`. (The stamp plus the epoch
/// numbers together are what completely identify one state of one
/// `T`.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Stamp(u64);
impl Stamp {
    fn new() -> Stamp {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Stamp(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Handle to one version of `T` in one `EpochState`.
pub struct EpochStateHandle<T> {
    // Note that we intentionally do *not* hold a borrow of the
    // `EpochState` here; that's the entire point of this
    // construct. Instead, the stamp and epoch snapshots uniquely
    // describe one version of one `T` and let us check if it's still
    // the same before turning this handle (plus an actual borrow of
    // the `EpochState` at use-time) into a `&T`.
    stamp: Stamp,
    epoch: Epoch,
    // Note: `fn() -> T`, not `&T` here, because we do not want a
    // lifetime associated with this handle (and it does not actually
    // contain any refs -- the eventual `&T` comes from `get` below
    // that takes another `&EpochState` borrow). And not plain `T`
    // either, so that the handle is `Send` and `Sync` regardless of
    // `T`: it does not hold a `T`.
    _unused: PhantomData<fn() -> T>,
}

// Manual `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` and `Hash` impls
// so we don't have unneeded constraints on `T`.
impl<T> Clone for EpochStateHandle<T> {
    fn clone(&self) -> EpochStateHandle<T> {
        *self
    }
}

impl<T> Copy for EpochStateHandle<T> {}

impl<T> fmt::Debug for EpochStateHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EpochStateHandle")
            .field("stamp", &self.stamp.0)
            .field("epoch", &self.epoch.0)
            .finish()
    }
}

impl<T> PartialEq for EpochStateHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.stamp, self.epoch) == (other.stamp, other.epoch)
    }
}

impl<T> Eq for EpochStateHandle<T> {}

impl<T> Hash for EpochStateHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.stamp, self.epoch).hash(state);
    }
}

impl<T> EpochState<T> {
    /// Create a new EpochState.
    pub fn new(inner: T) -> EpochState<T> {
        EpochState {
            inner,
            stamp: Stamp::new(),
            epoch: Epoch::default(),
        }
    }

    /// Take an immediate immutable borrow of the inner `T`. Does not
    /// invalidate any handles.
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Take a mutable borrow of the inner `T`. Invalidates any
    /// existing handles.
    pub fn get_mut(&mut self) -> &mut T {
        self.invalidate_handles();
        &mut self.inner
    }

    /// Invalidate any existing handles, without borrowing the inner
    /// `T`.
    ///
    /// This is for when the `T` may be mutated by means that this
    /// container cannot see, such as through raw pointers.
    pub fn invalidate_handles(&mut self) {
        self.epoch.increment();
    }

    /// Create a handle that can later be turned into a `&T`, as long
    /// as the `T` is not mutated between now and the later point at
    /// which it is dereferenced.
    pub fn handle(&self) -> EpochStateHandle<T> {
        EpochStateHandle {
            stamp: self.stamp,
            epoch: self.epoch,
            _unused: PhantomData,
        }
    }

    /// Consume this `EpochState`, returning the inner `T`.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> EpochStateHandle<T> {
    /// Dereference this handle, if it is still valid. Otherwise
    /// return `None`.
    ///
    /// A handle is valid exactly and only when `EpochState::get_mut`
    /// (or `EpochState::invalidate_handles`) has not been invoked
    /// between the `EpochState::handle` call that created it, and
    /// this call.
    ///
    /// # Panics
    ///
    /// Panics if this handle came from a different `EpochState`.
    pub fn get<'a>(&self, state: &'a EpochState<T>) -> Option<&'a T> {
        if self.is_valid(state) {
            Some(&state.inner)
        } else {
            None
        }
    }

    /// Determine if this handle is still valid.
    ///
    /// # Panics
    ///
    /// Panics if this handle came from a different `EpochState`.
    pub fn is_valid(&self, state: &EpochState<T>) -> bool {
        assert!(
            self.belongs_to(state),
            "`EpochStateHandle` used with a different `EpochState`"
        );
        state.epoch == self.epoch
    }

    /// Did this handle come from `state`?
    pub fn belongs_to(&self, state: &EpochState<T>) -> bool {
        state.stamp == self.stamp
    }
}
