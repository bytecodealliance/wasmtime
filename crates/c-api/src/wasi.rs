//! The WASI embedding API definitions for Wasmtime.

use crate::wasm_byte_vec_t;
use anyhow::Result;
use std::ffi::{c_char, CStr};
use std::fs::File;
use std::path::Path;
use std::slice;
use wasmtime_wasi::{preview1::WasiP1Ctx, WasiCtxBuilder};

unsafe fn cstr_to_path<'a>(path: *const c_char) -> Option<&'a Path> {
    CStr::from_ptr(path).to_str().map(Path::new).ok()
}

unsafe fn cstr_to_str<'a>(s: *const c_char) -> Option<&'a str> {
    CStr::from_ptr(s).to_str().ok()
}

unsafe fn open_file(path: *const c_char) -> Option<File> {
    File::open(cstr_to_path(path)?).ok()
}

unsafe fn create_file(path: *const c_char) -> Option<File> {
    File::create(cstr_to_path(path)?).ok()
}

#[repr(C)]
pub struct wasi_config_t {
    builder: WasiCtxBuilder,
}

wasmtime_c_api_macros::declare_own!(wasi_config_t);

impl wasi_config_t {
    pub fn into_wasi_ctx(mut self) -> Result<WasiP1Ctx> {
        Ok(self.builder.build_p1())
    }
}

#[no_mangle]
pub extern "C" fn wasi_config_new() -> Box<wasi_config_t> {
    Box::new(wasi_config_t {
        builder: WasiCtxBuilder::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_argv(
    config: &mut wasi_config_t,
    argc: usize,
    argv: *const *const c_char,
) -> bool {
    for arg in slice::from_raw_parts(argv, argc) {
        let arg = match CStr::from_ptr(*arg).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        };
        config.builder.arg(arg);
    }
    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_argv(config: &mut wasi_config_t) {
    config.builder.inherit_args();
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_env(
    config: &mut wasi_config_t,
    envc: usize,
    names: *const *const c_char,
    values: *const *const c_char,
) -> bool {
    let names = slice::from_raw_parts(names, envc);
    let values = slice::from_raw_parts(values, envc);

    for (k, v) in names.iter().zip(values) {
        let k = match cstr_to_str(*k) {
            Some(s) => s,
            None => return false,
        };
        let v = match cstr_to_str(*v) {
            Some(s) => s,
            None => return false,
        };
        config.builder.env(k, v);
    }
    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_env(config: &mut wasi_config_t) {
    config.builder.inherit_env();
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdin_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match open_file(path) {
        Some(f) => f,
        None => return false,
    };

    let file = tokio::fs::File::from_std(file);
    let stdin_stream =
        wasmtime_wasi::AsyncStdinStream::new(wasmtime_wasi::pipe::AsyncReadStream::new(file));
    config.builder.stdin(stdin_stream);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdin_bytes(
    config: &mut wasi_config_t,
    binary: &mut wasm_byte_vec_t,
) {
    let binary = binary.take();
    let binary = wasmtime_wasi::pipe::MemoryInputPipe::new(binary);
    config.builder.stdin(binary);
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdin(config: &mut wasi_config_t) {
    config.builder.inherit_stdin();
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdout_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    config.builder.stdout(wasmtime_wasi::OutputFile::new(file));

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdout(config: &mut wasi_config_t) {
    config.builder.inherit_stdout();
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stderr_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    config.builder.inherit_stderr();
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    config.builder.stderr(wasmtime_wasi::OutputFile::new(file));

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stderr(config: &mut wasi_config_t) {
    config.builder.inherit_stderr();
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_preopen_dir(
    config: &mut wasi_config_t,
    path: *const c_char,
    guest_path: *const c_char,
) -> bool {
    let guest_path = match cstr_to_str(guest_path) {
        Some(p) => p,
        None => return false,
    };

    let host_path = match cstr_to_path(path) {
        Some(p) => p,
        None => return false,
    };

    config
        .builder
        .preopened_dir(
            host_path,
            guest_path,
            wasmtime_wasi::DirPerms::all(),
            wasmtime_wasi::FilePerms::all(),
        )
        .is_ok()
}
