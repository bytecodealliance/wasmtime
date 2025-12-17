mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "readiness-guest",
    });
}

use {
    std::{mem, ptr},
    test_programs::async_::{
        BLOCKED, CALLBACK_CODE_EXIT, CALLBACK_CODE_WAIT, DROPPED, EVENT_NONE, EVENT_STREAM_READ,
        EVENT_STREAM_WRITE, context_get, context_set, waitable_join, waitable_set_drop,
        waitable_set_new,
    },
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[task-return]start"]
    fn task_return_start(_: u32, _: *const u8, _: usize);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_start(_: u32, _: *const u8, _: usize) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[stream-new-0]start"]
    fn stream_new() -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_new() -> u64 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[async-lower][stream-write-0]start"]
    fn stream_write(_: u32, _: *const u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_write(_: u32, _: *const u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[async-lower][stream-read-0]start"]
    fn stream_read(_: u32, _: *mut u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_read(_: u32, _: *mut u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[stream-drop-readable-0]start"]
    fn stream_drop_readable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_readable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/readiness")]
unsafe extern "C" {
    #[link_name = "[stream-drop-writable-0]start"]
    fn stream_drop_writable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_writable(_: u32) {
    unreachable!()
}

static BYTES_TO_WRITE: &[u8] = &[1, 3, 5, 7, 11];

enum State {
    S0 {
        rx: u32,
        expected: Vec<u8>,
    },
    S1 {
        set: u32,
        tx: Option<u32>,
        rx: Option<u32>,
        expected: Vec<u8>,
    },
}

#[unsafe(export_name = "[async-lift]local:local/readiness#start")]
unsafe extern "C" fn export_start(rx: u32, expected: u32, expected_len: u32) -> u32 {
    let expected_len = usize::try_from(expected_len).unwrap();

    unsafe {
        context_set(
            u32::try_from(Box::into_raw(Box::new(State::S0 {
                rx,
                expected: Vec::from_raw_parts(
                    expected as usize as *mut u8,
                    expected_len,
                    expected_len,
                ),
            })) as usize)
            .unwrap(),
        );

        callback_start(EVENT_NONE, 0, 0)
    }
}

#[unsafe(export_name = "[callback][async-lift]local:local/readiness#start")]
unsafe extern "C" fn callback_start(event0: u32, event1: u32, event2: u32) -> u32 {
    unsafe {
        let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
        match state {
            State::S0 { rx, expected } => {
                assert_eq!(event0, EVENT_NONE);

                // Do a zero-length read to wait until the writer is ready.
                //
                // Here we assume specific behavior from the writer, namely:
                //
                // - It is not immediately ready to send us anything.
                //
                // - When it _is_ ready, it will send us all the bytes it told us to
                // expect at once.
                let status = stream_read(*rx, ptr::null_mut(), 0);
                assert_eq!(status, BLOCKED);

                let set = waitable_set_new();

                waitable_join(*rx, set);

                let tx = {
                    let pair = stream_new();
                    let tx = u32::try_from(pair >> 32).unwrap();
                    let rx = u32::try_from(pair & 0xFFFFFFFF_u64).unwrap();

                    // Do a zero-length write to wait until the reader is ready.
                    //
                    // Here we assume specific behavior from the reader, namely:
                    //
                    // - It is not immediately ready to receive anything (indeed, it
                    // can't possibly be ready given that we haven't returned the
                    // read handle to it yet).
                    //
                    // - When it _is_ ready, it will accept all the bytes we told it
                    // to expect at once.
                    let status = stream_write(tx, ptr::null(), 0);
                    assert_eq!(status, BLOCKED);

                    waitable_join(tx, set);

                    task_return_start(rx, BYTES_TO_WRITE.as_ptr(), BYTES_TO_WRITE.len());

                    tx
                };

                *state = State::S1 {
                    set,
                    tx: Some(tx),
                    rx: Some(*rx),
                    expected: mem::take(expected),
                };

                CALLBACK_CODE_WAIT | (set << 4)
            }

            State::S1 {
                set,
                tx,
                rx,
                expected,
            } => {
                if event0 == EVENT_STREAM_READ {
                    let rx = rx.take().unwrap();
                    assert_eq!(event1, rx);
                    assert_eq!(event2, 0);

                    // The writer is ready now, so this read should not block.
                    //
                    // As noted above, we rely on the writer sending us all the
                    // expected bytes at once.
                    let received = &mut vec![0_u8; expected.len()];
                    let status = stream_read(rx, received.as_mut_ptr(), received.len());
                    assert_eq!(
                        status,
                        DROPPED | u32::try_from(received.len() << 4).unwrap()
                    );
                    assert_eq!(received, expected);

                    waitable_join(rx, 0);
                    stream_drop_readable(rx);

                    if tx.is_none() {
                        waitable_set_drop(*set);

                        CALLBACK_CODE_EXIT
                    } else {
                        CALLBACK_CODE_WAIT | (*set << 4)
                    }
                } else if event0 == EVENT_STREAM_WRITE {
                    let tx = tx.take().unwrap();
                    assert_eq!(event1, tx);
                    assert_eq!(event2, 0);

                    // The reader is ready now, so this write should not block.
                    //
                    // As noted above, we rely on the reader accepting all the
                    // expected bytes at once.
                    let status = stream_write(tx, BYTES_TO_WRITE.as_ptr(), BYTES_TO_WRITE.len());
                    assert_eq!(
                        status,
                        DROPPED | u32::try_from(BYTES_TO_WRITE.len() << 4).unwrap()
                    );

                    waitable_join(tx, 0);
                    stream_drop_writable(tx);

                    if rx.is_none() {
                        waitable_set_drop(*set);

                        CALLBACK_CODE_EXIT
                    } else {
                        CALLBACK_CODE_WAIT | (*set << 4)
                    }
                } else {
                    unreachable!()
                }
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
