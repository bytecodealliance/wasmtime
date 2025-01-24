// Here we avoid using wit-bindgen so that we can export our own post-return
// function and keep track of whether it was called.

use std::{
    alloc::{self, Layout},
    mem::ManuallyDrop,
    sync::Mutex,
};

static POST_RETURN_VALUE: Mutex<Option<String>> = Mutex::new(None);

#[unsafe(export_name = "local:local/post-return#foo")]
unsafe extern "C" fn export_foo(ptr: *mut u8, len: usize) -> *mut u8 {
    let result = alloc::alloc(Layout::from_size_align(8, 4).unwrap());
    *result.cast::<*mut u8>() = ptr;
    *result.add(4).cast::<usize>() = len;
    result
}

#[unsafe(export_name = "cabi_post_local:local/post-return#foo")]
unsafe extern "C" fn export_post_return_foo(ptr: *mut u8) {
    let s_ptr = *ptr.cast::<*mut u8>();
    let s_len = *ptr.add(4).cast::<usize>();
    alloc::dealloc(ptr, Layout::from_size_align(8, 4).unwrap());

    *POST_RETURN_VALUE.lock().unwrap() =
        Some(String::from_utf8(Vec::from_raw_parts(s_ptr, s_len, s_len)).unwrap());
}

#[unsafe(export_name = "local:local/post-return#get-post-return-value")]
unsafe extern "C" fn export_get_post_return_value() -> *mut u8 {
    let s = ManuallyDrop::new(POST_RETURN_VALUE.lock().unwrap().take().unwrap());
    let result = alloc::alloc(Layout::from_size_align(8, 4).unwrap());
    *result.cast::<*mut u8>() = s.as_ptr().cast_mut();
    *result.add(4).cast::<usize>() = s.len();
    result
}

#[unsafe(export_name = "cabi_post_local:local/post-return#get-post-return-value")]
unsafe extern "C" fn export_post_return_get_post_return_value(ptr: *mut u8) {
    let s_ptr = *ptr.cast::<*mut u8>();
    let s_len = *ptr.add(4).cast::<usize>();
    alloc::dealloc(ptr, Layout::from_size_align(8, 4).unwrap());

    drop(String::from_utf8(Vec::from_raw_parts(s_ptr, s_len, s_len)).unwrap());
}

#[cfg(target_arch = "wasm32")]
#[unsafe(link_section = "component-type:wit-bindgen:0.37.0:local:local:post-return-callee:encoded world")]
#[doc(hidden)]
#[allow(
    clippy::octal_escapes,
    reason = "this is a machine-generated binary blob"
)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 255] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07w\x01A\x02\x01A\x02\x01\
B\x04\x01@\x01\x01ss\0s\x04\0\x03foo\x01\0\x01@\0\0s\x04\0\x15get-post-return-va\
lue\x01\x01\x04\0\x17local:local/post-return\x05\0\x04\0\x1elocal:local/post-ret\
urn-callee\x04\0\x0b\x18\x01\0\x12post-return-callee\x03\0\0\0G\x09producers\x01\
\x0cprocessed-by\x02\x0dwit-component\x070.223.0\x10wit-bindgen-rust\x060.37.0";

/// # Safety
/// TODO
#[unsafe(export_name = "cabi_realloc")]
pub unsafe extern "C" fn cabi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    assert!(old_ptr.is_null());
    assert!(old_len == 0);

    alloc::alloc(Layout::from_size_align(new_size, align).unwrap())
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
