//! This file defines the extern "C" API, which is compatible with the
//! [Wasm C API](https://github.com/WebAssembly/wasm-c-api).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// TODO complete the C API

use anyhow::Result;
use once_cell::unsync::OnceCell;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::{mem, ptr, slice};
use wasmtime::{
    AnyRef, Config, Engine, ExportType, Extern, ExternType, Func, FuncType, Global, GlobalType,
    HostInfo, HostRef, ImportType, Instance, Limits, Memory, MemoryType, Module, Store, Table,
    TableType, Trap, Val, ValType,
};

mod ext;
mod wasi;

use crate::wasi::*;

pub type float32_t = f32;
pub type float64_t = f64;
pub type wasm_byte_t = u8;

pub type wasm_name_t = wasm_byte_vec_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_config_t {
    pub(crate) config: Config,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_engine_t {
    engine: HostRef<Engine>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_store_t {
    store: HostRef<Store>,
}
#[doc = ""]
pub type wasm_mutability_t = u8;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_limits_t {
    pub min: u32,
    pub max: u32,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_valtype_t {
    ty: ValType,
}

pub type wasm_valkind_t = u8;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_functype_t {
    functype: FuncType,
    params_cache: OnceCell<wasm_valtype_vec_t>,
    returns_cache: OnceCell<wasm_valtype_vec_t>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_globaltype_t {
    globaltype: GlobalType,
    content_cache: OnceCell<wasm_valtype_t>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_tabletype_t {
    tabletype: TableType,
    element_cache: OnceCell<wasm_valtype_t>,
    limits_cache: OnceCell<wasm_limits_t>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_memorytype_t {
    memorytype: MemoryType,
    limits_cache: OnceCell<wasm_limits_t>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_t {
    ty: ExternType,
    cache: OnceCell<wasm_externtype_t_type_cache>,
}

#[derive(Clone)]
enum wasm_externtype_t_type_cache {
    Func(wasm_functype_t),
    Global(wasm_globaltype_t),
    Memory(wasm_memorytype_t),
    Table(wasm_tabletype_t),
}

pub type wasm_externkind_t = u8;

const WASM_EXTERN_FUNC: wasm_externkind_t = 0;
const WASM_EXTERN_GLOBAL: wasm_externkind_t = 1;
const WASM_EXTERN_TABLE: wasm_externkind_t = 2;
const WASM_EXTERN_MEMORY: wasm_externkind_t = 3;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_t {
    ty: ImportType,
    module_cache: OnceCell<wasm_name_t>,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_t {
    ty: ExportType,
    name_cache: OnceCell<wasm_name_t>,
    type_cache: OnceCell<wasm_externtype_t>,
}

#[doc = ""]
#[repr(C)]
#[derive(Clone)]
pub struct wasm_ref_t {
    r: AnyRef,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct wasm_val_t {
    pub kind: wasm_valkind_t,
    pub of: wasm_val_t__bindgen_ty_1,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union wasm_val_t__bindgen_ty_1 {
    pub i32: i32,
    pub i64: i64,
    pub u32: u32,
    pub u64: u64,
    pub f32: float32_t,
    pub f64: float64_t,
    pub ref_: *mut wasm_ref_t,
    _bindgen_union_align: u64,
}

impl Default for wasm_val_t {
    fn default() -> Self {
        wasm_val_t {
            kind: 0,
            of: wasm_val_t__bindgen_ty_1 { i32: 0 },
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_frame_t {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_instance_t {
    instance: HostRef<Instance>,
    exports_cache: RefCell<Option<Vec<ExternHost>>>,
}
pub type wasm_message_t = wasm_name_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_trap_t {
    trap: HostRef<Trap>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_foreign_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_module_t {
    module: HostRef<Module>,
    imports: Vec<wasm_importtype_t>,
    exports: Vec<wasm_exporttype_t>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_shared_module_t {
    _unused: [u8; 0],
}

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_func_t {
    ext: wasm_extern_t,
}

impl wasm_func_t {
    fn func(&self) -> &HostRef<Func> {
        match &self.ext.which {
            ExternHost::Func(f) => f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

pub type wasm_func_callback_t =
    extern "C" fn(args: *const wasm_val_t, results: *mut wasm_val_t) -> Option<Box<wasm_trap_t>>,
;
pub type wasm_func_callback_with_env_t =
    extern "C" fn(
        env: *mut std::ffi::c_void,
        args: *const wasm_val_t,
        results: *mut wasm_val_t,
    ) -> Option<Box<wasm_trap_t>>,
;

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_global_t {
    ext: wasm_extern_t,
}

impl wasm_global_t {
    fn global(&self) -> &HostRef<Global> {
        match &self.ext.which {
            ExternHost::Global(g) => g,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_table_t {
    ext: wasm_extern_t,
}

impl wasm_table_t {
    fn table(&self) -> &HostRef<Table> {
        match &self.ext.which {
            ExternHost::Table(t) => t,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

pub type wasm_table_size_t = u32;

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_memory_t {
    ext: wasm_extern_t,
}

impl wasm_memory_t {
    fn memory(&self) -> &HostRef<Memory> {
        match &self.ext.which {
            ExternHost::Memory(m) => m,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

pub type wasm_memory_pages_t = u32;

#[derive(Clone)]
pub struct wasm_extern_t {
    which: ExternHost,
}

#[derive(Clone)]
enum ExternHost {
    Func(HostRef<Func>),
    Global(HostRef<Global>),
    Memory(HostRef<Memory>),
    Table(HostRef<Table>),
}

#[no_mangle]
pub extern "C" fn wasm_engine_delete(_engine: Box<wasm_engine_t>) {}

#[no_mangle]
pub extern "C" fn wasm_config_delete(_config: Box<wasm_config_t>) {}

#[no_mangle]
pub extern "C" fn wasm_config_new() -> Box<wasm_config_t> {
    Box::new(wasm_config_t {
        config: Config::default(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_engine_new() -> Box<wasm_engine_t> {
    Box::new(wasm_engine_t {
        engine: HostRef::new(Engine::default()),
    })
}

#[no_mangle]
pub extern "C" fn wasm_engine_new_with_config(c: Box<wasm_config_t>) -> Box<wasm_engine_t> {
    let config = c.config;
    Box::new(wasm_engine_t {
        engine: HostRef::new(Engine::new(&config)),
    })
}

#[no_mangle]
pub extern "C" fn wasm_extern_delete(_e: Box<wasm_extern_t>) {}

#[no_mangle]
pub extern "C" fn wasm_extern_as_func(e: &mut wasm_extern_t) -> Option<&mut wasm_func_t> {
    match &e.which {
        ExternHost::Func(_) => Some(unsafe { &mut *(e as *mut wasm_extern_t as *mut wasm_func_t) }),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn wasm_func_as_extern(f: &mut wasm_func_t) -> &mut wasm_extern_t {
    &mut (*f).ext
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_call(
    func: *const wasm_func_t,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> *mut wasm_trap_t {
    let func = (*func).func().borrow();
    let mut params = Vec::with_capacity(func.param_arity());
    for i in 0..func.param_arity() {
        let val = &(*args.add(i));
        params.push(val.val());
    }

    // We're calling arbitrary code here most of the time, and we in general
    // want to try to insulate callers against bugs in wasmtime/wasi/etc if we
    // can. As a result we catch panics here and transform them to traps to
    // allow the caller to have any insulation possible against Rust panics.
    let result = panic::catch_unwind(AssertUnwindSafe(|| func.call(&params)));
    match result {
        Ok(Ok(out)) => {
            for i in 0..func.result_arity() {
                let val = &mut (*results.add(i));
                *val = wasm_val_t::from_val(&out[i]);
            }
            ptr::null_mut()
        }
        Ok(Err(trap)) => {
            let trap = Box::new(wasm_trap_t {
                trap: HostRef::new(trap),
            });
            Box::into_raw(trap)
        }
        Err(panic) => {
            let trap = if let Some(msg) = panic.downcast_ref::<String>() {
                Trap::new(msg)
            } else if let Some(msg) = panic.downcast_ref::<&'static str>() {
                Trap::new(*msg)
            } else {
                Trap::new("rust panic happened")
            };
            let trap = Box::new(wasm_trap_t {
                trap: HostRef::new(trap),
            });
            Box::into_raw(trap)
        }
    }
}

impl wasm_val_t {
    fn default() -> wasm_val_t {
        wasm_val_t {
            kind: 0,
            of: wasm_val_t__bindgen_ty_1 { i32: 0 },
        }
    }

    fn set(&mut self, val: Val) {
        match val {
            Val::I32(i) => {
                self.kind = from_valtype(&ValType::I32);
                self.of = wasm_val_t__bindgen_ty_1 { i32: i };
            }
            Val::I64(i) => {
                self.kind = from_valtype(&ValType::I64);
                self.of = wasm_val_t__bindgen_ty_1 { i64: i };
            }
            Val::F32(f) => {
                self.kind = from_valtype(&ValType::F32);
                self.of = wasm_val_t__bindgen_ty_1 { u32: f };
            }
            Val::F64(f) => {
                self.kind = from_valtype(&ValType::F64);
                self.of = wasm_val_t__bindgen_ty_1 { u64: f };
            }
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    fn from_val(val: &Val) -> wasm_val_t {
        match val {
            Val::I32(i) => wasm_val_t {
                kind: from_valtype(&ValType::I32),
                of: wasm_val_t__bindgen_ty_1 { i32: *i },
            },
            Val::I64(i) => wasm_val_t {
                kind: from_valtype(&ValType::I64),
                of: wasm_val_t__bindgen_ty_1 { i64: *i },
            },
            Val::F32(f) => wasm_val_t {
                kind: from_valtype(&ValType::F32),
                of: wasm_val_t__bindgen_ty_1 { u32: *f },
            },
            Val::F64(f) => wasm_val_t {
                kind: from_valtype(&ValType::F64),
                of: wasm_val_t__bindgen_ty_1 { u64: *f },
            },
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    fn val(&self) -> Val {
        match into_valtype(self.kind) {
            ValType::I32 => Val::from(unsafe { self.of.i32 }),
            ValType::I64 => Val::from(unsafe { self.of.i64 }),
            ValType::F32 => Val::from(unsafe { self.of.f32 }),
            ValType::F64 => Val::from(unsafe { self.of.f64 }),
            _ => unimplemented!("wasm_val_t::val {:?}", self.kind),
        }
    }
}

enum Callback {
    Wasm(wasm_func_callback_t),
    Wasmtime(crate::ext::wasmtime_func_callback_t),
}

enum CallbackWithEnv {
    Wasm(wasm_func_callback_with_env_t),
    Wasmtime(crate::ext::wasmtime_func_callback_with_env_t),
}

unsafe fn create_function(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: Callback,
) -> *mut wasm_func_t {
    let store = &(*store).store.borrow();
    let ty = (*ty).functype.clone();
    let func = Func::new(store, ty, move |caller, params, results| {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let func = callback.expect("wasm_func_callback_t fn");
        let out = func(params.as_ptr(), out_results.as_mut_ptr());
        if let Some(trap) = out {
            return Err(trap.trap.borrow().clone());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    });
    Box::new(wasm_func_t {
        ext: wasm_extern_t {
            which: ExternHost::Func(HostRef::new(func)),
        },
    })
}

unsafe fn create_function_with_env(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: CallbackWithEnv,
    env: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> Box<wasm_func_t> {
    let store = &(*store).store.borrow();
    let ty = (*ty).functype.clone();

    // Create a small object which will run the finalizer when it's dropped, and
    // then we move this `run_finalizer` object into the closure below (via the
    // `drop(&run_finalizer)` statement so it's all dropped when the closure is
    // dropped.
    struct RunOnDrop<F: FnMut()>(F);
    impl<F: FnMut()> Drop for RunOnDrop<F> {
        fn drop(&mut self) {
            (self.0)();
        }
    }
    let run_finalizer = RunOnDrop(move || {
        if let Some(finalizer) = finalizer {
            finalizer(env);
        }
    });
    let func = Func::new(store, ty, move |caller, params, results| {
        drop(&run_finalizer);
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let out = match callback {
            CallbackWithEnv::Wasm(callback) => {
                callback(env, params.as_ptr(), out_results.as_mut_ptr())
            }
            CallbackWithEnv::Wasmtime(callback) => {
                let caller = crate::ext::wasmtime_caller_t { inner: caller };
                callback(&caller, env, params.as_ptr(), out_results.as_mut_ptr())
            }
        };
        if !out.is_null() {
            let trap: Box<wasm_trap_t> = Box::from_raw(out);
            return Err(trap.trap.borrow().clone());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    });

    let func = Box::new(wasm_func_t {
        ext: wasm_extern_t {
            which: ExternHost::Func(HostRef::new(func)),
        },
    });
    Box::into_raw(func)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: wasm_func_callback_t,
) -> Box<wasm_func_t> {
    create_function(store, ty, Callback::Wasm(callback))
}

#[no_mangle]
pub extern "C" fn wasm_func_delete(_f: Box<wasm_func_t>) {}

#[no_mangle]
pub extern "C" fn wasm_functype_new(
    params: &mut wasm_valtype_vec_t,
    results: &mut wasm_valtype_vec_t,
) -> Box<wasm_functype_t> {
    let params = params
        .take()
        .into_iter()
        .map(|vt| vt.unwrap().ty.clone())
        .collect::<Vec<_>>();
    let results = results
        .take()
        .into_iter()
        .map(|vt| vt.unwrap().ty.clone())
        .collect::<Vec<_>>();
    let functype = FuncType::new(params.into_boxed_slice(), results.into_boxed_slice());
    Box::new(wasm_functype_t {
        functype,
        params_cache: OnceCell::new(),  // TODO get from args?
        returns_cache: OnceCell::new(), // TODO get from args?
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_delete(_ft: Box<wasm_functype_t>) {}

#[no_mangle]
pub extern "C" fn wasm_instance_delete(_instance: Box<wasm_instance_t>) {}

impl wasm_instance_t {
    fn new(instance: Instance) -> wasm_instance_t {
        wasm_instance_t {
            instance: HostRef::new(instance),
            exports_cache: RefCell::new(None),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    store: &wasm_store_t,
    module: &wasm_module_t,
    imports: *const Box<wasm_extern_t>,
    result: &mut *mut wasm_trap_t,
) -> *mut wasm_instance_t {
    let mut externs: Vec<Extern> = Vec::with_capacity((*module).imports.len());
    for i in 0..(*module).imports.len() {
        let import = &*imports.add(i);
        externs.push(match &import.which {
            ExternHost::Func(e) => Extern::Func(e.borrow().clone()),
            ExternHost::Table(e) => Extern::Table(e.borrow().clone()),
            ExternHost::Global(e) => Extern::Global(e.borrow().clone()),
            ExternHost::Memory(e) => Extern::Memory(e.borrow().clone()),
        });
    }
    let store = &(*store).store.borrow();
    let module = &(*module).module.borrow();
    // FIXME(WebAssembly/wasm-c-api#126) what else can we do with the `store`
    // argument?
    if !Store::same(&store, module.store()) {
        if !result.is_null() {
            let trap = Trap::new("wasm_store_t must match store in wasm_module_t");
            let trap = Box::new(wasm_trap_t {
                trap: HostRef::new(trap),
            });
            (*result) = Box::into_raw(trap);
        }
        return ptr::null_mut();
    }
    handle_instantiate(Instance::new(module, &externs), result)
}

unsafe fn handle_instantiate(
    instance: Result<Instance>,
    result: *mut *mut wasm_trap_t,
) -> *mut wasm_instance_t {
    match instance {
        Ok(instance) => {
            let instance = Box::new(wasm_instance_t::new(instance));
            if !result.is_null() {
                (*result) = ptr::null_mut();
            }
            Box::into_raw(instance)
        }
        Err(trap) => {
            if !result.is_null() {
                let trap = match trap.downcast::<Trap>() {
                    Ok(trap) => trap,
                    Err(e) => Trap::new(format!("{:?}", e)),
                };
                let trap = Box::new(wasm_trap_t {
                    trap: HostRef::new(trap),
                });
                (*result) = Box::into_raw(trap);
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_instance_exports(instance: &wasm_instance_t, out: &mut wasm_extern_vec_t) {
    let mut cache = instance.exports_cache.borrow_mut();
    let exports = cache.get_or_insert_with(|| {
        let instance = &instance.instance.borrow();
        instance
            .exports()
            .iter()
            .map(|e| match e {
                Extern::Func(f) => ExternHost::Func(HostRef::new(f.clone())),
                Extern::Global(f) => ExternHost::Global(HostRef::new(f.clone())),
                Extern::Memory(f) => ExternHost::Memory(HostRef::new(f.clone())),
                Extern::Table(f) => ExternHost::Table(HostRef::new(f.clone())),
            })
            .collect()
    });
    let mut buffer = Vec::with_capacity(exports.len());
    for e in exports {
        let ext = Box::new(wasm_extern_t { which: e.clone() });
        buffer.push(Some(ext));
    }
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_module_delete(_module: Box<wasm_module_t>) {}

impl wasm_name_t {
    fn from_name(name: &str) -> wasm_name_t {
        name.to_string().into_bytes().into()
    }
}

/// Note that this function does not perform validation on the wasm
/// binary. To perform validation, use `wasm_module_validate`.
#[no_mangle]
pub extern "C" fn wasm_module_new(
    store: &wasm_store_t,
    binary: &wasm_byte_vec_t,
) -> Option<Box<wasm_module_t>> {
    let binary = binary.as_slice();
    let store = &store.store.borrow();
    let module = Module::from_binary(store, binary).ok()?;
    let imports = module
        .imports()
        .iter()
        .map(|i| wasm_importtype_t {
            ty: i.clone(),
            module_cache: OnceCell::new(),
            name_cache: OnceCell::new(),
            type_cache: OnceCell::new(),
        })
        .collect::<Vec<_>>();
    let exports = module
        .exports()
        .iter()
        .map(|e| wasm_exporttype_t {
            ty: e.clone(),
            name_cache: OnceCell::new(),
            type_cache: OnceCell::new(),
        })
        .collect::<Vec<_>>();
    Some(Box::new(wasm_module_t {
        module: HostRef::new(module),
        imports,
        exports,
    }))
}

#[no_mangle]
pub extern "C" fn wasm_module_validate(store: &wasm_store_t, binary: &wasm_byte_vec_t) -> bool {
    let binary = binary.as_slice();
    let store = &store.store.borrow();
    Module::validate(store, binary).is_ok()
}

#[no_mangle]
pub extern "C" fn wasm_store_delete(_store: Box<wasm_store_t>) {}

#[no_mangle]
pub extern "C" fn wasm_store_new(engine: &wasm_engine_t) -> Box<wasm_store_t> {
    let engine = &engine.engine;
    Box::new(wasm_store_t {
        store: HostRef::new(Store::new(&engine.borrow())),
    })
}

#[no_mangle]
pub extern "C" fn wasm_func_new_with_env(
    store: &wasm_store_t,
    ty: &wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    env: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> Box<wasm_func_t> {
    create_function_with_env(store, ty, CallbackWithEnv::Wasm(callback), env, finalizer)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: *mut wasm_val_t, source: &wasm_val_t) {
    *out = match into_valtype(source.kind) {
        ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => *source,
        _ => unimplemented!("wasm_val_copy arg"),
    };
}

fn into_valtype(kind: wasm_valkind_t) -> ValType {
    match kind {
        0 => ValType::I32,
        1 => ValType::I64,
        2 => ValType::F32,
        3 => ValType::F64,
        128 => ValType::AnyRef,
        129 => ValType::FuncRef,
        _ => panic!("unexpected kind: {}", kind),
    }
}

fn from_valtype(ty: &ValType) -> wasm_valkind_t {
    match ty {
        ValType::I32 => 0,
        ValType::I64 => 1,
        ValType::F32 => 2,
        ValType::F64 => 3,
        ValType::AnyRef => 128,
        ValType::FuncRef => 129,
        _ => panic!("wasm_valkind_t has no known conversion for {:?}", ty),
    }
}

#[no_mangle]
pub extern "C" fn wasm_valtype_new(kind: wasm_valkind_t) -> Box<wasm_valtype_t> {
    Box::new(wasm_valtype_t {
        ty: into_valtype(kind),
    })
}

#[no_mangle]
pub extern "C" fn wasm_valtype_delete(_vt: Box<wasm_valtype_t>) {}

#[no_mangle]
pub extern "C" fn wasm_frame_delete(_frame: Box<wasm_frame_t>) {}

#[no_mangle]
pub unsafe extern "C" fn wasm_frame_func_index(_arg1: *const wasm_frame_t) -> u32 {
    unimplemented!("wasm_frame_func_index")
}

#[no_mangle]
pub unsafe extern "C" fn wasm_frame_func_offset(_arg1: *const wasm_frame_t) -> usize {
    unimplemented!("wasm_frame_func_offset")
}

#[no_mangle]
pub unsafe extern "C" fn wasm_frame_instance(_arg1: *const wasm_frame_t) -> *mut wasm_instance_t {
    unimplemented!("wasm_frame_instance")
}

#[no_mangle]
pub unsafe extern "C" fn wasm_frame_module_offset(_arg1: *const wasm_frame_t) -> usize {
    unimplemented!("wasm_frame_module_offset")
}

#[no_mangle]
pub extern "C" fn wasm_trap_delete(_trap: Box<wasm_trap_t>) {}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_new(
    _store: &wasm_store_t,
    message: &wasm_message_t,
) -> Box<wasm_trap_t> {
    let message = message.as_slice();
    if message[message.len() - 1] != 0 {
        panic!("wasm_trap_new message stringz expected");
    }
    let message = String::from_utf8_lossy(&message[..message.len() - 1]);
    Box::new(wasm_trap_t {
        trap: HostRef::new(Trap::new(message)),
    })
}

#[no_mangle]
pub extern "C" fn wasm_trap_message(trap: &wasm_trap_t, out: &mut wasm_message_t) {
    let mut buffer = Vec::new();
    buffer.extend_from_slice(trap.trap.borrow().message().as_bytes());
    buffer.reserve_exact(1);
    buffer.push(0);
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_trap_origin(_trap: &wasm_trap_t) -> Option<Box<wasm_frame_t>> {
    None
}

#[no_mangle]
pub extern "C" fn wasm_trap_trace(_trap: &wasm_trap_t, out: &mut wasm_frame_vec_t) {
    out.set_buffer(Vec::new());
}

#[no_mangle]
pub extern "C" fn wasm_importtype_delete(_ty: Box<wasm_importtype_t>) {}

#[no_mangle]
pub extern "C" fn wasm_importtype_module(it: &wasm_importtype_t) -> &wasm_name_t {
    it.module_cache
        .get_or_init(|| wasm_name_t::from_name(&it.ty.module()))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_name(it: &wasm_importtype_t) -> &wasm_name_t {
    it.name_cache
        .get_or_init(|| wasm_name_t::from_name(&it.ty.name()))
}

#[no_mangle]
pub extern "C" fn wasm_importtype_type(it: &wasm_importtype_t) -> &wasm_externtype_t {
    it.type_cache.get_or_init(|| wasm_externtype_t {
        ty: it.ty.ty().clone(),
        cache: OnceCell::new(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_delete(_ty: Box<wasm_exporttype_t>) {}

#[no_mangle]
pub extern "C" fn wasm_exporttype_name(et: &wasm_exporttype_t) -> &wasm_name_t {
    et.name_cache
        .get_or_init(|| wasm_name_t::from_name(&et.ty.name()))
}

#[no_mangle]
pub extern "C" fn wasm_exporttype_type(et: &wasm_exporttype_t) -> &wasm_externtype_t {
    et.type_cache.get_or_init(|| wasm_externtype_t {
        ty: et.ty.ty().clone(),
        cache: OnceCell::new(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_extern_kind(e: &wasm_extern_t) -> wasm_externkind_t {
    match e.which {
        ExternHost::Func(_) => WASM_EXTERN_FUNC,
        ExternHost::Global(_) => WASM_EXTERN_GLOBAL,
        ExternHost::Table(_) => WASM_EXTERN_TABLE,
        ExternHost::Memory(_) => WASM_EXTERN_MEMORY,
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_type(e: &wasm_extern_t) -> Box<wasm_externtype_t> {
    Box::new(wasm_externtype_t {
        ty: match &e.which {
            ExternHost::Func(f) => ExternType::Func(f.borrow().ty().clone()),
            ExternHost::Global(f) => ExternType::Global(f.borrow().ty().clone()),
            ExternHost::Table(f) => ExternType::Table(f.borrow().ty().clone()),
            ExternHost::Memory(f) => ExternType::Memory(f.borrow().ty().clone()),
        },
        cache: OnceCell::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_functype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_functype_t> {
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let functype = et.ty.func().ok_or(())?.clone();
            let m = wasm_functype_t {
                functype,
                params_cache: OnceCell::new(),
                returns_cache: OnceCell::new(),
            };
            Ok(wasm_externtype_t_type_cache::Func(m))
        })
        .ok()?;

    match cache {
        wasm_externtype_t_type_cache::Func(m) => Some(m),
        _ => None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_globaltype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_globaltype_t> {
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let globaltype = et.ty.global().ok_or(())?.clone();
            let m = wasm_globaltype_t {
                globaltype,
                content_cache: OnceCell::new(),
            };
            Ok(wasm_externtype_t_type_cache::Global(m))
        })
        .ok()?;

    match cache {
        wasm_externtype_t_type_cache::Global(m) => Some(m),
        _ => None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_tabletype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_tabletype_t> {
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let tabletype = et.ty.table().ok_or(())?.clone();
            let m = wasm_tabletype_t {
                tabletype,
                limits_cache: OnceCell::new(),
                element_cache: OnceCell::new(),
            };
            Ok(wasm_externtype_t_type_cache::Table(m))
        })
        .ok()?;

    match cache {
        wasm_externtype_t_type_cache::Table(m) => Some(m),
        _ => None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_memorytype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_memorytype_t> {
    let cache = et
        .cache
        .get_or_try_init(|| -> Result<_, ()> {
            let memorytype = et.ty.memory().ok_or(())?.clone();
            let m = wasm_memorytype_t {
                memorytype,
                limits_cache: OnceCell::new(),
            };
            Ok(wasm_externtype_t_type_cache::Memory(m))
        })
        .ok()?;

    match cache {
        wasm_externtype_t_type_cache::Memory(m) => Some(m),
        _ => None,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_delete(et: *mut wasm_externtype_t) {
    let _ = Box::from_raw(et);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_kind(et: *const wasm_externtype_t) -> wasm_externkind_t {
    match &(*et).ty {
        ExternType::Func(_) => WASM_EXTERN_FUNC,
        ExternType::Table(_) => WASM_EXTERN_TABLE,
        ExternType::Global(_) => WASM_EXTERN_GLOBAL,
        ExternType::Memory(_) => WASM_EXTERN_MEMORY,
    }
}

#[no_mangle]
pub extern "C" fn wasm_func_type(f: &wasm_func_t) -> Box<wasm_functype_t> {
    Box::new(wasm_functype_t {
        functype: f.func().borrow().ty().clone(),
        params_cache: OnceCell::new(),
        returns_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_param_arity(f: *const wasm_func_t) -> usize {
    (*f).func().borrow().param_arity()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_result_arity(f: *const wasm_func_t) -> usize {
    (*f).func().borrow().result_arity()
}

#[no_mangle]
pub extern "C" fn wasm_functype_params(ft: &wasm_functype_t) -> &wasm_valtype_vec_t {
    ft.params_cache.get_or_init(|| {
        ft.functype
            .params()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_results(ft: &wasm_functype_t) -> &wasm_valtype_vec_t {
    ft.returns_cache.get_or_init(|| {
        ft.functype
            .results()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_content(gt: &wasm_globaltype_t) -> &wasm_valtype_t {
    gt.content_cache.get_or_init(|| wasm_valtype_t {
        ty: gt.globaltype.content().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_mutability(gt: &wasm_globaltype_t) -> wasm_mutability_t {
    use wasmtime::Mutability::*;
    match gt.globaltype.mutability() {
        Const => 0,
        Var => 1,
    }
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_limits(mt: &wasm_memorytype_t) -> &wasm_limits_t {
    mt.limits_cache.get_or_init(|| {
        let limits = mt.memorytype.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_module_exports(module: &wasm_module_t, out: &mut wasm_exporttype_vec_t) {
    let buffer = module
        .exports
        .iter()
        .map(|et| Some(Box::new(et.clone())))
        .collect::<Vec<_>>();
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_module_imports(module: &wasm_module_t, out: &mut wasm_importtype_vec_t) {
    let buffer = module
        .imports
        .iter()
        .map(|it| Some(Box::new(it.clone())))
        .collect::<Vec<_>>();
    out.set_buffer(buffer);
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_element(tt: &wasm_tabletype_t) -> &wasm_valtype_t {
    tt.element_cache.get_or_init(|| wasm_valtype_t {
        ty: tt.tabletype.element().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_limits(tt: &wasm_tabletype_t) -> &wasm_limits_t {
    tt.limits_cache.get_or_init(|| {
        let limits = tt.tabletype.limits();
        wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        }
    })
}

#[no_mangle]
pub extern "C" fn wasm_valtype_kind(vt: &wasm_valtype_t) -> wasm_valkind_t {
    from_valtype(&vt.ty)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_global(e: *mut wasm_extern_t) -> *mut wasm_global_t {
    match &(*e).which {
        ExternHost::Global(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn wasm_global_as_extern(g: &wasm_global_t) -> &wasm_extern_t {
    &g.ext
}

#[no_mangle]
pub extern "C" fn wasm_global_delete(_g: Box<wasm_global_t>) {}

#[no_mangle]
pub extern "C" fn wasm_global_copy(g: &wasm_global_t) -> Box<wasm_global_t> {
    Box::new(g.clone())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_same(
    g1: *const wasm_global_t,
    g2: *const wasm_global_t,
) -> bool {
    (*g1).global().ptr_eq(&(*g2).global())
}

#[no_mangle]
pub extern "C" fn wasm_global_new(
    store: &wasm_store_t,
    gt: &wasm_globaltype_t,
    val: &wasm_val_t,
) -> Option<Box<wasm_global_t>> {
    let global =
        HostRef::new(Global::new(&store.store.borrow(), gt.globaltype.clone(), val.val()).ok()?);
    Some(Box::new(wasm_global_t {
        ext: wasm_extern_t {
            which: ExternHost::Global(global),
        },
    }))
}

#[no_mangle]
pub extern "C" fn wasm_global_type(g: &wasm_global_t) -> Box<wasm_globaltype_t> {
    let globaltype = g.global().borrow().ty().clone();
    Box::new(wasm_globaltype_t {
        globaltype,
        content_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_get(g: *const wasm_global_t, out: *mut wasm_val_t) {
    (*out).set((*g).global().borrow().get());
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_set(g: *mut wasm_global_t, val: *const wasm_val_t) {
    let result = (*g).global().borrow().set((*val).val());
    drop(result); // TODO: should communicate this via the api somehow?
}

#[no_mangle]
pub unsafe extern "C" fn wasm_globaltype_delete(gt: *mut wasm_globaltype_t) {
    let _ = Box::from_raw(gt);
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_new(
    ty: Box<wasm_valtype_t>,
    mutability: wasm_mutability_t,
) -> Box<wasm_globaltype_t> {
    use wasmtime::Mutability::*;
    let mutability = match mutability {
        0 => Const,
        1 => Var,
        _ => panic!("mutability out-of-range"),
    };
    let globaltype = GlobalType::new(ty.ty.clone(), mutability);
    Box::new(wasm_globaltype_t {
        globaltype,
        content_cache: (*ty).into(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_memory(e: *mut wasm_extern_t) -> *mut wasm_memory_t {
    match &(*e).which {
        ExternHost::Memory(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn wasm_memory_as_extern(m: &wasm_memory_t) -> &wasm_extern_t {
    &m.ext
}

#[no_mangle]
pub extern "C" fn wasm_memory_delete(_m: Box<wasm_memory_t>) {}

#[no_mangle]
pub extern "C" fn wasm_memory_copy(m: &wasm_memory_t) -> Box<wasm_memory_t> {
    Box::new(m.clone())
}

#[no_mangle]
pub extern "C" fn wasm_memory_same(m1: &wasm_memory_t, m2: &wasm_memory_t) -> bool {
    m1.memory().ptr_eq(m2.memory())
}

#[no_mangle]
pub extern "C" fn wasm_memory_type(m: &wasm_memory_t) -> Box<wasm_memorytype_t> {
    let ty = m.memory().borrow().ty().clone();
    Box::new(wasm_memorytype_t {
        memorytype: ty,
        limits_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_memory_data(m: &wasm_memory_t) -> *mut u8 {
    m.memory().borrow().data_ptr()
}

#[no_mangle]
pub extern "C" fn wasm_memory_data_size(m: &wasm_memory_t) -> usize {
    m.memory().borrow().data_size()
}

#[no_mangle]
pub extern "C" fn wasm_memory_size(m: &wasm_memory_t) -> wasm_memory_pages_t {
    m.memory().borrow().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_grow(
    m: *mut wasm_memory_t,
    delta: wasm_memory_pages_t,
) -> bool {
    (*m).memory().borrow().grow(delta).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_new(
    store: *mut wasm_store_t,
    mt: *const wasm_memorytype_t,
) -> *mut wasm_memory_t {
    let memory = HostRef::new(Memory::new(
        &(*store).store.borrow(),
        (*mt).memorytype.clone(),
    ));
    let m = Box::new(wasm_memory_t {
        ext: wasm_extern_t {
            which: ExternHost::Memory(memory),
        },
    });
    Box::into_raw(m)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memorytype_delete(mt: *mut wasm_memorytype_t) {
    let _ = Box::from_raw(mt);
}

#[no_mangle]
pub extern "C" fn wasm_memorytype_new(limits: &wasm_limits_t) -> Box<wasm_memorytype_t> {
    let max = if limits.max == u32::max_value() {
        None
    } else {
        Some(limits.max)
    };
    let limits = Limits::new(limits.min, max);
    Box::new(wasm_memorytype_t {
        memorytype: MemoryType::new(limits),
        limits_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_table(e: *mut wasm_extern_t) -> *mut wasm_table_t {
    match &(*e).which {
        ExternHost::Table(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_as_extern(t: *mut wasm_table_t) -> *mut wasm_extern_t {
    &mut (*t).ext
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_as_ref(f: *mut wasm_func_t) -> *mut wasm_ref_t {
    let r = (*f).func().anyref();
    let f = Box::new(wasm_ref_t { r });
    Box::into_raw(f)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_ref_delete(r: *mut wasm_ref_t) {
    if !r.is_null() {
        let _ = Box::from_raw(r);
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_delete(t: *mut wasm_table_t) {
    let _ = Box::from_raw(t);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_copy(t: *const wasm_table_t) -> *mut wasm_table_t {
    Box::into_raw(Box::new((*t).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_new(
    store: *mut wasm_store_t,
    tt: *const wasm_tabletype_t,
    init: *mut wasm_ref_t,
) -> *mut wasm_table_t {
    let init: Val = if !init.is_null() {
        Box::from_raw(init).r.into()
    } else {
        Val::AnyRef(AnyRef::Null)
    };
    let table = match Table::new(&(*store).store.borrow(), (*tt).tabletype.clone(), init) {
        Ok(table) => table,
        Err(_) => return ptr::null_mut(),
    };
    let t = Box::new(wasm_table_t {
        ext: wasm_extern_t {
            which: ExternHost::Table(HostRef::new(table)),
        },
    });
    Box::into_raw(t)
}

unsafe fn into_funcref(val: Val) -> *mut wasm_ref_t {
    if let Val::AnyRef(AnyRef::Null) = val {
        return ptr::null_mut();
    }
    let anyref = match val.anyref() {
        Some(anyref) => anyref,
        None => return ptr::null_mut(),
    };
    let r = Box::new(wasm_ref_t { r: anyref });
    Box::into_raw(r)
}

unsafe fn from_funcref(r: *mut wasm_ref_t) -> Val {
    if !r.is_null() {
        Box::from_raw(r).r.into()
    } else {
        Val::AnyRef(AnyRef::Null)
    }
}

#[no_mangle]
pub extern "C" fn wasm_table_type(t: &wasm_table_t) -> Box<wasm_tabletype_t> {
    let ty = t.table().borrow().ty().clone();
    Box::new(wasm_tabletype_t {
        tabletype: ty,
        limits_cache: OnceCell::new(),
        element_cache: OnceCell::new(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_get(
    t: &wasm_table_t,
    index: wasm_table_size_t,
) -> *mut wasm_ref_t {
    match t.table().borrow().get(index) {
        Some(val) => into_funcref(val),
        None => into_funcref(Val::AnyRef(AnyRef::Null)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_set(
    t: &wasm_table_t,
    index: wasm_table_size_t,
    r: *mut wasm_ref_t,
) -> bool {
    let val = from_funcref(r);
    t.table().borrow().set(index, val).is_ok()
}

#[no_mangle]
pub extern "C" fn wasm_table_size(t: &wasm_table_t) -> wasm_table_size_t {
    t.table().borrow().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_grow(
    t: &wasm_table_t,
    delta: wasm_table_size_t,
    init: *mut wasm_ref_t,
) -> bool {
    let init = from_funcref(init);
    t.table().borrow().grow(delta, init).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_same(t1: *const wasm_table_t, t2: *const wasm_table_t) -> bool {
    (*t1).table().ptr_eq((*t2).table())
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_delete(_tt: Box<wasm_tabletype_t>) {
}

#[no_mangle]
pub extern "C" fn wasm_tabletype_new(
    ty: Box<wasm_valtype_t>,
    limits: &wasm_limits_t,
) -> Box<wasm_tabletype_t> {
    let max = if limits.max == u32::max_value() {
        None
    } else {
        Some(limits.max)
    };
    let limits = Limits::new(limits.min, max);
    Box::new(wasm_tabletype_t {
        tabletype: TableType::new(ty.ty, limits),
        element_cache: OnceCell::new(),
        limits_cache: OnceCell::new(),
    })
}

struct HostInfoState {
    info: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
}

impl HostInfo for HostInfoState {
    fn finalize(&mut self) {
        if let Some(f) = &self.finalizer {
            f(self.info);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_set_host_info_with_finalizer(
    instance: *mut wasm_instance_t,
    info: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
) {
    let info = if info.is_null() && finalizer.is_none() {
        None
    } else {
        let b: Box<dyn HostInfo> = Box::new(HostInfoState { info, finalizer });
        Some(b)
    };
    (*instance).instance.anyref().set_host_info(info);
}

macro_rules! declare_vecs {
    (
        $((
            name: $name:ident,
            ty: $elem_ty:ty,
            new: $new:ident,
            empty: $empty:ident,
            uninit: $uninit:ident,
            copy: $copy:ident,
            delete: $delete:ident,
        ))*
    ) => {$(
        #[repr(C)]
        #[derive(Clone)]
        pub struct $name {
            size: usize,
            data: *mut $elem_ty,
        }

        impl $name {
            fn set_buffer(&mut self, buffer: Vec<$elem_ty>) {
                let mut vec = buffer.into_boxed_slice();
                self.size = vec.len();
                self.data = vec.as_mut_ptr();
                mem::forget(vec);
            }

            fn as_slice(&self) -> &[$elem_ty] {
                unsafe { slice::from_raw_parts(self.data, self.size) }
            }

            fn take(&mut self) -> Vec<$elem_ty> {
                if self.data.is_null() {
                    return Vec::new();
                }
                let vec = unsafe {
                    Vec::from_raw_parts(self.data, self.size, self.size)
                };
                self.data = ptr::null_mut();
                self.size = 0;
                return vec;
            }
        }

        impl From<Vec<$elem_ty>> for $name {
            fn from(mut vec: Vec<$elem_ty>) -> Self {
                assert_eq!(vec.len(), vec.capacity());
                let result = $name {
                    size: vec.len(),
                    data: vec.as_mut_ptr(),
                };
                mem::forget(vec);
                result
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                drop(self.take());
            }
        }

        #[no_mangle]
        pub extern "C" fn $empty(out: &mut $name) {
            out.size = 0;
            out.data = ptr::null_mut();
        }

        #[no_mangle]
        pub extern "C" fn $uninit(out: &mut $name, size: usize) {
            out.set_buffer(vec![Default::default(); size]);
        }

        #[no_mangle]
        pub unsafe extern "C" fn $new(
            out: &mut $name,
            size: usize,
            ptr: *const $elem_ty,
        ) {
            let slice = slice::from_raw_parts(ptr, size);
            out.set_buffer(slice.to_vec());
        }

        #[no_mangle]
        pub extern "C" fn $copy(out: &mut $name, src: &$name) {
            out.set_buffer(src.as_slice().to_vec());
        }

        #[no_mangle]
        pub extern "C" fn $delete(out: &mut $name) {
            out.take();
        }
    )*};
}

declare_vecs! {
    (
        name: wasm_byte_vec_t,
        ty: u8,
        new: wasm_byte_vec_new,
        empty: wasm_byte_vec_new_empty,
        uninit: wasm_byte_vec_new_uninitialized,
        copy: wasm_byte_vec_copy,
        delete: wasm_byte_vec_delete,
    )
    (
        name: wasm_valtype_vec_t,
        ty: Option<Box<wasm_valtype_t>>,
        new: wasm_valtype_vec_new,
        empty: wasm_valtype_vec_new_empty,
        uninit: wasm_valtype_vec_new_uninitialized,
        copy: wasm_valtype_vec_copy,
        delete: wasm_valtype_vec_delete,
    )
    (
        name: wasm_functype_vec_t,
        ty: Option<Box<wasm_functype_t>>,
        new: wasm_functype_vec_new,
        empty: wasm_functype_vec_new_empty,
        uninit: wasm_functype_vec_new_uninitialized,
        copy: wasm_functype_vec_copy,
        delete: wasm_functype_vec_delete,
    )
    (
        name: wasm_globaltype_vec_t,
        ty: Option<Box<wasm_globaltype_t>>,
        new: wasm_globaltype_vec_new,
        empty: wasm_globaltype_vec_new_empty,
        uninit: wasm_globaltype_vec_new_uninitialized,
        copy: wasm_globaltype_vec_copy,
        delete: wasm_globaltype_vec_delete,
    )
    (
        name: wasm_tabletype_vec_t,
        ty: Option<Box<wasm_tabletype_t>>,
        new: wasm_tabletype_vec_new,
        empty: wasm_tabletype_vec_new_empty,
        uninit: wasm_tabletype_vec_new_uninitialized,
        copy: wasm_tabletype_vec_copy,
        delete: wasm_tabletype_vec_delete,
    )
    (
        name: wasm_memorytype_vec_t,
        ty: Option<Box<wasm_memorytype_t>>,
        new: wasm_memorytype_vec_new,
        empty: wasm_memorytype_vec_new_empty,
        uninit: wasm_memorytype_vec_new_uninitialized,
        copy: wasm_memorytype_vec_copy,
        delete: wasm_memorytype_vec_delete,
    )
    (
        name: wasm_externtype_vec_t,
        ty: Option<Box<wasm_externtype_t>>,
        new: wasm_externtype_vec_new,
        empty: wasm_externtype_vec_new_empty,
        uninit: wasm_externtype_vec_new_uninitialized,
        copy: wasm_externtype_vec_copy,
        delete: wasm_externtype_vec_delete,
    )
    (
        name: wasm_importtype_vec_t,
        ty: Option<Box<wasm_importtype_t>>,
        new: wasm_importtype_vec_new,
        empty: wasm_importtype_vec_new_empty,
        uninit: wasm_importtype_vec_new_uninitialized,
        copy: wasm_importtype_vec_copy,
        delete: wasm_importtype_vec_delete,
    )
    (
        name: wasm_exporttype_vec_t,
        ty: Option<Box<wasm_exporttype_t>>,
        new: wasm_exporttype_vec_new,
        empty: wasm_exporttype_vec_new_empty,
        uninit: wasm_exporttype_vec_new_uninitialized,
        copy: wasm_exporttype_vec_copy,
        delete: wasm_exporttype_vec_delete,
    )
    (
        name: wasm_val_vec_t,
        ty: wasm_val_t,
        new: wasm_val_vec_new,
        empty: wasm_val_vec_new_empty,
        uninit: wasm_val_vec_new_uninitialized,
        copy: wasm_val_vec_copy,
        delete: wasm_val_vec_delete,
    )
    (
        name: wasm_frame_vec_t,
        ty: Option<Box<wasm_frame_t>>,
        new: wasm_frame_vec_new,
        empty: wasm_frame_vec_new_empty,
        uninit: wasm_frame_vec_new_uninitialized,
        copy: wasm_frame_vec_copy,
        delete: wasm_frame_vec_delete,
    )
    (
        name: wasm_extern_vec_t,
        ty: Option<Box<wasm_extern_t>>,
        new: wasm_extern_vec_new,
        empty: wasm_extern_vec_new_empty,
        uninit: wasm_extern_vec_new_uninitialized,
        copy: wasm_extern_vec_copy,
        delete: wasm_extern_vec_delete,
    )
}
