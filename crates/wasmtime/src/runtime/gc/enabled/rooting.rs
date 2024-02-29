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
//! For example, goals (3) and (4) are in conflict when we think about how to
//! support (2). Ideally, for ergonomics, a root would automatically unroot
//! itself when dropped. But in the general case that requires holding a
//! reference to the store's root set, and that root set needs to be held
//! simultaneously by all GC roots, and they each need to mutate the set to
//! unroot themselves. That implies `Rc<RefCell<...>>` or `Arc<Mutex<...>>`! The
//! former makes the store and GC root types not `Send` and not `Sync`. The
//! latter imposes synchronization and locking overhead. So we instead make GC
//! roots indirect and require passing in a store context explicitly to unroot
//! in the general case. This trades worse ergonomics for better performance and
//! support for moving GC.
//!
//! ## Two Flavors of Rooting API
//!
//! Okay, with that out of the way, this module provides two flavors of rooting
//! API. One for the common, scoped lifetime case, and another for the rare case
//! where we really need a GC root with an arbitrary, non-LIFO/non-scoped
//! lifetime:
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
//! 2. `ManuallyRooted<T>`: This is the fully general rooting API used for
//!    holding onto non-LIFO GC roots with arbitrary lifetimes. However, users
//!    must manually unroot them. Failure to manually unroot a
//!    `ManuallyRooted<T>` before it is dropped will result in the GC object
//!    (and everything it transitively references) leaking for the duration of
//!    the `Store`'s lifetime.
//!
//!    This type is roughly similar to SpiderMonkey's [`PersistentRooted<T>`],
//!    although they avoid the manual-unrooting with internal mutation and
//!    shared references. (Our constraints mean we can't do those things, as
//!    mentioned explained above.)
//!
//!    [`PersistentRooted<T>`]: http://devdoc.net/web/developer.mozilla.org/en-US/docs/Mozilla/Projects/SpiderMonkey/JSAPI_reference/JS::PersistentRooted.html
//!
//! At the end of the day, both `Rooted<T>` and `ManuallyRooted<T>` are just
//! tagged indices into the store's `RootSet`. This indirection allows working
//! with Rust's borrowing discipline (we use `&mut Store` to represent mutable
//! access to the GC heap) while still allowing rooted references to be moved
//! around without tying up the whole store in borrows. Additionally, and
//! crucially, this indirection allows us to update the *actual* GC pointers in
//! the `RootSet` and support moving GCs (again, as mentioned above).
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

use crate::{
    store::{AutoAssertNoGc, StoreId, StoreOpaque},
    AsContext, AsContextMut, GcRef, Result, RootedGcRef,
};
use anyhow::anyhow;
use std::{fmt::Debug, hash::Hash, ops::Deref};
use wasmtime_runtime::{VMExternRef, VMGcRef};
use wasmtime_slab::{Id as SlabId, Slab};

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
        #[allow(private_interfaces)]
        fn transmute_ref(index: &GcRootIndex) -> &Self {
            // This is valid and safe because all implementers must be newtypes
            // over `GcRootIndex`, which is part of the contract for
            // implementing this trait.
            unsafe { std::mem::transmute(index) }
        }
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
        #[allow(private_interfaces)]
        fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef>;

        /// Like `get_gc_ref` but for mutable references.
        #[allow(private_interfaces)]
        fn get_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Option<&'a mut VMGcRef>;

        /// Same as `get_gc_ref` but returns an error instead of `None` for
        /// objects that have been unrooted.
        #[allow(private_interfaces)]
        fn try_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
            self.get_gc_ref(store).ok_or_else(|| {
                anyhow!("attempted to use a garbage-collected object that has been unrooted")
            })
        }

        /// Like `try_gc_ref` but for mutable references.
        #[allow(private_interfaces)]
        fn try_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Result<&'a mut VMGcRef> {
            self.get_gc_ref_mut(store).ok_or_else(|| {
                anyhow!("attempted to use a garbage-collected object that has been unrooted")
            })
        }
    }
}
pub(crate) use sealed::*;

