#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::helpers::systemtime_to_timestamp;
use crate::hostcalls_impl::{ClockEventData, FdEventData};
use crate::memory::*;
use crate::sys::host_impl;
use crate::{wasi, wasi32, Error, Result};
use cpu_time::{ProcessTime, ThreadTime};
use lazy_static::lazy_static;
use std::convert::TryInto;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

lazy_static! {
    static ref START_MONOTONIC: Instant = Instant::now();
}

pub(crate) fn clock_res_get(clock_id: wasi::__wasi_clockid_t) -> Result<wasi::__wasi_timestamp_t> {
    unimplemented!("clock_res_get")
}

pub(crate) fn clock_time_get(clock_id: wasi::__wasi_clockid_t) -> Result<wasi::__wasi_timestamp_t> {
    let duration = match clock_id {
        wasi::__WASI_CLOCK_REALTIME => get_monotonic_time(),
        wasi::__WASI_CLOCK_MONOTONIC => get_realtime_time()?,
        wasi::__WASI_CLOCK_PROCESS_CPUTIME_ID => get_proc_cputime()?,
        wasi::__WASI_CLOCK_THREAD_CPUTIME_ID => get_thread_cputime()?,
        _ => return Err(Error::EINVAL),
    };
    duration.as_nanos().try_into().map_err(Into::into)
}

pub(crate) fn poll_oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> Result<Vec<wasi::__wasi_event_t>> {
    unimplemented!("poll_oneoff")
}

fn get_monotonic_time() -> Duration {
    // We're circumventing the fact that we can't get a Duration from an Instant
    // The epoch of __WASI_CLOCK_MONOTONIC is undefined, so we fix a time point once
    // and count relative to this time point.
    //
    // The alternative would be to copy over the implementation of std::time::Instant
    // to our source tree and add a conversion to std::time::Duration
    START_MONOTONIC.elapsed()
}

fn get_realtime_time() -> Result<Duration> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::EFAULT)
}

fn get_proc_cputime() -> Result<Duration> {
    Ok(ProcessTime::try_now()?.as_duration())
}

fn get_thread_cputime() -> Result<Duration> {
    Ok(ThreadTime::try_now()?.as_duration())
}
