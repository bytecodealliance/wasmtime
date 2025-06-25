#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

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
// across yield points such as calls to `waitable-set.wait` in order to keep the
// code reentrant.

mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "round-trip-many",
    });
}

use {
    std::alloc::{self, Layout},
    test_programs::async_::{
        EVENT_SUBTASK, STATUS_RETURNED, subtask_drop, waitable_join, waitable_set_drop,
        waitable_set_new, waitable_set_wait,
    },
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/many")]
unsafe extern "C" {
    #[link_name = "[task-return][async]foo"]
    fn task_return_foo(ptr: *mut u8);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_foo(_ptr: *mut u8) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/many")]
unsafe extern "C" {
    #[link_name = "[async-lower][async]foo"]
    fn import_foo(params: *mut u8, results: *mut u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn import_foo(_params: *mut u8, _results: *mut u8) -> u32 {
    unreachable!()
}

#[unsafe(export_name = "[async-lift-stackful]local:local/many#[async]foo")]
unsafe extern "C" fn export_foo(args: *mut u8) {
    // Note that we're careful not to take the address of any stack-allocated
    // value here.  We need to avoid relying on the LLVM-generated shadow stack
    // in order to correctly support reentrancy.  It's okay to call functions
    // which use the shadow stack, as long as they pop everything off before we
    // reach a yield point such as a call to `waitable-set.wait`.

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
    let mut status = result & 0xf;
    let call = result >> 4;
    let set = waitable_set_new();
    if call != 0 {
        waitable_join(call, set);
    }
    while status != STATUS_RETURNED {
        // Note the use of `Box` here to avoid taking the address of a stack
        // allocation.
        let payload = Box::into_raw(Box::new([0i32; 2]));
        let event = waitable_set_wait(set, payload.cast());
        let payload = Box::from_raw(payload);
        if event == EVENT_SUBTASK {
            assert_eq!(call, payload[0] as u32);
            status = payload[1] as u32;
            if status == STATUS_RETURNED {
                subtask_drop(call);
                waitable_set_drop(set);
            }
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

// Unused function; required since this file is built as a `bin`:
fn main() {}
