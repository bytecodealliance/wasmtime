mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "cancel-caller",
    });
}

use test_programs::async_::{
    BLOCKED, CALLBACK_CODE_EXIT, CALLBACK_CODE_WAIT, EVENT_NONE, EVENT_SUBTASK,
    STATUS_RETURN_CANCELLED, STATUS_RETURNED, STATUS_START_CANCELLED, STATUS_STARTED,
    STATUS_STARTING, context_get, context_set, subtask_cancel, subtask_cancel_async, subtask_drop,
    waitable_join, waitable_set_drop, waitable_set_new,
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel")]
unsafe extern "C" {
    #[link_name = "[task-return]run"]
    fn task_return_run();
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_run() {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "local:local/backpressure")]
unsafe extern "C" {
    #[link_name = "inc-backpressure"]
    fn inc_backpressure();
    #[link_name = "dec-backpressure"]
    fn dec_backpressure();
}
#[cfg(not(target_arch = "wasm32"))]
unsafe fn inc_backpressure() {
    unreachable!()
}
#[cfg(not(target_arch = "wasm32"))]
unsafe fn dec_backpressure() {
    unreachable!()
}

mod yield_ {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/yield")]
    unsafe extern "C" {
        #[link_name = "[async-lower]yield-times"]
        pub fn yield_times(_: u64) -> u32;
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn yield_times(_: u64) -> u32 {
        unreachable!()
    }
}

mod yield_with_options {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/yield-with-options")]
    unsafe extern "C" {
        #[link_name = "[async-lower]yield-times"]
        pub fn yield_times(_: *mut u8) -> u32;
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub unsafe fn yield_times(_: *mut u8) -> u32 {
        unreachable!()
    }
}

const ON_CANCEL_TASK_RETURN: u8 = 0;
const ON_CANCEL_TASK_CANCEL: u8 = 1;

const _MODE_NORMAL: u8 = 0;
const MODE_TRAP_CANCEL_GUEST_AFTER_START_CANCELLED: u8 = 1;
const MODE_TRAP_CANCEL_GUEST_AFTER_RETURN_CANCELLED: u8 = 2;
const MODE_TRAP_CANCEL_GUEST_AFTER_RETURN: u8 = 3;
const _MODE_TRAP_CANCEL_HOST_AFTER_RETURN_CANCELLED: u8 = 4;
const _MODE_TRAP_CANCEL_HOST_AFTER_RETURN: u8 = 5;

#[repr(C)]
struct YieldParams {
    times: u64,
    on_cancel: u8,
    on_cancel_delay_times: u64,
    synchronous_delay: bool,
    mode: u8,
}

enum State {
    S0 {
        mode: u8,
        cancel_delay_times: u64,
    },
    S1 {
        mode: u8,
        set: u32,
        waitable: u32,
        params: *mut YieldParams,
    },
    S2 {
        mode: u8,
        set: u32,
        waitable: u32,
        params: *mut YieldParams,
    },
    S3 {
        set: u32,
        waitable: u32,
        params: *mut YieldParams,
    },
    S4 {
        set: u32,
        waitable: u32,
        params: *mut YieldParams,
    },
}

#[unsafe(export_name = "[async-lift]local:local/cancel#run")]
unsafe extern "C" fn export_run(mode: u8, cancel_delay_times: u64) -> u32 {
    unsafe {
        context_set(
            u32::try_from(Box::into_raw(Box::new(State::S0 {
                mode,
                cancel_delay_times,
            })) as usize)
            .unwrap(),
        );
        callback_run(EVENT_NONE, 0, 0)
    }
}

#[unsafe(export_name = "[callback][async-lift]local:local/cancel#run")]
unsafe extern "C" fn callback_run(event0: u32, event1: u32, event2: u32) -> u32 {
    unsafe {
        let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
        match state {
            State::S0 {
                mode,
                cancel_delay_times,
            } => {
                assert_eq!(event0, EVENT_NONE);

                // First, call and cancel `yield_with_options::yield_tiems`
                // with backpressure enabled.  Cancelling should not block since
                // the call will not even have started.

                inc_backpressure();

                let params = Box::into_raw(Box::new(YieldParams {
                    times: 60 * 60 * 1000,
                    on_cancel: ON_CANCEL_TASK_CANCEL,
                    on_cancel_delay_times: 0,
                    synchronous_delay: false,
                    mode: *mode,
                }));

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTING);

                let result = subtask_cancel_async(waitable);

                assert_eq!(result, STATUS_START_CANCELLED);

                if *mode == MODE_TRAP_CANCEL_GUEST_AFTER_START_CANCELLED {
                    // This should trap, since `waitable` has already been
                    // cancelled:
                    subtask_cancel_async(waitable);
                    unreachable!()
                }

                subtask_drop(waitable);

                // Next, call and cancel `yield_with_options::yield_times` with
                // backpressure disabled.  Cancelling should not block since we
                // specified zero cancel delay to the callee.

                dec_backpressure();

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel_async(waitable);

                assert_eq!(result, STATUS_RETURN_CANCELLED);

                if *mode == MODE_TRAP_CANCEL_GUEST_AFTER_RETURN_CANCELLED {
                    // This should trap, since `waitable` has already been
                    // cancelled:
                    subtask_cancel_async(waitable);
                    unreachable!()
                }

                subtask_drop(waitable);

                // Next, call and cancel `yield_with_options::yieldtimes` with
                // a non-zero cancel delay.  Cancelling _should_ block this
                // time.

                (*params).on_cancel_delay_times = *cancel_delay_times;

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel_async(waitable);

                assert_eq!(result, BLOCKED);

                let set = waitable_set_new();
                waitable_join(waitable, set);

                *state = State::S1 {
                    mode: *mode,
                    set,
                    waitable,
                    params,
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S1 {
                mode,
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *waitable);
                assert_eq!(event2, STATUS_RETURN_CANCELLED);

                waitable_join(*waitable, 0);
                subtask_drop(*waitable);

                // Next, call and cancel `yield_with_options::yield_times` with
                // a non-zero cancel delay, but this time specifying that the
                // callee should call `task.return` instead of `task.cancel`.
                // Cancelling _should_ block this time.

                (**params).on_cancel = ON_CANCEL_TASK_RETURN;

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel_async(waitable);

                assert_eq!(result, BLOCKED);

                waitable_join(waitable, *set);

                let set = *set;

                *state = State::S2 {
                    mode: *mode,
                    set,
                    waitable,
                    params: *params,
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S2 {
                mode,
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *waitable);
                assert_eq!(event2, STATUS_RETURNED);

                if *mode == MODE_TRAP_CANCEL_GUEST_AFTER_RETURN {
                    // This should trap, since `waitable` has already returned:
                    subtask_cancel_async(*waitable);
                    unreachable!()
                }

                waitable_join(*waitable, 0);
                subtask_drop(*waitable);

                // Next, call and cancel `yield_with_options::yield_times` with
                // a non-zero cancel delay, and specify that the callee should
                // delay the cancel by making a synchronous call.

                (**params).on_cancel = ON_CANCEL_TASK_CANCEL;
                (**params).synchronous_delay = true;

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel_async(waitable);

                // NB: As of this writing, Wasmtime spawns a new fiber for
                // async->async guest calls, which means the above call should
                // block asynchronously, giving us back control.  However, the
                // runtime could alternatively execute the call on the original
                // fiber, in which case the above call would block synchronously
                // and return `STATUS_RETURN_CANCELLED`.  If Wasmtime's behavior
                // changes, this test will need to be modified.
                assert_eq!(result, BLOCKED);

                waitable_join(waitable, *set);

                let set = *set;

                *state = State::S3 {
                    set,
                    waitable,
                    params: *params,
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S3 {
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *waitable);
                assert_eq!(event2, STATUS_RETURN_CANCELLED);

                waitable_join(*waitable, 0);
                subtask_drop(*waitable);

                // Next, call and cancel `yield_::yield_times`, which the callee
                // implements using both an synchronous lift and asynchronous
                // lower.  This should block asynchronously and yield a
                // `STATUS_RETURNED` when complete since the callee cannot
                // actually be cancelled.

                let status = yield_::yield_times(10);

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel_async(waitable);

                assert_eq!(result, BLOCKED);

                waitable_join(waitable, *set);

                let set = *set;

                *state = State::S4 {
                    set,
                    waitable,
                    params: *params,
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S4 {
                set,
                waitable,
                params,
            } => {
                assert_eq!(event0, EVENT_SUBTASK);
                assert_eq!(event1, *waitable);
                assert_eq!(event2, STATUS_RETURNED);

                waitable_join(*waitable, 0);
                subtask_drop(*waitable);
                waitable_set_drop(*set);

                // Next, call and cancel `yield_with_options::yield_times` with
                // a non-zero cancel delay, and specify that the callee should
                // delay the cancel by making a synchronous call.  Here we make
                // synchronous call to `subtask.cancel`, which should block
                // synchronously.

                (**params).synchronous_delay = true;

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel(waitable);

                assert_eq!(result, STATUS_RETURN_CANCELLED);

                waitable_join(waitable, 0);
                subtask_drop(waitable);

                // Finally, do the same as above, except specify that the callee
                // should delay the cancel asynchronously.

                (**params).synchronous_delay = false;

                let status = yield_with_options::yield_times(params.cast());

                let waitable = status >> 4;
                let status = status & 0xF;

                assert_eq!(status, STATUS_STARTED);

                let result = subtask_cancel(waitable);

                assert_eq!(result, STATUS_RETURN_CANCELLED);

                waitable_join(waitable, 0);
                subtask_drop(waitable);
                drop(Box::from_raw(*params));

                task_return_run();

                CALLBACK_CODE_EXIT
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
