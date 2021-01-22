
use std::os::windows::io::{AsRawHandle, RawHandle};
use wasi_c2::{
    file::WasiFile,
    sched::{Poll, WasiSched},
    Error,
};
pub struct SyncSched;

impl WasiSched for SyncSched {
    fn poll_oneoff<'a>(&self, poll: &'a Poll<'a>) -> Result<(), Error> {
        if poll.is_empty() {
            return Ok(());
        }
        todo!()
    }
    fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }
}

fn wasi_file_raw_handle(f: &dyn WasiFile) -> Option<RawHandle> {
    let a = f.as_any();
    if a.is::<crate::file::File>() {
        Some(
            a.downcast_ref::<crate::file::File>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stdin>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdin>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stdout>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdout>()
                .unwrap()
                .as_raw_handle(),
        )
    } else if a.is::<crate::stdio::Stderr>() {
        Some(
            a.downcast_ref::<crate::stdio::Stderr>()
                .unwrap()
                .as_raw_handle(),
        )
    } else {
        None
    }
}
