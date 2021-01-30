#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

use wasi_common::sched::WasiSched;

pub fn sched_ctx() -> Box<dyn WasiSched> {
    Box::new(SyncSched::new())
}
