// This tests callback-less (AKA stackful) async exports.
//
// Testing this case using Rust's LLVM-based toolchain is tricky because, as of
// this writing, LLVM does not produce reentrance-safe code.  Specifically, it
// allocates a single shadow stack for use whenever a program needs to take the
// address of a stack variable, which makes concurrent execution of multiple
// Wasm stacks in the same instance hazardous.
//
// Given the above, we write code directly against the component model ABI
// rather than use `wit-bindgen`, and we carefully avoid use of the shadow stack
// across yield points such as calls to `task.wait` in order to keep the code
// reentrant.

use std::alloc::{self, Layout};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/baz")]
unsafe extern "C" {
    #[link_name = "[task-return]foo"]
    fn task_return_foo(ptr: *mut u8, len: usize);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_foo(_ptr: *mut u8, _len: usize) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/baz")]
unsafe extern "C" {
    #[link_name = "[async]foo"]
    fn import_foo(params: *mut u8, results: *mut u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn import_foo(_params: *mut u8, _results: *mut u8) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "$root")]
unsafe extern "C" {
    #[link_name = "[task-wait]"]
    fn task_wait(results: *mut i32) -> i32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_wait(_results: *mut i32) -> i32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "$root")]
unsafe extern "C" {
    #[link_name = "[subtask-drop]"]
    fn subtask_drop(task: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn subtask_drop(_task: u32) {
    unreachable!()
}

const _STATUS_STARTING: u32 = 0;
const _STATUS_STARTED: u32 = 1;
const _STATUS_RETURNED: u32 = 2;
const STATUS_DONE: u32 = 3;

const _EVENT_CALL_STARTING: i32 = 0;
const _EVENT_CALL_STARTED: i32 = 1;
const _EVENT_CALL_RETURNED: i32 = 2;
const EVENT_CALL_DONE: i32 = 3;

#[unsafe(export_name = "[async-stackful]local:local/baz#foo")]
unsafe extern "C" fn export_foo(ptr: *mut u8, len: usize) {
    // Note that we're careful not to take the address of any stack-allocated
    // value here.  We need to avoid relying on the LLVM-generated shadow stack
    // in order to correctly support reentrancy.  It's okay to call functions
    // which use the shadow stack, as long as they pop everything off before we
    // reach a yield point such as a call to `task.wait`.

    let s = format!(
        "{} - entered guest",
        String::from_utf8(Vec::from_raw_parts(ptr, len, len)).unwrap()
    );

    let layout = Layout::from_size_align(8, 4).unwrap();

    let params = alloc::alloc(layout);
    *params.cast::<*mut u8>() = s.as_ptr().cast_mut();
    *params.add(4).cast::<usize>() = s.len();

    let results = alloc::alloc(layout);

    let result = import_foo(params, results);
    let mut status = result >> 30;
    let call = result & !(0b11 << 30);
    while status != STATUS_DONE {
        // Note the use of `Box` here to avoid taking the address of a stack
        // allocation.
        let payload = Box::into_raw(Box::new([0i32; 2]));
        let event = task_wait(payload.cast());
        let payload = Box::from_raw(payload);
        if event == EVENT_CALL_DONE {
            assert!(call == payload[0] as u32);
            subtask_drop(call);
            status = STATUS_DONE;
        }
    }
    alloc::dealloc(params, layout);

    let len = *results.add(4).cast::<usize>();
    let s = format!(
        "{} - exited guest",
        String::from_utf8(Vec::from_raw_parts(*results.cast::<*mut u8>(), len, len)).unwrap()
    );
    alloc::dealloc(results, layout);

    task_return_foo(s.as_ptr().cast_mut(), s.len());
}

// Copied from `wit-bindgen`-generated output
#[cfg(target_arch = "wasm32")]
#[unsafe(link_section = "component-type:wit-bindgen:0.35.0:local:local:round-trip:encoded world")]
#[doc(hidden)]
#[allow(
    clippy::octal_escapes,
    reason = "this is a machine-generated binary blob"
)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 239] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07o\x01A\x02\x01A\x04\x01\
B\x02\x01@\x01\x01ss\0s\x04\0\x03foo\x01\0\x03\0\x0flocal:local/baz\x05\0\x01B\x02\
\x01@\x01\x01ss\0s\x04\0\x03foo\x01\0\x04\0\x0flocal:local/baz\x05\x01\x04\0\x16\
local:local/round-trip\x04\0\x0b\x10\x01\0\x0around-trip\x03\0\0\0G\x09producers\
\x01\x0cprocessed-by\x02\x0dwit-component\x070.220.0\x10wit-bindgen-rust\x060.35\
.0";

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
