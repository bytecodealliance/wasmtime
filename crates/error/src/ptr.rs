//! Newtype wrappers over raw pointers that document ownership. We use these in
//! various places instead of safe references because our type-punning would
//! trigger UB otherwise.

use alloc::boxed::Box;
use core::{marker::PhantomData, ptr::NonNull};

/// A raw, owned pointer.
///
/// You are required to call `T`'s `Drop` and deallocate the pointer, this won't
/// automatically do it for you like `Box`.
#[repr(transparent)]
pub(crate) struct OwnedPtr<T>
where
    T: ?Sized,
{
    ptr: NonNull<T>,
}

impl<T> OwnedPtr<T> {
    pub(crate) fn new(ptr: NonNull<T>) -> Self {
        OwnedPtr { ptr }
    }

    pub(crate) fn cast<U>(self) -> OwnedPtr<U> {
        OwnedPtr::new(self.ptr.cast())
    }

    /// Make a raw copy of this pointer.
    pub(crate) fn raw_copy(&self) -> Self {
        Self::new(self.ptr)
    }

    pub(crate) fn into_non_null(self) -> NonNull<T> {
        self.ptr
    }

    /// # Safety
    ///
    /// It must be valid to call `NonNull::<T>::as_ref` on our underlying pointer.
    pub(crate) unsafe fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// # Safety
    ///
    /// It must be valid to call `Box::<T>::from_raw` on our underlying pointer.
    pub(crate) unsafe fn into_box(self) -> Box<T> {
        // Safety: same as our safety contract.
        unsafe { Box::from_raw(self.ptr.as_ptr()) }
    }
}

/// A raw pointer that is semantically a shared borrow.
#[repr(transparent)]
pub(crate) struct SharedPtr<'a, T>
where
    T: ?Sized,
{
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'a T>,
}

impl<'a, T> core::fmt::Debug for SharedPtr<'a, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SharedPtr").field("ptr", &self.ptr).finish()
    }
}

impl<T> Clone for SharedPtr<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> SharedPtr<'a, T>
where
    T: ?Sized,
{
    pub(crate) fn new(ptr: NonNull<T>) -> Self {
        SharedPtr {
            ptr,
            _lifetime: PhantomData,
        }
    }

    pub(crate) fn cast<U>(self) -> SharedPtr<'a, U> {
        SharedPtr::new(self.ptr.cast())
    }

    /// # Safety
    ///
    /// It must be valid to call `NonNull::<T>::as_ref` on the underlying
    /// pointer.
    pub(crate) unsafe fn as_ref(self) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Copy for SharedPtr<'_, T> where T: ?Sized {}

/// A raw pointer that is semantically an exclusive borrow.
#[repr(transparent)]
pub(crate) struct MutPtr<'a, T>
where
    T: ?Sized,
{
    ptr: NonNull<T>,
    _lifetime: PhantomData<&'a mut T>,
}

impl<'a, T> MutPtr<'a, T>
where
    T: ?Sized,
{
    pub(crate) fn new(ptr: NonNull<T>) -> Self {
        MutPtr {
            ptr,
            _lifetime: PhantomData,
        }
    }

    /// Make a raw copy of this pointer.
    pub(crate) fn raw_copy(&self) -> Self {
        Self::new(self.ptr)
    }

    pub(crate) fn cast<U>(self) -> MutPtr<'a, U> {
        MutPtr::new(self.ptr.cast())
    }

    pub(crate) fn as_shared_ptr(&self) -> SharedPtr<'_, T> {
        SharedPtr::new(self.ptr)
    }

    /// # Safety
    ///
    /// It must be valid to call `NonNull::<T>::as_ref` on the underlying pointer.
    pub(crate) unsafe fn as_ref(&self) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }

    /// # Safety
    ///
    /// It must be valid to call `NonNull::<T>::as_mut` on the underlying pointer.
    pub(crate) unsafe fn as_mut(&mut self) -> &'a mut T {
        unsafe { self.ptr.as_mut() }
    }
}
