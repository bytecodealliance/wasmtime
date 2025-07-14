mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "intertask-communication",
    });
}

use {
    std::sync::atomic::{AtomicU32, Ordering::Relaxed},
    test_programs::async_::{
        BLOCKED, CALLBACK_CODE_EXIT, CALLBACK_CODE_WAIT, COMPLETED, EVENT_FUTURE_WRITE, EVENT_NONE,
        context_get, context_set, waitable_join, waitable_set_drop, waitable_set_new,
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

fn future_new() -> (u32, u32) {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/intertask")]
    unsafe extern "C" {
        #[link_name = "[future-new-0]foo"]
        fn future_new() -> u64;
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn future_new() -> u64 {
        unreachable!()
    }

    let pair = unsafe { future_new() };
    (
        (pair >> 32).try_into().unwrap(),
        (pair & 0xFFFFFFFF_u64).try_into().unwrap(),
    )
}

fn future_write(writer: u32) -> u32 {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/intertask")]
    unsafe extern "C" {
        #[link_name = "[async-lower][future-write-0]foo"]
        fn future_write(_: u32, _: u32) -> u32;
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn future_write(_: u32, _: u32) -> u32 {
        unreachable!()
    }

    unsafe { future_write(writer, 0) }
}

fn future_read(reader: u32) -> u32 {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/intertask")]
    unsafe extern "C" {
        #[link_name = "[async-lower][future-read-0]foo"]
        fn future_read(_: u32, _: u32) -> u32;
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn future_read(_: u32, _: u32) -> u32 {
        unreachable!()
    }

    unsafe { future_read(reader, 0) }
}

fn future_drop_readable(reader: u32) {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/intertask")]
    unsafe extern "C" {
        #[link_name = "[future-drop-readable-0]foo"]
        fn future_drop_readable(_: u32);
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn future_drop_readable(_: u32) {
        unreachable!()
    }

    unsafe { future_drop_readable(reader) }
}

fn future_drop_writable(writer: u32) {
    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "local:local/intertask")]
    unsafe extern "C" {
        #[link_name = "[future-drop-writable-0]foo"]
        fn future_drop_writable(_: u32);
    }
    #[cfg(not(target_arch = "wasm32"))]
    unsafe extern "C" fn future_drop_writable(_: u32) {
        unreachable!()
    }

    unsafe { future_drop_writable(writer) }
}

static TASK_NUMBER: AtomicU32 = AtomicU32::new(0);
static SET: AtomicU32 = AtomicU32::new(0);

enum State {
    S0 { number: u32 },
    S1 { set: u32 },
}

#[unsafe(export_name = "[async-lift]local:local/run#[async]run")]
unsafe extern "C" fn export_run() -> u32 {
    unsafe {
        context_set(
            u32::try_from(Box::into_raw(Box::new(State::S0 {
                number: TASK_NUMBER.fetch_add(1, Relaxed),
            })) as usize)
            .unwrap(),
        );
        callback_run(EVENT_NONE, 0, 0)
    }
}

#[unsafe(export_name = "[callback][async-lift]local:local/run#[async]run")]
unsafe extern "C" fn callback_run(event0: u32, event1: u32, event2: u32) -> u32 {
    unsafe {
        let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
        match state {
            State::S0 { number } => {
                assert_eq!(event0, EVENT_NONE);

                match *number {
                    0 => {
                        // Create a new waitable-set, store it for the other task to
                        // find, then return `CALLBACK_CODE_WAIT` to wait on it.
                        // This would lead to an infinite wait, except that the
                        // other task will add to the waitable-set after this one
                        // has started waiting and then trigger an event to wake it
                        // up.
                        let set = waitable_set_new();

                        let old = SET.swap(set, Relaxed);
                        assert_eq!(old, 0);

                        *state = State::S1 { set };

                        CALLBACK_CODE_WAIT | (set << 4)
                    }
                    1 => {
                        // Retrieve the waitable-set our peer task is waiting on,
                        // create a future, write to write end, add the write end to
                        // the waitable-set, then read from the read end.  The read
                        // should trigger an event on the write end, waking up the
                        // peer task.
                        let set = SET.swap(0, Relaxed);
                        assert_ne!(set, 0);

                        let (tx, rx) = future_new();
                        let status = future_write(tx);
                        assert_eq!(status, BLOCKED);

                        waitable_join(tx, set);

                        let status = future_read(rx);
                        assert_eq!(status, COMPLETED); // i.e. one element was read

                        future_drop_readable(rx);

                        task_return_run();
                        CALLBACK_CODE_EXIT
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }

            State::S1 { set } => {
                assert_eq!(event0, EVENT_FUTURE_WRITE);
                assert_eq!(event2, COMPLETED); // i.e. one element was written

                waitable_join(event1, 0);
                waitable_set_drop(*set);
                future_drop_writable(event1);

                TASK_NUMBER.store(0, Relaxed);

                task_return_run();
                CALLBACK_CODE_EXIT
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
