//! Synchronization primitives for Wasmtime for `no_std`.
//!
//! These primitives are intended for use in `no_std` contexts and are not as
//! full-featured as the `std` brethren. Namely these panic on contention
//! unless the `custom-sync-primitives` feature is enabled. This serves to
//! continue to be correct in the face of actual multiple threads, but if a
//! system actually has multiple threads then the `custom-sync-primitives`
//! feature must be enabled to allow the external system to perform necessary
//! synchronization via host-provided locks.
//!
//! With `custom-sync-primitives` enabled, this module uses [`RawMutex`] and
//! [`RawRwLock`] which wrap host-provided synchronization primitives that
//! support true concurrent access with proper blocking behavior.
//!
//! See a brief overview of this module in `sync_std.rs` as well.

#![cfg_attr(
    all(feature = "std", not(test)),
    expect(dead_code, reason = "not used, but typechecked")
)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

pub struct OnceLock<T> {
    val: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
    mutex: raw::Mutex,
}

unsafe impl<T: Send> Send for OnceLock<T> {}
unsafe impl<T: Sync> Sync for OnceLock<T> {}

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

impl<T> OnceLock<T> {
    pub const fn new() -> OnceLock<T> {
        OnceLock {
            state: AtomicU8::new(UNINITIALIZED),
            val: UnsafeCell::new(MaybeUninit::uninit()),
            mutex: raw::Mutex::new(),
        }
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        if let Some(ret) = self.get() {
            return ret;
        }
        self.try_init::<()>(|| Ok(f())).unwrap()
    }

    pub fn get_or_try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        if let Some(ret) = self.get() {
            return Ok(ret);
        }
        self.try_init(f)
    }

    fn get(&self) -> Option<&T> {
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            // SAFETY: State is INITIALIZED, so val has been written
            Some(unsafe { (*self.val.get()).assume_init_ref() })
        } else {
            None
        }
    }

    #[cold]
    fn try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        let _guard = OnceLockGuard::new(&self.mutex);

        // Check state again under lock
        match self.state.load(Ordering::Acquire) {
            UNINITIALIZED => {
                self.state.store(INITIALIZING, Ordering::Release);
                match f() {
                    Ok(val) => {
                        // SAFETY: We hold the lock and state is INITIALIZING
                        let ret = unsafe { &*(*self.val.get()).write(val) };
                        self.state.store(INITIALIZED, Ordering::Release);
                        Ok(ret)
                    }
                    Err(e) => {
                        self.state.store(UNINITIALIZED, Ordering::Release);
                        Err(e)
                    }
                }
            }
            INITIALIZED => {
                // SAFETY: State is INITIALIZED, so val has been written
                Ok(unsafe { (*self.val.get()).assume_init_ref() })
            }
            _ => panic!("concurrent initialization"),
        }
    }
}

impl<T> Drop for OnceLock<T> {
    fn drop(&mut self) {
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            // SAFETY: State is INITIALIZED, so val has been written
            unsafe { (*self.val.get()).assume_init_drop() };
        }
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

struct OnceLockGuard<'a> {
    lock: &'a raw::Mutex,
}

impl<'a> OnceLockGuard<'a> {
    fn new(lock: &'a raw::Mutex) -> OnceLockGuard<'a> {
        lock.lock();
        OnceLockGuard { lock }
    }
}

impl Drop for OnceLockGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: We acquired the lock in OnceLockGuard::acquire
        unsafe {
            self.lock.unlock();
        }
    }
}

#[derive(Debug)]
pub struct RwLock<T> {
    val: UnsafeCell<T>,
    lock: raw::RwLock,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(val: T) -> RwLock<T> {
        RwLock {
            val: UnsafeCell::new(val),
            lock: raw::RwLock::new(),
        }
    }

    pub fn read(&self) -> impl Deref<Target = T> + '_ {
        self.lock.read();
        RwLockReadGuard { lock: self }
    }

    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        self.lock.write();
        RwLockWriteGuard { lock: self }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(T::default())
    }
}

struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: We hold the read lock
        unsafe { &*self.lock.val.get() }
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: This type represents a safe read lock being held, so it's
        // safe to perform the unlock here at the end.
        unsafe {
            self.lock.lock.read_unlock();
        }
    }
}

struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: We hold the write lock
        unsafe { &*self.lock.val.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: We hold the write lock
        unsafe { &mut *self.lock.val.get() }
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: This type represents a safe write lock being held, so it's
        // safe to perform the unlock here at the end.
        unsafe {
            self.lock.lock.write_unlock();
        }
    }
}

#[cfg(not(has_custom_sync))]
use panic_on_contention as raw;
#[cfg(not(has_custom_sync))]
mod panic_on_contention {
    use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    #[derive(Debug)]
    pub struct Mutex {
        locked: AtomicBool,
    }

    impl Mutex {
        pub const fn new() -> Mutex {
            Mutex {
                locked: AtomicBool::new(false),
            }
        }

        pub fn lock(&self) {
            if self.locked.swap(true, Ordering::Acquire) {
                panic!(
                    "concurrent lock request, must use `std` or `custom-sync-primitives` features to avoid panicking"
                );
            }
        }

        pub unsafe fn unlock(&self) {
            self.locked.store(false, Ordering::Release);
        }
    }

    #[derive(Debug)]
    pub struct RwLock {
        state: AtomicU32,
    }

