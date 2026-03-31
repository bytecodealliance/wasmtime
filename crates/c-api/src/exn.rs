use crate::{
    WasmtimeStoreContextMut, handle_result, wasm_trap_t, wasmtime_error_t, wasmtime_val_t,
};
use std::mem::MaybeUninit;
use wasmtime::{AsContextMut, ExnRef, ExnRefPre, ExnType, RootScope, Tag};

/// An opaque type representing a WebAssembly exception object.
pub struct wasmtime_exn_t {
    exn: wasmtime::OwnedRooted<ExnRef>,
}

wasmtime_c_api_macros::declare_own!(wasmtime_exn_t);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exn_new(
    mut store: WasmtimeStoreContextMut<'_>,
    tag: &Tag,
    fields: *const wasmtime_val_t,
    nfields: usize,
    exn_ret: &mut *mut wasmtime_exn_t,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);

    let result = (|| {
        let tag_ty = tag.ty(&scope);
        let exn_type = ExnType::from_tag_type(&tag_ty)?;
        let allocator = ExnRefPre::new(&mut scope, exn_type);
        let raw_fields = crate::slice_from_raw_parts(fields, nfields);
        let field_vals: Vec<wasmtime::Val> =
            raw_fields.iter().map(|f| f.to_val(&mut scope)).collect();
        ExnRef::new(&mut scope, &allocator, tag, &field_vals)
    })();

    handle_result(result, |rooted| {
        let owned = rooted.to_owned_rooted(&mut scope).unwrap();
        *exn_ret = Box::into_raw(Box::new(wasmtime_exn_t { exn: owned }));
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_tag(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exn_t,
    tag_ret: &mut Tag,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.exn.to_rooted(&mut scope);
    handle_result(rooted.tag(&mut scope), |tag| {
        *tag_ret = tag;
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_field_count(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exn_t,
) -> usize {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.exn.to_rooted(&mut scope);
    let ty = rooted.ty(&scope).unwrap();
    ty.fields().len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exn_field(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exn_t,
    index: usize,
    val_ret: &mut MaybeUninit<wasmtime_val_t>,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.exn.to_rooted(&mut scope);
    handle_result(rooted.field(&mut scope, index), |val| {
        crate::initialize(val_ret, wasmtime_val_t::from_val(&mut scope, val));
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_context_set_exception(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: Box<wasmtime_exn_t>,
) -> Box<wasm_trap_t> {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.exn.to_rooted(&mut scope);
    let thrown = scope.as_context_mut().throw::<()>(rooted).unwrap_err();
    Box::new(wasm_trap_t::new(wasmtime::Error::new(thrown)))
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_context_take_exception(
    mut store: WasmtimeStoreContextMut<'_>,
    exn_ret: &mut *mut wasmtime_exn_t,
) -> bool {
    match store.take_pending_exception() {
        Some(rooted) => {
            let owned = rooted.to_owned_rooted(&mut store).unwrap();
            *exn_ret = Box::into_raw(Box::new(wasmtime_exn_t { exn: owned }));
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_context_has_exception(store: WasmtimeStoreContextMut<'_>) -> bool {
    store.has_pending_exception()
}
