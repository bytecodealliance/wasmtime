use crate::{WasmtimeStoreContextMut, wasmtime_anyref_t, wasmtime_eqref_t, wasmtime_struct_type_t};
use std::mem::MaybeUninit;
use wasmtime::{OwnedRooted, RootScope, StructRef, StructRefPre, Val};

crate::anyref::ref_wrapper!({
    wasmtime: StructRef,
    capi: wasmtime_structref_t,
    clone: wasmtime_structref_clone,
    unroot: wasmtime_structref_unroot,
});

pub struct wasmtime_struct_ref_pre_t {
    pre: StructRefPre,
}
wasmtime_c_api_macros::declare_own!(wasmtime_struct_ref_pre_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_ref_pre_new(
    cx: WasmtimeStoreContextMut<'_>,
    ty: &wasmtime_struct_type_t,
) -> Box<wasmtime_struct_ref_pre_t> {
    let pre = StructRefPre::new(cx, ty.ty.clone());
    Box::new(wasmtime_struct_ref_pre_t { pre })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_new(
    mut cx: WasmtimeStoreContextMut<'_>,
    pre: &wasmtime_struct_ref_pre_t,
    fields: *const crate::wasmtime_val_t,
    nfields: usize,
    out: &mut MaybeUninit<wasmtime_structref_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let c_fields = crate::slice_from_raw_parts(fields, nfields);
    let mut scope = RootScope::new(&mut cx);
    let vals: Vec<Val> = c_fields.iter().map(|v| v.to_val(&mut scope)).collect();
    match StructRef::new(&mut scope, &pre.pre, &vals) {
        Ok(structref) => {
            let owned = structref
                .to_owned_rooted(&mut scope)
                .expect("just allocated");
            crate::initialize(out, Some(owned).into());
            None
        }
        Err(e) => {
            crate::initialize(out, None::<OwnedRooted<StructRef>>.into());
            Some(Box::new(e.into()))
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_to_anyref(
    structref: Option<&wasmtime_structref_t>,
    out: &mut MaybeUninit<wasmtime_anyref_t>,
) {
    let anyref = structref
        .and_then(|s| s.as_wasmtime())
        .map(|s| s.to_anyref());
    crate::initialize(out, anyref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_to_eqref(
    structref: Option<&wasmtime_structref_t>,
    out: &mut MaybeUninit<wasmtime_eqref_t>,
) {
    let eqref = structref
        .and_then(|s| s.as_wasmtime())
        .map(|s| s.to_eqref());
    crate::initialize(out, eqref.into());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_field(
    mut cx: WasmtimeStoreContextMut<'_>,
    structref: Option<&wasmtime_structref_t>,
    index: usize,
    out: &mut MaybeUninit<crate::wasmtime_val_t>,
) -> Option<Box<crate::wasmtime_error_t>> {
    let structref = structref
        .and_then(|s| s.as_wasmtime())
        .expect("non-null structref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = structref.to_rooted(&mut scope);
    match rooted.field(&mut scope, index) {
        Ok(val) => {
            let c_val = crate::wasmtime_val_t::from_val(&mut scope, val);
            crate::initialize(out, c_val);
            None
        }
        Err(e) => Some(Box::new(e.into())),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_structref_set_field(
    mut cx: WasmtimeStoreContextMut<'_>,
    structref: Option<&wasmtime_structref_t>,
    index: usize,
    val: &crate::wasmtime_val_t,
) -> Option<Box<crate::wasmtime_error_t>> {
    let structref = structref
        .and_then(|s| s.as_wasmtime())
        .expect("non-null structref required");
    let mut scope = RootScope::new(&mut cx);
    let rooted = structref.to_rooted(&mut scope);
    let rust_val = val.to_val(&mut scope);
    match rooted.set_field(&mut scope, index, rust_val) {
        Ok(()) => None,
        Err(e) => Some(Box::new(e.into())),
    }
}
