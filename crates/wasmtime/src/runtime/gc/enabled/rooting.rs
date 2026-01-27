//! Garbage collection rooting APIs.
//!
//! Rooting prevents GC objects from being collected while they are actively
//! being used.
//!
//! ## Goals
//!
//! We have a few sometimes-conflicting goals with our GC rooting APIs:
//!
//! 1. Safety: It should never be possible to get a use-after-free bug because
//!    the user misused the rooting APIs, the collector "mistakenly" determined
//!    an object was unreachable and collected it, and then the user tried to
//!    access the object. This is our highest priority.
//!
//! 2. Moving GC: Our rooting APIs should moving collectors (such as
//!    generational and compacting collectors) where an object might get
//!    relocated after a collection and we need to update the GC root's pointer
//!    to the moved object. This means we either need cooperation and internal
//!    mutability from individual GC roots as well as the ability to enumerate
//!    all GC roots on the native Rust stack, or we need a level of indirection.
//!
//! 3. Performance: Our rooting APIs should generally be as low-overhead as
//!    possible. They definitely shouldn't require synchronization and locking
//!    to create, access, and drop GC roots.
//!
//! 4. Ergonomics: Our rooting APIs should be, if not a pleasure, then at least
//!    not a burden for users. Additionally, the API's types should be `Sync`
//!    and `Send` so that they work well with async Rust.
//!
//! The two main design axes that trade off the above goals are:
//!
//! - Where the GC reference itself is held. A root object could
//!   directly hold the underlying GC reference (offset into the GC
//!   storage area), which would allow more efficient dereferencing
//!   and access to the referred-to GC object. However, goal (2)
//!   requires that the GC is able to update references when objects
//!   move during a GC. Thus, such "direct roots" would need to be
//!   registered somehow in a global root registry, and would need to
//!   unregister themselves when dropped.
//!
//!   The alternative is to hold some indirect kind of reference to a
//!   GC reference, with the latter stored directly in the `Store` so
//!   the GC can update it freely. This adds one pointer-chasing hop
//!   to accesses, but works much more nicely with ownership
//!   semantics. Logically, the `Store` "owns" the actual pointers;
//!   and rooting types own the slots that they are stored in.
//!
//!   For the above reasons, all of our rooting types below use
//!   indirection. This avoids the need for an unsafe
//!   intrusive-linked-list for global registration, or a shared
//!   reference to a mutex-protected registry, or some other
//!   error-prone technique.
//!
//! - How unrooting occurs. Ideally, a rooting type would implement
//!   the Rust `Drop` trait and unroot itself when the Rust value is
//!   dropped. However, because the rooting state is held in the
//!   `Store`, this direct approach would imply keeping a shared,
//!   mutex-protected handle to the registry in every rooting
//!   object. This would add synchronization overhead to the common
//!   case, and in general would be a bad tradeoff.
//!
//!   However, there are several other approaches:
//!
//!   - The user could use an RAII wrapper that *does* own the `Store`,
//!     and defines a "scope" in which roots are created and then
//!     bulk-unrooted at the close of the scope.
//!   - The rooting type could hold a shared reference to some state
//!     *other* than the full registry, and update a flag in that
//!     state indicating it has been dropped; the `Store` could then
//!     later observe that flag and remove the root. This would have
//!     some allocation cost, but the shared state would be
//!     independent of the `Store` and specific to each root, so would
//!     impose no synchronization overhead between different roots or
//!     the GC itself.
//!   - The rooting type could provide a fully manual `unroot` method,
//!     allowing the user to make use of their own knowledge of their
//!     application's lifetimes and semantics and remove roots when
//!     appropriate.
//!
//!   We provide an implementation of the first two of these
//!   strategies below in `Rooted` and `OwnedRooted`. The last, fully
//!   manual, approach is too difficult to use correctly (it cannot
//!   implement Rust's `Drop` trait, but there is no way in Rust to
//!   enforce that a value must be consumed rather than dropped) so it
//!   is not implemented.
//!
//! ## Two Flavors of Rooting API
//!
//! Okay, with that out of the way, this module provides two flavors
//! of rooting API. One for the common, scoped lifetime case, and one that
//! carries ownership until dropped, and can work as an RAII handle
//! that interacts well with Rust ownership semantics (but at a minor
//! performance cost):
//!
//! 1. `RootScope` and `Rooted<T>`: These are used for temporarily rooting GC
//!    objects for the duration of a scope. The internal implementation takes
//!    advantage of the LIFO property inherent in scopes, making creating and
//!    dropping `Rooted<T>`s and `RootScope`s super fast and roughly equivalent
//!    to bump allocation.
//!
//!    This type is vaguely similar to V8's [`HandleScope`].
//!
//!    [`HandleScope`]: https://v8.github.io/api/head/classv8_1_1HandleScope.html
//!
//!    Note that `Rooted<T>` can't be statically tied to its context scope via a
//!    lifetime parameter, unfortunately, as that would allow the creation of
//!    only one `Rooted<T>` at a time, since the `Rooted<T>` would take a borrow
//!    of the whole context.
//!
//!    This supports the common use case for rooting and provides good
//!    ergonomics.
//!
//! 2. `OwnedRooted<T>`: This is a root that manages rooting and
//!    unrooting automatically with its lifetime as a Rust value. In
//!    other words, the continued existence of the Rust value ensures
//!    the rooting of the underlying GC reference; and when the Rust
//!    value is dropped, the underlying GC reference is no longer
//!    rooted.
//!
//!    Internally, this root holds a shared reference to a
//!    *root-specific* bit of state that is also tracked and observed
//!    by the `Store`.  This means that there is minor memory
//!    allocation overhead (an `Arc<()>`) for each such root; this
//!    memory is shared over all clones of this root. The rooted GC
//!    reference is *logically* unrooted as soon as the last clone of
//!    this root is dropped. Internally the root may still exist until
//!    the next GC, or "root trim" when another `OwnedRooted` is
//!    created, but that is unobservable externally, and will not
//!    result in any additional GC object lifetime because it is
//!    always cleaned up before a gc.
//!
//!    This type is roughly similar to SpiderMonkey's [`PersistentRooted<T>`],
//!    although they register roots on a per-thread `JSContext`, avoiding
//!    mutation costs in a way that is not viable for Wasmtime (which needs
//!    `Store`s to be `Send`).
//!
//!    [`PersistentRooted<T>`]: http://devdoc.net/web/developer.mozilla.org/en-US/docs/Mozilla/Projects/SpiderMonkey/JSAPI_reference/JS::PersistentRooted.html
//!
//! At the end of the day, all of the above root types are just tagged
//! indices into the store's `RootSet`. This indirection allows
//! working with Rust's borrowing discipline (we use `&mut Store` to
//! represent mutable access to the GC heap) while still allowing
//! rooted references to be moved around without tying up the whole
//! store in borrows. Additionally, and crucially, this indirection
//! allows us to update the *actual* GC pointers in the `RootSet` and
//! support moving GCs (again, as mentioned above).
//!
//! ## Unrooted References
//!
//! We generally don't expose *unrooted* GC references in the Wasmtime API at
//! this time -- and I expect it will be a very long time before we do, but in
//! the limit we may want to let users define their own GC-managed types that
//! participate in GC tracing and all that -- so we don't have to worry about
//! failure to root an object causing use-after-free bugs or failing to update a
//! GC root pointer after a moving GC as long as users stick to our safe rooting
//! APIs. (The one exception is `ValRaw`, which does hold raw GC references. But
//! with `ValRaw` all bets are off and safety is 100% up to the user.)
//!
//! We do, however, have to worry about these things internally. So first of
//! all, try to avoid ever working with unrooted GC references if you
//! can. However, if you really must, consider also using an `AutoAssertNoGc`
//! across the block of code that is manipulating raw GC references.

use crate::runtime::vm::{GcRootsList, GcStore, VMGcRef};
use crate::{
    AsContext, AsContextMut, GcRef, Result, RootedGcRef,
    store::{AsStoreOpaque, AutoAssertNoGc, StoreId, StoreOpaque},
};
use crate::{ValRaw, prelude::*};
use alloc::sync::{Arc, Weak};
use core::any;
use core::marker;
use core::mem::{self, MaybeUninit};
use core::num::{NonZeroU64, NonZeroUsize};
use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};
use wasmtime_core::slab::{Id as SlabId, Slab};

mod sealed {
    use super::*;

    /// Sealed, `wasmtime`-internal trait for GC references.
    ///
    /// # Safety
    ///
    /// All types implementing this trait must:
    ///
    /// * Be a newtype of a `GcRootIndex`
    ///
    /// * Not implement `Copy` or `Clone`
    ///
    /// * Only have `&self` methods.
    pub unsafe trait GcRefImpl: Sized {
        /// Transmute a `&GcRootIndex` into an `&Self`.
        fn transmute_ref(index: &GcRootIndex) -> &Self;
    }

