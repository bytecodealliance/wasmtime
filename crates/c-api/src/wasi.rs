//! The WASI embedding API definitions for Wasmtime.
use crate::{wasm_extern_t, wasm_importtype_t, wasm_store_t, wasm_trap_t, ExternHost, ExternType};
use anyhow::Result;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::slice;
use wasi_common::{
    old::snapshot_0::WasiCtxBuilder as WasiSnapshot0CtxBuilder, preopen_dir,
    WasiCtxBuilder as WasiPreview1CtxBuilder,
};
use wasmtime::{HostRef, Linker, Store, Trap};
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi as WasiPreview1};

unsafe fn cstr_to_path<'a>(path: *const c_char) -> Option<&'a Path> {
    CStr::from_ptr(path).to_str().map(Path::new).ok()
}

unsafe fn open_file(path: *const c_char) -> Option<File> {
    File::open(cstr_to_path(path)?).ok()
}

unsafe fn create_file(path: *const c_char) -> Option<File> {
    File::create(cstr_to_path(path)?).ok()
}

pub enum WasiModule {
    Snapshot0(WasiSnapshot0),
    Preview1(WasiPreview1),
}

impl WasiModule {}

#[repr(C)]
#[derive(Default)]
pub struct wasi_config_t {
    args: Vec<Vec<u8>>,
    env: Vec<(Vec<u8>, Vec<u8>)>,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    preopens: Vec<(File, PathBuf)>,
    inherit_args: bool,
    inherit_env: bool,
    inherit_stdin: bool,
    inherit_stdout: bool,
    inherit_stderr: bool,
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_new() -> *mut wasi_config_t {
    Box::into_raw(Box::new(wasi_config_t::default()))
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_delete(config: *mut wasi_config_t) {
    drop(Box::from_raw(config));
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_argv(
    config: *mut wasi_config_t,
    argc: c_int,
    argv: *const *const c_char,
) {
    (*config).args = slice::from_raw_parts(argv, argc as usize)
        .iter()
        .map(|p| CStr::from_ptr(*p).to_bytes().to_owned())
        .collect();
    (*config).inherit_args = false;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_argv(config: *mut wasi_config_t) {
    (*config).args.clear();
    (*config).inherit_args = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_env(
    config: *mut wasi_config_t,
    envc: c_int,
    names: *const *const c_char,
    values: *const *const c_char,
) {
    let names = slice::from_raw_parts(names, envc as usize);
    let values = slice::from_raw_parts(values, envc as usize);

    (*config).env = names
        .iter()
        .map(|p| CStr::from_ptr(*p).to_bytes().to_owned())
        .zip(
            values
                .iter()
                .map(|p| CStr::from_ptr(*p).to_bytes().to_owned()),
        )
        .collect();
    (*config).inherit_env = false;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_env(config: *mut wasi_config_t) {
    (*config).env.clear();
    (*config).inherit_env = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdin_file(
    config: *mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match open_file(path) {
        Some(f) => f,
        None => return false,
    };

    (*config).stdin = Some(file);
    (*config).inherit_stdin = false;

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdin(config: *mut wasi_config_t) {
    (*config).stdin = None;
    (*config).inherit_stdin = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdout_file(
    config: *mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    (*config).stdout = Some(file);
    (*config).inherit_stdout = false;

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdout(config: *mut wasi_config_t) {
    (*config).stdout = None;
    (*config).inherit_stdout = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stderr_file(
    config: *mut wasi_config_t,
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
pub unsafe extern "C" fn wasi_config_inherit_stderr(config: *mut wasi_config_t) {
    (*config).stderr = None;
    (*config).inherit_stderr = true;
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_preopen_dir(
    config: *mut wasi_config_t,
    path: *const c_char,
    guest_path: *const c_char,
) -> bool {
    let guest_path = match cstr_to_path(guest_path) {
        Some(p) => p,
        None => return false,
    };

    let dir = match cstr_to_path(path) {
        Some(p) => match preopen_dir(p) {
            Ok(d) => d,
            Err(_) => return false,
        },
        None => return false,
    };

    (*config).preopens.push((dir, guest_path.to_owned()));

    true
}

enum WasiInstance {
    Preview1(WasiPreview1),
    Snapshot0(WasiSnapshot0),
}

macro_rules! config_to_builder {
    ($builder:ident, $config:ident) => {{
        let mut builder = $builder::new();

        if $config.inherit_args {
            builder.inherit_args();
        } else if !$config.args.is_empty() {
            builder.args($config.args);
        }

        if $config.inherit_env {
            builder.inherit_env();
        } else if !$config.env.is_empty() {
            builder.envs($config.env);
        }

        if $config.inherit_stdin {
            builder.inherit_stdin();
        } else if let Some(file) = $config.stdin {
            builder.stdin(file);
        }

        if $config.inherit_stdout {
            builder.inherit_stdout();
        } else if let Some(file) = $config.stdout {
            builder.stdout(file);
        }

        if $config.inherit_stderr {
            builder.inherit_stderr();
        } else if let Some(file) = $config.stderr {
            builder.stderr(file);
        }

        for preopen in $config.preopens {
            builder.preopened_dir(preopen.0, preopen.1);
        }

        builder
    }};
}

fn create_snapshot0_instance(store: &Store, config: wasi_config_t) -> Result<WasiInstance, String> {
    Ok(WasiInstance::Snapshot0(WasiSnapshot0::new(
        store,
        config_to_builder!(WasiSnapshot0CtxBuilder, config)
            .build()
            .map_err(|e| e.to_string())?,
    )))
}

fn create_preview1_instance(store: &Store, config: wasi_config_t) -> Result<WasiInstance, String> {
    Ok(WasiInstance::Preview1(WasiPreview1::new(
        store,
        config_to_builder!(WasiPreview1CtxBuilder, config)
            .build()
            .map_err(|e| e.to_string())?,
    )))
}

#[repr(C)]
pub struct wasi_instance_t {
    wasi: WasiInstance,
    export_cache: HashMap<String, Box<wasm_extern_t>>,
}

impl wasi_instance_t {
    pub fn add_to_linker(&self, linker: &mut Linker) -> Result<()> {
        match &self.wasi {
            WasiInstance::Snapshot0(w) => w.add_to_linker(linker),
            WasiInstance::Preview1(w) => w.add_to_linker(linker),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasi_instance_new(
    store: *mut wasm_store_t,
    name: *const c_char,
    config: *mut wasi_config_t,
    trap: *mut *mut wasm_trap_t,
) -> *mut wasi_instance_t {
    let store = &(*store).store.borrow();
    let config = Box::from_raw(config);

    let result = match CStr::from_ptr(name).to_str().unwrap_or("") {
        "wasi_snapshot_preview1" => create_preview1_instance(store, *config),
        "wasi_unstable" => create_snapshot0_instance(store, *config),
        _ => Err("unsupported WASI version".into()),
    };

    match result {
        Ok(wasi) => Box::into_raw(Box::new(wasi_instance_t {
            wasi,
            export_cache: HashMap::new(),
        })),
        Err(e) => {
            (*trap) = Box::into_raw(Box::new(wasm_trap_t {
                trap: HostRef::new(Trap::new(e.to_string())),
            }));

            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasi_instance_delete(instance: *mut wasi_instance_t) {
    drop(Box::from_raw(instance));
}

#[no_mangle]
pub unsafe extern "C" fn wasi_instance_bind_import(
    instance: *mut wasi_instance_t,
    import: *const wasm_importtype_t,
) -> *const wasm_extern_t {
    // The import should be a function (WASI only exports functions)
    let func_type = match (*import).ty.ty() {
        ExternType::Func(f) => f,
        _ => return std::ptr::null_mut(),
    };

    let module = (*import).ty.module();
    let name = (*import).ty.name();

    let import = match &(*instance).wasi {
        WasiInstance::Preview1(wasi) => {
            if module != "wasi_snapshot_preview1" {
                return std::ptr::null();
            }
            wasi.get_export(name)
        }
        WasiInstance::Snapshot0(wasi) => {
            if module != "wasi_unstable" {
                return std::ptr::null();
            }

            wasi.get_export(name)
        }
    };

    match import {
        Some(export) => {
            if export.ty() != func_type {
                return std::ptr::null_mut();
            }

            &**(*instance)
                .export_cache
                .entry(name.to_string())
                .or_insert_with(|| {
                    Box::new(wasm_extern_t {
                        which: ExternHost::Func(HostRef::new(export.clone())),
                    })
                }) as *const wasm_extern_t
        }
        None => std::ptr::null_mut(),
    }
}
