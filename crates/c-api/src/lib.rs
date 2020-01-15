//! This file defines the extern "C" API, which is compatible with the
//! [Wasm C API](https://github.com/WebAssembly/wasm-c-api).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// TODO complete the C API

use std::cell::RefCell;
use std::rc::Rc;
use std::{mem, ptr, slice};
use wasmtime::{
    AnyRef, Callable, Engine, ExportType, Extern, ExternType, Func, FuncType, Global, GlobalType,
    HostInfo, HostRef, ImportType, Instance, Limits, Memory, MemoryType, Module, Store, Table,
    TableType, Trap, Val, ValType,
};

macro_rules! declare_vec {
    ($name:ident, $elem_ty:path) => {
        #[repr(C)]
        #[derive(Clone)]
        pub struct $name {
            pub size: usize,
            pub data: *mut $elem_ty,
        }

        impl $name {
            #[allow(dead_code)]
            fn set_from_slice(&mut self, source: &[$elem_ty]) {
                let mut buffer = Vec::with_capacity(source.len());
                buffer.extend_from_slice(source);
                assert_eq!(buffer.len(), buffer.capacity());
                self.size = buffer.len();
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn set_buffer(&mut self, mut buffer: Vec<$elem_ty>) {
                assert_eq!(buffer.len(), buffer.capacity());
                self.size = buffer.len();
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn set_uninitialized(&mut self, size: usize) {
                let mut buffer = vec![Default::default(); size];
                self.size = size;
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn uninitialize(&mut self) {
                let _ = unsafe { Vec::from_raw_parts(self.data, self.size, self.size) };
            }

            #[allow(dead_code)]
            fn as_slice(&self) -> &[$elem_ty] {
                unsafe { slice::from_raw_parts(self.data, self.size) }
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
                self.uninitialize();
            }
        }
    };

    ($name:ident, *mut $elem_ty:path) => {
        #[repr(C)]
        #[derive(Clone)]
        pub struct $name {
            pub size: usize,
            pub data: *mut *mut $elem_ty,
        }

        impl $name {
            #[allow(dead_code)]
            fn set_from_slice(&mut self, source: &[*mut $elem_ty]) {
                let mut buffer = Vec::with_capacity(source.len());
                buffer.extend_from_slice(source);
                assert_eq!(buffer.len(), buffer.capacity());
                self.size = buffer.len();
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn set_buffer(&mut self, mut buffer: Vec<*mut $elem_ty>) {
                assert_eq!(buffer.len(), buffer.capacity());
                self.size = buffer.len();
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn set_uninitialized(&mut self, size: usize) {
                let mut buffer = vec![ptr::null_mut(); size];
                self.size = size;
                self.data = buffer.as_mut_ptr();
                mem::forget(buffer);
            }

            #[allow(dead_code)]
            fn uninitialize(&mut self) {
                for element in unsafe { Vec::from_raw_parts(self.data, self.size, self.size) } {
                    let _ = unsafe { Box::from_raw(element) };
                }
            }

            #[allow(dead_code)]
            fn as_slice(&self) -> &[*mut $elem_ty] {
                unsafe { slice::from_raw_parts(self.data, self.size) }
            }
        }

        impl From<Vec<*mut $elem_ty>> for $name {
            fn from(mut vec: Vec<*mut $elem_ty>) -> Self {
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
                self.uninitialize();
            }
        }
    };
}

pub type float32_t = f32;
pub type float64_t = f64;
pub type wasm_byte_t = u8;

declare_vec!(wasm_byte_vec_t, wasm_byte_t);

pub type wasm_name_t = wasm_byte_vec_t;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_config_t {
    _unused: [u8; 0],
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

declare_vec!(wasm_valtype_vec_t, *mut wasm_valtype_t);

pub type wasm_valkind_t = u8;
#[repr(C)]
#[derive(Clone)]
pub struct wasm_functype_t {
    functype: FuncType,
    params_cache: Option<wasm_valtype_vec_t>,
    returns_cache: Option<wasm_valtype_vec_t>,
}

declare_vec!(wasm_functype_vec_t, *mut wasm_functype_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_globaltype_t {
    globaltype: GlobalType,
    content_cache: Option<wasm_valtype_t>,
}

declare_vec!(wasm_globaltype_vec_t, *mut wasm_globaltype_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_tabletype_t {
    tabletype: TableType,
    element_cache: Option<wasm_valtype_t>,
    limits_cache: Option<wasm_limits_t>,
}

declare_vec!(wasm_tabletype_vec_t, *mut wasm_tabletype_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_memorytype_t {
    memorytype: MemoryType,
    limits_cache: Option<wasm_limits_t>,
}

declare_vec!(wasm_memorytype_vec_t, *mut wasm_memorytype_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_externtype_t {
    ty: ExternType,
    cache: wasm_externtype_t_type_cache,
}

#[derive(Clone)]
enum wasm_externtype_t_type_cache {
    Empty,
    Func(wasm_functype_t),
    Global(wasm_globaltype_t),
    Memory(wasm_memorytype_t),
    Table(wasm_tabletype_t),
}

declare_vec!(wasm_externtype_vec_t, *mut wasm_externtype_t);

pub type wasm_externkind_t = u8;

const WASM_EXTERN_FUNC: wasm_externkind_t = 0;
const WASM_EXTERN_GLOBAL: wasm_externkind_t = 1;
const WASM_EXTERN_TABLE: wasm_externkind_t = 2;
const WASM_EXTERN_MEMORY: wasm_externkind_t = 3;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_importtype_t {
    ty: ImportType,
    module_cache: Option<wasm_name_t>,
    name_cache: Option<wasm_name_t>,
}

declare_vec!(wasm_importtype_vec_t, *mut wasm_importtype_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_exporttype_t {
    ty: ExportType,
    name_cache: Option<wasm_name_t>,
    type_cache: Option<wasm_externtype_t>,
}

declare_vec!(wasm_exporttype_vec_t, *mut wasm_exporttype_t);

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

declare_vec!(wasm_val_vec_t, wasm_val_t);

#[repr(C)]
#[derive(Clone)]
pub struct wasm_frame_t {
    _unused: [u8; 0],
}

declare_vec!(wasm_frame_vec_t, *mut wasm_frame_t);

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

pub type wasm_func_callback_t = std::option::Option<
    unsafe extern "C" fn(args: *const wasm_val_t, results: *mut wasm_val_t) -> *mut wasm_trap_t,
>;
pub type wasm_func_callback_with_env_t = std::option::Option<
    unsafe extern "C" fn(
        env: *mut std::ffi::c_void,
        args: *const wasm_val_t,
        results: *mut wasm_val_t,
    ) -> *mut wasm_trap_t,
>;

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

declare_vec!(wasm_extern_vec_t, *mut wasm_extern_t);

#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_delete(v: *mut wasm_byte_vec_t) {
    (*v).uninitialize();
}

#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_new_uninitialized(out: *mut wasm_byte_vec_t, size: usize) {
    (*out).set_uninitialized(size);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_engine_delete(engine: *mut wasm_engine_t) {
    let _ = Box::from_raw(engine);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_engine_new() -> *mut wasm_engine_t {
    let engine = Box::new(wasm_engine_t {
        engine: HostRef::new(Engine::default()),
    });
    Box::into_raw(engine)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_func(e: *mut wasm_extern_t) -> *mut wasm_func_t {
    match &(*e).which {
        ExternHost::Func(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_vec_delete(v: *mut wasm_extern_vec_t) {
    (*v).uninitialize();
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_as_extern(f: *mut wasm_func_t) -> *mut wasm_extern_t {
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
    match func.call(&params) {
        Ok(out) => {
            for i in 0..func.result_arity() {
                let val = &mut (*results.add(i));
                *val = wasm_val_t::from_val(&out[i]);
            }
            ptr::null_mut()
        }
        Err(trap) => {
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

struct Callback {
    callback: wasm_func_callback_t,
}

impl Callable for Callback {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let func = self.callback.expect("wasm_func_callback_t fn");
        let out = unsafe { func(params.as_ptr(), out_results.as_mut_ptr()) };
        if !out.is_null() {
            let trap: Box<wasm_trap_t> = unsafe { Box::from_raw(out) };
            return Err(trap.trap.borrow().clone());
        }
        for i in 0..results.len() {
            results[i] = out_results[i].val();
        }
        Ok(())
    }
}

struct CallbackWithEnv {
    callback: wasm_func_callback_with_env_t,
    env: *mut std::ffi::c_void,
    finalizer: std::option::Option<unsafe extern "C" fn(env: *mut std::ffi::c_void)>,
}

impl Callable for CallbackWithEnv {
    fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
        let params = params
            .iter()
            .map(|p| wasm_val_t::from_val(p))
            .collect::<Vec<_>>();
        let mut out_results = vec![wasm_val_t::default(); results.len()];
        let func = self.callback.expect("wasm_func_callback_with_env_t fn");
        let out = unsafe { func(self.env, params.as_ptr(), out_results.as_mut_ptr()) };
        if !out.is_null() {
            let trap: Box<wasm_trap_t> = unsafe { Box::from_raw(out) };
            return Err(trap.trap.borrow().clone());
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
    let store = &(*store).store.borrow();
    let ty = (*ty).functype.clone();
    let callback = Rc::new(Callback { callback });
    let func = Box::new(wasm_func_t {
        ext: wasm_extern_t {
            which: ExternHost::Func(HostRef::new(Func::new(store, ty, callback))),
        },
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
    let functype = Box::new(wasm_functype_t {
        functype,
        params_cache: None,  // TODO get from args?
        returns_cache: None, // TODO get from args?
    });
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
    result: *mut *mut wasm_trap_t,
) -> *mut wasm_instance_t {
    let mut externs: Vec<Extern> = Vec::with_capacity((*module).imports.len());
    for i in 0..(*module).imports.len() {
        let import = *imports.add(i);
        externs.push(match &(*import).which {
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
    match Instance::new(module, &externs) {
        Ok(instance) => {
            let instance = Box::new(wasm_instance_t {
                instance: HostRef::new(instance),
                exports_cache: RefCell::new(None),
            });
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
pub unsafe extern "C" fn wasm_instance_exports(
    instance: *const wasm_instance_t,
    out: *mut wasm_extern_vec_t,
) {
    let mut cache = (*instance).exports_cache.borrow_mut();
    let exports = cache.get_or_insert_with(|| {
        let instance = &(*instance).instance.borrow();
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
        buffer.push(Box::into_raw(ext));
    }
    (*out).set_buffer(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_delete(module: *mut wasm_module_t) {
    let _ = Box::from_raw(module);
}

impl wasm_name_t {
    fn from_name(name: &str) -> wasm_name_t {
        name.to_string().into_bytes().into()
    }
}

/// Note that this function does not perform validation on the wasm
/// binary. To perform validation, use `wasm_module_validate`.
#[no_mangle]
pub unsafe extern "C" fn wasm_module_new(
    store: *mut wasm_store_t,
    binary: *const wasm_byte_vec_t,
) -> *mut wasm_module_t {
    let binary = (*binary).as_slice();
    let store = &(*store).store.borrow();
    let module = Module::new_unchecked(store, binary).expect("module");
    let imports = module
        .imports()
        .iter()
        .map(|i| wasm_importtype_t {
            ty: i.clone(),
            module_cache: None,
            name_cache: None,
        })
        .collect::<Vec<_>>();
    let exports = module
        .exports()
        .iter()
        .map(|e| wasm_exporttype_t {
            ty: e.clone(),
            name_cache: None,
            type_cache: None,
        })
        .collect::<Vec<_>>();
    let module = Box::new(wasm_module_t {
        module: HostRef::new(module),
        imports,
        exports,
    });
    Box::into_raw(module)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_validate(
    store: *mut wasm_store_t,
    binary: *const wasm_byte_vec_t,
) -> bool {
    let binary = (*binary).as_slice();
    let store = &(*store).store.borrow();
    Module::validate(store, binary).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_delete(store: *mut wasm_store_t) {
    let _ = Box::from_raw(store);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_store_new(engine: *mut wasm_engine_t) -> *mut wasm_store_t {
    let engine = &(*engine).engine;
    let store = Box::new(wasm_store_t {
        store: HostRef::new(Store::new(&engine.borrow())),
    });
    Box::into_raw(store)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_new_empty(out: *mut wasm_valtype_vec_t) {
    (*out).set_uninitialized(0);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_new(
    out: *mut wasm_valtype_vec_t,
    size: usize,
    data: *const *mut wasm_valtype_t,
) {
    let slice = slice::from_raw_parts(data, size);
    (*out).set_from_slice(slice);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_new_uninitialized(
    out: *mut wasm_valtype_vec_t,
    size: usize,
) {
    (*out).set_uninitialized(size);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_new_with_env(
    store: *mut wasm_store_t,
    ty: *const wasm_functype_t,
    callback: wasm_func_callback_with_env_t,
    env: *mut std::ffi::c_void,
    finalizer: std::option::Option<unsafe extern "C" fn(arg1: *mut std::ffi::c_void)>,
) -> *mut wasm_func_t {
    let store = &(*store).store.borrow();
    let ty = (*ty).functype.clone();
    let callback = Rc::new(CallbackWithEnv {
        callback,
        env,
        finalizer,
    });
    let func = Box::new(wasm_func_t {
        ext: wasm_extern_t {
            which: ExternHost::Func(HostRef::new(Func::new(store, ty, callback))),
        },
    });
    Box::into_raw(func)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_val_copy(out: *mut wasm_val_t, source: *const wasm_val_t) {
    *out = match into_valtype((*source).kind) {
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
pub unsafe extern "C" fn wasm_valtype_new(kind: wasm_valkind_t) -> *mut wasm_valtype_t {
    let ty = Box::new(wasm_valtype_t {
        ty: into_valtype(kind),
    });
    Box::into_raw(ty)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_delete(vt: *mut wasm_valtype_t) {
    drop(Box::from_raw(vt));
}

#[no_mangle]
pub unsafe extern "C" fn wasm_byte_vec_new(
    out: *mut wasm_byte_vec_t,
    size: usize,
    data: *const wasm_byte_t,
) {
    let slice = slice::from_raw_parts(data, size);
    (*out).set_from_slice(slice);
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
    (*frames).uninitialize();
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
    let message = (*message).as_slice();
    if message[message.len() - 1] != 0 {
        panic!("wasm_trap_new message stringz expected");
    }
    let message = String::from_utf8_lossy(message);
    let trap = Box::new(wasm_trap_t {
        trap: HostRef::new(Trap::new(message)),
    });
    Box::into_raw(trap)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_message(trap: *const wasm_trap_t, out: *mut wasm_message_t) {
    let mut buffer = Vec::new();
    buffer.extend_from_slice((*trap).trap.borrow().message().as_bytes());
    buffer.reserve_exact(1);
    buffer.push(0);
    (*out).set_buffer(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_origin(_trap: *const wasm_trap_t) -> *mut wasm_frame_t {
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_trap_trace(_trap: *const wasm_trap_t, out: *mut wasm_frame_vec_t) {
    (*out).set_uninitialized(0);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_importtype_module(
    it: *const wasm_importtype_t,
) -> *const wasm_name_t {
    if (*it).module_cache.is_none() {
        let it = (it as *mut wasm_importtype_t).as_mut().unwrap();
        it.module_cache = Some(wasm_name_t::from_name(&it.ty.module()));
    }
    (*it).module_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_importtype_name(it: *const wasm_importtype_t) -> *const wasm_name_t {
    if (*it).name_cache.is_none() {
        let it = (it as *mut wasm_importtype_t).as_mut().unwrap();
        it.name_cache = Some(wasm_name_t::from_name(&it.ty.name()));
    }
    (*it).name_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_importtype_type(
    it: *const wasm_importtype_t,
) -> *const wasm_externtype_t {
    let ty = Box::new(wasm_externtype_t {
        ty: (*it).ty.ty().clone(),
        cache: wasm_externtype_t_type_cache::Empty,
    });
    Box::into_raw(ty)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_importtype_vec_delete(vec: *mut wasm_importtype_vec_t) {
    (*vec).uninitialize();
}

#[no_mangle]
pub unsafe extern "C" fn wasm_exporttype_name(et: *const wasm_exporttype_t) -> *const wasm_name_t {
    if (*et).name_cache.is_none() {
        let et = (et as *mut wasm_exporttype_t).as_mut().unwrap();
        et.name_cache = Some(wasm_name_t::from_name(&et.ty.name()));
    }
    (*et).name_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_exporttype_type(
    et: *const wasm_exporttype_t,
) -> *const wasm_externtype_t {
    if (*et).type_cache.is_none() {
        let et = (et as *mut wasm_exporttype_t).as_mut().unwrap();
        et.type_cache = Some(wasm_externtype_t {
            ty: (*et).ty.ty().clone(),
            cache: wasm_externtype_t_type_cache::Empty,
        });
    }
    (*et).type_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_exporttype_vec_delete(et: *mut wasm_exporttype_vec_t) {
    (*et).uninitialize();
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_kind(e: *const wasm_extern_t) -> wasm_externkind_t {
    match (*e).which {
        ExternHost::Func(_) => WASM_EXTERN_FUNC,
        ExternHost::Global(_) => WASM_EXTERN_GLOBAL,
        ExternHost::Table(_) => WASM_EXTERN_TABLE,
        ExternHost::Memory(_) => WASM_EXTERN_MEMORY,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_type(e: *const wasm_extern_t) -> *mut wasm_externtype_t {
    let et = Box::new(wasm_externtype_t {
        ty: match &(*e).which {
            ExternHost::Func(f) => ExternType::Func(f.borrow().ty().clone()),
            ExternHost::Global(f) => ExternType::Global(f.borrow().ty().clone()),
            ExternHost::Table(f) => ExternType::Table(f.borrow().ty().clone()),
            ExternHost::Memory(f) => ExternType::Memory(f.borrow().ty().clone()),
        },
        cache: wasm_externtype_t_type_cache::Empty,
    });
    Box::into_raw(et)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_functype_const(
    et: *const wasm_externtype_t,
) -> *const wasm_functype_t {
    if let wasm_externtype_t_type_cache::Empty = (*et).cache {
        let functype = (*et).ty.unwrap_func().clone();
        let f = wasm_functype_t {
            functype,
            params_cache: None,
            returns_cache: None,
        };
        let et = (et as *mut wasm_externtype_t).as_mut().unwrap();
        et.cache = wasm_externtype_t_type_cache::Func(f);
    }
    match &(*et).cache {
        wasm_externtype_t_type_cache::Func(f) => f,
        _ => panic!("wasm_externtype_as_functype_const"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_globaltype_const(
    et: *const wasm_externtype_t,
) -> *const wasm_globaltype_t {
    if let wasm_externtype_t_type_cache::Empty = (*et).cache {
        let globaltype = (*et).ty.unwrap_global().clone();
        let g = wasm_globaltype_t {
            globaltype,
            content_cache: None,
        };
        let et = (et as *mut wasm_externtype_t).as_mut().unwrap();
        et.cache = wasm_externtype_t_type_cache::Global(g);
    }
    match &(*et).cache {
        wasm_externtype_t_type_cache::Global(g) => g,
        _ => panic!("wasm_externtype_as_globaltype_const"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_tabletype_const(
    et: *const wasm_externtype_t,
) -> *const wasm_tabletype_t {
    if let wasm_externtype_t_type_cache::Empty = (*et).cache {
        let tabletype = (*et).ty.unwrap_table().clone();
        let t = wasm_tabletype_t {
            tabletype,
            element_cache: None,
            limits_cache: None,
        };
        let et = (et as *mut wasm_externtype_t).as_mut().unwrap();
        et.cache = wasm_externtype_t_type_cache::Table(t);
    }
    match &(*et).cache {
        wasm_externtype_t_type_cache::Table(t) => t,
        _ => panic!("wasm_externtype_as_tabletype_const"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_externtype_as_memorytype_const(
    et: *const wasm_externtype_t,
) -> *const wasm_memorytype_t {
    if let wasm_externtype_t_type_cache::Empty = (*et).cache {
        let memorytype = (*et).ty.unwrap_memory().clone();
        let m = wasm_memorytype_t {
            memorytype,
            limits_cache: None,
        };
        let et = (et as *mut wasm_externtype_t).as_mut().unwrap();
        et.cache = wasm_externtype_t_type_cache::Memory(m);
    }
    match &(*et).cache {
        wasm_externtype_t_type_cache::Memory(m) => m,
        _ => panic!("wasm_externtype_as_memorytype_const"),
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
pub unsafe extern "C" fn wasm_func_param_arity(f: *const wasm_func_t) -> usize {
    (*f).func().borrow().param_arity()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_func_result_arity(f: *const wasm_func_t) -> usize {
    (*f).func().borrow().result_arity()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_functype_params(
    ft: *const wasm_functype_t,
) -> *const wasm_valtype_vec_t {
    if (*ft).params_cache.is_none() {
        let ft = (ft as *mut wasm_functype_t).as_mut().unwrap();
        let buffer = ft
            .functype
            .params()
            .iter()
            .map(|p| {
                let ty = Box::new(wasm_valtype_t { ty: p.clone() });
                Box::into_raw(ty)
            })
            .collect::<Vec<_>>();
        ft.params_cache = Some(buffer.into());
    }
    (*ft).params_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_functype_results(
    ft: *const wasm_functype_t,
) -> *const wasm_valtype_vec_t {
    if (*ft).returns_cache.is_none() {
        let ft = (ft as *mut wasm_functype_t).as_mut().unwrap();
        let buffer = ft
            .functype
            .results()
            .iter()
            .map(|p| {
                let ty = Box::new(wasm_valtype_t { ty: p.clone() });
                Box::into_raw(ty)
            })
            .collect::<Vec<_>>();
        ft.returns_cache = Some(buffer.into());
    }
    (*ft).returns_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_globaltype_content(
    gt: *const wasm_globaltype_t,
) -> *const wasm_valtype_t {
    if (*gt).content_cache.is_none() {
        let gt = (gt as *mut wasm_globaltype_t).as_mut().unwrap();
        gt.content_cache = Some(wasm_valtype_t {
            ty: (*gt).globaltype.content().clone(),
        });
    }
    (*gt).content_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_globaltype_mutability(
    gt: *const wasm_globaltype_t,
) -> wasm_mutability_t {
    use wasmtime::Mutability::*;
    match (*gt).globaltype.mutability() {
        Const => 0,
        Var => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memorytype_limits(
    mt: *const wasm_memorytype_t,
) -> *const wasm_limits_t {
    if (*mt).limits_cache.is_none() {
        let mt = (mt as *mut wasm_memorytype_t).as_mut().unwrap();
        let limits = (*mt).memorytype.limits();
        mt.limits_cache = Some(wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        });
    }
    (*mt).limits_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_exports(
    module: *const wasm_module_t,
    out: *mut wasm_exporttype_vec_t,
) {
    let buffer = (*module)
        .exports
        .iter()
        .map(|et| {
            let et = Box::new(et.clone());
            Box::into_raw(et)
        })
        .collect::<Vec<_>>();
    (*out).set_buffer(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_module_imports(
    module: *const wasm_module_t,
    out: *mut wasm_importtype_vec_t,
) {
    let buffer = (*module)
        .imports
        .iter()
        .map(|it| {
            let it = Box::new(it.clone());
            Box::into_raw(it)
        })
        .collect::<Vec<_>>();
    (*out).set_buffer(buffer);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_tabletype_element(
    tt: *const wasm_tabletype_t,
) -> *const wasm_valtype_t {
    if (*tt).element_cache.is_none() {
        let tt = (tt as *mut wasm_tabletype_t).as_mut().unwrap();
        tt.element_cache = Some(wasm_valtype_t {
            ty: (*tt).tabletype.element().clone(),
        });
    }
    (*tt).element_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_tabletype_limits(
    tt: *const wasm_tabletype_t,
) -> *const wasm_limits_t {
    if (*tt).limits_cache.is_none() {
        let tt = (tt as *mut wasm_tabletype_t).as_mut().unwrap();
        let limits = (*tt).tabletype.limits();
        tt.limits_cache = Some(wasm_limits_t {
            min: limits.min(),
            max: limits.max().unwrap_or(u32::max_value()),
        });
    }
    (*tt).limits_cache.as_ref().unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_kind(vt: *const wasm_valtype_t) -> wasm_valkind_t {
    from_valtype(&(*vt).ty)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_global(e: *mut wasm_extern_t) -> *mut wasm_global_t {
    match &(*e).which {
        ExternHost::Global(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_as_extern(g: *mut wasm_global_t) -> *mut wasm_extern_t {
    &mut (*g).ext
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_delete(g: *mut wasm_global_t) {
    let _ = Box::from_raw(g);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_copy(g: *const wasm_global_t) -> *mut wasm_global_t {
    Box::into_raw(Box::new((*g).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_same(
    g1: *const wasm_global_t,
    g2: *const wasm_global_t,
) -> bool {
    (*g1).global().ptr_eq(&(*g2).global())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_new(
    store: *mut wasm_store_t,
    gt: *const wasm_globaltype_t,
    val: *const wasm_val_t,
) -> *mut wasm_global_t {
    let global = HostRef::new(Global::new(
        &(*store).store.borrow(),
        (*gt).globaltype.clone(),
        (*val).val(),
    ));
    let g = Box::new(wasm_global_t {
        ext: wasm_extern_t {
            which: ExternHost::Global(global),
        },
    });
    Box::into_raw(g)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_get(g: *const wasm_global_t, out: *mut wasm_val_t) {
    (*out).set((*g).global().borrow().get());
}

#[no_mangle]
pub unsafe extern "C" fn wasm_global_set(g: *mut wasm_global_t, val: *const wasm_val_t) {
    (*g).global().borrow().set((*val).val())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_globaltype_delete(gt: *mut wasm_globaltype_t) {
    let _ = Box::from_raw(gt);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_globaltype_new(
    ty: *mut wasm_valtype_t,
    mutability: wasm_mutability_t,
) -> *mut wasm_globaltype_t {
    use wasmtime::Mutability::*;
    let ty = Box::from_raw(ty);
    let mutability = match mutability {
        0 => Const,
        1 => Var,
        _ => panic!("mutability out-of-range"),
    };
    let globaltype = GlobalType::new(ty.ty.clone(), mutability);
    let gt = Box::new(wasm_globaltype_t {
        globaltype,
        content_cache: Some(*ty),
    });
    Box::into_raw(gt)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_extern_as_memory(e: *mut wasm_extern_t) -> *mut wasm_memory_t {
    match &(*e).which {
        ExternHost::Memory(_) => e.cast(),
        _ => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_as_extern(m: *mut wasm_memory_t) -> *mut wasm_extern_t {
    &mut (*m).ext
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_delete(m: *mut wasm_memory_t) {
    let _ = Box::from_raw(m);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_copy(m: *const wasm_memory_t) -> *mut wasm_memory_t {
    Box::into_raw(Box::new((*m).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_same(
    m1: *const wasm_memory_t,
    m2: *const wasm_memory_t,
) -> bool {
    (*m1).memory().ptr_eq(&(*m2).memory())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_data(m: *mut wasm_memory_t) -> *mut u8 {
    (*m).memory().borrow().data_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_data_size(m: *const wasm_memory_t) -> usize {
    (*m).memory().borrow().data_size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_size(m: *const wasm_memory_t) -> wasm_memory_pages_t {
    (*m).memory().borrow().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_grow(
    m: *mut wasm_memory_t,
    delta: wasm_memory_pages_t,
) -> bool {
    (*m).memory().borrow().grow(delta)
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
pub unsafe extern "C" fn wasm_memorytype_new(
    limits: *const wasm_limits_t,
) -> *mut wasm_memorytype_t {
    let max = if (*limits).max == u32::max_value() {
        None
    } else {
        Some((*limits).max)
    };
    let limits = Limits::new((*limits).min, max);
    let mt = Box::new(wasm_memorytype_t {
        memorytype: MemoryType::new(limits),
        limits_cache: None,
    });
    Box::into_raw(mt)
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
    let t = Box::new(wasm_table_t {
        ext: wasm_extern_t {
            which: ExternHost::Table(HostRef::new(Table::new(
                &(*store).store.borrow(),
                (*tt).tabletype.clone(),
                init,
            ))),
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
pub unsafe extern "C" fn wasm_table_get(
    t: *const wasm_table_t,
    index: wasm_table_size_t,
) -> *mut wasm_ref_t {
    let val = (*t).table().borrow().get(index);
    into_funcref(val)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_set(
    t: *mut wasm_table_t,
    index: wasm_table_size_t,
    r: *mut wasm_ref_t,
) -> bool {
    let val = from_funcref(r);
    (*t).table().borrow().set(index, val)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_size(t: *const wasm_table_t) -> wasm_table_size_t {
    (*t).table().borrow().size()
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_grow(
    t: *mut wasm_table_t,
    delta: wasm_table_size_t,
    init: *mut wasm_ref_t,
) -> bool {
    let init = from_funcref(init);
    (*t).table().borrow().grow(delta, init)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_table_same(t1: *const wasm_table_t, t2: *const wasm_table_t) -> bool {
    (*t1).table().ptr_eq((*t2).table())
}

#[no_mangle]
pub unsafe extern "C" fn wasm_tabletype_delete(tt: *mut wasm_tabletype_t) {
    let _ = Box::from_raw(tt);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_tabletype_new(
    ty: *mut wasm_valtype_t,
    limits: *const wasm_limits_t,
) -> *mut wasm_tabletype_t {
    let ty = Box::from_raw(ty).ty;
    let max = if (*limits).max == u32::max_value() {
        None
    } else {
        Some((*limits).max)
    };
    let limits = Limits::new((*limits).min, max);
    let tt = Box::new(wasm_tabletype_t {
        tabletype: TableType::new(ty, limits),
        element_cache: None,
        limits_cache: None,
    });
    Box::into_raw(tt)
}

struct HostInfoState {
    info: *mut std::ffi::c_void,
    finalizer: std::option::Option<unsafe extern "C" fn(arg1: *mut std::ffi::c_void)>,
}

impl HostInfo for HostInfoState {
    fn finalize(&mut self) {
        if let Some(f) = &self.finalizer {
            unsafe {
                f(self.info);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_instance_set_host_info_with_finalizer(
    instance: *mut wasm_instance_t,
    info: *mut std::ffi::c_void,
    finalizer: std::option::Option<unsafe extern "C" fn(arg1: *mut std::ffi::c_void)>,
) {
    let info = if info.is_null() && finalizer.is_none() {
        None
    } else {
        let b: Box<dyn HostInfo> = Box::new(HostInfoState { info, finalizer });
        Some(b)
    };
    (*instance).instance.anyref().set_host_info(info);
}

#[no_mangle]
pub unsafe extern "C" fn wasm_valtype_vec_copy(
    out: *mut wasm_valtype_vec_t,
    src: *mut wasm_valtype_vec_t,
) {
    let slice = slice::from_raw_parts((*src).data, (*src).size);
    (*out).set_from_slice(slice);
}