    /// Sealed, `wasmtime`-internal trait for the common methods on rooted GC
    /// references.
    pub trait RootedGcRefImpl<T: GcRef> {
        /// Get this rooted GC reference's raw `VMGcRef` out of the store's GC
        /// root set.
        ///
        /// Returns `None` for objects that have since been unrooted (eg because
        /// its associated `RootedScope` was dropped).
        ///
        /// Panics if this root is not associated with the given store.
        fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef>;

        /// Same as `get_gc_ref` but returns an error instead of `None` for
        /// objects that have been unrooted.
        fn try_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
            self.get_gc_ref(store).ok_or_else(|| {
                format_err!("attempted to use a garbage-collected object that has been unrooted")
            })
        }

        /// Get a clone of this rooted GC reference's raw `VMGcRef` out of the
        /// store's GC root set.
        ///
        /// Returns `None` for objects that have since been unrooted (eg because
        /// its associated `RootedScope` was dropped).
        ///
        /// Panics if this root is not associated with the given store.
        fn clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Option<VMGcRef> {
            let gc_ref = self.get_gc_ref(store)?.unchecked_copy();
            Some(store.clone_gc_ref(&gc_ref))
        }

        /// Same as `clone_gc_ref` but returns an error instead of `None` for
        /// objects that have been unrooted.
        fn try_clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
            let gc_ref = self.try_gc_ref(store)?.unchecked_copy();
            Ok(store.clone_gc_ref(&gc_ref))
        }
    }
}
pub(crate) use sealed::*;

/// The index of a GC root inside a particular store's GC root set.
///
/// Can be either a LIFO- or owned-rooted object, depending on the
/// `PackedIndex`.
///
/// Every `T` such that `T: GcRef` must be a newtype over this `GcRootIndex`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
// Just `pub` to avoid `warn(private_interfaces)` in public APIs, which we can't
// `allow(...)` on our MSRV yet.
#[doc(hidden)]
#[repr(C)] // NB: if this layout changes be sure to change the C API as well
pub struct GcRootIndex {
    store_id: StoreId,
    generation: u32,
    index: PackedIndex,
}

const _: () = {
    // NB: these match the C API which should also be updated if this changes
    assert!(mem::size_of::<GcRootIndex>() == 16);
    assert!(mem::align_of::<GcRootIndex>() == mem::align_of::<u64>());
};

impl GcRootIndex {
    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.store_id == store.id()
    }

    /// Same as `RootedGcRefImpl::get_gc_ref` but not associated with any
    /// particular `T: GcRef`.
    ///
    /// We must avoid triggering a GC while holding onto the resulting raw
    /// `VMGcRef` to avoid use-after-free bugs and similar. The `'a` lifetime
    /// threaded from the `store` to the result will normally prevent GCs
    /// statically, at compile time, since performing a GC requires a mutable
    /// borrow of the store. However, if you call `VMGcRef::unchecked_copy` on
    /// the resulting GC reference, then all bets are off and this invariant is
    /// up to you to manually uphold. Failure to uphold this invariant is memory
    /// safe but will lead to general incorrectness such as panics and wrong
    /// results.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not associated with the given store.
    pub(crate) fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        if let Some(index) = self.index.as_lifo() {
            let entry = store.gc_roots().lifo_roots.get(index)?;
            if entry.generation == self.generation {
                Some(&entry.gc_ref)
            } else {
                None
            }
        } else if let Some(id) = self.index.as_owned() {
            let gc_ref = store.gc_roots().owned_rooted.get(id);
            debug_assert!(gc_ref.is_some());
            gc_ref
        } else {
            unreachable!()
        }
    }

    /// Same as `get_gc_ref` but returns an error instead of `None` if
    /// the GC reference has been unrooted.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not associated with the given store.
    pub(crate) fn try_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
        self.get_gc_ref(store).ok_or_else(|| {
            format_err!("attempted to use a garbage-collected object that has been unrooted")
        })
    }

    /// Same as `RootedGcRefImpl::clone_gc_ref` but not associated with any
    /// particular `T: GcRef`.
    pub(crate) fn try_clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
        let gc_ref = self.try_gc_ref(store)?.unchecked_copy();
        Ok(store.clone_gc_ref(&gc_ref))
    }
}

/// This is a bit-packed version of
///
/// ```ignore
/// enum {
///     Lifo(usize),
///     Owned(SlabId),
/// }
/// ```
///
/// where the high bit is the discriminant and the lower 31 bits are the
/// payload.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
struct PackedIndex(u32);

impl Debug for PackedIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(index) = self.as_lifo() {
            f.debug_tuple("PackedIndex::Lifo").field(&index).finish()
        } else if let Some(id) = self.as_owned() {
            f.debug_tuple("PackedIndex::Owned").field(&id).finish()
        } else {
            unreachable!()
        }
    }
}

impl PackedIndex {
    const DISCRIMINANT_MASK: u32 = 0b1 << 31;
    const LIFO_DISCRIMINANT: u32 = 0b0 << 31;
    const OWNED_DISCRIMINANT: u32 = 0b1 << 31;
    const PAYLOAD_MASK: u32 = !Self::DISCRIMINANT_MASK;

    fn new_lifo(index: usize) -> PackedIndex {
        let index32 = u32::try_from(index).unwrap();
        assert_eq!(index32 & Self::DISCRIMINANT_MASK, 0);
        let packed = PackedIndex(Self::LIFO_DISCRIMINANT | index32);
        debug_assert!(packed.is_lifo());
        debug_assert_eq!(packed.as_lifo(), Some(index));
        debug_assert!(!packed.is_owned());
        debug_assert!(packed.as_owned().is_none());
        packed
    }

    fn new_owned(id: SlabId) -> PackedIndex {
        let raw = id.into_raw();
        assert_eq!(raw & Self::DISCRIMINANT_MASK, 0);
        let packed = PackedIndex(Self::OWNED_DISCRIMINANT | raw);
        debug_assert!(packed.is_owned());
        debug_assert_eq!(packed.as_owned(), Some(id));
        debug_assert!(!packed.is_lifo());
        debug_assert!(packed.as_lifo().is_none());
        packed
    }

    fn discriminant(&self) -> u32 {
        self.0 & Self::DISCRIMINANT_MASK
    }

    fn is_lifo(&self) -> bool {
        self.discriminant() == Self::LIFO_DISCRIMINANT
    }

    fn is_owned(&self) -> bool {
        self.discriminant() == Self::OWNED_DISCRIMINANT
    }

    fn payload(&self) -> u32 {
        self.0 & Self::PAYLOAD_MASK
    }

    fn as_lifo(&self) -> Option<usize> {
        if self.is_lifo() {
            Some(usize::try_from(self.payload()).unwrap())
        } else {
            None
        }
    }

    fn as_owned(&self) -> Option<SlabId> {
        if self.is_owned() {
            Some(SlabId::from_raw(self.payload()))
        } else {
            None
        }
    }
}

/// The set of all embedder-API GC roots in a single store/heap.
#[derive(Debug, Default)]
pub(crate) struct RootSet {
    /// GC roots with arbitrary lifetime that are unrooted when
    /// liveness flags are cleared (seen during a trimming pass), for
    /// use with `OwnedRooted<T>`.
    owned_rooted: Slab<VMGcRef>,

    /// List of liveness flags and corresponding `SlabId`s into the
    /// `owned_rooted` slab.
    liveness_flags: Vec<(Weak<()>, SlabId)>,

    /// High-water mark for liveness flag trimming. We use this to
    /// ensure we have amortized constant-time behavior on adding
    /// roots. See note below on `trim_liveness_flags()`.
    liveness_trim_high_water: Option<NonZeroUsize>,

    /// Strictly LIFO-ordered GC roots, for use with `RootScope` and
    /// `Rooted<T>`.
    lifo_roots: Vec<LifoRoot>,

    /// Generation counter for entries to prevent ABA bugs with `RootScope` and
    /// `Rooted<T>`.
    lifo_generation: u32,
}

#[derive(Debug)]
struct LifoRoot {
    generation: u32,
    gc_ref: VMGcRef,
}

impl RootSet {
    pub(crate) fn trace_roots(&mut self, gc_roots_list: &mut GcRootsList) {
        log::trace!("Begin trace user LIFO roots");
        for root in &mut self.lifo_roots {
            unsafe {
                gc_roots_list.add_root((&mut root.gc_ref).into(), "user LIFO root");
            }
        }
        log::trace!("End trace user LIFO roots");

        log::trace!("Begin trace user owned roots");
        for (_id, root) in self.owned_rooted.iter_mut() {
            unsafe {
                gc_roots_list.add_root(root.into(), "user owned root");
            }
        }
        log::trace!("End trace user owned roots");
    }