/// The index of a GC root inside a particular store's GC root set.
///
/// Can be either a LIFO- or manually-rooted object, depending on the
/// `PackedIndex`.
///
/// Every `T` such that `T: GcRef` must be a newtype over this `GcRootIndex`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GcRootIndex {
    store_id: StoreId,
    generation: u32,
    index: PackedIndex,
}

impl GcRootIndex {
    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        self.store_id == store.id()
    }

    /// Same as `RootedGcRefImpl::get_gc_ref` but not associated with any
    /// particular `T: GcRef`.
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
        } else if let Some(id) = self.index.as_manual() {
            let gc_ref = store.gc_roots().manually_rooted.get(id);
            debug_assert!(gc_ref.is_some());
            gc_ref
        } else {
            unreachable!()
        }
    }

    /// Same as `get_gc_ref` but returns an error instead of `None` if the GC
    /// reference has been unrooted.
    pub(crate) fn try_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
        self.get_gc_ref(store).ok_or_else(|| {
            anyhow!("attempted to use a garbage-collected object that has been unrooted")
        })
    }

    /// Same as `RootedGcRefImpl::get_gc_ref_mut` but not associated with any
    /// particular `T: GcRef`.
    pub(crate) fn get_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Option<&'a mut VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        if let Some(index) = self.index.as_lifo() {
            let entry = store.gc_roots_mut().lifo_roots.get_mut(index)?;
            if entry.generation == self.generation {
                Some(&mut entry.gc_ref)
            } else {
                None
            }
        } else if let Some(id) = self.index.as_manual() {
            let gc_ref = store.gc_roots_mut().manually_rooted.get_mut(id);
            debug_assert!(gc_ref.is_some());
            gc_ref
        } else {
            unreachable!()
        }
    }

    /// Same as `get_gc_ref_mut` but returns an error instead of `None` if the
    /// GC reference has been unrooted.
    pub(crate) fn try_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Result<&'a mut VMGcRef> {
        self.get_gc_ref_mut(store).ok_or_else(|| {
            anyhow!("attempted to use a garbage-collected object that has been unrooted")
        })
    }
}

/// This is a bit-packed version of
///
/// ```ignore
/// enema {
///     Lifo(usize),
///     Manual(SlabId),
/// }
/// ```
///
/// where the high bit is the discriminant and the lower 31 bits are the
/// payload.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PackedIndex(u32);

impl Debug for PackedIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(index) = self.as_lifo() {
            f.debug_tuple("PackedIndex::Lifo").field(&index).finish()
        } else if let Some(id) = self.as_manual() {
            f.debug_tuple("PackedIndex::Manual").field(&id).finish()
        } else {
            unreachable!()
        }
    }
}

impl PackedIndex {
    const DISCRIMINANT_MASK: u32 = 0b1 << 31;
    const LIFO_DISCRIMINANT: u32 = 0b0 << 31;
    const MANUAL_DISCRIMINANT: u32 = 0b1 << 31;
    const PAYLOAD_MASK: u32 = !Self::DISCRIMINANT_MASK;

    fn new_lifo(index: usize) -> PackedIndex {
        let index32 = u32::try_from(index).unwrap();
        assert_eq!(index32 & Self::DISCRIMINANT_MASK, 0);
        let packed = PackedIndex(Self::LIFO_DISCRIMINANT | index32);
        debug_assert!(packed.is_lifo());
        debug_assert_eq!(packed.as_lifo(), Some(index));
        debug_assert!(!packed.is_manual());
        debug_assert!(packed.as_manual().is_none());
        packed
    }

