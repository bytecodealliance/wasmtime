//! Dummy implementations of things that a Wasm module can import.

use anyhow::Result;
use wasmtime::*;

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_linker<'module, T>(store: &mut Store<T>, module: &Module) -> Result<Linker<T>> {
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    for import in module.imports() {
        linker
            .define(
                import.module(),
                import.name(),
                dummy_extern(store, import.ty())?,
            )
            .unwrap();
    }
    Ok(linker)
}

/// Construct a dummy `Extern` from its type signature
pub fn dummy_extern<T>(store: &mut Store<T>, ty: ExternType) -> Result<Extern> {
    Ok(match ty {
        ExternType::Func(func_ty) => Extern::Func(dummy_func(store, func_ty)),
        ExternType::Global(global_ty) => Extern::Global(dummy_global(store, global_ty)),
        ExternType::Table(table_ty) => Extern::Table(dummy_table(store, table_ty)?),
        ExternType::Memory(mem_ty) => Extern::Memory(dummy_memory(store, mem_ty)?),
    })
}

/// Construct a dummy function for the given function type
pub fn dummy_func<T>(store: &mut Store<T>, ty: FuncType) -> Func {
    Func::new(store, ty.clone(), move |_, _, results| {
        for (ret_ty, result) in ty.results().zip(results) {
            *result = dummy_value(ret_ty);
        }
        Ok(())
    })
}

/// Construct a dummy value for the given value type.
pub fn dummy_value(val_ty: ValType) -> Val {
    match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0),
        ValType::ExternRef => Val::ExternRef(None),
        ValType::FuncRef => Val::FuncRef(None),
    }
}

/// Construct a sequence of dummy values for the given types.
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Vec<Val> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy global for the given global type.
pub fn dummy_global<T>(store: &mut Store<T>, ty: GlobalType) -> Global {
    let val = dummy_value(ty.content().clone());
    Global::new(store, ty, val).unwrap()
}

/// Construct a dummy table for the given table type.
pub fn dummy_table<T>(store: &mut Store<T>, ty: TableType) -> Result<Table> {
    let init_val = dummy_value(ty.element().clone());
    Table::new(store, ty, init_val)
}

/// Construct a dummy memory for the given memory type.
pub fn dummy_memory<T>(store: &mut Store<T>, ty: MemoryType) -> Result<Memory> {
    Memory::new(store, ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store<()> {
        let mut config = Config::default();
        config.wasm_multi_memory(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        Store::new(&engine, ())
    }

    #[test]
    fn dummy_table_import() {
        let mut store = store();
        let table = dummy_table(&mut store, TableType::new(ValType::ExternRef, 10, None)).unwrap();
        assert_eq!(table.size(&store), 10);
        for i in 0..10 {
            assert!(table
                .get(&mut store, i)
                .unwrap()
                .unwrap_externref()
                .is_none());
        }
    }

    #[test]
    fn dummy_global_import() {
        let mut store = store();
        let global = dummy_global(&mut store, GlobalType::new(ValType::I32, Mutability::Const));
        assert_eq!(*global.ty(&store).content(), ValType::I32);
        assert_eq!(global.ty(&store).mutability(), Mutability::Const);
    }

    #[test]
    fn dummy_memory_import() {
        let mut store = store();
        let memory = dummy_memory(&mut store, MemoryType::new(1, None)).unwrap();
        assert_eq!(memory.size(&store), 1);
    }

    #[test]
    fn dummy_function_import() {
        let mut store = store();
        let func_ty = FuncType::new(vec![ValType::I32], vec![ValType::I64]);
        let func = dummy_func(&mut store, func_ty.clone());
        assert_eq!(func.ty(&store), func_ty);
    }
}
