//! The WASI embedding API definitions for Wasmtime.

use crate::wasm_byte_vec_t;
use anyhow::Result;
use cap_std::ambient_authority;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::slice;
use wasi_common::pipe::ReadPipe;
use wasmtime_wasi::{
    sync::{Dir, TcpListener, WasiCtxBuilder},
    WasiCtx,
};

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
#[derive(Default)]
pub struct wasi_config_t {
    args: Vec<Vec<u8>>,
    env: Vec<(Vec<u8>, Vec<u8>)>,
    stdin: WasiConfigReadPipe,
    stdout: WasiConfigWritePipe,
    stderr: WasiConfigWritePipe,
    preopen_dirs: Vec<(Dir, PathBuf)>,
    preopen_sockets: HashMap<u32, TcpListener>,
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
}

wasmtime_c_api_macros::declare_own!(wasi_config_t);

impl wasi_config_t {
    pub fn into_wasi_ctx(self) -> Result<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();
        if self.inherit_args {
            builder.inherit_args()?;
        } else if !self.args.is_empty() {
            let args = self
                .args
                .into_iter()
                .map(|bytes| Ok(String::from_utf8(bytes)?))
                .collect::<Result<Vec<String>>>()?;
            builder.args(&args)?;
        }
        if self.inherit_env {
            builder.inherit_env()?;
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
            builder.envs(&env)?;
        }
        match self.stdin {
            WasiConfigReadPipe::None => {}
            WasiConfigReadPipe::Inherit => {
                builder.inherit_stdin();
            }
            WasiConfigReadPipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stdin(Box::new(file));
            }
            WasiConfigReadPipe::Bytes(binary) => {
                let binary = ReadPipe::from(binary);
                builder.stdin(Box::new(binary));
            }
        };
        match self.stdout {
            WasiConfigWritePipe::None => {}
            WasiConfigWritePipe::Inherit => {
                builder.inherit_stdout();
            }
            WasiConfigWritePipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stdout(Box::new(file));
            }
        };
        match self.stderr {
            WasiConfigWritePipe::None => {}
            WasiConfigWritePipe::Inherit => {
                builder.inherit_stderr();
            }
            WasiConfigWritePipe::File(file) => {
                let file = cap_std::fs::File::from_std(file);
                let file = wasi_cap_std_sync::file::File::from_cap_std(file);
                builder.stderr(Box::new(file));
            }
        };
        for (dir, path) in self.preopen_dirs {
            builder.preopened_dir(dir, path)?;
        }
        for (fd_num, listener) in self.preopen_sockets {
            builder.preopened_socket(fd_num, listener)?;
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

    (*config).preopen_dirs.push((dir, guest_path.to_owned()));

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_preopen_socket(
    config: &mut wasi_config_t,
    fd_num: u32,
    host_port: *const c_char,
) -> bool {
    let address = match cstr_to_str(host_port) {
        Some(s) => s,
        None => return false,
    };
    let listener = match std::net::TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(_) => return false,
    };

    if let Err(_) = listener.set_nonblocking(true) {
        return false;
    }

    // Caller cannot call in more than once with the same FD number so return an error.
    if (*config).preopen_sockets.contains_key(&fd_num) {
        return false;
    }

    (*config)
        .preopen_sockets
        .insert(fd_num, TcpListener::from_std(listener));

    true
}