    impl RwLock {
        pub const fn new() -> RwLock {
            RwLock {
                state: AtomicU32::new(0),
            }
        }

        pub fn read(&self) {
            const READER_LIMIT: u32 = u32::MAX / 2;
            match self
                .state
                .fetch_update(Ordering::Acquire, Ordering::Acquire, |x| match x {
                    u32::MAX => None,
                    n => {
                        let next = n + 1;
                        if next < READER_LIMIT {
                            Some(next)
                        } else {
                            None
                        }
                    }
                }) {
                Ok(_) => {}
                Err(_) => panic!(
                    "concurrent read request while locked for writing, must use `std` or `custom-sync-primitives` features to avoid panic"
                ),
            }
        }

        pub unsafe fn read_unlock(&self) {
            self.state.fetch_sub(1, Ordering::Release);
        }

        pub fn write(&self) {
            match self
                .state
                .compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(0) => {}
                _ => panic!(
                    "concurrent write request, must use `std` or `custom-sync-primitives` features to avoid panicking"
                ),
            }
        }

        pub unsafe fn write_unlock(&self) {
            match self.state.swap(0, Ordering::Release) {
                u32::MAX => {}
                _ => unreachable!(),
            }
        }
    }
}

#[cfg(has_custom_sync)]
use custom_capi as raw;
#[cfg(has_custom_sync)]
mod custom_capi {
    use crate::runtime::vm::capi;
    use core::cell::UnsafeCell;

    #[derive(Debug)]
    pub struct Mutex {
        storage: UnsafeCell<usize>,
    }

    impl Mutex {
        pub const fn new() -> Mutex {
            Mutex {
                storage: UnsafeCell::new(0),
            }
        }

        pub fn lock(&self) {
            unsafe {
                capi::wasmtime_sync_lock_acquire(self.storage.get());
            }
        }

        pub unsafe fn unlock(&self) {
            unsafe {
                capi::wasmtime_sync_lock_release(self.storage.get());
            }
        }
    }

    impl Drop for Mutex {
        fn drop(&mut self) {
            // SAFETY: We have exclusive access via &mut self
            // The host implementation handles the case where the lock was never used (still zero)
            unsafe {
                capi::wasmtime_sync_lock_free(self.storage.get());
            }
        }
    }

    #[derive(Debug)]
    pub struct RwLock {
        storage: UnsafeCell<usize>,
    }

    impl RwLock {
        pub const fn new() -> RwLock {
            RwLock {
                storage: UnsafeCell::new(0),
            }
        }

        pub fn read(&self) {
            unsafe {
                capi::wasmtime_sync_rwlock_read(self.storage.get());
            }
        }

        pub unsafe fn read_unlock(&self) {
            unsafe {
                capi::wasmtime_sync_rwlock_read_release(self.storage.get());
            }
        }

        pub fn write(&self) {
            unsafe {
                capi::wasmtime_sync_rwlock_write(self.storage.get());
            }
        }

        pub unsafe fn write_unlock(&self) {
            unsafe {
                capi::wasmtime_sync_rwlock_write_release(self.storage.get());
            }
        }
    }

    impl Drop for RwLock {
        fn drop(&mut self) {
            unsafe {
                capi::wasmtime_sync_rwlock_free(self.storage.get());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_once_lock() {
        let lock = OnceLock::new();
        assert!(lock.get().is_none());
        assert_eq!(*lock.get_or_init(|| 1), 1);
        assert_eq!(*lock.get_or_init(|| 2), 1);
        assert_eq!(*lock.get_or_init(|| 3), 1);
        assert_eq!(lock.get_or_try_init::<()>(|| Ok(3)), Ok(&1));

        let lock = OnceLock::new();
        assert_eq!(lock.get_or_try_init::<()>(|| Ok(3)), Ok(&3));
        assert_eq!(*lock.get_or_init(|| 1), 3);

        let lock = OnceLock::new();
        assert_eq!(lock.get_or_try_init(|| Err(())), Err(()));
        assert_eq!(*lock.get_or_init(|| 1), 1);
    }

    #[test]
    fn smoke_rwlock() {
        let lock = RwLock::new(1);
        assert_eq!(*lock.read(), 1);

        let a = lock.read();
        let b = lock.read();
        assert_eq!(*a, 1);
        assert_eq!(*b, 1);
        drop((a, b));

        assert_eq!(*lock.write(), 1);

        *lock.write() = 4;
        assert_eq!(*lock.read(), 4);
        assert_eq!(*lock.write(), 4);

        let a = lock.read();
        let b = lock.read();
        assert_eq!(*a, 4);
        assert_eq!(*b, 4);
        drop((a, b));
    }

    #[test]
    #[should_panic(expected = "concurrent write request")]
    fn rwlock_panic_read_then_write() {
        let lock = RwLock::new(1);
        let _a = lock.read();
        let _b = lock.write();
    }

    #[test]
    #[should_panic(expected = "concurrent read request")]
    fn rwlock_panic_write_then_read() {
        let lock = RwLock::new(1);
        let _a = lock.write();
        let _b = lock.read();
    }

    #[test]
    #[should_panic(expected = "concurrent write request")]
    fn rwlock_panic_write_then_write() {
        let lock = RwLock::new(1);
        let _a = lock.write();
        let _b = lock.write();
    }
}
