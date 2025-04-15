//! Dummy implementations of things that a Wasm module can import.

use anyhow::Context;
use wasmtime::*;

/// Create a set of dummy functions/globals/etc for the given imports.
pub fn dummy_linker<T>(store: &mut Store<T>, module: &Module) -> Result<Linker<T>> {
    let mut linker = Linker::new(store.engine());
    linker.allow_shadowing(true);
    for import in module.imports() {
        let extern_ = import.ty().default_value(&mut *store).with_context(|| {
            format!(
                "failed to create dummy value of `{}::{}` - {:?}",
                import.module(),
                import.name(),
                import.ty(),
            )
        })?;
        linker
            .define(&store, import.module(), import.name(), extern_)
            .unwrap();
    }
    Ok(linker)
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
        let table_type = TableType::new(RefType::EXTERNREF, 10, None);
        let table = table_type.default_value(&mut store).unwrap();
        assert_eq!(table.size(&store), 10);
        for i in 0..10 {
            assert!(table.get(&mut store, i).unwrap().unwrap_extern().is_none());
        }
    }

    #[test]
    fn dummy_global_import() {
        let mut store = store();
        let global_type = GlobalType::new(ValType::I32, Mutability::Const);
        let global = global_type.default_value(&mut store).unwrap();
        assert!(global.ty(&store).content().is_i32());
        assert_eq!(global.ty(&store).mutability(), Mutability::Const);
    }

    #[test]
    fn dummy_memory_import() {
        let mut store = store();
        let memory_type = MemoryType::new(1, None);
        let memory = memory_type.default_value(&mut store).unwrap();
        assert_eq!(memory.size(&store), 1);
    }

    #[test]
    fn dummy_function_import() {
        let mut store = store();
        let func_ty = FuncType::new(store.engine(), vec![ValType::I32], vec![ValType::I64]);
        let func = func_ty.default_value(&mut store).unwrap();
        let actual_ty = func.ty(&store);
        assert!(FuncType::eq(&actual_ty, &func_ty));
    }
}
