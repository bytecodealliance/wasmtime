//! The WASI embedding API definitions for Wasmtime.

use anyhow::Result;
use cap_std::ambient_authority;
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::slice;
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
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    preopens: Vec<(Dir, PathBuf)>,
    inherit_args: bool,
    inherit_env: bool,
    inherit_stdin: bool,
    inherit_stdout: bool,
    inherit_stderr: bool,
}

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
        if self.inherit_stdin {
            builder = builder.inherit_stdin();
        } else if let Some(file) = self.stdin {
            let file = cap_std::fs::File::from_std(file);
            let file = wasi_cap_std_sync::file::File::from_cap_std(file);
            builder = builder.stdin(Box::new(file));
        }
        if self.inherit_stdout {
            builder = builder.inherit_stdout();
        } else if let Some(file) = self.stdout {
            let file = cap_std::fs::File::from_std(file);
            let file = wasi_cap_std_sync::file::File::from_cap_std(file);
            builder = builder.stdout(Box::new(file));
        }
        if self.inherit_stderr {
            builder = builder.inherit_stderr();
        } else if let Some(file) = self.stderr {
            let file = cap_std::fs::File::from_std(file);
            let file = wasi_cap_std_sync::file::File::from_cap_std(file);
            builder = builder.stderr(Box::new(file));
        }
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
pub extern "C" fn wasi_config_delete(_config: Box<wasi_config_t>) {}

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

    config.stdin = Some(file);
    config.inherit_stdin = false;

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdin(config: &mut wasi_config_t) {
    config.stdin = None;
    config.inherit_stdin = true;
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

    config.stdout = Some(file);
    config.inherit_stdout = false;

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stdout(config: &mut wasi_config_t) {
    config.stdout = None;
    config.inherit_stdout = true;
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

    (*config).stderr = Some(file);
    (*config).inherit_stderr = false;

    true
}

#[no_mangle]
pub extern "C" fn wasi_config_inherit_stderr(config: &mut wasi_config_t) {
    config.stderr = None;
    config.inherit_stderr = true;
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
