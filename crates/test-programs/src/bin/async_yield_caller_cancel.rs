mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "yield-caller",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::run::Guest,
        local::local::{continue_, ready},
    },
    test_programs::async_::{STATUS_RETURNED, STATUS_STARTED, subtask_cancel},
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/run")]
unsafe extern "C" {
    #[link_name = "[async-lower]run"]
    fn run() -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn run() -> u32 {
    unreachable!()
}

struct Component;

impl Guest for Component {
    async fn run() {
        ready::set_ready(true);
        continue_::set_continue(true);

        unsafe {
            let status = run();
            let waitable = status >> 4;
            let status = status & 0xF;
            assert_eq!(status, STATUS_STARTED);

            // Here we assume the following:
            //
            // - Wasmtime will deliver a cancel event to the callee before returning
            // from `subtask_cancel`.
            //
            // - The callee will immediately return as soon as it receives the
            // event.
            let status = subtask_cancel(waitable);
            assert_eq!(status, STATUS_RETURNED);
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
