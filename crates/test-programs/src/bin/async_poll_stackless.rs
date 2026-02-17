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
        CALLBACK_CODE_EXIT, CALLBACK_CODE_YIELD, EVENT_NONE, EVENT_SUBTASK, STATUS_RETURNED,
        context_get, context_set, subtask_drop, waitable_join, waitable_set_drop, waitable_set_new,
        waitable_set_poll,
    },
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/run")]
unsafe extern "C" {
    #[link_name = "[task-return]run"]
    fn task_return_run();
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_run() {
    unreachable!()
}

fn async_when_ready(handle: u32) -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        _ = handle;
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "local:local/ready")]
        unsafe extern "C" {
            #[link_name = "[async-lower][method]thing.when-ready"]
            fn call_when_ready(handle: u32) -> u32;
        }
        unsafe { call_when_ready(handle) }
    }
}

enum State {
    S0,
    S1 {
        thing: Option<ready::Thing>,
        set: u32,
    },
    S2 {
        thing: Option<ready::Thing>,
        set: u32,
        call: u32,
    },
    S3 {
        thing: Option<ready::Thing>,
        set: u32,
        call: u32,
    },
    S4 {
        thing: Option<ready::Thing>,
        set: u32,
    },
    S5 {
        set: u32,
    },
}

#[unsafe(export_name = "[async-lift]local:local/run#run")]
unsafe extern "C" fn export_run() -> u32 {
    context_set(u32::try_from(Box::into_raw(Box::new(State::S0)) as usize).unwrap());
    callback_run(EVENT_NONE, 0, 0)
}

#[unsafe(export_name = "[callback][async-lift]local:local/run#run")]
unsafe extern "C" fn callback_run(event0: u32, _: u32, _: u32) -> u32 {
    let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
    match state {
        State::S0 => {
            assert_eq!(event0, EVENT_NONE);

            let thing = ready::Thing::new();
            thing.set_ready(false);

            let set = waitable_set_new();

            *state = State::S1 {
                thing: Some(thing),
                set,
            };

            CALLBACK_CODE_YIELD
        }

        &mut State::S1 { ref mut thing, set } => {
            let thing = thing.take().unwrap();
            let (event0, _, _) = waitable_set_poll(set);

            assert_eq!(event0, EVENT_NONE);

            let result = async_when_ready(thing.handle());
            let status = result & 0xf;
            let call = result >> 4;
            assert!(status != STATUS_RETURNED);
            waitable_join(call, set);

            *state = State::S2 {
                thing: Some(thing),
                set,
                call,
            };

            CALLBACK_CODE_YIELD
        }

        &mut State::S2 {
            ref mut thing,
            set,
            call,
        } => {
            let thing = thing.take().unwrap();
            let (event0, _, _) = waitable_set_poll(set);

            assert_eq!(event0, EVENT_NONE);

            thing.set_ready(true);

            *state = State::S3 {
                thing: Some(thing),
                set,
                call,
            };

            CALLBACK_CODE_YIELD
        }

        &mut State::S3 {
            ref mut thing,
            set,
            call,
        } => {
            let (event0, event1, event2) = waitable_set_poll(set);

            if event0 != EVENT_NONE {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, call);
                assert_eq!(event2, STATUS_RETURNED);

                subtask_drop(call);

                *state = State::S4 {
                    thing: thing.take(),
                    set,
                };
            }

            CALLBACK_CODE_YIELD
        }

        &mut State::S4 { ref mut thing, set } => {
            let thing = thing.take().unwrap();
            let (event0, _, _) = waitable_set_poll(set);

            assert_eq!(event0, EVENT_NONE);

            assert_eq!(async_when_ready(thing.handle()), STATUS_RETURNED);

            *state = State::S5 { set };

            CALLBACK_CODE_YIELD
        }

        &mut State::S5 { set } => {
            let (event0, _, _) = waitable_set_poll(set);

            assert_eq!(event0, EVENT_NONE);

            waitable_set_drop(set);

            drop(Box::from_raw(state));

            context_set(0);

            task_return_run();

            CALLBACK_CODE_EXIT
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
