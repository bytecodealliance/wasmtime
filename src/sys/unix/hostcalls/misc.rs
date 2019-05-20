#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use super::host_impl;

use crate::memory::*;
use crate::{host, wasm32};

use nix::convert_ioctl_res;
use nix::libc::{self, c_int};
use std::cmp;
use std::time::SystemTime;
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn clock_res_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_getres(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return host_impl::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map_or(wasm32::__WASI_EOVERFLOW, |resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                wasm32::__WASI_EINVAL
            } else {
                enc_timestamp_byref(memory, resolution_ptr, resolution)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            }
        })
}

#[wasi_common_cbindgen]
pub fn clock_time_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    // ignored for now, but will be useful once we put optional limits on precision to reduce side
    // channels
    _precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
    let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
    if res != 0 {
        return host_impl::errno_from_nix(nix::errno::Errno::last());
    }

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map_or(wasm32::__WASI_EOVERFLOW, |time| {
            enc_timestamp_byref(memory, time_ptr, time)
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
        })
}

#[wasi_common_cbindgen]
pub fn poll_oneoff(
    memory: &mut [u8],
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    if nsubscriptions as u64 > wasm32::__wasi_filesize_t::max_value() {
        return wasm32::__WASI_EINVAL;
    }
    enc_pointee(memory, nevents, 0).unwrap();
    let input_slice =
        dec_slice_of::<wasm32::__wasi_subscription_t>(memory, input, nsubscriptions).unwrap();

    let input: Vec<_> = input_slice.iter().map(|x| dec_subscription(x)).collect();

    let output_slice =
        dec_slice_of_mut::<wasm32::__wasi_event_t>(memory, output, nsubscriptions).unwrap();

    let timeout = input
        .iter()
        .filter_map(|event| match event {
            Ok(event) if event.type_ == wasm32::__WASI_EVENTTYPE_CLOCK => Some(ClockEventData {
                delay: wasi_clock_to_relative_ns_delay(unsafe { event.u.clock }) / 1_000_000,
                userdata: event.userdata,
            }),
            _ => None,
        })
        .min_by_key(|event| event.delay);
    let fd_events: Vec<_> = input
        .iter()
        .filter_map(|event| match event {
            Ok(event)
                if event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ
                    || event.type_ == wasm32::__WASI_EVENTTYPE_FD_WRITE =>
            {
                Some(FdEventData {
                    fd: unsafe { event.u.fd_readwrite.fd } as c_int,
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            _ => None,
        })
        .collect();
    if fd_events.is_empty() && timeout.is_none() {
        return wasm32::__WASI_ESUCCESS;
    }
    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = nix::poll::EventFlags::empty();
            match event.type_ {
                wasm32::__WASI_EVENTTYPE_FD_READ => flags.insert(nix::poll::EventFlags::POLLIN),
                wasm32::__WASI_EVENTTYPE_FD_WRITE => flags.insert(nix::poll::EventFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            nix::poll::PollFd::new(event.fd, flags)
        })
        .collect();
    let timeout = timeout.map(|ClockEventData { delay, userdata }| ClockEventData {
        delay: cmp::min(delay, c_int::max_value() as u128),
        userdata,
    });
    let poll_timeout = timeout.map_or(-1, |timeout| timeout.delay as c_int);
    let ready = loop {
        match nix::poll::poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                if nix::errno::Errno::last() == nix::errno::Errno::EINTR {
                    continue;
                }
                return host_impl::errno_from_nix(nix::errno::Errno::last());
            }
            Ok(ready) => break ready as usize,
        }
    };
    let events_count = if ready == 0 {
        poll_oneoff_handle_timeout_event(output_slice, timeout)
    } else {
        let events = fd_events.iter().zip(poll_fds.iter()).take(ready);
        poll_oneoff_handle_fd_event(output_slice, events)
    };
    if let Err(e) = enc_pointee(memory, nevents, events_count) {
        return enc_errno(e);
    }
    wasm32::__WASI_ESUCCESS
}

#[wasi_common_cbindgen]
pub fn sched_yield() -> wasm32::__wasi_errno_t {
    unsafe { libc::sched_yield() };
    wasm32::__WASI_ESUCCESS
}

// define the `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`
nix::ioctl_read_bad!(fionread, nix::libc::FIONREAD, c_int);

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> u128 {
    if wasi_clock.flags != wasm32::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return wasi_clock.timeout as u128;
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Current date is before the epoch")
        .as_nanos();
    let deadline = wasi_clock.timeout as u128;
    deadline.saturating_sub(now)
}

#[derive(Debug, Copy, Clone)]
struct ClockEventData {
    delay: u128,
    userdata: host::__wasi_userdata_t,
}
#[derive(Debug, Copy, Clone)]
struct FdEventData {
    fd: c_int,
    type_: host::__wasi_eventtype_t,
    userdata: host::__wasi_userdata_t,
}

fn poll_oneoff_handle_timeout_event(
    output_slice: &mut [wasm32::__wasi_event_t],
    timeout: Option<ClockEventData>,
) -> wasm32::size_t {
    if let Some(ClockEventData { userdata, .. }) = timeout {
        let output_event = host::__wasi_event_t {
            userdata,
            type_: wasm32::__WASI_EVENTTYPE_CLOCK,
            error: wasm32::__WASI_ESUCCESS,
            u: host::__wasi_event_t___wasi_event_u {
                fd_readwrite: host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                    nbytes: 0,
                    flags: 0,
                },
            },
        };
        output_slice[0] = enc_event(output_event);
        1
    } else {
        // shouldn't happen
        0
    }
}

fn poll_oneoff_handle_fd_event<'t>(
    output_slice: &mut [wasm32::__wasi_event_t],
    events: impl Iterator<Item = (&'t FdEventData, &'t nix::poll::PollFd)>,
) -> wasm32::size_t {
    let mut output_slice_cur = output_slice.iter_mut();
    let mut revents_count = 0;
    for (fd_event, poll_fd) in events {
        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };
        let mut nbytes = 0;
        if fd_event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ {
            let _ = unsafe { fionread(fd_event.fd, &mut nbytes) };
        }
        let output_event = if revents.contains(nix::poll::EventFlags::POLLNVAL) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EBADF,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLERR) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EIO,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLHUP) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLIN)
            | revents.contains(nix::poll::EventFlags::POLLOUT)
        {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: nbytes as host::__wasi_filesize_t,
                            flags: 0,
                        },
                },
            }
        } else {
            continue;
        };
        *output_slice_cur.next().unwrap() = enc_event(output_event);
        revents_count += 1;
    }
    revents_count
}
