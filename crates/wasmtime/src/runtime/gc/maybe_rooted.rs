//! GC references held in [`Val`](crate::Val)s and [`Ref`](crate::Ref)s.

use crate::runtime::vm::VMGcRef;
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, AsContext, AsContextMut, EqRef, GcRef, HeapType, OwnedRooted, Result, Rooted,
    StructRef, Unrooted, prelude::*,
};
use core::fmt;

/// A reference to a GC object of type `T`, as held in a [`Val`](crate::Val)
/// or [`Ref`](crate::Ref): either rooted or unrooted.
///
/// A [`MaybeRooted::Rooted`] reference keeps its object alive until the
/// current LIFO rooting scope exits; see [`Rooted`]. A
/// [`MaybeRooted::Unrooted`] reference is cheaper to create and needs only
/// shared access to the store, but is valid only until the store's GC heap
/// next changes; see [`Unrooted`].
///
/// APIs that read GC values, such as [`Global::get`](crate::Global::get),
/// [`StructRef::field`](crate::StructRef::field), or the results of
/// [`Func::call`](crate::Func::call), return unrooted references, so that
/// rooting is always opt-in: the caller can choose to root them, and how
/// (with [`MaybeRooted::to_rooted`], [`MaybeRooted::to_owned_rooted`], or
/// [`Val::ensure_all_rooted`](crate::Val::ensure_all_rooted) for several at
/// once), or to keep them unrooted.
///
/// APIs that take GC values to store or pass along, such as
/// [`Global::set`](crate::Global::set),
/// [`StructRef::new`](crate::StructRef::new), or the arguments of
/// [`Func::call`](crate::Func::call), mutate the store, which invalidates
/// unrooted references, and so they require rooted references, and return an
/// error for unrooted ones; root them first.
///
/// `MaybeRooted<T>` has no `OwnedRooted<T>` variant, so that it, like
/// [`Val`](crate::Val), is `Copy`.
pub enum MaybeRooted<T: GcRef> {
    /// A reference rooted in the current LIFO rooting scope; see [`Rooted`].
    Rooted(Rooted<T>),

    /// A reference that is not rooted, and is valid only until the store's GC
    /// heap next changes; see [`Unrooted`].
    Unrooted(Unrooted<T>),
}

impl<T: GcRef> Clone for MaybeRooted<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: GcRef> Copy for MaybeRooted<T> {}

impl<T: GcRef> fmt::Debug for MaybeRooted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaybeRooted::Rooted(r) => f.debug_tuple("Rooted").field(r).finish(),
            MaybeRooted::Unrooted(r) => f.debug_tuple("Unrooted").field(r).finish(),
        }
    }
}

impl<T: GcRef> From<Rooted<T>> for MaybeRooted<T> {
    #[inline]
    fn from(r: Rooted<T>) -> Self {
        MaybeRooted::Rooted(r)
    }
}

impl<T: GcRef> From<Unrooted<T>> for MaybeRooted<T> {
    #[inline]
    fn from(r: Unrooted<T>) -> Self {
        MaybeRooted::Unrooted(r)
    }
}

macro_rules! upcasts {
    ($($from:ty => $to:ty;)*) => {$(
        impl From<MaybeRooted<$from>> for MaybeRooted<$to> {
            #[inline]
            fn from(r: MaybeRooted<$from>) -> Self {
                match r {
                    MaybeRooted::Rooted(r) => MaybeRooted::Rooted(r.into()),
                    MaybeRooted::Unrooted(r) => MaybeRooted::Unrooted(r.into()),
                }
            }
        }

        impl From<Rooted<$from>> for MaybeRooted<$to> {
            #[inline]
            fn from(r: Rooted<$from>) -> Self {
                MaybeRooted::Rooted(r.into())
            }
        }

        impl From<Unrooted<$from>> for MaybeRooted<$to> {
            #[inline]
            fn from(r: Unrooted<$from>) -> Self {
                MaybeRooted::Unrooted(r.into())
            }
        }
    )*};
}

