use crate::store::{Store, StoreInner};
use std::ops::{Deref, DerefMut};

/// TODO
#[repr(transparent)] // here for the C API
pub struct StoreContext<'a, T>(pub(super) &'a StoreInner<T>);

/// TODO
#[repr(transparent)] // here for the C API
pub struct StoreContextMut<'a, T>(pub(super) &'a mut StoreInner<T>);

impl<'a, T> StoreContextMut<'a, T> {
    // TODO
    pub(crate) unsafe fn from_raw(
        store: *mut dyn wasmtime_runtime::Store,
    ) -> StoreContextMut<'a, T> {
        StoreContextMut(&mut *(store as *mut StoreInner<T>))
    }

    /// A helper method to erase the `T` on `Self` so the returned type has no
    /// generics. For some more information see [`StoreOpaque`] itself.
    ///
    /// The primary purpose of this is to help improve compile times where
    /// non-generic code can be compiled into libwasmtime.rlib.
    pub(crate) fn opaque(mut self) -> StoreOpaque<'a> {
        StoreOpaque {
            traitobj: self.traitobj(),
            inner: self.0,
        }
    }

    pub(crate) fn opaque_send(mut self) -> StoreOpaqueSend<'a>
    where
        T: Send,
    {
        StoreOpaqueSend {
            traitobj: self.traitobj(),
            inner: self.0,
        }
    }

    fn traitobj(&mut self) -> *mut dyn wasmtime_runtime::Store {
        unsafe {
            std::mem::transmute::<
                *mut (dyn wasmtime_runtime::Store + '_),
                *mut (dyn wasmtime_runtime::Store + 'static),
            >(self.0)
        }
    }
}

/// TODO
pub trait AsContext {
    /// TODO
    type Data;

    /// TODO
    fn as_context(&self) -> StoreContext<'_, Self::Data>;
}

/// TODO
pub trait AsContextMut: AsContext {
    /// TODO
    fn as_context_mut(&mut self) -> StoreContextMut<'_, Self::Data>;
}

impl<T> AsContext for Store<T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&self.inner)
    }
}

impl<T> AsContextMut for Store<T> {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        StoreContextMut(&mut self.inner)
    }
}

impl<T> AsContext for StoreContext<'_, T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&*self.0)
    }
}

impl<T> AsContext for StoreContextMut<'_, T> {
    type Data = T;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T> {
        StoreContext(&*self.0)
    }
}

impl<T> AsContextMut for StoreContextMut<'_, T> {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T> {
        StoreContextMut(&mut *self.0)
    }
}

// forward AsContext for &T
impl<T: AsContext> AsContext for &'_ T {
    type Data = T::Data;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T::Data> {
        T::as_context(*self)
    }
}

// forward AsContext for &mut T
impl<T: AsContext> AsContext for &'_ mut T {
    type Data = T::Data;

    #[inline]
    fn as_context(&self) -> StoreContext<'_, T::Data> {
        T::as_context(*self)
    }
}

// forward AsContextMut for &mut T
impl<T: AsContextMut> AsContextMut for &'_ mut T {
    #[inline]
    fn as_context_mut(&mut self) -> StoreContextMut<'_, T::Data> {
        T::as_context_mut(*self)
    }
}

//
impl<'a, T: AsContext> From<&'a T> for StoreContext<'a, T::Data> {
    fn from(t: &'a T) -> StoreContext<'a, T::Data> {
        t.as_context()
    }
}

impl<'a, T: AsContext> From<&'a mut T> for StoreContext<'a, T::Data> {
    fn from(t: &'a mut T) -> StoreContext<'a, T::Data> {
        T::as_context(t)
    }
}

impl<'a, T: AsContextMut> From<&'a mut T> for StoreContextMut<'a, T::Data> {
    fn from(t: &'a mut T) -> StoreContextMut<'a, T::Data> {
        t.as_context_mut()
    }
}

/// This structure is akin to a `StoreContextMut` except that the `T` is
/// "erased" to an opaque type.
///
/// This structure is used pervasively through wasmtime whenever the `T` isn't
/// needed (quite common!). This allows the compiler to erase generics and
/// compile more code in the wasmtime crate itself instead of monomorphizing
/// everything into consumer crates. The primary purpose of this is to help
/// compile times.
#[doc(hidden)] // this is part of `WasmTy`, but a hidden part, so hide this
pub struct StoreOpaque<'a> {
    /// The actual pointer to the `StoreInner` internals.
    inner: &'a mut StoreInner<dyn Opaque + 'a>,

    /// A raw trait object that can be used to invoke functions with. Note that
    /// this is a pointer which aliases with `inner` above, so extreme care
    /// needs to be used when using this (the above `inner` cannot be actively
    /// borrowed).
    pub traitobj: *mut dyn wasmtime_runtime::Store,
}

pub trait Opaque {}
impl<T> Opaque for T {}

// Deref impls to forward all methods on `StoreOpaque` to `StoreInner`.
impl<'a> Deref for StoreOpaque<'a> {
    type Target = StoreInner<dyn Opaque + 'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a> DerefMut for StoreOpaque<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

pub struct StoreOpaqueSend<'a> {
    /// The actual pointer to the `StoreInner` internals.
    inner: &'a mut StoreInner<dyn Opaque + Send + 'a>,
    pub traitobj: *mut dyn wasmtime_runtime::Store,
}

unsafe impl Send for StoreOpaqueSend<'_> {}

impl StoreOpaqueSend<'_> {
    pub fn opaque(&mut self) -> StoreOpaque<'_> {
        StoreOpaque {
            inner: &mut *self.inner,
            traitobj: self.traitobj,
        }
    }
}

impl<'a> Deref for StoreOpaqueSend<'a> {
    type Target = StoreInner<dyn Opaque + Send + 'a>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a> DerefMut for StoreOpaqueSend<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}
