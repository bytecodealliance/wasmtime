mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "cancel-callee",
    });
}

use test_programs::async_::{
    CALLBACK_CODE_EXIT, CALLBACK_CODE_WAIT, EVENT_CANCELLED, EVENT_NONE, EVENT_SUBTASK,
    STATUS_RETURN_CANCELLED, STATUS_RETURNED, STATUS_STARTED, context_get, context_set,
    subtask_cancel, subtask_drop, task_cancel, waitable_join, waitable_set_drop, waitable_set_new,
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/yield-with-options")]
unsafe extern "C" {
    #[link_name = "[task-return]yield-times"]
    fn task_return_yield_times();
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_yield_times() {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/yield")]
unsafe extern "C" {
    #[link_name = "yield-times"]
    fn yield_times(_: u64);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe fn yield_times(_: u64) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/yield")]
unsafe extern "C" {
    #[link_name = "[async-lower]yield-times"]
    fn yield_times_async(ms: u64) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe fn yield_times_async(_ms: u64) -> u32 {
    unreachable!()
}

const ON_CANCEL_TASK_RETURN: u8 = 0;
const ON_CANCEL_TASK_CANCEL: u8 = 1;

const _MODE_NORMAL: u8 = 0;
const _MODE_TRAP_CANCEL_GUEST_AFTER_START_CANCELLED: u8 = 1;
const _MODE_TRAP_CANCEL_GUEST_AFTER_RETURN_CANCELLED: u8 = 2;
const _MODE_TRAP_CANCEL_GUEST_AFTER_RETURN: u8 = 3;
const MODE_TRAP_CANCEL_HOST_AFTER_RETURN_CANCELLED: u8 = 4;
const MODE_TRAP_CANCEL_HOST_AFTER_RETURN: u8 = 5;
const MODE_LEAK_TASK_AFTER_CANCEL: u8 = 6;

#[derive(Clone, Copy)]
struct YieldParams {
    times: u64,
    on_cancel: u8,
    on_cancel_delay_times: u64,
    synchronous_delay: bool,
    mode: u8,
}

enum State {
    S0(YieldParams),
    S1 {
        set: u32,
        waitable: u32,
        params: YieldParams,
    },
    S2 {
        set: u32,
        waitable: u32,
        params: YieldParams,
    },
}

#[unsafe(export_name = "local:local/backpressure#set-backpressure")]
unsafe extern "C" fn export_set_backpressure(enabled: bool) {
    if enabled {
        wit_bindgen::backpressure_inc();
    } else {
        wit_bindgen::backpressure_dec();
    }
}

#[unsafe(export_name = "local:local/backpressure#inc-backpressure")]
unsafe extern "C" fn export_inc_backpressure() {
    wit_bindgen::backpressure_inc();
}

#[unsafe(export_name = "local:local/backpressure#dec-backpressure")]
unsafe extern "C" fn export_dec_backpressure() {
    wit_bindgen::backpressure_dec();
}

#[unsafe(export_name = "local:local/yield#yield-times")]
unsafe extern "C" fn export_yield_yield_times(times: u64) {
    unsafe {
        yield_times(times);
    }
}

#[unsafe(export_name = "[async-lift]local:local/yield-with-options#yield-times")]
unsafe extern "C" fn export_yield_with_options_yield_times(
    times: u64,
    on_cancel: u8,
    on_cancel_delay_times: u64,
    synchronous_delay: bool,
    mode: u8,
) -> u32 {
    unsafe {
        context_set(
            u32::try_from(Box::into_raw(Box::new(State::S0(YieldParams {
                times,
                on_cancel,
                on_cancel_delay_times,
                synchronous_delay,
                mode,
            }))) as usize)
            .unwrap(),
        );
        callback_yield_with_options_yield_times(EVENT_NONE, 0, 0)
    }
}

#[unsafe(export_name = "[callback][async-lift]local:local/yield-with-options#yield-times")]
unsafe extern "C" fn callback_yield_with_options_yield_times(
    event0: u32,
    event1: u32,
    event2: u32,
) -> u32 {
    unsafe {
        let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
        match state {
            State::S0(params) => {
                assert_eq!(event0, EVENT_NONE);

                let status = yield_times_async(params.times);

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let set = waitable_set_new();
                waitable_join(waitable, set);

                *state = State::S1 {
                    set,
                    waitable,
                    params: *params,
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S1 {
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_CANCELLED);

                let result = subtask_cancel(*waitable);

                assert_eq!(result, STATUS_RETURN_CANCELLED);

                if params.mode == MODE_TRAP_CANCEL_HOST_AFTER_RETURN_CANCELLED {
                    // This should trap, since `waitable` has already been
                    // cancelled:
                    subtask_cancel(*waitable);
                    unreachable!()
                }

                waitable_join(*waitable, 0);

                if params.mode != MODE_LEAK_TASK_AFTER_CANCEL {
                    subtask_drop(*waitable);
                }

                if params.on_cancel_delay_times == 0 {
                    match params.on_cancel {
                        ON_CANCEL_TASK_RETURN => task_return_yield_times(),
                        ON_CANCEL_TASK_CANCEL => task_cancel(),
                        _ => unreachable!(),
                    }

                    CALLBACK_CODE_EXIT
                } else if params.synchronous_delay {
                    yield_times(params.on_cancel_delay_times);

                    match params.on_cancel {
                        ON_CANCEL_TASK_RETURN => task_return_yield_times(),
                        ON_CANCEL_TASK_CANCEL => task_cancel(),
                        _ => unreachable!(),
                    }

                    CALLBACK_CODE_EXIT
                } else {
                    let status = yield_times_async(params.on_cancel_delay_times);

                    let waitable = status >> 4;
                    let status = status & 0xF;

                    assert_eq!(status, STATUS_STARTED);

                    waitable_join(waitable, *set);

                    let set = *set;

                    *state = State::S2 {
                        set,
                        waitable,
                        params: *params,
                    };

                    CALLBACK_CODE_WAIT | (set << 4)
                }
            }

            State::S2 {
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *waitable);
                assert_eq!(event2, STATUS_RETURNED);

                if params.mode == MODE_TRAP_CANCEL_HOST_AFTER_RETURN {
                    // This should trap, since `waitable` has already returned:
                    subtask_cancel(*waitable);
                    unreachable!()
                }

                waitable_join(*waitable, 0);
                subtask_drop(*waitable);
                waitable_set_drop(*set);

                match params.on_cancel {
                    ON_CANCEL_TASK_RETURN => task_return_yield_times(),
                    ON_CANCEL_TASK_CANCEL => task_cancel(),
                    _ => unreachable!(),
                }

                CALLBACK_CODE_EXIT
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
