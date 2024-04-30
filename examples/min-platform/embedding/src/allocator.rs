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

use dlmalloc::Dlmalloc;
use std::alloc::{GlobalAlloc, Layout};
use std::ptr;
use std::sync::Mutex;

#[global_allocator]
static MALLOC: MyGlobalDmalloc = MyGlobalDmalloc {
    dlmalloc: Mutex::new(Dlmalloc::new_with_allocator(MyAllocator)),
};

struct MyGlobalDmalloc {
    dlmalloc: Mutex<Dlmalloc<MyAllocator>>,
}

unsafe impl Send for MyGlobalDmalloc {}
unsafe impl Sync for MyGlobalDmalloc {}

struct MyAllocator;

unsafe impl GlobalAlloc for MyGlobalDmalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.dlmalloc
            .lock()
            .unwrap()
            .malloc(layout.size(), layout.align())
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.dlmalloc
            .lock()
            .unwrap()
            .calloc(layout.size(), layout.align())
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.dlmalloc
            .lock()
            .unwrap()
            .realloc(ptr, layout.size(), layout.align(), new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dlmalloc
            .lock()
            .unwrap()
            .free(ptr, layout.size(), layout.align())
    }
}

// Hand-copied from `crates/wasmtime/src/runtime/vm/sys/custom/capi.rs`.
const PROT_READ: u32 = 1 << 0;
const PROT_WRITE: u32 = 1 << 1;
extern "C" {
    fn wasmtime_mmap_new(size: usize, prot_flags: u32, ret: &mut *mut u8) -> i32;
    fn wasmtime_page_size() -> usize;
    fn wasmtime_munmap(ptr: *mut u8, size: usize) -> i32;
}

unsafe impl dlmalloc::Allocator for MyAllocator {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        unsafe {
            let mut ptr = ptr::null_mut();
            let rc = wasmtime_mmap_new(size, PROT_READ | PROT_WRITE, &mut ptr);
            if rc != 0 {
                (ptr::null_mut(), 0, 0)
            } else {
                (ptr, size, 0)
            }
        }
    }

    fn remap(&self, _ptr: *mut u8, _old: usize, _new: usize, _can_move: bool) -> *mut u8 {
        std::ptr::null_mut()
    }

    fn free_part(&self, _ptr: *mut u8, _old: usize, _new: usize) -> bool {
        false
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        unsafe {
            wasmtime_munmap(ptr, size);
            true
        }
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        false
    }

    fn allocates_zeros(&self) -> bool {
        true
    }

    fn page_size(&self) -> usize {
        unsafe { wasmtime_page_size() }
    }
}
