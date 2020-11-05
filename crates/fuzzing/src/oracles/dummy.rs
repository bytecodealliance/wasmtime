//! Dummy implementations of things that a Wasm module can import.

use wasmtime::{
    Extern, ExternType, Func, FuncType, Global, GlobalType, ImportType, Memory, MemoryType, Store,
    Table, TableType, Trap, Val, ValType,
};

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_imports<'module>(
    store: &Store,
    import_tys: impl Iterator<Item = ImportType<'module>>,
) -> Result<Vec<Extern>, Trap> {
    import_tys
        .map(|imp| {
            Ok(match imp.ty() {
                ExternType::Func(func_ty) => Extern::Func(dummy_func(&store, func_ty)),
                ExternType::Global(global_ty) => Extern::Global(dummy_global(&store, global_ty)?),
                ExternType::Table(table_ty) => Extern::Table(dummy_table(&store, table_ty)?),
                ExternType::Memory(mem_ty) => Extern::Memory(dummy_memory(&store, mem_ty)),
            })
        })
        .collect()
}

/// Construct a dummy function for the given function type
pub fn dummy_func(store: &Store, ty: FuncType) -> Func {
    Func::new(store, ty.clone(), move |_, _, results| {
        for (ret_ty, result) in ty.results().zip(results) {
            *result = dummy_value(ret_ty)?;
        }
        Ok(())
    })
}

/// Construct a dummy value for the given value type.
pub fn dummy_value(val_ty: ValType) -> Result<Val, Trap> {
    Ok(match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => {
            return Err(Trap::new(
                "dummy_value: unsupported function return type: v128".to_string(),
            ))
        }
        ValType::ExternRef => {
            return Err(Trap::new(
                "dummy_value: unsupported function return type: externref".to_string(),
            ))
        }
        ValType::FuncRef => {
            return Err(Trap::new(
                "dummy_value: unsupported function return type: funcref".to_string(),
            ))
        }
    })
}

/// Construct a sequence of dummy values for the given types.
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Result<Vec<Val>, Trap> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy global for the given global type.
pub fn dummy_global(store: &Store, ty: GlobalType) -> Result<Global, Trap> {
    let val = dummy_value(ty.content().clone())?;
    Ok(Global::new(store, ty, val).unwrap())
}

/// Construct a dummy table for the given table type.
pub fn dummy_table(store: &Store, ty: TableType) -> Result<Table, Trap> {
    let init_val = dummy_value(ty.element().clone())?;
    Ok(Table::new(store, ty, init_val).unwrap())
}

/// Construct a dummy memory for the given memory type.
pub fn dummy_memory(store: &Store, ty: MemoryType) -> Memory {
    Memory::new(store, ty)
}
