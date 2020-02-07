//! The WASI embedding API definitions for Wasmtime.
use crate::{wasm_extern_t, wasm_importtype_t, wasm_store_t, wasm_trap_t, ExternHost, ExternType};
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::slice;
use wasi_common::{preopen_dir, WasiCtxBuilder};
use wasmtime::{HostRef, Trap};
use wasmtime_wasi::Wasi;

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
pub struct wasi_config_t {
    builder: WasiCtxBuilder,
}

impl wasi_config_t {}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_new() -> *mut wasi_config_t {
    Box::into_raw(Box::new(wasi_config_t {
        builder: WasiCtxBuilder::new(),
    }))
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
    let mut config = Box::from_raw(config);
    config.builder = config.builder.args(
        slice::from_raw_parts(argv, argc as usize)
            .iter()
            .map(|a| slice::from_raw_parts(*a as *const u8, CStr::from_ptr(*a).to_bytes().len())),
    );
    std::mem::forget(config);
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_argv(config: *mut wasi_config_t) {
    let mut config = Box::from_raw(config);
    config.builder = config.builder.inherit_args();
    std::mem::forget(config);
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
    let mut config = Box::from_raw(config);

    for i in 0..envc as usize {
        config.builder = config.builder.env(
            CStr::from_ptr(names[i]).to_bytes(),
            CStr::from_ptr(values[i]).to_bytes(),
        );
    }

    std::mem::forget(config);
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_env(config: *mut wasi_config_t) {
    let mut config = Box::from_raw(config);
    config.builder = config.builder.inherit_env();
    std::mem::forget(config);
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdin(
    config: *mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match open_file(path) {
        Some(f) => f,
        None => return false,
    };

    let mut config = Box::from_raw(config);
    config.builder = config.builder.stdin(file);
    std::mem::forget(config);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdin(config: *mut wasi_config_t) {
    let mut config = Box::from_raw(config);
    config.builder = config.builder.inherit_stdin();
    std::mem::forget(config);
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stdout(
    config: *mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    let mut config = Box::from_raw(config);
    config.builder = config.builder.stdout(file);
    std::mem::forget(config);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdout(config: *mut wasi_config_t) {
    let mut config = Box::from_raw(config);
    config.builder = config.builder.inherit_stdout();
    std::mem::forget(config);
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_set_stderr(
    config: *mut wasi_config_t,
    path: *const c_char,
) -> bool {
    let file = match create_file(path) {
        Some(f) => f,
        None => return false,
    };

    let mut config = Box::from_raw(config);
    config.builder = config.builder.stderr(file);
    std::mem::forget(config);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stderr(config: *mut wasi_config_t) {
    let mut config = Box::from_raw(config);
    config.builder = config.builder.inherit_stderr();
    std::mem::forget(config);
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

    let mut config = Box::from_raw(config);
    config.builder = config.builder.preopened_dir(dir, guest_path);
    std::mem::forget(config);

    true
}

#[repr(C)]
pub struct wasi_instance_t {
    wasi: Wasi,
    export_cache: HashMap<String, *mut wasm_extern_t>,
}

impl Drop for wasi_instance_t {
    fn drop(&mut self) {
        for v in self.export_cache.values() {
            drop(unsafe { Box::from_raw(*v) });
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasi_instance_new(
    store: *mut wasm_store_t,
    config: *mut wasi_config_t,
    trap: *mut *mut wasm_trap_t,
) -> *mut wasi_instance_t {
    let store = &(*store).store.borrow();
    let config = Box::from_raw(config);

    match config.builder.build() {
        Ok(ctx) => Box::into_raw(Box::new(wasi_instance_t {
            wasi: Wasi::new(store, ctx),
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
    // TODO: support previous versions?
    if (*import).ty.module() != "wasi_snapshot_preview1" {
        return std::ptr::null_mut();
    }

    // The import should be a function (WASI only exports functions)
    let func_type = match (*import).ty.ty() {
        ExternType::Func(f) => f,
        _ => return std::ptr::null_mut(),
    };

    let name = (*import).ty.name();

    match (*instance).wasi.get_export(name) {
        Some(export) => {
            if export.ty().params() != func_type.params() {
                return std::ptr::null_mut();
            }

            if export.ty().results() != func_type.results() {
                return std::ptr::null_mut();
            }

            *(*instance)
                .export_cache
                .entry(name.to_string())
                .or_insert_with(|| {
                    Box::into_raw(Box::new(wasm_extern_t {
                        which: ExternHost::Func(HostRef::new(export.clone())),
                    }))
                })
        }
        None => std::ptr::null_mut(),
    }
}
