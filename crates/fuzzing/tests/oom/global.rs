#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Global, GlobalType, Mutability, Result, Store, Val, ValType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn global_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        let _global = wasmtime::Global::new(&mut store, ty, Val::I32(42))?;
        Ok(())
    })
}

#[test]
fn global_get() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        let global = Global::new(&mut store, ty, Val::I32(42))?;
        let val = global.get(&mut store);
        assert_eq!(val.unwrap_i32(), 42);
        Ok(())
    })
}

#[test]
fn global_set() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        let global = Global::new(&mut store, ty, Val::I32(42))?;
        global.set(&mut store, Val::I32(99))?;
        let val = global.get(&mut store);
        assert_eq!(val.unwrap_i32(), 99);
        Ok(())
    })
}

#[test]
fn global_ty() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = GlobalType::new(ValType::I32, Mutability::Var);
        let global = Global::new(&mut store, ty, Val::I32(42))?;
        let ty = global.ty(&store);
        assert!(ty.content().is_i32());
        Ok(())
    })
}
