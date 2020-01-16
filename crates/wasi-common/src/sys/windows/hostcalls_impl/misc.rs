#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::fdentry::Descriptor;
use crate::hostcalls_impl::{ClockEventData, FdEventData};
use crate::memory::*;
use crate::sys::host_impl;
use crate::{wasi, wasi32, Error, Result};
use cpu_time::{ProcessTime, ThreadTime};
use crossbeam::channel::{self, Receiver, Sender};
use lazy_static::lazy_static;
use log::{error, trace, warn};
use std::convert::TryInto;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type StdinPayload = io::Result<bool>;
struct StdinPoll {
    request_tx: Sender<()>,
    notify_rx: Receiver<StdinPayload>,
}

enum PollState {
    Ready,
    Closed,
    TimedOut,
    Error(Error),
}

impl StdinPoll {
    fn poll(&self, timeout: Option<Duration>) -> PollState {
        use crossbeam::channel::{RecvTimeoutError, TryRecvError};
        // Clean up possible unread result from previous poll
        match self.notify_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("FIXME"),
        }
        self.request_tx.send(()).expect("FIXME");
        let pollret = match timeout {
            Some(timeout) => self.notify_rx.recv_timeout(timeout),
            None => Ok(self.notify_rx.recv().expect("FIXME")),
        };
        match pollret {
            Ok(Ok(true)) => PollState::Ready,
            Ok(Ok(false)) => PollState::Closed,
            Ok(Err(e)) => PollState::Error(e.into()),
            Err(RecvTimeoutError::Timeout) => PollState::TimedOut,
            Err(RecvTimeoutError::Disconnected) => panic!("FIXME"),
        }
    }

    fn event_loop(request_rx: Receiver<()>, notify_tx: Sender<StdinPayload>) -> ! {
        use std::io::BufRead;
        loop {
            request_rx.recv().expect("FIXME");
            let buf = std::io::stdin().lock().fill_buf().map(|s| !s.is_empty());
            notify_tx.send(buf).expect("FIXME");
        }
    }
}

lazy_static! {
    static ref START_MONOTONIC: Instant = Instant::now();
    static ref PERF_COUNTER_RES: u64 = get_perf_counter_resolution_ns();
    static ref STDIN_POLL: StdinPoll = {
        let channel_size = 1;
        let (request_tx, request_rx) = channel::bounded(channel_size);
        let (notify_tx, notify_rx) = channel::bounded(channel_size);
        thread::spawn(move || StdinPoll::event_loop(request_rx, notify_tx));
        StdinPoll {
            request_tx,
            notify_rx,
        }
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

fn stdin_nonempty() -> bool {
    use std::io::Read;
    std::io::stdin().bytes().peekable().peek().is_some()
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

    trace!(
        "poll_oneoff_impl: timeout={:?}, fd_events={:?}",
        timeout,
        fd_events
    );

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

    // Currently WASI file support is only (a) regular files (b) directories (c) symlinks on Windows,
    // which are always ready to write on Unix.
    //
    // We need to consider stdin/stdout/stderr separately.
    // We treat stdout/stderr as always ready to write. I'm not sure if it's correct
    // on Windows but I have not find any way of checking if a write to stdout would block.
    // Therefore, we only poll the stdin.
    let mut stdin_events = vec![];
    let mut immediate_events = vec![];
    let mut pipe_events = vec![];
    let mut stdin_ready = None;

    for event in fd_events {
        match event.descriptor {
            Descriptor::Stdin if event.r#type == wasi::__WASI_EVENTTYPE_FD_READ => {
                // Cache the non-emptiness for better performance.
                let immediate = stdin_ready.get_or_insert_with(stdin_nonempty);
                if *immediate {
                    immediate_events.push(event)
                } else {
                    stdin_events.push(event)
                }
            }
            Descriptor::Stdin | Descriptor::Stderr | Descriptor::Stdout => {
                immediate_events.push(event)
            }
            Descriptor::OsHandle(os_handle) => {
                let ftype = unsafe { winx::file::get_file_type(os_handle.as_raw_handle()) }?;
                if ftype.is_unknown() || ftype.is_char() {
                    error!("poll_oneoff: unsupported file type: {:?}", ftype);
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

    // Process all the events that do not require waiting.
    if !immediate_events.is_empty() {
        trace!("    | have immediate events, will return immediately");
        for mut event in immediate_events {
            handle_rw_event(event, events);
        }
    } else if !stdin_events.is_empty() {
        // REVIEW: is there a better place to document this? Perhaps in
        // `struct PollStdin`?
        //
        // If there's a request to poll stdin, we spin up a separate thread to
        // waiting for data to arrive on stdin. This thread will not terminate.
        //
        // TODO more explain why this way
        trace!("     | passively waiting on stdin");
        let dur = timeout.map(|t| t.1);
        let state = STDIN_POLL.poll(dur);
        for event in stdin_events {
            match state {
                PollState::Ready => handle_rw_event(event, events),
                PollState::Closed => { /* error? FIXME */ }
                PollState::TimedOut => { /* FIXME */ }
                PollState::Error(ref e) => {
                    error!("PollState error");
                    handle_error_event(event, Error::ENOTSUP /*FIXME*/, events);
                }
            }
        }
    } else if !pipe_events.is_empty() {
        trace!("     | actively polling stdin or pipes");
        match timeout {
            Some((event, dur)) => {
                warn!("Polling pipes not supported on Windows, will just time out.");
                return Ok(handle_timeout(event, dur, events));
            }
            None => {
                error!("Polling only pipes with no timeout not supported on Windows.");
                return Err(Error::ENOTSUP);
            }
        }
        // TODO remove these old comments!!!
        // There are some stdin or pipe poll requests and there's no data available immediately

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

        // avoid issuing more syscalls if we're requested to return immediately
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
