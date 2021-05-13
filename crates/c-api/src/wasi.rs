//! The WASI embedding API definitions for Wasmtime.
use crate::{wasm_extern_t, wasm_importtype_t, wasm_store_t, wasm_trap_t};
use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::File;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::slice;
use std::str;
use wasmtime::{Extern, Linker, Trap};
use wasmtime_wasi::{
    sync::{
        snapshots::preview_0::Wasi as WasiSnapshot0, snapshots::preview_1::Wasi as WasiPreview1,
        Dir, WasiCtxBuilder,
    },
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
    preopens: Vec<(Dir, PathBuf)>,
    inherit_args: bool,
    inherit_env: bool,
    inherit_stdin: bool,
    inherit_stdout: bool,
    inherit_stderr: bool,
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
        Some(p) => match Dir::open_ambient_dir(p) {
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

fn create_wasi_ctx(config: wasi_config_t) -> Result<Rc<RefCell<WasiCtx>>> {
    let mut builder = WasiCtxBuilder::new();
    if config.inherit_args {
        builder = builder.inherit_args()?;
    } else if !config.args.is_empty() {
        let args = config
            .args
            .into_iter()
            .map(|bytes| Ok(String::from_utf8(bytes)?))
            .collect::<Result<Vec<String>>>()?;
        builder = builder.args(&args)?;
    }
    if config.inherit_env {
        builder = builder.inherit_env()?;
    } else if !config.env.is_empty() {
        let env = config
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
    if config.inherit_stdin {
        builder = builder.inherit_stdin();
    } else if let Some(file) = config.stdin {
        let file = unsafe { cap_std::fs::File::from_std(file) };
        let file = wasi_cap_std_sync::file::File::from_cap_std(file);
        builder = builder.stdin(Box::new(file));
    }
    if config.inherit_stdout {
        builder = builder.inherit_stdout();
    } else if let Some(file) = config.stdout {
        let file = unsafe { cap_std::fs::File::from_std(file) };
        let file = wasi_cap_std_sync::file::File::from_cap_std(file);
        builder = builder.stdout(Box::new(file));
    }
    if config.inherit_stderr {
        builder = builder.inherit_stderr();
    } else if let Some(file) = config.stderr {
        let file = unsafe { cap_std::fs::File::from_std(file) };
        let file = wasi_cap_std_sync::file::File::from_cap_std(file);
        builder = builder.stderr(Box::new(file));
    }
    for (dir, path) in config.preopens {
        builder = builder.preopened_dir(dir, path)?;
    }
    Ok(Rc::new(RefCell::new(builder.build()?)))
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
    store: &wasm_store_t,
    name: *const c_char,
    config: Box<wasi_config_t>,
    trap: &mut *mut wasm_trap_t,
) -> Option<Box<wasi_instance_t>> {
    let store = &store.store;

    let result = match CStr::from_ptr(name).to_str().unwrap_or("") {
        "wasi_snapshot_preview1" => {
            create_wasi_ctx(*config).map(|cx| WasiInstance::Preview1(WasiPreview1::new(store, cx)))
        }
        "wasi_unstable" => create_wasi_ctx(*config)
            .map(|cx| WasiInstance::Snapshot0(WasiSnapshot0::new(store, cx))),
        _ => Err(anyhow::anyhow!("unsupported WASI version")),
    };

    match result {
        Ok(wasi) => Some(Box::new(wasi_instance_t {
            wasi,
            export_cache: HashMap::new(),
        })),
        Err(e) => {
            *trap = Box::into_raw(Box::new(wasm_trap_t {
                trap: Trap::from(e),
            }));

            None
        }
    }
}

#[no_mangle]
pub extern "C" fn wasi_instance_delete(_instance: Box<wasi_instance_t>) {}

#[no_mangle]
pub extern "C" fn wasi_instance_bind_import<'a>(
    instance: &'a mut wasi_instance_t,
    import: &wasm_importtype_t,
) -> Option<&'a wasm_extern_t> {
    let module = &import.module;
    let name = str::from_utf8(import.name.as_ref()?.as_bytes()).ok()?;

    let export = match &instance.wasi {
        WasiInstance::Preview1(wasi) => {
            if module != "wasi_snapshot_preview1" {
                return None;
            }
            wasi.get_export(&name)?
        }
        WasiInstance::Snapshot0(wasi) => {
            if module != "wasi_unstable" {
                return None;
            }

            wasi.get_export(&name)?
        }
    };

    if &export.ty() != import.ty.func()? {
        return None;
    }

    let entry = instance
        .export_cache
        .entry(name.to_string())
        .or_insert_with(|| {
            Box::new(wasm_extern_t {
                which: Extern::Func(export.clone()),
            })
        });
    Some(entry)
}
