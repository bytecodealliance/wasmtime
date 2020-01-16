#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::fdentry::Descriptor;
use crate::hostcalls_impl::{ClockEventData, FdEventData};
use crate::memory::*;
use crate::sys::host_impl;
use crate::{wasi, wasi32, Error, Result};
use cpu_time::{ProcessTime, ThreadTime};
use lazy_static::lazy_static;
use log::{debug, error, trace, warn};
use std::convert::TryInto;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct StdinPoll {
    request_tx: Sender<()>,
    notify_rx: Receiver<PollState>,
}

enum PollState {
    Ready,
    Closed,
    NotReady, // it's not ready, but we didn't wait
    TimedOut, // it's not ready and a timeout has occurred
    Error(Error),
}

enum WaitMode {
    Timeout(Duration),
    Infinite,
    Immediate,
}

impl StdinPoll {
    // This function should not be used directly
    // Correctness of this function crucially depends on the fact that
    // mpsc::Receiver is !Sync.
    fn poll(&self, wait_mode: WaitMode) -> PollState {
        // Clean up possible unread result from the previous poll
        match self.notify_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("notify_rx channel closed"),
        }

        // Notify the worker thread that we want to poll stdin
        self.request_tx.send(()).expect("request_tx channel closed");

        // Wait for the worker thread to send a readiness notification
        let pollret = match wait_mode {
            WaitMode::Timeout(timeout) => {
                self.notify_rx
                    .recv_timeout(timeout)
                    .unwrap_or_else(|e| match e {
                        RecvTimeoutError::Disconnected => panic!("notify_rx channel closed"),
                        RecvTimeoutError::Timeout => PollState::TimedOut,
                    })
            }
            WaitMode::Infinite => self.notify_rx.recv().expect("notify_rx channel closed"),
            WaitMode::Immediate => self.notify_rx.try_recv().unwrap_or_else(|e| match e {
                TryRecvError::Disconnected => panic!("notify_rx channel closed"),
                TryRecvError::Empty => PollState::NotReady,
            }),
        };

        pollret
    }

    fn event_loop(request_rx: Receiver<()>, notify_tx: Sender<PollState>) -> ! {
        use std::io::BufRead;
        loop {
            request_rx.recv().expect("request_rx channel closed");
            let resp = match std::io::stdin().lock().fill_buf().map(|s| !s.is_empty()) {
                Ok(true) => PollState::Ready,
                Ok(false) => PollState::Closed,
                Err(e) => PollState::Error(e.into()),
            };
            notify_tx.send(resp).expect("notify_tx channel closed");
        }
    }
}

lazy_static! {
    static ref START_MONOTONIC: Instant = Instant::now();
    static ref PERF_COUNTER_RES: u64 = get_perf_counter_resolution_ns();
    static ref STDIN_POLL: Mutex<StdinPoll> = {
        let (request_tx, request_rx) = mpsc::channel();
        let (notify_tx, notify_rx) = mpsc::channel();
        thread::spawn(move || StdinPoll::event_loop(request_rx, notify_tx));
        Mutex::new(StdinPoll {
            request_tx,
            notify_rx,
        })
    };
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

fn make_rw_event(event: &FdEventData, nbytes: Result<u64>) -> wasi::__wasi_event_t {
    use crate::error::WasiErrno;
    let error = nbytes.as_wasi_errno();
    let nbytes = nbytes.unwrap_or_default();
    wasi::__wasi_event_t {
        userdata: event.userdata,
        r#type: event.r#type,
        error,
        u: wasi::__wasi_event_u_t {
            fd_readwrite: wasi::__wasi_event_fd_readwrite_t { nbytes, flags: 0 },
        },
    }
}

fn make_timeout_event(timeout: &ClockEventData) -> wasi::__wasi_event_t {
    wasi::__wasi_event_t {
        userdata: timeout.userdata,
        r#type: wasi::__WASI_EVENTTYPE_CLOCK,
        error: wasi::__WASI_ERRNO_SUCCESS,
        u: wasi::__wasi_event_u_t {
            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                nbytes: 0,
                flags: 0,
            },
        },
    }
}

fn make_hangup_event(fd_event: &FdEventData) -> wasi::__wasi_event_t {
    wasi::__wasi_event_t {
        userdata: fd_event.userdata,
        r#type: fd_event.r#type,
        error: wasi::__WASI_ERRNO_SUCCESS,
        u: wasi::__wasi_event_u_t {
            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                nbytes: 0,
                flags: wasi::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP,
            },
        },
    }
}

fn handle_timeout(
    timeout_event: ClockEventData,
    timeout: Duration,
    events: &mut Vec<wasi::__wasi_event_t>,
) {
    thread::sleep(timeout);
    handle_timeout_event(timeout_event, events);
}

fn handle_timeout_event(timeout_event: ClockEventData, events: &mut Vec<wasi::__wasi_event_t>) {
    let new_event = make_timeout_event(&timeout_event);
    events.push(new_event);
}

fn handle_hangup_event(event: FdEventData, events: &mut Vec<wasi::__wasi_event_t>) {
    let new_event = make_hangup_event(&event);
    events.push(new_event)
}

fn handle_rw_event(event: FdEventData, out_events: &mut Vec<wasi::__wasi_event_t>) {
    let size = match event.descriptor {
        Descriptor::OsHandle(os_handle) => {
            if event.r#type == wasi::__WASI_EVENTTYPE_FD_READ {
                os_handle.metadata().map(|m| m.len()).map_err(Into::into)
            } else {
                // The spec is unclear what nbytes should actually be for __WASI_EVENTTYPE_FD_WRITE and
                // the implementation on Unix just returns 0 here, so it's probably fine
                // to do the same on Windows for now.
                // cf. https://github.com/WebAssembly/WASI/issues/148
                Ok(0)
            }
        }
        // We return the only universally correct lower bound, see the comment later in the function.
        Descriptor::Stdin => Ok(1),
        // On Unix, ioctl(FIONREAD) will return 0 for stdout/stderr. Emulate the same behavior on Windows.
        Descriptor::Stdout | Descriptor::Stderr => Ok(0),
    };

    let new_event = make_rw_event(&event, size);
    out_events.push(new_event);
}

