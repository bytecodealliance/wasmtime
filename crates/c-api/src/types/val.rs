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
pub const WASM_EXTERNREF: wasm_valkind_t = 128;
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
        WASM_EXTERNREF => ValType::ExternRef,
        WASM_FUNCREF => ValType::FuncRef,
        WASMTIME_V128 => ValType::V128,
        _ => panic!("unexpected kind: {}", kind),
    }
}

pub(crate) fn from_valtype(ty: &ValType) -> wasm_valkind_t {
    match ty {
        ValType::I32 => WASM_I32,
        ValType::I64 => WASM_I64,
        ValType::F32 => WASM_F32,
        ValType::F64 => WASM_F64,
        ValType::ExternRef => WASM_EXTERNREF,
        ValType::FuncRef => WASM_FUNCREF,
        ValType::V128 => WASMTIME_V128,
    }
}

pub type wasmtime_valkind_t = u8;
pub const WASMTIME_I32: wasmtime_valkind_t = 0;
pub const WASMTIME_I64: wasmtime_valkind_t = 1;
pub const WASMTIME_F32: wasmtime_valkind_t = 2;
pub const WASMTIME_F64: wasmtime_valkind_t = 3;
pub const WASMTIME_V128: wasmtime_valkind_t = 4;
pub const WASMTIME_FUNCREF: wasmtime_valkind_t = 5;
pub const WASMTIME_EXTERNREF: wasmtime_valkind_t = 6;
