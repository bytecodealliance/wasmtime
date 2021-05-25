#[derive(Clone, Copy)]
pub struct UnsafeSendSync<T>(T);

impl<T> UnsafeSendSync<T> {
    /// Create a new `UnsafeSendSync` wrapper around the given value.
    ///
    /// The result is a type that is `Send` and `Sync` regardless of whether `T:
    /// Send + Sync`, so this constructor is unsafe.
    pub unsafe fn new(val: T) -> Self {
        UnsafeSendSync(val)
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}
