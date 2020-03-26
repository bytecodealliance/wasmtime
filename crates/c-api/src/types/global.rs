use crate::wasm_valtype_t;
use once_cell::unsync::OnceCell;
use wasmtime::GlobalType;

pub type wasm_mutability_t = u8;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_globaltype_t {
    pub(crate) globaltype: GlobalType,
    content_cache: OnceCell<wasm_valtype_t>,
}

impl wasm_globaltype_t {
    pub(crate) fn new(globaltype: GlobalType) -> wasm_globaltype_t {
        wasm_globaltype_t {
            globaltype,
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
    let globaltype = GlobalType::new(ty.ty.clone(), mutability);
    Box::new(wasm_globaltype_t {
        globaltype,
        content_cache: (*ty).into(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_content(gt: &wasm_globaltype_t) -> &wasm_valtype_t {
    gt.content_cache.get_or_init(|| wasm_valtype_t {
        ty: gt.globaltype.content().clone(),
    })
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_mutability(gt: &wasm_globaltype_t) -> wasm_mutability_t {
    use wasmtime::Mutability::*;
    match gt.globaltype.mutability() {
        Const => 0,
        Var => 1,
    }
}

#[no_mangle]
pub extern "C" fn wasm_globaltype_delete(_gt: Box<wasm_globaltype_t>) {}
