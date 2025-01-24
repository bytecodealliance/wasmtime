mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "poll",
    });

    use super::Component;
    export!(Component);
}

use bindings::{exports::local::local::run::Guest, local::local::ready};

fn task_poll() -> Option<(i32, i32, i32)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "$root")]
        unsafe extern "C" {
            #[link_name = "[task-poll]"]
            fn poll(_: *mut i32) -> i32;
        }
        let mut payload = [0i32; 3];
        if unsafe { poll(payload.as_mut_ptr()) } != 0 {
            Some((payload[0], payload[1], payload[2]))
        } else {
            None
        }
    }
}

fn async_when_ready() -> i32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "local:local/ready")]
        unsafe extern "C" {
            #[link_name = "[async]when-ready"]
            fn call_when_ready(_: *mut u8, _: *mut u8) -> i32;
        }
        unsafe { call_when_ready(std::ptr::null_mut(), std::ptr::null_mut()) }
    }
}

/// Call the `subtask.drop` canonical built-in function.
fn subtask_drop(subtask: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        _ = subtask;
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "$root")]
        unsafe extern "C" {
            #[link_name = "[subtask-drop]"]
            fn subtask_drop(_: u32);
        }
        unsafe {
            subtask_drop(subtask);
        }
    }
}

struct Component;

impl Guest for Component {
    fn run() {
        ready::set_ready(false);

        assert!(task_poll().is_none());

        async_when_ready();

        assert!(task_poll().is_none());

        ready::set_ready(true);

        let Some((3, task, _)) = task_poll() else {
            panic!()
        };

        subtask_drop(task as u32);

        assert!(task_poll().is_none());

        assert!(async_when_ready() == 3 << 30); // STATUS_DONE

        assert!(task_poll().is_none());
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
