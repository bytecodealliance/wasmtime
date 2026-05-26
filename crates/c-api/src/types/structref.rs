use crate::wasm_valtype_t;
use std::mem::{ManuallyDrop, MaybeUninit};
use wasmtime::{FieldType, Mutability, StorageType, StructType};

#[repr(C, u8)]
#[derive(Clone)]
pub enum wasmtime_storage_type_t {
    I8,
    I16,
    Val(Box<wasm_valtype_t>),
}

impl From<StorageType> for wasmtime_storage_type_t {
    fn from(ty: StorageType) -> Self {
        match ty {
            StorageType::I8 => Self::I8,
            StorageType::I16 => Self::I16,
            StorageType::ValType(ty) => Self::Val(Box::new(ty)),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_storage_type_clone(
    storage: &wasmtime_storage_type_t,
    out: &mut MaybeUninit<wasmtime_storage_type_t>,
) {
    out.write(storage.clone());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_storage_type_delete(
    out: Option<&mut ManuallyDrop<wasmtime_storage_type_t>>,
) {
    if let Some(out) = out {
        unsafe {
            ManuallyDrop::drop(out);
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct wasmtime_field_type_t {
    pub mutable_: bool,
    pub storage: wasmtime_storage_type_t,
}

impl From<FieldType> for wasmtime_field_type_t {
    fn from(field: FieldType) -> Self {
        let mutable_ = field.mutability() == Mutability::Var;
        let storage = field.element_type().clone().into();
        Self { mutable_, storage }
    }
}

impl wasmtime_field_type_t {
    pub(crate) fn to_wasmtime(&self) -> FieldType {
        let mutability = if self.mutable_ {
            Mutability::Var
        } else {
            Mutability::Const
        };
        let storage = match &self.storage {
            wasmtime_storage_type_t::I8 => StorageType::I8,
            wasmtime_storage_type_t::I16 => StorageType::I16,
            wasmtime_storage_type_t::Val(ty) => StorageType::ValType((**ty).clone()),
        };
        FieldType::new(mutability, storage)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_field_type_clone(
    field: &wasmtime_field_type_t,
    out: &mut MaybeUninit<wasmtime_field_type_t>,
) {
    out.write(field.clone());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wasmtime_field_type_delete(
    out: Option<&mut ManuallyDrop<wasmtime_field_type_t>>,
) {
    if let Some(out) = out {
        unsafe {
            ManuallyDrop::drop(out);
        }
    }
}

#[derive(Clone)]
pub struct wasmtime_struct_type_t {
    pub(crate) ty: StructType,
}
wasmtime_c_api_macros::declare_ty!(wasmtime_struct_type_t);

impl From<StructType> for wasmtime_struct_type_t {
    fn from(ty: StructType) -> Self {
        Self { ty }
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_type_num_fields(ty: &wasmtime_struct_type_t) -> usize {
    ty.ty.fields().len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_struct_type_field(
    ty: &wasmtime_struct_type_t,
    index: usize,
    out: &mut MaybeUninit<wasmtime_field_type_t>,
) -> bool {
    match ty.ty.field(index) {
        Some(field) => {
            out.write(field.into());
            true
        }
        None => false,
    }
}