    /// Enter a LIFO rooting scope.
    ///
    /// Returns an integer that should be passed unmodified to `exit_lifo_scope`
    /// when the scope is finished.
    ///
    /// Calls to `{enter,exit}_lifo_scope` must happen in a strict LIFO order.
    #[inline]
    pub(crate) fn enter_lifo_scope(&self) -> usize {
        self.lifo_roots.len()
    }

    /// Exit a LIFO rooting scope.
    ///
    /// The `scope` argument must be the result of the corresponding
    /// `enter_lifo_scope` call.
    ///
    /// Calls to `{enter,exit}_lifo_scope` must happen in a strict LIFO order.
    #[inline]
    pub(crate) fn exit_lifo_scope(&mut self, gc_store: Option<&mut GcStore>, scope: usize) {
        debug_assert!(self.lifo_roots.len() >= scope);

        // If we actually have roots to unroot, call an out-of-line slow path.
        if self.lifo_roots.len() > scope {
            self.exit_lifo_scope_slow(gc_store, scope);
        }
    }

    #[inline(never)]
    #[cold]
    fn exit_lifo_scope_slow(&mut self, mut gc_store: Option<&mut GcStore>, scope: usize) {
        self.lifo_generation += 1;

        // TODO: In the case where we have a tracing GC that doesn't need to
        // drop barriers, this should really be:
        //
        //     self.lifo_roots.truncate(scope);

        let mut lifo_roots = mem::take(&mut self.lifo_roots);
        for root in lifo_roots.drain(scope..) {
            // Only drop the GC reference if we actually have a GC store. How
            // can we have a GC reference but not a GC store? If we've only
            // created `i31refs`, we never force a GC store's allocation. This
            // is fine because `i31ref`s never need drop barriers.
            if let Some(gc_store) = &mut gc_store {
                gc_store.drop_gc_ref(root.gc_ref);
            }
        }
        self.lifo_roots = lifo_roots;
    }

    pub(crate) fn with_lifo_scope<S, T>(store: &mut S, f: impl FnOnce(&mut S) -> T) -> T
    where
        S: ?Sized + DerefMut<Target = StoreOpaque>,
    {
        let scope = store.gc_roots().enter_lifo_scope();
        let ret = f(store);
        store.exit_gc_lifo_scope(scope);
        ret
    }

    pub(crate) fn push_lifo_root(&mut self, store_id: StoreId, gc_ref: VMGcRef) -> GcRootIndex {
        let generation = self.lifo_generation;
        let index = self.lifo_roots.len();
        let index = PackedIndex::new_lifo(index);
        self.lifo_roots.push(LifoRoot { generation, gc_ref });
        GcRootIndex {
            store_id,
            generation,
            index,
        }
    }

    /// Trim any stale (dropped) owned roots.
    ///
    /// `OwnedRooted` is implemented in a way that avoids the need to
    /// have or keep a reference to the store (and thus this struct)
    /// during its drop operation: to allow it to be independent, it
    /// holds a shared reference to some other memory and sets that
    /// "liveness flag" appropriately, then we later observe dead
    /// liveness flags during a periodic scan and actually deallocate
    /// the roots. We use an `Arc<()>` for this: it permits cheap
    /// cloning, and it has minimal memory overhead. We hold a weak
    /// ref in a list alongside the actual `GcRootIndex`, and we free
    /// that index in the slab of owned roots when we observe that
    /// only our weak ref remains.
    ///
    /// This trim step is logically separate from a full GC, though it
    /// would not be very productive to do a GC without doing a
    /// root-trim first: the root-trim should be quite a lot cheaper,
    /// and it will allow for more garbage to exist.
    ///
    /// There is, additionally, nothing stopping us from doing trims
    /// more often than just before each GC, and there are reasons
    /// this could be a good idea: for example, a user program that
    /// creates and removes many roots (perhaps as it accesses the GC
    /// object graph) but ultimately is dealing with a static graph,
    /// without further allocation, may need the "root set" to be GC'd
    /// independently from the actual heap. Thus, we could trim before
    /// adding a new root to ensure we don't grow that unboundedly (or
    /// force an otherwise unneeded GC).
    ///
    /// The first case, just before GC, wants a "full trim": there's
    /// no reason not to unroot as much as possible before we do the
    /// expensive work of tracing the whole heap.
    ///
    /// On the other hand, the second case, adding a new root, wants a
    /// kind of trim that is amortized constant time. Consider: if we
    /// have some threshold for the trim, say N roots, and the user
    /// program continually adds and removes one root such that it
    /// goes just over the threshold, we might scan all N liveness
    /// flags for each step, resulting in quadratic behavior overall.
    ///
    /// Thus, we implement a "high water mark" algorithm to guard
    /// against this latter case: on the add-a-new-root case, we trim
    /// only if the list is longer than the high water mark, and we
    /// set the high water mark each time based on the after-trim
    /// size. See below for details on this algorithm.
    ///
    /// `eager` chooses whether we eagerly trim roots or pre-filter
    /// using the high-water mark.
    ///
    /// # Growth Algorithm
    ///
    /// We want to balance two factors: we must ensure that the
    /// algorithmic complexity of creating a new root is amortized
    /// O(1), and we must ensure that repeated creation and deletion
    /// of roots without any GC must result in a root-set that has a
    /// size linear in the actual live-root-set size. Stated formally:
    ///
    /// 1. Root creation must be O(1), amortized
    /// 2. liveness_flags.len() must be O(|max live owned roots|),
    ///    i.e., must not grow to larger than a constant multiple of
    ///    the maximum working-root-set size of the program.
    ///
    /// Note that a naive exponential-growth-of-threshold algorithm,
    /// where we trim when the root set reaches 1, 2, 4, 8, 16, 32,
    /// ... elements, provides the first but *not* the second
    /// property. A workload that has a constant live root set but
    /// creates and drops roots constantly (say, as it's traversing a
    /// static graph and moving "fingers" through it) will cause the
    /// `liveness_flags` array to grow unboundedly.
    ///
    /// Instead, it turns out that we can achieve both of these goals
    /// with a simple rule: we trim when the root list length reaches
    /// a high-water mark; and then *after* trimming, we set the
    /// high-water mark equal to the resulting live-root count
    /// multiplied by a factor (e.g., 2).
    ///
    /// ## Proof
    ///
    /// - Root creation is O(1)
    ///
    ///   Assume a sequence of root creation and drop events (and no
    ///   GCs, with a static GC graph, in the worst case -- only roots
    ///   are changing). We want to show that after N root creations,
    ///   we have incurred only only O(N) cost scanning the
    ///   `liveness_flags` list over the whole sequence.
    ///
    ///   Assume a default high-water mark of D (e.g., 8) at
    ///   initialization with an empty root list.
    ///
    ///   Consider "epochs" in the sequence split by trim events where
    ///   we scan the root list. Proceed by induction over epochs to
    ///   show: after each epoch, we will have scanned at most 2N
    ///   roots after N root creations.
    ///
    ///   (These epochs don't exist in the algorithm: this is just a
    ///   mechanism to analyze the behavior.)
    ///
    ///   Base case: after the first epoch, with D root creations, we
    ///   will scan D roots.
    ///
    ///   Induction step: we have created N roots and scanned at most
    ///   2N roots. After previous scan, L roots are still live; then
    ///   we set the high-water mark for next scan at 2L. The list
    ///   already has L, so after another L root creations, the epoch
    ///   ends. We will then incur a scan cost of 2L (the full
    ///   list). At that point we have thus seen N + L root creations,
    ///   with 2N + 2L scan cost; the invariant holds.
    ///
    ///   (It's counter-intuitive that *not* raising the high-water
    ///   mark exponentially can still result in a constant amortized
    ///   cost! One intuition to understand this is that each root
    ///   that remains alive after a scan pushes the next high-water
    ///   mark up by one, so requires a new root creation to "pay for"
    ///   its next scan. So any given root may be scanned many times,
    ///   but each such root ensures other root creations happen to
    ///   maintain the amortized cost.)
    ///
    /// - `liveness_flags.len()` is always O(|max live roots|)
    ///
    ///   Before the first trim, we have between 0 and D live roots,
    ///   which is O(1) (`D` is a compile-time constant).
    ///
    ///   Just after a trim, the `liveness_flags` list has only live
    ///   roots, and the max live-root count is at least the count at
    ///   this time, so the property holds.
    ///
    ///   The instantaneous maximum number of live roots is greater
    ///   than or equal to the maximum number of live roots observed
    ///   during a trim. (The trim is just some point in time, and the
    ///   max at some point in time is at most the overall max.)
    ///
    ///   The high-water mark is set at 2 * `liveness_flags.len()`
    ///   after a trim, i.e., the number of live roots at that
    ///   time. We trim when we reach the high-water mark. So the
    ///   length of the array cannot exceed 2 *
    ///   `liveness_flags.len()`, which is less than or equal to the
    ///   overall max. So transitively, the list length at any time is
    ///   always O(|max live roots|).
    ///
    /// We thus have tight bounds (deterministic, not randomized) for
    /// all possible sequences of root creation/dropping, ensuring
    /// robustness.
    pub(crate) fn trim_liveness_flags(&mut self, gc_store: &mut GcStore, eager: bool) {
        const DEFAULT_HIGH_WATER: usize = 8;
        const GROWTH_FACTOR: usize = 2;
        let high_water_mark = self
            .liveness_trim_high_water
            .map(|x| x.get())
            .unwrap_or(DEFAULT_HIGH_WATER);
        if !eager && self.liveness_flags.len() < high_water_mark {
            return;
        }

        self.liveness_flags.retain(|(flag, index)| {
            if flag.strong_count() == 0 {
                // No more `OwnedRooted` instances are holding onto
                // this; dealloc the index and drop our Weak.
                let gc_ref = self.owned_rooted.dealloc(*index);
                gc_store.drop_gc_ref(gc_ref);
                // Don't retain in the list.
                false
            } else {
                // Retain in the list.
                true
            }
        });

        let post_trim_len = self.liveness_flags.len();
        let high_water_mark = core::cmp::max(
            DEFAULT_HIGH_WATER,
            post_trim_len.saturating_mul(GROWTH_FACTOR),
        );
        self.liveness_trim_high_water = Some(NonZeroUsize::new(high_water_mark).unwrap());
    }
}

