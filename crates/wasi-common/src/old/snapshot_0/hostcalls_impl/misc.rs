#![allow(non_camel_case_types)]
use crate::old::snapshot_0::ctx::WasiCtx;
use crate::old::snapshot_0::fdentry::Descriptor;
use crate::old::snapshot_0::memory::*;
use crate::old::snapshot_0::sys::hostcalls_impl;
use crate::old::snapshot_0::wasi::{self, WasiError, WasiResult};
use crate::old::snapshot_0::wasi32;
use log::{error, trace};
use std::convert::TryFrom;

pub(crate) fn args_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argv_ptr: wasi32::uintptr_t,
    argv_buf: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "args_get(argv_ptr={:#x?}, argv_buf={:#x?})",
        argv_ptr,
        argv_buf,
    );

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in &wasi_ctx.args {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        enc_slice_of_u8(memory, arg_bytes, arg_ptr)?;

        argv.push(arg_ptr);

        let len = wasi32::uintptr_t::try_from(arg_bytes.len())?;
        argv_buf_offset = argv_buf_offset
            .checked_add(len)
            .ok_or(WasiError::EOVERFLOW)?;
    }

    enc_slice_of_wasi32_uintptr(memory, argv.as_slice(), argv_ptr)
}

pub(crate) fn args_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argc_ptr: wasi32::uintptr_t,
    argv_buf_size_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "args_sizes_get(argc_ptr={:#x?}, argv_buf_size_ptr={:#x?})",
        argc_ptr,
        argv_buf_size_ptr,
    );

    let argc = wasi_ctx.args.len();
    let argv_size = wasi_ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    trace!("     | *argc_ptr={:?}", argc);

    enc_usize_byref(memory, argc_ptr, argc)?;

    trace!("     | *argv_buf_size_ptr={:?}", argv_size);

    enc_usize_byref(memory, argv_buf_size_ptr, argv_size)
}

pub(crate) fn environ_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_ptr: wasi32::uintptr_t,
    environ_buf: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "environ_get(environ_ptr={:#x?}, environ_buf={:#x?})",
        environ_ptr,
        environ_buf,
    );

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in &wasi_ctx.env {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        enc_slice_of_u8(memory, env_bytes, env_ptr)?;

        environ.push(env_ptr);

        let len = wasi32::uintptr_t::try_from(env_bytes.len())?;
        environ_buf_offset = environ_buf_offset
            .checked_add(len)
            .ok_or(WasiError::EOVERFLOW)?;
    }

    enc_slice_of_wasi32_uintptr(memory, environ.as_slice(), environ_ptr)
}

pub(crate) fn environ_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_count_ptr: wasi32::uintptr_t,
    environ_size_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "environ_sizes_get(environ_count_ptr={:#x?}, environ_size_ptr={:#x?})",
        environ_count_ptr,
        environ_size_ptr,
    );

    let environ_count = wasi_ctx.env.len();
    let environ_size = wasi_ctx
        .env
        .iter()
        .try_fold(0, |acc: u32, pair| {
            acc.checked_add(pair.as_bytes_with_nul().len() as u32)
        })
        .ok_or(WasiError::EOVERFLOW)?;

    trace!("     | *environ_count_ptr={:?}", environ_count);

    enc_usize_byref(memory, environ_count_ptr, environ_count)?;

    trace!("     | *environ_size_ptr={:?}", environ_size);

    enc_usize_byref(memory, environ_size_ptr, environ_size as usize)
}

pub(crate) fn random_get(
    _wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    buf_ptr: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
) -> WasiResult<()> {
    trace!("random_get(buf_ptr={:#x?}, buf_len={:?})", buf_ptr, buf_len);

    let buf = dec_slice_of_mut_u8(memory, buf_ptr, buf_len)?;

    getrandom::getrandom(buf).map_err(|err| {
        error!("getrandom failure: {:?}", err);
        WasiError::EIO
    })
}

pub(crate) fn clock_res_get(
    _wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    clock_id: wasi::__wasi_clockid_t,
    resolution_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "clock_res_get(clock_id={:?}, resolution_ptr={:#x?})",
        clock_id,
        resolution_ptr,
    );

    let resolution = hostcalls_impl::clock_res_get(clock_id)?;

    trace!("     | *resolution_ptr={:?}", resolution);

    enc_timestamp_byref(memory, resolution_ptr, resolution)
}

pub(crate) fn clock_time_get(
    _wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    clock_id: wasi::__wasi_clockid_t,
    precision: wasi::__wasi_timestamp_t,
    time_ptr: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "clock_time_get(clock_id={:?}, precision={:?}, time_ptr={:#x?})",
        clock_id,
        precision,
        time_ptr,
    );

    let time = hostcalls_impl::clock_time_get(clock_id)?;

    trace!("     | *time_ptr={:?}", time);

    enc_timestamp_byref(memory, time_ptr, time)
}

pub(crate) fn sched_yield(_wasi_ctx: &WasiCtx, _memory: &mut [u8]) -> WasiResult<()> {
    trace!("sched_yield()");

    std::thread::yield_now();

    Ok(())
}

