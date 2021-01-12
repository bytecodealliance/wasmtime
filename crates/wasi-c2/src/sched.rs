use crate::Error;

pub trait WasiSched {
    // XXX poll oneoff needs args and results.
    fn poll_oneoff(&self) -> Result<(), Error>;
    fn sched_yield(&self) -> Result<(), Error>;
}

#[derive(Default)]
pub struct SyncSched {}

impl WasiSched for SyncSched {
    fn poll_oneoff(&self) -> Result<(), Error> {
        todo!()
    }
    fn sched_yield(&self) -> Result<(), Error> {
        std::thread::yield_now();
        Ok(())
    }
}
