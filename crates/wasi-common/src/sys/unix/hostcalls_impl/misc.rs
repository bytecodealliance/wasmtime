#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::hostcalls_impl::{ClockEventData, FdEventData};
use crate::wasi::{self, WasiError, WasiResult};
use std::io;
use yanix::clock::{clock_getres, clock_gettime, ClockId};

fn wasi_clock_id_to_unix(clock_id: wasi::__wasi_clockid_t) -> WasiResult<ClockId> {
    // convert the supported clocks to libc types, or return EINVAL
    match clock_id {
        wasi::__WASI_CLOCKID_REALTIME => Ok(ClockId::Realtime),
        wasi::__WASI_CLOCKID_MONOTONIC => Ok(ClockId::Monotonic),
        wasi::__WASI_CLOCKID_PROCESS_CPUTIME_ID => Ok(ClockId::ProcessCPUTime),
        wasi::__WASI_CLOCKID_THREAD_CPUTIME_ID => Ok(ClockId::ThreadCPUTime),
        _ => Err(WasiError::EINVAL),
    }
}

pub(crate) fn clock_res_get(
    clock_id: wasi::__wasi_clockid_t,
) -> WasiResult<wasi::__wasi_timestamp_t> {
    let clock_id = wasi_clock_id_to_unix(clock_id)?;
    let timespec = clock_getres(clock_id)?;

    // convert to nanoseconds, returning EOVERFLOW in case of overflow;
    // this is freelancing a bit from the spec but seems like it'll
    // be an unusual situation to hit
    (timespec.tv_sec as wasi::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as wasi::__wasi_timestamp_t))
        .map_or(Err(WasiError::EOVERFLOW), |resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                Err(WasiError::EINVAL)
            } else {
                Ok(resolution)
            }
        })
}

pub(crate) fn clock_time_get(
    clock_id: wasi::__wasi_clockid_t,
) -> WasiResult<wasi::__wasi_timestamp_t> {
    let clock_id = wasi_clock_id_to_unix(clock_id)?;
    let timespec = clock_gettime(clock_id)?;

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as wasi::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as wasi::__wasi_timestamp_t))
        .map_or(Err(WasiError::EOVERFLOW), Ok)
}

pub(crate) fn poll_oneoff(
    timeout: Option<ClockEventData>,
    fd_events: Vec<FdEventData>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> WasiResult<()> {
    use std::{convert::TryInto, os::unix::prelude::AsRawFd};
    use yanix::poll::{poll, PollFd, PollFlags};

    if fd_events.is_empty() && timeout.is_none() {
        return Ok(());
    }

    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = PollFlags::empty();
            match event.r#type {
                wasi::__WASI_EVENTTYPE_FD_READ => flags.insert(PollFlags::POLLIN),
                wasi::__WASI_EVENTTYPE_FD_WRITE => flags.insert(PollFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            unsafe { PollFd::new(event.descriptor.as_raw_fd(), flags) }
        })
        .collect();

    let poll_timeout = timeout.map_or(-1, |timeout| {
        let delay = timeout.delay / 1_000_000; // poll syscall requires delay to expressed in milliseconds
        delay.try_into().unwrap_or(libc::c_int::max_value())
    });
    log::debug!("poll_oneoff poll_timeout = {:?}", poll_timeout);

    let ready = loop {
        match poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                let last_err = io::Error::last_os_error();
                if last_err.raw_os_error().unwrap() == libc::EINTR {
                    continue;
                }
                return Err(last_err.into());
            }
            Ok(ready) => break ready,
        }
    };

    Ok(if ready == 0 {
        poll_oneoff_handle_timeout_event(timeout.expect("timeout should not be None"), events)
    } else {
        let ready_events = fd_events.into_iter().zip(poll_fds.into_iter()).take(ready);
        poll_oneoff_handle_fd_event(ready_events, events)?
    })
}

fn poll_oneoff_handle_timeout_event(
    timeout: ClockEventData,
    events: &mut Vec<wasi::__wasi_event_t>,
) {
    events.push(wasi::__wasi_event_t {
        userdata: timeout.userdata,
        error: wasi::__WASI_ERRNO_SUCCESS,
        r#type: wasi::__WASI_EVENTTYPE_CLOCK,
        fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
            flags: 0,
            nbytes: 0,
        },
    });
}

fn poll_oneoff_handle_fd_event<'a>(
    ready_events: impl Iterator<Item = (FdEventData<'a>, yanix::poll::PollFd)>,
    events: &mut Vec<wasi::__wasi_event_t>,
) -> WasiResult<()> {
    use crate::fdentry::Descriptor;
    use std::{convert::TryInto, os::unix::prelude::AsRawFd};
    use yanix::{file::fionread, poll::PollFlags};

    fn query_nbytes(fd: &Descriptor) -> WasiResult<u64> {
        // fionread may overflow for large files, so use another way for regular files.
        if let Descriptor::OsHandle(os_handle) = fd {
            let meta = os_handle.metadata()?;
            if meta.file_type().is_file() {
                use yanix::file::tell;
                let len = meta.len();
                let host_offset = unsafe { tell(os_handle.as_raw_fd())? };
                return Ok(len - host_offset);
            }
        }
        unsafe { Ok(fionread(fd.as_raw_fd())?.into()) }
    }

    for (fd_event, poll_fd) in ready_events {
        log::debug!("poll_oneoff_handle_fd_event fd_event = {:?}", fd_event);
        log::debug!("poll_oneoff_handle_fd_event poll_fd = {:?}", poll_fd);

        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };

        log::debug!("poll_oneoff_handle_fd_event revents = {:?}", revents);

        let nbytes = if fd_event.r#type == wasi::__WASI_EVENTTYPE_FD_READ {
            query_nbytes(fd_event.descriptor)?
        } else {
            0
        };

        let output_event = if revents.contains(PollFlags::POLLNVAL) {
            wasi::__wasi_event_t {
                userdata: fd_event.userdata,
                error: wasi::__WASI_ERRNO_BADF,
                r#type: fd_event.r#type,
                fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                    nbytes: 0,
                    flags: wasi::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLERR) {
            wasi::__wasi_event_t {
                userdata: fd_event.userdata,
                error: wasi::__WASI_ERRNO_IO,
                r#type: fd_event.r#type,
                fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                    nbytes: 0,
                    flags: wasi::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLHUP) {
            wasi::__wasi_event_t {
                userdata: fd_event.userdata,
                error: wasi::__WASI_ERRNO_SUCCESS,
                r#type: fd_event.r#type,
                fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                    nbytes: 0,
                    flags: wasi::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP,
                },
            }
        } else if revents.contains(PollFlags::POLLIN) | revents.contains(PollFlags::POLLOUT) {
            wasi::__wasi_event_t {
                userdata: fd_event.userdata,
                error: wasi::__WASI_ERRNO_SUCCESS,
                r#type: fd_event.r#type,
                fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                    nbytes: nbytes.try_into()?,
                    flags: 0,
                },
            }
        } else {
            continue;
        };

        events.push(output_event);
    }

    Ok(())
}
