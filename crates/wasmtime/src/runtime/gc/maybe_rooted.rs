//! GC references held in [`Val`](crate::Val)s and [`Ref`](crate::Ref)s.

use crate::runtime::vm::VMGcRef;
use crate::store::{AutoAssertNoGc, StoreOpaque};
use crate::{
    AnyRef, ArrayRef, AsContextMut, EqRef, GcRef, HeapType, OwnedRooted, Result, Rooted, StructRef,
};
use core::fmt;

/// A reference to a GC object of type `T`, as held in a [`Val`](crate::Val)
/// or [`Ref`](crate::Ref).
///
/// Currently the only kind of reference is one rooted in the current LIFO
/// rooting scope, [`MaybeRooted::Rooted`].
pub enum MaybeRooted<T: GcRef> {
    /// A reference rooted in the current LIFO rooting scope; see [`Rooted`].
    Rooted(Rooted<T>),
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
        }
    }
}

impl<T: GcRef> From<Rooted<T>> for MaybeRooted<T> {
    #[inline]
    fn from(r: Rooted<T>) -> Self {
        MaybeRooted::Rooted(r)
    }
}

macro_rules! upcasts {
    ($($from:ty => $to:ty;)*) => {$(
        impl From<MaybeRooted<$from>> for MaybeRooted<$to> {
            #[inline]
            fn from(r: MaybeRooted<$from>) -> Self {
                match r {
                    MaybeRooted::Rooted(r) => MaybeRooted::Rooted(r.into()),
                }
            }
        }

        impl From<Rooted<$from>> for MaybeRooted<$to> {
            #[inline]
            fn from(r: Rooted<$from>) -> Self {
                MaybeRooted::Rooted(r.into())
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
    /// Get this reference as a [`Rooted<T>`], if it is rooted.
    #[inline]
    pub fn as_rooted(&self) -> Option<Rooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => Some(*r),
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
    pub fn to_owned_rooted(&self, store: impl AsContextMut) -> Result<OwnedRooted<T>> {
        match self {
            MaybeRooted::Rooted(r) => r.to_owned_rooted(store),
        }
    }

    pub(crate) fn comes_from_same_store(&self, store: &StoreOpaque) -> bool {
        match self {
            MaybeRooted::Rooted(r) => r.comes_from_same_store(store),
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
        }
    }

    /// The referenced object, cloned for storing it somewhere, if this
    /// reference is still valid.
    ///
    /// # Panics
    ///
    /// Panics if this reference is associated with a different store.
    pub(crate) fn try_clone_gc_ref(&self, store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
        match self {
            MaybeRooted::Rooted(r) => r.try_clone_gc_ref(store),
        }
    }

    /// The raw bits of this reference, exposed to Wasm, if this reference is
    /// still valid.
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
