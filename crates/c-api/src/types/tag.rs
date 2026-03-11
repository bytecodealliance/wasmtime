use crate::{CExternType, wasm_engine_t, wasm_externtype_t, wasm_functype_t};
use wasmtime::TagType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasmtime_tagtype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasmtime_tagtype_t);

#[derive(Clone)]
pub(crate) struct CTagType {
    ty: TagType,
}

impl CTagType {
    pub(crate) fn new(ty: TagType) -> CTagType {
        CTagType { ty }
    }
}

impl wasmtime_tagtype_t {
    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasmtime_tagtype_t> {
        match &e.which {
            CExternType::Tag(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    fn cty(&self) -> &CTagType {
        match &self.ext.which {
            CExternType::Tag(t) => t,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

/// Creates a new tag type from a function type describing the exception payload.
///
/// The tag type takes ownership of the exception payload shape from `functype`
/// (which may have both params and results for forward compatibility with the
/// stack-switching proposal). Returns an owned `wasmtime_tagtype_t` that must
/// be freed with `wasmtime_tagtype_delete`.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tagtype_new(
    engine: &wasm_engine_t,
    functype: &wasm_functype_t,
) -> Box<wasmtime_tagtype_t> {
    let func_ty = functype.ty().ty(&engine.engine);
    let tag_ty = TagType::new(func_ty);
    Box::new(wasmtime_tagtype_t {
        ext: wasm_externtype_t::from_cextern_type(CExternType::Tag(CTagType::new(tag_ty))),
    })
}

/// Returns the function type describing the exception payload of this tag type.
///
/// The caller owns the returned `wasm_functype_t` and must free it with
/// `wasm_functype_delete`.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tagtype_functype(tt: &wasmtime_tagtype_t) -> Box<wasm_functype_t> {
    Box::new(wasm_functype_t::new(tt.cty().ty.ty().clone()))
}

/// Converts a `wasmtime_tagtype_t` to a `wasm_externtype_t` (borrowed).
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tagtype_as_externtype(ty: &wasmtime_tagtype_t) -> &wasm_externtype_t {
    &ty.ext
}

/// Converts a const `wasmtime_tagtype_t` to a const `wasm_externtype_t` (borrowed).
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_tagtype_as_externtype_const(
    ty: &wasmtime_tagtype_t,
) -> &wasm_externtype_t {
    &ty.ext
}

/// Converts a `wasm_externtype_t` to a `wasmtime_tagtype_t`, or NULL if not a tag.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_externtype_as_tagtype(
    et: &wasm_externtype_t,
) -> Option<&wasmtime_tagtype_t> {
    wasmtime_externtype_as_tagtype_const(et)
}

/// Converts a const `wasm_externtype_t` to a const `wasmtime_tagtype_t`, or NULL if not a tag.
#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_externtype_as_tagtype_const(
    et: &wasm_externtype_t,
) -> Option<&wasmtime_tagtype_t> {
    wasmtime_tagtype_t::try_from(et)
}
