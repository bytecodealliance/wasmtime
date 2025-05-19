use crate::sync::sched::windows::poll_oneoff_;
use crate::tokio::block_on_dummy_executor;
use crate::{Error, file::WasiFile, sched::Poll};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    // Tokio doesn't provide us the AsyncFd primitive on Windows, so instead
    // we use the blocking poll_oneoff implementation from the wasi_common::sync impl.
    // We provide a function specific to this impl's WasiFile types for downcasting
    // to a RawHandle.
    block_on_dummy_executor(move || poll_oneoff_(poll, wasi_file_is_stdin))
}

pub fn wasi_file_is_stdin(f: &dyn WasiFile) -> bool {
    f.as_any().is::<crate::tokio::stdio::Stdin>()
}
