use wasmtime::ValType;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_valtype_t {
    pub(crate) ty: ValType,
}

wasmtime_c_api_macros::declare_ty!(wasm_valtype_t);

pub type wasm_valkind_t = u8;
pub const WASM_I32: wasm_valkind_t = 0;
pub const WASM_I64: wasm_valkind_t = 1;
pub const WASM_F32: wasm_valkind_t = 2;
pub const WASM_F64: wasm_valkind_t = 3;
pub const WASM_ANYREF: wasm_valkind_t = 128;
pub const WASM_FUNCREF: wasm_valkind_t = 129;

#[no_mangle]
pub extern "C" fn wasm_valtype_new(kind: wasm_valkind_t) -> Box<wasm_valtype_t> {
    Box::new(wasm_valtype_t {
        ty: into_valtype(kind),
    })
}

#[no_mangle]
pub extern "C" fn wasm_valtype_kind(vt: &wasm_valtype_t) -> wasm_valkind_t {
    from_valtype(&vt.ty)
}

pub(crate) fn into_valtype(kind: wasm_valkind_t) -> ValType {
    match kind {
        WASM_I32 => ValType::I32,
        WASM_I64 => ValType::I64,
        WASM_F32 => ValType::F32,
        WASM_F64 => ValType::F64,
        WASM_ANYREF => ValType::AnyRef,
        WASM_FUNCREF => ValType::FuncRef,
        _ => panic!("unexpected kind: {}", kind),
    }
}

pub(crate) fn from_valtype(ty: &ValType) -> wasm_valkind_t {
    match ty {
        ValType::I32 => WASM_I32,
        ValType::I64 => WASM_I64,
        ValType::F32 => WASM_F32,
        ValType::F64 => WASM_F64,
        ValType::AnyRef => WASM_ANYREF,
        ValType::FuncRef => WASM_FUNCREF,
        _ => panic!("wasm_valkind_t has no known conversion for {:?}", ty),
    }
}
