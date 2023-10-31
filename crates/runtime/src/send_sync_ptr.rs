use std::fmt;
use std::ptr::NonNull;

/// A helper type in Wasmtime to store a raw pointer to `T` while automatically
/// inferring the `Send` and `Sync` traits for the container based on the
/// properties of `T`.
#[repr(transparent)]
pub struct SendSyncPtr<T: ?Sized>(NonNull<T>);

unsafe impl<T: Send + ?Sized> Send for SendSyncPtr<T> {}
unsafe impl<T: Sync + ?Sized> Sync for SendSyncPtr<T> {}

impl<T: ?Sized> SendSyncPtr<T> {
    /// Creates a new pointer wrapping the non-nullable pointer provided.
    pub fn new(ptr: NonNull<T>) -> SendSyncPtr<T> {
        SendSyncPtr(ptr)
    }

    /// Returns the underlying raw pointer.
    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    /// Unsafely assert that this is a pointer to valid contents and it's also
    /// valid to get a shared reference to it at this time.
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        self.0.as_ref()
    }

    /// Unsafely assert that this is a pointer to valid contents and it's also
    /// valid to get a mutable reference to it at this time.
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        self.0.as_mut()
    }

    /// Returns the underlying `NonNull<T>` wrapper.
    pub fn as_non_null(&self) -> NonNull<T> {
        self.0
    }
}

impl<T: ?Sized, U> From<U> for SendSyncPtr<T>
where
    U: Into<NonNull<T>>,
{
    fn from(ptr: U) -> SendSyncPtr<T> {
        SendSyncPtr::new(ptr.into())
    }
}

impl<T: ?Sized> fmt::Debug for SendSyncPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ptr().fmt(f)
    }
}

impl<T: ?Sized> fmt::Pointer for SendSyncPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ptr().fmt(f)
    }
}

impl<T: ?Sized> Clone for SendSyncPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for SendSyncPtr<T> {}

impl<T: ?Sized> PartialEq for SendSyncPtr<T> {
    fn eq(&self, other: &SendSyncPtr<T>) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for SendSyncPtr<T> {}
