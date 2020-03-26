use crate::{wasm_externtype_t, wasm_valtype_t, wasm_valtype_vec_t, CExternType};
use once_cell::unsync::OnceCell;
use wasmtime::FuncType;

#[repr(transparent)]
#[derive(Clone)]
pub struct wasm_functype_t {
    ext: wasm_externtype_t,
}

wasmtime_c_api_macros::declare_ty!(wasm_functype_t);

#[derive(Clone)]
pub(crate) struct CFuncType {
    pub(crate) ty: FuncType,
    params_cache: OnceCell<wasm_valtype_vec_t>,
    returns_cache: OnceCell<wasm_valtype_vec_t>,
}

impl wasm_functype_t {
    pub(crate) fn new(ty: FuncType) -> wasm_functype_t {
        wasm_functype_t {
            ext: wasm_externtype_t::new(ty.into()),
        }
    }

    pub(crate) fn try_from(e: &wasm_externtype_t) -> Option<&wasm_functype_t> {
        match &e.which {
            CExternType::Func(_) => Some(unsafe { &*(e as *const _ as *const _) }),
            _ => None,
        }
    }

    pub(crate) fn ty(&self) -> &CFuncType {
        match &self.ext.which {
            CExternType::Func(f) => &f,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl CFuncType {
    pub(crate) fn new(ty: FuncType) -> CFuncType {
        CFuncType {
            ty,
            params_cache: OnceCell::new(),
            returns_cache: OnceCell::new(),
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_functype_new(
    params: &mut wasm_valtype_vec_t,
    results: &mut wasm_valtype_vec_t,
) -> Box<wasm_functype_t> {
    let params = params
        .take()
        .into_iter()
        .map(|vt| vt.unwrap().ty.clone())
        .collect::<Vec<_>>();
    let results = results
        .take()
        .into_iter()
        .map(|vt| vt.unwrap().ty.clone())
        .collect::<Vec<_>>();
    let functype = FuncType::new(params.into_boxed_slice(), results.into_boxed_slice());
    Box::new(wasm_functype_t::new(functype))
}

#[no_mangle]
pub extern "C" fn wasm_functype_params(ft: &wasm_functype_t) -> &wasm_valtype_vec_t {
    let ft = ft.ty();
    ft.params_cache.get_or_init(|| {
        ft.ty
            .params()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_results(ft: &wasm_functype_t) -> &wasm_valtype_vec_t {
    let ft = ft.ty();
    ft.returns_cache.get_or_init(|| {
        ft.ty
            .results()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_as_externtype(ty: &wasm_functype_t) -> &wasm_externtype_t {
    &ty.ext
}

#[no_mangle]
pub extern "C" fn wasm_functype_as_externtype_const(ty: &wasm_functype_t) -> &wasm_externtype_t {
    &ty.ext
}
