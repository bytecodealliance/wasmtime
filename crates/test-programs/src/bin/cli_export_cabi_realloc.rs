//! `wit-component` handles modules which export `cabi_realloc` in a special way, using it instead of `memory.grow`
//! to allocate the adapter stack, hence this test.

#[export_name = "cabi_realloc"]
unsafe extern "C" fn cabi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    align: usize,
    new_len: usize,
) -> *mut u8 {
    use std::alloc::{self, Layout};

    let layout;
    let ptr = if old_len == 0 {
        if new_len == 0 {
            return align as *mut u8;
        }
        layout = Layout::from_size_align_unchecked(new_len, align);
        alloc::alloc(layout)
    } else {
        debug_assert_ne!(new_len, 0, "non-zero old_len requires non-zero new_len!");
        layout = Layout::from_size_align_unchecked(old_len, align);
        alloc::realloc(old_ptr, layout, new_len)
    };
    if ptr.is_null() {
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
    }
    return ptr;
}

fn main() {
    println!("hello, world");
}
