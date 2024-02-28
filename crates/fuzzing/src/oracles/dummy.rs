//! Dummy implementations of things that a Wasm module can import.

use anyhow::bail;
use wasmtime::*;

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_linker<'module, T>(store: &mut Store<T>, module: &Module) -> Result<Linker<T>> {
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    for import in module.imports() {
        let extern_ = dummy_extern(store, import.ty())?;
        linker
            .define(&store, import.module(), import.name(), extern_)
            .unwrap();
    }
    Ok(linker)
}

/// Construct a dummy `Extern` from its type signature
pub fn dummy_extern<T>(store: &mut Store<T>, ty: ExternType) -> Result<Extern> {
    Ok(match ty {
        ExternType::Func(func_ty) => Extern::Func(dummy_func(store, func_ty)?),
        ExternType::Global(global_ty) => Extern::Global(dummy_global(store, global_ty)?),
        ExternType::Table(table_ty) => Extern::Table(dummy_table(store, table_ty)?),
        ExternType::Memory(mem_ty) => Extern::Memory(dummy_memory(store, mem_ty)?),
    })
}

/// Construct a dummy function for the given function type
pub fn dummy_func<T>(store: &mut Store<T>, ty: FuncType) -> Result<Func> {
    let dummy_results = ty.results().map(dummy_value).collect::<Result<Vec<_>>>()?;
    Ok(Func::new(store, ty.clone(), move |_, _, results| {
        for (slot, dummy) in results.iter_mut().zip(&dummy_results) {
            *slot = dummy.clone();
        }
        Ok(())
    }))
}

/// Construct a dummy value for the given value type.
pub fn dummy_value(val_ty: ValType) -> Result<Val> {
    Ok(match val_ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0.into()),
        ValType::Ref(r) => match r.heap_type() {
            _ if !r.is_nullable() => bail!("cannot construct a dummy value of type `{r}`"),
            HeapType::Extern => Val::null_extern_ref(),
            HeapType::NoFunc | HeapType::Func | HeapType::Concrete(_) => Val::null_func_ref(),
            HeapType::Any | HeapType::I31 | HeapType::None => Val::null_any_ref(),
        },
    })
}

/// Construct a sequence of dummy values for the given types.
pub fn dummy_values(val_tys: impl IntoIterator<Item = ValType>) -> Result<Vec<Val>> {
    val_tys.into_iter().map(dummy_value).collect()
}

/// Construct a dummy global for the given global type.
pub fn dummy_global<T>(store: &mut Store<T>, ty: GlobalType) -> Result<Global> {
    let val = dummy_value(ty.content().clone())?;
    Global::new(store, ty, val)
}

/// Construct a dummy table for the given table type.
pub fn dummy_table<T>(store: &mut Store<T>, ty: TableType) -> Result<Table> {
    let init_val = dummy_value(ty.element().clone().into())?;
    Table::new(store, ty, init_val.ref_().unwrap())
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
        let table = dummy_table(&mut store, TableType::new(RefType::EXTERNREF, 10, None)).unwrap();
        assert_eq!(table.size(&store), 10);
        for i in 0..10 {
            assert!(table.get(&mut store, i).unwrap().unwrap_extern().is_none());
        }
    }

    #[test]
    fn dummy_global_import() {
        let mut store = store();
        let global =
            dummy_global(&mut store, GlobalType::new(ValType::I32, Mutability::Const)).unwrap();
        assert!(global.ty(&store).content().is_i32());
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
        let func_ty = FuncType::new(store.engine(), vec![ValType::I32], vec![ValType::I64]);
        let func = dummy_func(&mut store, func_ty.clone()).unwrap();
        let actual_ty = func.ty(&store);
        assert!(FuncType::eq(&actual_ty, &func_ty));
    }
}
