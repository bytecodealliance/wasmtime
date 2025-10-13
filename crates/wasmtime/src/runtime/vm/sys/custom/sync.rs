//! Synchronization primitives for Wasmtime using host-provided implementations.
//!
//! This module provides the same interface as `sync_nostd` but delegates the
//! actual locking mechanism to host-provided CAPI functions. The Rust side
//! manages the data storage while the host provides the synchronization.
//!
//! This allows embedders (especially in kernel/embedded contexts) to provide
//! optimal synchronization primitives for their environment.

#![cfg_attr(
    all(feature = "std", not(test)),
    expect(dead_code, reason = "not used, but typechecked")
)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU8, Ordering};

use super::capi::{
    wasmtime_sync_lock_acquire, wasmtime_sync_lock_free, wasmtime_sync_lock_new,
    wasmtime_sync_lock_release, wasmtime_sync_rwlock_read, wasmtime_sync_rwlock_read_release,
    wasmtime_sync_rwlock_write, wasmtime_sync_rwlock_write_release,
};

/// A host-provided lock handle.
///
/// The lock is stored as a `usize` initialized to 0. The host implementation
/// is responsible for lazy initialization via `wasmtime_sync_lock_new`.
#[derive(Debug)]
struct HostLock {
    storage: UnsafeCell<usize>,
}

impl HostLock {
    const fn new() -> HostLock {
        HostLock {
            storage: UnsafeCell::new(0),
        }
    }

    fn ensure(&self) -> *mut usize {
        let ptr = self.storage.get();
        // SAFETY: The host implementation handles lazy initialization and synchronization.
        // It's safe to call this multiple times; the implementation ensures idempotency.
        unsafe { wasmtime_sync_lock_new(ptr) };
        ptr
    }
}

/// OnceLock implementation using host-provided lock
pub struct OnceLock<T> {
    val: UnsafeCell<MaybeUninit<T>>,
    state: AtomicU8,
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
    fn try_init<E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        // Ensure lock is created
        let lock = self.ensure_lock();

        // SAFETY: ensure_lock() returns a valid lock handle from the host
        unsafe { wasmtime_sync_lock_acquire(lock) };

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
        unsafe { wasmtime_sync_lock_release(lock) };

        result
    }

    fn ensure_lock(&self) -> *mut usize {
        self.lock.ensure()
    }
}

impl<T> Default for OnceLock<T> {
    fn default() -> OnceLock<T> {
        OnceLock::new()
    }
}

impl<T> Drop for OnceLock<T> {
    fn drop(&mut self) {
        // SAFETY: We have exclusive access via &mut self
        let lock_value = unsafe { *self.lock.storage.get() };
        if lock_value != 0 {
            // SAFETY: Non-zero means the lock was initialized.
            // We're in Drop, so the lock is no longer in use.
            unsafe { wasmtime_sync_lock_free(self.lock.storage.get()) };
        }
        if self.state.load(Ordering::Acquire) == INITIALIZED {
            // SAFETY: State is INITIALIZED, so val has been written
            unsafe { (*self.val.get()).assume_init_drop() };
        }
    }
}

/// RwLock implementation using host-provided lock
#[derive(Debug)]
pub struct RwLock<T> {
    val: UnsafeCell<T>,
    lock: HostLock,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(val: T) -> RwLock<T> {
        RwLock {
            val: UnsafeCell::new(val),
            lock: HostLock::new(),
        }
    }

    pub fn read(&self) -> impl Deref<Target = T> + '_ {
        let lock = self.ensure_lock();
        // SAFETY: ensure_lock() returns a valid lock handle from the host
        unsafe { wasmtime_sync_rwlock_read(lock) };
        RwLockReadGuard {
            lock: self,
            handle: lock,
        }
    }

    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        let lock = self.ensure_lock();
        // SAFETY: Lock pointer is initialized after ensure_lock()
        unsafe { wasmtime_sync_rwlock_write(lock) };
        RwLockWriteGuard {
            lock: self,
            handle: lock,
        }
    }

    fn ensure_lock(&self) -> *mut usize {
        self.lock.ensure()
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(T::default())
    }
}

impl<T> Drop for RwLock<T> {
    fn drop(&mut self) {
        // SAFETY: We have exclusive access via &mut self
        let lock_value = unsafe { *self.lock.storage.get() };
        if lock_value != 0 {
            // SAFETY: Non-zero means the lock was initialized.
            // We're in Drop, so the lock is no longer in use.
            unsafe { wasmtime_sync_lock_free(self.lock.storage.get()) };
        }
    }
}

struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
    handle: *mut usize,
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
        // SAFETY: We acquired the read lock in RwLock::read()
        unsafe { wasmtime_sync_rwlock_read_release(self.handle) };
    }
}

struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
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

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We acquired the write lock in RwLock::write()
        unsafe { wasmtime_sync_rwlock_write_release(self.handle) };
    }
}