    fn new_manual(id: SlabId) -> PackedIndex {
        let raw = id.into_raw();
        assert_eq!(raw & Self::DISCRIMINANT_MASK, 0);
        let packed = PackedIndex(Self::MANUAL_DISCRIMINANT | raw);
        debug_assert!(packed.is_manual());
        debug_assert_eq!(packed.as_manual(), Some(id));
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

    fn is_manual(&self) -> bool {
        self.discriminant() == Self::MANUAL_DISCRIMINANT
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

    fn as_manual(&self) -> Option<SlabId> {
        if self.is_manual() {
            Some(SlabId::from_raw(self.payload()))
        } else {
            None
        }
    }
}

/// The set of all GC roots in a single store/heap.
#[derive(Debug, Default)]
pub(crate) struct RootSet {
    /// GC roots with arbitrary lifetime that are manually rooted and unrooted,
    /// for use with `ManuallyRooted<T>`.
    manually_rooted: Slab<VMGcRef>,

    /// Strictly LIFO-ordered GC roots, for use with `RootScope` and
    /// `Rooted<T>`.
    lifo_roots: Vec<LifoRoot>,

    /// Generation counter for entries to prevent ABA bugs with `RootScope` and
    /// `Rooted<T>`.
    lifo_generation: u32,
}

impl Drop for RootSet {
    fn drop(&mut self) {
        // Drop our `ExternRef` roots. In the future this will be a series of
        // calls to `wasmtime_runtime::GcRuntime::on_unroot` trait method hook
        // or something like that. However, this is all unnecessary for
        // non-reference-counting collectors.

        self.exit_lifo_scope(0);

        for (_id, gc_ref) in self.manually_rooted.drain() {
            // (Inlined copy of `self.unroot_gc_ref(gc_ref)` to avoid borrowing
            // `self` while `self.manually_rooted` is already borrowed).
            //
            // Safety: our mutable access to the root set means that no one else
            // should have concurrent access to the `VMExternRef`, so
            // decrementing the reference count here is safe.
            unsafe {
                assert!(VMGcRef::ONLY_EXTERN_REF_IMPLEMENTED_YET);
                let _ = VMExternRef::from_gc_ref(gc_ref);
            }
        }
    }
}

#[derive(Debug)]
struct LifoRoot {
    generation: u32,
    gc_ref: VMGcRef,
}

impl RootSet {
    /// Enter a LIFO rooting scope.
    ///
    /// Returns an integer that should be passed unmodified to `exit_lifo_scope`
    /// when the scope is finished.
    ///
    /// Calls to `{enter,exit}_lifo_scope` must happen in a strict LIFO order.
    pub(crate) fn enter_lifo_scope(&self) -> usize {
        let len = self.lifo_roots.len();
        log::debug!("Entering GC root set LIFO scope: {len}");
        len
    }

    /// Exit a LIFO rooting scope.
    ///
    /// The `scope` argument must be the result of the corresponding
    /// `enter_lifo_scope` call.
    ///
    /// Calls to `{enter,exit}_lifo_scope` must happen in a strict LIFO order.
    pub(crate) fn exit_lifo_scope(&mut self, scope: usize) {
        log::debug!("Exiting GC root set LIFO scope: {scope}");
        debug_assert!(self.lifo_roots.len() >= scope);

        // In the case where we have a tracing GC, this should really be:
        //
        //     self.lifo_roots.truncate(scope);
        //
        // In the meantime, without deferred reference-counting collector for
        // `externref`, we need to drop these references. In the future this
        // will be a `wasmtime_runtime::GcRuntime::on_unroot` trait method hook
        // or something like that, but where we can skip this whole loop and
        // just do the above truncate when the collector doesn't need it.
        for root in self.lifo_roots.drain(scope..) {
            // (Inlined copy of `self.unroot_gc_ref(root.gc_ref)` to avoid
            // borrowing `self` while `self.manually_rooted` is already
            // borrowed).
            //
            // Safety: our mutable access to the root set means that no one else
            // should have concurrent access to the `VMExternRef`, so
            // decrementing the reference count here is safe.
            unsafe {
                assert!(VMGcRef::ONLY_EXTERN_REF_IMPLEMENTED_YET);
                let _ = VMExternRef::from_gc_ref(root.gc_ref);
            }
        }

        self.lifo_generation += 1;
    }

    pub(crate) fn with_lifo_scope<T>(
        store: &mut StoreOpaque,
        f: impl FnOnce(&mut StoreOpaque) -> T,
    ) -> T {
        let scope = store.gc_roots().enter_lifo_scope();
        let ret = f(store);
        store.gc_roots_mut().exit_lifo_scope(scope);
        ret
    }

    fn unroot_gc_ref(&mut self, gc_ref: VMGcRef) {
        // Safety: our mutable access to the root set means that no one else
        // should have concurrent access to the `VMExternRef`, so
        // decrementing the reference count here is safe.
        unsafe {
            assert!(VMGcRef::ONLY_EXTERN_REF_IMPLEMENTED_YET);
            let _ = VMExternRef::from_gc_ref(gc_ref);
        }
    }
}

/// Clone a GC raw root.
///
/// In the future, this will be a method on a `wasmtime_runtime::GcRuntime`
/// trait or something like that.
///
/// # Safety
///
/// The given `gc_ref` must belong to the given store.
unsafe fn clone_root(_store: &mut StoreOpaque, gc_ref: VMGcRef) -> VMGcRef {
    // Safety: `externref`s are the only GC objects at this moment.
    assert!(VMGcRef::ONLY_EXTERN_REF_IMPLEMENTED_YET);
    unsafe {
        let externref = VMExternRef::clone_from_gc_ref(gc_ref);
        externref.into_gc_ref()
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
/// let hello: Rooted<ExternRef> = ExternRef::new(&mut store, "hello");
///
/// // Because `Rooted<T>` derefs to `T`, we can call `T` methods on a
/// // `Rooted<T>`. For example, we can call the `ExternRef::data` method when we
/// // have a `Rooted<ExternRef>`.
/// let data = hello
///     .data(&store)?
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
/// # Differences Between `Rooted<T>` and `ManuallyRooted<T>`
///
/// While `Rooted<T>` is automatically unrooted when its scope is exited, this
/// means that `Rooted<T>` is only valid for strictly last-in-first-out (LIFO,
/// aka stack order) lifetimes. This is in contrast to
/// [`ManuallyRooted<T>`][crate::ManuallyRooted], which supports rooting GC
/// objects for arbitrary lifetimes, but requires manual unrooting.
///
/// | Type                                         | Supported Lifetimes         | Unrooting |
/// |----------------------------------------------|-----------------------------|-----------|
/// | [`Rooted<T>`][crate::Rooted]                 | Strictly LIFO / stack order | Automatic |
/// | [`ManuallyRooted<T>`][crate::ManuallyRooted] | Arbitrary                   | Manual    |
///
/// `Rooted<T>` should suffice for most use cases, and provides better
/// ergonomics, but `ManuallyRooted<T>` exists as a fully-general escape hatch.
///
/// # Scopes
///
/// Wasmtime automatically creates two kinds of scopes:
///
/// 1. A [`Store`][crate::Store] is the outermost rooting scope. Creating a
///    `Root<T>` directly inside a `Store` permanently roots the underlying
///    object, similar to dropping a
///    [`ManuallyRooted<T>`][crate::ManuallyRooted] without unrooting it.
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
    _phantom: std::marker::PhantomData<T>,
}

impl<T: GcRef> Clone for Rooted<T> {
    fn clone(&self) -> Self {
        Rooted {
            inner: self.inner,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: GcRef> Copy for Rooted<T> {}

impl<T: GcRef> Debug for Rooted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = format!("Rooted<{}>", std::any::type_name::<T>());
        f.debug_struct(&name).field("inner", &self.inner).finish()
    }
}

impl<T: GcRef> PartialEq for Rooted<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: GcRef> Eq for Rooted<T> {}

impl<T: GcRef> Hash for Rooted<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }

    fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        // Safety: `Rooted<T>` is a `repr(transparent)` newtype over
        // `GcRootIndex`.
        let data: &[GcRootIndex] = unsafe { std::mem::transmute(data) };
        GcRootIndex::hash_slice(data, state);
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for Rooted<T> {
    #[allow(private_interfaces)]
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

    #[allow(private_interfaces)]
    fn get_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Option<&'a mut VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        let index = self.inner.index.as_lifo().unwrap();
        let entry = store.gc_roots_mut().lifo_roots.get_mut(index)?;
        if entry.generation == self.inner.generation {
            Some(&mut entry.gc_ref)
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
    /// # Safety
    ///
    /// `gc_ref` must be a valid GC reference pointing to an instance of the GC
    /// type that `T` represents.
    ///
    /// `gc_ref` must belong to `store`'s heap.
    pub(crate) unsafe fn new(store: &mut StoreOpaque, gc_ref: VMGcRef) -> Rooted<T> {
        let roots = store.gc_roots_mut();
        let generation = roots.lifo_generation;
        let index = roots.lifo_roots.len();
        let index = PackedIndex::new_lifo(index);

        roots.lifo_roots.push(LifoRoot { generation, gc_ref });

        Rooted {
            inner: GcRootIndex {
                store_id: store.id(),
                generation,
                index,
            },
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        debug_assert!(self.inner.index.is_lifo());
        self.inner.comes_from_same_store(store)
    }

    /// Create a [`ManuallyRooted<T>`][crate::ManuallyRooted] holding onto the
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
    /// let y: ManuallyRooted<_> = {
    ///     // Create a nested rooting scope.
    ///     let mut scope = RootScope::new(&mut store);
    ///
    ///     // `x` is only rooted within this nested scope.
    ///     let x: Rooted<_> = ExternRef::new(&mut scope, "hello!");
    ///
    ///     // Extend `x`'s rooting past its scope's lifetime by converting it
    ///     // to a `ManuallyRooted`.
    ///     x.to_manually_rooted(&mut scope)?
    /// };
    ///
    /// // Now we can still access the reference outside the scope it was
    /// // originally defined within.
    /// let data = y.data(&store)?;
    /// let data = data.downcast_ref::<&str>().unwrap();
    /// assert_eq!(*data, "hello!");
    ///
    /// // But we have to manually unroot `y`.
    /// y.unroot(&mut store);
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_manually_rooted(&self, mut store: impl AsContextMut) -> Result<ManuallyRooted<T>> {
        self._to_manually_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _to_manually_rooted(&self, store: &mut StoreOpaque) -> Result<ManuallyRooted<T>> {
        let mut store = AutoAssertNoGc::new(store);
        let gc_ref = *self.try_gc_ref(&store)?;

        // Safety: `gc_ref` belongs to the store, asserted by `try_gc_ref`.
        let gc_ref = unsafe { clone_root(&mut store, gc_ref) };

        // Safety: `gc_ref` is a `T`, since we got it from `self`.
        Ok(unsafe { ManuallyRooted::new(&mut store, gc_ref) })
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
    /// let a = ExternRef::new(&mut store, "hello");
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
    ///     let c = a.to_manually_rooted(&mut scope)?.into_rooted(&mut scope);
    ///     assert!(!Rooted::rooted_eq(a, c));
    /// }
    ///
    /// let x = ExternRef::new(&mut store, "goodbye");
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
    /// used to compare, for example, a `Rooted<T>` and a `ManuallyRooted<T>`.
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
    /// let a = ExternRef::new(&mut store, "hello");
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
    ///     let c = a.to_manually_rooted(&mut scope)?.into_rooted(&mut scope);
    ///     assert!(!Rooted::ref_eq(&scope, &a, &c)?);
    /// }
    ///
    /// let x = ExternRef::new(&mut store, "goodbye");
    ///
    /// // `a` and `x` are rooting different objects.
    /// assert!(!Rooted::ref_eq(&store, &a, &x)?);
    ///
    /// // You can also compare `Rooted<T>`s and `ManuallyRooted<T>`s with this
    /// // function.
    /// let d = a.to_manually_rooted(&mut store)?;
    /// assert!(Rooted::ref_eq(&store, &a, &d)?);
    ///
    /// d.unroot(&mut store);
    /// # Ok(())
    /// # }
    /// ```
    pub fn ref_eq(
        store: impl AsContext,
        a: &impl RootedGcRef<T>,
        b: &impl RootedGcRef<T>,
    ) -> Result<bool> {
        let store = store.as_context().0;
        let a = a.try_gc_ref(store)?;
        let b = b.try_gc_ref(store)?;
        Ok(a == b)
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
/// a = ExternRef::new(&mut store, 42);
///
/// // `a` is rooted, so we can access its data successfully.
/// assert!(a.data(&store).is_ok());
///
/// {
///     let mut scope1 = RootScope::new(&mut store);
///
///     // Root `b` in `scope1`.
///     b = ExternRef::new(&mut scope1, 36);
///
///     // Both `a` and `b` are rooted.
///     assert!(a.data(&scope1).is_ok());
///     assert!(b.data(&scope1).is_ok());
///
///     {
///         let mut scope2 = RootScope::new(&mut scope1);
///
///         // Root `c` in `scope2`.
///         c = ExternRef::new(&mut scope2, 36);
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
    initial_lifo_len: usize,
}

impl<C> Drop for RootScope<C>
where
    C: AsContextMut,
{
    fn drop(&mut self) {
        let len = self.initial_lifo_len;
        self.gc_roots().exit_lifo_scope(len);
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
        let initial_lifo_len = store.as_context().0.gc_roots().enter_lifo_scope();
        RootScope {
            store,
            initial_lifo_len,
        }
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

/// A rooted reference to a garbage-collected `T` with arbitrary lifetime.
///
/// A `ManuallyRooted<T>` is a strong handle to a garbage-collected `T`,
/// preventing its referent (and anything else transitively referenced) from
/// being collected by the GC until [`unroot`][crate::ManuallyRooted::unroot] is
/// explicitly called.
///
/// The primary way to create a `ManuallyRooted<T>` is to promote a temporary
/// `Rooted<T>` into a `ManuallyRooted<T>` via its
/// [`to_manually_rooted`][crate::Rooted::to_manually_rooted] method.
///
/// `ManuallyRooted<T>` dereferences to its underlying `T`, allowing you to call
/// `T`'s methods.
///
/// # Example
///
/// ```
/// # use wasmtime::*;
/// # fn _foo() -> Result<()> {
/// let mut store = Store::<Option<ManuallyRooted<ExternRef>>>::default();
///
/// // Create our `ManuallyRooted` in a nested scope to avoid rooting it for
/// // the duration of the store's lifetime.
/// let x = {
///     let mut scope = RootScope::new(&mut store);
///     let x = ExternRef::new(&mut scope, 1234);
///     x.to_manually_rooted(&mut scope)?
/// };
///
/// // Place `x` into our store.
/// *store.data_mut() = Some(x);
///
/// // Do a bunch stuff that may or may not access, replace, or take `x`...
///
/// // At any time, in any arbitrary scope, we can remove `x` from the store
/// // and unroot it:
/// if let Some(x) = store.data_mut().take() {
///     x.unroot(&mut store);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Differences Between `ManuallyRooted<T>` and `Rooted<T>`
///
/// While `ManuallyRooted<T>` can have arbitrary lifetimes, it requires manual
/// unrooting. This is in contrast to [`Rooted<T>`][crate::Rooted] which is
/// restricted to strictly last-in-first-out (LIFO, aka stack order) lifetimes,
/// but comes with automatic unrooting.
///
/// | Type                                         | Supported Lifetimes         | Unrooting |
/// |----------------------------------------------|-----------------------------|-----------|
/// | [`Rooted<T>`][crate::Rooted]                 | Strictly LIFO / stack order | Automatic |
/// | [`ManuallyRooted<T>`][crate::ManuallyRooted] | Arbitrary                   | Manual    |
///
/// `Rooted<T>` should suffice for most use cases, and provides better
/// ergonomics, but `ManuallyRooted<T>` exists as a fully-general escape hatch.
///
/// # Manual Unrooting
///
/// Failure to explicitly call [`unroot`][crate::ManuallyRooted::unroot] (or
/// another method that consumes `self` and unroots the reference, such as
/// [`into_rooted`][crate::ManuallyRooted::into_rooted]) will leak the
/// underlying GC object, preventing it from being garbage collected until its
/// owning [`Store`][crate::Store] is dropped. That means all of the following
/// will result in permanently rooting the underlying GC object:
///
/// * Implicitly dropping a `ManuallyRooted<T>`:
///
///   ```no_run
///   # use wasmtime::*;
///   # let get_manually_rooted = || -> ManuallyRooted<ExternRef> { todo!() };
///   {
///       let perma_root: ManuallyRooted<_> = get_manually_rooted();
///
///       // `perma_root` is implicitly dropped at the end of its scope,
///       // permanently rooting/leaking its referent.
///   }
///   ```
///
/// * Explicitly dropping a `ManuallyRooted<T>`: `drop(my_manually_rooted)`.
///
/// * Forgetting a `ManuallyRooted<T>`: `std::mem::forget(my_manually_rooted)`.
///
/// * Inserting a `ManuallyRooted<T>` into a `std::sync::Arc` or `std::rc::Rc`
///   cycle.
///
/// * Etc...
///
/// Wasmtime does *not* assert that a `ManuallyRooted<T>` is unrooted on `Drop`,
/// or otherwise raise a panic, log a warning, or etc... on failure to manually
/// unroot. Sometimes leaking is intentional and desirable, particularly when
/// dealing with short-lived [`Store`][crate::Store]s where unrooting would just
/// be busy work since the whole store is about to be dropped.
pub struct ManuallyRooted<T>
where
    T: GcRef,
{
    inner: GcRootIndex,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: GcRef> Debug for ManuallyRooted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = format!("ManuallyRooted<{}>", std::any::type_name::<T>());
        f.debug_struct(&name).field("inner", &self.inner).finish()
    }
}

impl<T: GcRef> Deref for ManuallyRooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        T::transmute_ref(&self.inner)
    }
}

impl<T> ManuallyRooted<T>
where
    T: GcRef,
{
    /// Construct a new manually-rooted GC root.
    ///
    /// # Safety
    ///
    /// `gc_ref` must be a valid GC reference pointing to an instance of the GC
    /// type that `T` represents.
    ///
    /// `gc_ref` must belong to `store`'s heap.
    pub(crate) unsafe fn new(store: &mut StoreOpaque, gc_ref: VMGcRef) -> Self {
        let id = store.gc_roots_mut().manually_rooted.alloc(gc_ref);
        ManuallyRooted {
            inner: GcRootIndex {
                store_id: store.id(),
                generation: 0,
                index: PackedIndex::new_manual(id),
            },
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        debug_assert!(self.inner.index.is_manual());
        self.inner.comes_from_same_store(store)
    }

    /// Clone this `ManuallyRooted`.
    ///
    /// Does not consume or unroot `self`: both `self` and the new
    /// `ManuallyRooted` return value will need to be manually unrooted.
    ///
    /// # Panics
    ///
    /// Panics if `self` is not associated with the given `store`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn _foo() -> Result<()> {
    /// let mut store = Store::<Vec<ManuallyRooted<ExternRef>>>::default();
    ///
    /// // Create our `ManuallyRooted` in a nested scope to avoid rooting it for
    /// // the duration of the store's lifetime.
    /// let x = {
    ///     let mut scope = RootScope::new(&mut store);
    ///     let x = ExternRef::new(&mut scope, 1234);
    ///     x.to_manually_rooted(&mut scope)?
    /// };
    ///
    /// // Push five clones of `x` into our store.
    /// for _ in 0..5 {
    ///     let x_clone = x.clone(&mut store);
    ///     store.data_mut().push(x_clone);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn clone(&self, mut store: impl AsContextMut) -> Self {
        self._clone(store.as_context_mut().0)
    }

    pub(crate) fn _clone(&self, store: &mut StoreOpaque) -> Self {
        let gc_ref = *self
            .get_gc_ref(store)
            .expect("ManuallyRooted always has a gc ref");
        // Safety: `gc_ref` belongs to this store, asserted by `get_gc_ref`.
        let gc_ref = unsafe { clone_root(store, gc_ref) };
        unsafe { Self::new(store, gc_ref) }
    }

    /// Unroot this GC object.
    ///
    /// Failure to call this method will result in the GC object, and anything
    /// it transitively references, being kept alive (aka "leaking") for the
    /// entirety of the store's lifetime.
    ///
    /// See the type-level docs for example usage.
    pub fn unroot(self, mut store: impl AsContextMut) {
        self._unroot(store.as_context_mut().0)
    }

    pub(crate) fn _unroot(self, store: &mut StoreOpaque) {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );

        let gc_ref = *self.get_gc_ref(store).unwrap();

        let id = self.inner.index.as_manual().unwrap();
        let roots = store.gc_roots_mut();
        roots.manually_rooted.dealloc(id);

        // Drop the `ExternRef`. In the future this will be a
        // `wasmtime_runtime::GcRuntime::on_unroot` trait method hook or
        // something like that.
        roots.unroot_gc_ref(gc_ref);
    }

    /// Clone this `ManuallyRooted<T>` into a `Rooted<T>`.
    ///
    /// This operation does not consume or unroot this `ManuallyRooted<T>`.
    ///
    /// The underlying GC object is re-rooted in the given context's scope. The
    /// resulting `Rooted<T>` is only valid during the given context's
    /// scope. See the [`Rooted<T>`][crate::Rooted] documentation for more
    /// details on rooting scopes.
    ///
    /// This operation does not consume or unroot this `ManuallyRooted<T>`.
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
    /// let manual = {
    ///     let mut scope = RootScope::new(&mut store);
    ///     root1 = ExternRef::new(&mut scope, 1234);
    ///     root1.to_manually_rooted(&mut scope)?
    /// };
    ///
    /// // `root1` is no longer accessible because it was unrooted when `scope`
    /// // was dropped.
    /// assert!(root1.data(&store).is_err());
    ///
    /// // But we can re-root `manual` into this scope.
    /// let root2 = manual.to_rooted(&mut store);
    /// assert!(root2.data(&store).is_ok());
    ///
    /// // And we also still have access to `manual` and we still have to
    /// // manually unroot it.
    /// assert!(manual.data(&store).is_ok());
    /// manual.unroot(&mut store);
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
        let gc_ref = *self.get_gc_ref(&store).unwrap();

        // Safety: `gc_ref` is associated with this store, asserted by
        // `get_gc_ref`.
        let gc_ref = unsafe { clone_root(&mut store, gc_ref) };

        // Safety: `gc_ref` points to a valid `T` because it came from `self`.
        unsafe { Rooted::new(&mut store, gc_ref) }
    }

    /// Convert this `ManuallyRooted<T>` into a `Rooted<T>`.
    ///
    /// The underlying GC object is re-rooted in the given context's scope. The
    /// resulting `Rooted<T>` is only valid during the given context's
    /// scope. See the [`Rooted<T>`][crate::Rooted] documentation for more
    /// details on rooting scopes.
    ///
    /// This operation consumes and unroots this `ManuallyRooted<T>`.
    ///
    /// # Panics
    ///
    /// Panics if this object is not associate with the given context's store.
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
    /// let manual = {
    ///     let mut scope = RootScope::new(&mut store);
    ///     root1 = ExternRef::new(&mut scope, 1234);
    ///     root1.to_manually_rooted(&mut scope)?
    /// };
    ///
    /// // `root1` is no longer accessible because it was unrooted when `scope`
    /// // was dropped.
    /// assert!(root1.data(&store).is_err());
    ///
    /// // But we can re-root `manual` into this scope.
    /// let root2 = manual.into_rooted(&mut store);
    /// assert!(root2.data(&store).is_ok());
    ///
    /// // `manual` was consumed by the `into_rooted` call, and we no longer
    /// // have access to it, nor need to manually unroot it.
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_rooted(self, mut context: impl AsContextMut) -> Rooted<T> {
        self._into_rooted(context.as_context_mut().0)
    }

    pub(crate) fn _into_rooted(self, store: &mut StoreOpaque) -> Rooted<T> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );
        let rooted = self._to_rooted(store);
        self._unroot(store);
        rooted
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for ManuallyRooted<T> {
    #[allow(private_interfaces)]
    fn get_gc_ref<'a>(&self, store: &'a StoreOpaque) -> Option<&'a VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );

        let id = self.inner.index.as_manual().unwrap();
        store.gc_roots().manually_rooted.get(id)
    }

    #[allow(private_interfaces)]
    fn get_gc_ref_mut<'a>(&self, store: &'a mut StoreOpaque) -> Option<&'a mut VMGcRef> {
        assert!(
            self.comes_from_same_store(store),
            "object used with wrong store"
        );

        let id = self.inner.index.as_manual().unwrap();
        store.gc_roots_mut().manually_rooted.get_mut(id)
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
        assert_eq!(std::mem::size_of::<ManuallyRooted<ExternRef>>(), 16);
    }
}
