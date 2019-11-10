#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
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
    static ref PERF_COUNTER_RES: u64 = get_perf_counter_resolution_ns();
}

// Timer resolution on Windows is really hard. We may consider exposing the resolution of the respective
// timers as an associated function in the future.
pub(crate) fn clock_res_get(clock_id: wasi::__wasi_clockid_t) -> Result<wasi::__wasi_timestamp_t> {
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
        _ => return Err(Error::EINVAL),
    })
}

pub(crate) fn clock_time_get(clock_id: wasi::__wasi_clockid_t) -> Result<wasi::__wasi_timestamp_t> {
    let duration = match clock_id {
        wasi::__WASI_CLOCKID_REALTIME => get_monotonic_time(),
        wasi::__WASI_CLOCKID_MONOTONIC => get_realtime_time()?,
        wasi::__WASI_CLOCKID_PROCESS_CPUTIME_ID => get_proc_cputime()?,
        wasi::__WASI_CLOCKID_THREAD_CPUTIME_ID => get_thread_cputime()?,
        _ => return Err(Error::EINVAL),
    };
    duration.as_nanos().try_into().map_err(Into::into)
}

fn stdin_nonempty() -> bool {
    use std::io::Read;
    std::io::stdin().bytes().peekable().peek().is_some()
}

pub(crate) fn poll_oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> Result<()> {
    use crate::fdentry::Descriptor;
    if fd_events.is_empty() && timeout.is_none() {
        return Ok(());
    }

    // Currently WASI file support is only (a) regular files (b) directories (c) symlinks on Windows,
    // which are always ready to write on Unix.
    //
    // We need to consider stdin/stdout/stderr separately.
    // We treat stdout/stderr as always ready to write. I'm not sure if it's correct
    // on Windows but I have not find any way of checking if a write to stdout would block.
    // Therefore, we only poll the stdin.
    let mut stdin_events = vec![];
    let mut immediate_events = vec![];

    for event in fd_events {
        match event.descriptor {
            Descriptor::Stdin if event.r#type == wasi::__WASI_EVENTTYPE_FD_READ => {
                stdin_events.push(event)
            }
            _ => immediate_events.push(event),
        }
    }

    // we have at least one immediate event, so we don't need to care about stdin
    if immediate_events.len() > 0 {
        for event in immediate_events {
            let size = match event.descriptor {
                Descriptor::OsHandle(os_handle)
                    if event.r#type == wasi::__WASI_EVENTTYPE_FD_READ =>
                {
                    os_handle
                        .metadata()
                        .expect("FIXME return a proper error")
                        .len()
                }
                Descriptor::Stdin => panic!("Descriptor::Stdin should have been filtered out"),
                // On Unix, ioctl(FIONREAD) will return 0 for stdout/stderr. Emulate the same behavior on Windows.
                //
                // Besides, the spec is unclear what nbytes should actually be for __WASI_EVENTTYPE_FD_WRITE and
                // the implementation on Unix just returns 0 here, so it's probably fine to do the same on Windows for now.
                // cf. https://github.com/WebAssembly/WASI/issues/148
                _ => 0,
            };

            events.push(wasi::__wasi_event_t {
                userdata: event.userdata,
                r#type: event.r#type,
                error: wasi::__WASI_ERRNO_SUCCESS,
                u: wasi::__wasi_event_u_t {
                    fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                        nbytes: size,
                        flags: 0,
                    },
                },
            })
        }
    } else {
        assert_ne!(stdin_events.len(), 0, "stdin_events should not be empty");

        // We are busy-polling the stdin with delay, unfortunately.
        //
        // We'd like to do the following:
        // (1) wait in a non-blocking way for data to be available in stdin, with timeout
        // (2) find out, how many bytes are there available to be read.
        // For one, using `WaitForSingleObject` on the stdin handle could possibly be one way of
        // achieving (1).
        // I don't know of any way to achieve (2).
        //
        // While both of these are not as trivial on Windows as they are on Linux, there's a much
        // more fundamental issue preventing us from achieving such behavior with the current
        // implementation of wasi-common.
        //
        // Precisely, in `fd_read` we are using `io::stdin` via the `BufRead` trait, which does
        // buffering on the libstd side. This means that even if there's still some unread data
        // in stdin, the Windows system calls may return false negatives, indicating that stdin is empty.
        // Therefore, avoiding the busy-poll here would require us to ditch libstd for the interaction
        // with stdin altogether.
        //
        // However, polling stdin is a relatively infrequent use case, so this hopefully won't be
        // a major issue.
        let poll_interval = Duration::from_millis(10);
        let poll_start = Instant::now();
        let timeout_duration = timeout
            .map(|t| t.delay.try_into().map(Duration::from_nanos))
            .transpose()?;
        let timeout = loop {
            if let Some(timeout_duration) = timeout_duration {
                if poll_start.elapsed() >= timeout_duration {
                    break timeout;
                }
            }
            if stdin_nonempty() {
                break None;
            }
            std::thread::sleep(poll_interval);
        };

        // TODO try refactoring pushing to events
        match timeout {
            // timeout occurred
            Some(timeout) => {
                for event in stdin_events {
                    events.push(wasi::__wasi_event_t {
                        userdata: timeout.userdata,
                        r#type: wasi::__WASI_EVENTTYPE_CLOCK,
                        error: wasi::__WASI_ERRNO_SUCCESS,
                        u: wasi::__wasi_event_u_t {
                            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                                nbytes: 0,
                                flags: 0,
                            },
                        },
                    });
                }
            }
            // stdin is ready for reading
            None => {
                for event in stdin_events {
                    assert_eq!(
                        event.r#type,
                        wasi::__WASI_EVENTTYPE_FD_READ,
                        "stdin was expected to be polled for reading"
                    );
                    events.push(wasi::__wasi_event_t {
                        userdata: event.userdata,
                        r#type: event.r#type,
                        error: wasi::__WASI_ERRNO_SUCCESS,
                        // Another limitation is that `std::io::BufRead` doesn't allow us
                        // to find out the number bytes available in the buffer,
                        // so we return the only universally correct lower bound.
                        u: wasi::__wasi_event_u_t {
                            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                                nbytes: 1,
                                flags: 0,
                            },
                        },
                    });
                }
            }
        }
    }

    Ok(())
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

fn get_perf_counter_resolution_ns() -> u64 {
    use winx::time::perf_counter_frequency;
    const NANOS_PER_SEC: u64 = 1_000_000_000;
    // This should always succeed starting from Windows XP, so it's fine to panic in case of an error.
    let freq = perf_counter_frequency().expect("QueryPerformanceFrequency returned an error");
    let epsilon = NANOS_PER_SEC / freq;
    epsilon
}
