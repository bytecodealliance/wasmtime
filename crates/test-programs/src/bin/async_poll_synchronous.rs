mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "poll",
        async: false,
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{exports::local::local::run::Guest, local::local::ready},
    test_programs::async_::{
        EVENT_NONE, EVENT_SUBTASK, STATUS_RETURNED, subtask_drop, waitable_join, waitable_set_drop,
        waitable_set_new, waitable_set_poll,
    },
};

fn async_when_ready() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "local:local/ready")]
        unsafe extern "C" {
            #[link_name = "[async-lower][async]when-ready"]
            fn call_when_ready() -> u32;
        }
        unsafe { call_when_ready() }
    }
}

struct Component;

impl Guest for Component {
    fn run() {
        unsafe {
            ready::set_ready(false);

            let set = waitable_set_new();

            assert_eq!(waitable_set_poll(set), (EVENT_NONE, 0, 0));

            let result = async_when_ready();
            let status = result & 0xf;
            let call = result >> 4;
            assert!(status != STATUS_RETURNED);
            waitable_join(call, set);

            assert_eq!(waitable_set_poll(set), (EVENT_NONE, 0, 0));

            ready::set_ready(true);

            let (event, task, code) = waitable_set_poll(set);
            assert_eq!(event, EVENT_SUBTASK);
            assert_eq!(call, task);
            assert_eq!(code, STATUS_RETURNED);

            subtask_drop(task);

            assert_eq!(waitable_set_poll(set), (EVENT_NONE, 0, 0));

            assert!(async_when_ready() == STATUS_RETURNED);

            assert_eq!(waitable_set_poll(set), (EVENT_NONE, 0, 0));

            waitable_set_drop(set);
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
