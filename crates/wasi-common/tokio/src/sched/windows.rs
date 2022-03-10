use crate::block_on_dummy_executor;
use wasi_cap_std_sync::sched::windows::poll_oneoff_;
use wasi_common::{file::WasiFile, sched::Poll, Error};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    // Tokio doesn't provide us the AsyncFd primitive on Windows, so instead
    // we use the blocking poll_oneoff implementation from the wasi-cap-std-crate.
    // We provide a function specific to this crate's WasiFile types for downcasting
    // to a RawHandle.
    block_on_dummy_executor(move || poll_oneoff_(poll, wasi_file_is_stdin))
}

pub fn wasi_file_is_stdin(f: &dyn WasiFile) -> bool {
    f.as_any().is::<crate::stdio::Stdin>()
}