upcasts! {
    EqRef => AnyRef;
    StructRef => AnyRef;
    ArrayRef => AnyRef;
}

impl<T: GcRef> MaybeRooted<T> {
    /// Is this a rooted reference?
    #[inline]
    pub fn is_rooted(&self) -> bool {
        matches!(self, MaybeRooted::Rooted(_))
    }

    /// Is this an unrooted reference?
    #[inline]
    pub fn is_unrooted(&self) -> bool {
        matches!(self, MaybeRooted::Unrooted(_))
    }

    /// Get this reference as a [`Rooted<T>`], if it is rooted.
    #[inline]
    pub fn as_rooted(&self) -> Option<Rooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => Some(*r),
            MaybeRooted::Unrooted(_) => None,
        }
    }

    /// Get this reference as a [`Rooted<T>`], panicking if it is not rooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is not rooted.
    #[inline]
    pub fn unwrap_rooted(&self) -> Rooted<T> {
        self.as_rooted()
            .expect("MaybeRooted::unwrap_rooted on a reference that is not rooted")
    }

    /// Get this reference as an [`Unrooted<T>`], if it is unrooted.
    #[inline]
    pub fn as_unrooted(&self) -> Option<Unrooted<T>> {
        match self {
            MaybeRooted::Rooted(_) => None,
            MaybeRooted::Unrooted(r) => Some(*r),
        }
    }

    /// Get this reference as an [`Unrooted<T>`], panicking if it is not
    /// unrooted.
    ///
    /// # Panics
    ///
    /// Panics if this reference is not unrooted.
    #[inline]
    pub fn unwrap_unrooted(&self) -> Unrooted<T> {
        self.as_unrooted()
            .expect("MaybeRooted::unwrap_unrooted on a reference that is not unrooted")
    }

    /// Get this reference as a [`Rooted<T>`], or return an error if it is not
    /// rooted.
    ///
    /// APIs that take GC values to store or pass along, such as
    /// [`StructRef::new`](crate::StructRef::new), use this to check that their
    /// values are rooted.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is not rooted.
    pub fn validate_rooted(&self) -> Result<Rooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => Ok(*r),
            MaybeRooted::Unrooted(_) => bail!(
                "expected a rooted GC reference, found an unrooted one; root it \
                 first with `MaybeRooted::ensure_rooted` or `Val::ensure_rooted`"
            ),
        }
    }

    /// Root this reference in place, if it is not rooted already, in the
    /// current LIFO rooting scope, and return it as a [`Rooted<T>`].
    ///
    /// Rooting a reference invalidates every other unrooted reference into the
    /// store's GC heap; use [`Rooted::to_all_rooted`] or
    /// [`Val::ensure_all_rooted`](crate::Val::ensure_all_rooted) to root
    /// several references at once.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is unrooted and no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn ensure_rooted(&mut self, mut store: impl AsContextMut) -> Result<Rooted<T>> {
        self._ensure_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _ensure_rooted(&mut self, store: &mut StoreOpaque) -> Result<Rooted<T>> {
        let rooted = self._to_rooted(store)?;
        *self = MaybeRooted::Rooted(rooted);
        Ok(rooted)
    }

    /// Get a [`Rooted<T>`] referencing the same GC object as this reference:
    /// this reference itself if it is rooted, or a new root in the current
    /// LIFO rooting scope if it is unrooted.
    ///
    /// Rooting a reference invalidates every other unrooted reference into the
    /// store's GC heap; use [`Rooted::to_all_rooted`] or
    /// [`Val::ensure_all_rooted`](crate::Val::ensure_all_rooted) to root
    /// several references at once.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is unrooted and no longer valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_rooted(&self, mut store: impl AsContextMut) -> Result<Rooted<T>> {
        self._to_rooted(store.as_context_mut().0)
    }

    pub(crate) fn _to_rooted(&self, store: &mut StoreOpaque) -> Result<Rooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => Ok(*r),
            MaybeRooted::Unrooted(r) => r._to_rooted(store),
        }
    }

    /// Get an [`Unrooted<T>`] referencing the same GC object as this
    /// reference.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid: if it is rooted
    /// and has since been unrooted, or if it is unrooted and the store's GC
    /// heap has since changed.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_unrooted(&self, store: impl AsContext) -> Result<Unrooted<T>> {
        self._to_unrooted(store.as_context().0)
    }

    pub(crate) fn _to_unrooted(&self, store: &StoreOpaque) -> Result<Unrooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => r._to_unrooted(store),
            MaybeRooted::Unrooted(r) => {
                r.gc_ref(store)?;
                Ok(*r)
            }
        }
    }

    /// Create an [`OwnedRooted<T>`] holding onto the same GC object as this
    /// reference.
    ///
    /// # Errors
    ///
    /// Returns an error if this reference is no longer valid.
    ///
    /// This function will return an [`OutOfMemory`](crate::OutOfMemory)
    /// error when memory allocation fails. See the `OutOfMemory` type's
    /// documentation for details on Wasmtime's out-of-memory handling.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub fn to_owned_rooted(&self, mut store: impl AsContextMut) -> Result<OwnedRooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => r.to_owned_rooted(store),
            MaybeRooted::Unrooted(r) => r._to_owned_rooted(store.as_context_mut().0),
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            MaybeRooted::Rooted(r) => r.comes_from_same_store(store),
            MaybeRooted::Unrooted(r) => r.comes_from_same_store(store),
        }
    }

    /// The referenced object, if this reference is still valid.
    ///
    /// The result borrows `store`, so nothing can collect garbage or
    /// otherwise mutate the GC heap while it is in use.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn try_gc_ref<'a>(&'a self, store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
        match self {
            MaybeRooted::Rooted(r) => r.try_gc_ref(store),
            MaybeRooted::Unrooted(r) => r.gc_ref(store),
        }
    }

    /// Root this reference in place, if it is unrooted, after the caller has
    /// checked that it is valid (with `try_gc_ref`), as part of rooting several
    /// references at once; see `Unrooted::prevalidated_gc_ref` for the
    /// conditions.
    pub(crate) fn ensure_rooted_prevalidated(&mut self, store: &mut AutoAssertNoGc<'_>) {
        if let MaybeRooted::Unrooted(r) = self {
            *self = MaybeRooted::Rooted(r.to_rooted_prevalidated(store));
        }
    }

    /// The referenced object, cloned for storing it somewhere, if this
    /// reference is rooted and still valid.
    ///
    /// Storing a reference mutates the store, so this is an error for an
    /// unrooted reference.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn try_clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
        self.validate_rooted()?.try_clone_gc_ref(store)
    }

    /// The raw bits of this reference, exposed to Wasm, if this reference is
    /// rooted and still valid.
    ///
    /// Exposing a reference to Wasm mutates the store, so this is an error for
    /// an unrooted reference.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn _to_raw(&self, store: &mut AutoAssertNoGc<'_>) -> Result<u32> {
        let gc_ref = self.try_clone_gc_ref(store)?;
        let raw = match store.optional_gc_store_mut() {
            Some(s) => s.expose_gc_ref_to_wasm(gc_ref)?,
            None => {
                // NB: do not force the allocation of a GC heap just because the
                // program is using `i31ref`s.
                debug_assert!(gc_ref.is_i31());
                gc_ref.as_raw_non_zero_u32()
            }
        };
        Ok(raw.get())
    }

    /// The raw bits of the optional reference `this`, exposed to Wasm; zero
    /// for null.
    pub(crate) fn option_to_raw(
        this: Option<&Self>,
        store: &mut AutoAssertNoGc<'_>,
    ) -> Result<u32> {
        match this {
            Some(r) => r._to_raw(store),
            None => Ok(0),
        }
    }
}

impl MaybeRooted<AnyRef> {
    /// The type of the referenced object; see [`AnyRef::ty`].
    pub(crate) fn _ty(&self, store: &StoreOpaque) -> Result<HeapType> {
        AnyRef::_ty_of(store, self.try_gc_ref(store)?)
    }
}
