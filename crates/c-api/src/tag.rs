use crate::{
    WasmtimeStoreContext, WasmtimeStoreContextMut, handle_result, wasm_tagtype_t, wasmtime_error_t,
};
use std::mem::MaybeUninit;
use wasmtime::Tag;

const _: () = {
    assert!(std::mem::size_of::<Tag>() == 24);
    assert!(std::mem::align_of::<Tag>() == 8);
};

/// Creates a new host-defined tag with the given type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_tag_new(
    mut store: WasmtimeStoreContextMut<'_>,
    tt: &wasm_tagtype_t,
    ret: &mut MaybeUninit<Tag>,
) -> Option<Box<wasmtime_error_t>> {
    let tag_type = tt.to_tag_type(store.engine());
    handle_result(Tag::new(&mut store, &tag_type), |tag| {
        ret.write(tag);
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
