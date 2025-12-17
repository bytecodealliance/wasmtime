use crate::runtime::vm::{GcStore, VMGcRef};
use crate::{
    AsContext, AsContextMut, GcRef, Result, RootedGcRef,
    store::{AutoAssertNoGc, StoreOpaque},
};
use core::convert::Infallible;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::marker;
use core::ops::{Deref, DerefMut};

mod sealed {
    use super::*;
    pub trait GcRefImpl {}
    pub trait RootedGcRefImpl<T: GcRef> {
        fn assert_unreachable<U>(&self) -> U;

        fn get_gc_ref<'a>(&self, _store: &'a StoreOpaque) -> Option<&'a VMGcRef> {
            self.assert_unreachable()
        }

        fn try_gc_ref<'a>(&self, _store: &'a StoreOpaque) -> Result<&'a VMGcRef> {
            self.assert_unreachable()
        }

        fn clone_gc_ref(&self, _store: &mut AutoAssertNoGc<'_>) -> Option<VMGcRef> {
            self.assert_unreachable()
        }

        fn try_clone_gc_ref(&self, _store: &mut AutoAssertNoGc<'_>) -> Result<VMGcRef> {
            self.assert_unreachable()
        }
    }
}
pub(crate) use sealed::*;

#[derive(Debug, Default)]
pub(crate) struct RootSet {}

impl RootSet {
    pub(crate) fn enter_lifo_scope(&self) -> usize {
        usize::MAX
    }

    pub(crate) fn exit_lifo_scope(&mut self, _gc_store: Option<&mut GcStore>, _scope: usize) {}
}

/// This type is disabled because the `gc` cargo feature was not enabled at
/// compile time.
pub struct Rooted<T: GcRef> {
    pub(crate) inner: Infallible,
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
    fn hash<H: Hasher>(&self, _state: &mut H) {
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

    pub fn to_owned_rooted(&self, _store: impl AsContextMut) -> Result<OwnedRooted<T>> {
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

/// This type has been disabled because the `gc` cargo feature was not enabled
/// at compile time.
pub struct RootScope<C>
where
    C: AsContextMut,
{
    inner: Infallible,
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

/// This type has been disabled because the `gc` cargo feature was not enabled
/// at compile time.
pub struct OwnedRooted<T>
where
    T: GcRef,
{
    pub(crate) inner: Infallible,
    _phantom: marker::PhantomData<T>,
}

impl<T: GcRef> Debug for OwnedRooted<T> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {}
    }
}

impl<T: GcRef> Deref for OwnedRooted<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.inner {}
    }
}

impl<T> OwnedRooted<T>
where
    T: GcRef,
{
    pub fn clone(&self, _store: impl AsContextMut) -> Self {
        match self.inner {}
    }

    pub fn to_rooted(&self, _context: impl AsContextMut) -> Rooted<T> {
        match self.inner {}
    }

    pub fn into_rooted(self, _context: impl AsContextMut) -> Rooted<T> {
        match self.inner {}
    }
}

impl<T: GcRef> RootedGcRefImpl<T> for OwnedRooted<T> {
    fn assert_unreachable<U>(&self) -> U {
        match self.inner {}
    }
}

pub(crate) struct OpaqueRootScope<S> {
    store: S,
}

impl<S> Deref for OpaqueRootScope<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<S> DerefMut for OpaqueRootScope<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl<S> OpaqueRootScope<S> {
    pub(crate) fn new(store: S) -> Self {
        OpaqueRootScope { store }
    }
}
