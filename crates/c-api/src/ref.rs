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

#[no_mangle]
pub extern "C" fn wasm_ref_get_host_info(_ref: &wasm_ref_t) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info(_ref: &wasm_ref_t, _info: *mut c_void) {
    eprintln!("`wasm_ref_set_host_info` is not implemented");
    std::process::abort();
}

#[no_mangle]
pub extern "C" fn wasm_ref_set_host_info_with_finalizer(
    _ref: &wasm_ref_t,
    _info: *mut c_void,
    _finalizer: Option<extern "C" fn(*mut c_void)>,
) {
    eprintln!("`wasm_ref_set_host_info_with_finalizer` is not implemented");
    std::process::abort();
}
