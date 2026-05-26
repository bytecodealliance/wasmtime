#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, FuncType, GlobalType, MemoryType, Mutability, RefType, Result, TableType,
    ValType,
};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn func_type_params_results() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let ty = FuncType::try_new(&engine, [ValType::I32, ValType::I64], [ValType::F32])?;
        assert_eq!(ty.params().len(), 2);
        assert_eq!(ty.results().len(), 1);
        Ok(())
    })
}

#[test]
fn table_type_accessors() -> Result<()> {
    OomTest::new().test(|| {
        let ty = TableType::new(RefType::FUNCREF, 1, Some(10));
        assert_eq!(ty.minimum(), 1);
        assert_eq!(ty.maximum(), Some(10));
        Ok(())
    })
}

#[test]
fn memory_type_accessors() -> Result<()> {
    OomTest::new().test(|| {
        let ty = MemoryType::new(1, Some(10));
        assert_eq!(ty.minimum(), 1);
        assert_eq!(ty.maximum(), Some(10));
        assert!(!ty.is_64());
        assert!(!ty.is_shared());
        Ok(())
    })
}

#[test]
fn global_type_accessors() -> Result<()> {
    OomTest::new().test(|| {
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        assert!(ty.content().is_i32());
        assert_eq!(ty.mutability(), Mutability::Var);
        Ok(())
    })
}

// Note: ExnType::new, StructType::new, and ArrayType::new are not tested under
// OOM yet because their construction goes through the type registry and GC
// layout computation which has additional .panic_on_oom() calls deep in
// crates/environ/src/gc.rs that need to be addressed first.
