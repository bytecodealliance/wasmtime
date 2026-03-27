use crate::{
    WasmtimeStoreContext, WasmtimeStoreContextMut, handle_result, wasm_extern_t, wasm_tagtype_t,
    wasmtime_error_t,
};
use wasmtime::{Extern, Tag};

const _: () = {
    assert!(std::mem::size_of::<Tag>() == 24);
    assert!(std::mem::align_of::<Tag>() == 8);
};

#[derive(Clone)]
#[repr(transparent)]
pub struct wasm_tag_t {
    ext: wasm_extern_t,
}

wasmtime_c_api_macros::declare_ref!(wasm_tag_t);

impl wasm_tag_t {
    pub(crate) fn try_from(e: &wasm_extern_t) -> Option<&wasm_tag_t> {
        match &e.which {
            Extern::Tag(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tag_as_extern(t: &mut wasm_tag_t) -> &mut wasm_extern_t {
    &mut t.ext
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tag_as_extern_const(t: &wasm_tag_t) -> &wasm_extern_t {
    &t.ext
}

/// Creates a new host-defined tag with the given type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_tag_new(
    mut store: WasmtimeStoreContextMut<'_>,
    tt: &wasm_tagtype_t,
    ret: &mut Tag,
) -> Option<Box<wasmtime_error_t>> {
    let tag_type = tt.to_tag_type(store.engine());
    handle_result(Tag::new(&mut store, &tag_type), |tag| {
        *ret = tag;
    })
}

/// Returns the type of this tag.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tag_type(
    store: WasmtimeStoreContext<'_>,
    tag: &Tag,
) -> Box<wasm_tagtype_t> {
    let ty = tag.ty(store);
    Box::new(wasm_tagtype_t::from_tag_type(ty))
}

/// Tests whether two tags are the same (identity equality).
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tag_eq(store: WasmtimeStoreContext<'_>, a: &Tag, b: &Tag) -> bool {
    Tag::eq(a, b, store)
}
