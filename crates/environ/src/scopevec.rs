use std::cell::RefCell;

/// Small data structure to help extend the lifetime of a slice to a higher
/// scope.
///
/// This is currently used during component translation where translation in
/// general works on a borrowed slice which contains all input modules, but
/// generated adapter modules for components don't live within the original
/// slice but the data structures are much easier if the dynamically generated
/// adapter modules live for the same lifetime as the original input slice. To
/// solve this problem this `ScopeVec` helper is used to move ownership of a
/// `Vec<T>` to a higher scope in the program, then borrowing the slice from
/// that scope.
pub struct ScopeVec<T> {
    data: RefCell<Vec<Box<[T]>>>,
}

impl<T> ScopeVec<T> {
    /// Creates a new blank scope.
    pub fn new() -> ScopeVec<T> {
        ScopeVec {
            data: Default::default(),
        }
    }

    /// Transfers ownership of `data` into this scope and then yields the slice
    /// back to the caller.
    ///
    /// The original data will be deallocated when `self` is dropped.
    pub fn push(&self, data: Vec<T>) -> &mut [T] {
        let mut data: Box<[T]> = data.into();
        let ptr = data.as_mut_ptr();
        let len = data.len();
        self.data.borrow_mut().push(data);

        // This should be safe for a few reasons:
        //
        // * The returned pointer on the heap that `data` owns. Despite moving
        //   `data` around it doesn't actually move the slice itself around, so
        //   the pointer returned should be valid (and length).
        //
        // * The lifetime of the returned pointer is connected to the lifetime
        //   of `self`. This reflects how when `self` is destroyed the `data` is
        //   destroyed as well, or otherwise the returned slice will be valid
        //   for as long as `self` is valid since `self` owns the original data
        //   at that point.
        //
        // * This function was given ownership of `data` so it should be safe to
        //   hand back a mutable reference. Once placed within a `ScopeVec` the
        //   data is never mutated so the caller will enjoy exclusive access to
        //   the slice of the original vec.
        //
        // This all means that it should be safe to return a mutable slice of
        // all of `data` after the data has been pushed onto our internal list.
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}
