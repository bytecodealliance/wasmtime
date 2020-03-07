#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::old::snapshot_0::hostcalls_impl::{ClockEventData, FdEventData};
use crate::old::snapshot_0::memory::*;
use crate::old::snapshot_0::sys::host_impl;
use crate::old::snapshot_0::wasi::{self, WasiError, WasiResult};
use crate::old::snapshot_0::wasi32;
use cpu_time::{ProcessTime, ThreadTime};
use lazy_static::lazy_static;
use std::convert::TryInto;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

lazy_static! {
    static ref START_MONOTONIC: Instant = Instant::now();
    static ref PERF_COUNTER_RES: u64 = get_perf_counter_resolution_ns();
}

// Timer resolution on Windows is really hard. We may consider exposing the resolution of the respective
// timers as an associated function in the future.
pub(crate) fn clock_res_get(
    clock_id: wasi::__wasi_clockid_t,
) -> WasiResult<wasi::__wasi_timestamp_t> {
    Ok(match clock_id {
        // This is the best that we can do with std::time::SystemTime.
        // Rust uses GetSystemTimeAsFileTime, which is said to have the resolution of
        // 10ms or 55ms, [1] but MSDN doesn't confirm this in any way.
        // Even the MSDN article on high resolution timestamps doesn't even mention the precision
        // for this method. [3]
        //
        // The timer resolution can be queried using one of the functions: [2, 5]
        // * NtQueryTimerResolution, which is undocumented and thus not exposed by the winapi crate
        // * timeGetDevCaps, which returns the upper and lower bound for the precision, in ms.
        // While the upper bound seems like something we could use, it's typically too high to be meaningful.
        // For instance, the intervals return by the syscall are:
        // * [1, 65536] on Wine
        // * [1, 1000000] on Windows 10, which is up to (sic) 1000 seconds.
        //
        // It's possible to manually set the timer resolution, but this sounds like something which should
        // only be done temporarily. [5]
        //
        // Alternatively, we could possibly use GetSystemTimePreciseAsFileTime in clock_time_get, but
        // this syscall is only available starting from Windows 8.
        // (we could possibly emulate it on earlier versions of Windows, see [4])
        // The MSDN are not clear on the resolution of GetSystemTimePreciseAsFileTime either, but a
        // Microsoft devblog entry [1] suggests that it kind of combines GetSystemTimeAsFileTime with
        // QueryPeformanceCounter, which probably means that those two should have the same resolution.
        //
        // See also this discussion about the use of GetSystemTimePreciseAsFileTime in Python stdlib,
        // which in particular contains some resolution benchmarks.
        //
        // [1] https://devblogs.microsoft.com/oldnewthing/20170921-00/?p=97057
        // [2] http://www.windowstimestamp.com/description
        // [3] https://docs.microsoft.com/en-us/windows/win32/sysinfo/acquiring-high-resolution-time-stamps?redirectedfrom=MSDN
        // [4] https://www.codeproject.com/Tips/1011902/High-Resolution-Time-For-Windows
        // [5] https://stackoverflow.com/questions/7685762/windows-7-timing-functions-how-to-use-getsystemtimeadjustment-correctly
        // [6] https://bugs.python.org/issue19007
        wasi::__WASI_CLOCKID_REALTIME => 55_000_000,
        // std::time::Instant uses QueryPerformanceCounter & QueryPerformanceFrequency internally
        wasi::__WASI_CLOCKID_MONOTONIC => *PERF_COUNTER_RES,
        // The best we can do is to hardcode the value from the docs.
        // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes
        wasi::__WASI_CLOCKID_PROCESS_CPUTIME_ID => 100,
        // The best we can do is to hardcode the value from the docs.
        // https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes
        wasi::__WASI_CLOCKID_THREAD_CPUTIME_ID => 100,
        _ => return Err(WasiError::EINVAL),
    })
}

pub(crate) fn clock_time_get(
    clock_id: wasi::__wasi_clockid_t,
) -> WasiResult<wasi::__wasi_timestamp_t> {
    let duration = match clock_id {
        wasi::__WASI_CLOCKID_REALTIME => get_monotonic_time(),
        wasi::__WASI_CLOCKID_MONOTONIC => get_realtime_time()?,
        wasi::__WASI_CLOCKID_PROCESS_CPUTIME_ID => get_proc_cputime()?,
        wasi::__WASI_CLOCKID_THREAD_CPUTIME_ID => get_thread_cputime()?,
        _ => return Err(WasiError::EINVAL),
    };
    duration.as_nanos().try_into().map_err(Into::into)
}

pub(crate) fn poll_oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> WasiResult<Vec<wasi::__wasi_event_t>> {
    unimplemented!("poll_oneoff")
}

fn get_monotonic_time() -> Duration {
    // We're circumventing the fact that we can't get a Duration from an Instant
    // The epoch of __WASI_CLOCKID_MONOTONIC is undefined, so we fix a time point once
    // and count relative to this time point.
    //
    // The alternative would be to copy over the implementation of std::time::Instant
    // to our source tree and add a conversion to std::time::Duration
    START_MONOTONIC.elapsed()
}

fn get_realtime_time() -> WasiResult<Duration> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| WasiError::EFAULT)
}

fn get_proc_cputime() -> WasiResult<Duration> {
    Ok(ProcessTime::try_now()?.as_duration())
}

fn get_thread_cputime() -> WasiResult<Duration> {
    Ok(ThreadTime::try_now()?.as_duration())
}

fn get_perf_counter_resolution_ns() -> u64 {
    use winx::time::perf_counter_frequency;
    const NANOS_PER_SEC: u64 = 1_000_000_000;
    // This should always succeed starting from Windows XP, so it's fine to panic in case of an error.
    let freq = perf_counter_frequency().expect("QueryPerformanceFrequency returned an error");
    let epsilon = NANOS_PER_SEC / freq;
    epsilon
}
