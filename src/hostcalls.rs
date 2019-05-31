#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::wasm32;
use std::convert::TryFrom;

use wasi_common_cbindgen::wasi_common_cbindgen;

pub use crate::sys::hostcalls::*;

#[wasi_common_cbindgen]
pub fn args_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in wasi_ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        if let Err(e) = enc_slice_of(memory, arg_bytes, arg_ptr) {
            return enc_errno(e);
        }

        argv.push(arg_ptr);

        argv_buf_offset = if let Some(new_offset) = argv_buf_offset.checked_add(
            wasm32::uintptr_t::try_from(arg_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    enc_slice_of(memory, argv.as_slice(), argv_ptr)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn args_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let argc = wasi_ctx.args.len();
    let argv_size = wasi_ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    if let Err(e) = enc_usize_byref(memory, argc_ptr, argc) {
        return enc_errno(e);
    }
    if let Err(e) = enc_usize_byref(memory, argv_buf_size_ptr, argv_size) {
        return enc_errno(e);
    }
    wasm32::__WASI_ESUCCESS
}

#[wasi_common_cbindgen]
pub fn environ_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in wasi_ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        if let Err(e) = enc_slice_of(memory, env_bytes, env_ptr) {
            return enc_errno(e);
        }

        environ.push(env_ptr);

        environ_buf_offset = if let Some(new_offset) = environ_buf_offset.checked_add(
            wasm32::uintptr_t::try_from(env_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }

    enc_slice_of(memory, environ.as_slice(), environ_ptr)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn environ_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let environ_count = wasi_ctx.env.len();
    if let Some(environ_size) = wasi_ctx.env.iter().try_fold(0, |acc: u32, pair| {
        acc.checked_add(pair.as_bytes_with_nul().len() as u32)
    }) {
        if let Err(e) = enc_usize_byref(memory, environ_count_ptr, environ_count) {
            return enc_errno(e);
        }
        if let Err(e) = enc_usize_byref(memory, environ_size_ptr, environ_size as usize) {
            return enc_errno(e);
        }
        wasm32::__WASI_ESUCCESS
    } else {
        wasm32::__WASI_EOVERFLOW
    }
}

#[wasi_common_cbindgen]
pub fn proc_exit(rval: wasm32::__wasi_exitcode_t) -> () {
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
    use rand::{thread_rng, RngCore};

    let buf = match dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len) {
        Ok(buf) => buf,
        Err(e) => return enc_errno(e),
    };

    thread_rng().fill_bytes(buf);

    return wasm32::__WASI_ESUCCESS;
}
