use crate::HostInfoState;
use std::os::raw::c_void;
use wasmtime::ExternRef;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_ref_t {
    pub(crate) r: Option<ExternRef>,
}

wasmtime_c_api_macros::declare_own!(wasm_ref_t);

#[no_mangle]
pub extern "C" fn wasm_ref_copy(r: &wasm_ref_t) -> Box<wasm_ref_t> {
    Box::new(r.clone())
}

#[no_mangle]
pub extern "C" fn wasm_ref_same(a: &wasm_ref_t, b: &wasm_ref_t) -> bool {
    match (a.r.as_ref(), b.r.as_ref()) {
        (Some(a), Some(b)) => a.ptr_eq(b),
        (None, None) => true,
        _ => false,
    }
}

pub(crate) fn get_host_info(r: &ExternRef) -> *mut c_void {
    let host_info = match r.host_info() {
        Some(info) => info,
        None => return std::ptr::null_mut(),
    };
    let host_info = host_info.borrow();
    match host_info.downcast_ref::<HostInfoState>() {
        Some(state) => state.info,
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn wasm_ref_get_host_info(a: &wasm_ref_t) -> *mut c_void {
    a.r.as_ref()
        .map_or(std::ptr::null_mut(), |r| get_host_info(r))
}

pub(crate) fn set_host_info(
    r: &ExternRef,
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
    a.r.as_ref().map(|r| set_host_info(r, info, None));
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    a: &wasm_ref_t,
    info: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    a.r.as_ref().map(|r| set_host_info(r, info, finalizer));
}
