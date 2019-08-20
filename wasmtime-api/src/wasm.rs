//! This file defines the extern "C" API, which is compatible with the
//! [Wasm C API](https://github.com/WebAssembly/wasm-c-api).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// TODO complete the C API

use super::{
    Callable, Engine, ExportType, Extern, Func, FuncType, ImportType, Instance, Module, Store,
    Trap, Val, ValType,
};
use std::boxed::Box;
use std::cell::RefCell;
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::slice;

pub type byte_t = ::std::os::raw::c_char;
pub type float32_t = f32;
pub type float64_t = f64;
pub type wasm_byte_t = byte_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_byte_vec_t {
    pub size: usize,
    pub data: *mut wasm_byte_t,
}
pub type wasm_name_t = wasm_byte_vec_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_config_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_engine_t {
    engine: Rc<RefCell<Engine>>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_store_t {
    store: Rc<RefCell<Store>>,
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
#[repr(C)]
#[derive(Clone)]
pub struct wasm_valtype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_valtype_t,
}
pub type wasm_valkind_t = u8;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_functype_t {
    functype: FuncType,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_functype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_functype_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_globaltype_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_globaltype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_globaltype_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_tabletype_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_tabletype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_tabletype_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_memorytype_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_memorytype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_memorytype_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_externtype_t,
}
pub type wasm_externkind_t = u8;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_t {
    ty: ImportType,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_importtype_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_t {
    ty: ExportType,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_exporttype_t,
}
#[doc = ""]
#[repr(C)]
#[derive(Clone)]
pub struct wasm_ref_t {
    _unused: [u8; 0],
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
    pub f32: float32_t,
    pub f64: float64_t,
    pub ref_: *mut wasm_ref_t,
    _bindgen_union_align: u64,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_val_vec_t {
    pub size: usize,
    pub data: *mut wasm_val_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_frame_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_frame_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_frame_t,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_instance_t {
    instance: Rc<RefCell<Instance>>,
}
pub type wasm_message_t = wasm_name_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_trap_t {
    trap: Rc<RefCell<Trap>>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_foreign_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_module_t {
    module: Rc<RefCell<Module>>,
    imports: Vec<wasm_importtype_t>,
    exports: Vec<wasm_exporttype_t>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_shared_module_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_func_t {
    func: Rc<RefCell<Func>>,
}
pub type wasm_func_callback_t = ::std::option::Option<
    unsafe extern "C" fn(args: *const wasm_val_t, results: *mut wasm_val_t) -> *mut wasm_trap_t,
>;
pub type wasm_func_callback_with_env_t = ::std::option::Option<
    unsafe extern "C" fn(
        env: *mut ::std::os::raw::c_void,
        args: *const wasm_val_t,
        results: *mut wasm_val_t,
    ) -> *mut wasm_trap_t,
>;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_global_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_table_t {
    _unused: [u8; 0],
}
pub type wasm_table_size_t = u32;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_memory_t {
    _unused: [u8; 0],
}
pub type wasm_memory_pages_t = u32;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_extern_t {
    ext: Rc<RefCell<Extern>>,
}
#[repr(C)]
#[derive(Clone)]
pub struct wasm_extern_vec_t {
    pub size: usize,
    pub data: *mut *mut wasm_extern_t,
}

#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_delete(v: *mut wasm_byte_vec_t) {
    let _ = Vec::from_raw_parts((*v).data, 0, (*v).size);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_new_uninitialized(out: *mut wasm_byte_vec_t, size: usize) {
    let mut buffer = vec![0; size];
    let result = out.as_mut().unwrap();
    result.size = buffer.capacity();
    result.data = buffer.as_mut_ptr();
    mem::forget(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_engine_delete(engine: *mut wasm_engine_t) {
    let _ = Box::from_raw(engine);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_engine_new() -> *mut wasm_engine_t {
    let engine = Box::new(wasm_engine_t {
        engine: Rc::new(RefCell::new(Engine::default())),
    });
    Box::into_raw(engine)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_func(e: *mut wasm_extern_t) -> *mut wasm_func_t {
    let func = (*e).ext.borrow().func().clone();
    let func = Box::new(wasm_func_t { func });
    Box::into_raw(func)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_vec_delete(v: *mut wasm_extern_vec_t) {
    let buffer = Vec::from_raw_parts((*v).data, (*v).size, (*v).size);
    for p in buffer {
        // TODO wasm_extern_delete
        let _ = Box::from_raw(p);
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_as_extern(f: *mut wasm_func_t) -> *mut wasm_extern_t {
    let ext = Extern::Func((*f).func.clone());
    let ext = Box::new(wasm_extern_t {
        ext: Rc::new(RefCell::new(ext)),
    });
    Box::into_raw(ext)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_call(
    func: *const wasm_func_t,
    args: *const wasm_val_t,
    results: *mut wasm_val_t,
) -> *mut wasm_trap_t {
    let func = (*func).func.borrow();
    let mut params = Vec::with_capacity(func.param_arity());
    for i in 0..func.param_arity() {
        let val = &(*args.offset(i as isize));
        params.push(val.val());
    }
    match func.call(&params) {
        Ok(out) => {
            for i in 0..func.result_arity() {
                let val = &mut (*results.offset(i as isize));
                *val = wasm_val_t::from_val(&out[i]);
            }
            ptr::null_mut()
        }
        Err(trap) => {
            let trap = Box::new(wasm_trap_t { trap });
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

    fn from_val(val: &Val) -> wasm_val_t {
        match val {
            Val::I32(i) => wasm_val_t {
                kind: from_valtype(ValType::I32),
                of: wasm_val_t__bindgen_ty_1 { i32: *i },
            },
            _ => unimplemented!("wasm_val_t::from_val {:?}", val),
        }
    }

    fn val(&self) -> Val {
        match into_valtype(self.kind) {
            ValType::I32 => Val::from(unsafe { self.of.i32 }),
            _ => unimplemented!("wasm_val_t::val {:?}", self.kind),
        }
    }
}

impl Callable for wasm_func_callback_t {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Rc<RefCell<Trap>>> {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let func = self.expect("wasm_func_callback_t fn");
        let out = unsafe { func(params.as_ptr(), out_results.as_mut_ptr()) };
        if out != ptr::null_mut() {
            let trap: Box<wasm_trap_t> = unsafe { Box::from_raw(out) };
            return Err((*trap).into());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    }
}

impl Into<Rc<RefCell<Trap>>> for wasm_trap_t {
    fn into(self) -> Rc<RefCell<Trap>> {
        self.trap
    }
}

struct CallbackWithEnv {
    callback: wasm_func_callback_with_env_t,
    env: *mut ::std::os::raw::c_void,
    finalizer: ::std::option::Option<unsafe extern "C" fn(env: *mut ::std::os::raw::c_void)>,
}

impl Callable for CallbackWithEnv {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Rc<RefCell<Trap>>> {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let func = self.callback.expect("wasm_func_callback_with_env_t fn");
        let out = unsafe { func(self.env, params.as_ptr(), out_results.as_mut_ptr()) };
        if out != ptr::null_mut() {
            let trap: Box<wasm_trap_t> = unsafe { Box::from_raw(out) };
            return Err((*trap).into());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    }
}

impl Drop for CallbackWithEnv {
    fn drop(&mut self) {
        if let Some(finalizer) = self.finalizer {
            unsafe {
                finalizer(self.env);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: wasm_func_callback_t,
) -> *mut wasm_func_t {
    let store = (*store).store.clone();
    let ty = (*ty).functype.clone();
    let callback = Rc::new(callback);
    let func = Box::new(wasm_func_t {
        func: Rc::new(RefCell::new(Func::new(store, ty, callback))),
    });
    Box::into_raw(func)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_delete(f: *mut wasm_func_t) {
    let _ = Box::from_raw(f);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_functype_new(
    params: *mut wasm_valtype_vec_t,
    results: *mut wasm_valtype_vec_t,
) -> *mut wasm_functype_t {
    let params = Vec::from_raw_parts((*params).data, (*params).size, (*params).size)
        .into_iter()
        .map(|vt| (*vt).ty.clone())
        .collect::<Vec<_>>();
    let results = Vec::from_raw_parts((*results).data, (*results).size, (*results).size)
        .into_iter()
        .map(|vt| (*vt).ty.clone())
        .collect::<Vec<_>>();
    let functype = FuncType::new(params.into_boxed_slice(), results.into_boxed_slice());
    let functype = Box::new(wasm_functype_t { functype });
    Box::into_raw(functype)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_functype_delete(ft: *mut wasm_functype_t) {
    let _ = Box::from_raw(ft);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_delete(instance: *mut wasm_instance_t) {
    let _ = Box::from_raw(instance);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    store: *mut wasm_store_t,
    module: *const wasm_module_t,
    imports: *const *const wasm_extern_t,
    _result: *mut *mut wasm_trap_t,
) -> *mut wasm_instance_t {
    let store = (*store).store.clone();
    let mut externs: Vec<Rc<RefCell<Extern>>> = Vec::with_capacity((*module).imports.len());
    for i in 0..(*module).imports.len() {
        let import = *imports.offset(i as isize);
        externs.push((*import).ext.clone());
    }
    let module = (*module).module.clone();
    match Instance::new(store, module, &externs) {
        Ok(instance) => {
            let instance = Box::new(wasm_instance_t {
                instance: Rc::new(RefCell::new(instance)),
            });
            Box::into_raw(instance)
        }
        _ => unimplemented!(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_exports(
    instance: *const wasm_instance_t,
    out: *mut wasm_extern_vec_t,
) {
    let instance = &(*instance).instance.borrow();
    let exports = instance.exports();
    let mut buffer = Vec::with_capacity(exports.len());
    for e in exports.iter() {
        let ext = Box::new(wasm_extern_t { ext: e.clone() });
        buffer.push(Box::into_raw(ext));
    }
    (*out).size = buffer.capacity();
    (*out).data = buffer.as_mut_ptr();
    mem::forget(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_delete(module: *mut wasm_module_t) {
    let _ = Box::from_raw(module);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_new(
    store: *mut wasm_store_t,
    binary: *const wasm_byte_vec_t,
) -> *mut wasm_module_t {
    let binary = slice::from_raw_parts((*binary).data as *const u8, (*binary).size);
    let store = (*store).store.clone();
    let module = Module::new(store, binary).expect("module");
    let imports = module
        .imports()
        .iter()
        .map(|i| wasm_importtype_t { ty: i.clone() })
        .collect::<Vec<_>>();
    let exports = module
        .exports()
        .iter()
        .map(|e| wasm_exporttype_t { ty: e.clone() })
        .collect::<Vec<_>>();
    let module = Box::new(wasm_module_t {
        module: Rc::new(RefCell::new(module)),
        imports,
        exports,
    });
    Box::into_raw(module)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_delete(store: *mut wasm_store_t) {
    let _ = Box::from_raw(store);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_new(engine: *mut wasm_engine_t) -> *mut wasm_store_t {
    let engine = (*engine).engine.clone();
    let store = Box::new(wasm_store_t {
        store: Rc::new(RefCell::new(Store::new(engine))),
    });
    Box::into_raw(store)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_new_empty(out: *mut wasm_valtype_vec_t) {
    (*out).data = ptr::null_mut();
    (*out).size = 0;
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new_with_env(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    env: *mut ::std::os::raw::c_void,
    finalizer: ::std::option::Option<unsafe extern "C" fn(arg1: *mut ::std::os::raw::c_void)>,
) -> *mut wasm_func_t {
    let store = (*store).store.clone();
    let ty = (*ty).functype.clone();
    let callback = Rc::new(CallbackWithEnv {
        callback,
        env,
        finalizer,
    });
    let func = Box::new(wasm_func_t {
        func: Rc::new(RefCell::new(Func::new(store, ty, callback))),
    });
    Box::into_raw(func)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: *mut wasm_val_t, source: *const wasm_val_t) {
    *out = match into_valtype((*source).kind) {
        ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => (*source).clone(),
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

fn from_valtype(ty: ValType) -> wasm_valkind_t {
    match ty {
        ValType::I32 => 0,
        ValType::I64 => 1,
        ValType::F32 => 2,
        ValType::F64 => 3,
        ValType::AnyRef => 128,
        ValType::FuncRef => 129,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_new(kind: wasm_valkind_t) -> *mut wasm_valtype_t {
    let ty = Box::new(wasm_valtype_t {
        ty: into_valtype(kind),
    });
    Box::into_raw(ty)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_new(
    out: *mut wasm_valtype_vec_t,
    size: usize,
    data: *const *mut wasm_valtype_t,
) {
    let slice = slice::from_raw_parts(data, size);
    let mut buffer = Vec::with_capacity(size);
    buffer.extend_from_slice(slice);
    assert!(size == buffer.capacity());
    (*out).size = size;
    (*out).data = buffer.as_mut_ptr();
    mem::forget(buffer);
}
#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_new(
    out: *mut wasm_byte_vec_t,
    size: usize,
    data: *const wasm_byte_t,
) {
    let slice = slice::from_raw_parts(data, size);
    let mut buffer = Vec::with_capacity(size);
    buffer.extend_from_slice(slice);
    assert!(size == buffer.capacity());
    (*out).size = size;
    (*out).data = buffer.as_mut_ptr();
    mem::forget(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_frame_delete(_arg1: *mut wasm_frame_t) {
    unimplemented!("wasm_frame_delete")
}

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
pub unsafe extern "C" fn wasm_frame_vec_delete(frames: *mut wasm_frame_vec_t) {
    let frames = Vec::from_raw_parts((*frames).data, (*frames).size, (*frames).size);
    for _frame in frames {
        unimplemented!("wasm_frame_vec_delete for frame")
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_delete(trap: *mut wasm_trap_t) {
    let _ = Box::from_raw(trap);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_new(
    _store: *mut wasm_store_t,
    message: *const wasm_message_t,
) -> *mut wasm_trap_t {
    let message = slice::from_raw_parts((*message).data as *const u8, (*message).size);
    if message[message.len() - 1] != 0 {
        panic!("wasm_trap_new message stringz expected");
    }
    let message = String::from_utf8_lossy(message).to_string();
    let trap = Box::new(wasm_trap_t {
        trap: Rc::new(RefCell::new(Trap::new(message))),
    });
    Box::into_raw(trap)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_message(trap: *const wasm_trap_t, out: *mut wasm_message_t) {
    let mut buffer = Vec::new();
    buffer.extend_from_slice((*trap).trap.borrow().message().as_bytes());
    buffer.reserve_exact(1);
    buffer.push(0);
    assert!(buffer.len() == buffer.capacity());
    (*out).size = buffer.capacity();
    (*out).data = buffer.as_mut_ptr() as *mut i8;
    mem::forget(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_origin(_trap: *const wasm_trap_t) -> *mut wasm_frame_t {
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_trace(_trap: *const wasm_trap_t, out: *mut wasm_frame_vec_t) {
    let mut buffer = Vec::new();
    (*out).size = 0;
    (*out).data = buffer.as_mut_ptr();
    mem::forget(buffer);
}
