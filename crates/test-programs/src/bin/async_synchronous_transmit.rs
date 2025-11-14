mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "synchronous-transmit-guest",
    });
}

use {
    std::mem,
    test_programs::async_::{
        CALLBACK_CODE_EXIT, CALLBACK_CODE_YIELD, COMPLETED, DROPPED, EVENT_NONE, context_get,
        context_set,
    },
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[task-return]start"]
    fn task_return_start(_: u32, _: *const u8, _: usize, _: u32, _: u8);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_start(_: u32, _: *const u8, _: usize, _: u32, _: u8) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-new-0]start"]
    fn stream_new() -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_new() -> u64 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-write-0]start"]
    fn stream_write(_: u32, _: *const u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_write(_: u32, _: *const u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-read-0]start"]
    fn stream_read(_: u32, _: *mut u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_read(_: u32, _: *mut u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-drop-readable-0]start"]
    fn stream_drop_readable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_readable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-drop-writable-0]start"]
    fn stream_drop_writable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_writable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[future-new-1]start"]
    fn future_new() -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_new() -> u64 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[future-write-1]start"]
    fn future_write(_: u32, _: *const u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_write(_: u32, _: *const u8) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[future-read-1]start"]
    fn future_read(_: u32, _: *mut u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_read(_: u32, _: *mut u8) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[future-drop-readable-1]start"]
    fn future_drop_readable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_drop_readable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/synchronous-transmit")]
unsafe extern "C" {
    #[link_name = "[future-drop-writable-1]start"]
    fn future_drop_writable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_drop_writable(_: u32) {
    unreachable!()
}

static STREAM_BYTES_TO_WRITE: &[u8] = &[1, 3, 5, 7, 11];
static FUTURE_BYTE_TO_WRITE: u8 = 13;

enum State {
    S0 {
        stream: u32,
        stream_expected: Vec<u8>,
        future: u32,
        future_expected: u8,
    },
    S1 {
        stream_tx: u32,
        stream: u32,
        stream_expected: Vec<u8>,
        future_tx: u32,
        future: u32,
        future_expected: u8,
    },
}

#[unsafe(export_name = "[async-lift]local:local/synchronous-transmit#start")]
unsafe extern "C" fn export_start(
    stream: u32,
    stream_expected: u32,
    stream_expected_len: u32,
    future: u32,
    future_expected: u8,
) -> u32 {
    let stream_expected_len = usize::try_from(stream_expected_len).unwrap();

    unsafe {
        context_set(
            u32::try_from(Box::into_raw(Box::new(State::S0 {
                stream,
                stream_expected: Vec::from_raw_parts(
                    stream_expected as usize as *mut u8,
                    stream_expected_len,
                    stream_expected_len,
                ),
                future,
                future_expected,
            })) as usize)
            .unwrap(),
        );

        callback_start(EVENT_NONE, 0, 0)
    }
}

#[unsafe(export_name = "[callback][async-lift]local:local/synchronous-transmit#start")]
unsafe extern "C" fn callback_start(event0: u32, _event1: u32, _event2: u32) -> u32 {
    unsafe {
        let state = &mut *(usize::try_from(context_get()).unwrap() as *mut State);
        match state {
            &mut State::S0 {
                stream,
                ref mut stream_expected,
                future,
                future_expected,
            } => {
                assert_eq!(event0, EVENT_NONE);

                let pair = stream_new();
                let stream_tx = u32::try_from(pair >> 32).unwrap();
                let stream_rx = u32::try_from(pair & 0xFFFFFFFF_u64).unwrap();

                let pair = future_new();
                let future_tx = u32::try_from(pair >> 32).unwrap();
                let future_rx = u32::try_from(pair & 0xFFFFFFFF_u64).unwrap();

                task_return_start(
                    stream_rx,
                    STREAM_BYTES_TO_WRITE.as_ptr(),
                    STREAM_BYTES_TO_WRITE.len(),
                    future_rx,
                    FUTURE_BYTE_TO_WRITE,
                );

                *state = State::S1 {
                    stream_tx,
                    stream,
                    stream_expected: mem::take(stream_expected),
                    future_tx,
                    future,
                    future_expected,
                };

                CALLBACK_CODE_YIELD
            }

            &mut State::S1 {
                stream_tx,
                stream,
                ref mut stream_expected,
                future_tx,
                future,
                future_expected,
            } => {
                // Now we synchronously read and write and expect that the
                // operations complete.

                let mut buffer = vec![0_u8; stream_expected.len()];
                let status = stream_read(stream, buffer.as_mut_ptr(), stream_expected.len());
                assert_eq!(
                    status,
                    DROPPED | u32::try_from(stream_expected.len() << 4).unwrap()
                );
                assert_eq!(&buffer[..], stream_expected);
                stream_drop_readable(stream);

                let status = stream_write(
                    stream_tx,
                    STREAM_BYTES_TO_WRITE.as_ptr(),
                    STREAM_BYTES_TO_WRITE.len(),
                );
                assert_eq!(
                    status,
                    DROPPED | u32::try_from(STREAM_BYTES_TO_WRITE.len() << 4).unwrap()
                );
                stream_drop_writable(stream_tx);

                let received = &mut 0_u8;
                let status = future_read(future, received);
                assert_eq!(status, COMPLETED);
                assert_eq!(*received, future_expected);
                future_drop_readable(future);

                let status = future_write(future_tx, &FUTURE_BYTE_TO_WRITE);
                assert_eq!(status, COMPLETED);
                future_drop_writable(future_tx);

                CALLBACK_CODE_EXIT
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
