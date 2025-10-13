//! Synchronization primitives for Wasmtime for `no_std`.
//!
//! These primitives are intended for use in `no_std` contexts are not as
//! full-featured as the `std` brethren. Namely these panic and/or return an
//! error on contention. This serves to continue to be correct in the face of
//! actual multiple threads, but if a system actually has multiple threads then
//! something will need to change in the Wasmtime crate to enable the external
//! system to perform necessary synchronization.
//!
//! See a brief overview of this module in `sync_std.rs` as well.

#![cfg_attr(
    all(feature = "std", not(test)),
    expect(dead_code, reason = "not used, but typechecked")
)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, AtomicU32, Ordering};

#[cfg(has_custom_sync)]
use crate::runtime::vm::capi::{
    wasmtime_sync_lock_acquire, wasmtime_sync_lock_free, wasmtime_sync_lock_new,
    wasmtime_sync_lock_release, wasmtime_sync_rwlock_free, wasmtime_sync_rwlock_new,
    wasmtime_sync_rwlock_read, wasmtime_sync_rwlock_read_release, wasmtime_sync_rwlock_write,
    wasmtime_sync_rwlock_write_release,
};

/// A host-provided lock handle.
///
/// The lock is stored as a `usize` initialized to 0. The host implementation
/// is responsible for lazy initialization via `wasmtime_sync_lock_new`.
#[cfg(has_custom_sync)]
#[derive(Debug)]
struct HostLock {
    storage: UnsafeCell<usize>,
}

#[cfg(has_custom_sync)]
impl HostLock {
    const fn new() -> HostLock {
        HostLock {
            storage: UnsafeCell::new(0),
        }
    }

    fn ensure(&self) -> *mut usize {
        let ptr = self.storage.get();
        // SAFETY: The host implementation handles lazy initialization and synchronization.
        unsafe {
            wasmtime_sync_lock_new(ptr);
        }
        ptr
    }
}

pub struct OnceLock<T> {
    val: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
    #[cfg(has_custom_sync)]
    lock: HostLock,
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
            #[cfg(has_custom_sync)]
            lock: HostLock::new(),
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
    #[cfg(not(has_custom_sync))]
    fn try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        match self.state.compare_exchange(
            UNINITIALIZED,
            INITIALIZING,
            Ordering::Acquire,
            Ordering::Acquire,
        ) {
            Ok(UNINITIALIZED) => match f() {
                Ok(val) => {
                    let ret = unsafe { &*(*self.val.get()).write(val) };
                    let prev = self.state.swap(INITIALIZED, Ordering::Release);
                    assert_eq!(prev, INITIALIZING);
                    Ok(ret)
                }
                Err(e) => match self.state.swap(UNINITIALIZED, Ordering::Release) {
                    INITIALIZING => Err(e),
                    _ => unreachable!(),
                },
            },
            Err(INITIALIZING) => panic!(
                "concurrent initialization only allowed with `std` or `custom-sync-primitives` features"
            ),
            Err(INITIALIZED) => Ok(self.get().unwrap()),
            _ => unreachable!(),
        }
    }

    #[cold]
    #[cfg(has_custom_sync)]
    fn try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        // Ensure lock is created
        let lock = self.lock.ensure();

        // SAFETY: lock.ensure() returns a valid lock handle from the host
        unsafe {
            wasmtime_sync_lock_acquire(lock);
        }

        // Check state again under lock
        let result = match self.state.load(Ordering::Acquire) {
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
        };

        // SAFETY: We acquired the lock above and must now release it
        unsafe {
            wasmtime_sync_lock_release(lock);
        }

        result
    }
}

impl<T> Drop for OnceLock<T> {
    fn drop(&mut self) {
        #[cfg(has_custom_sync)]
        {
            // SAFETY: We have exclusive access via &mut self
            let lock_value = unsafe { *self.lock.storage.get() };
            if lock_value != 0 {
                // SAFETY: Non-zero means the lock was initialized.
                unsafe {
                    wasmtime_sync_lock_free(self.lock.storage.get());
                }
            }
        }
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

#[derive(Debug)]
pub struct RwLock<T> {
    val: UnsafeCell<T>,
    #[cfg(not(has_custom_sync))]
    state: AtomicU32,
    #[cfg(has_custom_sync)]
    lock: HostLock,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(val: T) -> RwLock<T> {
        RwLock {
            val: UnsafeCell::new(val),
            #[cfg(not(has_custom_sync))]
            state: AtomicU32::new(0),
            #[cfg(has_custom_sync)]
            lock: HostLock::new(),
        }
    }

    #[cfg(not(has_custom_sync))]
    pub fn read(&self) -> impl Deref<Target = T> + '_ {
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
            Ok(_) => RwLockReadGuard { lock: self },
            Err(_) => panic!(
                "concurrent read request while locked for writing, must use `std` or `custom-sync-primitives` features to avoid panic"
            ),
        }
    }

    #[cfg(has_custom_sync)]
    pub fn read(&self) -> impl Deref<Target = T> + '_ {
        let handle = {
            let ptr = self.lock.storage.get();
            // SAFETY: The host implementation handles lazy initialization and synchronization.
            unsafe {
                wasmtime_sync_rwlock_new(ptr);
                wasmtime_sync_rwlock_read(ptr);
            }
            ptr
        };
        RwLockReadGuard {
            lock: self,
            #[cfg(has_custom_sync)]
            handle,
        }
    }

    #[cfg(not(has_custom_sync))]
    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        match self
            .state
            .compare_exchange(0, u32::MAX, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(0) => RwLockWriteGuard { lock: self },
            _ => panic!(
                "concurrent write request, must use `std` or `custom-sync-primitives` features to avoid panicking"
            ),
        }
    }

    #[cfg(has_custom_sync)]
    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        let handle = {
            let ptr = self.lock.storage.get();
            // SAFETY: The host implementation handles lazy initialization and synchronization.
            unsafe {
                wasmtime_sync_rwlock_new(ptr);
                wasmtime_sync_rwlock_write(ptr);
            }
            ptr
        };
        RwLockWriteGuard {
            lock: self,
            #[cfg(has_custom_sync)]
            handle,
        }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(T::default())
    }
}

#[cfg(has_custom_sync)]
impl<T> Drop for RwLock<T> {
    fn drop(&mut self) {
        // SAFETY: We have exclusive access via &mut self
        let lock_value = unsafe { *self.lock.storage.get() };
        if lock_value != 0 {
            // SAFETY: Non-zero means the lock was initialized.
            unsafe {
                wasmtime_sync_rwlock_free(self.lock.storage.get());
            }
        }
    }
}

struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
    #[cfg(has_custom_sync)]
    handle: *mut usize,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: We hold the read lock
        unsafe { &*self.lock.val.get() }
    }
}

#[cfg(not(has_custom_sync))]
impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(has_custom_sync)]
impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We acquired the read lock in RwLock::read()
        unsafe {
            wasmtime_sync_rwlock_read_release(self.handle);
        }
    }
}

struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
    #[cfg(has_custom_sync)]
    handle: *mut usize,
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

#[cfg(not(has_custom_sync))]
impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        match self.lock.state.swap(0, Ordering::Release) {
            u32::MAX => {}
            _ => unreachable!(),
        }
    }
}

#[cfg(has_custom_sync)]
impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We acquired the write lock in RwLock::write()
        unsafe {
            wasmtime_sync_rwlock_write_release(self.handle);
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
