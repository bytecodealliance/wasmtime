use crate::HostInfoState;
use std::os::raw::c_void;
use wasmtime::AnyRef;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_ref_t {
    pub(crate) r: AnyRef,
}

wasmtime_c_api_macros::declare_own!(wasm_ref_t);

#[no_mangle]
pub extern "C" fn wasm_ref_copy(r: &wasm_ref_t) -> Box<wasm_ref_t> {
    Box::new(r.clone())
}

#[no_mangle]
pub extern "C" fn wasm_ref_same(a: &wasm_ref_t, b: &wasm_ref_t) -> bool {
    a.r.ptr_eq(&b.r)
}

pub(crate) fn get_host_info(r: &AnyRef) -> *mut c_void {
    let host_info = match r.host_info() {
        Some(info) => info,
        None => return std::ptr::null_mut(),
    };
    match host_info.downcast_ref::<HostInfoState>() {
        Some(state) => state.info,
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn wasm_ref_get_host_info(a: &wasm_ref_t) -> *mut c_void {
    get_host_info(&a.r)
}

pub(crate) fn set_host_info(
    r: &AnyRef,
    info: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    let info = if info.is_null() && finalizer.is_none() {
        None
    } else {
        Some(Box::new(crate::HostInfoState { info, finalizer }) as Box<dyn std::any::Any>)
    };
    r.set_host_info(info);
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info(a: &wasm_ref_t, info: *mut c_void) {
    set_host_info(&a.r, info, None)
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    a: &wasm_ref_t,
    info: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    set_host_info(&a.r, info, finalizer)
}
