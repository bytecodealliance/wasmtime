//! The dummy `ExternRef` type used when the `gc` cargo feature is disabled.
//!
//! Providing a dummy type means that downstream users need to do fewer
//! `#[cfg(...)]`s versus if this type or its methods simply didn't exist. The
//! only methods that are left missing are constructors.

#![allow(missing_docs)]

use crate::prelude::*;
use crate::runtime::Uninhabited;
use crate::{store::StoreOpaque, AsContext, AsContextMut, GcRef, Result, RootedGcRef};
use core::any::Any;
use core::ffi::c_void;
use core::fmt::{self, Debug};
use core::hash::{self, Hash};
use core::marker;
use core::ops::Deref;
use wasmtime_runtime::VMExternRef;

mod sealed {
    use super::*;
    pub trait GcRefImpl {}
    pub trait RootedGcRefImpl<T: GcRef> {
        fn assert_unreachable<U>(&self) -> U;
    }
}
pub(crate) use sealed::*;

/// Represents an opaque reference to any data within WebAssembly.
///
/// Due to compilation configuration, this is an uninhabited type: enable the
/// `gc` cargo feature to properly use this type.
#[derive(Debug)]
pub struct ExternRef {
    _inner: Uninhabited,
}

impl GcRefImpl for ExternRef {}

impl ExternRef {
    pub(crate) fn from_vm_extern_ref(_store: &mut StoreOpaque, inner: VMExternRef) -> Rooted<Self> {
        inner.assert_unreachable()
    }

    pub(crate) fn into_vm_extern_ref(self) -> VMExternRef {
        match self._inner {}
    }

    pub(crate) fn try_to_vm_extern_ref(&self, _store: &mut StoreOpaque) -> Result<VMExternRef> {
        match self._inner {}
    }

    pub fn data<'a>(&self, _store: &'a impl AsContext) -> Result<&'a (dyn Any + Send + Sync)> {
        match self._inner {}
    }

    pub fn data_mut<'a>(
        &self,
        _store: &'a mut impl AsContextMut,
    ) -> Result<&'a mut (dyn Any + Send + Sync)> {
        match self._inner {}
    }

    pub unsafe fn from_raw(
        _store: impl AsContextMut,
        raw: *mut c_void,
    ) -> Option<Rooted<ExternRef>> {
        assert!(raw.is_null());
        None
    }

    pub unsafe fn to_raw(&self, _store: impl AsContextMut) -> Result<*mut c_void> {
        match self._inner {}
    }
}

#[derive(Debug, Default)]
pub(crate) struct RootSet {}

impl RootSet {
    pub(crate) fn enter_lifo_scope(&self) -> usize {
        usize::MAX
    }

    pub(crate) fn exit_lifo_scope(&mut self, _scope: usize) {}

    pub(crate) fn with_lifo_scope<T>(
        store: &mut StoreOpaque,
        f: impl FnOnce(&mut StoreOpaque) -> T,
    ) -> T {
        f(store)
    }
}

/// A scoped, rooted GC reference.
///
/// This type is disabled because the `gc` cargo feature was not enabled at
/// compile time.
pub struct Rooted<T: GcRef> {
    inner: Uninhabited,
    _phantom: marker::PhantomData<T>,
}

impl<T: GcRef> Clone for Rooted<T> {
    fn clone(&self) -> Self {
        match self.inner {}
    }
}

impl<T: GcRef> Copy for Rooted<T> {}

impl<T: GcRef> Debug for Rooted<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {}
    }
}

impl<T: GcRef> PartialEq for Rooted<T> {
    fn eq(&self, _other: &Self) -> bool {
        match self.inner {}
    }
}

impl<T: GcRef> Eq for Rooted<T> {}

impl<T: GcRef> Hash for Rooted<T> {
    fn hash<H: hash::Hasher>(&self, _state: &mut H) {
        match self.inner {}
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for Rooted<T> {
    fn assert_unreachable<U>(&self) -> U {
        match self.inner {}
    }
}

impl<T: GcRef> Deref for Rooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.inner {}
    }
}

impl<T: GcRef> Rooted<T> {
    pub(crate) fn comes_from_same_store(&self, _store: &StoreOpaque) -> bool {
        match self.inner {}
    }

    pub fn to_manually_rooted(&self, _store: impl AsContextMut) -> Result<ManuallyRooted<T>> {
        match self.inner {}
    }

    pub fn rooted_eq(a: Self, _b: Self) -> bool {
        match a.inner {}
    }

    pub fn ref_eq(
        _store: impl AsContext,
        a: &impl RootedGcRef<T>,
        _b: &impl RootedGcRef<T>,
    ) -> Result<bool> {
        a.assert_unreachable()
    }
}

/// Nested rooting scopes.
///
/// This type has been disabled because the `gc` cargo feature was not enabled
/// at compile time.
pub struct RootScope<C>
where
    C: AsContextMut,
{
    inner: Uninhabited,
    _phantom: marker::PhantomData<C>,
}

impl<C> RootScope<C>
where
    C: AsContextMut,
{
    pub fn reserve(&mut self, _additional: usize) {
        match self.inner {}
    }
}

impl<T> AsContext for RootScope<T>
where
    T: AsContextMut,
{
    type Data = T::Data;

    fn as_context(&self) -> crate::StoreContext<'_, Self::Data> {
        match self.inner {}
    }
}

impl<T> AsContextMut for RootScope<T>
where
    T: AsContextMut,
{
    fn as_context_mut(&mut self) -> crate::StoreContextMut<'_, Self::Data> {
        match self.inner {}
    }
}

/// A rooted reference to a garbage-collected `T` with arbitrary lifetime.
///
/// This type has been disabled because the `gc` cargo feature was not enabled
/// at compile time.
pub struct ManuallyRooted<T>
where
    T: GcRef,
{
    inner: Uninhabited,
    _phantom: marker::PhantomData<T>,
}

impl<T: GcRef> Debug for ManuallyRooted<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {}
    }
}

impl<T: GcRef> Deref for ManuallyRooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.inner {}
    }
}

impl<T> ManuallyRooted<T>
where
    T: GcRef,
{
    pub(crate) fn comes_from_same_store(&self, _store: &StoreOpaque) -> bool {
        match self.inner {}
    }

    pub fn clone(&self, _store: impl AsContextMut) -> Self {
        match self.inner {}
    }

    pub fn unroot(self, _store: impl AsContextMut) {
        match self.inner {}
    }

    pub fn to_rooted(&self, _context: impl AsContextMut) -> Rooted<T> {
        match self.inner {}
    }

    pub fn into_rooted(self, _context: impl AsContextMut) -> Rooted<T> {
        match self.inner {}
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for ManuallyRooted<T> {
    fn assert_unreachable<U>(&self) -> U {
        match self.inner {}
    }
}
