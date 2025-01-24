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
#[link(wasm_import_module = "[export]local:local/many")]
unsafe extern "C" {
    #[link_name = "[task-return]foo"]
    fn task_return_foo(ptr: *mut u8);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_foo(_ptr: *mut u8) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/many")]
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

#[unsafe(export_name = "[async-stackful]local:local/many#foo")]
unsafe extern "C" fn export_foo(args: *mut u8) {
    // Note that we're careful not to take the address of any stack-allocated
    // value here.  We need to avoid relying on the LLVM-generated shadow stack
    // in order to correctly support reentrancy.  It's okay to call functions
    // which use the shadow stack, as long as they pop everything off before we
    // reach a yield point such as a call to `task.wait`.

    // type                               | size | align | offset
    // ----------------------------------------------------------
    // string                             |    8 |     4 |      0
    // u32                                |    4 |     4 |      8
    // list<u8>                           |    8 |     4 |     12
    // tuple<u64, u64>                    |   16 |     8 |     24
    // tuple<list<u8>, bool, u64>         |   24 |     8 |     40
    // option<tuple<list<u8>, bool, u64>> |   32 |     8 |     64
    // result<tuple<list<u8>, bool, u64>> |   32 |     8 |     96
    // ----------------------------------------------------------
    // total                              |  128 |     8 |

    let len = *args.add(4).cast::<usize>();
    let s = format!(
        "{} - entered guest",
        String::from_utf8(Vec::from_raw_parts(*args.cast::<*mut u8>(), len, len)).unwrap()
    );

    let layout = Layout::from_size_align(128, 8).unwrap();

    let params = alloc::alloc(layout);
    *params.cast::<*mut u8>() = s.as_ptr().cast_mut();
    *params.add(4).cast::<usize>() = s.len();
    params.add(8).copy_from(args.add(8), 120);

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
    *results.cast::<*mut u8>() = s.as_ptr().cast_mut();
    *results.add(4).cast::<usize>() = s.len();

    task_return_foo(results);
}

// Copied from `wit-bindgen`-generated output
#[cfg(target_arch = "wasm32")]
#[unsafe(link_section = "component-type:wit-bindgen:0.35.0:local:local:round-trip:encoded world")]
#[doc(hidden)]
#[allow(
    clippy::octal_escapes,
    reason = "this is a machine-generated binary blob"
)]
pub static __WIT_BINDGEN_COMPONENT_TYPE: [u8; 392] = *b"\
\0asm\x0d\0\x01\0\0\x19\x16wit-component-encoding\x04\0\x07\x82\x02\x01A\x02\x01\
A\x04\x01B\x0a\x01pz\x01r\x03\x01a\0\x01b\x7f\x01cw\x04\0\x05stuff\x03\0\x01\x01\
p}\x01o\x02ww\x01k\x02\x01j\x01\x02\0\x01o\x07sy\x03\x04\x02\x05\x06\x01@\x07\x01\
as\x01by\x01c\x03\x01d\x04\x01e\x02\x01f\x05\x01g\x06\0\x07\x04\0\x03foo\x01\x08\
\x03\0\x10local:local/many\x05\0\x01B\x0a\x01pz\x01r\x03\x01a\0\x01b\x7f\x01cw\x04\
\0\x05stuff\x03\0\x01\x01p}\x01o\x02ww\x01k\x02\x01j\x01\x02\0\x01o\x07sy\x03\x04\
\x02\x05\x06\x01@\x07\x01as\x01by\x01c\x03\x01d\x04\x01e\x02\x01f\x05\x01g\x06\0\
\x07\x04\0\x03foo\x01\x08\x04\0\x10local:local/many\x05\x01\x04\0\x1blocal:local\
/round-trip-many\x04\0\x0b\x15\x01\0\x0fround-trip-many\x03\0\0\0G\x09producers\x01\
\x0cprocessed-by\x02\x0dwit-component\x070.224.0\x10wit-bindgen-rust\x060.38.0";

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
