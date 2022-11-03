//! The WASI embedding API definitions for Wasmtime.

use crate::wasm_byte_vec_t;
use anyhow::Result;
use cap_std::ambient_authority;
use std::ffi::CStr;
use std::fs::File;
use std::io::Cursor;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::slice;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime_wasi::{
    sync::{Dir, WasiCtxBuilder},
    WasiCtx,
};

unsafe fn cstr_to_path<'a>(path: *const c_char) -> Option<&'a Path> {
    CStr::from_ptr(path).to_str().map(Path::new).ok()
}

unsafe fn open_file(path: *const c_char) -> Option<File> {
    File::open(cstr_to_path(path)?).ok()
}

unsafe fn create_file(path: *const c_char) -> Option<File> {
    File::create(cstr_to_path(path)?).ok()
}

#[repr(C)]
#[derive(Default)]
pub struct wasi_config_t {
    args: Vec<Vec<u8>>,
    env: Vec<(Vec<u8>, Vec<u8>)>,
    stdin: WasiConfigReadPipe,
    stdout: WasiConfigWritePipe,
    stderr: WasiConfigWritePipe,
    preopens: Vec<(Dir, PathBuf)>,
    inherit_args: bool,
    inherit_env: bool,
}

#[repr(C)]
#[derive(Default)]
pub enum WasiConfigReadPipe {
    #[default]
    None,
    Inherit,
    File(File),
    Bytes(Vec<u8>),
}

#[repr(C)]
#[derive(Default)]
pub enum WasiConfigWritePipe {
    #[default]
    None,
    Inherit,
    File(File),
    Bytes(OutBytes),
}

#[repr(C)]
pub struct wasi_out_bytes_t {
    binary: OutBytes,
}

type OutBytes = WritePipe<Cursor<Box<[u8]>>>;

wasmtime_c_api_macros::declare_own!(wasi_config_t);
wasmtime_c_api_macros::declare_own!(wasi_out_bytes_t);

impl wasi_config_t {
    pub fn into_wasi_ctx(self) -> Result<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();
        if self.inherit_args {
            builder = builder.inherit_args()?;
        } else if !self.args.is_empty() {
            let args = self
                .args
                .into_iter()
                .map(|bytes| Ok(String::from_utf8(bytes)?))
                .collect::<Result<Vec<String>>>()?;
            builder = builder.args(&args)?;
        }
        if self.inherit_env {
            builder = builder.inherit_env()?;
        } else if !self.env.is_empty() {
            let env = self
                .env
                .into_iter()
                .map(|(kbytes, vbytes)| {
                    let k = String::from_utf8(kbytes)?;
                    let v = String::from_utf8(vbytes)?;
                    Ok((k, v))
                })
                .collect::<Result<Vec<(String, String)>>>()?;
            builder = builder.envs(&env)?;
        }
        builder = match self.stdin {
            WasiConfigReadPipe::None => builder,
            WasiConfigReadPipe::Inherit => builder.inherit_stdin(),
            WasiConfigReadPipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stdin(Box::new(file))
            }
            WasiConfigReadPipe::Bytes(binary) => {
                let binary = ReadPipe::from(binary);
                builder.stdin(Box::new(binary))
            }
        };
        builder = match self.stdout {
            WasiConfigWritePipe::None => builder,
            WasiConfigWritePipe::Inherit => builder.inherit_stdout(),
            WasiConfigWritePipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stdout(Box::new(file))
            }
            WasiConfigWritePipe::Bytes(binary) => builder.stdout(Box::new(binary)),
        };
        builder = match self.stderr {
            WasiConfigWritePipe::None => builder,
            WasiConfigWritePipe::Inherit => builder.inherit_stderr(),
            WasiConfigWritePipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stderr(Box::new(file))
            }
            WasiConfigWritePipe::Bytes(binary) => builder.stderr(Box::new(binary)),
        };
        for (dir, path) in self.preopens {
            builder = builder.preopened_dir(dir, path)?;
        }
        Ok(builder.build())
    }
}

#[no_mangle]
pub extern "C" fn wasi_config_new() -> Box<wasi_config_t> {
    Box::new(wasi_config_t::default())
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_argv(
    config: &mut wasi_config_t,
    argc: c_int,
    argv: *const *const c_char,
) {
    config.args = slice::from_raw_parts(argv, argc as usize)
        .iter()
        .map(|p| CStr::from_ptr(*p).to_bytes().to_owned())
        .collect();
    config.inherit_args = false;
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_argv(config: &mut wasi_config_t) {
    config.args.clear();
    config.inherit_args = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_env(
    config: &mut wasi_config_t,
    envc: c_int,
    names: *const *const c_char,
    values: *const *const c_char,
) {
    let names = slice::from_raw_parts(names, envc as usize);
    let values = slice::from_raw_parts(values, envc as usize);

    config.env = names
        .iter()
        .map(|p| CStr::from_ptr(*p).to_bytes().to_owned())
        .zip(
            values
                .iter()
                .map(|p| CStr::from_ptr(*p).to_bytes().to_owned()),
        )
        .collect();
    config.inherit_env = false;
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_env(config: &mut wasi_config_t) {
    config.env.clear();
    config.inherit_env = true;
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

    config.stdin = WasiConfigReadPipe::File(file);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdin_bytes(
    config: &mut wasi_config_t,
    binary: &mut wasm_byte_vec_t,
) {
    let binary = binary.take();

    config.stdin = WasiConfigReadPipe::Bytes(binary);
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdin(config: &mut wasi_config_t) {
    config.stdin = WasiConfigReadPipe::Inherit;
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

    config.stdout = WasiConfigWritePipe::File(file);

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_set_stdout_bytes(
    config: &mut wasi_config_t,
    size: usize,
) -> Box<wasi_out_bytes_t> {
    let binary = vec![0; size].into_boxed_slice();
    let binary = WritePipe::new(Cursor::new(binary));

    config.stdout = WasiConfigWritePipe::Bytes(binary.clone());

    Box::new(wasi_out_bytes_t { binary })
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdout(config: &mut wasi_config_t) {
    config.stdout = WasiConfigWritePipe::Inherit;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stderr_file(
    config: &mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    config.stderr = WasiConfigWritePipe::File(file);

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_set_stderr_bytes(
    config: &mut wasi_config_t,
    size: usize,
) -> Box<wasi_out_bytes_t> {
    let binary = vec![0; size].into_boxed_slice();
    let binary = WritePipe::new(Cursor::new(binary));

    config.stderr = WasiConfigWritePipe::Bytes(binary.clone());

    Box::new(wasi_out_bytes_t { binary })
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stderr(config: &mut wasi_config_t) {
    config.stderr = WasiConfigWritePipe::Inherit;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_preopen_dir(
    config: &mut wasi_config_t,
    path: *const c_char,
    guest_path: *const c_char,
) -> bool {
    let guest_path = match cstr_to_path(guest_path) {
        Some(p) => p,
        None => return false,
    };

    let dir = match cstr_to_path(path) {
        Some(p) => match Dir::open_ambient_dir(p, ambient_authority()) {
            Ok(d) => d,
            Err(_) => return false,
        },
        None => return false,
    };

    (*config).preopens.push((dir, guest_path.to_owned()));

    true
}

#[no_mangle]
pub extern "C" fn wasi_out_bytes_take(
    out_bytes: Box<wasi_out_bytes_t>,
    ret: &mut wasm_byte_vec_t,
) -> bool {
    match out_bytes.binary.try_into_inner() {
        Ok(binary) => {
            ret.set_buffer(binary.into_inner().to_vec());

            true
        }
        Err(_) => false,
    }
}
