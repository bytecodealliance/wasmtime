#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-callee",
    });
}

use {
    bindings::local::local::continue_,
    test_programs::async_::{CALLBACK_CODE_EXIT, CALLBACK_CODE_YIELD, EVENT_NONE},
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/run")]
unsafe extern "C" {
    #[link_name = "[task-return][async]run"]
    fn task_return_run();
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_run() {
    unreachable!()
}

#[unsafe(export_name = "[async-lift]local:local/run#[async]run")]
unsafe extern "C" fn export_run() -> u32 {
    callback_run(EVENT_NONE, 0, 0)
}

#[unsafe(export_name = "[callback][async-lift]local:local/run#[async]run")]
unsafe extern "C" fn callback_run(event0: u32, _event1: u32, _event2: u32) -> u32 {
    assert_eq!(event0, EVENT_NONE);

    if continue_::get_continue() {
        CALLBACK_CODE_YIELD
    } else {
        task_return_run();
        CALLBACK_CODE_EXIT
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
