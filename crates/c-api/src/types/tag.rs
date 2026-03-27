use crate::{CExternType, CFuncType, wasm_externtype_t, wasm_functype_t};
use std::cell::OnceCell;
use wasmtime::{Engine, TagType};

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_tagtype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_tagtype_t);

#[derive(Clone)]
pub(crate) struct CTagType {
    ty: CFuncType,
    functype_cache: OnceCell<Box<wasm_functype_t>>,
}

impl CTagType {
    pub(crate) fn new(ty: TagType) -> CTagType {
        CTagType {
            ty: CFuncType::new(ty.ty().clone()),
            functype_cache: OnceCell::new(),
        }
    }

    fn from_cfunc(ty: CFuncType) -> CTagType {
        CTagType {
            ty,
            functype_cache: OnceCell::new(),
        }
    }
}

impl wasm_tagtype_t {
    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_tagtype_t> {
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

    /// Converts this C tag type into a Wasmtime `TagType`.
    pub(crate) fn to_tag_type(&self, engine: &Engine) -> TagType {
        let func_type = self.cty().ty.ty(engine);
        TagType::new(func_type)
    }

    /// Creates a `wasm_tagtype_t` from a Wasmtime `TagType`.
    pub(crate) fn from_tag_type(ty: TagType) -> wasm_tagtype_t {
        wasm_tagtype_t {
            ext: wasm_externtype_t::from_cextern_type(CExternType::Tag(CTagType::new(ty))),
        }
    }
}

/// Creates a new tag type from a function type describing the exception payload.
/// Takes ownership of `functype`.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_new(functype: Box<wasm_functype_t>) -> Box<wasm_tagtype_t> {
    let cfunc = functype.ty().clone();
    Box::new(wasm_tagtype_t {
        ext: wasm_externtype_t::from_cextern_type(CExternType::Tag(CTagType::from_cfunc(cfunc))),
    })
}

/// Returns a borrowed reference to the function type describing this tag's exception payload.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_functype(tt: &wasm_tagtype_t) -> &wasm_functype_t {
    let cty = tt.cty();
    cty.functype_cache
        .get_or_init(|| Box::new(wasm_functype_t::from_cfunc(cty.ty.clone())))
}

/// Converts a `wasm_tagtype_t` to a `wasm_externtype_t` (borrowed).
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_as_externtype(ty: &wasm_tagtype_t) -> &wasm_externtype_t {
    &ty.ext
}

/// Converts a const `wasm_tagtype_t` to a const `wasm_externtype_t` (borrowed).
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_as_externtype_const(ty: &wasm_tagtype_t) -> &wasm_externtype_t {
    &ty.ext
}

/// Converts a `wasm_externtype_t` to a `wasm_tagtype_t`, or NULL if not a tag.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_externtype_as_tagtype(et: &wasm_externtype_t) -> Option<&wasm_tagtype_t> {
    wasm_externtype_as_tagtype_const(et)
}

/// Converts a const `wasm_externtype_t` to a const `wasm_tagtype_t`, or NULL if not a tag.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_externtype_as_tagtype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_tagtype_t> {
    wasm_tagtype_t::try_from(et)
}
