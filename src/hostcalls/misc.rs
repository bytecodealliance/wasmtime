#![allow(non_camel_case_types)]
use super::return_enc_errno;
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::sys::hostcalls_impl;
use crate::{host, wasm32};
use log::trace;
use std::convert::{identity, TryFrom};

use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn args_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "args_get(argv_ptr={:#x?}, argv_buf={:#x?})",
        argv_ptr,
        argv_buf,
    );

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in wasi_ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        if let Err(e) = enc_slice_of(memory, arg_bytes, arg_ptr) {
            return return_enc_errno(e);
        }

        argv.push(arg_ptr);

        argv_buf_offset = if let Some(new_offset) = argv_buf_offset.checked_add(
            wasm32::uintptr_t::try_from(arg_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return return_enc_errno(host::__WASI_EOVERFLOW);
        }
    }

    let ret = enc_slice_of(memory, argv.as_slice(), argv_ptr)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn args_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
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

    if let Err(e) = enc_usize_byref(memory, argc_ptr, argc) {
        return return_enc_errno(e);
    }

    trace!("     | *argv_buf_size_ptr={:?}", argv_size);

    if let Err(e) = enc_usize_byref(memory, argv_buf_size_ptr, argv_size) {
        return return_enc_errno(e);
    }

    return_enc_errno(host::__WASI_ESUCCESS)
}

#[wasi_common_cbindgen]
pub fn environ_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "environ_get(environ_ptr={:#x?}, environ_buf={:#x?})",
        environ_ptr,
        environ_buf,
    );

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in wasi_ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        if let Err(e) = enc_slice_of(memory, env_bytes, env_ptr) {
            return return_enc_errno(e);
        }

        environ.push(env_ptr);

        environ_buf_offset = if let Some(new_offset) = environ_buf_offset.checked_add(
            wasm32::uintptr_t::try_from(env_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return return_enc_errno(host::__WASI_EOVERFLOW);
        }
    }

    let ret = enc_slice_of(memory, environ.as_slice(), environ_ptr)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn environ_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "environ_sizes_get(environ_count_ptr={:#x?}, environ_size_ptr={:#x?})",
        environ_count_ptr,
        environ_size_ptr,
    );

    let environ_count = wasi_ctx.env.len();
    let ret = if let Some(environ_size) = wasi_ctx.env.iter().try_fold(0, |acc: u32, pair| {
        acc.checked_add(pair.as_bytes_with_nul().len() as u32)
    }) {
        trace!("     | *environ_count_ptr={:?}", environ_count);

        if let Err(e) = enc_usize_byref(memory, environ_count_ptr, environ_count) {
            return return_enc_errno(e);
        }

        trace!("     | *environ_size_ptr={:?}", environ_size);

        if let Err(e) = enc_usize_byref(memory, environ_size_ptr, environ_size as usize) {
            return return_enc_errno(e);
        }

        host::__WASI_ESUCCESS
    } else {
        host::__WASI_EOVERFLOW
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn proc_exit(rval: wasm32::__wasi_exitcode_t) -> () {
    trace!("proc_exit(rval={:?})", rval);
    // TODO: Rather than call std::process::exit here, we should trigger a
    // stack unwind similar to a trap.
    std::process::exit(dec_exitcode(rval) as i32);
}

#[wasi_common_cbindgen]
pub fn proc_raise(
    _wasi_ctx: &WasiCtx,
    _memory: &mut [u8],
    _sig: wasm32::__wasi_signal_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("proc_raise")
}

#[wasi_common_cbindgen]
pub fn random_get(
    memory: &mut [u8],
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    trace!("random_get(buf_ptr={:#x?}, buf_len={:?})", buf_ptr, buf_len);

    use rand::{thread_rng, RngCore};

    let buf = match dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len) {
        Ok(buf) => buf,
        Err(e) => return return_enc_errno(e),
    };

    thread_rng().fill_bytes(buf);

    return_enc_errno(host::__WASI_ESUCCESS)
}

#[wasi_common_cbindgen]
pub fn clock_res_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "clock_res_get(clock_id={:?}, resolution_ptr={:#x?})",
        clock_id,
        resolution_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let resolution = match hostcalls_impl::clock_res_get(clock_id) {
        Ok(resolution) => resolution,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *resolution_ptr={:?}", resolution);

    let ret = enc_timestamp_byref(memory, resolution_ptr, resolution)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn clock_time_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    // ignored for now, but will be useful once we put optional limits on precision to reduce side
    // channels
    precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "clock_time_get(clock_id={:?}, precision={:?}, time_ptr={:#x?})",
        clock_id,
        precision,
        time_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let time = match hostcalls_impl::clock_time_get(clock_id) {
        Ok(time) => time,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *time_ptr={:?}", time);

    let ret = enc_timestamp_byref(memory, time_ptr, time)
        .map(|_| host::__WASI_ESUCCESS)
        .unwrap_or_else(identity);

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn poll_oneoff(
    memory: &mut [u8],
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    trace!(
        "poll_oneoff(input={:#x?}, output={:#x?}, nsubscriptions={}, nevents={:#x?})",
        input,
        output,
        nsubscriptions,
        nevents,
    );

    if nsubscriptions as u64 > wasm32::__wasi_filesize_t::max_value() {
        return return_enc_errno(host::__WASI_EINVAL);
    }
    if let Err(e) = enc_pointee(memory, nevents, 0) {
        return return_enc_errno(e);
    }
    let input_slice =
        match dec_slice_of::<wasm32::__wasi_subscription_t>(memory, input, nsubscriptions) {
            Ok(input_slice) => input_slice,
            Err(e) => return return_enc_errno(e),
        };
    let input: Vec<_> = input_slice.iter().map(dec_subscription).collect();
    let output_slice =
        match dec_slice_of_mut::<wasm32::__wasi_event_t>(memory, output, nsubscriptions) {
            Ok(output_slice) => output_slice,
            Err(e) => return return_enc_errno(e),
        };
    let events_count = match hostcalls_impl::poll_oneoff(input, output_slice) {
        Ok(events_count) => events_count,
        Err(e) => return return_enc_errno(e),
    };

    trace!("     | *nevents={:?}", events_count);

    let ret = match enc_pointee(memory, nevents, events_count) {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}

#[wasi_common_cbindgen]
pub fn sched_yield() -> wasm32::__wasi_errno_t {
    trace!("sched_yield()");

    let ret = match hostcalls_impl::sched_yield() {
        Ok(()) => host::__WASI_ESUCCESS,
        Err(e) => e,
    };

    return_enc_errno(ret)
}