/// A scoped, rooted reference to a garbage-collected `T`.
///
/// A `Rooted<T>` is a strong handle to a garbage-collected `T`, preventing its
/// referent (and anything else transitively referenced) from being collected by
/// the GC during the scope within which this `Rooted<T>` was created.
///
/// When the context exits this `Rooted<T>`'s scope, the underlying GC object is
/// automatically unrooted and any further attempts to use access the underlying
/// object will return errors or otherwise fail.
///
/// `Rooted<T>` dereferences to its underlying `T`, allowing you to call `T`'s
/// methods.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut store = Store::<()>::default();
///
/// // Allocating a GC object returns a `Rooted<T>`.
/// let hello: Rooted<ExternRef> = ExternRef::new(&mut store, "hello")?;
///
/// // Because `Rooted<T>` derefs to `T`, we can call `T` methods on a
/// // `Rooted<T>`. For example, we can call the `ExternRef::data` method when we
/// // have a `Rooted<ExternRef>`.
/// let data = hello
///     .data(&store)?
///     .ok_or_else(|| Error::msg("externref has no host data"))?
///     .downcast_ref::<&str>()
///     .ok_or_else(|| Error::msg("not a str"))?;
/// assert_eq!(*data, "hello");
///
/// // A `Rooted<T>` roots its underlying GC object for the duration of the
/// // scope of the store/caller/context that was passed to the method that created
/// // it. If we only want to keep a GC reference rooted and alive temporarily, we
/// // can introduce new scopes with `RootScope`.
/// {
///     let mut scope = RootScope::new(&mut store);
///
///     // This `Rooted<T>` is automatically unrooted after `scope` is dropped,
///     // allowing the collector to reclaim its GC object in the next GC.
///     let scoped_ref = ExternRef::new(&mut scope, "goodbye");
/// }
///
/// let module = Module::new(store.engine(), r#"
///     (module
///         (global (export "global") (mut externref) (ref.null extern))
///         (table (export "table") 10 externref)
///         (func (export "func") (param externref) (result externref)
///             local.get 0
///         )
///     )
/// "#)?;
/// let instance = Instance::new(&mut store, &module, &[])?;
///
/// // GC references returned from calls into Wasm also return (optional, if the
/// // Wasm type is nullable) `Rooted<T>`s.
/// let result: Option<Rooted<_>> = instance
///     .get_typed_func::<Option<Rooted<ExternRef>>, Option<Rooted<ExternRef>>>(&mut store, "func")?
///     .call(&mut store, Some(hello))?;
///
/// // Similarly, getting a GC reference from a Wasm instance's exported global
/// // or table yields a `Rooted<T>`.
///
/// let global = instance
///     .get_global(&mut store, "global")
///     .ok_or_else(|| Error::msg("missing `global` export"))?;
/// let global_val = global.get(&mut store);
/// let global_ref: Option<&Rooted<_>> = global_val
///     .externref()
///     .ok_or_else(|| Error::msg("not an externref"))?;
///
/// let table = instance.get_table(&mut store, "table").unwrap();
/// let table_elem = table
///     .get(&mut store, 3)
///     .ok_or_else(|| Error::msg("table out of bounds"))?;
/// let table_elem_ref: Option<&Rooted<_>> = table_elem
///     .as_extern()
///     .ok_or_else(|| Error::msg("not an externref"))?;
/// # Ok(())
/// # }
/// ```
///
/// # Differences Between `Rooted<T>` and `OwnedRooted<T>`
///
/// While `Rooted<T>` is automatically unrooted when its scope is
/// exited, this means that `Rooted<T>` is only valid for strictly
/// last-in-first-out (LIFO, aka stack order) lifetimes. This is in
/// contrast to [`OwnedRooted<T>`][crate::OwnedRooted], which supports
/// rooting GC objects for arbitrary lifetimes.
///
/// | Type                                         | Supported Lifetimes         | Unrooting | Cost                             |
/// |----------------------------------------------|-----------------------------|-----------|----------------------------------|
/// | [`Rooted<T>`][crate::Rooted]                 | Strictly LIFO / stack order | Automatic | very low (LIFO array)            |
/// | [`OwnedRooted<T>`][crate::OwnedRooted]       | Arbitrary                   | Automatic | medium (separate `Arc` to track) |
///
/// `Rooted<T>` should suffice for most use cases, and provides decent
/// ergonomics. In cases where LIFO scopes are difficult to reason
/// about, e.g. heap-managed data structures, or when they may cause
/// erroneous behavior, e.g. in errors that are propagated up the call
/// stack, `OwnedRooted<T>` provides very safe ergonomics but at a
/// small dynamic cost for the separate tracking allocation.
///
/// # Scopes
///
/// Wasmtime automatically creates two kinds of scopes:
///
/// 1. A [`Store`][crate::Store] is the outermost rooting scope. Creating a
///    `Root<T>` directly inside a `Store` permanently roots the underlying
///    object.
///
/// 2. A [`Caller`][crate::Caller] provides a rooting scope for the duration of
///    a call from Wasm into a host function. Any objects rooted in a `Caller`
///    will be unrooted after the host function returns. Note that there can be
///    nested `Caller` scopes in the case where Wasm calls a host function,
///    creating the first `Caller` and its rooting scope , and then the host
///    function calls a Wasm function which then calls another host function,
///    creating a second `Caller` and a second rooting scope. This nesting can
///    be arbitrarily deep.
///
/// Additionally, if you would like to define finer-grained rooting scopes,
/// Wasmtime provides the [`RootScope`][crate::RootScope] type.
///
/// Scopes are always nested in a last-in-first-out (LIFO) order. An outer scope
/// is never exited (and the `Rooted<T>`s defined within it are never
/// automatically unrooted) while an inner scope is still active. All inner
/// scopes are exited before their outer scopes.
///
/// The following diagram illustrates various rooting scopes over time, how they
/// nest, and when their `Rooted<T>`s are automatically unrooted:
///
/// ```text
/// ----- new Store
///   |
///   |
///   | let a: Rooted<T> = ...;
///   |
///   |
///   | ----- call into Wasm
///   |   |
///   |   |
///   |   | ----- Wasm calls host function F
///   |   |   |
///   |   |   |
///   |   |   | let b: Rooted<T> = ...;
///   |   |   |
///   |   |   |
///   |   |   | ----- F calls into Wasm
///   |   |   |   |
///   |   |   |   |
///   |   |   |   | ----- Wasm call host function G
///   |   |   |   |   |
///   |   |   |   |   |
///   |   |   |   |   | let c: Rooted<T> = ...;
///   |   |   |   |   |
///   |   |   |   |   |
///   |   |   |   | ----- return to Wasm from host function G (unroots `c`)
///   |   |   |   |
///   |   |   |   |
///   |   |   | ----- Wasm returns to F
///   |   |   |
///   |   |   |
///   |   | ----- return from host function F (unroots `b`)
///   |   |
///   |   |
///   | ----- return from Wasm
///   |
///   |
///   | ----- let scope1 = RootScope::new(...);
///   |   |
///   |   |
///   |   | let d: Rooted<T> = ...;
///   |   |
///   |   |
///   |   | ----- let scope2 = RootScope::new(...);
///   |   |   |
///   |   |   |
///   |   |   | let e: Rooted<T> = ...;
///   |   |   |
///   |   |   |
///   |   | ----- drop `scope2` (unroots `e`)
///   |   |
///   |   |
///   | ----- drop `scope1` (unroots `d`)
///   |
///   |
/// ----- drop Store (unroots `a`)
/// ```
///
/// A `Rooted<T>` can be used successfully as long as it is still rooted so, in
/// the above diagram, `d` is valid inside `scope2` because `scope2` is wholly
/// contained within the scope `d` was rooted within (`scope1`).
///
/// See also the documentation for [`RootScope`][crate::RootScope].
#[repr(transparent)]
pub struct Rooted<T: GcRef> {
    inner: GcRootIndex,
    _phantom: marker::PhantomData<T>,
}

