use crate::WasmtimeStoreContextMut;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use wasmtime::{ExternRef, RootScope};

crate::anyref::ref_wrapper!({
    wasmtime: ExternRef,
    capi: wasmtime_externref_t,
    clone: wasmtime_externref_clone,
    unroot: wasmtime_externref_unroot,
    to_raw: wasmtime_externref_to_raw,
    from_raw: wasmtime_externref_from_raw,
});

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_externref_new(
    cx: WasmtimeStoreContextMut<'_>,
    data: *mut c_void,
    finalizer: Option<extern "C" fn(*mut c_void)>,
    out: &mut MaybeUninit<wasmtime_externref_t>,
) -> bool {
    let mut scope = RootScope::new(cx);
    let e = match ExternRef::new(&mut scope, crate::ForeignData { data, finalizer }) {
        Ok(e) => e,
        Err(_) => return false,
    };
    let e = e.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(e).into());
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_externref_data(
    cx: WasmtimeStoreContextMut<'_>,
    externref: Option<&wasmtime_externref_t>,
) -> *mut c_void {
    externref
        .and_then(|e| e.as_wasmtime())
        .and_then(|e| {
            let data = e.data(cx).ok()??;
            Some(data.downcast_ref::<crate::ForeignData>().unwrap().data)
        })
        .unwrap_or(core::ptr::null_mut())
}
