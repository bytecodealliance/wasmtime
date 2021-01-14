use crate::file::WasiFile;
use crate::sched::subscription::{RwSubscription, Subscription, SystemTimerSubscription};
use crate::sched::{Poll, WasiSched};
use crate::Error;
use std::any::Any;
use std::ops::Deref;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};

#[derive(Default)]
pub struct SyncSched {}

impl WasiSched for SyncSched {
    fn poll_oneoff(&self, poll: &Poll) -> Result<(), Error> {
        for s in poll.subscriptions() {
            match s {
                Subscription::Read(f) | Subscription::Write(f) => {
                    let raw_fd = wasi_file_raw_fd(f.file.deref()).ok_or(Error::Inval)?;
                    todo!()
                }
                Subscription::SystemTimer(t) => {
                    todo!()
                }
            }
        }
        Ok(())
    }
    fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }
}

fn wasi_file_raw_fd(f: &dyn WasiFile) -> Option<RawFd> {
    let a = f.as_any();
    if a.is::<cap_std::fs::File>() {
        Some(a.downcast_ref::<cap_std::fs::File>().unwrap().as_raw_fd())
    } else if a.is::<crate::stdio::Stdin>() {
        Some(a.downcast_ref::<crate::stdio::Stdin>().unwrap().as_raw_fd())
    } else if a.is::<crate::stdio::Stdout>() {
        Some(
            a.downcast_ref::<crate::stdio::Stdout>()
                .unwrap()
                .as_raw_fd(),
        )
    } else if a.is::<crate::stdio::Stderr>() {
        Some(
            a.downcast_ref::<crate::stdio::Stderr>()
                .unwrap()
                .as_raw_fd(),
        )
    } else {
        None
    }
}
