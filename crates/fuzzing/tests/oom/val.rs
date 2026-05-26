#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Result, Store, Val};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn val_i32_conversions() -> Result<()> {
    OomTest::new().test(|| {
        let val = Val::I32(42);
        assert_eq!(val.unwrap_i32(), 42);
        Ok(())
    })
}

#[test]
fn val_i64_conversions() -> Result<()> {
    OomTest::new().test(|| {
        let val = Val::I64(42);
        assert_eq!(val.unwrap_i64(), 42);
        Ok(())
    })
}

#[test]
fn val_null_func_ref() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let store = Store::try_new(&engine, ())?;
        let val = Val::null_func_ref();
        let _ = &store;
        assert!(val.ref_().is_some());
        Ok(())
    })
}
