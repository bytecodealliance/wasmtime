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
        let size = alloc_size(layout.size(), layout.align());
        unsafe { self.dlmalloc.lock().malloc(size, layout.align()) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = alloc_size(layout.size(), layout.align());
        unsafe { self.dlmalloc.lock().calloc(size, layout.align()) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = alloc_size(layout.size(), layout.align());
        let new_size = alloc_size(new_size, layout.align());
        unsafe {
            self.dlmalloc
                .lock()
                .realloc(ptr, old_size, layout.align(), new_size)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = alloc_size(layout.size(), layout.align());
        unsafe { self.dlmalloc.lock().free(ptr, size, layout.align()) }
    }
}

pub const PAGE_SIZE: usize = 4096;

/// Returns the size to actually allocate for a request of `size` bytes with
/// alignment `align`.
///
/// Page-aligned allocations are rounded up to a whole number of pages so that
/// they never share a page with any other allocation. This is relied upon by
/// the custom code memory in `lib.rs`: Wasmtime allocates code images with
/// page alignment, and the embedding changes the protection of the pages
/// containing code, which must not affect unrelated allocations.
fn alloc_size(size: usize, align: usize) -> usize {
    if align >= PAGE_SIZE {
        size.next_multiple_of(align)
    } else {
        size
    }
}

// Note that this is large enough to accommodate the wasi component as well as
// multiple threads concurrently loading and running modules.
const INITIAL_HEAP_SIZE: usize = 16 * 1024 * 1024;

static mut INITIAL_HEAP: [u8; INITIAL_HEAP_SIZE] = [0; INITIAL_HEAP_SIZE];
static mut INITIAL_HEAP_ALLOCATED: bool = false;

unsafe impl dlmalloc::Allocator for MyAllocator {
    fn alloc(&self, _size: usize) -> (*mut u8, usize, u32) {
        unsafe {
            if INITIAL_HEAP_ALLOCATED {
                (ptr::null_mut(), 0, 0)
            } else {
                INITIAL_HEAP_ALLOCATED = true;
                ((&raw mut INITIAL_HEAP).cast(), INITIAL_HEAP_SIZE, 0)
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
        PAGE_SIZE
    }
}

// Simple spin lock, which is sufficient for this example's allocator as it may
// be used concurrently from multiple threads. This would probably be replaced
// with a "real" mutex in a "real" embedding.
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

    fn lock(&self) -> impl DerefMut<Target = T> + '_ {
        while self.locked.swap(true, Acquire) {
            core::hint::spin_loop();
        }
        MutexGuard { lock: self }
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