fn handle_error_event(
    event: FdEventData,
    error: Error,
    out_events: &mut Vec<wasi::__wasi_event_t>,
) {
    let new_event = make_rw_event(&event, Err(error));
    out_events.push(new_event);
}

pub(crate) fn poll_oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> Result<()> {
    use std::fs::Metadata;
    use std::thread;

    let timeout = timeout
        .map(|event| {
            event
                .delay
                .try_into()
                .map(Duration::from_nanos)
                .map(|dur| (event, dur))
        })
        .transpose()?;

    // With no events to listen, poll_oneoff just becomes a sleep.
    if fd_events.is_empty() {
        match timeout {
            Some((event, dur)) => return Ok(handle_timeout(event, dur, events)),
            // `poll` invoked with nfds = 0, timeout = -1 appears to be an infinite sleep on Unix
            // usually meant to be interrupted by a signal. Unfortunately, WASI doesn't currently
            // support signals and there is no way to interrupt this infinite sleep, so we
            // intend to return `ENOTSUP`.
            //
            // Unfortunately, current implementation of poll_oneoff on Unix relies on
            // `poll_oneoff` returning `Ok(())` in such case, so we emulate this behavior for now
            // and will address it in a subsequent PR.
            None => {
                // error!("poll_oneoff: invoking with neither timeout nor fds not supported");
                return Ok(()); // Err(Error::ENOTSUP);
            }
        }
    }

    let mut stdin_events = vec![];
    let mut immediate_events = vec![];
    let mut pipe_events = vec![];

    for event in fd_events {
        match event.descriptor {
            Descriptor::Stdin if event.r#type == wasi::__WASI_EVENTTYPE_FD_READ => {
                stdin_events.push(event)
            }
            // stdout/stderr are always considered ready to write because there seems to
            // be no way of checking if a write to stdout would block.
            //
            // If stdin is polled for anything else then reading, then it is also
            // considered immediately ready, following the behavior on Linux.
            Descriptor::Stdin | Descriptor::Stderr | Descriptor::Stdout => {
                immediate_events.push(event)
            }
            Descriptor::OsHandle(os_handle) => {
                let ftype = unsafe { winx::file::get_file_type(os_handle.as_raw_handle()) }?;
                if ftype.is_unknown() || ftype.is_char() {
                    debug!("poll_oneoff: unsupported file type: {:?}", ftype);
                    handle_error_event(event, Error::ENOTSUP, events);
                } else if ftype.is_disk() {
                    immediate_events.push(event);
                } else if ftype.is_pipe() {
                    pipe_events.push(event);
                } else {
                    unreachable!();
                }
            }
        }
    }

    let immediate = !immediate_events.is_empty();
    // Process all the events that do not require waiting.
    if immediate {
        trace!("    | have immediate events, will return immediately");
        for mut event in immediate_events {
            handle_rw_event(event, events);
        }
    }
    if !stdin_events.is_empty() {
        // During the firt request to poll stdin, we spin up a separate thread to
        // waiting for data to arrive on stdin. This thread will not terminate.
        //
        // We'd like to do the following:
        // (1) wait in a non-blocking way for data to be available in stdin, with timeout
        // (2) find out, how many bytes are there available to be read.
        //
        // One issue is that we are currently relying on the Rust libstd for interaction
        // with stdin. More precisely, `io::stdin` is used via the `BufRead` trait,
        // in the `fd_read` function, which always does buffering on the libstd side. [1]
        // This means that even if there's still some unread data in stdin,
        // the lower-level Windows system calls may return false negatives,
        // claiming that stdin is empty.
        //
        // Theoretically, one could use `WaitForSingleObject` on the stdin handle
        // to achieve (1). Unfortunately, this function doesn't seem to honor the
        // requested timeout and to misbehaves after the stdin is closed.
        //
        // There appears to be no way of achieving (2) on Windows.
        // [1]: https://github.com/rust-lang/rust/pull/12422
        let waitmode = if immediate {
            trace!("     | tentatively checking stdin");
            WaitMode::Immediate
        } else {
            trace!("     | passively waiting on stdin");
            match timeout {
                Some((event, dur)) => WaitMode::Timeout(dur),
                None => WaitMode::Infinite,
            }
        };
        let state = STDIN_POLL.lock().unwrap().poll(waitmode);
        for event in stdin_events {
            match state {
                PollState::Ready => handle_rw_event(event, events),
                PollState::NotReady => {} // not immediately available, so just ignore
                PollState::Closed => handle_hangup_event(event, events), // TODO check if actually a POLLHUP on Linux
                PollState::TimedOut => handle_timeout_event(timeout.unwrap().0, events),
                PollState::Error(ref e) => {
                    error!("FIXME return real error");
                    handle_error_event(event, Error::ENOTSUP, events);
                }
            }
        }
    }

    if !immediate && !pipe_events.is_empty() {
        trace!("     | actively polling pipes");
        match timeout {
            Some((event, dur)) => {
                // In the tests stdin is replaced with a dummy pipe, so for now
                // we just time out. Support for pipes will be decided later on.
                warn!("Polling pipes not supported on Windows, will just time out.");
                handle_timeout(event, dur, events);
            }
            None => {
                error!("Polling only pipes with no timeout not supported on Windows.");
                return Err(Error::ENOTSUP);
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
