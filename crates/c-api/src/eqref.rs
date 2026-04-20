use crate::{
    WasmtimeStoreContextMut, wasmtime_anyref_t, wasmtime_arrayref_t, wasmtime_structref_t,
};
use std::mem::MaybeUninit;
use wasmtime::{ArrayRef, EqRef, I31, OwnedRooted, RootScope, StructRef};

crate::anyref::ref_wrapper!({
    wasmtime: EqRef,
    capi: wasmtime_eqref_t,
    clone: wasmtime_eqref_clone,
    unroot: wasmtime_eqref_unroot,
});

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_to_anyref(
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = eqref.and_then(|e| e.as_wasmtime()).map(|e| e.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_eqref_from_i31(
    cx: WasmtimeStoreContextMut<'_>,
    val: u32,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let mut scope = RootScope::new(cx);
    let eqref = EqRef::from_i31(&mut scope, I31::wrapping_u32(val));
    let eqref = eqref.to_owned_rooted(&mut scope).expect("in scope");
    crate::initialize(out, Some(eqref).into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_i31(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_i31(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_i31_get_u(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    dst: &mut MaybeUninit<u32>,
) -> bool {
    let mut scope = RootScope::new(cx);
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        if let Some(val) = eqref.as_i31(&mut scope).expect("in scope") {
            crate::initialize(dst, val.get_u32());
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_i31_get_s(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    dst: &mut MaybeUninit<i32>,
) -> bool {
    let mut scope = RootScope::new(cx);
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        if let Some(val) = eqref.as_i31(&mut scope).expect("in scope") {
            crate::initialize(dst, val.get_i32());
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_struct(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_struct(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_as_struct(
    mut cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> bool {
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = eqref.to_rooted(&mut scope);
        if let Ok(Some(structref)) = rooted.as_struct(&scope) {
            let owned = structref.to_owned_rooted(&mut scope).expect("in scope");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
    false
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_is_array(
    cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
) -> bool {
    match eqref.and_then(|e| e.as_wasmtime()) {
        Some(eqref) => eqref.is_array(&cx).expect("OwnedRooted always in scope"),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_eqref_as_array(
    mut cx: WasmtimeStoreContextMut<'_>,
    eqref: Option<&wasmtime_eqref_t>,
    out: &mut MaybeUninit<wasmtime_arrayref_t>,
) -> bool {
    if let Some(eqref) = eqref.and_then(|e| e.as_wasmtime()) {
        let mut scope = RootScope::new(&mut cx);
        let rooted = eqref.to_rooted(&mut scope);
        if let Ok(Some(arrayref)) = rooted.as_array(&scope) {
            let owned = arrayref.to_owned_rooted(&mut scope).expect("just created");
            crate::initialize(out, Some(owned).into());
            return true;
        }
    }
    crate::initialize(out, None::<OwnedRooted<ArrayRef>>.into());
    false
}
