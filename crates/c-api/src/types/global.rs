use crate::{wasm_externtype_t, wasm_valtype_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::GlobalType;

pub type wasm_mutability_t = u8;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_globaltype_t {
    ext: wasm_externtype_t,
}

#[derive(Clone)]
pub(crate) struct CGlobalType {
    pub(crate) ty: GlobalType,
    content_cache: OnceCell<wasm_valtype_t>,
}

impl wasm_globaltype_t {
    pub(crate) fn new(ty: GlobalType) -> wasm_globaltype_t {
        wasm_globaltype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_globaltype_t> {
        match &e.which {
            CExternType::Global(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CGlobalType {
        match &self.ext.which {
            CExternType::Global(f) => &f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl CGlobalType {
    pub(crate) fn new(ty: GlobalType) -> CGlobalType {
        CGlobalType {
            ty,
            content_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_new(
    ty: Box<wasm_valtype_t>,
    mutability: wasm_mutability_t,
) -> Box<wasm_globaltype_t> {
    use wasmtime::Mutability::*;
    let mutability = match mutability {
        0 => Const,
        1 => Var,
        _ => panic!("mutability out-of-range"),
    };
    let ty = GlobalType::new(ty.ty.clone(), mutability);
    Box::new(wasm_globaltype_t::new(ty))
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_content(gt: &wasm_globaltype_t) -> &wasm_valtype_t {
    let gt = gt.ty();
    gt.content_cache.get_or_init(|| wasm_valtype_t {
        ty: gt.ty.content().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_mutability(gt: &wasm_globaltype_t) -> wasm_mutability_t {
    use wasmtime::Mutability::*;
    let gt = gt.ty();
    match gt.ty.mutability() {
        Const => 0,
        Var => 1,
    }
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_as_externtype(ty: &wasm_globaltype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_as_externtype_const(
    ty: &wasm_globaltype_t,
) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_delete(_gt: Box<wasm_globaltype_t>) {}
