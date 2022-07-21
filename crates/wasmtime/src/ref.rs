#![allow(missing_docs)]

use crate::AsContextMut;
use std::any::Any;
use wasmtime_runtime::VMExternRef;

/// Represents an opaque reference to any data within WebAssembly.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct ExternRef {
    pub(crate) inner: VMExternRef,
}

impl ExternRef {
    /// Creates a new instance of `ExternRef` wrapping the given value.
    pub fn new<T>(value: T) -> ExternRef
    where
        T: 'static + Any + Send + Sync,
    {
        let inner = VMExternRef::new(value);
        ExternRef { inner }
    }

    /// Get the underlying data for this `ExternRef`.
    pub fn data(&self) -> &dyn Any {
        &*self.inner
    }

    /// Get the strong reference count for this `ExternRef`.
    ///
    /// Note that this loads the reference count with a `SeqCst` ordering to
    /// synchronize with other threads.
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    /// Does this `ExternRef` point to the same inner value as `other`?
    ///
    /// This is *only* pointer equality, and does *not* run any inner value's
    /// `Eq` implementation.
    pub fn ptr_eq(&self, other: &ExternRef) -> bool {
        VMExternRef::eq(&self.inner, &other.inner)
    }

    /// Creates a new strongly-owned [`ExternRef`] from the raw value provided.
    ///
    /// This is intended to be used in conjunction with [`Func::new_unchecked`],
    /// [`Func::call_unchecked`], and [`ValRaw`] with its `externref` field.
    ///
    /// This function assumes that `raw` is an externref value which is
    /// currently rooted within the [`Store`].
    ///
    /// # Unsafety
    ///
    /// This function is particularly `unsafe` because `raw` not only must be a
    /// valid externref value produced prior by `to_raw` but it must also be
    /// correctly rooted within the store. When arguments are provided to a
    /// callback with [`Func::new_unchecked`], for example, or returned via
    /// [`Func::call_unchecked`], if a GC is performed within the store then
    /// floating externref values are not rooted and will be GC'd, meaning that
    /// this function will no longer be safe to call with the values cleaned up.
    /// This function must be invoked *before* possible GC operations can happen
    /// (such as calling wasm).
    ///
    /// When in doubt try to not use this. Instead use the safe Rust APIs of
    /// [`TypedFunc`] and friends.
    ///
    /// [`Func::call_unchecked`]: crate::Func::call_unchecked
    /// [`Func::new_unchecked`]: crate::Func::new_unchecked
    /// [`Store`]: crate::Store
    /// [`TypedFunc`]: crate::TypedFunc
    /// [`ValRaw`]: crate::ValRaw
    pub unsafe fn from_raw(raw: usize) -> Option<ExternRef> {
        let raw = raw as *mut u8;
        if raw.is_null() {
            None
        } else {
            Some(ExternRef {
                inner: VMExternRef::clone_from_raw(raw),
            })
        }
    }

    /// Converts this [`ExternRef`] to a raw value suitable to store within a
    /// [`ValRaw`].
    ///
    /// # Unsafety
    ///
    /// Produces a raw value which is only safe to pass into a store if a GC
    /// doesn't happen between when the value is produce and when it's passed
    /// into the store.
    ///
    /// [`ValRaw`]: crate::ValRaw
    pub unsafe fn to_raw(&self, mut store: impl AsContextMut) -> usize {
        let externref_ptr = self.inner.as_raw();
        store
            .as_context_mut()
            .0
            .insert_vmexternref_without_gc(self.inner.clone());
        externref_ptr as usize
    }
}

impl std::fmt::Pointer for ExternRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.inner, f)
    }
}
