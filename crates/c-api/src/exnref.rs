use crate::{
    WasmtimeStoreContextMut, handle_result, wasm_trap_t, wasmtime_error_t, wasmtime_val_t,
};
use std::mem::MaybeUninit;
use wasmtime::{AsContextMut, ExnRef, ExnRefPre, ExnType, RootScope, Tag};

crate::anyref::ref_wrapper!({
    wasmtime: ExnRef,
    capi: wasmtime_exnref_t,
    clone: wasmtime_exnref_clone,
    unroot: wasmtime_exnref_unroot,
    to_raw: wasmtime_exnref_to_raw,
    from_raw: wasmtime_exnref_from_raw,
});

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exnref_new(
    mut store: WasmtimeStoreContextMut<'_>,
    tag: &Tag,
    fields: *const wasmtime_val_t,
    nfields: usize,
    exn_ret: &mut MaybeUninit<wasmtime_exnref_t>,
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
        exn_ret.write(owned.into());
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exnref_tag(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exnref_t,
    tag_ret: &mut Tag,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);
    let rooted = unsafe { exn.as_wasmtime()?.to_rooted(&mut scope) };
    handle_result(rooted.tag(&mut scope), |tag| {
        *tag_ret = tag;
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exnref_field_count(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exnref_t,
) -> usize {
    let mut scope = RootScope::new(&mut store);
    let rooted = match unsafe { exn.as_wasmtime() } {
        Some(e) => e.to_rooted(&mut scope),
        None => return 0,
    };
    let ty = rooted.ty(&scope).unwrap();
    ty.fields().len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exnref_field(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exnref_t,
    index: usize,
    val_ret: &mut MaybeUninit<wasmtime_val_t>,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.as_wasmtime()?.to_rooted(&mut scope);
    handle_result(rooted.field(&mut scope, index), |val| {
        crate::initialize(val_ret, wasmtime_val_t::from_val(&mut scope, val));
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_context_set_exception(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exnref_t,
) -> Option<Box<wasm_trap_t>> {
    let mut scope = RootScope::new(&mut store);
    let rooted = exn.as_wasmtime()?.to_rooted(&mut scope);
    let Err(thrown) = scope
        .as_context_mut()
        .throw::<std::convert::Infallible>(rooted);
    Some(Box::new(wasm_trap_t::new(wasmtime::Error::new(thrown))))
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_context_take_exception(
    mut store: WasmtimeStoreContextMut<'_>,
    out: &mut MaybeUninit<wasmtime_exnref_t>,
) -> bool {
    match store.take_pending_exception() {
        Some(rooted) => {
            let owned = rooted.to_owned_rooted(&mut store).unwrap();
            out.write(owned.into());
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_context_has_exception(store: WasmtimeStoreContextMut<'_>) -> bool {
    store.has_pending_exception()
}
