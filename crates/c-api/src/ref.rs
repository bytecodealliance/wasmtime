use crate::wasm_val_t;
use std::any::Any;
use std::os::raw::c_void;
use std::ptr;
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

pub(crate) fn ref_into_val(r: Option<Box<wasm_ref_t>>) -> Option<Val> {
    // Let callers decide whether to treat this as a null `funcref` or a
    // null `externref`.
    let r = r?;

    Some(match r.r {
        WasmRefInner::ExternRef(x) => Val::ExternRef(Some(x)),
        WasmRefInner::FuncRef(f) => Val::FuncRef(Some(f)),
    })
}

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
    eprintln!("`wasm_ref_set_host_info` is not implemented");
    std::process::abort();
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    _ref: Option<&wasm_ref_t>,
    _info: *mut c_void,
    _finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    eprintln!("`wasm_ref_set_host_info_with_finalizer` is not implemented");
    std::process::abort();
}

type wasmtime_externref_finalizer_t = extern "C" fn(*mut c_void);

struct CExternRef {
    data: *mut c_void,
    finalizer: Option<wasmtime_externref_finalizer_t>,
}

impl Drop for CExternRef {
    fn drop(&mut self) {
        if let Some(f) = self.finalizer {
            f(self.data);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_new(data: *mut c_void) -> wasm_val_t {
    wasmtime_externref_new_with_finalizer(data, None)
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_new_with_finalizer(
    data: *mut c_void,
    finalizer: Option<wasmtime_externref_finalizer_t>,
) -> wasm_val_t {
    wasm_val_t::from_val(Val::ExternRef(Some(ExternRef::new(CExternRef {
        data,
        finalizer,
    }))))
}

#[no_mangle]
pub extern "C" fn wasmtime_externref_data(val: &wasm_val_t, datap: *mut *mut c_void) -> bool {
    match val.val() {
        Val::ExternRef(None) => {
            unsafe {
                ptr::write(datap, ptr::null_mut());
            }
            true
        }
        Val::ExternRef(Some(x)) => {
            let data = match x.data().downcast_ref::<CExternRef>() {
                Some(r) => r.data,
                None => x.data() as *const dyn Any as *mut c_void,
            };
            unsafe {
                ptr::write(datap, data);
            }
            true
        }
        _ => false,
    }
}
