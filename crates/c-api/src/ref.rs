use wasmtime::AnyRef;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_ref_t {
    pub(crate) r: AnyRef,
}

#[no_mangle]
pub extern "C" fn wasm_ref_delete(_r: Option<Box<wasm_ref_t>>) {
}
