//! wit-bindgen-generated host binding types.

use wasmtime::{Result, ValType};

wasmtime::component::bindgen!({
    path: "wit",
    world: "bytecodealliance:wasmtime/debug-main",
    imports: {
        // Everything is async, even the seemingly simple things
        // like unwrapping a Wasm value, because we need to access
        // the Store in many places and that is an async access
        // via channels within the debuggee.
        default: async | trappable
    },
    exports: {
        default: async,
    },
    with: {
        "bytecodealliance:wasmtime/debuggee.debuggee": super::api::Debuggee,
        "bytecodealliance:wasmtime/debuggee.event-future": super::api::EventFuture,
        "bytecodealliance:wasmtime/debuggee.frame": super::api::Frame,
        "bytecodealliance:wasmtime/debuggee.instance": wasmtime::Instance,
        "bytecodealliance:wasmtime/debuggee.module": wasmtime::Module,
        "bytecodealliance:wasmtime/debuggee.table": wasmtime::Table,
        "bytecodealliance:wasmtime/debuggee.global": wasmtime::Global,
        "bytecodealliance:wasmtime/debuggee.memory": wasmtime::Memory,
        "bytecodealliance:wasmtime/debuggee.wasm-tag": wasmtime::Tag,
        "bytecodealliance:wasmtime/debuggee.wasm-func": wasmtime::Func,
        "bytecodealliance:wasmtime/debuggee.wasm-exception": super::api::WasmException,
        "bytecodealliance:wasmtime/debuggee.wasm-value": super::api::WasmValue,

        "wasi": wasmtime_wasi::p2::bindings,
    },
    trappable_error_type: {
        "bytecodealliance:wasmtime/debuggee.error" => wasmtime::Error,
    },
    require_store_data_send: true,
});

use bytecodealliance::wasmtime::debuggee as wit;

pub(crate) fn val_type_to_wasm_type(vt: &ValType) -> Result<wit::WasmType> {
    match vt {
        ValType::I32 => Ok(wit::WasmType::WasmI32),
        ValType::I64 => Ok(wit::WasmType::WasmI64),
        ValType::F32 => Ok(wit::WasmType::WasmF32),
        ValType::F64 => Ok(wit::WasmType::WasmF64),
        ValType::V128 => Ok(wit::WasmType::WasmV128),
        ValType::Ref(rt) if rt.heap_type().is_exn() => Ok(wit::WasmType::WasmExnref),
        ValType::Ref(rt) if rt.heap_type().is_func() => Ok(wit::WasmType::WasmFuncref),
        ValType::Ref(_) => Err(wit::Error::UnsupportedType.into()),
    }
}

pub(crate) fn wasm_type_to_val_type(wt: wit::WasmType) -> ValType {
    match wt {
        wit::WasmType::WasmI32 => ValType::I32,
        wit::WasmType::WasmI64 => ValType::I64,
        wit::WasmType::WasmF32 => ValType::F32,
        wit::WasmType::WasmF64 => ValType::F64,
        wit::WasmType::WasmV128 => ValType::V128,
        wit::WasmType::WasmFuncref => ValType::FUNCREF,
        wit::WasmType::WasmExnref => ValType::EXNREF,
    }
}
