use crate::{CExternType, wasm_externtype_t, wasm_valtype_t, wasm_valtype_vec_t};
use std::cell::OnceCell;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, FuncType, TagType, ValType};

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_tagtype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_tagtype_t);

/// Internal representation of a tag type, mirroring `CFuncType`'s lazy pattern
/// so that `wasm_tagtype_new` can be called without an `Engine`.
#[derive(Clone)]
pub(crate) struct CTagType {
    /// The underlying func type (exception payload). Wrapped in `Arc<Mutex<>>`
    /// for the lazy-construction path; once forced it holds the real `FuncType`.
    ty: Arc<Mutex<LazyTagType>>,
    params_cache: OnceCell<wasm_valtype_vec_t>,
}

#[derive(Clone)]
enum LazyTagType {
    Lazy { params: Vec<ValType> },
    Resolved(TagType),
}

impl LazyTagType {
    fn force(&mut self, engine: &Engine) -> TagType {
        match self {
            LazyTagType::Resolved(t) => t.clone(),
            LazyTagType::Lazy { params } => {
                let params = std::mem::take(params);
                let func_ty = FuncType::new(engine, params, []);
                let tag_ty = TagType::new(func_ty);
                *self = LazyTagType::Resolved(tag_ty.clone());
                tag_ty
            }
        }
    }

    fn params(&self) -> impl ExactSizeIterator<Item = ValType> + '_ {
        match self {
            LazyTagType::Lazy { params } => LazyTagTypeIter::Lazy(params.iter()),
            LazyTagType::Resolved(t) => LazyTagTypeIter::Resolved(t.ty().params()),
        }
    }
}

enum LazyTagTypeIter<'a, T> {
    Lazy(std::slice::Iter<'a, ValType>),
    Resolved(T),
}

impl<T: Iterator<Item = ValType>> Iterator for LazyTagTypeIter<'_, T> {
    type Item = ValType;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            LazyTagTypeIter::Lazy(i) => i.next().cloned(),
            LazyTagTypeIter::Resolved(i) => i.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            LazyTagTypeIter::Lazy(i) => i.size_hint(),
            LazyTagTypeIter::Resolved(i) => i.size_hint(),
        }
    }
}

impl<T: ExactSizeIterator<Item = ValType>> ExactSizeIterator for LazyTagTypeIter<'_, T> {}

impl CTagType {
    pub(crate) fn new(ty: TagType) -> CTagType {
        CTagType {
            ty: Arc::new(Mutex::new(LazyTagType::Resolved(ty))),
            params_cache: OnceCell::new(),
        }
    }

    pub(crate) fn lazy(params: Vec<ValType>) -> CTagType {
        CTagType {
            ty: Arc::new(Mutex::new(LazyTagType::Lazy { params })),
            params_cache: OnceCell::new(),
        }
    }

    pub(crate) fn ty(&self, engine: &Engine) -> TagType {
        self.ty.lock().unwrap().force(engine)
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

    pub(crate) fn ty(&self) -> &CTagType {
        match &self.ext.which {
            CExternType::Tag(t) => t,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_new(params: &mut wasm_valtype_vec_t) -> Box<wasm_tagtype_t> {
    let params = params
        .take()
        .into_iter()
        .map(|vt| vt.unwrap().ty.clone())
        .collect();
    Box::new(wasm_tagtype_t {
        ext: wasm_externtype_t::from_cextern_type(CExternType::Tag(CTagType::lazy(params))),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_params(tt: &wasm_tagtype_t) -> &wasm_valtype_vec_t {
    let tt = tt.ty();
    tt.params_cache.get_or_init(|| {
        let ty = tt.ty.lock().unwrap();
        ty.params()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_as_externtype(ty: &wasm_tagtype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_tagtype_as_externtype_const(ty: &wasm_tagtype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_externtype_as_tagtype(et: &wasm_externtype_t) -> Option<&wasm_tagtype_t> {
    wasm_externtype_as_tagtype_const(et)
}

#[unsafe(no_mangle)]
pub extern "C" fn wasm_externtype_as_tagtype_const(
    et: &wasm_externtype_t,
) -> Option<&wasm_tagtype_t> {
    wasm_tagtype_t::try_from(et)
}
