/// A `Box<T>` lookalike for memory that's stored in a `Store<T>`
///
/// This is intended to be quite similar to a `Box<T>` except without the
/// `Deref` implementations. The main motivation for this type's existence is to
/// appease the aliasing rules in miri to ensure that `StoreBox` can be moved
/// around without invalidating pointers to the contents within the box. The
/// standard `Box<T>` type does not implement this for example and moving that
/// will invalidate derived pointers.
pub struct StoreBox<T: ?Sized>(*mut T);

unsafe impl<T: Send + ?Sized> Send for StoreBox<T> {}
unsafe impl<T: Sync + ?Sized> Sync for StoreBox<T> {}

impl<T> StoreBox<T> {
    /// Allocates space on the heap to store `val` and returns a pointer to it
    /// living on the heap.
    pub fn new(val: T) -> StoreBox<T> {
        StoreBox(Box::into_raw(Box::new(val)))
    }
}

impl<T: ?Sized> StoreBox<T> {
    /// Returns the underlying pointer to `T` which is owned by the store.
    pub fn get(&self) -> *mut T {
        self.0
    }
}

impl<T: ?Sized> Drop for StoreBox<T> {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.0));
        }
    }
}
