use crate::{
    WasmtimeStoreContextMut, handle_result, wasm_tagtype_t, wasm_trap_t, wasmtime_error_t,
    wasmtime_val_t,
};
use std::mem::MaybeUninit;
use wasmtime::{AsContext, ExnRef, ExnRefPre, ExnType, RootScope, Tag};

/// An opaque type representing a WebAssembly exception object.
pub struct wasmtime_exn_t {
    exn: wasmtime::OwnedRooted<ExnRef>,
}

wasmtime_c_api_macros::declare_own!(wasmtime_exn_t);

/// Creates a new exception object with the given tag and field
/// values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_exn_new(
    mut store: WasmtimeStoreContextMut<'_>,
    tag: &Tag,
    tag_type: &wasm_tagtype_t,
    fields: *const wasmtime_val_t,
    nfields: usize,
    exn_ret: &mut *mut wasmtime_exn_t,
) -> Option<Box<wasmtime_error_t>> {
    let mut scope = RootScope::new(&mut store);

    // Build the ExnType from the tag type
    let tag_ty = tag_type.to_tag_type(scope.as_context().engine());
    let exn_type = match ExnType::from_tag_type(&tag_ty) {
        Ok(t) => t,
        Err(e) => return Some(Box::new(wasmtime_error_t::from(e))),
    };

    // Create the allocator
    let allocator = ExnRefPre::new(&mut scope, exn_type);

    // Convert field values
    let raw_fields = crate::slice_from_raw_parts(fields, nfields);
    let field_vals: Vec<wasmtime::Val> = raw_fields.iter().map(|f| f.to_val(&mut scope)).collect();

    // Allocate the exception
    let result = ExnRef::new(&mut scope, &allocator, tag, &field_vals);

    handle_result(result, |rooted| {
        let owned = rooted.to_owned_rooted(&mut scope).unwrap();
        *exn_ret = Box::into_raw(Box::new(wasmtime_exn_t { exn: owned }));
    })
}

/// Returns the tag associated with this exception object.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_tag(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exn_t,
    tag_ret: &mut Tag,
) -> Option<Box<wasmtime_error_t>> {
    let rooted = exn.exn.to_rooted(&mut store);
    handle_result(rooted.tag(&mut store), |tag| {
        *tag_ret = tag;
    })
}

/// Returns the number of fields in this exception object.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_field_count(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: &wasmtime_exn_t,
) -> usize {
    let rooted = exn.exn.to_rooted(&mut store);
    let ty = rooted.ty(&store).unwrap();
    ty.fields().len()
}

/// Reads a field value from this exception object by index.
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

/// Sets a pending exception on the store, to be propagated as a thrown
/// Wasm exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_context_set_exception(
    mut store: WasmtimeStoreContextMut<'_>,
    exn: Box<wasmtime_exn_t>,
) -> Box<wasm_trap_t> {
    let rooted = exn.exn.to_rooted(&mut store);
    let thrown = store.throw::<()>(rooted).unwrap_err();
    Box::new(wasm_trap_t::new(wasmtime::Error::new(thrown)))
}

/// Takes the pending exception from the store, if any.
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

/// Tests whether there is a pending exception on the store.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_context_has_exception(store: WasmtimeStoreContextMut<'_>) -> bool {
    store.has_pending_exception()
}
