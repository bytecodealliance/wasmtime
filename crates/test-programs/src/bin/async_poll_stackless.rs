#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "poll",
    });
}

use {
    bindings::local::local::ready,
    test_programs::async_::{
        CALLBACK_CODE_EXIT, CALLBACK_CODE_POLL, EVENT_NONE, EVENT_SUBTASK, STATUS_RETURNED,
        context_get, context_set, subtask_drop, waitable_join, waitable_set_drop, waitable_set_new,
    },
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

enum State {
    S0,
    S1 { set: u32 },
    S2 { set: u32, call: u32 },
    S3 { set: u32, call: u32 },
    S4 { set: u32 },
    S5 { set: u32 },
}

#[unsafe(export_name = "[async-lift]local:local/run#[async]run")]
unsafe extern "C" fn export_run() -> u32 {
    context_set(u32::try_from(Box::into_raw(Box::new(State::S0)) as usize).unwrap());
    callback_run(EVENT_NONE, 0, 0)
}

#[unsafe(export_name = "[callback][async-lift]local:local/run#[async]run")]
unsafe extern "C" fn callback_run(event0: u32, event1: u32, event2: u32) -> u32 {
    let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
    match state {
        State::S0 => {
            assert_eq!(event0, EVENT_NONE);

            ready::set_ready(false);

            let set = waitable_set_new();

            *state = State::S1 { set };

            CALLBACK_CODE_POLL | (set << 4)
        }

        State::S1 { set } => {
            assert_eq!(event0, EVENT_NONE);

            let set = *set;
            let result = async_when_ready();
            let status = result & 0xf;
            let call = result >> 4;
            assert!(status != STATUS_RETURNED);
            waitable_join(call, set);

            *state = State::S2 { set, call };

            CALLBACK_CODE_POLL | (set << 4)
        }

        State::S2 { set, call } => {
            assert_eq!(event0, EVENT_NONE);

            let set = *set;
            let call = *call;
            ready::set_ready(true);

            *state = State::S3 { set, call };

            CALLBACK_CODE_POLL | (set << 4)
        }

        State::S3 { set, call } => {
            let set = *set;

            if event0 != EVENT_NONE {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *call);
                assert_eq!(event2, STATUS_RETURNED);

                subtask_drop(*call);

                *state = State::S4 { set };
            }

            CALLBACK_CODE_POLL | (set << 4)
        }

        State::S4 { set } => {
            assert_eq!(event0, EVENT_NONE);

            let set = *set;
            assert!(async_when_ready() == STATUS_RETURNED);

            *state = State::S5 { set };

            CALLBACK_CODE_POLL | (set << 4)
        }

        State::S5 { set } => {
            assert_eq!(event0, EVENT_NONE);

            waitable_set_drop(*set);

            drop(Box::from_raw(state));

            context_set(0);

            task_return_run();

            CALLBACK_CODE_EXIT
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
