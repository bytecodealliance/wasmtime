mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "cancel-transmit-guest",
    });
}

use {
    std::{
        mem::{self, ManuallyDrop},
        slice,
    },
    test_programs::async_::{
        BLOCKED, CALLBACK_CODE_EXIT, CALLBACK_CODE_YIELD, COMPLETED, DROPPED, EVENT_NONE,
        context_get, context_set,
    },
};

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[task-return][async]start"]
    fn task_return_start(_: u32, _: *const u8, _: usize, _: u32, _: u8);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn task_return_start(_: u32, _: *const u8, _: usize, _: u32, _: u8) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-new-0][async]start"]
    fn stream_new() -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_new() -> u64 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[async-lower][stream-write-0][async]start"]
    fn stream_write(_: u32, _: *const u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_write(_: u32, _: *const u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[async-lower][stream-read-0][async]start"]
    fn stream_read(_: u32, _: *mut u8, _: usize) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_read(_: u32, _: *mut u8, _: usize) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-cancel-write-0][async]start"]
    fn stream_cancel_write(_: u32) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_cancel_write(_: u32) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-cancel-read-0][async]start"]
    fn stream_cancel_read(_: u32) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_cancel_read(_: u32) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-drop-readable-0][async]start"]
    fn stream_drop_readable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_readable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[stream-drop-writable-0][async]start"]
    fn stream_drop_writable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn stream_drop_writable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[future-new-1][async]start"]
    fn future_new() -> u64;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_new() -> u64 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[async-lower][future-write-1][async]start"]
    fn future_write(_: u32, _: *const u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_write(_: u32, _: *const u8) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[async-lower][future-read-1][async]start"]
    fn future_read(_: u32, _: *mut u8) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_read(_: u32, _: *mut u8) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[future-cancel-write-1][async]start"]
    fn future_cancel_write(_: u32) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_cancel_write(_: u32) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[future-cancel-read-1][async]start"]
    fn future_cancel_read(_: u32) -> u32;
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_cancel_read(_: u32) -> u32 {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[future-drop-readable-1][async]start"]
    fn future_drop_readable(_: u32);
}
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn future_drop_readable(_: u32) {
    unreachable!()
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "[export]local:local/cancel-transmit")]
unsafe extern "C" {
    #[link_name = "[future-drop-writable-1][async]start"]
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
        stream_read_buffer: *mut u8,
        stream_tx: u32,
        stream: u32,
        stream_expected: Vec<u8>,
        future_read_buffer: *mut u8,
        future_tx: u32,
        future: u32,
        future_expected: u8,
    },
}

#[unsafe(export_name = "[async-lift]local:local/cancel-transmit#[async]start")]
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

#[unsafe(export_name = "[callback][async-lift]local:local/cancel-transmit#[async]start")]
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

                // Here we assume specific behavior from the writers, namely:
                //
                // - They will not send us anything until after we cancel the
                // reads, and even then there will be a delay.
                //
                // - When they _do_ send, they will send us all the bytes it
                // told us to expect at once.
                let stream_read_buffer =
                    ManuallyDrop::new(vec![0_u8; stream_expected.len()]).as_mut_ptr();
                let status = stream_read(stream, stream_read_buffer, stream_expected.len());
                assert_eq!(status, BLOCKED);

                let future_read_buffer = Box::into_raw(Box::new(0_u8));
                let status = future_read(future, future_read_buffer);
                assert_eq!(status, BLOCKED);

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

                // Here we assume specific behavior from the readers, namely:
                //
                // - They will not read anything until after we cancel the
                // write, and even then there will be a delay.
                //
                // - When they _do_ read, they will accept all the bytes we told
                // it to expect at once.
                let status = stream_write(
                    stream_tx,
                    STREAM_BYTES_TO_WRITE.as_ptr(),
                    STREAM_BYTES_TO_WRITE.len(),
                );
                assert_eq!(status, BLOCKED);

                let status = future_write(future_tx, &FUTURE_BYTE_TO_WRITE);
                assert_eq!(status, BLOCKED);

                *state = State::S1 {
                    stream_read_buffer,
                    stream_tx,
                    stream,
                    stream_expected: mem::take(stream_expected),
                    future_read_buffer,
                    future_tx,
                    future,
                    future_expected,
                };

                CALLBACK_CODE_YIELD
            }

            &mut State::S1 {
                stream_read_buffer,
                stream_tx,
                stream,
                ref mut stream_expected,
                future_read_buffer,
                future_tx,
                future,
                future_expected,
            } => {
                // Now we synchronously cancel everything and expect that the
                // reads and writes complete.

                let status = stream_cancel_read(stream);
                assert_eq!(
                    status,
                    DROPPED | u32::try_from(stream_expected.len() << 4).unwrap()
                );
                let received = Box::from_raw(slice::from_raw_parts_mut(
                    stream_read_buffer,
                    stream_expected.len(),
                ));
                assert_eq!(&received[..], stream_expected);
                stream_drop_readable(stream);

                let status = stream_cancel_write(stream_tx);
                assert_eq!(
                    status,
                    DROPPED | u32::try_from(STREAM_BYTES_TO_WRITE.len() << 4).unwrap()
                );
                stream_drop_writable(stream_tx);

                let status = future_cancel_read(future);
                assert_eq!(status, COMPLETED);
                let received = Box::from_raw(future_read_buffer);
                assert_eq!(*received, future_expected);
                future_drop_readable(future);

                let status = future_cancel_write(future_tx);
                assert_eq!(status, COMPLETED);
                future_drop_writable(future_tx);

                CALLBACK_CODE_EXIT
            }
        }
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