impl<T: GcRef> Clone for Rooted<T> {
    fn clone(&self) -> Self {
        Rooted {
            inner: self.inner,
            _phantom: marker::PhantomData,
        }
    }
}

impl<T: GcRef> Copy for Rooted<T> {}

impl<T: GcRef> Debug for Rooted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("Rooted<{}>", any::type_name::<T>());
        f.debug_struct(&name).field("inner", &self.inner).finish()
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for Rooted<T> {
    fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        let index = self.inner.index.as_lifo().unwrap();
        let entry = store.gc_roots().lifo_roots.get(index)?;
        if entry.generation == self.inner.generation {
            Some(&entry.gc_ref)
        } else {
            None
        }
    }
}

impl<T: GcRef> Deref for Rooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        T::transmute_ref(&self.inner)
    }
}

impl<T: GcRef> Rooted<T> {
    /// Push the given `VMGcRef` onto our LIFO root set.
    ///
    /// `gc_ref` should belong to `store`'s heap; failure to uphold this is
    /// memory safe but will result in general failures down the line such as
    /// panics or incorrect results.
    ///
    /// `gc_ref` should be a GC reference pointing to an instance of the GC type
    /// that `T` represents. Failure to uphold this invariant is memory safe but
    /// will result in general incorrectness such as panics and wrong results.
    pub(crate) fn new(store: &mut AutoAssertNoGc<'_>, gc_ref: VMGcRef) -> Rooted<T> {
        let id = store.id();
        let roots = store.gc_roots_mut();
        let inner = roots.push_lifo_root(id, gc_ref);
        Rooted {
            inner,
            _phantom: marker::PhantomData,
        }
    }

    /// Create a new `Rooted<T>` from a `GcRootIndex`.
    ///
    /// Note that `Rooted::from_gc_root_index(my_rooted.index)` is not
    /// necessarily an identity function, as it allows changing the `T` type
    /// parameter.
    ///
    /// The given index should be a LIFO index of a GC reference pointing to an
    /// instance of the GC type that `T` represents. Failure to uphold this
    /// invariant is memory safe but will result in general incorrectness such
    /// as panics and wrong results.
    pub(crate) fn from_gc_root_index(inner: GcRootIndex) -> Rooted<T> {
        debug_assert!(inner.index.is_lifo());
        Rooted {
            inner,
            _phantom: marker::PhantomData,
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        debug_assert!(self.inner.index.is_lifo());
        self.inner.comes_from_same_store(store)
    }

    /// Create an [`OwnedRooted<T>`][crate::OwnedRooted] holding onto the
    /// same GC object as `self`.
    ///
    /// Returns `None` if `self` is used outside of its scope and has therefore
    /// been unrooted.
    ///
    /// This does not unroot `self`, and `self` remains valid until its
    /// associated scope is exited.
    ///
    /// # Panics
    ///
    /// Panics if this object is not associate with the given store.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let y: OwnedRooted<_> = {
    ///     // Create a nested rooting scope.
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // `x` is only rooted within this nested scope.
    ///     let x: Rooted<_> = ExternRef::new(&mut scope, "hello!")?;
    ///
    ///     // Extend `x`'s rooting past its scope's lifetime by converting it
    ///     // to an `OwnedRooted`.
    ///     x.to_owned_rooted(&mut scope)?
    /// };
    ///
    /// // Now we can still access the reference outside the scope it was
    /// // originally defined within.
    /// let data = y.data(&store)?.expect("should have host data");
    /// let data = data.downcast_ref::<&str>().expect("host data should be str");
    /// assert_eq!(*data, "hello!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_owned_rooted(&self, mut store: impl AsContextMut) -> Result<OwnedRooted<T>> {
        self._to_owned_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _to_owned_rooted(&self, store: &mut StoreOpaque) -> Result<OwnedRooted<T>> {
        let mut store = AutoAssertNoGc::new(store);
        let gc_ref = self.try_clone_gc_ref(&mut store)?;
        Ok(OwnedRooted::new(&mut store, gc_ref))
    }

    /// Are these two `Rooted<T>`s the same GC root?
    ///
    /// Note that this function can return `false` even when `a` and `b` are
    /// rooting the same underlying GC object, but the object was rooted
    /// multiple times (for example in different scopes). Use
    /// [`Rooted::ref_eq`][crate::Rooted::ref_eq] to test whether these are
    /// references to the same underlying GC object or not.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let a = ExternRef::new(&mut store, "hello")?;
    /// let b = a;
    ///
    /// // `a` and `b` are the same GC root.
    /// assert!(Rooted::rooted_eq(a, b));
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // `c` is a different GC root, in a different scope, even though it
    ///     // is rooting the same object.
    ///     let c = a.to_owned_rooted(&mut scope)?.to_rooted(&mut scope);
    ///     assert!(!Rooted::rooted_eq(a, c));
    /// }
    ///
    /// let x = ExternRef::new(&mut store, "goodbye")?;
    ///
    /// // `a` and `x` are different GC roots, rooting different objects.
    /// assert!(!Rooted::rooted_eq(a, x));
    /// # Ok(())
    /// # }
    /// ```
    pub fn rooted_eq(a: Self, b: Self) -> bool {
        a.inner == b.inner
    }

    /// Are these two GC roots referencing the same underlying GC object?
    ///
    /// This function will return `true` even when `a` and `b` are different GC
    /// roots (for example because they were rooted in different scopes) if they
    /// are rooting the same underlying GC object. To only test whether they are
    /// the same GC root, and not whether they are rooting the same GC object,
    /// use [`Rooted::rooted_eq`][crate::Rooted::rooted_eq].
    ///
    /// Returns an error if either `a` or `b` has been unrooted, for example
    /// because the scope it was rooted within has been exited.
    ///
    /// Because this method takes any `impl RootedGcRef<T>` arguments, it can be
    /// used to compare, for example, a `Rooted<T>` and a `OwnedRooted<T>`.
    ///
    /// # Panics
    ///
    /// Panics if either `a` or `b` is not associated with the given `store`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let a = ExternRef::new(&mut store, "hello")?;
    /// let b = a;
    ///
    /// // `a` and `b` are rooting the same object.
    /// assert!(Rooted::ref_eq(&store, &a, &b)?);
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // `c` is a different GC root, in a different scope, but still
    ///     // rooting the same object.
    ///     let c = a.to_owned_rooted(&mut scope)?.to_rooted(&mut scope);
    ///     assert!(!Rooted::ref_eq(&scope, &a, &c)?);
    /// }
    ///
    /// let x = ExternRef::new(&mut store, "goodbye")?;
    ///
    /// // `a` and `x` are rooting different objects.
    /// assert!(!Rooted::ref_eq(&store, &a, &x)?);
    ///
    /// // You can also compare `Rooted<T>`s and `OwnedRooted<T>`s with this
    /// // function.
    /// let d = a.to_owned_rooted(&mut store)?;
    /// assert!(Rooted::ref_eq(&store, &a, &d)?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn ref_eq(
        store: impl AsContext,
        a: &impl RootedGcRef<T>,
        b: &impl RootedGcRef<T>,
    ) -> Result<bool> {
        let store = store.as_context().0;
        Self::_ref_eq(store, a, b)
    }

    pub(crate) fn _ref_eq(
        store: &StoreOpaque,
        a: &impl RootedGcRef<T>,
        b: &impl RootedGcRef<T>,
    ) -> Result<bool> {
        let a = a.try_gc_ref(store)?;
        let b = b.try_gc_ref(store)?;
        Ok(a == b)
    }

