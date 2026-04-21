use wasmtime::{FieldType, Mutability, StorageType, StructType, ValType};

pub type wasmtime_storage_kind_t = u8;
pub const WASMTIME_STORAGE_KIND_I8: wasmtime_storage_kind_t = 9;
pub const WASMTIME_STORAGE_KIND_I16: wasmtime_storage_kind_t = 10;

#[repr(C)]
pub struct wasmtime_field_type_t {
    pub kind: wasmtime_storage_kind_t,
    pub mutable_: bool,
}

impl wasmtime_field_type_t {
    pub(crate) fn to_wasmtime(&self) -> FieldType {
        let mutability = if self.mutable_ {
            Mutability::Var
        } else {
            Mutability::Const
        };
        let storage = match self.kind {
            WASMTIME_STORAGE_KIND_I8 => StorageType::I8,
            WASMTIME_STORAGE_KIND_I16 => StorageType::I16,
            crate::WASMTIME_I32 => StorageType::ValType(ValType::I32),
            crate::WASMTIME_I64 => StorageType::ValType(ValType::I64),
            crate::WASMTIME_F32 => StorageType::ValType(ValType::F32),
            crate::WASMTIME_F64 => StorageType::ValType(ValType::F64),
            crate::WASMTIME_V128 => StorageType::ValType(ValType::V128),
            crate::WASMTIME_FUNCREF => StorageType::ValType(ValType::FUNCREF),
            crate::WASMTIME_EXTERNREF => StorageType::ValType(ValType::EXTERNREF),
            crate::WASMTIME_ANYREF => StorageType::ValType(ValType::ANYREF),
            crate::WASMTIME_EXNREF => StorageType::ValType(ValType::EXNREF),
            other => panic!("unknown wasmtime_storage_kind_t: {other}"),
        };
        FieldType::new(mutability, storage)
    }
}

pub struct wasmtime_struct_type_t {
    pub(crate) ty: StructType,
}
wasmtime_c_api_macros::declare_own!(wasmtime_struct_type_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_type_new(
    engine: &crate::wasm_engine_t,
    fields: *const wasmtime_field_type_t,
    nfields: usize,
) -> Box<wasmtime_struct_type_t> {
    let fields = unsafe { crate::slice_from_raw_parts(fields, nfields) };
    let field_types: Vec<FieldType> = fields.iter().map(|f| f.to_wasmtime()).collect();
    let ty = StructType::new(&engine.engine, field_types).expect("failed to create struct type");
    Box::new(wasmtime_struct_type_t { ty })
}
