#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::poll_oneoff;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::poll_oneoff;

use crate::{
    Error,
    sched::{Poll, WasiSched},
};
use std::thread;
use std::time::Duration;

pub struct SyncSched {}
impl SyncSched {
    pub fn new() -> Self {
        Self {}
    }
}
#[wiggle::async_trait]
impl WasiSched for SyncSched {
    async fn poll_oneoff<'a>(&self, poll: &mut Poll<'a>) -> Result<(), Error> {
        poll_oneoff(poll).await
    }
    async fn sched_yield(&self) -> Result<(), Error> {
        thread::yield_now();
        Ok(())
    }
    async fn sleep(&self, duration: Duration) -> Result<(), Error> {
        std::thread::sleep(duration);
        Ok(())
    }
}
pub fn sched_ctx() -> Box<dyn WasiSched> {
    Box::new(SyncSched::new())
}
