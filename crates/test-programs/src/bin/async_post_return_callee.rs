#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{
    alloc::{self, Layout},
    sync::Mutex,
};

static POST_RETURN_VALUE: Mutex<Option<String>> = Mutex::new(None);

mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "post-return-callee",
        // Here we avoid using wit-bindgen so that we can export our own
        // post-return function and keep track of whether it was called.
        skip: ["[async]foo"],
    });

    use super::Component;
    export!(Component);
}

struct Component;

#[unsafe(export_name = "local:local/post-return#[async]foo")]
unsafe extern "C" fn export_foo(ptr: *mut u8, len: usize) -> *mut u8 {
    let result = alloc::alloc(Layout::from_size_align(8, 4).unwrap());
    *result.cast::<*mut u8>() = ptr;
    *result.add(4).cast::<usize>() = len;
    result
}

#[unsafe(export_name = "cabi_post_local:local/post-return#[async]foo")]
unsafe extern "C" fn export_post_return_foo(ptr: *mut u8) {
    let s_ptr = *ptr.cast::<*mut u8>();
    let s_len = *ptr.add(4).cast::<usize>();
    alloc::dealloc(ptr, Layout::from_size_align(8, 4).unwrap());

    *POST_RETURN_VALUE.lock().unwrap() =
        Some(String::from_utf8(Vec::from_raw_parts(s_ptr, s_len, s_len)).unwrap());
}

impl bindings::exports::local::local::post_return::Guest for Component {
    fn get_post_return_value() -> String {
        POST_RETURN_VALUE.lock().unwrap().take().unwrap()
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
