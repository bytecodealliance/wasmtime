use std::os::raw::c_void;
use wasmtime::{ExternRef, Func, Val};

/// `*mut wasm_ref_t` is a reference type (`externref` or `funcref`), as seen by
/// the C API. Because we do not have a uniform representation for `funcref`s
/// and `externref`s, a `*mut wasm_ref_t` is morally a
/// `Option<Box<Either<ExternRef, Func>>>`.
///
/// A null `*mut wasm_ref_t` is either a null `funcref` or a null `externref`
/// depending on context (e.g. the table's element type that it is going into or
/// coming out of).
///
/// Note: this is not `#[repr(C)]` because it is an opaque type in the header,
/// and only ever referenced as `*mut wasm_ref_t`. This also lets us use a
/// regular, non-`repr(C)` `enum` to define `WasmRefInner`.
#[derive(Clone)]
pub struct wasm_ref_t {
    pub(crate) r: WasmRefInner,
}

#[derive(Clone)]
pub(crate) enum WasmRefInner {
    ExternRef(ExternRef),
    FuncRef(Func),
}

wasmtime_c_api_macros::declare_own!(wasm_ref_t);

pub(crate) fn ref_to_val(r: &wasm_ref_t) -> Val {
    match &r.r {
        WasmRefInner::ExternRef(x) => Val::ExternRef(Some(x.clone())),
        WasmRefInner::FuncRef(f) => Val::FuncRef(Some(f.clone())),
    }
}

pub(crate) fn val_into_ref(val: Val) -> Option<Box<wasm_ref_t>> {
    match val {
        Val::ExternRef(Some(x)) => Some(Box::new(wasm_ref_t {
            r: WasmRefInner::ExternRef(x),
        })),
        Val::FuncRef(Some(f)) => Some(Box::new(wasm_ref_t {
            r: WasmRefInner::FuncRef(f),
        })),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn wasm_ref_copy(r: Option<&wasm_ref_t>) -> Option<Box<wasm_ref_t>> {
    r.map(|r| Box::new(r.clone()))
}

fn abort(name: &str) -> ! {
    eprintln!("`{}` is not implemented", name);
    std::process::abort();
}

#[no_mangle]
pub extern "C" fn wasm_ref_same(a: Option<&wasm_ref_t>, b: Option<&wasm_ref_t>) -> bool {
    match (a.map(|a| &a.r), b.map(|b| &b.r)) {
        (Some(WasmRefInner::ExternRef(a)), Some(WasmRefInner::ExternRef(b))) => a.ptr_eq(b),
        (None, None) => true,
        // Note: we don't support equality for `Func`, so we always return
        // `false` for `funcref`s.
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn wasm_ref_get_host_info(_ref: Option<&wasm_ref_t>) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info(_ref: Option<&wasm_ref_t>, _info: *mut c_void) {
    abort("wasm_ref_set_host_info")
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    _ref: Option<&wasm_ref_t>,
    _info: *mut c_void,
    _finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    abort("wasm_ref_set_host_info_with_finalizer")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_extern(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_extern_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_extern_t> {
    abort("wasm_ref_as_extern_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_foreign(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_foreign_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_foreign_t> {
    abort("wasm_ref_as_foreign_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_func(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_func_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_func_t> {
    abort("wasm_ref_as_func_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_global(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_global_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_global_t> {
    abort("wasm_ref_as_global_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_instance(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_instance_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_instance_t> {
    abort("wasm_ref_as_instance_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_memory(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_memory_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_memory_t> {
    abort("wasm_ref_as_memory_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_module(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_module_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_module_t> {
    abort("wasm_ref_as_module_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_table(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_table_const(
    _ref: Option<&wasm_ref_t>,
) -> Option<&crate::wasm_table_t> {
    abort("wasm_ref_as_table_const")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_trap(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap")
}

#[no_mangle]
pub extern "C" fn wasm_ref_as_trap_const(_ref: Option<&wasm_ref_t>) -> Option<&crate::wasm_trap_t> {
    abort("wasm_ref_as_trap_const")
}

#[derive(Clone)]
#[repr(C)]
pub struct wasm_foreign_t {}

wasmtime_c_api_macros::declare_ref!(wasm_foreign_t);

#[no_mangle]
pub extern "C" fn wasm_foreign_new(_store: &crate::wasm_store_t) -> Box<wasm_foreign_t> {
    abort("wasm_foreign_new")
}
