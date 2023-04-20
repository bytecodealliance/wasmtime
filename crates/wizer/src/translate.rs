//! Type translator functions from `wasmparser` to `wasm_encoder`.

pub(crate) fn val_type(ty: wasmparser::ValType) -> wasm_encoder::ValType {
    use wasm_encoder::ValType;
    use wasmparser::ValType::*;
    match ty {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        V128 => ValType::V128,
        wasmparser::ValType::FUNCREF => ValType::FUNCREF,
        Ref(_) => panic!("not supported"),
    }
}

pub(crate) fn global_type(ty: wasmparser::GlobalType) -> wasm_encoder::GlobalType {
    wasm_encoder::GlobalType {
        val_type: val_type(ty.content_type),
        mutable: ty.mutable,
    }
}

pub(crate) fn memory_type(ty: wasmparser::MemoryType) -> wasm_encoder::MemoryType {
    wasm_encoder::MemoryType {
        minimum: ty.initial.into(),
        maximum: ty.maximum.map(|val| val.into()),
        memory64: ty.memory64,
        shared: ty.shared,
    }
}

pub(crate) fn export(kind: wasmparser::ExternalKind) -> wasm_encoder::ExportKind {
    match kind {
        wasmparser::ExternalKind::Func => wasm_encoder::ExportKind::Func,
        wasmparser::ExternalKind::Global => wasm_encoder::ExportKind::Global,
        wasmparser::ExternalKind::Table => wasm_encoder::ExportKind::Table,
        wasmparser::ExternalKind::Memory => wasm_encoder::ExportKind::Memory,
        wasmparser::ExternalKind::Tag => unreachable!(),
    }
}
