use crate::{CExternType, wasm_externtype_t, wasm_functype_t};
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
    ty: LazyTagType,
    tagtype_cache: OnceCell<TagType>,
    functype_cache: OnceCell<Box<wasm_functype_t>>,
}

#[derive(Clone)]
enum LazyTagType {
    TagType(TagType),
    Lazy(Box<wasm_functype_t>),
}

impl CTagType {
    pub(crate) fn new(ty: TagType) -> CTagType {
        CTagType {
            ty: LazyTagType::TagType(ty),
            tagtype_cache: OnceCell::new(),
            functype_cache: OnceCell::new(),
        }
    }

    fn lazy(functype: Box<wasm_functype_t>) -> CTagType {
        CTagType {
            ty: LazyTagType::Lazy(functype),
            tagtype_cache: OnceCell::new(),
            functype_cache: OnceCell::new(),
        }
    }
}

impl wasm_tagtype_t {
    pub(crate) fn new(ty: TagType) -> wasm_tagtype_t {
        wasm_tagtype_t {
            ext: wasm_externtype_t::from_extern_type(ty.into()),
        }
    }

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

    pub(crate) fn ty(&self, engine: &Engine) -> &TagType {
        let cty = self.cty();
        match &cty.ty {
            LazyTagType::TagType(ty) => ty,
            LazyTagType::Lazy(f) => cty
                .tagtype_cache
                .get_or_init(|| TagType::new(f.ty().ty(engine))),
        }
    }
}

/// Creates a new tag type from a function type describing the exception payload.
/// Takes ownership of `functype`.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_new(functype: Box<wasm_functype_t>) -> Box<wasm_tagtype_t> {
    Box::new(wasm_tagtype_t {
        ext: wasm_externtype_t::from_cextern_type(CExternType::Tag(CTagType::lazy(functype))),
    })
}

/// Returns a borrowed reference to the function type describing this tag's exception payload.
#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_functype(tt: &wasm_tagtype_t) -> &wasm_functype_t {
    let cty = tt.cty();
    match &cty.ty {
        LazyTagType::TagType(ty) => cty
            .functype_cache
            .get_or_init(|| Box::new(wasm_functype_t::new(ty.ty().clone()))),
        LazyTagType::Lazy(f) => f,
    }
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