    /// Hash this root.
    ///
    /// Note that, similar to `Rooted::rooted_eq`, this only operates on the
    /// root and *not* the underlying GC reference. That means that two
    /// different rootings of the same object will hash to different values
    /// (modulo hash collisions). If this is undesirable, use the
    /// [`ref_hash`][crate::Rooted::ref_hash] method instead.
    pub fn rooted_hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.inner.hash(state);
    }

    /// Hash the underlying rooted object reference.
    ///
    /// Note that, similar to `Rooted::ref_eq`, and operates on the underlying
    /// rooted GC object reference, not the root. That means that two
    /// *different* rootings of the same object will hash to the *same*
    /// value. If this is undesirable, use the
    /// [`rooted_hash`][crate::Rooted::rooted_hash] method instead.
    pub fn ref_hash<H>(&self, store: impl AsContext, state: &mut H) -> Result<()>
    where
        H: Hasher,
    {
        let gc_ref = self.try_gc_ref(store.as_context().0)?;
        gc_ref.hash(state);
        Ok(())
    }

    /// Cast `self` to a `Rooted<U>`.
    ///
    /// It is the caller's responsibility to ensure that `self` is actually a
    /// `U`. Failure to uphold this invariant will be memory safe but will
    /// result in general incorrectness such as panics and wrong results.
    pub(crate) fn unchecked_cast<U: GcRef>(self) -> Rooted<U> {
        Rooted::from_gc_root_index(self.inner)
    }

    /// Common implementation of the `WasmTy::store` trait method for all
    /// `Rooted<T>`s.
    pub(super) fn wasm_ty_store(
        self,
        store: &mut AutoAssertNoGc<'_>,
        ptr: &mut MaybeUninit<ValRaw>,
        val_raw: impl Fn(u32) -> ValRaw,
    ) -> Result<()> {
        let gc_ref = self.inner.try_clone_gc_ref(store)?;

        let raw = match store.optional_gc_store_mut() {
            Some(s) => s.expose_gc_ref_to_wasm(gc_ref),
            None => {
                // NB: do not force the allocation of a GC heap just because the
                // program is using `i31ref`s.
                debug_assert!(gc_ref.is_i31());
                gc_ref.as_raw_non_zero_u32()
            }
        };

        ptr.write(val_raw(raw.get()));
        Ok(())
    }

    /// Common implementation of the `WasmTy::load` trait method for all
    /// `Rooted<T>`s.
    pub(super) fn wasm_ty_load(
        store: &mut AutoAssertNoGc<'_>,
        raw_gc_ref: u32,
        from_cloned_gc_ref: impl Fn(&mut AutoAssertNoGc<'_>, VMGcRef) -> Self,
    ) -> Self {
        debug_assert_ne!(raw_gc_ref, 0);
        let gc_ref = VMGcRef::from_raw_u32(raw_gc_ref).expect("non-null");

        let gc_ref = match store.optional_gc_store_mut() {
            Some(s) => s.clone_gc_ref(&gc_ref),
            None => {
                // NB: do not force the allocation of a GC heap just because the
                // program is using `i31ref`s.
                debug_assert!(gc_ref.is_i31());
                gc_ref.unchecked_copy()
            }
        };

        from_cloned_gc_ref(store, gc_ref)
    }

    /// Common implementation of the `WasmTy::store` trait method for all
    /// `Option<Rooted<T>>`s.
    pub(super) fn wasm_ty_option_store(
        me: Option<Self>,
        store: &mut AutoAssertNoGc<'_>,
        ptr: &mut MaybeUninit<ValRaw>,
        val_raw: impl Fn(u32) -> ValRaw,
    ) -> Result<()> {
        match me {
            Some(me) => me.wasm_ty_store(store, ptr, val_raw),
            None => {
                ptr.write(val_raw(0));
                Ok(())
            }
        }
    }

    /// Common implementation of the `WasmTy::load` trait method for all
    /// `Option<Rooted<T>>`s.
    pub(super) fn wasm_ty_option_load(
        store: &mut AutoAssertNoGc<'_>,
        raw_gc_ref: u32,
        from_cloned_gc_ref: impl Fn(&mut AutoAssertNoGc<'_>, VMGcRef) -> Self,
    ) -> Option<Self> {
        let gc_ref = VMGcRef::from_raw_u32(raw_gc_ref)?;
        let gc_ref = store.clone_gc_ref(&gc_ref);
        Some(from_cloned_gc_ref(store, gc_ref))
    }
}

/// Nested rooting scopes.
///
/// `RootScope` allows the creation or nested rooting scopes for use with
/// [`Rooted<T>`][crate::Rooted]. This allows for fine-grained control over how
/// long a set of [`Rooted<T>`][crate::Rooted]s are strongly held alive, giving
/// gives you the tools necessary to avoid holding onto GC objects longer than
/// necessary. `Rooted<T>`s created within a `RootScope` are automatically
/// unrooted when the `RootScope` is dropped. For more details on
/// [`Rooted<T>`][crate::Rooted] lifetimes and their interaction with rooting
/// scopes, see [`Rooted<T>`][crate::Rooted]'s documentation.
///
/// A `RootScope<C>` wraps a `C: AsContextMut` (that is, anything that
/// represents exclusive access to a [`Store`][crate::Store]) and in turn
/// implements [`AsContext`][crate::AsContext] and
/// [`AsContextMut`][crate::AsContextMut] in terms of its underlying
/// `C`. Therefore, `RootScope<C>` can be used anywhere you would use the
/// underlying `C`, for example in the [`Global::get`][crate::Global::get]
/// method. Any `Rooted<T>`s created by a method that a `RootScope<C>` was
/// passed as context to are tied to the `RootScope<C>`'s scope and
/// automatically unrooted when the scope is dropped.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut store = Store::<()>::default();
///
/// let a: Rooted<_>;
/// let b: Rooted<_>;
/// let c: Rooted<_>;
///
/// // Root `a` in the store's scope. It will be rooted for the duration of the
/// // store's lifetime.
/// a = ExternRef::new(&mut store, 42)?;
///
/// // `a` is rooted, so we can access its data successfully.
/// assert!(a.data(&store).is_ok());
///
/// {
///     let mut scope1 = RootScope::new(&mut store);
///
///     // Root `b` in `scope1`.
///     b = ExternRef::new(&mut scope1, 36)?;
///
///     // Both `a` and `b` are rooted.
///     assert!(a.data(&scope1).is_ok());
///     assert!(b.data(&scope1).is_ok());
///
///     {
///         let mut scope2 = RootScope::new(&mut scope1);
///
///         // Root `c` in `scope2`.
///         c = ExternRef::new(&mut scope2, 36)?;
///
///         // All of `a`, `b`, and `c` are rooted.
///         assert!(a.data(&scope2).is_ok());
///         assert!(b.data(&scope2).is_ok());
///         assert!(c.data(&scope2).is_ok());
///
///         // Drop `scope2`.
///     }
///
///     // Now `a` and `b` are still rooted, but `c` was unrooted when we dropped
///     // `scope2`.
///     assert!(a.data(&scope1).is_ok());
///     assert!(b.data(&scope1).is_ok());
///     assert!(c.data(&scope1).is_err());
///
///     // Drop `scope1`.
/// }
///
/// // And now only `a` is still rooted. Both `b` and `c` were unrooted when we
/// // dropped their respective rooting scopes.
/// assert!(a.data(&store).is_ok());
/// assert!(b.data(&store).is_err());
/// assert!(c.data(&store).is_err());
/// # Ok(())
/// # }
/// ```
pub struct RootScope<C>
where
    C: AsContextMut,
{
    store: C,
    scope: usize,
}

impl<C> Drop for RootScope<C>
where
    C: AsContextMut,
{
    fn drop(&mut self) {
        self.store.as_context_mut().0.exit_gc_lifo_scope(self.scope);
    }
}

impl<C> RootScope<C>
where
    C: AsContextMut,
{
    // NB: we MUST NOT expose a method like
    //
    //     pub fn store(&mut self) -> &mut Store { ... }
    //
    // because callers could do treacherous things like
    //
    //     let scope1 = RootScope::new(&mut store1);
    //     let scope2 = RootScope::new(&mut store2);
    //     std::mem::swap(scope1.store(), scope2.store());
    //
    // and then we would start truncate the store's GC root set's LIFO roots to
    // the wrong lengths.
    //
    // Instead, we just implement `AsContext[Mut]` for `RootScope`.

    /// Construct a new scope for rooting GC objects.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// let mut store = Store::<()>::default();
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // Temporarily root GC objects in this nested rooting scope...
    /// }
    /// ```
    pub fn new(store: C) -> Self {
        let scope = store.as_context().0.gc_roots().enter_lifo_scope();
        RootScope { store, scope }
    }

    fn gc_roots(&mut self) -> &mut RootSet {
        self.store.as_context_mut().0.gc_roots_mut()
    }

    fn lifo_roots(&mut self) -> &mut Vec<LifoRoot> {
        &mut self.gc_roots().lifo_roots
    }

    /// Reserve enough capacity for `additional` GC roots in this scope.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// let mut store = Store::<()>::default();
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // Ensure we have enough storage pre-allocated to root five GC
    ///     // references inside this scope without any underlying reallocation.
    ///     scope.reserve(5);
    ///
    ///     // ...
    /// }
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.lifo_roots().reserve(additional);
    }
}

