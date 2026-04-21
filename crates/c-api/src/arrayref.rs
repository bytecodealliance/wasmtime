use crate::{WasmtimeStoreContextMut, wasmtime_anyref_t, wasmtime_array_type_t, wasmtime_eqref_t};
use std::mem::MaybeUninit;
use wasmtime::{ArrayRef, ArrayRefPre, OwnedRooted, RootScope};

crate::anyref::ref_wrapper!({
    wasmtime: ArrayRef,
    capi: wasmtime_arrayref_t,
    clone: wasmtime_arrayref_clone,
    unroot: wasmtime_arrayref_unroot,
});

pub struct wasmtime_array_ref_pre_t {
    pre: ArrayRefPre,
}
wasmtime_c_api_macros::declare_own!(wasmtime_array_ref_pre_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_array_ref_pre_new(
    cx: WasmtimeStoreContextMut<'_>,
    ty: &wasmtime_array_type_t,
) -> Box<wasmtime_array_ref_pre_t> {
    let pre = ArrayRefPre::new(cx, ty.ty.clone());
    Box::new(wasmtime_array_ref_pre_t { pre })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_new(
    mut cx: WasmtimeStoreContextMut<'_>,
    pre: &wasmtime_array_ref_pre_t,
    elem: &crate::wasmtime_val_t,
    len: u32,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut cx);
    let val = elem.to_val(&mut scope);
    match ArrayRef::new(&mut scope, &pre.pre, &val, len) {
        Ok(arrayref) => {
            let owned = arrayref
                .to_owned_rooted(&mut scope)
                .expect("just allocated");
            crate::initialize(out, Some(owned).into());
            None
        }
        Err(e) => {
            crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
            Some(Box::new(e.into()))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_to_anyref(
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = arrayref
        .and_then(|a| a.as_wasmtime())
        .map(|a| a.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_to_eqref(
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let eqref = arrayref.and_then(|a| a.as_wasmtime()).map(|a| a.to_eqref());
    crate::initialize(out, eqref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_len(
    cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    out: &mut MaybeUninit<u32>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    match arrayref.len(&cx) {
        Ok(len) => {
            crate::initialize(out, len);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_get(
    mut cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    index: u32,
    out: &mut MaybeUninit<crate::wasmtime_val_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = arrayref.to_rooted(&mut scope);
    match rooted.get(&mut scope, index) {
        Ok(val) => {
            let c_val = crate::wasmtime_val_t::from_val(&mut scope, val);
            crate::initialize(out, c_val);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_arrayref_set(
    mut cx: WasmtimeStoreContextMut<'_>,
    arrayref: Option<&wasmtime_arrayref_t>,
    index: u32,
    val: &crate::wasmtime_val_t,
) -> Option<Box<crate::wasmtime_error_t>> {
    let arrayref = arrayref
        .and_then(|a| a.as_wasmtime())
        .expect("non-null arrayref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = arrayref.to_rooted(&mut scope);
    let rust_val = val.to_val(&mut scope);
    match rooted.set(&mut scope, index, rust_val) {
        Ok(()) => None,
        Err(e) => Some(Box::new(e.into())),
    }
}