pub(crate) fn poll_oneoff(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    input: wasi32::uintptr_t,
    output: wasi32::uintptr_t,
    nsubscriptions: wasi32::size_t,
    nevents: wasi32::uintptr_t,
) -> WasiResult<()> {
    trace!(
        "poll_oneoff(input={:#x?}, output={:#x?}, nsubscriptions={}, nevents={:#x?})",
        input,
        output,
        nsubscriptions,
        nevents,
    );

    if u64::from(nsubscriptions) > wasi::__wasi_filesize_t::max_value() {
        return Err(WasiError::EINVAL);
    }

    enc_int_byref(memory, nevents, 0)?;

    let subscriptions = dec_subscriptions(memory, input, nsubscriptions)?;
    let mut events = Vec::new();

    let mut timeout: Option<ClockEventData> = None;
    let mut fd_events = Vec::new();
    for subscription in subscriptions {
        match subscription.u.tag {
            wasi::__WASI_EVENTTYPE_CLOCK => {
                let clock = unsafe { subscription.u.u.clock };
                let delay = wasi_clock_to_relative_ns_delay(clock)?;

                log::debug!("poll_oneoff event.u.clock = {:?}", clock);
                log::debug!("poll_oneoff delay = {:?}ns", delay);

                let current = ClockEventData {
                    delay,
                    userdata: subscription.userdata,
                };
                let timeout = timeout.get_or_insert(current);
                if current.delay < timeout.delay {
                    *timeout = current;
                }
            }

            wasi::__WASI_EVENTTYPE_FD_READ => {
                let wasi_fd = unsafe { subscription.u.u.fd_read.file_descriptor };
                let rights = wasi::__WASI_RIGHTS_FD_READ | wasi::__WASI_RIGHTS_POLL_FD_READWRITE;
                match unsafe {
                    wasi_ctx
                        .get_fd_entry(wasi_fd)
                        .and_then(|fe| fe.as_descriptor(rights, 0))
                } {
                    Ok(descriptor) => fd_events.push(FdEventData {
                        descriptor,
                        r#type: wasi::__WASI_EVENTTYPE_FD_READ,
                        userdata: subscription.userdata,
                    }),
                    Err(err) => {
                        let event = wasi::__wasi_event_t {
                            userdata: subscription.userdata,
                            error: err.as_raw_errno(),
                            r#type: wasi::__WASI_EVENTTYPE_FD_READ,
                            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                                nbytes: 0,
                                flags: 0,
                            },
                        };
                        events.push(event);
                    }
                };
            }

            wasi::__WASI_EVENTTYPE_FD_WRITE => {
                let wasi_fd = unsafe { subscription.u.u.fd_write.file_descriptor };
                let rights = wasi::__WASI_RIGHTS_FD_WRITE | wasi::__WASI_RIGHTS_POLL_FD_READWRITE;
                match unsafe {
                    wasi_ctx
                        .get_fd_entry(wasi_fd)
                        .and_then(|fe| fe.as_descriptor(rights, 0))
                } {
                    Ok(descriptor) => fd_events.push(FdEventData {
                        descriptor,
                        r#type: wasi::__WASI_EVENTTYPE_FD_WRITE,
                        userdata: subscription.userdata,
                    }),
                    Err(err) => {
                        let event = wasi::__wasi_event_t {
                            userdata: subscription.userdata,
                            error: err.as_raw_errno(),
                            r#type: wasi::__WASI_EVENTTYPE_FD_WRITE,
                            fd_readwrite: wasi::__wasi_event_fd_readwrite_t {
                                nbytes: 0,
                                flags: 0,
                            },
                        };
                        events.push(event);
                    }
                };
            }
            _ => unreachable!(),
        }
    }

    log::debug!("poll_oneoff timeout = {:?}", timeout);
    log::debug!("poll_oneoff fd_events = {:?}", fd_events);

    hostcalls_impl::poll_oneoff(timeout, fd_events, &mut events)?;

    let events_count = u32::try_from(events.len()).map_err(|_| WasiError::EOVERFLOW)?;

    enc_events(memory, output, nsubscriptions, events)?;

    trace!("     | *nevents={:?}", events_count);

    enc_int_byref(memory, nevents, events_count)
}

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: wasi::__wasi_subscription_clock_t,
) -> WasiResult<u128> {
    use std::time::SystemTime;

    if wasi_clock.flags != wasi::__WASI_SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME {
        return Ok(u128::from(wasi_clock.timeout));
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| WasiError::ENOTCAPABLE)?
        .as_nanos();
    let deadline = u128::from(wasi_clock.timeout);
    Ok(deadline.saturating_sub(now))
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClockEventData {
    pub(crate) delay: u128, // delay is expressed in nanoseconds
    pub(crate) userdata: wasi::__wasi_userdata_t,
}

#[derive(Debug)]
pub(crate) struct FdEventData<'a> {
    pub(crate) descriptor: &'a Descriptor,
    pub(crate) r#type: wasi::__wasi_eventtype_t,
    pub(crate) userdata: wasi::__wasi_userdata_t,
}

pub(crate) fn proc_exit(_wasi_ctx: &WasiCtx, _memory: &mut [u8], rval: wasi::__wasi_exitcode_t) {
    trace!("proc_exit(rval={:?})", rval);
    // TODO: Rather than call std::process::exit here, we should trigger a
    // stack unwind similar to a trap.
    std::process::exit(rval as i32);
}

pub(crate) fn proc_raise(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sig: wasi::__wasi_signal_t,
) -> WasiResult<()> {
    unimplemented!("proc_raise")
}
