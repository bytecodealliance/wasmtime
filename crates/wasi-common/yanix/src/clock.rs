use crate::{Error, Result};
use std::mem::MaybeUninit;

#[derive(Debug, Copy, Clone)]
pub enum ClockId {
    Realtime,
    Monotonic,
    ProcessCPUTime,
    ThreadCPUTime,
}

impl ClockId {
    pub fn as_raw(&self) -> libc::clockid_t {
        match self {
            Self::Realtime => libc::CLOCK_REALTIME,
            Self::Monotonic => libc::CLOCK_MONOTONIC,
            Self::ProcessCPUTime => libc::CLOCK_PROCESS_CPUTIME_ID,
            Self::ThreadCPUTime => libc::CLOCK_THREAD_CPUTIME_ID,
        }
    }
}

pub fn clock_getres(clock_id: ClockId) -> Result<libc::timespec> {
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    Error::from_success_code(unsafe {
        libc::clock_getres(clock_id.as_raw(), timespec.as_mut_ptr())
    })?;
    Ok(unsafe { timespec.assume_init() })
}

pub fn clock_gettime(clock_id: ClockId) -> Result<libc::timespec> {
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    Error::from_success_code(unsafe {
        libc::clock_gettime(clock_id.as_raw(), timespec.as_mut_ptr())
    })?;
    Ok(unsafe { timespec.assume_init() })
}