impl<T> AsContext for RootScope<T>
where
    T: AsContextMut,
{
    type Data = T::Data;

    fn as_context(&self) -> crate::StoreContext<'_, Self::Data> {
        self.store.as_context()
    }
}

impl<T> AsContextMut for RootScope<T>
where
    T: AsContextMut,
{
    fn as_context_mut(&mut self) -> crate::StoreContextMut<'_, Self::Data> {
        self.store.as_context_mut()
    }
}

/// Internal version of `RootScope` that only wraps a `&mut StoreOpaque` rather
/// than a whole `impl AsContextMut<Data = T>`.
pub(crate) struct OpaqueRootScope<S>
where
    S: AsStoreOpaque,
{
    store: S,
    scope: usize,
}

impl<S> Drop for OpaqueRootScope<S>
where
    S: AsStoreOpaque,
{
    fn drop(&mut self) {
        self.store.as_store_opaque().exit_gc_lifo_scope(self.scope);
    }
}

impl<S> Deref for OpaqueRootScope<S>
where
    S: AsStoreOpaque,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

// XXX: Don't use this `DerefMut` implementation to `mem::{swap,replace}` or
// etc... the underlying `StoreOpaque` in a `OpaqueRootScope`! That will result
// in truncating the store's GC root set's LIFO roots to the wrong length.
//
// We don't implement `DerefMut` for `RootScope` for exactly this reason, but
// allow it for `OpaqueRootScope` because it is only Wasmtime-internal and not
// publicly exported.
impl<S> DerefMut for OpaqueRootScope<S>
where
    S: AsStoreOpaque,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl<S> OpaqueRootScope<S>
where
    S: AsStoreOpaque,
{
    pub(crate) fn new(mut store: S) -> Self {
        let scope = store.as_store_opaque().gc_roots().enter_lifo_scope();
        OpaqueRootScope { store, scope }
    }
}

/// A rooted reference to a garbage-collected `T` with automatic lifetime.
///
/// An `OwnedRooted<T>` is a strong handle to a garbage-collected `T`,
/// preventing its referent (and anything else transitively referenced) from
/// being collected by the GC until it is dropped.
///
/// An `OwnedRooted<T>` keeps its rooted GC object alive at least
/// until the `OwnedRooted<T>` itself is dropped. The
/// "de-registration" of the root is automatic and is triggered (in a
/// deferred way) by the drop of this type.
///
/// The primary way to create an `OwnedRooted<T>` is to promote a temporary
/// `Rooted<T>` into an `OwnedRooted<T>` via its
/// [`to_owned_rooted`][crate::Rooted::to_owned_rooted] method.
///
/// `OwnedRooted<T>` dereferences to its underlying `T`, allowing you to call
/// `T`'s methods.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut store = Store::<Option<OwnedRooted<ExternRef>>>::default();
///
/// // Create our `OwnedRooted` in a nested scope to avoid rooting it for
/// // the duration of the store's lifetime.
/// let x = {
///     let mut scope = RootScope::new(&mut store);
///     let x = ExternRef::new(&mut scope, 1234)?;
///     x.to_owned_rooted(&mut scope)?
/// };
///
/// // Place `x` into our store.
/// *store.data_mut() = Some(x);
///
/// // Do a bunch stuff that may or may not access, replace, or take `x`...
/// # Ok(())
/// # }
/// ```
pub struct OwnedRooted<T>
where
    T: GcRef,
{
    inner: GcRootIndex,
    liveness_flag: Arc<()>,
    _phantom: marker::PhantomData<T>,
}

const _: () = {
    use crate::{AnyRef, ExternRef};

    // NB: these match the C API which should also be updated if this changes.
    //
    // The size is really "16 + pointer + alignment", which is either
    // 20 bytes on some 32-bit platforms or 24 bytes on other 32-bit
    // platforms (e.g., riscv32, which adds an extra 4 bytes of
    // padding) and 64-bit platforms.
    assert!(
        mem::size_of::<OwnedRooted<AnyRef>>() >= 16 && mem::size_of::<OwnedRooted<AnyRef>>() <= 24
    );
    assert!(mem::align_of::<OwnedRooted<AnyRef>>() == mem::align_of::<u64>());
    assert!(
        mem::size_of::<OwnedRooted<ExternRef>>() >= 16
            && mem::size_of::<OwnedRooted<ExternRef>>() <= 24
    );
    assert!(mem::align_of::<OwnedRooted<ExternRef>>() == mem::align_of::<u64>());
};

impl<T: GcRef> Debug for OwnedRooted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("OwnedRooted<{}>", any::type_name::<T>());
        f.debug_struct(&name).field("inner", &self.inner).finish()
    }
}

impl<T: GcRef> Deref for OwnedRooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        T::transmute_ref(&self.inner)
    }
}

impl<T: GcRef> Clone for OwnedRooted<T> {
    fn clone(&self) -> Self {
        OwnedRooted {
            inner: self.inner,
            liveness_flag: self.liveness_flag.clone(),
            _phantom: marker::PhantomData,
        }
    }
}

