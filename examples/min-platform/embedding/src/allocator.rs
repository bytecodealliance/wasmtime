//! An allocator definition for this embedding.
//!
//! The Rust standard library and Wasmtime require a memory allocator to be
//! configured. For custom embeddings of Wasmtime this might likely already be
//! defined elsewhere in the system in which case that should be used. This file
//! contains an example implementation using the Rust `dlmalloc` crate using
//! memory created by `wasmtime_*` platform symbols. This provides a file that
//! manages memory without any extra runtime dependencies, but this is just an
//! example.
//!
//! Allocators in Rust are configured with the `#[global_allocator]` attribute
//! and the `GlobalAlloc for T` trait impl. This should be used when hooking
//! up to an allocator elsewhere in the system.

use alloc::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::sync::atomic::{
    AtomicBool,
    Ordering::{Acquire, Release},
};
use dlmalloc::Dlmalloc;

#[global_allocator]
static MALLOC: MyGlobalDmalloc = MyGlobalDmalloc {
    dlmalloc: Mutex::new(Dlmalloc::new_with_allocator(MyAllocator)),
};

struct MyGlobalDmalloc {
    dlmalloc: Mutex<Dlmalloc<MyAllocator>>,
}

struct MyAllocator;

unsafe impl GlobalAlloc for MyGlobalDmalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.dlmalloc
            .try_lock()
            .unwrap()
            .malloc(layout.size(), layout.align())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.dlmalloc
            .try_lock()
            .unwrap()
            .calloc(layout.size(), layout.align())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.dlmalloc
            .try_lock()
            .unwrap()
            .realloc(ptr, layout.size(), layout.align(), new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dlmalloc
            .try_lock()
            .unwrap()
            .free(ptr, layout.size(), layout.align())
    }
}

const INITIAL_HEAP_SIZE: usize = 64 * 1024;
static mut INITIAL_HEAP: [u8; INITIAL_HEAP_SIZE] = [0; INITIAL_HEAP_SIZE];
static mut INITIAL_HEAP_ALLOCATED: bool = false;

unsafe impl dlmalloc::Allocator for MyAllocator {
    fn alloc(&self, _size: usize) -> (*mut u8, usize, u32) {
        unsafe {
            if INITIAL_HEAP_ALLOCATED {
                (ptr::null_mut(), 0, 0)
            } else {
                INITIAL_HEAP_ALLOCATED = true;
                (ptr::addr_of_mut!(INITIAL_HEAP).cast(), INITIAL_HEAP_SIZE, 0)
            }
        }
    }

    fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
        core::ptr::null_mut()
    }

    fn free_part(&self, _ptr: *mut u8, _old: usize, _new: usize) -> bool {
        false
    }

    fn free(&self, _ptr: *mut u8, _size: usize) -> bool {
        false
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }

    fn allocates_zeros(&self) -> bool {
        true
    }

    fn page_size(&self) -> usize {
        4096
    }
}

// Simple mutex which only supports `try_lock` at this time. This would probably
// be replaced with a "real" mutex in a "real" embedding.
struct Mutex<T> {
    data: UnsafeCell<T>,
    locked: AtomicBool,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    const fn new(val: T) -> Mutex<T> {
        Mutex {
            data: UnsafeCell::new(val),
            locked: AtomicBool::new(false),
        }
    }

    fn try_lock(&self) -> Option<impl DerefMut<Target = T> + '_> {
        if self.locked.swap(true, Acquire) {
            None
        } else {
            Some(MutexGuard { lock: self })
        }
    }
}

struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Release);
    }
}
