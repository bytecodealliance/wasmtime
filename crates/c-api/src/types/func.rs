use crate::{wasm_valtype_t, wasm_valtype_vec_t};
use once_cell::unsync::OnceCell;
use wasmtime::FuncType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_functype_t {
    pub(crate) functype: FuncType,
    params_cache: OnceCell<wasm_valtype_vec_t>,
    returns_cache: OnceCell<wasm_valtype_vec_t>,
}

impl wasm_functype_t {
    pub(crate) fn new(functype: FuncType) -> wasm_functype_t {
        wasm_functype_t {
            functype,
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
    ft.params_cache.get_or_init(|| {
        ft.functype
            .params()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_results(ft: &wasm_functype_t) -> &wasm_valtype_vec_t {
    ft.returns_cache.get_or_init(|| {
        ft.functype
            .results()
            .iter()
            .map(|p| Some(Box::new(wasm_valtype_t { ty: p.clone() })))
            .collect::<Vec<_>>()
            .into()
    })
}

#[no_mangle]
pub extern "C" fn wasm_functype_delete(_ft: Box<wasm_functype_t>) {}
