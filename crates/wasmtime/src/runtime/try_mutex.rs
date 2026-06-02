use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

const UNLOCKED: u8 = 0;
const LOCKED: u8 = 1;

/// A simple mutex which only supports the `try_lock` operation.
///
/// Suitable for use in Wasmtime when contention is never expected and is a bug
/// for example. This is effectively a `RefCell` that's `Sync` in that case. It
/// incurs runtime overhead that in theory is pure overhead at runtime at the
/// cost of "obviously safe" code during review.
pub struct TryMutex<T> {
    state: AtomicU8,
    data: UnsafeCell<T>,
}

// SAFETY: this type inherits `Send`-ness of the inner type `T`
unsafe impl<T: Send> Send for TryMutex<T> {}
// SAFETY: this is the standard `Sync` bound for any mutex-like structure.
unsafe impl<T: Send> Sync for TryMutex<T> {}

impl<T> TryMutex<T> {
    /// Creates a new `TryMutex` containing the given data.
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU8::new(UNLOCKED),
            data: UnsafeCell::new(data),
        }
    }

    /// Attempts to acquire the lock, returning `Some(_)` if
    /// successful and `None` if the lock is already held.
    ///
    /// This method does not block the current thread.
    ///
    /// The returned `TryMutexGuard` is an RAII guard to unlock this lock when
    /// dropped.
    pub fn try_lock(&self) -> Option<TryMutexGuard<'_, T>> {
        if self
            .state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(TryMutexGuard { mutex: self })
        } else {
            None
        }
    }
}

/// RAII guard returned by [`TryMutex::try_lock`] which can be used to access
/// the internal data in the lock.
pub struct TryMutexGuard<'a, T> {
    mutex: &'a TryMutex<T>,
}

impl<T> Deref for TryMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this object has exclusive access to the mutex so it is safe
        // to access the inner data.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for TryMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this object has exclusive access to the mutex so it is safe
        // to access the inner data.
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for TryMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.state.store(UNLOCKED, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_mutex() {
        let mutex = TryMutex::new(42);

        let mut guard = mutex.try_lock().expect("should acquire lock");
        assert_eq!(*guard, 42);
        assert!(mutex.try_lock().is_none());
        *guard = 43;
        drop(guard);
        let guard2 = mutex.try_lock().expect("should acquire lock again");
        assert_eq!(*guard2, 43);
    }
}
