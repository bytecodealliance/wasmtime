//! The WASI embedding API definitions for Wasmtime.
use crate::{wasm_extern_t, wasm_importtype_t, wasm_store_t, wasm_trap_t, ExternHost, ExternType};
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::slice;
use wasi_common::{
    old::snapshot_0::WasiCtxBuilder as WasiSnapshot0CtxBuilder, preopen_dir, WasiCtxBuilder,
};
use wasmtime::{HostRef, Trap};
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi};

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
    preview1: WasiCtxBuilder,
    snapshot0: WasiSnapshot0CtxBuilder,
}

impl wasi_config_t {}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_new() -> *mut wasi_config_t {
    Box::into_raw(Box::new(wasi_config_t {
        preview1: WasiCtxBuilder::new(),
        snapshot0: WasiSnapshot0CtxBuilder::new(),
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
    (*config).preview1.args(
        slice::from_raw_parts(argv, argc as usize)
            .iter()
            .map(|a| CStr::from_ptr(*a).to_bytes()),
    );
    (*config).snapshot0.args(
        slice::from_raw_parts(argv, argc as usize)
            .iter()
            .map(|a| CStr::from_ptr(*a).to_bytes()),
    );
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_argv(config: *mut wasi_config_t) {
    (*config).preview1.inherit_args();
    (*config).snapshot0.inherit_args();
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

    (*config).preview1.envs(
        names
            .iter()
            .map(|p| CStr::from_ptr(*p).to_bytes())
            .zip(values.iter().map(|p| CStr::from_ptr(*p).to_bytes())),
    );
    (*config).snapshot0.envs(
        names
            .iter()
            .map(|p| CStr::from_ptr(*p).to_bytes())
            .zip(values.iter().map(|p| CStr::from_ptr(*p).to_bytes())),
    );
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_env(config: *mut wasi_config_t) {
    (*config).preview1.inherit_env();
    (*config).snapshot0.inherit_env();
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

    let clone = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return false,
    };

    (*config).preview1.stdin(clone);
    (*config).snapshot0.stdin(file);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdin(config: *mut wasi_config_t) {
    (*config).preview1.inherit_stdin();
    (*config).snapshot0.inherit_stdin();
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

    let clone = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return false,
    };

    (*config).preview1.stdout(clone);
    (*config).snapshot0.stdout(file);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stdout(config: *mut wasi_config_t) {
    (*config).preview1.inherit_stdout();
    (*config).snapshot0.inherit_stdout();
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

    let clone = match file.try_clone() {
        Ok(f) => f,
        Err(_) => return false,
    };

    (*config).preview1.stderr(clone);
    (*config).snapshot0.stderr(file);

    true
}

#[no_mangle]
pub unsafe extern "C" fn wasi_config_inherit_stderr(config: *mut wasi_config_t) {
    (*config).preview1.inherit_stderr();
    (*config).snapshot0.inherit_stderr();
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

    let clone = match dir.try_clone() {
        Ok(f) => f,
        Err(_) => return false,
    };

    (*config).preview1.preopened_dir(clone, guest_path.clone());
    (*config).snapshot0.preopened_dir(dir, guest_path);

    true
}

#[repr(C)]
pub struct wasi_instance_t {
    preview1: Wasi,
    snapshot0: WasiSnapshot0,
}

#[no_mangle]
pub unsafe extern "C" fn wasi_instance_new(
    store: *mut wasm_store_t,
    config: *mut wasi_config_t,
    trap: *mut *mut wasm_trap_t,
) -> *mut wasi_instance_t {
    let store = &(*store).store.borrow();
    let mut config = Box::from_raw(config);

    let preview1 = match config.preview1.build() {
        Ok(ctx) => Wasi::new(store, ctx),
        Err(e) => {
            (*trap) = Box::into_raw(Box::new(wasm_trap_t {
                trap: HostRef::new(Trap::new(e.to_string())),
            }));

            return std::ptr::null_mut();
        }
    };

    let snapshot0 = match config.snapshot0.build() {
        Ok(ctx) => WasiSnapshot0::new(store, ctx),
        Err(e) => {
            (*trap) = Box::into_raw(Box::new(wasm_trap_t {
                trap: HostRef::new(Trap::new(e.to_string())),
            }));

            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(wasi_instance_t {
        preview1,
        snapshot0,
    }))
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

    let name = (*import).ty.name();

    let export = match (*import).ty.module() {
        "wasi_snapshot_preview1" => match (*instance).preview1.get_export(name) {
            Some(e) => e,
            None => return std::ptr::null_mut(),
        },
        "wasi_unstable" => match (*instance).snapshot0.get_export(name) {
            Some(e) => e,
            None => return std::ptr::null_mut(),
        },
        _ => return std::ptr::null_mut(),
    };

    if export.ty() != func_type {
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(wasm_extern_t {
        which: ExternHost::Func(HostRef::new(export.clone())),
    }))
}
