//! Type translator functions from `wasmparser` to `wasm_encoder`.

pub(crate) fn table_type(table_ty: wasmparser::TableType) -> wasm_encoder::TableType {
    wasm_encoder::TableType {
        element_type: val_type(table_ty.element_type),
        limits: limits(table_ty.limits),
    }
}

pub(crate) fn val_type(ty: wasmparser::Type) -> wasm_encoder::ValType {
    use wasm_encoder::ValType;
    use wasmparser::Type::*;
    match ty {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        FuncRef => ValType::FuncRef,
        V128 | ExternRef | ExnRef => panic!("not supported"),
        Func | EmptyBlockType => unreachable!(),
    }
}

pub(crate) fn global_type(ty: wasmparser::GlobalType) -> wasm_encoder::GlobalType {
    wasm_encoder::GlobalType {
        val_type: val_type(ty.content_type),
        mutable: ty.mutable,
    }
}

pub(crate) fn memory_type(ty: wasmparser::MemoryType) -> wasm_encoder::MemoryType {
    match ty {
        wasmparser::MemoryType::M32 {
            shared: false,
            limits: lims,
        } => wasm_encoder::MemoryType {
            limits: limits(lims),
        },
        _ => unreachable!("handled in validation"),
    }
}

pub(crate) fn limits(limits: wasmparser::ResizableLimits) -> wasm_encoder::Limits {
    wasm_encoder::Limits {
        min: limits.initial,
        max: limits.maximum,
    }
}

pub(crate) fn entity_type(ty: wasmparser::ImportSectionEntryType) -> wasm_encoder::EntityType {
    match ty {
        wasmparser::ImportSectionEntryType::Function(f) => wasm_encoder::EntityType::Function(f),
        wasmparser::ImportSectionEntryType::Table(tty) => {
            wasm_encoder::EntityType::Table(table_type(tty))
        }
        wasmparser::ImportSectionEntryType::Memory(mty) => {
            wasm_encoder::EntityType::Memory(memory_type(mty))
        }
        wasmparser::ImportSectionEntryType::Global(gty) => {
            wasm_encoder::EntityType::Global(global_type(gty))
        }
        wasmparser::ImportSectionEntryType::Instance(ity) => {
            wasm_encoder::EntityType::Instance(ity)
        }
        wasmparser::ImportSectionEntryType::Module(_) => {
            unreachable!(
                "we disallow importing/exporting modules so we shouldn't \
                 have module types"
            )
        }
        wasmparser::ImportSectionEntryType::Event(_) => unreachable!(),
    }
}

pub(crate) fn item_kind(kind: wasmparser::ExternalKind) -> wasm_encoder::ItemKind {
    match kind {
        wasmparser::ExternalKind::Function => wasm_encoder::ItemKind::Function,
        wasmparser::ExternalKind::Table => wasm_encoder::ItemKind::Table,
        wasmparser::ExternalKind::Memory => wasm_encoder::ItemKind::Memory,
        wasmparser::ExternalKind::Global => wasm_encoder::ItemKind::Global,
        wasmparser::ExternalKind::Module => wasm_encoder::ItemKind::Module,
        wasmparser::ExternalKind::Instance => wasm_encoder::ItemKind::Instance,
        wasmparser::ExternalKind::Type | wasmparser::ExternalKind::Event => unreachable!(),
    }
}

pub(crate) fn export(kind: wasmparser::ExternalKind, index: u32) -> wasm_encoder::Export {
    match kind {
        wasmparser::ExternalKind::Function => wasm_encoder::Export::Function(index),
        wasmparser::ExternalKind::Global => wasm_encoder::Export::Global(index),
        wasmparser::ExternalKind::Table => wasm_encoder::Export::Table(index),
        wasmparser::ExternalKind::Memory => wasm_encoder::Export::Memory(index),
        wasmparser::ExternalKind::Instance => wasm_encoder::Export::Instance(index),
        wasmparser::ExternalKind::Event
        | wasmparser::ExternalKind::Type
        | wasmparser::ExternalKind::Module => unreachable!(),
    }
}

pub(crate) fn instance_arg<'a>(
    arg: &wasmparser::InstanceArg<'a>,
) -> (&'a str, wasm_encoder::Export) {
    (arg.name, export(arg.kind, arg.index))
}