impl<T> OwnedRooted<T>
where
    T: GcRef,
{
    /// Construct a new owned GC root.
    ///
    /// `gc_ref` should belong to `store`'s heap; failure to uphold this is
    /// memory safe but will result in general failures down the line such as
    /// panics or incorrect results.
    ///
    /// `gc_ref` should be a GC reference pointing to an instance of the GC type
    /// that `T` represents. Failure to uphold this invariant is memory safe but
    /// will result in general incorrectness such as panics and wrong results.
    pub(crate) fn new(store: &mut AutoAssertNoGc<'_>, gc_ref: VMGcRef) -> Self {
        // We always have the opportunity to trim and unregister stale
        // owned roots whenever we have a mut borrow to the store. We
        // take the opportunity to do so here to avoid tying growth of
        // the root-set to the GC frequency -- it is much cheaper to
        // eagerly trim these roots. Note that the trimming keeps a
        // "high water mark" that grows exponentially, so we have
        // amortized constant time even though an individual trim
        // takes time linear in the number of roots.
        store.trim_gc_liveness_flags(false);

        let roots = store.gc_roots_mut();
        let id = roots.owned_rooted.alloc(gc_ref);
        let liveness_flag = Arc::new(());
        roots
            .liveness_flags
            .push((Arc::downgrade(&liveness_flag), id));
        OwnedRooted {
            inner: GcRootIndex {
                store_id: store.id(),
                generation: 0,
                index: PackedIndex::new_owned(id),
            },
            liveness_flag,
            _phantom: marker::PhantomData,
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        debug_assert!(self.inner.index.is_owned());
        self.inner.comes_from_same_store(store)
    }

    /// Clone this `OwnedRooted<T>` into a `Rooted<T>`.
    ///
    /// This operation does not consume or unroot this `OwnedRooted<T>`.
    ///
    /// The underlying GC object is re-rooted in the given context's scope. The
    /// resulting `Rooted<T>` is only valid during the given context's
    /// scope. See the [`Rooted<T>`][crate::Rooted] documentation for more
    /// details on rooting scopes.
    ///
    /// This operation does not consume or unroot this `OwnedRooted<T>`.
    ///
    /// # Panics
    ///
    /// Panics if this object is not associated with the given context's store.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let root1: Rooted<_>;
    ///
    /// let owned = {
    ///     let mut scope = RootScope::new(&mut store);
    ///     root1 = ExternRef::new(&mut scope, 1234)?;
    ///     root1.to_owned_rooted(&mut scope)?
    /// };
    ///
    /// // `root1` is no longer accessible because it was unrooted when `scope`
    /// // was dropped.
    /// assert!(root1.data(&store).is_err());
    ///
    /// // But we can re-root `owned` into this scope.
    /// let root2 = owned.to_rooted(&mut store);
    /// assert!(root2.data(&store).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_rooted(&self, mut context: impl AsContextMut) -> Rooted<T> {
        self._to_rooted(context.as_context_mut().0)
    }

    pub(crate) fn _to_rooted(&self, store: &mut StoreOpaque) -> Rooted<T> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        let mut store = AutoAssertNoGc::new(store);
        let gc_ref = self.clone_gc_ref(&mut store).unwrap();
        Rooted::new(&mut store, gc_ref)
    }

    /// Are these two GC roots referencing the same underlying GC object?
    ///
    /// This function will return `true` even when `a` and `b` are different GC
    /// roots (for example because they were rooted in different scopes) if they
    /// are rooting the same underlying GC object.
    ///
    /// Because this method takes any `impl RootedGcRef<T>` arguments, it can be
    /// used to compare, for example, a `Rooted<T>` and an `OwnedRooted<T>`.
    ///
    /// # Panics
    ///
    /// Panics if either `a` or `b` is not associated with the given `store`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn foo() -> Result<()> {
    /// let mut store = Store::<()>::default();
    ///
    /// let a;
    /// let b;
    /// let x;
    ///
    /// {
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     a = ExternRef::new(&mut scope, "hello")?.to_owned_rooted(&mut scope)?;
    ///     b = a.clone();
    ///
    ///     // `a` and `b` are rooting the same object.
    ///     assert!(OwnedRooted::ref_eq(&scope, &a, &b)?);
    ///
    ///     // `c` is a different GC root, is in a different scope, and is a
    ///     // `Rooted<T>` instead of a `OwnedRooted<T>`, but is still rooting
    ///     // the same object.
    ///     let c = a.to_rooted(&mut scope);
    ///     assert!(OwnedRooted::ref_eq(&scope, &a, &c)?);
    ///
    ///     x = ExternRef::new(&mut scope, "goodbye")?.to_owned_rooted(&mut scope)?;
    ///
    ///     // `a` and `x` are rooting different objects.
    ///     assert!(!OwnedRooted::ref_eq(&scope, &a, &x)?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn ref_eq(
        store: impl AsContext,
        a: &impl RootedGcRef<T>,
        b: &impl RootedGcRef<T>,
    ) -> Result<bool> {
        Rooted::ref_eq(store, a, b)
    }

    /// Hash this root.
    ///
    /// Note that, similar to `Rooted::rooted_eq`, this only operates on the
    /// root and *not* the underlying GC reference. That means that two
    /// different rootings of the same object will hash to different values
    /// (modulo hash collisions). If this is undesirable, use the
    /// [`ref_hash`][crate::OwnedRooted::ref_hash] method instead.
    pub fn rooted_hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.inner.hash(state);
    }

    /// Hash the underlying rooted object reference.
    ///
    /// Note that, similar to `Rooted::ref_eq`, and operates on the underlying
    /// rooted GC object reference, not the root. That means that two
    /// *different* rootings of the same object will hash to the *same*
    /// value. If this is undesirable, use the
    /// [`rooted_hash`][crate::Rooted::rooted_hash] method instead.
    pub fn ref_hash<H>(&self, store: impl AsContext, state: &mut H)
    where
        H: Hasher,
    {
        let gc_ref = self
            .get_gc_ref(store.as_context().0)
            .expect("OwnedRooted's get_gc_ref is infallible");
        gc_ref.hash(state);
    }

    /// Cast `self` to an `OwnedRooted<U>`.
    ///
    /// It is the caller's responsibility to ensure that `self` is actually a
    /// `U`. Failure to uphold this invariant will be memory safe but will
    /// result in general incorrectness such as panics and wrong results.
    pub(crate) fn unchecked_cast<U: GcRef>(self) -> OwnedRooted<U> {
        OwnedRooted {
            inner: self.inner,
            liveness_flag: self.liveness_flag,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Common implementation of the `WasmTy::store` trait method for all
    /// `OwnedRooted<T>`s.
    pub(super) fn wasm_ty_store(
        self,
        store: &mut AutoAssertNoGc<'_>,
        ptr: &mut MaybeUninit<ValRaw>,
        val_raw: impl Fn(u32) -> ValRaw,
    ) -> Result<()> {
        let gc_ref = self.try_clone_gc_ref(store)?;

        let raw = match store.optional_gc_store_mut() {
            Some(s) => s.expose_gc_ref_to_wasm(gc_ref),
            None => {
                debug_assert!(gc_ref.is_i31());
                gc_ref.as_raw_non_zero_u32()
            }
        };

        ptr.write(val_raw(raw.get()));
        Ok(())
    }

    /// Common implementation of the `WasmTy::load` trait method for all
    /// `OwnedRooted<T>`s.
    pub(super) fn wasm_ty_load(
        store: &mut AutoAssertNoGc<'_>,
        raw_gc_ref: u32,
        from_cloned_gc_ref: impl Fn(&mut AutoAssertNoGc<'_>, VMGcRef) -> Rooted<T>,
    ) -> Self {
        debug_assert_ne!(raw_gc_ref, 0);
        let gc_ref = VMGcRef::from_raw_u32(raw_gc_ref).expect("non-null");
        let gc_ref = store.clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = from_cloned_gc_ref(store, gc_ref);
            rooted._to_owned_rooted(store).expect("rooted is in scope")
        })
    }

    /// Common implementation of the `WasmTy::store` trait method for all
    /// `Option<OwnedRooted<T>>`s.
    pub(super) fn wasm_ty_option_store(
        me: Option<Self>,
        store: &mut AutoAssertNoGc<'_>,
        ptr: &mut MaybeUninit<ValRaw>,
        val_raw: impl Fn(u32) -> ValRaw,
    ) -> Result<()> {
        match me {
            Some(me) => me.wasm_ty_store(store, ptr, val_raw),
            None => {
                ptr.write(val_raw(0));
                Ok(())
            }
        }
    }

    /// Common implementation of the `WasmTy::load` trait method for all
    /// `Option<OwnedRooted<T>>`s.
    pub(super) fn wasm_ty_option_load(
        store: &mut AutoAssertNoGc<'_>,
        raw_gc_ref: u32,
        from_cloned_gc_ref: impl Fn(&mut AutoAssertNoGc<'_>, VMGcRef) -> Rooted<T>,
    ) -> Option<Self> {
        let gc_ref = VMGcRef::from_raw_u32(raw_gc_ref)?;
        let gc_ref = store.clone_gc_ref(&gc_ref);
        RootSet::with_lifo_scope(store, |store| {
            let rooted = from_cloned_gc_ref(store, gc_ref);
            Some(rooted._to_owned_rooted(store).expect("rooted is in scope"))
        })
    }

    #[doc(hidden)]
    pub fn into_parts_for_c_api(self) -> (NonZeroU64, u32, u32, *const ()) {
        (
            self.inner.store_id.as_raw(),
            self.inner.generation,
            self.inner.index.0,
            Arc::into_raw(self.liveness_flag),
        )
    }

    #[doc(hidden)]
    pub unsafe fn from_borrowed_raw_parts_for_c_api(
        a: NonZeroU64,
        b: u32,
        c: u32,
        d: *const (),
    ) -> OwnedRooted<T> {
        // We are given a *borrow* of the Arc. This is a little
        // sketchy because `Arc::from_raw()` takes *ownership* of the
        // passed-in pointer, so we need to clone then forget that
        // original.
        let liveness_flag = {
            let original = unsafe { Arc::from_raw(d) };
            let clone = original.clone();
            core::mem::forget(original);
            clone
        };
        OwnedRooted {
            inner: GcRootIndex {
                store_id: StoreId::from_raw(a),
                generation: b,
                index: PackedIndex(c),
            },
            liveness_flag,
            _phantom: marker::PhantomData,
        }
    }

    #[doc(hidden)]
    pub unsafe fn from_owned_raw_parts_for_c_api(
        a: NonZeroU64,
        b: u32,
        c: u32,
        d: *const (),
    ) -> OwnedRooted<T> {
        let liveness_flag = unsafe { Arc::from_raw(d) };
        OwnedRooted {
            inner: GcRootIndex {
                store_id: StoreId::from_raw(a),
                generation: b,
                index: PackedIndex(c),
            },
            liveness_flag,
            _phantom: marker::PhantomData,
        }
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for OwnedRooted<T> {
    fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );

        let id = self.inner.index.as_owned().unwrap();
        store.gc_roots().owned_rooted.get(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::ExternRef;

    use super::*;

    #[test]
    fn sizes() {
        // Try to keep tabs on the size of these things. Don't want them growing
        // unintentionally.
        assert_eq!(std::mem::size_of::<Rooted<ExternRef>>(), 16);
        assert!(std::mem::size_of::<OwnedRooted<ExternRef>>() <= 24);
    }
}
