use crate::block_on_dummy_executor;
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
use wasi_cap_std_sync::sched::windows::poll_oneoff_;
use wasi_common::{file::WasiFile, sched::Poll, Error};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    // Tokio doesn't provide us the AsyncFd primitive on Windows, so instead
    // we use the blocking poll_oneoff implementation from the wasi-cap-std-crate.
    // We provide a function specific to this crate's WasiFile types for downcasting
    // to a RawHandle.
    block_on_dummy_executor(move || poll_oneoff_(poll, wasi_file_is_stdin, wasi_file_raw_handle))
}

pub fn wasi_file_is_stdin(f: &dyn WasiFile) -> bool {
    f.as_any().is::<crate::stdio::Stdin>()
}

fn wasi_file_raw_handle(f: &dyn WasiFile) -> Option<RawHandleOrSocket> {
    let a = f.as_any();
    if a.is::<crate::file::File>() {
        Some(
            a.downcast_ref::<crate::file::File>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else if a.is::<crate::net::TcpListener>() {
        Some(
            a.downcast_ref::<crate::net::TcpListener>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else if a.is::<crate::net::TcpStream>() {
        Some(
            a.downcast_ref::<crate::net::TcpStream>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else if a.is::<crate::stdio::Stdin>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdin>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else if a.is::<crate::stdio::Stdout>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdout>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else if a.is::<crate::stdio::Stderr>() {
        Some(
            a.downcast_ref::<crate::stdio::Stderr>()
                .unwrap()
                .as_raw_handle_or_socket(),
        )
    } else {
        None
    }
}
